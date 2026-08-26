# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 90 of 150** — and
the gap is behavioural depth per member, not coverage: every registered member already has a case,
and both of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy
and fmt clean), `mwl test tests/conformance` is **486 passed, 0 failed** and `mwl test tests/` is
**576 passed, 0 failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all
(playbook, twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is
newer than it (playbook, *Running things*). Every section has had its depth pass, so what matters
from here is the *shape* a new case takes rather than the section it lands in; the four shapes are
named in the plan's *Open now*, and the agreement shape — one question asked of every member that
shares it — now carries eight cases and is still the one with the most room. **A pass must land its
claim as a new case file** (playbook, *Writing a test case*).

Four spellings a case cannot use: a closure called through the variable holding it, the
first-class callable `Class::method(...)` (both `mwl-ir` gap 1 — declare a `class` with a `public
static function` and call it *directly*, which every `arr` depth case does), `bool as int`, and an
array literal written straight into an `array<array<mixed>>` element, which reads as `array<mixed>`
and then will not satisfy `array<array<T>>`. What a case *can* do: an option bag as a brace literal
argument, whose values may be *variables* (playbook, *Writing MWL itself*); `catch (RuntimeError
$e) { … $e->message … }` at file scope; `Core\Str::countOf` over an accumulated verdict string as a
tally; a `public` property on a fixture class read back after a `Core` member drove it, which is
how `from`'s `{limit}` is pinned; and `Core\Arr::count($a) as int` wherever a `uint` meets an `int`
(playbook has the `E0407`).

## Next group

All three share `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`, and each reuses one
of *Open now*'s four shapes. Take them in this order.

- [ ] **`flattenDeep` is the fixpoint of `flatten`** — `flatten` (`arr.rs:2084`), `flattenDeep`
      (`arr.rs:2125`), spec § 2: one question — "what does a nested array unwrap to" — asked of
      both, where the claim is not two answers but that iterating `flatten` until it stops changing
      *is* `flattenDeep`'s answer, over a table climbing from depth 0 to depth 4 plus a ragged
      subject mixing depths in one array. The two edges are a flat subject, where both members are
      the identity and the fixpoint is reached in zero steps, and the empty array. The agreement
      shape. `Core\Json::encode` renders each stage, since a case cannot index into an
      `array<mixed>`'s elements (playbook).
- [ ] **`groupBy` and `countBy` are one partition** — `groupBy` (`arr.rs:1139`), `countBy`
      (`arr.rs:2515`), spec § 2: the one question is "which bucket does this entry fall into", and
      the assertion is that `countBy`'s number for a key is `Core\Arr::count` of `groupBy`'s bucket
      for that key on every key of every table, that the buckets' counts sum to the subject's own
      count (so every entry landed in exactly one), and that both members list their keys in
      first-occurrence order. Sweep both with and without the `{by}` callback. The agreement shape.
- [ ] **`Core\Arr::column` over a row that does not have the cell** — `column` (`arr.rs:2182`, three
      arguments), spec § 2: the edges are a row missing the value cell, a row missing the *key* cell
      while having the value one, and the `$key` argument absent against present — which is where
      the result stops being a list. Read `column`'s own doc comment first; this slice's claim
      depends on which of those it drops and which it throws for. The edges shape.

## Backlog

- Differential is 90 of 150 and has had no session in a while; `tests/differential/` owns the shape.
- `Core\Json::decodeAs<T>`'s decoder is unbuilt — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row — that module's doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- ADR 0086 § 1's substitution table — `crates/mwl-stdlib/src/cli.rs` gap 1.
