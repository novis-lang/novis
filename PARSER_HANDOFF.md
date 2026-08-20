# Parser work: handoff / continuation notes

Working notes for continuing M1's parser (`crates/mwl-syntax`) in a fresh session. Read
[CLAUDE.md](CLAUDE.md) first as always — this file is just "where the parser work stands and what's
next," not a replacement for the ADRs it references. Delete this file once the parser is far enough along
that it stops being useful (e.g. once M1 is verified per the plan).

## Where things stand

M1 is split into three chunks, agreed with the user: **expressions/precedence**, **statements**,
**class/interface/trait/enum declarations** — landed and reviewed one at a time.

**Chunk 1 (expressions + types) is done, tested, and committed. The three gaps it left (below) are also now
fixed, tested, and committed — chunk 2 (statements) can start clean.**

- [`crates/mwl-syntax/src/ast.rs`](crates/mwl-syntax/src/ast.rs) (~710 lines): `Type`/`TypeKind`/`TypeAtom`
  (ADR 0007 § 3's full grammar), `Expr`/`ExprKind` (every construct M1's plan names), plus a deliberately
  minimal `Block`/`Stmt`/`StmtKind` (`Expr`/`Return`/`Block` only) so a closure body has somewhere to live.
- [`crates/mwl-syntax/src/parser.rs`](crates/mwl-syntax/src/parser.rs) (~2450 lines): a `Parser` with a
  small lookahead buffer over the existing `Lexer`, the full PHP-precedence expression grammar via
  precedence climbing, the full type grammar, `parse_block`/`parse_statement` (same minimal set as above),
  and the three closed-construct rejections below. 66 unit tests inline (`#[cfg(test)] mod tests` at the
  bottom of `parser.rs`), all passing.
- `crates/mwl-syntax/src/lib.rs` updated: `pub mod ast;`, `mod parser;`, re-exports `Parser` and
  `parse_expression`.
- `crates/mwl-diagnostics/src/lib.rs`: two new E02xx codes, `E_STATIC_CLOSURE_UNSUPPORTED` (`E0210`) and
  `E_SUPERGLOBAL_UNSUPPORTED` (`E0211`) — see below.
- `cargo build` / `cargo test -p mwl-syntax` / `cargo fmt --check` / `cargo clippy --all-targets -- -D
  warnings` are all clean as of this commit.

### Design conventions established (keep following these)

- **AST nodes store `Span`s, never cooked values** — an int literal's digits, a string's escapes, are
  parsed later (checker/IR), never in the parser. Mirrors the lexer's own documented discipline
  (`crates/mwl-syntax/src/lexer.rs` module docs).
