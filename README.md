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
- `changelog_topic_id` is required if you want `--post-changelog` to post a checklist update.
- `tags` (optional) can label installs; they are emitted in list output formats.
- `dsc add` without `--interactive` appends a full `[[discourse]]` template containing every supported config key, using placeholders like `""`, `[]`, and `0`.
- Empty strings and `0` values are treated as “unset” (most commands behave as if the key is missing).

## Usage

General form: `dsc [--config dsc.toml] <command>`.

- List installs: `dsc list --format plaintext|markdown|markdown-table|json|yaml|csv`
- Add installs: `dsc add forum-a,forum-b [--interactive]`
- Import installs from file: `dsc import path/to/urls.txt` or `dsc import path/to/forums.csv`
- Update one install: `dsc update <name> [--post-changelog]`
- Update all installs: `dsc update all [--post-changelog]`
- Add emoji: `dsc emoji add <discourse> <emoji.png> <emoji-name>`
- Topic pull: `dsc topic pull <discourse> <topic-id> [local-path]`
- Topic push: `dsc topic push <discourse> <local-path> <topic-id>`
- Topic sync (auto pull or push based on freshest copy): `dsc topic sync <discourse> <topic-id> <local-path> [--yes]`
- Category list: `dsc category list <discourse> [--tree]`
- Category pull: `dsc category pull <discourse> <category-id> [local-path]`
- Category push: `dsc category push <discourse> <local-path> <category-id>`
- Category copy: `dsc category copy <discourse> <category-id>`
- Group list: `dsc group list <discourse>`
- Group info: `dsc group info <discourse> <group-id>`
- Group copy: `dsc group copy <source> <group-id> [--target <target>]`
- Backup create: `dsc backup create <discourse>`
- Backup list: `dsc backup list <discourse>`
- Backup restore: `dsc backup restore <discourse> <backup-path>`

Tips:

- Most commands require the discourse name as the first argument after the subcommand.
- `topic pull`/`category pull` write Markdown files; paths are created as needed.
- `topic sync` compares local mtime with the remote post timestamp; pass `--yes` to skip the prompt.
- `dsc update all` writes a progress log named `YYYY.MM.DD-dsc-update-all.log` in the current directory and stops at the first failure; `--concurrent` is disabled for this command.

## Development

- Build fast feedback: `cargo build`
- Lint/format (if you have rustfmt/clippy in toolchain): `cargo fmt` then `cargo clippy` (optional but recommended)
- Run example binary locally: `cargo run -- --help`

## Release

Releases are automated via Git tags and GitHub Actions using cargo-dist.

Steps:

1. Ensure your working tree is clean and tests pass.
2. Bump the version in `Cargo.toml` and commit it.
3. Tag the release as `v<version>` and push the tag.

- If you use `s/release <version>` (for example, `s/release 0.2.0`), it handles the commit + tag + push.

4. The `Release` workflow builds and uploads binaries to GitHub Releases.

Publishing to crates.io (initially manual):

1. Ensure you can publish (you may need to be added as an owner).
2. Authenticate:

- One-off local login: `cargo login <token>`
- Or per-command: `CARGO_REGISTRY_TOKEN=<token> cargo publish`

3. Publish: `cargo publish`

If you later want CI to publish to crates.io on tags, add `CARGO_REGISTRY_TOKEN` as a GitHub Actions secret and update the release workflow/cargo-dist config accordingly.

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
