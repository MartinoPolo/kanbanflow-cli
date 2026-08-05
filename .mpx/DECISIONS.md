# DECISIONS

## Platform & Infrastructure

### Rust with clap + reqwest + serde + keyring

Decided: 2026-07-31
What: The CLI is Rust; `clap` for commands, `reqwest` for HTTP, `serde` for JSON, `keyring` for token storage.
Why: Single static binary with ~5ms startup fits "one-shot it and it just works"; `keyring` gives Windows Credential Manager for free; author steers, agents write all code.
Rejected: TypeScript/Node (runtime dependency, ~80ms startup per agent call); Go (equally viable, lost on token-storage story and author's Rust curiosity).

### Distribution via GitHub releases with cargo-dist

Decided: 2026-07-31
What: Private GitHub repo `kanbanflow-cli` under the personal account; `cargo-dist` generates installers when publishing becomes desirable.
Why: Zero publishing effort now, one-command path to shell/PowerShell installers, npm shim, and Homebrew tap later.
Rejected: npm-first distribution (ties binary to Node); publishing publicly at v0 (premature).

### Binary named kf, repo named kanbanflow-cli

Decided: 2026-07-31
What: Two-letter binary `kf`, gh-style grammar `kf <noun> <verb>` with KanbanFlow nouns (`task`, not `issue`).
Why: Matches `gh`/`glab` muscle memory for agents and human; honest domain vocabulary.
Rejected: `kbf`/`kanban` (longer, no clarity gained); mimicking gh's `issue` noun (pretends wrong domain).

### API docs vendored as Markdown with a repeatable scraper

Decided: 2026-07-31
What: All 57 pages of kanbanflow.com/api-docs scraped to `docs/api/*.md`; scraper (`scripts/scrape-docs.mjs`, Node + turndown) committed for refreshes.
Why: Docs are server-rendered static HTML, trivially scrapable; a local snapshot makes agent-driven maintenance self-contained.
Rejected: Scrape-on-demand (needless network dependency; docs rarely change).

## CLI Design

### Tasks referenced by board number, IDs accepted

Decided: 2026-07-31
What: Commands take `E613`-style task numbers (resolved via get-tasks) or raw task IDs interchangeably.
Why: Numbers are what humans and the board UI show; the API's `number` object (`{prefix, value}`) makes resolution reliable.
Rejected: ID-only interface (unusable for humans reading the board).

### Canonical workflow states mapped per-board in .mpx/kanbanflow.json

Decided: 2026-07-31
What: `kf task move --to todo|wip|review|done|archive` resolves through a committed per-repo mapping written by `kf init`; column IDs are not secrets.
Why: Skills and agents stay portable across boards with different column names; auto-move triggers (start work → wip, MR open → review, MR merged → done) live in skills.
Rejected: Raw column names/IDs in every call (brittle, board-specific); webhooks for auto-move (needs a 24/7 public listener; a CLI cannot receive them).

### Acting user lives in the user-level registry, not in the committed config

Decided: 2026-08-05
What: `.mpx/kanbanflow.json` holds board facts only. Which board member `kf` acts as is settled once per board at `kf auth login` (or the first `kf init`), stored in `boards.json` beside the token, overridable with `KANBANFLOW_USER_ID`. A `userId` in an older config is still read, last, for compatibility.
Why: The file is committed so the team shares one column mapping, but the acting user differs per teammate — a cloned `userId` made `--mine` list its author's tasks and pointed the guardrail at the wrong cards. A KanbanFlow token is per board and the API has no "who am I" endpoint, so identity is a choice and cannot be derived.
Rejected: Gitignoring the whole config (loses the shared mapping that is the file's reason to exist); keeping `userId` and having each teammate re-run `kf init` (a merge conflict on every clone, and silently wrong until someone notices).

### `.mpx/` is tracked; only `.mpx/tmp/` is ignored

Decided: 2026-08-05
What: Repos commit `.mpx/` — `CONTEXT.md`, `DECISIONS.md`, `kanbanflow.json` — and gitignore `.mpx/tmp/`, the scratch area the `kf-` skills download attachments into.
Why: The directory's contents are now all team-shared by construction; one namespace tracked or ignored as a unit beats a root dotfile plus a separate scratch directory, and the skills already write scratch under `.mpx/tmp/`.
Rejected: A root `.kanbanflow.json` (a rename touching config, init, skills and docs for no functional gain); gitignoring all of `.mpx/` (would drop the shared column mapping and the decision log).

### Token in Windows Credential Manager, env var override

Decided: 2026-07-31
What: `kf auth login` stores the per-board token via `keyring`; `KANBANFLOW_TOKEN` overrides for dev/CI; nothing token-shaped ever in files.
Why: Shared-board credentials deserve OS-level storage; env override keeps agents and CI simple.
Rejected: Config-file token (leak risk in a repo-adjacent file); env-only (no persistence across shells).

### Shared-board guardrails on by default

Decided: 2026-07-31
What: Refuse to move/edit/delete a task whose responsible user isn't the token's user unless `--force`; never create labels implicitly.
Why: Team E is a shared board with 17 members; agent mistakes must not disturb teammates' cards.
Rejected: Trust-the-agent default (one bad loop spams the whole team).

### Agent output contract: --json everywhere, human tables by default

Decided: 2026-07-31
What: Every read command supports `--json`; write commands return the affected task number/ID; `view --download-attachments <dir>` saves images for agents to Read.
Why: Mirrors the `gh --json` contract agents already script against; the download flag turns image-reading into one step.
Rejected: JSON-only output (hostile to the human half of the workflow).

### Compound workflow commands over agent orchestration

Decided: 2026-07-31
What: First-class compound commands collapse multi-step agent dances: `kf task grab` (assign me + move to WIP + aggregated view + image downloads), `kf task finish` (comment from file + move + optional subtask check-off), `view` aggregating task+comments+attachments (the API forces 3 calls), `create --attach` (create + uploads + label validation), `edit --append-description` (lossless server-side merge).
Why: Every scripted step replaces an agentic step — deterministic, cheaper, faster; skills shrink to composing content and running one command.
Rejected: `kf next` (picking policy belongs in skills; CLI stays mechanical); `link-mr` and `watch` commands (no demonstrated need); leaving orchestration to agents (the pain this project exists to remove).

## Scope & Integration

### v1 scope: tasks, attachments, comments, subtasks, labels-read, board, init, auth

Decided: 2026-07-31
What: In v1: full task CRUD + move, attachment round-trip, comments, subtasks, `label list`, `board`, `init`, `auth login`. Out: hierarchy/relations, time tracking, webhooks, move-between-boards, custom fields.
Why: Covers everything observed in real Team E usage (subtask checklists included); work team uses no PRD/sub-issue hierarchy.
Rejected: Time tracking (author doesn't use it); webhooks (see auto-move decision); custom fields (deferred until work token reveals actual fields).

### Dedicated kf- skills instead of provider-switching existing skills

Decided: 2026-07-31
What: New `kf-task-create`, `kf-task-view`, `kf-task-edit` skills live in this repo under `skills/` and get symlinked into projects that use KanbanFlow.
Why: Work environment needs only a small skill set; symlink selection replaces any `MPX_TICKETING` switching variable; existing 13 gh-hardcoded skills stay untouched.
Rejected: Conditional branches in existing skills (13 mechanical edits, ongoing dual-path maintenance); a gh-compatible facade (GitHub concepts don't map 1:1).

### KanbanFlow CLI replaces the Obsidian board workaround

Decided: 2026-07-31
What: For KanbanFlow projects, images attach to tasks directly; the `.mpx/BOARD.md` + vault-junction pattern from BOARD_CONVENTION.md is not used.
Why: The workaround existed only because gh cannot round-trip images; the KanbanFlow API can.
Rejected: Running both systems side by side (duplicate state, two sources of truth).
