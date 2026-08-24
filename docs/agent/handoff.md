# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**The loop now has a Stage 0, and it comes before `Core` breadth.** Eleven ADRs (0080–0090) were accepted
after the milestones that own their work were reported done, so
[loop-goal.md](loop-goal.md) § *Stage 0* is an ordered catch-up list and
[loop-goal.toml](loop-goal.toml)'s `stage = "0 catch-up"` block is its machine half — `tools/loop.py` runs
that block **before** the program legs, so an unfinished catch-up item is what the ledger names rather than
a Stage 3 fixture. Every test it lists must exist and pass; most do not exist yet, and writing one is how
an item finishes. Do not open a Stage 3 slice while that section is non-empty.

**[ADR 0087](../adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md) is built** — the first of the
eleven to land. [`mwl_syntax::bidi`](../../crates/mwl-syntax/src/bidi.rs) is the one predicate, the lexer
reports `E0008` with no suppression over comments, string literals and inline-HTML runs **per line**, and
seven `.mwlt` cases pin it including the paper's two attack patterns and a balanced-Arabic round trip. Its
`Core\Html::escape` (M7) and `Core\Cli` (M8) sink halves are those milestones' and are not catch-up.

**[ADR 0090](../adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md) is the largest unbuilt
item and reaches three milestones at once.** `==` is the only equality operator; `===`/`!==` do not parse
(M1), two statically disjoint operand types are a compile error (M2), and the null tag test plus the
string, array and object rows want a runtime helper each (M3/M4, `mwl-ir`'s gap 19). 45 files write the
rejected spelling in 112 places, which is why it goes first — every case written meanwhile adds to it.

**[ADR 0080](../adr/0080-the-audience-mwl-is-built-for.md) still reorders everything after Stage 0**: MWL
is built for multi-tenant and regulated platforms, which ranks the framework and the dependency story above
new `Core` breadth, makes the pitch isolation and qualifiers rather than speed, and forbids any document
claiming PHP compatibility. **[0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)** and
**[0089](../adr/0089-convert-is-one-rule-table-with-two-modes.md)** are decided and unbuilt but belong to
M4S and M11; **0081–0086** belong to milestones that have not started.

## Next

**ADR 0090 § 1 — delete the two spellings** (Stage 0 item 1, M1). The lexer stops producing `===`/`!==`
with a diagnostic in the E00xx band naming `==`/`!=`, the shape ADR 0034/0045 already use;
`BinaryOp::Identical`/`NotIdentical` come out of the AST with them; and the 45 `.mwl`/`.mwlt` files that
write the rejected spelling are rewritten in the same commit, because the Stage 1–3 fixtures are among
them. Name the guard test `a_rejected_equality_spelling_is_a_compile_error`, which
[loop-goal.toml](loop-goal.toml) already requires. `corpus_parse.rs` needs nothing — it holds "the parser
does not panic", not "php-src parses cleanly".

## Backlog

- **The rest of Stage 0, in [loop-goal.md](loop-goal.md)'s order** — ADR 0090 § 2's disjoint-operand
  refusal and § 3's narrowing plus three helpers, ADR 0047 § 4's atoms, `private`/`protected`,
  `Comparable`/`Stringable`'s member signatures, ADR 0061's `autoload`, ADR 0069's `array + array`.
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
  the machinery exists, so each is a registry row and a body. Same for the rest of § 1 and ADR 0069's
  combination members.
