# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — run `sh .claude/brief.sh` first,
then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file only
points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact once").

**Last session closed [ADR 0014](docs/adr/0014-property-observer.md)'s remaining M2 item — the first item
on the prior queue — but found no new checker logic was needed, only a stale doc correction:**

- The item was "a property access on any receiver other than `$this`," which `mwl_hir::members`'s own
  module docs still listed as an open gap needing `mwl-types`' static types. It turned out
  `mwl-types::expr::check_property_access` had already closed it in an earlier session — it reports
  `E_UNKNOWN_MEMBER` for exactly that shape, and a fixture in `mwl-types::check`
  (`an_undeclared_property_on_a_typed_local_is_diagnosed`) already exercises it (`Foo $x = new Foo();
  $x->missing;`). Both `mwl_hir::members`'s and `mwl-types`'s own known-gap notes were simply never updated
  to say so. Fixed in [`members.rs`](crates/mwl-hir/src/members.rs) and
  [`lib.rs`](crates/mwl-types/src/lib.rs)'s module docs, and in `docs/implementation-plan.md`'s M2
  paragraph (ADR 0014 moved into the "done" list alongside 0013).
- **A second, separate staleness surfaced while re-reading ADR 0014 § 6 to verify the above:** its claim
  that "a method literally named `__call`/`__callStatic` is an ordinary method: it compiles" was never
  actually true once [ADR 0029](docs/adr/0029-identifier-casing-is-checked.md) shipped — methods never had
  a leading-underscore casing allowance (only properties/parameters/locals did, until
  [ADR 0030](docs/adr/0030-no-leading-underscores-constructor-spelling.md) revoked it), so
  `mwl-syntax::casing::check_method_name` has always rejected `__call` with the generic
  `E_BAD_METHOD_CASING` before any call-resolution logic runs. The *outcome* ADR 0014 § 6 wants still
  holds, more strongly than described — the name cannot be declared at all, not merely "declared but never
  dispatched." Corrected ADR 0014 § 6 and its M2 verification line, plus the identical stale claim
  ADR 0028 repeated for `__destruct` (§ 2) and `__set_state` (§ 5) and in its own M2/M4 verification lines
  — no code changed anywhere, since the casing checker already produces the right diagnostic for the right
  reason. `cargo build`/`test`/`clippy -D warnings`/`fmt --check` all clean (doc-only session, no new
  tests needed).

**The M2 work queue below is renumbered from before this session — item 1 (ADR 0014) is now done and
removed; everything else is unchanged, just shifted up:**

**Next, in the order that makes sense to attempt — independent, can land in any order or be split across
sessions:**

1. **ADR 0024 §§ 2-3** (`tainted` propagation/laundering) and **ADR 0027** (`callable` value-shape
   checking) — grammar for both landed in M1; checker-side is untouched.
2. **ADR 0033 §§ 2-4** (`secret` propagation, checked-conversion laundering, `Markup`/`Throwable`-message
   sink refusal) — its M1 grammar landed; `lower_atom` still maps all four `Secret*` atoms to `mixed`.
3. **`switch`/`try` definite-assignment precision** (both bodies conservatively contribute nothing today
   — safe, just imprecise) and **`parent` as a *type* atom** (`parent $x`, distinct from the already-working
   `new parent(...)`) — long-standing, low-value-until-the-above-lands gaps.
4. **Smaller independent polish**, any one a quick follow-up: a class constant's type (`Class::CONST` is
   always `mixed`, including a *non-enum* class — enum case access was fixed two sessions ago); a promoted
   constructor-parameter property (tracked as neither a property nor a definite-assignment obligation —
   mirrors a `mwl_hir::members` gap, likely fix both together); a named/spread call argument disables all
   per-argument checking for that call; a class with no explicit `constructor` isn't held to a zero-arg
   arity check on `new`, nor to the `parent::constructor(...)` obligation; a `set`-hooked property is
   exempted from ADR 0022's check entirely rather than verified against the hook's body; `implements
   Comparable`/`Stringable` is never checked for actually declaring `compareTo`/`toString` (needs general
   interface-method-completeness checking, which doesn't exist yet — likely its own small design decision
   first); `==`/`===` between two different enum types (wants a general equality-operand-compatibility
   pass, not an enum-only special case).
5. **ADR 0035's runtime side** — no code yet, and none is expected until M3's first backend exists.

Also worth a quick pass sometime, low priority: re-check the rest of the docs tree for the same
"`__foo` compiles as an ordinary method" phrasing pattern this session found stale in two places (ADR
0014 § 6, ADR 0028 §§ 2/5) — every double-underscore magic-method name is unconditionally rejected by
ADR 0029's method-casing rule before any resolution logic ever runs, so any ADR describing one as
"compiling, just never invoked" needs the same correction. A `grep -rn "compiles as an ordinary method"
docs/adr/` came back clean after this session's fixes, but the same idea may be phrased differently
elsewhere (checked `__isset`/`__unset`/`__debugInfo`'s own sections in ADR 0028 — those don't make the
claim, so no fix was needed there).

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs.
ADR 0007, 0010, 0013, 0014, 0022, 0028, 0036, and 0037's own entries are satisfied; the rest (0024, 0027,
0033's own entries, plus IR snapshot tests) depends on the work above or on `mwl-ir`, unstarted.
