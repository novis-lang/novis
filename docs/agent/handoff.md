# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **866 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **10 unresolved of the 128 named test entries** `loop-goal.toml` holds, down
from 14 of 132. The `nvs-types (targets and refusals)` block is down to **1 of 7**: four of its five
names were cause 2, every refusal they name already landed in the checker with a `.nvst` case
pinning it, so the names moved into the `conformance` `cases` list and three cases joined it — the
nullsafe write target (`E0479`), the element write through a hooked property, and ADR 0066 § 3's
table closed at both ends (`E0709`/`E0708`), which one case owns for both of its names. Each was run
green before it was listed.

Two of the item texts predicted work that was already on disk — ADR 0066's pair was described as
"reaches lowering as panics today" and reaches nothing below `nvs-types` at all. The playbook bullet
about a stale `loop-goal.toml` comment already covers the lesson; the entries in
`guard-name-debt.md` now say what is actually true.

Both of the debt file's counts stay derived off the tree by the pass its header describes — the
denominator by parsing `loop-goal.toml` with `tomllib` (a regex over `tests = [...]` undercounts:
it reads 123 where the parser reads 128), the numerator by counting `- [ ]` lines.

## Next group

**The last `nvs-types` guard name in the Stage 4 block, then the two Stage 6 ones.** All three are
**cause 3 — genuinely unwritten** — so this group writes tests rather than reconciling names, and the
triage above is done: do not re-derive it. Shared file set: `crates/nvs-types/src/defaults.rs` +
`signatures.rs`, `crates/nvs-types/src/expr/calls.rs`, `crates/nvs-types/tests/`, plus
`docs/agent/guard-name-debt.md` and `docs/agent/loop-goal.toml` for the reconciliation.

- [ ] **`a_property_default_accepts_every_compile_time_constant`** — ADR 0046 § 2's constant set at a
      *property* default, not a parameter one: `crates/nvs-types/src/defaults.rs:1`'s module doc is
      about parameters, and the property path is `crate::defaults::eval_property_default` called from
      `crates/nvs-types/src/signatures.rs:753`. Check which constant forms it folds (a class
      constant, an enum case, `consts.rs:257`'s `fold_const`) before deciding whether this is a test
      to write or a fold to widen.
- [ ] **`an_implicit_constructor_is_held_to_zero_arguments`** — the refusal is
      `reject_arguments_to_implicit_constructor` at `crates/nvs-types/src/expr/calls.rs:359`, a
      private function nothing calls from a test and no `.nvst` case reaches. A `reject/` case
      building a no-constructor class and passing `new Foo(1)` is the cheapest home.
- [ ] **`a_non_void_function_must_return_on_every_path`** — the *refusal* half has no diagnostic at
      all (nothing in `nvs-types` or `nvs-diagnostics` names a body that falls off its end), so this
      one owes a new code — next free is `E0739`. The accepting half is already two listed cases.

## Backlog

- `an_array_conversion_walks_its_elements` — blocked on `array<T> as array<U>` lowering
  (`crates/nvs-ir/src/lower/expr.rs:877`); `docs/agent/guard-name-debt.md` § Stage 4.
- `nvs-ir`'s two owned-temporaries names, both cause 3 — `guard-name-debt.md` § Stage 3.
- `a_fatal_releases_the_frames_locals` (`nvs-codegen`) and `every_refusal_is_a_diagnostic_or_decided`
  (Stage 8's end gate, 17 standing refusal sites) — `guard-name-debt.md` §§ Stage 5, Stage 8.
- `nvs-syntax`'s two unparsed shapes: a grouped `use`, and an enum case named with a keyword.
- `python tools/gaps.py` ranks the thinnest classes once the guard debt is closed.
