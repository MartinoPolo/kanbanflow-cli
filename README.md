# kanbanflow-cli

`kf` is a Rust command-line client for [KanbanFlow](https://kanbanflow.com), modelled on the
ergonomics of `gh` and `glab` but built for AI-agent workflows: predictable subcommands, scriptable
output, and — the feature `gh` and `glab` both lack — a full **image attachment round-trip**, so an
agent can upload a screenshot to a task and pull it back down again without leaving the terminal.

**Status:** scaffold, v1 in planning. See [`.mpx/CONTEXT.md`](.mpx/CONTEXT.md) and
[`.mpx/DECISIONS.md`](.mpx/DECISIONS.md) for the current design and decision log.

## Repo layout

| Path | Purpose |
| --- | --- |
| `src/` | Rust sources for the `kf` binary |
| `docs/api/` | Vendored snapshot of the KanbanFlow API documentation |
| `scripts/scrape-docs.mjs` | Refreshes `docs/api/` from the live documentation site |
| `skills/` | `kf-` prefixed Claude Code skills for agent-driven usage |

Refresh the vendored API docs with:

```bash
cd scripts && npm install && node scrape-docs.mjs ../docs/api
```

## Development

```bash
cargo build
cargo run -- --version
```

The API token is read from the `KANBANFLOW_TOKEN` environment variable. Tokens are **per board** and
are created in the board's **Settings → API & Webhooks**. Never commit a token.

## Planned command surface

```text
kf init
kf auth login
kf task     create | view | list | edit | move | delete
kf attach   add | list | download | delete
kf comment  add | list | edit | delete
kf subtask  add | list | check | uncheck
kf label    list
kf board
```

## License

MIT
