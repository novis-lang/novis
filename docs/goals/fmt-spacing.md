---
looper: 1
title: "`nvs fmt` decides every space on a line, every run of blank lines and every broken construct"
summary: "Every run between two tokens on one line has one answer from a table of token roles, a run the table does not name is the author's only for a token kind the coverage test lists, a formatted file lexes to the token kinds its source did, a body has no blank line after its opening brace or before its closing one and no two blank lines in a row, every binary operator chain breaks operator first with a nested broken chain one level further in and a `.` chain keeping its author's lines, a broken chain or ternary breaks the list that holds it, a broken ternary has a line per branch, every control header breaks like a condition, every expression kind and control structure is on one of the two layout lists, every file and page the corpus holds is formatted, the seven rules this goal designs are shipped, and the checks below pass."
context: [crates/nvs-fmt/src/print.rs, crates/nvs-fmt/src/lib.rs, docs/rules/tooling/fmt-one-run-between-two-tokens.md, docs/agent/playbook/tooling/a-layout-rule-nvs-fmt-learns-is-green-only-if-the-corpus.md]
stages:
  - id: 1
    title: the run between two tokens in an expression or a declaration has one answer
    status: done
    context: [crates/nvs-fmt/src/space.rs, crates/nvs-syntax/src/token.rs, crates/nvs-fmt/tests/token_rules.rs]
    extra:
      context:
        rules: [tooling/fmt-one-run-between-two-tokens, tooling/fmt-novis-constructs, tooling/fmt-never-reflows]
  - id: 2
    title: types, unary operators and the author's runs, and a formatted file lexes to its source's tokens
    status: done
    context: [crates/nvs-syntax/src/ast.rs, crates/nvs-syntax/src/walk.rs, crates/nvs-fmt/tests/corpus.rs]
    extra:
      context:
        rules: [tooling/fmt-one-run-between-two-tokens, tooling/fmt-is-idempotent]
  - id: 3
    title: no two blank lines in a row, and none at the inside edge of a body
    status: done
    context: [crates/nvs-fmt/src/blank.rs, crates/nvs-fmt/src/statement.rs, crates/nvs-fmt/tests/base_style.rs, docs/rules/tooling/fmt-at-most-one-blank-line.md]
    extra:
      context:
        rules: [tooling/fmt-at-most-one-blank-line, tooling/fmt-base-style-is-per]
  - id: 7
    title: every binary operator chain breaks operator first, a nested one is one level in, and `.` keeps its lines
    status: done
    context: [crates/nvs-fmt/src/chain.rs, crates/nvs-fmt/tests/chain_layout.rs, docs/rules/tooling/fmt-every-binary-operator-breaks-operator-first.md]
    extra:
      context:
        rules: [tooling/fmt-every-binary-operator-breaks-operator-first, tooling/fmt-a-broken-operator-chain-is-one-operand-per-line]
  - id: 8
    title: a broken ternary has a line per branch, and a broken item breaks its list
    status: open
    context: [crates/nvs-fmt/src/chain.rs, crates/nvs-fmt/src/list.rs, crates/nvs-fmt/tests/chain_layout.rs, crates/nvs-fmt/tests/list_layout.rs]
    extra:
      context:
        rules: [tooling/fmt-a-broken-ternary-has-a-line-per-branch, tooling/fmt-a-broken-item-breaks-its-list, tooling/fmt-a-list-is-one-line-or-one-item-per-line]
  - id: 9
    title: every control header breaks like a condition
    status: open
    context: [crates/nvs-fmt/src/chain.rs, crates/nvs-fmt/tests/chain_layout.rs, docs/rules/tooling/fmt-every-control-header-breaks-like-a-condition.md]
    extra:
      context:
        rules: [tooling/fmt-every-control-header-breaks-like-a-condition]
  - id: 10
    title: every construct is on one of the two layout lists, and the editor splits and joins the new ones
    status: open
    context: [crates/nvs-syntax/src/walk.rs, crates/nvs-fmt/src/split_join.rs, docs/rules/tooling/fmt-every-construct-has-a-broken-layout.md]
    extra:
      context:
        rules: [tooling/fmt-every-construct-has-a-broken-layout, ide/a-list-splits-onto-lines-and-joins-onto-one]
  - id: 4
    title: every file and page the corpus holds is formatted
    status: open
    context: [tools/nv/cmd/nvs-fmt.ts, tools/data/nvs-fmt-corpus.txt]
  - id: 5
    title: the rules are shipped, and the `nvs fmt` page says what the command now does
    status: open
    context: [docs/rules/tooling/fmt-base-style-is-per.md, docs/rules/tooling/fmt-novis-constructs.md, docs/rules/tooling/fmt-a-broken-operator-chain-is-one-operand-per-line.md, docs/rules/tooling/fmt-a-list-is-one-line-or-one-item-per-line.md, docs/examples/tools/cli/nvs-fmt/about.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: token-gaps
    stage: 2
    name: every row of the spacing table and its coverage test pass
    run: [cargo, test, --test, token_gaps]
  - id: chain-layout
    stage: 9
    name: every broken chain, ternary, list item and control header case passes
    run: [cargo, test, --test, chain_layout]
  - id: layout-coverage
    stage: 10
    name: every syntax node kind is on one of the two layout lists
    run: [cargo, test, --test, layout_coverage]
  - id: corpus-formatted
    stage: 4
    name: every file and page the corpus holds is already formatted
    run: [bun, nv, nvs-fmt, --check]
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, fmt-spacing]
    reach: true
    floor: false
