# kanbanflow-cli

`kf` is a Rust command-line client for [KanbanFlow](https://kanbanflow.com) with `gh`-style grammar
(`kf <noun> <verb>`), built for AI-agent workflows: predictable subcommands, `--json` on every read,
and — the feature `gh` and `glab` both lack — a full **image attachment round-trip**, so an agent can
upload a screenshot to an issue and pull it back down without leaving the terminal.

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
kf auth login                  # once per board: store its API token and who you are on it
cd /path/to/a/repo
kf init                        # updates only the issues binding in existing mpxconfig.json
```

`kf init` never asks again for a token or an identity that is already stored. With several boards
logged in it asks which one this repo belongs to; `--board <id|name>` answers that
non-interactively.

API tokens are **per board** and require a KanbanFlow premium plan; create one in the board's
**Settings → API & Webhooks**.

## Most-used commands

| Command | What it does |
| --- | --- |
| `kf init` | Update only the `issues` binding in an existing `mpxconfig.json` by mapping the board's columns to canonical states |
| `kf auth login` | Store the board's API token in the OS credential store |
| `kf issue list --open --mine` | List issues, filtered by state/column (`--state` repeats) or `--open` (everything but `done`/`archive`), and by ownership — `--mine` covers issues you are responsible for **or** collaborate on |
| `kf issue view E613 --download-attachments DIR` | Issue + comments + attachments in one view, files saved locally |
| `kf issue create --name ...` | Create an issue; assigned to you unless `--responsible none` |
| `kf issue grab E613` | Assign to yourself, move to `wip`, print the view |
| `kf issue finish E613 --comment-file F` | Closing comment, optional subtask check-off, move to `done` |
| `kf issue move E613 --to review` | Move an issue to a canonical state |
| `kf comment add E613 --text ...` | Comment on any issue — never blocked by the guardrail |
| `kf attach add E613 shot.png` | Upload files onto an issue |
| `kf attach download E613 --dir DIR` | Pull attachments down before their links expire |
| `kf board --counts` | Columns, canonical-state mapping, issue counts |

```bash
kf issue grab E613 --download-dir ./attachments && kf issue finish E613 --comment-file ./summary.md
```

Full reference — every command and flag: [`docs/COMMANDS.md`](docs/COMMANDS.md).

## Authentication

Tokens are never written into a file in the repo. Resolution order:

1. `KANBANFLOW_TOKEN` environment variable (wins whenever set and non-empty)
2. The OS credential store (Windows Credential Manager and equivalents, via `keyring`), keyed by
   board ID under the service `kanbanflow-cli`

`kf auth login` stores a token; `kf auth logout` deletes one; `kf auth status` shows which source
this directory would use, who you are on the board, and which boards are logged in.

The credential store cannot be enumerated portably, so the board IDs to look tokens up by are kept
in a **user-level registry** — `%APPDATA%\kanbanflow-cli\boards.json` on Windows,
`$XDG_CONFIG_HOME/kanbanflow-cli/boards.json` or `~/.config/…` elsewhere. It holds board IDs, board
names and your user ID on each board, never a token, and losing it costs nothing but one re-login.
`kf init` reads it to reuse a stored token while updating an existing `mpxconfig.json`.

A pasted token is sanitized before use: surrounding whitespace is trimmed silently, and interior
control codes, zero-width marks and BOMs are removed with a `note:` on stderr. Anything left that an
HTTP header cannot carry is refused with exit 4 instead of reaching the API.

## Who you are on a board

A KanbanFlow token belongs to a **board**, not to a person, and the API has no "who am I" endpoint —
so which board member you act as is a choice, made once per board at `kf auth login` (or the first
`kf init`) and answered non-interactively by `--user <id|name|email>`. It decides what `--mine`
matches and which issues the ownership guardrail protects.

Because it differs per teammate it is **never written into the repo**; it lives in the user-level
registry beside the token. Resolution order:

1. `KANBANFLOW_USER_ID` environment variable — for CI and agents with no registry to read
2. The board registry

For a board you logged in to before identities were kept, record yours without re-pasting the token:

```bash
kf auth login --board "Team E" --user you@example.com
```

## `mpxconfig.json`

`kf init` requires this committed MPX project config to exist. It updates only the `issues`
binding, preserving project, repository, tooling, and unknown root fields. It never writes tokens
or users. `boardName` and `archive` are optional; the four workflow states are required.

```json
{
  "schemaVersion": 1,
  "project": { "id": "acme/widget" },
  "repository": { "provider": "gitlab", "remote": "git@example/acme/widget.git" },
  "issues": {
    "provider": "kanbanflow",
    "boardId": "BEXAMPLE",
    "boardName": "Example board",
    "states": {
      "todo": "CTODO",
      "wip": "CWIP",
      "review": "CREVIEW",
      "done": "CDONE"
    }
  }
}
```

Ignore only MPX scratch data:

```gitignore
.mpx/tmp/
```

## Agent contract

Every read command takes `--json` and prints pretty JSON to stdout instead of the human table; every
mutation echoes the affected issue as `NUMBER (ISSUE_ID)`. Errors go to stderr prefixed with `error:`.

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Unclassified failure |
| 2 | Usage error (bad flags or arguments) |
| 3 | Shared-board guardrail refused the mutation; pass `--force` |
| 4 | No usable API token, or the API rejected it (including a Credential Manager failure) |
| 5 | Board, issue, comment or attachment not found |
| 6 | Rate limited (1000 requests/hour/board); back off |
| 7 | No usable `mpxconfig.json`; run `kf init` |

**Guardrail.** Issues you create are yours by default, so the normal loop never trips it. An issue is
yours when you are its responsible user or one of its collaborators; mutating one that is nobody's
or a teammate's is refused (exit 3) — take it with `kf issue grab` or override with `--force`.
`kf comment add` is exempt, and `kf issue grab` is the one command judged on the responsible user
alone, since it reassigns that field. Labels are never created implicitly:
`--label` / `--add-label` accept only names already on the board, matched case-insensitively.

Date-grouped columns (a work-board "Done" is one) hand out only their first 20 issues per cell. `kf`
follows the API's continuation cursor, so `issue list`, `issue view E613`, `label list` and
`board --counts` see the whole column; the extra requests are spent only on columns that report the
truncation.

## Claude Code plugin

This repo doubles as a Claude Code plugin named `kf`: the KanbanFlow + GitLab work-stack skills
that drive the `kf` CLI from an agent session. Load it by pointing Claude Code at this repo — the
flag is repeatable, so it stacks with other plugins:

```bash
claude --plugin-dir /path/to/kanbanflow-cli
```

Skills then invoke as `/kf:<name>`:

| Skill | What it does |
| --- | --- |
| `/kf:issue-create` | Compose a well-formed issue from the session context and file it after human approval |
| `/kf:issue-view` | Read an issue in full — description, subtasks, comments, attached images — in as few API calls as possible |
| `/kf:issue-edit` | Grab, note, comment, move, relabel and finish an issue as the work happens |
| `/kf:issue-grill` | Resolve a HITL issue into an AFK-ready one by settling its open decisions with the human |
| `/kf:execute` | Take one AFK issue to a draft GitLab MR with a green pipeline, in its own worktree |
| `/kf:batch-execute` | Run the execute flow over several AFK issues sequentially, each fully isolated |
| `/kf:mr` | Push the branch and open or update its GitLab merge request as a draft via `glab` |
| `/kf:board-sync` | Reconcile the board with reality: move issues forward to match their MR state (draft → `wip`, ready → `review`, merged → `done`) and local branches |

**Requirements**

- The `kf` binary on PATH — `cargo install --path .` from this repo (see [Install](#install)).
- A KanbanFlow token, via the `KANBANFLOW_TOKEN` environment variable or the OS keyring
  (`kf auth login`). See [Authentication](#authentication).
- An existing per-repo `mpxconfig.json` in each project the skills run against — `kf init` updates only its `issues` binding.
- `/kf:execute`, `/kf:batch-execute` and `/kf:mr` additionally need `glab` authenticated against
  the project's GitLab host.

The execution skills reference the `mp` plugin's `/mp:commit` skill and a set of `mp:mp-*`
sub-agents. Load the `mp` plugin alongside (another `--plugin-dir`) to get them; without it, the
skills fall back to running those phases in the main thread.

## Repo layout

| Path | Purpose |
| --- | --- |
| `src/` | Rust sources for the `kf` binary |
| `docs/COMMANDS.md` | Full command and flag reference |
| `docs/api/` | Vendored snapshot of the KanbanFlow API documentation |
| `scripts/scrape-docs.mjs` | Refreshes `docs/api/` from the live documentation site |
| `.claude-plugin/plugin.json` | Manifest for the `kf` Claude Code plugin |
| `skills/` | The `kf` plugin's skills, invoked as `/kf:<name>` (see above) |

Refresh the vendored API docs with:

```bash
cd scripts && npm install && node scrape-docs.mjs ../docs/api
```

## License

MIT
