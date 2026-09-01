# Handoff

## State

**All three `Core\Secret` slices landed**, as three `.nvst` cases over `reveal` and `revealBytes`. No
Rust change and no member change, so no new refcount edge and no valgrind run behind them. The class
now carries seven cases across its two members, and the questions the group added are the ones the
module doc's three headings state and no case asked: the second name reaches octets the first cannot,
the reason is read by nobody, and the two members do the same thing to the same content.

- **The octet sweep is counted twice, and the second count is what makes it a `bytes` claim** — all
  256 single-octet buffers come back identical and one octet long, and exactly **128** of them are
  valid UTF-8, so half the sweep is ground a `string` round-trip could not have covered. Lengths are
  the other axis: every prefix of a 16-octet non-text pattern, 17 of 17 including the empty one.
- **The reason is swept, not exampled** — seven reasons totalling 1,102 characters (empty, one
  character, a built string, one quoting the credential itself, one repeated forty times) against an
  expectation that is *another reveal* rather than a rewritten literal, so the claim is agreement and
  not a frozen answer.
- **The boundary case asserts three routes agree, and prints why that is not vacuous** — cast,
  `Core\Encoding::encodeText` and `::decodeText` back, 5 of 5 each, over content whose 37 characters
  are 41 octets. An ASCII-only sweep would have agreed for the wrong reason.

**The `[context] modules` warnings the last handoff reported are gone** — this session's pack printed
none, which is commit 5ed71b97 doing what it says. Nothing else was missing from the pack.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this
goal and is not a regression.

## Next group

**`Core\SignedCookie` is the next two-member class whose surface is one rule and no dependency — no
store, no socket, no clock — and it is one file set: `crates/nvs-stdlib/src/signed_cookie.rs` and
`tests/conformance/core/`. Its four existing cases are `signed-cookie-answers-cookie-safe-text-of-a-stated-length`,
`signed-cookie-open-refuses-every-forgery-with-one-message`,
`signed-cookie-seals-with-the-newest-key-and-opens-against-the-ring` and
`csrf-a-signed-cookie-of-the-session-is-not-a-token-under-the-same-key`, so what is left is the
round-trip as a sweep, the ring as a bound, and the one qualifier question ADR 0060 § 5 already
answers in prose.**

- [ ] **`seal` then `open` is the identity over a sweep of values** — counted, over the empty value,
      one holding `=` and `;` and a `.`, one that is multibyte and one long enough to matter, so a
      member that trimmed or re-encoded its payload fails where a single ASCII example passes. The
      expectation is the input itself, never a frozen cookie: the sealing key is random per run.
      `crates/nvs-stdlib/src/signed_cookie.rs:311`.
- [ ] **The ring is a bound asserted on both sides** — a cookie opens while the key that sealed it is
      anywhere in the ring and stops the moment it leaves, counted over a ring walked one key at a
      time, with the one-key ring named as the degenerate end. The existing rotation case asserts the
      newest key seals; this asserts what the *rest* of the ring is for.
      `crates/nvs-stdlib/src/signed_cookie.rs:344`.
- [ ] **What `open` does to `tainted`, against the standing decision that a verified signature does
      not launder** — the parameter is `Qual::Launder` and the return type is a plain `Str`, and ADR
      0060 § 5 is the home of the rule. Read that section first: if the answer is still `tainted` the
      case is a sink refusing it, and if it is not, the section and the row disagree and that is the
      finding. `crates/nvs-stdlib/src/signed_cookie.rs:131`.

## Backlog

- `Core\Totp` is thin at 4/4 but its four cases already cover the window, the replay and the step — a
  group there needs a new question, not a new row (`docs/adr/0060-application-security-protocols.md` § 1).
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case
  (`python tools/gaps.py`, differential gap; `crates/nvs-stdlib/src/task.rs:561`).
- `Core\Http\Response` and `Core\Mail` are the thinnest classes left at depth 3.0
  (`python tools/gaps.py`).
- Five catchable `thrown` sites still have no case that asserts them, in `command.rs`, `csv.rs`,
  `env.rs` and `password.rs` (`python tools/gaps.py --errors`).
