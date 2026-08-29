# Handoff

## State

**The failing acceptance check is closed.** `tests/conformance/core/program-implementing-enumerates-every-implementor.nvst`
is written and passes; it pins ADR 0061 § 3's *membership* — the four routes a class takes to the
interface (direct, through a class, through an abstract intermediate, through an interface that
extends it) — where the companion case pins the order and the expansion.

**Item 11's argument type-guard family is finished outside `arr.rs`.** Eleven guards now carry a
`unreachable from source` declaration and their twelve `OWED_A_CASE` lines are gone: `bytes.rs`,
`json.rs` (×2), `path.rs`, `debug.rs`, `str.rs` (×2), `test.rs` (×4) and `csv.rs`. Each was probed
with `nvs check`/`nvs run` before the comment was written, never assumed from the row.

**Two of those declarations are a different judgement from the eight `E0401` ones, and say so:**
`Core\Path::join`'s and `Core\Debug::dump`'s tails are `CoreTy::Variadic`, and
`nvs_ir::lower::lower_call_args` *builds* the array that fills the slot — no source expression
reaches it at all, well-typed or not. Reach for that wording for `Core\Str::format`'s argument-list
guard, which is the same shape and still on the ratchet.

**`csv.rs:521` did not owe a case after all** — the previous handoff predicted one. Both callers of
`write_record` are `array<string>`-typed, and the one route from untyped data
(`Core\Json::decode(…) as array<string>`) refuses per *element* at the conversion, at one level and
at two. The member's own `# Errors` doc claimed "`array<mixed>` reaches this member through `mixed`";
that sentence was stale and is now corrected.

**`Core\Router::urlAbsolute` gained its second case** —
`router-url-absolute-refuses-a-unit-with-no-configured-origin.nvst`, the pair asserted on both sides
over one name. It stays in `BELOW_THE_FLOOR` (`conformance_coverage.rs:264`): the floor is three and
it is at two.

**Untouched:** item 12's classification (`UNCLASSIFIED`, `crates/nvs-stdlib/src/registry.rs:1509`,
still 118).

**Found, not fixed:** `catch (Core\Error $e)` panics in `nvs-ir` instead of diagnosing — the
playbook bullet under *Writing a test case* has the reproducer.

**Orientation gap, thirteenth session running:** `[context] adrs` still does not carry `0085 §§ 1-4`,
and nothing in the pack names the stage-5 `[[check]]` blocks' `cases`/`tests` lists — so an
acceptance failure naming a `.nvst` still has to be triaged by `grep`ping `loop-goal.toml`, which is
that list's only home.

## Next group

**Item 11's two biggest remaining modules, then item 12's next class. Shared file set:**
`crates/nvs-stdlib/tests/conformance_coverage.rs:604` — `OWED_A_CASE` is the worklist and a slice is
done when its lines are gone — plus one `crates/nvs-stdlib/src/<class>.rs` per slice.

- [ ] **`str.rs`'s six remaining stems, goal § item 11.** Two families in one module, and the row
      decides which: `fromCodePoints` (`str.rs:2392`) is the `E0401` shape, `format`
      (`str.rs:2446`) is the `CoreTy::Variadic` shape above, `normalize` (`str.rs:2302`) is an enum
      case and needs its row checked. The other three are internal invariants that may owe a case:
      `str.rs:789`, `:1349`, `:1529`.
- [ ] **`time.rs`'s five stems, goal § item 11.** `time.rs:1480`, `:1488` and `:1493` are one
      `Core\Time\Instant` slot family and share a declaration; `:1687` and `:2521` are `uint`
      parameters and are the `E0401` shape; `:2707` and `:2907` are `could not place` formatter
      invariants, which are the ones most likely to owe a case.
- [ ] **`Core\Validate` (6) and `Core\Uuid` (2), all `Qual::Neutral`, goal § item 12.** The rows are
      `crates/nvs-stdlib/src/validate.rs:159` and `crates/nvs-stdlib/src/uuid.rs:117`; delete the
      eight lines they own from `UNCLASSIFIED` (`registry.rs:1509`), which the gate at `:1652`
      requires in the same commit.

## Backlog

- `catch (Core\Error $e)` ICEs in `nvs_ir::lower::expr` — a registry class in a `catch` clause needs
  a diagnostic. Owner: `crates/nvs-hir/src/errors.rs`' module doc names the rule it violates.
- `Core\Router::urlAbsolute` is one case short of the floor of three
  (`conformance_coverage.rs:264`).
- `Core\Math::atan2`, `Core\Regex::quote`, `Core\Attributes::all`, `Core\Program::implementing` and
  `Core\Time\TimeOfDay::compareTo` are the rest of `BELOW_THE_FLOOR`.
- `python tools/gaps.py --coverage` ranks item 10's remaining thin classes; `Core\Router` is
  thinnest.
