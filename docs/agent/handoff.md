# Handoff

## State

Goal `lang:expressions` is four features in. `and-the-ternary`, `arithmetic`,
`arrays-in-expressions` and `object-literals` each carry all five artefacts, and
`python tools/dossier.py --id` prints `complete.` for each. Eleven of the chapter's features still
owe theirs — `python tools/dossier.py --owed --group 'lang:expressions'` is the list.

One reference sentence was wrong and is fixed: `docs/reference/lang/30-expressions.md:571` said a
`[...$a]` spread copies `$a`'s entries "keys included", while `rule:types/arrays` and
`nvs_array_spread` both renumber an integer-looking key under the receiving literal's own counter.
The chapter's own example could not tell the two apart; the new `.nvst` case prints the keys.

Two findings are recorded rather than fixed. `??=` whose target is an absent array key throws
instead of writing the default (`crates/nvs-ir/src/lib.rs` gap 20), carried by
`docs/examples/lang/expressions/and-the-ternary/03-settings-with-their-gaps-filled-in.nvs`. A
self-referential `type` alias is refused by the type-depth bound with a message about arrays
(`crates/nvs-types/src/lower.rs`, `lower_type_at_depth` § *Known gaps*).

## Next group

One slice is one feature with all five proofs. The three below are the chapter's own refusal and
parsing features, which share what they have to explain — one file set:
`docs/reference/lang/30-expressions.md` plus the four proof trees under
`docs/examples/lang/expressions/`, `tests/hostile/lang/expressions/`,
`benches/members/lang/expressions/` and `tests/conformance/`.

- [ ] **`lang:expressions/refused-in-expression-position`** — owes examples, hostile, perf, tests.
      Every construct in it is a compile error, so the attack declares `// hostile: expect-refusal`
      and the three examples show the replacements running. `rule:testing/four-proofs`.
      `docs/reference/lang/30-expressions.md:661`
- [ ] **`lang:expressions/operators-php-has-that-do-not-parse`** — owes examples, hostile, perf,
      tests. Same shape as the one above. `rule:testing/four-proofs`.
      `docs/reference/lang/30-expressions.md:60`
- [ ] **`lang:expressions/precedence-and-associativity`** — owes examples, hostile, perf, tests.
      `rule:testing/four-proofs`. `docs/reference/lang/30-expressions.md:8`

## Backlog

- An array-literal round measures 363.5 ns/op over 7 allocations against 5.0 for an arithmetic one;
  `docs/perf/members.md` owns the figures.
- `??=` on an absent array key — `crates/nvs-ir/src/lib.rs` gap 20.
- A self-referential `type` alias has no diagnostic of its own —
  `crates/nvs-types/src/lower.rs` § *Known gaps*.
