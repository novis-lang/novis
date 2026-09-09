# Handoff

## State

**Goal 39 — stage 2's record half is on disk: a `Core\Log::write` and a `Core\Debug::dump` record
name the file, one-based line and enclosing `Class::member` they were produced at.** What is left of
stage 2 is the `Throwable` half, which is the next group. Goal 38's list is still this goal's Stage 1
floor, and [0165](../decisions/0165.md) § *Standing decisions* are not a session's to re-open.

The datum has one derivation and one carrier. `Lowering::source`
(`crates/nvs-ir/src/lower/mod.rs:2170`) builds it; `InstKind::SourceConst` carries it;
`nvs_runtime::source` is the byte format, with `encode` for `nvs-codegen` and `decode`/`of_operand`
for a producer; `nvs_stdlib::registry::RECORD_PRODUCERS` is the roster and owns the ABI — the
constant is **argument 0**, ahead of the receiver, because `dump` is variadic and has no fixed last
slot. `None` (the zero word) is a producer with no call site, which today is only the thunk a
callable reference synthesizes.

**The rule is still `designed`**, and stays so until `Throwable::$location` is the same datum. The
fragment's `guardedBy` gains `tests/conformance/core/a-record-names-the-member-it-was-produced-in.nvst`
in the commit that flips it.

## Next group

**Stage 2: a throw's location is that same datum** — one file set:
`crates/nvs-runtime/src/throwable.rs`, `crates/nvs-ir/src/lower/exception.rs`,
`crates/nvs-codegen/src/emit.rs`, `docs/rules/errors/`.

- [ ] **A throw carries the carrier the producers already take** — `crates/nvs-ir/src/lower/exception.rs:26`
      is where `throw` is lowered and where `Lowering::producer_source`
      (`crates/nvs-ir/src/lower/mod.rs:2006`) is already in reach; the slot it fills is
      `crates/nvs-runtime/src/throwable.rs:55`, which is written as the empty string at
      construction today. `rule:errors/a-record-names-where-it-was-produced`.
- [ ] **The two `-p nvs-runtime` tests the check names** —
      `a_thrown_object_reports_a_location_rather_than_an_empty_string` and
      `the_location_and_a_record_produced_at_the_same_site_agree`, beside the slot roster at
      `crates/nvs-runtime/src/throwable.rs:835`. The second is the *agreement* the rule exists for,
      so it has to read both readers of one construction, not two struct literals.
- [ ] **Ship the rule** — `docs/rules/errors.json`'s entry to `shipped` with the conformance case
      under `guardedBy`, then `python tools/rules.py --render`;
      `crates/nvs-runtime/src/source.rs:1` is the module doc that owns the format it names.

## Backlog

- Stage 3's debug stream is untouched; [0165](../decisions/0165.md) § 3 says it is not coalesced.
- `Envelope.count` (the § 2 half of 0165) has no producer yet — the log target's fixed window.
- `Core\Debug::render` is deliberately off `RECORD_PRODUCERS`; that roster's doc says why.
