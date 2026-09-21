# kf Basics

Shared contract for every `kf-` skill. `kf` is the KanbanFlow CLI (`gh`-style grammar:
`kf <noun> <verb>`).

## Preconditions

- The repo has an existing MPX project config, `mpxconfig.json`, whose `issues` binding is
  updated by `kf init`: board ID and the column → canonical state mapping. It is committed and
  shared, so it holds board facts only. Missing or broken → exit code **7**; stop and tell the
  human to provide a valid project config, then run `kf init`.
- `kf` is on PATH.
- Token comes from the OS credential store or `KANBANFLOW_TOKEN`. **Never** read, print, or
  write a token, and never put one in a file, a command line, or a commit.
- Which user you act as is per person, not per repo: it comes from the user-level board
  registry or `KANBANFLOW_USER_ID`, and decides what `--mine` matches and which issues the
  guardrail protects. Unknown → the command fails; tell the human to run `kf auth login`.
  **Never** write a user ID into `mpxconfig.json` — it would make every teammate act
  as whoever wrote it.

## Vocabulary

| Term | Meaning |
| --- | --- |
| Issue | A unit of board work. Referenced as `E613` (number) or a raw ID like `T3s6UGyzY`. |
| Canonical state | `backlog`, `todo`, `wip`, `review`, `done`, `archive` — mapped to real columns per board. |
| Label | Area/project tag. **Never created implicitly**; only existing labels can be applied. |
| Color | Issue color. Work convention: **red = bug/critical**, **green = normal**. |
| Subtask | Checklist item, addressed by exact name or 1-based position. |
| Responsible user | The single assignee. `kf issue create` assigns you unless `--responsible none`. |
| Collaborator | An extra person on the issue; the board draws their avatar too. Counts as "mine" for `kf issue list --mine` and for the guardrail. |
| Mine | Responsible user **or** collaborator. The one exception is `kf issue grab`, which reassigns the responsible user and so needs `--force` to take an issue off a teammate. |
| Open | Every column except `done` and `archive` — `kf issue list --open`. Includes board-specific lanes with no canonical state. |

Say "issue", never "task" or "card".

## Rate budget

1000 requests/hour/board; >5000/day locks the token.

- A full `kf issue view` costs **3 API calls** (issue + comments + attachments), plus 1 for
  number→ID resolution.
- `kf issue list` and `kf label list` each cost **one full board scan**.
- **Never poll.** Reuse the JSON you already fetched instead of re-reading an issue.
- No retry loops. One failure → read the message, fix the cause or ask the human.

## Exit codes

| Code | Meaning | Reaction |
| --- | --- | --- |
| 0 | Success | continue |
| 1 | Generic failure | read stderr, fix the cause |
| 2 | Usage error | fix the flags |
| 3 | Guardrail refusal (issue is not yours) | **Stop. Ask the human.** Never retry with `--force` on your own initiative. |
| 4 | No usable token / rejected, or unknown board user | tell the human to run `kf auth login` |
| 5 | Issue, comment or attachment not found | verify the reference |
| 6 | Rate limited | stop, report, do not retry |
| 7 | No usable `mpxconfig.json` | tell the human to run `kf init` |

## Output contract

- Every read command takes `--json`. Agents always pass it.
- Write commands print the affected issue number/ID.
- `--force` exists on every mutating command that can touch a teammate's issue (`issue create` and
  `comment add` have none, and need none). It is a **human decision only** — use it solely
  when the human said so in this conversation.
