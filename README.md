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

`dsc` reads configuration from `dsc.toml` in the working directory by default (override with `--config <path>`). Each Discourse instance lives under a `[[discourse]]` table. See [dsc.example.toml](dsc.example.toml) for a fuller template. Minimum useful fields are `name`, `baseurl`, `apikey`, and `api_username`.

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
- `fullname` is the Discourse site title (auto-populated when adding/importing if it can be fetched).
- `ssh_host` enables `update` over SSH (`./launcher rebuild app`). Configure keys in your SSH config.
- `changelog_topic_id` is required if you want `--post-changelog` to prompt and post a checklist update.
- `tags` (optional) can label installs; they are emitted in list output formats.
- Most forum read/write commands require `apikey` and `api_username`. If they are missing, `dsc` will fail with a clear message.
- `dsc add` without `--interactive` appends a full `[[discourse]]` template containing every supported config key, using placeholders like `""`, `[]`, and `0`.
- Empty strings and `0` values are treated as “unset” (most commands behave as if the key is missing).

## Usage

General form: `dsc [--config dsc.toml] <command>`.

- List installs: `dsc list --format plaintext|markdown|markdown-table|json|yaml|csv`
- Add installs: `dsc add forum-a,forum-b [--interactive]`
- Import installs from file: `dsc import path/to/urls.txt` or `dsc import path/to/forums.csv`
- Update one install: `dsc update <name> [--post-changelog]`
- Update all installs: `dsc update all [--post-changelog]`
  - `--post-changelog` prints the checklist and prompts before posting.
- Add emoji: `dsc emoji add <discourse> <emoji-path> [emoji-name]`
- List custom emoji: `dsc emoji list <discourse>`
- Topic pull: `dsc topic pull <discourse> <topic-id> [local-path]`
- Topic push: `dsc topic push <discourse> <local-path> <topic-id>`
- Topic sync (auto pull or push based on freshest copy): `dsc topic sync <discourse> <topic-id> <local-path> [--yes]`
- Category list: `dsc category list <discourse> [--tree]`
- Category pull: `dsc category pull <discourse> <category-id> [local-path]`
- Category push: `dsc category push <discourse> <local-path> <category-id>`
- Category copy: `dsc category copy <discourse> <category-id>`
- Group list: `dsc group list <discourse>`
- Group info: `dsc group info <discourse> <group-id> [--format json|yaml]`
- Group copy: `dsc group copy <source> <group-id> [--target <target>]`
- Backup create: `dsc backup create <discourse>`
- Backup list: `dsc backup list <discourse> [--format <format>]`
- Backup restore: `dsc backup restore <discourse> <backup-path>`
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
- `DSC_SSH_STRICT_HOST_KEY_CHECKING` (default: `accept-new`; set empty to omit)
- `DSC_SSH_OPTIONS` (extra ssh options, space-delimited)
- `DSC_UPDATE_LOG_DIR` (directory for `dsc update all` logs; defaults to current directory)

Tips:

- Most commands require the discourse name as the first argument after the subcommand.
- `topic pull`/`category pull` write Markdown files; paths are created as needed.
- `topic sync` compares local mtime with the remote post timestamp; pass `--yes` to skip the prompt.
- `dsc update all` writes a progress log named `YYYY.MM.DD-dsc-update-all.log` in the current directory and stops at the first failure; `--concurrent` is disabled for this command.
- `all` is reserved for `dsc update all`.
- `dsc list --tags` accepts comma or semicolon separators and matches any tag (case-insensitive).
- `dsc backup list --format` supports the same formats as `dsc list`.
- `dsc emoji add` accepts a file or directory path. Directory uploads all `.png`, `.jpg`, `.jpeg`, `.gif`, `.svg` files using the filename stem as the emoji name.
- `dsc emoji add`/`dsc emoji list` require an admin API key and username.
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
changelog_topic_id = 123        # optional unless testing update --post-changelog
test_topic_id = 456             # topic used by e2e topic tests
test_category_id = 789          # category used by e2e category tests
emoji_path = "./smile.png"     # optional; enables emoji add test
emoji_name = "smile"
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
