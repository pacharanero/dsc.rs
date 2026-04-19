# dsc post

Operations on individual posts (not whole topics). Complements `dsc topic pull/push/sync`, which only work on the OP.

## dsc post edit

```text
dsc post edit <discourse> <post-id> [<local-path>]
```

Replaces the body of the specified post with the contents of `<local-path>` (or stdin if omitted, or `-`). Requires either an admin API key or a key whose user is the post author.

```bash
dsc post edit myforum 98765 ./corrected.md
echo "Edited on the fly." | dsc post edit myforum 98765
```

## dsc post delete

```text
dsc post delete <discourse> <post-id>
```

Soft-deletes the post. Deleting the first post of a topic deletes the whole topic. Supports `--dry-run` (or `-n`).

## dsc post move

```text
dsc post move <discourse> <post-id> --to-topic <topic-id>
```

Moves the post to a different topic. `dsc` infers the source topic from the post itself, so you only supply the destination. Prints the URL of the target topic on success. Supports `--dry-run`.

```bash
dsc post move myforum 98765 --to-topic 1525
dsc -n post move myforum 98765 -t 1525   # dry run
```

## Notes

- Post edits and deletes require either admin scope or that the API user owns the post.
- Moving a post that is the first post of its topic will move the whole topic. Discourse's API enforces this.
