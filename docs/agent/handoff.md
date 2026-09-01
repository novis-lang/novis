# Handoff

## State

**All three `Core\Totp` slices landed**, and the class has left the 3.0 floor: `python tools/gaps.py`
now reads `4.0 4 4 2 Core\Totp check 4, code 4`, and the floor is eight classes at 3.0 of which
`Core\Jwt` is the next group. One `.nvst` case, one small refactor and two Rust tests.

- **`check`'s decision is now `match_step(secret, code, now, after)`** — the same loop, the same
  candidate skip and so the same constant time, lifted out of the helper
  (`crates/nvs-stdlib/src/totp.rs:293`) so a test can ask *it* the question `code_at` answers. The
  reason is in its own doc comment: the existing Rust test asserted the window by rebuilding
  `((now - DRIFT)..=(now + DRIFT))` beside it, and a reconstruction agrees with itself by
  construction. No behaviour change, no new refcount edge, so no valgrind run behind it.
- **Slices 2 and 3 are one commit with two clauses**, because both are `#[test]`s in one file and git
  cannot stage them apart. `git log` still reads a slice a clause.
- **The `.nvst` case measures the function, not a draw**: 48 redraws microseconds apart, then one
  `Core\Time::sleep` across a real wall-clock second — which is what sees a member reading the second
  rather than the step — counted as the biconditional, because a step boundary lands inside that
  second one run in thirty. The playbook bullet is the home of why.

**The two `orient.py` warnings are still there**: `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this
goal and is not a regression.

## Next group

**`Core\Jwt` is the floor's richest entry — ADR 0060 § 1's fourth roster entry, "the one whose
historical failures are all failures of *choice*". The file set is `crates/nvs-stdlib/src/jwt.rs` and
`tests/conformance/core/`; the two member bodies are seventy lines apart. Three cases already ask
about it — the two bounds on a key and a lifetime, one sentence for every unverifiable token, and
`exp`/`iat` written by `sign` itself — so read
`jwt-refuses-every-unverifiable-token-with-one-sentence.nvst` first.**

- [ ] **`sign` and `verify` agree over a sweep of claim shapes** — one question asked of both, counted:
      every shape that signs verifies back to the same claims, over an empty bag, a nested one, one
      carrying non-ASCII text, the integer bounds and a long string, so a serializer that reordered,
      coerced or truncated fails by count while round-tripping one flat bag plausibly.
      `crates/nvs-stdlib/src/jwt.rs:454` and `crates/nvs-stdlib/src/jwt.rs:524`.
- [ ] **A token verifies for the whole of its lifetime and not one second past it** — the bound named
      on both sides, which is reachable here because the minimum lifetime is one whole second: sign
      with `1s`, verify, `Core\Time::sleep` past the expiry, verify again. A member that compared
      `>=` where it means `>` prints plausibly against either half alone.
      `crates/nvs-stdlib/src/jwt.rs:524`.
- [ ] **Every token is three base64url segments with no padding and no `+` or `/`** — over a sweep of
      keys and claim shapes, counted, because one encoding in four carries a character the URL
      alphabet renames and a single token is silent about it. `crates/nvs-stdlib/src/jwt.rs:454`.

## Backlog

- `Core\Csrf`, `Core\Cli\Color`, `Core\RateLimit` and `Core\Http\Response` are the rest of the 3.0
  floor — `python tools/gaps.py`.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case
  (`fastcgi_finish_request`) — `crates/nvs-stdlib/src/task.rs:561`, and it belongs in
  `tests/differential/`.
- `[context] modules` patterns `fatal.rs`/`script.rs` warn while matching — `docs/agent/loop-goal.toml`.
- Non-UTF-8 octets cannot be written to disk from a `.nvst` case, which blocks the third
  `Core\Process\Result` slice — `docs/agent/playbook.md`.
