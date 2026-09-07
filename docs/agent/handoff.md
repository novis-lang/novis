# Handoff

## State

**Goal 12 — the resilient tree — has stages 2, 3, 4 and 5 landed and green.** Recovery is explicit
(`rule:ide/recovery-is-explicit`), the `SyntaxIndex` is the walk flattened, `crates/nvs-syntax/tests/prefixes.rs`
cuts every `examples/*.nvs` at every token boundary and asks all three of stage 4's questions of each cut, and
`fuzz/fuzz_targets/prefix.rs` — a `[[bin]]`, a third leg of the nightly `fuzz-smoke` matrix, seeds in
`fuzz/seeds/prefix/` because `.gitignore` eats `fuzz/corpus/` — parses both halves of a cut document.

**Stage 5's position arithmetic is on disk in one file**, `crates/nvs-diagnostics/src/source.rs`:
`SourceFile::utf16_col`, `SourceFile::offset_of(line, col, encoding)` and the `PositionEncoding` enum,
re-exported from the crate root. `rule:ide/positions-have-one-home` is satisfied and the check's four tests
pass.

**Two design calls, both owned by the doc comments at the site.** `PositionEncoding` carries `Utf32` beside
the `Utf8` and `Utf16` a server negotiates, because `Utf32` is what `line_col` already counts — one inverse
covers every column this crate hands out rather than most of them. And `offset_of` clamps rather than
refusing, because a position arrives from an editor whose buffer can be a keystroke ahead: a line past the
last is the end of the file, a column past its line's content stops *before* the terminator (so a CRLF
document answers what an LF one does), and a column falling inside a character is that character's start.

**No BOM policy lives here.** A BOM is left in the text and counted as one `char` and one code unit, so
every offset after it lands and round-trips; stripping it is the document store's job, which is what
`rule:ide/positions-have-one-home` says and what `nvs-lsp` will own when it exists.

**One residue, deliberate and unchanged:** `ExprKind::ClassConstAccess` carries a bare `Span` for its name,
so `Foo::` at the caret still carries a name nobody wrote. The `::` path routes `Missing` into the same arm
`Ident` takes rather than reporting an invented second diagnostic; the comment at that site says so, and the
Backlog carries the fix.

**Stage 0 is empty and stays empty**, unchanged: the four M4 language holes are closed and `bool as int` is
`E0708` under `rule:types/conversion`, not a hole.

## Next group

**Stage 6's shared section lexer**, in one crate and mostly one file — `crates/nvs-test/src/case.rs`, with
its module declared at `crates/nvs-test/src/lib.rs:161` and its tests in `crates/nvs-test/src/case.rs:563`.
`docs/decisions/0099.md` § 5 is the whole specification ("the section lexer is `nvs_test`'s, extracted to a
shared module so there is one parser for both formats"); the three test names the check demands are in
`docs/agent/loop-goal.toml`'s stage 6 block. The `.nvst` half must not move.

- [ ] **The section lexer becomes its own module, and a header carries an argument** — `struct Section` at
      `crates/nvs-test/src/case.rs:217` and `fn header` at `crates/nvs-test/src/case.rs:266` move out from
      under `pub fn parse` at `crates/nvs-test/src/case.rs:321`, declared beside `pub mod case;` at
      `crates/nvs-test/src/lib.rs:161`. `the_section_lexer_reads_a_header_with_an_argument` pins
      `--FILE <relative/path>--`'s argument, which is the shape `--REQUEST--` needs. `docs/decisions/0099.md` § 5.
- [ ] **A second format reuses it** — `the_section_lexer_is_reusable_by_a_second_format` in the test module at
      `crates/nvs-test/src/case.rs:563` drives the extracted lexer over a `.lspt`-shaped body it invents
      inline, proving nothing in it knows about `--EXPECT--`'s `.nvst` meaning. No `.lspt` runner is written
      here — that is off path per the goal's standing decisions. `docs/decisions/0099.md` § 5.
- [ ] **The `.nvst` half did not move** — `every_nvst_section_parses_exactly_as_it_did_before_the_extraction`,
      same test module, walks the real corpus through `pub fn parse` at `crates/nvs-test/src/case.rs:321` and
      asserts every section of every case still parses to what it did. `rule:testing/hostile-case-contract`
      and `docs/decisions/0099.md` § 5.

## Backlog

- `ExprKind::ClassConstAccess` should carry a `MemberName`, not a `Span` — `rule:ide/recovery-is-explicit`.
- Nothing calls `utf16_col`/`offset_of` yet; `nvs-lsp`'s negotiated encoding is their first caller —
  `rule:ide/positions-have-one-home`.
- Stage 7 is `nvs ast --json --resilient` and its frozen schema — `rule:ide/ast-json-schema-is-frozen`.
- `tools/reference.py --check` is a stage 7 check too, so a chapter edit must regenerate.
