---
name: issue-grill
description: "Resolves a HITL KanbanFlow issue into an AFK-ready one: extracts the open decisions, grills the human, records the answers on the issue and applies the AFK label."
when_to_use: "User asks to grill, groom, or resolve a KanbanFlow issue, or an execution skill stopped on an issue without the AFK label and the user chose to grill it."
argument-hint: "<issue-number> [<issue-number> ...]"
allowed-tools: Bash(kf issue view*), Bash(kf issue edit*), Bash(kf comment add*), Read, Agent
metadata:
  author: MartinoPolo
  version: "0.1"
  category: ticketing
---

# Grill KanbanFlow Issue

Turn an underspecified issue into one an agent can execute alone. $ARGUMENTS

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` and
`${CLAUDE_SKILL_DIR}/../shared/KF_WORKFLOW.md` now.

## Step 1: Read the issue

Reuse the aggregate JSON if this session already viewed the issue; otherwise:

```bash
kf issue view E613 --download-attachments .mpx/tmp/E613 --json
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

## Step 4: Record on the issue

One `kf issue edit` call, fields batched:

```bash
kf issue edit E613 \
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
- `AFK` refused as unknown label → tell the human to add it to any issue via the
  KanbanFlow UI, then re-run the edit.

## Output

Per issue: the decisions settled, the questions still open, and whether `AFK` was applied.
Multiple issue numbers are processed one at a time, Steps 1–4 each.
