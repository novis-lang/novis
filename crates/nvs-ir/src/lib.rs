//! Novis's CFG/SSA IR: the one representation between the checked AST and
//! `nvs-codegen`, carrying explicit safepoints, refcount operations and
//! runtime-helper calls, with a stable per-statement and per-edge id
//! (`rule:testing/debug-probes`).
//!
//! [`lower`] is the whole front-to-IR pass, split across `lower/` by area;
//! [`ir`] is the data; [`ty`] is this crate's own representation-level type
//! lattice; [`mod@print`] renders a program for the snapshot tests.
//!
//! # What lowers today
//!
//! Whole-program: [`lower::lower_file`] walks a file, [`lower::lower_method`]
//! one method, [`lower::lower_property_hook`] one `rule:classes/property-observer` accessor, and
//! [`lower::lower_script`] a file's own top-level statements as one synthesized
//! frame of ordinary locals with no receiver (`rule:statements/storage-that-outlives-a-call`), returning
//! [`ty::Ty::Tagged`] because that is what `rule:statements/require-is-the-only-inclusion-construct` types a `require`'s result.
//!
//! - **Statements** — typed and `var` local declarations (`rule:types/var-inference`), a typed
//!   one with no initializer at all (the type is fixed and remembered, and
//!   the first assignment binds it — [`lower::Lowering::declared_tys`]), the
//!   empty statement `;`, a run of inline HTML
//!   ([`lower::Lowering::lower_inline_html`] — the same
//!   [`ir::Helper::EchoStr`] call `echo` emits, over the raw span),
//!   reassignment, `return`, nested blocks, `echo`, `unset`, `if`, `while`,
//!   `do`/`while`, `for`, `switch`,
//!   `foreach` over every one of `rule:iteration/foreach-subjects`'s subjects, `break`/`continue`
//!   at any level ([`lower::Lowering::lower_break`] counts every enclosing
//!   loop and `switch`, PHP's own rule), `try`/`catch`, `throw`.
//! - **Expressions** — arithmetic and comparison, `.` concatenation and string
//!   interpolation, `new`, static/instance/`Core` calls, property and
//!   array-element read and write, array literals including an explicit
//!   `key =>` and `$a[] =` append, `&&`/`||`/`!` and the ternary/elvis
//!   operator, `rule:expressions/truthy-positions`'s truthy conversion, `rule:types/anonymous-function`'s anonymous functions,
//!   `is`, `??`, the literal `null`, `rule:types/conversion`'s scalar conversion
//!   rows — free, total and checked alike — and `rule:expressions/nullable-conversion`'s non-throwing
//!   `as ?T` over the checked numeric targets and over its § 3 parse roster.
//! - **Types** — `int`/`uint`/`float`/`bool`/`decimal` scalars, `string`,
//!   `bytes`,
//!   `array<T>` (element type erased — see [`ty::Ty`]), `object` (a class or
//!   enum, likewise erased), `mixed`, `null`, `?T` and any other union (each
//!   tagged — see [`ty::Ty::Tagged`]), and [`ty::Ty::Ref`] for an `inout $x`
//!   parameter. `string`, `bytes` and `array<T>` are refcounted and cross a
//!   local, call-argument, return and property boundary alike.
//! - **Generators** — `rule:iteration/generators`'s state-machine transform, in
//!   [`lower::generator::lower_generator`]: one declaration becomes a factory, an
//!   `advance()`, a `current()` and a synthesized state class.
//!
//! # Design choices worth knowing before widening this
//!
//! - **SSA, not a plain CFG.** `if`/`while`/`for` are each a hand-rolled
//!   merge, not a dominance-based phi-placement algorithm — sufficient for any
//!   structured nesting, since none produces a join of another shape. Every
//!   construct reuses the same building blocks: `merge_envs` for a set of
//!   incoming edges known up front (`if`, `for`'s step block, a `switch` case
//!   body's label and fall-through edges, a `match`'s arm phi), and
//!   `lower_while`'s seed-then-patch phi dance for a join whose back edge is
//!   not known until its body is lowered.
//! - **IR types are representation-level, not the checker's types.** See
//!   [`ty`]'s own module docs for why [`ty::Ty`] is a small, flat lattice
//!   rather than a reuse of `nvs_types::ty::Ty`.
//! - **This crate depends on `nvs-types` only for its typed-expression table,
//!   never on `nvs-hir`'s class graph directly.** Every *declared* type is read
//!   straight off the AST (`rule:types/declaration` requires it spelled out, so no name
//!   resolution is needed). What genuinely is absent from the AST is a call's
//!   *resolved target* — which class declares the callee, and its parameter
//!   and return types — so `nvs_types::expr_table::ExprTypeTable` publishes
//!   that, keyed by source span, and lowering reads it back. The alternative,
//!   re-running class-hierarchy resolution here, would have duplicated
//!   `nvs-types`. Lowering **trusts** that its input already passed
//!   `nvs_types::check_program` with the same tables, and panics naming an
//!   unsupported shape rather than diagnosing.
//! - **A receiver is an implicit first parameter, not a special case.** Every
//!   lowered method carries `$this` at [`ir::Function::params`] index 0,
//!   whether or not the body reads it — mirroring `nvs_types`' own
//!   `check_method`. A separate `Option<ValueId>` would have needed `Env`'s
//!   `$this` lookup to take a different path than every other local, for a
//!   value that behaves exactly like an ordinary parameter.
//! - **Refcount insertion is naive and syntactic, not a liveness analysis.**
//!   Correctness first; elision is a named later optimizer pass. Reading a
//!   value out of storage another binding still owns — [`lower::is_aliasing_read`]
//!   names the syntactic shapes, and `Lowering::aliasing_read` is the judgment
//!   every decision actually goes through, because not all of them are
//!   syntactic: an `rule:classes/property-observer` `get` hook is a call, and a field or element read
//!   whose *base* is a temporary owns its own result — and copying it into a second
//!   durable slot needs a retain; a freshly constructed value needs none,
//!   since it already has one natural owner. A slot's previous value is released when overwritten,
//!   every live slot is released at frame exit, and a binding **control flow
//!   drops** is released at the point it disappears — a local declared inside
//!   a loop body or inside one `if` branch. That last edge is the one a
//!   syntactic pass most easily gets wrong, which is why AGENTS.md says to run
//!   `tools/leak-check.sh` against a fixture exercising any new refcount edge.
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
//! - **An array key is a `string`, and an `int` subscript does not spell
//!   it.** [`ir::InstKind::ArrayGet`]'s and [`ir::InstKind::ArraySet`]'s `key`
//!   operand carries either [`ty::Ty::Str`] or [`ty::Ty::Int`], and codegen
//!   picks the runtime primitive off the operand's own representation — there
//!   is no second instruction, no key-kind field and no new [`ir::Helper`].
//!   The alternative shapes are each worse for a reason worth recording: a
//!   separate `ArrayGetIndex` doubles every array instruction and every match
//!   arm over them for one operand's type, and a `key_is_int: bool` states
//!   twice what `ty::Ty` already states once. The consequence a widening
//!   contributor must hold: **a key operand's `Ty` is load-bearing**, so
//!   the refcount decision at each of `lower_array_key`'s call sites turns on
//!   it — an `int` key owns nothing to retain or release. `rule:types/arrays` is
//!   unaffected; every key *is* a `string`, `"08"` is distinct from `"8"`,
//!   and `$a[8]` is `$a["8"]`. The decimal is never produced at all, which is
//!   `nvs_runtime::array`'s packed form making it unnecessary. A subscript
//!   renders where that form cannot take the key:
//!   a `uint`, because the runtime's index ABI is an `i64` and a `uint` above
//!   `i64::MAX` has no `i64` spelling naming the same key, and any key
//!   reaching [`ir::InstKind::ArrayUnset`], which has no index-shaped
//!   primitive beside it. `lower::Lowering::lower_array_key` and
//!   `lower_rendered_array_key` own those exceptions.
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
//!   `nvs-runtime` ABI question (M9 freezes that surface) and not a lowering
//!   one. Nothing depends on the copy happening, so it can be taken away
//!   without changing an observable.
//! - **A shape's wire contract is a constant of the call site, never of its
//!   class.** A member handed an inline shape as its type argument —
//!   `Core\Arr::shapeAs<{n: int}>` — needs the per-field wire types
//!   `nvs_runtime::CodecField` carries, exactly as `Json::decodeAs<User>`
//!   needs `User`'s. The descriptor those would hang on is shared:
//!   `lower::shape_class_label` keys a shape class on its sorted field
//!   *names* alone, so `{n: int}` and `{n: string}` are one class and one
//!   `nvs_runtime::ClassDesc`; and a shape class is synthesized in this crate
//!   rather than laid out in `nvs_types::layout`, so [`lower::lower_file`]'s
//!   codec join has no slot order to run one against either. The list
//!   therefore rides *beside* the descriptor as its own constant of the call —
//!   `nvs_runtime::ShapeCodec`, owned by the unit's class table and addressed
//!   the way [`ir::InstKind::ClassDescConst`] addresses a descriptor — and
//!   `nvs_runtime::ClassDesc::codec` stays the answer for a class and only for
//!   a class. `rule:types/shape-type` is the type this carries, and
//!   `rule:core-api/required-optional-and-nullable`'s three columns are what
//!   each field of it says.
//!   **What was refused is a class keyed on names *and* types.** That is the
//!   cheaper edit — the field list goes back onto the descriptor and
//!   `nvs_types::derive`'s existing recording path works untouched — and it
//!   spends a guarantee two other crates already rest on: `nvs_stdlib::json`'s
//!   encoder spells a shape slot by slot *because* one class cannot name a
//!   per-field type, and `nvs_stdlib::task`'s `all` builds its result with its
//!   argument's own descriptor because one field set is one class on both
//!   sides. Two descriptors for one field set turns each of those into a
//!   question about which spelling the unit met first. Encoding the contract
//!   as ordinary Novis values the call site builds was refused on the third
//!   priority instead: it is an allocation and a parse per call for a fact the
//!   checker already knows.
//!   The consequence a widening contributor must hold: **a call site whose
//!   type argument names a shape registers that shape's class itself.**
//!   `lower::Lowering::lower_anon_object` synthesizes one only where a
//!   literal is *written*, so a unit that hydrates a `{n: int}` it never
//!   spells would otherwise have no `$shape{n}` for the descriptor constant to
//!   resolve to — `lower::Lowering::written_type_constants` is where it does
//!   both, and the third constant it emits is
//!   [`ir::InstKind::ShapeCodecConst`] naming an entry of
//!   [`ir::Program::shape_codecs`]. The table is keyed on the contract rather
//!   than on the class (`lower::shape_codec_key`), so two sites writing
//!   `{n: int}` share one and a third writing `{n: string}` gets its own.
//!   What this spends, per `rule:programs/memory-priority`: one table per
//!   distinct written shape in the program, O(distinct types) like every other
//!   interned descriptor and never O(requests served).
//!
//! # What each area lowers, and the limit it holds within
//!
//! One bullet per area, read whole: what reaches the IR, which instruction or
//! [`ir::Helper`] carries it, and where the boundary is. A limit stated here is
//! a decision rather than a hole — `# Known gaps` at the foot of this file is
//! that register — and the doc comment each bullet names is the home of the
//! detail it points at.
//!
//! 1. **A fresh value is staged before anything that can throw, never released
//!    inline.** A landing block sweeps the frame's owned-temporaries stack as
//!    well as its locals, so a call's arguments and receiver, the operands of
//!    `.`, an interpolation and an `echo`, a `match` subject and a rendered
//!    subscript key ([`lower::Lowering::lower_array_key`]) are released on both
//!    edges. [`lower::Lowering::landing_block`] states the boundary and
//!    `lower::Lowering`'s own field doc the one asymmetry: a *transferred*
//!    argument is released on the error edge and forgotten on the normal one,
//!    the callee owning it from the instruction that reaches it.
//! 2. **A tagged value is built, carried, narrowed and dispatched on, on
//!    every path this bullet names.** [`ty::Ty::Tagged`] is the one representation `mixed`,
//!    `?T` and every other union erase to, and its own doc comment owns the
//!    decision and what it spends. What lowers today: a `?T` local, parameter,
//!    property, return value and call argument; the literal `null`;
//!    [`ir::InstKind::Tag`]/[`ir::InstKind::Untag`] at every boundary carrying
//!    a declared type ([`lower::Lowering::coerce`]); and `??`
//!    ([`lower::Lowering::lower_coalesce`]), whose non-`null` arm narrows
//!    against the type `nvs_types::expr_table::ExprInfo::Coalesce` records.
//!    `rule:expressions/nullable-conversion`'s `as ?T` ([`lower::Lowering::convert_or_null`]) *reads* a
//!    tag instead — its helper dispatches on the
//!    operand's, which is what a `mixed` source costs, and is the shape the
//!    rest of this bullet describes. `.` and `echo` read one the same way
//!    ([`ir::Helper::TaggedToString`]). `?->` reads one only to *test* it
//!    ([`lower::Lowering::open_nullsafe`]), as does `== null`/`!= null`,
//!    which is one [`ir::InstKind::IsNull`] rather than a comparison against
//!    a `null` constant — and a *read* of the local that test narrowed is one
//!    unchecked [`ir::InstKind::Untag`] at the read itself
//!    ([`lower::Lowering::untag_narrowed`], off
//!    `nvs_types::expr_table::ExprInfo::NarrowedRead`; `nvs_types::locals`
//!    owns the proof). Narrowing there rather than at each consumer is what
//!    lets a subscript base, a `foreach` subject, an array-write root and an
//!    argument all see the narrow representation with no site to forget;
//!    [`lower::Lowering::untag_receiver`] is the same move for a call
//!    receiver.
//!    `rule:expressions/truthy-positions`'s truthy table is read from the tag the same way
//!    ([`ir::Helper::ValueTruthy`]), and so is `rule:types/arithmetic`'s **ordering**
//!    table: `<`/`<=`/`>`/`>=`/`<=>` with a tagged operand take
//!    [`ir::Helper::ValueLt`] and its two siblings, which answer the rows the
//!    tags name and *throw* where that closed table names none — the one
//!    comparison helper family carrying `rule:errors/propagation`'s error edge, and its own doc
//!    comment is that decision's home. A subscript through a tagged base is
//!    not here either: `nvs_types` refuses it where it is written
//!    (`E0482`), an `array<T>` binding being what has an element type to check
//!    a read against.
//!    **Arithmetic** reads a tag the same way and by the same argument:
//!    `rule:types/arithmetic`'s rows over a tagged operand are the
//!    [`ir::Helper::ValueAdd`] family, whose own doc comment is that
//!    decision's home — one helper per operator dispatching on the pair of
//!    tags, each carrying the error edge a closed table needs, and no second
//!    representation anywhere.
//! 3. **Every conversion row lowers, in both spellings.**
//!    `rule:types/conversion`'s free, total and checked scalar rows all lower, in both
//!    the throwing form ([`lower::Lowering::convert`]) and `rule:expressions/nullable-conversion`'s
//!    non-throwing `as ?T` ([`lower::Lowering::convert_or_null`]). The integer
//!    *into* an enum row runs the declaration's own case set
//!    ([`lower::convert::whole_enum_set`]) through the membership chain the
//!    last bullet below describes, and the `?` spelling runs that chain with a
//!    `null` miss arm (`lower::Lowering`'s `lower_nullable_membership`).
//!    `EnumName` ↔
//!    `string` is not a row — `rule:types/conversion` leaves it out of the language.
//!    `rule:expressions/nullable-conversion-availability`'s own refusals are all `nvs_types`' and none reaches here:
//!    a conversion that cannot fail (`$i as ?string`) is
//!    `nvs_diagnostics::code::E_NULLABLE_CONVERSION_CANNOT_FAIL` and one that
//!    does not exist at all (`$arr as ?int`) is `E_NO_CONVERSION`, that
//!    table's completeness check applied to the `T` inside the sugar. The other direction —
//!    a row § 3 calls **available** — has its `?` helper: `$m as ?array<U>` is
//!    [`ir::Helper::ToArrayOfOrNull`], the same element walk the checked
//!    spelling runs, answering `null` where that one throws. The text
//!    targets have theirs:
//!    [`ir::Helper::ToStringOrNull`] is [`ir::Helper::TaggedAsString`]'s twin
//!    over one implementation of `rule:types/conversion`'s rows, `bytes`
//!    source included, answering `null` where that one throws — and
//!    [`ir::Helper::ToBytesOrNull`] is [`ir::Helper::TaggedToBytes`]'s twin
//!    over that pair's other direction, the two rows a tag can take into a
//!    `bytes` being the `string` and the `bytes` itself. The **class-target** refusal is
//!    § 3's —
//!    `nvs_diagnostics::code::E_CLASS_CONVERSION_TARGET`, and it is absolute,
//!    so no class reaches this crate through `as` at all: `Core\Uri::tryParse`
//!    is an ordinary member call.
//! 4. **A ternary — or a `match` — whose branches lower to two different
//!    [`ty::Ty`] representations joins at [`ty::Ty::Tagged`].**
//!    [`lower::Lowering::join_representations`] widens each branch in its own
//!    block through [`lower::Lowering::coerce`], and owns why it does not reach
//!    for `rule:types/arithmetic`'s promotion rows: the checker has already
//!    typed the whole expression as the *union* of its branches, so
//!    `$c ? 1 : 2.5` keeps PHP's `int` on the truthy path and a `float`
//!    binding widens once, at the binding. *Where* a short-circuit may
//!    appear is not a restriction: [`lower::Lowering::lower_expr`] owns
//!    a `&mut BlockId` and lowers its own sub-expressions through itself, so
//!    `&&`/`||`/`!`/ternary/`??` compose inside a call argument, an array
//!    element, a `.` operand or an `echo` operand alike.
//! 5. **Array access is compile-time-known-target-only, and `nvs_types` is
//!    what says so rather than this crate; property access is not
//!    compile-time-known-target-only.** A subscript whose base
//!    declares no element type — a `mixed`, a scalar, a `?array<T>` no test
//!    narrowed — is `E0482` where it is written, and `$a[]` anywhere but an
//!    assignment target is `E0481`, so both of
//!    [`lower::Lowering::lower_index`]'s panics and the assignment arm's
//!    matching one are invariant checks no source file reaches.
//!    Every `rule:types/erased-member-access` receiver lowers: a shape naming one
//!    of its own fields, a shape asked for a name it does not list, a
//!    plain `object`, and a `mixed`. Each is one
//!    [`ir::InstKind::SlotGet`] — § 4's
//!    **name-keyed** fetch, which is therefore right through a widened view
//!    too — or one [`ir::InstKind::SlotSet`], which additionally checks the
//!    incoming value's tag against what the concrete class declares the field
//!    to hold; `nvs_runtime::object`'s docs § *What a shape write checks*
//!    state what that granularity misses. The erased ones differ only in what
//!    the checker could record: no slot to hint (so `0`, which the runtime's
//!    by-name search corrects) and no type (so [`ty::Ty::Tagged`]), which
//!    makes § 4's missing-name throw reachable rather than theoretical. A
//!    `mixed` receiver adds the one thing the others cannot: an
//!    unproven *tag*, so `lower::expr`'s `ReceiverProof::Erased` emits no
//!    [`ir::InstKind::Untag`] and the whole tagged value reaches the runtime,
//!    which throws in PHP's own wording for a receiver that is not an object
//!    — every *statically* non-object receiver having been `E0495` at the
//!    checker (`rule:types/a-declared-type-answers-before-the-program-runs`). An
//!    anonymous `{a: 1}` literal
//!    constructs, as an instance of the class [`lower::shape_class_label`]
//!    names. Nullsafe `?->` *reads* — a call and a property alike, over
//!    the one guard [`lower::Lowering::open_nullsafe`] opens — but a nullsafe
//!    assignment target (`$a?->b = v`) is `E0479` in `nvs_types`, which is
//!    what PHP refuses too. An array-element write through a hooked property
//!    is `E0478` and one through an erased property `E0480`, both for the
//!    reason those codes' own rows state, and one whose root is no place at
//!    all — `$h->rows()["a"] = v`, `[1, 2]["0"] = v` — is `E0700`, another
//!    entry in the same `check_write_target` and the one shape
//!    [`lower::Lowering::write_back_array`]'s catch-all would otherwise be
//!    reached through. Parentheses are not such a root:
//!    `nvs_syntax::ast::Expr::unparenthesized` is what every walk on this path
//!    uses to find the holder, so `($a)["0"] = v` writes `$a` as it does in
//!    PHP. An *increment* is a write like any
//!    other and earns whichever of those three its own target does: `$a?->b++`
//!    desugars into the compound assignment this crate lowers by reading the
//!    target twice, so the checker refuses it where it is written rather than
//!    leaving it to that rewrite's own panic. A *nested* write — `$grid[0][1] =
//!    v`, whose base is itself an index expression — lowers: it flattens to
//!    its root holder plus one key per level, descends, and writes every
//!    level back with the outermost last, auto-vivifying an absent row the
//!    way PHP does. Neither
//!    [`ir::InstKind::ArrayGet`] throws on an absent key
//!    (`rule:types/absent-storage-is-never-a-zero-value`), which is the *read* side's answer to the same question this
//!    vivifying descent asks; [`ir::InstKind::ArraySet`] models no absent key
//!    at all, because a write is what makes one present.
//! 6. **An anonymous function lowers, and so does `$f(...)`, and what an argument
//!    is checked against travels on the callable object.** The call is one [`ir::Helper::CallCallable`]
//!    — `nvs_runtime::call_callable`, the same entry point native `Core` code
//!    reaches a callback through, so there is one body and not a second
//!    convention beside it ([`lower::Lowering::lower_callable_call`]). `rule:types/anonymous-function-self-name`'s self-name lowers too, and lowers to nothing: the callable it names
//!    is the invoke's own receiver, already bound under
//!    [`lower::anon_fn::FN_SELF`], so the recursive call is the same
//!    `CallCallable` with that binding as its callee and the environment class
//!    gains no field. What decides *which* bare name is one is
//!    `nvs_types::expr::calls::check_anon_fn`, whose record this crate
//!    reads. A `...` argument goes
//!    through [`ir::Helper::CallCallableArray`] instead — the whole list built
//!    into one array, because `CallCallable`'s own argument count is a literal
//!    in the emitted call and a spread's is not — and a `name:` one is refused
//!    where it is written (`E0712`), § 1 leaving no parameter for a name to
//!    fill at either end. The method-reference sentinel is the one
//!    `$f(...)` that makes no call: `rule:types/callable-values` gives
//!    `callable` a single inhabitant, so the site answers the callable `$f`
//!    already holds — PHP's own answer, retained once so the value leaves as a
//!    fresh owner. A `callable` carries
//!    no parameter list (§ 1), so no checker can compare a call site against
//!    the body it will reach: this crate packs the declared representations
//!    into the callable object's own `FN_PARAM_TAGS` word where the anonymous function is written
//!    ([`lower::anon_fn::param_tags_word`]), and
//!    `nvs_runtime::callable`'s `check_param_tags` compares one per argument
//!    inside `call_callable` — the one path a `Core` member's callback and
//!    `$f(1)` both take, so neither caller is left holding it. A site whose
//!    callee carries `rule:types/callable-signature`'s written signature was
//!    proven where it was written and pays nothing per argument, which is the
//!    only difference between the two. Neither half of `inout $x` is
//!    a gap: an anonymous function *capturing* an enclosing `inout $x` parameter takes
//!    § 2's by-value snapshot of the cell — one [`ir::InstKind::RefLoad`] where
//!    it is written, at the pointee type, retained like any other captured
//!    value, which is what lets the callable outlive the call that staged the
//!    cell — and an `inout $x` parameter on the anonymous function *itself* is `E0493`,
//!    `callable` carrying no parameter list for a call site to stage a cell
//!    against.
//! 7. **`decimal` lowers, and so does `<=>`.** `rule:types/decimal`'s scalar
//!    has a representation — [`ty::Ty::Decimal`], the same register pair
//!    [`ty::Ty::Tagged`] travels in, whose own doc comment owns the decision —
//!    and every row of that ADR's §§ 3-4 is an [`ir::Helper`]: the
//!    arithmetic operators, negation, the comparisons that
//!    [`lower::Lowering::lower_decimal_binary`] rewrites into the rest,
//!    truthiness, and both directions of every conversion. The spaceship
//!    operator answers at each of them: [`ir::Helper::DecimalCmp`] for a
//!    `decimal` pair, [`ir::BinOp::Cmp`] for a matched scalar one,
//!    [`ir::Helper::NumericCmp`] for a mixed numeric one, and
//!    `rule:classes/comparable`'s `compareTo` call for two objects. `**` over
//!    a `decimal` base is not a row at all: `rule:types/arithmetic` makes it a
//!    compile error, and `nvs_types` reports it.
//! 8. **A compound assignment is its binary form plus a staged target, so it
//!    gains an operator exactly when that form does.**
//!    [`lower::Lowering::lower_compound_assignment`] rewrites
//!    `$x op= e` into the `$x = $x op e` it means, so an operator gains its
//!    compound form exactly when its binary form lowers (`.=` on a plain
//!    `string` local is the one exception, and it takes
//!    [`ir::InstKind::StrAppend`] instead). The bitwise operators come that
//!    way rather than one at a time — `&`, `|`, `^`, `<<`, `>>` and unary `~`
//!    have their [`ir::BinOp`]/[`ir::UnOp`] variants, so `&=`, `|=`, `^=`,
//!    `<<=` and `>>=` cost nothing — and `**=` comes with
//!    [`ir::BinOp::Pow`], whose own doc comment states the one thing that
//!    operator does not share with the rest: an integer pair is a loop and a
//!    `float` one is a call. `$x++` and
//!    `--$x` lower through that same rewrite, because the target's **address**
//!    is computed before the rewrite is built
//!    ([`lower::Lowering::stage_target_address`]): staging is what makes a
//!    target re-readable, and it is also what gives the implicit `1` a
//!    representation to be emitted at, that `1` having no source span to
//!    build an [`nvs_syntax::ast::ExprKind::Int`] from. So
//!    `Box::make()->count += 1` calls `make()` once, and
//!    [`lower::Lowering::lower_read_modify_write`]'s own assertion has no
//!    reachable target left at all — its doc comment carries that proof, and
//!    `tests/conformance/lang/a-compound-assignments-target-is-evaluated-once.nvst`
//!    the observable half.
//!
//!    The same staging reaches one level further down. An element write whose
//!    root is a *property* would otherwise lower that property's receiver
//!    twice, once to read the array and once in
//!    [`lower::Lowering::write_back_array`] to store the separated copy back,
//!    so `$b->self()->rows["k"] = "z"` and
//!    `$b->self()->grid["r"]["k"] .= "b"` would run `self()` more often than
//!    PHP's once. [`lower::Lowering::lower_store`]'s element arm stages the
//!    root's address itself, and `stage_address_of` descends a property or
//!    element level rather than staging the level's *value*, which is what
//!    leaves the slot visible to the write-back at all. Pinned by
//!    `tests/conformance/lang/an-element-writes-holder-is-evaluated-once.nvst`.
//! 9. **The environment is one flat, function-wide map**, which is the whole of
//!    the scoping rule rather than a simplification of it: a binding is
//!    declared once, declaration is function-scoped as in PHP, and there is no
//!    shadowing (`rule:types/declaration`), so a nested block's declaration
//!    *is* the enclosing frame's. [`lower::Lowering::declared_tys`] is where a
//!    declaration with no value to bind still records its representation.
//! 10. **`rule:expressions/one-equality-operator` is built, and a
//!     cross-representation pair splits between arithmetic and ordering.**
//!     Every row of §§ 2, 3 and 5 lowers: `===`/`!==` do
//!     not lex, `== null` takes
//!     [`lower::Lowering::lower_null_identity`]'s tag test, a same-
//!     representation pair is [`ir::BinOp::Eq`]/[`ir::BinOp::NotEq`], a
//!     `string`, `array` or `object` pair is that `BinOp` with the row's own
//!     comparison chosen in `nvs-codegen`, a `mixed` or union operand is § 5's
//!     [`ir::Helper::Identical`], and § 2's numeric row — the one pairing
//!     whose two operands hold two *representations* — is
//!     [`ir::Helper::NumericEq`], settled in
//!     [`lower::Lowering::lower_binary`] rather than in `nvs-codegen`, whose
//!     "a `BinOp` has one representation" invariant therefore still holds.
//!     The disjoint-operand refusal of § 2 is `nvs_types`' half, not this
//!     crate's.
//!
//!     The same mismatch under a *different* operator lowers too, and it
//!     splits rather than following equality — which is
//!     `rule:types/arithmetic`'s own
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
//!     conversion carrying `rule:errors/propagation`'s error edge does not belong in one.
//!
//! 11. **`rule:types/single-value-types`, `rule:types/conversion` and `rule:types/conversion`'s scalar rows all run
//!     whole, a `mixed` source included, and so do § 2's
//!     *non-scalar* rows.** A union whose members all erase to one representation
//!     is that representation ([`lower::erase_checked_ty`]), so `"a"|"b"` is a
//!     `Ty::Str`, `1|2` a `Ty::Int` and `Mode::Read|Mode::Write` the enum's
//!     own tag rather than a `Ty::Tagged`, and § 4's
//!     checked row runs the membership test
//!     [`lower::Lowering::lower_single_value_membership`] emits — a comparison per
//!     member, throwing through [`ir::Helper::SingleValueMismatch`] with the
//!     accepted set named. § 3's enum-case subset is in that set: every
//!     lowering entry point takes the run's `nvs_types::EnumTable` (handed
//!     back by `nvs_types::check_program` rather than rebuilt, so `rule:enums/declaration`/§ 2's declaration errors are not reported twice), which is where a
//!     case's constant lives — [`nvs_types::ExprInfo::EnumCase`] carries one only for
//!     a case written as an *expression*, and a case named in a **type** has
//!     no expression to record one against. An enum operand is reinterpreted
//!     to its backing integer for the chain, because `nvs-codegen` lowers
//!     `BinOp::Eq` over `Ty::Int`/`Ty::Uint` and not over `Ty::Enum`.
//!
//!     `rule:types/conversion`'s other direction runs too, so that section is whole for
//!     a statically typed operand. `$n as Mode` is one free
//!     [`ir::InstKind::Reinterpret`] — an enum is a tag over its backing
//!     integer — in front of which
//!     [`lower::Lowering::lower_single_value_membership`] emits the same chain,
//!     built from **every** case of the declaration
//!     ([`lower::convert::whole_enum_set`], sorted by the case's constant because the
//!     table behind it is a hash map). An operand that is not already the
//!     backing scalar is converted to it by `rule:types/conversion`'s own rows first, by
//!     recursion inside [`lower::Lowering::convert`] rather than a row per
//!     source, so `$f as Rank` and `$s as Rank` each throw naming whichever
//!     of the two steps failed. An operand already at the enum's own
//!     representation is skipped entirely: `nvs_types` refuses a conversion
//!     between two *different* enums, so it names a case by construction.
//!
//!     A [`ty::Ty::Tagged`] source is one
//!     helper per *target*, chosen by the operand's runtime tag because
//!     nothing static names a row — [`ir::Helper::TaggedAsString`],
//!     [`ir::Helper::ToDecimal`], and [`ir::Helper::TaggedToInt`] with its
//!     unsigned and `float` twins, each throwing exactly where `rule:expressions/nullable-conversion`'s
//!     [`ir::Helper::ToIntOrNull`] answers `null` over the same rows in
//!     `nvs_runtime`. So `$any as int` runs, and `$any as Mode` and
//!     `$any as Mode::Read|Mode::Write` run through it: the recursion above
//!     converts the operand to the enum's backing scalar first, and the
//!     membership chain is unchanged.
//!
//!     `rule:types/conversion`'s `string` ↔ `bytes` pair runs too, and it is the one
//!     conversion whose two directions are lowered by different mechanisms:
//!     `string as bytes` is total and free, so it is an
//!     [`ir::InstKind::Reinterpret`] over the same allocation and emits no
//!     call, while `bytes as string` validates UTF-8 through
//!     [`ir::Helper::BytesToString`] and throws rather than substituting. An
//!     operand whose static type names neither side of that pair takes it
//!     from its runtime tag instead ([`ir::Helper::TaggedToBytes`]), which is
//!     the only shape of `as bytes` that reaches a call at all.
//!
//!     **Nothing panics.** Every operand/target pair naming no row of
//!     `rule:types/conversion`'s closed table is `E0708` where it is written
//!     (`nvs_types::expr::operators`' `reject_unconvertible`), every object
//!     target with no class to test against is `E0711` beside it, and
//!     `rule:core-classes/html-auto-escape`'s
//!     `string as Core\Html\Markup` is
//!     `lower::Lowering::lower_markup_lift`. That one is a *rule* rather than
//!     a test: the operand is a string literal in the source or it is `E0417`, so what the
//!     lowering does is **build** the carrier rather than check anything, and
//!     it is one `ir::InstKind::CoreCall` on a symbol `nvs-stdlib` owns
//!     because a `Helper` is a symbol `nvs-runtime` exports and that crate
//!     cannot reach a `Core` class's layout.
//!
//!     `array<T> as array<U>` is a **walk**
//!     rather than a row: what decides it is the target's element type, which
//!     [`ty::Ty::Array`] has erased, so `lower::Lowering::lower_array_restamp`
//!     reads it off the annotation and hands
//!     `lower::array_element_tags`' word — one tag nibble per level of `U`,
//!     the same four bits a callable parameter's entry check compares — to
//!     [`ir::Helper::ToArrayOf`], which walks the elements against it.
//!     [`ir::Helper::ToArrayOfOrNull`] is `rule:expressions/nullable-conversion`'s spelling of the same
//!     walk, out of one implementation. The buffer is not copied: an Novis
//!     array is copy-on-write, so the result is the operand's own allocation
//!     under one more reference and `rule:types/arrays`'s invariance is bought with
//!     tag tests rather than with bytes moved. An element type a tag cannot
//!     decide — a class, an enum, a single-value type, a union — is `E0711` where
//!     it is written (`reject_uncheckable_element_type`), which is the one
//!     home of that roster.
//!
//!     A [`ty::Ty::Tagged`] operand converted to an *object* —
//!     `$m as Plain` over a `mixed`, `rule:types/unions-and-mixed`'s checked way out
//!     of the one unchecked position — wants no helper.
//!     [`ir::InstKind::ClassTest`] already takes a tagged subject and already
//!     answers `false` for a tag that is not an object, so
//!     `lower::Lowering::lower_checked_downcast` is that test, a
//!     [`ir::Terminator::Throw`] on the false edge and one free
//!     [`ir::InstKind::Untag`] on the true one. A helper could not have
//!     carried it anyway: helper arguments are stored as
//!     `nvs_runtime::Value`s, and a class descriptor is not one. A `Core`
//!     class with instances is the same test against the descriptor the
//!     process publishes (`lower::anon_fn::declared_class` names which `Core`
//!     classes have one), and a shape is
//!     `lower::Lowering::lower_shape_conversion`'s field walk on the same two
//!     edges. The object targets left name nothing to test against — plain
//!     `object`, a `callable`, a `Core` namespace class, a shape whose fields
//!     carry a qualifier — and `nvs_types` refuses those from a non-object
//!     operand where they are written (`E0711`). The
//!     *statically* typed downcasts are in neither list:
//!     `object as Plain` and `Comparable as Cell` are one representation on
//!     both sides, so `rule:types/erased-member-access` leaves the check to the member access.
//!
//!     `null as string` is a row — the empty string, the same answer
//!     `concat_operand` gives that value.
//!
//!     A statically settled operand needs no check, since
//!     `nvs_types` refuses `E0470` before lowering ever sees it. `as ?"a"`
//!     (`rule:expressions/nullable-conversion`'s non-throwing form) runs the
//!     same chain with a miss arm that yields `null`, which is the one merge
//!     block the throwing form does not need. The `?` is found wherever the
//!     annotation writes it — on the whole of it as `?("a"|"b")`, on a union
//!     member as `?"a"|"b"`, or spelled out as `"a"|"b"|null` — and all three
//!     run that one chain, because the checker interns them as a single flat
//!     union and the target is what is left once `null` is dropped.
//!
//! # Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-ir/src/lib.rs` lists them.

