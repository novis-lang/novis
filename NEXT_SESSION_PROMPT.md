# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — run `sh .claude/brief.sh` first,
then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file only
points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact once").

**Last session closed [ADR 0036](docs/adr/0036-anonymous-object-shapes.md)'s M2 item** (§§ 1, 3-4's
checker semantics, on top of the M1 grammar landed two sessions ago): `object` carries real subtyping
(every class or shape type is `<: object`), a new `Ty::Shape` is checked structurally by width subtyping
plus ordinary field assignability, `ExprKind::ObjectLiteral` is now typed (the exact-fields shape its
initializers infer), a `type` alias naming a shape resolves for free, and a property access through a
shape-missing field or plain `object` is silently erased to `mixed` rather than diagnosed — deferred to
ADR 0014 § 5's runtime-checked-throw fallback, which needs M3/M4's IR/codegen to exist before it can throw
from anything. All in `crates/mwl-types` (`ty.rs`, `lower.rs`, `expr.rs`); 16 new tests. See
`docs/implementation-plan.md`'s M2 paragraph for the full account.

**Known gap opened, not attempted:** ADR 0028 § 3's `unset()`-on-a-declared-property refusal was not
extended to a shape-typed receiver's own fields — only an ordinary class property triggers it. Neither ADR
asks for this; revisit only if real code shows it's actually wanted.

**Next, in the order that makes sense to attempt — independent, can land in any order or be split across
sessions:**

1. **ADR 0010** (enum-vs-class atom distinction beyond "resolves to *a* symbol" — enum-specific
   operations; note `expr::class_of_ctx` always interns `Ty::Class` for `self`/`static`/`$this` even
   inside an enum, needs fixing first via `SymbolTable`'s `SymbolKind` the way `lower.rs`'s
   `resolve_name_type` already does).
2. **ADR 0014's typed-receiver gap** — re-check `mwl_hir::members`'s own known-gap note against what
   `expr::check_property_access` does today; it may already be closed in substance and just need the
   ADR's own diagnostic/wording, not new logic.
3. **ADR 0024 §§ 2-3** (`tainted` propagation/laundering) and **ADR 0027** (`callable` value-shape
   checking) — grammar for both landed in M1; checker-side is untouched.
4. **ADR 0033 §§ 2-4** (`secret` propagation, checked-conversion laundering, `Markup`/`Throwable`-message
   sink refusal) — its M1 grammar landed; `lower_atom` still maps all four `Secret*` atoms to `mixed`.
5. **`switch`/`try` definite-assignment precision** (both bodies conservatively contribute nothing today
   — safe, just imprecise) and **`parent` as a *type* atom** (`parent $x`, distinct from the already-working
   `new parent(...)`) — long-standing, low-value-until-the-above-lands gaps.
6. **Smaller independent polish**, any one a quick follow-up: a class constant's type (`Class::CONST` is
   always `mixed`); a promoted constructor-parameter property (tracked as neither a property nor a
   definite-assignment obligation — mirrors a `mwl_hir::members` gap, likely fix both together); a
   named/spread call argument disables all per-argument checking for that call; a class with no explicit
   `constructor` isn't held to a zero-arg arity check on `new`, nor to the `parent::constructor(...)`
   obligation; a `set`-hooked property is exempted from ADR 0022's check entirely rather than verified
   against the hook's body; `implements Comparable`/`Stringable` is never checked for actually declaring
   `compareTo`/`toString` (needs general interface-method-completeness checking, which doesn't exist yet —
   likely its own small design decision first).
7. **ADR 0035's runtime side** — no code yet, and none is expected until M3's first backend exists.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs.
ADR 0007, 0013, 0022, 0028, and now 0036's own entries are satisfied; the rest (0010, 0014, 0024, 0027,
0033's own entries, plus IR snapshot tests) depends on the work above or on `mwl-ir`, unstarted.