---

# `nvs fmt` decides every space on a line, every run of blank lines and every broken construct

## Why

`nvs fmt` lays out lines: indentation, braces, one statement per line, and lists and chains. Inside a
line it decides only a few runs: a qualifier's space, a one-line anonymous object's braces and the
space before a K&R brace. Every other run is copied as the author wrote it, because the printer keeps a
whitespace run that no stage names (`crates/nvs-fmt/src/print.rs:74`). So `$x=$a+$b   ;`,
`if($x>3)`, `foo ( $x )`, `$this -> total` and `class   Cart` all survive formatting. Three blank lines
in a body stay three. Two programs that differ only in spacing stay different, which is the thing
`rule:tooling/fmt-is-one-canonical-style` exists to prevent. `rule:tooling/fmt-base-style-is-per`
promises PER, and PER-CS fixes most of these runs.

The line breaks have the same hole. A broken `&&`, `||`, `??` or `.` chain, a `->` chain, a list and
an `if`, `elseif`, `while` or `do … while` condition have one layout each. A broken `+` or `>` chain, a
ternary and a `match`, `switch`, `foreach` or `for` header are copied as written. A break inside a
higher-precedence group leaves an `&&` under the `if` where it reads as top-level, and a broken
concatenation inside a call leaves its `)` on its last operand's line. A `.` chain of short pieces
becomes a column of one piece per line. Users write every construct the grammar accepts, whether the
corpus has one or not, so every construct gets a layout or is listed as keeping its author's.

## Standing decisions

**The table is `rule:tooling/fmt-one-run-between-two-tokens`'s, and the blank lines are
`rule:tooling/fmt-at-most-one-blank-line`'s.** Both are written as `designed` before the run, so a
session implements a row and never chooses one. A pair the table does not name keeps the author's run.
If a row turns out to break a corpus file's meaning, the safe fallback is to move that pair to the
author's list, and to say so in the rule.

**The line-break layouts are the five rules this goal designs, and the user chose each one**:
`rule:tooling/fmt-every-binary-operator-breaks-operator-first`,
`rule:tooling/fmt-a-broken-item-breaks-its-list`, `rule:tooling/fmt-a-broken-ternary-has-a-line-per-branch`,
`rule:tooling/fmt-every-control-header-breaks-like-a-condition` and
`rule:tooling/fmt-every-construct-has-a-broken-layout`. The `.` exception is the user's call: a broken
`.` chain keeps its author's lines with each `.` first, and is never one operand per line. The
formatter never adds parentheses to show a nested group; one more level of indentation shows it.

**The line-break layouts live in `chain.rs` and `list.rs`, as the existing ones do.** `OPERATORS`
(`crates/nvs-fmt/src/chain.rs:91`) grows to every binary operator, with `.` taking its own path, and
`CONDITIONS` (`crates/nvs-fmt/src/chain.rs:94`) grows to every control structure. A ternary is a
chain of two seams, `?` and `:`, and goes in `chain.rs` beside the operator chains. If `chain.rs`
passes about 1,200 lines, the session moves the headers into `crates/nvs-fmt/src/header.rs`.

