# `kf` command reference

Every command, every flag, as the binary reports it. Grammar: `kf <noun> <verb> [args] [flags]`.

`<TASK>` is always a task number (`E613`) or a task ID. Commands marked **guarded** refuse to
mutate a task that is nobody's or a teammate's and exit `3` until `--force` is passed; being the
responsible user **or** a collaborator makes it yours.

- [init](#init)
- [auth](#auth) — [login](#auth-login), [logout](#auth-logout), [status](#auth-status)
- [task](#task) — [create](#task-create), [view](#task-view), [list](#task-list), [edit](#task-edit), [move](#task-move), [delete](#task-delete), [grab](#task-grab), [finish](#task-finish)
- [attach](#attach) — [add](#attach-add), [list](#attach-list), [download](#attach-download), [delete](#attach-delete)
- [comment](#comment) — [add](#comment-add), [list](#comment-list), [edit](#comment-edit), [delete](#comment-delete)
- [subtask](#subtask) — [add](#subtask-add), [list](#subtask-list), [check](#subtask-check), [uncheck](#subtask-uncheck)
- [label](#label) — [list](#label-list)
- [board](#board)

Canonical states everywhere: `todo`, `wip`, `review`, `done`, `archive`.
Card colors everywhere: `yellow`, `white`, `red`, `green`, `blue`, `purple`, `orange`, `cyan`,
`brown`, `magenta`.

---

## init

Set up this repo: verify the token, map columns to canonical states, write `.mpx/kanbanflow.json`.
`--json`: yes. Guarded: no.

Token order: `KANBANFLOW_TOKEN`, `--token`/`--token-stdin`, the stored token of the board this repo
is already wired to, the stored token of a logged-in board (`--board`, or a question when several
are logged in), then a hidden prompt. A token is pasted once per board, not once per repo.

```
kf init [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--token` | `TOKEN` | Use this API token. Lands in shell history — prefer `--token-stdin`. | stored token, else prompt |
| `--token-stdin` | — | Read the token as a single line from stdin. | off |
| `--board` | `BOARD` | Use the stored token of this board (ID, name, or 1-based index from `kf auth status`). Skips the question when several boards are logged in. | one board: it; several: asks |
| `--no-store` | — | Do not save the token in the OS credential store. | stores |
| `--map` | `STATE=COLUMN` | Map a canonical state to a column; repeatable. Any use switches mapping to non-interactive mode and leaves unlisted states unmapped. `COLUMN` may be a name, a `uniqueId`, or a 1-based index. | interactive |
| `--user` | `USER` | User ID, full name, or email the token acts as. | interactive |
| `--overwrite` | — | Overwrite an existing `.mpx/kanbanflow.json`. Alias `--force`. | refuses |
| `--json` | — | Print the written configuration as JSON. | human |

```bash
kf init --map todo=To-do --map wip="In progress" --map done=Done --user U9kJ2b --json
```

---

## auth

### auth login

Store an API token for the board it belongs to, and record that board so `kf init` can reuse the
token in any repo. Run once per board. `--json`: no. Guarded: no.

```
kf auth login [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--token` | `TOKEN` | The API token, visible in shell history. | prompt |
| `--token-stdin` | — | Read the token as a single line from stdin. | off |
| `--with-token-file` | `PATH` | Read the token from this file instead of prompting. | off |

```bash
printf '%s' "$KANBANFLOW_TOKEN" | kf auth login --token-stdin
```

### auth logout

Delete a board's stored token and drop it from the board registry. `.mpx/kanbanflow.json` is left
alone, so `kf auth login` restores access. `--json`: no. Guarded: no.

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

Show which token source this directory would use, and which boards are logged in. Makes no HTTP
request. `--json`: yes (adds `knownBoards`). Guarded: no.

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

## task

### task create

Create a task, optionally uploading attachments in the same step. `--json`: yes. Guarded: no.

```
kf task create --name <NAME> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--name` | `NAME` | Task name. **Required.** | — |
| `--description` | `TEXT` | Task description, Markdown as the board renders it. | empty |
| `--color` | color | Card color. | board default |
| `--to` | state | Canonical state (column) to create the task in. | `todo` |
| `--label` | `NAME` | Existing board label to apply; repeatable. Unknown labels are refused. | none |
| `--responsible` | `me` \| user ID \| `none` | Responsible user. **Defaults to you**; pass `none` to leave the task unassigned. | you |
| `--attach` | `FILE` | File to upload onto the new task; repeatable. | none |
| `--grouping-date` | `YYYY-MM-DD` | Grouping date for date-grouped columns. | server: today |
| `--json` | — | Print JSON instead of the human-readable output. | human |

```bash
kf task create --name "Fix login redirect" --to wip --label bug --attach ./shot.png --json
```

### task view

Show a task with its comments and attachments in one aggregated view. `--json`: yes. Guarded: no.

```
kf task view <TASK> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--download-attachments` | `DIR` | Save every attachment into this directory. Links expire, so do it now. | off |
| `--json` | — | Print JSON instead of the human-readable output. | human |

```bash
kf task view E613 --download-attachments ./attachments
```

### task list

List tasks on the board. `--json`: yes. Guarded: no.

```
kf task list [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--state` | state | Only tasks in this canonical state; repeatable, and several states select the union of their columns. | all |
| `--column` | `NAME_OR_ID` | Only tasks in this column, by name or ID — for columns with no canonical state. | all |
| `--open` | — | Only unfinished work: every column except those mapped to `done` and `archive`. | all |
| `--mine` | — | Only tasks you are responsible for **or** a collaborator on. | all |
| `--json` | — | Print JSON instead of the human-readable output. | human |

`--state`, `--column` and `--open` are mutually exclusive.

State names are matched case-insensitively. A state with no column mapped in
`.mpx/kanbanflow.json` is an error (exit 7), not an empty result.

`--open` filters by **exclusion**, which is what makes it board-agnostic: it drops the `done`
and `archive` columns and keeps everything else, including lanes the board invented that map to no
canonical state (a "Do today", a "Blocked"). Listing the open states by hand instead would silently
miss those. It needs at least one of `done` / `archive` mapped — with neither, there is nowhere for
work to end and the command exits 7.

`--mine` follows the board's own reading of "assigned to me": KanbanFlow draws your avatar on a
card whether you are its responsible user or one of its collaborators, and teams that assign work
by adding collaborators would otherwise see nothing. The `PEOPLE` column shows the responsible
user first, then each collaborator prefixed with `+` — so `+me` is a task you collaborate on but
are not responsible for. A `+me` task is yours to mutate as well; only `kf task grab` still judges
by the responsible user alone.

The `STATE` column prints the canonical state, or the column's name when it has none.

```bash
kf task list --open --mine                     # everything of mine that is not finished
kf task list --state todo --state wip --mine   # the same, restricted to two named states
kf task list --state wip --mine
```

### task edit

Change a task's fields. `--json`: yes. **Guarded** (`--force`).

```
kf task edit <TASK> [OPTIONS]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--name` | `NAME` | Replace the task name. |
| `--description` | `TEXT` | Replace the description. |
| `--append-description` | `TEXT` | Append to the description after a blank line (lossless merge). |
| `--color` | color | Card color. |
| `--add-label` | `NAME` | Add an existing board label; repeatable. |
| `--remove-label` | `NAME` | Remove a label; repeatable. |
| `--responsible` | `me` \| user ID \| `none` | Set or clear the responsible user. |
| `--force` | — | Mutate a task that is not yours. |
| `--json` | — | Print JSON instead of the human-readable output. |

```bash
kf task edit E613 --append-description "Repro on Safari 17." --add-label bug
```

### task move

Move a task to a canonical workflow state. `--json`: no. **Guarded** (`--force`).

```
kf task move <TASK> --to <TO> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--to` | state | Target canonical state. **Required.** | — |
| `--grouping-date` | `YYYY-MM-DD` | Grouping date when the target column is date grouped. | server: today UTC |
| `--force` | — | Mutate a task that is not yours. | off |

```bash
kf task move E613 --to done --grouping-date 2026-07-31
```

### task delete

Delete a task. `--json`: no. **Guarded** (`--force`).

```
kf task delete <TASK> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--yes` | — | Skip the confirmation prompt (required when stdin is not a terminal). | prompts |
| `--force` | — | Delete a task that is not yours. | off |

```bash
kf task delete E613 --yes
```

### task grab

Assign the task to yourself, move it to `wip` and print the aggregated view — one call instead of
three. `--json`: yes. **Guarded** (`--force`).

```
kf task grab <TASK> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--download-dir` | `DIR` | Save the task's image attachments into this directory. | off |
| `--force` | — | Grab a task someone else is responsible for. | off |
| `--json` | — | Print JSON instead of the human-readable output. | human |

Grabbing rewrites the responsible user, so it is the one guarded command judged on that field
alone: collaborating on a teammate's task does not let you pull their name off it without
`--force`. An unassigned task is fair game.

```bash
kf task grab E613 --download-dir ./attachments --json
```

### task finish

Post a closing comment, optionally check off every subtask, and move the task on. `--json`: no.
**Guarded** (`--force`).

```
kf task finish <TASK> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--comment-file` | `PATH` | File whose contents become the closing comment. | no comment |
| `--to` | state | Target canonical state. | `done` |
| `--check-subtasks` | — | Mark every unfinished subtask finished. | off |
| `--force` | — | Finish a task that is not yours. | off |

```bash
kf task finish E613 --comment-file ./summary.md --check-subtasks
```

---

## attach

### attach add

Upload one or more files to a task. `--json`: no. **Guarded** (`--force`).

```
kf attach add <TASK> <FILES>...
```

| Argument / flag | Value | Effect |
| --- | --- | --- |
| `<FILES>...` | paths | Files to upload. At least one required. |
| `--force` | — | Upload even when the task belongs to someone else. |

```bash
kf attach add E613 ./before.png ./after.png
```

### attach list

List a task's attachments. `--json`: yes. Guarded: no.

```
kf attach list <TASK> [--json]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--json` | — | Print JSON instead of the human-readable output. |

```bash
kf attach list E613 --json
```

### attach download

Download a task's attachments before their links expire. `--json`: yes. Guarded: no.

```
kf attach download <TASK> [OPTIONS]
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

Remove an attachment from a task. `--json`: no. **Guarded** (`--force`).

```
kf attach delete <TASK> --name <NAME> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--name` | `NAME` | Exact name of the attachment to remove. **Required.** | — |
| `--yes` | — | Skip the confirmation prompt. | prompts |
| `--force` | — | Delete even when the task belongs to someone else. | off |

```bash
kf attach delete E613 --name shot.png --yes
```

---

## comment

### comment add

Add a comment to a task. `--json`: no. **Guardrail-exempt** — commenting on a teammate's task never
needs `--force`, and no `--force` flag exists. Exactly one of `--text` / `--file` is required.

```
kf comment add <TASK> <--text <TEXT>|--file <FILE>>
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--text` | `TEXT` | Comment text. |
| `--file` | `FILE` | Read the comment text from a file. |

```bash
kf comment add E613 --text "Deployed to staging, awaiting QA."
```

### comment list

List a task's comments with their IDs. `--json`: yes. Guarded: no.

```
kf comment list <TASK> [--json]
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
kf comment edit <TASK> --id <ID> <--text <TEXT>|--file <FILE>> [--force]
```

| Flag | Value | Effect |
| --- | --- | --- |
| `--id` | `ID` | ID of the comment to change; see `kf comment list`. **Required.** |
| `--text` | `TEXT` | New comment text. |
| `--file` | `FILE` | Read the new comment text from a file. |
| `--force` | — | Edit even when the task belongs to someone else. |

```bash
kf comment edit E613 --id cM4kQ2 --text "Corrected: staging, not production."
```

### comment delete

Delete a comment. `--json`: no. **Guarded** (`--force`).

```
kf comment delete <TASK> --id <ID> [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--id` | `ID` | ID of the comment to delete; see `kf comment list`. **Required.** | — |
| `--yes` | — | Skip the confirmation prompt. | prompts |
| `--force` | — | Delete even when the task belongs to someone else. | off |

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
kf subtask add <TASK> <NAME> [--force]
```

| Argument / flag | Value | Effect |
| --- | --- | --- |
| `<NAME>` | text | Checklist item name. **Required.** |
| `--force` | — | Add even when the task belongs to someone else. |

```bash
kf subtask add E613 "Write regression test"
```

### subtask list

List a task's checklist items with 1-based positions. `--json`: yes. Guarded: no.

```
kf subtask list <TASK> [--json]
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
kf subtask check <TASK> <SUBTASK> [--force]
```

| Argument / flag | Value | Effect |
| --- | --- | --- |
| `<SUBTASK>` | name \| position | Exact checklist item name, or its 1-based position. **Required.** |
| `--force` | — | Change even when the task belongs to someone else. |

```bash
kf subtask check E613 2
```

### subtask uncheck

Mark a checklist item unfinished. `--json`: no. **Guarded** (`--force`).

```
kf subtask uncheck <TASK> <SUBTASK> [--force]
```

| Argument / flag | Value | Effect |
| --- | --- | --- |
| `<SUBTASK>` | name \| position | Exact checklist item name, or its 1-based position. **Required.** |
| `--force` | — | Change even when the task belongs to someone else. |

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

Show the board's columns and the canonical-state mapping from `.mpx/kanbanflow.json`.
`--json`: yes. Guarded: no.

```
kf board [OPTIONS]
```

| Flag | Value | Effect | Default |
| --- | --- | --- | --- |
| `--counts` | — | Also show how many tasks sit in each column; costs one extra request. | off |
| `--json` | — | Print the raw `GET /board` response. | human |

```bash
kf board --counts
```
