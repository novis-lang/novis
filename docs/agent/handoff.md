# Handoff

## State

**All three `Core\Jwt` slices landed**, and the class has left the 3.0 floor: `python tools/gaps.py`
no longer lists it, at 6 cases over 2 members. Three `.nvst` cases, no Rust change, no member change,
so no new refcount edge and no valgrind run behind them.

- **`sign` and `verify` agree over a sweep of claim shapes** — six bags signed and verified back,
  counted twice (the caller's claims plus exactly the registered pair, and every claim identical):
  an empty bag, a flat one, a claim whose value is itself JSON text, non-ASCII in both a name and a
  value, the integer bounds with a padded number beside them, and a 2048-character claim reported by
  length. `payload_of` escapes a name and a value through the same `quoted`, which is why a name is
  in the sweep at all.
- **The lifetime bound is named on both sides** — the last accepted second is `iat`, the first
  refused one is `exp`, and they are adjacent because the minimum lifetime is one whole second. The
  playbook bullet added this session is the home of how that is made deterministic without a fixed
  clock; the case also pins the complement of `jwt-refuses-every-unverifiable-token-with-one-sentence`,
  that the expiry refusal is the *one* failure allowed its own sentence.
- **Twelve tokens over two keys and six claim widths are three unpadded base64url segments**, with a
  non-vacuity line: at least one segment carries a byte standard base64 would spell `+` or `/`, so
  the counts say the alphabet was exercised rather than merely not contradicted.

**Nothing echoes a claim.** `verify` answers `array<tainted string>` (ADR 0060 § 5) and `echo` is a
sink, so every assertion over a claim is a comparison or a `Core\Str::length`, which is neutral.

**The two `orient.py` warnings are still there**: `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this
goal and is not a regression.

## Next group

**`Core\Csrf` is ADR 0060 § 1's second roster entry and the floor's next security one — one of seven
classes now sitting at 3.0. The file set is `crates/nvs-stdlib/src/csrf.rs` and
`tests/conformance/core/`; the two member bodies are twenty lines apart. Three cases already ask
about it — the binding to one session, every shape of session, and a signed cookie of the same
session not being a token — so the shapes left are the ones about the token itself.**

- [ ] **Two tokens issued for the same session under the same key differ, and both verify** — the
      seal is nonce-bearing, so `issue` is not a function of its arguments; counted over a sweep of
      issues, so a member that went deterministic (and made a token a stable secret every response
      reprints) fails by count while one round trip still looks right.
      `crates/nvs-stdlib/src/csrf.rs:316` and `crates/nvs-stdlib/src/csrf.rs:339`.
- [ ] **Every single-character mutation of a token is refused, and none of them throws** — over a
      sweep of positions across all three of a token's regions, counted, which is what a comparison
      written over a prefix or a decode that ignored trailing bytes fails.
      `crates/nvs-stdlib/src/csrf.rs:339`.
- [ ] **A wrong key is `false` and a key that was never a key is a `LogicError`** — the bound named
      on both sides at 32 octets, per the card's own two error rows: rotating the key refuses every
      outstanding token without telling the holder why. `crates/nvs-stdlib/src/csrf.rs:339`.

## Backlog

- Six more classes at the 3.0 floor after `Core\Csrf`: `Core\Cli\Color`, `Core\Http\Response`,
  `Core\RateLimit`, `Core\Cli\Progress`, `Core\Cli\Style`, `Core\Mail` — `python tools/gaps.py`.
- The one differential gap left: `Core\Task::afterResponse` against `fastcgi_finish_request`,
  `crates/nvs-stdlib/src/task.rs:561`, and it belongs in `tests/differential/`.
- `orient.py`'s `[context] modules` matcher misses a pattern naming a module by its exact path —
  `docs/agent/loop-goal.toml`.
- `crates/nvs-stdlib/src/time.rs:3214`'s `wall_clock` doc says "every wall-clock reading in `Core`,
  which today is `Core\Time::now` and `Core\Uuid::v7`" — `jwt.rs:351` and `totp.rs:257` read it too.
