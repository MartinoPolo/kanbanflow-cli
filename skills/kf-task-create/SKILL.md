---
name: kf-task-create
description: "Creates a KanbanFlow task from the current work context with the right color, labels and file attachments."
when_to_use: "User asks to create, file, or open a KanbanFlow task/ticket/card, or to turn a bug or finding from the current session into a task."
argument-hint: "<what the task is about> [--label <name>] [--attach <file>]"
allowed-tools: Bash(kf task create*), Bash(kf task view*), Bash(kf label list*), Bash(kf attach add*), Bash(kf subtask add*), Read, Glob, Grep
metadata:
  author: MartinoPolo
  version: "1.0"
  category: ticketing
---

# Create KanbanFlow Task

Turn the current work context into one well-formed task on the board. $ARGUMENTS

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` now — task references, exit codes, rate
budget, token rules.

## Step 1: Compose the content

From `$ARGUMENTS` and the session context (failing test, error output, code you just read,
the human's description):

- **Name** — one imperative line, under ~70 characters. No ticket prefix, no trailing period.
- **Description** — Markdown, as the board renders it. Include only what a future reader
  needs: what is wrong or wanted, where in the code (`path:line`), how to reproduce, what
  "done" looks like. No session narration, no restating the name.

Gather evidence files while you are here: screenshots, logs, diffs already on disk.
Do not invent paths — verify each one exists before passing it to `--attach`.

## Step 2: Choose color and state

| Situation | Flag |
| --- | --- |
| Bug, regression, crash, security, anything critical | `--color red` |
| Normal work: feature, chore, refactor, docs | `--color green` |

Default target column is `todo`. Pass `--to wip` only when the human is starting the work
immediately.

## Step 3: Create it

Labels are validated against the board and **never created implicitly**. `kf label list`
costs a full board scan — do not pre-list. Pass the label you believe in and let `create`
validate:

```bash
kf task create \
  --name "Fix expired session redirect loop" \
  --description "$(cat notes.md)" \
  --color red \
  --to todo \
  --label "Yoursafe Components" \
  --attach ./screenshots/loop.png \
  --json
```

Flags: `--name` (required), `--description`, `--color`, `--to <state>`, `--label <name>`
(repeatable), `--responsible me|<userId>|none` (defaults to you; `none` leaves the task
unassigned), `--attach <file>` (repeatable),
`--grouping-date YYYY-MM-DD`, `--json`.

**On an unknown-label error** (exit 1): the message lists every label on the board. Pick the
closest existing one and re-run, or drop `--label` entirely. Never ask for a label to be
created — that is a human action in the KanbanFlow UI.

## Step 4: Attachments and checklist

- `--attach` uploads happen after the task exists. An upload failure is reported per file and
  is **not** fatal — check the `attachments` array in the JSON and retry only the failed file
  with `kf attach add <ref> <file>`.
- Add checklist items only when the human framed the work as steps:
  `kf subtask add <ref> "Reproduce on staging"`.

## Output

Report, in one block:

- Task number (`E613`) — from the `number` field of the `--json` output.
- Name, color, state, labels applied.
- Attachments uploaded, and any that failed with the reason.

Do not re-read the task to confirm; the create response already carries the number.
