# kf Basics

Shared contract for every `kf-` skill. `kf` is the KanbanFlow CLI (`gh`-style grammar:
`kf <noun> <verb>`).

## Preconditions

- The repo has `.mpx/kanbanflow.json` (written by `kf init`): board ID, column → canonical
  state mapping, your user ID. Missing or broken → exit code **7**; stop and tell the human
  to run `kf init`.
- `kf` is on PATH.
- Token comes from the OS credential store or `KANBANFLOW_TOKEN`. **Never** read, print, or
  write a token, and never put one in a file, a command line, or a commit.

## Vocabulary

| Term | Meaning |
| --- | --- |
| Task | A card. Referenced as `E613` (number) or a raw ID like `T3s6UGyzY`. |
| Canonical state | `todo`, `wip`, `review`, `done`, `archive` — mapped to real columns per board. |
| Label | Area/project tag. **Never created implicitly**; only existing labels can be applied. |
| Color | Card color. Work convention: **red = bug/critical**, **green = normal**. |
| Subtask | Checklist item, addressed by exact name or 1-based position. |
| Responsible user | The single assignee. `kf task create` assigns you unless `--responsible none`. |
| Collaborator | An extra person on the task; the board draws their avatar too. Counts as "mine" for `kf task list --mine` and for the guardrail. |
| Mine | Responsible user **or** collaborator. The one exception is `kf task grab`, which reassigns the responsible user and so needs `--force` to take a task off a teammate. |
| Open | Every column except `done` and `archive` — `kf task list --open`. Includes board-specific lanes with no canonical state. |

Say "task", never "issue" or "card".

## Rate budget

1000 requests/hour/board; >5000/day locks the token.

- A full `kf task view` costs **3 API calls** (task + comments + attachments), plus 1 for
  number→ID resolution.
- `kf task list` and `kf label list` each cost **one full board scan**.
- **Never poll.** Reuse the JSON you already fetched instead of re-reading a task.
- No retry loops. One failure → read the message, fix the cause or ask the human.

## Exit codes

| Code | Meaning | Reaction |
| --- | --- | --- |
| 0 | Success | continue |
| 1 | Generic failure | read stderr, fix the cause |
| 2 | Usage error | fix the flags |
| 3 | Guardrail refusal (task is not yours) | **Stop. Ask the human.** Never retry with `--force` on your own initiative. |
| 4 | No usable token / rejected | tell the human to run `kf auth login` |
| 5 | Task, comment or attachment not found | verify the reference |
| 6 | Rate limited | stop, report, do not retry |
| 7 | No usable `.mpx/kanbanflow.json` | tell the human to run `kf init` |

## Output contract

- Every read command takes `--json`. Agents always pass it.
- Write commands print the affected task number/ID.
- `--force` exists on every mutating command that can touch a teammate's task (`task create` and
  `comment add` have none, and need none). It is a **human decision only** — use it solely
  when the human said so in this conversation.
