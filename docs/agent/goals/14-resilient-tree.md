---
milestone: M4B
---
# Loop goal 14 — the resilient tree, and one home for a position

Finish the half of `rule:ide/one-grammar-one-tree` that
goal `doc-comments` did not build. That goal landed the trivia layer because a doc comment cannot be read without
it; what is still owed is the part only an editor needs — **an index from a byte offset to the
innermost node and its ancestors, and recovery a consumer can tell apart from what the user wrote.**
[docs/plan/m4b.md](../../plan/m4b.md) is the milestone and this file does not restate it.

This goal opens no server and writes no editor. It is the three things every later editor slice reads:
the tree, the position arithmetic, and the section lexer the `.lspt` format will share. Goals `lsp-server` and `editor`
are unwritable without it, which is why it is first and why its acceptance list is entirely properties
rather than features.

**One thing every session must hold:** `parse_file` keeps its exact behaviour. `nvs check` and
`nvs run` are the same parse followed by "refuse if anything was reported", and no call site changes.
A resilient tree that alters what the strict path reports is a regression wearing a feature's clothes,
and the test that says otherwise is named in the acceptance list.

## Why here

It is order 6 of the milestone table and it sits here because everything it reads is now built: goal
13 landed the trivia layer, goal `typed-callable` the typed `callable`, and the request set is written against a
language that can open a file and reach a database. The split is by file set, the way the parity
program's was — no manifest names both a parser module and a TypeScript tree. The tree half
`rule:ide/one-grammar-one-tree` s1 still owes: the `SyntaxIndex` and explicit recovery, plus the one
home for a position (`utf16_col`/`offset_of`), the section lexer `.lspt` will share, and `nvs ast
--json`.

## Stage 0 — the catch-up

**Nothing**, and that is a finding rather than a default. The four M4 language holes M4B was staged with
are closed: `Class::method(...)` and `$obj->method(...)` build an ordinary `callable`
(`tests/conformance/core/a-first-class-callable-lowers.nvst`), a closure is callable through the variable
holding it in the same case, `do`/`while` lowers, and `bool as string` renders `""` for `false`. The
fourth item's other half is **not** a hole and must not be reopened: `bool as int` is refused by
`rule:types/conversion` — `E0708`, whose help line names `$b ? 1 : 0` —
which is a decision, not a gap.

## Stage 1 — the floor

Goal `doc-comments`'s whole acceptance list, which is the parity program plus goals `temp-sweep` through `doc-comments`. Never traded. Nothing in
this goal touches `nvs-stdlib`, `nvs-db` or `nvs-server`, so a failure there is a real regression and
never a scope question.

## Stage 2 — recovery a consumer can read

`MemberName::Missing(Span)` beside `MemberName::Ident` in `crates/nvs-syntax/src/ast.rs`, produced where
the synthesized name is made today (`crates/nvs-syntax/src/parser/expr.rs` — `python tools/peek.py
--locate` finds the site), and a span on `ExprKind::Error` naming what it stood in for. **A consumer must
never infer "did the user write this, or did the parser invent it at the cursor" from an empty span** —
completion's entire behaviour hangs on that one distinction, and it is cheaper to build now than to
retrofit under a request handler.

## Stage 3 — the index

`SyntaxIndex` joins `stmts` and `trivia` in the `Parsed` goal `doc-comments` built, filled by **one walk** and
answering `at(offset) -> NodePath` — the innermost node plus its ancestors, in order. `parse_file` stays
a thin wrapper over the resilient entry point, so the strict path is unchanged by construction rather
than by care. The ancestor list is not an implementation detail: it is `selectionRange`'s whole response
at goal `lsp-server`, which is why it is a list and not a leaf.

## Stage 4 — the prefix sweep, and the fuzz target

Every prefix of every `examples/*.nvs` at a token boundary: no panic, a `SyntaxIndex` answer at the final
offset, and a diagnostic on each prefix that is genuinely incomplete. Then a `fuzz_target` over truncated
and mid-edit inputs, **separate** from the existing whole-file `parse` target — a truncated input is a
different shape of input, and folding it into the existing target hides which one found a crash
(`rule:ide/one-grammar-one-tree` *Verification*).

## Stage 5 — position arithmetic has exactly one home

`utf16_col` and `offset_of` beside `line_col` in `crates/nvs-diagnostics/src/source.rs`, whose column
counts `char`s and is therefore **neither** encoding an LSP client can negotiate. This lands here rather
than in the server goal for one reason: `nvs-diagnostics` is this goal's file set, and the rule that pays
for it is *no other crate does its own conversion*. It is invisible on ASCII — which every fixture in this
repository is — and wrong on the first line holding a multi-byte character, so the test is a round trip
through both encodings on such a line, not a spot check.

## Stage 6 — the section lexer both case formats read

Extract `nvs_test`'s section lexer — its `header` and `parse` — into a module a second format can read,
with `.nvst` behaviour byte-identical afterwards. **That is the whole of this stage.** The `.lspt` format
itself, `nvs lsp-test`, and the canonical rendering are goal `lsp-server`'s: [conventions.md](../conventions.md)
§ *An `.lspt` case* puts the format's home in `crates/nvs-lsp`'s module doc and the rendering in
`nvs_lsp::render`, and a format specified before its only consumer exists is specified twice.

## Stage 7 — `nvs ast --json`, and the reference's first two headings

`--json` and `--resilient` on `run_ast` in `crates/nvs-cli/src/main.rs`. A node is `kind`, `span` as
`[start, end]`, its own scalar fields and `children`; **trivia and recovery nodes are included**, because
the panel this feeds is least useful on a file that compiles. `--resilient` is the default, and the schema
is frozen by a snapshot over `examples/`. `rule:ide/the-ast-panel-shells-out-to-the-cli` reads this flag; it does not exist yet, and
`{stmts:#?}` has no stability contract.

It is documented under `docs/reference/tools/10-cli.md`'s **existing** `# nvs ast` heading — two new
flags on a subcommand that already ships, not a new feature — so it adds no row to
`rule:testing/feature-proofs`'s derived roster. The new chapter
`40-editor.md` and its own headings belong to goals `lsp-server` and `editor`, which is where `nvs lsp` first exists.
`python tools/reference.py --check` is in the acceptance list because a chapter edit that stops
regenerating is how `docs/novis.md` goes quietly stale.

## Standing decisions — pre-authorized, do not stop the loop for these

- **One grammar, one tree. The `rowan` question is closed** by `rule:ide/one-grammar-one-tree`, which names what was given
  up (incremental reparse) and what protects the trade (goal `lsp-server`'s latency guard). If the implementation
  seems to force the opposite conclusion, keep this design, record *that* in `nvs-syntax`'s module doc
  with the reason, and put the CST in the handoff's `## Backlog` — never start a rewrite mid-run.
- **No ADR slots.** ADRs 0099, 0040 and 0137 decide everything here. Anything smaller is
  decided-and-recorded in the crate's own module doc or `docs/adr/README.md` § *Decisions taken at project
  start*, never a new number and never `BLOCKED`.
- **`SyntaxIndex` is built by the walk that already exists**, not by a second traversal added beside it.
  If the parse cannot fill it in one pass, fill it in one post-order walk of the finished tree and say so —
  what is refused is two walks that both know the shape of the tree.
- **The `.lspt` runner is not here** and a slice that reaches for it is off path: `## Backlog`, move on.
- **`.lspt` and `.nvst` stay two suites.** They share a section lexer and nothing else. `nvs test`'s
  `N passed` is the number the floor gates on, and making it count two unlike things breaks that gate and
  `conformance_coverage.rs` with it.
