# Next session prompt

Continue MWL. M1 (front end) is done except for one new pending item (see below). M2 (HIR/types/IR)
is in progress — see git history for detail on how earlier items landed; it's not repeated here per
CLAUDE.md's "state a fact once" rule.

**Last session was docs-only: [ADR 0033](docs/adr/0033-secret-qualifier-for-confidential-values.md)
was accepted**, adding a `secret` compile-time qualifier for confidential values (passwords, API
keys) — independent of and composable with ADR 0024's `tainted` (`secret tainted string` is valid;
`secret` must come first). No code changed. Read the ADR itself for the full design (grammar,
propagation, sinks, `Core\Secret::reveal()`); the one-paragraph version: erased before codegen like
`tainted`, but unlike `tainted` it has **no ambient source** (nothing in MWL is host-populated, so a
value is only ever `secret` where a developer spells it), and it's refused by default at HTML/response
output (no auto-escape bypass, unlike `tainted`), `Core\Log` (opposite default from `tainted`, which
wants attacker input logged), debug-dump output (`var_dump`/`print_r` show a redaction placeholder,
amending ADR 0028 § 4), `Throwable` messages, and `serialize()`/the `spawn worker`/`spawn script`
boundary (one refusal, both callers, per ADR 0023's unification). Cross-links landed in ADRs 0007,
0020, 0024, 0028, `docs/adr/README.md`, `CLAUDE.md`'s two tables, and the plan's M1/M2/M4/M5/M8
paragraphs — nothing else should need touching for the decision itself.

**Two known rough edges the ADR names explicitly, not fixed, just flagged for whoever implements it:**
checked `as` conversions strip `secret` the same way they strip `tainted`, even though — unlike
`tainted` — proving a value's shape says nothing about its confidentiality (accepted for grammar
consistency; see the ADR's *Alternatives rejected*/*Revisiting*); and `Core\Log`'s `fields:
array<string, mixed>` parameter needs call-site argument inspection to catch a `secret` operand, since
its declared type is deliberately open and a parameter-type refusal (the mechanism every other
`tainted` sink uses) doesn't apply there.

**Deliberately out of scope for this slice — the next thread to pick up, in the order that makes
sense to attempt them:**

1. **ADR 0033's own M1 grammar addition** — a new `Keyword::Secret`, and a new grammar production
   alongside `tainted`'s existing `TaintedString`/`TaintedBytes` atoms in `mwl-syntax`, handling all
   four combinations (`string`, `tainted string`, `secret string`, `secret tainted string`) plus the
   `tainted secret string` (wrong order) diagnostic. Worth doing in the same pass as item 2 below if
   both land in the same session, since they'll share qualifier-handling code paths in `mwl-types`.
2. **ADR 0027 (`callable` value-shape), ADR 0014's interplay with a typed (non-`$this`) receiver,
   ADR 0024 §§ 2-3 (tainted propagation/laundering) — and now ADR 0033 §§ 2-4 (secret propagation,
   checked-conversion laundering, and its `Markup`/`Throwable`-message sink refusals) once item 1
   above lands — and ADR 0010 (enum-vs-class atom distinction beyond "resolves to *a* symbol" —
   enum-specific operations).** Each is its own self-contained checker-side rule layered on top of the
   type table, signature table, `ClassGraph`, and the "reserved global interface" pattern ADR
   0013/0028 both used — none of them depend on each other, so they can land in any order or be split
   across sessions.
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
3. **A pre-existing parser bug, surfaced but not fixed a couple of sessions ago:** `(string)$x;`
   written as its own bare statement — no enclosing assignment — mis-parses as a `LocalDecl`
   redeclaring `$x` with a parenthesized type, rather than as a cast expression-statement (reproduce
   with `mwl check` on a two-line file: `Foo $a = new Foo(); (string)$a;` reports `$a` already
   declared). `mwl-syntax`'s statement-vs-declaration lookahead doesn't yet disambiguate a
   parenthesized legacy-cast prefix (`(string)`, `(int)`, ...) from a parenthesized *type* prefix in
   statement position. Worth fixing in the same session as item 1 if `mwl-syntax` is already open.
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

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done. The ADR 0007, 0013, 0022 and 0028 corpus entries are now all
satisfied; the rest of that Verify line (ADR 0010, 0014, 0024, 0027's own corpus entries, plus IR
snapshot tests, plus ADR 0033's own entries added this session) still depends on the work items
above, or on `mwl-ir`, which hasn't started.
