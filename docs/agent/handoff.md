# Handoff

## State

**`Core\Totp` is closed for depth work.** The three slices the previous handoff named were already
on disk — `eefef3eb` landed the window's `$after` bound on both sides and the no-widening-argument
case with the feature itself, and `e5d260ec` landed the code/check agreement sweep this morning — so
this session asked the two questions the six existing cases still did not, both `.nvst`-only, no Rust
change and so no new refcount edge.

- **Both qualifier axes, which no case had touched** — `crates/nvs-stdlib/src/totp.rs`'s two rows are
  `Qual::Neutral` on the `secret` and the `tainted` axis respectively, and the class is unusable
  without either: a `secret` goes into `code` and printable text comes out (ADR 0033 § 2, the
  `Core\Password::hash` shape), a `tainted` code out of a form goes into `check` and an ordinary
  `int` comes back, while the six characters stay tainted afterwards — ADR 0060 § 5's rule for the
  third roster entry, asserted through the pattern sink exactly as the SignedCookie/Jwt pair is.
- **RFC 4226 § 4's floor, named on both sides and by both members** — 1/4/8/15 octets refused and
  16/17/20/32/64 accepted, counted, then 15 and 16 printed one after the other with the refusal's own
  sentence. Every earlier case asked well below the floor, so a `>` written where `>=` is meant
  passed all of them.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a complete Part II, which needs spec §§ 15-19 from goal 6. Not a regression and not closable here.
Nothing was missing from this session's pack; `gaps.py` is what the group below rests on.

## Next group

**`Core\Cache`'s local tier is `python tools/gaps.py`'s thinnest class that needs no Docker — depth
3.5, `shared` 3 and `local` 4 — and `Core\Cache\Store` is 4/4 beside it. One file set:
`crates/nvs-stdlib/src/cache.rs` and `tests/conformance/core/`. Four cases exist and their
`--TEST--` lines are the boundary: absence answers `null`, an entry is a copy in both directions,
the two tiers are two contracts, and the local tier forgets rather than failing a write. None of the
three below is one of those.**

- [ ] **A key is compared as the bytes it is** — a sweep of keys differing by one byte, by case, by a
      trailing space and by nothing at all, counted, so a store that normalised or truncated a key
      passes row by row and fails the count.
      `crates/nvs-stdlib/src/cache.rs:799`.
- [ ] **The last write wins, over a sweep** — `put` over a key that already holds a value replaces it
      rather than keeping either the first or a merge of both, counted over a run of writes to one
      key, with the read taken after all of them.
      `crates/nvs-stdlib/src/cache.rs:841`.
- [ ] **`local()` twice is one store** — the entries written through the first handle are read back
      through a second, which is ADR 0059 § 1's "per core" from the language rather than from the
      module doc, and is the half the copy-in-both-directions case cannot see.
      `crates/nvs-stdlib/src/cache.rs:740`.

## Backlog

- `Core\Cldr::pluralCategory` at 4 cases, one member, offline — `crates/nvs-stdlib/src/cldr.rs`.
- `Core\Http\Response` (`status` 3, `text` 3) and `Core\Mail::send` (3) are `gaps.py`'s thinnest
  outright, and both need an endpoint — check what the existing cases stand up before taking them.
- `Core\Task::afterResponse` is the one member with a PHP twin and no oracle case —
  `crates/nvs-stdlib/src/task.rs:561`, and a differential case, never a conformance one.
- 112 unasserted error paths remain (`gaps.py`, tail); 5 are `thrown` and catchable from a case.
- Stage 10's `every_part_two_spec_member_is_registered` needs spec §§ 15-19 — goal 6's, per
  `docs/agent/loop-goal.toml`.
