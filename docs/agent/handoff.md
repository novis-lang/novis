# Handoff

## State

**Goal `finish-response` — stage 3's marker class is on disk, and the raise is not.**
`nvs_hir::errors::FINISH_MARKER` is `Core\Script\Finished`, a second, parentless root of `TREE`
(`crates/nvs-hir/src/errors.rs:121`). It is above and below nothing: `conforms_to` answers an empty
set, it declares no properties and no constructor, and no row extends it — so every `catch` arm in
the tree is false against it through the ordinary self-or-ancestor walk, with no case in the arm
walk and none in `InstanceOf`. Guard: `the_finish_marker_is_a_root_of_its_own_and_is_above_and_below_nothing`.

**`TREE`'s consumers took the second root unedited.** `hierarchy::seed_exception_tree`
(`crates/nvs-hir/src/hierarchy.rs:297`) maps `parent.iter()`, `members::default`
(`crates/nvs-hir/src/members.rs:191`) seeds the root's members off `ROOT` alone,
`nvs_types::error_lib::seed` (`crates/nvs-types/src/error_lib.rs:79`) branches on `qname == root`,
and `build_class_layouts` (`crates/nvs-types/src/layout.rs:185`) builds from `own_properties`, which
is empty for the marker. Only the two tests that spelled the single root needed rewriting — the tree
test in `crates/nvs-hir/src/errors.rs` and the roster test in `crates/nvs-types/src/layout.rs:626`,
where the marker is the one row with no `backtrace` slot. `nvs meta`'s `exceptions` roster carries
the marker last, so `exceptions[0]` is still `Throwable`.

**Stage 3's remaining item cannot land alone, and the next group is why.** The raise from `lower`
has no caller until `Core\Script::finish()` is a registered member: a lowering helper with no call
site is dead code, and stage 3's own `-p nvs-codegen` checks
(`a_finish_runs_every_finally_between_the_call_and_the_root` and its two siblings) compile Novis
source, so they need the member too. The raise and stage 4's member therefore land as one group,
below.

## Next group

**Stage 4: the member, its raise, and the fourth ending** — one file set:
`crates/nvs-stdlib/src/script.rs`, `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-host/src/isolate.rs`.
Tag it stage 4 so `[context.stage.4]`'s overlay applies.

- [ ] **Register `Core\Script::finish()` beside `onExit`**, whose row is at
      `crates/nvs-stdlib/src/script.rs:894` and whose symbol table is `:387`. It takes no argument
      and returns nothing; `rule:concurrency/after-response-outlives-the-connection` is the trigger
      it brings forward, and the goal's stage 4 is the classification it owes.
- [ ] **Lower the call to an ordinary `Terminator::Throw` of a `FINISH_MARKER` instance**
      (`crates/nvs-hir/src/errors.rs:121`), against `crates/nvs-ir/src/lower/mod.rs:239`'s
      `TryFrame::handler`: every enclosing region's finally-and-re-raise block then runs and the
      dispatch's no-arm-matched default hands the same reference onward. `Terminator::Throw`'s
      operand doc (`crates/nvs-ir/src/ir.rs:2586`) already describes this operand.
      `rule:errors/propagation`.
- [ ] **Add the fourth ending to the report and keep the request an ordinary end**: a case beside
      `EXIT_CALL` in `crates/nvs-stdlib/src/script.rs:209` and an arm beside the `Err(EXITED)` one in
      `run_exit_hooks` at `crates/nvs-stdlib/src/script.rs:457`, status `0` with `error` null; the
      request root's `Completion::ok` stays `true` (`crates/nvs-host/src/isolate.rs:747`, the goal's
      own anchor) so both drain gates pass unedited.
      `rule:observability/three-endings-fire-the-exit-queue`.

## Backlog

- Stage 3's two refusals — a `catch` arm and a `throw` naming the marker — are `E0780`'s siblings
  in `crates/nvs-diagnostics/src/lib.rs`, and their checks are filed under `-p nvs-types`.
- `QName::is_reserved_global_class`'s doc (`crates/nvs-hir/src/qname.rs:125`) counts `TREE`'s
  namespaced rows as one and there are now five; a count in a comment is what
  `docs/agent/conventions.md` § *A code comment* refuses.
- Stage 5's record and the rule fragments it creates are untouched — `docs/agent/loop-goal.md`
  § *Stage 5* is the list.
