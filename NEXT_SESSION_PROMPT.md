# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — run `sh .claude/brief.sh` first,
then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file only
points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact once").

**Last session was another docs-only pass: the periodic ADR cleanup from `DOC_CLEANUP_PROMPT.md`.** A
read-only survey first checked every Amends/Amended-by pair for a genuine merge candidate under the
project's strict "default to NOT merging" policy (decided 2026-08-21) — none qualified: no ADR has zero
surviving unique content after a later amendment, and only 0009 is Proposed (no Proposed/Proposed pair
exists to merge). No stale content or broken cross-links turned up either. So the pass was a pure trim:
26 of the 37 ADRs (0002-0038) had their `## Context`/`## Investigation`/`## Alternatives rejected`
sections compressed to short bullet lists, prior-art surveys collapsed to a one-line callout, and
duplicated reasoning between sections removed — `## Decision`, `## Consequences`, metadata blocks,
diagnostic names, and every cross-reference link are unchanged in meaning. Total ADR line count:
8130 -> 7940. **The actual next-code-work item below is unchanged from before this cleanup pass** — it
was never started.

**The real next job — implement ADR 0038 (`lateinit`) in the checker, mirroring how ADR 0022 §2 already
landed** (`ctor_init.rs`, `signatures.rs`'s `required_properties`/`own_required_properties`,
`E_UNINITIALIZED_PROPERTY` `E0409`, `E_MISSING_PARENT_CONSTRUCTOR_CALL` `E0410` — read that module before
starting, it's the template):

1. **Parser**: a `lateinit` modifier token on a property declaration (`mwl-syntax`), positioned alongside
   `public`/`static`/`readonly`. Needs its own lexer/parser test coverage the same way other modifiers have.
2. **`signatures.rs`**: a `lateinit` property must be excluded from `required_properties` (it has no
   constructor-must-assign obligation per ADR 0038 §1) — likely a new flag alongside the existing
   non-nullable/no-default/non-hooked criteria that already gate `required_properties` membership.
3. **New diagnostics**, per ADR 0038 §1 — pick real codes following the `E04xx` sequence `ctor_init.rs`'s
   session used (`E0409`/`E0410` were the last two added):
   - `E_LATEINIT_NOT_OBJECT_TYPE` — `lateinit` on a scalar/enum-typed property.
   - `E_LATEINIT_NULLABLE` — `lateinit` on a `?T` property.
   - `E_LATEINIT_PROMOTED_PARAM` — `lateinit` on a promoted constructor parameter.
   - `E_LATEINIT_READONLY_CONFLICT` — `lateinit` combined with `readonly`.
4. **`ctor_init.rs`** (or a sibling pass): the § 3 intraprocedural check — a `lateinit` property enters
   *every* method body (not just constructors) in "not yet proven written" state; any call to another
   method/function conservatively moves it to "assumed written" (never flag past a call — no false
   positives is the explicit design constraint in the ADR); a read reached with no intervening write and no
   intervening call on that path is `E_LATEINIT_READ_BEFORE_WRITE_LOCAL`. This likely reuses `locals.rs`'s
   existing dataflow-join machinery (`if`/`else`/`switch`/`try` handling from an earlier session) rather
   than writing new control-flow plumbing.
5. Runtime throw itself (ADR 0038 §2, reusing ADR 0022 §3's "never written" tag) is **M4** work, same as ADR
   0022's own residual case — no backend exists yet, nothing to do there this session.

Also still open from before (independent, low priority, pick up if there's time left over): a `set`-hooked
property is exempted from ADR 0022's check entirely rather than verified against the hook's body — same
open item now applies to whether a `lateinit` + hooked property should discharge on the hook's first commit
(ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs.
ADR 0007, 0010, 0013, 0014, 0022, 0024, 0027, 0028, 0033 (its M2-reachable entries), 0036, and 0037's own
entries are satisfied; ADR 0038 joins the list once the work above lands. The rest (0033's
`Core\Log`/`var_dump`/`serialize` entries, plus IR snapshot tests) depends on later milestones or on
`mwl-ir`, unstarted.
