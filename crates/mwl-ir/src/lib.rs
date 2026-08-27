//! MWL's CFG/SSA IR: the one representation between the checked AST and
//! `mwl-codegen`, carrying explicit safepoints, refcount operations and
//! runtime-helper calls, with a stable per-statement and per-edge id
//! ([ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)).
//!
//! [`lower`] is the whole front-to-IR pass, split across `lower/` by area;
//! [`ir`] is the data; [`ty`] is this crate's own representation-level type
//! lattice; [`print`] renders a program for the snapshot tests.
//!
//! # What lowers today
//!
//! Whole-program: [`lower::lower_file`] walks a file, [`lower::lower_method`]
//! one method, [`lower::lower_property_hook`] one ADR 0014 accessor, and
//! [`lower::lower_script`] a file's own top-level statements as one synthesized
//! frame of ordinary locals with no receiver (ADR 0008 § 2), returning
//! [`ty::Ty::Tagged`] because that is what ADR 0021 types a `require`'s result.
//!
//! - **Statements** — typed and `var` local declarations (ADR 0037),
//!   reassignment, `return`, nested blocks, `echo`, `unset`, `if`, `while`,
//!   `do`/`while`, `for`, `switch`,
//!   `foreach` over all three of ADR 0053 § 3's subjects, `break`/`continue`
//!   at any level ([`lower::Lowering::lower_break`] counts every enclosing
//!   loop and `switch`, PHP's own rule), `try`/`catch`, `throw`.
//! - **Expressions** — arithmetic and comparison, `.` concatenation and string
//!   interpolation, `new`, static/instance/`Core` calls, property and
//!   array-element read and write, array literals including an explicit
//!   `key =>` and `$a[] =` append, `&&`/`||`/`!` and the ternary/elvis
//!   operator, ADR 0035's truthy conversion, ADR 0031 closure literals,
//!   `instanceof`, `??`, the literal `null`, ADR 0007 § 2's scalar conversion
//!   rows — free, total and checked alike — and ADR 0066's non-throwing
//!   `as ?T` over the checked numeric targets and over its § 3 parse roster.
//! - **Types** — `int`/`uint`/`float`/`bool`/`decimal` scalars, `string`,
//!   `bytes`,
//!   `array<T>` (element type erased — see [`ty::Ty`]), `object` (a class or
//!   enum, likewise erased), `mixed`, `null`, `?T` and any other union (all
//!   three tagged — see [`ty::Ty::Tagged`]), and [`ty::Ty::Ref`] for a `&$x`
//!   parameter. `string`, `bytes` and `array<T>` are refcounted and cross a
//!   local, call-argument, return and property boundary alike.
//! - **Generators** — ADR 0053 § 4's state-machine transform, in
//!   [`lower::lower_generator`]: one declaration becomes a factory, an
//!   `advance()`, a `current()` and a synthesized state class.
//!
//! # Design choices worth knowing before widening this
//!
//! - **SSA, not a plain CFG.** `if`/`while`/`for` are each a hand-rolled
//!   merge, not a dominance-based phi-placement algorithm — sufficient for any
//!   structured nesting, since none produces a join of another shape. There
//!   are two building blocks, and every construct added so far has reused
//!   them: `merge_envs` for a set of incoming edges known up front (`if`,
//!   `for`'s step block, a `switch` case body's label and fall-through edges,
//!   a `match`'s arm phi), and `lower_while`'s seed-then-patch phi dance for a
//!   join whose back edge is not known until its body is lowered.
//! - **IR types are representation-level, not the checker's types.** See
//!   [`ty`]'s own module docs for why [`ty::Ty`] is a small, flat lattice
//!   rather than a reuse of `mwl_types::ty::Ty`.
//! - **This crate depends on `mwl-types` only for its typed-expression table,
//!   never on `mwl-hir`'s class graph directly.** Every *declared* type is read
//!   straight off the AST (ADR 0007 § 1 requires it spelled out, so no name
//!   resolution is needed). What genuinely is absent from the AST is a call's
//!   *resolved target* — which class declares the callee, and its parameter
//!   and return types — so `mwl_types::expr_table::ExprTypeTable` publishes
//!   that, keyed by source span, and lowering reads it back. The alternative,
//!   re-running class-hierarchy resolution here, would have duplicated
//!   `mwl-types`. Lowering **trusts** that its input already passed
//!   `mwl_types::check_program` with the same tables, and panics naming an
//!   unsupported shape rather than diagnosing.
//! - **A receiver is an implicit first parameter, not a special case.** Every
//!   lowered method carries `$this` at [`ir::Function::params`] index 0,
//!   whether or not the body reads it — mirroring `mwl_types`' own
//!   `check_method`. A separate `Option<ValueId>` would have needed `Env`'s
//!   `$this` lookup to take a different path than every other local, for a
//!   value that behaves exactly like an ordinary parameter.
//! - **Refcount insertion is naive and syntactic, not a liveness analysis.**
//!   Correctness first; elision is a named later optimizer pass. Reading a
//!   value out of storage another binding still owns — [`lower::is_aliasing_read`]
//!   names the three shapes, and `Lowering::aliasing_read` is the judgment
//!   every decision actually goes through, because two of them are not
//!   syntactic: an ADR 0014 `get` hook is a call, and a field or element read
//!   whose *base* is a temporary owns its own result — and copying it into a second
//!   durable slot needs a retain; a freshly constructed value needs none,
//!   since it already has one natural owner. A slot's previous value is released when overwritten,
//!   every live slot is released at frame exit, and a binding **control flow
//!   drops** is released at the point it disappears — a local declared inside
//!   a loop body or inside one `if` branch. That last half was missed until
//!   `examples/report.mwl`'s valgrind leg found it, which is why AGENTS.md
//!   says to run `tools/leak-check.sh` against a fixture exercising any new
//!   refcount edge.
//! - **A closed, engine-owned runtime helper gets [`ir::InstKind::HelperCall`]
//!   with a `#[non_exhaustive]` [`ir::Helper`] tag**, not [`ir::InstKind::Call`]
//!   with a synthetic target. A helper has neither a class-graph origin nor a
//!   receiver, so folding it into `Call` would blur the line virtual dispatch
//!   depends on staying sharp; and the helper set is closed and known to this
//!   crate, so an enum buys exhaustiveness a string name would lose.
//! - **Nothing may assume the CFG is reducible.** A generator's `advance()`
//!   dispatches straight into a block inside a loop body, giving that loop a
//!   second entry. Cranelift accepts one, so M3's backend does not care — this
//!   is a constraint on any later pass added here.
//! - **Ids are stable, not global.** See [`ids`]'s own module docs.
//! - **An array key is a `string`, and an `int` subscript no longer spells
//!   it.** [`ir::InstKind::ArrayGet`]'s and [`ir::InstKind::ArraySet`]'s `key`
//!   operand carries either [`ty::Ty::Str`] or [`ty::Ty::Int`], and codegen
//!   picks the runtime primitive off the operand's own representation — there
//!   is no second instruction, no key-kind field and no new [`ir::Helper`].
//!   The alternative shapes were each worse for a reason worth recording: a
//!   separate `ArrayGetIndex` doubles every array instruction and every match
//!   arm over them for one operand's type, and a `key_is_int: bool` states
//!   twice what `ty::Ty` already states once. The consequence a widening
//!   contributor must hold: **a key operand's `Ty` is now load-bearing**, so
//!   the refcount decision at each of `lower_array_key`'s call sites turns on
//!   it — an `int` key owns nothing to retain or release. ADR 0007 § 5 is
//!   untouched by any of this; every key still *is* a `string`, `"08"` is
//!   still distinct from `"8"`, and `$a[8]` is still `$a["8"]`. What moved is
//!   only where the decimal is produced, which is `mwl_runtime::array`'s
//!   packed form deciding it never has to be. Two subscripts still render:
//!   a `uint`, because the runtime's index ABI is an `i64` and a `uint` above
//!   `i64::MAX` has no `i64` spelling naming the same key, and any key
//!   reaching [`ir::InstKind::ArrayUnset`], which has no index-shaped
//!   primitive beside it. `lower::Lowering::lower_array_key` and
//!   `lower_rendered_array_key` own both exceptions.
//! - **A nested element write always separates the inner row, and that is
//!   the price of having no branch.** `$grid[0][1] = v` flattens to its root
//!   plus one key per level and descends with [`ir::Helper::ArrayRowForWrite`]
//!   — a row that arrives owning a reference of its own, because the
//!   [`ir::InstKind::ArraySet`] climbing back out consumes one. So every level
//!   below the root reaches its write at a refcount of at least two and ADR
//!   0007 § 5's copy-on-write separates it, even when nothing else was ever
//!   going to observe the old row. **The cost is O(inner) per write**, paid
//!   once per level, and it is correct rather than merely acceptable: a copy
//!   is what the value semantics promise, and only an optimizer can tell that
//!   this particular one is unobservable. The follow-up that removes it is a
//!   *write-through* descent — one runtime entry point that takes the whole
//!   key chain, walks it holding the parent's borrow rather than a reference,
//!   and separates only where the count genuinely says it must — which is a
//!   `mwl-runtime` ABI question (M9 freezes that surface) and not a lowering
//!   one. Nothing depends on the copy happening, so it can be taken away
//!   without changing an observable.
//!
//! # Known gaps
//!
//! Each panics naming itself rather than miscompiling.
//!
//! 1. **`do`/`while` does not lower** — [`lower::Lowering::lower_while`] with
//!    the branch moved below the body, and nothing new to build. Every other
//!    control-flow statement does: `for`
//!    ([`lower::Lowering::lower_for`]), whose one restriction is a condition
//!    clause of more than one comma-separated expression, and `switch`
//!    ([`lower::Lowering::lower_switch`]) and `match`
//!    ([`lower::Lowering::lower_match`]), whose one restriction is a label
//!    whose representation differs from the subject's. Both of the latter
//!    lower to an equality chain of [`ir::Terminator::Branch`]es rather than
//!    to [`ir::Terminator::Switch`] — that terminator selects on an integer,
//!    while a label is any expression of the subject's type; `lower_switch`'s
//!    own doc comment owns why one shape for every subject type beats two.
//! 2. **A `finally` does not run when a `catch` clause's own body throws.**
//!    [`lower::Lowering::lower_try`] owns that one — every other exit from a
//!    protected region runs its `finally`, including a `return`, a `break` and
//!    a `continue`. A second, narrower gap sits inside what does lower: a
//!    landing block now sweeps the frame's owned-temporaries stack as well as
//!    its locals, so a call's arguments and receiver, and the operands of `.`,
//!    an interpolation and an `echo`, are released on both edges. A producer
//!    that still releases its fresh value inline — a normalized subscript key,
//!    a `match` subject — is not on that stack yet and leaks.
//!    [`lower::Lowering::landing_block`] states the boundary and
//!    `lower::Lowering`'s own field doc states the one hole shaped differently
//!    (an argument being *transferred* when a later one throws).
//! 3. **A tagged value can be built, carried and narrowed, but not yet
//!    dispatched on.** [`ty::Ty::Tagged`] is the one representation `mixed`,
//!    `?T` and every other union erase to, and its own doc comment owns the
//!    decision and what it spends. What lowers today: a `?T` local, parameter,
//!    property, return value and call argument; the literal `null`;
//!    [`ir::InstKind::Tag`]/[`ir::InstKind::Untag`] at every boundary carrying
//!    a declared type ([`lower::Lowering::coerce`]); and `??`
//!    ([`lower::Lowering::lower_coalesce`]), whose non-`null` arm narrows
//!    against the type `mwl_types::expr_table::ExprInfo::Coalesce` records.
//!    ADR 0066's `as ?T` ([`lower::Lowering::convert_or_null`]) is the first
//!    thing here that *reads* a tag instead — its helper dispatches on the
//!    operand's, which is what a `mixed` source costs, and is the shape the
//!    rest of this gap closes in. `.` and `echo` read one the same way
//!    ([`ir::Helper::TaggedToString`]). `?->` reads one only to *test* it
//!    ([`lower::Lowering::open_nullsafe`]), as does `== null`/`!= null`,
//!    which is one [`ir::InstKind::IsNull`] rather than a comparison against
//!    a `null` constant — and a *read* of the local that test narrowed is one
//!    unchecked [`ir::InstKind::Untag`] at the read itself
//!    ([`lower::Lowering::untag_narrowed`], off
//!    `mwl_types::expr_table::ExprInfo::NarrowedRead`; `mwl_types::locals`
//!    owns the proof). Narrowing there rather than at each consumer is what
//!    lets a subscript base, a `foreach` subject, an array-write root and an
//!    argument all see the narrow representation with no site to forget;
//!    [`lower::Lowering::untag_receiver`] is the same move for the one
//!    consumer that predates it.
//!    What does not: reading a tagged value *without* a checker-proven
//!    narrowing — arithmetic on a `mixed`, ADR 0035's truthy table, an array
//!    access through a tagged base. Each panics naming itself, and closing
//!    them adds [`ir::Helper`] variants dispatching on the tag, not a second
//!    representation.
//! 4. **One conversion row is missing, and every ADR 0066 § 3 refusal is.**
//!    ADR 0007 § 2's free, total and checked scalar rows all lower, in both
//!    the throwing form ([`lower::Lowering::convert`]) and ADR 0066's
//!    non-throwing `as ?T` ([`lower::Lowering::convert_or_null`]). The row
//!    still absent is ADR 0010 § 5's integer *into* an enum, in either form:
//!    it throws on a value no case names, which needs the declaration's case
//!    set carried to the check, and nothing here expresses one. `EnumName` ↔
//!    `string` is not a gap — ADR 0010 § 5 leaves it out of the language.
//!    ADR 0066 § 3 makes `as ?T` a **compile error** where the conversion
//!    cannot fail (`$i as ?string`) or does not exist at all (`$arr as ?int`);
//!    `mwl_types` refuses neither yet, so both reach lowering and panic naming
//!    that ADR instead of being diagnosed. The **class-target** refusal is the
//!    one of § 3's that does exist —
//!    `mwl_diagnostics::code::E_CLASS_CONVERSION_TARGET`, and it is absolute,
//!    so no class reaches this crate through `as` at all. It used to carry a
//!    two-class exception, the *parse roster*, lowered here to one
//!    [`ir::InstKind::CoreCall`] on a symbol the expression table had to carry
//!    because every `?T` erases to [`ty::Ty::Tagged`]. § 3 withdrew it, and
//!    `Core\Uri::tryParse` is an ordinary member call now.
//! 5. **A ternary — or a `match` — whose branches lower to two different
//!    [`ty::Ty`] representations panics.** Neither has a recorded result type
//!    to widen its arms to, which is the one thing
//!    `mwl_types::expr_table::ExprInfo::Coalesce` supplies for `??` — so
//!    closing it is that same recording, plus [`lower::Lowering::coerce`] on
//!    each arm. *Where* a short-circuit may
//!    appear is no longer a restriction: [`lower::Lowering::lower_expr`] owns
//!    a `&mut BlockId` and lowers its own sub-expressions through itself, so
//!    `&&`/`||`/`!`/ternary/`??` compose inside a call argument, an array
//!    element, a `.` operand or an `echo` operand alike.
//! 6. **Array access is compile-time-known-target-only, and `mwl_types` now
//!    says so rather than leaving it here; property access is not
//!    compile-time-known-target-only any more.** A subscript whose base
//!    declares no element type — a `mixed`, a scalar, a `?array<T>` no test
//!    narrowed — is `E0482` where it is written, and `$a[]` anywhere but an
//!    assignment target is `E0481`, so both of
//!    [`lower::Lowering::lower_index`]'s panics and the assignment arm's
//!    matching one are invariant checks no source file reaches.
//!    Every ADR 0036 § 4 receiver lowers: a shape naming one
//!    of its own fields, a shape asked for a name it does not list, a
//!    plain `object`, and a `mixed`. All four are one
//!    [`ir::InstKind::SlotGet`] — § 4's
//!    **name-keyed** fetch, which is therefore right through a widened view
//!    too — or one [`ir::InstKind::SlotSet`], which additionally checks the
//!    incoming value's tag against what the concrete class declares the field
//!    to hold; `mwl_runtime::object`'s docs § *What a shape write checks*
//!    state what that granularity misses. The erased ones differ only in what
//!    the checker could record: no slot to hint (so `0`, which the runtime's
//!    by-name search corrects) and no type (so [`ir::Ty::Tagged`]), which
//!    makes § 4's missing-name throw reachable rather than theoretical. A
//!    `mixed` receiver adds the one thing the other three cannot: an
//!    unproven *tag*, so `lower::expr`'s `ReceiverProof::Erased` emits no
//!    [`ir::InstKind::Untag`] and the whole tagged value reaches the runtime,
//!    which throws in PHP's own wording for a receiver that is not an object
//!    — every *statically* non-object receiver having been `E0495` at the
//!    checker (ADR 0007 § 7 row 13). An
//!    anonymous `{a: 1}` literal
//!    constructs, as an instance of the class [`lower::shape_class_label`]
//!    names. Nullsafe `?->` *reads* — a call and a property alike, over
//!    the one guard [`lower::Lowering::open_nullsafe`] opens — but a nullsafe
//!    assignment target (`$a?->b = v`) is `E0479` in `mwl_types`, which is
//!    what PHP refuses too. An array-element write through a hooked property
//!    is `E0478` and one through an erased property `E0480`, both for the
//!    reason those codes' own rows state. A *nested* write — `$grid[0][1] =
//!    v`, whose base is itself an index expression — lowers: it flattens to
//!    its root holder plus one key per level, descends, and writes every
//!    level back with the outermost last, auto-vivifying an absent row the
//!    way PHP does. Neither
//!    [`ir::InstKind::ArrayGet`] throws on an absent key (ADR 0007 § 7 row
//!    11), which is the *read* side's answer to the same question this
//!    vivifying descent asks; [`ir::InstKind::ArraySet`] models no absent key
//!    at all, because a write is what makes one present. A **static** property
//!    is narrower still: it reads, but
//!    [`lower::Lowering`]'s assignment arm has no target for one, so
//!    `C::$p = v` panics.
//! 7. **Virtual dispatch resolves by name, not by slot.** An instance call
//!    lowers to [`ir::InstKind::Call`] — bound to the statically resolved
//!    label — only when nothing in the program overrides that declaration;
//!    `mwl_types` answers that whole-program question once, per call, as
//!    `mwl_types::expr_table::ResolvedCall::overridden`. When something does,
//!    and for the two shapes with no static answer at all (`static::`/`new
//!    static`, and a call resolving to a body-less declaration), the call
//!    goes through [`ir::InstKind::CallVirtual`]/[`ir::InstKind::NewDynamic`]
//!    over [`ty::Ty::ClassDesc`], which looks the name up in the per-class
//!    method table [`ir::Class::methods`] carries. A real vtable would index
//!    that table by slot instead, which is the remaining half — a lookup
//!    cost, not a correctness gap.
//! 8. **No named or spread call argument.** The array-literal half is closed:
//!    a `...spread` element is one [`ir::InstKind::ArraySpread`], and a
//!    `&value` element is not a shape the language has at all — `mwl_types`
//!    refuses one as `E0483`, ADR 0031 § 2 and ADR 0023 leaving an aliasing
//!    element no owner. What is left is the *call* argument: `mwl_types` does
//!    not fully positionally type-check a named or spread one, so there is no
//!    resolved per-argument type to lower against. A **variadic** signature is no
//!    longer among them: `lower::Lowering::lower_variadic_tail` collects every
//!    argument from that parameter's position into one fresh array, which is
//!    the single value the parameter receives — see
//!    `mwl_stdlib::registry::CoreTy::Variadic` for why that shape rather than a
//!    second, count-carrying calling convention.
//! 9. **A closure literal lowers, and so does `$f(...)`; what nothing checks
//!    is the argument *types*.** The call is one [`ir::Helper::CallClosure`]
//!    — `mwl_runtime::call_closure`, the same entry point native `Core` code
//!    reaches a callback through, so there is one body and not a second
//!    convention beside it ([`lower::Lowering::lower_closure_call`]). ADR 0031
//!    § 3's self-name is still parsed and ignored, and a `name:` or `...`
//!    argument panics here for gap 8's reason. The type gap is **not** this
//!    crate's to close and is older than this lowering — a `callable` carries
//!    no parameter list (§ 1), so a closure declaring `string $s` reads a
//!    caller's `int` payload as a pointer whether that caller is `$f(1)` or
//!    `Core\Arr::map` over an `array<int>`; `mwl_runtime::closure`'s module
//!    doc owns it and states what closing it costs. Neither half of `&$x` is
//!    a gap any more: a closure *capturing* an enclosing `&$x` parameter takes
//!    § 2's by-value snapshot of the cell — one [`ir::InstKind::RefLoad`] at
//!    the literal, at the pointee type, retained like any other captured
//!    value, which is what lets the closure outlive the call that staged the
//!    cell — and a `&$x` parameter on the closure *itself* is `E0493`,
//!    `callable` carrying no parameter list for a call site to stage a cell
//!    against.
//! 10. **A `&$x` argument's copy-back is emitted at the enclosing statement**,
//!     so a read of the holder sequenced after the call but inside the same
//!     statement (`$n + Adder::bump($n)`) sees the pre-call value. This is no
//!     longer a scoping limit — [`lower::Lowering::lower_expr`] holds an
//!     `&mut Env` now — but the staging list is still drained at the
//!     statement. [`lower::Lowering::pending_refs`] owns it.
//! 11. **A `secret` value compared against a `mixed` one is not compared in
//!     constant time.** The qualifiers themselves are no longer a gap: all
//!     six of ADR 0024/0033's atoms erase to the plain `string`/`bytes` they
//!     share an allocation with ([`lower::lower_checked_ty`]), and ADR 0033
//!     § 5's constant-time `==` reaches every pair whose two operands are
//!     both that representation, through [`ir::Helper::SecretEq`] and the
//!     `mwl_types::expr_table::ExprInfo::SecretEquality` the checker records
//!     at the comparison. What that arm declines is the pair where one side
//!     is [`ty::Ty::Tagged`]: it has no buffer to read, so the comparison
//!     falls to [`ir::Helper::Identical`] and short-circuits. ADR 0033 § 2's
//!     poisoning makes the shape rare, and closing it means teaching
//!     `mwl_runtime::value_identical` the property rather than adding a
//!     lowering arm.
//! 12. **A `Stringable` operand stringifies; a `Core`-owned one does not.**
//!     `.`, an interpolated piece, `echo`/`print` and `as string` all desugar
//!     to the `toString()` `mwl_types::expr::operators::require_stringable`
//!     resolved under the operand's own span
//!     ([`lower::Lowering::lower_to_string_call`]) — the checker records it
//!     because none of those four sites is a call expression, so no
//!     `ExprInfo::Call` exists to read, and it reaches an operand typed at the
//!     interface itself as readily as a concrete implementor. What is left is
//!     a `Core`-owned class, which that check exempts and so records nothing
//!     for; it panics here. One arriving inside a [`ty::Ty::Tagged`] value
//!     throws instead, because [`ir::Helper::TaggedToString`] decides by tag
//!     at runtime and has no row for it.
//! 13. **Inline HTML at file scope is not lowered.** The lowering is the same
//!     [`ir::Helper::EchoStr`] call `echo` emits over the raw span; it is out
//!     only because `mwl_types` treats `InlineHtml` as a no-op too, so landing
//!     it widens two crates at once.
//! 14. **Two of the safepoint's four flags still do nothing.**
//!     [`ir::InstKind::Safepoint`] is emitted at function entry and every loop
//!     back edge, and `mwl-codegen` lowers it to a real poll: `CPU_LIMIT` and
//!     `CANCEL` stop the request, and the function-entry site also carries
//!     ADR 0020 § 1's call-stack compare. `COLLECT` and `DEBUG_BREAK` are
//!     cleared and otherwise ignored — there is no collector and no debugger
//!     to hand the frame to. Nothing in this crate is what is missing; see
//!     `mwl_runtime::mwl_safepoint`.
//! 15. **`decimal` lowers, but `<=>` over one does not.** ADR 0054's scalar
//!     has a representation now — [`ty::Ty::Decimal`], the same register pair
//!     [`ty::Ty::Tagged`] travels in, whose own doc comment owns the decision —
//!     and every row of that ADR's §§ 3-4 is an [`ir::Helper`]: the five
//!     arithmetic operators, negation, three comparisons that
//!     [`lower::Lowering::lower_decimal_binary`] rewrites into all six,
//!     truthiness, and both directions of every conversion. What is left is the
//!     spaceship operator, which has no `decimal` row here and no `int` one
//!     either — `<=>` reaches [`lower::Lowering::lower_expr`]'s panic for every
//!     scalar operand, and only ADR 0013's *object* form lowers. `**` is not a
//!     gap: ADR 0054 § 3 makes a `decimal` base a compile error, and
//!     `mwl_types` reports it.
//! 16. **`$x++` and `--$x` do not lower, and a compound assignment inherits
//!     whatever its binary form is missing.**
//!     [`lower::Lowering::lower_compound_assignment`] rewrites
//!     `$x op= e` into the `$x = $x op e` it means, so an operator gains its
//!     compound form exactly when its binary form lowers (`.=` on a plain
//!     `string` local is the one exception, and it takes
//!     [`ir::InstKind::StrAppend`] instead). The bitwise five closed that way
//!     rather than one at a time — `&`, `|`, `^`, `<<`, `>>` and unary `~` now
//!     have their [`ir::BinOp`]/[`ir::UnOp`] variants, so `&=`, `|=`, `^=`,
//!     `<<=` and `>>=` cost nothing — which leaves `**=` out for the same
//!     reason `**` itself is: [`ir::BinOp`] has no row for it, so
//!     [`lower::Lowering::lower_expr`] panics naming the operator. The rewrite
//!     also reads its target twice, so
//!     `is_reevaluable_target` refuses `f()->count += 1` rather than calling
//!     `f()` twice where PHP calls it once; closing that means splitting the
//!     target's address computation out of
//!     [`lower::Lowering::lower_reassignment`]. An increment needs the same
//!     split for a second reason: its `1` has no source span to build an
//!     [`mwl_syntax::ast::ExprKind::Int`] from, so it cannot be desugared
//!     into an AST node the way every other compound form is.
//! 17. **The environment is one flat, function-wide map**, so a nested block
//!     declaring a local that shadows an outer one is not distinguished from a
//!     reassignment. Not observable for any program in scope today, but worth
//!     knowing before trusting `Env` further.
//! 18. **An abandoned generator never runs the `finally` it is suspended
//!     inside.** A `break` out of a `foreach` leaves the state machine parked
//!     at its `yield` and nothing resumes it; PHP resumes the generator in a
//!     return-like mode when it is destroyed, so a `try { … yield … } finally`
//!     prints its `finally` there and not here. The two suites pin the halves
//!     that do agree — `tests/differential/iter/a-generators-finally-matches-phps.mwlt`
//!     for a drained generator and `…/a-generator-abandoned-by-break-matches-php.mwlt`
//!     for a body with no protected region — so only the suspended-inside-a-`finally`
//!     case is open. Closing it wants a "resume to unwind" entry point on the
//!     state machine plus a release-path call to it, which is the first thing
//!     in this crate that looks like a destructor
//!     ([ADR 0028](../../../docs/adr/0028-closing-the-remaining-magic-methods.md)
//!     says MWL has none), so it is a design call, not a patch.
//! 19. **ADR 0090 is built; what a cross-representation pair still cannot do
//!     is *arithmetic*.** Every row of §§ 2, 3 and 5 lowers: `===`/`!==` no
//!     longer lex, `== null` takes
//!     [`lower::Lowering::lower_null_identity`]'s tag test, a same-
//!     representation pair is [`ir::BinOp::Eq`]/[`ir::BinOp::NotEq`], a
//!     `string`, `array` or `object` pair is that `BinOp` with the row's own
//!     comparison chosen in `mwl-codegen`, a `mixed` or union operand is § 5's
//!     [`ir::Helper::Identical`], and § 2's numeric row — the one pairing
//!     whose two operands hold two *representations* — is
//!     [`ir::Helper::NumericEq`], settled in
//!     [`lower::Lowering::lower_binary`] rather than in `mwl-codegen`, whose
//!     "a `BinOp` has one representation" invariant therefore still holds.
//!     The disjoint-operand refusal of § 2 is `mwl_types`' half, not this
//!     crate's.
//!
//!     The same mismatch under a *different* operator is closed too, and it
//!     splits in two rather than following equality — which is
//!     [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 4's own
//!     division, not a new one. **Arithmetic widens**: `$n + $f` is that
//!     table's "either operand a `float`" row, so the integer side is
//!     converted in [`lower::Lowering::lower_binary`] through the very
//!     [`ir::Helper::IntToFloat`] a written `$n as float` emits — § 2 names
//!     this as the one implicit conversion in the language and gives it that
//!     conversion's own behaviour, exact or throwing above 2^53.
//!     **Ordering does not**: § 4 closes by saying a comparison "has an exact
//!     answer in the mathematical integers and can be lowered as one", so
//!     `$n < $f` takes [`ir::Helper::NumericLt`] beside its equality sibling.
//!     Widening there would be wrong twice — rounding away every integer past
//!     2^53, and, since the widening throws rather than rounds, raising
//!     `ArithmeticError` where PHP answers an ordering.
//!
//!     Nothing of this reached [`lower::Lowering::coerce`], whose rows
//!     reconcile [`ty::Ty::Tagged`] and emit nothing that can fail; a
//!     conversion carrying ADR 0002's error edge does not belong in one.
//!
//! 20. **ADR 0047 § 5, ADR 0010 § 5 and ADR 0007 § 2's scalar rows all run
//!     whole, a `mixed` source included; what is left is § 2's two
//!     *non-scalar* rows.** A union whose members all erase to one representation
//!     is that representation ([`lower::lower_checked_ty`]), so `"a"|"b"` is a
//!     `Ty::Str`, `1|2` a `Ty::Int` and `Mode::Read|Mode::Write` the enum's
//!     own tag rather than the `Ty::Tagged` every union used to be, and § 4's
//!     checked row runs the membership test
//!     [`lower::Lowering::lower_literal_membership`] emits — a comparison per
//!     member, throwing through [`ir::Helper::LiteralMismatch`] with the
//!     accepted set named. § 3's enum-case subset is in that set now: every
//!     lowering entry point takes the run's `mwl_types::EnumTable` (handed
//!     back by `mwl_types::check_program` rather than rebuilt, so ADR 0010
//!     § 1/§ 2's declaration errors are not reported twice), which is where a
//!     case's constant lives — [`ir::ExprInfo::EnumCase`] carries one only for
//!     a case written as an *expression*, and a case named in a **type** has
//!     no expression to record one against. An enum operand is reinterpreted
//!     to its backing integer for the chain, because `mwl-codegen` lowers
//!     `BinOp::Eq` over `Ty::Int`/`Ty::Uint` and not over `Ty::Enum`.
//!
//!     ADR 0010 § 5's other direction runs too, so that section is whole for
//!     a statically typed operand. `$n as Mode` is one free
//!     [`ir::InstKind::Reinterpret`] — an enum is a tag over its backing
//!     integer — in front of which
//!     [`lower::Lowering::lower_literal_membership`] emits the same chain,
//!     built from **every** case of the declaration
//!     ([`lower::whole_enum_set`], sorted by the case's constant because the
//!     table behind it is a hash map). An operand that is not already the
//!     backing scalar is converted to it by ADR 0007 § 2's own rows first, by
//!     recursion inside [`lower::Lowering::convert`] rather than a row per
//!     source, so `$f as Rank` and `$s as Rank` each throw naming whichever
//!     of the two steps failed. An operand already at the enum's own
//!     representation is skipped entirely: `mwl_types` refuses a conversion
//!     between two *different* enums, so it names a case by construction.
//!
//!     A [`ty::Ty::Tagged`] source is no longer the hole it was: it is one
//!     helper per *target*, chosen by the operand's runtime tag because
//!     nothing static names a row — [`ir::Helper::TaggedToString`],
//!     [`ir::Helper::ToDecimal`], and [`ir::Helper::TaggedToInt`] with its
//!     unsigned and `float` twins, each throwing exactly where ADR 0066's
//!     [`ir::Helper::ToIntOrNull`] answers `null` over the same rows in
//!     `mwl_runtime`. So `$any as int` runs, and `$any as Mode` and
//!     `$any as Mode::Read|Mode::Write` run through it: the recursion above
//!     converts the operand to the enum's backing scalar first, and the
//!     membership chain is unchanged.
//!
//!     ADR 0009 § 3's `string` ↔ `bytes` pair runs too, and it is the one
//!     conversion whose two directions are lowered by different mechanisms:
//!     `string as bytes` is total and free, so it is an
//!     [`ir::InstKind::Reinterpret`] over the same allocation and emits no
//!     call, while `bytes as string` validates UTF-8 through
//!     [`ir::Helper::BytesToString`] and throws rather than substituting.
//!
//!     What panics is ADR 0007 § 2's `array<T> as array<U>`, whose O(n)
//!     element walk is the one row in that table that is not a single helper
//!     call, and a [`ty::Ty::Tagged`] operand converted to `bytes` — the one
//!     target with no runtime-tag row of its own.
//!
//!     A statically settled operand needs no check and already worked, since
//!     `mwl_types` refuses `E0470` before lowering ever sees it. `as ?"a"`
//!     ([ADR 0066](../../../docs/adr/0066-nullable-conversion-operator.md)'s
//!     non-throwing form) runs no membership test either: its yield-`null`
//!     miss arm has no shared representation with its hit arm, so it needs a
//!     merge the throwing form does not.
//! 21. **A binding declared at ADR 0007 § 3's opaque `object` top has no
//!     representation arm.** `erase_checked_ty` maps a *named* class to
//!     [`ty::Ty::Object`], but the checker's own `object` reaches no arm at
//!     all, so `object $o = $obj;` — or any parameter or return declared
//!     `object` — panics naming itself. The representation is not in question:
//!     it is the same pointer a named class already erases to, and the arm is
//!     one line. What is unverified is whether anything below this crate reads
//!     a class *label* off an operand it would now receive without one, which
//!     is what a session landing it owes a check of.
//! 22. **A `require`d file's own top-level statements are not run.**
//!     [`lower::lower_program`] gives a script frame to `files[0]` — the entry
//!     point — and takes only the *declarations* of every other file, and a
//!     `require` in statement position lowers to nothing, because the graph is
//!     already resolved by the time lowering starts. So a required file that
//!     writes `echo "loaded";` at file scope compiles and stays silent, and
//!     ADR 0021 § 3's value form (`$c = require 'config.mwl';`) has no arm at
//!     all — it panics in `lower_expr` like any other unsupported shape. The
//!     shape that closes both is one frame per file, called from the site, and
//!     the open question it raises is whether that frame shares the caller's
//!     locals (ADR 0021's "no isolation") or not — which is why this is
//!     recorded rather than guessed at. The declaration half, which is what
//!     ADR 0061's autoload map needs, runs today.

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
