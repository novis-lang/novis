---
looper: 1
title: "`nvs fmt` decides every space on a line and every run of blank lines"
summary: "Every run between two tokens on one line has one answer from a table of token roles, a run the table does not name is the author's only for a token kind the coverage test lists, a formatted file lexes to the token kinds its source did, a body has no blank line after its opening brace or before its closing one and no two blank lines in a row, every file and page the corpus holds is formatted, rules tooling/fmt-one-run-between-two-tokens and tooling/fmt-at-most-one-blank-line are shipped, and the checks below pass."
context: [crates/nvs-fmt/src/print.rs, crates/nvs-fmt/src/lib.rs, docs/rules/tooling/fmt-one-run-between-two-tokens.md, docs/agent/playbook/tooling/a-layout-rule-nvs-fmt-learns-is-green-only-if-the-corpus.md]
stages:
  - id: 1
    title: the run between two tokens in an expression or a declaration has one answer
    status: open
    context: [crates/nvs-fmt/src/space.rs, crates/nvs-syntax/src/token.rs, crates/nvs-fmt/tests/token_rules.rs]
    extra:
      context:
        rules: [tooling/fmt-one-run-between-two-tokens, tooling/fmt-novis-constructs, tooling/fmt-never-reflows]
  - id: 2
    title: types, unary operators and the author's runs, and a formatted file lexes to its source's tokens
    status: open
    context: [crates/nvs-syntax/src/ast.rs, crates/nvs-syntax/src/walk.rs, crates/nvs-fmt/tests/corpus.rs]
    extra:
      context:
        rules: [tooling/fmt-one-run-between-two-tokens, tooling/fmt-is-idempotent]
  - id: 3
    title: no two blank lines in a row, and none at the inside edge of a body
    status: open
    context: [crates/nvs-fmt/src/blank.rs, crates/nvs-fmt/src/statement.rs, crates/nvs-fmt/tests/base_style.rs, docs/rules/tooling/fmt-at-most-one-blank-line.md]
    extra:
      context:
        rules: [tooling/fmt-at-most-one-blank-line, tooling/fmt-base-style-is-per]
  - id: 4
    title: every file and page the corpus holds is formatted
    status: open
    context: [tools/nv/cmd/nvs-fmt.ts, tools/data/nvs-fmt-corpus.txt]
  - id: 5
    title: the rules are shipped, and the `nvs fmt` page says what the command now does
    status: open
    context: [docs/rules/tooling/fmt-base-style-is-per.md, docs/rules/tooling/fmt-novis-constructs.md, docs/examples/tools/cli/nvs-fmt/about.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: token-gaps
    stage: 2
    name: every row of the spacing table and its coverage test pass
    run: [cargo, test, --test, token_gaps]
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

# `nvs fmt` decides every space on a line and every run of blank lines

## Why

`nvs fmt` lays out lines: indentation, braces, one statement per line, and lists and chains. Inside a
line it decides only a few runs: a qualifier's space, a one-line anonymous object's braces and the
space before a K&R brace. Every other run is copied as the author wrote it, because the printer keeps a
whitespace run that no stage names (`crates/nvs-fmt/src/print.rs:74`). So `$x=$a+$b   ;`,
`if($x>3)`, `foo ( $x )`, `$this -> total` and `class   Cart` all survive formatting. Three blank lines
in a body stay three. Two programs that differ only in spacing stay different, which is the thing
`rule:tooling/fmt-is-one-canonical-style` exists to prevent. `rule:tooling/fmt-base-style-is-per`
promises PER, and PER-CS fixes most of these runs.

## Standing decisions

**The table is `rule:tooling/fmt-one-run-between-two-tokens`'s, and the blank lines are
`rule:tooling/fmt-at-most-one-blank-line`'s.** Both are written as `designed` before the run, so a
session implements a row and never chooses one. A pair the table does not name keeps the author's run.
If a row turns out to break a corpus file's meaning, the safe fallback is to move that pair to the
author's list, and to say so in the rule.

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
corpus diff and one round of perf recording for the benches whose text changes.

**Lanes.** Stages 1 to 3 touch `crates/nvs-fmt` alone and run in order: 1 and 2 share `gap.rs`, and 3
shares `print.rs`. Stage 3 is disjoint from `gap.rs` and may run beside stage 2 in its own worktree.
Stage 4 needs stages 1 to 3 landed. Stage 5 touches docs alone and may run beside stage 4.

## Stages

### 1. the run between two tokens in an expression or a declaration has one answer

**Does:** `gap.rs` writes every row of the table except the type, unary and author's rows.

- [ ] `gap.rs`: lex once, walk each pair of adjacent tokens with no line break and no comment between them, take each token's role from its innermost node, and push the row's run. `crates/nvs-fmt/src/gap.rs:1`, `crates/nvs-syntax/src/lexer.rs:1700`
- [ ] Its `mod gap;`, its line in the module list of the crate doc, and its `extend` after `statement::runs` (`rule:tooling/fmt-one-run-between-two-tokens`). `crates/nvs-fmt/src/lib.rs:95`, `crates/nvs-fmt/src/lib.rs:58`, `crates/nvs-fmt/src/print.rs:153`
- [ ] The binary operator, `=>`, ternary, `,`, `;`, bracket, callee and control-keyword rows. `crates/nvs-fmt/src/gap.rs:1`
- [ ] The `:`, `->` and `::`, keyword, two-word, `#[` and trailing-comment rows. `crates/nvs-fmt/src/gap.rs:1`
- [ ] Tests: one case per row, each input wrong about its row and nothing else, as `token_rules.rs` writes its cases. Run once by name with `cargo test --test token_gaps`. `crates/nvs-fmt/tests/token_gaps.rs:1`, `crates/nvs-fmt/tests/token_rules.rs:13`

### 2. types, unary operators and the author's runs, and a formatted file lexes to its source's tokens

**Does:** the rows that need more than an operator's node, and the two guards that keep the table
honest.

- [ ] The span of every written `ast::Type`, collected once per file: parameters, return types, properties, locals, constants, type aliases and `catch` types. Type punctuation inside one has no space. `crates/nvs-fmt/src/gap.rs:1`, `crates/nvs-syntax/src/ast.rs:61`
- [ ] The unary row, read from the node kinds `walk.rs` gives a unary operator (`Neg`, `Not`, `BitNot`, `PreIncDec`, `PostIncDec` and `Suppress` among them). `crates/nvs-fmt/src/gap.rs:1`, `crates/nvs-syntax/src/walk.rs:1`
- [ ] The three runs that stay the author's: after a block comment on a code line, beside `<?nvs`, `<?=` and `?>`, and a pair the table does not name. A run is never emptied where the joined pair lexes as another token. `crates/nvs-fmt/src/gap.rs:1`
- [ ] The coverage test: it reads the variants of `pub enum TokenKind` from `token.rs` and fails naming any variant that is neither in `gap.rs`'s table nor on its list of kinds whose runs are the author's. `crates/nvs-fmt/tests/token_gaps.rs:1`, `crates/nvs-syntax/src/token.rs:99`
- [ ] The corpus test `every_formatted_corpus_file_lexes_to_its_inputs_token_kinds`, beside the test that compares trees. `crates/nvs-fmt/tests/corpus.rs:147`
- [ ] Tests: a type, `?int`, `int|string`, `array<string, int>`, `-$x`, `$a - -$b`, `!$ok`, `$i++` and each author's run. `cargo test --test token_gaps` passes. `crates/nvs-fmt/tests/token_gaps.rs:1`

### 3. no two blank lines in a row, and none at the inside edge of a body

**Does:** a whitespace run in code carries at most two line breaks, and one after an opening bracket
that ends its line or before a closing one that starts its line.

- [ ] `blank.rs` names each whitespace run with three or more line breaks and writes two, outside the text nodes `space.rs` lists (`rule:tooling/fmt-at-most-one-blank-line`). `crates/nvs-fmt/src/blank.rs:29`, `crates/nvs-fmt/src/space.rs:82`
- [ ] The run after a line that ends in `{`, `(` or `[`, and the run before a line that starts with `}`, `)` or `]`, carry one line break. `crates/nvs-fmt/src/blank.rs:29`, `crates/nvs-fmt/src/statement.rs:1`
- [ ] The blank lines `rule:tooling/fmt-base-style-is-per` requires still win on their offset, as `print.rs` orders them now. `crates/nvs-fmt/src/print.rs:143`
- [ ] Tests: three blank lines become one, a blank line after a method's `{` and before its `}` is deleted, and a heredoc's and a template's blank lines are kept. Run once by name with `cargo test --test base_style`. `crates/nvs-fmt/tests/base_style.rs:1`, `tests/fmt/input/blank.nvs:1`, `tests/fmt/formatted/blank.nvs:1`

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
- [ ] Both rules are `shipped`, with `token_gaps.rs`, `base_style.rs` and `corpus.rs` in `guardedBy`, then `bun nv rules --render` and `bun nv owners --closes fmt-spacing`. `data/rules/tooling/fmt-one-run-between-two-tokens.json:3`, `data/rules/tooling/fmt-at-most-one-blank-line.json:3`

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

The goal has not started. Both rules are written as `designed`, and `gap.rs` does not exist. The
printer copies every whitespace run that no stage names (`crates/nvs-fmt/src/print.rs:74`), and
`space.rs` is the nearest stage in shape: it scans code runs and pushes `(offset, run)` pairs
(`crates/nvs-fmt/src/space.rs:84`).

### Next

**Stage 1: the table for expressions and declarations**, one file set: `crates/nvs-fmt/src/gap.rs`,
`crates/nvs-fmt/src/lib.rs`, `crates/nvs-fmt/src/print.rs`, `crates/nvs-fmt/tests/token_gaps.rs`.
Gate: `docs/goals/README.md` names `fmt-spacing` live, after `routes-by-method` is reached.

- [ ] `gap.rs`: lex once, walk adjacent pairs on one line, and push each row's run. `crates/nvs-fmt/src/gap.rs:1`
- [ ] Its module, its crate-doc line, and its `extend` after `statement::runs`. `crates/nvs-fmt/src/print.rs:153`
- [ ] The binary operator, `=>`, ternary, `,`, `;`, bracket, callee and control-keyword rows. `crates/nvs-fmt/src/gap.rs:1`
- [ ] The `:`, `->` and `::`, keyword, two-word, `#[` and trailing-comment rows. `crates/nvs-fmt/src/gap.rs:1`
- [ ] Tests: one case per row, run once with `cargo test --test token_gaps`. `crates/nvs-fmt/tests/token_gaps.rs:1`

### Backlog

- A test that the editor's split and join (`crates/nvs-fmt/src/split_join.rs`) writes what `gap.rs` would, if stage 4 finds a joined list it re-spaces.
