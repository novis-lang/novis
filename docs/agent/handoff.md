# Handoff

## State

**M4's Stage 8, depth.** The tree is at **780 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Hash\Stream` left the
frontier this session (2 → 4.0) and `Core\Hash` is at 5.0. The thinnest classes are now
`Core\Bytes` (median 2, floor 2) and `Core\Test` (2, floor 2). Nothing is blocked.

- **A language hole was found and closed on the way**, in `nvs-types`: a written type
  annotation naming a `Core` enum — `Core\Digest $d` on a parameter, a binding or a return —
  interned as `Ty::Class` rather than `Ty::Enum`, because `resolve_name_type` had no arm for a
  name that is in `crate::enums`'s seeded table but in no symbol table. Every `Core` signature
  row builds `Ty::Enum`, so the two never unified and the mismatch arrived as the uninformative
  `expected `Core\Digest`, found `Core\Digest``. `crates/nvs-types/src/lower.rs:576` now mirrors
  the arm `lower_member_type` at `:244` already had, guarded by a unit test in that file's own
  `mod tests`. The consequence is that a `Core` enum can be passed through a user function at
  all — which is what let both cases below factor their sweeps into helpers.
- Two conformance cases, both of the *agreement* shape.
  `hash-stream-updates-agree-with-hash-of-for-every-digest.nvst` asks each of `Core\Digest`'s
  six cases whether `update`/`finish` answer what `Core\Hash::of` answers, over 2769 chunkings
  of one 70-octet subject: the two degenerate ones, all 71 two-way splits, all 2556 three-way
  splits, and every fixed width 1..70 both plain and with an empty `update` interleaved before
  every chunk. 70 crosses SHA-256's block edge and its 55-octet padding mark while staying
  inside SHA-512's single block. The count is asserted against a printed denominator and the
  six digests are shown pairwise distinct, so neither half can pass vacuously.
- `hash-hmac-agrees-with-its-streamed-construction.nvst` pins `Core\Hash::hmac` against RFC
  2104's construction *streamed* — `H((K ⊕ opad) ‖ H((K ⊕ ipad) ‖ m))` as two `Core\Hash\Stream`s
  fed two chunks each — over three key shapes crossed with three messages, for each of
  `StrongDigest`'s three cases. The keys are chosen so both pads are writable by hand (zero
  octets, five 0x36 octets, and exactly one block of them), which is what makes the block width
  the two halves of `StrongDigest` disagree on — 64 against 128 — the thing under test.

## Next group

**`Core\Bytes`'s floor: `at`, `compare` and `contains`** — the file set is
`crates/nvs-stdlib/src/bytes.rs` plus `tests/conformance/core/`. All three sit at two cases and
none of them has an *edges* or *bound* case; read the registry rows together at `bytes.rs:172`
before taking the first, since the three share one parameter shape.

- [ ] **`at`'s bound, asserted on both sides** (`crates/nvs-stdlib/src/bytes.rs:172` the row,
      `:490` the implementation) — the last accepted index and the first refused one named
      together, plus the empty receiver, which has no accepted index at all. Check the `Fault::`
      constructor at the site before assuming the refusal is catchable.
- [ ] **`compare` agrees with the ordering `Core\Str::compare` gives the same octets, and is a
      total order over a swept table** (`bytes.rs:197`, `:588`) — sign, antisymmetry and the
      prefix rule counted over every pair of a small table rather than read off a line, with the
      empty buffer and a shared prefix of differing length included.
- [ ] **`contains` agrees with `Core\Bytes::indexOf` over every needle a subject contains**
      (`bytes.rs:204`, `:608`) — every substring of one subject swept, including the empty
      needle and the whole subject, so a member that grew its own search fails the count.

## Backlog

- `Core\Test` is the other class at a median of 2 (`assertCount`, `assertDoesNotThrow`,
  `assertNull`) — `python tools/gaps.py --coverage`.
- `Core\Regex\Match::offset` is the lowest single member left at 1 case — same tool.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- The unit test added to `nvs-types/src/lower.rs` is not named in `loop-goal.toml`'s guard list;
  if that list is meant to cover checker fixes, it wants an entry.
