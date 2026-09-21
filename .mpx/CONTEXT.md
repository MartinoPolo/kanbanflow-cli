# CONTEXT

## What This Is

`kf` is a Rust CLI for KanbanFlow, modeled on `gh` and `glab`, for human and agent issue workflows. It creates, reads, edits, moves, and round-trips image attachments without leaving the terminal.

## Public Domain Language

**Issue** — A board work item. Users and agents reference it by a human-facing number such as `E613` or by its internal ID.

**Column** — A board lane holding workflow state.

**Canonical state** — The CLI abstraction `backlog | todo | wip | review | done | archive`, mapped to real column IDs per project.

**Label** — An existing area or project tag. The CLI never creates labels implicitly.

**Color** — The board color assigned to an issue.

**Subtask** — A checklist item with a name and completion state.

**Responsible user** — The single assignee.

**Collaborator** — An additional person whose participation also makes the issue count as theirs for normal guardrails.

KanbanFlow's upstream wire vocabulary remains confined to API models, payloads, and vendored API documentation. Public prose and commands say “issue.”

## Current Command Surface

- `kf issue create|view|list|edit|move|delete|grab|finish`
- `kf attach add|list|download|delete`
- `kf comment add|list|edit|delete`
- `kf subtask add|list|check|uncheck`
- `kf label list`, `kf board`
- `kf auth login|logout|status`
- `kf init`

## Project Configuration and Identity

The committed project config is the root `mpxconfig.json`. Its KanbanFlow integration is the `issues` binding containing the provider, board facts, and canonical-state column mapping.

`kf init` requires a pre-existing valid MPX project manifest. It replaces only the `issues` binding and preserves unrelated and unknown root configuration. It does not create a manifest.

Tokens stay in the OS credential store or `KANBANFLOW_TOKEN`. Acting identity stays in the user-level board registry or `KANBANFLOW_USER_ID`. No committed legacy config or `userId` fallback is read.

## Key Constraints

- API tokens are per board and premium-only.
- Rate limit: 1000 requests/hour/board; more than 5000 requests/day locks the token.
- Shared board: never mutate an issue belonging to somebody else unless the human explicitly authorizes `--force`.
- Attachment links expire, so download them immediately and never store their links.
- The upstream issue-detail endpoint includes issue fields but not comments or attachments, so an aggregate issue view requires separate API calls.
- Every read command supports `--json`; write commands report the affected issue.
- Vendored upstream API documentation lives in `docs/api/`.
