# Handoff

## State

**ADR 0010 § 5 runs in both directions, so ADR 0047 § 5 and it are whole for a statically typed operand.**
`$n as Mode` is one free `InstKind::Reinterpret` — an enum is a zero-byte tag over its backing integer —
behind the same membership chain ADR 0047 § 3's named subset gets, built from **every** case of the
declaration (`lower::whole_enum_set`, sorted by the case's constant because `EnumInfo::cases` is a hash
map and an unsorted throw message could not be pinned). An operand that is not already the backing scalar
is converted to it by ADR 0007 § 2's own rows first, by recursion inside `Lowering::convert` rather than a
row per source, so `$f as Rank` and `$s as Rank` each throw naming whichever of the two steps failed. An
operand already at the enum's own representation skips the chain: two *different* enums are never
interconvertible, so it names a case by construction. `crates/mwl-ir/src/lib.rs:306` (gap 20) owns the
design.

**What is left is not about enums.** A `Ty::Tagged` operand converts to `string` and to `decimal` and to
nothing else, so `$any as int`, `$any as Mode` and `$any as Mode::Read|Mode::Write` all panic in
`Lowering::convert` (the last only after running its membership test correctly). That is ADR 0007 § 2's
row, and it is the next group.

`python tools/verify.py` green, 1393 tests; 359 conformance cases pass. Striking `loop-goal.md`
§ *Stage 0* item 5 **empties Stage 0** — all nine are done, so the group after the one below is the first
that comes from Stage 3 again (`examples/collect.mwl`, and `Core\Path` is its cheapest slice). The group
below is still worth taking first: it is three files, it closes gap 20, and every consumer of untyped
input needs it — `Core\Request::query`, `Core\Script::args` and `Core\Json::decode` all hand back `mixed`,
and today nothing can convert one to an `int`.

## Next group — ADR 0007 § 2's checked `mixed` → scalar rows (`mwl-ir` gap 20's remainder)

**Shared file set:** `crates/mwl-runtime/src/helpers.rs` and `crates/mwl-ir/src/ir.rs` for the first,
`crates/mwl-ir/src/lower/expr.rs` and `crates/mwl-codegen/src/emit.rs` for the second, then
`tests/conformance/lang/` for the cases. ADR 0066's non-throwing twin already exists at every anchor
below, so each edit is a sibling of a line already there.

- [ ] **Three throwing helpers beside `Helper::ToIntOrNull`'s three.** `helpers.rs:389`'s `to_int` is the
      row set (it answers `Value::null()` on a miss); the throwing form is the same dispatch with
      `does_not_fit` at the far end, exactly as `mwl_str_to_int` (`helpers.rs:333`) does for a `string`.
      Register each in the symbol table beside `mwl_to_int_or_null` (`helpers.rs:902`), and declare
      `TaggedToInt`/`TaggedToUint`/`TaggedToFloat` beside `Helper::TaggedToString` (`ir.rs:1136`).
- [ ] **Wire them into `convert` and codegen.** The checked-row `match` arm is `lower/expr.rs:743` and the
      helper pick is the `match (from, to)` under it at `:760`; adding `Ty::Tagged` to the source set of
      the `→ Int|Uint|Float` rows is the whole change, and the `_ => panic!` at `:784` then names only
      genuinely unmodeled pairs. Symbol names go beside `Helper::TaggedToString` at `emit.rs:2154`.
      Nothing releases the operand — a `Ty::Tagged` payload is refcounted, so it takes the same
      `is_aliasing_read` release the `(Ty::Tagged, Ty::Str)` row at `:688` already writes.
- [ ] **Conformance cases.** ADR 0007 § 6's own headline —
      `uint $id = Core\Request::query('id') as uint;` — is the shape: a `mixed` holding `"abc"`, `-1` and
      `""` each throws rather than becoming `0`. Put one beside
      `tests/conformance/lang/a-lossy-conversion-throws.mwlt`, and one in `tests/conformance/enum/` for
      `$any as Mode` and `$any as Mode::Read|Mode::Write`, which is what closes gap 20. Then rewrite
      gap 20 as closed; nothing in `loop-goal.md` needs striking, since Stage 0 is already empty.

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

Fixed this session: `[context] adrs` in `loop-goal.toml` still selected ADR 0061's sections two groups
after that work landed, so this group paid for ADR 0010 § 5 not being printed and worked from gap 20's
restatement instead. It now selects `0007 §2`, `0007 §3`, `0007 §6`, `0066 §§1-3` and `0010 §5`, which is
the next group's set. **Swap it again when the group changes** — a stale `adrs` list is the one manifest
field that silently costs every session the same slice.
