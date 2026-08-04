# kanbanflow-cli

`kf` is a Rust command-line client for [KanbanFlow](https://kanbanflow.com) with `gh`-style grammar
(`kf <noun> <verb>`), built for AI-agent workflows: predictable subcommands, `--json` on every read,
and — the feature `gh` and `glab` both lack — a full **image attachment round-trip**, so an agent can
upload a screenshot to a task and pull it back down without leaving the terminal.

**Status:** v1 command surface implemented. Design and decision log:
[`.mpx/CONTEXT.md`](.mpx/CONTEXT.md), [`.mpx/DECISIONS.md`](.mpx/DECISIONS.md).

## Install

Build from source (Rust stable):

```bash
git clone git@github.com:MartinoPolo/kanbanflow-cli.git && cd kanbanflow-cli
cargo build --release          # binary at target/release/kf.exe — put its directory on PATH
cargo install --path .         # or install straight into ~/.cargo/bin
```

Prebuilt cargo-dist installers are planned but **not yet published**.

Authenticate **once per board**, then wire up as many repos as you like:

```bash
kf auth login                  # once per board: store its API token
cd /path/to/a/repo
kf init                        # reuses the stored token, maps columns, writes .mpx/kanbanflow.json
```

`kf init` never asks for a token that is already stored. With several boards logged in it asks which
one this repo belongs to; `--board <id|name>` answers that non-interactively.

API tokens are **per board** and require a KanbanFlow premium plan; create one in the board's
**Settings → API & Webhooks**.

## Most-used commands

| Command | What it does |
| --- | --- |
| `kf init` | Map the board's columns to canonical states and write `.mpx/kanbanflow.json` |
| `kf auth login` | Store the board's API token in the OS credential store |
| `kf task list --state wip --mine` | List tasks, filtered by state/column and ownership |
| `kf task view E613 --download-attachments DIR` | Task + comments + attachments in one view, files saved locally |
| `kf task create --name ...` | Create a task; assigned to you unless `--responsible none` |
| `kf task grab E613` | Assign to yourself, move to `wip`, print the view |
| `kf task finish E613 --comment-file F` | Closing comment, optional subtask check-off, move to `done` |
| `kf task move E613 --to review` | Move a task to a canonical state |
| `kf comment add E613 --text ...` | Comment on any task — never blocked by the guardrail |
| `kf attach add E613 shot.png` | Upload files onto a task |
| `kf attach download E613 --dir DIR` | Pull attachments down before their links expire |
| `kf board --counts` | Columns, canonical-state mapping, task counts |

```bash
kf task grab E613 --download-dir ./attachments && kf task finish E613 --comment-file ./summary.md
```

Full reference — every command and flag: [`docs/COMMANDS.md`](docs/COMMANDS.md).

## Authentication

Tokens are never written into a file in the repo. Resolution order:

1. `KANBANFLOW_TOKEN` environment variable (wins whenever set and non-empty)
2. The OS credential store (Windows Credential Manager and equivalents, via `keyring`), keyed by
   board ID under the service `kanbanflow-cli`

`kf auth login` stores a token; `kf auth logout` deletes one; `kf auth status` shows which source
this directory would use and which boards are logged in.

The credential store cannot be enumerated portably, so the board IDs to look tokens up by are kept
in a **user-level registry** — `%APPDATA%\kanbanflow-cli\boards.json` on Windows,
`$XDG_CONFIG_HOME/kanbanflow-cli/boards.json` or `~/.config/…` elsewhere. It holds board IDs and
names, never a token, and losing it costs nothing but one re-login. `kf init` reads it to reuse a
stored token in a repo that has no `.mpx/kanbanflow.json` yet.

A pasted token is sanitized before use: surrounding whitespace is trimmed silently, and interior
control codes, zero-width marks and BOMs are removed with a `note:` on stderr. Anything left that an
HTTP header cannot carry is refused with exit 4 instead of reaching the API.

## `.mpx/kanbanflow.json`

Written by `kf init` and **committed** — column IDs are not secrets, and the canonical-state mapping
is what keeps the `kf-` skills board-agnostic. Unmapped states are omitted.

```json
{
  "boardId": "F2QMK1B",
  "boardName": "My first board",
  "userId": "UHJ9JgtA",
  "states": {
    "todo": "C0LIn5sEEpqT",
    "wip": "C9LIn5sEEpqT",
    "done": "CqL5n5sEEpqT"
  }
}
```

## Agent contract

Every read command takes `--json` and prints pretty JSON to stdout instead of the human table; every
mutation echoes the affected task as `NUMBER (TASK_ID)`. Errors go to stderr prefixed with `error:`.

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Unclassified failure |
| 2 | Usage error (bad flags or arguments) |
| 3 | Shared-board guardrail refused the mutation; pass `--force` |
| 4 | No usable API token, or the API rejected it (including a Credential Manager failure) |
| 5 | Board, task, comment or attachment not found |
| 6 | Rate limited (1000 requests/hour/board); back off |
| 7 | No usable `.mpx/kanbanflow.json`; run `kf init` |

**Guardrail.** Tasks you create are yours by default, so the normal loop never trips it. Mutating a
task that is unassigned or belongs to a teammate is refused (exit 3) — take it with `kf task grab`
or override with `--force`. `kf comment add` is exempt. Labels are never created implicitly:
`--label` / `--add-label` accept only names already on the board, matched case-insensitively.

Date-grouped columns (a work-board "Done" is one) hand out only their first 20 tasks per cell. `kf`
follows the API's continuation cursor, so `task list`, `task view E613`, `label list` and
`board --counts` see the whole column; the extra requests are spent only on columns that report the
truncation.

## Repo layout

| Path | Purpose |
| --- | --- |
| `src/` | Rust sources for the `kf` binary |
| `docs/COMMANDS.md` | Full command and flag reference |
| `docs/api/` | Vendored snapshot of the KanbanFlow API documentation |
| `scripts/scrape-docs.mjs` | Refreshes `docs/api/` from the live documentation site |
| `skills/` | `kf-` prefixed Claude Code skills for agent-driven usage |

Refresh the vendored API docs with:

```bash
cd scripts && npm install && node scrape-docs.mjs ../docs/api
```

## License

MIT
