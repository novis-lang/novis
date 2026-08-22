# ADR 0045 — PHP's `and`/`or`/`xor` keyword operators are rejected; `&&`/`||` are the only logical connectives

- **Status:** Accepted
- **Date:** 2026-08-22
- **Scope:** PHP's low-precedence logical keyword operators (`and`, `or`, `xor`); the corresponding AST
  variants (`BinaryOp::LowAnd`/`LowOr`/`LowXor`) and diagnostic. `&&`, `||`, and `!` are untouched.
- **Amends:** none. `and`/`or`/`xor` were parsed but never lowered past `mwl-types` — every reachable
  attempt to run one already panicked naming the gap (`mwl-ir`'s `lower_expr_top` doc comment and its crate
  doc's known-gaps list). This ADR closes that gap by removing the construct rather than implementing it.
- **Amended by:** none.
- **Relates to:** [0035](0035-truthy-boolean-context.md) (names exactly six syntax positions —
  `if`/`while`/`for`/`?:`/`&&`/`||`/`!` — as the truthy-table exception; `and`/`or`/`xor` were never among
  them), [0034](0034-legacy-cast-syntax-rejected.md) and
  [0021](0021-single-file-inclusion-construct.md) (the same shape: collapse several spellings of one
  behaviour to one, reject the rest at parse time naming the survivor), [0015](0015-no-name-aliasing.md)
  (nothing gets a second runtime-reachable name — extended here to a second *operator* spelling).

> **In short:** `and`, `or`, and `xor` no longer parse as operators. Each is a parse-time diagnostic
> (`E0226`) — `and`/`or` name `&&`/`||` as the exact replacement; `xor` has none, since MWL has no `^^`
> operator, and the diagnostic says so rather than inventing one. `&&` and `||` are now the *only* spelling
> for logical AND/OR in MWL, at every precedence — PHP's habit of keeping both a symbol and a word form for
> the same connective, inherited from Perl, is not carried forward.

## Context

- PHP's `and`/`or`/`xor` are not synonyms for `&&`/`||` — they bind *looser* than `=`, a precedence PHP
  itself inherited from Perl (`open($f) or die;` relies on `or` binding after the assignment so the whole
  call, not just its right operand, is what `die` guards). That precedence gap is a well-known footgun: PSR-12
  and most PHP style guides recommend `&&`/`||` over the word forms for exactly this reason, and real-world
  PHP code is overwhelmingly written with the symbols — the word forms appear almost nowhere outside of
  legacy code and this specific idiom.
- MWL already declined to inherit the idiom that motivates keeping them. Priority 2 is PHP-*compatible
  observable behaviour*, not PHP-*identical syntax* — the same distinction that let ADR 0034 drop `(int)$x`
  and ADR 0027 drop string/array callables. A precedence trap with no counterbalancing benefit is not
  behaviour worth preserving.
- The status quo was already a half-built feature, not a working one: `mwl-syntax` parses `and`/`or`/`xor`
  into `BinaryOp::LowAnd`/`LowOr`/`LowXor`, and `mwl-types` type-checks them to `bool` — but `mwl-ir`
  deliberately never lowers them ([`lower_expr_top`'s doc comment](../../crates/mwl-ir/src/lower.rs) names
  ADR 0035 as covering only `&&`/`||`/`!`, and the crate's own known-gaps list repeats this). Any program
  that reached one at codegen already hit `mwl-ir`'s "arithmetic/equality/ordering operators" panic. Nothing
  observable is lost by rejecting them outright; a category of previously-reachable panic is closed instead.
- Two spellings of one operation, with different precedence to boot, is exactly the shape
  [ADR 0015](0015-no-name-aliasing.md) and [ADR 0021](0021-single-file-inclusion-construct.md) already rule
  against elsewhere.

## Decision

**`and`, `or`, and `xor` are rejected at parse time, in every position.** `mwl-syntax` still recognizes each
keyword where an infix logical operator would go, so it can name the exact fix, but produces
`ExprKind::Error` — the AST loses `BinaryOp::LowAnd`, `BinaryOp::LowOr`, and `BinaryOp::LowXor` entirely,
the same way ADR 0034 dropped `ExprKind::Cast`.

### 1. `&&`/`||` are the only surviving logical connectives

There is no second, lower-precedence spelling of AND or OR. `$a && $b` and `$a || $b` — already the sole
short-circuit truthy positions ADR 0035 names — are now also the *only* way to spell those connectives at
all. `!` is untouched; it never had a keyword sibling to begin with.

### 2. `xor` has no replacement, and the diagnostic says so

Unlike `and`/`or`, `xor` is not a duplicate spelling of an existing MWL operator — PHP has no `^^`, and
neither does MWL. Its diagnostic does not name a one-token fix; it names the two general rewrites instead:
`(a || b) && !(a && b)` for the fully general truthy case, or `a != b` when both operands are already
`bool`. Losing keyword `xor` is a real, if narrow, capability subtraction — see *Consequences*.

### 3. Diagnostic

Each keyword, in operator position, produces `E0226` naming the fix (or its absence):

```php
$a and $b      // rejected — "use `&&` instead — it is the only logical connective MWL keeps"
$a or $b       // rejected — "use `||` instead — it is the only logical connective MWL keeps"
$a xor $b      // rejected — "there is no direct replacement — write `(a || b) && !(a && b)`,
               //              or `a != b` when both operands are already `bool`"
```

The keywords stay reserved words — not freed for use as identifiers — the same way `include`/`require_once`
remain reserved purely to be diagnosed (ADR 0021 § 2).

## Consequences

**Positive**

- One spelling per logical connective, full stop — nothing to disambiguate in documentation, `mwl-fmt`, or a
  code review comment, and no PHP-inherited precedence trap to teach around.
- Closes a standing gap rather than opening one: `and`/`or`/`xor` were reachable from the parser and type
  checker but panicked in `mwl-ir`. That three-crate half-implementation (`mwl-syntax` parses it,
  `mwl-types` types it, `mwl-ir` panics on it) is deleted along with the ADR 0035-adjacent doc comments that
  existed only to explain why the gap was intentional.
- `mwl-syntax`'s `BinaryOp` loses three variants that existed only to carry a syntax choice with no
  semantics of their own (`LowAnd`/`LowOr` behave exactly like `And`/`Or`, distinguished only by which
  keyword produced them — `mwl-types` already treated all three identically, per `expr.rs`'s shared match
  arm).

**Negative**

- **Keyword `xor` has no one-token replacement**, unlike every other rejected-and-replaced construct this
  project has recorded so far (ADR 0021, ADR 0034). A PHP program using it needs a real rewrite, not a
  mechanical substitution — `mwl convert` (M11) can offer `(a || b) && !(a && b)` as its default rewrite
  (correct for any truthy operand, matching PHP's own semantics) but cannot safely narrow to `a != b`
  without knowing both operands are already `bool`.
- **A further subtraction from the "pragmatic superset" promise.** A PHP file using `and`/`or`/`xor`
  anywhere no longer parses without a rewrite pass. Smaller than it looks: real-world PHP rarely uses these
  outside the `open(...) or die;` idiom, which converts particularly cleanly (`if (!open(...)) { die; }`
  reads at least as well as the original).

## Alternatives rejected

- **Implement `and`/`or`/`xor` with PHP's precedence instead of rejecting them.** The status quo's
  unfinished half — parses, type-checks, panics in codegen. Rejected because it buys nothing: it would mean
  building and testing a whole extra lowering path (a second short-circuit shape in `mwl-ir`, at a third
  precedence tier) whose only purpose is to reproduce a footgun this project has no reason to keep.
- **Keep `and`/`or` as plain aliases for `&&`/`||` (same precedence, just a second spelling), drop only
  `xor`.** Rejected for the same reason ADR 0034 rejected keeping `(int)$x` as a permanent `as int` alias:
  a second spelling buys nothing once the semantics already match, and costs a second thing to teach
  everywhere else in the language has refused one.
- **Deprecate first, remove later.** Every other rejected-construct diagnostic in this project is a hard
  error with no warning tier (ADR 0029 § 3's reasoning applies here too); the standard library and any real
  userland code are still unwritten, the cheapest possible time to make the change outright.

## Verification

- `crates/mwl-syntax/src/parser.rs`'s `and_or_xor_keywords_are_diagnosed` test asserts `E0226` for `and`,
  `or`, and `xor` each in infix position, and that the result is `ExprKind::Error` with the right-hand
  operand still consumed (so a chained `$a and $b and $c` reports once per keyword rather than cascading
  into unrelated "expected token" errors).
- `BinaryOp::LowAnd`, `LowOr`, and `LowXor` are removed from `crates/mwl-syntax/src/ast.rs`;
  `crates/mwl-types/src/expr.rs` drops them from its `bool`-result match arm, relying on exhaustiveness
  checking (within `mwl-syntax` itself; `BinaryOp` is `#[non_exhaustive]` to every other crate, which already
  match it with a wildcard arm) to catch anything missed.
- `crates/mwl-ir/src/lower.rs` and `crates/mwl-ir/src/lib.rs` drop the doc comments explaining `and`/`or`/
  `xor` as a deliberately-out-of-scope gap — there is no longer a gap, since the construct does not parse.
