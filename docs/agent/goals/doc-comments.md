---
milestone: post-parity
---
# Loop goal 13 — `///` is a doc comment, and one JSON carries it

Give Novis source the one thing it has no way to say: the sentence a type cannot carry. `///` becomes a
doc comment, its content is prose plus exactly `@see` and `@example`, every other `@tag` is a
diagnostic, both tags are *checked* rather than rendered on trust, and `nvs meta --json` grows a program
argument so the two renderers already on that pipeline get user declarations for free.
`rule:tooling/doc-comment-is-three-slashes` is the whole design.

This goal closes a hole rather than adding a feature.
`rule:ide/the-request-set-is-closed`'s `textDocument/hover` row
already promises "for a declaration, the `TriviaKind::DocComment` run attached to it" — and nothing in
the tree defines which trivium that is, because the trivia layer does not exist yet.

Its floor is goal `typed-callable`'s whole list.

## Why here

Post-parity: `rule:tooling/doc-comment-is-three-slashes`'s doc comments. After goal `typed-callable` because both
reach the front end and goal `typed-callable`'s signature work is the larger edit there, and because its stage 2
builds `rule:ide/one-grammar-one-tree` s1's trivia layer — M4B's own tree item, landing early
because a doc comment cannot be read without it. M4B then starts with that half done.

## What lands here that M4B was going to build

**Stage 2 is M4B's tree item, landing early.** `rule:ide/one-grammar-one-tree`'s `Parsed { stmts, trivia, index }` is
M4B's, and M4B is carried by goals `resilient-tree`, `lsp-server` and `editor` — the end of this chain. A doc comment cannot be read
without retaining it,
so this goal builds the `Trivia` vector and `TriviaKind` **as `rule:ide/one-grammar-one-tree` specifies them**, plus that
section's fourth variant. M4B then inherits it done and keeps the rest: the `SyntaxIndex`, `nvs-lsp`,
syntax highlighting, `.lspt`. [docs/plan/m4b.md](../../plan/m4b.md) records the move.

**Hover is the one row this goal cannot contain.** It needs `crates/nvs-lsp`, which does not exist. It
needs no note either: `rule:ide/the-request-set-is-closed`'s row and `docs/plan/m4b.md` were both folded when `rule:tooling/doc-comment-is-three-slashes` landed,
so M4B arrives knowing what hover reads and which half of its tree is already built.

## The surface, in one block

```php
/// The price in cents. Money is `decimal`, never `float`.
/// A negative amount throws; zero is allowed and is a no-op.
///
/// @see Core\Money::fromCents
/// @example examples/charge.nvs
public function charge(uint $cents): void { … }

// An ordinary comment. Nothing reads it.
//// ─────────────────────────  also ordinary: four or more slashes
```

## Stage 0 — the catch-up

Nothing. `rule:tooling/doc-comment-is-three-slashes` landed with this goal. The seven files already carrying `///` reclassify with no edit
and read correctly as-is — that is stage 2's own test, not a migration.

## Stage 1 — the floor

Goal `typed-callable`'s whole acceptance list, never traded.

## Stage 2 — the keystone: trivia, and the fourth variant

One file set: `crates/nvs-syntax/src/lexer.rs`, `crates/nvs-syntax/src/token.rs`,
`crates/nvs-syntax/src/parser/mod.rs`.

1. **`Trivia` and `TriviaKind`** — `rule:ide/one-grammar-one-tree`'s shape exactly: `Lexer` gains a flag, `skip_trivia`
   (`crates/nvs-syntax/src/lexer.rs:357`) pushes a `Trivia { kind, span }` instead of only advancing.
   The variants are `Whitespace`, `LineComment`, `BlockComment` and `DocComment`. `TriviaKind` lives
   beside the token types in `crates/nvs-syntax/src/token.rs`.
2. **The run-length rule** — exactly three `/` is `DocComment`; four or more is `LineComment`, which is
   Rust's rule and is why the `//// ____` divider in
   `tests/conformance/core/encoding-every-encoder-agrees-with-its-own-decoder-over-a-table.nvst:114`
   stays an ordinary comment. A `#` comment is never a doc comment at any length.
3. **`parse_file` becomes `Parsed`** — `crates/nvs-syntax/src/parser/mod.rs:504` returns `stmts` and
   `trivia`; the `SyntaxIndex` field is **M4B's** and is not built here. The strict entry point stays a
   thin wrapper so no call site changes, exactly as `rule:ide/one-grammar-one-tree` requires.
4. **Losslessness is the acceptance property, not an assertion** — concatenating every token's and every
   trivium's source text in offset order equals the file byte-for-byte, over `examples/` and
   `tests/`.

## Stage 3 — attachment, and the closed set

One file set: `crates/nvs-syntax/src/parser/decl.rs`, `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-diagnostics/src/lib.rs`.

1. **Attachment** — a run of consecutive `///` lines separated by nothing but whitespace is one comment,
   attached to the declaration that follows it. A blank line breaks attachment. A run attached to
   nothing is a diagnostic. `crates/nvs-syntax/src/parser/decl.rs` is where a declaration and its
   attributes already meet, so it is where its doc comment joins them.
