# dsc.rs

dsc.rs is a very cleverly-named Discourse CLI tool written in Rust, which does many of the things I personally want to be able to do with Discourse forums remotely from the command line.

## Features

- Keeping track of Discourse installs
- Performing updates over SSH
- Updating the Changelog automatically
- Synchronising Categories and Topics down to local Markdown files and back again

---

## Command Spec

Global option:

- `dsc --config <path> <command>` (or `-c <path>`) to select a config file (default: `dsc.toml`).

### `dsc list [--format <format>] [--tags <tag1,tag2,...>]`

Lists all Discourse installs known to dsc.rs, optionally filtered by tags.

Tag filters accept comma or semicolon separators and match any tag (case-insensitive).

List formats:

- `plaintext` || `txt`(default)
- `markdown` || `md`
- `markdown-table` || `md-table`
- `json`
- `yaml` || `yml`
- `csv`

### `dsc list tidy`

Orders the `dsc.toml` file entries alphabetically by name.
Collects any missing full names by querying the Discourse URLs.

### `dsc add <name>,<name>,... [--interactive]`

Adds one or more Discourses to `dsc.toml`, creating one entry per name.

- Default (non-interactive) mode appends a full `[[discourse]]` template entry for each name, including all known fields, using placeholders:
  - `""` for string fields
  - `[]` for list fields
  - `0` for numeric fields

  Empty strings (`""`) and `0` are treated as **unset** when the config is loaded (they are converted to `None` internally), so leaving placeholders in place is equivalent to leaving the field blank.

- `--interactive` (or `-i`) prompts for base URL, API key, username, tags, ssh_host, and changelog_topic_id. Fields can be left blank to stay unset.

### `dsc import [<path>]`

Imports Discourses from stdin or a file.
Supported formats:

- Text file with one Discourse URL per line
- CSV file with "name, url, tags" columns

If `<path>` is omitted, input is read from stdin.

`dsc` will attempt to populate the name and fullname fields by querying the Discourse URL for the site title.

### `dsc update <name|all> [--post-changelog] [--concurrent] [--max <n>]`

Updates the Discourse install identified by `<name>` over SSH.
Optionally makes a post in the Changelog topic about the update.

Version and cleanup data should be collected during the update and used to fill the checklist:

```md
- [x] Ubuntu OS updated
- [x] Server rebooted
- [x] Updated Discourse to version [x.y.z] (where x.y.z is the new version number)
- [x] `./launcher cleanup` Total reclaimed space: [reclaimed space] (where [reclaimed space] is the amount of disk space reclaimed)
```

Flags:

- `--post-changelog` (or `-p`) prints the checklist to stdout and prompts before posting to `changelog_topic_id`.
- `--concurrent` (or `-C`) is disabled for `dsc update all` because updates stop at first failure.
- `--max <n>` (or `-m <n>`) is ignored when `--concurrent` is disabled.

Environment variables (optional overrides for SSH commands):

- `DSC_SSH_OS_UPDATE_CMD` (default: `sudo -n DEBIAN_FRONTEND=noninteractive apt update && sudo -n DEBIAN_FRONTEND=noninteractive apt upgrade -y`)
- `DSC_SSH_OS_UPDATE_ROLLBACK_CMD` (optional command to run if OS update fails)
- `DSC_SSH_REBOOT_CMD` (default: `sudo -n reboot`)
- `DSC_SSH_OS_VERSION_CMD` (default: `lsb_release -d | cut -f2`, fallback to `/etc/os-release`)
- `DSC_SSH_UPDATE_CMD` (default: `cd /var/discourse && sudo -n ./launcher rebuild app`)
- `DSC_SSH_CLEANUP_CMD` (default: `cd /var/discourse && sudo -n ./launcher cleanup`)
- `DSC_SSH_STRICT_HOST_KEY_CHECKING` (default: `accept-new`; set empty to omit)
- `DSC_SSH_OPTIONS` (extra ssh options, space-delimited)
- `DSC_UPDATE_LOG_DIR` (directory for `dsc update all` logs; defaults to current directory)

> SSH credentials are not stored in `dsc.toml`; it is advised to set up SSH keys and use an SSH config file.

If the OS update command fails, `dsc update` aborts after attempting the rollback command (when configured).

Note: most forum read/write commands require `apikey` and `api_username`; if missing, the command fails with a clear message.

### `dsc update all [--post-changelog]`

Updates all Discourses known to `dsc` over SSH.

Notes:

- Writes a progress log named `YYYY.MM.DD-dsc-update-all.log` in the current working directory.
- Stops at the first failure to avoid cascading problems.
- `--concurrent` is disabled for `dsc update all` because it must stop at the first failure.
- `all` is reserved for `dsc update all`.

> SSH credentials are not stored in `dsc.toml`; it is advised to set up SSH keys and use an SSH config file.

### `dsc completions <shell> [--dir <path>]`

Generates shell completion scripts for bash, zsh, or fish.

If `--dir` is provided, writes the completion script to the given directory. Otherwise, writes to stdout.

## Emoji

I like to use custom emoji on my forums but the current interface for uploading them is quite tedious, being restricted to the same number of simultaneous uploads as you have set for Posts, and the maximum is currently 20. dsc.rs can bulk upload emoji from a local directory to a target Discourse install.

Having a complete set of the Fontawesome icons used in the Discourse UI as Custom Emoji makes it far easier to explain the user interface in posts to users.

### `dsc emoji add <discourse> <emoji-path> [emoji-name]`

Adds a new emoji to a Discourse install from a local image file. If `emoji-name` is omitted, the filename stem is used (slugified; dashes converted to underscores).

