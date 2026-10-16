---
looper: 1
title: "`nvs fmt` has no open gap"
summary: "Every gap record under data/gaps/nvs-fmt/ is closed and deleted, nvs fmt's exit status says what it found, and the checks below pass."
context: [crates/nvs-fmt/src/lib.rs, docs/rules/tooling/fmt-base-style-is-per.md, docs/rules/tooling/fmt-is-idempotent.md]
stages:
  - id: 1
    title: "the exit status says what `nvs fmt` found, and the walk leaves `vendor/` alone"
    status: open
    context: [crates/nvs-cli/src/fmt.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/check.rs, crates/nvs-cli/tests/fmt.rs, docs/rules/tooling/fmt-exit-status-says-what-it-found.md, docs/rules/tooling/fmt-check-writes-nothing.md]
    extra:
      context:
        rules: [tooling/fmt-exit-status-says-what-it-found, tooling/fmt-check-writes-nothing]
  - id: 2
    title: "one statement per line, a closing brace on its own line, and `elseif` as one word"
    status: open
    context: [crates/nvs-fmt/src/brace.rs, crates/nvs-fmt/src/indent.rs, crates/nvs-fmt/tests/base_style.rs, crates/nvs-syntax/src/parser/stmt.rs, data/gaps/nvs-fmt/closing-brace-is-moved-onto-a.json, data/gaps/nvs-fmt/else-if-is-kept-as-two-words.json]
    extra:
      context:
        rules: [tooling/fmt-base-style-is-per, tooling/fmt-is-idempotent]
  - id: 3
    title: "a shape type and an attribute are nodes, so their lists and literals are formatted"
    status: open
    context: [crates/nvs-syntax/src/walk.rs, crates/nvs-fmt/src/list.rs, crates/nvs-fmt/src/tokens.rs, crates/nvs-fmt/src/space.rs, crates/nvs-cli/tests/ast.rs, crates/nvs-fmt/tests/list_layout.rs, data/gaps/nvs-fmt/comma-separated-list-whose-closing-delimiter.json, data/gaps/nvs-fmt/literal-written-inside-an-attribute-keeps.json]
    extra:
      context:
        rules: [tooling/fmt-a-list-is-one-line-or-one-item-per-line, tooling/fmt-trailing-commas, tooling/fmt-quotes, ide/ast-json-schema-is-frozen]
  - id: 4
    title: "a comment moves with its import, and every line is placed"
    status: open
    context: [crates/nvs-fmt/src/imports.rs, crates/nvs-fmt/src/indent.rs, crates/nvs-fmt/tests/use_block.rs, data/gaps/nvs-fmt/comment-written-between-two-imports-holds.json, data/gaps/nvs-fmt/line-the-tree-does-not-place.json]
    extra:
      context:
        rules: [tooling/fmt-sorts-the-use-block, tooling/fmt-base-style-is-per, tooling/fmt-never-reflows]
  - id: 5
    title: "PER's blank lines are written, and the printer's record is closed"
    status: open
    context: [crates/nvs-fmt/src/lib.rs, crates/nvs-fmt/src/print.rs, crates/nvs-fmt/tests/corpus.rs, data/gaps/nvs-fmt/rules-written-into-the-printer-are.json]
    extra:
      context:
        rules: [tooling/fmt-base-style-is-per, ide/tokens-plus-trivia-reproduce-the-file]
checks:
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, formatter-complete]
    reach: true
    floor: false
extra:
  milestone: M10
---

# `nvs fmt` has no open gap

## Why

`nvs fmt` is built: `--check`, `--diff` and `--stdin`, a corpus idempotence test and a comment
round trip. Seven gap records under `data/gaps/nvs-fmt/` say where its output still differs from
the `fmt-*` rules: a closing brace, `else if`, a shape type's list, an attribute's literal, a
comment inside the `use` block, lines the tree does not place, and PER's blank lines. Three CLI
details are also wrong. A file that does not parse exits `1`, like a file that would change, so a
hook cannot tell the two apart. The directory walk formats `vendor/`. `--check` is refused beside
`--stdin`, so an editor or a hook cannot ask whether a buffer is formatted. This goal closes all of
it.

## Standing decisions

**Exit `1` means a file would change, and exit `2` means a file was refused.** The planning
session's call, as Prettier and Black do it. Today both exit `1` (`crates/nvs-cli/src/fmt.rs:93`,
`crates/nvs-cli/src/fmt.rs:165`). `rule:tooling/fmt-exit-status-says-what-it-found` holds it.

**A new node in `crates/nvs-syntax/src/walk.rs` changes `nvs ast --json`'s frozen document.**
Stage 3 adds a node for a shape type and one for an attribute, so the frozen example in
`crates/nvs-cli/tests/ast.rs:178` is re-recorded in the same commit, and
`rule:ide/ast-json-schema-is-frozen` is read first: a new `kind` value is an addition the schema
allows, and a reshaped node is not.

**The closing brace waits for one statement per line.** A `}` that shares a line with the
statement before it can only move once the statement before it is on a line of its own, so stage 2
does that first.

**Trade-offs.** No program's behaviour changes, and nothing runs on the request path. `nvs fmt`
output changes for files that differ from the rules, which is the point. Two more node kinds make
the syntax walk a little larger and every formatter rule simpler, because a shape type's `{` and an
attribute's arguments no longer need a second scan. The exit status split costs nothing and makes
a hook simpler to write.

