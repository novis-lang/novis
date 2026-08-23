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
//! known `int`/`uint`/`string` key (`$arr[$i]`, `$arr[$i] = expr;`), and now
//! an `if`/`while` condition that isn't already `bool` — a scalar or
//! `array<T>` converts through ADR 0035's truthy table (a class instance or
//! enum case folds straight to a constant `true`, always truthy) — and now a
//! `mixed`-typed local/parameter/return value/call-argument, which exists as
//! an IR representation and round-trips, though nothing yet converts one
//! through arithmetic, `.` concatenation, ADR 0035's truthy table or an
//! array-element access, and now `$a[] = expr;` append syntax on the write
//! side (`$a[]` as a read has no PHP meaning at all, so it stays unsupported
//! by design, not by gap), and now `foreach` over an `array<T>` and
//! `unset($a[$k])`, the array's two remaining consumers — see
//! [`lower::Lowering::lower_foreach`] and [`lower::Lowering::lower_unset`],
//! which own those two policies — and now a Tier 0 `Core` member call, which
//! resolves through the same `ResolvedCall` a user-declared static call does
//! but lowers to [`ir::InstKind::CoreCall`], naming a helper symbol rather
//! than a compiled MWL function and *borrowing* its arguments rather than
//! transferring them — [`lower::lower_method`] is the entry point for
//! one method, and [`lower::lower_script`] the entry point for a file's own
//! top-level statements, which are one synthesized frame of ordinary locals
//! with no receiver parameter (ADR 0008 § 2 — see that function's own doc
//! comment for the only two ways it differs from `lower_method`, and the
//! `echo`/script-body slice paragraph below). No
//! `for`/`switch`/`try`, no `break N`/`continue N` for `N > 1`, no `...spread`/`&value`
//! array-literal element, no concatenation of a `Stringable`-object operand (a class/enum
//! value itself also has a representation, [`ty::Ty::Object`], just not a
//! way to refcount one yet, nor a way to invoke its `toString()` from here),
//! and now `break`/`continue` for a `while` or `foreach` loop (level 1 only —
//! see the twenty-first slice below for `for`/`switch`, which still aren't
//! lowered at all).
//! The straight-line
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
//! `key =>`) lowers to a new [`ir::InstKind::ArrayNew`] — see that variant's
//! own doc comment for the fixed `(key, value)`-pairs shape and why each key
//! is a decimal string computed at lowering time, never a lowered
//! expression. [`ty::Ty::Array`] is [`ty::Ty::is_refcounted`], so
//! [`lower::Lowering::lower_expr`]'s new `ArrayLiteral` arm applies the exact
//! same caller-side retain [`lower::Lowering::lower_call_args`] already gives
//! a refcounted, aliasing call argument to each element that
//! [`lower::is_aliasing_read`]s existing storage — no new policy, only a new
//! call site for the existing one. A `...spread` element and a `&value`
//! element are still unsupported — see the known gaps below. An explicit
//! `key =>` element landed later, in the sixteenth slice below.
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
//! for yet — and a non-`int`/`uint`/`string` subscript still panics in
//! `lower_array_key` naming the case, now as an unreachable internal-
//! invariant panic rather than a live gap: `mwl_types` rejects a `float`/
//! `bool`/`null` key at check time as of the sixteenth slice below.
//!
//! An array literal's explicit `key =>` element is the sixteenth slice, and
//! the two-part fix the recorded next-session pick called for:
//! `mwl_types::expr::check_array_key_type` (new) is called from both
//! `check_array_literal`'s explicit-key arm and `check_expr`'s `Index` arm,
//! rejecting a `float`/`bool`/`null` key with a new `E_ARRAY_KEY_INVALID_TYPE`
//! diagnostic and leaving `int`/`uint`/`string` (and anything statically
//! unknown, like `mixed`) alone — the same "erase to `mixed` rather than
//! guess" split `division_result`/`bitwise_result` already draw. On the
//! lowering side, a literal with at least one explicit key now builds an
//! *empty* `ArrayNew` followed by one `ArraySet` per element in source
//! order, reusing [`lower::Lowering::lower_array_key`] verbatim for every
//! key (explicit or positional) rather than adding a second key-lowering
//! path — see [`ir::InstKind::ArrayNew`]'s own doc comment for the full
//! split and the one PHP behavior it deliberately doesn't reproduce (a
//! positional element mixed after an explicit `int`/`uint` key numbers from
//! "how many positional elements came before it," not PHP's real "highest
//! int key used so far," since that needs the very "next available integer
//! key" counter the paragraph above already names as a separate, bigger gap).
//! A purely positional literal is untouched — same single `ArrayNew`, same
//! snapshots, zero behavior change — since the split only triggers once an
//! explicit key actually appears. `...spread` and `&value` elements remain
//! exactly as unsupported as before this slice.
//!
//! ADR 0035's truthy-table conversion for a non-`bool` `if`/`while` condition
//! is the seventeenth slice, and a deliberately partial one, exactly as the
//! plan called for: a `bool` condition passes straight through as before,
//! and a scalar (`int`/`uint`/`float`/`string`) or [`ty::Ty::Array`] condition
//! now converts through five new [`ir::Helper`] variants (`IntTruthy`/
//! `UintTruthy`/`FloatTruthy`/`StrTruthy`/`ArrayTruthy`) reusing the exact
//! [`ir::InstKind::HelperCall`] shape the twelfth slice already introduced —
//! [`lower::Lowering::lower_if`]/[`lower::Lowering::lower_while`] both now
//! call a new [`lower::Lowering::lower_truthy_cond`] instead of asserting the
//! condition is already `bool`. A [`ty::Ty::Object`] condition (a class
//! instance or an enum case) needs no helper at all: ADR 0035 § 4 makes
//! either always truthy, so this folds straight to a fresh
//! [`ir::InstKind::ConstBool`] `true` rather than emitting a call with
//! nothing to inspect at runtime. A refcounted condition
//! (`Ty::Str`/`Ty::Array`) that isn't [`lower::is_aliasing_read`] — a fresh
//! call/`new`/literal result whose only use is the truthy test — is released
//! right after the helper reads it, the same precedent
//! [`lower::Lowering::concat_operand`]'s own caller already set for `.`
//! concatenation; an aliasing read (a bare variable, a compile-time-known
//! property or array-element read) needs no extra release here, since its
//! owning slot already releases it normally at reassignment or scope exit.
//! What's still out of scope, deliberately: `null` (no nullable-type IR
//! representation exists yet to convert *from*) and `mixed`/a union (as of
//! the eighteenth slice below, `mixed` *does* have an IR representation, but
//! nothing yet converts one through this table — see that paragraph) — both
//! still wait on more representation work, not on a new `Helper` variant.
//! `&&`/`||`/`!` and the ternary/elvis condition (ADR 0035's other four
//! truthy positions) are unaffected by this seventeenth slice itself — only
//! `if`/`while`'s own condition changed here; see the nineteenth slice below
//! for where those four land.
//!
//! [`ty::Ty::Mixed`] is the eighteenth slice, and the first slice to widen
//! this crate's own IR type lattice rather than what it lowers — the
//! recommended pick left at the end of the seventeenth slice's own session,
//! since it was named as the single biggest unblock left (arithmetic's
//! `mixed` fallback, ADR 0035's `null`/`mixed` truthy case, and item 3's
//! mixed-erased-array-base gap were all separately waiting on it). Scoped
//! narrowly, exactly as that recommendation called for: enough
//! representation for a `mixed`-typed local, parameter, return value or call
//! argument to exist and round-trip, and nothing that dispatches on a
//! `mixed` value's actual runtime type. [`lower::lower_decl_type`] gained a
//! `TypeAtom::Mixed => Ty::Mixed` arm and [`lower::lower_checked_ty`] a
//! `CheckedTy::Mixed => Ty::Mixed` one, mirroring exactly how both functions
//! already erase a class/enum name to [`ty::Ty::Object`] — no new
//! [`lower::Lowering`] insertion point was needed at all, since
//! [`lower::Lowering::bind_local`], [`lower::Lowering::lower_call_args`],
//! [`lower::Lowering::release_all_locals`] and `StmtKind::Return`'s own arm
//! all key off [`ty::Ty::is_refcounted`]/[`lower::is_aliasing_read`] rather
//! than naming any concrete `Ty` variant directly, the same "extend the
//! judgment, not the call sites" precedent the `bytes`/`array<T>` slices
//! already established. [`ty::Ty::is_refcounted`] does **not** include
//! [`ty::Ty::Mixed`] — see that variant's own doc comment for why "erase to
//! one opaque representation" and "know whether to retain/release it" are
//! two separate questions, and why leaving the second one open doesn't break
//! the round-trip this slice actually promises. The real design question the
//! milestone text poses — how a `mixed` value's runtime type tag is
//! represented, needed before any code can branch on what a `mixed` value
//! actually holds — is deliberately **not** answered here: every place that
//! would need it (arithmetic, `.` concatenation, ADR 0035's truthy table,
//! array-element access through a `mixed`-erased base) still panics naming
//! the gap exactly as before, now reachable rather than theoretical for the
//! truthy-table case specifically (a `mixed`-typed `if`/`while` condition
//! could not exist as input before this slice; it can now, and still
//! panics — see [`lower::Lowering::lower_truthy_cond`]'s own doc comment).
//!
//! The nineteenth slice lowers ADR 0035's other four truthy positions:
//! `&&`/`||`/`!` and the ternary/elvis operator (`cond ? then : else`/
//! `cond ?: else`) — the recommended pick left at the end of the eighteenth
//! slice's own session, since none of the four were lowered by this crate at
//! all before it, not even for a plain `bool` operand. PHP's low-precedence
//! `and`/`or`/`xor` keyword operators were deliberately left out of this
//! slice's scope — ADR 0035 names only `&&`/`||`/`!`, not their keyword
//! siblings — and ADR 0045 later removed them from the language entirely, so
//! no lowering for them was ever needed.
//!
//! `&&`/`||` need genuine short-circuit control flow, not just a value
//! computation: [`lower::Lowering::lower_and`]/[`lower::Lowering::lower_or`]
//! lower to the same branch/merge-block shape
//! [`lower::Lowering::lower_if`]'s own module-doc section describes, except
//! the join point produces the expression's own [`ty::Ty::Bool`] value via a
//! fresh [`ir::InstKind::Phi`] instead of merging named locals. This is why a
//! new [`lower::Lowering::lower_expr_top`] entry point exists at all: unlike
//! every other expression form this crate lowers, `&&`/`||`/ternary can
//! redirect "the current block" mid-expression, so only positions that
//! already own a mutable `cur: &mut BlockId` — a local declaration's
//! initializer, `return`'s value, a plain reassignment's right-hand side
//! (including a property/array-index target), and any condition under test —
//! route through it instead of the plain, non-branching
//! [`lower::Lowering::lower_expr`]. Everywhere else — a call argument, an
//! array-literal element, a `.`-operand, a nested arithmetic/comparison
//! operand — still panics naming the gap if it contains one of these forms,
//! since those callers only ever own a fixed `cur: BlockId`. `!` never
//! branches on its own, but recurses through `lower_expr_top` for its own
//! operand so `!($a && $b)` composes at a top-level position; nested `!` (no
//! `&mut BlockId` available) still applies ADR 0035's truthy table plus a
//! negate via the plain [`lower::Lowering::lower_expr`], just without that
//! composition — see [`lower::Lowering::negate_truthy`], now shared by both
//! paths, and fixing a latent bug the seventeenth slice's own table
//! introduced: unary `!` previously passed its operand's own type straight
//! through to its result rather than always producing `Ty::Bool`, silently
//! correct only because the one existing fixture happened to negate an
//! already-`bool` local.
//!
//! The ternary/elvis operator ([`lower::Lowering::lower_ternary`]) is the
//! same branch/merge shape once more, joining a `then`/`else` value instead
//! of a named local. Elvis (`then` omitted) needs one exception to every
//! other truthy-tested position's "release a fresh, non-aliasing refcounted
//! operand once its truthy test is done" rule
//! ([`lower::Lowering::truthy_value`]): PHP evaluates a `?:` condition
//! exactly once, so the truthy path's own *value* is `cond` itself, not a
//! fresh conversion — releasing it as part of the truthy test would
//! use-after-free that reuse. [`lower::Lowering::truthy_convert`] (the bare
//! conversion, factored out of `truthy_value`) lets `lower_ternary` decide
//! that ownership question itself: release `cond` once `then` is given
//! (nothing left to reuse it for), or retain it once more when `then` is
//! omitted and `cond` is an aliasing read gaining a second independent owner.
//! The same question applies to every `then`/`else` branch, not just elvis's
//! reused `cond`: nothing else treats a ternary's own result as anything but
//! an ordinary fresh value (`is_aliasing_read` never lists `ExprKind::Ternary`
//! itself), so a branch whose own expression *is* an aliasing read needs its
//! own retain right there, converting a still-slot-owned reference into the
//! ternary's own independent one — caught by one of this slice's own tests
//! (`elvis_retains_an_aliased_refcounted_condition` and its sibling covering
//! the non-elvis case) rather than assumed correct by inspection alone. A
//! `then`/`else` pair that lowers to two different [`ty::Ty`] representations
//! still panics naming the case — the checker's own union of their static
//! types has no IR representation this crate can fold into yet, the same
//! open question [`ty::Ty::Mixed`]'s own doc comment names for why a union
//! isn't automatically folded into it.
//!
//! `while`'s own lowering needed one adjustment to host a branching condition
//! at all: the loop header's phis still physically live in the fixed
//! `header_block` created before condition lowering runs, but the loop's own
//! `Branch` terminator now seals onto whichever block condition lowering
//! actually ends in (`cond_end`, tracked separately) — unchanged from
//! `header_block` itself unless the condition contains one of these new
//! forms.
//!
//! One more pre-existing gap surfaced (and is fixed) as part of this slice:
//! `ExprKind::Paren` — a parenthesized `(expr)` — was never unwrapped
//! anywhere in this crate's expression lowering at all (only in type
//! position, `lower_decl_type`'s own `TypeKind::Paren` arm), even though
//! `mwl_types::expr::check_expr`'s own `Paren` arm has always treated it as
//! fully transparent. Invisible until now because no earlier slice's own
//! fixtures happened to need explicit parens; `!($a && $b)` does, since `!`
//! binds tighter than `&&`/`||` in the grammar. Both
//! [`lower::Lowering::lower_expr`] and [`lower::Lowering::lower_expr_top`]
//! now have their own transparent `Paren` arm, recursing back into
//! themselves respectively.
//!
//! The twentieth slice lowers `$a[] = expr;` — PHP's array append syntax on
//! its write side, the recommended pick left at the end of the nineteenth
//! slice's own session. A new [`ir::InstKind::ArrayAppend`] instruction
//! deliberately carries no key at all, unlike [`ir::InstKind::ArraySet`]:
//! PHP's real "next available integer key" rule tracks the highest `int` key
//! ever used as part of the array's own runtime state (surviving earlier
//! explicit-`int`-keyed inserts, removals and appends alike), which is
//! genuinely not something a lowering pass can compute from the source text
//! the way a literal's positional index or an explicit key already can — see
//! that variant's own doc comment for why its storage and increment are left
//! entirely to whatever `mwl-codegen`'s own array representation does with
//! them, the same "shape now, functional once a backend exists" deferral
//! [`ir::InstKind::Safepoint`] already gets. [`lower::Lowering::lower_reassignment`]'s
//! `Index`-target arm now matches on the subscript itself: `None` lowers the
//! base and the new value only (no [`lower::Lowering::lower_array_key`] call
//! at all, since there is no key to normalize) and emits `ArrayAppend`,
//! retaining the value first under the exact same
//! [`ty::Ty::is_refcounted`]/[`lower::is_aliasing_read`] policy
//! [`ir::InstKind::ArraySet`]'s own value already gets; `Some(index)` is the
//! unchanged pre-existing `ArraySet` path. `$a[]` as a *read* (no subscript,
//! no assignment) is unaffected by this slice and stays a permanent panic in
//! [`lower::Lowering::lower_expr`]'s `Index` arm — it has no PHP meaning at
//! all (PHP itself rejects it as "cannot use `[]` for reading"), a closed
//! question rather than a gap this crate is waiting to fill, even though
//! `mwl-syntax` parses it in any expression position and
//! `mwl_types::expr::check_expr`'s `Index` arm doesn't reject it as a read
//! either (see that test's own doc comment). With this, the array-access
//! shape started in the fifteenth slice is complete for every combination of
//! `int`/`uint`/`string` key, explicit `key =>`, and append syntax — what
//! remains of item 3 in the crate's own known-gap list is only a
//! `mixed`-erased base, still blocked on the runtime type-tag design
//! question the eighteenth slice's own paragraph names.
//!
//! The twenty-first slice lowers `break`/`continue` for a `while` loop —
//! the recommended pick left at the end of the twentieth slice's own
//! session, chosen over the `mixed` runtime type-tag design question and a
//! `...spread`/`&value` array-literal element as the more mechanical of the
//! three, and deliberately narrowed to `while` alone (`for`/`switch` still
//! don't lower at all) rather than attempting every loop-exit-shaped
//! construct in one slice. A new [`lower::LoopFrame`] — pushed onto a new
//! [`lower::Lowering::loop_stack`] field before [`lower::Lowering::lower_while`]
//! lowers its body, popped back off once it returns — carries the loop's
//! `header_block`/`after_block` plus one `(BlockId, Env)` pair per
//! `break`/`continue` actually lowered inside that body, at any nesting
//! depth reachable through `if`/nested `{}`; [`lower::Lowering::lower_break`]/
//! [`lower::Lowering::lower_continue`] each just read the top frame, record
//! their own edge into it, and seal the current block with a plain
//! [`ir::Terminator::Jump`] to `after_block`/`header_block` respectively — no
//! new `Terminator`/`InstKind` shape was needed, confirming the known-gaps
//! list's own prediction that `if`/`while` had already exercised everything
//! `for`/`switch`/`break`/`continue` would need. The actual design work was
//! folding the recorded edges back into the two join points `lower_while`
//! already built: a `continue` is exactly one more loop back edge, so its
//! edge joins the body's own fall-through exit (when the body reaches one)
//! before [`lower::Lowering::lower_while`]'s existing header-phi-patch loop
//! runs, widened from "patch with the one fall-through value" to "patch
//! with every back edge's value, fall-through included"; a `break` is a new
//! kind of join `lower_while` never had before at all — before this slice,
//! a loop's exit environment was always exactly `header_env`, since the
//! condition's own false edge was the only way out — so `after_block` now
//! runs the same [`lower::Lowering::merge_envs`] general-purpose join
//! [`lower::Lowering::lower_if`] already uses for its own merge block,
//! combining the false edge (carrying `header_env`) with every recorded
//! `break` edge; with no `break` at all this degenerates back to exactly
//! the prior single-clone behavior (`merge_envs`'s own `[(_, only)]` case),
//! so a break-free loop's lowering is unchanged bit-for-bit apart from the
//! extra clone. `break N`/`continue N` for `N > 1` and a `break`/`continue`
//! with a non-literal level both still panic naming the gap — see
//! [`lower::Lowering::loop_exit_level`]'s own doc comment for why unwinding
//! more than one loop needs `loop_stack` walked past its innermost frame,
//! left for whenever a fixture actually needs it — and so does either
//! keyword reached with an empty `loop_stack` (outside any loop at all):
//! `mwl_types` does not yet reject that itself (see the known gaps below),
//! so this crate's own stack-emptiness check is the one place it's still
//! caught, defensively, rather than building a `Jump` to a block that was
//! never created.
//!
//! `echo` and the **script body as a function** are the latest slice, and the
//! two that put the whole front end on `.claude/loop-goal.md`'s acceptance
//! program. [`lower::lower_script`] synthesizes one frame from a file's own
//! top-level statements — skipping declarations, whose methods
//! [`lower::lower_method`] lowers separately, and descending into a
//! `namespace X { ... }` block's body, since a namespace scopes names rather
//! than storage. Its return representation is [`ty::Ty::Mixed`], because
//! ADR 0021 types what a `require`d file hands back that way. `echo` itself
//! needed no new instruction: [`lower::Lowering::lower_echo`] reuses
//! `.` concatenation's own [`lower::Lowering::concat_operand`] to get each
//! operand to [`ty::Ty::Str`], then emits one [`ir::Helper::EchoStr`]
//! [`ir::InstKind::HelperCall`] per operand — the first [`ir::Helper`]
//! invoked for an effect rather than a conversion, and so the first
//! `HelperCall` emitted with no `result` at all.
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
//! - **A suspension point will be a *terminator*, not an instruction — a
//!   representation decision taken now, with the transform itself left to
//!   M4.** [ADR 0053](../../../docs/adr/0053-iteration-and-generators.md) § 4
//!   lowers a generator to an explicit state machine rather than onto the
//!   coroutine substrate, and its *Consequences* make "M2's IR must model a
//!   suspension point inside a loop body" this crate's obligation — the
//!   transform may land later, foreclosing it is the expensive mistake, the
//!   same argument [`ir::InstKind::StmtMarker`]'s probe ids already rest on.
//!   Nothing here lowers `yield` today. What this bullet fixes is the shape
//!   it will take, so that widening this crate in the meantime does not
//!   quietly rule it out. Four properties carry it; each already holds, and
//!   each is now an invariant rather than an accident:
//!   1. **A `yield` ends its block.** It becomes a [`ir::Terminator`]
//!      variant carrying the yielded value and naming the block resumption
//!      re-enters, not an [`ir::InstKind`] sitting mid-block. A generator
//!      body is then already split at exactly its suspension points when the
//!      transform sees it, so the pass never has to re-split a block and
//!      re-run the phi bookkeeping [`lower::Lowering::merge_envs`] and
//!      `lower::Lowering::lower_while`'s seed-then-patch dance do at lowering
//!      time. Inside a loop body this is the whole difficulty: the resumption
//!      block is a block *within* the loop that gains a second predecessor
//!      which is not the loop header, and that is only expressible if the
//!      split is a real CFG edge to begin with.
//!   2. **The resumption dispatch is the N-way terminator `switch` already
//!      needs.** Resuming means entering at a state tag, i.e. a multi-way
//!      jump at function entry — structurally identical to `switch`'s own
//!      dispatch (the known gaps below still list `switch` as unlowered).
//!      Whoever lands `switch` should add *one* N-way [`ir::Terminator`]
//!      general enough for both, not a two-way `Branch` chain that a later
//!      state machine would have to work around. [`ir::Terminator::Branch`]
//!      carries an [`ids::EdgeId`] per outgoing edge for ADR 0018's branch
//!      probe; an N-way terminator carries one per arm on the same grounds.
//!   3. **A local live across a suspension becomes an object field, and both
//!      instructions for that already exist.** ADR 0053 § 4 stores every such
//!      local in the state object; [`ir::InstKind::FieldSet`] before the
//!      suspend and [`ir::InstKind::FieldGet`] at the resumption block are
//!      exactly that, with the retain/release policy those two already carry.
//!      No new instruction is needed — what the pass needs from this crate is
//!      the ability to *compute* which locals are live, which is why
//!      [`ir::InstKind::Phi`] keeps its `(predecessor, value)` pairs
//!      explicitly at the block head rather than in a side table. Keep it
//!      that way.
//!   4. **Nothing may assume the CFG is reducible.** Dispatching straight
//!      into a block inside a loop body gives that loop a second entry, so a
//!      post-transform generator function is irreducible in the general case.
//!      Lowering itself never produces such a CFG — it is structured, and
//!      [`lower::LoopFrame`] only ever sees single-entry loops — so this
//!      costs nothing today; it is a constraint on any *later* pass or
//!      backend assumption added here. Cranelift accepts an irreducible CFG,
//!      so M3's backend does not need to care either.
//!
//! # Known gaps (all deliberate, all deferred to a later widening session)
//!
//! - **Inline HTML at file scope is not lowered.** `?>text<?mwl` reaches
//!   [`lower::lower_script`] as an [`mwl_syntax::ast::StmtKind::InlineHtml`]
//!   statement, which `lower::Lowering::lower_stmt` panics on like any other
//!   unsupported shape. PHP writes such a run to output verbatim, so the
//!   lowering is the same [`ir::Helper::EchoStr`] call `echo` already emits,
//!   over a [`ir::InstKind::ConstStr`] of the raw span — left out here only
//!   because `.claude/loop-goal.md`'s acceptance program has none, and
//!   `mwl_types` does not check one either (its own `check_stmt` treats
//!   `InlineHtml` as a no-op), so landing it would widen two crates at once.
//! - **`try`/`catch`/`throw` lower; `finally` does not.** See
//!   [`ir::Inst::on_error`] for the error edge every call-shaped instruction
//!   now carries, [`lower::Lowering::landing_block`] for what each landing
//!   block releases and why the propagate and catch exits release different
//!   things, and [`lower::Lowering::lower_try`] for the one-clause,
//!   global-`Throwable`-only restriction and the `catch` variable's
//!   clause-scoped lifetime. Out of scope, each panicking rather than
//!   miscompiling: `finally`, a second `catch` clause, a user exception class
//!   (all three now want only the exception *surface* decision
//!   `.claude/loop-goal.md` records — [`ir::InstKind::InstanceOf`], the type
//!   test each needs, already lowers), `throw` in expression position, and
//!   `Throwable::getTrace()`, which
//!   returns `array<…>` and is `.claude/loop-goal.md`'s explicit M4
//!   carry-over. One narrower gap sits inside what *does* lower: the landing
//!   sweep covers the frame's locals, not a temporary still in flight inside
//!   the expression that threw — see [`lower::Lowering::landing_block`]'s own
//!   doc comment.
//! - `for`/`switch`/`match` are still unsupported: lowering panics
//!   naming the statement. [`ir::Terminator::Branch`] and [`ids::EdgeId`]
//!   are both already exercised by `if`/`while`, so widening to the rest is
//!   expected to reuse the same shapes rather than add new ones — see
//!   [`lower`]'s module docs. `foreach` *does* lower, over an `array<T>`
//!   subject only — see [`lower::Lowering::lower_foreach`], which owns the
//!   whole policy. Out of scope there, each panicking rather than
//!   miscompiling: an ADR 0053 `Iterable`/`Iterator` subject, a `&$v`
//!   by-reference value binding, and a key binding declared as anything but
//!   `string`.
//! - **`break`/`continue` lower for a `while` or `foreach` loop, level 1
//!   only.** See the twenty-first-slice paragraph above for
//!   [`lower::LoopFrame`]'s shape and how a `continue`'s edge folds into the
//!   header's own phi-patch loop while a `break`'s folds into a new
//!   [`lower::Lowering::merge_envs`] call at the after-block; a `foreach`
//!   adds one thing to both, [`lower::LoopFrame::iteration_owned`]'s
//!   per-iteration release. What's still out of scope: `break N`/`continue N`
//!   for any `N > 1` (a multi-level exit — panics naming it), a
//!   non-literal level expression (also panics), and either keyword inside
//!   a `for`/`switch` body, since neither of those statements lowers at all
//!   yet. A `break`/`continue` with no enclosing loop at all also still
//!   reaches this crate unrejected — `mwl_types` does not itself check loop
//!   nesting (see its own known gaps) — so [`lower::Lowering::loop_stack`]
//!   being empty is this crate's own defensive check, not something a
//!   well-formed input program could trigger; picking up that checker-side
//!   gap would let this crate's own panic go from "defensive" to
//!   "unreachable," the same way `mwl_types` growing ADR 0007 § 4's
//!   literal-magnitude check already turned this crate's own out-of-range
//!   integer-literal panic from a live gap into an unreachable-input
//!   invariant (see the "Integer literal magnitude range-checking" bullet
//!   below).
//! - **An `if`/`while` condition converts through ADR 0035's truthy table for
//!   a `bool`, scalar, or `Ty::Array` operand; a `Ty::Object` operand (a
//!   class instance or an enum case) is always truthy and needs no helper at
//!   all (ADR 0035 § 4).** See the seventeenth-slice paragraph above for the
//!   `Helper` variants this added and the release policy for a fresh
//!   `Ty::Array` condition. What's still out of scope: `null` (no
//!   nullable-type IR representation exists yet to convert *from*, so
//!   lowering panics naming this and nothing in scope today can actually
//!   reach that arm) and `mixed`/a union — [`ty::Ty::Mixed`] (the eighteenth
//!   slice, above) does now give `mixed` an IR representation, so a
//!   `mixed`-typed condition *can* reach `lower_truthy_cond` as of this
//!   session, but converting one through the table still needs the runtime
//!   type-tag representation named in the runtime-helper-calls known gap
//!   below, so it still panics — now naming a live gap rather than a
//!   theoretical one. `&&`/`||`/`!` and the ternary/elvis condition — ADR
//!   0035's other four truthy positions — now lower too, as of the
//!   nineteenth slice; see the next bullet.
//! - **`&&`/`||`/`!` and the ternary/elvis operator all lower, at any
//!   position that already owns a mutable `cur: &mut BlockId`.** See the
//!   nineteenth-slice paragraph above for the full shape
//!   ([`lower::Lowering::lower_and`]/[`lower::Lowering::lower_or`]/
//!   [`lower::Lowering::lower_not`]/[`lower::Lowering::lower_ternary`], all
//!   reached through the new [`lower::Lowering::lower_expr_top`] entry
//!   point). What's still out of scope: any of the four nested inside a
//!   position that only owns a fixed `cur: BlockId` — a call argument, an
//!   array-literal element, a `.`-operand, a nested arithmetic/comparison
//!   operand — still panics naming the gap, since [`lower::Lowering::
//!   lower_expr`] itself was deliberately not widened to redirect the
//!   current block. A ternary whose `then`/`else` branches lower to two
//!   different [`ty::Ty`] representations also still panics — folding the
//!   checker's own union into this crate's flatter `Ty` lattice is its own
//!   decision, the same open question `Ty::Mixed`'s own doc comment names.
//!   PHP's low-precedence `and`/`or`/`xor` keyword operators need no lowering
//!   at all — ADR 0045 removed them from the language, so `mwl-syntax` never
//!   produces the AST shape that would have reached this crate.
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
//!   that erased to `mixed` (`mwl_types::expr::check_expr`'s own `Index` arm
//!   records no entry in that case, only the checked-type fallback to
//!   `mixed` itself) still has no such entry, so lowering panics naming it,
//!   the same split `ExprInfo::Property` already draws for a shape/plain-
//!   `object` receiver. [`ty::Ty::Mixed`] (the eighteenth slice, above) gives
//!   this case somewhere to fall back *to* — a representation now exists for
//!   what such a read/write would produce/accept — but wiring that fallback
//!   in is deliberately left for a future slice rather than folded into this
//!   session's narrower round-trip-only scope; the panic and its message are
//!   unchanged. Neither instruction models what happens when the key
//!   is actually absent at runtime (PHP's own warning-and-`null` read,
//!   autovivification on write) — that question is deferred wholesale, the
//!   same way every other checked-throw is (no `try`/`throw` lowering exists
//!   yet), not something this slice had to weigh a design against (see the
//!   fifteenth-slice paragraph above for why `mwl_types` itself has no
//!   compile-time "is this key present" concept to consult in the first
//!   place). `$a[] = expr;` (append syntax, `index` is `None`) lowers too, as
//!   of the twentieth slice — see that paragraph above for
//!   [`ir::InstKind::ArrayAppend`]'s no-key shape. `$a[]` as a *read* has no
//!   PHP meaning at all and stays a permanent panic, not a gap. A `float`/`bool`/
//!   `null` subscript is rejected by `mwl_types::expr::check_array_key_type`
//!   at check time as of the sixteenth slice, so `lower::Lowering::
//!   lower_array_key`'s `other` panic arm is unreachable for it now, not a
//!   live gap. An array literal's explicit `key =>` element is landed too
//!   (same slice) — see [`ir::InstKind::ArrayNew`]'s own doc comment for the
//!   `ArrayNew`-then-`ArraySet*` shape it takes and the one PHP behavior it
//!   deliberately doesn't reproduce. A `...spread` element or a `&value`
//!   element is still unsupported either way — lowering panics naming
//!   whichever is used, since neither has a merge/reference representation
//!   in this crate yet.
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
//! - **No virtual dispatch, except where the class is a run-time value** —
//!   [`ir::InstKind::Call`]'s `target` is still always the statically resolved
//!   declaring class from `mwl_types::expr_table::ResolvedCall`, for a static
//!   call, `new`'s constructor and an instance method call alike, so an
//!   overridden method reached through a base-typed local still calls the
//!   base's. What *is* dispatched at run time are the two shapes where no
//!   static answer exists at all: `static::method(...)`/`new static(...)`,
//!   and a call resolving to a declaration with no *body* (an `abstract`
//!   method, or the interface method an ADR 0043 § 2 default body calls back
//!   into). Both lower to
//!   [`ir::InstKind::CallVirtual`]/[`ir::InstKind::NewDynamic`], which look
//!   the method up on the class carried in [`ty::Ty::ClassDesc`] — the
//!   receiver's own, or the late-static-binding one. That is not general
//!   virtual dispatch — but it builds the per-class method table
//!   ([`ir::Class::methods`]) a real vtable would index, so closing this gap
//!   is now a question of picking a compile-time slot index over a name, not
//!   of building a table. `mwl_runtime::object`'s module docs own that
//!   decision.
//! - **No variadic, named, or spread call argument** —
//!   `Lowering::lower_call_args` (in [`lower`]) panics naming any of the
//!   three; `mwl_types` itself doesn't fully positionally type-check a
//!   named/spread argument against a signature yet either (see its own known
//!   gaps), so there is no resolved per-argument type to lower against even
//!   if this crate wanted to try.
//! - Safepoints are reserved, not functional. [`ir::InstKind::Safepoint`] is
//!   emitted at function entry and at every `while` back edge — the body's
//!   own fall-through exit and every `continue` alike, as of the
//!   twenty-first slice above — but it is inert — no codegen exists yet to
//!   lower it to an actual CPU-limit/cancellation/cycle-collector check, and
//!   no guard test needs it functional before M3's backend does. `for`
//!   loops will need the same back-edge marker once they land.
//! - **Runtime-helper calls (the milestone's third named ingredient) now
//!   cover `.`'s scalar-to-`string` conversion and ADR 0035's truthy
//!   conversion for a scalar or `Ty::Array` `if`/`while` condition.**
//!   [`ir::InstKind::HelperCall`]/[`ir::Helper`] are used by both
//!   [`lower::Lowering::concat_operand`] (the twelfth slice) and
//!   [`lower::Lowering::lower_truthy_cond`] (the seventeenth slice, above).
//!   Ordinary arithmetic still lowers directly to [`ir::InstKind::BinOp`]/
//!   [`ir::InstKind::UnOp`] with no helper fallback, since dispatching on a
//!   `mixed`/union operand's *actual* runtime type needs a type-tag
//!   representation this crate still doesn't have — [`ty::Ty::Mixed`] (the
//!   eighteenth slice, above) gives `mixed` an opaque representation to
//!   *erase into*, deliberately not one to *dispatch on*, so this gap (and
//!   the `null`/`mixed` half of ADR 0035's own truthy conversion left open
//!   above, and item 3's mixed-erased-array-base gap below) are all still
//!   open, all still expected to add new [`ir::Helper`] variants once that
//!   tag representation lands, not a second call-shaped instruction.
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
//!   lowered at all.~~ ~~Heredoc/nowdoc is out of scope pending a
//!   flexible-heredoc indentation-stripping story.~~ **Done, for every
//!   shape.** A numeric escape (`\xHH` hex, `\NNN` octal, `\u{...}` Unicode)
//!   cooks to the byte/codepoint it names, and any `ExprKind::Interpolated`
//!   — double-quoted or heredoc-sourced — lowers to the same
//!   [`ir::InstKind::Concat`] chain a written-out `.` expression already
//!   produces — see `lower::Lowering::lower_interpolated_parts`'s own doc
//!   comment for the one refcount subtlety a single-part `"$x"` (no
//!   surrounding literal text, so no `Concat` ever runs) forces into the
//!   open. A heredoc/nowdoc's own closing-marker indentation is stripped
//!   from every body line first (PHP 7.3's "flexible heredoc" rule,
//!   `mwl_types::string_lit::heredoc_shape`/`dedent_heredoc_run`) — a nowdoc
//!   then applies no escapes at all, exactly like a single-quoted literal
//!   minus even `\\`/`\'`, while a heredoc runs the identical escape grammar
//!   a double-quoted literal does. All of this is shared with the checker
//!   through `mwl_types::string_lit`, which is what lets
//!   `mwl_types::expr::infer`'s own `ExprKind::Str`/`ExprKind::Interpolated`
//!   arms diagnose exactly the cooking this crate performs (including
//!   `E_HEREDOC_MIXED_INDENT`/`E_HEREDOC_INSUFFICIENT_INDENT` for a
//!   malformed marker/body line), rather than the two crates disagreeing on
//!   what a given literal means.
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