2. **The two tags parse** — `@see <member>` and `@example <path>`, each on its own line in a trailing
   block. `rule:tooling/doc-comment-tags-are-see-and-example` is the shape.
3. **Every other `@tag` at the start of a line is a diagnostic** — this item *is* the closed set; without
   it the set is a convention, and a convention is how PHPDoc came to document a signature twice.
   `@param`, `@return` and `@throws` each get their own wording naming what to write instead, per
   `rule:tooling/doc-comment-tags-are-see-and-example`.

## Stage 4 — the two checks that keep a tag honest

One file set: `crates/nvs-hir/`, plus wherever the `nvs check` path already walks declarations.

1. **`@see` must resolve** — against the same class and member tables `nvs-hir` builds, or it is a
   diagnostic. The standard is the one `python tools/check-links.py` already holds 282 markdown files to.
2. **`@example` must exist *and* be inside a directory the test corpus walks** — `examples/` is such a
   directory today. This is the whole reason the tag survived the cut: an example that stops compiling
   fails the build instead of rotting inside a rendered page.

## Stage 5 — `nvs meta --json` grows an argument

One file set: `crates/nvs-cli/src/main.rs`, `crates/nvs-cli/src/meta.rs`.

1. **`nvs meta --json <entry>`** — `Command::Meta` (`crates/nvs-cli/src/main.rs:509`) takes an optional
   path. With none, the output is **byte-identical to today's**, which is the check that protects both
   renderers on this pipeline. With one, the program's own declarations are emitted beside the `Core`
   registry.
2. **The user-declaration shape mirrors the registry's** — name, signature, prose, `@see` list,
   `@example` list. `rule:tooling/meta-json` owns the `Core` half and is not touched; `rule:tooling/meta-json-takes-a-program` owns this half.

## Stage 6 — the renderer and the lint

One file set: a new `crates/nvs-cli/src/doc.rs`, `crates/nvs-cli/src/main.rs`.

1. **`nvs doc <entry>`** — one Markdown page per class, from stage 5's JSON and from nothing else. It
   decides nothing and is deliberately cheap to replace; `tools/reference.py` and the website already
   cover every in-tree consumer, so this exists for a project that does not have them.
2. **`nvs check --strict-docs`** — a **public** member with no attached doc comment is reported. Silent
   without the flag, in every project, at every other setting. There is nothing for an autofix to
   generate, which is the property `rule:tooling/strict-docs` relies on.
3. **No hover code lands here.** `rule:ide/the-request-set-is-closed`'s hover row
   already names the `TriviaKind::DocComment` run as what it reads, and
   [docs/plan/m4b.md](../../plan/m4b.md) already records which half of its tree this goal built — both
   folded when `rule:tooling/doc-comment-is-three-slashes` landed. There is nothing left for this stage to write down.

## Standing decisions

- **`rule:tooling/doc-comment-is-three-slashes` is settled and is not re-derived.** Its four decisions — `///` as the marker with `////`
  ordinary, prose plus exactly `@see` and `@example` with any other tag a diagnostic, `nvs meta --json`
  as the one machine-readable source with `nvs doc` as a renderer over it, and enforcement silent by
  default — were taken with the user before this goal was written. A session that finds an
  implementation reason to differ records it in that ADR's *Revisiting* and implements the decision as
  written.
- **This goal may open no new ADR number.** `rule:tooling/doc-comment-is-three-slashes`'s body is the home for a rule, the touched module's
  doc comment for a mechanism, the playbook for a trap.
- **The tag set does not grow, in this goal or in a session's judgement.** A third tag needs a *check* it
  makes possible, argued in `rule:tooling/doc-comment-is-three-slashes`'s *Revisiting*, not a rendering it would improve.
- **No `@param`-shaped structure, even where it would be easy.** Per-parameter documentation is parked
  against the package manager in `rule:tooling/doc-comment-is-three-slashes`'s *Revisiting*, and if it ever lands it lands as a
  registry-shaped field in stage 5's JSON — never as a tag.
- **Diagnostic codes come from `python tools/brief.py`, not from this file.** ADR 0137 §
  *Diagnostics* names the *bands* — parser `E01xx` for an unknown tag and an unattached run, name
  resolution `E03xx` for the two checks and for `--strict-docs` — deliberately without numbers, because
  another agent claims a code from the same directory.
- **The bidi check is reused, never duplicated** (`rule:security/bidi-predicate`):
  the lexer already checks every comment span, so `nvs doc` and `nvs meta --json` emit text that has
  already been accepted and neither grows a check of its own.
- **`SyntaxIndex` is not built here.** It is M4B's, it has no consumer in this goal, and building it
  early would mean maintaining it through five goals with nothing reading it.
- **Ambiguity about where a rule lives resolves toward `nvs-syntax`** — a doc comment is a lexical
  fact, and everything downstream reads it rather than re-deriving it. Decided-and-recorded in that
  crate's module doc, never `BLOCKED`.
