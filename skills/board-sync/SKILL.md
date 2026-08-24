---
name: board-sync
description: "Reconciles the KanbanFlow board with where the work actually is: derives each open issue's expected column from its merge request state and local branches, then moves drifted issues forward."
argument-hint: "[<issue-number>|<range> ...] [--report-only]"
disable-model-invocation: true
allowed-tools: Bash(kf issue list*), Bash(kf issue move*), Bash(git branch*), Bash(git worktree list*), Bash(glab mr list*), Read
metadata:
  author: MartinoPolo
  version: "0.1"
  category: project-management
---

# Board Sync

Project the human's VCS decisions onto the board. $ARGUMENTS

Once work leaves the editor, the VCS is the source of truth: the human marks the MR
ready and merges it there, and this skill moves the matching issues to where those
decisions already put them. It moves issues **forward only** (todo → wip → review →
done), so a manual board move ahead of the evidence is never undone, and it never
touches `archive` or demotes anything.

## Step 0: Load the contract

Read `${CLAUDE_SKILL_DIR}/../shared/KF_BASICS.md` now — exit codes, rate budget,
guardrail, issue references.

## Step 1: Select issues

- Default: `kf issue list --open --mine --json` — one board scan, reused for the whole
  run.
- Explicit issue numbers in `$ARGUMENTS` replace the default selection; a range like
  `E610-E615` expands numerically over the same prefix.

## Step 2: Resolve the VCS provider

The committed `repository.provider` key in `mpxconfig.json` names the provider.
Read the matching provider file now —
`${CLAUDE_SKILL_DIR}/providers/GITLAB.md` — and use its commands in Step 3. An unknown
value → stop and report it; supporting a new provider means adding
`providers/<NAME>.md` implementing the evidence contract below.

**Evidence contract** — for each selected issue's ticket number, the provider reports
one of:

| Verdict     | Meaning                                                |
| ----------- | ------------------------------------------------------ |
| `merged`    | an MR for this ticket was merged                       |
| `ready`     | an MR is open and not a draft                          |
| `draft`     | an MR is open and still a draft                        |
| `none`      | no open or merged MR mentions the ticket               |
| `ambiguous` | several distinct open MRs match — skip the issue, report it |

plus the MR URL for everything except `none` and `ambiguous`. A closed-without-merge
MR is `none`.

## Step 3: Collect evidence

1. **VCS** — run the provider file's listing commands once for the whole run and
   match every selected ticket against that same result set.
2. **Local** — one pass, matched case-insensitively against the lowercased tickets
   (branches spell `e613`, the board spells `E613`):

   ```bash
   git branch --all --list
   git worktree list
   ```

   A branch or worktree whose name contains the ticket is evidence that work started.

## Step 4: Derive the expected state

Strongest evidence wins:

| Evidence                                      | Expected state    |
| --------------------------------------------- | ----------------- |
| MR `merged`                                   | `done`            |
| MR `ready`                                    | `review`          |
| MR `draft`                                    | `wip`             |
| No MR, but a local branch/worktree with the ticket | `wip`        |
| No trace anywhere                             | current (no move) |

**Forward-only rule**: rank `todo` < `wip` < `review` < `done`. Move an issue only when
its expected state outranks its current one. An issue in a column with no canonical
state (a board's own "Do today") ranks as `todo` for this comparison — flag it in the
report so the human sees the skill's reading.

## Step 5: Apply

Skip this step entirely when `--report-only` is in `$ARGUMENTS`.

For each issue whose expected state outranks its current one:

```bash
kf issue move "$ISSUE_ID" --to review
```

Use the selected issue's raw `_id` as `ISSUE_ID`; using its number would trigger another
board scan for number resolution and make the rate accounting inaccurate. The server supplies
the default grouping date for date-grouped columns (a work-board "Done" is one). Exit 3 →
skip the issue and report it; never retry with `--force`.

## Step 6: Report

One table, then one line of totals:

| Issue | Name | Column → Expected | Evidence | Action |
| ---- | ---- | ----------------- | -------- | ------ |

- **Evidence** — the MR URL, or the branch/worktree name for local-only evidence.
- **Action** — `moved`, `in sync`, or `skipped: <reason>` (ambiguous MRs, guardrail,
  unmapped column).

Totals: how many in sync, moved, skipped. Rate cost of the run: one board scan plus
one move per drifted issue; provider calls are outside the KanbanFlow budget.
