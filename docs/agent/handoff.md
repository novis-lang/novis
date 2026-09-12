# Handoff

## State

**Goal `fmt` (M10), stage 5: three of the four constructs have their layout.** A `tainted` or `secret`
qualifier sits one space before the type it qualifies, a one-line object literal has one space inside
each brace, and an `fn` closure's body brace stays on the line its signature and `=>` were written on.
The two spacing rules are `crates/nvs-fmt/src/space.rs`; the closure is one kind added to
`brace.rs`'s `CARRIES_ITS_BRACES`, because an `fn` body is spliced into the closure and has no `Block`
of its own.

**What a whitespace run must be now has one home.** `crates/nvs-fmt/src/print.rs`'s `Runs` holds the
`(offset, run)` pairs every stage contributes and answers both questions the printer asks of them —
which run to write over, and which has no run to write over — so `brace.rs` and `space.rs` each hand in
pairs and neither owns the lookup. A `Rewrite` still cannot cover a trivium, which is why a spacing
rule is a `Runs` pair and not one of those.

The fragment's own examples were corrected against the grammar (see the playbook bullet), and
`examples/` was reformatted for the object literal's braces. The `match` arm list is the one item left
in the stage, and nothing is blocked.

## Next group

**Stage 5: the last construct PER never saw** — one file set: `crates/nvs-fmt/src/indent.rs`,
`crates/nvs-fmt/src/tokens.rs` and `crates/nvs-fmt/tests/novis_constructs.rs`.

- [ ] **A `match` arm list written across lines is one arm per line, each at one level in from the
      `match`** — `rule:tooling/fmt-novis-constructs`. An arm is a line `crates/nvs-fmt/src/indent.rs:101`'s
      `of_line` answers `None` for today, which is why the crate's known-gap 2 names it: the depth comes
      from the body kinds at `crates/nvs-fmt/src/indent.rs:63`, and `Match` is not a brace-delimited body
      but an expression whose arms are its children. The trailing comma half is already landed —
      `Match` is in `crates/nvs-fmt/src/tokens.rs:111`'s `LISTS` — so what this adds is the arm's own
      line and its depth, and the test is `a_match_arm_list_written_across_lines_is_one_arm_per_line`
      in `crates/nvs-fmt/tests/novis_constructs.rs:1`. A one-line arm list stays one line
      (`rule:tooling/fmt-never-reflows`).
- [ ] **The crate's known gaps lose what this landed** — `crates/nvs-fmt/src/lib.rs:71`'s gap 2 names a
      `match` arm list among the lines the tree does not place, and the arm rule is what deletes that
      clause. Gap 1's list of what each module decides gains the arm line beside `indent.rs`.

## Backlog

- Stage 6 is the command: `nvs fmt` in `nvs-cli`, with `--check`, `--diff` and `--stdin`
  (`docs/agent/loop-goal.toml`'s `stage = "6 the command"`).
- A shape type's braces get no space and a shape's fields no trailing comma, because a type is no node
  in `crates/nvs-syntax/src/walk.rs` — `crates/nvs-fmt/src/lib.rs`'s known gap 5.
- An attribute's object literal is outside every node in that walk, so neither its quotes nor its braces
  are formatted — the same crate doc's gap 4.
- The space after `fn` is nobody's rule: the corpus writes `fn(`, and no test names it.
