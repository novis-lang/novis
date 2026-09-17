# Handoff

## State

**Goal `decided-closures`, stage 3 — the checker and the front end.** Of that stage's three
acceptance checks, `nvs-types` and `nvs-syntax` are green; `nvs-hir`'s two tests are not written yet
and are the next group.

The front end's four gaps are down to one. A `type` alias's name is PascalCase at both sites one is
written (`crates/nvs-syntax/src/casing.rs`, category `"type alias"`, one production so one answer),
and `rule:core-api/identifier-casing`'s scope table and `rule:types/type-alias` were amended in that
slice. PHP's `use function` / `use const` are refused by name — `E0252`
(`E_IMPORT_OF_FUNCTION_OR_CONST_UNSUPPORTED`) on the keyword, the path behind it still parsed — while
`function` stays an ordinary name segment everywhere else, which is what
`crates/nvs-syntax/src/token.rs`'s new `keywords!` table makes testable: the enum, `from_lowercase`,
`name()` and `ALL` all come from one row per word, so a sweep over every spelling cannot go stale.

`python tools/owners.py --closes decided-closures` names 35 gaps, one of them in `nvs-syntax`
(`crates/nvs-syntax/src/lib.rs:96`, the shape-typed local). Nothing is blocked.

## Next group

**Stage 4 of the goal prose, stage `3 the checker` of the checks: `nvs-hir`'s two gaps** — one file
set: `crates/nvs-hir/src/`, with that crate's own tests. Landing both turns the stage's last red
check green.

- [ ] **`crates/nvs-hir/src/requires.rs:104` — fold a `const` and a literal concatenation in a
      `require` path before the graph walk.** `rule:statements/require-is-the-only-inclusion-construct`
      is what a static path buys; the walk is `crates/nvs-hir/src/requires.rs:349` and its two
      siblings, which call `check_declarations` per file, and the folder runs before the checker's own.
      A class constant is the only constant there is, so reading one means resolving a class out of the
      table this walk is building — the decided answer is a small folder ahead of the walk, not a
      second resolver. The acceptance check names
      `a_require_path_folds_a_const_and_a_literal_concatenation` under `cargo test -p nvs-hir`.
- [ ] **`crates/nvs-hir/src/hierarchy.rs:44` — hand `nvs-hir` a roster of `Core` names at
      construction.** Today a target under `Core` is trusted because `QName::is_core` is a spelling
      test and this crate may not name `nvs_stdlib::registry` (`rule:core-api/core-means-always-present`
      is what the roster asserts); the decided answer passes the names in as a slice, so every link
      error comes from one pass and no dependency is added. The acceptance check names
      `a_core_name_the_roster_lacks_is_refused_by_the_link_pass`.

## Backlog

- `crates/nvs-syntax/src/lib.rs:96` — the shape-typed local's targeted error; that bullet now names
  the tell and the false positive the scan must not claim.
- `crates/nvs-stdlib/` holds most of the goal's remaining gaps — `python tools/owners.py --closes
  decided-closures` is the list, `docs/agent/loop-goal.md` § *Standing decisions* the three ways out.
