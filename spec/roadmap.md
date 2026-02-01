This is a living checklist of the remaining work to fully implement and/or reconcile the behaviour described in `spec/spec.md` with the current codebase.

## Completed

- [x] Decide the supported installation paths (GitHub Releases binaries, crates.io, both) and document the supported matrix (OS/arch).
- [x] Keep cargo-dist configuration up to date (targets, installers) and verify `v*` tag releases work end-to-end.
- [x] Decide whether crates.io publishing is manual (initially) or automated in CI; document the required secrets/permissions.
- [x] Add a short changelog/release-notes process (even if minimal) so tags map cleanly to user-visible changes.
- [x] Implement `dsc list --tags <tag1,tag2,...>` filtering (spec defines this flag; CLI currently lists all installs).
- [x] Decide how tag filtering parses separators (comma vs semicolon) and document it in the spec (import currently accepts `,` and `;` for tags).
- [x] Implement a bulk emoji upload command (spec narrative says bulk upload from a directory; only single `dsc emoji add` exists today).
- [x] Implement actual Ubuntu OS update functionality in `dsc update` command.
- [x] Add OS update commands (e.g., `sudo apt update && sudo apt upgrade -y`) before the Discourse rebuild process.
- [x] Add server reboot functionality after OS updates.
- [x] Add environment variable configuration for OS update commands (similar to existing `DSC_SSH_UPDATE_CMD` and `DSC_SSH_CLEANUP_CMD`).
- [x] Add detection of current OS version before and after updates to verify success.
- [x] Decide what `changelog_path` is for (it exists in config but is unused). Either implement local changelog file updating or remove/replace the field.
- [x] Decide whether `dsc add --interactive` should prompt for `ssh_host` and `changelog_topic_id` (needed for `update`/`--post-changelog`) or keep those as manual config edits.
- [x] Add tests for list tag filtering once `--tags` is implemented.
- [x] Harden config file permissions on write to protect API keys.
- [x] Prevent SSH option injection via `ssh_host` or discourse `name`.
- [x] Improve first-time SSH host handling (allow `StrictHostKeyChecking=accept-new` or configurable SSH options).
- [x] Avoid symlink/hijack risk for `dsc update all` log file in CWD.
- [x] Discourse name `all` is reserved by `dsc update`; document the behavior.
- [x] Add a human-friendly output mode for `backup list` (it currently prints raw JSON), or explicitly document JSON as the intended output.
- [x] Consider adding `--format json|yaml` for `group info` and `backup list` for consistency with `dsc list`.
- [x] Add the ability to change a Site Setting on all Discourses (or selected by tag) via the API.
- [x] Ensure proper error handling and rollback capabilities for failed OS updates.
- [x] Backup feature should be able to say when the last backup was done, and where the backup is stored (eg S3).
- [x] Add ability to push and pull colour palettes, as there is no obvious "import" at the moment.

## In Progress

- [~] Add an e2e test for `dsc completions <shell> [--dir <path>]`.

## Planned


## Future
- [ ] Add ability to manage plugins via the CLI - eg list installed plugins, install new plugins, remove plugins.
- [ ] Add ability to manage themes via the CLI - eg list installed themes, install new themes, remove themes.
- [ ] Consider `dsc emoji list --inline` to show emoji images via terminal image protocols (kitty/iTerm2/sixel) with text fallback for unsupported terminals.
