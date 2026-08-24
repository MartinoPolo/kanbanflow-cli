---
name: issue-view
description: "Reads a KanbanFlow issue in full — description, subtasks, comments and attached images — before working on it."
when_to_use: "User references an issue number (E613), asks what an issue says, or asks you to start work on a KanbanFlow issue and you need its content first."
argument-hint: "<issue-number>"
allowed-tools: Bash(kf issue view*), Bash(kf issue list*), Bash(kf attach download*), Bash(kf comment list*), Bash(kf subtask list*), Read
metadata:
  author: MartinoPolo
  version: "1.0"
  category: ticketing
---

# View KanbanFlow Issue

Load everything an issue says — including its images — in as few API calls as possible.
$ARGUMENTS

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` now — issue references, exit codes, rate
budget, token rules.

## Step 1: Read the issue once

```bash
kf issue view E613 --download-attachments .mpx/tmp/E613 --json
```

This is the **aggregate**: issue + comments + attachments in one command, plus every
attachment saved to disk. It costs 3–4 API calls, so **run it once per issue per session** and
reuse the JSON. Never poll for changes.

- `--download-attachments <dir>` is not optional when the issue might have screenshots:
  attachment links are pre-signed and expire in ~24h, so there is no "download later".
- Use a scratch directory (`.mpx/tmp/<ref>/`), never the repo tree.
- Omit `--download-attachments` only when a prior aggregate already showed zero attachments.

If you need to find the reference first: `kf issue list --open --mine --json` for everything of
yours still open, or `--state wip --mine --json` for one state (`--state` repeats). That is a
full board scan — do it once, not per issue.

## Step 2: Read the images

Every saved path is printed (human mode) or derivable from the `attachments` array. Open each
image with the Read tool. Screenshots usually carry the actual bug — treat them as primary
content, not decoration.

## Step 3: Interpret the JSON

| Field | Read it as |
| --- | --- |
| `issue.number` | The human reference, `{prefix, value}` → `E613`. |
| `issue.columnId` | Workflow state. Map through `mpxconfig.json`; human output prints the canonical state directly. |
| `issue.responsibleUserId` | Assignee. Neither this nor `collaborators[]` is your user ID → the issue is a teammate's; mutations will be refused (exit 3). |
| `issue.collaborators[].userId` | Extra people. Your ID here makes the issue yours to mutate, but `kf issue grab` still needs `--force` to take it off the responsible user. |
| `issue.color` | `red` = bug/critical, `green` = normal (work convention). |
| `issue.labels[].name` | Area/project. Tells you which part of the codebase to look at. |
| `issue.subTasks[]` | Checklist, `{name, finished}`, addressed by 1-based position. Unfinished items are the remaining work. |
| `comments[]` | Chronological discussion; the newest often supersedes the description. |
| `attachments[]` | `name`, `mimeType`, `size`. `link` is expiring — never store or re-use it. |

## Step 4: Report

Summarize before touching code:

- `E613 — <name>` · state · color meaning · labels · responsible.
- What the issue asks for, in your own words, one short paragraph.
- Unfinished subtasks as a checklist.
- What each image shows.
- Anything the comments changed relative to the description.
- Open questions or contradictions → surface them, do not guess.

If the human wants to start work now, hand off to `/kf:issue-edit` (which uses
`kf issue grab` — that command re-reads the aggregate itself, so skip Step 1 in that case).
