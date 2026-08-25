# Handoff

## State

**ADR 0047 § 5 runs whole, enum cases included.** Every `mwl_ir::lower` entry point now takes the run's
`mwl_types::EnumTable`, which `mwl_types::check_program` **hands back** rather than dropping — a caller
that rebuilt it would report ADR 0010 § 1/§ 2's declaration errors twice — and `Lowering::enums` is where
lowering reads a case's constant. `$m as Mode::Read|Mode::Write` over an enum-typed operand emits the same
comparison chain a set of `int` literals gets, run one representation down on the backing integer.
`crates/mwl-ir/src/lib.rs:306` (gap 20) owns the whole design and states what is left.

**What is left is the base conversion, not the membership test.** `Tagged as Enum(_)` and `Int as Enum(_)`
have no arm in `Lowering::convert`, so `$any as Mode::Read|Mode::Write` over a `mixed` runs its test and
then panics converting, and a plain `$n as Mode` never gets that far. That is ADR 0010 § 5's own row and
the next group below.

`python tools/verify.py` green, 1393 tests; 357 conformance cases pass. `loop-goal.md` § *Stage 0* item 5
stays unstruck — it is closed by the group below, not by this one.

## Next group — ADR 0010 § 5's `int` into an enum (`mwl-ir` gap 20's remainder)

**Shared file set:** `crates/mwl-ir/src/lower/expr.rs` alone for the first two, reading
`crates/mwl-types/src/enums.rs`, then `tests/conformance/enum/` for the cases. The enum table is already
in hand — `Lowering::enums` — so nothing needs plumbing.

- [ ] **Give `Lowering::convert` its `Int`/`Uint` → `Enum` row.** The `match (from, to)` is at
      `lower/expr.rs:638` and row 1 (`Enum → backing`) is the line under it at `:639`; the panic naming
      this gap is `:751`. The conversion itself is the same free `InstKind::Reinterpret`; what it owes
      first is the check — throw unless the value is one of the declaration's cases.
- [ ] **Reuse the membership chain for a whole enum.** `closed_literal_set` (`lower/expr.rs:3026`) builds
      an `AcceptedSet` from a union's atoms; a `CheckedTy::Enum` target wants one built from
      `EnumTable::get(qname)`'s whole `EnumInfo::cases` map (`mwl-types/src/enums.rs:82`, `:94`).
      **Sort it** — that map is an `FxHashMap`, so an unsorted set would render in a different order run to
      run and no snapshot of the throw's message would be stable. `lower_literal_membership`
      (`lower/expr.rs:3123`) already reinterprets an enum operand to its backing integer and needs nothing.
      `Tagged → Enum` is the same set tested through `Helper::Identical`, exactly as the literal rows do.
- [ ] **Two conformance cases** beside
      `tests/conformance/lang/a-conversion-into-a-closed-set-of-enum-cases-is-checked-at-run-time.mwlt`
      (this session's, and the shape to copy): `$n as Mode` hitting and missing, and the `mixed` operand.
      Then strike `loop-goal.md` § *Stage 0* item 5 and rewrite gap 20 as closed.

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

`[context] adrs` in `loop-goal.toml` still selects ADR 0061's sections and ADR 0007 §§ 3/6 — last group's
set. This group's item named **ADR 0047 § 3 and ADR 0010 § 5** and neither was printed; the work was
possible only because `mwl-ir`'s own gap 20 restates the design. Swap `0061`'s sections for `0047` §§ 3-5
and `0010` § 5 before the next session, or it pays the same slice cost again.
