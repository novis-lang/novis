# Handoff

## State

**M4's Stage 8, depth.** The tree is at **793 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Uri` left the frontier this
session — `compareTo` and `buildQuery` were its two floor-1 members and each gained a depth case, so
the class's floor is 2 and `Core\Time\Zone` is now the only class ranked thinner. Nothing is blocked.

Two of the three items the previous handoff named were **already on disk** and were not rewritten:
the four encoders' round-trip sweep is
`uri-percent-coders-are-two-inverse-pairs-over-a-byte-sweep.nvst` (which already counts both pairs
over all 128 ASCII bytes, both cross directions, and the two-byte disagreement), and `with` agreeing
with the seven readers is `uri-seven-readers-with-and-tostring-are-one-parse.nvst`. The playbook
bullet this session added owns the check that finds this in two seconds.

- **`uri-compare-to-is-a-total-order-and-absent-sorts-before-empty.nvst`** asserts what no
  normalization row can: that the answer is an *order*. All 100 ordered pairs of a ten-reference
  corpus are counted for antisymmetry, for being one of ADR 0013's three literals, and for the
  zeros landing exactly on the diagonal plus the one pair § 6.2.2 folds together; all 1000 triples
  are counted for transitivity, which is the property a sort actually depends on and the only one
  needing three references to state. Which order it is is then named — components in the readers'
  declared order, with the port compared as a *number*, witnessed by `:9` sorting before `:10`
  while `Core\Str::compare` on the two texts answers the other way. The edge is that an absent
  component sorts before an empty one, so `…/`, `…/?` and `…/#` are three references and not two.
  ADR 0090 § 4 closes it: two `Uri`s comparing `0` are still two objects under `==`.
- **`uri-build-query-round-trips-except-where-the-wire-cannot-tell-two-keys-apart.nvst`** counts
  the pair's inverse property over eight query strings in both senses — the array survives
  `parseQuery(buildQuery(...))`, compared whole through `Core\Json::encode`, and the text is a
  fixed point so nothing is escaped twice. Key order is the array's own in both directions, which
  is what a signed request depends on. The stronger half is the bound the sweep stays inside: the
  inverse holds for every array `parseQuery` produced and not for every array, and the two keys the
  wire cannot spell are named — `"a[b]"` is escaped correctly and reads back as the nested value it
  is now indistinguishable from, and an empty name is written as a bare `=v` and dropped by the
  reader. PHP's `http_build_query`/`parse_str` lose both the same way, checked against 8.5.9.

## Next group

**`Core\Time\Zone`'s floor** -- `gaps.py --coverage` ranks it thinnest at median 3.0 with a floor of
1, four members and 17 cases. The file set is `crates/nvs-stdlib/src/time.rs` plus
`tests/conformance/core/`. **Run `ls tests/conformance/core/ | grep -i zone` and `sed -n '2p'` over
the seven hits first** — this session's playbook bullet is exactly why.

- [ ] **`Zone::system` is asserted by invariance, never by value** (`time.rs:1902`) -- one case. It
      reads the host, so a conformance case cannot name its answer: assert instead that it is
      *stable* across calls, that its id round-trips through the zone reader, and that a `DateTime`
      built in it agrees with one built in the zone that id names. The *invariance over a sweep*
      shape, with the sweep being repetition.
- [ ] **`Zone::fixed`'s offset bound, on both sides** (`time.rs:1875`) -- one case. The last
      accepted offset and the first refused one, named together, plus what a fixed zone's id and
      `offsetAt` answer when there is no rule to look up.
- [ ] **`offsetAt` across a transition** (`time.rs:1921`) -- one case. The instant before and the
      instant after a DST change in one named zone, plus a zone that has never had one, which is
      the *bound asserted on both sides* shape over a table rather than a scalar.

## Backlog
- `Core\Bytes` (floor 2, 13 members) and `Core\Encoding` (floor 2, 11 members) are the next two
  after `Core\Time\Zone` -- `python tools/gaps.py --coverage`.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs --
  `docs/agent/guard-name-debt.md`.
- `array<T> as array<U>` does not lower, which is what leaves `Core\Csv::format`'s
  non-`string`-cell refusal unreachable from source -- the playbook's own bullet.
