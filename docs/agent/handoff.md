# Handoff

## State

**M4's Stage 8, depth.** The tree is at **775 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Time` left the frontier this
session (2 → 3.0) and the thinnest classes are now `Core\Bytes` (median 2, floor 1 at `join`),
`Core\Encoding` (2), `Core\Test` (2) and `Core\Hash\Stream` (2). Nothing is blocked.

- Three cases landed, all over members carrying a single case each. `Core\Time`'s two: the three
  epoch readings are one number at three resolutions, swept over 28 second/`nanos` rows and
  counted, with `fromEpoch` rebuilding the instant from each reading through the floor
  decomposition its `uint nanos` wants — 9 of those rows read one second *higher* than they were
  written, which is the truncation-towards-zero the pair does; and `monotonic` is a twelve-read
  sweep that never steps backwards, whose differences are `Duration`s in `Duration`'s own algebra,
  plus one 20ms `sleep` the clock has to have noticed (the whole case costs 45ms).
- `Core\Test`'s: `assertThrows` consumes the throw it matched **without** discharging the ledger
  entry that raised it, so `expectFailure` around an `assertThrows(…, Core\Test\Failure::class)`
  body still discharges — `nvs_stdlib::test`'s own doc comment on `nvs_core_test_assert_throws`
  states this and nothing observed it. The failure *messages* of `assertCount` and `assertThrows`
  were already pinned by the two cases `gaps.py --member` names, so this case took the ledger edge
  instead of re-pinning them.

## Next group

**`Core\Encoding`'s three text members** — the file set is `crates/nvs-stdlib/src/encoding.rs` plus
`tests/conformance/core/`. Two cases already touch them
(`encoding-text-trio-converts-through-a-charset.nvst` and
`encoding-charset-conversion-names-the-character-it-cannot-spell.nvst`), so read those first and
take the boundary each leaves, not another row of what they already sweep.

- [ ] **`encodeText`/`decodeText` round-trip every charset the registry names**
      (`crates/nvs-stdlib/src/encoding.rs:321` `encodeText`, `:328` `decodeText`, implemented at
      `encoding.rs:751` and `:781`) — the *agreement* shape: one table of texts crossed with the
      charsets, counting the round trips that come back byte-identical against the ones that throw,
      so a charset table that grew its own transliteration fails the count.
- [ ] **`isValidText` agrees with `decodeText`'s verdict on every row of that same table**
      (`encoding.rs:335`, implemented at `encoding.rs:809`) — the predicate is the throw, spelled as
      a `bool`, and nothing yet asserts the two cannot disagree.
- [ ] **`Core\Bytes::join` is the one member with a single case** (`gaps.py --coverage` floor 1) —
      a different file set (`crates/nvs-stdlib/src/bytes.rs`), so take it only as a third.

## Backlog

- `Core\Hash\Stream::update`/`finish` carry two cases each and are the smallest class on the
  frontier — `docs/agent/loop-goal.md` § Stage 8.
- `Core\Regex\Match::offset` carries one case, `Core\Uri::buildQuery`/`compareTo` one each —
  `python tools/gaps.py --coverage`.
- 54 of the 156 guard tests `loop-goal.toml` names still match nothing — `docs/agent/guard-name-debt.md`.