**The coverage test reads the node kinds from `walk.rs`**, as `space.rs` already spells them, and the
two lists are constants in `chain.rs`. A node kind found on neither list fails the test by name. A
session that meets an unlisted kind puts it on the author's list, one level in, and adds it to the
rule's list in the same slice.

**One new module, `crates/nvs-fmt/src/gap.rs`, writes the table.** It hands the printer the same
`(offset, run)` pairs every other stage does, so nothing in `print.rs` changes but one `extend`. Its
runs are pushed last (`crates/nvs-fmt/src/print.rs:153`). `Runs::new` keeps the first answer for an
offset, so a list, chain, brace or statement stage that wants a line break always wins over a space.

**`gap.rs` lexes the file once more with `nvs_syntax::tokenize`** (`crates/nvs-syntax/src/lexer.rs:1700`),
into a scratch `Diagnostics` it drops. `Parsed` does not gain a token vector. Why: every parse the
compiler and the editor run would pay for that vector, and only the formatter reads it. The cost is one
more lex per formatted file, which is linear and runs only in `nvs fmt` and the editor's formatting
request.

**A token's role comes from the innermost node `SyntaxIndex` gives its offset**, with the node kinds
spelled as `crates/nvs-syntax/src/walk.rs` spells them, as `space.rs` already does. A written type is
not an index node, so stage 2 collects each `ast::Type` span (`crates/nvs-syntax/src/ast.rs:61`) once
per file and treats `?`, `|`, `&`, `<`, `>`, `(` and `)` inside one as type punctuation.

**A formatted file lexes to exactly the token kinds its source did.** It is a test over the corpus
(stage 2), and it is what keeps `- -$x` from becoming `--$x`. Before `gap.rs` empties a run, it lexes
the two tokens joined, and keeps one space if they lex as anything else.

**`fn(` keeps no space after `fn`**, as `rule:tooling/fmt-novis-constructs` already writes it. The
examples under `docs/examples/` write `fn(` in 208 places and `fn (` in 127, and the formatter makes
them one.

**The whole corpus is formatted in one stage, and the stage runs on the chain, never as a side goal.**
Stage 4 rewrites `.nvs` files in every tree, so it shares files with any goal that writes one. The
corpus is mostly PER already: 6,070 files put a space between a control keyword and its `(`, and 67
files leave it out. Hand alignment is lost in
the files that use it: 62 files align a trailing comment, 10 align `=`, and 8 align `=>`.

**Trade-offs.** Latency and throughput: nothing on the request path changes. `nvs fmt` does one more
lex per file. Memory: one token vector and one list of type spans per file, both dropped before the
next file, which is O(one file) in a command-line tool and an editor request. Simplicity: one module
and one table, held by a test that names every token kind. Usability: programs that differ only in
spacing format to the same bytes, and hand-aligned columns become single spaces. The cost is one large
corpus diff and one round of perf recording for the benches whose text changes. The line-break
layouts cost a walk over the nodes `chain.rs` already walks. A broken `.` chain has three layouts where
every other construct has two, which is the price of not splitting a sentence into a column.

**`|>` is on the author's list, not an operator chain.** The parser substitutes a pipeline into the
nested call it stands for (`crates/nvs-syntax/src/parser/expr.rs:629`) and leaves no node behind, so
`operator_of` has nothing to read. Giving it a layout would mean a node the parser does not build, or
reading the tokens for it. Simplicity wins: its breaks stay the author's, and
`rule:tooling/fmt-every-construct-has-a-broken-layout` lists it with the reason. **Operators of one
precedence level are one chain**, so `$a + $b\n- $c` is one chain of three, and a nested chain is
always a tighter-binding level.

**Lanes.** Every stage but 5 touches `crates/nvs-fmt` alone. Stages 1 and 2 share `gap.rs` and run in
order. Stages 7, 8, 9 and 10 share `chain.rs` and run in order, after stage 3, since 3 and 7 both touch
the code that writes a line's opening. Stages 1 and 2 are disjoint from 7 to 10 except one `extend` in
`print.rs`, so they may run side by side in their own worktrees, and the second to land rebases over
that line. Stage 4 needs stages 1 to 3 and 7 to 10 landed, so the corpus is formatted once. Stage 5
touches docs alone and may run beside stage 4.

