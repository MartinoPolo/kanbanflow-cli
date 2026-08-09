---
name: mr
description: 'Pushes the current branch and creates or updates its GitLab merge request as a draft via glab. Use when the user says "create MR", "open a merge request", "update MR", or "push and open a PR".'
allowed-tools: Read, Write, Bash(git status *), Bash(git log *), Bash(git diff *), Bash(git rev-parse *), Bash(git push *), Bash(git symbolic-ref *), Bash(git merge-base *), Bash(git rev-list *), Bash(glab *), Bash(yarn lint), Bash(yarn test)
metadata:
  author: MartinoPolo
  version: '0.1'
  category: git-workflow
---

# Merge Request

Push the current branch and open or update its merge request from the commits already on it. Merging stays with the human — this skill takes it as far as a draft MR.

Branch naming and the **Definition of done** live in AGENTS.md; everything about merge requests is here.

## Workflow

1. **Require a clean tree.** `git status --porcelain`. If anything is uncommitted, stop and offer to commit it first (`/mp:commit`) — an MR describes committed work, and leftover changes would silently miss it.

2. **Pre-MR gate.** Run `yarn format`, then `yarn lint` and `yarn test` (or the host repo's equivalent check scripts). Continue only when both pass; otherwise stop and report.
   - Detect UI changes: `git diff --name-only $(git merge-base main HEAD)...HEAD`. If any path matches `src/components/`, `*.svelte`, `*.css`, `storybook/`, or `e2e/`, print a clear reminder to run `yarn test:e2e` and leave that run to the dev — it needs a live Storybook at `https://localhost:8101` plus certs (see AGENTS.md).

3. **Push if needed.** `git rev-parse --abbrev-ref HEAD` for the branch. If there's no upstream, or local is ahead of `@{u}`, run `git push -u origin HEAD`. If already up to date, skip.

4. **Derive the ticket** from the branch name `<author>/<ticket>-<slug>`: take the `<ticket>` segment and **uppercase** it (e.g. `E602`). Omit it if the branch doesn't match.

5. **Compose title + description** from the branch's commits and diff (`git log main..HEAD --pretty=%s%n%b`, `git diff --stat main...HEAD`):

   - **Title** — `<TICKET>: ` (from step 4) followed by one line that leads with _why_ the change was made (its value), not the branch name. The ticket goes **here**, and the commit subjects stay free of it. Compose it without a `Draft:` prefix — GitLab stores draft state _as_ that prefix, and `--draft` adds it on create.
   - **Description** (write to a temp file) — the _why_ first, then what/how as bullets:

     ```
     <1–2 lines: the motivation/value>

     - <what changed / notable approach>
     - <…>
     ```

     Always write one, and match its length to the change: most MRs here are ~3 lines, and a one-line why plus three bullets is a complete description for an ordinary change. Add `##` headings or prop tables only for large or breaking changes.

   - Keep `Topic:`, `Reviewed-by:` and other people's names out: both are all over `git log` and older MR descriptions, but they are two contributors' Gerrit-era habit, and the ticket is already in the title.

6. **Create or update** via `glab` (target branch from `$ARGUMENTS` or `main`; source = current branch, auto-detected):
   - Check for an existing MR: `glab mr view` (current branch). If one exists → update title and description in place:
     `glab mr update --title "<title>" --description "$(cat <descfile>)"`
     Preserve whatever draft state the MR already has: re-prepend `Draft: ` to the title when `glab mr view` shows it as a draft, since a title without the prefix publishes it for review.
   - Otherwise create it **as a draft** — the human decides when it is ready for review, and marks it so in GitLab:
     `glab mr create --target-branch main --title "<title>" --description "$(cat <descfile>)" --draft --yes`

7. **Report** — MR URL and number, whether created or updated, the base branch, and (if applicable) the e2e reminder from step 2.

## Troubleshooting

| Problem                | Fix                                                                         |
| ---------------------- | --------------------------------------------------------------------------- |
| glab not authenticated | `glab auth status`; re-auth if needed                                       |
| No commits to MR       | Ensure commits exist on the branch (`git log main..HEAD`) — `/mp:commit` first |
| Base branch not found  | Pass the base explicitly as an argument                                     |
| MR already exists      | Expected — step 6 updates it in place                                       |
