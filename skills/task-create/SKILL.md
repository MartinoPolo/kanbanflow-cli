---
name: task-create
description: "Creates a KanbanFlow task from the current work context with the right color, labels and file attachments, classified AFK or HITL and approved by the human before creation."
when_to_use: "User asks to create, file, or open a KanbanFlow task/ticket/card, or to turn a bug or finding from the current session into a task."
argument-hint: "<what the task is about> [--label <name>] [--attach <file>]"
allowed-tools: Bash(kf task create*), Bash(kf label list*), Bash(kf attach add*), Bash(kf subtask add*), Read, Glob
metadata:
  author: MartinoPolo
  version: "1.1"
  category: ticketing
---

# Create KanbanFlow Task

Turn the current work context into one well-formed task on the board. $ARGUMENTS

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` and
`${CLAUDE_SKILL_DIR}/../shared/KF_WORKFLOW.md` now — task references, exit codes, rate
budget, token rules, and the AFK convention.

## Step 1: Compose the content

From `$ARGUMENTS` and the session context (failing test, error output, code you just read,
the human's description):

- **Name** — one imperative line, under ~70 characters. No ticket prefix, no trailing period.
- **Description** — Markdown, as the board renders it. Include only what a future reader
  needs: what is wrong or wanted, where in the code (`path:line`), how to reproduce, what
  "done" looks like. No session narration, no restating the name.
- **Brevity is a hard requirement**: at most **15 sentences**, and **5–10** for a small
  task. Every sentence must earn its place — cut background the code already tells.

Gather evidence files while you are here: screenshots, logs, diffs already on disk.
Verify each evidence-file path exists on disk (Glob) before passing it to `--attach`.

## Step 2: Classify AFK or HITL

Judge whether the task can be executed autonomously ([KF_WORKFLOW.md](../shared/KF_WORKFLOW.md)
§ AFK convention):

- **AFK** — well-defined scope, clear acceptance criteria, no open design questions.
  Gets `--label AFK`.
- **HITL** — unanswered questions or decisions the human must make. Gets **no** label,
  and the description **starts** with the questions:

  ```markdown
  > **Unanswered questions:**
  > - <specific question>
  ```

When genuinely unsure which it is, put the classification question to the human in Step 4
instead of guessing.

## Step 3: Choose color and state

| Situation | Flag |
| --- | --- |
| Bug, regression, crash, security, anything critical | `--color red` |
| Normal work: feature, chore, refactor, docs | `--color green` |

Default target column is `todo`. Pass `--to wip` only when the human is starting the work
immediately.

## Step 4: Human approval — always

Show the human exactly what will be created before creating it: the full name, the full
description text, color, state, labels (including `AFK` or its absence), attachments and
subtasks. Wait for their approval; apply their edits verbatim. Create nothing they have
not seen.

## Step 5: Create it

Labels are validated against the board and **never created implicitly**. `kf label list`
costs a full board scan, so pass the labels directly and let `create` validate them.
`--label` and `--attach` values given in `$ARGUMENTS` pass straight through to the
matching flags:

```bash
kf task create \
  --name "Fix expired session redirect loop" \
  --description "$(cat notes.md)" \
  --color red \
  --to todo \
  --label AFK \
  --attach ./screenshots/loop.png \
  --json
```

Flags: `--name` (required), `--description`, `--color`, `--to <state>`, `--label <name>`
(repeatable), `--responsible me|<userId>|none` (defaults to you; `none` leaves the task
unassigned), `--attach <file>` (repeatable), `--grouping-date YYYY-MM-DD`, `--json`.

**On an unknown-label error** (exit 1): the message lists every label on the board. For a
project/area label, pick the closest existing one and re-run, or drop it. For `AFK`
specifically, create the task without it and tell the human to add the label via the
KanbanFlow UI — labels only exist while some task carries them. Never ask for any other
label to be created.

## Step 6: Attachments and checklist

- `--attach` uploads happen after the task exists. An upload failure is reported per file and
  is **not** fatal — check the `attachments` array in the JSON and retry only the failed file
  with `kf attach add <ref> <file>`.
- Add checklist items only when the human framed the work as steps:
  `kf subtask add <ref> "Reproduce on staging"`.

## Output

Report, in one block:

- Task number (`E613`) — from the `number` field of the `--json` output.
- Name, color, state, labels applied, and the AFK/HITL classification.
- Attachments uploaded, and any that failed with the reason.

Trust the create response — it already carries the number; a confirming re-read only
spends rate budget.
