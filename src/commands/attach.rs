//! `kf attach` — the image round-trip this CLI exists for.
//!
//! Attachment links are pre-signed S3 URLs that expire about 24 hours after the
//! listing call, so `download` streams them to disk immediately and human output
//! never presents a link as something worth keeping.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Context as _;
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::api::models::{AddAttachmentResponse, Attachment, Issue};
use crate::api::{ApiError, Client};
use crate::context::Context;
use crate::files::unique_destination;
use crate::guard;
use crate::output::{self, Table};
use crate::prompt::confirm;
use crate::resolve;

#[derive(Debug, Subcommand)]
pub enum AttachCommand {
    /// Upload files to an issue.
    Add(AddArgs),
    /// List an issue's attachments.
    List(ListArgs),
    /// Download an issue's attachments before their links expire.
    Download(DownloadArgs),
    /// Remove an attachment from an issue.
    Delete(DeleteArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Files to upload.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// Upload even when the issue belongs to someone else.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Only the attachment with this exact name; default is every attachment.
    #[arg(long)]
    pub name: Option<String>,
    /// Directory to save into; created when missing.
    #[arg(long, default_value = ".")]
    pub dir: PathBuf,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Exact name of the attachment to remove.
    #[arg(long)]
    pub name: String,
    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,
    /// Delete even when the issue belongs to someone else.
    #[arg(long)]
    pub force: bool,
}

/// One saved file, as reported by `attach download --json`.
#[derive(Debug, Serialize)]
struct DownloadedFile {
    name: String,
    path: String,
    bytes: u64,
}

pub fn run(command: AttachCommand) -> anyhow::Result<()> {
    let context = Context::load()?;
    match command {
        AttachCommand::Add(args) => add(&context, args),
        AttachCommand::List(args) => list(&context, args),
        AttachCommand::Download(args) => download(&context, args),
        AttachCommand::Delete(args) => delete(&context, args),
    }
}

fn add(context: &Context, args: AddArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("attaching files to issue {}", issue.reference()))?;

    let path = format!("tasks/{}/attachments", issue.id);
    let mut failures = 0usize;
    // The first typed API failure becomes the source of the aggregate error, so
    // an all-401 or all-429 run still exits with its own code instead of 1.
    let mut first_api_error: Option<ApiError> = None;
    for file in &args.files {
        if !file.is_file() {
            failures += 1;
            println!("failed    {}: not a readable file", file.display());
            continue;
        }
        match context
            .client
            .upload_file::<AddAttachmentResponse>(&path, file)
        {
            Ok(response) => println!(
                "uploaded  {}  ({})",
                file.display(),
                response.task_attachment_id
            ),
            Err(error) => {
                failures += 1;
                // Every file gets its own verdict; one bad path must not hide the rest.
                println!("failed    {}: {error}", file.display());
                first_api_error.get_or_insert(error);
            }
        }
    }
    if failures > 0 {
        let summary = format!(
            "{failures} of {} file(s) could not be attached to issue {}",
            args.files.len(),
            issue.reference()
        );
        return Err(match first_api_error {
            Some(error) => anyhow::Error::new(error).context(summary),
            None => anyhow::Error::msg(summary),
        });
    }
    output::print_affected_issue(&issue);
    Ok(())
}

fn list(context: &Context, args: ListArgs) -> anyhow::Result<()> {
    let issue_id = resolve::resolve_issue_id_named(&context.client, &args.issue)?;
    let attachments = fetch_attachments(&context.client, &issue_id)?;

    if args.json {
        output::print_json(&attachments)?;
        return Ok(());
    }
    let mut table = Table::new(&["NAME", "SIZE", "UPLOADED", "BY"]);
    for attachment in &attachments {
        table.row([
            attachment.name.clone(),
            format_size(attachment.size),
            attachment.created_timestamp.clone().unwrap_or_default(),
            attachment.created_by_full_name.clone().unwrap_or_default(),
        ]);
    }
    table.print_or("No attachments on this issue.");
    Ok(())
}

