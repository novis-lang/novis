# Handoff

## State

**M4's Stage 8, depth.** The tree is at **802 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Uri` is still the
thinnest class on the board at median 3.0, but its three *floor-2* members are now different
ones — this session took `buildQuery`, `compareTo` and `decodeComponent`, and the tool now
names `decodeFormValue`, `path` and `resolve`. Nothing is blocked.

One finding worth a session, recorded as a playbook bullet and a backlog line rather than
acted on here: **no `Core` class reaches `nvs_hir::implements_interface`**, so `Core\Uri`'s
`compareTo` exists while `$a < $b` over two `Uri`s is `E0411`. The case landed pins the
current, reachable truth — `compareTo` written out is the only order two `Uri`s have — and
says so in its own comments, so closing the gap does not invalidate it.

The three cases landed are the invariance, the bound-and-refusals and the edges shapes:

- **`uri-build-query-is-the-form-encoder-plus-its-joiners-over-a-swept-table.nvst`** — a table
  of twelve names, each also its own value, with three properties counted over all of it:
  a one-pair string is exactly `encodeFormValue` on both halves joined by `=` (so the escaping
  is the encoder member's and not a second one that could drift), the halves are not
  interchangeable, and `parseQuery` reads each back. Then the whole table as one map: twelve
  pairs, eleven separators, the text pinned byte for byte against PHP's `http_build_query`,
  and the parsed result equal to the map key order included. Plus `scalar_text`'s own row —
  a `bool` writes `1`/`0`, a `null` drops its pair — and the empty map's empty string.
- **`uri-compare-to-answers-one-of-three-values-and-is-the-only-order-two-uris-have.nvst`** —
  the bound the two existing cases cannot see: all 36 ordered pairs of a six-URI corpus answer
  exactly `-1`, `0` or `1`, with all three occurring (14/8/14), so a byte difference would fail
  here while still being a total order. The corpus notes that § 6.2.2 is *syntax-based* and
  stops there — `http://a:80/` is not a third spelling of `http://a/`. Then the refusals:
  `Core\Arr::min`, `max` and `sort` each throw on an object subject, and the way through is
  `{comparator: ...}` calling this member.
- **`uri-decode-component-is-an-escape-only-when-it-is-well-formed-and-refuses-a-non-utf-8-octet.nvst`**
  — the two edges with the value on each side. A `%` is an escape only with two hex digits, so
  `a%`, `a%4` and `a%zzb` decode to themselves and `%41` to `A`, all four agreeing with PHP's
  `rawurldecode`; and the ADR 0009 divergence, where five families of invalid sequence — a lone
  `%FF`, a truncation, a surrogate, an overlong form and an out-of-range lead — are all refused
  (5/5, counted, because validating the first octet alone accepts three of them), with one
  message pinned. On the accepting side: `%00` is an ordinary character, and the multi-byte
  sequences decode to the one character each spells.

## Next group

**`Core\Uri`'s remaining floor**, the three members `gaps.py --coverage` now names, over the
same file set as this session: `crates/nvs-stdlib/src/uri.rs` plus `tests/conformance/core/`.
Seventeen-plus `uri-*` cases exist, so check what each member is already asked before writing:
`ls tests/conformance/core/ | grep uri` and `sed -n '2p'` over the hits.

- [ ] **`Core\Uri::resolve`'s refusals, asserted as a bound** (`uri.rs:1628`) — one case. The
      RFC 3986 § 5.4 abnormal table is already run row by row; what no case asks is where
      resolution *stops* — a base that is not absolute, a base with no scheme — and whether
      every refusal is the one catchable `Fault::thrown` message rather than a plausible answer.
- [ ] **`$uri->path` is never `null` and is the one component that always exists** (`uri.rs:1503`)
      — one case. Counted over a corpus spanning all five RFC 3986 § 3.3 shapes: an empty path,
      a rootless one, an absolute one, one with an authority, and one from `resolve`'s output;
      the invariant is that `path()` answers a `string` on every one while the other six readers
      answer `?string`.
- [ ] **`Core\Uri::decodeFormValue`'s edges** (`uri.rs:1772`) — one case, the twin of the
      `decodeComponent` case landed this session: `+` is a space and `%2B` the typed `+`, the
      malformed-escape rule is the same, and the non-UTF-8 refusal is the same message under a
      different member name — asserted as *agreement* with `decodeComponent` over a table where
      the two must answer alike, and disagreement on exactly the two bytes the form encoding
      spells differently.

## Backlog

- No `Core` class implements an interface as far as `nvs_hir::implements_interface` is
  concerned, so ADR 0013's `<`/`<=>` over two `Core\Uri` (or `Core\Time\Date`) is `E0411` —
  `crates/nvs-stdlib/src/registry.rs` records no interface list. Owned by ADR 0013 and the
  registry's own module doc.
- `mixed as Core\Uri` is `E0711` with a message saying the target "names no class", which is
  the same missing registry fact seen from `nvs_types`' conversion table.
- Stage 8's depth ranking is `python tools/gaps.py --coverage`; `--errors` still lists the
  unasserted refusal sites, and `docs/agent/guard-name-debt.md` still owns the 54 guard names
  the acceptance gate never reaches.
