# Parser work: handoff / continuation notes

Working notes for continuing M1 (`crates/mwl-syntax`, `crates/mwl-cli`) in a fresh session. Read
[CLAUDE.md](CLAUDE.md) first as always — this file is just "where the parser work stands and what's
next," not a replacement for the ADRs it references. Delete this file once M1 is verified per the
plan (see *Verification still open* below) and this file stops being useful.

**Standing rule for whoever (human or agent) picks this up next: update this file and commit before
ending the session.** This file is only useful if it reflects reality — a stale handoff is worse than
none, because the next session trusts it. If you change what's built, what's left, or what a known gap
is, edit the relevant section here (or delete the file if M1 is fully closed) and commit alongside the
code change, not as a separate afterthought.

## Where things stand

The parser was built in three chunks, agreed with the user: **expressions/precedence**, **statements**,
**class/interface/trait/enum declarations**. **All three are done, tested, and committed.** The parser
now covers the full pragmatic-superset grammar M1's plan names.

**Follow-up after chunk 3, before M1 verification could start:** the parser had no whole-file entry
point at all, and no handling anywhere in `parse_statement` for the HTML-mode tokens the lexer already
produced (`InlineHtml`, `OpenTagMwl`/`OpenTagPhp`/`OpenTagEcho`, `CloseTag`) — every test had always
manually bumped a leading open tag before parsing. That's now fixed:

- [`crates/mwl-syntax/src/parser.rs`](crates/mwl-syntax/src/parser.rs): `parse_statement` opens with a
  loop that consumes bare tag tokens and, spec `00-overview.md` § 1, produces a `StmtKind::InlineHtml`
  for real HTML text or hands off to `parse_short_echo_tag` for `<?= expr ?>`. Reachable anywhere a
  statement is expected, not just at file scope, because `?>`/`<?php` can reopen mid-block
  (`if ($x) { ?>html<?php }` is legal, exactly as in PHP). A new free function `parse_file` drives the
  whole thing top to bottom and is what `mwl ast` calls.
- [`crates/mwl-syntax/src/ast.rs`](crates/mwl-syntax/src/ast.rs): new `StmtKind::InlineHtml(Span)`.
  A bare tag token with no HTML payload (an open tag, or a close tag immediately followed by another
  open tag) reuses `StmtKind::Empty` rather than a dedicated marker — deliberately minimal, since
  nothing downstream needs to know *which* tag spelling closed/reopened code mode yet.
