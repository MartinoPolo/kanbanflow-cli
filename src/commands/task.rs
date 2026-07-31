//! `kf task` — create, read and change tasks.
//!
//! The compound verbs (`view`, `grab`, `finish`, `create --attach`) exist because
//! the API forces multi-call dances: a full task read is task + comments +
//! attachments, and attachment links expire ~24h after they are handed out.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;

use crate::api::models::{
    Attachment, Comment, CreateComment, CreateCommentResponse, CreateTask, CreateTaskResponse,
    Label, SubTask, SubTaskPayload, Task, TaskGroup, UpdateTask,
};
use crate::api::{ApiError, Client};
use crate::config::{CanonicalState, Config};
use crate::context::Context;
use crate::files::unique_destination;
use crate::guard::{self, GuardError};
use crate::labels;
use crate::output::{self, Table};
use crate::prompt::confirm;
use crate::resolve;
use crate::tasks;
use crate::users::UserNames;

#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    /// Create a task, optionally uploading attachments in the same step.
    Create(CreateArgs),
    /// Show a task with its comments and attachments.
    View(ViewArgs),
    /// List tasks on the board.
    List(ListArgs),
    /// Change a task's fields.
    Edit(EditArgs),
    /// Move a task to a canonical workflow state.
    Move(MoveArgs),
    /// Delete a task.
    Delete(DeleteArgs),
    /// Assign the task to yourself, move it to WIP and show it.
    Grab(GrabArgs),
    /// Comment, move out of WIP and optionally check off subtasks.
    Finish(FinishArgs),
}

/// The card colors the API accepts (`docs/api/create-task.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum CardColor {
    Yellow,
    White,
    Red,
    Green,
    Blue,
    Purple,
    Orange,
    Cyan,
    Brown,
    Magenta,
}

impl CardColor {
    fn as_str(self) -> &'static str {
        match self {
            CardColor::Yellow => "yellow",
            CardColor::White => "white",
            CardColor::Red => "red",
            CardColor::Green => "green",
            CardColor::Blue => "blue",
            CardColor::Purple => "purple",
            CardColor::Orange => "orange",
            CardColor::Cyan => "cyan",
            CardColor::Brown => "brown",
            CardColor::Magenta => "magenta",
        }
    }
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Task name.
    #[arg(long)]
    pub name: String,
    /// Task description (Markdown, as the board renders it).
    #[arg(long)]
    pub description: Option<String>,
    /// Card color.
    #[arg(long, value_enum)]
    pub color: Option<CardColor>,
    /// Canonical state (column) to create the task in.
    #[arg(long, value_enum, default_value_t = CanonicalState::Todo)]
    pub to: CanonicalState,
    /// Existing board label to apply; repeatable. Unknown labels are refused.
    #[arg(long = "label", value_name = "NAME")]
    pub labels: Vec<String>,
    /// Responsible user: `me` or a user ID; defaults to you, pass `none` to leave unassigned.
    #[arg(long)]
    pub responsible: Option<String>,
    /// File to upload onto the new task; repeatable.
    #[arg(long = "attach", value_name = "FILE")]
    pub attachments: Vec<PathBuf>,
    /// Grouping date (`YYYY-MM-DD`) for date-grouped columns; server defaults to today.
    #[arg(long, value_name = "YYYY-MM-DD")]
    pub grouping_date: Option<String>,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ViewArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// Save every attachment into this directory (links expire, so do it now).
    #[arg(long, value_name = "DIR")]
    pub download_attachments: Option<PathBuf>,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Only tasks in this canonical state.
    #[arg(long, value_enum)]
    pub state: Option<CanonicalState>,
    /// Only tasks in this column, by name or ID (for columns with no canonical state).
    #[arg(long, value_name = "NAME_OR_ID", conflicts_with = "state")]
    pub column: Option<String>,
    /// Only tasks you are responsible for.
    #[arg(long)]
    pub mine: bool,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// Replace the task name.
    #[arg(long)]
    pub name: Option<String>,
    /// Replace the description.
    #[arg(long, conflicts_with = "append_description")]
    pub description: Option<String>,
    /// Append to the description, separated by a blank line (lossless merge).
    #[arg(long, value_name = "TEXT")]
    pub append_description: Option<String>,
    /// Card color.
    #[arg(long, value_enum)]
    pub color: Option<CardColor>,
    /// Add an existing board label; repeatable.
    #[arg(long = "add-label", value_name = "NAME")]
    pub add_labels: Vec<String>,
    /// Remove a label; repeatable.
    #[arg(long = "remove-label", value_name = "NAME")]
    pub remove_labels: Vec<String>,
    /// Responsible user: `me`, a user ID, or `none` to clear.
    #[arg(long)]
    pub responsible: Option<String>,
    /// Mutate a task that is not yours.
    #[arg(long)]
    pub force: bool,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MoveArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// Target canonical state.
    #[arg(long, value_enum)]
    pub to: CanonicalState,
    /// Grouping date (`YYYY-MM-DD`) when the target column is date grouped.
    /// Omitted, the server files the task under today's UTC date.
    #[arg(long, value_name = "YYYY-MM-DD")]
    pub grouping_date: Option<String>,
    /// Mutate a task that is not yours.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,
    /// Delete a task that is not yours.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct GrabArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// Save the task's image attachments into this directory.
    #[arg(long, value_name = "DIR")]
    pub download_dir: Option<PathBuf>,
    /// Grab a task someone else is responsible for.
    #[arg(long)]
    pub force: bool,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct FinishArgs {
    /// Task number (`E613`) or task ID.
    pub task: String,
    /// File whose contents become the closing comment.
    #[arg(long, value_name = "PATH")]
    pub comment_file: Option<PathBuf>,
    /// Target canonical state.
    #[arg(long, value_enum, default_value_t = CanonicalState::Done)]
    pub to: CanonicalState,
    /// Mark every unfinished subtask finished.
    #[arg(long)]
    pub check_subtasks: bool,
    /// Finish a task that is not yours.
    #[arg(long)]
    pub force: bool,
}

