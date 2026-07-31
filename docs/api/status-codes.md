# Status codes

> Source: https://kanbanflow.com/api-docs/status-codes
> Scraped: 2026-07-30

# API Documentation

* * *

## HTTP status codes

All requests you make return an HTTP status code.

| Code | Text | Description |
| --- | --- | --- |
| 200 | OK | Successful request. |
| 401 | Unauthorized | Credentials were missing or incorrect. |
| 403 | Forbidden | Your request is understood, but it is refused. An error message should explain why. |
| 404 | Not found | The resource is not found. |
| 429 | Too many requests | You have sent too many requests in a given period of time. Respect rate limit to avoid having API token locked. |
| 500 | Internal server error | Something went wrong. Please report recurring 500 responses. |
