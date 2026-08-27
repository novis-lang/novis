# Handoff

## State

**M4 — language completeness.** The array-literal element list is finished: item 17 is
closed, its `&value` half as `E0483` and its `...spread` half as a lowering plus the key
semantics it needed.

`[...$a]` is one `InstKind::ArraySpread` (`crates/mwl-ir/src/lower/expr.rs:3645`), which
`mwl-codegen` emits as one call to `mwl_runtime::array`'s `mwl_array_spread` — the
runtime walks the subject once and owns which keys survive. **The key rule is ADR 0007
§ 5's**, folded into its own bullet there: an integer-looking key is renumbered under the
destination's append counter, every other key is preserved. ADR 0069's Context paragraph
carries the one sentence that keeps the two consistent (a spread is written at the site;
`merge($a, $b)` is a name). `tests/differential/lang/an-array-literal-spread-renumbers-like-php.mwlt`
pins twelve rows against PHP, the append refusal included, all byte-identical.

Refcounts, valgrind-green over `.agent-tmp/spread-all.mwl` on both throwing edges: the
subject is **borrowed** (a fresh producer is staged as an owned temporary and released
after the copy), each entry copied is retained by the runtime, and the array under
construction is now itself staged and re-pointed after every write —
`Lowering::retarget_temporary`/`forget_temporary` in `lower/call.rs:496`. That last one
fixes the keyed shape too: a literal whose element expression throws used to leave the
half-built array named by nothing.

`verify.py` 6 of 6 green — conformance **584**, differential **163**. `python
tools/holes.py` is at **34 sites**; item 17's own line is gone and the 5 it still lists
are other items' panics sharing `lower/expr.rs`.

## Next group

**The call-argument spread**, item 16, in the order `loop-goal.md` § *Standing decisions*
fixes — checker first, lowering second. The two halves share nothing but the shape, so
they are two file sets, and the checker half is where to start.

- [ ] **Item 16's checker half: a named or spread *call* argument type-checks** —
      `crates/mwl-types/src/expr/args.rs:38` (`check_args_typed`) and `:117`
      (`check_arg`). The spread half is `check_spread_element`
      (`crates/mwl-types/src/expr/literals.rs:668`) one position along: a variadic
      parameter's element type is the expectation, and a subject that is not an
      `array<T>` takes the same refusal (`E0484`). A **named** argument needs the
      parameter matched by name and a resolved per-argument type recorded, which is the
      fact `crates/mwl-ir/src/lower/call.rs:75` panics for the absence of.
- [ ] **Item 16's lowering half** — `crates/mwl-ir/src/lower/call.rs:75`, once the
      checker records a per-argument type. A spread argument into a **variadic**
      parameter is `lower_variadic_tail` plus this session's `InstKind::ArraySpread`:
      the tail already builds one fresh array, and a spread element of it is the same
      copy. A spread into a fixed parameter list needs the subject's length at compile
      time and does not have it — refuse it by name rather than lowering it.
- [ ] **A positional element after an explicit key still numbers from its own
      position** — `crates/mwl-ir/src/lower/expr.rs:3645`'s keyed shape, the one
      divergence a spread-carrying literal no longer shares. `[1, "k" => 2, 3]` puts `3`
      at `"1"` where PHP puts it at `"2"`, and the fix is the `ArrayAppend` the spread
      path already takes. `lower::tests::a_positional_element_after_an_explicit_key_keeps_its_own_position_counter`
      is the snapshot that pins the current answer, and `ir::InstKind::ArrayNew`'s doc
      comment is where it is written down as deliberate.

## Backlog

- Item 1's 11 sites are catch-alls only — `mwl-codegen` gap, `docs/implementation-plan.md`.
- Items 4, 6, 7: the bitwise operators, `<=>`, `$x++` — `python tools/holes.py --item N`.
- Item 19/20: a `&$x` parameter under a closure, and `foreach (… as &$v)`.
- The two unattributed `mwl-codegen/src/ty.rs` sites belong to no item yet — `holes.py`.
- `Core\Json::decodeAs<T>`'s wider codec-reachable set — `mwl_stdlib::json` gaps.
- 20 of the 32 named `.mwlt` cases are still to write — `python tools/loop.py --list`.
