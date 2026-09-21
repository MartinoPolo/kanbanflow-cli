//! `kf issue` — create, read and change issues.
//!
//! The compound verbs (`view`, `grab`, `finish`, `create --attach`) exist because
//! the API forces multi-call dances: a full issue read is issue + comments +
//! attachments, and attachment links expire ~24h after they are handed out.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use clap::{ArgGroup, Args, Subcommand, ValueEnum};
use serde::Serialize;

use crate::api::models::{
    Attachment, Board, Column, Comment, CreateComment, CreateCommentResponse, CreateIssue,
    CreateIssueResponse, Issue, IssueGroup, Label, SubTask, SubTaskPayload, UpdateIssue,
};
use crate::api::{ApiError, Client};
use crate::config::{CanonicalState, Config, ConfigError};
use crate::context::Context;
use crate::files::unique_destination;
use crate::guard;
use crate::issues;
use crate::labels;
use crate::output::{self, Table};
use crate::prompt::confirm;
use crate::resolve;
use crate::users::UserNames;

#[derive(Debug, Subcommand)]
pub enum IssueCommand {
    /// Create an issue, optionally uploading attachments in the same step.
    Create(CreateArgs),
    /// Show an issue with its comments and attachments.
    View(ViewArgs),
    /// List issues on the board.
    List(ListArgs),
    /// Change an issue's fields.
    Edit(EditArgs),
    /// Move an issue to a canonical state or board column.
    Move(MoveArgs),
    /// Delete an issue.
    Delete(DeleteArgs),
    /// Assign the issue to yourself, move it to WIP and show it.
    Grab(GrabArgs),
    /// Comment, move out of WIP and optionally check off subtasks.
    Finish(FinishArgs),
}

/// The issue colors the API accepts (`docs/api/create-task.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum IssueColor {
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

impl IssueColor {
    fn as_str(self) -> &'static str {
        match self {
            IssueColor::Yellow => "yellow",
            IssueColor::White => "white",
            IssueColor::Red => "red",
            IssueColor::Green => "green",
            IssueColor::Blue => "blue",
            IssueColor::Purple => "purple",
            IssueColor::Orange => "orange",
            IssueColor::Cyan => "cyan",
            IssueColor::Brown => "brown",
            IssueColor::Magenta => "magenta",
        }
    }
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Issue name.
    #[arg(long)]
    pub name: String,
    /// Issue description (Markdown, as the board renders it).
    #[arg(long)]
    pub description: Option<String>,
    /// Issue color.
    #[arg(long, value_enum, ignore_case = true)]
    pub color: Option<IssueColor>,
    /// Canonical state (column) to create the issue in.
    #[arg(long, value_enum, ignore_case = true, default_value_t = CanonicalState::Todo)]
    pub to: CanonicalState,
    /// Existing board label to apply; repeatable. Unknown labels are refused.
    #[arg(long = "label", value_name = "NAME")]
    pub labels: Vec<String>,
    /// Responsible user: `me` or a user ID; defaults to you, pass `none` to leave unassigned.
    #[arg(long)]
    pub responsible: Option<String>,
    /// File to upload onto the new issue; repeatable.
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
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Save every attachment into this directory (links expire, so do it now).
    #[arg(long, value_name = "DIR")]
    pub download_attachments: Option<PathBuf>,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Only issues in this canonical state; repeatable (`--state todo --state wip`).
    #[arg(long, value_enum, ignore_case = true)]
    pub state: Vec<CanonicalState>,
    /// Only issues in this column, by name or ID (for columns with no canonical state).
    #[arg(long, value_name = "NAME_OR_ID", conflicts_with = "state")]
    pub column: Option<String>,
    /// Only open work: every column except the ones mapped to `done` and `archive`.
    #[arg(long, conflicts_with_all = ["state", "column"])]
    pub open: bool,
    /// Only issues you are responsible for or a collaborator on.
    #[arg(long)]
    pub mine: bool,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Replace the issue name.
    #[arg(long)]
    pub name: Option<String>,
    /// Replace the description.
    #[arg(long, conflicts_with = "append_description")]
    pub description: Option<String>,
    /// Append to the description, separated by a blank line (lossless merge).
    #[arg(long, value_name = "TEXT")]
    pub append_description: Option<String>,
    /// Issue color.
    #[arg(long, value_enum, ignore_case = true)]
    pub color: Option<IssueColor>,
    /// Add an existing board label; repeatable.
    #[arg(long = "add-label", value_name = "NAME")]
    pub add_labels: Vec<String>,
    /// Remove a label; repeatable.
    #[arg(long = "remove-label", value_name = "NAME")]
    pub remove_labels: Vec<String>,
    /// Responsible user: `me`, a user ID, or `none` to clear.
    #[arg(long)]
    pub responsible: Option<String>,
    /// Mutate an issue that is not yours.
    #[arg(long)]
    pub force: bool,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("target")
        .required(true)
        .multiple(false)
        .args(["to", "column"])
))]
pub struct MoveArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Target canonical state. Required unless `--column` is used.
    #[arg(long, value_enum, ignore_case = true)]
    pub to: Option<CanonicalState>,
    /// Target board column by exact ID or unique name. Name matching ignores
    /// ASCII case only; non-ASCII characters must match exactly.
    #[arg(long, value_name = "NAME_OR_ID")]
    pub column: Option<String>,
    /// Grouping date (`YYYY-MM-DD`) when the target column is date grouped.
    /// Omitted, the server files the issue under today's UTC date.
    #[arg(long, value_name = "YYYY-MM-DD")]
    pub grouping_date: Option<String>,
    /// Mutate an issue that is not yours.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,
    /// Delete an issue that is not yours.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct GrabArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Save the issue's image attachments into this directory.
    #[arg(long, value_name = "DIR")]
    pub download_dir: Option<PathBuf>,
    /// Grab an issue someone else is responsible for.
    #[arg(long)]
    pub force: bool,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct FinishArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// File whose contents become the closing comment.
    #[arg(long, value_name = "PATH")]
    pub comment_file: Option<PathBuf>,
    /// Target canonical state.
    #[arg(long, value_enum, ignore_case = true, default_value_t = CanonicalState::Done)]
    pub to: CanonicalState,
    /// Mark every unfinished subtask finished.
    #[arg(long)]
    pub check_subtasks: bool,
    /// Finish an issue that is not yours.
    #[arg(long)]
    pub force: bool,
}

