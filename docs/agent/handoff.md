# Handoff

## State

**Goal `resource-ceilings` — every stage is landed and the goal's own checks are green.** Stage 7
closed this session: `docs/decisions/0174.md` is the record, the three rules its `changes.creates`
names are in the rulebook and rendered, and `crates/nvs-runtime/src/string.rs:1089`'s block comment
over the primitives no longer argues they cannot fail. `python tools/verify.py` is 9 of 9 green.

**The stage 1 floor is green again.** `python tools/rules.py --check` was red on
`docs/agent/goals/47-webcrypto.toml`, whose two `want` strings spelled `rule:` tokens for rules that
goal has yet to create. They name the bare id now; the playbook's newest Tooling bullet is the trap.

**One cross-request store this goal never named is unbracketed, and it is not the regex one.**
`crates/nvs-stdlib/src/bus.rs:168` is a per-core mailbox queue held in a process-wide registry: a
publish encodes an `Envelope` on the publishing core's balance and the receiving core frees it, which
is stage 6's hole one core apart. `budget::Detached`'s balance is signed for exactly that case. The
compiled-pattern cache stays unbracketed on purpose — `crates/nvs-stdlib/src/regex.rs`'s gap 4 is the
finding in full, and it waits on M6's arena.

## Next group

**Stage 6, reopened for the store the audit found** — one file set, `crates/nvs-stdlib/src/bus.rs`
with `crates/nvs-runtime/src/budget.rs` beside it, and
`rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance` as the specification for both
slices. The rule already says what is owed: the bracket is held by the store, around every path that
allocates or frees what it holds.

- [ ] **The bus queue takes the bracket at both ends** — `crates/nvs-stdlib/src/bus.rs:168` is the
      queue and `crates/nvs-stdlib/src/bus.rs:147` the per-core mailbox. The publish side allocates
      the `Envelope` and the receive side drops it, on two different cores, so the two ends take the
      guard separately; `crates/nvs-runtime/src/budget.rs:225` is why a signed per-thread balance is
      what makes that sound. `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`.
- [ ] **A guard that says a publish is charged to neither request** — beside the ones stage 6 wrote
      at `crates/nvs-runtime/tests/detached_accounting.rs:1`, which is the shape to copy: a publish
      raises no ceiling on the publisher and a receive lowers none on the receiver.
      `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`.
- [ ] **Finish the sweep of the remaining process-lifetime stores** — `crates/nvs-stdlib/src/topic.rs:315`
      holds `Weak`s and nothing unsubscribes, and `crates/nvs-stdlib/src/channel.rs:228` is bounded
      inside one request tree; decide per store whether it retains bytes across requests and say so
      in its module doc either way. `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`.

## Backlog

- The compiled-pattern cache cannot be bracketed before the arena — `crates/nvs-stdlib/src/regex.rs` § *Known gaps* 4.
- macOS enforces no CPU ceiling and says so at boot — `crates/nvs-host/src/cpuclock.rs`'s module doc.
- 0174 has no `docs/decisions.toml` summary entry; that pass is user-fired, per `docs/agent/decisions-summary.md`.
- The backtracking step budget is still a constant rather than a directive — `crates/nvs-stdlib/src/regex.rs` § *Known gaps* 2.
