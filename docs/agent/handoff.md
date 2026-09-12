# Handoff

## State

**Goal `fmt` (M10), stage 4 has its first name green.** `crates/nvs-fmt/src/tokens.rs` is the home of
the rules that rewrite a token rather than the whitespace around it, and what is in it is the quote
rule: a double-quoted literal goes out single-quoted when both spellings name the same string, which
is a body with no `'` and no `\` in it. `rule:tooling/fmt-quotes`'s fragment now records that reading
of its "would force to be escaped" clause — the two quotings do not share an escape grammar, so
respelling `"one\ttwo"` would change the string's value.

The scan is `crates/nvs-fmt/src/brace.rs`'s, over the code runs the printer tiles: a `"` is an
opening quote only when the innermost node at it is a `Str` that starts there, so a heredoc body, an
interpolation site and a comment are never candidates. A literal an attribute writes is not one
either — that is gap 4 in `crates/nvs-fmt/src/lib.rs`.

**The corpus absorbed it**: 99 `.nvs` files, 743 lines, each a `"` pair swapped for a `'` pair and
nothing else. `NVS_FMT_ACCEPT=1 cargo test -p nvs-fmt --test identity` is how that edit is made now;
the accepting run still fails, so the diff is read before it is committed. Nothing is blocked.

## Next group

**Stage 4: the token rules, continued** — one file set: `crates/nvs-fmt/src/tokens.rs`,
`crates/nvs-fmt/src/print.rs` and `crates/nvs-fmt/tests/token_rules.rs`. Each of these rewrites a
code run, so `crates/nvs-fmt/src/print.rs:38`'s `Rewrite` is the shape and
`crates/nvs-fmt/src/tokens.rs:71` is the scan that keeps a literal's own bytes out of it.

- [ ] **The trailing comma** — `rule:tooling/fmt-trailing-commas`. A comma-separated list the author
      spread over more than one line gets a comma after its last element and a one-line list never
      does; whether it spans lines is the author's, `rule:tooling/fmt-never-reflows`. An insertion is
      a `Rewrite` with `start == end` (`crates/nvs-fmt/src/print.rs:38`). The index carries no
      element spans, so what a closer's own node gives you is the list, and the last element ends at
      the last non-trivia byte before the closing `)`, `]` or `}` — `crates/nvs-fmt/src/tokens.rs:71`
      is the walk. Test `a_multi_line_list_gains_a_trailing_comma_and_a_one_line_list_has_none`.
- [ ] **The `use` block's order** — `rule:tooling/fmt-sorts-the-use-block`. A run of `UseDecl` nodes
      with only trivia between them is a permutation of itself, which is exactly
      `crates/nvs-fmt/src/modifiers.rs:43`'s shape: each declaration's text goes out at one of the
      run's own spans, so the blank lines and comments between them stay where they were written —
      and a comment naming the import above it therefore ends up over a different one, which is the
      thing to decide and then write into the crate's gaps. Find them as
      `crates/nvs-fmt/src/tokens.rs:71` finds a literal, and sort by the path
      (`crates/nvs-syntax/src/ast.rs:1766`). Test `consecutive_use_declarations_are_sorted_by_full_path`.
- [ ] **The two negatives** — `rule:tooling/fmt-never-inserts-visibility` and
      `rule:tooling/fmt-never-reorders-members`. Two tests over what is already true, beside the
      quote rule's own at `crates/nvs-fmt/tests/token_rules.rs:20`: a member with no visibility keyword comes back
      with none, and a class whose members are in no particular order comes back in that same order.
      Tests `no_visibility_keyword_is_ever_inserted` and `class_members_keep_their_declaration_order`.
      The reserved-spelling half of that stage's check is the playbook bullet above: decide the
      refusal question first, because `<?NVS` and `5Min` do not parse.

## Backlog

- The `nvs fmt` command itself is unlanded — `nvs fmt` is still `unrecognized subcommand`, so every
  stage's evidence is a `cargo test` (`docs/agent/loop-goal.md`, stage 6).
- PER's blank lines, and the constructs PER never saw (`rule:tooling/fmt-novis-constructs`) —
  gap 1 in `crates/nvs-fmt/src/lib.rs`.
- A one-line body comes back as a header, a line and a `{ … }` until one-statement-per-line lands —
  gap 3 in `crates/nvs-fmt/src/lib.rs`.