- `crates/mwl-cli` **is now scaffolded** (it wasn't before), with exactly one subcommand: `mwl ast
  <file>` parses via `parse_file`, prints the diagnostics (if any) to stderr through the existing
  `Renderer`, pretty-prints the `Vec<Stmt>` to stdout, and exits non-zero on any error — see
  [`crates/mwl-cli/src/main.rs`](crates/mwl-cli/src/main.rs). It only has this one command; `run`/`test`
  are M3's job per the plan's architecture diagram.
- 6 new tests in `parser.rs` cover the round trip: pure code, leading HTML before the first tag,
  a file that's nothing but HTML, closing and reopening mid-block, and the short-echo tag's optional
  semicolon. All pass; the pre-existing 109 chunk-1–3 tests are unaffected (they still manually bump the
  leading open tag, unchanged).
- `crates/mwl-syntax/src/lib.rs` now carries an actual "Known gaps" doc section (the plan's status
  block already pointed here; the list just didn't exist yet). Kept in sync with the list below —
  update both together, or better, delete this file's copy and point here only, once M1 closes.

- [`crates/mwl-syntax/src/ast.rs`](crates/mwl-syntax/src/ast.rs): `Type`/`TypeKind`/`TypeAtom` (ADR 0007
  § 3's full grammar); `Expr`/`ExprKind` (every construct M1's plan names); `Block`/`Stmt`/`StmtKind`
  covering every control-flow statement, the statement-shaped rejects, and now every declaration form —
  `ClassDecl`/`InterfaceDecl`/`TraitDecl`/`EnumDecl`, their members (`ClassMember`/`ClassMemberKind`:
  `PropertyMember` with PHP 8.4 hooks, `ConstMember`, `MethodMember`, `UseTraitMember` with
  `insteadof`/`as` adaptations), `AttributeGroup`/`Attribute`, and the file-scope
  `NamespaceDecl`/`UseDecl`/`TypeAliasDecl`.
- [`crates/mwl-syntax/src/parser.rs`](crates/mwl-syntax/src/parser.rs): a `Parser` with backtracking
  (`checkpoint`/`restore`) reused, as anticipated, for nothing new in chunk 3 — every declaration-grammar
  ambiguity chunk 3 hit (property vs. method, case vs. member, `function` declaration vs. anonymous
  closure statement) resolved with one or two tokens of lookahead instead. 109 unit tests inline across
  the lexer and parser as of chunk 3 (up from chunk 2's 94), plus the one `insta` snapshot test from
  chunk 2 — since grown to 115 by this session's HTML/tag round-trip work (see above), all passing.
- `crates/mwl-diagnostics`: nine new `E02xx` codes for chunk 3's rejects (`E0212`–`E0220`) — import
  rename, both trait `as` forms, top-level `function`/`const`, the reserved `Core` namespace, and enum
  `implements`/`: string`/non-case members. `E_BAD_MODIFIER` (E0106) and `E_BAD_PARAM_LIST` (E0107)
  remain reserved-but-unused: modifier parsing stayed permissive everywhere (see below), so nothing
  needed them.
- `cargo build` / `cargo test` (whole workspace) / `cargo fmt --check` / `cargo clippy --all-targets -- -D
  warnings` are all clean as of this commit.

### Design decisions worth knowing before touching this code

- **Modifiers are one shared, permissive `Modifier` enum**, not a per-position type. `Param.modifiers`
  (chunk 1) already accepted any modifier on any parameter and deferred "does this combination make
  sense" to a later check; chunk 3 extended `Modifier` itself (`Static`, `Abstract`, `Final`,
  `SetVisibility(Visibility)` for PHP 8.4's `private(set)` etc.) and reused the *same* parsing loop
  (`Parser::parse_modifiers`) for parameters, properties, constants, methods and class headers alike.
  This is why `E_BAD_MODIFIER` is still unused: no combination is rejected at parse time anywhere.
- **Declarations reuse the ordinary statement grammar.** `class`/`interface`/`trait`/`enum`/
  `namespace`/`use`/`type`-alias/the rejected top-level `function`/`const` are all `StmtKind` variants,
  reached through `Parser::parse_statement`'s existing dispatch, exactly like `if`/`while`/`global`
  already are. Chunk 3 deliberately didn't add a `File`/whole-program entry point on top of this —
  that arrived later, as `parse_file`, once `mwl-cli` needed one to verify against (see *Where things
  stand* above).
- **The "parse it fully, then reject" pattern (chunk 2's `global`/`goto`/function-scope `static`)
  covers every chunk-3 reject too**: `use X as Y;`, a trait `use` block's `as` clause (both the rename
  and visibility-only forms), a top-level `function`/`const`, `namespace Core`, and an enum's
  `implements`/`: string`/non-case member all parse into a full AST node and *then* get a diagnostic.
  Nothing chunk 3 rejects uses the "discard the payload" shape (`eval`/`extract`/`settype`'s
  `ExprKind::Error`) — there was always something worth keeping for the diagnostic or for tooling.
- **`type Id = SomeClass;` parses with no special case.** ADR 0015 § 6's "not a single bare class atom"
  restriction is explicitly M2's job (both the ADR and the spec say so); the parser calls the ordinary
  `parse_type` and stores whatever comes back.
- **The "class declared directly under `Core`" half of ADR 0011 § 2 is *not* checked.** Only
  `namespace Core;`/`namespace Core\Sub { ... }` is rejected at parse time (a direct text check on the
  namespace name). Catching a class that's merely *inside* a `Core`-named namespace needs "what
  namespace is this declaration in" state threaded across statements, which is name-resolution's job
  (M2) — the standalone, one-declaration-at-a-time parser has no such state and shouldn't grow one just
  for this.
- **Property hooks are parsed as literally PHP 8.4 already has them** (`get => expr;` / `set(Type $v) {
  ... }` / abstract `get;`), per ADR 0014's own instruction that it "adds no new syntax." `get`/`set` are
  contextual identifiers, not keywords, matched the same way `spawn`/`script`/`with`/`type` already are.
- **Enum bodies mix cases and (always-rejected) members structurally**, since ADR 0010 gives MWL's enum
  no `case` keyword — a case is just a bare name, comma-separated, directly in the body. The parser peels
  off any `#[...]` attributes first, then dispatches on whether an `Ident` or a member-starting keyword
  follows, since attributes alone don't tell a case and a member apart.
- **Anonymous classes (`new class { ... }`) are implemented**, closing the gap chunk 1 flagged. Unlike an
  ordinary `new Name(args)`, the constructor's `(args)` sits right after `class`, before
  `extends`/`implements`/the body — `Parser::parse_new_anon_class` handles that ordering directly rather
  than reusing `parse_new`'s generic post-target `args` parsing.

## Known gaps

Moved to [`crates/mwl-syntax/src/lib.rs`](crates/mwl-syntax/src/lib.rs)'s "Known gaps" module-doc
section — that's a durable home this scratch file isn't, and it's what the plan's status block now
points at. Don't re-duplicate the list here; update the doc comment instead and note below only that
it changed. As of this session it still lists: PHP's alternative colon syntax (`if (...): ... endif;`
and friends), `goto` target labels, grouped `use`/`use function`/`use const`, legacy `var $x;`, an
enum case named with a reserved-keyword spelling, and spot-checked-only `Core\Static`-style name
collisions past the first segment. The HTML/code-tag round trip that used to be an *undocumented* gap
(nothing in `parse_statement` handled `InlineHtml`/tag tokens at all) is fixed — see *Where things
stand* above — and is no longer on this list.

