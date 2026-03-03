# dsc.rs

dsc.rs is a very cleverly-named Discourse CLI tool written in Rust, which does many of the things I personally want to be able to do with Discourse forums remotely from the command line.

It acts as a command-line companion that keeps multiple forums in sync from your terminal. It can track installs, run upgrades over SSH, manage emojis, perform backups, and save topics or categories as local Markdown so you can edit with your own tools.

Most functionality is provided through interactions with the Discourse REST API, apart from `dsc update` which runs a remote rebuild via SSH.

## Features

- Track any number of Discourse installs via a single config file.
- Manage categories, topics, settings and groups across installs.
- Run rebuilds over SSH and optionally post changelog updates.
- Import from text or CSV, or add them ad-hoc.
- Pull/push individual topics or whole categories as Markdown.
- Upload custom emojis in bulk to a forum.

## Installation

- Prerequisites: a recent Rust toolchain (edition 2024; install via rustup).
- From source (debug):
  ```bash
  cargo build
  target/debug/dsc --help
  ```
- From source (optimized):
  ```bash
  cargo build --release
  target/release/dsc --help
  ```
- Install into your Cargo bin dir:
  ```bash
  cargo install --path .
  ```

## Configuration

If `--config <path>` is not provided, `dsc` searches for a config in this order:

1. `./dsc.toml`
2. `$XDG_CONFIG_HOME/dsc/dsc.toml` (or `~/.config/dsc/dsc.toml` when `XDG_CONFIG_HOME` is unset)
3. System config locations (`$XDG_CONFIG_DIRS` entries as `<dir>/dsc/dsc.toml`, then `/etc/xdg/dsc/dsc.toml`, `/etc/dsc/dsc.toml`, `/etc/dsc.toml`, `/usr/local/etc/dsc.toml`)

If none are found, it defaults to `./dsc.toml` (created on first write command). Each Discourse instance lives under a `[[discourse]]` table. See [dsc.example.toml](dsc.example.toml) for a fuller template. Minimum useful fields are `name`, `baseurl`, `apikey`, and `api_username`.

```toml
[[discourse]]
name = "myforum"
fullname = "My Forum"
baseurl = "https://forum.example.com"
apikey = "your_api_key_here"
api_username = "system"
changelog_topic_id = 123
ssh_host = "forum.example.com"
```

Notes:

- `baseurl` should not end with a trailing slash.
- `name` should be short and slugified (avoid spaces). Use `fullname` for the display name.
- `fullname` is the Discourse site title (auto-populated when adding/importing if it can be fetched).
- `ssh_host` enables `update` over SSH (`./launcher rebuild app`). Configure keys in your SSH config.
- `changelog_topic_id` is required if you want `dsc update` to prompt and post a checklist update (default behavior).
- `tags` (optional) can label installs; they are emitted in list output formats.
- Most forum read/write commands require `apikey` and `api_username`. If they are missing, `dsc` will fail with a clear message.
- `dsc add` without `--interactive` appends a full `[[discourse]]` template containing every supported config key, using placeholders like `""`, `[]`, and `0`.
- Empty strings and `0` values are treated as “unset” (most commands behave as if the key is missing).

## Usage

General form: `dsc [--config <path>] <command>`.

For the complete command/flag surface, use `dsc --help` and `dsc <command> --help`.

- List installs: `dsc list --format text|markdown|markdown-table|json|yaml|csv`
- Tidy/sort config entries: `dsc list tidy`
- Add installs: `dsc add forum-a,forum-b [--interactive]`
- Import installs from file: `dsc import path/to/urls.txt` or `dsc import path/to/forums.csv`
- Update one install: `dsc update <name> [--no-changelog] [--yes]`
- Update all installs: `dsc update all [--parallel] [--max <n>] [--no-changelog] [--yes]`

Environment variables for `dsc update`:

- `DSC_SSH_OS_UPDATE_CMD` (default: `sudo -n DEBIAN_FRONTEND=noninteractive apt update && sudo -n DEBIAN_FRONTEND=noninteractive apt upgrade -y`)
- `DSC_SSH_OS_UPDATE_ROLLBACK_CMD` (optional command to run if OS update fails)
- `DSC_SSH_REBOOT_CMD` (default: `sudo -n reboot`)
- `DSC_SSH_OS_VERSION_CMD` (default: `lsb_release -d | cut -f2`, fallback to `/etc/os-release`)
- `DSC_SSH_UPDATE_CMD` (default: `cd /var/discourse && sudo -n ./launcher rebuild app`)
- `DSC_SSH_CLEANUP_CMD` (default: `cd /var/discourse && sudo -n ./launcher cleanup`)
- `DSC_SSH_STRICT_HOST_KEY_CHECKING` (default: `accept-new`; set empty to omit)
- `DSC_SSH_OPTIONS` (extra ssh options, space-delimited)
- `DSC_DISCOURSE_BOOT_WAIT_SECS` (default: `15`; seconds to wait after rebuild before fetching `about.json`)
- `DSC_COLOR` (`auto`|`always`|`never`, default: `auto`) controls ANSI color output for friendly discourse labels in update logs (`NO_COLOR` also disables color)

