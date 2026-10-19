---
looper: 1
title: "`nvs fmt` has no open gap"
summary: "Every gap record under data/gaps/nvs-fmt/ is closed and deleted, nvs fmt's exit status says what it found, and the checks below pass."
context: [crates/nvs-fmt/src/lib.rs, docs/rules/tooling/fmt-base-style-is-per.md, docs/rules/tooling/fmt-is-idempotent.md]
stages:
  - id: 1
    title: "the exit status says what `nvs fmt` found, and the walk leaves `vendor/` alone"
    status: open
    context: [crates/nvs-cli/src/fmt.rs, crates/nvs-cli/src/check.rs, crates/nvs-cli/tests/fmt.rs, tools/nv/cmd/scaling.ts, docs/rules/tooling/fmt-exit-status-says-what-it-found.md, docs/rules/tooling/fmt-check-writes-nothing.md]
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
    title: "a shape type is a node, so its fields are formatted as a list"
    status: open
    context: [crates/nvs-syntax/src/walk.rs, crates/nvs-fmt/src/list.rs, crates/nvs-cli/tests/ast.rs, crates/nvs-fmt/tests/list_layout.rs, data/gaps/nvs-fmt/comma-separated-list-whose-closing-delimiter.json]
    extra:
      context:
        rules: [tooling/fmt-a-list-is-one-line-or-one-item-per-line, tooling/fmt-trailing-commas, ide/ast-json-schema-is-frozen]
  - id: 6
    title: "an attribute is a node, so a literal inside it is formatted"
    status: open
    context: [crates/nvs-syntax/src/walk.rs, crates/nvs-fmt/src/tokens.rs, crates/nvs-fmt/src/space.rs, crates/nvs-cli/tests/ast.rs, data/gaps/nvs-fmt/literal-written-inside-an-attribute-keeps.json]
    extra:
      context:
        rules: [tooling/fmt-quotes, ide/ast-json-schema-is-frozen]
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

**Exit `1` means a file would change, and exit `2` means a file was refused.** As Prettier and Black
do it. Today both exit `1` (`crates/nvs-cli/src/fmt.rs:145`, `crates/nvs-cli/src/fmt.rs:171`).
`rule:tooling/fmt-exit-status-says-what-it-found` holds it. `bun nv scaling`'s fmt ladder already
fails only on a code above `1` (`tools/nv/cmd/scaling.ts:567`), and its doc comment is rewritten to
the new meaning.

**A new node in `crates/nvs-syntax/src/walk.rs` changes `nvs ast --json`'s frozen document.**
Stages 3 and 6 each add a node, so the frozen example in `crates/nvs-cli/tests/ast.rs:184` is
re-recorded in the same commit, and `rule:ide/ast-json-schema-is-frozen` is read first: a new `kind`
value is an addition the schema allows, and a reshaped node is not.

**The closing brace waits for one statement per line.** A `}` that shares a line with the
statement before it can only move once the statement before it is on a line of its own, so stage 2
does that first.

**A line no rule places yet.** A `switch` follows PER-CS: `case` one level in, its statements one
level further, because PER is the base style (`rule:tooling/fmt-base-style-is-per`). A template
region's own text stays the author's, because that text is what the program prints, and a formatter
that changed it would change the program's output. Stage 4 writes both into the rule fragment before
the code.

**The repository's own `.nvs` files are not reformatted in bulk.** Nothing runs `nvs fmt --check`
over the tree. `bun nv verify`'s `nvs-fmt` step formats only the `.nvs` files under `tests/` and `examples/`
that the working tree changed (`tools/nv/cmd/verify.ts:162`), and skips `tests/fmt/input/`. So a
stage that changes output (2, 3, 4, 5, 6) reformats nothing else. A later session that edits one of
those files gets that one file reformatted, and its expectations are checked in that session's verify.
A bulk pass would move line numbers that `.out` and `.nvst` expectations pin, for no reader's gain.

**Lanes.** Stage 1 is in `nvs-cli` and `tools/nv/cmd/scaling.ts`, and shares no file with stages 2
to 6, so it can run first, last, or as a side goal. Stages 3 and 6 share `crates/nvs-syntax/src/walk.rs`
and `crates/nvs-cli/tests/ast.rs`, so 6 follows 3. Neither shares a file with 2 or 5. Stage 4 follows
stage 2, because both edit `crates/nvs-fmt/src/indent.rs`. Stage 5 runs last, because its module doc
lists every module the other stages leave writing a rule. No stage waits on another goal.

