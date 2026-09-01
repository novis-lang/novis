# Handoff

## State

**All three `Core\SignedCookie` slices landed**, as three `.nvst` cases over `seal` and `open`. No Rust
change and no member change, so no new refcount edge and no valgrind run behind them. The class now
carries seven cases across its two members, and the three questions the group added are the ones the
module doc's own headings state and no case asked: the answer is the payload and never the cookie, the
ring is membership and not position, and `open` is the one verified signature in the language that
launders.

- **The round trip is asserted against the input, twice per value** — four payload *shapes* rather than
  four punctuations: the empty one, the cookie delimiters, one whose 27 octets carry 14 characters, and
  one of 1,408 characters whose cookie is 1,931. Each is sealed twice and both cookies opened, so a
  member answering a plaintext cached against the cookie text passes the first count and fails the
  second.
- **The ring is counted from both ends, one key at a time** — five cookies each sealed under a one-key
  ring, then prefix rings 1..5 (opens exactly the ring's own length, refuses the rest) and tail rings
  1..4 (4, 3, 2, 1 survive). The growing walk names the last accepted key and the first refused one on
  every line; the shrinking one moves every key's index, so a member keyed off position agrees with the
  first half and fails the second.
- **The laundering rule is asserted from both sides in one file, under one key** — a `tainted` cookie
  goes into `open` and a plain `string` comes out, while `Core\Jwt::verify` hands back
  `array<tainted string>` for a token signed a line earlier. The pattern sink takes the cookie payload
  directly and takes the claim only through `Core\Regex::quote`, so ADR 0060 § 5 is legible as code.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over a
*complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this goal and
is not a regression. Nothing was missing from this session's pack.

## Next group

**`Core\Totp` is the last of ADR 0060 § 1's roster with a thin surface, and it is the same one file set
as this group: `crates/nvs-stdlib/src/totp.rs` and `tests/conformance/core/`. `python tools/gaps.py`
ranks it at depth 4.0 with both members at the floor — `check` 4, `code` 4 — and its one existing case
is `totp-codes-are-six-characters-and-never-cross-between-secrets`, so the window, the agreement between
the two members and the replay refusal are all unasked.**

- [ ] **The window is a bound asserted on both sides** — the last counter step `check` still accepts and
      the first it refuses, named together on one line rather than one accepted code, and asserted
      alongside the fact that no argument anywhere in the surface widens it. A member one step wide on
      either side prints plausibly against a single in-window code.
      `crates/nvs-stdlib/src/totp.rs:342`.
- [ ] **`code` and `check` agree over a sweep of secrets and counters** — counted: every code the first
      member emits is one the second accepts at the same instant, over several secrets and several
      steps, so a member that grew its own truncation or its own digit count fails here while still
      answering six characters on its own line. `crates/nvs-stdlib/src/totp.rs:326`.
- [ ] **The replay refusal the caller can actually enforce** — read what `check`'s three arguments and
      its answer are before writing anything: the module doc says the enforcement is the caller's and
      the surface is what makes it possible, so the case pins what a program has to store to reject a
      second use of one code. `crates/nvs-stdlib/src/totp.rs:342`.

## Backlog

- `Core\Cache\Store`'s `get`/`put` are at four cases each and need a Docker Redis — ADR 0059 § 2.
- `Core\Cldr::pluralCategory` is one member with four cases — ADR 0082 § 2.
- `Core\Task::afterResponse` is the only member left with a PHP twin and no oracle case, so it belongs in
  `tests/differential/` — `crates/nvs-stdlib/src/task.rs:561`.
- `Core\Csv::format`'s non-string column throw is unasserted — `crates/nvs-stdlib/src/csv.rs:610`.
- `Core\Env::get`'s invalid-UTF-8 throw is unasserted — `crates/nvs-stdlib/src/env.rs:200`.
- Part II's registration gate needs spec §§ 15-19, which are goal 6's — not closeable here.
