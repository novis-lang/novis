# Handoff

## State

**Stage 4's two counts are the frontier — conformance 475 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean)
and `mwl test tests/conformance` is **475 passed, 0 failed** — run it as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice).

**`verify.py` is no longer deterministic, and the cause is a use-after-free, not a flake.** It is the
next group's first slice and the anchors are there. `Core\Arr`'s depth pass — the last section's — is
two slices of three in: the window pair and `sort` have landed, `unique` has not.

**`Core\Arr`'s window pair and `sort` are now done to depth.** The window is pinned by the same 72
sign combinations `Core\Str`'s pair sweeps, over an array, plus the cross-unit claim that joining the
sliced entries is slicing the joined characters — so the two members read *one* window in two units —
and by `chunk`'s run `k` being `slice($a, k * $size, $size)` over every size from the first that
terminates to one past the subject's own count, with `size: 0` refused by its own sentence. `sort` is
pinned as two invariants rather than another table: the answer is a permutation of the subject counted
*by multiplicity* under all ten combinations of its four options — a member answering the set of its
entries would pass a `diff`-both-ways check and fails this one — and equal keys keep their input order
across five more, which is what makes `Core\Order::Desc` reverse the comparison rather than the result.
The descending answer is asserted not to be the ascending one read backwards; reversing the result, the
obvious implementation, would fail that line.

**Three case shapes are established** and named in the plan's *Open now*: a section's edges, invariance
over a sweep, and a bound asserted on both sides. Reuse them rather than inventing a fourth. A `class`
with a `public static function`, declared at the top of the case file, is how a sweep factors out a
predicate — `mwl-ir` gap 1 still refuses calling a closure through the variable holding it, and both
new `arr` cases use a class instead.

## Next group

The first slice is priority 1 and shares no files with the other two; that ordering is AGENTS.md's, not
a grouping choice. Slices 2 and 3 share `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`.

- [ ] **The uncaught-throw use-after-free** — `crates/mwl-codegen/tests/throwing.rs:107` is the failing
      test, `crates/mwl-codegen/tests/common/mod.rs:67` `run_with` the harness around it,
      `crates/mwl-runtime/src/ctx.rs:702` `pending` the read that sees freed memory,
      `crates/mwl-runtime/src/ctx.rs:348` `Pending::message` the borrow behind it, `ctx.rs:724`
      `take_thrown` the path `mwl run` uses instead (stable over a dozen runs, which is the clue), and
      `crates/mwl-runtime/src/object.rs:1220` `drop_one` where the misaligned dereference lands. Repro:
      run `target/debug/deps/throwing-*.exe --test-threads=1 an_uncaught_throw_leaves_the_status_and_the_message_on_the_context`
      in a loop of 40 — 22 failed. ADR 0002 owns the propagation rule; this is a refcount edge inside
      it, so it wants the WSL valgrind leg as well as `verify.py`.
- [ ] **`Core\Arr::unique` depth** — `crates/mwl-stdlib/src/arr.rs:3462`; spec § 2 and ADR 0090 § 3. One
      `arr-` case reading that ADR's identity table one row per representation, exactly as § 9's
      collections already do — and `contains`/`diff`/`intersect` share the comparison
      (`crates/mwl-runtime/src/identity.rs`), so they belong in the same case rather than a fourth.
- [ ] **The section after `arr`** — with `arr` finished, every section has had its pass, so the next
      depth target is chosen by re-reading the earliest passes against the three shapes: `encoding` and
      `csv` were written before the shapes existed. Say which in that session's own handoff.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2, and ADR 0071's non-scalar fields.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — `mwl_stdlib::hash` module doc.
- ADR 0086 § 1's substitution table for the terminal sink — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- Calling a closure through the variable holding it — `mwl-ir` gap 1's remainder.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
