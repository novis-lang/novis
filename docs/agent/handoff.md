# Handoff

## State

**Goal `unowned-closures`, stage 5 is closed.** No goal-owned module-doc gap is left in it: the
artifact loader's aarch64 half is built and the last `docs/agent/carried-gaps.md` row goal
`m7-server-surface` left behind is struck. `python tools/owners.py` reads 81 items, `--deferrals`
green; `unowned: 15` is stage 6 and is now the whole of what the goal still owes.

**The artifact loader relocates an aarch64 payload.** `nvs_cli::cache`'s `Form` is the one place
the two architectures differ — x86-64's flat little-endian field beside a `CALL26`, an `ADRP` page
and a scaled 12-bit page offset, each a bit range inside one instruction — and `form_of` is the one
place either container format's aarch64 relocation numbering is read. `HOST_ARCH` names both
architectures; `HOST_PUBLISHES` is its answer asked of the host, so a COFF aarch64 target, which
the object backend has no aarch64 relocation spelled for, writes no artifact instead of paying a
second Cranelift walk for a file nothing reads.

**The aarch64 halves of those `#[cfg]`s are not compiled here** — the playbook's *Running things*
bullet says why — so the field writers are asserted as whole instruction words against what an
assembler produces, which is a check any host can make.

## Next group

**Stage 6: the register** — one file set: `crates/nvs-cli/src/openapi.rs`, then
`crates/nvs-cli/src/runner.rs`. Each item is a scheduling question, so the slice is a decision
written into the gap's own `— owner:` line: a goal slug on the chain, or an M9+ deferral whose
plan states the scope. `python tools/owners.py --deferrals` is the gate on the second kind.

- [ ] **The three openapi gaps that wait on something outside the emitter** —
      `crates/nvs-cli/src/openapi.rs:32` gaps 1 to 3, whose rule is
      `rule:routing/api-document-is-generated-from-the-route-table`. A response or request body
      that is a class waits on `rule:core-classes/derive-attribute`'s codec roster reaching the
      route row, and `components.securitySchemes` waits on a scheme having a home at all.
- [ ] **The two that wait on a configuration key or a capture's set** —
      `crates/nvs-cli/src/openapi.rs:55` gaps 4 and 5, same rule: `info.version` wants an
      `nvs.toml` key, and an enum-case subset wants `nvs_types::routes`' `closed_set` to answer
      for a capture declared at a case.
- [ ] **The suite is not run in parallel** — `crates/nvs-cli/src/runner.rs:85` gap 1, whose
      reasoning is `docs/decisions/0079.md:158` with that record's milestone table at
      `docs/decisions/0079.md:872`. Every test already runs in an isolate of its own, so this is a
      scheduling question and not an isolation one.

## Backlog

- The remaining nine unowned items, by file set: `crates/nvs-host/src/{group,placed,worker}.rs`
  (placement), then `nvs-db/src/span.rs`, `nvs-db/src/tds/mod.rs`, `nvs-runtime/src/ctx/mod.rs`,
  `nvs-runtime/src/graph.rs`, `nvs-stdlib/src/cache.rs`, `nvs-stdlib/src/response.rs`.
- `crates/nvs-db/src/span.rs` gap 1 stands and is still unowned: the `debug.trace` grant reaches no
  `DebugFlags` bit, and sampling answers neither half of it.
- Eight items defer to a milestone the program has already passed, which `owners.py` reports as a
  finding rather than a failure — `docs/agent/carried-gaps.md` is where each would be re-homed.