fn download(context: &Context, args: DownloadArgs) -> anyhow::Result<()> {
    let issue_id = resolve::resolve_issue_id_named(&context.client, &args.issue)?;
    let attachments = fetch_attachments(&context.client, &issue_id)?;
    let selected: Vec<&Attachment> = match &args.name {
        Some(name) => attachments
            .iter()
            .filter(|attachment| &attachment.name == name)
            .collect(),
        None => attachments.iter().collect(),
    };
    if selected.is_empty() {
        match &args.name {
            Some(name) => anyhow::bail!(
                "No attachment named `{name}` on issue {}.{}",
                args.issue,
                available_names(&attachments)
            ),
            None => anyhow::bail!("Issue {} has no attachments.", args.issue),
        }
    }

    let mut used_names: HashSet<String> = HashSet::new();
    let mut saved = Vec::with_capacity(selected.len());
    for attachment in selected {
        let destination = unique_destination(&args.dir, &attachment.name, &mut used_names);
        let bytes = context
            .client
            .download_to_file(&attachment.link, &destination)
            .with_context(|| format!("downloading attachment `{}`", attachment.name))?;
        saved.push(DownloadedFile {
            name: attachment.name.clone(),
            path: destination.display().to_string(),
            bytes,
        });
    }

    if args.json {
        output::print_json(&saved)?;
    } else {
        for file in &saved {
            println!("{}", file.path);
        }
    }
    Ok(())
}

fn delete(context: &Context, args: DeleteArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("deleting an attachment of issue {}", issue.reference()))?;

    let attachments = fetch_attachments(&context.client, &issue.id)?;
    let attachment = exactly_one_named(&attachments, &args.name, &issue)?;

    if !args.yes
        && !confirm(&format!(
            "Delete attachment `{}` from issue {}?",
            attachment.name,
            issue.reference()
        ))?
    {
        println!("Cancelled.");
        return Ok(());
    }

    context
        .client
        .delete(&format!("tasks/{}/attachments/{}", issue.id, attachment.id))
        .with_context(|| {
            format!(
                "deleting attachment `{}` from issue {}",
                attachment.name,
                issue.reference()
            )
        })?;
    println!("Deleted attachment `{}`.", attachment.name);
    output::print_affected_issue(&issue);
    Ok(())
}

fn fetch_attachments(client: &Client, issue_id: &str) -> anyhow::Result<Vec<Attachment>> {
    client
        .get_json(&format!("tasks/{issue_id}/attachments"), &[])
        .with_context(|| format!("reading the attachments of issue {issue_id}"))
}

/// Exactly one attachment must carry the name; anything else is a refusal,
/// because deleting the wrong file on a shared board is unrecoverable.
fn exactly_one_named<'a>(
    attachments: &'a [Attachment],
    name: &str,
    issue: &Issue,
) -> anyhow::Result<&'a Attachment> {
    let matches: Vec<&Attachment> = attachments
        .iter()
        .filter(|attachment| attachment.name == name)
        .collect();
    match matches.as_slice() {
        [single] => Ok(single),
        [] => anyhow::bail!(
            "No attachment named `{name}` on issue {}.{}",
            issue.reference(),
            available_names(attachments)
        ),
        several => anyhow::bail!(
            "Issue {} has {} attachments named `{name}`; delete them from the KanbanFlow UI.",
            issue.reference(),
            several.len()
        ),
    }
}

fn available_names(attachments: &[Attachment]) -> String {
    if attachments.is_empty() {
        return String::new();
    }
    let names: Vec<&str> = attachments
        .iter()
        .map(|attachment| attachment.name.as_str())
        .collect();
    format!(" Available: {}", names.join(", "))
}

fn format_size(size: Option<u64>) -> String {
    match size {
        None => String::new(),
        Some(bytes) if bytes < 1024 => format!("{bytes} B"),
        Some(bytes) if bytes < 1024 * 1024 => format!("{:.1} KB", bytes as f64 / 1024.0),
        Some(bytes) => format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attachment(name: &str) -> Attachment {
        Attachment {
            id: format!("A{name}"),
            provider: "KanbanFlow".to_string(),
            name: name.to_string(),
            size: Some(10),
            mime_type: None,
            link: "https://example.invalid/file".to_string(),
            link_expires_timestamp: None,
            created_timestamp: None,
            created_by_full_name: None,
        }
    }

    fn issue() -> Issue {
        serde_json::from_str(r#"{"_id":"T3s6UGyzY","name":"Report","columnId":"C1"}"#)
            .expect("fixture parses")
    }

    #[test]
    fn format_size_scales_units() {
        assert_eq!(format_size(None), "");
        assert_eq!(format_size(Some(512)), "512 B");
        assert_eq!(format_size(Some(2048)), "2.0 KB");
        assert_eq!(format_size(Some(3 * 1024 * 1024)), "3.0 MB");
    }

    #[test]
    fn exactly_one_named_rejects_missing_and_ambiguous() {
        let issue = issue();
        let attachments = vec![
            attachment("a.png"),
            attachment("a.png"),
            attachment("b.png"),
        ];
        assert_eq!(
            exactly_one_named(&attachments, "b.png", &issue)
                .expect("unique name resolves")
                .name,
            "b.png"
        );
        assert!(exactly_one_named(&attachments, "a.png", &issue).is_err());
        assert!(exactly_one_named(&attachments, "c.png", &issue).is_err());
    }
}
