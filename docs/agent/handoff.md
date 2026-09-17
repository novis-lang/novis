# Handoff

## State

**Goal `decided-closures`, stage 3 — one gap left in the whole goal.** `python tools/owners.py
--closes decided-closures` now names **1**: `crates/nvs-diagnostics/src/embedded.rs:30`, autoload
roots resolved into a bundle at build time. Stage 4 — the library — owns none, and the other two
stage-6 gates are green.

**`{x: int} $point;` is `E0134` now**, naming `type Point = {x: int}; Point $point;`. The tell is
`crates/nvs-syntax/src/parser/stmt.rs`'s `at_shape_typed_local`: a braced run that opens like a field
list *and* a variable after the **matched** `}`, so a block a variable happens to follow
(`{ echo 1; } $x = 1;`) keeps its ordinary parse and a discarded object literal keeps `E0117`. It is
the only place the parser looks further ahead than three tokens, bounded by that module's
`SHAPE_TYPED_LOCAL_SCAN_LIMIT`, and the `Parser` doc comment states it. The language fact's home is
`rule:types/shape-type`'s type-expression bullet: a local is the one slot where naming the shape
first is required rather than optional.

## Next group

**Stage 3: autoload roots resolved into the bundle** — one file set: `crates/nvs-cli/src/bundle.rs`,
`crates/nvs-hir/src/autoload.rs` and the gap paragraph in `crates/nvs-diagnostics/src/embedded.rs`.

- [ ] **Freeze the autoload root set into the payload at build time** —
      `crates/nvs-cli/src/bundle.rs:148` (`build`) collects the payload from the `require` graph
      alone, which is why a bundled program that reaches a name only through a root does not resolve
      it. `crates/nvs-hir/src/autoload.rs:351` (`enumerate`) already answers with every
      `(QName, PathBuf)` the roots declare, which is the list to append. Rule:
      `rule:programs/no-runtime-autoload` (the file graph closes while compiling) and
      `rule:packaging/a-bundled-require-resolves-at-build-time`.
- [ ] **Answer a bundled probe out of the payload rather than the disk** —
      `crates/nvs-hir/src/autoload.rs:459` and `crates/nvs-hir/src/autoload.rs:576` are the two
      `std::fs::read_dir` calls that list real directories; they read the embedded table first, the
      way `crate::SourceMap::load` already does (`crates/nvs-diagnostics/src/embedded.rs:7`).
      Rule: `rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`.
- [ ] **Strike the gap and pin it** — delete the `**Known gap.**` paragraph and its `Decided:`/owner
      tags at `crates/nvs-diagnostics/src/embedded.rs:30`, rewriting it as what the module now does,
      and add the case that a bundled program reaches a class only an autoload root declares.

## Backlog

- That gap is the goal's last: `docs/agent/loop-goal.md` § *The six steps* names the three gates a
  `DONE` claim runs first (`verify.py --doc`, `owners.py --closes`, `playbook.py --closes`).
- `crates/nvs-syntax/src/lib.rs` now has no `# Known gaps` section at all — a future one starts the
  heading again rather than editing a leftover.
