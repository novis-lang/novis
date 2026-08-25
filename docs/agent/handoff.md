# Handoff

## State

**Stage 0 item 8's vehicle is built; item 8c itself is blocked on a slice nobody had named.** A `.mwlt`
case can now carry more than one file: `--FILE <relative/path>--` repeats, each writing another file into
the case's own working directory beside `case.mwl`, creating the directories along the way. The path is
relative, `/`-separated, refuses a `.`/`..`/empty segment, a drive letter, a leading `/` and the four names
the runner writes itself — so containment is a parse-time property with no sanitiser at the write. The
format's one home is `crates/mwl-test`'s module doc, whose section table and new *More than one file*
section carry the worked ADR 0061 example. Eight new unit tests; `python tools/verify.py` green, 1391 tests.

**`mwl_hir::resolve_program` has no production caller.** `mwl-cli`'s `front_end` (main.rs:222) resolves,
type-checks, lays out and lowers exactly one `SourceId`, calling `mwl_hir::resolve_file`. So `require
'./src/Greeter.mwl'; new App\Greeter()` is `E0303: not declared` from `mwl run` today, and the whole
require/autoload graph — including everything item 8a/8b landed — is exercised only by `mwl-hir`'s own
unit tests. **No conformance case for ADR 0061 can be written until that is wired**, which is why the next
group is the wiring rather than the cases. Verified by hand against a real three-file tree, not inferred.

## Next group — make the front end multi-file, then item 8c's cases

**Shared file set:** `crates/mwl-cli/src/main.rs`, `crates/mwl-hir/src/requires.rs`,
`crates/mwl-types/src/check.rs`, then `tests/conformance/lang/`. The rule is
[`loop-goal.md`](loop-goal.md) § *Stage 0* item 8; the semantics are
[ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) §§ 1, 2 and 5 and
[ADR 0021](../adr/0021-single-file-inclusion-construct.md).

- [ ] **Have `resolve_program` hand back the files it loaded.** It owns a `Vec<Loaded>` (`requires.rs:117`,
      `{ id: SourceId, stmts: Vec<Stmt> }`) and drops it at the end of the walk — the entry file's own
      statements go with it, which is why `front_end` cannot use it today. Return `(Module, Vec<Loaded>)`
      with `Loaded` made public, in a deterministic order (entry first, then load order), and update the
      nine unit tests in `requires.rs` that destructure the current return.
- [ ] **Call it from `mwl-cli`'s `front_end`** (`main.rs:222`), replacing the `resolve_file` on
      `main.rs:234`. `mwl_types::check_program` (`check.rs`, one `&SourceFile` + one `&[Stmt]`) and
      `mwl_types::build_class_layouts` (`main.rs:250`) each take one file's pair; both need to run over the
      set with one shared `TypeInterner`/`ExprTypeTable`, and `Checked` (`main.rs:209`) must carry the set
      so `mwl-ir` lowering sees every declaration. Keep `Checked.id` as the entry point's id — it is what
      names the program.
- [ ] **Then item 8c's `.mwlt` cases**, `tests/conformance/lang/`: a class reached only through `autoload`
      runs; an explicit prefix shadows a `discover` glob; a second root is probed only after the first
      misses; `E0317`'s one-declaration-per-file rule fires with `--EXPECTF-ERROR--`. The format is in
      `crates/mwl-test/src/lib.rs`'s module doc — the *More than one file* section has a working example.
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

`orient.py` printed nothing about `mwl-cli`, which is where this item's real blocker lives. Add
`crates/mwl-cli/**` to `[context] modules` in `loop-goal.toml` — the next session needs `front_end`'s shape
before it can wire anything.
