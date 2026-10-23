---
looper: 1
title: "`nvs fmt` has no open gap"
summary: "Every gap record under data/gaps/nvs-fmt/ is closed and deleted, nvs fmt's exit status says what it found, and the checks below pass."
context: [crates/nvs-fmt/src/lib.rs, docs/rules/tooling/fmt-base-style-is-per.md, docs/rules/tooling/fmt-is-idempotent.md]
stages:
  - id: 1
    title: "the exit status says what `nvs fmt` found, and the walk leaves `vendor/` alone"
    status: done
    context: [crates/nvs-cli/src/fmt.rs, crates/nvs-cli/src/check.rs, crates/nvs-cli/tests/fmt.rs, tools/nv/cmd/scaling.ts, docs/rules/tooling/fmt-exit-status-says-what-it-found.md, docs/rules/tooling/fmt-check-writes-nothing.md]
    extra:
      context:
        rules: [tooling/fmt-exit-status-says-what-it-found, tooling/fmt-check-writes-nothing]
  - id: 2
    title: "one statement per line, a closing brace on its own line, and `elseif` as one word"
    status: done
    context: [crates/nvs-fmt/src/brace.rs, crates/nvs-fmt/src/indent.rs, crates/nvs-fmt/tests/base_style.rs, crates/nvs-syntax/src/parser/stmt.rs, data/gaps/nvs-fmt/closing-brace-is-moved-onto-a.json, data/gaps/nvs-fmt/else-if-is-kept-as-two-words.json]
    extra:
      context:
        rules: [tooling/fmt-base-style-is-per, tooling/fmt-is-idempotent]
  - id: 3
    title: "a shape type is a node, so its fields are formatted as a list"
    status: done
    context: [crates/nvs-syntax/src/walk.rs, crates/nvs-fmt/src/list.rs, crates/nvs-cli/tests/ast.rs, crates/nvs-fmt/tests/list_layout.rs]
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
  - id: 7
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
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

**A stage that changes output reformats the corpus in the same commit.**
`crates/nvs-fmt/tests/identity.rs` holds every `.nvs` file under `tests/` and `examples/` to the
formatter's output, `tests/fmt/input/` aside, so a rule the corpus does not follow is red there.
`NVS_FMT_ACCEPT=1 cargo test --test identity` writes the files back. Before accepting, list the
changed files that have an expectation file beside them (`.out`, `.nvst`, `.nvsr`) and check that
none pins a line number the reformat moves. Stage 2 moved 57 files, all but one under
`tests/hostile/`, and no expectation beside them names a line.

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

- [x] A refused or unreadable file exits `2`, and a file that would change exits `1`. `2` wins when both happen (`rule:tooling/fmt-exit-status-says-what-it-found`). `crates/nvs-cli/src/fmt.rs:129`, `crates/nvs-cli/src/fmt.rs:154`
- [x] The walk skips a directory named `vendor`, as `nvs check --fix` leaves vendored code alone. A path named on the command line is still formatted. `crates/nvs-cli/src/fmt.rs:177`, `crates/nvs-cli/src/check.rs:215`
- [x] `--check` with `--stdin` writes nothing and exits by the same codes. `crates/nvs-cli/src/fmt.rs:95`
- [x] A test for each of the three. `crates/nvs-cli/tests/fmt.rs:78`
- [x] `rule:tooling/fmt-check-writes-nothing` says the walk skips `vendor/`. `docs/rules/tooling/fmt-check-writes-nothing.md:1`
- [x] The fmt ladder's doc comment says `1` is a file that would change and `2` a refusal. `tools/nv/cmd/scaling.ts:561`
- [x] `rule:tooling/fmt-exit-status-says-what-it-found` is `shipped`. `data/rules/tooling/fmt-exit-status-says-what-it-found.json:3`

### 2. one statement per line, a closing brace on its own line, and `elseif` as one word

**Does:** the brace paragraph of `rule:tooling/fmt-base-style-is-per` is fully applied.

- [x] Two statements on one line are split onto two lines. `crates/nvs-fmt/src/indent.rs:116`
- [x] A `}` that shares a line with the statement before it moves onto a line of its own. `crates/nvs-fmt/src/brace.rs:87`
- [x] `else if` is written `elseif`. The parser reads both into one arm, so the tree does not change. `crates/nvs-syntax/src/parser/stmt.rs:411`, `crates/nvs-fmt/src/tokens.rs:163`
- [x] A base-style test for each, and the corpus idempotence test covers the new output (`rule:tooling/fmt-is-idempotent`). `crates/nvs-fmt/tests/base_style.rs:1`, `crates/nvs-fmt/tests/corpus.rs:52`
- [x] Both records are deleted. `data/gaps/nvs-fmt/closing-brace-is-moved-onto-a.json:1`, `data/gaps/nvs-fmt/else-if-is-kept-as-two-words.json:1`

### 3. a shape type is a node, so its fields are formatted as a list

