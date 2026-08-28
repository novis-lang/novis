# Handoff

## State

**M4's Stage 8, depth.** The tree is at **778 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Encoding` left the frontier
this session (2 → 3.0) and `Core\Bytes` came off its floor of 1. The thinnest classes are now
`Core\Bytes` (median 2, floor 2), `Core\Test` (2) and `Core\Hash\Stream` (2, and only two members).
Nothing is blocked.

- Three cases landed, all of the *agreement* shape over a swept table rather than another row of
  what the existing cases already answer.
- `Core\Encoding`'s two. The round-trip case crosses eight texts with all 41 charsets the registry
  names, counting what each charset spelled against what came back byte-identical (328 pairs, 173
  spelled, 171 identical). The two survivors that are spelled but not identical are named rather
  than smoothed over: `euc-jp` and `shift_jis` both spell U+00A5 as `0x5c`, which their own
  decoders read as U+005C — the WHATWG index's many-to-one encoder, not a mode Novis added, and
  `iso-2022-jp` escaping into JIS X 0201 Roman is the contrast row that shows it. The second case
  asks `isValidText` and `decodeText` the same question about each charset's output under all 41
  charsets (1681 + 861 + 410 checks, agreement total), plus every prefix of a 1/3/4-byte UTF-8 run,
  which is where a validator written apart from its decoder diverges first.
- `Core\Bytes::join`'s: the separator arithmetic swept over part counts 0 to 8 with an empty
  element at every position, and the answer compared byte for byte with `Core\Str::join` over the
  same content — the pairing `bytes.rs`'s own doc comment and ADR 0063 R6 assert and nothing
  observed. Its live-slot walk is pinned too: two `unset` elements take their separators with them.

## Next group

**`Core\Hash\Stream`'s two members, and `Core\Hash`'s own stream/hmac rows** — the file set is
`crates/nvs-stdlib/src/hash.rs` plus `tests/conformance/core/`. Two cases already touch the
streaming pair (`hash-streams-a-digest-in-chunks.nvst` and
`hash-stream-chunking-is-not-observable.nvst`), so read those first and take the boundary each
leaves rather than another chunking row.

- [ ] **`update`/`finish` agree with `Core\Hash::of` over a sweep of chunk boundaries**
      (`crates/nvs-stdlib/src/hash.rs:243` `update`, `:250` `finish`, implemented at `:573` and
      `:601`) — the agreement shape: one input cut every way there is, counting the digests that
      equal the one-shot answer, so a stream that carried a chunk boundary into its state fails the
      count while every written-out row still prints.
- [ ] **`finish`'s edges** (`hash.rs:601`, `Core\Hash::stream` at `:215`/`:549`) — an empty stream,
      a `finish` with no `update` at all, and whatever a second `finish` or a post-`finish` `update`
      does; check the `Fault::` constructor at the site before assuming a `catch` reaches it.
- [ ] **`Core\Hash::hmac` agrees with the streamed digest of its own construction** (`hash.rs:201`,
      implemented at `:468`) — HMAC is two hashes of a padded key, so the member and the pieces it
      is built from can be asked the same question and counted.

## Backlog

- `E0401` "expected `Core\Charset`, found `Core\Charset`": a source-written `Core` enum type never
  unifies with the registry's, so no case can pass one through a parameter — the playbook bullet
  this session added has the workaround, but the checker hole is unowned (`nvs-types`).
- `Core\Test` (median 2) and `Core\Bytes`'s `at`/`compare`/`contains` (median 2) are the frontier
  after the group above — `python tools/gaps.py --coverage`.
- `docs/agent/guard-name-debt.md`: 54 of the 156 guard tests `loop-goal.toml` names match nothing
  `cargo test` runs.
- `Core\Regex\Match::offset` and `Core\Uri::buildQuery` are still at one case each.
