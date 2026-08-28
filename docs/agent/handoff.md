# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **866 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **39 unresolved of 149 named guard tests**, down from 54 of 156. Fifteen were
reconciled over one file set — `docs/agent/guard-name-debt.md` and `docs/agent/loop-goal.toml` —
in three passes: the five confirmed cause-1 renames, all of Stage 3, and five of Stage 4's seven.
Seven of the fifteen were cause 2 (the work landed as a `.nvst` case), which is why the denominator
moved: a cause-2 fix deletes the `cargo-named` entry and names the case in the `nvs-suite` check
instead. Three cases were added to that list — `a-break-level-that-names-no-target-is-a-diagnostic`,
`an-operator-agrees-whether-its-operand-is-typed-or-tagged` and
`as-bytes-over-a-tagged-operand-is-decided-by-its-runtime-tag` — each run green first.

What is left in the two stages that were touched is **cause 3 and honest**: the two owned-temporaries
shapes (`an_inline_producer_releases_its_value_on_the_throw_path`,
`a_transferred_argument_is_released_when_a_later_one_throws`) and `an_array_conversion_walks_its_
elements`, which cannot be written until `array<T> as array<U>` stops panicking `nvs-ir`. Those stay
listed and unticked on purpose; the debt file now records the cause per line.

The debt file's own header sentence is the running count and is the thing to keep true.

## Next group

**Carry the same three-way triage through Stages 5 and 6**, which is 23 of the remaining 39. One
shared file set, unchanged from this session: `docs/agent/guard-name-debt.md` and
`docs/agent/loop-goal.toml`, with `grep -rn "fn <name>" crates/<crate>/src crates/<crate>/tests` and
`find tests/conformance -name "*<topic>*"` as the two probes, and `python tools/loop.py --list` as
the toml's parse check.

- [ ] **Stage 5's 13**, at `docs/agent/guard-name-debt.md:121-146` — four checks, one per crate
      (`nvs-types` 123, `nvs-ir` 131, `nvs-stdlib` 139, `nvs-codegen` 144). The `nvs-ir` five are
      observable behaviour (`inline_html_at_file_scope_lowers_to_an_echo`, the two `require` ones),
      so expect cause 2 and check `tests/conformance/` before reaching for a rename.
- [ ] **Stage 6's 13**, at `docs/agent/guard-name-debt.md:148-172` — `nvs-types (narrowing and
      reachability)` at 150 is 9 of 11 and two of its lines already carry a "likely" hint;
      `nvs-syntax (the last unparsed shapes)` at 167 is 4 of 4.
- [ ] **The one Stage 4 line still undecided**, `the_object_top_type_erases_to_the_pointer_a_class_
      does` at `docs/agent/guard-name-debt.md:104` — read
      `a_property_access_through_a_plain_object_receiver_reads_by_name`
      (`crates/nvs-ir/src/lower/tests.rs:963`) and its write twin at 1349 and decide rename or write.

## Backlog

- Stages 7 and 8's remaining 4, `docs/agent/guard-name-debt.md:174-194` — both are cause 2/3 already
  diagnosed in the file, and `every_refusal_is_a_diagnostic_or_decided` is red on its merits.
- `gaps.py`'s per-member count no longer tracks coverage at the top of its table — the playbook
  bullet from the previous session owns the check.
- ADR 0007 § 2's `array<T> as array<U>` row does not lower (`crates/nvs-ir/src/lower/expr.rs:877`);
  it blocks one guard test and a `Core\Csv::format` error path — `docs/agent/playbook.md` records
  both.
