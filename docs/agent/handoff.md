# Handoff

## State

**M8 goal 5, stage 10.** `Core\Taint::assertTrusted` is landed whole — ADR 0024 § 3's signature as a
`Qual::Launder` row in a new `crates/nvs-stdlib/src/taint.rs`, its reference card, the helper (the
identity at run time, exactly as `Core\Secret::reveal` is), the `address()` arm, three `.nvst` cases and
`docs/reference/core/Taint.md`. The registry's `Qual` doc comment already held the decision; the module
doc holds only what that one does not. `crates/nvs-types/src/core_lib.rs` now closes the roster at one
class, the `tainted` twin of the `secret` roster beside it.

**The `secret` axis does not reach this member, and that is the design.** `Qual::Launder` refuses a
`secret` argument (`admits_secret_argument` is `Qual::Reveal`'s alone), so a value carrying both passes
`Core\Secret::reveal` first and `assertTrusted` second; the other order is an `E0401` and
`tests/conformance/reject/assert-trusted-removes-tainted-and-refuses-a-secret.nvst` pins it. There is no
`assertTrustedBytes`: `taint.rs`'s module doc owns why the one `bytes` sink has no launderer on purpose.

**The driver's failing acceptance check is stale driver state, not a regression, and no session in this
run can clear it.** Thirteen valgrind fixtures are reported red with `exit 1`, which the `tools/loop.py`
on disk cannot emit — session 0004 fixed the sweep and the running driver still holds the import from
before it. The playbook bullet is the evidence. Do not re-fix it.

**One real leak is still open and is the next group.** 292 bytes (112 direct, 180 indirect), *definitely*
lost, on the JIT → `nvs_array_set` → `NvsArray::set` → `make_unique` stack — a copy-on-write clone nobody
released. Seen once during a five-wide sweep of `examples/transaction.nvs` and not reproducible standalone.

**Orientation gap, carried:** `[context]` still has no field that can name a `docs/spec/` file.

## Next group

**One file set: `crates/nvs-runtime/src/array.rs`, with `examples/transaction.nvs` as the fixture and
`tools/leak-check.sh` as the harness.**

- [ ] **Audit the copy-on-write clone against every path that can drop it.** Read `make_unique` at
      `crates/nvs-runtime/src/array.rs:1032` and every early return between it and the entry point at
      `crates/nvs-runtime/src/array.rs:1482`; a clone installed but not released on a failure path is
      the shape the valgrind stack describes. Do **not** run four copies of `examples/transaction.nvs`
      at once — they race on `create table accounts` and die on `pg_type_typname_nsp_index`, which is
      the harness and not the bug.
- [ ] **Pin whatever the audit finds with a counting test over `nvs_runtime::budget::live_bytes`**, in
      `crates/nvs-runtime/src/array.rs`'s own `mod tests`: share an array, force the clone through
      `crates/nvs-runtime/src/array.rs:1482`, release both, and assert the live reading returns to where
      it started. The playbook's `budget` bullet says why no test binary may install its own allocator.
- [ ] **If the audit finds nothing, write the reproduction instead** — a loop that clones and releases at
      `crates/nvs-runtime/src/array.rs:1032` while watching `nvs_runtime::budget::live_bytes` drift is
      the cheapest evidence either way, and it costs nothing once the test above exists.

## Backlog
- The valgrind acceptance check is unclosable inside this run — `docs/agent/playbook.md`, *Tooling*.
- No `Core\Taint::assertTrustedBytes`; `crates/nvs-stdlib/src/taint.rs`'s module doc owns why.
- `[context]` has no selector for a `docs/spec/` file — `docs/agent/loop-goal.toml`.
- The rest of stage 10's corpus, ranked — `python tools/gaps.py`.
- `Core\Db\Settings.host` is still the one caller ADR 0067 § 3 names and has no case of its own.