pub fn run(command: IssueCommand) -> anyhow::Result<()> {
    let context = Context::load()?;
    match command {
        IssueCommand::Create(args) => create(&context, args),
        IssueCommand::View(args) => view(&context, args),
        IssueCommand::List(args) => list(&context, args),
        IssueCommand::Edit(args) => edit(&context, args),
        IssueCommand::Move(args) => move_issue(&context, args),
        IssueCommand::Delete(args) => delete(&context, args),
        IssueCommand::Grab(args) => grab(&context, args),
        IssueCommand::Finish(args) => finish(&context, args),
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
    #[serde(rename = "issueId")]
    issue_id: String,
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

    let payload = CreateIssue {
        name: args.name,
        column_id,
        description: args.description,
        color: args.color.map(|color| color.as_str().to_string()),
        responsible_user_id: create_responsible_user_id(
            args.responsible.as_deref(),
            context.my_user_id()?,
        )?,
        grouping_date: args.grouping_date,
        labels,
        ..Default::default()
    };
    let created: CreateIssueResponse = context
        .client
        .post_json("tasks", &payload)
        .context("creating the issue")?;

    let number = created.number.as_ref().map(ToString::to_string);
    // The issue exists from here on: an upload failure is reported, never fatal.
    let uploads = upload_attachments(&context.client, &created.task_id, &args.attachments);

    if args.json {
        output::print_json(&CreateOutcome {
            issue_id: created.task_id,
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

fn upload_attachments(client: &Client, issue_id: &str, files: &[PathBuf]) -> Vec<UploadOutcome> {
    files
        .iter()
        .map(|file| {
            let path = format!("tasks/{issue_id}/attachments");
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

/// The three calls a full issue read costs, as one `--json` object.
#[derive(Debug, Serialize)]
struct IssueAggregate {
    issue: Issue,
    comments: Vec<Comment>,
    attachments: Vec<Attachment>,
}

fn fetch_aggregate(client: &Client, issue: Issue) -> Result<IssueAggregate, ApiError> {
    let comments = client.get_json(&format!("tasks/{}/comments", issue.id), &[])?;
    let attachments = client.get_json(&format!("tasks/{}/attachments", issue.id), &[])?;
    Ok(IssueAggregate {
        issue,
        comments,
        attachments,
    })
}

fn view(context: &Context, args: ViewArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    let aggregate = fetch_aggregate(&context.client, issue)
        .with_context(|| format!("reading issue {}", args.issue))?;

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
    let mut users = UserNames::new(&context.client, context.my_user_id_optional());
    print_aggregate(&aggregate, &context.config, &mut users);
    Ok(())
}

fn print_aggregate(aggregate: &IssueAggregate, config: &Config, users: &mut UserNames<'_>) {
    let issue = &aggregate.issue;
    println!("{}  {}", issue.reference(), issue.name);
    println!(
        "State:         {}",
        describe_column(config, &issue.column_id)
    );
    if let Some(color) = &issue.color {
        println!("Color:         {color}");
    }
    println!(
        "Responsible:   {}",
        match issue.responsible_user_id.as_deref() {
            Some(user_id) => users.display(user_id),
            None => "unassigned".to_string(),
        }
    );
    if !issue.collaborators.is_empty() {
        println!(
            "Collaborators: {}",
            issue
                .collaborators
                .iter()
                .map(|collaborator| users.display(&collaborator.user_id))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if let Some(date) = &issue.grouping_date {
        println!("Grouped:       {date}");
    }
    if !issue.labels.is_empty() {
        println!("Labels:        {}", join_labels(&issue.labels));
    }

    if let Some(description) = issue.description.as_deref().map(str::trim) {
        if !description.is_empty() {
            println!("\nDescription\n{description}");
        }
    }

    if !issue.sub_tasks.is_empty() {
        println!("\nSubtasks");
        for (index, sub_issue) in issue.sub_tasks.iter().enumerate() {
            println!("{}", format_subtask_line(index, sub_issue));
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

/// Render one checklist line for `issue view`. `index` is the 0-based position in
/// `sub_tasks`; the printed number is 1-based so it matches `kf subtask list` and the
/// position accepted by `kf subtask check`.
fn format_subtask_line(index: usize, sub_issue: &SubTask) -> String {
    let mark = if sub_issue.finished { 'x' } else { ' ' };
    format!("  {}. [{mark}] {}", index + 1, sub_issue.name)
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

/// The `PEOPLE` cell: the responsible user, then collaborators marked with `+`.
fn describe_people(issue: &Issue, users: &mut UserNames<'_>) -> String {
    let responsible = issue
        .responsible_user_id
        .as_deref()
        .map(|user_id| users.display(user_id));
    let collaborators = issue
        .collaborators
        .iter()
        .map(|collaborator| format!("+{}", users.display(&collaborator.user_id)));
    responsible
        .into_iter()
        .chain(collaborators)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Which columns a `issue list` run keeps. The variants are mutually exclusive on
/// the command line, so exactly one of `--state`, `--column` and `--open` decides.
#[derive(Debug, PartialEq, Eq)]
enum ColumnFilter<'a> {
    /// No filter: the whole board.
    Every,
    /// `--state`, one entry per state — several states select the union.
    OnlyThese(Vec<&'a str>),
    /// `--open`: everything but the closed columns.
    AllButThese(Vec<&'a str>),
    /// `--column`, matched against a column's ID or its name.
    NamedOrIdentified(&'a str),
}

impl ColumnFilter<'_> {
    fn keeps(&self, group: &IssueGroup) -> bool {
        match self {
            ColumnFilter::Every => true,
            ColumnFilter::OnlyThese(columns) => columns.contains(&group.column_id.as_str()),
            ColumnFilter::AllButThese(columns) => !columns.contains(&group.column_id.as_str()),
            ColumnFilter::NamedOrIdentified(needle) => {
                group.column_id == *needle || group.column_name.eq_ignore_ascii_case(needle)
            }
        }
    }
}

/// An unmapped state is an error rather than an empty slice: silently dropping it
/// would under-report the board without saying so.
fn column_filter<'a>(
    args: &'a ListArgs,
    config: &'a Config,
) -> Result<ColumnFilter<'a>, ConfigError> {
    if args.open {
        return Ok(ColumnFilter::AllButThese(config.closed_column_ids()?));
    }
    if !args.state.is_empty() {
        let columns = args
            .state
            .iter()
            .map(|state| config.column_id(*state))
            .collect::<Result<_, _>>()?;
        return Ok(ColumnFilter::OnlyThese(columns));
    }
    Ok(match args.column.as_deref() {
        Some(needle) => ColumnFilter::NamedOrIdentified(needle),
        None => ColumnFilter::Every,
    })
}

fn select_groups<'a>(groups: &'a [IssueGroup], filter: &ColumnFilter<'_>) -> Vec<&'a IssueGroup> {
    groups.iter().filter(|group| filter.keeps(group)).collect()
}

fn list(context: &Context, args: ListArgs) -> anyhow::Result<()> {
    // Paged, not raw: a silently truncated Done column would make `list` lie
    // about what is on the board.
    let groups: Vec<IssueGroup> =
        issues::fetch_all_groups(&context.client).context("listing the board's issues")?;

    let filter = column_filter(&args, &context.config)?;
    let selected = select_groups(&groups, &filter);

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

    // `--mine` is the only filter that has to know who we are, so the identity
    // is demanded here rather than for every listing.
    let me = args.mine.then(|| context.my_user_id()).transpose()?;
    let issues: Vec<&Issue> = selected
        .iter()
        .flat_map(|group| group.tasks.iter())
        .filter(|issue| me.is_none_or(|me| guard::is_mine(issue, me)))
        .collect();

    if args.json {
        return Ok(output::print_json(&issues)?);
    }

    let mut users = UserNames::new(&context.client, context.my_user_id_optional());
    let mut table = Table::new(&["NUMBER", "STATE", "NAME", "PEOPLE", "LABELS"]);
    // A column with no canonical state still has a name in the listing response,
    // which beats printing its raw ID.
    let column_names: HashMap<&str, &str> = groups
        .iter()
        .map(|group| (group.column_id.as_str(), group.column_name.as_str()))
        .collect();
    for issue in &issues {
        table.row([
            issue.reference(),
            match context.config.state_for_column(&issue.column_id) {
                Some(state) => state.to_string(),
                None => column_names
                    .get(issue.column_id.as_str())
                    .map(|name| (*name).to_string())
                    .unwrap_or_else(|| issue.column_id.clone()),
            },
            output::truncate_cell(&issue.name, 60),
            describe_people(issue, &mut users),
            join_labels(&issue.labels),
        ]);
    }
    table.print_or("No issues match.");
    Ok(())
}

// ---------------------------------------------------------------------------
// edit
// ---------------------------------------------------------------------------

fn edit(context: &Context, args: EditArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("editing issue {}", issue.reference()))?;

    let mut update = UpdateIssue {
        name: args.name,
        color: args.color.map(|color| color.as_str().to_string()),
        ..Default::default()
    };
    update.description = match &args.append_description {
        Some(addition) => Some(append_description(issue.description.as_deref(), addition)),
        None => args.description,
    };
    if let Some(responsible) = &args.responsible {
        update.responsible_user_id = Some(
            responsible_user_id(responsible, context.my_user_id()?)?
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
        update.labels = changed_labels(&issue.labels, &added, &args.remove_labels);
    }

    let changes = serde_json::to_value(&update).context("building the update payload")?;
    if changes.as_object().map(serde_json::Map::len) == Some(0) {
        anyhow::bail!("Nothing to change: pass at least one of --name, --description, --append-description, --color, --add-label, --remove-label, --responsible.");
    }

    context
        .client
        .post_json_discard(&format!("tasks/{}", issue.id), &update)
        .with_context(|| format!("updating issue {}", issue.reference()))?;

    if args.json {
        // The write already landed; only the read-back can still fail, and the
        // caller must not mistake that for a rejected edit.
        let updated = resolve::resolve_issue(&context.client, &issue.id).with_context(|| {
            format!(
                "the edit to issue {} SUCCEEDED, but reading the issue back for --json failed",
                issue.reference()
            )
        })?;
        return Ok(output::print_json(&updated)?);
    }
    output::print_affected_issue(&issue);
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

fn move_issue(context: &Context, args: MoveArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    let target_description = match (args.to, args.column.as_deref()) {
        (Some(state), None) => state.to_string(),
        (None, Some(column)) => format!("column `{column}`"),
        _ => anyhow::bail!("exactly one of `--to` or `--column` is required"),
    };
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("moving issue {} to {target_description}", issue.reference()))?;

    let column_id = if let Some(state) = args.to {
        context.config.column_id(state)?.to_string()
    } else {
        let column = args
            .column
            .as_deref()
            .expect("target choice was validated above");
        let board: Board = context
            .client
            .get_json("board", &[])
            .context("reading board columns for `--column`")?;
        resolve_move_column(&board.columns, column)?.to_string()
    };
    let update = build_move_update(column_id, args.grouping_date)?;
    context
        .client
        .post_json_discard(&format!("tasks/{}", issue.id), &update)
        .with_context(|| format!("moving issue {} to {target_description}", issue.reference()))?;
    output::print_affected_issue(&issue);
    Ok(())
}

fn resolve_move_column<'a>(columns: &'a [Column], needle: &str) -> anyhow::Result<&'a str> {
    let needle = needle.trim();
    if let Some(column) = columns.iter().find(|column| column.unique_id == needle) {
        return Ok(&column.unique_id);
    }

    let matches: Vec<_> = columns
        .iter()
        .filter(|column| column.name.eq_ignore_ascii_case(needle))
        .collect();
    match matches.as_slice() {
        [column] => Ok(&column.unique_id),
        [] => anyhow::bail!(
            "no column ID or name matches `{needle}`. Known columns: {}",
            describe_move_columns(columns)
        ),
        matches => anyhow::bail!(
            "column name `{needle}` is ambiguous; use one of these exact IDs: {}",
            matches
                .iter()
                .map(|column| column.unique_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn describe_move_columns(columns: &[Column]) -> String {
    columns
        .iter()
        .map(|column| format!("{} ({})", column.name, column.unique_id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_move_update(
    column_id: String,
    grouping_date: Option<String>,
) -> anyhow::Result<UpdateIssue> {
    if let Some(date) = &grouping_date {
        ensure_iso_date(date)?;
    }
    // groupingDate is omitted unless asked for: on a date-grouped column the
    // server then files the issue under today's UTC date.
    Ok(UpdateIssue {
        column_id: Some(column_id),
        grouping_date,
        ..Default::default()
    })
}

fn delete(context: &Context, args: DeleteArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("deleting issue {}", issue.reference()))?;
    if !args.yes
        && !confirm(&format!(
            "Delete issue {} ({})?",
            issue.reference(),
            issue.name
        ))?
    {
        println!("Cancelled.");
        return Ok(());
    }
    context
        .client
        .delete(&format!("tasks/{}", issue.id))
        .with_context(|| format!("deleting issue {}", issue.reference()))?;
    output::print_affected_issue(&issue);
    Ok(())
}

// ---------------------------------------------------------------------------
// grab / finish
// ---------------------------------------------------------------------------

fn grab(context: &Context, args: GrabArgs) -> anyhow::Result<()> {
    let mut issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_take_over(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("grabbing issue {}", issue.reference()))?;
    let column_id = context.config.column_id(CanonicalState::Wip)?.to_string();

    let update = UpdateIssue {
        column_id: Some(column_id.clone()),
        responsible_user_id: Some(serde_json::Value::String(context.my_user_id()?.to_string())),
        ..Default::default()
    };
    context
        .client
        .post_json_discard(&format!("tasks/{}", issue.id), &update)
        .with_context(|| format!("grabbing issue {}", issue.reference()))?;
    // Mirror the accepted update locally instead of spending a re-read.
    issue.column_id = column_id;
    issue.responsible_user_id = Some(context.my_user_id()?.to_string());

    // Everything below is read-only follow-up. The single POST above already
    // landed both the assignment and the move, so failures here say so.
    let grabbed = format!(
        "issue {} WAS assigned to you and moved to wip",
        issue.reference()
    );
    let aggregate = fetch_aggregate(&context.client, issue).with_context(|| {
        format!(
            "reading issue {} back after the grab ({grabbed})",
            args.issue
        )
    })?;
    if let Some(directory) = &args.download_dir {
        let saved = download_attachments(&context.client, &aggregate.attachments, directory, true)
            .with_context(|| format!("downloading the issue's images ({grabbed})"))?;
        if !args.json {
            for path in &saved {
                println!("saved {}", path.display());
            }
        }
    }

    if args.json {
        return Ok(output::print_json(&aggregate)?);
    }
    let mut users = UserNames::new(&context.client, context.my_user_id_optional());
    print_aggregate(&aggregate, &context.config, &mut users);
    Ok(())
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
        format!("{}; the issue was NOT moved", self.landed())
    }
}

fn finish(context: &Context, args: FinishArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("finishing issue {}", issue.reference()))?;
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
            .post_json(&format!("tasks/{}/comments", issue.id), &comment)
            .with_context(|| {
                format!(
                    "commenting on issue {} ({})",
                    issue.reference(),
                    progress.note_before_move()
                )
            })?;
        progress.comment_posted = true;
    }

    if args.check_subtasks {
        let unfinished = unfinished_subtask_indexes(&issue);
        progress.subtasks_to_check = unfinished.len();
        for index in unfinished {
            let payload = SubTaskPayload {
                finished: Some(true),
                ..Default::default()
            };
            context
                .client
                .post_json_discard(
                    &format!("tasks/{}/subtasks/by-index/{index}", issue.id),
                    &payload,
                )
                .with_context(|| {
                    format!(
                        "checking subtask {index} of issue {} ({})",
                        issue.reference(),
                        progress.note_before_move()
                    )
                })?;
            progress.subtasks_checked += 1;
        }
    }

    let update = UpdateIssue {
        column_id: Some(column_id),
        ..Default::default()
    };
    context
        .client
        .post_json_discard(&format!("tasks/{}", issue.id), &update)
        .with_context(|| {
            format!(
                "moving issue {} to {} ({})",
                issue.reference(),
                args.to,
                progress.note_before_move()
            )
        })?;
    output::print_affected_issue(&issue);
    Ok(())
}

fn unfinished_subtask_indexes(issue: &Issue) -> Vec<usize> {
    issue
        .sub_tasks
        .iter()
        .enumerate()
        .filter(|(_, sub_issue)| !sub_issue.finished)
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

/// Created issues are yours by default, so the ownership guard does not block follow-ups.
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
    use std::io::{Read as _, Write as _};
    use std::net::{TcpListener, TcpStream};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::config::StateColumns;

    fn read_http_request(stream: &mut TcpStream) -> std::io::Result<String> {
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let bytes_read = stream.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..bytes_read]);

            let Some(header_end) = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|position| position + 4)
            else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= header_end + content_length {
                break;
            }
        }
        String::from_utf8(request)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    fn start_json_server(
        response_bodies: Vec<&'static str>,
    ) -> (String, JoinHandle<std::io::Result<Vec<String>>>) {
        start_json_server_with_accept_timeout(response_bodies, Duration::from_secs(5))
    }

    fn start_json_server_with_accept_timeout(
        response_bodies: Vec<&'static str>,
        expected_request_timeout: Duration,
    ) -> (String, JoinHandle<std::io::Result<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = std::thread::spawn(move || -> std::io::Result<Vec<String>> {
            listener.set_nonblocking(true)?;
            let expected_request_count = response_bodies.len();
            let mut requests = Vec::new();
            for (request_index, body) in response_bodies.into_iter().enumerate() {
                let deadline = Instant::now() + expected_request_timeout;
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if Instant::now() >= deadline {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    format!(
                                        "missing expected HTTP request {} of {}",
                                        request_index + 1,
                                        expected_request_count
                                    ),
                                ));
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => return Err(error),
                    }
                };
                requests.push(read_http_request(&mut stream)?);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )?;
                stream.flush()?;
            }

            let deadline = Instant::now() + Duration::from_millis(200);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        requests.push(read_http_request(&mut stream)?);
                        write!(
                            stream,
                            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        )?;
                        stream.flush()?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error),
                }
            }
            Ok(requests)
        });
        (format!("http://{address}"), server)
    }

    fn request_line(request: &str) -> &str {
        request.lines().next().expect("request line")
    }

    fn request_body(request: &str) -> serde_json::Value {
        serde_json::from_str(
            request
                .split_once("\r\n\r\n")
                .expect("request has headers")
                .1,
        )
        .expect("request body is JSON")
    }

    fn labels(names: &[&str]) -> Vec<Label> {
        names
            .iter()
            .map(|name| Label {
                name: name.to_string(),
                pinned: None,
            })
            .collect()
    }

    /// A board with a lane that maps to no canonical state, because that is the
    /// case `--open` has to decide about.
    fn board() -> Vec<IssueGroup> {
        serde_json::from_str(
            r#"[{"columnId":"CBACKLOG","columnName":"Backlog","tasks":[]},
                {"columnId":"CTODO","columnName":"To-do","tasks":[]},
                {"columnId":"CWIP","columnName":"In progress","tasks":[]},
                {"columnId":"CTODAY","columnName":"Do today","tasks":[]},
                {"columnId":"CDONE","columnName":"Done","tasks":[]}]"#,
        )
        .expect("fixture parses")
    }

    fn board_config() -> Config {
        Config {
            board_id: "F2QMK1B".to_string(),
            board_name: Some("My first board".to_string()),
            states: StateColumns {
                backlog: Some("CBACKLOG".to_string()),
                todo: Some("CTODO".to_string()),
                wip: Some("CWIP".to_string()),
                review: None,
                done: Some("CDONE".to_string()),
                archive: None,
            },
        }
    }

    fn list_args(states: &[CanonicalState], column: Option<&str>, open: bool) -> ListArgs {
        ListArgs {
            state: states.to_vec(),
            column: column.map(str::to_string),
            open,
            mine: false,
            json: false,
        }
    }

    fn kept_columns(groups: &[IssueGroup], args: &ListArgs) -> Result<Vec<String>, ConfigError> {
        let config = board_config();
        let filter = column_filter(args, &config)?;
        Ok(select_groups(groups, &filter)
            .iter()
            .map(|group| group.column_name.clone())
            .collect())
    }

    #[test]
    fn several_states_select_the_union_of_their_columns() {
        let args = list_args(
            &[CanonicalState::Backlog, CanonicalState::Todo],
            None,
            false,
        );
        assert_eq!(
            kept_columns(&board(), &args).expect("both states are mapped"),
            ["Backlog", "To-do"]
        );
    }

    #[test]
    fn a_state_with_no_column_is_an_error_not_an_empty_result() {
        let args = list_args(&[CanonicalState::Review], None, false);
        assert!(matches!(
            kept_columns(&board(), &args),
            Err(ConfigError::UnmappedState {
                state: CanonicalState::Review
            })
        ));
    }

    /// `--open` filters by exclusion, so Backlog and a lane the board invented —
    /// "Do today" — stay in the answer instead of being dropped.
    #[test]
    fn open_drops_only_the_closed_columns() {
        let args = list_args(&[], None, true);
        assert_eq!(
            kept_columns(&board(), &args).expect("done is mapped"),
            ["Backlog", "To-do", "In progress", "Do today"]
        );
    }

    #[test]
    fn open_needs_somewhere_for_work_to_end() {
        let groups = board();
        let mut config = board_config();
        config.states.set(CanonicalState::Done, None);
        let args = list_args(&[], None, true);
        assert!(matches!(
            column_filter(&args, &config),
            Err(ConfigError::NoClosedState)
        ));
        assert_eq!(
            select_groups(&groups, &ColumnFilter::AllButThese(vec!["CDONE"])).len(),
            groups.len() - 1
        );
    }

    #[test]
    fn no_filter_keeps_the_whole_board_and_a_column_name_matches_case_insensitively() {
        let groups = board();
        assert_eq!(
            kept_columns(&groups, &list_args(&[], None, false)).expect("no filter"),
            ["Backlog", "To-do", "In progress", "Do today", "Done"]
        );
        assert_eq!(
            kept_columns(&groups, &list_args(&[], Some("in PROGRESS"), false)).expect("by name"),
            ["In progress"]
        );
        assert!(
            kept_columns(&groups, &list_args(&[], Some("Future"), false))
                .expect("unknown name")
                .is_empty()
        );
    }

    #[test]
    fn json_server_times_out_when_an_expected_request_is_missing() {
        let (_base_url, server) =
            start_json_server_with_accept_timeout(vec!["{}"], Duration::from_millis(30));

        let error = server
            .join()
            .expect("test server thread did not panic")
            .expect_err("missing request must time out");

        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(
            error
                .to_string()
                .contains("missing expected HTTP request 1 of 1"),
            "{error}"
        );
    }

    #[test]
    fn move_column_lookup_prefers_exact_id_over_names() {
        let columns = vec![
            crate::api::models::Column {
                name: "Target".to_string(),
                unique_id: "C-TARGET".to_string(),
                description: None,
            },
            crate::api::models::Column {
                name: "C-TARGET".to_string(),
                unique_id: "C-OTHER".to_string(),
                description: None,
            },
        ];

        assert_eq!(
            resolve_move_column(&columns, "C-TARGET").unwrap(),
            "C-TARGET"
        );
    }

    #[test]
    fn move_column_lookup_accepts_unique_ascii_case_insensitive_name() {
        let columns = vec![
            crate::api::models::Column {
                name: "Do today".to_string(),
                unique_id: "C-TODAY".to_string(),
                description: None,
            },
            crate::api::models::Column {
                name: "À faire".to_string(),
                unique_id: "C-NON-ASCII".to_string(),
                description: None,
            },
        ];

        assert_eq!(
            resolve_move_column(&columns, "do TODAY").unwrap(),
            "C-TODAY"
        );
        assert_eq!(
            resolve_move_column(&columns, "À faire").unwrap(),
            "C-NON-ASCII"
        );
        assert!(resolve_move_column(&columns, "à faire").is_err());
    }

    #[test]
    fn move_column_lookup_rejects_unknown_ambiguous_and_numeric_index() {
        let columns = vec![
            crate::api::models::Column {
                name: "Waiting".to_string(),
                unique_id: "C-WAIT-ONE".to_string(),
                description: None,
            },
            crate::api::models::Column {
                name: "waiting".to_string(),
                unique_id: "C-WAIT-TWO".to_string(),
                description: None,
            },
        ];

        let ambiguous = resolve_move_column(&columns, "WAITING")
            .expect_err("duplicate names must require an ID")
            .to_string();
        assert!(ambiguous.contains("ambiguous"), "{ambiguous}");
        assert!(ambiguous.contains("C-WAIT-ONE"), "{ambiguous}");
        assert!(ambiguous.contains("C-WAIT-TWO"), "{ambiguous}");

        let unknown = resolve_move_column(&columns, "Missing")
            .expect_err("unknown names must fail")
            .to_string();
        assert!(unknown.contains("Known columns"), "{unknown}");

        assert!(resolve_move_column(&columns, "1").is_err());
    }

    #[test]
    fn move_update_contains_column_and_optional_grouping_date() {
        let update = build_move_update("C-DONE".to_string(), Some("2026-07-31".to_string()))
            .expect("valid grouping date");
        assert_eq!(
            serde_json::to_value(update).unwrap(),
            serde_json::json!({"columnId": "C-DONE", "groupingDate": "2026-07-31"})
        );
        assert_eq!(
            serde_json::to_value(build_move_update("C-TODO".to_string(), None).unwrap()).unwrap(),
            serde_json::json!({"columnId": "C-TODO"})
        );
        assert!(build_move_update("C-DONE".to_string(), Some("31/07/2026".to_string())).is_err());
    }

    #[test]
    fn move_to_named_column_resolves_issue_checks_guard_reads_board_then_posts() {
        let (base_url, server) = start_json_server(vec![
            r#"{"_id":"T3s6UGyzY","name":"Write report","columnId":"CTODO","responsibleUserId":"UME"}"#,
            r#"{"_id":"BOARD","name":"Work","columns":[{"name":"Do today","uniqueId":"CTODAY"}]}"#,
            "",
        ]);
        let context = Context::for_test(
            board_config(),
            Client::with_base_url("test-token".to_string(), base_url).expect("build test client"),
            Some("UME".to_string()),
        );

        move_issue(
            &context,
            MoveArgs {
                issue: "T3s6UGyzY".to_string(),
                to: None,
                column: Some("do TODAY".to_string()),
                grouping_date: Some("2026-07-31".to_string()),
                force: false,
            },
        )
        .expect("move succeeds");
        let requests = server
            .join()
            .expect("test server thread did not panic")
            .expect("test server handled requests");

        assert_eq!(
            requests
                .iter()
                .map(|request| request_line(request))
                .collect::<Vec<_>>(),
            [
                "GET /tasks/T3s6UGyzY HTTP/1.1",
                "GET /board HTTP/1.1",
                "POST /tasks/T3s6UGyzY HTTP/1.1"
            ]
        );
        assert_eq!(
            request_body(&requests[2]),
            serde_json::json!({"columnId": "CTODAY", "groupingDate": "2026-07-31"})
        );
    }

    #[test]
    fn move_to_named_column_stops_after_guard_rejection() {
        let (base_url, server) = start_json_server(vec![
            r#"{"_id":"T3s6UGyzY","name":"Write report","columnId":"CTODO","responsibleUserId":"UOTHER"}"#,
        ]);
        let context = Context::for_test(
            board_config(),
            Client::with_base_url("test-token".to_string(), base_url).expect("build test client"),
            Some("UME".to_string()),
        );

        let error = move_issue(
            &context,
            MoveArgs {
                issue: "T3s6UGyzY".to_string(),
                to: None,
                column: Some("Do today".to_string()),
                grouping_date: None,
                force: false,
            },
        )
        .expect_err("a teammate's issue is guarded");
        let requests = server
            .join()
            .expect("test server thread did not panic")
            .expect("test server handled requests");

        assert!(matches!(
            error.downcast_ref::<crate::guard::GuardError>(),
            Some(crate::guard::GuardError::NotMine { .. })
        ));
        assert_eq!(
            requests
                .iter()
                .map(|request| request_line(request))
                .collect::<Vec<_>>(),
            ["GET /tasks/T3s6UGyzY HTTP/1.1"]
        );
    }

    #[test]
    fn format_subtask_line_numbers_from_one() {
        let sub_issue = SubTask {
            name: "Reproduce on staging".to_string(),
            finished: false,
            user_id: None,
            due_date_timestamp: None,
            due_date_timestamp_local: None,
        };
        assert_eq!(
            format_subtask_line(0, &sub_issue),
            "  1. [ ] Reproduce on staging"
        );
        let finished = SubTask {
            finished: true,
            ..sub_issue
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
        let issue: Issue = serde_json::from_str(
            r#"{"_id":"T1","name":"n","columnId":"C1","subTasks":[
                {"name":"a","finished":true},{"name":"b"},{"name":"c"}]}"#,
        )
        .expect("fixture parses");
        assert_eq!(unfinished_subtask_indexes(&issue), vec![1, 2]);
    }

    #[test]
    fn finish_progress_names_every_write_that_landed() {
        assert_eq!(
            FinishProgress::default().note_before_move(),
            "nothing had been written yet; the issue was NOT moved"
        );
        assert_eq!(
            FinishProgress {
                comment_posted: true,
                subtasks_checked: 2,
                subtasks_to_check: 3,
            }
            .note_before_move(),
            "the closing comment WAS posted and 2/3 subtasks WERE checked; the issue was NOT moved"
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
}
