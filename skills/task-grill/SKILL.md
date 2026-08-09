---
name: task-grill
description: "Resolves a HITL KanbanFlow task into an AFK-ready one: extracts the open decisions, grills the human, records the answers on the task and applies the AFK label."
when_to_use: "User asks to grill, groom, or resolve a KanbanFlow task, or an execution skill stopped on a task without the AFK label and the user chose to grill it."
argument-hint: "<task-number> [<task-number> ...]"
allowed-tools: Bash(kf task view*), Bash(kf task edit*), Bash(kf comment add*), Read, Agent
metadata:
  author: MartinoPolo
  version: "0.1"
  category: ticketing
---

# Grill KanbanFlow Task

Turn an underspecified task into one an agent can execute alone. $ARGUMENTS

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` and
`${CLAUDE_SKILL_DIR}/../shared/KF_WORKFLOW.md` now.

## Step 1: Read the task

Reuse the aggregate JSON if this session already viewed the task; otherwise:

```bash
kf task view E613 --download-attachments .mpx/tmp/E613 --json
```

Read the attached images — screenshots usually carry the actual requirement.

## Step 2: Extract the decision points

Collect everything that would force an executing agent to guess:

- The `> **Unanswered questions:**` blockquote, when present.
- Vague wording, missing acceptance criteria, unhandled edge cases, open design or
  naming choices, contradictions between description and newer comments.

Answer codebase questions yourself: spawn `Explore` (breadth: medium) with the concrete
questions instead of asking the human what the code already knows.

## Step 3: Grill

Ask the remaining questions directly in the conversation, batched by theme, each with a
recommended answer. Continue until every decision point is settled or the human parks it.

## Step 4: Record on the task

One `kf task edit` call, fields batched:

```bash
kf task edit E613 \
  --append-description "**Resolved decisions:** <one line per decision>" \
  --add-label AFK
```

- Record only what the description did not already settle. Never restate its analysis.
  One line per decision. A discussion too long for the description, or one needing a
  code block, goes to `kf comment add E613 --file <path>` with a one-line pointer
  appended to the description instead.
- If the appended text ends up longer than the original description, something was
  restated — find it and delete it.
- **Partially resolved** — record what settled, keep the remaining questions in the
  blockquote, and leave `AFK` off.
- `AFK` refused as unknown label → tell the human to add it to any task via the
  KanbanFlow UI, then re-run the edit.

## Output

Per task: the decisions settled, the questions still open, and whether `AFK` was applied.
Multiple task numbers are processed one at a time, Steps 1–4 each.
