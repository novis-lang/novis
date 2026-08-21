# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — see git history for
detail on how earlier items landed; it's not repeated here per CLAUDE.md's "state a fact once"
rule.

**Last session closed item 1 of the prior session's follow-up list**: property, method-call,
`new`, `match` and ternary expression typing in `mwl-types`. Concretely:

- [`crates/mwl-types/src/signatures.rs`](crates/mwl-types/src/signatures.rs) — new module.
  `SignatureTable` records every class/interface/trait/enum's own declared property types and
  method parameter/return types (`MethodSig`), built by `build_signatures` in its own walk *ahead*
  of any body-checking (same two-pass idea `mwl-hir`'s own resolvers already use, so a method can
  call another declared later in the same file). `resolve_property`/`resolve_method` look a name up
  on a class and, failing that, walk `extends`/`implements`/trait-use ancestors — the same shape
  `mwl_hir::members::member_declared` already walks for existence-only checking, generalised to
  return the type found. **One deliberate wrinkle documented in the module's own doc comment:**
  `build_signatures` constructs its own `Env` with `signatures` pointed at an unrelated, empty
  placeholder table (collection writes into a separate `&mut SignatureTable` return value instead,
  never through `env.signatures`) — that's what lets it run before the table it produces exists,
  since `crate::Env` holds `interner`/`diags` as exclusive `&mut` borrows and a table under active
  construction can't also be borrowed immutably through the same `Env` at once.
- [`crates/mwl-types/src/check.rs`](crates/mwl-types/src/check.rs) — `check_program` now builds the
  `SignatureTable` before checking any body, and `check_method` seeds `$this` into the method's
  `LocalScope`, typed as the enclosing class (via `expr::class_of_ctx`, now `pub(crate)`). This was
  necessary, not optional: without it, `$this->whatever` inside a method body would hit
  `check_read`'s "not declared" branch and report a spurious `E_UNDEFINED_VARIABLE`, since `$this`
  was never treated as an ordinary parameter before. Not gated on a `static` modifier — a static
  method's own body referencing `$this` is a distinct, unrelated diagnostic this slice doesn't add.
- [`crates/mwl-types/src/expr.rs`](crates/mwl-types/src/expr.rs) — `infer` now resolves:
  - **Property access** (`$obj->prop`) and **static property access** (`Class::$prop`) to the
    resolved property's type via `class_qname_of` (peels a `Ty::Class`/`Ty::Enum` off the receiver's
    already-inferred type) plus `resolve_property`.
  - **Instance method calls** (`$obj->method(...)`) and **static calls** (`Class::method(...)`) to
    the resolved method's return type via `resolve_method`, with **positional arity/argument-type
    checking** against the resolved `MethodSig` (`check_args_typed`) — skipped (falls back to the
    old "just walk nested exprs" behaviour) whenever no signature resolves, or any argument is named
    or spread, since PHP's named/variadic call resolution isn't a straight positional mapping and
    modeling that is out of scope for this slice.
  - **`new`** — `new parent(...)` now resolves through the class graph (previously always `mixed`);
    a resolved constructor's own `MethodSig` is looked up under the name `"constructor"` and the
    call arguments checked against it the same way an ordinary method call's are.
  - **`match`/ternary** — the union of every arm's/branch's own type, via `TypeInterner::make_union`,
    instead of always `mixed`. A ternary with `then` omitted (`$a ?: $b`) unions `$a`'s own type
    (`cond`'s inferred type) with `$b`'s, matching PHP's short-circuit semantics.
  - **One diagnostic-placement rule worth knowing, spelled out in `expr.rs`'s own module docs**:
    `mwl_hir::members` already checks a `self::`/`static::`/`parent::`/explicit-class-name static
    reference's *existence*, and a `$this->prop` access's existence — this session's code only
    recovers the *type* for those two shapes and reports **no second diagnostic** when the lookup
    fails (avoiding double-reporting the same error two different ways). Every other shape — an
    instance method call on *any* receiver including `$this` (never checked by `mwl_hir` at all,
    since it had no static type to check a receiver against), and a property access on anything but
    `$this` — genuinely never had a check before this session, so those get a new `E_UNKNOWN_MEMBER`
    (`E0405` — already reserved in `mwl-diagnostics`, unused until now) when a receiver class is
    known but the member isn't declared on it or any ancestor. `E_ARITY_MISMATCH` (`E0402`, also
    previously reserved but unused) is new for the same reason: nothing checked call-site arity
    before this session at all.
