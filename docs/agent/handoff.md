# Handoff

## State

**M4's Stage 8, depth.** The tree is at **805 conformance plus 189 differential**.
`Core\Uri` is no longer the thinnest class on the board: three sessions of depth took it
from median 3.0 to **4.0**, and `python tools/gaps.py --coverage` now ranks `Core\Bytes`,
`Core\Encoding`, `Core\Test`, `Core\Random` and `Core\Debug` at 3.0 above it. Nothing is
blocked.

The gap recorded last session still stands and is still worth a session on its own: **no
`Core` class reaches `nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists
while `$a < $b` over two `Uri`s is `E0411`. It is in the backlog below.

Three cases landed, all over `crates/nvs-stdlib/src/uri.rs`:

- **`uri-resolve-stops-at-a-base-that-is-not-absolute-and-every-refusal-is-catchable.nvst`**
  — the other half of the § 5.4 abnormal table, which is all rows that resolve. A corpus of
  fourteen bases is swept and the verdicts *counted*: 4 answer, 6 are refused as relative
  references and 3 as opaque, 0 unaccounted, so a member reading `//host/path` as if it
  were `http://host` fails here. Three minimal pairs name the bound — `http:/rooted` against
  `http:g`, `http://a` against `//a`, `file:///x` against `file:x` — each one token apart
  and on opposite sides. The reference side is bounded too: `""` is the last accepted one,
  and ungrammatical text is a *third* catchable message naming `resolve` rather than
  `parse`. Both base-side messages are pinned once and compared against thereafter.
- **`uri-path-is-the-one-component-that-always-exists.nvst`** — `path` is the only reader
  typed `string` rather than `?string`, which is a claim about a grammar, so it is counted
  over a fourteen-row corpus: path is absent 0 times where scheme is 5, host 7, query 11,
  fragment 11, userInfo 13 and port 13. Seven of those paths are `""`, and every one of the
  fourteen appears in the recomposed text. The static half needs no assertion and cannot
  have one — `$uri->path() == null` does not compile under ADR 0090 § 4 — and the case says
  so where the `??` it does not need would have been.
- **`uri-decode-form-value-is-decode-component-plus-the-plus.nvst`** — the twin, asserted as
  agreement. Over the 95 printable ASCII bytes the two decoders agree on 94 and differ on
  exactly `[+]`; once each byte is run through `encodeComponent` they agree on all 95 and
  round-trip all 95, `%2B` included. Then one shared decoder, counted rather than rewritten:
  six malformed-escape rows answer identically under both members (five decode to
  themselves, `%%41` being the sixth because the scan steps one byte rather than giving up),
  and all five ADR 0009 non-UTF-8 families are refused by both. Checked against PHP 8.5's
  `urldecode`/`rawurldecode`, which differ on exactly `chr(43)` over the same sweep.

## Next group

**`Core\Bytes`'s floor** — the three members `gaps.py --coverage` names at two cases each,
over one file set: `crates/nvs-stdlib/src/bytes.rs` plus `tests/conformance/core/`. Check
what each is already asked before writing: `ls tests/conformance/core/ | grep bytes`.

- [ ] **`Core\Bytes::at`'s bound, on both sides** (`bytes.rs:490`) — one case. The last
      in-range index and the first out-of-range one, named together, at both ends and over
      an empty receiver; and whether an out-of-range read is a catchable throw or a `?int`.
- [ ] **`Core\Bytes::endsWith` and `startsWith` agree at their shared edges**
      (`bytes.rs:629`) — one case. The empty needle, a needle longer than the subject, a
      needle equal to it, and the fact that the two answer the same question from two ends,
      counted over one swept table rather than read off a line.
- [ ] **`Core\Bytes::repeat`'s degenerate counts** (`bytes.rs:669`) — one case. Zero, one,
      an empty subject repeated, and the first count the member refuses; `Core\Str::repeat`
      takes a `uint`, so check this one's parameter type before writing the sweep.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `$a < $b` over two `Uri`s is
  `E0411` while `compareTo` exists — docs/agent/loop-goal.md, a session of its own.
- `Core\Encoding`'s floor (`toBase32`, `toBase64`, `toBase64Url` at two cases each) —
  `crates/nvs-stdlib/src/encoding.rs:890`, the next group after `Core\Bytes`.
- `Core\Test`, `Core\Random` and `Core\Debug` are also at median 3.0 — `gaps.py --coverage`.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  docs/agent/guard-name-debt.md.
- `Core\Str` and `Core\Arr` have single-case members (`fold`, `graphemes`, `column`,
  `flattenDeep`) under an otherwise deep median — `gaps.py --coverage`.
