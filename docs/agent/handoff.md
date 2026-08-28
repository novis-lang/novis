# Handoff

## State

**Goal 1, stage 2. The acceptance check runs again.** `loop.py`'s fixture-existence gate had aborted
it before any build for two sessions (`0s over 1 check(s)` in the ledger): goal 1 adds four fixtures
and only `examples/program.nvs` existed. All four are on disk now, so stage 0's three checks and
stage 1's 61 floor checks run before anything else can fail. The playbook's *Tooling* section holds
the trap.

**Where each new fixture stands, checked against its frozen `want` with the driver's own binary:**

- `examples/intrinsics.nvs` — **passes today**, all four lines. ADR 0057 § 4 is why: the fold is only
  an earlier answer, so the runtime path already produces what the prepared one will.
- `examples/routes.nvs` — `E0405`, `Core\Router` has no `url`/`urlAbsolute`. Nothing else in it is
  wrong; the `#[\Core\Route]` half is not even reached, because no scan runs until a `Core\Router`
  member resolves (ADR 0077 § 5).
- `examples/commands.nvs` — `E0726` ×4: `Core\Command` and `Core\Option` are not shape-typed `type`
  aliases. That is item 8's first edit, and the fixture reaches it because it enumerates its
  declaring classes through `Core\Program::implementing<App\Command>()`, which forces § 3's scan.

**Neither table has a reader, and that is a real gap between the ADRs and the frozen outputs.**
ADR 0077 § 4's roster (`match`, `methodsFor`, `url`, `urlAbsolute`) exposes no enumeration of the
route table, ADR 0086 § 6's exposes only `run`/`help`/`completions` — and `run` plus the terminal are
out of this goal's scope while `Core\Cli\Text` carries no member until M8. Both fixtures' frozen
outputs name the tables' contents, so: `routes.nvs` echoes its three route lines and leans on the
*compile-time* half (a literal `url()` name, and a `$params` array not covering the captures, are
compile errors), while `commands.nvs` reads the declaring classes through the enumeration instead. If
a table is meant to be read back at run time, that is a new member and a decision nobody has taken.

**`python tools/verify.py` is still red at `crates/nvs-ir/tests/refusals.rs:179`** — the same 17
unattributed lowering refusals the goal switch orphaned, unrelated to this session, which touched no
Rust. It is the first failure, so the gate stops there and never reaches the `.nvst` trees or clippy.

## Next group

**ADR 0061 § 3's acceptance tests — untouched, and still what stage 0 reports.** One file set:
`crates/nvs-hir/src/hierarchy.rs`, `crates/nvs-types/src/program.rs`, `docs/agent/loop-goal.toml`.

- [ ] **`an_interface_enumeration_is_sorted_by_qualified_name`** in `nvs-hir`. Over `implementors`
      (`crates/nvs-hir/src/hierarchy.rs:463`), whose sort compares *segments*, so the case worth
      pinning is the one a rendered-string sort gets wrong — `App\Sub\A` against `App\Beta`.
      `crates/nvs-hir/src/hierarchy.rs:571` is the existing near-twin; do not rename it. ADR 0061 § 3.
- [ ] **`an_abstract_class_is_not_enumerated`** in `nvs-hir`, over `ClassLinks::concrete`
      (`crates/nvs-hir/src/hierarchy.rs:467` is the filter, `:451` the doc that states the rule).
- [ ] **`an_implementor_without_a_no_argument_constructor_is_named`.** The diagnostic is already
      `E0744` in `crates/nvs-types/src/program.rs`, because constructor arity is a `SignatureTable`
      fact and `nvs-hir` has no signatures. Settle it: either `nvs-hir` grows enough of the harvest to
      answer, or the check moves to the `nvs-types` block at `docs/agent/loop-goal.toml:165`. Moving a
      floor check is the edit that file warns about — say so out loud in the commit either way.

## Backlog

- `routes.nvs` needs `Core\Router::url`/`urlAbsolute` and a `type Core\Route` on
  `nvs_types::derive::ATTRIBUTES` — stage 2 items 3–4, ADR 0077 §§ 1–5 and ADR 0102 § 6.
- `commands.nvs` needs `type Core\Command`/`Core\Option` and § 6's table — stage 2 item 8,
  ADR 0086 § 6.
- **`urlAbsolute`'s origin has no home on disk.** No `nvs.toml` exists at the repo root and `nvs-cli`
  reads none, so ADR 0102 § 6's `[app] origin` must resolve somehow before `routes.nvs`'s last line
  can pass — a decision, not a lookup.
- `crates/nvs-ir/tests/refusals.rs`: 17 unattributed lowering refusals, red at HEAD. The test asks for
  a diagnostic, a closed item, or a standing decision — never an allowlist entry.
- The `nvs-hir (the program scan)` check's third test is homed in the wrong crate —
  `docs/agent/loop-goal.toml:165`.
- M4's residue: the 1000-case conformance corpus count, orders 1–4 —
  `docs/implementation-plan.md` *Open now*.
