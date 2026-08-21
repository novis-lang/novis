# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — see git history for
detail on how earlier items landed; it's not repeated here per CLAUDE.md's "state a fact once"
rule.

**Last session closed item 1's `Stringable`/`unset()` entries**: [ADR 0028](docs/adr/0028-closing-the-remaining-magic-methods.md)
§§ 1 and 3 — reusing [ADR 0013](docs/adr/0013-comparable-interface.md)'s `Comparable` shape end to
end, exactly as the prior prompt suggested. Concretely:

- [`crates/mwl-hir/src/qname.rs`](crates/mwl-hir/src/qname.rs) — `QName::is_reserved_global_interface`
  now recognizes `"Stringable"` alongside `"Comparable"`.
- [`crates/mwl-diagnostics/src/lib.rs`](crates/mwl-diagnostics/src/lib.rs) — two new codes:
  `E_STRINGABLE_REQUIRED` (`E0412`), `E_UNSET_ON_PROPERTY` (`E0413`).
- [`crates/mwl-types/src/expr.rs`](crates/mwl-types/src/expr.rs):
  - `require_stringable(ty, span, env)`: refuses a `Ty::Class` operand that doesn't provably
    implement `Stringable` via `mwl_hir::implements_interface` (same reachability walk ADR 0013
    added), skipping an unmodeled `Core` class and any non-`Ty::Class` operand — identical scoping
    to `object_comparison_result`. Wired into every implicit string-conversion site: `Interpolated`
    (each part), `Binary`'s `Concat` arm (both operands), `Print`, `Cast{ty: CastType::String, ..}`
    (the legacy `(string)` spelling), and `Conversion` when the lowered target type is `Ty::String`
    (the `as string` spelling). `locals.rs`'s `StmtKind::Echo` arm calls it too (split out of the
    old combined `Echo | Unset` arm — see below).
  - `ExprKind::PropertyAccess`'s existing existence-checking logic was factored out of `infer` into
    a new `check_property_access(object, property, is_unset, ...)`, parameterized by whether it's
    being reached as `unset()`'s operand. When `is_unset` and the property *does* resolve (i.e. it's
    actually declared), it reports `E_UNSET_ON_PROPERTY` instead of just returning the property's
    type; the not-found branch (`E_UNKNOWN_MEMBER`/deferring to `mwl_hir`'s `E_UNDEFINED_PROPERTY` for
    `$this`) is untouched. `check_unset_target(expr, ...)` is the new public entry point `locals.rs`
    calls for each of `unset()`'s operands — it only special-cases a `PropertyAccess` operand and
    falls back to plain `check_expr` for everything else (an array element, a local variable), since
    ADR 0028 § 3 scopes the refusal to object properties only.
- [`crates/mwl-types/src/locals.rs`](crates/mwl-types/src/locals.rs) — `StmtKind::Echo(xs) |
  StmtKind::Unset(xs)` was one arm before; now `Echo` calls `require_stringable` on each operand's
  type and `Unset` calls the new `check_unset_target` on each operand.
- [`docs/implementation-plan.md`](docs/implementation-plan.md)'s M2 paragraph updated: the prior
  ADR 0013 paragraph re-labeled ("the session before last"), this session's ADR 0028 paragraph
  added, and the "known gaps left" list's ADR reference list dropped `0028` (its M2-relevant scope
  is now closed; §§ 2/4/5/6 need no code change at all, or wait on M4/M11 as the ADR's own
  Verification section already says).

Full workspace `cargo test` (7 new tests in `mwl-types::check`, 1 new in `mwl-hir::qname` — 75 tests
total in `mwl-types`, 70 in `mwl-hir`, everything else still green), `cargo clippy --all-targets --
-D warnings`, and `cargo fmt --check` are all clean. The CLI was also smoke-tested by hand: `echo
$a;` for a non-`Stringable` `Foo $a` produced `E0412` naming `Stringable`; the same on a `Name
implements Stringable` with a real `toString` produced no diagnostics; `unset($a->x)` for a declared
`public int $x` produced `E0413` naming ADR 0022's guarantee.

