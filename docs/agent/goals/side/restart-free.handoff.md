# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stage 2 is complete. Stage 3 has two items left.** A request for a path with a pointer does one
map lookup and makes no file-system call (`Compiler::compiled`). `Compiler::revalidate` runs on one
`nvs-revalidate` thread (`script::watch`), once per `revalidate_freq`, from `nvs serve` and
`nvs run`. A change is held back until the program has been quiet for `settle`
(`Compiler::unsettled`). After a background compile, `Compiler::moved` reads the entry file again
and checks the new trace. If anything moved, the compile is discarded (`Compiler::release`), nothing
is recorded, and the next pass retries. A cold request keeps what it compiled. `nvs test` starts no
watcher.

Two calls in ADR 0218 are mine and are not confirmed with the user. A mount re-expansion that
meets a match boot would refuse logs it and leaves it out (§ 9). Keeping the replaced unit
(§ 7, § 8) was also my call. The `FLOOR` of 10ms between two watcher passes is also mine.
`live_edit` has 16 cases, and all of them pass. `a_file_that_changes_during_a_compile_discards_that_compile`
was checked to fail with the discard turned off. Its `heavy()` helper (2000 functions, about 2s of
codegen on a debug build after the warning) is how a case lands a write inside a compile.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 3: the check leaves the request path** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/tests/live_edit.rs`.

- [ ] **The entry path is resolved through every link before and after a compile**
      (`rule:config/an-edit-reaches-the-next-request-without-a-restart` § *An atomic deploy is
      atomic*, ADR 0218). `compile` at `crates/nvs-cli/src/script.rs:1217` should canonicalize
      the entry path once and read the program through that real directory. After the compile,
      `moved` at `crates/nvs-cli/src/script.rs:995` should resolve the path again, and a different
      answer counts as moved. Test
      `a_link_switch_during_a_compile_never_builds_a_program_from_both_releases` goes after
      `crates/nvs-cli/tests/live_edit.rs:938`, and can reuse `heavy()` and `requiring_lib()`.
      On Windows it needs a junction or a symlink the test can create. Nothing has checked this.
- [ ] **Units stay bounded** — test `units_held_stay_bounded_after_ten_thousand_edits`, against
      `Compiler::held` and `record` at `crates/nvs-cli/src/script.rs:1147`.

## Backlog

- Stage 4, the mount table re-expanded by the same background check (goal file § Stage 4).
- `rule:config/a-startup-default-is-never-flipped`'s `dispatch` and `static` rows are not derived
  from the mode (`crates/nvs-server/src/mount.rs:35`). That is outside this goal.
- A reload that changes `[opcache]` does not reach `Compiler::revalidation`, which is read once at
  `Compiler::new`. That is stage 5's business.
