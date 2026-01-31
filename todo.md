# Security/CLI Review Notes

- [x] Harden config file permissions on write to protect API keys. `save_config` uses `fs::write` with default permissions; consider 0600 on Unix and warn if permissions are world-readable. (src/config.rs)
- [x] Prevent SSH option injection via `ssh_host` or discourse `name`. `run_ssh_command` passes the target directly to `ssh`; if it starts with `-` or includes whitespace, it can be interpreted as options. Add validation or pass `--` before the target. (src/main.rs)
- [x] Improve first-time SSH host handling. With `BatchMode=yes`, `ssh` fails if the host key is unknown; consider `StrictHostKeyChecking=accept-new` or a configurable SSH options env var to avoid confusing failures. (src/main.rs)
- [x] Avoid symlink/hijack risk for `dsc update all` log file in CWD. Consider using `create_new(true)` or a configurable log directory, and document the behavior. (src/main.rs)
- [x] Discourse name `all` is reserved by `dsc update`; consider documenting this explicitly or allowing an escape hatch so a discourse named `all` can still be updated. (src/main.rs, README.md)
 - [ ] Add the ability to change a Site Setting on all Discourses (or selected by tag) via the API - this is particularly useful in bulk for example if a new setting (eg auto grid) is added and you want to enable/disable it across multiple sites.
 - [ ] Backup feature should be able to say when the last backup was done, and where the backup is stored (eg S3)
 - [ ] Add ability to push and pull colour palettes, as there is no obvious 'import' at the moment.
 - [ ] Add ability to manage plugins via the CLI - eg list installed plugins, install new plugins, remove plugins.
 - [ ] Add ability to manage themes via the CLI - eg list installed themes, install new themes, remove themes.
  
