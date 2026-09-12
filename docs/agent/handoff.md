# Handoff

## State

**Goal `fmt` (M10), stage 3 is closed.** All four of its names pass, the last being
`a_declaration_brace_is_allman_and_a_control_brace_is_k_and_r`.
`crates/nvs-fmt/src/brace.rs` answers one question — what whitespace belongs immediately before a
byte — and the printer applies it: a declaration's `{` gets a line break and the declaration's own
indentation, a control structure's gets one space, and `elseif`/`else`/`catch`/`finally` get one
space after the brace that closed the clause above. Which body a brace opens is read off
`SyntaxIndex::at`, so a `{` inside a string literal or an attribute's object literal is never one —
its innermost node is an expression, not a declaration.

**`print::Rewrite` is the shared edit type now** (`crates/nvs-fmt/src/print.rs:37`), and an empty
range inserts: that is how a brace with no run in front of it (`class Queue{`) gets its line break.
`modifiers.rs` and `brace.rs` both produce them and the printer sorts them into one stream.

**The corpus is reformatted and stage 2's floor is green** (`crates/nvs-fmt/tests/identity.rs:36`):
42 `.nvs` files moved a brace, and `nvs fmt` is a fixed point of all of them again. Nothing is
blocked.

## Next group

**Stage 4: the token rules** — one file set: `crates/nvs-fmt/src/print.rs`, a new
`crates/nvs-fmt/src/tokens.rs`, and a new `crates/nvs-fmt/tests/token_rules.rs`. Every one of these
edits a code run rather than a whitespace one, so `crates/nvs-fmt/src/print.rs:37`'s `Rewrite` is the
shape, and `crates/nvs-fmt/src/brace.rs:169` — classify a byte by the innermost node at it, over the
code runs the printer already tiles — is the pattern that keeps a literal's bytes out of it.

- [ ] **Quotes, and the two bodies never touched** — `rule:tooling/fmt-quotes`. A plain string goes
      out single-quoted unless it interpolates or holds a `'`; a heredoc body and every comment are
      copied byte for byte. A literal is an expression node, so its span comes off the index the way
      `crates/nvs-fmt/src/brace.rs:116` takes one. Tests
      `a_plain_string_is_rewritten_to_single_quotes` and
      `a_heredoc_body_and_a_comment_are_never_touched`.
- [ ] **The trailing comma** — `rule:tooling/fmt-trailing-commas`. A list the author spread over
      several lines ends with one and a one-line list has none, which is the last element's node end
      plus the bytes up to the closer: `crates/nvs-fmt/src/print.rs:81`. Test
      `a_multi_line_list_gains_a_trailing_comma_and_a_one_line_list_has_none`.
- [ ] **The `use` block's order** — `rule:tooling/fmt-sorts-the-use-block`. Consecutive `UseDecl`
      statements are whole-statement spans, so this is one `Rewrite` per moved line and no reflow:
      `crates/nvs-fmt/src/print.rs:37`. Test `consecutive_use_declarations_are_sorted_by_full_path`.
- [ ] **Reserved spellings, and the two negatives** —
      `rule:tooling/fmt-normalizes-only-reserved-spellings`,
      `rule:tooling/fmt-never-inserts-visibility`, `rule:tooling/fmt-never-reorders-members`. The
      open tag and a duration unit are lower-cased; the other two are tests over what the printer
      already does and belong beside the base style's, at
      `crates/nvs-fmt/tests/base_style.rs:94`.

## Backlog

- A `}` sharing a line with the statement before it stays there — gap 3, `crates/nvs-fmt/src/lib.rs`.
- A `do { } while` keeps the author's break before `while`; the rule names only
  `elseif`/`else`/`catch`/`finally` (`rule:tooling/fmt-base-style-is-per`).
- A closure's, an anonymous class's and a property hook's brace are left alone on purpose —
  `crates/nvs-fmt/src/brace.rs`'s module doc says why.
- Nothing in `crates/nvs-cli` depends on `nvs-fmt` yet, so reformatting the corpus needs a throwaway
  test rather than a command — stage 6 of the goal owns the leg.
- PER's blank lines are still the author's — gap 1, `crates/nvs-fmt/src/lib.rs`.
