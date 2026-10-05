# Side goal — a line break the author writes decides the layout `nvs fmt` gives a list, a call chain and an operator chain

When this goal is green, `nvs fmt` gives every list, `->` call chain and `&&`, `||`, `??` or `.` chain
one of two layouts: on one line, or one part per line. A line break the author wrote at the
construct's own level selects the second, and nothing else does: there is still no line width, and
`nvs fmt` never joins a broken construct. A broken `if`, `elseif`, `while` or `do … while` condition
puts its `(` and `) {` on lines of their own. In the editor, *Put on separate lines* and *Join onto one
line* add or remove the breaks. The rules are written: ADR 0267 and the four rules it creates.

## Why a side goal

The user decided the design on 2026-10-05 and asked for it as a side goal, so the chain run is not
disturbed. Nothing on the chain waits for it. It touches `crates/nvs-fmt` and one action in
`crates/nvs-lsp`, plus whatever `.nvs` files in the tree the new layout moves.

## What is on disk today

- **The rules.** `rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`,
  `rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line`,
  `rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line` and
  `rule:ide/a-list-splits-onto-lines-and-joins-onto-one` are `designed`, with empty `guardedBy`.
  `rule:tooling/fmt-never-reflows`, `rule:tooling/fmt-trailing-commas`, `rule:tooling/fmt-is-idempotent`
  and `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows` already state the new
  behaviour. ADR 0267 has the reasoning and the user's examples.
- **The formatter copies every continuation line.** `crates/nvs-fmt/src/indent.rs:24-32`:
  `Indent::of_line` answers `None` for a line the tree does not place, and the printer copies the
  author's whitespace. A continuation line inside a call is one of those. The gap record
  `data/gaps/nvs-fmt/line-the-tree-does-not-place.json` names this goal as what places them.
- **A test asserts the old behaviour.** `crates/nvs-fmt/tests/never_reflows.rs:20`
  `an_authors_line_break_inside_an_expression_is_kept` expects a broken array with misindented items to
  keep that indentation. It changes in stage 2.
- **The whole corpus is held to `nvs fmt`.** `crates/nvs-fmt/tests/identity.rs` checks that the
  tree's `.nvs` files already format to themselves, `crates/nvs-fmt/tests/fixtures.rs` pairs
  `tests/fmt/input/` with `tests/fmt/formatted/`, and `nv verify` runs `nvs fmt` over the `.nvs`
  files a change touches. Not checked: how many corpus files have a list wrapped several items to a
  line. Stage 2's first step is to count them with the new rule.
- **The action precedent.** `crates/nvs-lsp/src/html_template.rs` is the one action the server computes
  for itself today (`rule:ide/a-string-converts-to-an-html-template`), with cases under
  `tests/lsp/actions/`.

## Stage 1 — the floor

Main's carried floor, which a side run is always checked against. Never traded.

## Stage 2 — a list is one line or one item per line

**Does:** Lays out every list from the author's line break: one line, or one item per line with a
trailing comma and the closer on its own line.

File set: `crates/nvs-fmt/src/lib.rs`, `crates/nvs-fmt/src/indent.rs`, `crates/nvs-fmt/src/print.rs`,
`crates/nvs-fmt/src/tokens.rs`, `crates/nvs-fmt/tests/list_layout.rs` (new),
`crates/nvs-fmt/tests/never_reflows.rs`, `tests/fmt/`, and the corpus `.nvs` files the new layout
moves.

- **The lists:** call arguments including `new`'s, parameter lists of functions, methods and anonymous
  functions, array literals, anonymous objects, shape types, and an enum's case list. Not `match` arms
  or the `use` block.
- **An enum on one line** keeps its `{` on the `enum` line with one space inside each brace:
  `enum AxisPosition { Left, Right }`. Today `nvs fmt` moves that brace down and leaves
  `{ Left, Right }` on a line of its own (run on 2026-10-05). A broken enum takes the Allman brace and
  one case per line. `rule:tooling/fmt-base-style-is-per` and `rule:tooling/fmt-novis-constructs`
  already state this.
- **Broken** means a line break in the trivia at the list's own level: after the opener, between two
  items, or before the closer. A break inside an item, including a function body and a heredoc, is the
  item's.
