# dsc topic

Pull, push, and sync individual topics as local Markdown files.

## dsc topic pull

```
dsc topic pull <discourse> <topic-id> [<local-path>]
```

Pulls the specified topic into a local Markdown file.

If `<local-path>` is omitted, the topic is written to a new file in the current directory (named from the topic title). Directories are created as needed.

## dsc topic push

```text
dsc topic push <discourse> <topic-id> <local-path>
```

Pushes the local Markdown file up to the specified topic, updating it with the file contents.

## dsc topic sync

```
dsc topic sync <discourse> <topic-id> <local-path> [--yes]
```

Intelligently syncs the topic with the local Markdown file, using the most recently modified version as the source of truth.

Timestamps of both copies are shown before proceeding. Pass `--yes` (or `-y`) to skip the confirmation prompt.

## dsc topic reply

```text
dsc topic reply <discourse> <topic-id> [<local-path>]
```

Posts a new reply at the end of the topic. Reads from `<local-path>` if given, otherwise from stdin (equivalent to passing `-`).

Examples:

```bash
dsc topic reply myforum 1525 ./note.md
git log --since=yesterday --oneline | dsc topic reply myforum 1525
```

## dsc topic new

```text
dsc topic new <discourse> <category-id> --title <title> [<local-path>]
```

Creates a new topic in the given category with the specified title. Reads the body from `<local-path>` if given, otherwise from stdin.

Examples:

```bash
dsc topic new myforum 42 --title "Release notes" ./notes.md
df -h | dsc topic new myforum 42 -t "Disk report $(date -I)"
```
