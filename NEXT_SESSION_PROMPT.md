# Next session prompt

Continue MWL. M1 (front end) is done except for one in-progress item (see below, currently
non-compiling — fix it first). M2 (HIR/types/IR) is in progress — see git history for detail on how
earlier items landed; it's not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session was housekeeping, not milestone work: PHP 8.5's pipe operator (`|>`) was considered and
rejected.** No grammar work for it had ever been started (confirmed by search before touching anything),
so there was nothing to revert — this only *records* the decision. It's pure call-chain sugar
(`$x |> f(...) |> g(...)` is just `g(f($x))`) with no expressiveness a nested call or a local variable
doesn't already give, and it undercuts its own usual justification here: ADR 0011 makes every function a
method, so idiomatic MWL code already reaches for `->` chaining instead of the free-function nesting `|>`
exists to flatten in vanilla PHP. Documented as a bullet in `crates/mwl-syntax/src/lib.rs`'s "Deliberately
rejected" module-doc list (same tier as PHP's alternative colon syntax — no ADR file, since there's no
subtle/contested reasoning here), and added to the plan's PHP-compatibility exclusion row. Committed as
`3153466`.

**Correction to this file's own prior claim:** ADR 0033's M1 grammar addition (`Keyword::Secret`, the
`Secret*`/`SecretTainted*` atoms, `E_SECRET_TAINTED_ORDER`/`E_SECRET_NON_SCALAR`) was already landed
(commit `adebebb`, before last session) — an earlier version of this prompt said it was still pending and
was never refreshed after it landed. It is done; do not redo it.

**The one real open thread — ADR 0036 §§ 2-3's grammar addition is mid-flight, uncommitted, and the
working tree currently fails to build.** What's there:

- `crates/mwl-syntax/src/ast.rs` — `TypeAtom::Shape(Vec<ShapeField>)`, `ShapeField`, `ObjectLiteralField`,
  and `ExprKind::ObjectLiteral(Vec<ObjectLiteralField>)` are all added.
- `crates/mwl-diagnostics/src/lib.rs` — the three new codes are stubbed: `E_OBJECT_LITERAL_NEEDS_PARENS`
  (E0117), `E_OBJECT_LITERAL_SHORTHAND` (E0118), `E_OBJECT_LITERAL_COMPUTED_KEY` (E0119).
- `crates/mwl-syntax/src/parser.rs` — `parse_shape_type` (the **type-position** `{name: T, ...}` shape) is
  written and wired into `parse_type_atom`/`token_starts_type`.

What's missing, in order:

1. **Fix the build first**: `parser.rs` calls `ShapeField { .. }` without importing it —
   `error[E0422]: cannot find struct ... ShapeField`. One line (`use crate::ast::ShapeField;` or qualify
   it), confirmed via `cargo build -p mwl-syntax` last session.
2. **The value-literal side has no parser code at all yet**: no `parse_object_literal_expr`, and nothing
   dispatches `{` to it from primary-expression parsing. ADR 0036 § 2's shape: `{name: value, ...}`, no
   shorthand (→ `E_OBJECT_LITERAL_SHORTHAND`), no computed key (→ `E_OBJECT_LITERAL_COMPUTED_KEY`), at
   least one field (an empty `{}` parses as a block, never `ExprKind::ObjectLiteral` — this is what keeps
   it from colliding with a block statement).
3. **The two disambiguation diagnostics ADR 0036 § 2 names aren't implemented**: `fn() => {...}` already
   means a block body (`parse_fn_expr`) and a statement-initial `{` already means a block statement
   (`parse_statement_inner`) — both need the same parenthesize-to-force-expression fix JavaScript uses
   (`fn() => ({a: 1})`), reported via `E_OBJECT_LITERAL_NEEDS_PARENS` when a bare `{` in either position
   looks like it was meant as a literal (i.e. `name:` follows the opening brace).
4. Round-trip tests for all of the above (declaration slots + the ambiguity cases) before calling this M1
   item done — mirror the coverage `tainted`/`secret`'s own grammar additions already have.

**Backlog after ADR 0036's grammar lands (unchanged from before, still in this order):**

1. **ADR 0027 (`callable` value-shape), ADR 0014's interplay with a typed (non-`$this`) receiver,
   ADR 0024 §§ 2-3 (tainted propagation/laundering), ADR 0033 §§ 2-4 (secret propagation, checked-conversion
   laundering, `Markup`/`Throwable`-message sink refusals), ADR 0010 (enum-vs-class atom distinction beyond
   "resolves to *a* symbol"), and ADR 0036's own checker semantics (`object`'s real subtyping, the shape
   type's structural width-subtyping check, extending ADR 0014 § 5's diagnostic path to an erased
   `object`/shape view).** Each is a self-contained checker-side rule layered on the type table, signature
   table, `ClassGraph`, and the "reserved global interface" pattern ADR 0013/0028 both used — none depend on
   each other, so they can land in any order or be split across sessions.
   - **ADR 0014's typed-receiver gap**: re-read `mwl_hir::members`'s own known-gap note against what
     `expr.rs`'s `check_property_access` does today — it already reports `E_UNKNOWN_MEMBER` for a
     non-`$this` receiver's missing property, so check first whether this is "give it ADR 0014's own
     wording" rather than "implement from scratch."
   - Note for whichever lands first: `expr::class_of_ctx` always interns `Ty::Class(qname)` for
     `self`/`static`/`$this`, even for an enum — ADR 0010's item needs to distinguish via
     `SymbolTable`'s `SymbolKind` first.
2. **`switch`/`try` definite-assignment precision, and `parent` as a *type* atom** (`parent $x`) — both
   conservative-but-safe gaps, worth revisiting once the higher-value items above are done.
3. **Smaller independent polish items:** a class constant's type (`Class::CONST` stays `mixed`, no
   const-value type table yet); a promoted constructor-parameter property is recorded as neither a property
   nor a definite-assignment obligation anywhere; a named/spread call argument disables all per-argument
   checking for that call; a class with no explicit `constructor` isn't held to a zero-arity check on
   `new Foo(...)` nor the `parent::constructor(...)` obligation; a `set`-hook-backed property is exempted
   from ADR 0022 entirely rather than verified; `implements Comparable`/`Stringable` is never checked for
   actually declaring the matching method (needs general interface-method-completeness checking, which
   doesn't exist yet — likely its own small design decision first).
4. **ADR 0035's runtime side has no code yet** (`mixed`/union-typed condition's dynamic truthiness
   dispatch) — arrives with M3's first backend; nothing to do until `mwl-ir`/codegen exist.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done. ADR 0007, 0013, 0022, 0028's corpus entries are satisfied; the rest (ADR
0010, 0014, 0024, 0027's own entries, IR snapshot tests, ADR 0033 and ADR 0036's own entries) still depends
on the work above, or on `mwl-ir`, which hasn't started.
