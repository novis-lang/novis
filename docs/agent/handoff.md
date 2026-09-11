# Handoff

## State

**Goal 44 — markup is written as a literal, not composed with an operator — has just started; nothing of it has landed yet.** Goal `finish-response`'s whole list is this goal's Stage 1 floor.

**The design is already decided and landed.** [ADR 0169](../decisions/0169.md) holds all of it and
`rule:core-classes/html-literal` states it at `designed`; no session writes a record for this goal, and
the last stage is what flips that rule to `shipped`. The three things not to re-decide are in
§ *Standing decisions*, and the first of them is the one that matters: **the compiler learns no HTML.**
No tag tracking, no attribute model, no refusal that depends on where a hole sits.

The literal is the double-quoted-string lexer with a different delimiter. Every frame it needs already
exists — `{$` opens an ordinary code frame at `crates/nvs-syntax/src/lexer.rs:1260-1264`, brace depth is
counted at `:1064-1090` so a closure inside a hole does not close it, and
`crates/nvs-syntax/src/parser/expr.rs:2505-2527` already builds the `Vec<StringPart>`. A session that
finds itself writing a scanner has taken a wrong turn.

## Next group

**Stage 2: the literal lexes** — one file set: `crates/nvs-syntax/`, all of it.

- [ ] **The delimiter and the closer** — `crates/nvs-syntax/src/token.rs`, beside `DoubleQuoteOpen` and
      `ComplexInterpClose`: `` html` `` opens, `` ` `` closes, and the mode between them is the
      double-quoted one with those two swapped in.
- [ ] **The two escapes** — `crates/nvs-syntax/src/lexer.rs`, in the segment scanner: `` \` `` is a
      backtick and `\{` is a brace. Nothing else gains an escape, because a segment is opaque bytes.
- [ ] **The unterminated arm** — `crates/nvs-syntax/src/lexer.rs:210-232`, one more `Mode` row, so an
      unterminated literal reports `E0002` at the delimiter that opened it rather than running to end
      of file unnamed. No new diagnostic code; ADR 0169 § *Diagnostics* is why.
- [ ] **The node** — `crates/nvs-syntax/src/ast.rs`, carrying the same `StringPart` vector the
      interpolated string carries, with the carrier type attached. One node, not a tree.

## Backlog

- **Stage 3, the type** — `crates/nvs-types/`. The literal is a `Core\Html\Markup`; a hole faces
  exactly an interpolation's checks and a `Markup`-typed hole needs no conversion. Cheap to take
  beside stage 2 only if stage 2 landed the node cleanly.
- **Stage 4, the lowering** — `crates/nvs-ir/`, `crates/nvs-codegen/`. The constant fold and the
  write-through. A different file set; its own session.
- **Stage 5, `Core\Html::join`** — `crates/nvs-stdlib/src/html.rs`, `crates/nvs-types/src/core_lib.rs`.
  One member on the five-edit shape, and the conformance case that is a `foreach` and a join.
- **Stage 6, the rulebook** — flip `core-classes/html-literal` to `shipped`, fill its `guardedBy`,
  `python tools/rules.py --render`, and correct stage 0's four sentences in
  `crates/nvs-stdlib/src/html.rs`. Cheap beside stage 5, which is already in that file.
- When this goal's last check goes green the driver takes goal `gap-zero`.
