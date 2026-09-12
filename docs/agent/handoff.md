# Handoff

## State

**Goal `fmt` (M10), stage 4 is one name from green.** `crates/nvs-fmt/src/tokens.rs` holds both rules
that rewrite a code byte: the quote a plain literal is delimited by, and the comma a multi-line list
ends its last element on. The comma's operative test is where the **closing delimiter** sits — a line
break between the last element and the closer means the comma is there, a closer that follows its
element on that element's line means it is not — and both directions are applied, so a comma written
in front of a trailing closer is deleted. `rule:tooling/fmt-trailing-commas`'s fragment now records
that reading. Which lists it reaches is the node that ends at the closer, so a parameter list, an
enum-case list and a shape type's fields are gap 5 in `crates/nvs-fmt/src/lib.rs`.

**`crates/nvs-fmt/src/imports.rs` is new** and holds the `use` block's order. What it moves is the
**path**, never the declaration: `print::one_code_run` is the printer's own invariant and
`use Core\Str;` is two code runs either side of the space after the keyword. A comment between two
imports ends the run and each half sorts alone — gap 6 in the same doc.

Six of stage 4's seven test names are green; the seventh is the reserved spellings, below. The corpus
absorbed both rules in three files (`examples/pool.nvs`, `examples/xml-tree.nvs`,
`examples/schema.nvs`). Nothing is blocked.

## Next group

**Stage 4: the reserved spellings, then stage 5's two negatives** — one file set:
`crates/nvs-fmt/src/tokens.rs`, `crates/nvs-syntax/src/lexer.rs`, `crates/nvs-syntax/src/duration.rs`
and the crate's tests. The first item is a decision the goal pre-authorizes; the other two are claims
the printer already satisfies.

- [ ] **The reserved spellings, and the refusal that hides them** —
      `rule:tooling/fmt-normalizes-only-reserved-spellings`. The lexer takes `<?NVS` and reports
      "`<?nvs` must be written in lower case" at `crates/nvs-syntax/src/lexer.rs:384`, and a
      mis-cased duration unit is `MisCasedUnit` at `crates/nvs-syntax/src/duration.rs:49`. Both are
      errors, and `crates/nvs-fmt/src/lib.rs:139` refuses a file whose parse reports one — so the
      formatter never reaches the two spellings the rule names. Decide that under the rule's own
      reasoning and write it into the fragment: either a mis-cased reserved spelling is a diagnostic
      the parse carries without refusing the file, or the rewrite is the editor's code action and
      what `nvs fmt` owes is the refusal. The scan that would hold it is
      `crates/nvs-fmt/src/tokens.rs:123`. Test `a_mis_cased_open_tag_and_duration_unit_are_lower_cased`.
- [ ] **Markup and inline HTML come back byte-identical** — the goal's standing decision that bytes a
      program prints are never touched, and `rule:tooling/fmt-never-reflows`. Two tests over what is
      already true, in the shape of `crates/nvs-fmt/tests/never_rewrites_a_declaration.rs:21`: an
      `?> … <?nvs` region and a markup literal's body survive a format unchanged. Tests
      `an_inline_html_region_is_byte_identical_after_formatting` and
      `a_markup_literal_body_is_byte_identical_after_formatting`.
- [ ] **A close tag is never moved onto or off a line** — the other half of stage 5's first check,
      and the one that needs `crates/nvs-fmt/src/indent.rs:1`'s template-region gap read first: a
      `?>` that begins its line sits at its block's depth, and one that follows a statement stays
      where it is. Tests `a_close_tag_that_begins_its_line_sits_at_the_depth_of_its_block` and
      `a_close_tag_is_never_moved_onto_or_off_a_line`.

## Backlog

- Parameter lists, enum cases and shape fields get no trailing comma — gap 5, `crates/nvs-fmt/src/lib.rs`.
- A comment inside a `use` block pins the run around it — gap 6, same doc.
- A literal inside an attribute keeps its quotes — gap 4, same doc.
- PER's blank lines, the `use` block's own one included — `rule:tooling/fmt-base-style-is-per`, stage 5.
- Stage 6 is the command: in place, `--check`, `--diff`, `--stdin`, and the frozen `tests/fmt/` pairs.
