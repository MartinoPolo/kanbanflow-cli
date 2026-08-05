---
name: kf-task-edit
description: "Updates a KanbanFlow task during and after work: grab, append notes, comment, move between states, finish."
when_to_use: "User starts work on a task, wants to record progress on it, opens or merges an MR for it, or asks to move, reassign, relabel or close a KanbanFlow task."
argument-hint: "<task-number> [grab|note|comment|move <state>|finish]"
allowed-tools: Bash(kf task grab*), Bash(kf task finish*), Bash(kf task edit*), Bash(kf task move*), Bash(kf comment add*), Bash(kf subtask*), Bash(kf attach add*), Read, Write
metadata:
  author: MartinoPolo
  version: "1.0"
  category: ticketing
---

# Edit KanbanFlow Task

Keep the board in sync with the work as it happens. $ARGUMENTS

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` now — task references, exit codes, rate
budget, token rules.

## Ownership guardrail — read before any mutation

Every mutating command refuses a task that is not yours and exits **3**. Yours means you are its
responsible user **or** one of its collaborators.

- Exit 3 → **stop and ask the human.** Report whose task it is.
- Never add `--force` on your own initiative. Use it only in the same turn the human
  explicitly said to change a teammate's task.
- Adding a comment is exempt: commenting on anyone's task is legitimate collaboration.

## Auto-move triggers

Fire these from the host repo's `glab` workflow. One command per event, no polling.

| Event | Command |
| --- | --- |
| Work starts on the task | `kf task grab <ref> --download-dir .mpx/tmp/<ref> --json` |
| Merge request opened | `kf task move <ref> --to review` |
| Merge request merged | `kf task finish <ref> --comment-file <file> --check-subtasks` |
| Work parked / handed back | `kf task move <ref> --to todo` and `kf task edit <ref> --responsible none` |

## Start of work: grab

```bash
kf task grab E613 --download-dir .mpx/tmp/E613 --json
```

One command: assigns the task to you, moves it to `wip`, returns the full aggregate
(task + comments + attachments) and saves the **image** attachments. Read those images.
Do not run `kf task view` afterwards — grab already paid for it.

Unassigned tasks grab freely; a teammate's task needs `--force`, so exit 3 here means ask.

## During work: notes and comments

- **Short factual note, lossless** — appends after a blank line, never overwrites:
  ```bash
  kf task edit E613 --append-description "Root cause: session cookie dropped on 302 in auth/middleware.ts:88"
  ```
- **Longer body** (findings, reproduction, decisions) — write a file first, then:
  ```bash
  kf comment add E613 --file .mpx/tmp/E613/note.md
  ```
  `--text` exists for one-liners; prefer `--file` for anything multi-line.
- **New evidence**: `kf attach add E613 ./screenshots/after-fix.png`
- **Checklist**: `kf subtask check E613 "Reproduce on staging"` (exact name or 1-based
  position); `kf subtask add E613 "<step>"` when the work grew.
- **Metadata**: `kf task edit E613 --name`, `--description` (replaces — prefer
  `--append-description`), `--color red|green`, `--add-label <name>`, `--remove-label <name>`,
  `--responsible me|<userId>|none`. Labels must already exist on the board; an unknown name is
  refused with the full list.

Batch changes into a single `kf task edit` call rather than one call per field.

## Moving states

```bash
kf task move E613 --to review
```

States: `todo`, `wip`, `review`, `done`, `archive`. `--grouping-date YYYY-MM-DD` only when the
target column is date-grouped and the human wants a date other than today.

## End of work: finish

```bash
kf task finish E613 --comment-file .mpx/tmp/E613/summary.md --check-subtasks
```

Comments from the file, optionally checks off every unfinished subtask, then moves to `done`
(override with `--to review|archive`).

Write the summary file first. Keep it to what a reviewer needs: what changed, where
(`path:line`), how it was verified, the MR reference. No session narration.

If `--check-subtasks` would tick items that are genuinely not done, omit the flag and say so.

## Output

Report the task number, the state it now sits in, and each change applied (note appended,
comment added, subtasks checked, labels changed, files attached). On exit 3, report the owner
and stop.
