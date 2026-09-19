# Handoff

## State

Goal `lang:expressions` is eight features in: `and-the-ternary`, `arithmetic`, `arrays-in-expressions`,
`object-literals`, `refused-in-expression-position`, `the-pipeline-operator`,
`operators-php-has-that-do-not-parse` and `precedence-and-associativity` each carry all five
artefacts. Eight of the chapter's sixteen features still owe theirs — `python tools/dossier.py
--owed --group 'lang:expressions'` is the list.

Three rows of the table `operators-php-has-that-do-not-parse` is written around disagreed with the
binary, and the binary decided each. `<>` is `E0241` at the two characters, where the chapter said
it was accepted as a second spelling of `!=`. `|>` is Novis's own pipeline operator, where the
chapter said the token does not parse, so that row is gone and a sentence points at *The pipeline
operator*. And `array<string, T>`, which the `$$name` row named as the replacement, is not a type:
`array<T>` takes one argument and its keys are already `string`.

That last one was also in the help of `E0202` and of `E0235`, so both told a reader to write
something that does not compile; both now name `array<T>`, and the two rows of
`docs/reference/tools/30-php-differences.md` that copied it follow.

Editing the chapter staled the perf figure of every feature measured against it, as it does every
time. `--record-perf` re-measured the five and every deterministic count came back identical.
`python tools/verify.py` is 13 of 13 green, conformance at 2159.

## Next group

One slice is one feature with all five proofs. The three below are the chapter's remaining
operator-and-truth features, and they share the file set the eight landed ones used:
`docs/reference/lang/30-expressions.md` plus the four proof trees under
`docs/examples/lang/expressions/`, `tests/hostile/lang/expressions/`,
`benches/members/lang/expressions/` and `tests/conformance/`.

**Stage 2: the dossier** — one file set, named above.

- [ ] **`lang:expressions/comparison-and-equality`** — owes all five. `==` never converts, a disjoint
      pair is refused where it is written, and `<=>` answers three values, so the two cases split
      into one sweep over the type table and one reject case over the pairs that have no comparison.
      `rule:expressions/equality-semantics`, `rule:expressions/disjoint-comparison-refused`.
      `docs/reference/lang/30-expressions.md:194`
- [ ] **`lang:expressions/logical-operators-and-truth`** — owes all five. The truthy table is PHP's
      over Novis's type set, and `&&`/`||` run only the side they end up using, which
      `tests/conformance/lang/every-short-operator-runs-only-the-side-it-ends-up-using.nvst` already
      pins from one side. `rule:expressions/truthy-table`,
      `rule:expressions/no-keyword-logical-operators`. `docs/reference/lang/30-expressions.md:262`
- [ ] **`lang:expressions/string-operators`** — owes all five. `.` joins, `.=` appends, and every
      operand that has no string form is refused at the join, so the bench is the one place in this
      group where an allocation count is the figure worth declaring.
      `docs/reference/lang/30-expressions.md:282`

## Backlog

- `lang:expressions/assignment` owes examples, a bench and its second case; `match`, `calls`,
  `closures` and `is-new-clone-throw-print-exit-isset-empty` owe all five —
  `python tools/dossier.py --owed --group 'lang:expressions'`.
- The precedence attack trips the parser's 96-level recursion limit, so it declares
  `expect-refusal` and nothing in it runs past the parse; a run-time attack on the same feature
  would need the nesting built out of values.
- `docs/decisions/0071.md` and `0126.md` name `array<string, T>` too. A record is frozen history and
  is not edited to track a later decision.
