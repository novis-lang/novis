# Handoff

## State

**Goal `decided-closures`, stage 3 — the checker and the front end.** `nvs-types` and `nvs-syntax`
are green; `nvs-hir`'s check names two tests and the first of them is now landed, so the stage's
last red check needs only `a_core_name_the_roster_lacks_is_refused_by_the_link_pass`.

A `require` path now folds class constants as well as literal concatenations
(`crates/nvs-hir/src/requires.rs`): a path is read into `Segment`s — cooked literal text, or a
`Class::CONST` resolved through the namespace and imports in force — and each target is folded
against a `ConstTable` **as it stands when that target is resolved**, which is what makes
`require 'paths.nvs'; require Paths::LIB . 'db.nvs';` resolve. `collect_consts` is a declaration-only
pass filling that table from each file the moment the walk reaches it. The bound is the module doc's
own prose, and requires.rs has no `# Known gaps` section left.

`python tools/owners.py --closes decided-closures` names 34 gaps. Nothing is blocked.

## Next group

**Stage 4 of the goal prose, stage `3 the checker` of the checks: `nvs-hir`'s remaining gap, the
`Core` roster** — one file set: `crates/nvs-hir/src/hierarchy.rs` and the entry points that would
hand it the roster. It is two slices because the parameter and the refusal are separable and the
parameter alone touches five other crates. `python tools/peek.py --locate` resolves every anchor
below in one call.

- [ ] **`crates/nvs-hir/src/hierarchy.rs:44` — thread a roster of `Core` names into the link pass,
      behaviour unchanged.** `rule:core-api/reserved-namespace` is what makes `Core` compiler-owned
      and so makes a roster the authority on which of its names exist. The trust branch to feed is
      `crates/nvs-hir/src/hierarchy.rs:430` (`resolve_supertype`'s `is_core()` early return, beside
      the two reserved-global tests, which stay). The roster's source is
      `crates/nvs-stdlib/src/registry.rs:1439` (`CLASSES`, one `.name` per row); `nvs-hir` depends on
      `nvs-diagnostics` and `nvs-syntax` only and must keep doing so, so the names arrive as a slice
      the caller builds. Construction sites inside the crate:
      `crates/nvs-hir/src/requires.rs:216` and `crates/nvs-hir/src/resolve.rs:311`. Public entries to
      widen: `crates/nvs-hir/src/requires.rs:190` (`resolve_program`), `:208`
      (`resolve_program_linted`), and `resolve_file` in `crates/nvs-hir/src/resolve.rs`. Call sites
      outside: `crates/nvs-cli/src/main.rs:1576`, `crates/nvs-lsp/src/document.rs:425`,
      `crates/nvs-types/tests/common/mod.rs:309`, `crates/nvs-codegen/src/lib.rs:2566`,
      `crates/nvs-codegen/tests/common/mod.rs:50`, and four in `crates/nvs-ir/src/lower/tests.rs`
      (`:23`, `:80`, `:114`, `:2260`). **Decide first what a caller with no stdlib passes** — an
      empty slice that means "no `Core` name exists" refuses every `Core` link in those fixtures,
      so the honest shapes are a two-case roster type or a real slice at each site; whichever is
      chosen, say so in the module doc, because "empty means trust" is the invariant a later reader
      cannot see.
- [ ] **`crates/nvs-hir/src/hierarchy.rs:430` — refuse a `Core` link target the roster lacks, and
      strike the gap.** `E_UNDEFINED_CLASS` through `crate::hierarchy::undeclared_name`, the same
      diagnostic a userland name gets, so every link error comes from one pass. The acceptance check
      names `a_core_name_the_roster_lacks_is_refused_by_the_link_pass` under `cargo test -p nvs-hir`;
      `crates/nvs-hir/src/hierarchy.rs:608` is where that crate's tests build a module. Watch for
      false refusals: only `extends`/`implements` reach this branch, and a legal `Core` target that
      `CLASSES` does not hold (a reserved *global* interface is not one — `Comparable` and its
      siblings are `crates/nvs-hir/src/interfaces.rs:44`) would start failing programs that compile
      today. Rewrite the module doc's `**Known gap:**` paragraph as what the pass now does.

## Backlog

- 34 gaps still name this goal — `python tools/owners.py --closes decided-closures` is the list.
- `crates/nvs-syntax/src/lib.rs:96` gap 1, the shape-typed local, is the front end's last one.
- Goal stage 5's prepared-pattern channel has the goal's one ADR slot — `docs/agent/loop-goal.md`
  § *Standing decisions*.
- `crates/nvs-hir/src/requires.rs` gap 2 (`Probe::tried` in the unit key) belongs to a different
  goal — `docs/agent/goals/60-unowned-closures.md:57`.
