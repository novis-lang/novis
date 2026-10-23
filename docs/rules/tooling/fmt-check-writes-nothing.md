`nvs fmt <path>...` rewrites the named files in place. `nvs fmt --check <path>...` (alias `--dry-run`)
writes nothing: it prints which files would change and exits non-zero if any would, in both cases without
touching disk — the mode a CI job or a pre-commit hook runs, mirroring `rustfmt --check`.
`nvs fmt --diff <path>...` prints a unified diff instead of a bare file list, and `--stdin` formats what
it is given. A directory is walked for its `.nvs` files with the project walk
`rule:tooling/nvs-code-answers-about-the-project` describes, the one `nvs code` and the editor use: it
skips what a `.gitignore` or `.ignore` names, and every `vendor` and `target` directory and dot-directory
under it. `vendor` is the code a project pulls in rather than writes, as `nvs check --fix` leaves it
alone. A directory named on the command line is formatted whatever it is called, because naming it is
the choice, and a file named there is formatted even when an ignore file names it.

These, with the target paths, are the only flags: every one is an I/O mode, and none changes the output
(`rule:tooling/fmt-is-one-canonical-style`). `--check` exits `0` on an already-formatted file because the
formatter is a fixed point (`rule:tooling/fmt-is-idempotent`). A non-zero exit names the file, and
`rule:tooling/fmt-exit-status-says-what-it-found` says what each code means. Formatting and repair are
never one command (`rule:tooling/fmt-is-never-a-diagnostic`).
