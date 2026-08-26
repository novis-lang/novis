# Handoff

## State

**Spec §§ 1-12 is registered whole and Stage 3 is green.**
`crates/mwl-stdlib/tests/spec-members-outstanding.txt` holds no keys, which is this project's own
definition of Part I being registered, and all seven Stage 3 fixtures produce their frozen output.
`Core\Out::capture` was the last key: `crates/mwl-stdlib/src/out.rs` runs its closure with
`mwl_runtime::Ctx`'s capture stack in force (`ctx.rs`'s `begin_capture`/`end_capture`, closed on
the throwing edge too), always swallows, nests, and answers **`Core\Cli\Text`** — the carrier of
the sink in force per ADR 0088 § 5, not a `string`. The carrier's name and one-slot layout live in
`mwl_runtime` (`CARRIER_CLI_TEXT`, `CARRIER_TEXT_SLOT`) so `value_to_string` can render one
without naming `mwl-stdlib`; `crates/mwl-stdlib/src/cli.rs`'s module doc owns why it has no member
yet. `echo` of a `Core` instance used to **panic** `mwl-ir` and is now the same tag-dispatched
conversion a union takes. Verify is green: **1592** tests, 74 suites, clippy and fmt clean, and a
WSL `leak-check.sh` over both capture edges reports zero.

`examples/collect.mwl` also carried two bugs the parse error had masked — `Core\Uuid::isValid`,
which spec § 11 says does not exist, and an `Arr::first` result indexed through the
nullable-array hole — both fixed; see the playbook's two new bullets.

**The first red acceptance check is now Stage 0's `mwl-codegen (packed subscript)`**, which names
a test that does not exist. That is the next group.

## Next group

Item 19's unheld half plus its ledger entry, sharing `crates/mwl-codegen/tests/arrays.rs`,
`crates/mwl-runtime/src/counting_alloc.rs` and `docs/perf/`. Slices 1 and 2 are independent; 3 is
only worth taking if the first two leave you well short of the ceiling.

- [ ] **`an_integer_subscript_reaches_the_packed_form_from_compiled_code`** — `loop-goal.toml:419`
      names it and nothing in the tree defines it, so the check reports *did not run*. It is
      `mwl_runtime::array`'s `an_integer_subscript_allocates_no_key`
      (`crates/mwl-runtime/src/array.rs:1646`) asserted over **compiled** code instead of the
      primitive: an `$a[$i]` read and write through `mwl-codegen`, with
      `crates/mwl-runtime/src/counting_alloc.rs:58`'s byte counter showing no key allocated.
      The lowering it must exercise is `crates/mwl-ir/src/lower/expr.rs:1878`
      (`mwl_array_get_index`/`mwl_array_set_index`, nothing rendered). Put it in
      `crates/mwl-codegen/tests/arrays.rs`, whose neighbours already build and run a unit.
- [ ] **`docs/perf/history.ndjson`, append-only** — Stage 0 item 15 asked for it and item 19 still
      owes it; `docs/perf/userland-gap.md` is the prose ledger it complements, and
      `python tools/bench.py` is what produces a row. Decide the row's fields (suite case, ratio,
      commit, toolchain) and write the first one from the current 0.80× median.
- [ ] **Strike the two from `Open now`** once both are green, and re-point
      `docs/agent/loop-goal.toml`'s `[context]` at Stage 4: its `modules`/`adrs` selectors still
      name `Core\Out`'s files, which the next group does not touch.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2 owns it.
- ADR 0088's registry-wide qualifier classification on every `Core` member row, and the test that
  refuses an unclassified `string`/`bytes` parameter — `docs/plan/m4s.md`.
- ADR 0086 § 1's substitution table and the rest of `Core\Cli` — `crates/mwl-stdlib/src/cli.rs`'s
  gaps 1 and 2, `docs/plan/m8.md`.
- ADR 0090 § 3's string, array and object equality rows, one runtime helper each.
- `do`/`while` lowering — the one M4 control-flow statement that does not, `mwl-ir`'s own gaps.
- `docs/spec/02-php-migration.md` at 31% classified — `python tools/check-migration.py`.
