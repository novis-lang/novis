`nvs fmt <path>...` rewrites the named files in place. `nvs fmt --check <path>...` (alias `--dry-run`)
writes nothing: it prints which files would change and exits non-zero if any would, in both cases without
touching disk — the mode a CI job or a pre-commit hook runs, mirroring `rustfmt --check`.
`nvs fmt --diff <path>...` prints a unified diff instead of a bare file list, and `--stdin` formats what
it is given.

These, with the target paths, are the only flags: every one is an I/O mode, and none changes the output
(`rule:tooling/fmt-is-one-canonical-style`). `--check` exits `0` on an already-formatted file because the
formatter is a fixed point (`rule:tooling/fmt-is-idempotent`); a non-zero exit names the file, and it means
one thing only, since formatting and repair are never one command
(`rule:tooling/fmt-is-never-a-diagnostic`).
