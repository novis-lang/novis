# Handoff

## State

**The failing acceptance check is closed.** `nvs-stdlib (Reflect, Ast, Decimal) [8 introspection]`
named `a_reflective_property_write_runs_the_hook_an_ordinary_write_runs`, which did not exist because
the member it is about did not: ADR 0019 § 2 gives reflection *"reading or writing a reflected
property"* and only the read had landed.

**`Core\Reflect\ClassInfo::set` is on disk**, `set(mixed $object, string $name, mixed $value): void`
after `get` in row order. It writes `get`'s four refusals in `get`'s order and nothing else: the
store, the check of the incoming value against what the concrete class declares the property to hold,
and ADR 0014 § 3's observer step all belong to **`nvs_runtime::write_erased_property`**
(`crates/nvs-runtime/src/object.rs:2404`), which is `nvs_object_slot_set`'s former body factored out.
That is `ClassInfo::call`'s decision — § 2's *"fails the same way an ordinary write would"* read as
*the same code* — applied to the other direction, and both those doc comments own the reasoning.

**It closed a live divergence rather than just adding a member.** The observer step had no home for an
erased write: ADR 0014 § 4 answers conformance from the *declaration*, so a known-class write emits
`onPropertySet` beside its `FieldSet` and an erased one emitted nothing. The playbook bullet added
this session is the general shape of that.

**The read half is still divergent and is deliberately not in this slice**: `nvs_object_slot_get` and
`Core\Reflect\ClassInfo::get` run no `onPropertyGet`. Backlogged with anchors — it is the same twenty
lines in the same two functions, and it changes a hot path, so it wants its own verify. The second
standing gap is a *hooked* property: an erased write reaches storage, not the hook a known class's
write lowers to, so reflection writes past ADR 0014 § 1 exactly as the ordinary erased write does.
Both are gap 3 of `crates/nvs-stdlib/src/reflect.rs`'s module doc, which is their home.

`Core\Cli::displayWidth` was this session's item and is untouched; the acceptance failure outranked
it. `orient.py` printed no section of ADR 0019 or ADR 0014 although the item was entirely inside
them: `[context] adrs` needs `0019:1`, `0019:2` and `0014:3`, and still needs `0086:1`, `0086:3` and
`0033:4`; `[context] spec` still misses `docs/spec/01-core-library.md` § 15.

## Next group

**`Core\Cli`'s last member and the case that pins the other half, over `crates/nvs-stdlib/src/cli.rs`
and `tests/conformance/core/`** — unchanged from the last handoff, because this session spent itself
on the acceptance check instead.

- [ ] **`Core\Cli::displayWidth`** — ADR 0086 § 3's `displayWidth(string $value): uint`, UAX #11
      columns rather than `Core\Str::length`'s graphemes. The five edits of a `Core` member, at
      `crates/nvs-stdlib/src/cli.rs:213` (the rows — it goes after `colorDepth`, spec order),
      `crates/nvs-stdlib/src/cli.rs:362` (the cards, in row order beside `WIDTH_DOC`),
      `crates/nvs-stdlib/src/cli.rs:1020` (the `address()` arm) and
      `crates/nvs-stdlib/src/cli.rs:1286` (`nvs_core_cli_width`, the body to write beside). The
      decision to make first is where the width table comes from: a dependency is pre-authorized
      under ADR 0051 § 4 and owes the `[workspace.dependencies]` comment, `cargo deny check` and
      `python tools/gen-attribution.py`.
- [ ] **A `.nvst` case for `E0797`** — `nvs_types::expr::quals::reject_secret_logged_argument`
      (`crates/nvs-types/src/expr/quals.rs:658`) refuses a `secret` at `Core\Log::write`'s `fields`
      and has only a `-p nvs-types` test. The case is `--EXPECTF-ERROR--` and has to reproduce the
      diagnostic's own indentation; `tests/conformance/reject/` is where its siblings live.

## Backlog

- ADR 0014 § 3's **read** half over an erased receiver — `crates/nvs-runtime/src/object.rs:2262`
  (`nvs_object_slot_get`) and `Core\Reflect\ClassInfo::get`; ADR 0014 § 3 owns the rule.
- Stage 7 owes reading `[log] target` — `docs/agent/loop-goal.toml`'s stage 7 block.
- `Core\IO::truncate` and `lock` — the plan's *Open now*, stage 2's handle half.
- `Core\Cli`'s gap 1 is `displayWidth`'s UAX #11 table — `crates/nvs-stdlib/src/cli.rs`'s module doc.
