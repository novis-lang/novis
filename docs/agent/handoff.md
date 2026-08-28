# Handoff

## State

**Goal 1 of the parity program: ADR 0061 § 3's program enumeration is landed in `nvs-hir`, and only
there.** `AutoloadMap::enumerate` lists every name the autoload roots declare,
`nvs_hir::implementors` filters the class graph to the non-abstract classes reaching one interface,
and `requires::resolve_program` runs the scan for — and only for — a program that writes
`Core\Program::implementing<T>()`. `nvs-ir` and `nvs-codegen` are untouched, as this goal requires.

**What is missing is the call itself.** `Core\Program` has no `nvs-stdlib` registry row, so a program
writing `implementing<T>()` today triggers the scan in HIR and is then rejected by `nvs-types` as an
unknown `Core` class. Nothing below `nvs-types` is in the way; the next group is the whole remainder.

The group's ordering had a gap worth knowing: the call site cannot expand until the classes are *in*
the graph, so the load had to land before it. That is done — the group below starts where the last one
meant to.

**`python tools/verify.py` is red at `HEAD`, and not from this work.**
`crates/nvs-ir/tests/refusals.rs` fails with 17 unattributed `nvs-ir` lowering refusals — the goal
switch orphaned every site M4's item list used to claim, and goal 1's does not. It is the *first*
failure, so the gate stops there and never reaches the `.nvst` trees or clippy. This session ran the
remaining steps by hand instead: fmt, build, `--workspace --no-fail-fast` (81 targets green, that one
red), conformance 880/880, differential 189/189, clippy clean. Deciding which goal claims those sites
is the user's call, not a session's — the test refuses its own allowlist as the fix.

## Next group

**ADR 0061 § 3's call site.** One file set: `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/expr/calls.rs`, `examples/program.nvs`.

- [ ] **`Core\Program::implementing<T>()` resolves as a `Core` member.** The class needs its own
      module and a `CLASSES` row (`crates/nvs-stdlib/src/registry.rs:709`) — the four edits in
      `docs/agent/conventions.md`, with the wrinkle that `T` is a *type argument* rather than a
      parameter, so the row's shape is the open question, not the body. ADR 0061 § 3.
- [ ] **The call expands to an array literal of `new` expressions.**
      `crates/nvs-types/src/expr/calls.rs:182` (`infer_static_call`) is the arm; the list is
      `nvs_hir::implementors(&target, &env.graph)`, already sorted. Every selected class needs a
      no-argument constructor and a diagnostic names any that does not — next free in the types band
      is `E0743`. ADR 0061 § 3.
- [ ] **`examples/program.nvs` and its conformance case.** Three classes implementing one interface
      across two files, one of them abstract, asserting the order and the exclusion. ADR 0061 § 3.

## Backlog

- **The refusals guard is red at `HEAD`** (`crates/nvs-ir/tests/refusals.rs`, 17 sites from
  `python tools/holes.py --unattributed`). Until a goal claims them, every session's `verify.py` stops
  at `test` and never runs the `.nvst` trees or clippy.
- ADR 0077's route table filters the same enumeration — `AutoloadMap::enumerate` is built for it and
  needs no second walk (ADR 0077 § 5).
- A scanned file is held to `autoload::check_file_shape`, so a root holding a non-declaration file
  starts diagnosing once a program scans; worth a case when § 3 is reachable from source.
- `Core\Command`'s table and the OpenAPI emitter are the other two filters of the same scan
  (ADR 0086, ADR 0085).
- `python tools/gaps.py` still owns the per-class conformance depth worklist for M4S Part I.
- `python tools/check-migration.py --report` is the parity program's own measure, unmoved by this
  session.