**Trade-offs.** No program's behaviour changes, and nothing runs on the request path. `nvs fmt`
output changes for files that differ from the rules, which is the point. Two more node kinds make
the syntax walk a little larger and every formatter rule simpler, because a shape type's `{` and an
attribute's arguments no longer need a second scan. The exit status split costs nothing and makes
a hook simpler to write.

## Stages

### 1. the exit status says what `nvs fmt` found, and the walk leaves `vendor/` alone

**Does:** `nvs fmt` exits `0`, `1` or `2` by what it found, a directory walk skips `vendor/`, and
`--check` works with `--stdin`.

- [ ] A refused or unreadable file exits `2`, and a file that would change exits `1`. `2` wins when both happen (`rule:tooling/fmt-exit-status-says-what-it-found`). `crates/nvs-cli/src/fmt.rs:129`, `crates/nvs-cli/src/fmt.rs:154`
- [ ] The walk skips a directory named `vendor`, as `nvs check --fix` leaves vendored code alone. A path named on the command line is still formatted. `crates/nvs-cli/src/fmt.rs:177`, `crates/nvs-cli/src/check.rs:215`
- [ ] `--check` with `--stdin` writes nothing and exits by the same codes. `crates/nvs-cli/src/fmt.rs:95`
- [ ] A test for each of the three. `crates/nvs-cli/tests/fmt.rs:78`
- [ ] `rule:tooling/fmt-check-writes-nothing` says the walk skips `vendor/`. `docs/rules/tooling/fmt-check-writes-nothing.md:1`
- [ ] The fmt ladder's doc comment says `1` is a file that would change and `2` a refusal. `tools/nv/cmd/scaling.ts:561`
- [ ] `rule:tooling/fmt-exit-status-says-what-it-found` is `shipped`. `data/rules/tooling/fmt-exit-status-says-what-it-found.json:3`

### 2. one statement per line, a closing brace on its own line, and `elseif` as one word

**Does:** the brace paragraph of `rule:tooling/fmt-base-style-is-per` is fully applied.

- [ ] Two statements on one line are split onto two lines. `crates/nvs-fmt/src/indent.rs:116`
- [ ] A `}` that shares a line with the statement before it moves onto a line of its own. `crates/nvs-fmt/src/brace.rs:87`
- [ ] `else if` is written `elseif`. The parser reads both into one arm, so the tree does not change. `crates/nvs-syntax/src/parser/stmt.rs:411`, `crates/nvs-fmt/src/tokens.rs:163`
- [ ] A base-style test for each, and the corpus idempotence test covers the new output (`rule:tooling/fmt-is-idempotent`). `crates/nvs-fmt/tests/base_style.rs:1`, `crates/nvs-fmt/tests/corpus.rs:52`
- [ ] Both records are deleted. `data/gaps/nvs-fmt/closing-brace-is-moved-onto-a.json:1`, `data/gaps/nvs-fmt/else-if-is-kept-as-two-words.json:1`

### 3. a shape type is a node, so its fields are formatted as a list

**Does:** a shape type's fields follow the list rule and the trailing comma rule.

**Split:** at stage start, an `Explore` subagent lists every consumer that matches on the walk's
node kinds (`Node::kind`, the kind list at `crates/nvs-syntax/src/walk.rs:148`) outside `nvs-fmt`:
the LSP, the checker, `nvs ast`. It returns `file.rs:NN` and one line each. That list is what stages 3
and 6 read against a new kind.

- [ ] A node for a type, a shape type among them, in the syntax walk, and each consumer on the Explore list still matches. `crates/nvs-syntax/src/walk.rs:148`
- [ ] The frozen `ast --json` example is re-recorded (`rule:ide/ast-json-schema-is-frozen`). `crates/nvs-cli/tests/ast.rs:184`
- [ ] `list.rs` lays out a shape type's fields as a list (`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`, `rule:tooling/fmt-trailing-commas`), with a test. `crates/nvs-fmt/src/list.rs:94`, `crates/nvs-fmt/tests/list_layout.rs:1`
- [ ] The record is deleted. `data/gaps/nvs-fmt/comma-separated-list-whose-closing-delimiter.json:1`
- [ ] `rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line` is `shipped`. `data/rules/tooling/fmt-a-list-is-one-line-or-one-item-per-line.json:3`