**Does:** a shape type's fields follow the list rule and the trailing comma rule.

**Split:** at stage start, an `Explore` subagent lists every consumer that matches on the walk's
node kinds (`Node::kind`, the kind list at `crates/nvs-syntax/src/walk.rs:148`) outside `nvs-fmt`:
the LSP, the checker, `nvs ast`. It returns `file.rs:NN` and one line each. That list is what stages 3
and 6 read against a new kind.

- [x] A node for a type, a shape type among them, in the syntax walk, and each consumer on the Explore list still matches. `crates/nvs-syntax/src/walk.rs:148`
- [x] The frozen `ast --json` example is re-recorded (`rule:ide/ast-json-schema-is-frozen`). `crates/nvs-cli/tests/ast.rs:184`
- [x] `list.rs` lays out a shape type's fields as a list (`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`, `rule:tooling/fmt-trailing-commas`), with a test. `crates/nvs-fmt/src/list.rs:94`, `crates/nvs-fmt/tests/list_layout.rs:1`
- [x] The record is deleted. `data/gaps/nvs-fmt/comma-separated-list-whose-closing-delimiter.json:1`
- [x] `rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line` is `shipped`. `data/rules/tooling/fmt-a-list-is-one-line-or-one-item-per-line.json:3`

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

### 7. the goal is verified once, and every failure is fixed

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

Stages 1, 2 and 3 are done. Stage 3 is 0b7e77765. The syntax walk writes a `ShapeType` node
wherever a shape type is written, inside another type too, and no other type is a node. So a file
with no shape walks as before. `push_type` is the one place (`crates/nvs-syntax/src/walk.rs:1111`).
`Core\Ast`'s `productions!` gains `ShapeType` at the same sorted place. The redaction walk skips the
node in front of a property's initializer (`crates/nvs-types/src/redaction.rs:199`). `list.rs` reads
a shape's commas by depth, as it reads a parameter list's (`bracketed`). `space.rs` gives a one-line
shape one space inside each brace (`rule:tooling/fmt-novis-constructs`). The corpus was reformatted:
36 files, spacing only, and no line moved. `nvs ast --json`'s frozen document is `examples/hello.nvs`,
which has no shape, so it is unchanged and the schema check over every example passes.

Explore's consumer list for stages 3 and 6: the kind tables are `KINDS` and `productions!`. Position
readers are `redaction.rs`'s `property` (first child is the initializer) and
`nvs-types/src/deprecated/fix.rs:947` (a child's rank in `If`, `Switch`, `Foreach`). Other position
readers are `nvs-lsp/src/arguments.rs:524`, `hover.rs:247` and `completion.rs:1322` (call children
after the `(`), and `nvs-lsp/src/index.rs:1266` (an enum case's first child). Counting `.nvst` cases
are under `tests/conformance/core/` (`core-ast-*`), and `.out` files under `docs/examples/core/Ast*`.

The acceptance sweep's reds are unchanged: the `env-secret-proofs` log shows exit 0, and no nightly
has run since 39922764699. The last handoff's triage stands. If the next nightly's macOS
`Serve soak` is red again, take that slice first.

Expected red at the gate: nothing known. `list_layout`, `novis_constructs`, `identity` and `ast`
passed after this change. An LSP completion case whose cursor is inside a shape type now has
`ShapeType` as its innermost node, and no case in `tests/lsp/` writes one.

### Next

**Stage 6: an attribute is a node, so a literal inside it is formatted.** Files:
`crates/nvs-syntax/src/walk.rs`, `crates/nvs-fmt/src/tokens.rs`, `crates/nvs-fmt/src/space.rs`,
`crates/nvs-cli/tests/ast.rs`. The walk's module doc says why an attribute's payload is no node
today: a payload node under a method moves what `nvs fmt`'s indenter and `Core\Ast` print
(`crates/nvs-syntax/src/walk.rs:31`). Read that first, and rewrite it with the change. Read the
consumer list above against the new kind. Add the kind to `productions!` at the same sorted place.
Run `cargo test --test identity` and accept the corpus as stages 2 and 3 did. `examples/` has
attributes, so the frozen-schema check in `cargo test --test ast` is the one to run.

- [ ] A node for an attribute and its arguments in the syntax walk, each consumer on the list above still matching, and the frozen `ast --json` example re-recorded if it changes. `crates/nvs-syntax/src/walk.rs:31`, `crates/nvs-cli/tests/ast.rs:184`
- [ ] `tokens.rs` respells a literal inside it (`rule:tooling/fmt-quotes`), with a test. `crates/nvs-fmt/src/tokens.rs:227`
- [ ] The record is deleted. `data/gaps/nvs-fmt/literal-written-inside-an-attribute-keeps.json:1`

### Backlog

- Formatting an html template's markup is `rule:ide/a-template-region-gets-the-editors-services-and-formatter`'s, not this goal's (`docs/rules/ide/a-template-region-gets-the-editors-services-and-formatter.md`).