## Stages

### 1. the run between two tokens in an expression or a declaration has one answer

**Does:** `gap.rs` writes every row of the table except the type, unary and author's rows.

- [x] `gap.rs`: lex once, walk each pair of adjacent tokens with no line break and no comment between them, take each token's role from its innermost node, and push the row's run. `crates/nvs-fmt/src/gap.rs:1`, `crates/nvs-syntax/src/lexer.rs:1700`
- [x] Its `mod gap;`, its line in the module list of the crate doc, and its `extend` after `statement::runs` (`rule:tooling/fmt-one-run-between-two-tokens`). `crates/nvs-fmt/src/lib.rs:95`, `crates/nvs-fmt/src/lib.rs:58`, `crates/nvs-fmt/src/print.rs:153`
- [x] The binary operator, `=>`, ternary, `,`, `;`, bracket, callee and control-keyword rows. `crates/nvs-fmt/src/gap.rs:1`
- [x] The `:`, `->` and `::`, keyword, two-word, `#[` and trailing-comment rows. `crates/nvs-fmt/src/gap.rs:1`
- [x] Tests: one case per row, each input wrong about its row and nothing else, as `token_rules.rs` writes its cases. Run once by name with `cargo test --test token_gaps`. `crates/nvs-fmt/tests/token_gaps.rs:1`, `crates/nvs-fmt/tests/token_rules.rs:13`

### 2. types, unary operators and the author's runs, and a formatted file lexes to its source's tokens

**Does:** the rows that need more than an operator's node, and the two guards that keep the table
honest.

- [x] The span of every written `ast::Type`, collected once per file: parameters, return types, properties, locals, constants, type aliases and `catch` types. Type punctuation inside one has no space. `crates/nvs-fmt/src/gap.rs:1`, `crates/nvs-syntax/src/ast.rs:61`
- [x] The unary row, read from the node kinds `walk.rs` gives a unary operator (`Neg`, `Not`, `BitNot`, `PreIncDec`, `PostIncDec` and `Suppress` among them). `crates/nvs-fmt/src/gap.rs:1`, `crates/nvs-syntax/src/walk.rs:1`
- [x] The three runs that stay the author's: after a block comment on a code line, beside `<?nvs`, `<?=` and `?>`, and a pair the table does not name. A run is never emptied where the joined pair lexes as another token. `crates/nvs-fmt/src/gap.rs:1`
- [x] The coverage test: it reads the variants of `pub enum TokenKind` from `token.rs` and fails naming any variant that is neither in `gap.rs`'s table nor on its list of kinds whose runs are the author's. `crates/nvs-fmt/tests/token_gaps.rs:1`, `crates/nvs-syntax/src/token.rs:99`
- [x] The corpus test `every_formatted_corpus_file_lexes_to_its_inputs_token_kinds`, beside the test that compares trees. `crates/nvs-fmt/tests/corpus.rs:147`
- [x] Tests: a type, `?int`, `int|string`, `array<string, int>`, `-$x`, `$a - -$b`, `!$ok`, `$i++` and each author's run. `cargo test --test token_gaps` passes. `crates/nvs-fmt/tests/token_gaps.rs:1`

### 3. no two blank lines in a row, and none at the inside edge of a body

**Does:** a whitespace run in code carries at most two line breaks, and one after an opening bracket
that ends its line or before a closing one that starts its line.

- [x] `blank.rs` names each whitespace run with three or more line breaks and writes two, outside the text nodes `space.rs` lists (`rule:tooling/fmt-at-most-one-blank-line`). `crates/nvs-fmt/src/blank.rs:29`, `crates/nvs-fmt/src/space.rs:82`
- [x] The run after a line that ends in `{`, `(` or `[`, and the run before a line that starts with `}`, `)` or `]`, carry one line break. `crates/nvs-fmt/src/blank.rs:29`, `crates/nvs-fmt/src/statement.rs:1`
- [x] The blank lines `rule:tooling/fmt-base-style-is-per` requires still win on their offset, as `print.rs` orders them now. `crates/nvs-fmt/src/print.rs:143`
- [x] Tests: three blank lines become one, a blank line after a method's `{` and before its `}` is deleted, and a heredoc's and a template's blank lines are kept. Run once by name with `cargo test --test base_style`. `crates/nvs-fmt/tests/base_style.rs:1`, `tests/fmt/input/blank.nvs:1`, `tests/fmt/formatted/blank.nvs:1`

