# Handoff

## State

Goal `core-html-and-1-more` is under way. `Core\Html::escape` and `Core\Html::join` own every feature
proof: an `about.md`, three examples, one attack, one bench and a Rust `#[test]` carrying a `covers:`
marker in `crates/nvs-stdlib/src/html.rs`'s `mod tests`. Neither proof found a bug. `Core\Html::parse`
is the one member left in this goal's group.

## Next group

One slice is one feature with all its feature proofs.

- [ ] **`Core\Html::parse`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`,
      `rule:core-classes/html-parsing`. `crates/nvs-stdlib/src/html.rs:191`. Its answer is a
      `Core\Xml` tree, so an example reads that tree's members; `docs/examples/core/Html/escape/`
      and `.../join/` are this goal's landed shape.

## Backlog

- A string that is not a source literal has no way to become `Markup` except `Core\Html::escape`,
  so an example that adds a number to a row escapes it (`docs/examples/core/Html/join/03-rows-of-a-table.nvs`).
  Nothing to fix; owned by `rule:core-classes/html-auto-escape`.
