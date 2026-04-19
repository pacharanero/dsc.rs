# dsc user

Operations that act from a user's perspective. The symmetric group-side commands live under [`dsc group`](group.md).

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
