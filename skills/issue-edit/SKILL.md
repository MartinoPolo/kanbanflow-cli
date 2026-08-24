---
name: issue-edit
description: "Updates a KanbanFlow issue during and after work: grab, append notes, comment, move between states, finish."
when_to_use: "User starts work on an issue, wants to record progress on it, opens or merges an MR for it, or asks to move, reassign, relabel or close a KanbanFlow issue."
argument-hint: "<issue-number> [grab|note|comment|move <state>|finish]"
allowed-tools: Bash(kf issue grab*), Bash(kf issue finish*), Bash(kf issue edit*), Bash(kf issue move*), Bash(kf comment add*), Bash(kf subtask*), Bash(kf attach add*), Read, Write
metadata:
  author: MartinoPolo
  version: "1.1"
  category: ticketing
---

# Edit KanbanFlow Issue

Keep the board in sync with the work as it happens. $ARGUMENTS

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` now — issue references, exit codes, rate
budget, token rules.

## Ownership guardrail — read before any mutation

Every mutating command refuses an issue that is not yours and exits **3**. Yours means you are its
responsible user **or** one of its collaborators.

- Exit 3 → **stop and ask the human.** Report whose issue it is.
- Never add `--force` on your own initiative. Use it only in the same turn the human
  explicitly said to change a teammate's issue.
- Adding a comment is exempt: commenting on anyone's issue is legitimate collaboration.

## Auto-move triggers

Fire these from the host repo's `glab` workflow. One command per event, no polling.

| Event | Command |
| --- | --- |
| Work starts on the issue | `kf issue grab <ref> --download-dir .mpx/tmp/<ref> --json` |
| Merge request opened | `kf comment add <ref> --text "MR: <url>"` — the issue **stays in `wip`**; the human moves it to `review` after checking the work ([KF_WORKFLOW.md](../shared/KF_WORKFLOW.md)) |
| Merge request merged — only when the human says so | `kf issue finish <ref> --comment-file <file> --check-subtasks` |
| Work parked / handed back | `kf issue move <ref> --to todo` and `kf issue edit <ref> --responsible none` |

## Start of work: grab

```bash
kf issue grab E613 --download-dir .mpx/tmp/E613 --json
```

One command: assigns the issue to you, moves it to `wip`, returns the full aggregate
(issue + comments + attachments) and saves the **image** attachments. Read those images.
Do not run `kf issue view` afterwards — grab already paid for it.

Unassigned issues grab freely; a teammate's issue needs `--force`, so exit 3 here means ask.

## During work: notes and comments

- **Short factual note, lossless** — appends after a blank line, never overwrites:
  ```bash
  kf issue edit E613 --append-description "Root cause: session cookie dropped on 302 in auth/middleware.ts:88"
  ```
- **Longer body** (findings, reproduction, decisions) — write a file first, then:
  ```bash
  kf comment add E613 --file .mpx/tmp/E613/note.md
  ```
  `--text` exists for one-liners; prefer `--file` for anything multi-line.
- **New evidence**: `kf attach add E613 ./screenshots/after-fix.png`
- **Checklist**: `kf subtask check E613 "Reproduce on staging"` (exact name or 1-based
  position); `kf subtask add E613 "<step>"` when the work grew.
- **Metadata**: `kf issue edit E613 --name`, `--description` (replaces — prefer
  `--append-description`), `--color red|green`, `--add-label <name>`, `--remove-label <name>`,
  `--responsible me|<userId>|none`. Labels must already exist on the board; an unknown name is
  refused with the full list.

Batch changes into a single `kf issue edit` call rather than one call per field.

## Moving states

```bash
kf issue move E613 --to review
```

States: `todo`, `wip`, `review`, `done`, `archive`. `--grouping-date YYYY-MM-DD` only when the
target column is date-grouped and the human wants a date other than today.

## End of work: finish

```bash
kf issue finish E613 --comment-file .mpx/tmp/E613/summary.md --check-subtasks
```

Comments from the file, optionally checks off every unfinished subtask, then moves to `done`
(override with `--to review|archive`).

Write the summary file first. Keep it to what a reviewer needs: what changed, where
(`path:line`), how it was verified, the MR reference. No session narration.

If `--check-subtasks` would tick items that are genuinely not done, omit the flag and say so.

## Output

Report the issue number, the state it now sits in, and each change applied (note appended,
comment added, subtasks checked, labels changed, files attached). On exit 3, report the owner
and stop.
