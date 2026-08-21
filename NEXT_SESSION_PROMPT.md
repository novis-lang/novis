# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — run `sh .claude/brief.sh` first,
then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file only
points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact once").

**Last session closed [ADR 0033](docs/adr/0033-secret-qualifier-for-confidential-values.md) §§ 2-4
(`secret` propagation, checked-conversion laundering, and its two M2-reachable sink refusals) — the next
item on the prior queue, entirely inside `mwl-types` (plus one small, deliberately narrow `mwl-hir`
addition the Throwable sink needed):**

- `secret` reuses `tainted`'s exact machinery rather than duplicating it: `ty::Ty` gained four atoms
  (`SecretString`/`SecretBytes`/`SecretTaintedString`/`SecretTaintedBytes`), `lower_atom` now maps
  `mwl-syntax`'s matching atoms onto them, and `expr.rs` grew a shared vocabulary (`is_secret` alongside
  `is_tainted`, `qualifiable_base`/`qualified_scalar` to move between the two independent qualifier bits and
  the one atom-per-combination representation) that concatenation/interpolation, `is_assignable`'s widening
  rule, and `ExprKind::Conversion`'s laundering rule (renamed `apply_qualifier_conversion_rule`) all route
  through — both qualifiers poison/widen/launder identically and independently, no separate code path per
  axis. `secret` follows `tainted`'s laundering shape exactly, including the one deliberately accepted
  inconsistency ADR 0033 § 2 names: a checked `as uint`/`int`/`float`/`bool`/enum-backing-type conversion
  strips `secret` too, even though "shape-proof implies safe" never actually justified that for
  confidentiality.
- Two sinks, both reachable without any stdlib existing yet: a `secret` operand converted `as
  Core\Html\Markup` gets its own diagnostic (`E_SECRET_MARKUP_UNSUPPORTED`, `E0421`) ahead of the existing
  literal-required one, naming *why* (escaping doesn't restore confidentiality); a `secret` value passed as
  a `Throwable`-shaped class's constructor message is refused (`E_SECRET_THROWABLE_MESSAGE`, `E0422`).
  **The Throwable sink needed one small enabling change, not just a check:** `Throwable`/`Exception`/`Error`
  had no code trusting them to exist at all (no declared stdlib), so `new Exception(...)` used to resolve to
  `mixed` and nothing was checkable. A new `mwl_hir::QName::is_reserved_global_class` (mirroring
  `is_reserved_global_interface`'s existing treatment of `Comparable`/`Stringable`) trusts those three bare
  names the same way `Core`'s own classes are trusted — wired into `hierarchy::resolve_supertype` (so
  `class MyError extends Exception {}` resolves), `mwl-types`' `check_new_target`/`resolve_name_type`, and
  the two `E_UNKNOWN_MEMBER` guards that already exempt an unmodeled `Core` class (so `$e->getMessage()`
  stays silently `mixed` rather than newly erroring, since no member table exists for these three either).
- `check_args_typed` now returns each argument's own checked type (previously discarded) so the `New` arm can
  read the first one back for the Throwable-message check without a second, diagnostic-duplicating pass over
  the same expression.
- 12 new tests in `mwl-types::check`, one in `mwl-hir::qname`, one in `mwl-hir::hierarchy`, one in
  `mwl-types::lower`. `cargo build`/`test`/`clippy -D warnings`/`fmt --check` all clean.
- **Known gaps, not attempted this session, all deferred by ADR 0033's own *Verification* section to a
  later milestone rather than left unnoticed:** `Core\Log`'s call-site inspection (M8 — the sink is a `mixed`
  parameter by design, so this needs argument-expression inspection at the call site, not a parameter-type
  refusal); `var_dump`/`print_r`'s redaction placeholder for a `secret`-typed property (M4, once those
  exist); `serialize()`/the `spawn worker`/`spawn script` boundary's refusal (M5, once that boundary exists).

**The M2 work queue below is renumbered from before this session — the old item 1 (ADR 0033) is done and
removed; everything else is unchanged, just shifted up:**

**Next, in the order that makes sense to attempt — independent, can land in any order or be split across
sessions:**

1. **`switch`/`try` definite-assignment precision** (both bodies conservatively contribute nothing today —
   safe, just imprecise) and **`parent` as a *type* atom** (`parent $x`, distinct from the already-working
   `new parent(...)`) — long-standing, low-value-until-now gaps.
2. **Smaller independent polish**, any one a quick follow-up: a class constant's type (`Class::CONST` is
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
3. **ADR 0035's runtime side** — no code yet, and none is expected until M3's first backend exists.

Also worth a quick pass sometime, low priority: re-check the rest of the docs tree for the same
"`__foo` compiles as an ordinary method" phrasing pattern a previous session found stale in ADR 0014 § 6
and ADR 0028 §§ 2/5 — every double-underscore magic-method name is unconditionally rejected by ADR 0029's
method-casing rule before any resolution logic ever runs, so any ADR describing one as "compiling, just
never invoked" needs the same correction. A `grep -rn "compiles as an ordinary method" docs/adr/` came back
clean as of that session, but the same idea may be phrased differently elsewhere.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs.
ADR 0007, 0010, 0013, 0014, 0022, 0024, 0027, 0028, 0033 (its M2-reachable entries), 0036, and 0037's own
entries are satisfied; the rest (0033's `Core\Log`/`var_dump`/`serialize` entries, plus IR snapshot tests)
depends on the work above, on later milestones, or on `mwl-ir`, unstarted.
