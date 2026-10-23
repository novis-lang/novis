`nvs fmt <path>...` rewrites the named files in place. `nvs fmt --check <path>...` (alias `--dry-run`)
writes nothing: it prints which files would change and exits non-zero if any would, in both cases without
touching disk — the mode a CI job or a pre-commit hook runs, mirroring `rustfmt --check`.
`nvs fmt --diff <path>...` prints a unified diff instead of a bare file list, and `--stdin` formats what
it is given. A directory is walked for its `.nvs` files, and the walk skips every directory named `vendor`
under it, the code a project pulls in rather than writes, as `nvs check --fix` leaves it alone. A
`vendor` directory named on the command line is formatted, because naming it is the choice.

These, with the target paths, are the only flags: every one is an I/O mode, and none changes the output
(`rule:tooling/fmt-is-one-canonical-style`). `--check` exits `0` on an already-formatted file because the
formatter is a fixed point (`rule:tooling/fmt-is-idempotent`). A non-zero exit names the file, and
`rule:tooling/fmt-exit-status-says-what-it-found` says what each code means. Formatting and repair are
never one command (`rule:tooling/fmt-is-never-a-diagnostic`).
