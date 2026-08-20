# Parser work: handoff / continuation notes

Working notes for continuing M1's parser (`crates/mwl-syntax`) in a fresh session. Read
[CLAUDE.md](CLAUDE.md) first as always — this file is just "where the parser work stands and what's
next," not a replacement for the ADRs it references. Delete this file once the parser is far enough along
that it stops being useful (e.g. once M1 is verified per the plan).

## Where things stand

M1 is split into three chunks, agreed with the user: **expressions/precedence**, **statements**,
**class/interface/trait/enum declarations** — landed and reviewed one at a time.

**Chunks 1 and 2 are done, tested, and committed. Chunk 3 (declarations) has not been started.**

- [`crates/mwl-syntax/src/ast.rs`](crates/mwl-syntax/src/ast.rs): `Type`/`TypeKind`/`TypeAtom` (ADR 0007
  § 3's full grammar); `Expr`/`ExprKind` (every construct M1's plan names, now including
  `ExprKind::Include` for `include`/`include_once`/`require`/`require_once` — an expression, not a
  statement); `Block`/`Stmt`/`StmtKind` now cover every control-flow statement, `echo`, `unset`, ADR 0007
  § 3.1's typed local declaration, § 3.3's destructuring statement, and the statement-shaped rejects
  (`global`, `goto`, function-scope `static`) — see the new-types list below.
- [`crates/mwl-syntax/src/parser.rs`](crates/mwl-syntax/src/parser.rs): a `Parser` with a small lookahead
  buffer over the `Lexer`, the full type and expression grammar (chunk 1), and now the full statement
  grammar (chunk 2). 94 unit tests inline (`#[cfg(test)] mod tests`), all passing, plus one `insta` snapshot
  test (the first real use of that dev-dependency — see *Backtracking and snapshot testing* below).
- `crates/mwl-diagnostics/src/lib.rs`: two more E02xx codes landed this chunk, closing the two that were
  reserved-but-unused: `E_SETTYPE_UNSUPPORTED` (`E0208`) and `E_STATIC_LOCAL_UNSUPPORTED` (`E0209`).
- `crates/mwl-diagnostics/src/diagnostic.rs`: `Diagnostics::truncate(len)` — new, undoes a speculative
  parse's diagnostics on backtrack. See *Backtracking* below.
- `crates/mwl-syntax/src/lexer.rs`: `Lexer` and its private `Mode` enum are now `Clone` — needed so the
  parser can checkpoint and restore a lexer position wholesale. See *Backtracking* below.
- `cargo build` / `cargo test` (whole workspace) / `cargo fmt --check` / `cargo clippy --all-targets -- -D
  warnings` are all clean as of this commit.

### New AST added this chunk (in `ast.rs`)

- `StmtKind`: `Empty`, `If`, `While`, `DoWhile`, `For`, `Foreach`, `Switch`, `Break`, `Continue`, `Try`,
  `Echo`, `Unset`, `LocalDecl`, `Destructure`, `Global`, `Goto`, `StaticLocal` (plus the pre-existing
  `Expr`/`Return`/`Block`/`Error`).
- Supporting structs/enums: `ForeachBinding`, `CatchClause`, `SwitchCase`, `DestructureElement`
  (`Skip`/`Leaf`/`Nested`), `DestructureTarget`, `StaticVar`, `IncludeKind`.
- `ExprKind::Include { kind: IncludeKind, path: Box<Expr> }` — `include`/`include_once`/`require`/
  `require_once`, parsed in `parse_primary` next to `print`/`throw` since it is an expression
  ([`docs/spec/00-overview.md` § 2](docs/spec/00-overview.md): `$x = include 'a.php';` is legal).

### A real chunk-1 bug found and fixed while building `foreach`

`parse_type_intersection` used to treat any `&` right after a type as continuing an intersection
(`A&B`) unconditionally. That's wrong whenever the `&` is actually a by-reference marker sitting right
after a type with nothing between — `int &$v` in a `foreach` value binding, but **also already possible in
chunk 1's own `parse_param`**: `function f(int &$x)` would have hit `parse_type`'s intersection loop, tried
to parse another type atom after `&`, found `$x` instead, and reported a spurious "expected a type" error.
No chunk-1 test happened to cover a by-ref *typed* parameter, so this shipped unnoticed.

Fixed with one extra token of lookahead (`Parser::at_intersection_amp`): `&` continues an intersection only
if what follows it is itself a type-start token, via the same `token_starts_type` list `can_start_type` and
`at_function_scope_static` share. A bare `$name` after `&` now always means "by-reference marker," never "a
second intersection member" — mirrors real PHP 8.1's own resolution of the identical ambiguity. This is a
parser-wide fix, not something scoped to `foreach`: every existing and future caller of `parse_type`
benefits.

