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
//!   `foreach` over all three of ADR 0053 § 3's subjects, `break`/`continue`
//!   at level 1, `try`/`catch`, `throw`.
//! - **Expressions** — arithmetic and comparison, `.` concatenation and string
//!   interpolation, `new`, static/instance/`Core` calls, property and
//!   array-element read and write, array literals including an explicit
//!   `key =>` and `$a[] =` append, `&&`/`||`/`!` and the ternary/elvis
//!   operator, ADR 0035's truthy conversion, ADR 0031 closure literals,
//!   `instanceof`, `??`, the literal `null`, ADR 0007 § 2's scalar conversion
//!   rows — free, total and checked alike — and ADR 0066's non-throwing
//!   `as ?T` over the checked numeric targets.
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
//!    a `null` constant — and a plain `->` on the receiver that test narrowed
//!    is one unchecked [`ir::InstKind::Untag`]
//!    ([`lower::Lowering::untag_receiver`]; `mwl_types::locals` owns the
//!    proof).
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
//!    Separately, ADR 0066 § 3 makes `as ?T` a **compile error** where the
//!    conversion cannot fail (`$i as ?string`) or does not exist at all
//!    (`$arr as ?int`); `mwl_types` refuses neither yet, so both reach
//!    lowering and panic naming that ADR instead of being diagnosed.
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
//! 6. **Property and array access are compile-time-known-target-only.** A
//!    receiver that erased to a plain `object`, or a shape asked for a field
//!    it does not name (ADR 0036 § 4), has no `ExprInfo` entry, so lowering
//!    panics; the checker defers that runtime check to M4. A shape receiver
//!    naming one of its own fields does lower — one
//!    [`ir::InstKind::SlotGet`] at the index the checker resolved — but only
//!    as a **read**: writing a shape's field still panics. An anonymous
//!    `{a: 1}` literal now constructs, as an instance of the class
//!    [`lower::shape_class_label`] names, and that fixed-offset read is
//!    **wrong through a widened view** — ADR 0036 § 4 calls for a name-keyed
//!    fetch precisely because two values satisfying one shape lay their
//!    fields out differently, and nothing here does that yet, so a `{y: int}`
//!    parameter handed a `{x: 1, y: 2}` reads slot 0 and answers `1`. The
//!    same hole is open for a *named class* flowing into a shape-typed
//!    binding, which predates the literal. Until it closes, a shape type is
//!    sound only where it is the value's own exact shape. Nullsafe `?->` *reads* — a call and a property alike, over
//!    the one guard [`lower::Lowering::open_nullsafe`] opens — but a nullsafe
//!    assignment target (`$a?->b = v`) panics, which PHP refuses outright and
//!    `mwl_types` does not diagnose yet. An
//!    array-element write through a hooked property is refused: the
//!    copy-on-write separation would have to be written back through the `set`
//!    hook, and no PHP-compatible rule for that exists yet. A *nested* write —
//!    `$grid[0][1] = v`, whose base is itself an index expression — is refused
//!    for the same reason: the separated inner array has to be written back
//!    into the outer one, and only a local or a known property is a place this
//!    crate can write back to. Reading `$grid[0][1]` is fine. Neither
//!    [`ir::InstKind::ArrayGet`] nor [`ir::InstKind::ArraySet`] models an
//!    absent key at runtime — deferred wholesale, like every other checked
//!    throw. A **static** property is narrower still: it reads, but
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
//! 8. **No named or spread call argument**, and no `...spread` or `&value`
//!    array-literal element. `mwl_types` does not fully positionally
//!    type-check a named or spread argument either, so there is no resolved
//!    per-argument type to lower against. A **variadic** signature is no
//!    longer among them: `lower::Lowering::lower_variadic_tail` collects every
//!    argument from that parameter's position into one fresh array, which is
//!    the single value the parameter receives — see
//!    `mwl_stdlib::registry::CoreTy::Variadic` for why that shape rather than a
//!    second, count-carrying calling convention.
//! 9. **A closure literal lowers; `$f(...)` does not.** The only caller today
//!    is native `Core` code going through `mwl_runtime::mwl_closure_call`.
//!    ADR 0031 § 3's self-name is parsed and ignored, and a `&$x` capture or
//!    parameter panics — the cell it addresses is the caller's, and a closure
//!    may outlive the call that staged it.
//! 10. **A `&$x` argument's copy-back is emitted at the enclosing statement**,
//!     because that is the nearest scope holding an `&mut Env` — so such a
//!     call lowers only as a bare expression statement or an assignment's
//!     right-hand side. [`lower::Lowering::pending_refs`] owns it.
//! 11. **A `tainted`/`secret`-qualified type has no IR arm.**
//!     [`lower::lower_checked_ty`] handles the plain `string`/`bytes` only;
//!     ADR 0024/0033's qualifiers are compile-time-only and need no runtime
//!     representation, but the erasure has to be written.
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
//! 14. **Safepoints are reserved, not functional.** [`ir::InstKind::Safepoint`]
//!     is emitted at function entry and every loop back edge, but nothing
//!     lowers it to a real CPU-limit or cancellation check yet.
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
//! 16. **`$x++` and `--$x` do not lower, the bitwise operators have no
//!     [`ir::BinOp`] variant at all, and a compound assignment inherits both
//!     holes.** [`lower::Lowering::lower_compound_assignment`] rewrites
//!     `$x op= e` into the `$x = $x op e` it means, so an operator gains its
//!     compound form exactly when its binary form lowers — which leaves
//!     `&=`, `|=`, `^=`, `<<=`, `>>=` and `**=` out for the same reason `&`
//!     and `**` themselves are out: [`ir::BinOp`] stops at the arithmetic,
//!     equality and ordering rows, so [`lower::Lowering::lower_expr`] panics
//!     naming the operator. The rewrite also reads its target twice, so
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
//!     What is left is the same mismatch under a *different* operator, and it
//!     belongs to [ADR 0007](../../../docs/adr/0007-explicit-type-system.md)
//!     § 4 rather than to 0090: `mwl_types` gives `$n + $f` a result type and
//!     `$n < $f` a `bool` without either side being converted, so both still
//!     reach `mwl-codegen` as two representations and are refused there. ADR
//!     0007 § 4's promotion table is what says which side widens, and unlike
//!     equality that widening *is* the semantics rather than an approximation
//!     of it, so it belongs in [`lower::Lowering::convert`]'s existing rows.
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
