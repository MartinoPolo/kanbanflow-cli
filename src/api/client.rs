//! Blocking HTTP client for the KanbanFlow API v1.
//!
//! Rate budget is tight (1000 requests/hour/board, 5000/day locks the token),
//! so every helper here is one request and nothing retries except a single
//! 429 retry when the server tells us how long to wait.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use reqwest::blocking::multipart;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub const API_BASE_URL: &str = "https://kanbanflow.com/api/v1";

/// Longest `Retry-After` we are willing to sleep through before giving up.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Errors mapped from the documented status codes (`docs/api/status-codes.md`).
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("KanbanFlow rejected the API token (401 Unauthorized). Run `kf auth login` with a valid token.")]
    Unauthorized,
    #[error("KanbanFlow refused the request (403 Forbidden): {message}")]
    Forbidden { message: String },
    #[error("Not found (404): {message}")]
    NotFound { message: String },
    #[error("Rate limit hit (429). {advice}")]
    RateLimited { advice: String },
    #[error("KanbanFlow server error ({status}): {message}")]
    Server { status: u16, message: String },
    #[error("Unexpected response from KanbanFlow ({status}): {message}")]
    Unexpected { status: u16, message: String },
    #[error("HTTP request to KanbanFlow failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Could not parse the KanbanFlow response as JSON: {0}")]
    Decode(#[source] serde_json::Error),
    #[error("Writing the downloaded file failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Authenticated client bound to one board (KanbanFlow tokens are per-board).
pub struct Client {
    http: reqwest::blocking::Client,
    token: String,
    base_url: String,
}

impl Client {
    /// Build a client against the production API.
    pub fn new(token: String) -> Result<Self, ApiError> {
        Self::with_base_url(token, API_BASE_URL.to_string())
    }

    /// Build a client against an arbitrary base URL (tests, staging).
    pub fn with_base_url(token: String, base_url: String) -> Result<Self, ApiError> {
        let http = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("kf/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            http,
            token,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// `path` is relative to the API base, e.g. `"tasks/T3s6UGyzY/comments"`.
    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// GET returning a deserialized body. `query` pairs are URL-encoded.
    pub fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, ApiError> {
        let url = self.url(path);
        let response = self.send(|| self.http.get(&url).bearer_auth(&self.token).query(query))?;
        parse_json(response)
    }

    /// POST with a JSON body, returning a deserialized body.
    pub fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let response = self.post_raw(path, body)?;
        parse_json(response)
    }

    /// POST with a JSON body, discarding the (often empty) response body.
    pub fn post_json_discard<B: Serialize>(&self, path: &str, body: &B) -> Result<(), ApiError> {
        self.post_raw(path, body).map(|_| ())
    }

    fn post_raw<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<reqwest::blocking::Response, ApiError> {
        let url = self.url(path);
        self.send(|| self.http.post(&url).bearer_auth(&self.token).json(body))
    }

    /// DELETE; the API returns no meaningful body.
    pub fn delete(&self, path: &str) -> Result<(), ApiError> {
        let url = self.url(path);
        self.send(|| self.http.delete(&url).bearer_auth(&self.token))
            .map(|_| ())
    }

    /// Multipart upload of a single file under the form field `file`, which is
    /// the only field `POST /tasks/<id>/attachments` accepts.
    pub fn upload_file<T: DeserializeOwned>(
        &self,
        path: &str,
        file_path: &Path,
    ) -> Result<T, ApiError> {
        let url = self.url(path);
        // A multipart::Form is consumed by send(), so it is rebuilt per attempt.
        let build_form = || -> Result<multipart::Form, ApiError> {
            Ok(multipart::Form::new().file("file", file_path)?)
        };
        let response = match self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .multipart(build_form()?)
            .send()
        {
            Ok(response) if response.status() == StatusCode::TOO_MANY_REQUESTS => {
                match retry_delay(&response) {
                    Some(delay) => {
                        std::thread::sleep(delay);
                        self.http
                            .post(&url)
                            .bearer_auth(&self.token)
                            .multipart(build_form()?)
                            .send()?
                    }
                    None => return Err(rate_limit_error(&response)),
                }
            }
            Ok(response) => response,
            Err(error) => return Err(ApiError::Transport(error)),
        };
        parse_json(check_status(response)?)
    }

    /// Stream an attachment link straight to disk. Attachment links are
    /// pre-signed S3 URLs, so this request carries no Authorization header.
    pub fn download_to_file(&self, link: &str, destination: &Path) -> Result<u64, ApiError> {
        if let Some(parent) = destination.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut response = check_status(self.http.get(link).send()?)?;
        let mut file = std::fs::File::create(destination)?;
        let bytes = response.copy_to(&mut file)?;
        file.flush()?;
        Ok(bytes)
    }

    /// Send a request, retrying once on 429 when the server supplies a short
    /// `Retry-After`, then map the status onto `ApiError`.
    fn send<F>(&self, build: F) -> Result<reqwest::blocking::Response, ApiError>
    where
        F: Fn() -> reqwest::blocking::RequestBuilder,
    {
        let response = build().send()?;
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            match retry_delay(&response) {
                Some(delay) => {
                    std::thread::sleep(delay);
                    return check_status(build().send()?);
                }
                None => return Err(rate_limit_error(&response)),
            }
        }
        check_status(response)
    }
}

/// `Retry-After` in seconds, when present and short enough to be worth waiting.
fn retry_delay(response: &reqwest::blocking::Response) -> Option<Duration> {
    let seconds: u64 = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let delay = Duration::from_secs(seconds);
    (delay <= MAX_RETRY_AFTER).then_some(delay)
}

fn rate_limit_error(response: &reqwest::blocking::Response) -> ApiError {
    let reset = response
        .headers()
        .get("X-RateLimit-Reset")
        .and_then(|value| value.to_str().ok())
        .map(|value| format!(" Quota resets at UTC epoch second {value}."))
        .unwrap_or_default();
    ApiError::RateLimited {
        advice: format!(
            "The board's budget of 1000 requests/hour is exhausted; wait before retrying \
             (more than 5000 requests/day locks the token).{reset}"
        ),
    }
}

/// Turn a non-2xx response into a typed error carrying the server's message.
fn check_status(
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response, ApiError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(rate_limit_error(&response));
    }
    let message = error_message(response);
    Err(match status {
        StatusCode::UNAUTHORIZED => ApiError::Unauthorized,
        StatusCode::FORBIDDEN => ApiError::Forbidden { message },
        StatusCode::NOT_FOUND => ApiError::NotFound { message },
        _ if status.is_server_error() => ApiError::Server {
            status: status.as_u16(),
            message,
        },
        _ => ApiError::Unexpected {
            status: status.as_u16(),
            message,
        },
    })
}

/// The API reports failures either as JSON (`{"message": ...}`) or plain text.
fn error_message(response: reqwest::blocking::Response) -> String {
    let body = response.text().unwrap_or_default();
    if body.trim().is_empty() {
        return "no message returned".to_string();
    }
    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(value) => ["message", "error", "errorMessage"]
            .iter()
            .find_map(|key| value.get(key).and_then(|found| found.as_str()))
            .map(str::to_string)
            .unwrap_or(body),
        Err(_) => body,
    }
}

fn parse_json<T: DeserializeOwned>(response: reqwest::blocking::Response) -> Result<T, ApiError> {
    let status = response.status().as_u16();
    let body = response.text()?;
    // Some write endpoints answer 200 with an empty body; `()` and Option targets
    // still need valid JSON, so an empty body becomes `null`.
    let body = if body.trim().is_empty() {
        "null"
    } else {
        &body
    };
    serde_json::from_str(body).map_err(|error| {
        if status >= 400 {
            ApiError::Unexpected {
                status,
                message: body.to_string(),
            }
        } else {
            ApiError::Decode(error)
        }
    })
}