- `crates/mwl-types/src/lib.rs`'s crate docs, module list and known-gaps list are updated to match;
  `crates/mwl-types/src/lower.rs`'s test-only `Env` construction gained the two new required fields
  (`graph`, `signatures`).

Full workspace `cargo test` (14 new tests across `mwl-types::check`/`mwl-types::signatures`,
everything else still green — 52 tests total in `mwl-types` alone), `cargo clippy --all-targets --
-D warnings`, `cargo fmt --check`, and `cargo doc -p mwl-types --no-deps` (no broken intra-doc
links) are all clean. The CLI was also smoke-tested by hand: a fixture with a valid `new
Foo(1)`/`$f->bump()` pair produced no diagnostics, while an undeclared `$f->missing` property, a
`new Foo("wrong")` constructor-argument type mismatch, and a `$f->bump(1, 2)` arity mismatch each
produced exactly the expected diagnostic and nothing else — read `CLAUDE.md` first, then run `sh
.claude/brief.sh` for the live status slice.

**Deliberately out of scope for this slice — the next thread to pick up, in the order that makes
sense to attempt them:**

1. **ADR 0022 — definite property initialization in constructors.** The exact same flow-analysis
   machinery `locals.rs` already has for local variables, extended to a second binding kind
   (properties), per the ADR's own framing. Should reuse `locals.rs`'s `check_block`/`terminates`
   shape rather than duplicating it — worth checking whether that machinery can be parameterized
   over "which names must end up assigned" instead of being local-variable-specific. This item was
   already next in line before this session started (this session did item 1 from the list before
   it, not this one) — still not started.
2. **ADR 0013 (`Comparable`), ADR 0027 (`callable` value-shape), ADR 0028 (`Stringable`, `unset()`
   refusal), ADR 0024 §§ 2-3 (tainted propagation/laundering), ADR 0010 (enum-vs-class atom
   distinction beyond "resolves to *a* symbol" — enum-specific operations).** Each is its own
   self-contained checker-side rule layered on top of the type table and signature table this and
   the prior session built; none of them depend on each other, so they can land in any order or be
   split across sessions. Note for whichever of these lands first: `expr::class_of_ctx` currently
   always interns `Ty::Class(qname)` for `self`/`static`/`$this`, even when the enclosing
   declaration is an enum — ADR 0010's own item will need to fix that (distinguish via
   `SymbolTable`'s `SymbolKind`, the same check `lower.rs`'s `resolve_name_type` already does for an
   ordinary type position) before enum-specific operations can tell `self` apart from a class.
3. **`switch`/`try` definite-assignment precision, and `parent` as a *type* atom** (`parent $x` —
   distinct from `new parent(...)`, which this session's slice already resolves) — both named as
   known gaps for a while now, both safe-but-imprecise today (reject a few extra valid programs
   rather than ever accepting an invalid one), worth revisiting once the higher-value items above
   are done rather than before.
4. **Smaller, independent polish items surfaced this session, any of which could be a quick
   follow-up on its own:** a class constant's type (`Class::CONST` stays `mixed` regardless of
   receiver — there's no const-value type table yet); a promoted constructor-parameter property
   (`function constructor(public int $x) {}`) is recorded as neither a property nor anything else —
   this mirrors a pre-existing `mwl_hir::members` gap, so fixing it well might mean fixing both
   crates together; a named or spread call argument disables *all* per-argument checking for that
   call rather than being matched positionally where possible; a class with no explicit
   `constructor` is not held to a zero-argument arity check on `new Foo(...)`.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done. The ADR 0007 corpus entries are still fully satisfied; the rest
of that Verify line (ADR 0010, 0013, 0014, 0022, 0024, 0027, 0028's own corpus entries, plus IR
snapshot tests) still depends on the work items above, or on `mwl-ir`, which hasn't started.
