//! `kf comment` — issue comments.
//!
//! Adding a comment to a teammate's issue is legitimate collaboration, so only
//! editing and deleting go through the shared-board guardrail.

use std::path::PathBuf;

use anyhow::Context as _;
use clap::{ArgGroup, Args, Subcommand};

use crate::api::models::{Comment, CreateComment, CreateCommentResponse, UpdateComment};
use crate::api::Client;
use crate::context::Context;
use crate::guard;
use crate::output::{self, Table};
use crate::prompt::confirm;
use crate::resolve;
use crate::users::UserNames;

#[derive(Debug, Subcommand)]
pub enum CommentCommand {
    /// Add a comment to an issue.
    Add(AddArgs),
    /// List an issue's comments.
    List(ListArgs),
    /// Replace a comment's text.
    Edit(EditArgs),
    /// Delete a comment.
    Delete(DeleteArgs),
}

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("body").required(true).args(["text", "file"])))]
pub struct AddArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Comment text.
    #[arg(long)]
    pub text: Option<String>,
    /// Read the comment text from a file.
    #[arg(long)]
    pub file: Option<PathBuf>,
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
#[command(group(ArgGroup::new("body").required(true).args(["text", "file"])))]
pub struct EditArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// ID of the comment to change (see `kf comment list`).
    #[arg(long)]
    pub id: String,
    /// New comment text.
    #[arg(long)]
    pub text: Option<String>,
    /// Read the new comment text from a file.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Edit even when the issue belongs to someone else.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// ID of the comment to delete (see `kf comment list`).
    #[arg(long)]
    pub id: String,
    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,
    /// Delete even when the issue belongs to someone else.
    #[arg(long)]
    pub force: bool,
}

pub fn run(command: CommentCommand) -> anyhow::Result<()> {
    let context = Context::load()?;
    match command {
        CommentCommand::Add(args) => add(&context, args),
        CommentCommand::List(args) => list(&context, args),
        CommentCommand::Edit(args) => edit(&context, args),
        CommentCommand::Delete(args) => delete(&context, args),
    }
}

fn add(context: &Context, args: AddArgs) -> anyhow::Result<()> {
    let text = comment_text(args.text, args.file)?;
    let issue_id = resolve::resolve_issue_id_named(&context.client, &args.issue)?;
    let created: CreateCommentResponse = context
        .client
        .post_json(
            &format!("tasks/{issue_id}/comments"),
            &CreateComment { text },
        )
        .with_context(|| format!("adding a comment to issue {}", args.issue))?;
    println!("{}", created.task_comment_id);
    output::print_affected(&issue_id, None);
    Ok(())
}

fn list(context: &Context, args: ListArgs) -> anyhow::Result<()> {
    let issue_id = resolve::resolve_issue_id_named(&context.client, &args.issue)?;
    let comments = fetch_comments(&context.client, &issue_id)?;

    if args.json {
        output::print_json(&comments)?;
        return Ok(());
    }
    // The comment payload carries only author IDs; names cost one extra request,
    // which `UserNames` spends lazily — an empty listing spends nothing. Authors
    // are shown by name even when they are us: `me` would hide who wrote what.
    let mut authors = UserNames::new(&context.client, None);
    let mut table = Table::new(&["ID", "AUTHOR", "CREATED", "TEXT"]);
    for comment in &comments {
        let author = comment
            .author_user_id
            .as_deref()
            .map(|user_id| authors.display(user_id))
            .unwrap_or_default();
        table.row([
            comment.id.clone(),
            author,
            comment.created_timestamp.clone().unwrap_or_default(),
            output::truncate_cell(&comment.text, 80),
        ]);
    }
    table.print_or("No comments on this issue.");
    Ok(())
}

fn edit(context: &Context, args: EditArgs) -> anyhow::Result<()> {
    let text = comment_text(args.text, args.file)?;
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("editing a comment on issue {}", issue.reference()))?;

    context
        .client
        .post_json_discard(
            &format!("tasks/{}/comments/{}", issue.id, args.id),
            &UpdateComment { text },
        )
        .with_context(|| {
            format!(
                "updating comment {} on issue {}",
                args.id,
                issue.reference()
            )
        })?;
    println!("Updated comment {}.", args.id);
    output::print_affected_issue(&issue);
    Ok(())
}

fn delete(context: &Context, args: DeleteArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("deleting a comment on issue {}", issue.reference()))?;

    if !args.yes
        && !confirm(&format!(
            "Delete comment {} from issue {}?",
            args.id,
            issue.reference()
        ))?
    {
        println!("Cancelled.");
        return Ok(());
    }

    context
        .client
        .delete(&format!("tasks/{}/comments/{}", issue.id, args.id))
        .with_context(|| {
            format!(
                "deleting comment {} from issue {}",
                args.id,
                issue.reference()
            )
        })?;
    println!("Deleted comment {}.", args.id);
    output::print_affected_issue(&issue);
    Ok(())
}

fn fetch_comments(client: &Client, issue_id: &str) -> anyhow::Result<Vec<Comment>> {
    client
        .get_json(&format!("tasks/{issue_id}/comments"), &[])
        .with_context(|| format!("reading the comments of issue {issue_id}"))
}

/// The comment body from exactly one of `--text` / `--file`. clap's ArgGroup
/// already rejects zero or both, so this only reads the file.
fn comment_text(text: Option<String>, file: Option<PathBuf>) -> anyhow::Result<String> {
    match (text, file) {
        (Some(text), None) => Ok(text),
        (None, Some(path)) => {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading the comment text from {}", path.display()))?;
            if text.trim().is_empty() {
                anyhow::bail!("{} is empty; a comment needs text.", path.display());
            }
            Ok(text)
        }
        (Some(_), Some(_)) => anyhow::bail!("Use either --text or --file, not both."),
        (None, None) => anyhow::bail!("A comment needs --text or --file."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_text_prefers_the_single_given_source() {
        assert_eq!(
            comment_text(Some("hello".to_string()), None).expect("text is accepted"),
            "hello"
        );
        assert!(comment_text(None, None).is_err());
        assert!(comment_text(Some("hello".to_string()), Some(PathBuf::from("x"))).is_err());
    }

    #[test]
    fn comment_text_reads_a_file() {
        let path = std::env::temp_dir().join("kf-comment-text-test.md");
        std::fs::write(&path, "from a file\n").expect("temp file is writable");
        assert_eq!(
            comment_text(None, Some(path.clone())).expect("file is read"),
            "from a file\n"
        );
        std::fs::write(&path, "   ").expect("temp file is writable");
        assert!(comment_text(None, Some(path.clone())).is_err());
        std::fs::remove_file(&path).ok();
    }
}
