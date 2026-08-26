# Handoff

## State

**Stage 4's two counts are the frontier — conformance 455 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean)
and `mwl test tests/conformance` is **455 passed, 0 failed** — run it as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice).

**`Core\Random` and `Core\Bytes` are each done to depth at 4 and 6 cases**, joining `hash`, `csv`,
`validate`, `out`, `heap`, `uuid`, `path` and `json`. Random is pinned by the one shape a draw admits —
invariance counted over a run: 200 die draws all inside `[1, 6]` with both endpoints reached as the
sweep's own extremes, a range whose width overflows `int` (`-2^62 … 2^62-1`), `float()` filling both
halves of `[0, 1)`, 24 buffers all of their stated length and none repeated, 120 `pick`s whose
`Arr::diff` against the subject is empty and whose distinct count is the whole subject, and `shuffle`
over a subject carrying duplicates, where sorting the answer has to give the sorted subject back — the
one assertion a re-draw over the element set cannot pass. Bytes is pinned by each `pack` integer code's
range being the *union* of its width's signed and unsigned halves, asserted at both accepted extremes
and at the value one past each end, by `unpack` reading every code but `c` back unsigned, and by
`slice`'s window clamping at the boundary offsets rather than one past them.

**Three case shapes are established** and named in the plan's *Open now*: a section's edges, invariance
over a sweep, and a bound asserted on both sides. Reuse them rather than inventing a fourth.

## Next group

Three sections that have not had a depth pass, each its own domain module plus its
`tests/conformance/core/<name>-*.mwlt`. The file set they share is
`crates/mwl-stdlib/src/{objmap,objset,math,regex}.rs` and `tests/conformance/core/`.

- [ ] **§ 9's two collections depth** — `crates/mwl-stdlib/src/objmap.rs:212` `set`, `:256` `get`,
      `:293` `remove`, `:359` `iterate`, `crates/mwl-stdlib/src/objset.rs:237` `add`, `:262` `has`,
      `:324` `union`; spec § 9. Five `object-` cases. The edges are identity ones under ADR 0090 § 4:
      two equal-content objects are two keys, the same object re-`set` replaces rather than appends,
      `remove` of an absent key, and iteration order being insertion order through a remove.
- [ ] **`Core\Regex` depth** — `crates/mwl-stdlib/src/regex.rs:1014` `replace`, `:1193` `split`;
      spec § 6. Six cases over ten members. The boundaries: an empty match and what it does to
      `split`'s and `replace`'s cursor, a `$1` referring to a group that did not participate, the
      `limit` option at 0 and at 1, and ADR 0056's two tiers — which patterns reach the backtracking
      one and what its budget refuses.
- [ ] **`Core\Math` depth** — `crates/mwl-stdlib/src/math.rs:837` `clamp`, `:861` `round`, `:892`
      `mod`, `:974` `log`; spec § 3. Eight cases over 38 members, the thinnest per member left. The
      boundaries: `int` overflow at each arithmetic member, `mod` and division by zero, every
      `RoundMode` at a tie, `log` at and below zero, and the `int`/`uint`/`float` row that does not
      lower (playbook — `$n + $f` still fails in codegen, so check a row before writing it).

## Backlog

- `Core\Json::decodeAs<T>`'s decoder and ADR 0071's non-scalar fields — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — `mwl_stdlib::hash` module doc.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1's remainder.
- The differential corpus is 90 of 150 — `docs/plan/M4S.md`'s Stage 4 paragraph.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0090 § 3's string/array/object equality helpers are still owed — that ADR's own body.