If `emoji-path` is a directory, uploads all `.png`, `.jpg`, `.jpeg`, `.gif`, `.svg` files using the filename stem as the emoji name.

Requires an admin API key and username.

If your instance requires a `client_id` query parameter for admin emoji endpoints, set `DSC_EMOJI_CLIENT_ID` to append it automatically.

### `dsc emoji list <discourse>`

Lists custom emojis on a Discourse (name + URL).

Note: this uses an admin endpoint and requires an admin API key and username.

## Topics

### `dsc topic pull <discourse> <topic-id> [<local-path>]`

Pulls the specified topic into a local Markdown file.

If `<local-path>` is omitted, the topic is written to a new file in the current directory (named from the topic title). If the path does not exist, it will be created.

### `dsc topic push <discourse> <local-path> <topic-id>`

Pushes the specified local Markdown file up to the specified topic in the Discourse install, updating the topic with the contents of the local file.

### `dsc topic sync <discourse> <topic-id> <local-path> [--yes]`

Intelligently syncs the specified topic with the specified local Markdown file, using the most recently modified as the updated version.

The timestamps of both files will be shown before proceeding, and the user will be prompted to confirm the sync unless `--yes` (or `-y`) is passed.

---

## Categories

### `dsc category list <discourse>`

Lists all categories in the specified Discourse install, with their IDs and names.

List formats are the same as `dsc list`.

Flags:

- `--tree` prints categories in a hierarchy, with subcategories indented under parents.

### `dsc category copy <discourse> <category-id>`

Copies the specified category on the specified Discourse.

Copy behaviour:

- The copied category name is set to `Copy of <original category name>`.
- The copied category slug suffixed with `-copy` (e.g., `staff` -> `staff-copy`).
- All other category fields should match the source category, except the ID which is assigned automatically by Discourse.

`<category-id>` can be found using `dsc category list <discourse>`.

### `dsc category pull <discourse> <category-id> [<local-path>]`

Pulls the specified category into a directory of Markdown files.

If `<local-path>` is omitted, the category is written to a new folder in the current directory (named from the category slug/name). If the path does not exist, it will be created. Files will be named from the topic titles.

### `dsc category push <discourse> <local-path> <category-id>`

Pushes the specified local Markdown files up to the specified category in the Discourse install, creating or updating topics as necessary.

---

## Colour Palettes

### `dsc palette list <discourse>`

Lists available colour palettes (color schemes) on the specified Discourse.

### `dsc palette pull <discourse> <palette-id> [<local-path>]`

Exports the specified palette to a local JSON/YAML file for editing.
If `<local-path>` is omitted, writes `palette-<id>.json` in the current directory.

### `dsc palette push <discourse> <local-path> [<palette-id>]`

Updates the specified palette with the colors in the local file.
If `<palette-id>` is omitted, a new palette is created and the file is updated with the new ID.

---

## Groups

### `dsc group list <discourse>`

Lists all groups in the specified Discourse install, with their IDs, names, and full names.

List formats are the same as `dsc list`.

### `dsc group info <discourse> <group-id> [--format json|yaml]`

### `dsc group copy <source-discourse> <group-id> [--target <target-discourse>]`

Copies the specified group from the source Discourse install to the target Discourse install. If no target is specified, it will copy to the same Discourse.

`<group-id>` can be found using `dsc group list <discourse>`.

Copy behaviour:

- The copied group name is slugified and suffixed with `-copy` (e.g., `staff` -> `staff-copy`).
- The copied group full name is set to `Copy of <original full name>`.
- All other group fields should match the source group, except the ID which is assigned by Discourse.

---

## Backup and Restore

### `dsc backup create <discourse>`

Creates a backup on the specified Discourse install.
It doesn't download the backup, just triggers its creation on the server side.

### `dsc backup list <discourse> [--format <format>]`

Backup list supports the same formats as `dsc list`.

Lists all backups on the specified Discourse install.

### `dsc backup restore <discourse> <backup-path>`

Restores the specified backup on the specified Discourse install.

NOTES: where are these stored locally?
`<backup-path>` can be found using `dsc backup list <discourse>`.

---

## Site Settings

### `dsc setting set <setting> <value> [--tags <tag1,tag2>]`

Updates a site setting across all configured Discourses, optionally filtered by tags (matches any tag, case-insensitive).
Requires an admin API key and username for each target Discourse.

---

## Internals

### dsc.toml Spec

dsc.toml is the configuration file used by dsc.rs to keep track of Discourse installs.

```toml
[[discourse]]
name = "myforum"
baseurl = "https://forum.example.com"

# Optional fields
apikey = "your_api_key_here"
api_username = "system"
fullname = "My Forum" # Discourse site title (optional)
changelog_topic_id = 123
ssh_host = "myforum" # optional SSH config host name used for updates
enabled = true # optional; defaults to true
tags = ["tag1", "tag2"] # optional way to organise installs

# Placeholders are supported for optional fields:
# - "" (empty string) and 0 are treated as unset when loading
# - [] is an empty tag list
#
# Example placeholders (as written by non-interactive `dsc add`):
# apikey = ""
# api_username = ""
# changelog_topic_id = 0
# ssh_host = ""
# tags = []
```

### Release / Distribution

- GitHub Releases ship prebuilt binaries for:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`
  - `x86_64-pc-windows-msvc`
- crates.io publishing is automated in CI on `v*` tags (requires `CARGO_REGISTRY_TOKEN`).
- `CHANGELOG.md` should be updated for each release.
