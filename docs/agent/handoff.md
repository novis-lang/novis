# Handoff

## State

**Conformance is at 538 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*). The tree is clean.

**`Core\Math` is closed but for one pair, and `Core\Path` is the next thinnest area.** The three
slices below are unchanged from the previous session and none of them has been taken — the two that
landed since (`math-the-hyperbolic-members-are-their-exponential-definitions`,
`math-the-inverse-circular-members-undo-the-circular-ones-only-on-their-own-branch`) were the ones
above them in that list.

**This session added no code. It settled the routing surface end to end and landed
[ADR 0102](../adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md),
which repairs three defects that were already in the tree.** In order of how much they cost if
missed: `#[Access]` was a compiler-recognized attribute that ADR 0096 never added to ADR 0071 § 1's
closed list, so as written a userland `type Access = {...};` satisfied the mandatory-sibling check
and the compile error passed on a decision nobody made; ADR 0096 § 4 and ADR 0097 § 7 flatly
contradicted each other about whether the server holds the route table, which is what ADR 0096's
CSRF check and ADR 0076 § 1's `route` label are both specified against; and ADR 0085 § 1 documented
a query-parameter binding no ADR had ever defined. 0102 § 1 resolves the second by making the server
match **once**, before the handler, with `Core\Request::route()` as that match — measured at
**32.7 ns** against the **135.9 ns** the server already spends parsing the request line and ten
headers, so it *removes* a match for any request that also runs CSRF or emits the `route` label
rather than adding one. The bench that produced those numbers is not in the tree; it was a
throwaway over `matchit` 0.8.6, the crate ADR 0077 § 2 names as its trie model, and the numbers are
recorded in 0102 § 1 as an upper bound because `matchit` returns captures through a map where a
compiled table writes fixed shape slots.

**Nothing routing-related is built** — `grep -rn "Router" crates/ --include=*.rs` is empty, and M4S
owns the table pass with M7 the matcher. The one already-built thing 0102 touches is
`crates/mwl-types/src/derive.rs:72`'s `ATTRIBUTES` const, which gains `Core\Route`, `Core\Access` and
`Core\Query` when M4S opens; it is already the right shape and needs no restructure. **ADR 0071 § 1
is now the one home for that roster**, as a table naming the registry it must agree with, and the
running count every amending ADR used to restate is deleted. **ADR 0077's claim that routing is a
forcing case for typed `callable` is withdrawn** (0102 § 9): routes do not share a signature,
handlers share no return type, and § 4's *boundary* ground survives the deferral either way — so the
count is back to three, and a future ADR re-arguing it must not cite routing.

## Next group

The first slice closes `Core\Math` and reads `crates/mwl-stdlib/src/math.rs`; the other two are
`Core\Path` and both read `crates/mwl-stdlib/src/path.rs`. All three add a new file under
`tests/conformance/core/` and all three are the *agreement* shape from conventions.md.
`docs/spec/01-core-library.md` §§ 3 and 7 own the rules. The tolerance spelling, the well-conditioned
direction and the exact-on-both-legs rows are all playbook bullets under *Writing a test case* — do
not re-derive any of them.

- [ ] **`format` and `round` agree wherever both name the same precision** — `Core\Math::format($n,
      {decimals: $d})` renders what `Core\Math::round($n, {precision: $d})` answers, on every row of
      a table, and parts from it only in the options `round` has no opinion about (the separators,
      whose defaults are MWL's and not `number_format`'s). `crates/mwl-stdlib/src/math.rs:110`
      (`round`), `:313` (`format`), `:449` (`FORMAT_OPTIONS`, where the empty group separator is
      decided).
- [ ] **`split` and `join` are inverses through `normalize`'s normal form** — `join` of what `split`
      returned is `normalize($p)` on every row of a table, `split` never yields an empty segment
      however many separators were repeated, and the round trip is asserted separator-free or
      through `Core\Str::replace($p, Core\Path::SEPARATOR, "/")`, per the goal's *Path and the two
      legs* decision. `crates/mwl-stdlib/src/path.rs:104` (`join`), `:111` (`split`), `:118`
      (`normalize`).
- [ ] **`relativeTo` and `join` undo each other, and the boundary is where they stop** —
      `join($base, relativeTo($p, $base))` normalizes back to `$p` over a table, and the pair parts
      exactly where no relative path exists (a different drive, `isAbsolute` disagreeing between the
      two arguments). Both sides of that bound named together. `crates/mwl-stdlib/src/path.rs:104`
      (`join`), `:118` (`normalize`), `:125` (`isAbsolute`), `:132` (`relativeTo`).

## Backlog

- `basename`/`dirname`/`extension`/`withExtension` agree on where the name ends — read
  `path-decomposes-a-path-without-touching-the-disk.mwlt` first; it may already own the claim.
- `Core\Csv` is 2 members over 5 cases and `Core\Validate` 5 cases — the two thinnest areas after
  `Core\Path`; `python tools/gaps.py` is the worklist (`docs/implementation-plan.md`, *Open now*).
- `Core\Json::decodeAs<T>`'s decoder reads scalar-fielded classes only — ADR 0071,
  `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `docs/adr/0097-development-server-and-proxied-origin.md:47` links `../plan/m13.md`, which that ADR
  itself deleted. Pre-existing and unrelated to routing; the link should go, not the sentence.
