# Handoff

## State

**Stage 4's two counts are the frontier — conformance 444 of 600, differential 90 of 150** — and the gap
is behavioural depth per member, not coverage: every registered member already has a case, and both of
Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean) and
`mwl test tests/conformance` is **444 passed, 0 failed** — run it as well as `verify.py`, which executes
no `.mwlt` case at all (playbook, twice).

**`Core\Hash` is done to depth at 6 cases and `Core\Csv` at 3.** `Hash\Stream` is pinned by *invariance*
rather than by a vector: a loop sweeps every two-update split of a 130-byte subject, offsets 0 through 130
inclusive, against `Core\Hash::of` over the whole, for SHA-256's 64-byte block and SHA-512's 128-byte one,
so a boundary lands inside a block, on a block edge and inside the padded tail; three-update and
empty-chunk splittings and the tag change at a block edge follow, and the zero-update stream is written as
the identity against `Core\Hash::of("")`. `Core\Csv` gains the non-default dialect on both legs with a
round trip of each, the same document read with and without `{escape: "\\"}` — which is two different
documents, which is why the option exists — the four bytes the writer quotes on, the three dialect
refusals with their messages, and every ragged-record rule `crates/mwl-stdlib/src/csv.rs`'s module doc
states.

A `for` header, `!`, a ternary and `Core\Str::slice`/`repeat` all lower, so an invariance *sweep* is
available to a `.mwlt` case and is a far stronger assertion than a pasted constant wherever a member has
an identity to compare against.

## Next group

Three thin § 12 / § 7 sections, each its own domain module plus its `tests/conformance/core/<name>-*.mwlt`.
The file set they share is `crates/mwl-stdlib/src/{validate,out,path}.rs` and `tests/conformance/core/`.
None needs the `bytes` trap; item 3 lives inside the standing decision about `Path::SEPARATOR` differing
between the two legs, so a case must normalize a built path or assert something separator-free.

- [ ] **`Core\Validate` depth** — `crates/mwl-stdlib/src/validate.rs:163` `isEmail`, `:170` `isDomain`,
      `:177` `isIp`, `:184` `isMac`, `:191` `isAscii`, `:198` `isPrintable`; spec § 7. One case today,
      `tests/conformance/core/validate-members.mwlt`, which walks the roster once each way; each member
      wants the boundary its own predicate is written around (v4 against v6, a trailing dot, an empty
      label, a label over 63 bytes, a non-ASCII domain, a `DEL` against a space in `isPrintable`).
- [ ] **`Core\Out` depth** — `crates/mwl-stdlib/src/out.rs:96` `mwl_core_out_capture`; ADR 0088 § 5, which
      is what makes the result the sink's *carrier* rather than a `string`. One case today,
      `tests/conformance/core/out-capture-answers-the-sinks-carrier.mwlt`. What it does not reach: a
      nested capture, a capture over an `echo` of several operands, a capture that throws part way, and
      re-emitting a captured carrier with `echo` — the round trip that ADR names as the reason for the
      type.
- [ ] **`Core\Path` depth** — `crates/mwl-stdlib/src/path.rs:589` `join`, `:664` `normalize`; spec § 7.
      Two cases today, `path-decomposes-a-path-without-touching-the-disk.mwlt` and
      `path-join-normalize-and-relative-to.mwlt`. What is thin: `..` climbing past the root, a trailing
      separator, an empty segment in `join`, an absolute segment joined onto a relative base, and
      `relativeTo` where the two share no prefix.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder and ADR 0071's non-scalar fields — `mwl_stdlib::json` gap 2.
- ADR 0088's registry-wide qualifier classification for every `Core` member row — `mwl_stdlib::hash` doc.
- ADR 0090 § 3's string/array/object equality helpers — the plan's `Open now`.
- `do`/`while` does not lower — `mwl-ir`'s own module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- Differential is 90 of 150 and no session has moved it lately — `docs/plan/m4s.md`.
