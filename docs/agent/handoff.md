# Handoff

## State

**`Core\Crypto` is off `gaps.py`'s thin list entirely** — `open` 3→6, `seal` 3→6, `generateKey` 12→15 —
and `Core\Process\Result` (exitCode 3, stderr 3, stdout 4) is the floor now. All three of the group's
slices landed, as three `.nvst` files and nothing else: `crates/nvs-stdlib/src/crypto.rs` is unchanged,
so there is no new refcount edge and no valgrind run behind them.

- **The key-length bound is one verdict per width for both members, not a line each.** A width is a key
  or it is not, answered by whether `keyed`'s `LogicError` arrives, and `seal` and `open` are counted as
  agreeing at all eight widths — 31, 32 and 33 printed adjacent so neither half of the bound reads
  plausibly alone. The all-`A` 32-octet key seals the probe every `open` is asked about, so the only
  thing a width is ever refused for there is not being a key.
- **`generateKey`'s length invariant is behavioural because a `secret` cannot be measured** — that is a
  playbook bullet now. The case asserts it through the members that refuse every other width, and adds
  the two a single draw cannot see: 28 pairs distinct, and 56 ordered cross-opens refused.
- **What the nonce case adds over the landed pair-of-seals assertion is *where* two seals differ.** A
  member that drew a fresh nonce, prefixed it and then sealed deterministically behind it passes every
  round trip; the case slices the 24-octet prefix off each of eight seals and counts both the prefixes
  and the bodies distinct across all 28 pairs.

**The two `orient.py` warnings are still there**: `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over a
*complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this goal
and is not a regression.

## Next group

**`Core\Process\Result` is the floor — three members, one rule, ADR 0044 § 1's "a result is a value the
run left behind, not a pipe still open". The file set is `crates/nvs-stdlib/src/process.rs` and
`tests/conformance/core/`; the three member bodies are within twenty lines of each other. Five cases
already ask about it — the empty capture, the status on both sides, the two streams kept apart, every
member agreeing with itself twice, and two results held at once — so read
`process-a-child-that-writes-nothing-answers-two-empty-captures.nvst` and
`process-a-completed-runs-two-streams-stay-apart.nvst` first.**

- [ ] **A capture is whole, not a pipe's worth** — a child writing far more than an OS pipe buffer on
      both streams at once is captured entire, asserted by length over a sweep of sizes that crosses
      the 64 KiB boundary, so a `run` that stopped draining at a buffer's edge fails here while
      answering plausibly for a short child. `crates/nvs-stdlib/src/process.rs:324` and
      `crates/nvs-stdlib/src/process.rs:382`.
- [ ] **The two captures are octets, not text** — `stdout` and `stderr` are `bytes`, so a child writing
      a NUL, a lone `0xFF` and an unpaired surrogate's encoding hands them all back unchanged rather
      than lossily converted, counted over the sweep. `crates/nvs-stdlib/src/process.rs:382` and
      `crates/nvs-stdlib/src/process.rs:390`.
- [ ] **A non-zero status keeps both captures** — over a sweep of statuses the child still wrote both
      streams, and every one of them arrives beside its `exitCode`, so a `run` that kept output only on
      success fails by count. `crates/nvs-stdlib/src/process.rs:324` and
      `crates/nvs-stdlib/src/process.rs:373`.

## Backlog

- `Core\Csrf` (issue 3, verify 3) and `Core\Jwt` (sign 3, verify 3) are the next floor after this one —
  ADR 0060 § 1.
- `Core\Task::afterResponse` has a PHP twin (`fastcgi_finish_request`) and no oracle case, the last one
  `gaps.py` reports — `tests/differential/`, `crates/nvs-stdlib/src/task.rs:561`.
- A `spawn script` child never drains its `onExit` queue: `nvs_stdlib::script::run_exit_hooks` has one
  caller, `crates/nvs-cli/src/main.rs:896`. Open decision — ADR 0127 § 2 says "at most once per script".
- `orient.py`'s `[context] modules` matcher rejects two live paths — `docs/agent/loop-goal.toml`.
- Unasserted thrown paths a case could catch: `crates/nvs-stdlib/src/env.rs:200` (a variable whose
  bytes are not UTF-8) and `crates/nvs-stdlib/src/csv.rs:610` (a column that is not a string).
- `Core\Cli\Color` (index 3, rgb 3) — ADR 0086.
