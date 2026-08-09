---
name: execute
description: "Executes one KanbanFlow task end to end in its own worktree: grab, analyze, TDD implementation, review fixes, conventional commit, draft GitLab MR, pipeline green. Stops at the draft MR — reviewing, merging and finishing the task stay with the human."
argument-hint: "<task-number> [--full-review]"
disable-model-invocation: true
allowed-tools: Bash(kf *), Bash(git *), Bash(glab *), Bash(yarn *), Read, Write, Agent
metadata:
  author: MartinoPolo
  version: "0.1"
  category: execution
---

# Execute KanbanFlow Task

Take one task from the board to a draft MR with a green pipeline. $ARGUMENTS

The main conversation is a pure orchestrator: sub-agents do the heavy phases and return
bounded results. If a named `mp-*` agent type is not available in this session, do that
phase in the main thread instead.

## Step 0: Load the contracts

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` and
`${CLAUDE_SKILL_DIR}/../shared/KF_WORKFLOW.md` now. AGENTS.md § Definition of done is the
completion bar for every implementation step.

## Step 1: Read the task

```bash
kf task view E613 --download-attachments .mpx/tmp/E613 --json
```

Read the attached images. Interpret the JSON per the `/kf:task-view` skill's field table:
unfinished `subTasks[]` are the remaining work, the newest comments supersede the
description.

## Step 2: AFK gate

Per [KF_WORKFLOW.md](../shared/KF_WORKFLOW.md) § AFK convention — the task must carry the
`AFK` label. Without it, stop before touching anything and ask the human: grill it now
(`/kf:task-grill`), proceed on their explicit say-so, or abort. Also stop when
`responsibleUserId` is a teammate (exit 3 territory — never `--force`).

## Step 3: Worktree and branch

Per KF_WORKFLOW § Branch and worktree: reuse the worktree whose directory name contains
the ticket, else create one from `origin/main` on `<author>/<ticket>-<slug>`. **Every
later step runs inside the task's worktree.**

## Step 4: Grab

```bash
kf task grab E613 --json
```

Assigns the task to you and moves it to `wip`. Exit 3 → stop and ask the human.

## Step 5: Analyze

Spawn `mp:mp-issue-analyzer` with: the task's name, description, comments and unfinished
subtasks (from the Step 1 JSON), the worktree path, and a pointer to AGENTS.md. It returns
the fix plan — files to touch, behaviors to implement, test strategy. On external-library
uncertainty, spawn `mp:mp-context7-docs-fetcher` with the specific question.

## Step 6: Implement (TDD)

Spawn `mp:mp-tdd-executor` with the plan's behaviors and the worktree path. Red-green-refactor;
done means `yarn lint` and `yarn test` (or the host repo's equivalent check scripts) pass with
no errors.

## Step 7: Verify and fix

Spawn `mp:mp-check-fixer` on the worktree — static checks plus the default reviewer set.
With `--full-review` in `$ARGUMENTS`, tell it to add the security, performance and
error-handling reviewers.

## Step 8: Commit

Follow the `/mp:commit` skill (from the mp plugin loaded alongside): one conventional
commit, type from the diff, ticket stays out of the message.

## Step 9: Draft MR

Follow the `mr` skill (`${CLAUDE_SKILL_DIR}/../mr/SKILL.md`): pre-MR gate (`yarn format`,
`yarn lint`, `yarn test`), push, title `<TICKET>: <why>`, created **as draft**. The task
stays in `wip` — the human moves it to `review` after checking the work.

## Step 10: Board signal

```bash
kf comment add E613 --text "MR: <url>"
```

## Step 11: Pipeline green

Per KF_WORKFLOW § Pipeline watch: `glab ci status --live` in the worktree. On a failure
caused by the diff: diagnose from `glab ci trace`, spawn `mp:mp-executor` with the bounded
fix, commit per the `/mp:commit` skill, push, re-watch — at most 3 attempts. Environmental
failures (certs, Storybook boot, e2e flake unrelated to the diff) → stop and report.

## Step 12: Report

Keep the report to three things at most:

```
Gates: <lint / test / e2e / CI>

<a blocker or action needing a human>

<a fact that changes how to review the diff>
```

Cut anything that serves none of those, then write it to a temp file and attach it to the
MR:

```bash
glab mr note <mr-number> --message "$(cat <file>)"
```

Then report in the conversation: task, branch, worktree path, MR URL (draft), pipeline
state, and what remains for the human — review, move the task to `review`, mark the MR
ready, merge, `kf task finish`.
