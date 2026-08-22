//! MWL's CFG/SSA IR — `docs/implementation-plan.md`'s M2 milestone's last open
//! thread. See `NEXT_SESSION_PROMPT.md` for how this crate grew: the milestone
//! text ("a CFG/SSA IR carrying explicit safepoints, refcount operations and
//! runtime-helper calls, with a stable per-statement/per-edge id" — see
//! [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md))
//! is milestone-sized on its own, so this first slice deliberately narrows to
//! exactly what the plan's own M2 *Verify* bullet asks for first: "lower a
//! first, narrow slice ... end to end with a snapshot test, before widening."
//!
//! # What this crate lowers so far
//!
//! One method whose body is typed local declarations, a `var $x = expr;`
//! inferred-type declaration (ADR 0037), plain `$x = expr;` reassignment,
//! `return`, nested `{}` blocks, `if`/`while`, `new`, a static method call
//! (`self::method(...)`/`Class::method(...)`), an instance method call
//! (`$obj->method(...)`, including `$this->…`), a compile-time-known
//! property access (`$obj->prop`, including `$this->prop`) both read and
//! written (`$obj->prop = expr;`), and a `string`- or `bytes`-typed local/
//! parameter/return value/call-argument/property-field, initialized,
//! reassigned, passed, returned, read or written from a literal (`string`
//! only — `bytes` has no literal syntax), another local, a compile-time-known
//! property or a resolved call's own result — with refcount retain/release
//! operations around every one of those boundaries, plus `.` string
//! concatenation, including a scalar (`int`/`uint`/`float`/`bool`) operand
//! converted through this crate's first runtime-helper-call shape, and now a
//! positional `array<T>` literal (`[...]`/legacy `array(...)`, refcounted
//! like `string`/`bytes`), and now an array-element read and write through a
//! known `int`/`uint`/`string` key (`$arr[$i]`, `$arr[$i] = expr;`) —
//! [`lower::lower_method`] is the entry point. No `for`/`switch`/`try`, no
//! `break`/`continue`, no `$a[]` append syntax on either side, no explicit
//! `key =>`/`...spread`/`&value` array-literal element, no
//! concatenation of a `Stringable`-object operand (a class/enum value itself
//! also has a representation, [`ty::Ty::Object`], just not a way to refcount
//! one yet, nor a way to invoke its `toString()` from here). The straight-line
//! subset was deliberately the *first* slice landed
//! (see git history and `docs/implementation-plan.md`'s M2 paragraph) because
//! it was the smallest shape exercising every structural IR piece with no
//! merge point at all; `if`/`while` came next, and are where SSA's actual
//! join/phi question gets answered — see [`lower`]'s own module docs for
//! exactly how. `new`/a static call were the third slice, and the first to
//! need more than the AST alone — see the next section for the dependency
//! that unlocked them. An instance method call is the fourth slice, and the
//! first to need a receiver represented as a real value — see the
//! design-choices section below for the implicit-receiver-parameter shape
//! that unlocked it. A property access is the fifth slice, and reuses that
//! same receiver-as-a-value machinery, only for a field read instead of a
//! call — see [`ir::InstKind::FieldGet`]'s own doc comment for the
//! compile-time-known-field-only shape landed here. `var` locals and
//! multi-base (`0x`/`0o`/`0b`) integer-literal cooking are the sixth slice,
//! closing out two gaps this crate had been carrying since the straight-line
//! slice — neither needed a new IR shape, only reusing `Lowering::lower_expr`'s
//! existing `expected: None` inference path for `var`, and widening a new
//! `int_literal_digits` helper's radix handling for the literal forms.
//! `string` locals and the retain/release IR shape are the seventh slice,
//! and the first non-scalar *data* representation to land at all — see the
//! design-choices section below for the retain/release insertion policy this
//! needed. Widening that same `string` representation across a call
//! argument, a resolved return type and a compile-time-known property field
//! is the eighth slice, and needed no new IR shape at all — only
//! [`lower::lower_checked_ty`] gaining a `String` arm and the existing
//! aliasing-vs-fresh judgment ([`lower::is_aliasing_read`]) extending to a
//! property read and a call argument/return boundary, both described in the
//! design-choices section below. A bare call/`new` used purely as its own
//! statement (`doSomething();`, with no assignment at all — the ordinary way
//! to invoke a `void`-returning method) is the ninth slice: `StmtKind::Expr`
//! now dispatches through [`lower::Lowering::lower_expr_stmt`], which routes
//! a plain reassignment to the existing [`lower::Lowering::lower_reassignment`]
//! and a bare `MethodCall`/`StaticCall`/`New` through the ordinary
//! `lower_expr` path, releasing its result immediately when
//! [`ty::Ty::is_refcounted`] since nothing else will ever bind or return it —
//! no new `InstKind` needed. A `string`-typed property *write*
//! (`$obj->prop = expr;`) is the tenth slice, closing the read/write
//! asymmetry the eighth slice left open: [`lower::Lowering::lower_reassignment`]
//! now matches on the assignment target — a plain local still binds into
//! `Env` exactly as before, and an [`mwl_syntax::ast::ExprKind::PropertyAccess`]
//! target lowers to a new [`ir::InstKind::FieldSet`], wrapped in the same
//! retain-then-release policy [`lower::Lowering::bind_local`] already applies
//! to a local — retain the new value first if it's an aliasing read, then
//! read the field's *previous* value back with a `FieldGet` and release it
//! (a field has no `Env` entry to consult before the overwrite the way a
//! local does, so re-reading it is the only way to name the value being
//! replaced). `.` string concatenation between two `string` operands is the
//! eleventh slice: [`mwl_syntax::ast::ExprKind::Binary`] gains a dedicated
//! arm ahead of the scalar-operator table for `BinaryOp::Concat`, lowering
//! to a new [`ir::InstKind::Concat`] rather than [`ir::InstKind::BinOp`]
//! (concatenation allocates a fresh buffer, unlike a native scalar op) — see
//! that variant's own doc comment for why neither operand needs a retain
//! (each is only read, never stored into a second durable slot) and why the
//! result needs none either (a concatenation is a fresh producer, same as a
//! literal or a call's result). A non-`string` operand — a scalar or a
//! `Stringable` object, both of which `mwl_types::expr::check_expr`'s own
//! `require_stringable` already accepts — still panics naming the mismatch:
//! converting either to `string` needs a runtime-helper call this crate has
//! no shape for yet (see the known gaps below).
//!
//! Runtime-helper calls themselves — the milestone's third named ingredient
//! — are the twelfth slice, and land narrowly scoped to exactly the gap the
//! eleventh slice named: a new [`ir::InstKind::HelperCall`] instruction,
//! tagged with a closed, non-exhaustive [`ir::Helper`] enum (`IntToString`/
//! `UintToString`/`FloatToString`/`BoolToString`), lets
//! [`lower::Lowering::concat_operand`] convert a scalar `.` operand to
//! `Ty::Str` before [`ir::InstKind::Concat`] ever sees it — closing that
//! part of the gap, while a `Stringable`-object operand still panics (see
//! the design-choices section below for why that half needs more than a new
//! IR shape). Landing this also surfaced and fixed a latent leak in the
//! eleventh slice's own `Concat` lowering: a fresh, non-aliasing `Ty::Str`
//! operand consumed only by `Concat` and never bound into any durable slot
//! (a bare literal, previously) had nothing that would ever release it —
//! `concat_operand` now reports whether the value it returns
//! [`lower::is_aliasing_read`]s a durable slot, and `Concat`'s own caller in
//! [`lower::Lowering::lower_expr`] releases it right after when it doesn't,
//! the same "release a fresh value once its one and only use is done"
//! precedent the ninth slice's bare call/`new` statement already set.
//!
//! `bytes` is the thirteenth slice, and lands exactly as the seventh slice's
//! own doc comment anticipated: a mechanical repeat of [`ty::Ty::Str`]'s
//! shape, not a new design. [`ty::Ty::Bytes`] is a second refcounted,
//! heap-allocated representation; [`lower::lower_decl_type`] and
//! [`lower::lower_checked_ty`] both gained a `Bytes`/`CheckedTy::Bytes` arm
//! alongside their existing `String` one, and [`ty::Ty::is_refcounted`] now
//! includes it. No new [`lower::Lowering`] insertion point was needed at all,
//! since [`lower::Lowering::bind_local`], [`lower::Lowering::lower_call_args`],
//! [`lower::Lowering::release_all_locals`] and
//! [`lower::Lowering::lower_reassignment`]'s property-target arm all key off
//! [`ty::Ty::is_refcounted`]/[`lower::is_aliasing_read`] rather than naming
//! `Ty::Str` directly. One asymmetry with `string`, not a gap in this slice:
//! `mwl-syntax`'s grammar has no `bytes` literal syntax at all (no `b"..."`
//! form or equivalent), so unlike `string`'s [`ir::InstKind::ConstStr`],
//! nothing produces a *fresh* `bytes` value from a literal — every `bytes`
//! value a fixture lowers today originates as a parameter or a compile-time-
//! known property read, both already-covered [`lower::is_aliasing_read`]
//! shapes. A `Core\Bytes` conversion/constructor, once one exists (M7/M8),
//! would be the first fresh producer; nothing about this slice's
//! representation needs to change when it lands.
//!
//! `array<T>` is the fourteenth slice, and the first widening this crate has
//! done since `string`/`bytes` landed a third refcounted representation:
//! [`ty::Ty::Array`] is a bare, opaque unit variant carrying no element type
//! at all — the same "representation, not identity" erasure
//! [`ty::Ty::Object`] already gives a class/enum, chosen because no lowering
//! decision made so far needs to branch on an array's *element* type at this
//! IR level (`mwl_types::ty::Ty::Array(TypeId)` already enforces that at
//! check time — see [`ty::Ty`]'s own module doc for the full split). A
//! positional array literal (`[...]`/legacy `array(...)`, no explicit
//! `key =>`, no `...spread`, no `&value`) lowers to a new
//! [`ir::InstKind::ArrayNew`] — see that variant's own doc comment for the
//! fixed `(key, value)`-pairs shape and why each key is a decimal string
//! computed at lowering time, never a lowered expression. [`ty::Ty::Array`]
//! is [`ty::Ty::is_refcounted`], so [`lower::Lowering::lower_expr`]'s new
//! `ArrayLiteral` arm applies the exact same caller-side retain
//! [`lower::Lowering::lower_call_args`] already gives a refcounted, aliasing
//! call argument to each element that [`lower::is_aliasing_read`]s existing
//! storage — no new policy, only a new call site for the existing one. An
//! explicit `key =>` entry, a `...spread` element, and a `&value` element are
//! all still unsupported — see the known gaps below.
//!
//! Array-element access (`$arr[$i]`, both read and write) is the fifteenth
//! slice, and the natural pickup once `ir::InstKind::ArrayNew` existed to
//! give an array a representation at all. The design question this slice
//! actually had to answer was narrower than "how does a missing key
//! behave": `mwl_types::expr::check_expr`'s own `ExprKind::Index` arm has no
//! concept of key *presence* at compile time at all — it resolves the same
//! element type regardless of whether a given key exists at runtime, exactly
//! the way `check_property_access`'s shape/`object`-erasure case already
//! does for a field — so the missing-key runtime behavior (PHP's own
//! warning-and-`null` read, autovivification on write) was never actually in
//! scope to decide; it is deferred wholesale, the same way every other
//! checked-throw is (no `try`/`throw` lowering exists in this crate yet —
//! see the design-choices section's `HelperCall` bullet for the identical
//! reasoning already applied to `Call`/`New`). What this slice *did* land: a
//! new `mwl_types::expr_table::ExprInfo::Index { elem_ty }` entry — recorded
//! by `check_expr`'s `Index` arm exactly when the base statically resolved
//! to a known `array<T>` element type, and left unrecorded when it erased to
//! `mixed`, mirroring `ExprInfo::Property`'s own split — which
//! [`lower::Lowering::lower_expr`]'s new `Index` arm reads back to type a
//! new [`ir::InstKind::ArrayGet`], and
//! [`lower::Lowering::lower_reassignment`]'s new `Index`-target arm reads
//! back the same way to emit a new [`ir::InstKind::ArraySet`]. Both need a
//! `Ty::Str` key, so a new [`lower::Lowering::lower_array_key`] normalizes an
//! `int`/`uint` subscript to its decimal-string form (ADR 0007 § 5, `$a[8]`
//! is `$a["8"]`) by reusing [`ir::Helper::IntToString`]/
//! [`ir::Helper::UintToString`] verbatim — the exact conversion
//! `concat_operand` already had, needing no new `Helper` variant. `ArraySet`
//! deliberately does *not* mirror `FieldSet`'s read-old-value-then-release
//! shape: a class field always exists once its instance is definitely
//! initialized (ADR 0022), but an array key may or may not already be
//! present, so a conditional get here would model exactly the question this
//! slice already deferred — see that variant's own doc comment for why the
//! whole replace-or-insert stays bundled into one instruction instead,
//! deferred to whatever `mwl-codegen`'s own array-mutation primitive does
//! with a repeated key. [`lower::is_aliasing_read`] gained `ExprKind::Index`
//! alongside `ExprKind::PropertyAccess` — an array read borrows the same
//! "storage some other binding still owns" reference a property read does
//! (ADR 0007 § 5's copy-on-write semantics), so every existing retain call
//! site (`bind_local`, `lower_call_args`, `StmtKind::Return`) picked this up
//! with no new insertion point, the same "extend the judgment, not the call
//! sites" pattern the thirteenth/fourteenth slices already established for
//! `bytes`/`array<T>` themselves. `$a[]`/`$a[] = expr;` (PHP's append
//! syntax, `index` is `None`) is unsupported on both sides — it needs a
//! "next available integer key" counter this crate has no representation
//! for yet — and a non-`int`/`uint`/`string` subscript (a `float`/`bool`/
//! `null` key ADR 0007 § 5 itself rejects, which `mwl_types` doesn't yet
//! enforce either — the same known gap `ArrayNew`'s own doc comment already
//! names for an array literal's explicit `key =>`) still panics in
//! `lower_array_key` naming the case.
//!
//! # Design choices worth knowing before widening this further
//!
//! - **SSA, not a plain CFG.** `docs/implementation-plan.md`'s M2 paragraph
//!   already commits to "a CFG/**SSA** IR" (not left open by this session) —
//!   adopted here rather than reopened, per CLAUDE.md's "mechanical
//!   follow-through of what the plan already committed to" carve-out.
//!   `if`/`while` are each a single, hand-rolled two-predecessor (or
//!   pre-loop/back-edge) merge, not a general dominance-based phi-placement
//!   algorithm — sufficient for any structured `if`/`while` nesting, since
//!   neither ever produces a join point of another shape. `for`/`switch`
//!   will reuse the same two building blocks (`Lowering::merge_envs` for a
//!   fixed set of incoming edges known up front, the seed-then-patch phi
//!   dance in `Lowering::lower_while` for a join whose back edge isn't known
//!   until its body is lowered) rather than needing a new algorithm.
//! - **IR types are representation-level, not the checker's types.** See
//!   [`ty`]'s own module docs for why [`ty::Ty`] is a small, flat lattice
//!   distinct from `mwl_types::ty::Ty` rather than a reuse of it.
//! - **This crate depends on `mwl-types`, but only for its typed-expression
//!   table — never for `mwl-hir`'s class graph/signature tables directly.**
//!   Every *declared* type (a parameter's, a local's, a method's return type)
//!   is still read straight off the `mwl-syntax` AST via `lower::lower_decl_type`
//!   (renamed from the earlier slice's `lower_scalar_type`, since it now
//!   covers one non-scalar case too), exactly as before: ADR 0007 § 1 already
//!   requires it to be spelled out there in full, so no name resolution is
//!   needed to answer "what type is this" — a plain class-name atom now
//!   erases to [`ty::Ty::Object`] the same way a scalar atom erases to its own
//!   `Ty` variant, needing no more resolution than a scalar did. What *did*
//!   need a new dependency is a call's or `new`'s *resolved target* — which
//!   class actually declares the callee, its parameter/return types — since
//!   that is genuinely absent from the AST (a call site only spells the
//!   method name, not which class in an inheritance chain declares it).
//!   The two options weighed for that were (a) this crate depending on
//!   `mwl-types` and duplicating/re-running its class-hierarchy resolution,
//!   or (b) `mwl-types` publishing a persisted result this crate reads back.
//!   (b) was chosen: `mwl_types::expr_table::ExprTypeTable` is a narrow,
//!   purpose-built table — one `ExprInfo::Call`/`ExprInfo::New` entry per
//!   resolved call/`new`, keyed by the expression's own source span (see that
//!   module's own docs for why a span, not an id, is the lookup key across
//!   this crate boundary) — that `mwl_types::check_program` populates once and
//!   [`lower::lower_method`] reads afterward, via two new parameters
//!   (`exprs`/`checked_types`). This keeps the coupling narrow: this crate
//!   still never depends on `mwl-hir`, `mwl_types::signatures`, or
//!   `mwl_types::ClassGraph` — only on the one table and the type interner
//!   needed to translate a recorded `TypeId` into this crate's own `Ty` (see
//!   `lower::lower_checked_ty`). [`lower::lower_method`] still deliberately
//!   **trusts** that its input already passed `mwl_types::check_program` —
//!   with the very same `exprs`/`checked_types` handed to it — and panics
//!   (naming the unsupported shape) rather than diagnosing when handed
//!   something outside this slice's scope, or when a table lookup comes back
//!   empty for an expression that should have one.
//! - **`$this`/a receiver is an implicit first parameter, not a special-cased
//!   field.** Landing an instance method call needed `$this` (and any other
//!   receiver) represented as a real `ValueId` first — [`lower::lower_method`]
//!   used to seed `Env` only from `m.params`. The shape chosen mirrors
//!   `mwl_types::check.rs`'s `check_method`, which already seeds `$this` into
//!   its own `LocalScope` the same way, unconditionally and not gated on a
//!   `static` modifier (a static method's body referencing `$this` is a
//!   distinct, unrelated diagnostic neither crate adds here): every lowered
//!   method's [`ir::Function::params`] now carries the receiver at index 0,
//!   ahead of every explicit parameter, whether or not the body ever reads
//!   `$this`. The alternative — a receiver-only special case that leaves
//!   `Function::params` untouched and threads a separate `Option<ValueId>`
//!   just for `$this` — was rejected: it would need `Env`'s `$this` lookup to
//!   go through a different path than every other local, duplicating the
//!   `ExprKind::Variable` handling `lower_expr` already has, for a value that
//!   behaves exactly like an ordinary parameter in every other respect. This
//!   changes every existing snapshot's function signature line (regenerated
//!   via `cargo insta test --accept -p mwl-ir` when this landed) — an
//!   IR-representation choice, not a change visible to an MWL developer.
//! - **Ids are stable, not global.** See [`ids`]'s own module docs.
//! - **Refcount insertion is naive and syntactic, not a liveness/move
//!   analysis — correctness first, elision left to a later optimizer pass.**
//!   `docs/implementation-plan.md`'s own optimizer feature list already names
//!   "refcount elision" as separate future work, distinct from *emitting* the
//!   operations at all — this session only had to answer the latter. Two
//!   designs were weighed: (a) a full last-use/move analysis that only
//!   retains when a value is genuinely shared and skips it otherwise, or (b)
//!   inserting a retain everywhere a value is copied into a second durable
//!   slot and a release everywhere a slot's value is overwritten or the slot
//!   itself goes out of scope, with no attempt to prove a copy was
//!   unnecessary. (b) was chosen: CLAUDE.md's priority ordering ranks
//!   correctness and simplicity ahead of memory/latency, nothing can execute
//!   this IR yet to make (a)'s payoff measurable, and ADR 0004/0006/the
//!   project's own architecture notes already commit to "a refcount per
//!   value ... moved only when the refcount is 1" as a *codegen-time*
//!   optimization for the isolate-boundary case specifically — generalizing
//!   that to ordinary lowering here would be scope creep beyond what any ADR
//!   asks for, not a mechanical extension of it. Concretely: reading a value
//!   out of storage some other binding still owns — [`lower::is_aliasing_read`]
//!   names exactly three such shapes today, a bare `ExprKind::Variable`, a
//!   compile-time-known `ExprKind::PropertyAccess`, and (as of the fifteenth
//!   slice) a compile-time-known `ExprKind::Index` — and copying it into
//!   another durable slot needs a retain first; a freshly constructed value
//!   (a string literal, `new`, or a call's own result) needs none, since it
//!   already has exactly one natural owner and the copy just gives that
//!   owner a new name/slot. A slot's *previous* value is released whenever
//!   it's overwritten, and every slot still live at a
//!   `return`/implicit-`void`-fallthrough is released too — except the one
//!   slot whose value is the return expression itself when that expression is
//!   a bare `$name` read, which transfers out instead (see
//!   [`lower::Lowering::release_all_locals`]'s own doc comment for exactly
//!   why excluding it there, rather than retaining it and releasing
//!   everything unconditionally, keeps the count exactly balanced even under
//!   aliasing). This one judgment now covers every "durable slot" a `string`
//!   value can be copied into: a local bind
//!   ([`lower::Lowering::bind_local`]), a resolved call's argument
//!   ([`lower::Lowering::lower_call_args`] — the callee's own parameter is
//!   just another local, released at the callee's own exit, so the caller's
//!   retain and the callee's release are a symmetric pair, exactly mirroring
//!   what a local's own declare/drop already does), and a returned value
//!   (`Lowering::lower_stmt`'s `StmtKind::Return` arm — a property read has
//!   no local slot for `release_all_locals` to exclude the way a bare
//!   variable does, so it retains explicitly there instead). `.`
//!   concatenation ([`ir::InstKind::Concat`]) needed neither a retain of its
//!   operands (each is read, not copied into a new durable slot — the same
//!   treatment [`ir::InstKind::FieldGet`] already gives its `object`
//!   receiver) nor of its own result (a fresh producer, same as `ConstStr`/
//!   `New`/`Call`) — see that variant's own doc comment. It does need a
//!   *release* of either operand right after `Concat` reads it, when that
//!   operand [`lower::is_aliasing_read`] is `false` — i.e. when nothing else
//!   already owns a slot that will release it later. This was missed when
//!   the eleventh slice landed (a bare `"a" . "b"` leaked both literals) and
//!   is fixed as part of this slice, in [`lower::Lowering::concat_operand`]'s
//!   caller. What stays a known gap: converting a `Stringable`-object
//!   operand for `.` (needs a resolved `toString` call this crate can't
//!   synthesize from a bare `.` operand — see the design-choices bullet
//!   below), and a `tainted`/`secret`-qualified string (`lower_checked_ty`
//!   only handles the plain, unqualified `string` type — see the known gaps
//!   below).
//! - **A closed, engine-owned runtime-helper call gets its own
//!   [`ir::InstKind::HelperCall`], tagged by a non-exhaustive [`ir::Helper`]
//!   enum, rather than reusing [`ir::InstKind::Call`] with a synthetic
//!   target label or a string helper name.** Three designs were weighed: (a)
//!   a dedicated instruction with an enum tag, (b) `InstKind::Call` with a
//!   reserved-namespace string `target` (e.g. `"Core::intToString"`), (c) a
//!   dedicated instruction with a string name instead of an enum. (a) was
//!   chosen. Against (b): `Call::target`'s own doc comment already scopes it
//!   to a target [`mwl_types::expr_table::ExprTypeTable`] actually resolved
//!   from the class hierarchy, and `Call::receiver` only makes sense for a
//!   user-level instance call — a runtime helper has neither a class-graph
//!   origin nor a receiver, so folding it into `Call` would blur exactly the
//!   line the "no virtual dispatch" known gap below depends on staying
//!   sharp (a future interface-dispatch lookup only ever has to consider
//!   `Call`, never a helper). Against (c): the helper set is small, closed,
//!   and known entirely to this crate and the future `mwl-codegen` helper
//!   table — never user-extensible — so a string buys nothing a
//!   `#[non_exhaustive]` enum doesn't already give for free, while losing
//!   compile-time exhaustiveness checking and typo-safety; [`ir::BinOp`]/
//!   [`ir::UnOp`] already establish the enum-for-a-closed-operator-set
//!   precedent this follows. `HelperCall` also does **not** yet model ADR
//!   0002's checked-return convention (no status value, no error edge) —
//!   deliberately, since [`ir::InstKind::Call`]/[`ir::InstKind::New`]
//!   themselves don't either: nothing in this crate models a call that can
//!   fail at all yet (`try`/`throw` are both still unsupported — see the
//!   known gaps below), so giving only `HelperCall` a checked-return shape
//!   would be a partial, inconsistent step rather than the "shape now,
//!   functional once a backend exists" treatment [`ir::InstKind::Safepoint`]/
//!   [`ir::InstKind::Release`] already get. That convention is expected to
//!   land for `Call`/`New`/`HelperCall` together, whenever `try`/`throw`
//!   lowering needs it.
//!
//! # Known gaps (all deliberate, all deferred to a later widening session)
//!
//! - `for`/`switch`/`match`/`try`, and `break`/`continue` of any kind, are
//!   still unsupported: lowering panics naming the statement.
//!   [`ir::Terminator::Branch`] and [`ids::EdgeId`] are both already
//!   exercised by `if`/`while`, so widening to the rest is expected to reuse
//!   the same shapes rather than add new ones — see [`lower`]'s module docs.
//! - An `if`/`while` condition must already be statically `bool` — ADR
//!   0035's full truthy-table conversion for a non-`bool` condition needs a
//!   `bool`-producing runtime helper per source type (PHP's truthy rule
//!   differs by type: `0`/`0.0`/`""`/`"0"`/an empty array/`null` are all
//!   falsy, everything else truthy), most of which have no IR representation
//!   to convert *from* yet (no `array<T>`, no nullable type) — so this is
//!   naturally sequenced after those land, not purely a "no `HelperCall`
//!   shape" gap now that one exists (see the design-choices section above).
//!   Lowering panics naming this.
//! - No block-scoped shadowing: the environment `crate::lower` threads
//!   through is one flat, function-wide map, exactly like the straight-line
//!   slice's `locals` was. A nested `{}` declaring a local that shadows an
//!   outer one of the same name is not distinguished from a reassignment of
//!   the outer binding — not observable for any program in scope today (no
//!   shape here can declare a same-named local in a narrower scope in a way
//!   that matters), but worth knowing before trusting `Env` further.
//! - **Array-element access is happy-path-only, and only through an
//!   `int`/`uint`/`string` key.** `$arr[$i]`/`$arr[$i] = expr;` lower to
//!   [`ir::InstKind::ArrayGet`]/[`ir::InstKind::ArraySet`] whenever the base
//!   statically resolved to a known `array<T>` element type (an
//!   `mwl_types::expr_table::ExprInfo::Index` entry exists for it) — a base
//!   that erased to `mixed` has no such entry, so lowering panics naming it,
//!   the same split `ExprInfo::Property` already draws for a shape/plain-
//!   `object` receiver. Neither instruction models what happens when the key
//!   is actually absent at runtime (PHP's own warning-and-`null` read,
//!   autovivification on write) — that question is deferred wholesale, the
//!   same way every other checked-throw is (no `try`/`throw` lowering exists
//!   yet), not something this slice had to weigh a design against (see the
//!   fifteenth-slice paragraph above for why `mwl_types` itself has no
//!   compile-time "is this key present" concept to consult in the first
//!   place). `$a[]`/`$a[] = expr;` (append syntax, `index` is `None`) is
//!   unsupported on either side — it needs a "next available integer key"
//!   counter this crate has no representation for yet. A `float`/`bool`/
//!   `null` subscript — the three source types ADR 0007 § 5 itself rejects
//!   as a key, which `mwl_types` doesn't yet enforce either — still panics
//!   in `lower::Lowering::lower_array_key` naming the case. A literal with
//!   an explicit `key =>`, a `...spread` element, or a `&value` element is
//!   equally unsupported — `crate::ir::InstKind::ArrayNew`'s own doc comment
//!   explains why (`mwl_types::expr::check_array_literal` itself has no
//!   key-normalization/rejection logic yet either, so lowering an explicit
//!   key would mean guessing at a runtime conversion this crate can't yet
//!   synthesize).
//! - **Property access, read or write, is compile-time-known-field-only.** A
//!   receiver whose static type resolved to a known declaring class lowers a
//!   read to [`ir::InstKind::FieldGet`] and a write (`$obj->prop = expr;`) to
//!   [`ir::InstKind::FieldSet`], both reading
//!   `mwl_types::expr_table::ExprInfo::Property` the same way a call reads
//!   `ExprInfo::Call`. A receiver that erased to a shape or plain `object`
//!   (ADR 0036 § 4) has no such entry at all — the checker itself defers
//!   that case's runtime-checked fallback to M4, with no IR/codegen yet to
//!   throw from, so lowering panics naming it rather than guessing a
//!   representation. A nullsafe access (`?->`) is equally unsupported today
//!   on either side, same as a nullsafe method call.
//! - **`string`/`bytes`/`array<T>` all cross a local, call-argument,
//!   resolved-return, and compile-time-known property-read *and write*
//!   boundary.** [`lower::lower_checked_ty`] has a `CheckedTy::String =>
//!   Ty::Str` arm, a `CheckedTy::Bytes => Ty::Bytes` one, and, as of the
//!   fourteenth slice, a `CheckedTy::Array(_) => Ty::Array` one beside
//!   them — so a call/`new` argument, a resolved return type, and a property
//!   read or write (`$obj->prop`/`$obj->prop = expr;`) all lower for
//!   `array<T>` too, with the same retain policy a local already had; no new
//!   insertion point was needed, the same way `bytes` needed none (see the
//!   design-choices section above). `array<T>` can now read back an
//!   already-lowered array's own element too (`$arr[$i]`, both read and
//!   write) — see the array-access bullet above for its own, narrower known
//!   gaps. Every representation here still only covers the plain,
//!   unqualified `string`/`bytes` type:
//!   `lower_checked_ty` has no arm for any of the eight qualified
//!   `CheckedTy::TaintedString`/`SecretString`/`SecretTaintedString`/
//!   `TaintedBytes`/`SecretBytes`/`SecretTaintedBytes` variants (ADR
//!   0024/0033), so a `tainted`/`secret`-qualified parameter, return or field
//!   still panics there — those qualifiers need their own laundering/sink
//!   story before they can flow through an IR value at all, deliberately out
//!   of scope here. [`ty::Ty::Object`] is a reference too, but nothing
//!   allocates or frees the memory behind one yet, and no retain/release is
//!   emitted for one — see that variant's own doc comment for exactly what is
//!   and isn't modeled; extending `Ty::is_refcounted` to include it is
//!   expected to reuse the exact same
//!   `bind_local`/`lower_call_args`/`release_all_locals` insertion points
//!   `Ty::Str`/`Ty::Bytes`/`Ty::Array` already use, not new ones.
//! - **No virtual dispatch** — [`ir::InstKind::Call`]'s `target` is always the
//!   statically resolved declaring class from
//!   `mwl_types::expr_table::ResolvedCall`, exactly as MWL's checker resolved
//!   it, for a static call, `new`'s constructor, and now an instance method
//!   call alike. Every instance call lowered so far still has its receiver's
//!   *static* type equal to its *runtime* class (a concrete, non-interface
//!   local/`new` result) — whether a real vtable/interface-dispatch lookup is
//!   ever needed at this IR level (as opposed to purely at codegen) remains a
//!   question for whichever session first lowers a call through an
//!   interface-typed or overridden-method receiver, where the two can
//!   actually differ.
//! - **No variadic, named, or spread call argument** —
//!   `Lowering::lower_call_args` (in [`lower`]) panics naming any of the
//!   three; `mwl_types` itself doesn't fully positionally type-check a
//!   named/spread argument against a signature yet either (see its own known
//!   gaps), so there is no resolved per-argument type to lower against even
//!   if this crate wanted to try.
//! - Safepoints are reserved, not functional. [`ir::InstKind::Safepoint`] is
//!   emitted at function entry and at every `while` back edge (see that
//!   variant's own doc comment), but it is inert — no codegen exists yet to
//!   lower it to an actual CPU-limit/cancellation/cycle-collector check, and
//!   no guard test needs it functional before M3's backend does. `for`
//!   loops will need the same back-edge marker once they land.
//! - **Runtime-helper calls (the milestone's third named ingredient) now
//!   exist, but only for `.`'s scalar-to-`string` conversion.**
//!   [`ir::InstKind::HelperCall`]/[`ir::Helper`] are landed and used by
//!   [`lower::Lowering::concat_operand`] — see the twelfth-slice paragraph
//!   above and the design-choices section for the shape this took. Ordinary
//!   arithmetic still lowers directly to [`ir::InstKind::BinOp`]/
//!   [`ir::InstKind::UnOp`] with no helper fallback, since a `mixed`/union
//!   operand has no IR representation to dispatch on yet — that, and ADR
//!   0035's truthy conversion above, are expected to add new [`ir::Helper`]
//!   variants to the same enum rather than a second call-shaped instruction.
//! - ~~Integer literal magnitude range-checking.~~ **Done**, at check time:
//!   `mwl_types::expr::infer`'s own `ExprKind::Int` arm now enforces ADR 0007
//!   § 4's exact rule — a literal too large for `int` is legal only where a
//!   `uint` is expected, and one too large even for `uint`'s full `u64` range
//!   is a diagnostic regardless — so [`lower::Lowering::lower_expr`]'s
//!   `ExprKind::Int` arm treats an out-of-range literal as unreachable input,
//!   the same "trusts `mwl_types::check_program` already ran" contract every
//!   other panic in this crate already relies on. A negative literal (`-5`)
//!   needed no new check at all: it's a separate, wrapping `ExprKind::Unary`
//!   node whose inner literal is checked with no expected type, so `-5`
//!   always types as plain `int`, and assigning that `int` into a `uint`
//!   target already reports the ordinary `E_TYPE_MISMATCH` `is_assignable`
//!   gives any other `int`-into-`uint` mismatch — no magnitude-specific
//!   diagnostic was needed for that half. One asymmetry deliberately left
//!   unhandled: a literal that overflows `int` by exactly one (a bare
//!   `9223372036854775808` immediately negated) is reported as "too large for
//!   `int`" even though `-9223372036854775808` is `i64::MIN`, a perfectly
//!   representable value — recognizing that one specific
//!   `ExprKind::Unary { op: UnaryOp::Neg, expr: Int(_) }` shape as a signed
//!   literal rather than an unsigned one negated would be a second, narrower
//!   rule ADR 0007 § 4's own text doesn't ask for (and `as int` is not itself
//!   a workaround: `ExprKind::Conversion`'s own inner-expression check also
//!   passes `expected: None` down to the literal, so `9223372036854775808 as
//!   int` reports the identical diagnostic) — left as a real, if narrow,
//!   MWL/PHP divergence for whoever picks up `Core`'s integer-limit constants
//!   to note.
//! - ~~String-literal cooking is escape-incomplete, and interpolation isn't
//!   lowered at all.~~ **Done for every double-quoted-sourced case.** A
//!   numeric escape (`\xHH` hex, `\NNN` octal, `\u{...}` Unicode) now cooks to
//!   the byte/codepoint it names, and a non-heredoc `ExprKind::Interpolated`
//!   lowers to the same [`ir::InstKind::Concat`] chain a written-out `.`
//!   expression already produces — see `lower::Lowering::lower_interpolated_parts`'s
//!   own doc comment for the one refcount subtlety a single-part `"$x"` (no
//!   surrounding literal text, so no `Concat` ever runs) forces into the
//!   open. Both share `mwl_types::string_lit::cook_double_quoted_text`'s
//!   escape grammar with the checker, which is what lets `mwl_types::expr::infer`'s
//!   own `ExprKind::Str`/`ExprKind::Interpolated` arms diagnose exactly the
//!   cooking this crate performs, rather than the two crates disagreeing on
//!   what a given escape means. **Still unsupported, and still a panic naming
//!   the case:** a heredoc/nowdoc-sourced `ExprKind::Str` or `ExprKind::Interpolated`
//!   — this crate has no flexible-heredoc indentation-stripping story yet, so
//!   it refuses to guess a representation rather than emit text with the
//!   wrong leading whitespace baked in.
//! - **`.` string concatenation still doesn't cover a `Stringable`-object
//!   operand.** [`lower::Lowering::concat_operand`] converts a scalar
//!   operand to `string` through [`ir::InstKind::HelperCall`], but an object
//!   whose class implements `Stringable` — which
//!   `mwl_types::expr::check_expr`'s own `require_stringable` already
//!   accepts, since PHP's `.` implicitly stringifies it via `toString()` —
//!   still panics naming the case. Closing this needs more than a new IR
//!   shape: `.`'s desugaring would have to synthesize a resolved call to the
//!   receiver's own `toString()`, but a bare `.` operand isn't a call
//!   expression, so `mwl_types::expr_table::ExprTypeTable` records no
//!   `ExprInfo::Call` for it the way an actual `$obj->toString()` call site
//!   would have — this either needs the checker to start recording that
//!   resolution for a `.` operand too, or this crate to re-resolve
//!   `toString` on its own account (the second `mwl-types` dependency this
//!   crate has so far tried to avoid — see the design-choices section
//!   above). Worth deciding deliberately rather than guessing at, whenever a
//!   fixture actually needs it.

pub mod ids;
pub mod ir;
pub mod lower;
pub mod print;
pub mod ty;

pub use ir::{Function, Program};
pub use ty::Ty;

use mwl_diagnostics::{SourceFile, Span};

pub(crate) fn span_text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

/// Strips a variable's leading `$` sigil, if present — the same idiom
/// `mwl-types` uses.
pub(crate) fn strip_sigil(s: &str) -> &str {
    s.strip_prefix('$').unwrap_or(s)
}
