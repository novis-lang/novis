# Handoff

## State

**Goal `fmt` (M10), stages 4 and 5's first check are green.** A mis-cased reserved spelling is now
the one error `nvs fmt` rewrites rather than refuses: `crates/nvs-fmt/src/lib.rs`'s `format` refuses
a file for an error the token rules do not themselves repair, and `tokens::spellings` lower-cases
the bytes under each `E_RESERVED_SPELLING_CASE` report's primary span. That code now carries both
spellings the rule names — the lexer reports a mis-cased duration unit under it and keeps the
literal's own `DurationLiteral` token, so the tree behind `30S` is whole and one error is reported
rather than a parse cascade. The reasoning is in `rule:tooling/fmt-normalizes-only-reserved-spellings`'s
fragment; the refusal's own criterion is `crates/nvs-fmt/src/lib.rs` § *A file the parse could not
place is refused*.

**The `?>` that leaves code mode is placed like a statement.** `Indent::of_line` answers for a line
opening with a close tag — no node starts at one, so the question is asked of the text — while the
region after it stays the author's. `crates/nvs-fmt/tests/two_modes.rs` holds that and the two
byte-identity claims. Nothing is blocked.

## Next group

**Stage 5: every construct PER never saw has its one layout** — one file set:
`crates/nvs-fmt/src/brace.rs`, `crates/nvs-fmt/src/indent.rs`, `crates/nvs-fmt/src/tokens.rs` and
the crate's tests. All three items are `rule:tooling/fmt-novis-constructs`, and the check that wants
them is the second `stage = "5 the two modes"` block in `docs/agent/loop-goal.toml`.

- [ ] **A qualifier sits one space before its type, and a one-line object literal has one space
      inside each brace** — nothing owns spacing *inside* a code run today:
      `crates/nvs-fmt/src/print.rs:100`'s `push_code` copies a run byte for byte except where a
      `Rewrite` covers it, so both of these are a new producer of those, in the shape of
      `crates/nvs-fmt/src/tokens.rs:98` — the one module that already edits code bytes rather than
      the whitespace around them. Tests `a_qualifier_sits_one_space_before_its_type` and
      `an_object_literal_on_one_line_has_one_space_inside_each_brace`.
- [ ] **A `fn` closure's body brace stays on its signature line** — `crates/nvs-fmt/src/brace.rs:1`
      decides which line an opening brace sits on, in two rows: a declaration's is Allman and a
      control structure's is K&R. A closure is neither, so read that list first; `Fn` is already a
      body in `crates/nvs-fmt/src/indent.rs:60`, so only the brace's line is open. Test
      `a_fn_closure_body_brace_stays_on_its_signature_line`.
- [ ] **A `match` arm list written across lines is one arm per line** — `Match` is in
      `crates/nvs-fmt/src/indent.rs:82`'s `OPAQUE`, so every line inside one keeps the author's
      whitespace today. Landing this is taking it off that list and placing an arm's line from the
      `Match` that holds it, which is the shape the close tag took in `of_line` at
      `crates/nvs-fmt/src/indent.rs:119`. Test
      `a_match_arm_list_written_across_lines_is_one_arm_per_line`.

## Backlog

- Stage 6 is the command itself — `nvs fmt`, `--check`, `--diff`, `--stdin` — and none of it exists
  yet; the check is `docs/agent/loop-goal.toml`'s `stage = "6 the command"`.
- A template region's own text is still unplaced, and a `switch`'s lines with it —
  `crates/nvs-fmt/src/lib.rs` gap 2.
- A parameter list, an enum-case list and a shape type's fields still keep the author's comma —
  `crates/nvs-fmt/src/lib.rs` gap 5, which wants a node boundary in `crates/nvs-syntax/src/walk.rs`.
- A literal inside an attribute keeps its double quotes — the same absence, `lib.rs` gap 4.
