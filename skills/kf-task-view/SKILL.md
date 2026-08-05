---
name: kf-task-view
description: "Reads a KanbanFlow task in full — description, subtasks, comments and attached images — before working on it."
when_to_use: "User references a task number (E613), asks what a task says, or asks you to start work on a KanbanFlow task and you need its content first."
argument-hint: "<task-number>"
allowed-tools: Bash(kf task view*), Bash(kf task list*), Bash(kf attach download*), Bash(kf comment list*), Bash(kf subtask list*), Read
metadata:
  author: MartinoPolo
  version: "1.0"
  category: ticketing
---

# View KanbanFlow Task

Load everything a task says — including its images — in as few API calls as possible.
$ARGUMENTS

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` now — task references, exit codes, rate
budget, token rules.

## Step 1: Read the task once

```bash
kf task view E613 --download-attachments .mpx/tmp/E613 --json
```

This is the **aggregate**: task + comments + attachments in one command, plus every
attachment saved to disk. It costs 3–4 API calls, so **run it once per task per session** and
reuse the JSON. Never poll for changes.

- `--download-attachments <dir>` is not optional when the task might have screenshots:
  attachment links are pre-signed and expire in ~24h, so there is no "download later".
- Use a scratch directory (`.mpx/tmp/<ref>/`), never the repo tree.
- Omit `--download-attachments` only when a prior aggregate already showed zero attachments.

If you need to find the reference first: `kf task list --active --mine --json` for everything of
yours still open, or `--state wip --mine --json` for one state (`--state` repeats). That is a
full board scan — do it once, not per task.

## Step 2: Read the images

Every saved path is printed (human mode) or derivable from the `attachments` array. Open each
image with the Read tool. Screenshots usually carry the actual bug — treat them as primary
content, not decoration.

## Step 3: Interpret the JSON

| Field | Read it as |
| --- | --- |
| `task.number` | The human reference, `{prefix, value}` → `E613`. |
| `task.columnId` | Workflow state. Map through `.mpx/kanbanflow.json`; human output prints the canonical state directly. |
| `task.responsibleUserId` | Assignee. Neither this nor `collaborators[]` is your user ID → the task is a teammate's; mutations will be refused (exit 3). |
| `task.collaborators[].userId` | Extra people. Your ID here makes the task yours to mutate, but `kf task grab` still needs `--force` to take it off the responsible user. |
| `task.color` | `red` = bug/critical, `green` = normal (work convention). |
| `task.labels[].name` | Area/project. Tells you which part of the codebase to look at. |
| `task.subTasks[]` | Checklist, `{name, finished}`, addressed by 1-based position. Unfinished items are the remaining work. |
| `comments[]` | Chronological discussion; the newest often supersedes the description. |
| `attachments[]` | `name`, `mimeType`, `size`. `link` is expiring — never store or re-use it. |

## Step 4: Report

Summarize before touching code:

- `E613 — <name>` · state · color meaning · labels · responsible.
- What the task asks for, in your own words, one short paragraph.
- Unfinished subtasks as a checklist.
- What each image shows.
- Anything the comments changed relative to the description.
- Open questions or contradictions → surface them, do not guess.

If the human wants to start work now, hand off to `kf-task-edit` (which uses
`kf task grab` — that command re-reads the aggregate itself, so skip Step 1 in that case).
