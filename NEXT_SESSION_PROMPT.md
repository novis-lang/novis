# Next session prompt

Continue MWL. M1 (front end) is now fully done, including every grammar item discovered after the
milestone was first reported complete. M2 (HIR/types/IR) is in progress — see git history for detail
on how earlier items landed; it's not repeated here per CLAUDE.md's "state a fact once" rule.

**Note: another session ran concurrently against this same working directory during the work below**
(its commits — `3153466` documenting PHP 8.5's pipe operator as deliberately unparsed, and `3adc789`
refreshing this file — sit interleaved with the two commits described here). No conflict resulted since
the two sessions touched non-overlapping files at each point in time, but if this file ever again
describes state that doesn't match `git log`/`sh .claude/brief.sh`, trust the repository over the prose
and treat the mismatch as a sign two sessions raced — don't assume the file is simply stale by one
session's own hand.

**Last session landed both of M1's remaining pending grammar items, back to back, since they touch
different parts of the grammar and share no code:**

1. **[ADR 0033](docs/adr/0033-secret-qualifier-for-confidential-values.md) § 1's `secret` qualifier**
   (commit `adebebb`) — a new `Keyword::Secret`, and `SecretString`/`SecretBytes`/`SecretTaintedString`/
   `SecretTaintedBytes` atoms alongside the existing `TaintedString`/`TaintedBytes` pair. `parse_type_atom`'s
   `Secret` arm recurses into `parse_type_atom` for its operand exactly as the `Tainted` arm already did, so
   `secret tainted string` composes for free; the wrong order (`tainted secret string`) is caught by
   teaching the `Tainted` arm to recognize an already-`Secret*` inner atom and report
   `E_SECRET_TAINTED_ORDER` (`E0116`) instead of the generic `E_TAINTED_NON_SCALAR`. A `secret`-qualified
   non-scalar gets `E_SECRET_NON_SCALAR` (`E0115`). `mwl-types`' `lower_atom` falls through its existing
   wildcard arm to `mixed` for all four new atoms — propagation/laundering/sink rules (ADR 0033 §§ 2-4) are
   unstarted.
2. **[ADR 0036](docs/adr/0036-anonymous-object-shapes.md) §§ 2-3's anonymous object literal and inline
   shape type** (commit `6ecacd5`) — `{name: value, ...}` as a new primary expression
   (`ExprKind::ObjectLiteral`), and `{name: T, ...}` wired into `token_starts_type`/`parse_type_atom` as
   `TypeAtom::Shape`, so it composes for free with unions/intersections/`array<T>`. No shorthand field or
   computed key (`E_OBJECT_LITERAL_SHORTHAND`/`E_OBJECT_LITERAL_COMPUTED_KEY`, `E0118`/`E0119`). The two
   grammar collisions the ADR names — `fn() => {...}` already meaning a block body (ADR 0031), and a
   statement-initial `{` already meaning a block — are resolved by a one-token-past-`{` lookahead
   (`{ ident :`) at exactly those two call sites: a match still parses the literal (so its own
   shorthand/computed-key diagnostics fire) but discards the result as `ExprKind::Error` behind
   `E_OBJECT_LITERAL_NEEDS_PARENS` (`E0117`), the same diagnose-then-`Error`-recover shape the legacy-cast
   rejection already uses. An empty `{}` never matches that lookahead, so it stays an ordinary empty block
   at both sites. `mwl-syntax::casing` gained an `ObjectLiteral` arm: field names are ordinary property
   names per the ADR, so they get the same camelCase/no-leading-underscore check a class property does.
   `mwl-types`' `lower_atom` again falls through to `mixed` for `TypeAtom::Shape` — `object`'s real
   subtyping and the shape's structural check (ADR 0036 §§ 1, 3-4) are unstarted.