pub fn run(command: TaskCommand) -> anyhow::Result<()> {
    let context = Context::load()?;
    match command {
        TaskCommand::Create(args) => create(&context, args),
        TaskCommand::View(args) => view(&context, args),
        TaskCommand::List(args) => list(&context, args),
        TaskCommand::Edit(args) => edit(&context, args),
        TaskCommand::Move(args) => move_task(&context, args),
        TaskCommand::Delete(args) => delete(&context, args),
        TaskCommand::Grab(args) => grab(&context, args),
        TaskCommand::Finish(args) => finish(&context, args),
    }
}

// ---------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------

/// What one `--attach` file did, so a failed upload never hides the others.
#[derive(Debug, Serialize)]
struct UploadOutcome {
    file: String,
    uploaded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateOutcome {
    #[serde(rename = "taskId")]
    task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attachments: Vec<UploadOutcome>,
}

fn create(context: &Context, args: CreateArgs) -> anyhow::Result<()> {
    let column_id = context.config.column_id(args.to)?.to_string();
    let labels = if args.labels.is_empty() {
        None
    } else {
        let vocabulary = labels::fetch_names(&context.client)?;
        Some(labels::canonicalize(&args.labels, &vocabulary)?)
    };
    if let Some(date) = &args.grouping_date {
        ensure_iso_date(date)?;
    }

    let payload = CreateTask {
        name: args.name,
        column_id,
        description: args.description,
        color: args.color.map(|color| color.as_str().to_string()),
        responsible_user_id: create_responsible_user_id(
            args.responsible.as_deref(),
            context.my_user_id(),
        )?,
        grouping_date: args.grouping_date,
        labels,
        ..Default::default()
    };
    let created: CreateTaskResponse = context
        .client
        .post_json("tasks", &payload)
        .context("creating the task")?;

    let number = created.number.as_ref().map(ToString::to_string);
    // The task exists from here on: an upload failure is reported, never fatal.
    let uploads = upload_attachments(&context.client, &created.task_id, &args.attachments);

    if args.json {
        output::print_json(&CreateOutcome {
            task_id: created.task_id,
            number,
            attachments: uploads,
        })?;
        return Ok(());
    }

    output::print_affected(&created.task_id, number);
    for upload in &uploads {
        match &upload.error {
            None => println!("attached {}", upload.file),
            Some(error) => eprintln!("could not attach {}: {error}", upload.file),
        }
    }
    Ok(())
}

fn upload_attachments(client: &Client, task_id: &str, files: &[PathBuf]) -> Vec<UploadOutcome> {
    files
        .iter()
        .map(|file| {
            let path = format!("tasks/{task_id}/attachments");
            match client.upload_file::<crate::api::models::AddAttachmentResponse>(&path, file) {
                Ok(response) => UploadOutcome {
                    file: file.display().to_string(),
                    uploaded: true,
                    attachment_id: Some(response.task_attachment_id),
                    error: None,
                },
                Err(error) => UploadOutcome {
                    file: file.display().to_string(),
                    uploaded: false,
                    attachment_id: None,
                    error: Some(error.to_string()),
                },
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// view
// ---------------------------------------------------------------------------

/// The three calls a full task read costs, as one `--json` object.
#[derive(Debug, Serialize)]
struct TaskAggregate {
    task: Task,
    comments: Vec<Comment>,
    attachments: Vec<Attachment>,
}

fn fetch_aggregate(client: &Client, task: Task) -> Result<TaskAggregate, ApiError> {
    let comments = client.get_json(&format!("tasks/{}/comments", task.id), &[])?;
    let attachments = client.get_json(&format!("tasks/{}/attachments", task.id), &[])?;
    Ok(TaskAggregate {
        task,
        comments,
        attachments,
    })
}

fn view(context: &Context, args: ViewArgs) -> anyhow::Result<()> {
    let task = resolve::resolve_task_named(&context.client, &args.task)?;
    let aggregate = fetch_aggregate(&context.client, task)
        .with_context(|| format!("reading task {}", args.task))?;

    if let Some(directory) = &args.download_attachments {
        let saved =
            download_attachments(&context.client, &aggregate.attachments, directory, false)?;
        if !args.json {
            for path in &saved {
                println!("saved {}", path.display());
            }
        }
    }

    if args.json {
        return Ok(output::print_json(&aggregate)?);
    }
    let mut users = UserNames::new(&context.client, context.my_user_id());
    print_aggregate(&aggregate, &context.config, &mut users);
    Ok(())
}

fn print_aggregate(aggregate: &TaskAggregate, config: &Config, users: &mut UserNames<'_>) {
    let task = &aggregate.task;
    println!("{}  {}", task.reference(), task.name);
    println!("State:        {}", describe_column(config, &task.column_id));
    if let Some(color) = &task.color {
        println!("Color:        {color}");
    }
    println!(
        "Responsible:  {}",
        match task.responsible_user_id.as_deref() {
            Some(user_id) => users.display(user_id),
            None => "unassigned".to_string(),
        }
    );
    if let Some(date) = &task.grouping_date {
        println!("Grouped:      {date}");
    }
    if !task.labels.is_empty() {
        println!("Labels:       {}", join_labels(&task.labels));
    }

    if let Some(description) = task.description.as_deref().map(str::trim) {
        if !description.is_empty() {
            println!("\nDescription\n{description}");
        }
    }

    if !task.sub_tasks.is_empty() {
        println!("\nSubtasks");
        for (index, sub_task) in task.sub_tasks.iter().enumerate() {
            println!("{}", format_subtask_line(index, sub_task));
        }
    }

    println!("\nComments ({})", aggregate.comments.len());
    for comment in &aggregate.comments {
        let author = comment
            .author_user_id
            .as_deref()
            .map(|user_id| users.display(user_id))
            .unwrap_or_else(|| "unknown".to_string());
        let when = comment.created_timestamp.as_deref().unwrap_or("");
        println!("  {author}  {when}");
        for line in comment.text.lines() {
            println!("    {line}");
        }
    }

    println!("\nAttachments ({})", aggregate.attachments.len());
    let mut table = Table::new(&["NAME", "SIZE", "TYPE", "ADDED"]);
    for attachment in &aggregate.attachments {
        table.row([
            output::truncate_cell(&attachment.name, 48),
            attachment
                .size
                .map(|size| size.to_string())
                .unwrap_or_default(),
            attachment.mime_type.clone().unwrap_or_default(),
            attachment.created_timestamp.clone().unwrap_or_default(),
        ]);
    }
    if !table.is_empty() {
        table.print();
    }
}

/// Render one checklist line for `task view`. `index` is the 0-based position in
/// `sub_tasks`; the printed number is 1-based so it matches `kf subtask list` and the
/// position accepted by `kf subtask check`.
fn format_subtask_line(index: usize, sub_task: &SubTask) -> String {
    let mark = if sub_task.finished { 'x' } else { ' ' };
    format!("  {}. [{mark}] {}", index + 1, sub_task.name)
}

/// Save attachments into `directory`, optionally only the images.
fn download_attachments(
    client: &Client,
    attachments: &[Attachment],
    directory: &Path,
    images_only: bool,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut saved = Vec::new();
    let mut used_names: HashSet<String> = HashSet::new();
    for attachment in attachments {
        if images_only && !is_image(attachment) {
            continue;
        }
        let destination = unique_destination(directory, &attachment.name, &mut used_names);
        client
            .download_to_file(&attachment.link, &destination)
            .with_context(|| format!("downloading attachment `{}`", attachment.name))?;
        saved.push(destination);
    }
    Ok(saved)
}

fn is_image(attachment: &Attachment) -> bool {
    if let Some(mime_type) = &attachment.mime_type {
        return mime_type.starts_with("image/");
    }
    matches!(
        extension_of(&attachment.name).as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg")
    )
}

fn extension_of(name: &str) -> Option<String> {
    name.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

fn list(context: &Context, args: ListArgs) -> anyhow::Result<()> {
    // Paged, not raw: a silently truncated Done column would make `list` lie
    // about what is on the board.
    let groups: Vec<TaskGroup> =
        tasks::fetch_all_groups(&context.client).context("listing the board's tasks")?;

    let wanted_column = match args.state {
        Some(state) => Some(context.config.column_id(state)?.to_string()),
        None => None,
    };
    let selected: Vec<&TaskGroup> = groups
        .iter()
        .filter(|group| match (&wanted_column, &args.column) {
            (Some(column_id), _) => &group.column_id == column_id,
            (None, Some(needle)) => {
                group.column_id == *needle || group.column_name.eq_ignore_ascii_case(needle)
            }
            (None, None) => true,
        })
        .collect();

    if let Some(needle) = &args.column {
        if selected.is_empty() {
            anyhow::bail!(
                "No column named or numbered `{needle}` on this board. Known columns: {}",
                groups
                    .iter()
                    .map(|group| group.column_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    let tasks: Vec<&Task> = selected
        .iter()
        .flat_map(|group| group.tasks.iter())
        .filter(|task| {
            !args.mine || task.responsible_user_id.as_deref() == Some(context.my_user_id())
        })
        .collect();

    if args.json {
        return Ok(output::print_json(&tasks)?);
    }

    let mut users = UserNames::new(&context.client, context.my_user_id());
    let mut table = Table::new(&["NUMBER", "STATE", "NAME", "RESPONSIBLE", "LABELS"]);
    for task in &tasks {
        table.row([
            task.reference(),
            describe_column(&context.config, &task.column_id),
            output::truncate_cell(&task.name, 60),
            task.responsible_user_id
                .as_deref()
                .map(|user_id| users.display(user_id))
                .unwrap_or_default(),
            join_labels(&task.labels),
        ]);
    }
    table.print_or("No tasks match.");
    Ok(())
}

// ---------------------------------------------------------------------------
// edit
// ---------------------------------------------------------------------------

fn edit(context: &Context, args: EditArgs) -> anyhow::Result<()> {
    let task = resolve::resolve_task_named(&context.client, &args.task)?;
    guard::ensure_can_mutate(&task, context.my_user_id(), args.force)
        .with_context(|| format!("editing task {}", task.reference()))?;

    let mut update = UpdateTask {
        name: args.name,
        color: args.color.map(|color| color.as_str().to_string()),
        ..Default::default()
    };
    update.description = match &args.append_description {
        Some(addition) => Some(append_description(task.description.as_deref(), addition)),
        None => args.description,
    };
    if let Some(responsible) = &args.responsible {
        update.responsible_user_id = Some(
            responsible_user_id(responsible, context.my_user_id())?
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    if !args.add_labels.is_empty() || !args.remove_labels.is_empty() {
        let added = if args.add_labels.is_empty() {
            Vec::new()
        } else {
            let vocabulary = labels::fetch_names(&context.client)?;
            labels::canonicalize(&args.add_labels, &vocabulary)?
                .into_iter()
                .map(|label| label.name)
                .collect()
        };
        // The API replaces the whole label collection, so adding one label means
        // writing back the set read a moment ago. A teammate's concurrent label
        // change in that window is silently lost — the endpoint offers no
        // per-label add/remove and no version to check against.
        update.labels = changed_labels(&task.labels, &added, &args.remove_labels);
    }

    let changes = serde_json::to_value(&update).context("building the update payload")?;
    if changes.as_object().map(serde_json::Map::len) == Some(0) {
        anyhow::bail!("Nothing to change: pass at least one of --name, --description, --append-description, --color, --add-label, --remove-label, --responsible.");
    }

    context
        .client
        .post_json_discard(&format!("tasks/{}", task.id), &update)
        .with_context(|| format!("updating task {}", task.reference()))?;

    if args.json {
        // The write already landed; only the read-back can still fail, and the
        // caller must not mistake that for a rejected edit.
        let updated = resolve::resolve_task(&context.client, &task.id).with_context(|| {
            format!(
                "the edit to task {} SUCCEEDED, but reading the task back for --json failed",
                task.reference()
            )
        })?;
        return Ok(output::print_json(&updated)?);
    }
    output::print_affected_task(&task);
    Ok(())
}

/// Append to a description without touching what is already there; the blank
/// line keeps the two blocks separate in the board's Markdown rendering.
fn append_description(current: Option<&str>, addition: &str) -> String {
    let current = current.unwrap_or("").trim_end();
    let addition = addition.trim();
    if current.trim().is_empty() {
        return addition.to_string();
    }
    format!("{current}\n\n{addition}")
}

/// New label collection after adds and removes, or `None` when nothing changes.
/// `labels` replaces the whole collection server-side, so this is a full rewrite.
fn changed_labels(current: &[Label], added: &[String], removed: &[String]) -> Option<Vec<Label>> {
    let mut labels: Vec<Label> = current
        .iter()
        .filter(|label| {
            !removed
                .iter()
                .any(|name| name.trim().eq_ignore_ascii_case(&label.name))
        })
        .cloned()
        .collect();
    for name in added {
        let already_present = labels
            .iter()
            .any(|label| label.name.eq_ignore_ascii_case(name));
        if !already_present {
            labels.push(Label {
                name: name.clone(),
                pinned: None,
            });
        }
    }
    let unchanged = labels.len() == current.len()
        && labels
            .iter()
            .zip(current)
            .all(|(new, old)| new.name == old.name);
    (!unchanged).then_some(labels)
}

// ---------------------------------------------------------------------------
// move / delete
// ---------------------------------------------------------------------------

fn move_task(context: &Context, args: MoveArgs) -> anyhow::Result<()> {
    let task = resolve::resolve_task_named(&context.client, &args.task)?;
    guard::ensure_can_mutate(&task, context.my_user_id(), args.force)
        .with_context(|| format!("moving task {} to {}", task.reference(), args.to))?;
    let column_id = context.config.column_id(args.to)?.to_string();
    if let Some(date) = &args.grouping_date {
        ensure_iso_date(date)?;
    }

    // groupingDate is omitted unless asked for: on a date-grouped column the
    // server then files the task under today's UTC date, which is what a move
    // into Done means anyway.
    let update = UpdateTask {
        column_id: Some(column_id),
        grouping_date: args.grouping_date,
        ..Default::default()
    };
    context
        .client
        .post_json_discard(&format!("tasks/{}", task.id), &update)
        .with_context(|| format!("moving task {} to {}", task.reference(), args.to))?;
    output::print_affected_task(&task);
    Ok(())
}

fn delete(context: &Context, args: DeleteArgs) -> anyhow::Result<()> {
    let task = resolve::resolve_task_named(&context.client, &args.task)?;
    guard::ensure_can_mutate(&task, context.my_user_id(), args.force)
        .with_context(|| format!("deleting task {}", task.reference()))?;
    if !args.yes
        && !confirm(&format!(
            "Delete task {} ({})?",
            task.reference(),
            task.name
        ))?
    {
        println!("Cancelled.");
        return Ok(());
    }
    context
        .client
        .delete(&format!("tasks/{}", task.id))
        .with_context(|| format!("deleting task {}", task.reference()))?;
    output::print_affected_task(&task);
    Ok(())
}

// ---------------------------------------------------------------------------
// grab / finish
// ---------------------------------------------------------------------------

fn grab(context: &Context, args: GrabArgs) -> anyhow::Result<()> {
    let mut task = resolve::resolve_task_named(&context.client, &args.task)?;
    ensure_can_grab(&task, context.my_user_id(), args.force)
        .with_context(|| format!("grabbing task {}", task.reference()))?;
    let column_id = context.config.column_id(CanonicalState::Wip)?.to_string();

    let update = UpdateTask {
        column_id: Some(column_id.clone()),
        responsible_user_id: Some(serde_json::Value::String(context.my_user_id().to_string())),
        ..Default::default()
    };
    context
        .client
        .post_json_discard(&format!("tasks/{}", task.id), &update)
        .with_context(|| format!("grabbing task {}", task.reference()))?;
    // Mirror the accepted update locally instead of spending a re-read.
    task.column_id = column_id;
    task.responsible_user_id = Some(context.my_user_id().to_string());

    // Everything below is read-only follow-up. The single POST above already
    // landed both the assignment and the move, so failures here say so.
    let grabbed = format!(
        "task {} WAS assigned to you and moved to wip",
        task.reference()
    );
    let aggregate = fetch_aggregate(&context.client, task)
        .with_context(|| format!("reading task {} back after the grab ({grabbed})", args.task))?;
    if let Some(directory) = &args.download_dir {
        let saved = download_attachments(&context.client, &aggregate.attachments, directory, true)
            .with_context(|| format!("downloading the task's images ({grabbed})"))?;
        if !args.json {
            for path in &saved {
                println!("saved {}", path.display());
            }
        }
    }

    if args.json {
        return Ok(output::print_json(&aggregate)?);
    }
    let mut users = UserNames::new(&context.client, context.my_user_id());
    print_aggregate(&aggregate, &context.config, &mut users);
    Ok(())
}

/// Grabbing assigns the task to us, so an unassigned task is fair game; only a
/// teammate's task needs `--force`.
fn ensure_can_grab(task: &Task, my_user_id: &str, force: bool) -> Result<(), GuardError> {
    match task.responsible_user_id.as_deref() {
        Some(owner) if owner != my_user_id => guard::ensure_can_mutate(task, my_user_id, force),
        _ => Ok(()),
    }
}

/// `finish` writes in three steps and the API has no transaction, so a failure
/// partway leaves earlier writes in place. Tracking them lets the error say
/// exactly what landed instead of leaving the caller to guess.
#[derive(Debug, Default, Clone, Copy)]
struct FinishProgress {
    comment_posted: bool,
    subtasks_checked: usize,
    subtasks_to_check: usize,
}

impl FinishProgress {
    /// What has already landed on the board, in one clause.
    fn landed(&self) -> String {
        let mut parts = Vec::new();
        if self.comment_posted {
            parts.push("the closing comment WAS posted".to_string());
        }
        if self.subtasks_to_check > 0 {
            parts.push(format!(
                "{}/{} subtasks WERE checked",
                self.subtasks_checked, self.subtasks_to_check
            ));
        }
        if parts.is_empty() {
            return "nothing had been written yet".to_string();
        }
        parts.join(" and ")
    }

    /// The landed-writes clause for a failure before the move.
    fn note_before_move(&self) -> String {
        format!("{}; the task was NOT moved", self.landed())
    }
}

fn finish(context: &Context, args: FinishArgs) -> anyhow::Result<()> {
    let task = resolve::resolve_task_named(&context.client, &args.task)?;
    guard::ensure_can_mutate(&task, context.my_user_id(), args.force)
        .with_context(|| format!("finishing task {}", task.reference()))?;
    let column_id = context.config.column_id(args.to)?.to_string();
    let mut progress = FinishProgress::default();

    if let Some(path) = &args.comment_file {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading the closing comment from `{}`", path.display()))?;
        let comment = CreateComment {
            text: text.trim_end().to_string(),
        };
        let _: CreateCommentResponse = context
            .client
            .post_json(&format!("tasks/{}/comments", task.id), &comment)
            .with_context(|| {
                format!(
                    "commenting on task {} ({})",
                    task.reference(),
                    progress.note_before_move()
                )
            })?;
        progress.comment_posted = true;
    }

    if args.check_subtasks {
        let unfinished = unfinished_subtask_indexes(&task);
        progress.subtasks_to_check = unfinished.len();
        for index in unfinished {
            let payload = SubTaskPayload {
                finished: Some(true),
                ..Default::default()
            };
            context
                .client
                .post_json_discard(
                    &format!("tasks/{}/subtasks/by-index/{index}", task.id),
                    &payload,
                )
                .with_context(|| {
                    format!(
                        "checking subtask {index} of task {} ({})",
                        task.reference(),
                        progress.note_before_move()
                    )
                })?;
            progress.subtasks_checked += 1;
        }
    }

    let update = UpdateTask {
        column_id: Some(column_id),
        ..Default::default()
    };
    context
        .client
        .post_json_discard(&format!("tasks/{}", task.id), &update)
        .with_context(|| {
            format!(
                "moving task {} to {} ({})",
                task.reference(),
                args.to,
                progress.note_before_move()
            )
        })?;
    output::print_affected_task(&task);
    Ok(())
}

fn unfinished_subtask_indexes(task: &Task) -> Vec<usize> {
    task.sub_tasks
        .iter()
        .enumerate()
        .filter(|(_, sub_task)| !sub_task.finished)
        .map(|(index, _)| index)
        .collect()
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// The canonical state of a column, or the raw column ID when it is unmapped.
fn describe_column(config: &Config, column_id: &str) -> String {
    match config.state_for_column(column_id) {
        Some(state) => state.to_string(),
        None => column_id.to_string(),
    }
}

fn join_labels(labels: &[Label]) -> String {
    labels
        .iter()
        .map(|label| label.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `me` resolves to the token's user, `none` clears, anything else is a user ID.
fn responsible_user_id(value: &str, my_user_id: &str) -> anyhow::Result<Option<String>> {
    match value.trim() {
        "" => anyhow::bail!("--responsible needs `me`, a user ID, or `none`"),
        "me" => Ok(Some(my_user_id.to_string())),
        "none" | "null" => Ok(None),
        user_id => Ok(Some(user_id.to_string())),
    }
}

/// Created tasks are yours by default, so the ownership guard does not block follow-ups.
fn create_responsible_user_id(
    value: Option<&str>,
    my_user_id: &str,
) -> anyhow::Result<Option<String>> {
    match value {
        None => Ok(Some(my_user_id.to_string())),
        Some(value) => responsible_user_id(value, my_user_id),
    }
}

fn ensure_iso_date(value: &str) -> anyhow::Result<()> {
    if is_iso_date(value) {
        return Ok(());
    }
    anyhow::bail!("`{value}` is not a date in YYYY-MM-DD format")
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .filter(|(index, _)| ![4, 7].contains(index))
            .all(|(_, byte)| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(names: &[&str]) -> Vec<Label> {
        names
            .iter()
            .map(|name| Label {
                name: name.to_string(),
                pinned: None,
            })
            .collect()
    }

    #[test]
    fn format_subtask_line_numbers_from_one() {
        let sub_task = SubTask {
            name: "Reproduce on staging".to_string(),
            finished: false,
            user_id: None,
            due_date_timestamp: None,
            due_date_timestamp_local: None,
        };
        assert_eq!(
            format_subtask_line(0, &sub_task),
            "  1. [ ] Reproduce on staging"
        );
        let finished = SubTask {
            finished: true,
            ..sub_task
        };
        assert_eq!(
            format_subtask_line(2, &finished),
            "  3. [x] Reproduce on staging"
        );
    }

    #[test]
    fn append_description_separates_blocks_with_a_blank_line() {
        assert_eq!(
            append_description(Some("Original text"), "  Added later  "),
            "Original text\n\nAdded later"
        );
    }

    #[test]
    fn append_description_on_empty_description_keeps_only_the_addition() {
        assert_eq!(append_description(None, "First words"), "First words");
        assert_eq!(
            append_description(Some("   \n"), "First words"),
            "First words"
        );
    }

    #[test]
    fn changed_labels_adds_removes_and_detects_no_op() {
        let current = labels(&["Bug", "Yoursafe Components"]);
        let added = vec!["Flowguard SDK".to_string()];
        let changed = changed_labels(&current, &added, &["bug".to_string()])
            .expect("adding and removing changes the collection");
        assert_eq!(
            changed
                .iter()
                .map(|label| label.name.as_str())
                .collect::<Vec<_>>(),
            ["Yoursafe Components", "Flowguard SDK"]
        );
        assert!(changed_labels(&current, &["BUG".to_string()], &[]).is_none());
        assert!(changed_labels(&current, &[], &["missing".to_string()]).is_none());
    }

    #[test]
    fn responsible_accepts_me_ids_and_none() {
        assert_eq!(
            responsible_user_id("me", "UHJ9JgtA").expect("valid"),
            Some("UHJ9JgtA".to_string())
        );
        assert_eq!(
            responsible_user_id("UOTHER", "UHJ9JgtA").expect("valid"),
            Some("UOTHER".to_string())
        );
        assert_eq!(
            responsible_user_id("none", "UHJ9JgtA").expect("valid"),
            None
        );
        assert!(responsible_user_id("  ", "UHJ9JgtA").is_err());
    }

    #[test]
    fn create_defaults_responsible_to_the_config_user() {
        assert_eq!(
            create_responsible_user_id(None, "UHJ9JgtA").expect("valid"),
            Some("UHJ9JgtA".to_string())
        );
        assert_eq!(
            create_responsible_user_id(Some("none"), "UHJ9JgtA").expect("valid"),
            None
        );
        assert_eq!(
            create_responsible_user_id(Some("UOTHER"), "UHJ9JgtA").expect("valid"),
            Some("UOTHER".to_string())
        );
        assert_eq!(
            create_responsible_user_id(Some("me"), "UHJ9JgtA").expect("valid"),
            Some("UHJ9JgtA".to_string())
        );
    }

    #[test]
    fn iso_dates_are_validated() {
        assert!(is_iso_date("2026-07-31"));
        assert!(!is_iso_date("2026-7-31"));
        assert!(!is_iso_date("31/07/2026"));
    }

    #[test]
    fn unfinished_subtasks_are_reported_by_index() {
        let task: Task = serde_json::from_str(
            r#"{"_id":"T1","name":"n","columnId":"C1","subTasks":[
                {"name":"a","finished":true},{"name":"b"},{"name":"c"}]}"#,
        )
        .expect("fixture parses");
        assert_eq!(unfinished_subtask_indexes(&task), vec![1, 2]);
    }

    #[test]
    fn finish_progress_names_every_write_that_landed() {
        assert_eq!(
            FinishProgress::default().note_before_move(),
            "nothing had been written yet; the task was NOT moved"
        );
        assert_eq!(
            FinishProgress {
                comment_posted: true,
                subtasks_checked: 2,
                subtasks_to_check: 3,
            }
            .note_before_move(),
            "the closing comment WAS posted and 2/3 subtasks WERE checked; the task was NOT moved"
        );
        assert_eq!(
            FinishProgress {
                comment_posted: true,
                ..Default::default()
            }
            .landed(),
            "the closing comment WAS posted"
        );
        assert_eq!(
            FinishProgress {
                comment_posted: false,
                subtasks_checked: 0,
                subtasks_to_check: 2,
            }
            .landed(),
            "0/2 subtasks WERE checked"
        );
    }

    #[test]
    fn grab_only_needs_force_for_someone_elses_task() {
        let mut task: Task =
            serde_json::from_str(r#"{"_id":"T1","name":"n","columnId":"C1"}"#).expect("parses");
        assert!(ensure_can_grab(&task, "UME", false).is_ok());
        task.responsible_user_id = Some("UME".to_string());
        assert!(ensure_can_grab(&task, "UME", false).is_ok());
        task.responsible_user_id = Some("UOTHER".to_string());
        assert!(ensure_can_grab(&task, "UME", false).is_err());
        assert!(ensure_can_grab(&task, "UME", true).is_ok());
    }
}
