# Implementation Plan

## Agents

- Run tests after all code changes and ensure tests pass.
- Make regular commits with clear messages reflecting progress.
- Keep documents in spec/ up to date with any changes or decisions made during implementation.

## Source Spec

- Read and implement the CLI described in `spec/spec.md`.

## CLI + Project

- Build the `dsc` CLI in Rust.
- Cover every command in the spec with an end-to-end test.
- Every flag should have a shortened alias for convenience (e.g., `--format` -> `-f`).
- Ensure the CLI syntax adheres to standard conventions and is documented in the Unix style.
- Tab completion scripts for bash, zsh, and fish should be provided.
- Distribution note: Rust CLIs commonly use `clap_complete` to generate shell completion scripts (often at build time into `OUT_DIR` for packaging) and/or commit pre-generated scripts for users/packagers. For now, we will generate scripts into `completions/` and refresh them when CLI flags change.

## Code Comments

- Ensure all public functions and structs have appropriate Rustdoc comments.
- Add inline comments for complex logic sections.

## End-to-End Testing

- Each `dsc` command must have an end-to-end test that:
  - Sends messages to a test Discourse.
  - Verifies the correct response on the forum.
  - Deletes any test data created on the test forum, at the end of testing.
- Test Discourse credentials/config will be provided in `testdsc.toml`.
- Tests should be organised in a modular fashion within the `tests/` directory.

## Configuration Files

- Add a version-controlled example config file for `dsc.toml`.
- Ensure full `dsc.toml` files are gitignored.

## Questions Tracking

- Collect any open questions in `queries.md` for user follow-up.
- Remove resolved questions from `queries.md` as the implementation progresses.
