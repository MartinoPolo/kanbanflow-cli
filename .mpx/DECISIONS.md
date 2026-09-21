# DECISIONS

## Platform and Infrastructure

### Rust with clap, reqwest, serde, and keyring

The CLI is Rust. `clap` defines commands, `reqwest` handles HTTP, `serde` handles JSON, and `keyring` stores tokens in the operating-system credential store. This keeps startup fast and distribution self-contained.

### Binary named `kf`

The binary uses `gh`-style `kf <noun> <verb>` grammar. The public work-item noun is `issue`, with commands under `kf issue`.

### Vendored API documentation

The upstream API documentation is stored in `docs/api/` and refreshed by `scripts/scrape-docs.mjs`. This keeps wire-contract maintenance reproducible without making routine development depend on the live documentation site.

## CLI Design

### Issues accept board numbers and internal IDs

Commands accept an `E613`-style issue number or an internal ID. Numbers match what people see on the board; IDs remain useful for automation.

### Public issue vocabulary, scoped wire vocabulary

Public prose, modules, and commands use Issue and `kf issue`. KanbanFlow's upstream wire vocabulary remains unchanged only in API models, payloads, serde fields, and vendored API documentation.

### Canonical workflow states live in the root project config

`kf issue move --to todo|wip|review|done|archive` resolves through the `issues.metadata.states` mapping in root `mpxconfig.json`. `todo`, `wip`, `review`, and `done` are required; `archive` is optional. Every mapped state uses a distinct column so reverse lookup is unambiguous.

### Init updates an existing manifest only

`kf init` requires a pre-existing valid root `mpxconfig.json`. It sets `issues.provider` and updates only `issues.metadata` while preserving unrelated root, issue, metadata, and state fields. It never creates a new manifest.

The shared file is updated through a sibling temporary file and rename so a failed write cannot truncate unrelated project configuration.

### Acting identity is user-level only

Which board member `kf` acts as is stored in the user-level board registry, with `KANBANFLOW_USER_ID` as the environment override. The committed `issues` binding contains no identity. There is no fallback to `.mpx/kanbanflow.json` and no fallback to a legacy `userId` field.

### Token storage

Tokens live in the OS credential store, with `KANBANFLOW_TOKEN` as an override. No token is written to project files.

### Shared-board guardrails

Mutations of an issue that is not yours are refused unless the human explicitly supplies `--force`. Responsible users and collaborators count as owners for normal mutations; grabbing remains stricter because it reassigns responsibility. Labels are never created implicitly.

### Agent output contract

Every read command supports `--json`; human-readable tables remain the default. Aggregate issue views can download image attachments for agents to inspect.

### Compound workflow commands

`kf issue grab` combines assignment, movement to `wip`, and an aggregate view. `kf issue finish` can add a closing comment, check subtasks, and move the issue. These deterministic commands replace fragile multi-command agent orchestration.

## Scope and Integration

The implemented surface covers issues, attachments, comments, subtasks, label reads, board metadata, initialization, and authentication. Time tracking, webhooks, cross-board movement, and custom fields remain outside the current scope.

The plugin skills use the same Issue and `kf issue` vocabulary as the CLI. `.mpx/` contains project context, decisions, and ignored scratch downloads; it does not contain board configuration.
