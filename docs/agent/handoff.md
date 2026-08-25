# Handoff

## State

**ADR 0061 is closed end to end — resolution *and* reporting.** § 1's last sentence runs:
`mwl check --autoload-map` prints the resolved prefix → roots map, what a `discover` glob passed over in
silence, and what an explicit prefix shadowed. `mwl_hir::AutoloadMap` keeps the last two rather than
dropping them where they happen, `mwl_hir::resolve_program` hands the map back as its third element, and
`AutoloadMap::render` is the printer — its three-counted-section shape, and why paths are rendered relative
and `/`-separated, are `autoload.rs`'s own module doc.

**Ten conformance cases pin that ADR** (`tests/conformance/lang/`, the eight `*autoload*` plus the two
`*discover*`). The one added this session is
`a-discover-glob-skips-a-directory-that-is-not-a-namespace-segment.mwlt`: a glob sweeping `vendor/`,
`node_modules/`, `_Private/` and `.git/` beside `Plugin/src/` discovers only the last and diagnoses none of
the rest. `requires.rs`'s own tests pin the printer's whole rendering
(`the_rendered_map_names_what_was_skipped_and_what_was_shadowed`), so the group's third item is done too.

`python tools/verify.py` green, 1393 tests. `loop-goal.md` § *Stage 0* item 8 is struck through and now
names only the cache half; **item 5 is the one item left unstruck there**, and its text is stale — the plan
says ADR 0047 §§ 4 and 5 are built, so what item 5 actually still owes is `mwl-ir` gap 20 below.

## Next group — ADR 0047 § 3 / ADR 0010 § 5, the run-time enum-case check (`mwl-ir` gap 20)

**Shared file set:** `crates/mwl-ir/src/lower/mod.rs` and `crates/mwl-ir/src/lower/expr.rs`, reading
`crates/mwl-types/src/enums.rs`, with `tests/conformance/lang/` for the cases. The gap states the whole
design already: `crates/mwl-ir/src/lib.rs:306`.

- [ ] **Plumb `mwl_types::EnumTable` into lowering.** `lower_program` (`lower/mod.rs:298`) and
      `lower_file` (`lower/mod.rs:480`) take the checked tables; the enum table
      (`mwl-types/src/enums.rs:87`, with `EnumTable::case` at `:114` and `backing_of` at `:106`) is the one
      they do not, which is why `ir::ExprInfo::EnumCase` carries a backing value only for a case written as
      an expression. Both remaining rows need it, so this lands once.
- [ ] **`$any as Mode::Read|Mode::Write` runs its membership test.** `lower_checked_ty`
      (`lower/mod.rs:1901`) already folds a same-representation union to that representation;
      `lower_literal_membership` (`lower/expr.rs:3109`, called from `:2984` and `:2988`) emits one
      comparison per member with `Helper::LiteralMismatch` at the far end. The enum-case arm is the same
      shape over each case's backing value. `as ?T` runs no test (ADR 0066) — the gap says why.
- [ ] **ADR 0010 § 5's `int`-into-an-enum row is the same check**, reached from a conversion whose target
      is the enum rather than a subset of its cases; close it in the same slice and say so in the gap.
- [ ] **Two conformance cases**, and then strike `loop-goal.md` § *Stage 0* item 5 with what is built.
      A row the checker accepts is not a row that runs — scratch it under `.agent-tmp/` first (playbook).

## Backlog

- ADR 0061 § 5's probe trace is produced and dropped; folding it into the cache key needs ADR 0042's
  `PathEntry` table (`requires.rs:95`).
- `mwl-ir` gap 22: a required file's own top-level statements, and `require` in value position.
- `mwl-ir` gap 19: `$n + $f` and `$n < $f` still fail in codegen (ADR 0007 § 4's promotion table).
- `examples/collect.mwl` is Stage 3's first failing fixture: `Core\Path` is the cheapest slice, then
  `Encoding`/`Hash`/`Uuid`, then `ObjectSet`/`ObjectMap` (which need `new Core\X<T>()` to parse).
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows, with M4S.
- `docs/spec/02-php-migration.md` is 31% classified; one pass per PHP domain remains
  (`python tools/check-migration.py`).

## Orientation gaps

None left open. The gap the last two handoffs named is closed in this session's commit:
`crates/mwl-hir/src/autoload.rs` is now in `[context] modules` in `loop-goal.toml`. Everything the next
group needs is already selected — `crates/mwl-ir/src/lower/**`, `crates/mwl-ir/src/ir.rs` and
`crates/mwl-types/src/enums.rs` are all in that list.