Update summary notes:

- Version/commit info is extracted from the homepage `<meta name="generator" ...>` tag; the commit hash is printed as a GitHub link when available.
- If the version fetch fails, the summary includes the reason in-line.
  - Changelog posting is on by default for `dsc update`; pass `--no-changelog` to skip posting.
  - `--yes` auto-confirms the changelog post prompt (non-interactive mode).
- Add emoji: `dsc emoji add <discourse> <emoji-path> [emoji-name]`
- List custom emoji: `dsc emoji list <discourse> [--format text|json|yaml] [--inline]`
- Topic pull: `dsc topic pull <discourse> <topic-id> [local-path]`
- Topic push: `dsc topic push <discourse> <local-path> <topic-id>`
- Topic sync (auto pull or push based on freshest copy): `dsc topic sync <discourse> <topic-id> <local-path> [--yes]`
- Category list: `dsc category list <discourse> [--format text|json|yaml] [--tree]`
- Category pull: `dsc category pull <discourse> <category-id-or-slug> [local-path]`
- Category push: `dsc category push <discourse> <local-path> <category-id-or-slug>`
- Category copy: `dsc category copy <source> <category-id-or-slug> [--target <target>]`
- Palette list: `dsc palette list <discourse> [--format text|json|yaml]`
- Palette pull: `dsc palette pull <discourse> <palette-id> [local-path]`
- Palette push: `dsc palette push <discourse> <local-path> [palette-id]`
- Plugin list: `dsc plugin list <discourse> [--format text|json|yaml]`
- Plugin install: `dsc plugin install <discourse> <url>`
- Plugin remove: `dsc plugin remove <discourse> <name>`
- Theme list: `dsc theme list <discourse> [--format text|json|yaml]`
- Theme install: `dsc theme install <discourse> <url>`
- Theme remove: `dsc theme remove <discourse> <name>`
- Group list: `dsc group list <discourse> [--format text|json|yaml]`
- Group info: `dsc group info <discourse> <group-id> [--format json|yaml]`
- Group members: `dsc group members <discourse> <group-id> [--format text|json|yaml]`
- Group copy: `dsc group copy <source> <group-id> [--target <target>]`
- Backup create: `dsc backup create <discourse>`
- Backup list: `dsc backup list <discourse> [--format text|markdown|markdown-table|json|yaml|csv]`
- Backup restore: `dsc backup restore <discourse> <backup-path>`
- Set a site setting: `dsc setting set <setting> <value> [--tags alpha,beta]`
- List filtered by tags: `dsc list --tags alpha,beta`

## Installation

Prebuilt binaries are published to GitHub Releases for:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

If you have Rust installed, you can also install from crates.io:

```bash
cargo install dsc
```

## Shell completions

Generate completions:

```bash
# Bash
dsc completions bash --dir /usr/local/share/bash-completion/completions

# Zsh
dsc completions zsh --dir ~/.zsh/completions
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
autoload -Uz compinit && compinit

# The Zsh generator writes `_dsc` in that directory.

# Fish
dsc completions fish --dir ~/.config/fish/completions
```

If you omit `--dir`, the completion script is printed to stdout so you can redirect it.

Update environment variables (optional overrides for SSH commands):

- `DSC_SSH_OS_UPDATE_CMD` (default: `sudo -n DEBIAN_FRONTEND=noninteractive apt update && sudo -n DEBIAN_FRONTEND=noninteractive apt upgrade -y`)
- `DSC_SSH_OS_UPDATE_ROLLBACK_CMD` (optional command to run if OS update fails)
- `DSC_SSH_REBOOT_CMD` (default: `sudo -n reboot`)
- `DSC_SSH_OS_VERSION_CMD` (default: `lsb_release -d | cut -f2`, fallback to `/etc/os-release`)
- `DSC_SSH_UPDATE_CMD` (default: `cd /var/discourse && sudo -n ./launcher rebuild app`)
- `DSC_SSH_CLEANUP_CMD` (default: `cd /var/discourse && sudo -n ./launcher cleanup`)
- `DSC_SSH_PLUGIN_INSTALL_CMD` (template command for `dsc plugin install`; supports `{url}` and `{name}`)
- `DSC_SSH_PLUGIN_REMOVE_CMD` (template command for `dsc plugin remove`; supports `{name}` and `{url}`)
- `DSC_SSH_THEME_INSTALL_CMD` (template command for `dsc theme install`; supports `{url}` and `{name}`)
- `DSC_SSH_THEME_REMOVE_CMD` (template command for `dsc theme remove`; supports `{name}` and `{url}`)
- `DSC_SSH_STRICT_HOST_KEY_CHECKING` (default: `accept-new`; set empty to omit)
- `DSC_SSH_OPTIONS` (extra ssh options, space-delimited)

