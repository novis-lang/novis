# Handoff

## State

**Stage 4's two counts are the frontier — conformance 473 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean)
and `mwl test tests/conformance` is **473 passed, 0 failed** — run it as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice).

**`Core\Time` and `Core\Str` are now done to depth, leaving `Core\Arr` the only section that has never
had a pass.** Time is pinned by ADR 0070's grammar reached from its second entry point: a literal's
refusals are compile errors, so `Duration::parse` is the only place a program can watch the grammar say
no, and all eleven refusals are asserted by their exact sentence and as a `ParseError`; and by each
unit's carry, one count below the next unit's length and at it, with the type's range at `i64::MAX`
nanoseconds accepted and one past refused — a `ParseError` from `parse` but a `RuntimeError` from a
constructor, since one is a text that will not read and the other a value that will not fit. Str is
pinned by `slice` and `replaceRange` being proved to read *one* window — substituting a window with its
own content restores the subject over all 72 sign combinations of offset and length, which is the
agreement PHP's `substr`/`substr_replace` pair does not manage — by `split`'s `limit` at the exact piece
count, one past it and the negative that drops every piece, and by the input at which `replace` and the
pad members are the identity (limit `0`, an empty search, a width the subject already has), with empty
padding refused only at the first width where it would have to be used.

One implementation edit rode with the Str slice: `padding_run`'s throw said `Core\Str::padStart:` where
every other member says `Core\Str::padStart():`, and the case that pins the message is what found it.

**Three case shapes are established** and named in the plan's *Open now*: a section's edges, invariance
over a sweep, and a bound asserted on both sides. Reuse them rather than inventing a fourth.

## Next group

`Core\Arr` is the last section with no depth pass, and it is large enough to be three slices rather than
one. The file set they share is `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`.

- [ ] **`Core\Arr` window depth** — `crates/mwl-stdlib/src/arr.rs:1599` `slice`, `:1704` `chunk`; spec
      § 2. One `arr-` case. `slice` is the same bound as `Core\Str`'s over a different subject, so the
      sweep in `tests/conformance/core/str-slice-and-replace-range-read-one-window.mwlt` is the shape to
      copy — including whether an array's window agrees with a string's at every sign. `chunk`'s own
      bound is a size of `0` against `1`, and a size at and past the subject's own count.
- [ ] **`Core\Arr::sort` depth** — `crates/mwl-stdlib/src/arr.rs:2765`; spec § 2. One `arr-` case, shaped
      as an invariant rather than a table: the answer is a permutation of the input (counted, as § 9's
      sweep counts), and equal keys keep their input order.
- [ ] **`Core\Arr::unique` depth** — `crates/mwl-stdlib/src/arr.rs:3462`; spec § 2 and ADR 0090 § 3. One
      `arr-` case reading that ADR's identity table one row per representation, exactly as § 9's
      collections already do — and `contains`/`diff`/`intersect` share the comparison, so they belong in
      the same case rather than a fourth.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder and ADR 0071's non-scalar fields — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — `mwl_stdlib::hash` module doc.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1's remainder.
- The differential corpus is 90 of 150 — `docs/plan/M4S.md`'s Stage 4 paragraph.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0090 § 3's string/array/object equality helpers are still owed — that ADR's own body.
