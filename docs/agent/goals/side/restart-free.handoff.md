# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stage 2 is complete. Stage 3 has one item left.** A request for a path with a pointer does one
map lookup and makes no file-system call (`Compiler::compiled`). `Compiler::revalidate` runs on one
`nvs-revalidate` thread (`script::watch`), once per `revalidate_freq`, from `nvs serve` and
`nvs run`. A change is held back until the program has been quiet for `settle`. A compile resolves
the entry path through its links once (`real_path`) and reads the program through that real path,
which the trace keeps (`Trace::real`). `describes_the_disk` resolves it again, so both the check
after a background compile (`Compiler::moved`) and every background check treat a switched link as
a moved tree. `nvs serve <file>` keeps its entry unresolved at boot (`serve::one_mount`), and only
its mount root is canonical, so its static files still come from the release it started with.

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes. `live_edit` has 17 cases. The link-switch case was
checked to fail with `real_path` returning the written path (a request got `one second`). It uses a
directory junction on Windows (`mklink /J`, no privilege needed) and a renamed symlink on Unix.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 3: the check leaves the request path** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/tests/live_edit.rs`.

- [ ] **Units stay bounded** (`rule:config/an-edit-reaches-the-next-request-without-a-restart`,
      ADR 0218 § 8). Test `units_held_stay_bounded_after_ten_thousand_edits` in
      `crates/nvs-cli/tests/live_edit.rs:1018`'s file, or as a unit test beside the others in
      `crates/nvs-cli/src/script.rs`, against `Compiler::held` at
      `crates/nvs-cli/src/script.rs:578` and `record` at `crates/nvs-cli/src/script.rs:1160`.
      Ten thousand server edits cost a compile each, so a unit test with `revalidate()` by hand is
      likely the only affordable shape.

## Backlog

- Stage 4, the mount table re-expanded by the same background check (goal file § Stage 4). It
  also owns the link-switch gaps the rule's **What is on disk** names: a `[[server.mount]]` entry
  and root, and `one_mount`'s root (`crates/nvs-cli/src/serve.rs:1771`), are resolved at boot.
- `rule:config/a-startup-default-is-never-flipped`'s `dispatch` and `static` rows are not derived
  from the mode (`crates/nvs-server/src/mount.rs:35`). That is outside this goal.
- A reload that changes `[opcache]` does not reach `Compiler::revalidation`, which is read once at
  `Compiler::new`. That is stage 5's business.
