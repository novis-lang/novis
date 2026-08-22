# Loop goal

Reach a **running CLI hello world**: `mwl run examples/hello.mwl` compiles the file through the real
pipeline (`mwl-syntax` -> `mwl-hir` -> `mwl-types` -> `mwl-ir` -> `mwl-codegen`) and prints `Hello, World!`
to stdout, exiting 0.

This is milestone **M3** (baseline Cranelift backend) in `docs/implementation-plan.md`. Read that
milestone's paragraph for scope; do not re-derive it here.

## Acceptance (the driver checks this itself, every iteration)

    cargo run --quiet -p mwl-cli -- run examples/hello.mwl

must exit 0 and print exactly `Hello, World!`. Nothing else counts as done — not a passing unit test, not
an IR snapshot. When this command passes, the loop stops on its own.

## Standing decisions for the road to it

These are pre-authorized; do not stop the loop to ask about them.

- Finish M2's open items first (ADR 0043 `by`-delegation resolution, then `for`/`switch` lowering, then the
  `mixed` runtime type-tag representation) only insofar as M3 needs them. If a slice is not on the path to
  the acceptance command, put it in `## Backlog` and move on.
- The first `mwl-codegen` backend may be as narrow as the acceptance command requires: no optimization
  tier, no inline caching, no GC integration beyond what `Hello, World!` touches. Breadth comes later.
- `echo` of a constant string is enough of a `Core` output surface for M3; the full `Core\Cli` /
  `sprintf`-family design stays deferred to M7/M8 per ADR 0011.
- Prefer landing a narrow vertical slice that runs over a wide horizontal one that does not.
