# Handoff

## State

**Conformance is the only frontier left, at 488 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **488 passed, 0 failed** and `mwl test tests/` is **639 passed,
0 failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` is the worklist and it is down to 100 sites from 109.**
`Core\Bytes::pack` is closed: its six argument refusals and its three range refusals each have a
case asserting the *message*, which is what parts a real boundary from a member that merely threw.
Both cases are the *Edges* and *bound-asserted-on-both-sides* shapes and both close with a counted
claim; the plan's `Open now` holds what each counts and why.

**A `fatal` site in that list is not case-assertable.** Four of `bytes.rs`'s eight are
`Fault::fatal` — `pack`'s two argument-tag checks, `unpack`'s and `join`'s — and no handler sees
one, so `--errors`' remaining 100 are not all reachable work. Check the `Fault::` constructor at
the site before planning a case around it (playbook, *Writing a test case*).

**`orient.py`'s `[context] modules` was short again.** It printed `src/arr.rs`, `src/registry.rs`
and `src/str.rs`; `src/bytes.rs` — the file both slices are about — was absent, exactly as the
previous session reported. The next group needs `src/bytes.rs` **and** `src/encoding.rs`. The
`adrs` field was adequate this time (0009 §§ 1 and 3, 0007 § 3 are the ones the work used);
ADR 0063 R4 is named by the items and its text was not needed.

## Next group

All three slices share `crates/mwl-stdlib/src/bytes.rs`, `crates/mwl-stdlib/src/encoding.rs` and
`tests/conformance/core/`, and each is the *Edges* shape over a `Fault::thrown`. `gaps.py --errors`
already holds the anchors, so do not re-derive them. Spec § 7 owns the `Bytes` rows and § 8 the
`Encoding` ones. Take them in this order.

- [ ] **`Core\Bytes::unpack`'s buffer bounds and the two single-value ones** —
      `crates/mwl-stdlib/src/bytes.rs:1136` (a code reading past the buffer's end),
      `:1244` (a format that leaves octets unread, the header-then-body case its message names),
      `:459` (`Core\Bytes::at` outside the buffer) and `:618` (`Core\Bytes::fill` given something
      that is not one octet). This empties `bytes.rs`'s reachable list. The bound is asserted on
      both sides: the last offset a code reads at, beside the first it cannot.
- [ ] **`Core\Encoding`'s four decoder refusals** —
      `crates/mwl-stdlib/src/encoding.rs:840` (`fromBase64`), `:872` (`fromBase64Url`),
      `:915` (`fromBase32`) and `:960` (`fromHex`). One *Agreement* case: four decoders sharing
      one rule, asserting that they **agree** about what is not their alphabet, what padding they
      accept and what length they refuse, rather than what each answered on its own line.
      `Core\Encoding::toHex` is already every `bytes` case's assertion spelling, so the round trip
      is the invariant to count.
- [ ] **`Core\Encoding`'s charset pair** — `crates/mwl-stdlib/src/encoding.rs:755`
      (`encodeText`: the target `Core\Charset` has no spelling for a character) and `:785`
      (`decodeText`: the byte sequence at an offset is not that charset's). ADR 0009 § 3's
      checked-never-repaired rule is the whole reason neither substitutes a U+FFFD; the two are
      each other's inverse only where both succeed, which is the counted claim.

## Backlog

- The eight members with an uncalled PHP twin (`gaps.py --differential`) — the gate they feed is
  met, so they are off-path under the goal's *Backlog items are off-path* rule.
- `Core\Str::wrap`'s multibyte `--ORACLE-DIVERGES--` file: the one item of the closed `Core\Str`
  section that is not counted (plan, `Open now`).
- `src/time.rs` × 19, `src/arr.rs` × 18 and `src/str.rs` × 13 unasserted `Fault::` sites — the
  three largest blocks left in `gaps.py --errors`.
- `src/json.rs` × 9, `src/uri.rs` × 7 and `src/path.rs` × 6, the next tier of the same list.
- `Core\Json::decodeAs<T>`'s decoder past a scalar-fielded class (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification, absent from every `mwl-stdlib` member row
  (`mwl_stdlib::hash`'s module doc).