## Verification still open for all of M1

From the plan: `mwl ast file.mwl` dumping the AST — **done this session**, see *Where things stand*
above. Still open: `cargo fuzz` on the lexer and parser finding no panics in a 1h run, and parsing the
full local PHP 8.5 install's `.php` files without crashing (not *checking* — that's M2). Neither is
started. The known-gaps list (now in `mwl-syntax`'s module docs, linked above) is the most likely
source of surprises once the corpus-parse step actually runs; check those first rather than debugging
blind, and expect to come back and extend the parser rather than treating M1's grammar as frozen the
moment a real-world `.php` file trips over one of them. `cargo fuzz` needs `cargo-fuzz` set up in this
workspace first (nightly toolchain, `fuzz/` target directory) — not done yet either.

## Housekeeping

- ADR 0009's grapheme-segmentation cost guard test (`benches/abi-probe`) is independent of all of this and
  can slot in whenever — see the plan's status block.
- Fixed in passing: ADR 0010 had two places (the `In short` summary and § 2's body prose) saying an
  enum's default backing type is `uint`, contradicting the ADR's own `Amends` block and worked example,
  both of which say `int`. Corrected the two prose spots to `int`, matching the majority and the more
  precise sources; the parser itself never depended on this (it just parses whatever optional `: Type`
  clause is there).
- This session: scaffolded `crates/mwl-cli` (`mwl ast` only), added `mwl_syntax::parse_file` and the
  HTML/code-tag round trip it needed, moved the known-gaps list into `mwl-syntax`'s module docs, and
  updated `docs/implementation-plan.md`'s status block and `README.md`'s repository-layout table to
  match (`mwl-cli` now "exists" early, ahead of its M3 column, with a note explaining why). `cargo
  build` / `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` all clean
  across the whole workspace as of this commit.
- Once M1's verification above is done, close M1 and update
  [`docs/implementation-plan.md`](docs/implementation-plan.md)'s status block accordingly — that's the one
  place "where the plan stands" actually lives; don't let this file become a second copy of it. At that
  point, delete this file.
