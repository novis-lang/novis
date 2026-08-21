# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — run `sh .claude/brief.sh` first,
then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file only
points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact once").

**Last session closed [ADR 0010](docs/adr/0010-enums-are-a-value-type.md)'s M2 item** — the enum-vs-class
atom distinction beyond "resolves to *a* symbol" — the first item on the prior queue:

- Two spots always interned `Ty::Class` for a resolved name without checking whether it was actually an
  enum: `expr::class_of_ctx` (`self`/`static`/`$this`'s type) and `lower::resolve_special` (the
  `self`/`static` *type* atom). Both now branch on `mwl_hir::SymbolTable`'s `SymbolKind`, the same check
  `lower::resolve_name_type` already made for an explicit enum name, and intern `Ty::Enum` when the
  enclosing declaration is one — relevant only because `mwl-syntax`'s parser recovers a rejected enum
  method member (`E_ENUM_MEMBER_UNSUPPORTED`) and hands its body to this checker anyway.
- `expr.rs`'s `ClassConstAccess` arm — previously `mixed` unconditionally for every `Class::CONST`-shaped
  access — now recovers `Ty::Enum` for a case access (`Status::Active`) specifically, the same
  split-by-receiver shape every other static reference in this module uses (existence already checked by
  `mwl_hir::members`, this only recovers the type). A plain class constant's type stays unmodeled `mixed`,
  a separate, already-tracked gap.
- ADR 0010 § 5's two remaining rules are now diagnostics instead of silent `mixed` fallthrough: an
  arithmetic/bitwise operator applied directly to an enum operand is `E_ENUM_ARITHMETIC_UNSUPPORTED`
  (`E0415`, new, naming `as int`/`as uint`); converting one enum to a *different* enum via `as` is
  `E_ENUM_CONVERSION_UNSUPPORTED` (`E0416`, new, naming an explicit `match`) — converting to the same
  enum, or to/from its underlying type, is untouched.
- 8 new tests in `mwl-types::check`. Verified: `cargo build`/`test`/`clippy -D warnings`/`fmt --check` all
  clean.
- **Known gap, not attempted:** `==`/`===` between two different enum types is not diagnosed — no general
  equality-operand-compatibility check exists for *any* type pair yet (not even `int` against `uint`), so
  an enum-only special case here would be inconsistent; wants its own pass once equality is checked at all.

Docs updated in the same session: `crates/mwl-types/src/lib.rs`'s "Known gaps" list (ADR 0010 moved out),
`docs/implementation-plan.md`'s M2 paragraph (new landed-work entry, the trailing "known gaps left" list
narrowed).

**The M2 work queue below is renumbered from before this session — item 1 is now done, everything else is
unchanged:**

**Next, in the order that makes sense to attempt — independent, can land in any order or be split across
sessions:**

1. **ADR 0014's typed-receiver gap** — re-check `mwl_hir::members`'s own known-gap note against what
   `expr::check_property_access` does today; it may already be closed in substance and just need the
   ADR's own diagnostic/wording, not new logic.
2. **ADR 0024 §§ 2-3** (`tainted` propagation/laundering) and **ADR 0027** (`callable` value-shape
   checking) — grammar for both landed in M1; checker-side is untouched.
3. **ADR 0033 §§ 2-4** (`secret` propagation, checked-conversion laundering, `Markup`/`Throwable`-message
   sink refusal) — its M1 grammar landed; `lower_atom` still maps all four `Secret*` atoms to `mixed`.
4. **`switch`/`try` definite-assignment precision** (both bodies conservatively contribute nothing today
   — safe, just imprecise) and **`parent` as a *type* atom** (`parent $x`, distinct from the already-working
   `new parent(...)`) — long-standing, low-value-until-the-above-lands gaps.
5. **Smaller independent polish**, any one a quick follow-up: a class constant's type (`Class::CONST` is
   always `mixed`, including a *non-enum* class — enum case access was fixed last session); a promoted
   constructor-parameter property (tracked as neither a property nor a definite-assignment obligation —
   mirrors a `mwl_hir::members` gap, likely fix both together); a named/spread call argument disables all
   per-argument checking for that call; a class with no explicit `constructor` isn't held to a zero-arg
   arity check on `new`, nor to the `parent::constructor(...)` obligation; a `set`-hooked property is
   exempted from ADR 0022's check entirely rather than verified against the hook's body; `implements
   Comparable`/`Stringable` is never checked for actually declaring `compareTo`/`toString` (needs general
   interface-method-completeness checking, which doesn't exist yet — likely its own small design decision
   first); `==`/`===` between two different enum types (see this session's known gap above — wants a
   general equality-operand-compatibility pass, not an enum-only special case).
6. **ADR 0035's runtime side** — no code yet, and none is expected until M3's first backend exists.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs.
ADR 0007, 0010, 0013, 0022, 0028, 0036, and 0037's own entries are satisfied; the rest (0014, 0024, 0027,
0033's own entries, plus IR snapshot tests) depends on the work above or on `mwl-ir`, unstarted.