## Stages

### 1. the exit status says what `nvs fmt` found, and the walk leaves `vendor/` alone

**Does:** `nvs fmt` exits `0`, `1` or `2` by what it found, a directory walk skips `vendor/`, and
`--check` works with `--stdin`.

- [ ] A refused or unreadable file exits `2`, and a file that would change exits `1`. `2` wins when both happen (`rule:tooling/fmt-exit-status-says-what-it-found`). `crates/nvs-cli/src/fmt.rs:93`, `crates/nvs-cli/src/fmt.rs:152`
- [ ] The walk skips a directory named `vendor`, as `nvs check --fix` leaves vendored code alone. A path named on the command line is still formatted. `crates/nvs-cli/src/fmt.rs:128`, `crates/nvs-cli/src/check.rs:135`
- [ ] `--check` with `--stdin` writes nothing and exits by the same codes. `crates/nvs-cli/src/main.rs:819`
- [ ] Tests for each, and `rule:tooling/fmt-check-writes-nothing` says the walk skips `vendor/`. `crates/nvs-cli/tests/fmt.rs:1`
- [ ] `rule:tooling/fmt-exit-status-says-what-it-found` is `shipped`.

### 2. one statement per line, a closing brace on its own line, and `elseif` as one word

**Does:** the brace paragraph of `rule:tooling/fmt-base-style-is-per` is fully applied.

- [ ] Two statements on one line are split onto two lines. `crates/nvs-fmt/src/indent.rs:1`
- [ ] A `}` that shares a line with the statement before it moves onto a line of its own. `crates/nvs-fmt/src/brace.rs:1`, `data/gaps/nvs-fmt/closing-brace-is-moved-onto-a.json:1`
- [ ] `else if` is written `elseif`. The parser reads both into one arm, so the tree does not change. `crates/nvs-syntax/src/parser/stmt.rs:411`, `data/gaps/nvs-fmt/else-if-is-kept-as-two-words.json:1`
- [ ] The corpus idempotence test still passes, and both records are deleted. `crates/nvs-fmt/tests/corpus.rs:1`

### 3. a shape type and an attribute are nodes, so their lists and literals are formatted

**Does:** a shape type's fields follow the list rule and the trailing comma rule, and a literal
inside an attribute takes the quote every other literal takes.

- [ ] A node for a type, a shape type among them, in the syntax walk, and the frozen `ast --json` example re-recorded (`rule:ide/ast-json-schema-is-frozen`). `crates/nvs-syntax/src/walk.rs:1`, `crates/nvs-cli/tests/ast.rs:178`
- [ ] `list.rs` lays out a shape type's fields as a list (`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`, `rule:tooling/fmt-trailing-commas`). `crates/nvs-fmt/src/list.rs:54`
- [ ] A node for an attribute and its arguments, and `tokens.rs` respells a literal inside it (`rule:tooling/fmt-quotes`). `crates/nvs-fmt/src/tokens.rs:1`
- [ ] Both records are deleted, and `rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line` is `shipped`.

### 4. a comment moves with its import, and every line is placed

**Does:** the `use` block sorts across a comment, and each line the tree does not place today gets
a rule.

- [ ] An import carries the comment written above it as a second span, and the whole block sorts (`rule:tooling/fmt-sorts-the-use-block`). `crates/nvs-fmt/src/imports.rs:1`
- [ ] A comment line is placed at the indentation of the line below it (`rule:tooling/fmt-base-style-is-per`). `crates/nvs-fmt/src/indent.rs:1`
- [ ] A `switch` body and a template region's text are placed by the rule that names them. Where no rule names one, the rule is written first, and an open question goes to the user. `crates/nvs-fmt/src/indent.rs:1`
- [ ] A continuation line that is not part of a broken list, chain or operator chain stays the author's, and `indent.rs`'s module doc says so (`rule:tooling/fmt-never-reflows`).
- [ ] Both records are deleted.

### 5. PER's blank lines are written, and the printer's record is closed

**Does:** the blank lines `rule:tooling/fmt-base-style-is-per` names are written by the printer,
not kept from the author.

- [ ] One blank line after `namespace`, one after the `use` block, and one between two members, with a member's comment kept above it. `crates/nvs-fmt/src/print.rs:1`
- [ ] The crate's module doc lists every module that writes a rule, `list.rs` and `chain.rs` included. The record names neither today. `crates/nvs-fmt/src/lib.rs:1`, `data/gaps/nvs-fmt/rules-written-into-the-printer-are.json:1`
- [ ] The record is deleted, and `bun nv owners --closes formatter-complete` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written.

### Next

Stage 1, the CLI side, on `crates/nvs-cli/src/fmt.rs`, `crates/nvs-cli/src/main.rs` and
`crates/nvs-cli/tests/fmt.rs`:
- exit `2` for a refused file (`rule:tooling/fmt-exit-status-says-what-it-found`). `crates/nvs-cli/src/fmt.rs:93`
- the walk skips `vendor/` (`rule:tooling/fmt-check-writes-nothing`). `crates/nvs-cli/src/fmt.rs:128`
- `--check` with `--stdin`. `crates/nvs-cli/src/main.rs:819`

### Backlog

- Formatting an html template's markup is `rule:ide/a-template-region-gets-the-editors-services-and-formatter`'s, not this goal's (`docs/rules/ide/a-template-region-gets-the-editors-services-and-formatter.md`).
