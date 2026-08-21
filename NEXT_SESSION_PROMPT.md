# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — run `sh .claude/brief.sh` first,
then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file only
points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact once").

**Last session closed [ADR 0024](docs/adr/0024-taint-tracking-for-injection-sinks.md) §§ 2-3 (`tainted`
propagation/laundering) and [ADR 0027](docs/adr/0027-callable-is-closures-only.md) (`callable` value-shape
checking) — the next two items on the prior queue, both entirely inside `mwl-types::expr`, no grammar
changes needed since both ADRs' M1 halves had already landed:**

- **ADR 0024:** concatenation and interpolation now poison their result whenever either operand is
  `tainted string`/`tainted bytes` (`is_tainted`). `ExprKind::Conversion` gained
  `apply_taint_conversion_rule`: a checked conversion to `uint`/`int`/`float`/`bool`/an enum's backing type
  launders for free (no code needed — those targets never carried the qualifier); `bytes`/`string` keep it
  across either direction (ADR 0009 § 3), including the identity-shaped `tainted string as string`, which
  is deliberately *not* treated as laundering — that would be a silent bypass. `is_assignable` gained one
  new rule: a plain `string`/`bytes` is assignable into its `tainted` counterpart (a trusted value is a
  safe over-approximation of "may be tainted"), never the reverse — without it no fixture could even build
  a `tainted`-typed local from a literal. `reject_non_literal_markup_conversion` covers ADR 0024 § 5's one
  M2-scoped rule: `as Core\Html\Markup` accepts only a literal string token, tainted or not.
- **ADR 0027:** a bare string or `[$obj, 'method']`-shaped array literal reaching a `callable`-typed
  position now gets a targeted diagnostic (`E_CALLABLE_STRING_UNSUPPORTED`/`E_CALLABLE_ARRAY_UNSUPPORTED`)
  naming the first-class-callable-syntax replacement, ahead of the generic mismatch; `$obj(...)` is refused
  whenever `$obj`'s static type resolves to a class (`E_NOT_CALLABLE`) — MWL has no `__invoke`, and a method
  literally named `__invoke` can't even be declared (ADR 0029/0030's casing rule already rejects it).
  **A real gap was found and fixed, not just documented:** `$obj->method(...)`/`Foo::bar(...)` (first-class
  callable syntax) was typing as the referenced method's own *return type* rather than `callable` — fixed
  in `expr::infer`'s `MethodCall`/`StaticCall`/`Call` arms, which now check for the
  `CallArgs::FirstClassCallable` sentinel first.
- Four new diagnostic codes (`E0417`-`E0420`), 21 new tests in `mwl-types::check`.
  `cargo build`/`test`/`clippy -D warnings`/`fmt --check` all clean.
- **Known gap, not attempted this session:** ADR 0024 § 4's named sinks (`Core\Db`, `Core\Process`,
  `Core\Http`, `Core\Fs`) have no code refusing anything yet — none of those `Core` classes exist as
  declared stdlib until M7/M8. A plain-typed parameter on an ordinary user-declared method already acts as
  an equivalent sink today (via the `tainted string`-vs-`string` assignability rule), but the ADR's own
  named sinks still wait on the stdlib that defines them; § 5's auto-escape default and `Markup + Markup`
  composition wait on `Core\Html` existing too.

**The M2 work queue below is renumbered from before this session — the old item 1 (ADR 0024/0027) is done
and removed; everything else is unchanged, just shifted up:**

**Next, in the order that makes sense to attempt — independent, can land in any order or be split across
sessions:**

1. **ADR 0033 §§ 2-4** (`secret` propagation, checked-conversion laundering, `Markup`/`Throwable`-message
   sink refusal) — its M1 grammar landed earlier; `lower_atom` still maps all four `Secret*` atoms to
   `mixed`. The taint-propagation code just added (`is_tainted`/`apply_taint_conversion_rule` in
   `mwl-types::expr`) is a close structural precedent — `secret` is an independent, composable qualifier
   axis, not a variant of `tainted`, so expect a parallel set of helpers rather than a reuse of these ones,
   but the same shape (poison through concat/interpolation, launder through a checked conversion, refuse at
   a sink) applies.
2. **`switch`/`try` definite-assignment precision** (both bodies conservatively contribute nothing today
   — safe, just imprecise) and **`parent` as a *type* atom** (`parent $x`, distinct from the already-working
   `new parent(...)`) — long-standing, low-value-until-the-above-lands gaps.
3. **Smaller independent polish**, any one a quick follow-up: a class constant's type (`Class::CONST` is
   always `mixed`, including a *non-enum* class — enum case access was fixed several sessions ago); a
   promoted constructor-parameter property (tracked as neither a property nor a definite-assignment
   obligation — mirrors a `mwl_hir::members` gap, likely fix both together); a named/spread call argument
   disables all per-argument checking for that call; a class with no explicit `constructor` isn't held to a
   zero-arg arity check on `new`, nor to the `parent::constructor(...)` obligation; a `set`-hooked property
   is exempted from ADR 0022's check entirely rather than verified against the hook's body; `implements
   Comparable`/`Stringable` is never checked for actually declaring `compareTo`/`toString` (needs general
   interface-method-completeness checking, which doesn't exist yet — likely its own small design decision
   first); `==`/`===` between two different enum types (wants a general equality-operand-compatibility
   pass, not an enum-only special case).
4. **ADR 0035's runtime side** — no code yet, and none is expected until M3's first backend exists.

Also worth a quick pass sometime, low priority: re-check the rest of the docs tree for the same
"`__foo` compiles as an ordinary method" phrasing pattern a previous session found stale in ADR 0014 § 6
and ADR 0028 §§ 2/5 — every double-underscore magic-method name is unconditionally rejected by ADR 0029's
method-casing rule before any resolution logic ever runs, so any ADR describing one as "compiling, just
never invoked" needs the same correction. A `grep -rn "compiles as an ordinary method" docs/adr/` came back
clean as of that session, but the same idea may be phrased differently elsewhere.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs.
ADR 0007, 0010, 0013, 0014, 0022, 0024, 0027, 0028, 0036, and 0037's own entries are satisfied; the rest
(0033's own entries, plus IR snapshot tests) depends on the work above or on `mwl-ir`, unstarted.