### 7. every binary operator chain breaks operator first, a nested one is one level in, and `.` keeps its lines

**Does:** the operator chains cover every binary operator, a broken chain inside a broken chain is
indented one more level, and `.` keeps the lines its author wrote.

- [x] `OPERATORS` names the arithmetic operators, the comparisons, `<=>` and the bitwise operators beside `&&`, `||`, `??` and `.`, each with its precedence level, read from the `Binary` node's operator as `operator_of` reads it; `|>` is the author's, as Standing decisions says (`rule:tooling/fmt-every-binary-operator-breaks-operator-first`). `crates/nvs-fmt/src/chain.rs:91`, `crates/nvs-fmt/src/chain.rs:380`
- [x] A broken chain that is an operand of a broken chain opens its lines one level in from its first operand. `crates/nvs-fmt/src/chain.rs:555`, `crates/nvs-fmt/src/chain.rs:588`
- [x] A broken `.` chain writes a line break only where its author wrote one, and moves the `.` to the start of the line after it. `crates/nvs-fmt/src/chain.rs:400`, `crates/nvs-fmt/src/chain.rs:588`
- [x] The crate doc of `chain.rs` says what the module now does. `crates/nvs-fmt/src/chain.rs:1`
- [x] Tests: a broken `+` chain, a broken comparison, `||` over a broken `&&` group, a `.` chain broken after one operand and one broken before it, and a `.` chain with three lines. `crates/nvs-fmt/tests/chain_layout.rs:1`

### 8. a broken ternary has a line per branch, and a broken item breaks its list

**Does:** a ternary has its two layouts, and a broken chain or ternary inside a one-line list breaks
the innermost list around it.

- [ ] A `Ternary` with a line break at its own level opens a line for its `?` and its `:`, one level in from its condition's line, and a short ternary for its `?:` (`rule:tooling/fmt-a-broken-ternary-has-a-line-per-branch`). `crates/nvs-fmt/src/chain.rs:211`
- [ ] A list item that is a broken operator chain, call chain or ternary breaks the innermost list holding it; an anonymous function's body, a nested list and a heredoc still do not (`rule:tooling/fmt-a-broken-item-breaks-its-list`). `crates/nvs-fmt/src/list.rs:246`, `crates/nvs-fmt/src/list.rs:286`
- [ ] Tests: a ternary broken after its condition, one broken before its `:`, a short ternary, a broken concatenation as a call's only argument, a broken call chain as an argument, and an anonymous function's body that still leaves its list on one line. `crates/nvs-fmt/tests/chain_layout.rs:1`, `crates/nvs-fmt/tests/list_layout.rs:1`

### 9. every control header breaks like a condition

**Does:** `switch`, `match`, `foreach` and `for` headers take the condition layout, and a break inside
a nested operator chain breaks the header.

- [ ] `CONDITIONS` names `Switch`, `Match`, `Foreach` and `For` beside `If`, `While` and `DoWhile`, and `condition_of` finds each one's header between its `(` and `)` (`rule:tooling/fmt-every-control-header-breaks-like-a-condition`). `crates/nvs-fmt/src/chain.rs:94`, `crates/nvs-fmt/src/chain.rs:427`
- [ ] A line break inside an operator chain at any depth of the header, outside brackets of its own, breaks the header. `crates/nvs-fmt/src/chain.rs:448`
- [ ] A broken `foreach` header starts `as` and its binding on a line, with `$key => $value` kept together. A broken `for` header puts each clause on a line. `crates/nvs-fmt/src/chain.rs:567`
- [ ] Tests: each header kind broken after its `(`, a broken `match` subject, a `foreach` with a key, a `for` header, and an `if` broken inside its `&&` group. `cargo test --test chain_layout` passes. `crates/nvs-fmt/tests/chain_layout.rs:1`

