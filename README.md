# dsc

A Discourse CLI tool written in Rust. Manage multiple Discourse forums from your terminal — track installs, run upgrades over SSH, manage emojis, perform backups, and sync topics and categories as local Markdown.

Most functionality uses the Discourse REST API. `dsc update` runs remote rebuilds via SSH.

## Features

- Track any number of Discourse installs via a single config file.
- Manage categories, topics, settings, and groups across installs.
- Run rebuilds over SSH and optionally post changelog updates.
- Import from text or CSV, or add installs ad-hoc.
- Pull/push individual topics or whole categories as Markdown.
- Upload custom emojis in bulk.
- List, install, and remove themes and plugins.
- Create, list, and restore backups.

## Installation

### Shell installer (Linux and macOS)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/pacharanero/dsc/releases/latest/download/dsc-rs-installer.sh | sh
```

Downloads a prebuilt binary for your platform and installs it to `~/.cargo/bin` (or `$CARGO_HOME/bin` if set). Supports `x86_64` and `aarch64` on both Linux and macOS.

### From crates.io

If you already have a Rust toolchain:

```bash
cargo install dsc-rs
```

The crate is published as `dsc-rs` (the `dsc` name was taken), but the installed binary is still `dsc`.

### Direct download

Prebuilt archives for Linux, macOS, and Windows are attached to every [GitHub release](https://github.com/pacharanero/dsc/releases/latest). Download, extract, and drop `dsc` (or `dsc.exe`) anywhere on your `PATH`.

### From source

Requires a recent Rust toolchain (edition 2024; install via [rustup](https://rustup.rs)).

```bash
git clone https://github.com/pacharanero/dsc.git
cd dsc
cargo install --path .
```

## Quick start

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
dsc list

# Pull a topic into Markdown for editing
dsc topic pull myforum 42

# Push the edited topic back up
dsc topic push myforum 42 ./topic-title.md

# Update a forum over SSH
dsc update myforum
```

## Documentation

- [Configuration](docs/configuration.md) — config file format, search order, field reference
- **Commands:**
  - [list](docs/list.md) — list and filter installs
  - [open](docs/open.md) — open a Discourse in the browser
  - [add](docs/add.md) — add installs to config
  - [import](docs/import.md) — import installs from file or stdin
  - [update](docs/update.md) — run OS and Discourse updates over SSH
  - [search](docs/search.md) — search topics on a Discourse
  - [upload](docs/upload.md) — upload a file and return its short URL
  - [emoji](docs/emoji.md) — upload and list custom emoji
  - [topic](docs/topic.md) — pull, push, and sync topics as Markdown
  - [post](docs/post.md) — edit, delete, and move individual posts
  - [category](docs/category.md) — list, pull, push, and copy categories
  - [palette](docs/palette.md) — list, pull, and push colour palettes
  - [plugin](docs/plugin.md) — list, install, and remove plugins
  - [theme](docs/theme.md) — list, install, remove, pull, push, and duplicate themes
  - [group](docs/group.md) — list, inspect, copy, and bulk-add members
  - [user](docs/user.md) — list, inspect, suspend, and manage group memberships
  - [backup](docs/backup.md) — create, list, and restore backups
  - [setting](docs/setting.md) — get and set site settings
  - [tag](docs/tag.md) — list tags and apply/remove them on topics
  - [config](docs/config.md) — inspect and validate the dsc config itself
- [Shell completions](docs/completions.md) — bash, zsh, and fish
- [Development](docs/development.md) — building, testing, releasing, project layout

## License

MIT. See [LICENSE](LICENSE).
