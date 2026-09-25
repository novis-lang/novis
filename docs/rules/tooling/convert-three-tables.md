Three tables, three homes, no fourth copy.

1. **Names → `docs/spec/02-php-migration.md`**, one row per PHP built-in, already CI-checked by
   `bun nv migration`. The converter's name mapping is *generated* from it: a row whose Novis
   cell is exactly one `Core` member spelling is machine-read as a mechanical rename; any other cell
   must carry a rule id, because prose like "`Core\Str::format` into `$file->write`" is a rewrite,
   not a rename. A `dropped` row with neither is a checker error once the converter exists.
2. **Constructs → `crates/nvs-convert/rules/*.toml`**, one file per PHP domain, one record per rule
   (`rule:tooling/convert-one-table-two-modes`'s field list). It is data, not code, so a rule can be
   reviewed by someone who does not read Rust; the browsable copy under `docs/spec/` is generated and
   CI-checked identical, never hand-edited — the discipline `rule:testing/attribution-is-diffed-in-ci`
   applies to notices.
3. **Semantic deltas → the decision that created each one.** A rule's `when` predicates are drawn
   from a **closed vocabulary** — the inferred type of an operand, its qualifier, a literal's shape,
   the dialect, whether a name resolves — and its `diverges` sentence cites the record by number
   rather than restating its reasoning.

The table grows for years. That is the accepted price, and it is why the growth is one data row
rather than one branch in a match.
