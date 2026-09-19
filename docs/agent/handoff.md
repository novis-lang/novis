# Handoff

## State

Goal `lang:expressions` is five features in. `and-the-ternary`, `arithmetic`,
`arrays-in-expressions`, `object-literals` and `refused-in-expression-position` each carry all five
artefacts. Ten of the chapter's features still owe theirs — `python tools/dossier.py --owed --group
'lang:expressions'` is the list.

`refused-in-expression-position`'s attack found a compiler crash, and it is fixed rather than
recorded. `self::`, `static::` and `parent::` written at the top level of a file passed `nvs check`
with no errors at all and then panicked `nvs-ir`, at each of the four sites that resolve a class
side: a class constant, a static property, a static call and `new`. `E0303` is reported there now,
by `crates/nvs-types/src/expr/mod.rs`'s `reject_class_side_outside_class`, whose doc comment owns
why the report sits at those four call sites and not in `check_expr`'s own arm.

One reference sentence was wrong and is fixed: `docs/reference/lang/30-expressions.md:663` listed
backticks among the constructs "parsed only so the diagnostic can name the replacement". A bare
backtick is `E0001` from the lexer — there is no shell-execution form for a diagnostic to name a
replacement for (`rule:core-classes/process-is-argv-only`) and the character itself is
``html`…` ``'s delimiter (`rule:core-classes/html-literal`).

That chapter edit staled the perf figure of every feature measured against the file, as the last one
did. `--record-perf` re-measured all five and every deterministic count came back identical.

## Next group

One slice is one feature with all five proofs. The two below are the chapter's remaining parsing
features, and they share one file set with the three landed ones:
`docs/reference/lang/30-expressions.md` plus the four proof trees under
`docs/examples/lang/expressions/`, `tests/hostile/lang/expressions/`,
`benches/members/lang/expressions/` and `tests/conformance/`.

**Stage 2: the dossier** — one file set, named above.

- [ ] **`lang:expressions/operators-php-has-that-do-not-parse`** — owes all five. Every entry is a
      parse error, so the attack declares `// hostile: expect-refusal` and the three examples show
      the replacements running. A `[skip]` on `perf` is likely right for the same reason
      `refused-in-expression-position` took one, and
      `tools/data/dossier-policy.toml` is where it goes with its reason.
      `rule:testing/four-proofs`. `docs/reference/lang/30-expressions.md:61`
- [ ] **`lang:expressions/precedence-and-associativity`** — owes all five. This one does run, so it
      owes a bench: chain the operands so no optimiser can hoist the loop, and measure a written
      expression whose shape the table decides. `rule:testing/four-proofs`.
      `docs/reference/lang/30-expressions.md:9`

## Backlog

- `var $x =& $y` reports `E0102` plus `E0706` (a bitwise-AND type error) rather than `E0701`; the
  declared spelling `$x =& $y` is refused correctly — `crates/nvs-types/src/expr/assign.rs:526`.
- Eight features of this chapter still owe every artefact —
  `python tools/dossier.py --owed --group 'lang:expressions'`.
- Any edit to `docs/reference/lang/30-expressions.md` stales every landed feature's perf figure in
  this chapter; budget a `--record-perf` run with it.
