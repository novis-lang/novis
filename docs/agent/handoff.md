# Handoff

## State

**Stage 0 item 8b is done; 8c — end-to-end cases — is what is left of item 8.** ADR 0061 § 1's lookup runs
as a fixpoint on the same worklist that walks `require`: `mwl_hir::requires::resolve_program` harvests three
things per file (its `require` targets, its `autoload` declarations, every name used where a class is
meant), builds a `mwl_hir::autoload::AutoloadMap` when the `require` graph drains, and pushes each placed
file back onto the worklist. Longest prefix wins, roots probe in declaration order, an explicit prefix
shadows a `discover` glob, and a mis-cased on-disk entry is a **miss** rather than a diagnostic. Four new
codes: `E0315` duplicate prefix, `E0316` `autoload` inside the autoloaded sub-graph, `E0317` § 2's file
shape, `E0318` a malformed glob. Nine unit tests in `requires.rs`; `python tools/verify.py` green, 1383
tests.

Two things are deliberately unfinished and are recorded in `requires.rs`'s own known gaps, not here: § 5's
probe trace is produced (`autoload::Probe::tried`) and dropped, since ADR 0042's `PathEntry` table does not
exist yet, and the name harvest is an over-approximation that does not reach an attribute's name.

**8c has no vehicle yet.** A `.mwlt` case is one file — `crates/mwl-test`'s section table has no auxiliary
-file section — and `tests/` holds no multi-file fixture of any kind, so nothing today can express "a class
reached only through `autoload`" end to end. Deciding that is the first half of the next group.

## Next group — item 8c, and the vehicle it needs

**Shared file set:** `crates/mwl-test/src/lib.rs`, `crates/mwl-test/src/case.rs` (the section parser), `tests/conformance/lang/`. The rule is [`loop-goal.md`](loop-goal.md) § *Stage 0* item 8;
the semantics are [ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) §§ 1, 2 and 5.

- [ ] **Give `.mwlt` an auxiliary-file section** — a repeated `--FILE <relative/path>--` (or `--AUX--`)
      written into a temp tree beside the case's own `--FILE--`, so a case can hold a bootstrap file, an
      autoload root and a class. The format is documented in `crates/mwl-test/src/lib.rs`'s module doc,
      whose section table is at [lib.rs:23](../../crates/mwl-test/src/lib.rs#L23); extend that table in the
      same edit, since it is the format's only home. Prefer this to a `mwl-cli` integration test: a fixture
      that only the Rust suite can read is invisible to the Stage 4 conformance count.
- [ ] **The `.mwlt` cases**, `tests/conformance/lang/`: a class reached only through `autoload` runs; a
      `discover` glob reaches a second module; a vendor root is overridden by an earlier root; `E0317` for
      a file declaring two things; `E0316` for an `autoload` in an autoloaded file. Never `--ORACLE--` here
      (playbook § *Writing a test case*). Diagnostic cases need `--EXPECTF-ERROR--` with the diagnostic's
      own indentation.
- [ ] **Widen the name harvest to attributes** if it is cheap once the cases exist —
      `requires.rs`'s `walk_class_members` never looks at `ClassMember::attributes`, so
      `#[Framework\Route]` does not autoload. Anchors: [requires.rs:712](../../crates/mwl-hir/src/requires.rs#L712)
      (`walk_class_members`), [requires.rs:458](../../crates/mwl-hir/src/requires.rs#L458) (`walk_type`).

## Backlog

- `mwl-ir` gap 12's whole remainder: a `Core`-owned class is exempt from `require_stringable`, so
  `echo $someCoreObject` panics. Saying which `Core` classes stringify is `mwl_stdlib::registry`'s answer.
- `mwl-ir` gap 21: a binding declared at the opaque `object` top has no representation arm, so
  `object $o = $obj;` panics — that gap's text says what a session landing it owes.
- ADR 0061 § 5's probe trace, folded into ADR 0042's cache key — `mwl-hir`'s `requires` known gaps.
- ADR 0094's one uncovered shape: a promoted constructor parameter is no table's property, so nothing
  resolves it to check — `mwl-types`' `signatures` known gaps. `private(set)` is § 3's write half and is
  not modeled at all.
- ADR 0043 § 4's `by $field` delegation exempts a whole class from conformance — `mwl-types`'
  `conformance` module doc.
- `mwl-ir` gap 19: `$n + $f` and `$n < $f` type-check and still fail in codegen.

## Orientation gaps

`[context]` still has **no `spec` field**, so nothing selects a `docs/spec/` section; every `Core`-breadth
item will pay for that read out of budget. This session also needed `crates/mwl-syntax/src/ast.rs`'s
shapes (`TypeAtom`, `NewTarget`, `ImplementsClause`, `PropertyHook`) — `modules` names the file, but its
one-line map entry cannot carry a variant list, so budget a targeted `grep -n` rather than treating it as a
manifest gap. `adrs` naming ADR 0061 §§ 1, 2, 4, 5 was exactly right.
