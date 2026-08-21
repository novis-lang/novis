# Next session prompt

Continue MWL. M1 (front end) is done except for two new pending items (see below). M2 (HIR/types/IR)
is in progress — see git history for detail on how earlier items landed; it's not repeated here per
CLAUDE.md's "state a fact once" rule.

**Last session was a design decision only, no implementation yet: [ADR 0036](docs/adr/0036-anonymous-object-shapes.md)
was accepted.** It answers "can MWL offer something like PHP's `stdClass`" without reopening ADR 0014's
no-dynamic-properties rule: `object` (already a reserved keyword/atom in `mwl-syntax`/`mwl-types`, but with
no subtyping code anywhere yet) becomes the real opaque top of every class type, named or anonymous;
`{a: 1, b: 2}` builds an anonymous, methodless instance of a compiler-synthesized class with no constructor
needed (the literal itself satisfies ADR 0022's definite-assignment obligation); and an inline
`{name: T, ...}` shape type in type position is a structural, width-subtyped, compile-time-checked
constraint — MWL's one deliberate, tightly scoped exception to otherwise fully nominal typing. Reading
through the atom's real receiver-erased view (plain `object`, or a field a shape doesn't list) extends
ADR 0014 § 5's existing runtime-checked-property-name fallback to a second trigger condition, and adds a
type-check on write that § 5 never needed before. Two grammar collisions were found by reading
`mwl-syntax/src/parser.rs` directly rather than assuming: `fn() => {...}` already means a block body
(`parse_fn_expr`), and a statement-initial `{` already means a block statement (`parse_statement_inner`) —
both resolved by the same parenthesize-to-force-expression fix JavaScript already uses for the identical
ambiguity (`fn() => ({a: 1, b: 2})`). Cross-links landed in ADR 0007/0014/0015/0022/0023/0031's own
"Amends"/"Relates to" lines, `docs/adr/README.md`, `CLAUDE.md`'s two tables, and the plan's M1 and M2
paragraphs — nothing else should need touching for the decision itself. No code changed this session.

**Deliberately out of scope for this slice — the next thread to pick up, in the order that makes
sense to attempt them:**

1. **ADR 0033's own M1 grammar addition** — a new `Keyword::Secret`, and a new grammar production
   alongside `tainted`'s existing `TaintedString`/`TaintedBytes` atoms in `mwl-syntax`, handling all
   four combinations (`string`, `tainted string`, `secret string`, `secret tainted string`) plus the
   `tainted secret string` (wrong order) diagnostic. Still pending — untouched this session.
2. **ADR 0036 §§ 2-3's own M1 grammar addition** — the anonymous object-literal expression
   (`{name: value, ...}`, no shorthand, no computed key) as a new primary-expression production, and the
   inline shape type (`{name: T, ...}`) added to `token_starts_type`/`parse_type`. Include the two
   parenthesize-to-disambiguate diagnostics that ADR's § 2 names (`fn() => {...}` and a bare `{...};`
   statement) in the same pass, since they're the same lookahead problem solved the same way. Worth doing
   alongside item 1 if both land in one session — they touch different parts of the grammar and don't share
   code, but both are "M1 isn't quite done" items blocking the same milestone close-out.
3. **ADR 0027 (`callable` value-shape), ADR 0014's interplay with a typed (non-`$this`) receiver,
   ADR 0024 §§ 2-3 (tainted propagation/laundering), ADR 0033 §§ 2-4 (secret propagation, checked-conversion
   laundering, and its `Markup`/`Throwable`-message sink refusals) once item 1 above lands, ADR 0010 (enum-vs-class
   atom distinction beyond "resolves to *a* symbol" — enum-specific operations) — and now ADR 0036's checker
   semantics once item 2 above lands: `object`'s real subtyping (every named or literal-synthesized class
   type `<: object`), the shape type's structural check (width subtyping + ordinary field assignability),
   and extending ADR 0014 § 5's diagnostic path to fire for a read/write through an erased `object`/shape
   view.** Each is its own self-contained checker-side rule layered on top of the type table, signature
   table, `ClassGraph`, and the "reserved global interface" pattern ADR 0013/0028 both used — none of them
   depend on each other, so they can land in any order or be split across sessions.
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
4. **`switch`/`try` definite-assignment precision, and `parent` as a *type* atom** (`parent $x` —
   distinct from `new parent(...)`, which resolves since several sessions ago), plus the equivalent
   precision gap `ctor_init.rs` shares with `locals.rs` (both conservatively contribute nothing
   through `switch`/`try`'s body and catches) — all named as known gaps for a while now, all
   safe-but-imprecise today (reject a few extra valid programs rather than ever accepting an invalid
   one), worth revisiting once the higher-value items above are done rather than before.
5. **Smaller, independent polish items surfaced across the last few sessions, any of which could be
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
6. **ADR 0035's runtime side has no code yet** — a `mixed`/union-typed condition's dynamic truthiness
   dispatch (the full table: `"0"` vs `"0.0"`, `-0.0`, `NAN`, empty-vs-non-empty array regardless of
   element type, an enum case backed by `0` staying truthy) arrives with M3's first backend, per that
   ADR's *Verification*. Nothing to do here until `mwl-ir`/codegen exist — noted so it isn't
   rediscovered as a surprise gap later.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done. The ADR 0007, 0013, 0022 and 0028 corpus entries are now all
satisfied; the rest of that Verify line (ADR 0010, 0014, 0024, 0027's own corpus entries, plus IR
snapshot tests, plus ADR 0033 and ADR 0036's own entries) still depends on the work items above, or on
`mwl-ir`, which hasn't started.