### 10. every construct is on one of the two layout lists, and the editor splits and joins the new ones

**Does:** a test proves every syntax node kind has a layout or is listed as the author's, and the
editor's split and join actions cover the new constructs.

- [ ] The two lists in `chain.rs`: each construct with a broken layout and the rule that gives it, and each construct whose breaks are the author's, as `rule:tooling/fmt-every-construct-has-a-broken-layout` lists them. `crates/nvs-fmt/src/chain.rs:82`
- [ ] The coverage test reads every node kind `walk.rs` names and fails naming each that is on neither list. `cargo test --test layout_coverage` passes. `crates/nvs-fmt/tests/layout_coverage.rs:1`, `crates/nvs-syntax/src/walk.rs:1`
- [ ] **Put on separate lines** and **Join onto one line** are offered on every binary operator chain, a ternary and a control header, and write what `nvs fmt` writes. Split on a `.` chain puts one operand per line (`rule:ide/a-list-splits-onto-lines-and-joins-onto-one`). `crates/nvs-fmt/src/split_join.rs:1`
- [ ] Tests: split then format, and join then format, change nothing more for a `+` chain, a ternary and a `match` header. `crates/nvs-lsp/tests/actions.rs:154`

### 4. every file and page the corpus holds is formatted

**Does:** the corpus is rewritten once with the new rules, and every file that quotes a position in
it is made true again.

**Split:** `bun nv nvs-fmt --bless` is one command. A `reader` subagent takes the list of comments that
name a line number (`tools/nv/cmd/nvs-fmt.ts:152`) and returns `file:line` and the right number for
each. Writing subagents, one per tree, fix those comments side by side, and none of them builds.

- [ ] `bun nv nvs-fmt --bless` over the corpus, then `bun nv nvs-fmt --check` names nothing. `tools/nv/cmd/nvs-fmt.ts:1`, `tools/data/nvs-fmt-corpus.txt:1`
- [ ] Every expectation file it names that quotes a line or a column is checked against the program, and every comment it names that cites a moved line is rewritten. `tools/nv/cmd/nvs-fmt.ts:144`, `tools/nv/cmd/nvs-fmt.ts:152`
- [ ] A `.replay` file beside a rewritten website snippet repeats the snippet's text, so it is fixed by hand. `website/snippets/guides/how-to-use/field-paths/01-complete-a-path.replay:1`
- [ ] The follow-ups the tool prints, in its order: `bun nv render && bun nv render --website`, then `bun nv site --check stale questions` and each restamp it names. `tools/nv/cmd/nvs-fmt.ts:48`
- [ ] Once the sweep is committed, `bun nv proofs --record-perf --group <g>` for each group that `bun nv proofs --gate --limit 0` names stale, one group at a time. `tools/nv/cmd/nvs-fmt.ts:52`

### 5. the rules are shipped, and the `nvs fmt` page says what the command now does

**Does:** the rules say what the code does, and the page an end user reads matches it.

