# Handoff

## State

**ADR 0061's resolution half is closed, and `loop-goal.md` § *Stage 0* item 8 is struck through.** The
name harvest now reaches every declaration site's `#[...]` groups — `mwl_hir::requires::walk_attributes`,
fed from `ClassDecl`/`InterfaceDecl`/`EnumDecl`, each `EnumCase`, every class member, every property hook
and every parameter — so a class written only as `#[Route(...)]` is placed by its prefix and its file is
loaded and checked. `requires.rs`'s own module doc owns what the harvest still cannot see.

**Nine conformance cases pin that ADR now** (`tests/conformance/lang/`, the eight `*autoload*` plus
`a-malformed-discover-glob-is-a-compile-error.mwlt`). The two added this session are
`a-class-named-only-by-an-attribute-is-autoloaded.mwlt` (the attribute-named file is loaded, so § 2's
one-declaration rule reports `E0317` from a file nothing else names) and
`an-autoload-probe-compares-the-on-disk-spelling-exactly.mwlt` (`src/mailer.mwl` is a *miss* for
`App\Mailer`, reported as `E0303` at the naming site and never as a path diagnostic).

`python tools/verify.py` green, 1392 tests; `mwl test tests/conformance/` 355 passed.

**What is left of ADR 0061 is reporting, not resolution:** § 1's `mwl check --autoload-map` does not exist,
and § 5's probe trace is produced and dropped. Separately, `mwl-ir` gap 22 is unchanged — a required
file's own top-level statements are not run, and ADR 0021 § 3's `$c = require '…';` has no lowering arm.

## Next group — ADR 0061's reporting half

**Shared file set:** `crates/mwl-hir/src/autoload.rs`, then `crates/mwl-cli/src/main.rs`, with
`tests/conformance/lang/` for the one case. The rule is
[ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) § 1's last two bullets. `AutoloadMap`
and its `Site`/`SiteKind`/`Probe` types are `autoload.rs`'s own module doc; the `.mwlt` multi-file format is
`crates/mwl-test/src/lib.rs:46` § *More than one file*, and the nine cases above are the shape to copy.

- [ ] **A case for a skipped `discover` directory.** § 1 skips a matched directory whose name is not a
      legal `PascalCase` segment *in silence* — `is_namespace_segment` (`autoload.rs:333`), reached from
      the glob expansion just above it. A glob over a tree holding `vendor/` and `Plugin/src/` discovers
      only the second, with no diagnostic for the first; the run proves it by calling into `Plugin\…`.
      Note `.mwlt` writes only files, so a directory is created by writing a file inside it.
- [ ] **`mwl check --autoload-map`.** § 1's last sentence promises it and nothing implements it: no flag in
      `mwl-cli`, no printer in `mwl-hir`. It has to print the resolved map *including what was skipped and
      what was shadowed*, which means `AutoloadMap` has to keep both rather than dropping them at
      construction — the same widening `Probe` already models for misses. Decide the output shape in
      `autoload.rs`'s module doc, one line per prefix with its ordered roots.
- [ ] **Pin the printer in `mwl-hir`'s own `tests/`**, not in `tests/conformance/` — a `.mwlt` case runs a
      program and cannot invoke `mwl check`. An `insta` snapshot over a map built from a fixture tree is
      the cheapest shape; a moved module takes its snapshots with it (playbook).

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

`orient.py` covered this session. The gap the last handoff named is still open and matters more for the
group above: `crates/mwl-hir/src/autoload.rs` is missing from `[context] modules` in `loop-goal.toml`, and
it owns every line the next three bullets touch. `crates/mwl-cli/src/main.rs` is already in the selector.
