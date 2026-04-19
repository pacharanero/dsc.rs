# dsc invite

Send invitations to one or many email addresses.

## dsc invite send

```text
dsc invite send <discourse> <email> [--group <id>] [--topic <id>] [--message <text>]
```

Creates a single invite and prints the magic link Discourse generates. `--group` is repeatable to add the invitee to multiple groups on accept. `--topic` lands them on a specific topic. `--message` attaches a custom note.

```bash
dsc invite send myforum alice@example.com
dsc invite send myforum bob@example.com -g 42 -g 17 -t 1525 -m "Welcome to the team"
dsc -n invite send myforum charlie@example.com   # dry run, no API call
```

## dsc invite bulk

```text
dsc invite bulk <discourse> [<local-path>] [--group <id>] [--topic <id>] [--message <text>]
```

Iterates the same single-invite endpoint per email. Input is one email per line; blank lines and `#` comments (full-line and inline) are ignored; duplicates collapse. Reads stdin when path is omitted or `-`. Group / topic / message flags apply to every invite. Shows a progress bar.

Honours `--dry-run`, which prints the cleaned email list without sending.

```bash
dsc invite bulk myforum ./onboarding.txt -g 42
printf 'alice@example.com\nbob@example.com\n' | dsc invite bulk myforum
dsc -n invite bulk myforum ./onboarding.txt
```

## Notes

- Requires an admin API key; non-admin keys can only invite themselves.
- The bulk command iterates client-side (one HTTP call per email). On large lists the cross-cutting 429 retry handles transient rate limits transparently; if you hit the per-IP nginx limiter you may want to break the file into chunks.
- The Discourse-side equivalent of bulk-adding *existing* users to a group (no invite, no email) is [`dsc group add`](group.md#dsc-group-add).
