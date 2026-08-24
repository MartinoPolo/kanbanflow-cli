---
name: batch-execute
description: "Executes a batch of AFK-labeled KanbanFlow issues sequentially, each in its own worktree with its own branch and draft GitLab MR. Reviewing and merging every MR stays with the human."
argument-hint: "[<issue-number> ...] [--full-review]"
disable-model-invocation: true
allowed-tools: Bash(kf *), Bash(git *), Bash(glab *), Bash(yarn *), Read, Write, Agent, AskUserQuestion
metadata:
  author: MartinoPolo
  version: "0.1"
  category: execution
---

# Batch-Execute KanbanFlow Issues

Work through several issues in one session — each one fully isolated: own worktree, own
branch, own draft MR, exactly as if executed alone. $ARGUMENTS

The main conversation is a pure orchestrator; per-issue implementation runs in sub-agents
that return bounded results. If a named `mp-*` agent type is not available in this
session, do that phase in the main thread instead.

## Step 0: Load the contracts

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` and
`${CLAUDE_SKILL_DIR}/../shared/KF_WORKFLOW.md` now.

## Step 1: Select the issues

- **No issue numbers given** — the default selection from KF_WORKFLOW § Who automation may
  touch: `kf issue list --state todo --state wip --mine --json`, filtered to issues with the
  `AFK` label whose `responsibleUserId` is you (`kf auth status --json` → `.userId`) or
  empty. One board scan, reused for the whole batch.
- **Explicit numbers** — take them as given; a range like `E610-E615` expands
  numerically over the same prefix.

Order: `wip` before `todo` (finish started work first), then board order.

## Step 2: AFK gate

A selected issue without the `AFK` label stops the batch **before any implementation**:
ask via `AskUserQuestion` — skip it, grill it now (`/kf:issue-grill`), or include it anyway
on the human's explicit say-so. The default selection never trips this; only explicit
numbers can.

## Step 3: Execute each issue, sequentially

Run once at the start: `git fetch origin main`. Then per issue:

1. **Read** — `kf issue view <ref> --download-attachments .mpx/tmp/<ref> --json`; read the
   images.
2. **Worktree** — per KF_WORKFLOW § Branch and worktree: reuse or create
   `../<repo-name>.worktrees/<ticket>-<slug>` on `<author>/<ticket>-<slug>`.
3. **Grab** — `kf issue grab <ref> --json`. Exit 3 → skip the issue, note it for the report.
4. **Implement** — spawn a `claude` sub-agent with `model: "opus"`: pass the issue JSON
   content, the worktree path, AGENTS.md § Definition of done, and an instruction to commit
   conventionally per the `/mp:commit` skill (from the mp plugin loaded alongside). It
   implements test-first, gets `yarn lint` and `yarn test` (or the host repo's equivalent
   check scripts) green, commits conventionally in the worktree, and returns a bounded
   summary — the sub-agent cannot spawn agents, so it works directly and leaves push, MR
   and board updates to the orchestrator.
5. **Review** — spawn `mp:mp-check-fixer` on the worktree (default reviewer set;
   `--full-review` in `$ARGUMENTS` adds security, performance, error-handling).
6. **Draft MR** — follow the `mr` skill (`${CLAUDE_SKILL_DIR}/../mr/SKILL.md`); the issue
   stays in `wip`; then `kf comment add <ref> --text "MR: <url>"`.
7. **Pipeline** — per KF_WORKFLOW § Pipeline watch, at most 3 fix attempts via
   `mp:mp-executor` with a bounded scope. Environmental e2e failures → mark the issue's row
   as needing the human and move on.
8. **Report** — keep to gate results in one line, an unresolved
   blocker or human action, a fact that changes how to review the diff. Cut anything that
   serves none of those, then attach it to the MR (`glab mr note <mr-number> --message
   "$(cat <file>)"`).

An issue that cannot be finished (fix attempts exhausted, blocked, guardrail) is **parked**:
`kf comment add <ref> --text "<one line: where it stopped and why>"`, worktree left in
place, batch continues with the next issue.

## Step 4: Report

One table: issue · branch · worktree · MR URL · pipeline state · status
(draft-MR-ready / parked / skipped). Then what remains for the human, per KF_WORKFLOW:
review each MR, move issues to `review`, mark ready, merge, `kf issue finish`.
