# Handoff

## State

Goal `lang:expressions` is two features in. `lang:expressions/and-the-ternary` and
`lang:expressions/arithmetic` each carry all five artefacts, and `python tools/dossier.py --id`
prints `complete.` for both. Thirteen of the chapter's features still owe theirs —
`python tools/dossier.py --owed --group 'lang:expressions'` is the list.

One finding is recorded rather than fixed. `??=` whose target is an array key that is not there
throws `undefined array key` instead of writing the default, which is `crates/nvs-ir/src/lib.rs`
gap 20; every other `??=` target is already right. The proof that carries it is
`docs/examples/lang/expressions/and-the-ternary/03-settings-with-their-gaps-filled-in.nvs`, which
is red on purpose and whose `.out` holds what the program should print.

`docs/agent/loop-goal.toml`'s `[context]` gained `docs/examples/README.md` and three rules this
session had to slice by hand, so the next one is handed them.

## Next group

One slice is one feature with all five proofs. The list runs in the chapter's own file order, so
neighbours share the section they are read from — one file set:
`docs/reference/lang/30-expressions.md` plus the four proof trees under
`docs/examples/lang/expressions/`, `tests/hostile/lang/expressions/`,
`benches/members/lang/expressions/` and `tests/conformance/`.

- [ ] **`lang:expressions/arrays-in-expressions`** — owes examples, hostile, perf, tests.
      `rule:testing/four-proofs`. `docs/reference/lang/30-expressions.md:570`
- [ ] **`lang:expressions/object-literals`** — owes examples, hostile, perf, tests.
      `rule:testing/four-proofs`. `docs/reference/lang/30-expressions.md:637`
- [ ] **`lang:expressions/refused-in-expression-position`** — owes examples, hostile, perf, tests.
      `rule:testing/four-proofs`. `docs/reference/lang/30-expressions.md:662`

A file under `tests/hostile/` is in the formatter's corpus and a file under `docs/examples/` is
not, so `rule:tooling/fmt-quotes` applies to the attack alone: write its unescaped strings in
single quotes, or `nvs fmt` the file before `verify.py` reaches the identity test.

## Backlog
- `docs/reference/tools/30-php-differences.md:120` lists `??=` among the operators that work as
  PHP's do, which is the target rather than today's behaviour — `crates/nvs-ir/src/lib.rs` gap 20
  is the work that makes the line true, and the line is left alone until it lands.
- Neither feature is pinned from Rust; both cases are `.nvst`. `rule:testing/four-proofs`'s data
  does not ask a `lang` feature for one, so this is a note and not a debt.
- The eleven `lang:expressions` features behind the next group — `python tools/dossier.py --owed
  --group 'lang:expressions'` ranks them, and `docs/agent/loop-goal.md` § *Standing decisions* is
  what a session may settle alone.
