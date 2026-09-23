# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stage 2 is complete, and stage 3's first item has landed.** A request for a path with a pointer
does one map lookup and makes no file-system call (`Compiler::compiled`). `Compiler::revalidate`
walks steps 2 to 5 for every pointer. `script::watch` runs it on one `nvs-revalidate` thread,
once per `revalidate_freq`, from `nvs serve` and `nvs run`. A change is held back until the
program has been quiet for `settle` (`Compiler::unsettled`). The newest `mtime` among the entry
file and the files its traces read decides that. A failed check leaves the pointer and names
the failure in `PathEntry::failed`. A path that never compiled is still looked at by each request
that names it. `nvs test` starts no watcher, because nothing edits a file mid-suite.
`rule:packaging/autoload-probes-fold-into-the-cache-key` is now `shipped`.

Two calls in ADR 0218 are mine and are not confirmed with the user. A mount re-expansion that
meets a match boot would refuse logs it and leaves it out (§ 9). Keeping the replaced unit
(§ 7, § 8) was also my call. The `FLOOR` of 10ms between two watcher passes is also mine. The
unit tests call `revalidate()` by hand after each edit. `live_edit` has 14 cases, and all of them
pass.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 3: the check leaves the request path** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/tests/live_edit.rs`.

- [ ] **A file that moves during the compile discards that compile, and it is retried once quiet**
      (`rule:config/an-edit-reaches-the-next-request-without-a-restart`, ADR 0218). The settle wait
      is on disk at `crates/nvs-cli/src/script.rs:830`. After `compile` at
      `crates/nvs-cli/src/script.rs:1180`, `take` at `crates/nvs-cli/src/script.rs:710` should
      check every file of the new trace again, and the entry file too. If one moved, it drops the
      `Compiling` placeholder, lands the flight, and records nothing, so the next pass retries.
      Only the background path should discard, because a cold request has nothing else to serve.
      Tests `a_copy_that_pauses_less_than_settle_is_compiled_once_from_the_finished_tree` and
      `a_file_that_changes_during_a_compile_discards_that_compile` go at the end of
      `crates/nvs-cli/tests/live_edit.rs:863`.
- [ ] **The entry path is resolved through every link before and after a compile** — test
      `a_link_switch_during_a_compile_never_builds_a_program_from_both_releases`, at
      `crates/nvs-cli/src/script.rs:1180`. On Windows the test needs a junction or a symlink that
      the test can create. Nothing has checked this yet.
- [ ] **Units stay bounded** — test `units_held_stay_bounded_after_ten_thousand_edits`, against
      `Compiler::held` and `record` at `crates/nvs-cli/src/script.rs:1110`.

## Backlog

- Stage 4, the mount table re-expanded by the same background check (goal file § Stage 4).
- `rule:config/a-startup-default-is-never-flipped`'s `dispatch` and `static` rows are not derived
  from the mode (`crates/nvs-server/src/mount.rs:35`). That is outside this goal.
- A reload that changes `[opcache]` does not reach `Compiler::revalidation`, which is read once at
  `Compiler::new`. That is stage 5's business.