### 6. an attribute is a node, so a literal inside it is formatted

**Does:** a literal inside an attribute takes the quote every other literal takes.

- [ ] A node for an attribute and its arguments in the syntax walk, each consumer on stage 3's Explore list still matching, and the frozen `ast --json` example re-recorded (`rule:ide/ast-json-schema-is-frozen`). `crates/nvs-syntax/src/walk.rs:148`, `crates/nvs-cli/tests/ast.rs:184`
- [ ] `tokens.rs` respells a literal inside it (`rule:tooling/fmt-quotes`), with a test. `crates/nvs-fmt/src/tokens.rs:220`
- [ ] The record is deleted. `data/gaps/nvs-fmt/literal-written-inside-an-attribute-keeps.json:1`

### 4. a comment moves with its import, and every line is placed

**Does:** the `use` block sorts across a comment, and each line the tree does not place today gets
a rule.

- [ ] An import carries the comment written above it as a second span, and the whole block sorts (`rule:tooling/fmt-sorts-the-use-block`), with a test. `crates/nvs-fmt/src/imports.rs:112`, `crates/nvs-fmt/tests/use_block.rs:1`
- [ ] A comment line is placed at the indentation of the line below it (`rule:tooling/fmt-base-style-is-per`). `crates/nvs-fmt/src/indent.rs:270`
- [ ] A `switch` body is placed as PER-CS places it, and a template region's text stays the author's, each written into `rule:tooling/fmt-base-style-is-per` first. `crates/nvs-fmt/src/indent.rs:107`, `docs/rules/tooling/fmt-base-style-is-per.md:1`
- [ ] A continuation line that is not part of a broken list, chain or operator chain stays the author's, and `indent.rs`'s module doc says so (`rule:tooling/fmt-never-reflows`). `crates/nvs-fmt/src/indent.rs:29`
- [ ] Both records are deleted. `data/gaps/nvs-fmt/comment-written-between-two-imports-holds.json:1`, `data/gaps/nvs-fmt/line-the-tree-does-not-place.json:1`

### 5. PER's blank lines are written, and the printer's record is closed

**Does:** the blank lines `rule:tooling/fmt-base-style-is-per` names are written by the printer,
not kept from the author.

- [ ] One blank line after `namespace`, one after the `use` block, and one between two members, with a member's comment kept above it. `crates/nvs-fmt/src/print.rs:137`
- [ ] The crate's module doc lists every module that writes a rule, `list.rs` and `chain.rs` included. The record names neither today. `crates/nvs-fmt/src/lib.rs:21`
- [ ] The record is deleted. `data/gaps/nvs-fmt/rules-written-into-the-printer-are.json:1`

## Handoff

### State

No stage has started. The anchors were checked against the tree when the goal was written.

### Next

**Stage 1: the CLI side**, one file set: `crates/nvs-cli/src/fmt.rs`, `crates/nvs-cli/tests/fmt.rs`,
`tools/nv/cmd/scaling.ts`. No gate.

- [ ] Exit `2` for a refused file, `1` for one that would change (`rule:tooling/fmt-exit-status-says-what-it-found`). `crates/nvs-cli/src/fmt.rs:129`, `crates/nvs-cli/src/fmt.rs:154`
- [ ] The walk skips `vendor/` (`rule:tooling/fmt-check-writes-nothing`). `crates/nvs-cli/src/fmt.rs:177`, `crates/nvs-cli/src/check.rs:215`
- [ ] `--check` with `--stdin`. `crates/nvs-cli/src/fmt.rs:95`
- [ ] A test for each of the three. `crates/nvs-cli/tests/fmt.rs:78`
- [ ] The rule fragment names the `vendor/` skip. `docs/rules/tooling/fmt-check-writes-nothing.md:1`
- [ ] The fmt ladder's doc comment states the new codes. `tools/nv/cmd/scaling.ts:561`
- [ ] `rule:tooling/fmt-exit-status-says-what-it-found` is `shipped`. `data/rules/tooling/fmt-exit-status-says-what-it-found.json:3`

### Backlog

- Formatting an html template's markup is `rule:ide/a-template-region-gets-the-editors-services-and-formatter`'s, not this goal's (`docs/rules/ide/a-template-region-gets-the-editors-services-and-formatter.md`).
