# Handoff

## State

**Goal `markup-literal`, stage 2 is landed: `` html`…` `` lexes, and the AST node exists.** Stage 1 (goal
`finish-response`'s list) is the untouched floor.

On disk in `crates/nvs-syntax/`: `TokenKind::MarkupOpen`/`MarkupClose`, a `Mode::Markup` frame that is
`Mode::DoubleQuoted` with the closer swapped, the unterminated arm under the existing `E0002`, and
`ExprKind::Markup(Vec<StringPart>)` with the two nvs-syntax walks reaching its holes. Both stage-2
acceptance checks pass (seven `-p nvs-syntax` tests, all in `lexer.rs`'s test module).

**Nothing produces `ExprKind::Markup` yet** — the parser primary is the next group's first item. Every
`ExprKind` match outside `nvs-syntax` has a catch-all, so the day the parser produces one the build stays
green while inference and lowering silently ignore it; the playbook bullet under *Writing Novis itself*
is that trap, and the next group's items are where the arms get written deliberately.

The escapes needed no new code: `lex_escape_in_place` already consumes `` \` `` and `\{` with the
segment, which is exactly what the rule asks for, so the two escape slices landed as tests over the
scanner rather than as cases in it.

## Next group

**Stage 3: the type, and what a hole admits** — one file set: `crates/nvs-syntax/src/parser/` and
`crates/nvs-types/src/expr/`.

- [ ] **The parser primary** — `crates/nvs-syntax/src/parser/expr.rs:1333`, beside the
      `TokenKind::DoubleQuoteOpen` arm: `MarkupOpen` runs `parse_string_body(TokenKind::MarkupClose)`
      (`crates/nvs-syntax/src/parser/expr.rs:2505`) into `ExprKind::Markup`. Unlike a string it must
      **not** collapse a hole-free body to `ExprKind::Str` — `collapse_string_parts`
      (`crates/nvs-syntax/src/parser/mod.rs:811`) is that collapse, and the node, not the part count, is
      what says `Core\Html\Markup` (`rule:core-classes/html-literal`).
- [ ] **The inference arm** — `crates/nvs-types/src/expr/mod.rs:258`, beside the `Interpolated` arm:
      the literal is `Core\Html\Markup`, and each hole is checked exactly as `infer_interpolated`
      (`crates/nvs-types/src/expr/literals.rs:409`) checks an interpolation's operand, except that a
      hole already holding a `Markup` needs no conversion because it is spliced. Tests
      `a_markup_literal_types_as_core_html_markup`, `a_markup_typed_hole_needs_no_conversion`.
- [ ] **The qualified holes** — the same arm, `crates/nvs-types/src/expr/mod.rs:258`: a `tainted`
      operand is admitted because the sink escapes it,
      a `secret` one is refused where it is written (`rule:security/secret-sinks-refuse`). Tests
      `a_tainted_hole_is_admitted_because_the_sink_escapes_it`,
      `a_secret_hole_is_refused_where_it_is_written`. No new diagnostic code — ADR 0169 § *Diagnostics*.

## Backlog

- Stage 4, the lowering and the two shapes that pay nothing — `docs/agent/loop-goal.md` § *Stage 4*.
- Stage 5, `Core\Html::join` as one `Core` member — `docs/agent/loop-goal.md` § *Stage 5*.
- Stage 6 flips `rule:core-classes/html-literal` to shipped and lands the three tool sentences —
  `docs/agent/loop-goal.md` § *Stage 6*.
- The `ExprKind` catch-alls in `nvs-hir/src/{members,requires}.rs`, `nvs-lsp/src/{hints,semantic}.rs`
  and `nvs-ir/src/lower/control.rs` still ignore `Markup`; stages 3 and 4 own the arms they need.
