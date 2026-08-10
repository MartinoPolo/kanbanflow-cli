# CONTEXT

## What This Is

`kf` is a Rust CLI for KanbanFlow, modeled on `gh`/`glab`, built so AI agents can manage work tickets — create, read, edit, move, and (the differentiator) round-trip **image attachments**, which GitHub/GitLab CLIs cannot do. It exists because the author's work team mandates KanbanFlow (shared board "Team E") and agent workflows need first-class ticket access. Companion `kf-` skills in this repo get symlinked into work projects, replacing the GitHub-issue skill family there.

## Domain Language

**Board** — One KanbanFlow board; the API token's entire universe (tokens are per-board).
**Task** — A card on the board. KanbanFlow's equivalent of an issue/ticket.
**Task number** — Human-facing reference like `E613` (`{ prefix, value }` in API). Primary way users and agents reference tasks; distinct from the internal task ID (`T3s6UGyzY`-style).
**Column** — Board lane holding workflow state (work board: To-do, WIP, Review, Done, Archive). State lives in the column, not in labels.
**Canonical state** — CLI-level abstraction (`todo | wip | review | done | archive`) mapped to real column IDs in `.mpx/kanbanflow.json` so skills stay board-agnostic.
**Label** — Colored tag on a task, used at work for area/project (e.g. `Yoursafe Components`, `Flowguard SDK`).
**Color** — Card color, semantic at work (red ≈ bug/critical, green ≈ normal). API enum: yellow, green, blue, red, orange, purple, magenta, cyan.
**Subtask** — Checklist item on a task (name + finished flag). Team E uses these.
**Attachment** — File on a task. Upload = multipart POST; download = expiring S3 link from the API.
**Responsible user** — The single assignee of a task; basis for the ownership guardrail.

_Avoid_: "issue" for Task (GitHub vocabulary), "swimlane" (work board has none), "card" in code (API says task).

## Relationships

- Board 1:N Columns, 1:N Labels, 1:N Users, 1:N Tasks
- Task N:1 Column, N:M Labels, 1:N Subtasks, 1:N Comments, 1:N Attachments, 0..1 Responsible user
- API token N:1 Board (a token reaches exactly one board, acting as its creating user)

## Core Features

- `kf init` — planned. Verify token, fetch board, interactively map columns → canonical states, store user ID, write `.mpx/kanbanflow.json`.
- `kf auth login` — planned. Token into Windows Credential Manager (`keyring` crate); `KANBANFLOW_TOKEN` env overrides.
- `kf task grab|finish` — planned. Compound workflow verbs: `grab` = assign me + move to WIP + aggregated view + image downloads; `finish` = comment from `--comment-file` + move + optional `--check-subtasks`.
- `kf task create|view|list|edit|move|delete` — planned. Reference by task number; `create --attach <file>...` (create + uploads, labels validated, never created implicitly); `view` aggregates task + comments + attachments, `--download-attachments <dir>`; `edit --append-description` (lossless server-side merge); `move --to <canonical-state>`.
- `kf attach add|list|download|delete` — planned. The image round-trip.
- `kf comment add|list|edit|delete` — planned.
- `kf subtask add|list|check|uncheck` — planned.
- `kf label list`, `kf board` — planned. Read-only board metadata.
- `kf-` skills (`kf-task-create`, `kf-task-view`, `kf-task-edit`) — planned. Live in `skills/`, symlinked per-project.

## Key Constraints

- API token is per-board and premium-only; work-board token pending admin (author's role sees no Settings menu on Team E).
- Rate limit: 1000 requests/hour/board; >5000/day locks the token. Cache board metadata; be frugal.
- Shared team board: never move/edit tasks whose responsible user isn't you (unless `--force`); never create labels implicitly.
- Attachment download links expire (`linkExpiresTimestamp`, ~24h); download immediately, never store links.
- `GET /tasks/<id>` returns subtasks/labels/custom fields inline but never comments or attachments — a full task read is 3 API calls (task, comments, attachments). Verified 2026-07-31 on sandbox.
- No secrets in the repo — token via Credential Manager or `KANBANFLOW_TOKEN` only.
- Work board pairs with GitLab (`gitlab.com`, group `bitsafe`); auto-move triggers fire from `glab` MR events in skills, not webhooks.
- `--json` on every read command; meaningful exit codes — the agent contract.
- Sandbox for development: personal board `F2QMK1B` ("My first board", 14-day trial started 2026-07-31), columns To-do / Do today / In progress / Done.
- API docs snapshot lives in `docs/api/` (57 pages); refresh with `node scripts/scrape-docs.mjs docs/api`.

## Flagged Ambiguities

- "Done" column on work board is date-grouped — moving there may require `groupingDate`; verify against sandbox before implementing `move`.
- Work board's `Requested By:` chip is likely a custom field; confirm via API once work token exists. Custom fields are out of v1 scope until then.