**Known gap opened by item 2, documented in `mwl-syntax`'s module docs rather than silently left to be
rediscovered:** a local variable declaration typed with a *bare* inline shape type (`{x: int} $point;`)
does not parse — statement-initial `{` already commits to a block before any type-prefix lookahead would
run, and unlike the object-literal collision this ADR names and this session resolved, disambiguating a
*type* prefix from a block needs lookahead past a matched, possibly-nested `{...}` all the way to a
following `$name`, which wasn't attempted. Every other declaration slot (parameter, return type, property,
class constant, `foreach` binding) supports a bare shape type fine; the workaround is the same named-alias
spelling the ADR's own example uses: `type Point = {x: int}; Point $point;`. Revisit only if this proves to
be real friction, not preemptively.

Both items were verified with `cargo build --workspace`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check`, `cargo test --workspace` (183 `mwl-syntax` lib tests, up from 174), and
`cargo test -p mwl-syntax --test corpus_parse` (still zero panics against the full local `php-src`
checkout) — all clean. The 5-minute WSL `cargo fuzz` re-run itself was **not** repeated this session
(same scoping call the `secret` grammar landing made): nothing suggests either addition changed the
lexer/parser's panic-safety, and the fuzz targets exercise arbitrary byte input rather than this specific
new grammar, so it's reasonable to skip per-slice rather than after every keyword/production added — worth
doing before M1 is *next* declared "verified" for a release-shaped reason, not before every commit.

**Deliberately out of scope for this slice — the next thread to pick up, in the order that makes
sense to attempt them:**

1. **ADR 0027 (`callable` value-shape), ADR 0014's interplay with a typed (non-`$this`) receiver,
   ADR 0024 §§ 2-3 (tainted propagation/laundering), ADR 0033 §§ 2-4 (secret propagation, checked-conversion
   laundering, and its `Markup`/`Throwable`-message sink refusals — now unblocked, since its M1 grammar
   landed last session), ADR 0010 (enum-vs-class atom distinction beyond "resolves to *a* symbol" —
   enum-specific operations), and ADR 0036's checker semantics (also now unblocked): `object`'s real
   subtyping (every named or literal-synthesized class type `<: object`), the shape type's structural check
   (width subtyping + ordinary field assignability), and extending ADR 0014 § 5's diagnostic path to fire
   for a read/write through an erased `object`/shape view.** Each is its own self-contained checker-side
   rule layered on top of the type table, signature table, `ClassGraph`, and the "reserved global interface"
   pattern ADR 0013/0028 both used — none of them depend on each other, so they can land in any order or be
   split across sessions.
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
   - Note for ADR 0036's item specifically: `mwl-types`' `lower_atom` currently maps every
     `TypeAtom::Shape` straight to `mixed` via its wildcard arm (same for the four `Secret*` atoms above) —
     this item is where `Ty` actually needs a real shape/field representation, not just a diagnostic tweak.
2. **`switch`/`try` definite-assignment precision, and `parent` as a *type* atom** (`parent $x` —
   distinct from `new parent(...)`, which resolves since several sessions ago), plus the equivalent
   precision gap `ctor_init.rs` shares with `locals.rs` (both conservatively contribute nothing
   through `switch`/`try`'s body and catches) — all named as known gaps for a while now, all
   safe-but-imprecise today (reject a few extra valid programs rather than ever accepting an invalid
   one), worth revisiting once the higher-value items above are done rather than before.
3. **Smaller, independent polish items surfaced across the last few sessions, any of which could be
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
4. **ADR 0035's runtime side has no code yet** — a `mixed`/union-typed condition's dynamic truthiness
   dispatch (the full table: `"0"` vs `"0.0"`, `-0.0`, `NAN`, empty-vs-non-empty array regardless of
   element type, an enum case backed by `0` staying truthy) arrives with M3's first backend, per that
   ADR's *Verification*. Nothing to do here until `mwl-ir`/codegen exist — noted so it isn't
   rediscovered as a surprise gap later.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done. The ADR 0007, 0013, 0022 and 0028 corpus entries are now all
satisfied; the rest of that Verify line (ADR 0010, 0014, 0024, 0027's own corpus entries, plus IR
snapshot tests, plus ADR 0033 and ADR 0036's own entries) still depends on the work items above, or on
`mwl-ir`, which hasn't started.
