# Handoff

## State

**Goal `unowned-closures`, and the register is at `unowned: 15`.** `python tools/owners.py` reports
`unowned: 15`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0` and `sections outside Known gaps: 0`
over 101 items; `python tools/owners.py --deferrals` is green.

**`requires.rs` gap 2 is closed: every `match` the require/autoload harvest makes over an
`nvs_syntax::ast` enum names every variant that enum declares.** The wildcard arm each one still
carries is what `#[non_exhaustive]` requires of a cross-crate `match`, and the module doc
(`crates/nvs-hir/src/requires.rs:76`) is that bound's home. Names it did not reach before and does
now: a destructuring leaf's declared type (`[Framework\Row $row] = $pair;`, the new guard test),
`TypeAtom::PropertyKey`'s argument, a hole in an `html` markup literal, an object literal's field
values, and whatever a refused top-level `function`/`const` wrote inside itself.

**`ctor_init.rs` and `lateinit.rs` keep three fewer gaps.** The sheet answered all three with "keep
it", so each is written as the module's own prose bound per the goal's § *Standing decisions*:
a `set`-hooked property carries no definite-initialization obligation, only a class's own `lateinit`
properties are tracked, and a `set`-hooked `lateinit` property is not modeled either. `lateinit.rs`
has no `# Known gaps` block left at all.

**Stage 3's list now holds two builds, and the goal's remaining `unowned` work is one item.** Of the
15 unowned, 14 need answers only the user can give; the last is
`crates/nvs-runtime/src/graph.rs:74` gap 1 — binding a decoded `Core` instance by its own class name,
refused today by `E0496` and `E0711`, both `nvs-types`'. Whether that is a gap to build or a bound to
state is the open question, so `unowned: 0` is a `BLOCKED` the moment it is answered.

## Next group

**Stage 3: the checker and the front end** — one file set: `crates/nvs-types/src/{ctor_init.rs,
signatures.rs}` and `crates/nvs-hir/src/requires.rs`.

- [ ] **A class with no explicit `constructor` refuses every own required property that has no
      default** — `crates/nvs-types/src/ctor_init.rs:75` is the gap and the sheet's answer is to
      build it, so `check_class_init` at `crates/nvs-types/src/ctor_init.rs:131` gains the
      no-constructor arm over `crate::signatures::own_required_properties`
      (`crates/nvs-types/src/signatures.rs:1756`). `rule:classes/definite-property-initialization` is
      the rule, and it promises the check only for a declared constructor today — amend the fragment
      in the same slice. The next free code in the band is `E0824`.
- [ ] **`Probe::tried` folds into the compiled-unit key** — `crates/nvs-hir/src/requires.rs:100` is
      the gap (now gap 2): the probe trace the walk hands back is read by nobody, and what closes it
      is on the cache side — a probed miss becomes a negative `PathEntry`, and the trace's digest
      joins the unit key beside the content hash, at `crates/nvs-config/src/cache.rs:36` and
      `crates/nvs-cli/src/script.rs:164`.
      `rule:packaging/autoload-probes-fold-into-the-cache-key` is what it owes.

## Backlog

- `crates/nvs-runtime/src/graph.rs:74` gap 1 — the last unowned item a session could settle, and the
  one that turns this goal into a `BLOCKED`; the module doc owns it.
- Stage 3's M1 pair — `crates/nvs-syntax/src/lib.rs` gaps 2 and 3 — is untaken; the goal file owns
  the description.
- `crates/nvs-types/src/intrinsics.rs` gaps 1–5 (the prepared-pattern channel) are the stage's
  largest Decided block and the one ADR slot this goal may open.