- **`as` conversion is parsed as a postfix operator** (in `parse_postfix`'s loop), so it binds tighter than
  any binary operator per ADR 0007 § 2 — verified by a test using the ADR's own `$a as int + 1` example.
- **`array<T>`'s closing `>` may arrive fused into `>>`/`>=`/`>>=`** by the lexer (it has no reason to know
  it's in type position). `Parser::expect_type_close_angle` splits the token in place and pushes the
  remainder back onto the lookahead buffer. This is the mechanism `array<array<uint>>` needs, and it's
  covered by a test (`nested_array_generic_closes_through_a_split_shift_token`).
- **Error recovery**: every `parse_*` method always returns something (never `Result`); a missing token is
  `E_EXPECTED_TOKEN` at an empty span, a missing expression is `E_EXPECTED_EXPR` + `ExprKind::Error`.
  `parse_block`'s statement loop force-consumes one token if a statement made no progress at all, so a
  construct that isn't implemented yet can't hang the parser.
- **Testing style**: plain `let-else` + `matches!` structural assertions in an inline test module, not
  `insta` snapshots — avoids needing the `cargo insta review` workflow for now. The plan's M1 verification
  section explicitly wants one **`insta`** snapshot test for the `foreach`-header `as` wrinkle (see below);
  that'll be the first real use of the `insta` dev-dependency that's already in `Cargo.toml`.

### A real bug chunk 1's tests caught (already fixed, worth knowing about)

`bytes` is a reserved keyword (ADR 0009), matched case-insensitively by the lexer like every other keyword.
So `Core\Bytes` — literally the spec's own example
([`docs/spec/00-overview.md` § 5](docs/spec/00-overview.md)) — lexes `Bytes` as `Keyword(Bytes)`, not
`Ident`. Fixed by having `Parser::parse_name` accept a keyword-shaped token as a qualified-name *segment*
(after a `\` — never as the unqualified leading token, which still goes through the ordinary
`Ident`/`Keyword` dispatch in `parse_primary`/`parse_type_atom`), the same reasoning already used for a
member name after `->`/`::`. Keep an eye out for the same class of collision elsewhere (`Core\Static`? —
currently untested).

## Gaps chunk 1 left, now closed

Reading ADR 0008 in depth (for chunk 2's statement grammar) surfaced three things chunk 1 quietly did
*wrong* per the ADRs, because chunk 1 was scoped to "parse the grammar," not "reject what's closed." All
three are fixed and tested (commit after this handoff update):

1. **`static function`/`static fn`** — `Parser::parse_closure`/`parse_arrow_fn` now call
   `report_static_closure_modifier` when `is_static`, reporting `E_STATIC_CLOSURE_UNSUPPORTED` (new code,
   see below) naming ADR 0008 § 5's rule, then still bump past `static` and build the closure node as
   before (pragmatic-superset style). Tests: `static_closure_modifier_is_diagnosed_but_still_parses`,
   `static_arrow_fn_modifier_is_diagnosed_but_still_parses`, `ordinary_closure_is_not_diagnosed`.
2. **Superglobal variable names** — `parse_primary`'s `Variable` case now calls `check_superglobal`, which
   matches the raw `$…` text against ADR 0012 § 8's exact list and reports `E_SUPERGLOBAL_UNSUPPORTED` (new
   code) with that section's exact replacement text per name (`$GLOBALS`/`$_REQUEST` get "does not exist,"
   no `Core` class suggested). Covers all twelve ADR 0012 spellings, including `$_COOKIE`/`$_FILES`/`$argc`/
   `$_ARGS` which the handoff note's example list didn't spell out but the ADR's table does. Still builds
   the ordinary `ExprKind::Variable` node. Tests: `superglobals_are_diagnosed_but_still_parse_as_variables`
   (parametrized over all 12), `an_ordinary_variable_is_not_mistaken_for_a_superglobal`.
3. **`$$name`/`${expr}`** — `parse_primary` now has a `TokenKind::Dollar` arm: it bumps the `$`, then
   best-effort consumes the rest of the shape (`Variable` for `$$name`, `{expr}` for `${expr}`) so the
   caller doesn't immediately trip over a leftover token, reports `E_VARIABLE_VARIABLE` (already existed)
   naming "defeats name resolution and type inference" (the reason already on `TokenKind::Dollar`'s own doc
   comment) with a suggested `array<string, T>` replacement, and returns `ExprKind::Error`. Tests:
   `dollar_dollar_name_is_variable_variable`, `dollar_brace_expr_is_variable_variable`.

All three follow the existing "report and keep going" convention — nothing here introduces a `Result` or
changes the "every `parse_*` always returns something" rule.

## New diagnostic codes added

Added to `crates/mwl-diagnostics/src/lib.rs`'s `code` module (E02xx = "rejected PHP constructs"), used by
the fixes above:

- `E_STATIC_CLOSURE_UNSUPPORTED` (`E0210`) — `static function`/`static fn`.
- `E_SUPERGLOBAL_UNSUPPORTED` (`E0211`) — any of the twelve ADR 0012 spellings.

**Still not added — reserved for chunk 2, do not reuse these numbers for anything else:**

- **`settype()`** — ADR 0007 § 2: *"joins `eval`, `$$var`, `goto`, `global` and `extract()` on the rejected
  list, with a diagnostic naming `as` as the replacement."* Needs e.g. `E_SETTYPE_UNSUPPORTED` (`E0208`).
- **function-scope `static`** (`static int $calls = 0;` inside a function) — ADR 0008 § 5's exact wording:
  *"function-scope `static` is not supported; declare a `private static` property on a class, or pass the
  value as a parameter."* Needs e.g. `E_STATIC_LOCAL_UNSUPPORTED` (`E0209`).

These weren't added yet because nothing calls them until chunk 2 parses statements (`static $x;` as a
statement) and resolves the `eval`/`extract`/`settype` open question below — adding an unused constant now
would just be dead code until then. `E0208`/`E0209` are reserved (skipped) so the numbering already agreed
here doesn't shift later; add them as real constants only once chunk 2 wires up their call sites, same
pattern as `E0210`/`E0211` above.

**Open question, worth resolving early in chunk 2**: `eval(...)`, `extract(...)` and `settype(...)` are
*syntactically* ordinary function calls — nothing in the grammar distinguishes `eval($x)` from any other
call except the name `eval`. Chunk 1 already special-cases `isset`/`empty`/`exit` by recognizing their
*keyword* token in `parse_primary`, but `eval`/`extract`/`settype` are plain identifiers, not keywords. Two
ways to reject them at the right stage:
  - **(a)** Special-case them by identifier text in `parse_primary`'s `Ident` branch (like the
    `spawn`/`script` contextual-keyword check already does), immediately after `parse_name`, before
    building the ordinary `ConstFetch`/postfix-call chain — parser-stage rejection, matching how the plan's
    M1 text frames this ("Rejects... every construct").
  - **(b)** Leave them as ordinary `Call` nodes and defer the rejection to name resolution (M2), since
    that's genuinely where a call's callee name is authoritative.
  Given M1's plan text explicitly lists these under **M1's** reject column, (a) is probably right — but
  confirm this doesn't conflict with anything M2 already assumes before committing to it.

## Chunk 2 plan: statements

Not yet started. Scope, gathered from the plan + ADR 0007 § 3 + ADR 0008 + ADR 0012:

- **Control flow**: `if`/`elseif`/`else`, `while`, `do`/`while`, `for`, `switch`/`case`/`default` (with
  fallthrough), `break`/`continue` with an optional integer level.
- **`foreach`, with ADR 0007 § 3.2's typed bindings**:
  `foreach (expr as type $k => type &? $v) block` — both key and value are **mandatory** typed bindings (no
  untyped `foreach ($rows as $row)` left), reference marker only ever on the value. **The grammar wrinkle
  the plan's M1 verification explicitly requires a snapshot test for**: inside a `foreach` header, `as`
  belongs to `foreach`, not to the conversion operator, so converting the *subject* needs parens —
  `foreach (($m as array<int>) as int $v)`. Chunk 1's expression parser already produces exactly this
  misparse-without-parens behavior by construction (see "`as` is parsed as a postfix operator" above) — this
  is expected, not a bug to fix; write the snapshot test to pin it down, per
  [`docs/spec/00-overview.md` § 3.2](docs/spec/00-overview.md).
- **`try`/`catch`/`finally`**, multi-type catch via `Type|Type $e`.
- **`echo expr, expr, …;`**
- **`unset(expr, …);`** (kept — this is an ordinary accepted builtin, unlike `extract`/`settype`).
- **ADR 0007 § 3.1 typed local declaration**: `type '$' identifier ('=' expr)? ';'`.
- **ADR 0007 § 3.3 destructuring statement**: bracket form with typed leaves at any nesting depth
  (`[int $a, string $b] = $pair;`, empty-slot skips, string/expr keys), plus `list(...)` as a second
  spelling with the same typed-leaf requirement (no untyped form of either). This is a **distinct**
  production from chunk 1's plain array-literal expression grammar — do not try to reuse
  `parse_array_literal_brackets`, its elements are shaped completely differently (`type '&'? '$' name`
  instead of an arbitrary expression).
- **`include`/`require`/`include_once`/`require_once`** — note these are PHP *expressions* (they return a
  value; `$x = include 'a.php';` is legal PHP), not statements. Likely belongs next to `throw`/`print` in
  the expression grammar (chunk 1's territory) rather than here — retrofit `parse_primary` alongside this
  chunk rather than adding an ExprKind. See [`docs/spec/00-overview.md` § 2](docs/spec/00-overview.md).
- **The rejects**: `goto`/labels (`Keyword::Goto` already exists in the lexer — parse `goto ident;`, reject
  with the existing `E_GOTO_UNSUPPORTED`), `global $x, $y;` (`Keyword::Global` exists — reject with the
  existing `E_GLOBAL_UNSUPPORTED`), function-scope `static $x;` (new code, see above), plus the
  `eval`/`extract`/`settype` question above.
- **Open, not yet decided where it belongs**: `type Name = TypeExpr;` (ADR 0007 § 3.5 / ADR 0015 § 5) sits
  "at file/namespace scope, alongside `use` and `namespace` — never inside a class body." Namespace/`use`
  declarations themselves aren't clearly assigned to chunk 2 or chunk 3 yet either — decide when starting
  this chunk; leaning toward: file-scope declarations (`namespace`, `use`, `type` alias) travel with chunk 3
  since they're closer in spirit to "declarations" than to executable statements, but this isn't settled.

## Chunk 3 (not started): class/interface/trait/enum declarations

Everything else the M1 milestone names: classes, interfaces, traits, enums (cases + optional backing type
only, ADR 0010), methods, attributes (`#[...]` — deliberately *not* touched in chunk 1, see its module
docs), property hooks feeding `PropertyObserver` (ADR 0014), asymmetric visibility, promoted constructor
parameters (the `Param.modifiers` field already exists in `ast.rs` for this — chunk 1 parses the modifiers
generically on every parameter; restricting them to constructors is presumably a later semantic check, not
a chunk-3 parser change), `insteadof` without `as` (ADR 0015), rejecting `class_alias`/import `as`/trait-use
`as`, rejecting a `function`/`const` outside a class body and a class/namespace literally named `Core`
(ADR 0011).

## Verification still open for all of M1

From the plan: `mwl ast file.mwl` dumping the AST (no CLI crate exists yet — `mwl-cli` isn't in the
workspace per `.claude/brief.sh`'s "what exists on disk" listing, so this needs the crate scaffolded, not
just a function), `cargo fuzz` on the parser finding no panics in a 1h run, and parsing the full local PHP
8.5 install's `.php` files without crashing (not *checking* — that's M2). None of this is started.

## Housekeeping

- ADR 0009's grapheme-segmentation cost guard test (`benches/abi-probe`) is independent of all of this and
  can slot in whenever — see the plan's status block.
- Once chunks 2 and 3 land and the verification above is done, M1 closes and update
  [`docs/implementation-plan.md`](docs/implementation-plan.md)'s status block accordingly — that's the one
  place "where the plan stands" actually lives; don't let this file become a second copy of it.
