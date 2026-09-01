# Handoff

## State

**`Core\Script\ExitReport` is off `gaps.py`'s list entirely** — its three members went from three cases
each to five, and `Core\Crypto`, `Core\Process\Result` and `Core\Cli\Color` are the floor now. Two cases
landed, not the three the group named, and the third is not the one that was written down.

- **The group's second and third items as written are unreachable, and the finding is why.** Both asked
  for one case spanning two of ADR 0127 § 2's endings, and a script has exactly one ending. The obvious
  route is a `spawn script` child per ending — and **a spawned child never drains its `onExit` queue**:
  `nvs_stdlib::script::run_exit_hooks` has one caller, `crates/nvs-cli/src/main.rs:896`. A child ends
  `ok=true` with a captured `output` holding only what its body echoed. That is recorded as a playbook
  trap and as the backlog's open decision; § 2 says "at most once per script" and a `spawn script` child
  is a script, so this is a gap rather than a scope.
- **What landed in the second slot is the column no case read**: `exit($n)` puts the very `$n` on the
  report. Both landed `exit` cases end at 0, which is also the `Normal` row's status, so nothing told
  "the report carries the number the program chose" from "the report carries 0 unless something threw".
  42 is not 0, not 1, and the case asserts the non-zero status arrives with `error` still `null` — half
  the bound the item wanted, in the only form one ending admits.
- **Identity is what makes the agreement case more than a re-read.** One report is built per ending and
  handed to every hook, so two hooks' readings collapse into one entry of a `Core\ObjectSet<mixed>`, and
  `error`'s two readings collapse the same way because § 2's third column is the live `Throwable`.
- Nothing in `crates/nvs-stdlib/src/script.rs` changed — two `.nvst` files and nothing else — so there is
  no new refcount edge and no valgrind run behind them.

**The two `orient.py` warnings are still there**: the `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no module
and are then printed in the scoped map anyway. The manifest is right; the matcher is what to check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over a
*complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this goal and
is not a regression.

## Next group

**`Core\Crypto` is the thinnest class left — `open` 3, `seal` 3, `generateKey` 12 — and all three are
bound by one rule, ADR 0051 § 3's "AEAD only, no ECB, no unauthenticated CBC, no cipher-name-as-string":
the cipher, the mode, the padding and the nonce are all off the call, so what is left to assert is the
key and the nonce. The file set is `crates/nvs-stdlib/src/crypto.rs` and `tests/conformance/core/`; the
three helper bodies are within fifty lines of each other. The landed cases are
`crypto-seals-and-opens-with-no-cipher-argument.nvst`,
`crypto-round-trips-every-message-length-with-a-flat-overhead.nvst` and
`crypto-open-refuses-every-forgery-with-one-message.nvst` — read those first, because the forgery
refusal and the length sweep are both already asked.**

- [ ] **A case that names the key-length bound on both sides** — `keyed` refuses any `$key` that is not
      `KEY_LEN`, through one `wrong_key_length` both members share, so the case names the last accepted
      length beside the first refused one on each side of it, and asserts `seal` and `open` refuse
      identically rather than each on its own line. `crates/nvs-stdlib/src/crypto.rs:420`,
      `crates/nvs-stdlib/src/crypto.rs:447` and `crates/nvs-stdlib/src/crypto.rs:463`.
- [ ] **A case that asserts `generateKey`'s invariants by counting** — over a sweep of keys, every one is
      distinct, every one is exactly the length `seal` accepts, and no message sealed under one opens
      under another, counted rather than read off a line.
      `crates/nvs-stdlib/src/crypto.rs:431`.
- [ ] **An agreement case over the nonce** — `seal` called twice on one message and one key answers two
      *different* sealed values, because the nonce is drawn per call and prefixed, and both open to the
      one message: agreement about the plaintext and disagreement about the ciphertext, in one case.
      `crates/nvs-stdlib/src/crypto.rs:444` and `crates/nvs-stdlib/src/crypto.rs:463`.

## Backlog

- Decide and record whether a `spawn script` child drains its `onExit` queue — ADR 0127 § 2 against
  `crates/nvs-cli/src/main.rs:896` being the only caller.
- `Core\Script\ExitReport`'s roster sweep, reframed to one ending: the closed `Core\Script\ExitReason`
  roster walked and exactly one case matching the report — `docs/adr/0127-…` § 2.
- `orient.py`'s two `[context] modules` patterns that match no module — `docs/agent/loop-goal.toml`.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case —
  `crates/nvs-stdlib/src/task.rs:561`, `tests/differential/`.
- `Core\Process\Result` and `Core\Cli\Color` are the floor after `Core\Crypto` — `python tools/gaps.py`.
