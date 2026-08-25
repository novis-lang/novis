# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**Stage 0 item 1 is done: `===`/`!==` no longer exist.** The lexer consumes either spelling whole, reports
**`E0232`** (`E_IDENTITY_OPERATOR_UNSUPPORTED`) naming the two-character replacement, and pushes
`EqualsEquals`/`BangEquals` so one file still reports every one of its own problems in one run.
`TokenKind::EqualsEqualsEquals`/`BangEqualsEquals` and `BinaryOp::Identical`/`NotIdentical` are deleted, so
`mwl_types::locals::null_test` and `mwl-ir`'s `lower_null_identity` both read `Eq`/`NotEq` now — that half
of Stage 0 item 3 moved here because deleting the variants forced it. 48 files and 93 lines of corpus were
rewritten; a `--ORACLE--` body is PHP and was left alone.

**The band choice is a deviation from what loop-goal.md predicted, made deliberately.** That file said
E00xx because the lexer emits it; the code is E02xx because that band *is* "rejected PHP constructs", every
sibling ADR (0034/0045/0049/0050) lives there, and `E_RESERVED_SPELLING_CASE` is already a lexer-emitted
member of it. The band comment in `mwl-diagnostics` now names the two lexical members explicitly.

Four `.mwlt` cases were renamed from `identity-*` to `equality-*`, and
`tests/conformance/lang/equality-over-strings-and-bools.mwlt` gained ADR 0090 § 3's divergent string row
(`"1" == "01"` and `"1e3" == "1000"` are both false) — its old `==` vs `===` contrast became a duplicate.

`python tools/verify.py` is green (1303 tests), and `mwl test tests/` is 418 passed / 0 failed with the
PHP oracle available. `examples/errors.mwl`, the one Stage 1 floor fixture the rewrite touched, still
prints its frozen output exactly.

## Next group — the equality pass (Stage 0 items 2, 8, 3)

**Shared file set:** `crates/mwl-types/src/expr/operators.rs` (items 2 and 8 are two arms of it),
`crates/mwl-diagnostics/src/lib.rs`, then `crates/mwl-runtime/` + `crates/mwl-ir/src/lower/` for item 3.
One ADR, one semantic surface, and six of `loop-goal.toml`'s named Stage 0 tests. Do them in this order —
item 2's table is what decides which operand pairs can still reach item 3's helpers at all.

- [ ] **Item 2 — ADR 0090 § 2: two statically disjoint operands do not compile** (M2). A new E04xx code
      (`brief.py` prints the next free one) over
      [that ADR](../adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md) § 2's table:
      `string` against `int`, `string` against `bytes`, an enum against its underlying integer, two
      unrelated classes, and a non-nullable type against `null`. Its § 6 makes a `switch` label and a
      `match` arm the same check against the subject. The site is the equality arm, which returns `bool`
      for every operand pair today. Tests: `a_disjoint_equality_does_not_compile`, plus
      `an_equality_null_test_narrows` — the behaviour that one pins already exists, only the test is owed.
- [ ] **Item 8 — ADR 0069: `array + array` does not compile** (M4's *Verify* list). `arithmetic_result`
      at [operators.rs:191](../../crates/mwl-types/src/expr/operators.rs#L191) falls through to `mixed`
      with no diagnostic for two array operands, and `$a += $b` with it. Same file as item 2, one arm
      over. Test: `two_arrays_do_not_combine_with_plus`.
- [ ] **Item 3 — ADR 0090 § 3's three non-scalar rows** (M3/M4). The narrowing half landed with item 1;
      what is left is one runtime helper each for strings (text, never numeric), arrays (ordered,
      element-wise, recursive) and objects — `mwl_runtime::identity::value_identical` already is the
      object comparison — with § 5's `mixed` pairing answering `false` and never throwing, plus the IR
      lowering that reaches them. Tests: `equal_strings_compare_as_text_and_never_as_numbers`,
      `equal_arrays_compare_ordered_and_element_wise`, `equal_objects_compare_by_identity`.

**The next group after this one** is Stage 0 items 4, 5 and 6 — ADR 0047 § 4's literal and enum-case type
atoms, `private`/`protected` enforcement, and `Comparable`/`Stringable`'s member signatures. All three are
`mwl-types` name-and-member resolution keyed on the accessing class, so they share their file set the same
way. Item 7 (`autoload`) is `mwl-syntax` + `mwl-hir` and shares nothing with either — it gets its own.

## Backlog

- **The rest of Stage 0, in [loop-goal.md](loop-goal.md)'s order** — items 4, 5, 6 as one group and
  item 7 (`autoload`) as its own, both named under *Next group* above.
- **`Core\Path` — spec § 11** — the cheapest slice inside `examples/collect.mwl` and the first thing after
  Stage 0: `join` (variadic, which exists), `basename({withoutExtension})`, `extension(): ?string`,
  `SEPARATOR`, no new dependency. loop-goal.md § *Standing decisions* has the two-legs rule for `SEPARATOR`.
- **`Core\Encoding`, `Hash`, `Uuid`, `Csv`, `Validate`, `Random`, `Out`, `Uri::parseQuery`** — the rest of
  that fixture; each needs a dependency picked under [ADR 0051](../adr/0051-standard-library-tiers.md) § 4.
- **`Core\ObjectSet`/`ObjectMap`** — spec § 8, and the one item needing *language* work first:
  `new Core\X<T>()` does not parse. `mwl_runtime::identity` is the comparison they need, and is also what
  ADR 0090 § 3's object row lowers to.
- **The registry's qualifier classification** — [0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
  § 2: a per-parameter field on `mwl-stdlib`'s member rows, the fail-closed default, and the test that
  refuses an unclassified member. Lands with M4S's remaining sections.
- **`Core\Time\Date`/`TimeOfDay`/`Month`, `DateTime::date`/`timeOfDay`/`withTime`** — `time.rs`'s gap 1;
  the machinery exists, so each is a registry row and a body.
