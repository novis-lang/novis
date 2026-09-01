# Handoff

## State

**Two of `Core\Process\Result`'s three slices landed**, as two `.nvst` files and nothing else:
`crates/nvs-stdlib/src/process.rs` is unchanged, so there is no new refcount edge and no valgrind run
behind them. `gaps.py` no longer lists the class at all — `exitCode` 3→4, `stdout` 4→6, `stderr` 3→5 —
and the floor is now the six classes at depth 3.0, of which `Core\Totp` is the next group.

- **The capture case measures a sweep, not a size.** 1,000 / 65,500 / 65,600 / 262,100 octets on both
  streams at once, with the 64 KiB boundary named on both sides and `stopped at the buffer=0` reading
  the truncation failure directly, since a capture that ends where the pipe filled measures exactly
  65,536 and no size in the sweep does. Each shell writes exactly one hundred octets per unit —
  `printf` pads to a field width, `cmd` echoes a 98-octet line and its own CRLF — and the counts are
  `>=` because a shell that pads a byte of its own is not that case's subject.
- **The status case asserts the three answers belong to the same run.** The two landed cases either
  ask what the status is over a silent child or what the streams carry over a successful one; a
  member that captured only what a successful child wrote passes both. `0` is in the sweep as the
  control, `255` bounds it.

**The third slice — the captures are octets, not text — is blocked on a missing writer, not on the
member.** There is no way to put non-UTF-8 octets on disk from a `.nvst` case today; that is a
playbook bullet now, with the half of the design that does work (`cat`/`type` both round-trip the
probe through a pipe unchanged) recorded so it is not re-derived.

**The two `orient.py` warnings are still there**: `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this
goal and is not a regression.

## Next group

**`Core\Totp` is the floor now — two members, one rule, ADR 0060 § 1's "a window that has no widening
argument and a replay refusal the caller can actually enforce". The file set is
`crates/nvs-stdlib/src/totp.rs` and `tests/conformance/core/`; the two member bodies are sixteen lines
apart. Three cases already ask about it — the window narrowed from both sides, a code accepted once
inside a window, and six characters that never cross between secrets — so read
`totp-accepts-a-code-once-inside-a-window-with-no-widening-argument.nvst` first.**

- [ ] **A code is a function of its step and nothing else** — the same secret at the same step answers
      the same code every time and two adjacent steps never agree, counted over a sweep rather than
      sampled at one pair, so a member that folded in the wall clock fails by count while answering
      plausibly for a single draw. `crates/nvs-stdlib/src/totp.rs:302`.
- [ ] **`check` and `code` agree at every offset the window accepts** — one question asked of both
      members over the whole window, asserting that they agree rather than what each answered, so a
      `check` that grew its own derivation fails here while still looking right on its own line.
      `crates/nvs-stdlib/src/totp.rs:302` and `crates/nvs-stdlib/src/totp.rs:318`.
- [ ] **Every code is six ASCII digits, leading zeros kept** — over a sweep of secrets and steps, so a
      rendering that went through a number loses its leading zero and fails by count. This is the
      *digits* reading of `totp-codes-are-six-characters-and-never-cross-between-secrets.nvst`'s
      length, and the overlap is worth checking before writing.
      `crates/nvs-stdlib/src/totp.rs:302`.

## Backlog

- The captures are octets, not text — blocked on a bytes-writing route; the playbook bullet under
  *Writing a test case* holds the finding and the candidates.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case — `gaps.py`.
- `orient.py`'s `[context] modules` matcher misses `fatal.rs` and `script.rs` — `docs/agent/loop-goal.toml`.
- `Core\Csrf`, `Core\Jwt`, `Core\Http\Response`, `Core\Cli\Color` and `Core\RateLimit` are the other
  depth-3.0 floors — `gaps.py`.
