# Handoff

## State

**Goal 12 — the resilient tree — has stages 2, 3 and 4 landed and green.** Recovery is explicit
(`rule:ide/recovery-is-explicit`), the `SyntaxIndex` is the walk flattened, `crates/nvs-syntax/tests/prefixes.rs`
cuts every `examples/*.nvs` at every token boundary and asks all three of stage 4's questions of each cut,
and stage 4's remaining fuzz target is now on disk.

**The fuzz target is `fuzz/fuzz_targets/prefix.rs`.** The input's *last* byte says where to cut and
everything before it is the source, so one entry parses both a truncated document and one that starts
mid-construct through `nvs_syntax::parse`, asserting nothing beyond "it returned"; the module doc owns the
format and why the selector is at the end. It is a `[[bin]]` in `fuzz/Cargo.toml` and a third leg of the
nightly `fuzz-smoke` matrix, and five hand-written seeds sit in `fuzz/seeds/prefix/` rather than in
`fuzz/corpus/prefix/`, which `.gitignore` ignores — the playbook bullet owns that trap. It was compiled and
run for real under WSL nightly, not read: `cargo +nightly check` is clean and a `cargo +nightly fuzz run
prefix` over the seeds found no panic.

**The acceptance check that is red is stage 5, and nothing is written for it yet:** `nvs-diagnostics` has
`SourceFile::line_col`, which counts `char`s, and neither `utf16_col` nor `offset_of`. That is the next
group below, and it is one file.

**One residue, deliberate and unchanged:** `ExprKind::ClassConstAccess` carries a bare `Span` for its name,
so `Foo::` at the caret still carries a name nobody wrote. The `::` path routes `Missing` into the same arm
`Ident` takes rather than reporting an invented second diagnostic; the comment at that site says so, and
the Backlog carries the fix.

**Stage 0 is empty and stays empty**, unchanged: the four M4 language holes are closed and `bool as int` is
`E0708` under `rule:types/conversion`, not a hole.

## Next group

**Stage 5's position arithmetic**, all of it in one file — `crates/nvs-diagnostics/src/source.rs`, the
functions beside `line_col` and the four tests in its own `mod tests`. `rule:ide/positions-have-one-home`
is the whole specification and says what each answer must be; the four test names the check demands are in
`docs/agent/loop-goal.toml`'s stage 5 block.

- [ ] **`utf16_col` counts code units, not characters** — beside `line_col` at
      `crates/nvs-diagnostics/src/source.rs:95`, with `utf16_col_counts_code_units_not_chars` in the test
      module at `crates/nvs-diagnostics/src/source.rs:243`. A `ß` is one `char`, one UTF-16 code unit and
      two bytes; an emoji is one `char`, *two* UTF-16 code units and four bytes, which is the case that
      separates this from `line_col`. `rule:ide/positions-have-one-home`.
- [ ] **`offset_of(line, col, encoding)` inverts it** — the same anchor,
      `crates/nvs-diagnostics/src/source.rs:95`, taking the encoding the rule says is negotiated rather
      than assuming one. `offset_of_inverts_line_col_on_a_multibyte_line` and
      `an_offset_past_the_last_line_is_clamped_rather_than_panicking` go at
      `crates/nvs-diagnostics/src/source.rs:243`: a position the client invents past the end is clamped to
      the last offset, because a server that panics on a stale position is a server that dies mid-edit.
      `rule:ide/positions-have-one-home`.
- [ ] **The round trip, and the two documents that break naive arithmetic** —
      `a_position_round_trips_through_utf16_and_utf8` at `crates/nvs-diagnostics/src/source.rs:243`, plus
      the rule's second paragraph on the same anchors: a leading BOM is skipped *and counted*, and CRLF is
      preserved exactly, so a CRLF document's columns match an LF one's. `rule:ide/positions-have-one-home`.

## Backlog

- `ExprKind::ClassConstAccess` should carry a `MemberName`, not a bare `Span` — `rule:ide/recovery-is-explicit`.
- The `.lspt` runner and `nvs lsp-test` — ADR 0099 § 5, and off this goal's path by its own standing decisions.
- `nvs ast --json`'s frozen schema — `rule:ide/ast-json-schema-is-frozen`, named by M4B and not started.
- The latency guard on a 1,000-line re-analysis — `rule:ide/a-full-reanalysis-stays-under-a-bound`, goal 14.
- Seeds for `lex` and `parse` under `fuzz/seeds/<target>/` — the CI step already picks up any target that grows one.