### The `eval`/`extract`/`settype` open question — closed

The previous handoff left open whether to intercept `eval`/`extract`/`settype` by identifier text in
`parse_primary` (like `spawn`/`script`) or defer rejection to name resolution. Turns out the lexer already
reserves all three as hard keywords (`Keyword::Eval`/`Extract`/`Settype` in `token.rs`) — they are not
plain identifiers at all, so the "defer to M2" option was never actually on the table; there is no
`Ident`-shaped `eval` for a resolver to see. `parse_primary` now has dedicated arms for all three
(`Parser::parse_eval`/`parse_extract`/`parse_settype`, sharing `Parser::skip_call_args`), each reporting its
ADR-0007-§2 diagnostic and producing `ExprKind::Error` — the same "report and keep going, discard the
payload" shape `$$var` already used in chunk 1.

### Backtracking, and why it exists now

Two statement forms are ambiguous on a token prefix alone, and resolving them needs more than one token of
lookahead:

- **Typed local declaration vs. an ordinary expression statement.** `Foo $x = ...;` (a local of type `Foo`)
  and `Foo::bar();` (a static call) both start with an `Ident`. `Foo|Bar $x;` (a union-typed local) and
  `Foo | Bar;` (a bitwise-or expression statement) both start with an `Ident` followed by `|`. The type
  grammar is compositional (`?T`, `array<T>`, `A|B`, `(A&B)|C`, qualified names) so no fixed lookahead
  distance resolves every case — only "does a `Variable` immediately follow wherever the type parse
  concludes" does, which is exactly the language's own committed rule ("the type comes first, in the same
  position PHP already uses for a parameter").
- **Destructuring target vs. a plain array-literal expression statement.** `[int $a] = $p;` versus
  `[1, 2, 3];` (legal, if useless). Same shape of ambiguity, resolved the same way: try the destructuring
  grammar, keep it only if `=` follows.

Rather than hand-write a lookahead classifier that re-implements `parse_type`'s (or the destructuring
grammar's) shape just to decide "does this end where I think it does" — a second copy that *would* drift,
exactly the kind of thing [CLAUDE.md](CLAUDE.md) calls a bug — the parser now has genuine backtracking:
`Parser::checkpoint`/`Parser::restore`, backed by `Lexer: Clone` and `Diagnostics::truncate`. A speculative
parse commits to the trial's tokens *and* diagnostics only if the trial's deciding token shows up;
otherwise every token and every diagnostic the trial produced is undone and the alternative production
runs instead. See `Parser::parse_stmt_maybe_local_decl` and `Parser::parse_stmt_maybe_destructure` (the
latter's own key-detection, `Parser::parse_destructure_key`, nests the same trick one level deeper for
`(string-literal | expr) '=>'`). `list(...)` needed none of this — it collides with no expression grammar,
so it always means a destructuring target.

This is new, reusable infrastructure: chunk 3 (declarations) may well hit its own token-prefix ambiguities
and can reach for the same `checkpoint`/`restore` pair instead of inventing another mechanism.

### The `foreach`-header `as` wrinkle — pinned down with `insta`

ADR 0007 § 2 / [`docs/spec/00-overview.md` § 3.2](docs/spec/00-overview.md): inside a `foreach` header, `as`
is `foreach`'s own separator, not the conversion operator — converting the *subject* needs parens:
`foreach (($m as array<int>) as int $v)`. Implemented with a `Parser::suppress_as` field and
`Parser::parse_expr_no_top_as` (used only for the subject): `parse_postfix`'s `as`-handling arm is skipped
while it's set, and `Parser::parse_expr` (the ordinary, public entry point every nested sub-expression
already goes through — call arguments, array items, a parenthesized group, ternary branches, …) always
clears it for its own duration and restores it after. That one property is what makes the parenthesized
form work correctly: entering `(...)` re-enters via `parse_expr`, which lifts the suppression just for
what's inside the parens, so the *inner* `$m as array<int>` still converts while the outer, un-parenthesized
`as` stays reserved for `foreach`.

Pinned down with the plan's requested `insta` snapshot test —
`foreach_header_as_belongs_to_foreach_not_conversion` in `parser.rs`'s test module — the first real use of
the `insta` dev-dependency. Its snapshot file is
`crates/mwl-syntax/src/snapshots/mwl_syntax__parser__tests__foreach_header_as_belongs_to_foreach_not_conversion.snap`,
committed alongside the code. **Anyone adding another snapshot test needs `cargo insta review` (or
`INSTA_UPDATE=always cargo test -p mwl-syntax` for a first pass, then eyeball the diff before committing
the `.snap` file)** — a fresh snapshot test fails on its first run by design (no `.snap` file exists yet)
and that is not a bug in the test.

### Where `type` alias / `namespace` / `use` ended up

