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

use crate::api::models::{AddAttachmentResponse, Attachment, Task};
use crate::api::{ApiError, Client};
use crate::context::Context;
use crate::files::unique_destination;
use crate::guard;
use crate::output::{self, Table};
use crate::prompt::confirm;
use crate::resolve;

#[derive(Debug, Subcommand)]
pub enum AttachCommand {
    /// Upload files to a task.
    Add(AddArgs),
    /// List a task's attachments.
    List(ListArgs),
    /// Download a task's attachments before their links expire.
    Download(DownloadArgs),
    /// Remove an attachment from a task.
    Delete(DeleteArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// Files to upload.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// Upload even when the task belongs to someone else.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
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
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// Exact name of the attachment to remove.
    #[arg(long)]
    pub name: String,
    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,
    /// Delete even when the task belongs to someone else.
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
    let task = resolve::resolve_task_named(&context.client, &args.task)?;
    guard::ensure_can_mutate(&task, context.my_user_id()?, args.force)
        .with_context(|| format!("attaching files to task {}", task.reference()))?;

    let path = format!("tasks/{}/attachments", task.id);
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
            "{failures} of {} file(s) could not be attached to task {}",
            args.files.len(),
            task.reference()
        );
        return Err(match first_api_error {
            Some(error) => anyhow::Error::new(error).context(summary),
            None => anyhow::Error::msg(summary),
        });
    }
    output::print_affected_task(&task);
    Ok(())
}

fn list(context: &Context, args: ListArgs) -> anyhow::Result<()> {
    let task_id = resolve::resolve_task_id_named(&context.client, &args.task)?;
    let attachments = fetch_attachments(&context.client, &task_id)?;

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
    table.print_or("No attachments on this task.");
    Ok(())
}

fn download(context: &Context, args: DownloadArgs) -> anyhow::Result<()> {
    let task_id = resolve::resolve_task_id_named(&context.client, &args.task)?;
    let attachments = fetch_attachments(&context.client, &task_id)?;
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
                "No attachment named `{name}` on task {}.{}",
                args.task,
                available_names(&attachments)
            ),
            None => anyhow::bail!("Task {} has no attachments.", args.task),
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
    let task = resolve::resolve_task_named(&context.client, &args.task)?;
    guard::ensure_can_mutate(&task, context.my_user_id()?, args.force)
        .with_context(|| format!("deleting an attachment of task {}", task.reference()))?;

    let attachments = fetch_attachments(&context.client, &task.id)?;
    let attachment = exactly_one_named(&attachments, &args.name, &task)?;

    if !args.yes
        && !confirm(&format!(
            "Delete attachment `{}` from task {}?",
            attachment.name,
            task.reference()
        ))?
    {
        println!("Cancelled.");
        return Ok(());
    }

    context
        .client
        .delete(&format!("tasks/{}/attachments/{}", task.id, attachment.id))
        .with_context(|| {
            format!(
                "deleting attachment `{}` from task {}",
                attachment.name,
                task.reference()
            )
        })?;
    println!("Deleted attachment `{}`.", attachment.name);
    output::print_affected_task(&task);
    Ok(())
}

fn fetch_attachments(client: &Client, task_id: &str) -> anyhow::Result<Vec<Attachment>> {
    client
        .get_json(&format!("tasks/{task_id}/attachments"), &[])
        .with_context(|| format!("reading the attachments of task {task_id}"))
}

/// Exactly one attachment must carry the name; anything else is a refusal,
/// because deleting the wrong file on a shared board is unrecoverable.
fn exactly_one_named<'a>(
    attachments: &'a [Attachment],
    name: &str,
    task: &Task,
) -> anyhow::Result<&'a Attachment> {
    let matches: Vec<&Attachment> = attachments
        .iter()
        .filter(|attachment| attachment.name == name)
        .collect();
    match matches.as_slice() {
        [single] => Ok(single),
        [] => anyhow::bail!(
            "No attachment named `{name}` on task {}.{}",
            task.reference(),
            available_names(attachments)
        ),
        several => anyhow::bail!(
            "Task {} has {} attachments named `{name}`; delete them from the KanbanFlow UI.",
            task.reference(),
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

    fn task() -> Task {
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
        let task = task();
        let attachments = vec![
            attachment("a.png"),
            attachment("a.png"),
            attachment("b.png"),
        ];
        assert_eq!(
            exactly_one_named(&attachments, "b.png", &task)
                .expect("unique name resolves")
                .name,
            "b.png"
        );
        assert!(exactly_one_named(&attachments, "a.png", &task).is_err());
        assert!(exactly_one_named(&attachments, "c.png", &task).is_err());
    }
}
