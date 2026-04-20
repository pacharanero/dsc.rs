# dsc

A Discourse CLI written in Rust. Manage multiple Discourse forums from your terminal — track installs, run upgrades over SSH, manage users and groups, sync topics and categories as local Markdown, upload files, search, archive activity, and more.

Most functionality uses the Discourse REST API; `dsc update` runs remote rebuilds via SSH.

## Install

The one-liner (Linux and macOS):

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/pacharanero/dsc/releases/latest/download/dsc-rs-installer.sh | sh
```

Or via Cargo:

```bash
cargo install dsc-rs
```

The crate is named `dsc-rs` (the `dsc` name was taken on crates.io), but the installed binary is `dsc`. See the [project README](https://github.com/pacharanero/dsc#readme) for other platforms.

## A minimal config

`dsc` reads `dsc.toml`. The only required fields are `name` and `baseurl`:

```toml
[[discourse]]
name = "myforum"
baseurl = "https://forum.example.com"
apikey = "<admin api key>"       # required for anything that isn't public read
api_username = "system"
ssh_host = "forum.example.com"   # only needed for `dsc update`
```

Full field reference: [Configuration](configuration.md).

## Three things `dsc` does well

### 1. Multi-install operations

One config describes every forum you run. Act on all of them at once:

```bash
dsc list                         # tag-filterable overview
dsc update all --parallel        # rebuilds over SSH across every forum
dsc setting set --tags production site_name "New Name"
dsc config check                 # verify API + SSH reachability for every install
```

### 2. Topics and categories as Markdown

Edit content in your editor, commit it to git, run it through AI drafting — whatever your workflow demands:

```bash
dsc topic pull myforum 1234 ./drafts/
dsc topic push myforum 1234 ./drafts/edited.md
dsc category pull myforum support ./support-category/
```

### 3. Composable read/write primitives

Every write command reads from stdin or a file, so shell pipelines just work:

```bash
git log --since=yesterday --oneline | dsc topic reply myforum 1525
df -h | dsc topic new myforum 42 --title "Disk report $(date -I)"
dsc user activity meta pacharanero --since 7d --format markdown \
  | dsc topic reply myjournalforum 1234
```

## Command index

Browse by area:

- **Content** — [`topic`](topic.md), [`post`](post.md), [`category`](category.md), [`search`](search.md), [`upload`](upload.md), [`tag`](tag.md), [`emoji`](emoji.md)
- **Users & access** — [`user`](user.md), [`group`](group.md), [`invite`](invite.md), [`pm`](pm.md), [`api-key`](api-key.md)
- **Install management** — [`list`](list.md), [`add`](add.md), [`import`](import.md), [`open`](open.md), [`update`](update.md), [`config`](config.md)
- **Site admin** — [`setting`](setting.md), [`backup`](backup.md), [`theme`](theme.md), [`plugin`](plugin.md), [`palette`](palette.md)
- **Meta** — [Shell completions](completions.md), [Development](development.md)

## Safe by default

- Destructive operations (`backup restore`, `setting set`, `topic push`, plugin/theme install/remove, and more) honour a global `--dry-run` / `-n` flag.
- HTTP 429 responses are retried automatically with the Retry-After header the server returns.
- Read-only commands like `user activity` work without an API key on forums that allow public reads.
