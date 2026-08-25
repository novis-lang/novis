# Handoff

## State

**ADR 0061 is observable from `mwl run` and pinned by seven conformance cases.** All seven live in
`tests/conformance/lang/`; six carry `autoload` in the name and the seventh is
`a-malformed-discover-glob-is-a-compile-error.mwlt`. Between them they pin § 1's lookup (a class no
`require` names is found by its prefix and runs, including a two-segment name under a subdirectory), the
shadowing rule (an explicit prefix beats a `discover` glob that would produce the same one), ordered
probing (a prefix's second root answers only where the first misses), and the four diagnostics —
`E0315` duplicate prefix, `E0316` an `autoload` in an autoloaded file, `E0317` § 2's
one-declaration-per-file rule, `E0318` a malformed glob — each from a `--EXPECTF-ERROR--` case.

`python tools/verify.py` green, 1392 tests; `mwl test tests/conformance/` 353 passed.

**Two gaps stay open and are recorded as `mwl-ir` gap 22** (`mwl-ir/src/lib.rs`): a required file's *own*
top-level statements are not run, and ADR 0021 § 3's value form (`$c = require '…';`) has no lowering arm.
Closing either means one frame per file called from the site, which re-opens ADR 0021's "no isolation"
question — deliberately not guessed at here.

**The name harvest is still an over-approximation with a hole in it**, and that is the last thing between
`loop-goal.md` § *Stage 0* item 8 and a strike-through: an attribute's name reaches nobody, so a class
named only by `#[Route(...)]` does not autoload. `mwl-hir`'s module doc (`requires.rs:90`) owns the gap.

## Next group — the attribute harvest, then the two remaining § 1 shapes

**Shared file set:** `crates/mwl-hir/src/requires.rs`, then `tests/conformance/lang/` for each case. The
rule is [ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) § 1; the `.mwlt` multi-file
format is `crates/mwl-test/src/lib.rs:46` § *More than one file*, and the seven cases above are the shape
to copy. `mwl_syntax::ast::AttributeGroup` is `ast.rs:410`, `Attribute` is `ast.rs:419`, and every
declaration site carries a `Vec<AttributeGroup>` under the field name `attributes`.

- [ ] **Widen the name harvest to attributes.** `walk_stmt` (`requires.rs:545`) already reaches
      `ClassDecl`/`InterfaceDecl`/`EnumDecl` at `requires.rs:651-670` and their members through
      `walk_class_members`; none of those arms looks at `decl.attributes`. Feed each `Attribute`'s name to
      `record_name` (`requires.rs:468`) and its argument expressions to the existing expression walk. Then
      a `.mwlt` case: an attribute on a class naming a class that lives only under an autoload root, with
      the run proving the file was pulled in. Direction is always "harvest more" — a missed name costs a
      class that fails to autoload, a spurious one costs a file that is loaded and ignored.
- [ ] **A case for the exact-spelling probe.** § 1 compares the on-disk entry's name exactly, so
      `mailer.mwl` is a *miss* for `App\Mailer` and not a diagnostic — `spelled_exactly`
      (`autoload.rs:345`). Pin it as an `E0303`-not-declared `--EXPECTF-ERROR--` case, which is what keeps
      a case-insensitive Windows filesystem from accepting what Linux refuses.
- [ ] **A case for a skipped `discover` directory.** § 1 skips a matched directory whose name is not a
      legal `PascalCase` segment in silence — `is_namespace_segment` (`autoload.rs:333`). A glob over a
      tree holding `vendor/` and `Plugin/src/` should discover only the second, with no diagnostic for the
      first.

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

`orient.py` printed everything this session needed. One thing worth adding to `[context]` for the group
above: `crates/mwl-hir/src/autoload.rs` is not in the `modules` selector, and it owns both diagnostics the
last two bullets pin.
