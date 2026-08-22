# Loop goal

Reach a **running CLI hello world**: `mwl run examples/hello.mwl` compiles the file through the real
pipeline (`mwl-syntax` -> `mwl-hir` -> `mwl-types` -> `mwl-ir` -> `mwl-codegen`) and prints `Hello, World!`
to stdout, exiting 0.

This is milestone **M3** (baseline Cranelift backend) in `docs/implementation-plan.md`. Read that
milestone's paragraph for scope; do not re-derive it here. Reaching this command is a *vertical slice* of
M3, not M3 itself — that milestone's own *Verify* bullet is wider (a throw across several JIT frames, a
helper panic terminating with `FATAL`, an MWL-level backtrace, `--dump-asm`, a benched typed-arithmetic
loop). The loop stops at the slice; the rest of M3 stays queued.

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

### The three gaps that actually sit on the path

Named here because none of them is visible from the milestone text, and each is decided already — they are
work, not questions:

- **The script body is a function.** `mwl-types::check::check_stmts` walks only declarations (`_ => {}`
  swallows every top-level statement), and `mwl-ir` exposes only `lower_method` — so a top-level
  `echo "...";` is today neither type-checked nor lowered, and the acceptance program is exactly that shape.
  [ADR 0008](../docs/adr/0008-static-and-global.md) § 2 already settles the design: a file's top-level
  statements are one synthesized frame whose variables are locals. Reuse `check_method`/`lower_method`
  rather than inventing a second walk.
- **`echo` has no lowering at all.** `StmtKind::Echo` has no arm in `mwl-ir`'s `lower.rs` and no
  `InstKind`/`Helper` behind it. It needs an output helper plus the runtime that owns stdout.
- **`mwl-codegen` and `mwl-runtime` do not exist yet.** When creating them, give each its own `[lints]`
  block with `unsafe_code = "deny"` and narrow reasoned allows — *not* `lints.workspace = true`, which is
  `forbid` workspace-wide and makes a JIT unimplementable. See `Cargo.toml`'s lint-policy comment and the
  plan's *Unsafe policy* section.

### Decided by the user, 2026-08-23

- **ADR 0018's debug-flags probe check lands with the safepoint poll, in the first `mwl-codegen` commit** —
  not deferred until after hello world prints. That ADR's § *Revisiting* verification list names M3 explicitly and its whole
  argument is that this is the one thing not to retrofit. The narrow-backend authorization above does not
  extend to it.
- **`echo` under `mwl run` writes raw bytes to stdout, with no escaping.** ADR 0024 § 5's auto-escaping
  sink is the *HTTP response* write, and `Core\Html\Markup` does not exist until M7/M8. Whether `echo`
  under `mwl serve` becomes that sink is an M7 decision; do not pre-empt it, and do not make `echo` depend
  on `Markup` now.
- **`list(...)` is rejected** ([ADR 0050](../docs/adr/0050-list-destructuring-spelling-rejected.md)) —
  landed, nothing left to do; noted only so no session re-opens it.
