# kf Workflow — this repo's execution policy

Work-specific policy shared by `/kf:execute`, `/kf:batch-execute`, `/kf:issue-grill` and
`/kf:issue-create`. [KF_BASICS.md](KF_BASICS.md) is the CLI contract; this file is what the
team's board and review culture add on top.

## AFK convention

- An issue carrying the **`AFK` label** is approved for autonomous execution.
- **Absence of the label means HITL**: open questions remain, so execution skills stop
  before implementing. Offer the human three ways forward: grill it now (`/kf:issue-grill`),
  proceed anyway on their explicit say-so in this conversation, or skip it.
- Labels only exist on this board while at least one issue carries them. If `AFK` is
  refused as unknown, tell the human to add it to any issue via the KanbanFlow UI — the CLI
  never creates labels.

## Who automation may touch

Automation only takes issues whose `responsibleUserId` is **you or empty** — `kf issue grab`
guards on the responsible field alone, so a collaborator-only issue would refuse (exit 3).
Your own ID comes from `kf auth status --json` (`.userId`, no HTTP request).

Default batch selection:

```bash
kf issue list --state todo --state wip --mine --json
```

then filter client-side: `labels[].name` contains `AFK` (case-insensitive) **and**
`responsibleUserId` is you or empty.

## Branch and worktree per issue

Every issue is executed in its **own worktree on its own branch** — never in the main
checkout, so the human can inspect each issue's result independently.

- Branch: `<author>/<ticket>-<slug>` (AGENTS.md § Version control). `<author>` is the
  GitLab username from `glab api user --jq .username`; `<ticket>` is the lowercase issue
  number (`e613`); `<slug>` is a short kebab-case cut of the issue name.
- Worktree: `<main-checkout>/../<repo-name>.worktrees/<ticket>-<slug>`, next to the main
  checkout. The main checkout is the first line of `git worktree list`; `<repo-name>` is that
  checkout's directory name.
- **Reuse before creating**: a worktree whose directory name contains the ticket is the
  issue's worktree — continue there. Otherwise:

  ```bash
  git fetch origin main
  git worktree add ../<repo-name>.worktrees/<ticket>-<slug> -b <author>/<ticket>-<slug> origin/main
  ```

- Worktrees **stay in place** after execution for the human's review; cleanup is theirs.

## Draft MR only — the human merges

Execution stops at a **draft MR** (the `mr` skill's contract). The human reviews the work,
moves the issue to `review`, marks the MR ready, merges it, and runs `kf issue finish`.
Agents leave the issue in **`wip`** when the MR opens, and never mark an MR ready, merge
one, or move an issue past `wip`.

Board signal at MR time — one comment, nothing else:

```bash
kf comment add <ref> --text "MR: <url>"
```

The final execution report goes on the MR as a note (`glab mr note <mr> --message
"$(cat <file>)"`), where the reviewer reads it.

## Pipeline watch

After the draft MR exists, watch CI from the issue's worktree:

```bash
glab ci status --live          # follows the pipeline for the current branch
glab ci trace <job>            # logs of a failing job
```

- On failure: diagnose from the trace, fix, commit (per the `/mp:commit` skill), push, and
  re-watch — **at most 3 fix attempts**, then stop and report.
- A failure that looks environmental — certs, Storybook boot, `test:e2e` flake with no
  plausible link to the diff — is reported, not chased.
