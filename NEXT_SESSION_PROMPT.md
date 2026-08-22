# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Also landed a couple of sessions back, independent of the `mwl-ir` thread below: [ADR 0039](docs/adr/0039-canonical-code-formatting.md)**
decides `mwl fmt`'s actual formatting rules — PER as the base style, explicit rules for the MWL-only
constructs PER never saw (`fn` closures, `tainted`/`secret`, `lateinit`, shape types, `match`), a
gofmt-style no-reflow model (never wraps/collapses an expression by width), zero configuration ever, and a
hard separation from the compiler (`mwl fmt --check` warns; `mwl check` never does). Docs only — `mwl-fmt`
itself doesn't exist until M10, so there is nothing to build from this yet.

**Last session landed the second half of item 3 below — an instance method call, plus a
compile-time-known property access.** Both reuse `mwl-types`' `ExprTypeTable` exactly the way `new`/a
static call already did: `ExprInfo::Call` for `$obj->method(...)`/`$this->…` (the producer side was
already tested from the session before), and a new `ExprInfo::Property { class, name, ty }` variant for
`$obj->prop`/`$this->prop`, recorded by `mwl_types::expr::check_property_access` and keyed by the
`PropertyAccess` expression's own span (derived in-function as `object.span.to(*name_span)` — provably
identical to the parser's own span construction, so no extra parameter needed to thread it through; see
that function's comment).

On the `mwl-ir` side: every lowered method's `Function::params` now carries an implicit receiver at index
0 (`$this`; index 0 whether or not the body reads it) ahead of every explicit parameter — the design
question flagged open last session, settled in favor of the "implicit first parameter" shape (mirroring
`mwl_types::check.rs`'s `check_method`, which already seeds `$this` unconditionally) over a
receiver-only special case, because the latter would duplicate `ExprKind::Variable`'s `Env`-lookup path
for a value that behaves like an ordinary parameter in every other respect. This changed every existing
snapshot's function signature line (regenerated via `cargo insta test --accept -p mwl-ir`). `InstKind::Call`
now sets `receiver: Some(v)` for an instance call; a new `InstKind::FieldGet { object, class, field }`
reads a compile-time-known field — `class`/`field` are labels for a future codegen layout pass, the same
"resolved identity, not yet a machine offset" shape `Call`/`New` already use. A receiver that erased to a
shape or plain `object` (ADR 0036 § 4) has no `ExprInfo::Property` entry at all — lowering panics naming
that case, since the checker itself defers the runtime-checked fallback to M4 with no IR/codegen yet to
throw from. A nullsafe access of either kind (`?->`) is equally out of scope today.

Four new tests landed: two `insta` snapshots (`a_this_property_access`, `a_property_access_through_a_local_receiver`)
plus two `#[should_panic]` tests for the nullsafe and shape/`object`-erasure refusals, alongside the
instance-call tests from the producer session before. `mwl-types` is now at 162 tests, `mwl-ir` at 20.
`cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check` all clean across the whole
workspace.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, and a compile-time-known property access.~~ **Done.**
   Two shapes remain, independent of each other and of everything above:
   - **Array access (`$arr[$i]`)** is still unsupported; lowering panics naming the expression. No
     non-scalar *data* representation exists yet either (see item 4), so this may naturally land together
     with that slice rather than alone — worth deciding at the start of whichever session picks it up.
   - **A property access through a shape or plain-`object` receiver** (ADR 0036 § 4's erasure case) has no
     `ExprTypeTable` entry to read and no IR representation decided — is there a checked-throw instruction
     now, a placeholder/panic until M4, or something else? This is the same kind of IR-level design
     question the instance-call session flagged for its own slice; worth 1-2 paragraphs before writing
     lowering code, per CLAUDE.md's "ask about tradeoffs" bar.
4. **Non-scalar *data* values (`string`/`bytes`, arrays) and refcount operations.** The milestone text's
   third named ingredient; `Ty::Object` (landed two sessions back) covers the object-reference case but
   carries no refcount operations yet either — still nothing to attach one to until this lands.
5. **Runtime-helper calls** — the milestone's fourth named ingredient, for `mixed`/union operands once they
   exist in the IR, and also what a non-`bool` `if`/`while` condition's ADR 0035 truthy conversion needs.
6. **Virtual dispatch** — every call/access lowered so far (`new`'s constructor, a static call, an instance
   call, a property access) has its receiver's *static* type equal to its *runtime* class — none has gone
   through an interface-typed or overridden-method/property receiver yet, which is the first place the two
   could actually differ. Whether a real vtable/interface-dispatch lookup belongs at this IR level (as
   opposed to purely at codegen, once M3 exists) is an open question for whichever session first hits that
   shape.
7. `var` locals (ADR 0037) and full-magnitude/multi-base integer-literal cooking (hex/octal/binary) are
   smaller, independent gaps that can land whenever convenient.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).
