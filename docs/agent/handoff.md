# Handoff

## State

**Stage 4's two counts are the frontier — conformance 484 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **484 passed, 0 failed** and `mwl test tests/` is **574 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook, twice),
and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook,
*Running things*). Every section has had its depth pass, so what matters from here is the *shape* a new
case takes rather than the section it lands in; the four shapes are named in the plan's *Open now*, and
the agreement shape — one question asked of every member that shares it — now carries seven cases and is
still the one with the most room. **A pass must land its claim as a new case file** (playbook, *Writing
a test case*).

Four spellings a case cannot use: a closure called through the variable holding it, the first-class
callable `Class::method(...)` (both `mwl-ir` gap 1 — declare a `class` with a `public static function`
and call it *directly*, which every `arr` depth case does), `bool as int`, and an array literal written
straight into an `array<array<mixed>>` element, which reads as `array<mixed>` and then will not satisfy
`array<array<T>>` (playbook, *Writing MWL itself*). What a case *can* do: `{preserveKeys: true}` and
other option bags as a brace literal argument; `catch (RuntimeError $e) { … $e->message … }` at file
scope, which is how both of this session's refusals are pinned; `Core\Str::countOf` over an accumulated
verdict string as a tally; and `Core\Arr::count($a) as int` wherever a `uint` meets an `int` (playbook
has the `E0407`).

## Next group

All three share `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`, and each reuses one of
*Open now*'s four shapes. Take them in this order.

- [ ] **The array constructors agree about length and keys** — `fill` (`arr.rs:2277`), `fillKeys`
      (`arr.rs:2301`), `fromKeysAndValues` (`arr.rs:2454`), `padStart` (`arr.rs:1938`), `padEnd`
      (`arr.rs:1955`), spec § 2 and ADR 0069 § 3: every one of these builds an array whose *count* is
      stated by an argument rather than by a subject, so the one question is "how many entries, under
      which keys" — asked of all five, with the count argument at zero and at one, and the key shape
      each of them invents (a list from `fill` and both pads, the argument's own keys from `fillKeys`
      and `fromKeysAndValues`) collapsed to one tally. The agreement shape.
- [ ] **`Core\Arr::from`'s `{limit}` is a bound on both sides** — `from` (`arr.rs:2419`), spec § 2 and
      ADR 0053 § 1: a limit of `0`, one at the sequence's own length and one past it, over an array and
      over a generator — where the claim is that the *drive* stops rather than the answer being
      truncated afterwards, which only the generator side can show (count the segments its body runs).
      An omitted limit does not stop at all, and the answer is always a list whatever it drained.
- [ ] **`flatten` is one level and `flattenDeep` is the fixpoint** — `flatten` (`arr.rs:2084`),
      `flattenDeep` (`arr.rs:2125`), ADR 0069 § 3: over subjects nested one, two and three deep,
      `flattenDeep` is `flatten` applied until nothing nests, every level's keys are discarded at every
      level, and both answer a list from any subject. Invariance over a sweep.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2, and ADR 0071's non-scalar fields.
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — `mwl_stdlib::hash`'s
  module doc.
- ADR 0086 § 1's substitution table for the terminal sink — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `orient.py` still has no `[context]` selector for `docs/spec/01-core-library.md`, which every `arr`
  slice is specified by, and `[context] adrs` still wants `0007:5` and `0069:3`; this session worked off
  the members' own doc comments again, which was enough but is not the spec.
