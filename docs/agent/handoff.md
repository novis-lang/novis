# Handoff

## State

**Conformance is at 548 of 600, and it is the only frontier left.** Verify is green (1597 cargo
tests, 74 suites, 548 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt`
trees itself, so after a green `verify.py` there is nothing else to run (playbook, *Running
things*).

This session added no library code and took **one** slice, not two: the group's remaining slices
are both on `crates/mwl-stdlib/src/uri.rs`, a file this session never opened, so AGENTS.md § 2's
file-set test stopped it at one. It ended at roughly 40k of a 200k ceiling — the cheap pairing is
[1] and [2] of the next group, which share `uri.rs` with each other.

What landed is `tests/conformance/core/time-duration-constructors-and-readers-are-one-scale.mwlt`,
a counted sweep over `crates/mwl-stdlib/src/time.rs`'s `Duration` finishing `Core\Time`: one table
of twelve counts spanning both signs and both ends of what `weeks` holds, over which each of the
eight constructors is its own one-unit literal `multipliedBy` the count (96) and each reader
inverts its own constructor (96); truncation toward zero stated as two properties of the remainder
rather than a direction — shorter than one of its own units, and never crossing zero — over
thirteen ragged counts and three readers (39 each); `multipliedBy` as iterated `plus`/`minus` over
six bases × six factors (36) with negation commuting through it (36); and `parse`/`toString` as one
grammar over eleven renderings, where every magnitude reads back but only the six non-negative ones
render to text `parse` will accept. Its tail is the range's one-nanosecond asymmetry: `negated`
is total everywhere except `i64::MIN` nanoseconds, asserted on both sides.

`orient.py`'s pack was complete for this work; nothing was fetched outside it beyond `time.rs`'s
`Duration` block and `mwl-syntax`'s `duration::render`/`parse`.

**A by-hand pass over `docs/adr/` is still in flight and is not loop work.** 103 modified ADRs plus
`ground-rules.md` have been uncommitted for five sessions now. That is [doc-cleanup.md](doc-cleanup.md)'s
pass, which AGENTS.md says the user fires and the loop never does. **Do not stage it and do not
`git commit -a`**: stage your own paths, exactly as `session.py --wrap` already does.

## Next group

Three slices, **all three on `crates/mwl-stdlib/src/uri.rs`** — so this is the cheap two-slice
session, and [1] plus [2] is the pairing. `docs/spec/01-core-library.md` § 7 owns the `Uri` rules.
`Core\Uri` is the thinnest real class at 0.53 (`Instant` 0.00 and `DateTime` 0.06 are the
playbook's false alarm — a class whose values are never spelled out reads as zero).

- [ ] **`Core\Uri`'s seven readers, `with` and `toString` are one parse** — over a table of absolute
      URIs, each reader answers the component `toString` writes back, and `with` on one component
      leaves the other six untouched. `crates/mwl-stdlib/src/uri.rs:347` (`parse`), `:405`–`:454`
      (the seven readers and `toString`), `:461` (`with`, six named options).
- [ ] **The four percent-coders are two inverse pairs over one byte sweep** —
      `encodeComponent`/`decodeComponent` and `encodeFormValue`/`decodeFormValue` are the identity
      over every byte, and the two pairs differ on exactly the bytes the form encoding spells
      differently. `crates/mwl-stdlib/src/uri.rs:361`–`:382` (the four rows), `:603` (`encode`),
      `:638` (`decode`).
- [ ] **`parseQuery` and `buildQuery` are one bracket convention** — the standing decision in
      `loop-goal.md` puts PHP's `a[]=1`/`a[b]=c` nesting in `parseQuery` in full; assert the round
      trip with `Core\Json::encode` on the parsed side (playbook: an `array<mixed>`'s elements
      cannot be indexed past the first level). `crates/mwl-stdlib/src/uri.rs:389`, `:396`.

## Backlog

- `Core\Uri::resolve` and `compareTo` — RFC 3986 § 5 reference resolution and the normalized order;
  `uri.rs:499`, `:506`. `docs/spec/01-core-library.md` § 7.
- `Core\Time\Instant`'s and `DateTime`'s depth rows read low only because their values are never
  spelled; check with `gaps.py --member` before writing to them. `docs/agent/playbook.md`.
- `Core\Json::decodeAs<T>` still reads a scalar-fielded class only — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
