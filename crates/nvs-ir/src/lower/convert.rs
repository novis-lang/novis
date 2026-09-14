//! `rule:types/conversion`'s conversion table — the implicit widenings `convert` applies at
//! a binding, and the explicit `as` the § 2 grid decides.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory under the
//! rule [`super::expr`]'s own header states: the methods are `pub(crate)`, so they
//! reach across these modules and no further.
//!
//! The closed-set machinery at the bottom — [`AcceptedSet`], `closed_literal_set`
//! and the two membership lowerings — is here rather than beside the operators
//! because its only callers are `lower_conversion` and `lower_checked_downcast`:
//! it exists to answer "does this operand's type name a set the target tests
//! against", which is § 2's question and no one else's.

use super::*;

impl<'a> Lowering<'a> {
    /// Lowers one `expr as T` — `rule:types/conversion`'s conversion table, plus
    /// `rule:types/conversion`'s two enum rows.
    ///
    /// Each row is one of these shapes:
    ///
    /// * **Free.** The two representations are identical, so nothing runs. A
    ///   conversion to the same representation is the operand itself; an enum
    ///   to its own backing `int`/`uint` is an [`InstKind::Reinterpret`],
    ///   which `rule:types/conversion` spells out as "total, free ... same
    ///   representation, reinterpreted." `rule:types/conversion`'s `string as bytes` is
    ///   the third one, free for the same reason: one `NvsStr` allocation
    ///   under two tags, minus the UTF-8 promise.
    /// * **Total.** A scalar to `string` reuses the same [`Helper`]
    ///   conversions `.` concatenation already goes through
    ///   ([`Self::concat_operand`]), and any value to `bool` reuses `rule:expressions/truthy-positions`'s
    ///   truthy table ([`Self::truthy_convert`]) — `as bool` is the explicit
    ///   spelling of exactly the test a condition applies implicitly, so
    ///   giving it a second table would be two answers to one question.
    /// * **Widening.** The target admits more than one runtime shape and is
    ///   therefore [`Ty::Tagged`], so the value travels unchanged under a tag
    ///   — one [`InstKind::Tag`], free in the same sense the free rows are.
    /// * **Checked.** `int` ↔ `uint`, `float` → an integer, `string` → a
    ///   number and `rule:types/conversion`'s `bytes as string` each go through a
    ///   [`Helper`] that either produces the value or throws, emitted through
    ///   [`Self::emit_fallible`] so it carries `rule:errors/propagation`'s error edge like any
    ///   other call.
    /// * **Into an enum.** `rule:types/conversion`'s other direction is row 1 run
    ///   backwards: the operand is converted to the enum's *backing* scalar
    ///   through whichever row above applies, and a free
    ///   [`InstKind::Reinterpret`] puts the tag back on.
    ///
    /// That last row does **not** emit the section's "throws on a value no
    /// case names" itself, and cannot: the case set lives on the enum's
    /// declaration, which [`Ty::Enum`] has already erased to a backing type by
    /// the time this runs. [`Self::lower_conversion`] emits it instead — the
    /// same membership chain `rule:types/literal-types`'s closed set gets, built from every
    /// case of the declaration ([`Self::closed_literal_set`]) — and it is this
    /// function's only caller, so the two halves cannot come apart.
    ///
    /// `operand` is the un-lowered source expression, used only to decide
    /// whether a refcounted operand this conversion consumed was borrowed
    /// storage or a fresh value nothing else will release — the same
    /// [`is_aliasing_read`] judgment [`Self::concat_operand`]'s caller makes.
    pub(crate) fn convert(
        &mut self,
        v: ValueId,
        from: Ty,
        to: Ty,
        operand: &Expr,
        env: &mut Env,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        if from == to {
            // Nothing runs, but the *ownership* still has to come out right:
            // every consumer of a conversion expression reads `is_aliasing_read`
            // off the `as` node, which reports it as a fresh value the consumer
            // owns — so a free row that hands back borrowed storage gives the
            // local, the argument or the returned value a reference nobody
            // took, and the second release of the pair corrupts the heap. One
            // retain makes the free row honour the contract every other row
            // already does. `$s as string` is one spelling of it, and
            // `rule:types/literal-types`'s erasure makes `$s as "a"|"b"` another.
            if to.is_refcounted() && self.aliasing_read(operand) {
                self.emit_retain(cur, v);
            }
            return (v, to);
        }
        match (from, to) {
            // `rule:types/conversion`, row 1 — an enum to its own underlying type.
            (Ty::Enum(EnumRepr::Int), Ty::Int) | (Ty::Enum(EnumRepr::Uint), Ty::Uint) => {
                self.emit(cur, to, InstKind::Reinterpret { operand: v })
            }
            // `rule:types/conversion`, row 2 — the underlying type back into the enum,
            // and free for the same reason row 1 is: `Ty::Enum` is a
            // zero-byte tag over that integer, so the tag costs one
            // reinterpret and no test.
            //
            // An operand that is not already the backing scalar is converted
            // to it by the rows below *first*, by recursion rather than by a
            // row per source: `$f as Rank` is `rule:types/conversion`'s checked
            // `float → int` and then this, which is the same two steps the
            // author wrote and keeps every one of those rows' throw messages
            // naming the conversion that actually failed. A `Ty::Tagged`
            // operand is one of those sources — `$any as Mode` is
            // `Helper::TaggedToInt` and then this — so `mixed` reaches an enum
            // through the same two steps every other source does.
            //
            // The value is not tested against the declaration's cases here;
            // see this function's own doc comment for where that happens and
            // why it cannot happen at this point.
            (_, Ty::Enum(repr)) => {
                let backing = match repr {
                    EnumRepr::Int => Ty::Int,
                    EnumRepr::Uint => Ty::Uint,
                };
                let (backed, _) = self.convert(v, from, backing, operand, env, cur);
                self.emit(cur, to, InstKind::Reinterpret { operand: backed })
            }
            // `rule:types/conversion`'s total row: `string as bytes` is free, because a
            // `bytes` *is* the `string`'s allocation minus the UTF-8 promise
            // (`Ty::Bytes`, and `nvs_runtime::Value::bytes`). Valid UTF-8 is
            // already a valid byte sequence, so there is nothing to check and
            // nothing to copy — one `Reinterpret`, exactly as `rule:types/conversion`'s
            // enum row above, and the tag only differs where a `Ty::Tagged`
            // value is built.
            //
            // The ownership is the `from == to` branch's, for its reason: a
            // consumer reads `is_aliasing_read` off the `as` node and owns
            // what it gets, so borrowed storage handed straight back needs the
            // one retain that makes this row honour the same contract every
            // helper row does.
            (Ty::Str, Ty::Bytes) => {
                if self.aliasing_read(operand) {
                    self.emit_retain(cur, v);
                }
                self.emit(cur, to, InstKind::Reinterpret { operand: v })
            }
            // **Widening.** The target admits more than one runtime shape, so
            // it is `Ty::Tagged` and the value keeps the payload it already
            // has under a tag — one `InstKind::Tag`, the same instruction
            // `Self::coerce` emits where a *declaration* is the wider side.
            // `rule:types/literal-types`'s heterogeneous set is the shape that needs it
            // (`$s as 1|"a"`: the set is closed, its members share no one
            // representation, so the whole target erases to a tagged value
            // and the membership test below runs on tags), and `as mixed` is
            // the same row written plainly.
            //
            // The ownership is the `from == to` branch's, for its reason:
            // `Tag` transfers its operand's reference to its result, so
            // borrowed storage handed through it owes the one retain that
            // makes this row honour the contract every helper row does.
            (_, Ty::Tagged) => {
                if from.is_refcounted() && self.aliasing_read(operand) {
                    self.emit_retain(cur, v);
                }
                self.emit(cur, to, InstKind::Tag { operand: v })
            }
            (_, Ty::Bool) => {
                let b = self.truthy_convert(v, from, cur, env);
                if from.is_refcounted() && !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                (b, Ty::Bool)
            }
            (Ty::Bool | Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal, Ty::Str) => {
                let helper = match from {
                    Ty::Bool => Helper::BoolToString,
                    Ty::Int => Helper::IntToString,
                    Ty::Uint => Helper::UintToString,
                    // `rule:types/conversion`'s row: total, and scale-preserving.
                    Ty::Decimal => Helper::DecimalToString,
                    _ => Helper::FloatToString,
                };
                self.emit_fallible(
                    cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                    env,
                )
            }
            // `rule:types/class-reference`'s `class<T>` → `string` row — the descriptor's own
            // fully qualified name, through the same [`Helper::ClassDescName`]
            // `$obj::class` reads one with. No conversion of the operand: a
            // [`Ty::ClassDesc`] slot is already spelled the way that helper
            // reads it (`nvs_codegen::ty::tag_of`), and the descriptor is not
            // refcounted, so there is nothing to retain or release around it.
            (Ty::ClassDesc, Ty::Str) => self.emit_fallible(
                cur,
                Ty::Str,
                InstKind::HelperCall {
                    helper: Helper::ClassDescName,
                    args: vec![v],
                },
                env,
            ),
            // `rule:types/conversion`'s "anything → `string`" row at `null`, and the same
            // answer `Self::concat_operand` gives the same value: PHP renders
            // `null` as the empty string, and a `?string` holding one already
            // does through `Helper::TaggedToString`. The static case rendering
            // anything else — or being refused — would diverge from both. The
            // operand has already been lowered for its effects and `Ty::Null`
            // is not refcounted, so there is nothing here to release.
            (Ty::Null, Ty::Str) => self.emit(cur, Ty::Str, InstKind::ConstStr(String::new())),
            // `rule:classes/stringable`'s row: `as string` is the explicit spelling of the
            // same implicit conversion `.` and `echo` apply, so it goes
            // through the same resolved `toString()` rather than a second
            // answer of its own (`Self::lower_to_string_call`). The receiver's own
            // ownership is settled there too, so nothing is released here.
            //
            // The `None` half is `Self::concat_operand`'s, verbatim and for its
            // reason: an operand whose static type named no class to resolve
            // against — the erased `object` of `rule:types/erased-member-access`, or a `Core`-owned
            // class — is decided by its *runtime* class instead, which is what
            // `nvs_runtime::stringify` is. Answering that here and something
            // else at `echo` would make one value render two ways depending on
            // which spelling read it.
            (Ty::Object, Ty::Str) => match self.lower_to_string_call(operand, v, env, cur) {
                Some(s) => (s, Ty::Str),
                None => {
                    let out = self.emit_fallible(
                        cur,
                        Ty::Str,
                        InstKind::HelperCall {
                            helper: Helper::TaggedToString,
                            args: vec![v],
                        },
                        env,
                    );
                    if !self.aliasing_read(operand) {
                        self.emit_release(cur, v);
                    }
                    out
                }
            },
            // The same rows again, from a union operand — one fallible helper
            // picking by runtime tag, shared verbatim with `.` and `echo`
            // (`Self::concat_operand`). See `Helper::TaggedToString`.
            (Ty::Tagged, Ty::Str) => {
                let out = self.emit_fallible(
                    cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper: Helper::TaggedToString,
                        args: vec![v],
                    },
                    env,
                );
                if !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                out
            }
            // `rule:types/conversion`'s checked rows. Each either produces the value or
            // throws, so each is a fallible helper carrying `rule:errors/propagation`'s error
            // edge — the same call shape a method call already has. The
            // operand is a scalar in every one of these except the `string`
            // rows, whose operand is released once the helper has read it if
            // nothing else owns it (`Self::concat_operand`'s caller's policy).
            (Ty::Int, Ty::Uint)
            | (Ty::Uint, Ty::Int)
            | (Ty::Int | Ty::Uint, Ty::Float)
            | (Ty::Float, Ty::Int | Ty::Uint)
            | (Ty::Str, Ty::Int | Ty::Uint | Ty::Float)
            // `rule:types/conversion`'s rows. `→ decimal` is one helper for every source
            // (including `Ty::Tagged`, whose row only its runtime tag names),
            // the same "one tag per target" arrangement `Helper::ToIntOrNull`
            // already follows; the three out of `decimal` are per-target, like
            // every other row here.
            | (Ty::Int | Ty::Uint | Ty::Float | Ty::Str | Ty::Tagged, Ty::Decimal)
            | (Ty::Decimal, Ty::Int | Ty::Uint | Ty::Float)
            // `rule:types/unions-and-mixed`'s `mixed`: the three numeric targets, each one
            // helper for every source because only the operand's runtime tag
            // names a row — the same arrangement `Ty::Tagged`'s `string` and
            // `decimal` targets above already use. Each throws where
            // `Helper::ToIntOrNull` answers `null`, over one shared row set in
            // `nvs_runtime`.
            | (Ty::Tagged, Ty::Int | Ty::Uint | Ty::Float) => {
                let helper = match (from, to) {
                    (_, Ty::Decimal) => Helper::ToDecimal,
                    (Ty::Decimal, Ty::Int) => Helper::DecimalToInt,
                    (Ty::Decimal, Ty::Uint) => Helper::DecimalToUint,
                    (Ty::Decimal, _) => Helper::DecimalToFloat,
                    (Ty::Int, Ty::Uint) => Helper::IntToUint,
                    (Ty::Uint, Ty::Int) => Helper::UintToInt,
                    (Ty::Int, _) => Helper::IntToFloat,
                    (Ty::Uint, _) => Helper::UintToFloat,
                    (Ty::Float, Ty::Int) => Helper::FloatToInt,
                    (Ty::Float, _) => Helper::FloatToUint,
                    (Ty::Tagged, Ty::Int) => Helper::TaggedToInt,
                    (Ty::Tagged, Ty::Uint) => Helper::TaggedToUint,
                    (Ty::Tagged, _) => Helper::TaggedToFloat,
                    (_, Ty::Int) => Helper::StrToInt,
                    (_, Ty::Uint) => Helper::StrToUint,
                    _ => Helper::StrToFloat,
                };
                let out = self.emit_fallible(
                    cur,
                    to,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                    env,
                );
                if from.is_refcounted() && !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                out
            }
            // `rule:types/conversion`'s checked row, and the half of that pair that runs
            // anything: the buffer is validated as well-formed UTF-8 and
            // becomes the `string` over the same allocation, or it throws.
            // Never a replacement character and never a truncation, so it is
            // fallible like every other checked row and carries `rule:errors/propagation`'s
            // error edge.
            //
            // Its own operand is refcounted, so it follows the string rows'
            // policy exactly: released once the helper has read it unless a
            // durable slot still owns it.
            (Ty::Bytes, Ty::Str) => {
                let out = self.emit_fallible(
                    cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper: Helper::BytesToString,
                        args: vec![v],
                    },
                    env,
                );
                if !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                out
            }
            // `rule:types/conversion`'s row the other way, from an operand whose static
            // type names no row — a `mixed`, a `?T`, any other union. The
            // statically typed spelling is the free `Reinterpret` above and
            // reaches no helper at all, so this is the only shape of
            // `as bytes` that runs anything: `Helper::TaggedToBytes` reads the
            // tag the value already carries, hands back the same allocation
            // under the other tag for a `string` or a `bytes`, and throws for
            // every tag § 2's table gives no row.
            //
            // Its operand's ownership is the numeric rows' — `Ty::Tagged` may
            // hold a refcounted payload, so it is released once the helper has
            // read it unless a durable slot still owns it.
            (Ty::Tagged, Ty::Bytes) => {
                let out = self.emit_fallible(
                    cur,
                    Ty::Bytes,
                    InstKind::HelperCall {
                        helper: Helper::TaggedToBytes,
                        args: vec![v],
                    },
                    env,
                );
                if !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                out
            }
            // Nothing reaches here: `nvs_types`' `reject_unconvertible` refuses
            // every pair `rule:types/conversion`'s closed table has no row for
            // (`E0708`) and every object target with no class to test against
            // (`E0711`), and the rows whose decision is a *label* `Ty` has
            // erased are arms of `Self::lower_conversion` rather than of this
            // table.
            _ => panic!(
                "nvs-ir lowers `rule:types/conversion`'s scalar conversion rows, `rule:types/conversion`'s `string` ↔ \
                 `bytes` pair, both of `rule:types/conversion`'s enum ones, a `Ty::Tagged` operand into \
                 every scalar target among them and into `bytes`, and every operand into a \
                 tagged target — got `{from:?} as {to:?}`. No row is missing: every other pair \
                 is `E0708` or `E0711` a phase up, so a pair arriving here is a rule that was \
                 admitted and never lowered. Three targets do not reach this table at all, each \
                 because what decides it is a *label* one `Ty` has erased: a declared class is \
                 `Self::lower_checked_downcast`, an `array<U>` is `Self::lower_array_restamp` \
                 and `rule:core-classes/html-auto-escape`'s `Core\\Html\\Markup` is `Self::lower_markup_lift`, all \
                 callers of it rather than rows of it. See the crate docs' known gaps"
            ),
        }
    }
    /// Lowers one `expr as ?T` —
    /// `rule:expressions/nullable-conversion`'s non-throwing form of [`Self::convert`], where `to` is the target
    /// *inside* the `?`.
    ///
    /// One [`InstKind::HelperCall`] per target type, and for the numeric ones
    /// no error edge at all: the helper answers `null` where the throwing row
    /// would throw, so it cannot fail and needs neither [`Self::emit_fallible`]
    /// nor a landing block. The `string` target is the one exception, and for a
    /// reason that is not the conversion's — it may run the operand's own
    /// `toString()`, and *that* can throw. The
    /// result is [`Ty::Tagged`] — the one representation `?T` has
    /// ([`Ty::Tagged`]'s own doc comment) — and every source is one helper,
    /// because the helper dispatches on the operand's runtime tag rather than
    /// on a statically chosen row. That is what makes § 2's "a `null` operand
    /// yields `null`" and § 3's "from `mixed` every target has a checked path"
    /// need no branch here: a [`Ty::Tagged`] operand is already the `Value`
    /// the helper reads.
    ///
    /// Ownership matches the checked rows exactly: a refcounted operand this
    /// conversion consumed is released once the helper has read it, unless a
    /// durable slot still owns it ([`is_aliasing_read`]). Nothing is retained
    /// here either — [`Helper::ToStringOrNull`] and [`Helper::ToBytesOrNull`]
    /// are the two whose result carries a refcounted payload, and each arrives
    /// with the single reference every helper that hands back a buffer owes.
    ///
    /// # Panics
    ///
    /// Panics for a row `rule:expressions/nullable-conversion-availability` calls **available** and this crate has no
    /// `?` helper to run — `$m as ?array<T>`.
    /// Both of that section's *refusals* belong to `nvs_types`, so neither
    /// reaches here: a conversion that cannot fail is `E0709` and a pair
    /// naming no row is `E0708`, both where the conversion is written. The
    /// remaining target is the same missing lowering [`Self::convert`]'s own
    /// catch-all names, minus the targets this form adds — `string` and
    /// `bytes`, each a row that throws in the checked spelling and so needs a
    /// null-answering twin rather than the same helper.
    pub(crate) fn convert_or_null(
        &mut self,
        v: ValueId,
        from: Ty,
        to: Ty,
        operand: &Expr,
        env: &mut Env,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        let helper = match to {
            _ if from == to => panic!(
                "nvs-ir: `{from:?} as ?{to:?}` — one representation on both sides, and \
                 `nvs_types` has refused every pair of that shape whose conversion cannot fail \
                 (`E0709`). What is left is two *different* checked types sharing one \
                 representation: a `?T` whose `T` is itself a union, which erases to one \
                 `Ty::Tagged` the same way `mixed` does. `array<T> as ?array<U>` is not among \
                 them — `Lowering::lower_conversion` takes an array target before this, since \
                 the element type both sides erased is what decides it"
            ),
            Ty::Int => Helper::ToIntOrNull,
            Ty::Uint => Helper::ToUintOrNull,
            Ty::Float => Helper::ToFloatOrNull,
            Ty::Decimal => Helper::ToDecimalOrNull,
            Ty::Str => Helper::ToStringOrNull,
            Ty::Bytes => Helper::ToBytesOrNull,
            other => panic!(
                "nvs-ir lowers `rule:expressions/nullable-conversion`'s `as ?T` for the checked scalar targets and for \
                 `bytes`, and through `Self::lower_nullable_membership` for `rule:types/literal-types`'s literal \
                 and enum-case ones — got `{from:?} as ?{other:?}`. Both of § 3's refusals are \
                 `nvs_types`' now (`E0709` for a row that cannot fail, `E0708` for a pair naming \
                 no row), so what reaches here is a row that exists, can fail, and has no `?` \
                 helper to run it: the object target `Lowering::convert`'s own catch-all already \
                 names. The `array<U>` target had been the other one and is \
                 `Self::lower_array_restamp` now, which answers both spellings out of one walk. \
                 See the crate docs' known gaps"
            ),
        };
        let call = InstKind::HelperCall {
            helper,
            args: vec![v],
        };
        // One row can *throw*, and it is not the conversion failing: a
        // `string` target may run the operand's own `toString()`, whose
        // exception is the *program's* and propagates unchanged — `null` here
        // means "this conversion had no answer" and nothing else
        // (`Helper::ToStringOrNull`). The numeric rows and the `bytes` one
        // cannot fault at all, and still take the same edge, because every
        // helper returns a status and an uncatchable one has to leave the
        // frame swept (`Inst::on_error`).
        let out = self.emit_fallible(cur, Ty::Tagged, call, env);
        if from.is_refcounted() && !self.aliasing_read(operand) {
            self.emit_release(cur, v);
        }
        out
    }
    /// Converts an already-lowered `(v, ty)` pair through `rule:expressions/truthy-positions`'s truthy
    /// table, with no ownership decision attached — see [`Self::truthy_value`]
    /// for the usual "release a fresh, non-aliasing refcounted operand once
    /// its truthy test is done" wrapper every caller but
    /// [`Self::lower_ternary`]'s elvis arm wants; elvis needs the bare
    /// conversion on its own, since its truthy-path *value* is `v` itself
    /// (PHP only evaluates a `?:` condition once) and releasing it here would
    /// use-after-free that reuse.
    ///
    /// `Ty::Bool` passes straight through; `Ty::Int`/`Ty::Uint`/`Ty::Float`/
    /// `Ty::Decimal`/`Ty::Str`/`Ty::Bytes` each convert through their own
    /// [`Helper`] variant; [`Ty::Array`]
    /// converts through [`Helper::ArrayTruthy`] (falsy iff empty, `rule:expressions/truthy-positions`'s
    /// table); and [`Ty::Object`] — a class instance or an enum case — needs
    /// no helper at all, since `rule:enums/truthiness` makes either always truthy: this
    /// folds straight to a fresh [`InstKind::ConstBool`] `true` rather than
    /// emitting a call with nothing to inspect at runtime.
    ///
    /// [`Ty::Tagged`] — a `mixed`, a union, or a `?T` no test narrowed — is
    /// the one row this table does **not** settle here: it converts through
    /// [`Helper::ValueTruthy`], which reads the value's tag and applies
    /// whichever of the rows above it names. That is `rule:expressions/truthy-table`'s own last
    /// line rather than a fallback, and it is why `rule:types/conversion` can make
    /// `mixed` the one unchecked position without a condition being a hole in
    /// it: the question a condition asks has an answer for every tag.
    ///
    /// # Panics
    ///
    /// The table has a row for every representation, so what is left is the
    /// two shapes no program puts in a condition. `Ty::Void` is guarded by
    /// `E0719`: `rule:types/declaration` makes every return type written and
    /// keeps `void`/`never` out of value position, so a call returning nothing
    /// is refused at the condition that wrote it. [`Ty::Ref`] is an engine
    /// invariant rather than a refusal — a cell is only ever read through
    /// [`InstKind::RefLoad`], which yields the referent's own representation,
    /// so no cell is ever the value being converted. [`Ty::Null`] *is* in the
    /// table and is reachable only from the literal `null`: a `?T` is one
    /// [`Ty::Tagged`] slot and takes that row instead.
    pub(crate) fn truthy_convert(
        &mut self,
        v: ValueId,
        ty: Ty,
        cur: BlockId,
        env: &Env,
    ) -> ValueId {
        match ty {
            Ty::Bool => v,
            Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal | Ty::Str | Ty::Bytes => {
                let helper = match ty {
                    Ty::Int => Helper::IntTruthy,
                    Ty::Uint => Helper::UintTruthy,
                    Ty::Float => Helper::FloatTruthy,
                    Ty::Decimal => Helper::DecimalTruthy,
                    Ty::Str => Helper::StrTruthy,
                    Ty::Bytes => Helper::BytesTruthy,
                    Ty::Bool
                    | Ty::Void
                    | Ty::Null
                    | Ty::Object
                    | Ty::Array
                    | Ty::Tagged
                    | Ty::Enum(_)
                    | Ty::ClassDesc
                    | Ty::Ref => {
                        unreachable!("matched above")
                    }
                };
                self.emit_fallible(
                    cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                    env,
                )
                .0
            }
            Ty::Array => {
                self.emit_fallible(
                    cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper: Helper::ArrayTruthy,
                        args: vec![v],
                    },
                    env,
                )
                .0
            }
            // A class instance or an enum case — `rule:enums/truthiness`, always
            // truthy, nothing to inspect at runtime. The enum arm is the whole
            // reason `Ty::Enum` is a representation of its own rather than the
            // backing integer it is made of: `Rank::Bronze` is backed by `0`
            // and is still `true` here, where a plain `int` `0` goes through
            // `Helper::IntTruthy` and comes back `false`.
            Ty::Object | Ty::Enum(_) => self.emit(cur, Ty::Bool, InstKind::ConstBool(true)).0,
            // `rule:expressions/truthy-table`'s first row, reachable only from the *literal*
            // `null` — a `?T` is one `Ty::Tagged` slot and goes through the
            // arm below. `empty(null)`, `!null` and `if (null)` are the three
            // spellings that get here, and nothing about the value needs
            // reading to answer them.
            Ty::Null => self.emit(cur, Ty::Bool, InstKind::ConstBool(false)).0,
            // `rule:enums/truthiness`'s class-instance row read one representation over:
            // a class *reference* is a descriptor, so a `class<T>` is always
            // truthy the way an instance is — and `rule:types/class-reference`'s `?class<T>`
            // is the same row with § 2's first, `null` for the miss. That
            // pairing is exactly the word against zero, since the two answers
            // are one representation here ([`Ty::ClassDesc`], which owns the
            // decision and names every site it obliges).
            Ty::ClassDesc => {
                let word = self.class_desc_word(v, ty, cur);
                let (zero, _) = self.emit(cur, Ty::Int, InstKind::ConstInt(0));
                self.emit(
                    cur,
                    Ty::Bool,
                    InstKind::BinOp {
                        op: BinOp::NotEq,
                        lhs: word,
                        rhs: zero,
                    },
                )
                .0
            }
            // `rule:expressions/truthy-table`'s last row, and the one this table answers at run
            // time rather than at compile time: a `mixed`, a union or a `?T`
            // no test narrowed carries its row in its tag, so the dispatch
            // moves into `Helper::ValueTruthy` and the arms above become the
            // cases where a static type already picked one.
            Ty::Tagged => {
                self.emit_fallible(
                    cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper: Helper::ValueTruthy,
                        args: vec![v],
                    },
                    env,
                )
                .0
            }
            // `rule:types/declaration` keeps `void` out of value position, and
            // a condition is the position an author most often tries it in —
            // `if (log())` over a method declared `: void`. That is `E0719`,
            // raised where it is written, so the row here is the guarantee
            // rather than a conversion.
            Ty::Void => guarded_by!(
                code::E_VOID_IS_NOT_A_CONDITION,
                "nvs-ir reached the truthy conversion over a `void`. A call returning nothing \
                 is refused in every condition where one is written, which is the only way a \
                 value of this representation could arrive"
            ),
            // An engine invariant, not a refusal: a `Ty::Ref` is the *cell* an
            // `inout` binding ties two names to, and nothing converts one
            // because nothing reads one. `InstKind::RefLoad` is the single
            // read of a cell in this IR and it yields the referent's own
            // representation, which is what reaches this table instead.
            Ty::Ref => panic!(
                "nvs-ir: unreachable — a `Ty::Ref` cell reached the truthy conversion; \
                 `InstKind::RefLoad` is the one read of a cell and it yields the referent"
            ),
        }
    }
    /// [`Self::truthy_convert`] plus the ownership half every truthy-tested
    /// position but elvis wants: a refcounted operand (`Ty::Str`/`Ty::Array`)
    /// that isn't [`is_aliasing_read`] — a fresh call/`new`/literal result
    /// whose only use is this truthy test — is released right after it's
    /// read, the same "release a fresh value once its one and only use is
    /// done" precedent [`Self::concat_operand`]'s own caller already sets for
    /// `.` concatenation; an aliasing read (a bare variable, a
    /// compile-time-known property or array-element read) still durably
    /// belongs to whatever slot it came from and needs no release here.
    pub(crate) fn truthy_value(
        &mut self,
        v: ValueId,
        ty: Ty,
        is_alias: bool,
        cur: BlockId,
        env: &Env,
    ) -> ValueId {
        let cond_v = self.truthy_convert(v, ty, cur, env);
        if ty.is_refcounted() && !is_alias {
            self.emit_release(cur, v);
        }
        cond_v
    }
    /// Lowers `cond` — an `if`/`while` condition, or `&&`/`||`'s own operand
    /// (see [`Self::lower_and`]/[`Self::lower_or`]) — through
    /// [`Self::lower_expr`] (so a nested `&&`/`||`/`!`/ternary composes,
    /// e.g. `if ($a && $b)`) and then [`Self::truthy_value`]'s table.
    /// `*cur` is updated to whichever block `cond`'s own evaluation ends in —
    /// unchanged unless `cond` itself needed to branch.
    ///
    /// # Panics
    ///
    /// See [`Self::truthy_convert`]'s own panic doc — the same restriction
    /// applies here.
    pub(crate) fn lower_truthy_cond(
        &mut self,
        cond: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        let (v, ty) = self.lower_expr(cond, None, env, cur);
        let is_alias = self.aliasing_read(cond);
        self.truthy_value(v, ty, is_alias, *cur, env)
    }

    /// `rule:types/conversion`'s `as` — the one conversion spelling. The target
    /// type is resolved by `lower_decl_type`, which reads the checker's
    /// own answer for the annotation, so an enum target/source is
    /// already the right representation by the time `convert` sees it.
    pub(crate) fn lower_conversion(
        &mut self,
        inner: &Expr,
        ty: &Type,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // `rule:expressions/nullable-conversion`'s `as ?T` is read off the *annotation*, before
        // `lower_decl_type` erases it: `?string` and `?int` are both
        // `Ty::Tagged`, so a conversion between them would look like
        // `from == to` — the one shape `Self::convert` answers by
        // doing nothing at all.
        //
        // Every `as ?T` that reaches here is a row of `rule:types/conversion`'s table or
        // one of `rule:types/literal-types`'s types. A **class** target never does: `rule:expressions/nullable-conversion-availability`'s class row is absolute, so `nvs_types` has already refused it
        // with `E0473`, so nothing about a class reaches this function.
        match nullable_target(ty) {
            Some(target) => {
                // `rule:types/class-reference`'s `as ?class<T>`, ahead of everything below:
                // its answer is a `Ty::ClassDesc` rather than the `Ty::Tagged`
                // every other `?T` erases to, so it never reaches
                // `Self::convert_or_null` and needs no `?` helper of its own —
                // [`InstKind::ClassDescIn`]'s miss *is* the `null`.
                if self.class_ref_base(ty).is_some() {
                    return self.lower_class_reference(inner, ty, true, env, cur);
                }
                // No placement here, unlike the arm below: placing a
                // literal at the target would make `3 as ?uint` the
                // `from == to` shape `rule:expressions/nullable-conversion-availability` calls a compile
                // error, which `nvs_types` does not refuse yet, so it
                // would panic where it now converts.
                let (v, from) = self.lower_expr(inner, None, env, cur);
                // `rule:expressions/nullable-conversion-availability`'s one **available** row with no `?` helper of
                // its own, and it needs none: the element walk is the same
                // walk either way, so the `null` is an answer its shared
                // implementation already had. It is taken off the whole `?T`
                // annotation rather than from `lower_decl_type(target)`
                // because a `?array<U>` reaches `convert_or_null` down two
                // different paths below, and this is the one point above both.
                if let Some(tags) = self
                    .exprs
                    .declared_ty(ty.span)
                    .and_then(|id| super::array_element_tags(id, self.checked_types))
                {
                    return self.lower_array_restamp(v, from, tags, inner, true, env, cur);
                }
                // `rule:types/property-key`'s two run-time rows under `rule:expressions/nullable-conversion-availability`'s sugar:
                // the same set and the same chain the checked form below gets,
                // with the miss answering `null` where that one throws. Ahead
                // of the atom walk, which cannot answer it — `property<T>` is
                // one atom and not a closed set of literal types, so
                // `Self::closed_set_of_atoms` reports no set and the operand
                // would fall through to the free `Ty::Str` → `Ty::Str` row with
                // nothing checked at all.
                if let Some(accepted) = self.property_key_set(ty) {
                    return self.lower_nullable_membership(
                        v,
                        from,
                        Ty::Str,
                        &accepted,
                        inner,
                        ty.span,
                        env,
                        cur,
                    );
                }
                // `rule:expressions/nullable-conversion-availability` row 2 — a literal or enum-case target, the
                // "non-throwing twin" of the checked conversion. The target
                // is `T|null` minus `null`, which is the one place it still
                // exists: see `Self::nullable_target_atoms` for why the `T`
                // node's own span answers nothing.
                let Some(atoms) = self.nullable_target_atoms(ty) else {
                    let to = lower_decl_type(target, self.exprs, self.checked_types);
                    return self.convert_or_null(v, from, to, inner, env, *cur);
                };
                let to = shared_repr(&atoms, self.checked_types);
                let Some(accepted) = self.closed_set_of_atoms(&atoms, None, from) else {
                    return self.convert_or_null(v, from, to, inner, env, *cur);
                };
                self.lower_nullable_membership(v, from, to, &accepted, inner, ty.span, env, cur)
            }
            None => {
                let to = lower_decl_type(ty, self.exprs, self.checked_types);
                // `rule:types/class-reference`'s two rows into a `class<T>`, and the
                // compile-time fold of a written-out `Foo::class` under the
                // same roof — see [`Self::lower_class_reference`].
                //
                // Ahead of the literal placement below, which no operand of
                // this row can be, and ahead of [`Self::convert`], which must
                // never see the pair: a descriptor on both sides is one
                // representation, so its free `from == to` row would hand a
                // `class<Animal>` through under a `class<Dog>` declaration
                // with nothing checked at all.
                if to == Ty::ClassDesc {
                    return self.lower_class_reference(inner, ty, false, env, cur);
                }
                // `rule:types/numeric-literal-placement`: `expr as T` is itself a *placing*
                // position, so a numeric literal written directly
                // under one takes `T` as its target rather than being
                // typed first and converted afterwards. Mirrors
                // `nvs_types::expr::check_expr`'s own `Conversion`
                // arm, operand shape included — without it
                // `19.99 as decimal` would round-trip through an
                // `f64` and lose everything past ~17 digits.
                //
                // An enum target places at its *backing* scalar rather than
                // at `Ty::Enum` itself, because `rule:types/conversion`'s row 2 is
                // written on that integer: without it `5 as Rank` over a
                // `uint`-backed enum would lower its literal to the `Ty::Int`
                // an unplaced one defaults to and then need a checked
                // `int → uint` to undo it.
                let placed =
                    matches!(inner.kind, ExprKind::Int(_) | ExprKind::Float(_)).then(|| match to {
                        Ty::Enum(EnumRepr::Int) => Ty::Int,
                        Ty::Enum(EnumRepr::Uint) => Ty::Uint,
                        other => other,
                    });
                let (v, from) = self.lower_expr(inner, placed, env, cur);
                // `rule:types/property-key`'s `string` and `property<U>` rows. Both operands
                // are already a `Ty::Str` here — a key *is* a name, which is the
                // whole of `rule:types/property-key`'s representation — so `Self::convert`
                // would answer this pair by its free `from == to` row and check
                // nothing, exactly as it would hand a `class<Animal>` through a
                // `class<Dog>` two arms above. The conversion's entire content
                // is therefore the membership test, and its value is the
                // operand it just proved.
                //
                // It owes the same retain that free row owes, and for the same
                // reason stated there: every consumer reads `is_aliasing_read`
                // off the `as` node and treats the result as a fresh value it
                // owns, so handing back borrowed storage gives it a reference
                // nobody took and the second release of the pair corrupts the
                // heap. The chain itself releases only the name constants it
                // makes. The retain goes after it because
                // `Self::lower_literal_membership` moves `*cur` to the block
                // the hit arm lands in, which is the one block the value leaves
                // through.
                if let Some(accepted) = self.property_key_set(ty) {
                    self.lower_literal_membership(v, from, &accepted, ty.span, env, cur, None);
                    if Ty::Str.is_refcounted() && self.aliasing_read(inner) {
                        self.emit_retain(*cur, v);
                    }
                    return (v, Ty::Str);
                }
                // `rule:types/unions-and-mixed`'s checked way out of `mixed`, and the one row
                // of this operator whose test is a *class* rather than a tag.
                // It is here rather than in `Self::convert` because it
                // branches, and that function's `cur` is by value — the same
                // reason `Self::lower_literal_membership` is a caller of this
                // arm rather than a row of the table.
                if from == Ty::Tagged
                    && to == Ty::Object
                    && let Some(class) =
                        super::closure::declared_class(ty, self.exprs, self.checked_types)
                {
                    return self.lower_checked_downcast(v, &class, inner, ty.span, env, cur);
                }
                // `rule:core-classes/html-auto-escape`'s `"<b>" as Core\Html\Markup`, and it is here
                // for the downcast's reason exactly: both targets erase to
                // `Ty::Object`, so only the written name tells a class that is
                // *tested* from the one class that is *built*. See
                // [`Self::lower_markup_lift`] for which of the two this is.
                if to == Ty::Object && self.markup_target(ty) {
                    return self.lower_markup_lift(v, inner, env, cur);
                }
                // `rule:types/conversion`'s `array<T> as array<U>` row, and it is here
                // for the reason the downcast above is: what decides it is the
                // *element* type, which `Ty::Array` has erased. Both sides of
                // `array<int> as array<string>` are one representation, so
                // `Self::convert` would take its free `from == to` row and
                // hand the `int`s through under the other declaration.
                //
                // A `None` here is that free row and belongs to it: the one
                // element type `nvs_types` lets through undescribed is a class
                // in `array<Foo> as array<Foo>`, where the two sides are the
                // identical type and there is nothing to check.
                if let Some(tags) = self
                    .exprs
                    .declared_ty(ty.span)
                    .and_then(|id| super::array_element_tags(id, self.checked_types))
                {
                    return self.lower_array_restamp(v, from, tags, inner, false, env, cur);
                }
                let Some(accepted) = self.closed_literal_set(ty, inner, from) else {
                    return self.convert(v, from, to, inner, env, *cur);
                };
                // `rule:types/literal-types`'s membership test, on whichever side of
                // the base conversion still holds the value the author
                // wrote. A `Ty::Tagged` operand into a **literal** set is
                // tested **first**, against its own runtime tag:
                // converting one to the base would run
                // `Helper::TaggedToString`, which turns a `mixed` holding
                // `1` into `"1"` and would let it satisfy a set naming
                // `"1"` — exactly the coercion § 4's "throws unless the
                // value equals one of the named literals" refuses. Every
                // other operand is converted first instead, so the
                // comparison is over one representation and stays a
                // `BinOp::Eq` machine compare rather than
                // `nvs-codegen`'s refusal of a mismatched pair.
                //
                // An **enum** target is deliberately not in that first
                // case, whether the set is § 3's named subset or the whole
                // declaration. `rule:types/conversion` words the `mixed → EnumName`
                // row as "exactly the shape `as uint` already has for
                // untrusted input", and its own example converts
                // `Core\Request::query('status')` — a string at run time —
                // into a case whose value is an integer. So the base
                // conversion runs first there, which is `rule:types/conversion`'s
                // whole-string numeric row and not a coercion of its own,
                // and the chain then compares two integers. The `"1"`
                // hazard above cannot arise: an enum's base is never
                // `string`.
                if from == Ty::Tagged && !matches!(to, Ty::Enum(_)) {
                    self.lower_literal_membership(v, from, &accepted, ty.span, env, cur, None);
                    // A `bool` set is the one target with no conversion left
                    // to run. Every other base is reached by a row that
                    // happens to be an identity once the test above has
                    // passed (`Helper::TaggedToString` over a tag proved to
                    // be a string), but `as bool`'s row is `rule:expressions/truthy-positions`'s truthy
                    // table — and running it here would answer `true` for a
                    // `mixed` holding `1` that the test has just refused, or
                    // rather could not, since `Helper::Identical` compared
                    // tags first. The test *is* the check, so what is left is
                    // one unchecked `Untag`: the same shape
                    // `Self::untag_narrowed` emits over a tag `nvs_types`
                    // proved, on a tag this chain proved instead.
                    if to == Ty::Bool {
                        let (out, _) = self.emit(*cur, Ty::Bool, InstKind::Untag { operand: v });
                        // The proof says the payload is a `bool` and so owns
                        // nothing — but a fresh tagged operand is released
                        // anyway, exactly as every row of `Self::convert`
                        // releases one, because the consumer of a `Ty::Bool`
                        // never will and a release over a tag that owns
                        // nothing is a no-op the runtime already handles.
                        if !self.aliasing_read(inner) {
                            self.emit_release(*cur, v);
                        }
                        return (out, Ty::Bool);
                    }
                    return self.convert(v, from, to, inner, env, *cur);
                }
                let (converted, converted_ty) = self.convert(v, from, to, inner, env, *cur);
                self.lower_literal_membership(
                    converted,
                    converted_ty,
                    &accepted,
                    ty.span,
                    env,
                    cur,
                    None,
                );
                (converted, converted_ty)
            }
        }
    }

    /// `rule:types/class-reference`'s two rows into a `class<T>` — `as` being a class reference's only
    /// source, this function is the only place a [`Ty::ClassDesc`] a program
    /// can name comes from.
    ///
    /// **A written-out `Foo::class` never reaches the run time.** § 2 decides
    /// `Dog::class as class<Animal>` where it stands — the checker having
    /// already refused the pair that is not a widening — so it folds to the one
    /// [`InstKind::ClassDescConst`] `Foo::bar()` already bakes, and the factory
    /// shape the ADR calls the common case pays nothing at all.
    ///
    /// Everything else is [`InstKind::ClassDescIn`], whose own doc comment owns
    /// how the two dynamic rows are answered and what they cost. This function
    /// owns only what happens to its **null**: § 2's rows are checked rows of
    /// `rule:types/conversion`'s grid, so a miss throws rather than substituting, and the
    /// throw is built exactly the way [`Self::lower_checked_downcast`]'s is —
    /// same class, same `{previous}` bag, same landing block. The comparison
    /// that finds the null goes through [`InstKind::Reinterpret`] because a
    /// descriptor and the machine word holding one are the same bits; see that
    /// instruction for why this is a relabelling rather than a fourth
    /// instruction.
    ///
    /// Nothing here touches a reference count. The operand is *read* rather
    /// than consumed — the value that leaves is a descriptor, which is immortal
    /// and outside the refcount discipline entirely ([`Ty::ClassDesc`]) — so a
    /// fresh `string` operand stays the statement's own temporary and is
    /// released by [`Self::release_temporaries_since`] on the normal edge and
    /// by [`Self::landing_block`] on the throwing one, with no arm of its own.
    ///
    /// **`nullable` is `rule:expressions/nullable-conversion`'s `as ?class<T>`, and it is the same lowering
    /// with the refused edge deleted.** § 2 says the sugar "yields `null`
    /// exactly where it would throw", and here that is not a second path but a
    /// shorter one: [`InstKind::ClassDescIn`] answers a miss with the *null
    /// descriptor* already, so the null test below — the only thing the
    /// checked form adds — is what the `?` takes away. `?class<T>` erases to
    /// [`Ty::ClassDesc`] rather than to [`Ty::Tagged`] for exactly that
    /// reason; that variant's own doc comment owns the decision and names every
    /// site it obliges. The compile-time fold above is taken under `?`
    /// as well, and safely: § 2's written-out `::class` operand is decided by
    /// `nvs_types` under **both** spellings — a name outside the hierarchy is
    /// refused either way, by that crate's own
    /// `reject_impossible_class_reference_conversion` — so what folds here is
    /// always a pair that is a widening.
    ///
    /// # Known gaps
    ///
    /// The message names the bound, not the name that failed to resolve; § 2
    /// asks for the offending class in it, which needs the operand's own string
    /// concatenated in on the refused edge.
    ///
    /// # Panics
    ///
    /// Panics for a `class<T>` target whose `T` this crate cannot name, which
    /// would mean the checker accepted a class reference it did not resolve.
    fn lower_class_reference(
        &mut self,
        inner: &Expr,
        ty: &Type,
        nullable: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let base = self.class_ref_base(ty).unwrap_or_else(|| {
            panic!(
                "nvs-ir: `as class<T>` at {:?} whose argument names no resolved class — \
                 `nvs_types` reports `E0795` for every other spelling, so this is a checker \
                 that did not run",
                ty.span
            )
        });
        // `rule:types/class-reference`'s compile-time row. `Foo::class` travels as the resolved
        // name in the same `ExprInfo::CoreConst` an ordinary class constant
        // does — `Self::lower_expr`'s own `ClassNameConst` arm explains why the
        // name is the checker's to give — so the fold reads it from there
        // rather than from the spelling the author wrote.
        if matches!(inner.kind, ExprKind::ClassNameConst { .. })
            && let Some(ExprInfo::CoreConst {
                value: nvs_types::ConstArg::Str(name),
            }) = self.exprs.lookup(inner.span)
        {
            let class = name.clone();
            return self.emit(*cur, Ty::ClassDesc, InstKind::ClassDescConst { class });
        }
        let (subject, _) = self.lower_expr(inner, None, env, cur);
        let (desc, _) = self.emit(
            *cur,
            Ty::ClassDesc,
            InstKind::ClassDescIn {
                subject,
                base: base.clone(),
            },
        );
        // `rule:expressions/nullable-conversion-availability` over § 2's rows: the miss `ClassDescIn` just answered
        // *is* the `null`, so the sugar's whole content is stopping here.
        if nullable {
            return (desc, Ty::ClassDesc);
        }
        let (word, _) = self.emit(*cur, Ty::Int, InstKind::Reinterpret { operand: desc });
        let (zero, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(0));
        let (missed, _) = self.emit(
            *cur,
            Ty::Bool,
            InstKind::BinOp {
                op: BinOp::Eq,
                lhs: word,
                rhs: zero,
            },
        );
        let hit = self.new_block();
        let refused = self.new_block();
        let hit_edge = self.ids.next_edge(ty.span);
        let refused_edge = self.ids.next_edge(ty.span);
        self.seal(
            *cur,
            Terminator::Branch {
                cond: missed,
                then_block: refused,
                then_edge: refused_edge,
                else_block: hit,
                else_edge: hit_edge,
            },
        );
        let (message, _) = self.emit(
            refused,
            Ty::Str,
            InstKind::ConstStr(format!(
                "cannot convert to `class<{base}>`: the value does not denote a class that is \
                 a `{base}`"
            )),
        );
        let (absent, _) = self.emit(refused, Ty::Null, InstKind::ConstNull);
        let absent = self.coerce(refused, absent, Ty::Null, Ty::Tagged, env);
        let (exception, _) = self.emit_fallible(
            refused,
            Ty::Object,
            InstKind::New {
                class: "RuntimeError".to_owned(),
                ctor: Some(THROWABLE_CTOR.to_owned()),
                args: vec![message, absent],
            },
            env,
        );
        let source = self.throw_source(refused);
        let landing = self.landing_block(env);
        self.seal(
            refused,
            Terminator::Throw {
                value: exception,
                source,
                landing,
            },
        );
        *cur = hit;
        (desc, Ty::ClassDesc)
    }

    /// The class a `class<T>` annotation bounds its descriptors by, as the
    /// label [`crate::ir::Program::classes`] carries — `None` for any other
    /// annotation.
    ///
    /// The `nvs_hir::QName` is destructured here rather than handed on for the
    /// reason [`super::closure::declared_class`] gives: `nvs-hir` is only a
    /// dev-dependency of this crate, so a signature naming that type would not
    /// compile.
    fn class_ref_base(&self, ty: &Type) -> Option<String> {
        let id = self.exprs.declared_ty(ty.span)?;
        // `?class<T>` is `Union([Null, ClassRef])` and the `T` node inside the
        // sugar records no checked type of its own (`Self::nullable_target_atoms`
        // says why), so the `?` is unwrapped here — the same place and the same
        // way `super::array_element_tags` unwraps it for `as ?array<U>`.
        let id = match self.checked_types.get(id) {
            CheckedTy::Union(members) => {
                let mut named = members
                    .iter()
                    .filter(|member| !matches!(self.checked_types.get(**member), CheckedTy::Null));
                let only = *named.next()?;
                named.next().is_none().then_some(only)?
            }
            _ => id,
        };
        let CheckedTy::ClassRef(argument) = self.checked_types.get(id) else {
            return None;
        };
        match self.checked_types.get(*argument) {
            CheckedTy::Class(qname, _) => Some(qname.to_string()),
            _ => None,
        }
    }

    /// `rule:types/unions-and-mixed`'s checked downcast: a [`Ty::Tagged`] operand — a `mixed`,
    /// a `?C`, a union of classes — converted to the declared class `class`
    /// names.
    ///
    /// Nothing new is needed to express it, which is why this is a shape
    /// rather than a helper. [`InstKind::InstanceOf`] already takes a tagged
    /// subject and already answers `false` for a tag that is not an object at
    /// all (`nvs_codegen`'s `emit_instanceof` routes one through
    /// `nvs_value_instanceof` for exactly that), so the row is a test, a
    /// [`Terminator::Throw`] on the false edge and one free
    /// [`InstKind::Untag`] on the true one. A [`Helper`] row could not have
    /// carried it: helper arguments are stored as `nvs_runtime::Value`s, and a
    /// class descriptor is not one.
    ///
    /// The throw is a `RuntimeError` and not the `LogicError` a closure
    /// parameter's identical check raises
    /// ([`super::closure`]'s `check_param_class`): this is the `as` operator,
    /// whose string and non-numeric rows throw that class through
    /// `nvs_runtime::helpers`' `does_not_fit` (its numeric rows are `rule:types/arithmetic`'s `ArithmeticError`), and an operand out of `mixed` is untrusted
    /// input rather than a call written wrong. What arrived is not
    /// named in the message, for the reason that function's doc comment
    /// records — no [`InstKind`] reads an object's class name.
    ///
    /// **Ownership follows [`Self::convert`]'s free row, split across the two
    /// edges.** [`InstKind::Untag`] is a relabelling, so the object shares the
    /// tagged operand's reference: a borrowed operand is retained on the way
    /// out, because the consumer of an `as` owns its result, and a fresh one
    /// transfers instead. The false edge is where that asymmetry has to be
    /// said out loud — a fresh operand reaches no consumer there, so it is
    /// released before the throw rather than abandoned on it.
    fn lower_checked_downcast(
        &mut self,
        value: ValueId,
        class: &str,
        operand: &Expr,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let borrowed = self.aliasing_read(operand);
        let (is_instance, _) = self.emit(
            *cur,
            Ty::Bool,
            InstKind::InstanceOf {
                value,
                class: TestedClass::Named(class.to_owned()),
            },
        );
        let hit = self.new_block();
        let refused = self.new_block();
        let hit_edge = self.ids.next_edge(span);
        let refused_edge = self.ids.next_edge(span);
        self.seal(
            *cur,
            Terminator::Branch {
                cond: is_instance,
                then_block: hit,
                then_edge: hit_edge,
                else_block: refused,
                else_edge: refused_edge,
            },
        );
        if !borrowed {
            self.emit_release(refused, value);
        }
        let (message, _) = self.emit(
            refused,
            Ty::Str,
            InstKind::ConstStr(format!(
                "cannot convert a value of another type to `{class}`"
            )),
        );
        // Argument 2 is the `{previous}` bag flattened to its own `null`
        // default, widened into the `Ty::Tagged` slot spec § 10's
        // `Throwable|null` erases to — the same list `Self::lower_match`'s
        // unmatched throw builds by hand, and for the same reason.
        let (absent, _) = self.emit(refused, Ty::Null, InstKind::ConstNull);
        let absent = self.coerce(refused, absent, Ty::Null, Ty::Tagged, env);
        let (exception, _) = self.emit_fallible(
            refused,
            Ty::Object,
            InstKind::New {
                class: "RuntimeError".to_owned(),
                ctor: Some(THROWABLE_CTOR.to_owned()),
                args: vec![message, absent],
            },
            env,
        );
        let source = self.throw_source(refused);
        let landing = self.landing_block(env);
        self.seal(
            refused,
            Terminator::Throw {
                value: exception,
                source,
                landing,
            },
        );
        *cur = hit;
        let (out, _) = self.emit(hit, Ty::Object, InstKind::Untag { operand: value });
        if borrowed {
            self.emit_retain(hit, out);
        }
        (out, Ty::Object)
    }

    /// Whether `ty` names `rule:core-classes/html-auto-escape`'s `Core\Html\Markup` — the one class target this crate lowers by
    /// *building* rather than by testing.
    ///
    /// [`super::closure::declared_class`] cannot answer it and should not:
    /// that helper reports only a class the program itself declared, `Core`
    /// names deliberately excluded, because every target it feeds is checked
    /// against a descriptor the compiled unit laid out and a `Core` class has
    /// none in it. This target is decided by a *rule* instead — a source
    /// literal, refused as `E0417` where it is not — so the name is all that
    /// has to survive, and it comes from `nvs_stdlib` through `nvs_types`
    /// rather than being spelled again here.
    fn markup_target(&self, ty: &Type) -> bool {
        let Some(id) = self.exprs.declared_ty(ty.span) else {
            return false;
        };
        match self.checked_types.get(id) {
            // `QName` is destructured rather than named, for
            // `super::closure::declared_class`'s reason: `nvs-hir` is a
            // dev-dependency of this crate.
            CheckedTy::Class(qname, _) => qname.to_string() == nvs_types::CORE_HTML_MARKUP_CLASS,
            _ => false,
        }
    }

    /// `rule:core-classes/html-auto-escape`'s `"<b>" as Core\Html\Markup` — the sink's only raw-write bypass,
    /// and the one conversion in this crate whose result is *constructed*.
    ///
    /// **Nothing is decided here.** § 5's rule is that the operand is a source
    /// literal and never anything computed, and `nvs_types::expr::quals` has
    /// already refused every other operand where it was written — a `tainted`
    /// value, a `secret` one and anything with a run-time step in it
    /// (`E0417`). So what is left by this point is one trusted `string` and
    /// the layout it has to be wrapped in.
    ///
    /// **It is an [`InstKind::CoreCall`] rather than a [`Helper`] row**, which
    /// is the whole of what makes it a decision rather than a transcription.
    /// Every `Helper` symbol is one `nvs-runtime` exports, and a `Markup` is a
    /// one-slot instance of a class registered in `nvs-stdlib` — a crate
    /// `nvs-runtime` may not depend on, since the dependency runs the other
    /// way. `nvs-stdlib` owns the layout, so `nvs-stdlib` owns the symbol, and
    /// this crate names it through `nvs_types` exactly as `spawn script` and a
    /// duration literal name theirs.
    ///
    /// **Ownership is the string rows' in [`Self::convert`]**: a `CoreCall`
    /// borrows its arguments, the callee takes its own reference for the slot,
    /// so a fresh operand is released once the call has read it and a borrowed
    /// one is left alone — the pair that leaves exactly one reference for the
    /// consumer of an `as` to own.
    fn lower_markup_lift(
        &mut self,
        v: ValueId,
        operand: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let out = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::CoreCall {
                symbol: nvs_types::CORE_HTML_MARKUP,
                args: vec![v],
            },
            env,
        );
        if !self.aliasing_read(operand) {
            self.emit_release(*cur, v);
        }
        out
    }

    /// `rule:types/conversion`'s
    /// `array<T> as array<U>` row: every element must satisfy `U`, checked as
    /// the walk goes, in [`Helper::ToArrayOf`] — or in
    /// [`Helper::ToArrayOfOrNull`] when `or_null`, `rule:expressions/nullable-conversion`'s spelling of the
    /// same walk.
    ///
    /// **This is not a row of [`Self::convert`] and cannot be one**, for the
    /// reason [`Self::lower_checked_downcast`] is not either: what decides it
    /// is the target's *element* type, and by the time `convert` sees a pair
    /// of [`Ty`]s that has erased to one [`Ty::Array`] on both sides. So the
    /// row is read off the annotation here, where the checked type still
    /// exists, and travels as [`super::array_element_tags`]' word — one tag
    /// nibble per level of `U`, which is what a helper argument can carry.
    ///
    /// An `array<mixed>` target from an operand that is already an array is
    /// the one shape that runs nothing: every tag satisfies `mixed`, so the
    /// walk could only answer `true`, and what is left is `convert`'s free
    /// widening row. A [`Ty::Tagged`] operand still calls, because the tag
    /// test on the operand *itself* is `rule:types/unions-and-mixed`'s whole content there.
    ///
    /// **Ownership is the `bytes` rows'**, and the buffer is not copied:
    /// [`Helper::ToArrayOf`] hands back the operand's own allocation under one
    /// more reference, so a borrowed operand needs nothing and a fresh one is
    /// released once the helper has read it — the pair that leaves exactly one
    /// reference for the consumer of an `as` to own, either way.
    ///
    /// # Panics
    ///
    /// Panics for an operand representation that is neither an array nor a
    /// tagged value: `rule:types/conversion` gives no other operand a row into an array,
    /// and `nvs_types` refuses each where it is written (`E0708`).
    #[expect(
        clippy::too_many_arguments,
        reason = "the same context `lower_conversion` itself threads, plus the one bit that \
                  chooses between `rule:types/conversion`'s spelling of this row and `rule:expressions/nullable-conversion`'s"
    )]
    fn lower_array_restamp(
        &mut self,
        v: ValueId,
        from: Ty,
        tags: u64,
        operand: &Expr,
        or_null: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        assert!(
            matches!(from, Ty::Array | Ty::Tagged),
            "nvs-ir lowers `rule:types/conversion`'s `array<T> as array<U>` row from an array or from a \
             tagged value — got representation {from:?}, every other operand being `E0708` at \
             the checker"
        );
        if !or_null && from == Ty::Array && tags == u64::from(super::FN_PARAM_TAG_ANY) {
            return self.convert(v, from, Ty::Array, operand, env, *cur);
        }
        let (word, _) = self.emit(*cur, Ty::Uint, InstKind::ConstUint(tags));
        let call = InstKind::HelperCall {
            helper: if or_null {
                Helper::ToArrayOfOrNull
            } else {
                Helper::ToArrayOf
            },
            args: vec![v, word],
        };
        // The `null`-answering walk cannot fault, and still takes the error
        // edge its checked twin does: every helper returns a status and an
        // uncatchable one has to leave the frame swept (`Inst::on_error`).
        let out = if or_null {
            self.emit_fallible(*cur, Ty::Tagged, call, env)
        } else {
            self.emit_fallible(*cur, Ty::Array, call, env)
        };
        if !self.aliasing_read(operand) {
            self.emit_release(*cur, v);
        }
        out
    }

    /// The closed set of literals an `expr as T` has to test its operand
    /// against at run time —
    /// `rule:types/literal-types`'s
    /// "the only place either type costs anything at runtime" — or `None`
    /// where this conversion is one of § 4's ordinary rows.
    ///
    /// A **whole enum** is one of these sets too, and is where
    /// `rule:types/conversion`'s
    /// "throws on a value no case names" is emitted from: the annotation
    /// names no members, but the declaration does, so the set is built from
    /// every case of it ([`whole_enum_set`]) and the chain that follows
    /// is the same one a named subset gets.
    ///
    /// Read off the **checked** type rather than the annotation, because that
    /// is where the values still are: § 2's `Foo::TYPE_A` folded to the string
    /// it names during checking, and nothing in the AST says which string that
    /// was. It is also the last place they exist at all — `lower_decl_type`
    /// erases the whole set to the one base its members share.
    ///
    /// `None` in each of these cases, and each is a decision:
    ///
    /// * The target is not a closed set. One wider atom — `string`, or the
    ///   `null` an `as ?T` adds — is a member the operand may reach, so there
    ///   is nothing to test against. This mirrors `closed_set_atoms` in
    ///   `nvs_types::expr::operators`, which decides the same question for
    ///   § 6's compile-time half.
    /// * The operand already names one value, which the checker has therefore
    ///   already settled: a singleton operand outside the set is `E0469`/
    ///   `E0470` and never reaches lowering, so `"a" as "a"|"b"` needs no
    ///   comparison and an enum case needs none either.
    /// * There is no recorded checked type for the annotation, which is the
    ///   shape `lower_decl_type` answers from the AST alone.
    /// * The target is a whole enum and `from` is already that enum's own
    ///   representation, so every value the operand can hold is a case by
    ///   construction. It is the *same* enum and not merely one with the same
    ///   backing type, because `nvs_types` refuses a conversion between two
    ///   different enums outright (`reject_enum_to_enum_conversion`).
    fn closed_literal_set(&self, ty: &Type, inner: &Expr, from: Ty) -> Option<AcceptedSet> {
        let target = self.exprs.declared_ty(ty.span)?;
        let atoms: Vec<TypeId> = match self.checked_types.get(target) {
            CheckedTy::Union(members) => members.clone(),
            _ => vec![target],
        };
        self.closed_set_of_atoms(&atoms, Some(inner), from)
    }

    /// The set of names a checked `as property<T>` accepts —
    /// `rule:types/property-key`'s two run-time rows, as the same [`AcceptedSet`] `rule:types/enum-case-type`'s
    /// literal union already tests against.
    ///
    /// Read off [`ExprInfo::PropertyKey`] rather than off the checked type,
    /// because the type does not carry it: `property<T>`'s values are `T`'s
    /// public declared property names, which needs the class hierarchy and the
    /// signature table this crate has neither of. That variant's own doc
    /// comment owns why the entry is keyed by the annotation's span, which is
    /// what lets this take a `Type` and nothing else.
    ///
    /// `None` for every other annotation, **and for a written-out operand**:
    /// § 2 decides `"email" as property<User>` where it is written, so the
    /// checker records no entry for one and the conversion is the free
    /// `Ty::Str` → `Ty::Str` row [`Self::convert`] already answers.
    ///
    /// The rendering is the throw's whole message past the name that failed —
    /// `nvs_runtime`'s `nvs_literal_mismatch` writes "`x` is not one of " in
    /// front of it — which is why the class is named here rather than left to a
    /// bare list, and § 2 asks for exactly that.
    fn property_key_set(&self, ty: &Type) -> Option<AcceptedSet> {
        let ExprInfo::PropertyKey { class, names } = self.exprs.lookup(ty.span)? else {
            return None;
        };
        Some(AcceptedSet {
            members: names
                .iter()
                .map(|name| LiteralAtom::Str(name.clone()))
                .collect(),
            rendered: if names.is_empty() {
                format!("`{class}`'s public declared properties, of which it has none")
            } else {
                format!(
                    "`{class}`'s public declared properties: {}",
                    names
                        .iter()
                        .map(|name| format!("`${name}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        })
    }

    /// [`Self::closed_literal_set`] over an atom list the caller already
    /// expanded, which is what an `as ?T` needs: its annotation's checked type
    /// is the union `T|null`, so the target is what is left once `null` is
    /// dropped ([`Self::nullable_target_atoms`]) and there is no single
    /// [`TypeId`] naming it — this crate holds the interner by shared
    /// reference and cannot intern one.
    ///
    /// `operand` is `None` for exactly that caller, and the omission is the
    /// rule rather than a shortcut: [`Self::operand_names_one_value`] is this
    /// function deferring to a decision **the checker already took**
    /// (`reject_impossible_literal_conversion`), and that check bails on a
    /// target holding one wider atom — which `null` is. So `3 as Mode` is
    /// settled at compile time and `3 as ?Mode` is not, and the second one
    /// still owes the run-time chain the first one is excused.
    fn closed_set_of_atoms(
        &self,
        atoms: &[TypeId],
        operand: Option<&Expr>,
        from: Ty,
    ) -> Option<AcceptedSet> {
        let types = self.checked_types;
        if let [target] = atoms
            && let CheckedTy::Enum(qname, backing) = types.get(*target)
        {
            let repr = match backing {
                nvs_types::EnumBacking::Int => EnumRepr::Int,
                nvs_types::EnumBacking::Uint => EnumRepr::Uint,
            };
            if from == Ty::Enum(repr) {
                return None;
            }
            let info = self.enums.get(qname).unwrap_or_else(|| {
                panic!(
                    "nvs-ir: `{qname}` is an interned enum type with no entry in the run's \
                     enum table — `nvs_types` interns one only for an enum it resolved, so \
                     the two tables disagree"
                )
            });
            return Some(whole_enum_set(info, &qname.to_string()));
        }
        if operand.is_some_and(|inner| self.operand_names_one_value(inner)) {
            return None;
        }
        // One pass, not a `closed` predicate and then a map over the same
        // atoms: two matches over one list is two places to add an atom kind
        // to, and the second one's catch-all could only be a panic no program
        // reaches — an internal-consistency check between a list and itself.
        // `collect::<Option<_>>` makes "this target is not a closed set" the
        // same answer here as it is above.
        let members: Vec<LiteralAtom> = atoms
            .iter()
            .map(|id| match types.get(*id) {
                CheckedTy::StringLiteral(text) => Some(LiteralAtom::Str(text.clone())),
                CheckedTy::IntLiteral(value) => Some(LiteralAtom::Int(*value)),
                // `rule:types/grammar`'s two `bool` singletons — one value each, so a
                // closed set of exactly the kind § 5 tests, and the reason
                // `$m as true` is `as bool` plus a membership test rather than
                // a target the language parses and cannot lower.
                CheckedTy::True => Some(LiteralAtom::Bool(true)),
                CheckedTy::False => Some(LiteralAtom::Bool(false)),
                // § 3's enum-case subset. The checked type names the enum and
                // the case but deliberately not the value (see
                // `nvs_types::ty::Ty::EnumCase`'s own doc comment for why
                // folding it to an int literal would reopen `rule:types/conversion`), so
                // the constant comes from the run's own enum table — the one
                // place it still exists by the time lowering runs.
                CheckedTy::EnumCase(qname, _, case) => {
                    let value = self.enums.case(qname, case).unwrap_or_else(|| {
                        panic!(
                            "nvs-ir: `{qname}::{case}` is an interned enum-case type with no \
                             entry in the run's enum table — `nvs_types` interns one only for a \
                             case it resolved, so the two tables disagree"
                        )
                    });
                    Some(LiteralAtom::EnumCase(value))
                }
                // One wider atom and the target is not a closed set at all —
                // `string`, or the `null` an `as ?T` adds. See this function's
                // own doc comment: that is one of its `None`s, not a shape it
                // declines to lower.
                _ => None,
            })
            .collect::<Option<_>>()?;
        // § 6: the accepted set is generated from the type, never written per
        // site — the same rendering `reject_impossible_literal_conversion`
        // produces for the compile-time half, so the two messages read alike.
        let rendered = atoms
            .iter()
            .map(|id| format!("`{}`", types.describe(*id)))
            .collect::<Vec<_>>()
            .join(", ");
        Some(AcceptedSet { members, rendered })
    }

    /// The atoms of an `as ?T` annotation's target — the checker's type for
    /// the *whole* `?T` with `null` dropped.
    ///
    /// Read off the whole annotation and not off the `T` inside it, because
    /// only the whole one was recorded: `nvs_types::lower::lower_type` calls
    /// [`ExprTypeTable::record_type`] once, at its own entry point, so a
    /// nested `Type` node has no entry at all and
    /// [`lower_decl_type`] would fall back to answering `?Mode`'s target from
    /// the AST — where a name-shaped atom is a class and an enum is
    /// indistinguishable from one. That fallback is what would make
    /// `$m as ?Mode` panic on `Tagged as ?Object`.
    ///
    /// `None` where the checker never visited the annotation, which is the
    /// same shape [`lower_decl_type`] answers from the AST alone.
    fn nullable_target_atoms(&self, ty: &Type) -> Option<Vec<TypeId>> {
        let whole = self.exprs.declared_ty(ty.span)?;
        // `?T` is interned as `T|null` — the checker has no separate nullable
        // type (`erase_checked_ty`'s `CheckedTy::Null` arm says so).
        let CheckedTy::Union(members) = self.checked_types.get(whole) else {
            return None;
        };
        let kept: Vec<TypeId> = members
            .iter()
            .copied()
            .filter(|id| !matches!(self.checked_types.get(*id), CheckedTy::Null))
            .collect();
        (!kept.is_empty()).then_some(kept)
    }

    /// Whether a conversion's operand names exactly one value, so that
    /// `nvs_types::expr::operators::reject_impossible_literal_conversion` has
    /// already decided this conversion's outcome at compile time.
    ///
    /// The same expression shapes that checker's own
    /// `conversion_operand_singleton` accepts, asked here only as a yes/no:
    /// what the value *is* does not matter, because a singleton the target
    /// rejects is `E0469`/`E0470` and never reaches lowering, so one that
    /// arrives here is in the set by construction.
    fn operand_names_one_value(&self, inner: &Expr) -> bool {
        match &inner.kind {
            ExprKind::Str(_) | ExprKind::Int(_) | ExprKind::Bool(_) => true,
            ExprKind::Paren(nested) => self.operand_names_one_value(nested),
            ExprKind::ClassConstAccess { .. } => {
                matches!(
                    self.exprs.lookup(inner.span),
                    Some(ExprInfo::EnumCase { .. })
                )
            }
            _ => false,
        }
    }

    /// Relabels an enum value as the `int`/`uint` its cases *are*, leaving
    /// every other representation exactly as it arrived.
    ///
    /// `rule:enums/no-class-machinery` makes
    /// a case a compile-time integer constant, and [`Ty::Enum`] is a zero-byte
    /// tag over it — so this is the free [`InstKind::Reinterpret`] row 1 of
    /// that ADR's *5* already uses for `$m as int`, emitting no machine
    /// instruction at all. Every comparison over an enum goes through it,
    /// because `nvs-codegen`'s `BinOp` table is `Ty::Int`/`Ty::Uint`/`Ty::Bool`
    /// and carries no `Ty::Enum` row: `rule:types/literal-types`'s membership chain, and
    /// `rule:expressions/disjoint-comparison-refused`'s `==` between two cases of one enum.
    ///
    /// Nothing is released or retained around it: an enum is a scalar, so the
    /// relabelled value borrows no ownership from the operand.
    pub(crate) fn reinterpret_enum_to_backing(
        &mut self,
        value: ValueId,
        value_ty: Ty,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        match value_ty {
            Ty::Enum(EnumRepr::Int) => {
                self.emit(*cur, Ty::Int, InstKind::Reinterpret { operand: value })
            }
            Ty::Enum(EnumRepr::Uint) => {
                self.emit(*cur, Ty::Uint, InstKind::Reinterpret { operand: value })
            }
            _ => (value, value_ty),
        }
    }

    /// `rule:expressions/nullable-conversion-availability`
    /// row 2 — `expr as ?T` where `T` is a literal type, an enum-case subset
    /// or a whole enum: "that conversion is already checked and throwing;
    /// this is its non-throwing twin."
    ///
    /// Built as the twin rather than as a redirect of the throwing one.
    /// [`Self::landing_block`] ends in `Terminator::Catch`/`Propagate` and a
    /// pending `Throwable`, so re-pointing the checked lowering's error edge
    /// at a null-producing block would have to discard that object and account
    /// for its reference — `lower::exception`'s plumbing, for a form that
    /// needs no exception to exist at all. Two substitutions on
    /// [`Self::lower_conversion`]'s non-nullable arm buy the same thing: the
    /// membership chain takes a `miss` block instead of the throw
    /// ([`Self::lower_literal_membership`]), and the base conversion runs
    /// through [`Self::convert_or_null`] wherever its row can fail.
    ///
    /// The two shapes below are the same split that arm already makes, for
    /// the same reason:
    ///
    /// * A [`Ty::Tagged`] operand into a **literal** set is tested first, on
    ///   its own runtime tag, and **needs no conversion at all** — the result
    ///   of `as ?T` is a [`Ty::Tagged`] value, and on a hit the operand
    ///   already *is* one, holding exactly the value the chain just proved it
    ///   holds. Converting first would run `Helper::TaggedToString` and let a
    ///   `mixed` holding `1` satisfy a set naming `"1"`, which is the coercion
    ///   `rule:types/literal-types` refuses.
    /// * Everything else converts to the target's own base first — an
    ///   **enum** target included, `rule:types/conversion` wording that row as "exactly
    ///   the shape `as uint` already has for untrusted input" — and a row
    ///   that can fail runs as its `?` form, whose `null` matches no member
    ///   and so reaches the same miss edge with no test of its own. That is
    ///   why the fallible branch keeps the tagged answer and compares through
    ///   [`Helper::Identical`]: an `Untag` of a `null` would read a zero
    ///   payload, and an enum with a case backed by `0` would then *hit* on a
    ///   conversion that failed.
    ///
    /// Ownership is one rule for both shapes: `answer` is a [`Ty::Tagged`]
    /// value this frame owns by the time the chain runs — retained where the
    /// operand was borrowed storage, produced fresh by the conversion
    /// otherwise — so the hit path hands the consumer the reference every
    /// other conversion row hands it, and the miss path releases it, which is
    /// a runtime no-op for every tag that owns nothing.
    #[allow(clippy::too_many_arguments)]
    fn lower_nullable_membership(
        &mut self,
        v: ValueId,
        from: Ty,
        to: Ty,
        accepted: &AcceptedSet,
        inner: &Expr,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let miss = self.new_block();
        let join = self.new_block();
        let (probe, probe_ty, answer) = if from == Ty::Tagged && !matches!(to, Ty::Enum(_)) {
            if self.aliasing_read(inner) {
                self.emit_retain(*cur, v);
            }
            (v, Ty::Tagged, v)
        } else {
            let base = match to {
                Ty::Enum(EnumRepr::Int) => Ty::Int,
                Ty::Enum(EnumRepr::Uint) => Ty::Uint,
                other => other,
            };
            if conversion_can_fail(from, base) {
                let (tagged, _) = self.convert_or_null(v, from, base, inner, env, *cur);
                (tagged, Ty::Tagged, tagged)
            } else {
                let (converted, converted_ty) = self.convert(v, from, to, inner, env, *cur);
                // A heterogeneous set erases to `Ty::Tagged` already, and
                // `Self::convert`'s widening row put the tag on — a second
                // one would tag a `Value`.
                let answer = if converted_ty == Ty::Tagged {
                    converted
                } else {
                    self.emit(*cur, Ty::Tagged, InstKind::Tag { operand: converted })
                        .0
                };
                (converted, converted_ty, answer)
            }
        };
        self.lower_literal_membership(probe, probe_ty, accepted, span, env, cur, Some(miss));
        let hit = *cur;
        self.seal(hit, Terminator::Jump(join));
        self.emit_release(miss, answer);
        let (null_v, _) = self.emit(miss, Ty::Null, InstKind::ConstNull);
        let null_v = self.coerce(miss, null_v, Ty::Null, Ty::Tagged, env);
        self.seal(miss, Terminator::Jump(join));
        let (merged, _) = self.emit(
            join,
            Ty::Tagged,
            InstKind::Phi {
                incoming: vec![(hit, answer), (miss, null_v)],
            },
        );
        *cur = join;
        (merged, Ty::Tagged)
    }

    /// `rule:types/literal-types`'s
    /// membership test: a chain of equality comparisons, each branching
    /// straight to the one block where the conversion succeeded, with the
    /// throw at the far end where every one of them missed.
    ///
    /// A chain and not one runtime call over an encoded set, because the set
    /// is small, closed and compile-time-known: each arm is a `BinOp::Eq`,
    /// which `nvs-codegen` turns into a machine comparison for an integer and
    /// a direct two-pointer `nvs_str_eq` for a string. One helper call over an
    /// encoded set would instead pay `rule:errors/propagation`'s calling convention *and* parse
    /// that encoding on every conversion. Only a [`Ty::Tagged`] operand pays a
    /// call, and it pays exactly the one `rule:expressions/mixed-equality` already charges a
    /// `mixed` `==`: [`Helper::Identical`], which answers `false` for a
    /// mismatched tag rather than converting either side.
    ///
    /// Nothing is merged at the join: the value under test is defined before
    /// the chain and dominates every block in it, so there is no
    /// [`InstKind::Phi`] here and no `Env` to reconcile — every block this
    /// builds is straight-line and assigns nothing.
    ///
    /// `miss` is what happens where every comparison missed, and it is the
    /// whole difference between the two spellings `rule:expressions/nullable-conversion-availability` row 2 calls
    /// twins. `None` is `expr as T`: the throw above, on `rule:errors/propagation`'s error
    /// edge. `Some(block)` is `expr as ?T`, which jumps there instead and
    /// answers `null` — [`Self::lower_nullable_membership`] owns that block,
    /// because only it knows what the result value and its ownership are.
    #[allow(clippy::too_many_arguments)]
    fn lower_literal_membership(
        &mut self,
        value: ValueId,
        value_ty: Ty,
        accepted: &AcceptedSet,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
        miss: Option<BlockId>,
    ) {
        // An enum operand is tested one representation down, on the integer
        // its cases *are*. The value the conversion answers with is
        // untouched: this reinterpret feeds the comparisons alone.
        let (value, value_ty) = self.reinterpret_enum_to_backing(value, value_ty, cur);
        let hit = self.new_block();
        for member in &accepted.members {
            let (kind, ty) = literal_constant(member);
            let (wanted, _) = self.emit(*cur, ty, kind);
            let (equal, _) = if value_ty == Ty::Tagged {
                self.emit_fallible(
                    *cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper: Helper::Identical,
                        args: vec![value, wanted],
                    },
                    env,
                )
            } else {
                self.emit(
                    *cur,
                    Ty::Bool,
                    InstKind::BinOp {
                        op: BinOp::Eq,
                        lhs: value,
                        rhs: wanted,
                    },
                )
            };
            // The constant is fresh and this comparison is its one and only
            // use — the same policy `lower_binary` applies to the string
            // literal in `$key == "bad"`.
            if ty.is_refcounted() {
                self.emit_release(*cur, wanted);
            }
            let next = self.new_block();
            let hit_edge = self.ids.next_edge(span);
            let miss_edge = self.ids.next_edge(span);
            self.seal(
                *cur,
                Terminator::Branch {
                    cond: equal,
                    then_block: hit,
                    then_edge: hit_edge,
                    else_block: next,
                    else_edge: miss_edge,
                },
            );
            *cur = next;
        }
        // `rule:expressions/nullable-conversion-availability` row 2's non-throwing twin: every comparison missed, so
        // the answer is `null` and the caller's own block builds it.
        if let Some(block) = miss {
            self.seal(*cur, Terminator::Jump(block));
            *cur = hit;
            return;
        }
        let (listed, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(accepted.rendered.clone()));
        let landing = self.landing_block(env);
        self.block_insts[cur.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::HelperCall {
                helper: Helper::LiteralMismatch,
                args: vec![value, listed],
            },
            on_error: Some(landing),
        });
        // No release for `listed`: `Helper::LiteralMismatch` owns it, for the
        // reason that variant states — one emitted here would sit in the
        // block only an `Ok` return reaches, which this call never makes.
        //
        // `Helper::LiteralMismatch` never returns normally, so this jump is
        // unreachable — written anyway because a block still owes a
        // terminator, and `hit` is the block control would have reached.
        self.seal(*cur, Terminator::Jump(hit));
        *cur = hit;
    }
}

/// The two blocks a `?->` guard still owes once its member access is lowered
/// — see [`Lowering::open_nullsafe`], which is the only thing that builds one,
/// and [`Lowering::close_nullsafe`], which is the only thing that consumes it.
///
/// Absent (`None`) whenever the receiver's representation proved it cannot be
/// `null`, which is what makes a nullsafe access on a non-nullable receiver
/// cost exactly nothing.
/// Every case of one enum declaration, as the [`AcceptedSet`] an
/// `expr as EnumName` tests its operand against —
/// `rule:types/conversion`'s "throws
/// on a value no case names" made concrete, and the one thing that keeps an
/// enum a *closed* set once a plain integer can be converted into it.
///
/// A free function rather than a method because it needs nothing of the
/// lowering state: `name` is the enum's resolved name already rendered, which
/// is how this stays clear of `nvs_hir::QName` — this crate does not depend on
/// `nvs-hir`, and [`Lowering::closed_literal_set`] holds the one reference to
/// one long enough to do the table lookup itself.
///
/// **Sorted by the case's own constant**, which is not cosmetic:
/// `EnumInfo::cases` is an `FxHashMap`, so an unsorted set would render the
/// throw's accepted list in a different order from run to run and no test
/// could pin the message. By the constant rather than by the name so that the
/// ordinary declaration — no `= n` clause anywhere, values auto-incrementing
/// from 0 (`rule:enums/one-backing-type`) — reads back in the order it was written; the name
/// breaks a tie, so the order is total either way.
pub(crate) fn whole_enum_set(info: &nvs_types::EnumInfo, name: &str) -> AcceptedSet {
    let mut cases: Vec<(&str, nvs_types::EnumValue)> = info
        .cases
        .iter()
        .map(|(case, value)| (case.as_str(), *value))
        .collect();
    cases.sort_by_key(|(case, value)| {
        let ordinal = match value {
            nvs_types::EnumValue::Int(n) => i128::from(*n),
            nvs_types::EnumValue::Uint(n) => i128::from(*n),
        };
        (ordinal, *case)
    });
    AcceptedSet {
        members: cases
            .iter()
            .map(|(_, value)| LiteralAtom::EnumCase(*value))
            .collect(),
        rendered: cases
            .iter()
            .map(|(case, _)| format!("`{name}::{case}`"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// The closed set of values a checked `as` into an
/// `rule:types/literal-types` literal
/// type accepts — see [`Lowering::closed_literal_set`], which is the only
/// thing that builds one, and [`Lowering::lower_literal_membership`], which is
/// the only thing that consumes it.
pub(crate) struct AcceptedSet {
    /// The values themselves, in the order the target type states them — or,
    /// for a whole enum, in the order [`whole_enum_set`] sorts the
    /// declaration's cases into, since a hash map states no order at all.
    members: Vec<LiteralAtom>,
    /// Those same values rendered for the throw's message — built here rather
    /// than at run time because a literal type does not survive erasure, so
    /// this is the last point at which the set can be named at all.
    rendered: String,
}

/// One member of an [`AcceptedSet`], already reduced to the constant that
/// tests for it — and the payload half of `rule:types/type-test`'s literal
/// row, which asks the identical question of the identical atom
/// ([`Lowering::lower_type_test`]).
///
/// § 3's enum case keeps its [`nvs_types::EnumValue`] rather than collapsing
/// into [`Self::Int`]: the backing type decides both the constant's
/// instruction and its representation, and an enum's tag is not `Ty::Int`
/// even where its backing is (see [`Ty::Enum`]).
pub(crate) enum LiteralAtom {
    Str(String),
    Int(i64),
    /// `true` or `false` — `rule:types/grammar`'s two `bool` singletons, whose
    /// closed set is the smallest one this crate builds.
    Bool(bool),
    EnumCase(nvs_types::EnumValue),
}

/// The constant one [`LiteralAtom`] tests for: the instruction that
/// materializes it, and the representation it lands at.
///
/// A table of its own because two operators reduce the same atom the same
/// way — `as`'s [`Lowering::lower_literal_membership`] and `is`'s
/// [`Lowering::lower_type_test`] — and a second copy would be a second place
/// an atom kind has to be added to.
///
/// An enum case lands at its *backing* scalar rather than at [`Ty::Enum`],
/// which is what the atom kept an [`nvs_types::EnumValue`] for: the
/// comparison happens one representation down, and the two backings are two
/// instructions.
pub(crate) fn literal_constant(atom: &LiteralAtom) -> (InstKind, Ty) {
    match atom {
        LiteralAtom::Str(text) => (InstKind::ConstStr(text.clone()), Ty::Str),
        LiteralAtom::Int(number) => (InstKind::ConstInt(*number), Ty::Int),
        LiteralAtom::Bool(value) => (InstKind::ConstBool(*value), Ty::Bool),
        LiteralAtom::EnumCase(nvs_types::EnumValue::Int(n)) => (InstKind::ConstInt(*n), Ty::Int),
        LiteralAtom::EnumCase(nvs_types::EnumValue::Uint(n)) => (InstKind::ConstUint(*n), Ty::Uint),
    }
}

/// The `T` of an `as ?T` annotation, or `None` for any other target.
///
/// Read off the AST rather than off the lowered [`Ty`] because that erasure is
/// exactly what loses the distinction: `?string` and `?int` are both
/// [`Ty::Tagged`] (see its own doc comment), so a conversion between them is
/// indistinguishable from a conversion to the type the value already has.
/// `(...)` is transparent here, the same way [`lower_decl_type`] treats it.
/// A `null|T` *union* spelling is deliberately not folded in: `rule:expressions/nullable-conversion`
/// defines the operator over `?T`, and a union target has no lowering at all
/// yet — one gap is better than a second spelling that half works.
/// The one representation a target's atoms share, or [`Ty::Tagged`] where
/// they share none — [`erase_checked_ty`]'s own `CheckedTy::Union` fold, over
/// an atom list rather than over an interned union.
///
/// The `?T` half of `rule:types/literal-types`'s "zero additional runtime representation"
/// needs this separately because `T|null` is the union that *is* interned, and
/// folding that one would answer [`Ty::Tagged`] for every target: `null` and
/// `Ty::Str` are two representations, not one.
fn shared_repr(atoms: &[TypeId], checked_types: &TypeInterner) -> Ty {
    let mut shared: Option<Ty> = None;
    for atom in atoms {
        match (erase_checked_ty(*atom, checked_types), shared) {
            (Some(ty), None) => shared = Some(ty),
            (Some(ty), Some(seen)) if ty == seen => {}
            _ => return Ty::Tagged,
        }
    }
    shared.unwrap_or(Ty::Tagged)
}

/// Whether [`Lowering::convert`] would emit an error edge for this row —
/// which is the same question as "does this row have a `?` form to run
/// instead", and is asked only by [`Lowering::lower_nullable_membership`].
///
/// The `false` arms are that function's own free, total and widening rows,
/// listed in the order its doc comment names them; everything else is one of
/// its checked rows and goes through [`Lowering::convert_or_null`]. A row that
/// can fail and has no `?` helper — `bytes`/an object into a *string* literal
/// set — reaches that function's own panic naming `as ?string`, which is the
/// gap it already names for the plain `$b as ?string` spelling rather than a
/// second one this form opens.
fn conversion_can_fail(from: Ty, to: Ty) -> bool {
    if from == to {
        return false;
    }
    !matches!(
        (from, to),
        (Ty::Enum(EnumRepr::Int), Ty::Int)
            | (Ty::Enum(EnumRepr::Uint), Ty::Uint)
            | (Ty::Str, Ty::Bytes)
            | (
                Ty::Bool | Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal,
                Ty::Str
            )
            | (_, Ty::Bool | Ty::Tagged)
    )
}

fn nullable_target(ty: &Type) -> Option<&Type> {
    match &ty.kind {
        TypeKind::Nullable(inner) => Some(inner),
        TypeKind::Paren(inner) => nullable_target(inner),
        _ => None,
    }
}