- **Layout:** the opener ends its line, each item starts a line one level in from the opener's line, a
  trailing comma follows the last item, and the closer starts a line at the opener line's indentation.
  An item spanning several lines keeps its own layout, moved in with its first line.
- **Comments** stay on the line of the item they follow, or keep their own line. A line comment makes
  the list broken.
- **The tests** in `list_layout.rs`, one per case: the names in the record's checks.
- **Reformat the corpus** in its own commit once the tests are green, and update
  `never_reflows.rs` to the new layout. Add a `tests/fmt/` pair that moves a list wrapped two items
  to a line.
- **When green:** set `rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line` to `shipped` with `guardedBy`
  `crates/nvs-fmt/tests/list_layout.rs`, and rewrite the gap record and `indent.rs`'s module doc to
  say what is placed now.

## Stage 3 — call chains and operator chains

**Does:** Puts every call of a broken `->` chain, and every operand of a broken `&&`, `||`, `??` or `.`
chain, on a line of its own, and lays out a broken condition with `(` and `) {` on their own lines.

File set: `crates/nvs-fmt/src/indent.rs`, `crates/nvs-fmt/src/print.rs`, `crates/nvs-fmt/src/space.rs`,
`crates/nvs-fmt/tests/chain_layout.rs` (new), `tests/fmt/`, and the corpus `.nvs` files the new layout
moves.

- **Call chain:** two or more `->` or `?->` calls, broken by a line break before any arrow. The receiver
  stays on the first line; each arrow starts a line one level in.
- **Operator chain:** broken by a line break between two operands. The first operand stays; each
  following operand starts a line one level in, operator first. A mixed run is the run at the lowest
  precedence, and a higher-precedence operand is judged on its own.
- **Condition** of `if`, `elseif`, `while`, `do … while`: broken by a break after `(`, between two
  top-level operands, or before `)`. Then `(` ends its line, each operand starts a line one level in,
  and `)` starts its own line followed by ` {`.
- **When green:** set both chain rules to `shipped` with `guardedBy`
  `crates/nvs-fmt/tests/chain_layout.rs`.

## Stage 4 — *Put on separate lines* and *Join onto one line*

**Does:** Offers the two editor actions on the innermost list, call chain or operator chain around the
cursor.

File set: `crates/nvs-lsp/src/` (a new module beside `html_template.rs`, and where that one is
registered), `crates/nvs-lsp/tests/actions.rs`, `tests/lsp/actions/`.

- Both actions write the layout `nvs fmt` gives the construct after the break is added or removed. They
  reuse `nvs-fmt`'s layout rather than a second copy of it (`rule:ide/one-server-two-thin-clients`).
- Kind `refactor.rewrite`, never `source.fixAll.nvs` or `quickfix`. Join is not offered when the
  construct holds a line comment.
- **When green:** set `rule:ide/a-list-splits-onto-lines-and-joins-onto-one` to `shipped` with its
  `guardedBy` the three cases in the record's checks.

## Standing decisions

These are the user's calls, made on 2026-10-05, unless marked as mine. No session re-decides one.

- **No line width, soft or hard.** `nvs fmt` never splits a line because it is long and never joins one.
- **Two layouts per construct:** one line, or one part per line. No packing several items onto a line.
- **The signal is the author's line break at the construct's own level**, including a break after the
  opener or before the closer. A break inside a function body or another nested part does not count,
  whatever the number of function arguments.
- **A broken condition** has the operator first on each line, and `(` and `) {` on lines of their own.
- **Editor actions** add and remove the breaks; `nvs fmt` does not.
- **An enum's case list is a list.** On one line it is `enum AxisPosition { Left, Right }`, brace on
  the `enum` line; broken, it is the Allman brace and one case per line.
- **One side goal**, so the chain run is not disturbed.
- **Mine, 2026-10-05:** `??` and `?->` join the chains; shape types and parameter lists are lists; a
  mixed operator run breaks at its lowest precedence; Join is not offered over a line comment; the
  action titles; the test and case names.
- **ADR slots: none.** ADR 0267 is written. Amend a rule fragment only where it states behaviour this
  goal changes.
- **The tradeoffs, stated once.** Runtime performance and memory: none, because nothing in `nvs` run
  or the server changes. `nvs fmt` stays one pass over the lossless tree. Developers get one rule for
  every list-shaped construct and no surprising breaks. Already formatted files see a one-time diff.
