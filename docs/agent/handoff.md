# Handoff

## State

**Goal 1 of the parity program: ADR 0061 § 3 is landed end to end and `examples/program.nvs` runs.**
`Core\Program` has a registry row (`crates/nvs-stdlib/src/program.rs`), the call is expanded while
checking (`crates/nvs-types/src/program.rs`), and `nvs-ir` emits the array literal of `new`
expressions from what the checker recorded. A program that writes `implementing<T>()` now enumerates
the classes its autoload roots declare, filters them to the non-abstract implementors, and
constructs one of each in fully-qualified-name order.

**The shape of the fold is `crate::retrieval`'s, with one difference worth knowing.** Retrieval
records `ExprInfo::CoreConst` because its answer is a constant; this records
`ExprInfo::ProgramInstances { classes, ctors }` because a `new` allocates. Both replace the
`ExprInfo::Call` for the same span, so `nvs_stdlib::program`'s aborting body is the alarm if one
ever reaches a helper. The `ctors` half carries the *declaring* constructor's label for
`ExprInfo::New`'s reason: `nvs-ir` cannot re-walk the hierarchy.

**The acceptance list's `nvs-hir (the program scan)` check is the next thing in the way, and one
third of it is homed in the wrong crate.** It names three tests, none of which exists. Two are
`nvs-hir`'s to write over `implementors` (`crates/nvs-hir/src/hierarchy.rs:463`), whose existing
tests already assert overlapping facts under other names. The third,
`an_implementor_without_a_no_argument_constructor_is_named`, is § 3's constructor diagnostic — and
that landed this session as `E0744` in `nvs-types`, because constructor arity is a
`SignatureTable` fact and `nvs-hir` has no signatures. Settle which way that goes before writing it:
either `nvs-hir` grows enough of the harvest to answer, or the check moves to the `nvs-types` block
two entries below it in `docs/agent/loop-goal.toml:165`. Moving a floor check is the kind of edit
that file warns about, so say so out loud in the commit either way.

**`python tools/verify.py` is still red at `crates/nvs-ir/tests/refusals.rs`, and not from this
work** — the same 17 unattributed lowering refusals the previous session reported, orphaned by the
goal switch. It is the first failure, so the gate stops there and never reaches the `.nvst` trees or
clippy; this session ran the rest by hand: build, fmt, `-p nvs-stdlib -p nvs-types -p nvs-hir` all
green, conformance 881/881, differential 189/189, clippy clean across the workspace.

## Next group

**ADR 0061 § 3's acceptance tests.** One file set: `crates/nvs-hir/src/hierarchy.rs`,
`crates/nvs-types/src/program.rs`, `docs/agent/loop-goal.toml`.

- [ ] **`an_interface_enumeration_is_sorted_by_qualified_name`** in `nvs-hir`. Over
      `implementors` (`crates/nvs-hir/src/hierarchy.rs:463`), whose sort compares *segments*, so the
      case worth pinning is the one a rendered-string sort gets wrong — `App\Sub\A` against
      `App\Beta`. `crates/nvs-hir/src/hierarchy.rs:571` is the existing near-twin; do not rename it.
      ADR 0061 § 3.
- [ ] **`an_abstract_class_is_not_enumerated`** in `nvs-hir`, over `ClassLinks::concrete`
      (`crates/nvs-hir/src/hierarchy.rs:75`) — and the interface itself, which is never its own
      implementor. `crates/nvs-hir/src/hierarchy.rs:548` is the existing near-twin. ADR 0061 § 3.
- [ ] **`an_implementor_without_a_no_argument_constructor_is_named`.** The diagnostic is already
      built and reported at `crates/nvs-types/src/program.rs:95`; what is open is the crate it is
      tested in, per `## State`. A `.nvst` refusal case beside it is the second half —
      `--EXPECTF-ERROR--`, per `docs/agent/conventions.md`. ADR 0061 § 3.

## Backlog

- `crates/nvs-ir/tests/refusals.rs`: 17 unattributed lowering refusals, red at `HEAD`, owned by
  whichever goal claims those sites — `docs/agent/loop-goal.toml`.
- ADR 0077's `#[Route]` table filters this same enumeration — `docs/adr/0077-compile-time-routing.md`
  § 5, and `examples/routes.nvs` is its fixture.
- ADR 0086's `#[Command]` table is the third caller of the scan — `examples/commands.nvs`.
- `Core\Program` is spec § 13, which `spec_registry_coverage.rs` deliberately does not read; nothing
  is owed there — `crates/nvs-stdlib/tests/spec_registry_coverage.rs`.
- ADR 0057's literal folding, `examples/intrinsics.nvs` — `docs/agent/loop-goal.toml` stage 0.
