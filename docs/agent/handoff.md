# Handoff

## State

**Stage 4's two counts are the frontier — conformance 441 of 600, differential 90 of 150** — and the gap
is behavioural depth per member, not coverage: every registered member already has a case, and both of
Stage 4's named guards pass. Verify is green: **1596** tests, 74 suites, clippy and fmt clean.

**`Core\Hash`'s class members are done to depth and the section is 5 cases.** `of` is pinned per algorithm
against the document that publishes each vector (FIPS 180-4, RFC 1321, the CRC catalogue's check value),
including the empty input for all six cases — the row that exercises padding and nothing else — and a
`00 ff 80 fe` buffer that no `"…" as bytes` can spell. `hmac` is pinned against RFC 4231's Cases 1, 4, 6
and 7, which is the key-length axis: shorter than the block, binary, longer than the block and therefore
reduced first, and data longer than a block on top of an over-long key. The two rules RFC 2104 states are
written as **identities inside the case** rather than as pasted hex — an over-long key equals
`Core\Hash::of` of that key used as the key, reduced with the HMAC's own digest, and the empty key equals
an all-zero block — so they stay true if the constants behind them ever move. `equals` now answers over
unequal lengths (a prefix, two different algorithms' widths, empty against one octet), which is the case a
constant-time compare still has to answer immediately: a digest's width is never a secret.

`Core\Hash\Stream` is the one § 11 member still at a single chunking, and it is item 1 below.

## Next group

Item 1 finishes `Core\Hash`; items 2 and 3 are the same shape one module over, each a thin section's own
domain module plus its `tests/conformance/core/<name>-*.mwlt`. All three live inside the playbook's
`bytes` trap — `"…" as bytes` and `Core\Encoding::fromHex(…)` are the only two spellings and `toHex` is
the only assertion — and item 1 inside the `Core`-instance trap, since `Hash\Stream` accumulates into a
slot rather than holding a native context.

- [ ] **`Core\Hash\Stream` chunk boundaries** — `crates/mwl-stdlib/src/hash.rs:549` `stream`, `:573`
      `update`, `:601` `finish`; spec § 11's fourth row. Zero updates (the stream's own empty input, which
      must equal `Core\Hash::of("")` for the same algorithm), a value split across two and three updates
      matching `of` over the whole, a chunk boundary landing inside a compression block, and what a second
      `finish` does — the module's docs say `finish` closes the stream and `update` after it throws.
      `tests/conformance/core/hash-streams-a-digest-in-chunks.mwlt` pins one chunking today.
- [ ] **`Core\Csv` depth** — `crates/mwl-stdlib/src/csv.rs:143` `parse`, `:150` `format`, and the two
      options bags at `:171` and `:203`. One case today,
      `tests/conformance/core/csv-reads-and-writes-rfc-4180-records.mwlt`; what it does not reach is a
      non-default `separator`/`quote`/`escape`, `header: false`, an embedded newline and a quote inside a
      quoted field, and a round trip of each.
- [ ] **`Core\Validate` depth** — `crates/mwl-stdlib/src/validate.rs:163` `isEmail`, `:170` `isDomain`,
      `:177` `isIp`, `:184` `isMac`, `:191` `isAscii`, `:198` `isPrintable`. One case today,
      `tests/conformance/core/validate-members.mwlt`, which walks the roster once each way; each member
      wants the boundary its own predicate is written around (v4 against v6, a trailing dot, an empty
      label, a non-ASCII domain).

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `crates/mwl-stdlib/src/json.rs` gap 2, ADR 0071.
- ADR 0088's registry-wide qualifier classification on `mwl-stdlib`'s member rows — M8. `Core\Hash::hmac`'s
  `secret bytes $key` is the parameter that wants it first (`mwl_stdlib::hash`'s module doc).
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- After the three above, the next-thinnest sections are `path`, `heap` and `uuid` at two cases each.
- The differential corpus is 90 of 150 — `tests/differential/`, and it needs PHP on the leg.
