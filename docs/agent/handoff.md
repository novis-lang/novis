# Handoff

## State

**Conformance is at 550 of 600, and it is the only frontier left.** Verify is green (1597 cargo
tests, 74 suites, 550 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt`
trees itself, so after a green `verify.py` there is nothing else to run (playbook, *Running
things*).

This session took **two** slices, both on `crates/mwl-stdlib/src/uri.rs`'s spec § 12 surface, and
added no library code. It ended near 55k of the 200k ceiling. What landed:
`uri-seven-readers-with-and-tostring-are-one-parse.mwlt`, a counted sweep over eight absolute URIs
where RFC 3986 § 5.3's recomposition written out of the seven readers must equal `toString()` (56
recompositions, counting the 48 `Uri`s `with` builds on top of the 8 parsed), each of the six `with`
options carries the other six components through untouched (288) and replaces its own (48), with a
tail on the empty-port exception; and
`uri-percent-coders-are-two-inverse-pairs-over-a-byte-sweep.mwlt`, which sweeps all 128 ASCII bytes
through the four coders' four cross-products and counts the one direction that is not the identity.
Both findings are a playbook bullet under *Divergences and refusals already pinned*.

`orient.py`'s pack was complete; nothing was fetched outside it beyond `uri.rs`'s registry rows,
`encode`/`decode`/`with`, and the six existing `Uri` cases' titles.

**`gaps.py --coverage`'s thinnest-class ranking has stopped being a worklist.** `Core\ObjectMap`
(0.78) and `Core\ObjectSet` (0.89) have ten dedicated cases between them, `Core\Validate` (0.83) has
five including two sweeps, and `Core\Random` (0.86) has six. Every §§ 1–12 class has had a pass, so
the ratio now measures member count rather than depth. The seam with room left is conventions.md's
*agreement* shape — one question asked of every member that shares a rule — which is what the next
group is.

**A by-hand pass over `docs/adr/` is still in flight and is not loop work.** 103 modified ADRs plus
`ground-rules.md` have been uncommitted for six sessions now. That is [doc-cleanup.md](doc-cleanup.md)'s
pass, which AGENTS.md says the user fires and the loop never does. **Do not stage it and do not
`git commit -a`**: stage your own paths, exactly as `session.py --wrap` already does.

## Next group

Three slices on **one rule and one file set** — `crates/mwl-stdlib/src/ordering.rs`'s
`compare_values` and its four call sites in `arr.rs`, `math.rs` and `heap.rs`. [1] and [2] share
every anchor, so they are the pairing. Nothing here needs a new member: the whole group asks whether
the members that already share this helper agree.

- [ ] **One total order, asked of every member that shares it** — `Core\Arr::sort`, `Core\Arr::min`,
      `Core\Arr::max`, `Core\Math::min`, `Core\Math::max`, `Core\Math::clamp` and `Core\Heap` all
      order through the same helper, so over one table they must agree about which value is smallest
      and about the sign of every pair, asserted by counting agreements rather than by echoing each
      answer. `crates/mwl-stdlib/src/ordering.rs:33` (`compare_values`),
      `crates/mwl-stdlib/src/arr.rs:2894` (`sort`), `:4368` (`min`/`max`),
      `crates/mwl-stdlib/src/math.rs:867` (`clamp`, `min`/`max` at `:1246`),
      `crates/mwl-stdlib/src/heap.rs:278`.
- [ ] **The refusal is one refusal** — a pair with no natural order raises `Fault::thrown` from that
      same helper, which a `catch (Throwable $e)` does reach (playbook), so every member above
      refuses the *same* mixed subject and the messages differ only in the member name. Assert that
      they agree about refusing, and that each names itself. Same anchors as [1].
- [ ] **`Core\Arr::sort`'s one deliberate divergence from PHP's `sort`** — `arr.rs:2777`'s doc
      comment names it; pin it as its own file rather than inside [1], because the gate counts files.

## Backlog

- `Core\Uri::parseQuery`/`buildQuery` as one counted round trip over a table of queries, with
  `Core\Json::encode` on the parsed side — `uri.rs:389`, `:396`, bracket rules at `uri.rs:126`.
  Three cases already read the convention row by row, so this is depth, not coverage.
- `Core\Json::decodeAs<T>`'s decoder still reads a scalar-fielded class only — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row — `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `mwl-ir` gaps 1 and 18: a property's declared default, `do`/`while`, a closure called through the
  variable holding it, ADR 0043's `by`-delegation.
- `gaps.py --differential` still names 9 members with a PHP twin and no oracle case, all
  `Core\Time` and `Core\Encoding`; the differential gate is already met at 159 of 150.
