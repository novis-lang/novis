# Parser work: handoff / continuation notes

Working notes for continuing M1 (`crates/mwl-syntax`) in a fresh session. Read
[CLAUDE.md](CLAUDE.md) first as always — this file is just "where the parser work stands and what's
next," not a replacement for the ADRs it references. Delete this file once M1 is verified per the
plan (see *What's left for M1* below) and this file stops being useful.

## Where things stand

The parser was built in three chunks, agreed with the user: **expressions/precedence**, **statements**,
**class/interface/trait/enum declarations**. **All three are done, tested, and committed.** The parser
now covers the full pragmatic-superset grammar M1's plan names.

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
  the lexer and parser (up from chunk 2's 94), plus the one `insta` snapshot test from chunk 2, all
  passing.
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
- **Declarations reuse the ordinary statement grammar.** There is no `File`/`parse_file`
  whole-program entry point yet, and chunk 3 deliberately didn't add one — `class`/`interface`/`trait`/
  `enum`/`namespace`/`use`/`type`-alias/the rejected top-level `function`/`const` are all new
  `StmtKind` variants, reached through `Parser::parse_statement`'s existing dispatch, exactly like
  `if`/`while`/`global` already are. Building a whole-file driver is verification's job (`mwl ast`,
  below), not the parser grammar's, and needs `mwl-cli` scaffolded first.
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

## Known gaps (still open, none from chunk 3 itself)

Carried over from chunk 2, unaffected by chunk 3:

- **PHP's alternative colon syntax** (`if (...): ... endif;`, `while`/`for`/`foreach`/`switch` likewise)
  is entirely unimplemented. The `Keyword::End*` variants are lexed but nothing in the parser recognizes
  the `:`-delimited body form.
- **`goto` target labels** (`label:` as its own statement) are unparsed — only `goto ident;` itself is
  handled (rejected).
- **`Core\Static`-style keyword-segment name collisions past the first segment** are still only
  spot-checked.

New from chunk 3, deliberately scoped out (all low-priority, real-world-rare constructs — see the
"Design decisions" list above for the ones that were deliberate simplifications rather than gaps):

- **Grouped `use`** (`use App\{Foo, Bar};`) and **`use function`/`use const`** are not parsed — ADR 0015's
  own *Revisiting* note says these don't exist yet, so this isn't a regression, just not built.
  Only single `use Path\To\Name;` per statement is supported.
  A file using grouped `use` will misparse at the corpus-parse verification step below.
- **Legacy `var $x;`** (PHP 4's property declarator) is not handled — `Keyword::Var` is lexed but nothing
  in the parser recognizes it; it falls through to a generic parse error. Vanishingly rare in modern code.
- **A method/const/case name that is itself a reserved keyword spelling** works for methods and consts
  (`Parser::parse_decl_name` accepts any name-segment, keyword or not, matching `parse_member_name`'s
  existing allowance) but **not for enum cases**, which require a plain `Ident` — a case literally named
  e.g. `Static` would misparse. Narrow enough that it wasn't worth the extra ambiguity against
  member-starting keywords.

## Verification still open for all of M1

From the plan: `mwl ast file.mwl` dumping the AST (no CLI crate exists yet — `mwl-cli` isn't in the
workspace per `.claude/brief.sh`'s "what exists on disk" listing, so this needs the crate scaffolded, not
just a function), `cargo fuzz` on the lexer and parser finding no panics in a 1h run, and parsing the full
local PHP 8.5 install's `.php` files without crashing (not *checking* — that's M2). **None of this is
started**, and it is now the only thing standing between here and closing M1 — the grammar itself is
done. The known-gaps list above (alternative colon syntax, `goto` labels, grouped `use`) is the most
likely source of surprises once the corpus-parse step actually runs; check those first rather than
debugging blind, and expect to come back and extend the parser rather than treating M1's grammar as
frozen the moment a real-world `.php` file trips over one of them.

## Housekeeping

- ADR 0009's grapheme-segmentation cost guard test (`benches/abi-probe`) is independent of all of this and
  can slot in whenever — see the plan's status block.
- Fixed in passing: ADR 0010 had two places (the `In short` summary and § 2's body prose) saying an
  enum's default backing type is `uint`, contradicting the ADR's own `Amends` block and worked example,
  both of which say `int`. Corrected the two prose spots to `int`, matching the majority and the more
  precise sources; the parser itself never depended on this (it just parses whatever optional `: Type`
  clause is there).
- Once M1's verification above is done, close M1 and update
  [`docs/implementation-plan.md`](docs/implementation-plan.md)'s status block accordingly — that's the one
  place "where the plan stands" actually lives; don't let this file become a second copy of it. At that
  point, delete this file.
