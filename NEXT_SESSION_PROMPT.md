# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — see git history for
detail on how earlier items landed; it's not repeated here per CLAUDE.md's "state a fact once"
rule.

**Last session closed item 1's next entry**: [ADR 0013](docs/adr/0013-comparable-interface.md)'s
`Comparable` interface — `<`/`>`/`<=`/`>=`/`<=>` between two objects. Concretely:

- [`crates/mwl-hir/src/qname.rs`](crates/mwl-hir/src/qname.rs) — `QName::is_reserved_global_interface`:
  a bare (no-namespace) name the compiler trusts to exist with no source declaration, the same
  "trusted, not checked" treatment `QName::is_core` already gives `Core\*`. Only `"Comparable"` is
  in the set today; `Stringable`/`PropertyObserver` (ADR 0028/0014) will each add their own name to
  it when their turn comes, not before.
- [`crates/mwl-hir/src/hierarchy.rs`](crates/mwl-hir/src/hierarchy.rs) — `resolve_supertype` now
  trusts a reserved global interface the same place it already trusts `is_core()`, so
  `class Money implements Comparable {}` resolves with no `interface Comparable { ... }` anywhere
  in source. New `implements_interface(qname, target, graph)`: a plain reachability walk over every
  `extends`/`implements`/trait-use ancestor (the same shape `signatures::resolve_method` already
  walks, generalised from "resolve a member" to "is `target` anywhere in this chain") — works
  whether `target` has its own `ClassGraph` entry or not, since equality against `target` is checked
  before ever calling `graph.get` on it. Re-exported at the crate root.
- [`crates/mwl-diagnostics/src/lib.rs`](crates/mwl-diagnostics/src/lib.rs) — one new code:
  `E_COMPARISON_REQUIRES_COMPARABLE` (`E0411`).
- [`crates/mwl-types/src/expr.rs`](crates/mwl-types/src/expr.rs) — `binary_result`'s `Cmp`/`Lt`/
  `LtEq`/`Gt`/`GtEq` arms now call a new `object_comparison_result` first: when both operands are
  `Ty::Class`, it diagnoses two different classes, or a class not provably implementing
  `Comparable`, as `E_COMPARISON_REQUIRES_COMPARABLE`; the same class provably implementing it types
  as `bool` (`int` for `<=>`), matching ADR 0013 § 6's table. Returns `None` for any other operand
  shape (including an enum, or `mixed`) so the pre-existing scalar/`mixed` fallback is untouched —
  this is an amendment to ADR 0007 § 4's table, not a replacement.
- [`docs/implementation-plan.md`](docs/implementation-plan.md)'s M2 paragraph updated: the
  session-before-last's signature-typing item folded into settled-fact prose, the prior session's
  ADR 0022 paragraph re-labeled, and this session's ADR 0013 paragraph added, per the doc's own
  "prior session / this session" convention.

Full workspace `cargo test` (4 new tests in `mwl-types::check`, 5 new in `mwl-hir::hierarchy` —
69 tests total in `mwl-hir`, 66 in `mwl-types`, everything else still green), `cargo clippy
--all-targets -- -D warnings`, and `cargo fmt --check` are all clean. The CLI was also smoke-tested
by hand: `Foo $a; Foo $b; $a < $b;` for a non-`Comparable` `Foo` produced `E0411` naming
`Comparable`; the same comparison on a `Money implements Comparable` with a real `compareTo`
produced no diagnostics.

**Deliberately out of scope for this slice — the next thread to pick up, in the order that makes
sense to attempt them:**

1. **ADR 0027 (`callable` value-shape), ADR 0028 (`Stringable`, `unset()` refusal), ADR 0014's
   interplay with a typed (non-`$this`) receiver, ADR 0024 §§ 2-3 (tainted propagation/laundering),
   ADR 0010 (enum-vs-class atom distinction beyond "resolves to *a* symbol" — enum-specific
   operations).** Each is its own self-contained checker-side rule layered on top of the type
   table, signature table, `ClassGraph`, and now `Comparable`'s "reserved global interface" pattern
   this and the last few sessions built; none of them depend on each other, so they can land in any
   order or be split across sessions.
   - **`Stringable`** can reuse `Comparable`'s exact shape: add `"Stringable"` to
     `QName::is_reserved_global_interface`, then check `implements_interface` at every implicit
     string-conversion site (interpolation, concatenation, `echo`, `as string`) the same way
     `object_comparison_result` checks it for the five ordering operators.
   - **ADR 0014's typed-receiver gap**: re-read `mwl_hir::members`'s own known-gap note (a typed
     local/chained-call-result/`new Foo()` receiver's missing property "needs `mwl-types`' static
     types") against what `expr.rs`'s `PropertyAccess` arm does today — it already reports
     `E_UNKNOWN_MEMBER` for a non-`$this` receiver's missing property, so check first whether this
     item is actually "give it ADR 0014's own diagnostic/wording" rather than "implement the check
     from scratch," before assuming there's a full gap to close.
   - Note for whichever of these lands first: `expr::class_of_ctx` currently always interns
     `Ty::Class(qname)` for `self`/`static`/`$this`, even when the enclosing declaration is an
     enum — ADR 0010's own item will need to fix that (distinguish via `SymbolTable`'s
     `SymbolKind`, the same check `lower.rs`'s `resolve_name_type` already does for an ordinary
     type position) before enum-specific operations can tell `self` apart from a class.
2. **`switch`/`try` definite-assignment precision, and `parent` as a *type* atom** (`parent $x` —
   distinct from `new parent(...)`, which resolves since three sessions ago), plus the equivalent
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
   before it's worth building, not a quick fix).

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done. The ADR 0007, 0013 and 0022 corpus entries are now all
satisfied; the rest of that Verify line (ADR 0010, 0014, 0024, 0027, 0028's own corpus entries, plus
IR snapshot tests) still depends on the work items above, or on `mwl-ir`, which hasn't started.