The previous handoff left this "not settled." Settled now: they travel with **chunk 3**, alongside the
declaration grammar, not with chunk 2's statements — `type Name = TypeExpr;` sits at file/namespace scope
per ADR 0015 § 5 and is closer in spirit to a declaration than to anything in this chunk's executable-
statement list.

## New diagnostic codes added this chunk

- `E_SETTYPE_UNSUPPORTED` (`E0208`) — `settype(...)`.
- `E_STATIC_LOCAL_UNSUPPORTED` (`E0209`) — function-scope `static $x (= ...)?;`.

Both were reserved-but-unused placeholders in the previous handoff; both are now real constants with real
call sites. No codes remain reserved-and-unused.

## Known gaps chunk 2 leaves (not silently — read this before relying on the parser for anything these touch)

- **PHP's alternative colon syntax is not implemented**: `if (...): ... endif;`, `while (...): ... endwhile;`,
  `for (...): ... endfor;`, `foreach (...): ... endforeach;`, `switch (...): ... endswitch;`. The `Keyword::
  End*` variants already exist in `token.rs` (lexed, unused), but nothing in `parser.rs` recognizes the
  `:` -delimited body form — a statement like `if ($x): echo 1; endif;` will currently misparse (the `:`
  after `)` is not `{`, so `parse_statement`'s body call falls through to an ordinary expression statement,
  and `endif;` becomes its own, likely-erroring statement). This is real PHP syntax still found in template-
  style code, so it is a plausible source of failures once M1's "parse the full local PHP 8.5 install"
  verification step actually runs. Worth doing before that step, or at least confirming the local PHP
  install's corpus doesn't use it.
- **`goto` target labels** (`label:` as its own statement, independent of any `goto`) are not parsed. Only
  `goto ident;` itself is handled (rejected). A bare `done:` statement will currently fall into the generic
  expression-statement path and misparse (`done` parses as a `ConstFetch`, then the `:` is unexpected).
  Low priority since `goto` itself is rejected outright and has no legitimate use left to label, but a
  real `.php` file with a leftover label could still trip the M1 corpus-parse verification step.
- **`Core\Static`-style keyword-segment collisions past the first one are still only spot-checked.** Chunk
  1's handoff flagged this as untested for names beyond `Core\Bytes`; still true, not investigated further
  this chunk.
- Attributes (`#[...]`) remain entirely unparsed, as chunk 1 already noted — still deferred to chunk 3.

## Chunk 3 (not started): class/interface/trait/enum declarations, plus file-scope declarations

Everything else M1 names, now confirmed to include the file-scope forms too:

- Classes, interfaces, traits, enums (cases + optional backing type only, ADR 0010), methods, attributes
  (`#[...]`), property hooks feeding `PropertyObserver` (ADR 0014), asymmetric visibility, promoted
  constructor parameters (the `Param.modifiers` field already exists in `ast.rs` for this — chunk 1 parses
  the modifiers generically on every parameter; restricting them to constructors is presumably a later
  semantic check, not a chunk-3 parser change), `insteadof` without `as` (ADR 0015), rejecting
  `class_alias`/import `as`/trait-use `as`, rejecting a `function`/`const` outside a class body and a
  class/namespace literally named `Core` (ADR 0011).
- **`namespace` and `use` declarations.**
- **`type Name = TypeExpr;`** (ADR 0007 § 3.5 / ADR 0015 § 5) — file/namespace scope, never inside a class
  body. Parses like any other alias declaration; the restriction that `TypeExpr` isn't a single bare
  class/interface/enum atom (ADR 0015 § 6) is M2's job, not the parser's.
- Consider whether `Parser::checkpoint`/`Parser::restore` (new this chunk, see above) resolves any
  declaration-grammar ambiguity chunk 3 runs into, before inventing a second backtracking mechanism.

## Verification still open for all of M1

From the plan: `mwl ast file.mwl` dumping the AST (no CLI crate exists yet — `mwl-cli` isn't in the
workspace per `.claude/brief.sh`'s "what exists on disk" listing, so this needs the crate scaffolded, not
just a function), `cargo fuzz` on the parser finding no panics in a 1h run, and parsing the full local PHP
8.5 install's `.php` files without crashing (not *checking* — that's M2). None of this is started. The
alternative-colon-syntax and `goto`-label gaps above are the most likely sources of surprises once the
corpus-parse step actually runs — check those first rather than debugging blind.

## Housekeeping

- ADR 0009's grapheme-segmentation cost guard test (`benches/abi-probe`) is independent of all of this and
  can slot in whenever — see the plan's status block.
- Once chunk 3 lands and the verification above is done, M1 closes and update
  [`docs/implementation-plan.md`](docs/implementation-plan.md)'s status block accordingly — that's the one
  place "where the plan stands" actually lives; don't let this file become a second copy of it.