pub mod ids;
pub mod ir;
pub mod lower;
pub mod print;
pub mod ty;

pub use ir::{Function, Program};
/// An attached attribute's site and its folded payload, re-exported on
/// [`ClassConstant`]'s terms: [`ir::Class::attributes`] is a straight copy of
/// the front end's roster and `nvs-codegen` has to be able to name what it
/// holds.
pub use nvs_types::ClassAttribute;
/// A class constant's declaration and its folded value, re-exported because
/// [`ir::Class::constants`] is a straight copy of the front end's roster and
/// `nvs-codegen` — which reads that field but deliberately does not depend on
/// `nvs-types` — has to be able to name what it holds.
pub use nvs_types::ClassConstant;
pub use nvs_types::consts::ConstValue;
/// One row of [`ir::Class::methods`], re-exported on [`ClassConstant`]'s terms
/// and for its reason: that field is a straight copy of the front end's roster
/// and `nvs-codegen` keeps its own copy of it under the same name.
pub use nvs_types::layout::MethodEntry;
pub use ty::Ty;

use nvs_diagnostics::{SourceFile, Span};

pub(crate) fn span_text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

/// Strips a variable's leading `$` sigil, if present — the same idiom
/// `nvs-types` uses.
pub(crate) fn strip_sigil(s: &str) -> &str {
    s.strip_prefix('$').unwrap_or(s)
}
