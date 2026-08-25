# Handoff

## State

**The front end is multi-file end to end, so ADR 0061 is finally observable from `mwl run`.**
`mwl-cli`'s `front_end` (`main.rs:222`) calls `mwl_hir::resolve_program`, and `mwl_types::ProgramFile`
(`mwl-types/src/lib.rs:249`) is the `{ src, stmts }` pair every phase now takes a **slice** of:
`check_program`, `build_signatures`, `build_enum_table`, `build_const_table` and `build_class_layouts` each
build one table across the whole set before any body is checked. `mwl_ir::lower::lower_program`
(`lower/mod.rs:296`) lowers every file's declarations and gives a script frame to `files[0]` alone;
`lower_file` is now the one-file spelling of it. A `require` in statement position lowers to nothing
(`lower/stmt.rs:265`), because the graph is resolved before lowering starts.

Verified by hand under `.agent-tmp/multifile/`: `require './src/Greeter.mwl'` reaching a static method, and
a class reached only through `autoload 'App' from './src'` in a `require`d bootstrap file, both run and exit
0. `python tools/verify.py` green, 1392 tests.

**Two gaps stay open and are recorded as `mwl-ir` gap 22** (`mwl-ir/src/lib.rs`): a required file's *own*
top-level statements are not run, and ADR 0021 § 3's value form (`$c = require '…';`) has no lowering arm.
Closing either means one frame per file called from the site, which re-opens ADR 0021's "no isolation"
question — deliberately not guessed at here.

## Next group — item 8c's conformance cases, then the harvest widening

**Shared file set:** `tests/conformance/lang/`, then `crates/mwl-hir/src/requires.rs`. The rule is
[`loop-goal.md`](loop-goal.md) § *Stage 0* item 8; the semantics are
[ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) §§ 1 and 2. The `.mwlt` multi-file
format, with a working ADR 0061 example, is `crates/mwl-test/src/lib.rs:46` § *More than one file*.

- [ ] **Item 8c's `.mwlt` cases**, new files under `tests/conformance/lang/` (no registration needed): a
      class reached only through `autoload` runs; an explicit prefix shadows a `discover` glob; a second
      root is probed only after the first misses; `E0317`'s one-declaration-per-file rule fires with
      `--EXPECTF-ERROR--`. Two traps that already cost a session: a promoted constructor parameter is not
      recorded as a property (`signatures.rs` known gaps), so a fixture class needs an explicit
      `private int $n;`, and `--EXPECTF-ERROR--` must reproduce the diagnostic's own indentation.
- [ ] **Add `E0315`/`E0316`/`E0318` cases too** if the first bullet leaves room — duplicate prefix, an
      `autoload` inside an autoloaded file, a malformed glob. Same file set, same `--EXPECTF-ERROR--` shape;
      the codes are declared in `mwl-diagnostics/src/lib.rs` and reported from `mwl-hir/src/autoload.rs`.
- [ ] **Widen the name harvest to attributes** (`requires.rs`, `record_name` / `walk_stmt`) — `mwl-hir`'s
      module doc records it as a known gap: `#[Route(...)]` naming an autoloadable class does not pull that
      file in. Cheap once a case exists to pin it.

## Backlog

- `mwl-ir` gap 22 — a required file's top-level statements, and `$c = require '…';` (`mwl-ir/src/lib.rs`).
- ADR 0061 § 5's probe trace is produced and dropped; folding it into the cache key is ADR 0042's slice.
- `Core` breadth where Stage 3 stopped — `examples/collect.mwl` needs §§ 7-9 and 11-12
  (`docs/implementation-plan.md` § *Open now*).
- ADR 0088's registry-wide qualifier classification, landing with M4S (`docs/adr/0088-*`).
- `docs/spec/02-php-migration.md` is 31% classified, one pass per PHP domain remaining
  (`python tools/check-migration.py`).
- A promoted constructor parameter is not a property in any table — `mwl-types/src/signatures.rs` known
  gaps, and `mwl-types/src/derive.rs` gap 2 depends on it.
