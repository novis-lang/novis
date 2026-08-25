# Handoff

## State

**`resolve_program` now hands its files back, so half of the multi-file wiring is built.** It returns
`(Module, Vec<Loaded>)`; `mwl_hir::Loaded` is the public `{ id: SourceId, stmts: Vec<Stmt> }` pair for one
file, in **entry-first load order** — the entry file's own statements are in the vector, which is what
`front_end` needed and could not get before. The order contract and why the walk keeps the statements at all
live in `crates/mwl-hir/src/requires.rs`'s module doc and on `resolve_program` itself. One new unit test
pins entry-first + each file exactly once over a diamond; `python tools/verify.py` green, 1392 tests.

**The consumer is still unwired, and that is the whole remaining blocker for ADR 0061's conformance cases.**
`mwl-cli`'s `front_end` (`main.rs:222`) calls `mwl_hir::resolve_file` over exactly one `SourceId`, so
`require './src/Greeter.mwl'; new App\Greeter()` is still `E0303` from `mwl run`, and the require/autoload
graph is exercised only by `mwl-hir`'s own unit tests. Nothing in `tests/conformance/` can reach it yet.

## Next group — make the front end multi-file, then item 8c's cases

**Shared file set:** `crates/mwl-cli/src/main.rs`, `crates/mwl-types/src/check.rs`,
`crates/mwl-ir/src/lower/mod.rs`, then `tests/conformance/lang/`. The rule is
[`loop-goal.md`](loop-goal.md) § *Stage 0* item 8; the semantics are
[ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) §§ 1, 2 and 5 and
[ADR 0021](../adr/0021-single-file-inclusion-construct.md).

- [ ] **Call `resolve_program` from `mwl-cli`'s `front_end`** (`main.rs:222`), replacing the `resolve_file`
      on `main.rs:234`. `mwl_types::check_program` (`check.rs`, one `&SourceFile` + one `&[Stmt]`) and
      `mwl_types::build_class_layouts` (`main.rs:250`) each take one file's pair; both must run over the
      whole `Vec<Loaded>` with one shared `TypeInterner`/`ExprTypeTable`, and `Checked` (`main.rs:209`) must
      carry the set. Keep `Checked.id` as the entry point's id — it names the program.
- [ ] **Decide what a non-entry file's top-level statements lower to.** `mwl_ir::lower::lower_file`
      (`main.rs:288`) synthesizes one `<script>` frame per file (`SCRIPT`, `main.rs:270`), and N files cannot
      all be `<script>`. ADR 0021 § *no isolation* says a required file's statements run at the `require`
      site; the cheap correct answer is that only the entry gets a script frame and every other file
      contributes its *declarations* only, with a diagnostic or a documented gap for a required file that
      writes a bare statement. Record whichever is chosen in `mwl-ir`'s module doc.
- [ ] **Then item 8c's `.mwlt` cases**, `tests/conformance/lang/`: a class reached only through `autoload`
      runs; an explicit prefix shadows a `discover` glob; a second root is probed only after the first
      misses; `E0317`'s one-declaration-per-file rule fires with `--EXPECTF-ERROR--`. The format is in
      `crates/mwl-test/src/lib.rs`'s module doc — its *More than one file* section has a working example.
- [ ] **Widen the name harvest to attributes** (`requires.rs`, `record_name` / `walk_stmt`) if it is cheap
      once the cases exist; `mwl-hir`'s module doc records it as a known gap today.

## Backlog

- ADR 0061 § 5's probe trace is produced (`autoload::Probe::tried`) and dropped; folding it into the cache
  key needs ADR 0042's `PathEntry` table — that ADR's own slice.
- `Core` breadth where Stage 3 stopped: `examples/collect.mwl` needs spec §§ 7-9 and 11-12
  (`docs/implementation-plan.md` § *Open now*).
- ADR 0088's qualifier classification for every `mwl-stdlib` member row (`docs/implementation-plan.md`).
- `docs/spec/02-php-migration.md` is 31% classified; one pass per PHP domain remains
  (`python tools/check-migration.py`).
- `mwl-ir` gap 18: an abandoned generator never runs the `finally` it is suspended inside.

## Orientation gaps

`crates/mwl-cli/src/main.rs` is now in `[context] modules` in `loop-goal.toml`, so `orient.py` prints its
map line — the gap the previous handoff named is closed. `mwl-ir/src/lower/mod.rs` is already selected, so
the next group needs no further manifest change.
