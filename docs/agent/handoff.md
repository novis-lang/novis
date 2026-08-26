# Handoff

## State

**Stage 4's two counts are the frontier — conformance 462 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean)
and `mwl test tests/conformance` is **462 passed, 0 failed** — run it as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice).

**§ 9's two collections and `Core\Regex` are now done to depth**, at 10 and 8 cases, joining `hash`,
`csv`, `validate`, `out`, `heap`, `uuid`, `path`, `json`, `random`, `bytes` and `encoding`. The
collections are pinned by what identity *is* — ADR 0090 § 3's table read one row per representation, so
`1`/`1 as uint`/`1.0` are one member while `"1"`, `true` and `null` are three more; an array is one
member per content *in order*; a string is its bytes however it was built; and an object is only itself,
so three same-named tags are three keys and a mutated key is still its own — by the map's two lists
staying positionally paired through a re-`set` (replaces in place), a removal (closes the same gap in
both) and a re-add (lands at the end), and by a twelve-tag sweep of scattered removals whose survivors
are proved equal to the directly-built set with `diff` in both directions. Regex is pinned by the
replacement grammar's accepted and rejected spellings — `$0`/`$1`/`${name}`/`$$` accepted, `\1` left
literal, an unknown group empty, and `$10` against `${1}0` as the boundary — and by `split`'s `limit` at
the exact piece count, one past it, and a negative that drops every piece, with `keepEmpty` proved to
apply *after* the limit.

**Three case shapes are established** and named in the plan's *Open now*: a section's edges, invariance
over a sweep, and a bound asserted on both sides. Reuse them rather than inventing a fourth.

## Next group

The three sections that have not had a depth pass, weakest first. Each is its own domain module plus its
`tests/conformance/core/<name>-*.mwlt`; the file set they share is
`crates/mwl-stdlib/src/{math,uri,time}.rs` and `tests/conformance/core/`.

- [ ] **`Core\Math` depth** — `crates/mwl-stdlib/src/math.rs:837` `clamp`, `:861` `round`, `:892` `mod`,
      `:875` `intDiv`; spec § 3. Three `math-` cases. The shape is a bound on both sides: `clamp` at each
      end and one past it, `round`'s `RoundMode` × a half-way value (the one input every mode answers
      differently), `intDiv`/`mod` at zero and at `int`'s own extremes, where ADR 0007's overflow rule
      makes the answer a throw rather than a wrap.
- [ ] **`Core\Uri` depth** — `crates/mwl-stdlib/src/uri.rs:1769` `parseQuery`, `:1596` `resolve`,
      `:1657` `compareTo`, `:1543` `with`; spec § 12. Three `uri-` cases. `parseQuery`'s bracket
      convention is a sweep (`a[]=1&a[]=2`, `a[b]=c`, nesting), asserted with `Core\Json::encode` because
      a case cannot index an `array<mixed>` (playbook); `resolve` is RFC 3986 § 5.4's own table of
      normal and abnormal examples; `compareTo` is the content equality `==` refuses (ADR 0090 § 4).
- [ ] **`Core\Time` depth** — `crates/mwl-stdlib/src/time.rs:473` `Duration::parse` and the eight
      component readers at `:405`-`:458`; spec § 4. Two `time-` cases. `parse` shares ADR 0070's literal
      grammar, so the case is that one grammar reached from two entry points, plus each unit's boundary.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder and ADR 0071's non-scalar fields — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — `mwl_stdlib::hash` module doc.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1's remainder.
- The differential corpus is 90 of 150 — `docs/plan/M4S.md`'s Stage 4 paragraph.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0090 § 3's string/array/object equality helpers are still owed — that ADR's own body.
