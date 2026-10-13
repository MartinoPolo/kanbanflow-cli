# Changelog

## Unreleased

### Added

- Added `--description-file` to `kf issue create` and `kf issue edit`, so multi-line Markdown needs no shell command substitution.
- Added optional `backlog` canonical-state mapping.
- Added `kf issue move --column <NAME_OR_ID>` for direct moves to board columns by exact ID or unique name. Name matching ignores ASCII case only; non-ASCII characters must match exactly.
