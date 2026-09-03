# Handoff

## State

**Goal 11 — `///` is a doc comment — has just started; nothing of it has landed yet.** Goal 10's whole
list is this goal's Stage 1 floor. The design is settled and is not to be re-derived:
ADR 0137 landed with this goal and its
four decisions were taken with the user — `///` as the marker with `////` staying ordinary, prose plus
exactly `@see` and `@example` with every other `@tag` a diagnostic, `nvs meta --json` as the one
machine-readable source with `nvs doc` a renderer over it, and enforcement silent by default.

Two things about the shape of this goal that are easy to get wrong:

- **Stage 2 is M4B's tree item landing early.** ADR 0099 § 1's `Trivia`/`TriviaKind` do not exist in the
  tree at all, and a doc comment cannot be read without them. Build them *as that section specifies*, so
  M4B inherits them done. The `SyntaxIndex` in the same struct is **M4B's** and has no consumer here.
- **Hover is not in this goal.** It needs `crates/nvs-lsp`, which does not exist. Stage 6's last item
  hands `docs/plan/m4b.md` one line; no hover code lands.

## Next group

**Stage 2: the trivia layer and its fourth variant** — one file set:
`crates/nvs-syntax/src/lexer.rs`, `crates/nvs-syntax/src/token.rs`,
`crates/nvs-syntax/src/parser/mod.rs`.

- [ ] **`Trivia` and `TriviaKind`** — ADR 0099 § 1's shape exactly. `Lexer` gains a flag; `skip_trivia`
      (`crates/nvs-syntax/src/lexer.rs:357`) pushes a `Trivia { kind, span }` instead of only advancing.
      Variants: `Whitespace`, `LineComment`, `BlockComment`, `DocComment`. `TriviaKind` goes beside the
      token types in `crates/nvs-syntax/src/token.rs`.
- [ ] **The run-length rule** — exactly three `/` is `DocComment`, four or more is `LineComment`, `#` is
      never one. The `//// ____` divider at
      `tests/conformance/core/encoding-every-encoder-agrees-with-its-own-decoder-over-a-table.nvst:114`
      is the case that proves it, and the seven files already carrying `///` are the case that proves
      reclassification needs no edit — both are named tests of the TOML's stage 2 checks.
- [ ] **`parse_file` returns `Parsed`** — `crates/nvs-syntax/src/parser/mod.rs:504`. Keep the strict
      entry point as a thin wrapper so no call site changes, which ADR 0099 § 1 requires and which is
      what keeps this slice from touching every crate. Losslessness (`tokens ⊕ trivia` reproduces the
      file) is the acceptance property, over `examples/`, `tests/` and the vendored `php-src` corpus.

## Backlog

- Stage 3 (attachment and the closed tag set — `crates/nvs-syntax/src/parser/decl.rs`,
  `crates/nvs-syntax/src/ast.rs`, plus the new codes in `crates/nvs-diagnostics/src/lib.rs`) shares the
  crate with stage 2 but not its files. Worth taking in the same session if stage 2 lands under 120k:
  the lexer is already loaded and the tag diagnostics are the half that makes the set closed rather
  than advisory.
- Stage 4's two checks live in `nvs-hir` and share nothing with stages 2-3.
- Stages 5 and 6 are both `crates/nvs-cli` and belong together: `meta`'s argument, then `nvs doc` and
  `--strict-docs` over it. The `reference.py --check` command check in stage 5 is what holds the
  no-argument output byte-identical, which is what protects the website's `sync:core` too.
- Take diagnostic codes from `python tools/brief.py` at the moment you write them — parser `E01xx`,
  name resolution `E03xx` — never from the ADR, which names bands deliberately.
- When this goal's last check goes green the driver takes goal 12 — the resilient tree, the first of
  M4B's four entries, which starts with the trivia half this goal built already done.
