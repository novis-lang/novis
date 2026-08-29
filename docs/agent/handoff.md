# Handoff

## State

**Stage 0b item 28 is on disk: every `Core` parameter's name is on its registry row.** `CoreMethod::names`
is one entry per `positional()` slot; the trailing bag is not on the row, because its one name is
`registry::OPTIONS_NAME` beside `CoreTy::Options`. All 345 rows carry theirs. Three guards hold it and
they are exactly the three names `loop-goal.toml` stage `0b named arguments` asks of `nvs-stdlib`.
`crates/nvs-stdlib/src/registry.rs`'s `CoreMethod::names` doc is the rule's home; the plan's `Open now`
holds where the three sources came from. **Nothing about any member's shape changed.**

**Items 29 and 30 are untouched and are the next group.** `crates/nvs-types/src/core_lib.rs:104` still
seeds `param_names: None` and its comment now says so, so a name at a `Core` call site is still `E0485`
where it is written.

**Two things the spec is authoritative for and does not write well**, both recorded here rather than
fixed, because the standing decision forbids a shape change and neither is one:

- `Core\Arr::map`, `filter`'s siblings and four others take a parameter the spec names `$fn`, which is
  the closure keyword. Item 29 has to decide whether `map(fn: $f)` parses; if it cannot, the fix is a
  spec rename (a breaking change under R2), not a registry one.
- § 4's prose writes `Date::at(int $y, uint $m, uint $d)` — one-letter names, taken verbatim, and the
  only row in the registry whose names read worse than the member does. `DateTime::at`'s own table row
  writes `year`/`month`/`day`. Nothing compares the two, because the guard reads table rows and this is
  prose.

**The acceptance check the driver reports failing is Stage 7's, unchanged and still an item rather than a
regression.** `crates/nvs-test` has no `tests/` directory at all, so none of the four names it asks for —
`each_test_runs_in_its_own_isolate_sharing_only_compiled_code` first — can exist until ADR 0079's
`#[Test]` surface is built. That is a stage of work.

**Orientation gaps, unchanged from the last session:** `[context] adrs` prints neither ADR 0006
§ *Decision* nor § *Failure is a value, not an exception* nor ADR 0116 § 4; `[context] shapes` prints the
`.nvst` shape without the `--FILE <path>--` auxiliary section; `[context] modules` still has no pattern
for `nvs-runtime/src/graph.rs`. New this session: `[context] modules` has no pattern for
`nvs-types/src/expr/args.rs` either, which items 29's file set needs.

## Next group

**Stage 0b items 29 and 30 — the resolution and its cases.** One file set:
`crates/nvs-types/src/{core_lib,signatures}.rs` and `expr/args.rs`, then `tests/conformance/`.

- [ ] **Item 29 — read the names.** `crates/nvs-types/src/core_lib.rs:104` builds `param_names: None`;
      make it `Some(names + OPTIONS_NAME where a bag is last)`. The reader already exists —
      `crates/nvs-types/src/signatures.rs:237` is `MethodSig::index_of_name`, and `signatures.rs:910` is
      how a *user* method fills the same field, which is the shape to copy. Every R2 rule
      (`docs/adr/0063-core-api-conventions.md` § 1, row R2) is already enforced against
      `param_names` for a user method, so this is a seeding change and not a checker change. The four
      tests are `loop-goal.toml` stage `0b named arguments`, `nvs-types` check — start with
      `a_core_member_is_called_by_the_names_the_spec_writes`. Watch the variadic tail: R2 says a name
      never reaches one, `Core\Str::format` is the only row with one, and
      `a_name_at_a_core_variadic_tail_is_unknown` is that assertion.
- [ ] **Item 30 — the three cases**, named by the `conformance (Core by name)` check:
      `tests/conformance/core/a-core-member-is-called-by-the-names-the-spec-writes.nvst`,
      `tests/conformance/error/a-throwable-is-constructed-by-name.nvst`,
      `tests/conformance/reject/a-misspelled-name-at-a-core-member-is-unknown.nvst`. The third needs
      `E0486`-or-whatever `index_of_name` returning `None` reports today; `crates/nvs-diagnostics/src/lib.rs:1027`
      is `E0485`, which is the *no names at all* refusal and stops applying the moment item 29 lands.
- [ ] **When all three Stage 0b checks are green**, delete the four Stage 0b modules and the `0063 §1`
      entry from `[context]` in `docs/agent/loop-goal.toml`, and copy that file over
      `docs/agent/goals/<goal>.toml` — the two are byte-identical by construction. Its own comments at
      lines 71, 92 and 161 say so.

## Backlog

- `$fn` as a callable-by-name parameter, and `Date::at`'s `$y`/`$m`/`$d` — both above, both
  `docs/spec/01-core-library.md`'s to settle, neither a registry change.
- Stage 7 is unbuilt: ADR 0079's `#[Test]` table, and `crates/nvs-test/tests/` does not exist.
- ADR 0023 § 2's unresolvable-class question is asked of the answer and not the argument;
  `crates/nvs-host/src/isolate.rs`'s module doc names the `nvs_runtime::script` seam change that closes it.
- `Core\Secret::reveal()` is still absent from the registry, so ADR 0033's escape hatch is open at both
  ends — the plan's `Open now` owns why.
- M4's 1000-case conformance floor, which orders 1–4 meet as the suite grows.