**A pre-existing parser bug surfaced (not fixed) while writing this session's tests:** `(string)$x;`
written as its own bare statement — no enclosing assignment — mis-parses as a `LocalDecl`
redeclaring `$x` with a parenthesized type, rather than as a cast expression-statement (reproduce
with `mwl check` on a two-line file: `Foo $a = new Foo(); (string)$a;` reports `$a` already
declared). `mwl-syntax`'s statement-vs-declaration lookahead doesn't yet disambiguate a
parenthesized legacy-cast prefix (`(string)`, `(int)`, ...) from a parenthesized *type* prefix in
statement position; both look identical for one token of lookahead. This session's own tests work
around it by always assigning the cast's result (`string $s = (string)$a;`), which is unambiguous.
**Whoever picks up parser work next should fix this** — it's an M1-scope grammar ambiguity that
happened to go unexercised until a checker-side test needed a bare cast statement.

**Deliberately out of scope for this slice — the next thread to pick up, in the order that makes
sense to attempt them:**

1. **The parser bug above**, if the next session is touching `mwl-syntax` at all — otherwise it's
   fine to leave as a known gap a bit longer, since every real cast site so far happens to be inside
   a larger expression.
2. **ADR 0027 (`callable` value-shape), ADR 0014's interplay with a typed (non-`$this`) receiver,
   ADR 0024 §§ 2-3 (tainted propagation/laundering), ADR 0010 (enum-vs-class atom distinction beyond
   "resolves to *a* symbol" — enum-specific operations).** Each is its own self-contained
   checker-side rule layered on top of the type table, signature table, `ClassGraph`, and the
   "reserved global interface" pattern ADR 0013/0028 both used — none of them depend on each other,
   so they can land in any order or be split across sessions.
   - **ADR 0014's typed-receiver gap**: re-read `mwl_hir::members`'s own known-gap note (a typed
     local/chained-call-result/`new Foo()` receiver's missing property "needs `mwl-types`' static
     types") against what `expr.rs`'s `check_property_access` does today (formerly the
     `PropertyAccess` arm) — it already reports `E_UNKNOWN_MEMBER` for a non-`$this` receiver's
     missing property, so check first whether this item is actually "give it ADR 0014's own
     diagnostic/wording" rather than "implement the check from scratch," before assuming there's a
     full gap to close.
   - Note for whichever of these lands first: `expr::class_of_ctx` currently always interns
     `Ty::Class(qname)` for `self`/`static`/`$this`, even when the enclosing declaration is an
     enum — ADR 0010's own item will need to fix that (distinguish via `SymbolTable`'s
     `SymbolKind`, the same check `lower.rs`'s `resolve_name_type` already does for an ordinary
     type position) before enum-specific operations can tell `self` apart from a class.
3. **`switch`/`try` definite-assignment precision, and `parent` as a *type* atom** (`parent $x` —
   distinct from `new parent(...)`, which resolves since several sessions ago), plus the equivalent
   precision gap `ctor_init.rs` shares with `locals.rs` (both conservatively contribute nothing
   through `switch`/`try`'s body and catches) — all named as known gaps for a while now, all
   safe-but-imprecise today (reject a few extra valid programs rather than ever accepting an invalid
   one), worth revisiting once the higher-value items above are done rather than before.
4. **Smaller, independent polish items surfaced across the last few sessions, any of which could be
   a quick follow-up on its own:** a class constant's type (`Class::CONST` stays `mixed` regardless
   of receiver — there's no const-value type table yet); a promoted constructor-parameter property
   (`function constructor(public int $x) {}`) is recorded as neither a property nor a
   definite-assignment obligation anywhere — this mirrors a pre-existing `mwl_hir::members` gap, so
   fixing it well might mean fixing both crates together, `signatures.rs`, and `ctor_init.rs` in the
   same pass; a named or spread call argument disables *all* per-argument checking for that call
   rather than being matched positionally where possible; a class with no explicit `constructor` is
   not held to a zero-argument arity check on `new Foo(...)`, nor to the
   `parent::constructor(...)` obligation `ctor_init.rs` checks for a class that does declare one; a
   property backed by a `set` hook is exempted from ADR 0022's check entirely rather than verified
   against whether the hook's own body actually commits a value; a class claiming
   `implements Comparable` is never checked for actually declaring a matching `compareTo` — see
   ADR 0013's own known-gap note in the plan (this needs general interface-method-completeness
   checking, which doesn't exist for any interface yet — likely its own small design decision
   before it's worth building, not a quick fix); the same gap now also applies to `Stringable`'s
   `toString` for the identical reason.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done. The ADR 0007, 0013, 0022 and 0028 corpus entries are now all
satisfied; the rest of that Verify line (ADR 0010, 0014, 0024, 0027's own corpus entries, plus IR
snapshot tests) still depends on the work items above, or on `mwl-ir`, which hasn't started.
