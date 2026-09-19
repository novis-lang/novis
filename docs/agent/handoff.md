# Handoff

## State

Goal `lang:types`: every feature in `docs/reference/lang/20-types.md` owes the five artefacts of
`rule:testing/four-proofs`, and the chapter is the whole file set — 18 features, one section each.

Thirteen are complete: the eleven that were, plus `the-conversion-operator-as` and `narrowing`. Five
still owe; `python tools/dossier.py --owed --group lang:types` is the list. Nothing is blocked.

`narrowing`'s third example found a bug and it is **fixed, not recorded**: no union-typed default
took any literal at all — `?string $label = "plain"` was `E0472` and `public const "open"|"closed"
SHUT = "closed"` folded to no value — because `crates/nvs-types/src/defaults.rs`'s grid asks about
one atom and nothing resolved a union to the member the literal inhabits. The chapter's clause
saying a `?T` property takes no `null` default went with it: the binary has always accepted that.
`tests/conformance/lang/a-default-and-a-class-constant-place-a-literal-against-the-union-member-it-inhabits.nvst`
pins all of it and is attributed to `lang:types/properties-and-constants`, which is therefore at 1
of its 2 cases already.

`python tools/verify.py` is 13 of 13 green, at conformance 2135. `crates/nvs-host/src/watchdog.rs:926`'s
`the_watchdog_reports_a_wedged_worker_without_a_heartbeat` failed once under load and passed both
alone and in the second full run, so it is a heartbeat timeout that load breaks rather than anything
this session touched.

Still open from before: `docs/examples/lang/types/widening-without-as/03-a-price-list-that-mixes-both.nvs`
is marked `dossier: known-gap` for item 19 of `crates/nvs-ir/src/lib.rs` § *Known gaps*.

## Next group

**Stage 2: the dossier** — one file set: `docs/reference/lang/20-types.md` and the four proof trees
under `docs/examples/lang/types/`, `tests/hostile/lang/types/`, `benches/members/lang/types/` and
`tests/conformance/`. One slice is one feature with all five artefacts. `docs/agent/loop-goal.toml`'s
`[context] rules` and its copy at `docs/agent/goals/dossier/80-lang-types.toml` already carry these
three sections' rules; keep swapping them per group rather than naming the chapter's 47 `types/`
fragments.

- [ ] **`lang:types/truthiness`** — owes all five. `rule:expressions/truthy-positions` names the six
      positions and `rule:expressions/truthy-table` the values; `rule:enums/truthiness` is the row a
      ported `if ($status)` turns on. A count over the six positions is the case shape.
      `docs/reference/lang/20-types.md:735`
- [ ] **`lang:types/parameters`** — owes all five. `rule:statements/inout-is-the-by-reference-spelling`,
      `rule:statements/inout-is-written-at-the-call` and `rule:core-api/parameters-are-callable-by-name`
      specify it; a variadic tail and a by-name call are the two a reader gets wrong.
      `docs/reference/lang/20-types.md:760`
- [ ] **`lang:types/properties-and-constants`** — owes four, one case landed above.
      `rule:classes/no-free-functions-or-constants` and `rule:types/class-constant` specify the
      constant half, `rule:classes/definite-property-initialization` the property half.
      `docs/reference/lang/20-types.md:799`

## Backlog

- A `decimal` and a non-empty array literal are still `E0472` at a property default, and neither has
  a `ConstArg` — `crates/nvs-types/src/defaults.rs` § the grid owns whether that is worth closing.
- The `nvs-host` watchdog test above is load-sensitive — one red run in two; nothing tracks it yet.
- `lang:types/qualifiers-tainted-and-secret` and `lang:types/what-does-not-exist` are the group's
  last two after the three above; `python tools/dossier.py --owed --group lang:types` is the list.
