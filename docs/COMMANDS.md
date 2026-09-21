# `kf` command reference

Every command, every flag, as the binary reports it. Grammar: `kf <noun> <verb> [args] [flags]`.

`<ISSUE>` is always an issue number (`E613`) or an issue ID. Commands marked **guarded** refuse to
mutate an issue that is nobody's or a teammate's and exit `3` until `--force` is passed; being the
responsible user **or** a collaborator makes it yours.

- [init](#init)
- [auth](#auth) — [login](#auth-login), [logout](#auth-logout), [status](#auth-status)
- [issue](#issue) — [create](#issue-create), [view](#issue-view), [list](#issue-list), [edit](#issue-edit), [move](#issue-move), [delete](#issue-delete), [grab](#issue-grab), [finish](#issue-finish)
- [attach](#attach) — [add](#attach-add), [list](#attach-list), [download](#attach-download), [delete](#attach-delete)
- [comment](#comment) — [add](#comment-add), [list](#comment-list), [edit](#comment-edit), [delete](#comment-delete)
- [subtask](#subtask) — [add](#subtask-add), [list](#subtask-list), [check](#subtask-check), [uncheck](#subtask-uncheck)
- [label](#label) — [list](#label-list)
- [board](#board)

Canonical states everywhere: `todo`, `wip`, `review`, `done`, `archive`.
Issue colors everywhere: `yellow`, `white`, `red`, `green`, `blue`, `purple`, `orange`, `cyan`,
`brown`, `magenta`.

---

## init

Set `issues.provider` and update KanbanFlow board data under `issues.metadata` in an existing valid
`mpxconfig.json`: verify the token and map columns to canonical states while preserving unrelated
root, metadata, and state fields. The root needs a non-empty `projectId`; `repository` is optional.
`--json`: yes. Guarded: no.

Token order: `KANBANFLOW_TOKEN`, `--token`/`--token-stdin`, the stored token of the board this repo
is already wired to, the stored token of a logged-in board (`--board`, or a question when several
are logged in), then a hidden prompt. A token is pasted once per board, not once per repo.

The written file holds board facts only. Who you are on the board goes into the user-level registry
instead, and is asked for only when that board has no identity recorded yet — see
[`auth login`](#auth-login).

```
kf init [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--token` | `TOKEN` | Use this API token. Lands in shell history — prefer `--token-stdin`. | stored token, else prompt |
| `--token-stdin` | — | Read the token as a single line from stdin. | off |
| `--board` | `BOARD` | Use the stored token of this board (ID, name, or 1-based index from `kf auth status`). Skips the question when several boards are logged in. | one board: it; several: asks |
| `--no-store` | — | Do not save the token in the OS credential store. | stores |
| `--map` | `STATE=COLUMN` | Map a canonical state to a column; repeatable. Any use switches mapping to non-interactive mode. `todo`, `wip`, `review`, and `done` are required; `archive` is optional. Each mapped state must use a distinct column. `COLUMN` may be a name, a `uniqueId`, or a 1-based index. | interactive |
| `--user` | `USER` | Which board member you are: user ID, full name, or email. Recorded for you only, never written to the repo. | the recorded identity, else interactive |
| `--json` | — | Print the resulting configuration as JSON, plus the `userId` this run settled on. | human |

```bash
kf init --map todo=To-do --map wip="In progress" --map review=Review --map done=Done --user U9kJ2b --json
```

---

## auth

### auth login

Store an API token for the board it belongs to, record that board so `kf init` can reuse the token
in any repo, and settle which board member you are. Run once per board. `--json`: no. Guarded: no.

A KanbanFlow token belongs to a board rather than to a person, and the API has no "who am I"
endpoint, so the acting user is a choice. It decides what `--mine` matches and which issues the
ownership guardrail protects, it differs per teammate, and it is therefore kept in the user-level
registry rather than in the committed `mpxconfig.json`. A board with a single member needs no
question. `KANBANFLOW_USER_ID` overrides the recorded value for one invocation.

```
kf auth login [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--token` | `TOKEN` | The API token, visible in shell history. | prompt |
| `--token-stdin` | — | Read the token as a single line from stdin. | off |
| `--with-token-file` | `PATH` | Read the token from this file instead of prompting. | off |
| `--board` | `BOARD` | Re-use the token already stored for this board (ID, name, or 1-based index from `kf auth status`) instead of asking for one. Conflicts with the token flags. | asks for a token |
| `--user` | `USER` | Which board member you are: user ID, full name, or email. | the recorded identity, else sole member, else asks |

```bash
printf '%s' "$KANBANFLOW_TOKEN" | kf auth login --token-stdin --user martin@example.com
```

`--board` exists for boards logged in before identities were recorded: it settles who you are
without a trip to the KanbanFlow settings page for a token this machine already holds.

```bash
kf auth login --board "Team E" --user martin@example.com
```

### auth logout

Delete a board's stored token and drop it from the board registry, along with the identity recorded
for it. `mpxconfig.json` is left alone, so `kf auth login` restores both. `--json`: no.
Guarded: no.

```
kf auth logout [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--board` | `BOARD` | The board to forget: ID, name, or 1-based index from `kf auth status`. A raw board ID also works for a token stored before the registry existed. | one board: it; several: refuses |
| `--all` | — | Forget every logged-in board. Conflicts with `--board`. | off |
| `--yes` | — | Skip the confirmation prompt. | asks |

With several boards logged in and no `--board`, the command refuses rather than guessing which
credential to delete. Forgetting a board that has no stored token is not an error — the registry
entry is removed either way.

```bash
kf auth logout --board "Team E" --yes
```

### auth status

Show which token source this directory would use, who you act as on this repo's board, and which
boards are logged in. Makes no HTTP request. `--json`: yes (adds `userId` and `knownBoards`).
Guarded: no.

```
kf auth status [--json]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--json` | — | Print JSON instead of the human-readable output. |

```bash
kf auth status --json
```

---

## issue

### issue create

Create an issue, optionally uploading attachments in the same step. `--json`: yes. Guarded: no.

```
kf issue create --name <NAME> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--name` | `NAME` | Issue name. **Required.** | — |
| `--description` | `TEXT` | Issue description, Markdown as the board renders it. | empty |
| `--color` | color | Issue color. | board default |
| `--to` | state | Canonical state (column) to create the issue in. | `todo` |
| `--label` | `NAME` | Existing board label to apply; repeatable. Unknown labels are refused. | none |
| `--responsible` | `me` \| user ID \| `none` | Responsible user. **Defaults to you**; pass `none` to leave the issue unassigned. | you |
| `--attach` | `FILE` | File to upload onto the new issue; repeatable. | none |
| `--grouping-date` | `YYYY-MM-DD` | Grouping date for date-grouped columns. | server: today |
| `--json` | — | Print JSON instead of the human-readable output. | human |

```bash
kf issue create --name "Fix login redirect" --to wip --label bug --attach ./shot.png --json
```

### issue view

Show an issue with its comments and attachments in one aggregated view. `--json`: yes. Guarded: no.

```
kf issue view <ISSUE> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--download-attachments` | `DIR` | Save every attachment into this directory. Links expire, so do it now. | off |
| `--json` | — | Print JSON instead of the human-readable output. | human |

```bash
kf issue view E613 --download-attachments ./attachments
```

### issue list

List issues on the board. `--json`: yes. Guarded: no.

```
kf issue list [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--state` | state | Only issues in this canonical state; repeatable, and several states select the union of their columns. | all |
| `--column` | `NAME_OR_ID` | Only issues in this column, by name or ID — for columns with no canonical state. | all |
| `--open` | — | Only unfinished work: every column except those mapped to `done` and `archive`. | all |
| `--mine` | — | Only issues you are responsible for **or** a collaborator on. | all |
| `--json` | — | Print JSON instead of the human-readable output. | human |

`--state`, `--column` and `--open` are mutually exclusive.

State names are matched case-insensitively. A state with no column mapped in
`mpxconfig.json` is an error (exit 7), not an empty result.

`--open` filters by **exclusion**, which is what makes it board-agnostic: it drops the required
`done` column and also drops the optional `archive` column when present. It keeps everything else,
including lanes the board invented that map to no canonical state (a "Do today", a "Blocked").
Listing the open states by hand instead would silently miss those.

`--mine` follows the board's own reading of "assigned to me": KanbanFlow draws your avatar on an
issue whether you are its responsible user or one of its collaborators, and teams that assign work
by adding collaborators would otherwise see nothing. The `PEOPLE` column shows the responsible
user first, then each collaborator prefixed with `+` — so `+me` is an issue you collaborate on but
are not responsible for. A `+me` issue is yours to mutate as well; only `kf issue grab` still judges
by the responsible user alone.

The `STATE` column prints the canonical state, or the column's name when it has none.

```bash
kf issue list --open --mine                     # everything of mine that is not finished
kf issue list --state todo --state wip --mine   # the same, restricted to two named states
kf issue list --state wip --mine
```

### issue edit

Change an issue's fields. `--json`: yes. **Guarded** (`--force`).

```
kf issue edit <ISSUE> [OPTIONS]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--name` | `NAME` | Replace the issue name. |
| `--description` | `TEXT` | Replace the description. |
| `--append-description` | `TEXT` | Append to the description after a blank line (lossless merge). |
| `--color` | color | Issue color. |
| `--add-label` | `NAME` | Add an existing board label; repeatable. |
| `--remove-label` | `NAME` | Remove a label; repeatable. |
| `--responsible` | `me` \| user ID \| `none` | Set or clear the responsible user. |
| `--force` | — | Mutate an issue that is not yours. |
| `--json` | — | Print JSON instead of the human-readable output. |

```bash
kf issue edit E613 --append-description "Repro on Safari 17." --add-label bug
```

### issue move

Move an issue to a canonical workflow state. `--json`: no. **Guarded** (`--force`).

```
kf issue move <ISSUE> --to <TO> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--to` | state | Target canonical state. **Required.** | — |
| `--grouping-date` | `YYYY-MM-DD` | Grouping date when the target column is date grouped. | server: today UTC |
| `--force` | — | Mutate an issue that is not yours. | off |

```bash
kf issue move E613 --to done --grouping-date 2026-07-31
```

### issue delete

Delete an issue. `--json`: no. **Guarded** (`--force`).

```
kf issue delete <ISSUE> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--yes` | — | Skip the confirmation prompt (required when stdin is not a terminal). | prompts |
| `--force` | — | Delete an issue that is not yours. | off |

```bash
kf issue delete E613 --yes
```

### issue grab

Assign the issue to yourself, move it to `wip` and print the aggregated view — one call instead of
three. `--json`: yes. **Guarded** (`--force`).

```
kf issue grab <ISSUE> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--download-dir` | `DIR` | Save the issue's image attachments into this directory. | off |
| `--force` | — | Grab an issue someone else is responsible for. | off |
| `--json` | — | Print JSON instead of the human-readable output. | human |

Grabbing rewrites the responsible user, so it is the one guarded command judged on that field
alone: collaborating on a teammate's issue does not let you pull their name off it without
`--force`. An unassigned issue is fair game.

```bash
kf issue grab E613 --download-dir ./attachments --json
```

### issue finish

Post a closing comment, optionally check off every subtask, and move the issue on. `--json`: no.
**Guarded** (`--force`).

```
kf issue finish <ISSUE> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--comment-file` | `PATH` | File whose contents become the closing comment. | no comment |
| `--to` | state | Target canonical state. | `done` |
| `--check-subtasks` | — | Mark every unfinished subtask finished. | off |
| `--force` | — | Finish an issue that is not yours. | off |

```bash
kf issue finish E613 --comment-file ./summary.md --check-subtasks
```

---

## attach

### attach add

Upload one or more files to an issue. `--json`: no. **Guarded** (`--force`).

```
kf attach add <ISSUE> <FILES>...
```

| Argument / flag | Value | Effect |
| --- | --- | --- |
| `<FILES>...` | paths | Files to upload. At least one required. |
| `--force` | — | Upload even when the issue belongs to someone else. |

```bash
kf attach add E613 ./before.png ./after.png
```

### attach list

List an issue's attachments. `--json`: yes. Guarded: no.

```
kf attach list <ISSUE> [--json]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--json` | — | Print JSON instead of the human-readable output. |

```bash
kf attach list E613 --json
```

### attach download

Download an issue's attachments before their links expire. `--json`: yes. Guarded: no.

```
kf attach download <ISSUE> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--name` | `NAME` | Only the attachment with this exact name. | every attachment |
| `--dir` | `DIR` | Directory to save into; created when missing. | `.` |
| `--json` | — | Print JSON instead of the human-readable output. | human |

```bash
kf attach download E613 --dir ./attachments
```

### attach delete

Remove an attachment from an issue. `--json`: no. **Guarded** (`--force`).

```
kf attach delete <ISSUE> --name <NAME> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--name` | `NAME` | Exact name of the attachment to remove. **Required.** | — |
| `--yes` | — | Skip the confirmation prompt. | prompts |
| `--force` | — | Delete even when the issue belongs to someone else. | off |

```bash
kf attach delete E613 --name shot.png --yes
```

---

## comment

### comment add

Add a comment to an issue. `--json`: no. **Guardrail-exempt** — commenting on a teammate's issue never
needs `--force`, and no `--force` flag exists. Exactly one of `--text` / `--file` is required.

```
kf comment add <ISSUE> <--text <TEXT>|--file <FILE>>
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--text` | `TEXT` | Comment text. |
| `--file` | `FILE` | Read the comment text from a file. |

```bash
kf comment add E613 --text "Deployed to staging, awaiting QA."
```

### comment list

List an issue's comments with their IDs. `--json`: yes. Guarded: no.

```
kf comment list <ISSUE> [--json]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--json` | — | Print JSON instead of the human-readable output. |

```bash
kf comment list E613 --json
```

### comment edit

Replace a comment's text. `--json`: no. **Guarded** (`--force`). Exactly one of `--text` / `--file`
is required.

```
kf comment edit <ISSUE> --id <ID> <--text <TEXT>|--file <FILE>> [--force]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--id` | `ID` | ID of the comment to change; see `kf comment list`. **Required.** |
| `--text` | `TEXT` | New comment text. |
| `--file` | `FILE` | Read the new comment text from a file. |
| `--force` | — | Edit even when the issue belongs to someone else. |

```bash
kf comment edit E613 --id cM4kQ2 --text "Corrected: staging, not production."
```

### comment delete

Delete a comment. `--json`: no. **Guarded** (`--force`).

```
kf comment delete <ISSUE> --id <ID> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--id` | `ID` | ID of the comment to delete; see `kf comment list`. **Required.** | — |
| `--yes` | — | Skip the confirmation prompt. | prompts |
| `--force` | — | Delete even when the issue belongs to someone else. | off |

```bash
kf comment delete E613 --id cM4kQ2 --yes
```

---

## subtask

Checklist positions are **1-based on every surface** — `subtask list` numbers from 1, and
`check` / `uncheck` accept that same number (or the exact item name).

### subtask add

Append a checklist item. `--json`: no. **Guarded** (`--force`).

```
kf subtask add <ISSUE> <NAME> [--force]
```

| Argument / flag | Value | Effect |
| --- | --- | --- |
| `<NAME>` | text | Checklist item name. **Required.** |
| `--force` | — | Add even when the issue belongs to someone else. |

```bash
kf subtask add E613 "Write regression test"
```

### subtask list

List an issue's checklist items with 1-based positions. `--json`: yes. Guarded: no.

```
kf subtask list <ISSUE> [--json]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--json` | — | Print JSON instead of the human-readable output. |

```bash
kf subtask list E613 --json
```

### subtask check

Mark a checklist item finished. `--json`: no. **Guarded** (`--force`).

```
kf subtask check <ISSUE> <SUBTASK> [--force]
```

| Argument / flag | Value | Effect |
| --- | --- | --- |
| `<SUBTASK>` | name \| position | Exact checklist item name, or its 1-based position. **Required.** |
| `--force` | — | Change even when the issue belongs to someone else. |

```bash
kf subtask check E613 2
```

### subtask uncheck

Mark a checklist item unfinished. `--json`: no. **Guarded** (`--force`).

```
kf subtask uncheck <ISSUE> <SUBTASK> [--force]
```

| Argument / flag | Value | Effect |
| --- | --- | --- |
| `<SUBTASK>` | name \| position | Exact checklist item name, or its 1-based position. **Required.** |
| `--force` | — | Change even when the issue belongs to someone else. |

```bash
kf subtask uncheck E613 "Write regression test"
```

---

## label

### label list

List the labels in use on the board. Labels are never created implicitly, so this is the set
`--label` / `--add-label` accept (matched case-insensitively). `--json`: yes. Guarded: no.

```
kf label list [--json]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--json` | — | Print JSON instead of the human-readable output. |

```bash
kf label list --json
```

---

## board

Show the board's columns and the canonical-state mapping from `mpxconfig.json`.
`--json`: yes. Guarded: no.

```
kf board [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--counts` | — | Also show how many issues sit in each column; costs one extra request. | off |
| `--json` | — | Print the raw `GET /board` response. | human |

```bash
kf board --counts
```
