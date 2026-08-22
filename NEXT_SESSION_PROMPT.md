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

**Last session resolved the `mwl-ir`/`mwl-types` coupling question and landed `new`/a static call —
item 3 in the widening order below, first half.** The architecture decision (flagged open for two prior
sessions) is settled: `mwl-types` grew `crate::expr_table::ExprTypeTable` (see
`crates/mwl-types/src/expr_table.rs`'s own module docs for the full design), a narrow, purpose-built table
of `ExprInfo::Call`/`ExprInfo::New` entries — the declaring class, method name, and resolved
parameter/return `TypeId`s — that `mwl_types::check_program` now populates once (via a new `exprs: &mut
ExprTypeTable` parameter, threaded through `Env`) and hands back for a later pass to read. The lookup key is
the expression's own source `Span`, not an independently-assigned id: `mwl-ir`'s lowering walk and
`mwl-types`' checking walk are two separately-ordered traversals of the same AST, so a span is the only
thing both crates can agree on without coordinating walk order (see the module's "Why a lookup is keyed by
Span" section). `mwl-ir` now depends on `mwl-types`, but only for this table plus `TypeInterner` — never for
`mwl-hir`, `mwl_types::signatures`, or `mwl_types::ClassGraph` directly.

On the `mwl-ir` side: `ir::Ty` gained one non-scalar variant, `Ty::Object` — an opaque class/enum reference
with no identity carried in the IR at all (a call's/`new`'s target is already resolved to a concrete
`"Class::method"` label by `ExprTypeTable` before lowering ever sees it) and no refcount operations yet,
reserved the same "shape now, functional later" way `InstKind::Safepoint` already was. `ir::InstKind` gained
`Call { target, receiver, args }` (`receiver` is always `None` today — reserved for an eventual instance
call, so landing one needs no new `InstKind` variant) and `New { class, args }`. `lower::lower_scalar_type`
was renamed `lower_decl_type` and widened to erase a plain class-name AST atom (`TypeAtom::Name(_)`) to
`Ty::Object` — needs no resolution, since ADR 0007 § 1 already requires it spelled out in full, same as a
scalar atom always did. A new `lower_checked_ty` translates a `TypeId` recorded in the table (a resolved
call's declared parameter/return type) into `Ty` the same way, for the cases `lower_decl_type` can't reach
(there may be no local `Type` AST node at all — an inherited method's parameter is declared on a different
class's source). `Lowering::lower_call_args` lowers a resolved call's/`new`'s positional argument list,
panicking on anything variadic/named/spread (both known gaps, and `mwl_types` doesn't fully positionally
type-check those against a signature yet either).

Four new `insta` snapshot tests in `crates/mwl-ir/src/lower.rs` cover: `new` with a resolved
one-parameter constructor, `new` against a class with no declared constructor (empty arg list, no
lookup), a `self::` static call with a scalar argument/return, and a class-typed local initialized from
`new`. The test harness (`lower_first_method`) now actually runs a fixture through
`mwl_hir::resolve_file` + `mwl_types::check_program` instead of only trusting it would pass — lowering a
call/`new` needs a real `ExprTypeTable` to read from, so a hand-waved "would pass" fixture is no longer
enough. `mwl-types` is now at 160 tests (was 154; +6 for `expr_table`'s own producer/consumer tests),
`mwl-ir` is now at 13 (was 9). `cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check` all
clean across the whole workspace.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, and the `mwl-ir`/`mwl-types` coupling decision.~~ **Done, first half — the
   instance-call half is next:**
   - **An instance method call (`$obj->method(...)`, including `$this->…`) is still unsupported.**
     `ExprTypeTable::record` already stores an `ExprInfo::Call` entry for a resolved `MethodCall`, not just
     `StaticCall` — the *producer* side is done and tested
     (`expr_table::tests::an_instance_method_call_records_the_resolved_call`). What's missing is entirely on
     the `mwl-ir` consumer side: `$this` (and any other receiver) needs to become a real `ValueId` first.
     Today `lower_method` seeds `Env` only from `m.params` — `$this` has no binding at all. The cleanest fix
     is almost certainly an implicit receiver parameter threaded the same way `mwl_types::check.rs`'s
     `check_method` already seeds `$this` into its own `LocalScope` (see that function for the precedent) —
     but doing this changes `Function::params`'/every lowered method's shape (whether it references `$this`
     or not), which is why last session scoped it out rather than rushing it. `InstKind::Call` already
     reserves a `receiver: Option<ValueId>` field for exactly this, so landing it needs no new `InstKind`
     shape — only: (a) deciding how `$this`/a receiver becomes a value in `Env` (an implicit first
     parameter is the leading candidate; weigh it against the alternative of a receiver-only special case
     that doesn't touch `Function::params` at all), and (b) wiring the `ExprKind::MethodCall` arm in
     `lower_expr` to look up its `ExprInfo::Call` entry (same shape as `StaticCall`'s arm, just with
     `receiver: Some(lowered_object)` instead of `None`). This is a real design question worth 1-2
     paragraphs before writing lowering code — CLAUDE.md's "ask about tradeoffs" bar is arguably met here
     (it changes every existing snapshot's function signature line), so a session picking this up should
     settle it explicitly rather than picking silently, though the direction is fairly clear from the
     `check_method` precedent.
   - **Property access (`$obj->prop`) and array access (`$arr[$i]`)** are unrelated to the instance-call
     question and can land independently, in either order. Both are harder than `new`/a static call was:
     ADR 0036 § 4's shape/`object`-erasure semantics (a field a shape type names is proven present at
     compile time and never throws; a name it doesn't list, or a plain `object` receiver, is erased to
     `mixed` with the actual runtime-checked throw deferred to M4) need an actual decision at the IR level —
     is there a compile-time-known-field-offset instruction now, a placeholder/panic until M4, or something
     else? `ExprTypeTable` does **not** yet have a `Property`/`Index` variant — that's new design work for
     this slice, following the same "narrow, purpose-built, span-keyed" shape `Call`/`New` already
     established, not a reason to revisit the table's overall design.
4. **Non-scalar *data* values (`string`/`bytes`, arrays) and refcount operations.** The milestone text's
   third named ingredient; `Ty::Object` (landed last session) covers the object-reference case but carries
   no refcount operations yet either — still nothing to attach one to until this lands.
5. **Runtime-helper calls** — the milestone's fourth named ingredient, for `mixed`/union operands once they
   exist in the IR, and also what a non-`bool` `if`/`while` condition's ADR 0035 truthy conversion needs.
6. **Virtual dispatch** — every call lowered so far (`new`'s constructor, a static call) is statically
   resolved with no dispatch question at all. Whether a real vtable/interface-dispatch lookup belongs at
   this IR level (as opposed to purely at codegen, once M3 exists) is an open question for whichever session
   first lowers a call through an interface-typed or overridden-method receiver — likely the instance-call
   session above, since that's the first shape where the receiver's *static* type and its *runtime* class
   can actually differ.
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