- [ ] `rule:tooling/fmt-base-style-is-per` says every other blank line is the author's, but never more than one in a row, and cites `rule:tooling/fmt-at-most-one-blank-line`. `docs/rules/tooling/fmt-base-style-is-per.md:17`
- [ ] `rule:tooling/fmt-novis-constructs`'s attribute line is rewritten: an attribute `#[...]` is spaced by `rule:tooling/fmt-one-run-between-two-tokens`'s `#[` row. `docs/rules/tooling/fmt-novis-constructs.md:27`
- [ ] The `nvs fmt` page says in two plain sentences what is set inside a line and how many blank lines are kept, held to § *Text an end user reads* in `AGENTS.md`. `docs/examples/tools/cli/nvs-fmt/about.md:1`
- [ ] `rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line` says a `.` chain keeps its author's lines and every control header breaks as its condition does, citing the two rules that now say so. `docs/rules/tooling/fmt-a-broken-operator-chain-is-one-operand-per-line.md:1`
- [ ] `rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`'s exception for an item that spans lines names the items that break the list, citing `rule:tooling/fmt-a-broken-item-breaks-its-list`. `docs/rules/tooling/fmt-a-list-is-one-line-or-one-item-per-line.md:25`
- [ ] `rule:ide/a-list-splits-onto-lines-and-joins-onto-one` names every construct stage 10 offers the actions on. `docs/rules/ide/a-list-splits-onto-lines-and-joins-onto-one.md:1`
- [ ] The `nvs fmt` page also says how a broken condition, chain and ternary are laid out, within the page's word limit. `docs/examples/tools/cli/nvs-fmt/about.md:1`
- [ ] The seven rules are `shipped`: the two spacing rules with `token_gaps.rs`, `base_style.rs` and `corpus.rs` in `guardedBy`, and the five layout rules with `chain_layout.rs`, `list_layout.rs` and `layout_coverage.rs`. Then `bun nv rules --render` and `bun nv owners --closes fmt-spacing`. `data/rules/tooling/fmt-one-run-between-two-tokens.json:3`, `data/rules/tooling/fmt-at-most-one-blank-line.json:3`, `data/rules/tooling/fmt-every-binary-operator-breaks-operator-first.json:3`, `data/rules/tooling/fmt-a-broken-item-breaks-its-list.json:3`, `data/rules/tooling/fmt-a-broken-ternary-has-a-line-per-branch.json:3`, `data/rules/tooling/fmt-every-control-header-breaks-like-a-condition.json:3`, `data/rules/tooling/fmt-every-construct-has-a-broken-layout.json:3`

### 6. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1, 2, 3 and 7 are done. `crates/nvs-fmt/src/gap.rs` is the whole table, and `cargo test --test
token_gaps` passes its 25 cases, the coverage test among them.

- Operator chains: `OPERATORS` (`crates/nvs-fmt/src/chain.rs:103`) gives every `Binary` operator its
  precedence level, and `flatten` joins one level. `Operators::opens_line` is the `.` exception.
  `operand_opening` (`crates/nvs-fmt/src/chain.rs:624`) places a chain's first operand from the
  broken chain or condition around it, which is the one-level-further nesting, and it works where the
  author never broke that line. `cargo test --test chain_layout` passes 14. `|>` is the author's.
- Blank lines: `blank::collapsed` (`crates/nvs-fmt/src/blank.rs:62`), pushed after `statement::runs`.
- Types and unary: `gap::runs` (`crates/nvs-fmt/src/gap.rs:180`); `AUTHORS` (`gap.rs:158`) is the
  author's list, and `crates/nvs-fmt/tests/corpus.rs:202` holds the token kinds over the corpus.
- The sweep's reds are stale: nightly `55b03e97a` predates the Miri fixes; `env-secret-proofs` is 0.

Expected red until stage 4: `crates/nvs-fmt/tests/identity.rs`, and every corpus file and fixture
with a binary chain broken at its own level, a `.` ending a line, a pair, type or unary spaced
another way (`tests/fmt/formatted/comments.nvs`), or a blank line the rule removes. The LSP suites
calling `written_types_in` were built,
not run; stage 6 runs them.

### Next

**Stage 8: a broken ternary has a line per branch, and a broken item breaks its list**, one file
set: `crates/nvs-fmt/src/chain.rs`, `crates/nvs-fmt/src/list.rs`, `crates/nvs-fmt/tests/chain_layout.rs`,
`crates/nvs-fmt/tests/list_layout.rs`. The items are the stage's own, under `### 8.` above; stages 9
and 10 follow in the same files.

- [ ] A broken `Ternary` opens a line for its `?` and its `:`, and `?:` stays one token pair. A ternary is a chain of two seams, so it can be an `Operators` with the two spellings, and `operand_opening` (`crates/nvs-fmt/src/chain.rs:624`) then nests it. `crates/nvs-fmt/src/chain.rs:249`, `crates/nvs-fmt/src/chain.rs:680`
- [ ] A list item that is a broken chain or ternary breaks the innermost list. `crates/nvs-fmt/src/list.rs:246`, `crates/nvs-fmt/src/list.rs:286`
- [ ] The tests the stage names, run once by name. `crates/nvs-fmt/tests/chain_layout.rs:1`, `crates/nvs-fmt/tests/list_layout.rs:1`

### Backlog

- A test that the editor's split and join (`crates/nvs-fmt/src/split_join.rs`) writes what `gap.rs` would, if stage 4 finds a joined list it re-spaces.