Tips:

- Most commands require the discourse name as the first argument after the subcommand.
- `topic pull`/`category pull` write Markdown files; paths are created as needed.
- `topic sync` compares local mtime with the remote post timestamp; pass `--yes` to skip the prompt.
- `dsc update all` stops at the first failure; `--parallel` is disabled for this command.
- `all` is reserved for `dsc update all`.
- `dsc list --tags` accepts comma or semicolon separators and matches any tag (case-insensitive).
- `dsc backup list --format` supports the same formats as `dsc list`.
- `dsc emoji add` accepts a file or directory path. Directory uploads all `.png`, `.jpg`, `.jpeg`, `.gif`, `.svg` files using the filename stem as the emoji name.
- `dsc emoji add`/`dsc emoji list` require an admin API key and username.
- `dsc emoji list --inline` uses terminal image protocols; set `DSC_EMOJI_INLINE_PROTOCOL=iterm2|kitty|off` to override detection.
- If your instance requires a `client_id` query parameter for admin emoji endpoints, set `DSC_EMOJI_CLIENT_ID`.

## Development

- Build fast feedback: `cargo build`
- Lint/format (if you have rustfmt/clippy in toolchain): `cargo fmt` then `cargo clippy` (optional but recommended)
- Run example binary locally: `cargo run -- --help`
- Verbose e2e output: `DSC_TEST_VERBOSE=1 cargo test -- --nocapture`
  - Note: `-v` / `--verbose` are not supported by the Rust test harness; they will fail with "no option -v".

## Release

Releases are automated via Git tags and GitHub Actions using cargo-dist.

Steps:

1. Update `CHANGELOG.md` with release notes.
2. Ensure your working tree is clean and tests pass.
3. Run `s/release <version>` (for example, `s/release 0.2.0`).
4. The script commits the version bump, tags `v<version>`, and pushes.
5. The `Release` workflow builds and uploads binaries to GitHub Releases.
6. The `crates-io` job publishes the crate (requires `CARGO_REGISTRY_TOKEN`).

Artifacts include platform-specific archives, checksums, and a shell installer script.

## Testing

- Standard test suite: `cargo test`
- End-to-end tests hit a real Discourse. Provide credentials in `testdsc.toml` (or point `TEST_DSC_CONFIG` to a file) using the shape shown below; otherwise e2e tests auto-skip.

```toml
[[discourse]]
name = "myforum"
baseurl = "https://forum.example.com"
apikey = "<admin api key>"
api_username = "system"
changelog_topic_id = 123        # optional unless testing update changelog posting
test_topic_id = 456             # topic used by e2e topic tests
test_category_id = 789          # category used by e2e category tests
test_color_scheme_id = 321      # palette used by e2e palette tests
emoji_path = "./smile.png"     # optional; enables emoji add test
emoji_name = "smile"
test_plugin_url = "https://github.com/discourse/discourse-reactions"
test_plugin_name = "discourse-reactions"
test_theme_url = "https://github.com/discourse/discourse-brand-header"
test_theme_name = "discourse-brand-header"
```

## Project layout

- CLI entrypoint and commands: [src/main.rs](src/main.rs)
- API client and forum interactions: [src/discourse.rs](src/discourse.rs)
- Config structures and helpers: [src/config.rs](src/config.rs)
- Utility helpers (slugify, I/O): [src/utils.rs](src/utils.rs)
- Example configuration: [dsc.example.toml](dsc.example.toml)
- Specification notes: [spec/spec.md](spec/spec.md)

## License

MIT. See [LICENSE](LICENSE).

## Example workflow

```bash
# Create a config file
cat > dsc.toml <<'EOF'
[[discourse]]
name = "myforum"
baseurl = "https://forum.example.com"
apikey = "<api key>"
api_username = "system"
ssh_host = "forum.example.com"
changelog_topic_id = 123
EOF

# List configured forums
./target/release/dsc list --format markdown

# Pull a topic into Markdown for editing
./target/release/dsc topic pull myforum 42 ./content

# Push the edited topic back up
./target/release/dsc topic push myforum ./content/topic-title.md 42
```
