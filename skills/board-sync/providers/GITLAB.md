# GitLab evidence via glab

Implements the board-sync evidence contract for GitLab: for each ticket number, report
`merged | ready | draft | none` plus the MR URL.

## Preconditions

`glab` authenticated against the repo's GitLab host (`glab auth status`). Run every
command from inside the repo so glab resolves the project from the git remote.

## Listing commands — once per run

```bash
glab mr list --author=@me --output json --per-page 100            # open: draft + ready
glab mr list --author=@me --merged --output json --per-page 100   # merged
```

Relevant fields per MR: `title`, `draft`, `state`, `source_branch`, `web_url`.

## Matching an MR to a ticket

Case-insensitive, two signals, either suffices:

1. **Title prefix** — the repo convention puts the uppercased ticket first:
   `E621: <why>`. A draft carries GitLab's `Draft:` prefix before it, so match the
   ticket anywhere in the title's leading segment, not only at position 0.
2. **Source branch** — the convention `<author>/<ticket>-<slug>` puts the lowercased
   ticket in the second segment: `<author>/e621-input-validation`.

Prefer the title match when the two disagree. Count only `state: opened` and
`state: merged` MRs; `closed` is not evidence.

## Verdict

| glab result                          | Verdict  |
| ------------------------------------ | -------- |
| matched MR with `state: merged`      | `merged` |
| matched MR, `state: opened`, `draft: false` | `ready` |
| matched MR, `state: opened`, `draft: true`  | `draft` |
| no match in either listing           | `none`   |

A merged **and** an open MR matching the same ticket is not ambiguous — the open one
wins (follow-up work reopened the ticket's story). Two **open** MRs matching one
ticket is ambiguous: report, skip the move.

## Adding another provider

Create `providers/<NAME>.md` beside this file with the same sections — preconditions,
listing commands, ticket matching, verdict mapping — and teach the repo's
`mpxconfig.json` the value: `"repository": { "provider": "<name>", ... }`. For Gerrit, the natural signals
are the change's `status` (`NEW`/`MERGED`), its WIP flag for the draft verdict, and
the ticket in the commit-message topic or footer.
