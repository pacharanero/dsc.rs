# dsc user

User-level operations. The symmetric group-side commands live under [`dsc group`](group.md).

## dsc user list

```text
dsc user list <discourse> [--listing active|new|staff|suspended|silenced|staged] [--page N] [--format text|json|yaml]
```

Lists users via Discourse's admin users endpoint. Default listing is `active`. Each page is up to 100 rows; use `--page` to iterate. Text mode shows username, id, trust level, and a role flag (admin/mod/suspended/silenced/-).

```bash
dsc user list myforum                         # first page of active users
dsc user list myforum --listing suspended --format json
```

## dsc user info

```text
dsc user info <discourse> <username> [--format text|json|yaml]
```

Shows id, username, name, email (if returned by the server), trust level, role, suspension / silence state, last-seen, created-at, post count, and group count.

## dsc user suspend

```text
dsc user suspend <discourse> <username> [--until <when>] [--reason <text>]
```

Suspends a user. `--until` defaults to `forever`; otherwise pass an ISO-8601 timestamp such as `2026-12-31T00:00:00Z`. `--reason` is stored in the audit log and shown to the user. Honours `--dry-run`.

## dsc user unsuspend

```text
dsc user unsuspend <discourse> <username>
```

Lifts an existing suspension. Honours `--dry-run`.

## dsc user silence

```text
dsc user silence <discourse> <username> [--until <ts>] [--reason <text>]
```

Silences a user (prevents posting; less prominent than suspend — the user can still log in and read). `--until` is an ISO-8601 timestamp; omit for indefinite. Honours `--dry-run`.

## dsc user unsilence

```text
dsc user unsilence <discourse> <username>
```

Lifts an existing silence. Honours `--dry-run`.

## dsc user promote

```text
dsc user promote <discourse> <username> --role admin|moderator
```

Grants the role to the user. Use `--role admin` to make them an admin, `--role moderator` for a moderator. Honours `--dry-run`.

## dsc user demote

```text
dsc user demote <discourse> <username> --role admin|moderator
```

Revokes the role from the user. Honours `--dry-run`.

## dsc user activity

```text
dsc user activity <discourse> <username> [--since <when>] [--types <csv>] [--limit N] [--format text|json|yaml|markdown|csv]
```

Reads the user's public activity feed (`/user_actions.json`) and renders it in the chosen format. Default output is **markdown**, defaulting to **topics + replies** for the last time window you specify.

### Use case: archive your activity to a personal journal Discourse

The command is built for this workflow: keep a personal "journal" Discourse of your own, and use a nightly/weekly cron to roll up everything you posted on another forum into a single topic. Because the output is a markdown list of `- [Title](URL) — date`, it drops straight into a reply on the journal forum without any massaging.

**Weekly roll-up via cron** — one topic per week, subject containing the ISO week:

```bash
dsc user activity someforum marcus --since 7d \
  | dsc topic new myjournalforum 42 --title "Activity for $(date -u +%Y-W%V)"
```

**Appending to a single rolling archive topic** — one topic forever, daily reply:

```bash
dsc user activity someforum marcus --since 24h \
  | dsc topic reply myjournalforum 1234
```

Put either in `crontab -e` or a systemd timer and the archive maintains itself.

### Flags

- `--since` accepts a relative duration (`7d`, `24h`, `30m`, `1w`, `90s`) or an ISO-8601 date/timestamp. Omit to paginate everything available.
- `--types` is a comma-separated list. Default `topics,replies`. Recognised names: `topics`, `replies`, `mentions`, `quotes`, `likes`, `edits`, `responses`.
- `--limit` caps the number of items, independently of `--since`.
- `--format markdown` (default) prints `- [Title](URL) — date` lines; `text` is a wider one-row-per-item human view; `json`, `yaml`, `csv` are structured.

### Scope

Activity endpoint only returns entries the caller is allowed to see, so PMs and private-category posts are filtered out automatically — exactly what you want for a public archive.

## dsc user groups list

```text
dsc user groups list <discourse> <username> [--format text|json|yaml]
```

Lists the groups the given user belongs to, sorted by group name. Default text output is two columns (name, id).

```bash
dsc user groups list myforum alice
# moderators  id:42
# staff       id:3

dsc user groups list myforum alice --format json
```

## dsc user groups add

```text
dsc user groups add <discourse> <username> <group-id> [--notify]
```

Adds the user to the group. With `--notify`, Discourse sends a notification to the user. Honours `--dry-run`.

## dsc user groups remove

```text
dsc user groups remove <discourse> <username> <group-id>
```

Removes the user from the group. Honours `--dry-run`.

## Notes

- Requires an admin API key (group membership changes are admin-scoped).
- The bulk / list-driven variant of adding is [`dsc group add`](group.md#dsc-group-add), which accepts a file of email addresses.
