# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — run `sh .claude/brief.sh` first,
then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file only
points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact once").

**Last session landed [ADR 0037](docs/adr/0037-var-local-type-inference.md)**, a new decision made and
built in the same session (not a queued item): `var $name = expr;` — a local declaration with no written
type, inferred from `expr`'s own checked type and fixed forever, exactly as if written by hand. This
amends [ADR 0007](docs/adr/0007-explicit-type-system.md) § 1 rather than reopening its core "everything
declared" decision: it resolves the ADR's own long-deferred "local type inference" *Revisiting* entry, now
that the feature is wanted in the language itself rather than left to M11's `mwl convert` alone. Both
halves landed together, grammar and checker:

- `crates/mwl-syntax`: `parse_var_local_decl` (a plain, unambiguous parse — `var` never starts anything
  else at statement position, so no trial parse), `ast::StmtKind::LocalDecl.ty` is now `Option<Type>`
  (`None` = `var`).
- `crates/mwl-types`: `locals.rs`'s `LocalDecl` arm routes `None` through `check_expr`'s existing
  no-`expected` synthesis path and declares the result like any written type. The one initializer shape
  refused is a bare array literal (`var $x = [1, 2];`, `E_VAR_ARRAY_LITERAL_NEEDS_TYPE`/`E0414`) — no
  target to synthesize an element type from, same reasoning as ADR 0007 § 5's array-literal check.

Docs updated in the same session: ADR 0007 (§ 1's table, amendment metadata, its now-resolved
*Alternatives rejected*/*Revisiting* entries removed rather than left stale), `docs/adr/README.md`'s
index, `docs/implementation-plan.md` (a new landed-grammar paragraph, plus the M11 paragraph and the
architecture-tradeoffs list both narrowed to say a plain local no longer needs the converter's inference
pass — only a `foreach` binding and a destructuring target still do), and CLAUDE.md (`Where to look` row,
and the "type inference belongs in `mwl convert`" ground rule corrected to name `var` as the one
exception). Verified: `cargo build`/`test`/`clippy -D warnings`/`fmt --check` all clean; a manual `mwl
check` run on a fixture confirms both the inference and the array-literal diagnostic end to end.

**The M2 work queue below is unchanged from before this session** — ADR 0037 was an out-of-band addition,
not a substitute for it:

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
ADR 0007, 0013, 0022, 0028, 0036, and now 0037's own entries are satisfied; the rest (0010, 0014, 0024,
0027, 0033's own entries, plus IR snapshot tests) depends on the work above or on `mwl-ir`, unstarted.
