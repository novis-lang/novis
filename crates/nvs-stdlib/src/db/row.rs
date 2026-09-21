//! The accessors on what a statement returned — `Core\Db\Rows`,
//! `Core\Db\Row`, `Core\Db\Column` and `Core\Db\Write` — and the
//! hydration `queryAs` does into a declared class.
//!
//! A row is an array the query already built, so every member here reads that
//! and never the wire. The typed accessors are where the type a value is stored
//! as meets the one the caller asked for, which is why [`Requested`] has three
//! answers and not two: [ADR 0067 § 6](/docs/decisions/0067.md)
//! names "no such column" and "a value that will not fit" as different
//! refusals, and a `bool` that is really a `0` has to be told from a `7`.
//!
//! [`nvs_core_db_row_instant`] is the one member here with no reading on
//! SQLite: its columns are the two that carry their own zone, and
//! [`nvs_db::SqliteColumn::column_type`] folds every `TIMESTAMP` spelling into
//! [`nvs_db::ColumnType::DateTime`], which this member refuses by class. Every
//! proof program in this repository connects to an in-memory SQLite block, so
//! that member's example, attack and bench are skip entries in
//! `tools/data/dossier-policy.toml` and its tests are not.

use super::*;

/// The rows one of [`ROWS`]'s members reads, borrowed from its receiver.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot holding anything but an array: the slot is
/// written by [`nvs_core_db_connection_query`] and
/// [`nvs_core_db_connection_query_as`] out of one [`queried_rows`] and by
/// nothing else, so that is a paste error in this crate rather than anything a
/// program can cause.
pub(super) fn result_rows(
    args: &[Value],
    member: &str,
) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let receiver = crate::instance::receiver(args[0], &ROWS, member)?;
    let held = crate::instance::slot(receiver, ROWS_AT);
    let array = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{ROWS_NAME}::{member} found tag {} in its `{ROWS_SLOT}` slot",
            held.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// The class a [`ROWS`] hydrates its rows into, or `None` where they stay
/// [`ROW`]s — [`ROWS_CLASS_SLOT`], read by the three members that hand a row
/// out and by nothing else.
///
/// # Errors
///
/// [`crate::instance::receiver`]'s, for a receiver of the wrong class. The slot
/// itself cannot refuse: a value that is not a descriptor is the `null`
/// [`nvs_core_db_connection_query`] wrote.
pub(super) fn rows_class(
    args: &[Value],
    member: &str,
) -> Result<Option<*const nvs_runtime::ClassDesc>, Fault> {
    let receiver = crate::instance::receiver(args[0], &ROWS, member)?;
    Ok(crate::instance::slot(receiver, ROWS_CLASS_AT).as_class_desc())
}

/// One row of a [`ROWS`] as the object its member answers with: a [`ROW`] over
/// the very array the receiver holds, or — where [`rows_class`] named one — an
/// instance of the class `queryAs<T>`'s call site wrote.
///
/// # Errors
///
/// [`hydrate`]'s, for the second shape, and a [`Fault::fatal`] for a row slot
/// holding anything but an array, which is [`row_at`]'s paste error.
pub(super) fn row_object(
    ctx: &mut nvs_runtime::Ctx,
    row: Value,
    class: Option<*const nvs_runtime::ClassDesc>,
    member: &str,
) -> Result<Value, Fault> {
    let Some(class) = class else {
        return Ok(crate::instance::build(&ROW, [owned(row)]));
    };
    let held = row.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{ROWS_NAME}::{member} found tag {} where a row should be",
            row.tag_byte()
        ))
    })?;
    let columns = crate::arr::borrowed(held);
    #[expect(
        unsafe_code,
        reason = "the descriptor came out of a `ClassDescConst` the compiled unit \
                  owns, written into this receiver's own slot by \
                  `nvs_core_db_connection_query_as`, so it outlives this call"
    )]
    unsafe {
        hydrate(ctx, class, &columns)
    }
}

/// One row built into the class `queryAs<T>`'s call site wrote — `rule:core-classes/derive-reports-every-field`'s
/// accumulate-then-construct, over a row whose columns `rule:core-classes/db-column-types`'s type map
/// has already decoded.
///
/// **Every field is a check and not a parse**, which is the whole difference
/// from [`crate::json`]'s walk over the same [`nvs_runtime::CodecField`] list:
/// a column arrives as the Novis value § 9 names for its SQL type, so what is
/// left is whether that value is the one the field declares — and § 6's
/// "losslessly or throws" is what decides the two integer types against each
/// other, exactly as [`ROW`]'s own typed readers do.
///
/// **§ 5's `path` is the column name**, which that section says outright for
/// the `Db` half, so a list element's position rides in its message rather
/// than in a dotted path.
///
/// **Two doors, and the class picks which.**
/// `rule:core-classes/derive-generates-what-is-missing` makes the row format
/// one-directional, so `Core\Db\Codec` declares `fromRow` alone and a class
/// that writes it carries no attribute to go with it. Such a class records no
/// mapping, so an empty [`nvs_runtime::ClassDesc::db_codec`] is the *written*
/// member's door rather than a refusal: the row is handed over as the
/// [`ROW`] every other caller would have been given, and whatever the member
/// built is the instance. The mapping below is the other door, and
/// [ADR 0067 § 6](/docs/decisions/0067.md) admits them beside each other.
///
/// # Errors
///
/// A `ParseError` carrying every bad column at once — `ParseError` rather than
/// § 8's `DbError` because this module's known gap 2 is that the latter is not
/// in spec § 10's tree, and because `issues` is a property only the former
/// declares. A class that took neither door — no `#[Db\Derive]` and no
/// `fromRow` — is a `LogicError` instead: it is the program's mistake rather
/// than the row's, and `nvs_types::derive`'s `check_row_sites` says it as
/// `E0806` while compiling — what stays here is the backstop for a class built
/// by hand, which no call site named.
///
/// # Safety
///
/// `class` must refer to a live descriptor whose method table `nvs-codegen` has
/// filled.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
pub(super) unsafe fn hydrate(
    ctx: &mut nvs_runtime::Ctx,
    class: *const nvs_runtime::ClassDesc,
    row: &NvsArray,
) -> Result<Value, Fault> {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let desc = unsafe { &*class };
    let fields = desc.db_codec();
    if fields.is_empty() {
        return hand_written(ctx, desc, row);
    }
    let mut ctor_args = vec![Value::null(); desc.ctor_arity()];
    let mut filled = vec![false; desc.ctor_arity()];
    let mut issues: Vec<(String, String)> = Vec::new();
    for field in fields {
        let outcome = match row.get(field.key.as_bytes()) {
            Some(held) => hydrated(field, held).map(owned),
            // A column the result does not carry is this door's absent key, and
            // `rule:core-api/required-optional-and-nullable` answers it here as
            // `crate::json` answers it at the other one: a field whose
            // constructor parameter declares a default is filled from that
            // constant, which rides on the field because a hydration is not the
            // call site that would otherwise emit it.
            None => match &field.default {
                Some(default) => Ok(default.materialize()),
                None => Err(format!(
                    "the result has no column `{}` — a `#[Db\\Field(name: \"…\")]` is how a field \
                     reads one under another name",
                    field.key
                )),
            },
        };
        match outcome {
            Ok(value) => match ctor_args.get_mut(field.param) {
                Some(slot) => {
                    *slot = value;
                    filled[field.param] = true;
                }
                None => {
                    release_all(&ctor_args);
                    // Unreachable from source with no diagnostic to name:
                    // `field.param` and `desc.ctor_arity()` are two readings of
                    // one class's own constructor, both written while compiling
                    // that class.
                    return Err(Fault::fatal(format!(
                        "internal error: `{}`'s `{}` field names constructor parameter {} of {}",
                        desc.name(),
                        field.key,
                        field.param,
                        desc.ctor_arity()
                    )));
                }
            },
            Err(why) => issues.push((field.key.clone(), why)),
        }
    }

    if !issues.is_empty() {
        release_all(&ctor_args);
        return Err(Fault::thrown_with_issues(
            ThrownClass::Parse,
            format!(
                "{QUERY_AS}: {} column(s) of `{}` did not match the row",
                issues.len(),
                desc.name()
            ),
            crate::issue::list(
                issues
                    .iter()
                    .map(|(path, message)| (path.as_str(), message.as_str())),
            ),
        ));
    }
    // `rule:core-classes/derive-field-list`'s skipped field, exactly as
    // `Core\Json::decodeAs` meets it: a field the mapping carries is filled from
    // its own default above, and a property `skip: true` removed from the
    // mapping leaves a parameter no field names, so there is no field here to
    // read a constant off. `nvs_types::derive`'s `check_row_sites` refuses a
    // `queryAs<T>` naming such a class while compiling, as `check_json_sites`
    // does at the document door; this stays as the backstop for a class built
    // by hand.
    if let Some(index) = filled.iter().position(|done| !done) {
        release_all(&ctor_args);
        return Err(Fault::fatal(format!(
            "{QUERY_AS}: `{}`'s constructor parameter {index} is not a codec field, which \
             `E0806` refuses at the call that names such a class",
            desc.name()
        )));
    }
    #[expect(
        unsafe_code,
        reason = "the same live descriptor, and every argument is one this frame \
                  owns and hands over"
    )]
    unsafe {
        nvs_runtime::construct(ctx, class, &ctor_args)
    }
}

/// The one member `Core\Db\Codec` declares —
/// `static fromRow(Db\Row $row): static`. `nvs_types::derive` owns the spelling
/// as the half a `#[Db\Derive]` generates, and the two never both exist on one
/// class: `E0757` refuses an attribute that would generate what the class
/// already wrote.
const FROM_ROW: &str = "fromRow";

/// [`hydrate`]'s other door: the row handed to the [`FROM_ROW`] the class wrote
/// itself, as the [`ROW`] a `query` would have answered with.
///
/// The object is built here rather than borrowed from a caller because
/// [`hydrate`]'s two callers hold the columns and not a row object — a
/// `stream`'s row is one it has just decoded, and a `queryAs`'s is a slot of
/// the result array. It is this frame's own reference for the length of the
/// call: [`nvs_runtime::call_static_on`] retains every argument it passes, so
/// the release below is what frees it and the array it wraps.
///
/// # Errors
///
/// [`ThrownClass::Logic`] where the class declares no [`FROM_ROW`] either,
/// which is the backstop [`hydrate`]'s own docs describe, and
/// [`Fault::Pending`] where the member itself threw.
fn hand_written(
    ctx: &mut nvs_runtime::Ctx,
    desc: &nvs_runtime::ClassDesc,
    row: &NvsArray,
) -> Result<Value, Fault> {
    let written = crate::instance::build(&ROW, [Value::array(row.clone())]);
    #[expect(
        unsafe_code,
        reason = "the caller of `hydrate` owes the liveness of the descriptor, \
                  and the row object is this frame's own reference"
    )]
    let called =
        unsafe { nvs_runtime::call_static_on(ctx, std::ptr::from_ref(desc), FROM_ROW, &[written]) };
    release_all(&[written]);
    called?.ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{QUERY_AS}: `{}` carries no `#[Db\\Derive]` and declares no `{FROM_ROW}`, so \
                 there is neither a column mapping to build one from nor a member to hand the row \
                 to — `rule:core-classes/derive-attribute`'s opt-in is that attribute, and \
                 `rule:core-classes/derive-generates-what-is-missing` is the hand-written half \
                 beside it. `E0806` is where a call site that wrote such a class is refused while \
                 compiling",
                desc.name()
            ),
        )
    })
}

/// One column as the value one [`nvs_runtime::CodecField`] takes, borrowed from
/// the row — or § 5's message for why it is not that value.
///
/// The reference is *not* taken here: [`hydrate`] does that with [`owned`] on
/// the one value it keeps, so a list's element checks below cost nothing and
/// leak nothing.
pub(super) fn hydrated(field: &nvs_runtime::CodecField, held: Value) -> Result<Value, String> {
    if held.tag() == Some(Tag::Null) {
        return if field.nullable {
            Ok(held)
        } else {
            Err(
                "the column is SQL NULL and the field is not declared `?T` — `rule:core-classes/db-column-types` reads a \
                 NULL back as `null` whatever the column's type is"
                    .to_owned(),
            )
        };
    }
    let nvs_runtime::CodecTy::List = field.ty else {
        return converted(field.ty, field.cases.as_ref(), field.class.as_deref(), held);
    };
    let Some(element) = field.element.as_ref() else {
        return Err(
            "this field is a list whose element type the derive pass did not record".to_owned(),
        );
    };
    // A column is one value, so a row's list is flat: `nvs_types::derive`'s
    // row-site check refuses a nested list or an inline shape at the
    // declaration, and this is the backstop for a descriptor built by hand.
    if element.element.is_some() {
        return Err(
            "this field is a list of lists, and `rule:core-classes/db-column-types` maps no \
             column to one"
                .to_owned(),
        );
    }
    let held_ptr = held
        .array_ptr()
        .ok_or_else(|| wanted("an `array<T>` column", held))?;
    let elements = crate::arr::borrowed(held_ptr);
    let mut from = 0usize;
    while let Some(slot) = elements.next_slot(from) {
        let one = elements
            .value_at(slot)
            .expect("next_slot only names live entries");
        // A NULL element is taken as it comes: a list field's element carries
        // no nullability of its own on `CodecField`, and PostgreSQL's array
        // types all admit one.
        if one.tag() != Some(Tag::Null) {
            converted(
                element.ty,
                field.cases.as_ref(),
                field.class.as_deref(),
                one,
            )
            .map_err(|why| format!("element {slot}: {why}"))?;
        }
        from = slot + 1;
    }
    Ok(held)
}

/// One value against one wire type: itself where it already is that type, the
/// same number under the other integer tag where `rule:core-classes/db-column-types`'s "losslessly or
/// throws" allows it, and § 5's message otherwise.
///
/// `class` is the rendered name the *declaration* carried, where `ty` is a
/// [`nvs_runtime::CodecTy::Class`]: the field's own class for a scalar field
/// and the element's for a list's element, which is exactly how
/// [`nvs_runtime::CodecField::class`] holds it — so both callers hand over the
/// same field's, and neither has to know which of the two it is.
///
/// Never a heap value it did not receive, so nothing here allocates or takes a
/// reference — see [`hydrated`].
pub(super) fn converted(
    ty: nvs_runtime::CodecTy,
    cases: Option<&nvs_runtime::EnumCases>,
    class: Option<&str>,
    held: Value,
) -> Result<Value, String> {
    use nvs_runtime::CodecTy;

    match ty {
        // `rule:types/declaration`'s `mixed`: whatever the column held, unchecked.
        CodecTy::Mixed => Ok(held),
        // § 6's three crossings, and a field asks for one in exactly the words
        // a `Db\Row` reader does: the helpers below are the rule's one home, so
        // `queryAs<T>` cannot drift from `->bool()` the way two copies would.
        CodecTy::Bool => match requested_bool(held) {
            Requested::Is(flag) => Ok(Value::bool(flag)),
            Requested::Lossy(holds) => Err(lossy("`bool`", &holds)),
            Requested::Mismatched => Err(wanted("`bool`", held)),
        },
        CodecTy::Int => match requested_int(held) {
            Requested::Is(number) => Ok(Value::int(number)),
            Requested::Lossy(holds) => Err(lossy("`int`", &holds)),
            Requested::Mismatched => Err(wanted("`int`", held)),
        },
        CodecTy::Uint => match requested_uint(held) {
            Requested::Is(number) => Ok(Value::uint(number)),
            Requested::Lossy(holds) => Err(lossy("`uint`", &holds)),
            Requested::Mismatched => Err(wanted("`uint`", held)),
        },
        CodecTy::Float => match held.tag() {
            Some(Tag::Float) => Ok(held),
            _ => Err(wanted("`float`", held)),
        },
        CodecTy::Str => match held.tag() {
            Some(Tag::Str) => Ok(held),
            _ => Err(wanted("`string`", held)),
        },
        // § 9 maps `DECIMAL`/`NUMERIC`/`MONEY` to this and nothing else, so a
        // value reaching here already carries every digit the column held —
        // [`column_value`] is where a scale the type cannot hold is refused
        // (`rule:types/decimal`), before any field asks. What is left is the
        // question the arms above ask: is it what the field declared.
        CodecTy::Decimal => match held.tag() {
            Some(Tag::Decimal) => Ok(held),
            _ => Err(wanted("`decimal`", held)),
        },
        // § 9's binary families. A `bytes` and a `string` point at the same
        // heap shape and are told apart by the tag alone, which is exactly the
        // distinction § 6 makes when it hands a `BLOB` back tainted.
        CodecTy::Bytes => match held.tag() {
            Some(Tag::Bytes) => Ok(held),
            _ => Err(wanted("`bytes`", held)),
        },
        // `rule:enums/representation`: a case *is* the integer behind it by the time it is a
        // `Value`, so this is a membership test and not a construction.
        CodecTy::Enum => {
            let Some(cases) = cases else {
                return Err(
                    "this field is an enum whose cases the derive pass did not record".to_owned(),
                );
            };
            let backing = held
                .as_int()
                .map(i128::from)
                .or_else(|| held.as_uint().map(i128::from))
                .ok_or_else(|| wanted("an enum's backing integer", held))?;
            if cases.values.binary_search(&backing).is_err() {
                return Err(format!(
                    "the column holds {backing}, which this enum declares no case for"
                ));
            }
            Ok(if cases.unsigned {
                #[expect(
                    clippy::cast_sign_loss,
                    clippy::cast_possible_truncation,
                    reason = "the value is one of the declared cases, which a `uint`-backed \
                              enum's are all `u64`"
                )]
                Value::uint(backing as u64)
            } else {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the value is one of the declared cases, which an `int`-backed \
                              enum's are all `i64`"
                )]
                Value::int(backing as i64)
            })
        }
        // A row is a flat list of columns and `nvs_types::derive`'s own
        // `db_reachable` maps none of them to a nested class, so this arm is
        // § 9's own value types and nothing else — the `Instant` that carries a
        // wire type of its own, and the rest, which the erasure leaves as a
        // class label. Both are answered the same way. The instance is built long
        // before hydration reads it — [`column_value`] is where a driver's
        // components become a `Core\Time` or a `Core\Uuid`, and
        // [`sqlite_column_value`] is where the one driver that sends no
        // components does it off the column's declaration instead — so all this
        // arm asks is the question every other one asks: is it what the field
        // declared.
        //
        // By rendered name rather than by descriptor pointer, because the two
        // sides are written at different times: the label is what
        // `nvs_types::derive` read off the declaration, and the descriptor is
        // the one `crate::instance` gave the value the driver's components
        // built. `rule:core-api/tier-placement` keeps the `Core\` prefix for Tier 0, so no program
        // can declare a second class answering to one of these five names.
        CodecTy::Class | CodecTy::Instant => {
            let Some(declared) = class else {
                return Err(
                    "this field is a class whose name the derive pass did not record".to_owned(),
                );
            };
            let Some(object) = held.obj_ptr() else {
                return Err(wanted(&format!("`{declared}`"), held));
            };
            #[expect(
                unsafe_code,
                reason = "the value argument owns a reference to a live allocation, \
                          so it is live for the length of this call"
            )]
            let found = unsafe { &*nvs_runtime::NvsObj::class_of(object) };
            if found.name() == declared {
                Ok(held)
            } else {
                Err(format!(
                    "the column came back as a `{}` and this field declares `{declared}` — ADR \
                     0067 § 9's type map is what each column reads back as",
                    found.name()
                ))
            }
        }
        // `nvs_types::derive` erases an inline shape to this, and its own gap 1
        // owns the erasure. `check_row_sites` says this as `E0806` while
        // compiling; what is left here is the backstop for a class built by
        // hand, which no call site named.
        // An inline shape is not a column type at all
        // (`rule:core-classes/db-column-types`), which `nvs_types::derive`'s
        // reachable set refuses at the declaration, so both of these are the
        // backstop for a class built by hand and named by no call site.
        CodecTy::Opaque | CodecTy::Shape => Err(
            "this field's declared type is one no column reads back — an inline \
                                shape"
                .to_owned(),
        ),
        // Unreachable: [`hydrated`] takes the list arm before this is called,
        // and a list's element is never itself a list (`CodecTy::List`).
        CodecTy::List => Err("a list of lists is not a column type".to_owned()),
    }
}

/// § 5's message for a column that came back as something else, said with what
/// it actually is — [`wrong_column_type`]'s shape, for the walk that has a
/// field's declared type in hand rather than a reader's name.
pub(super) fn wanted(want: &str, held: Value) -> String {
    format!(
        "the column came back as {} and this field declares {want} — `rule:core-classes/db-column-types`'s type map is \
         what each column reads back as",
        held.tag().map_or_else(
            || format!("tag {}", held.tag_byte()),
            |tag| tag.describe().to_owned()
        )
    )
}

/// [`wanted`]'s other half, and [`column_out_of_range`]'s counterpart on this
/// side: the column is of the family the field declares and holds a value that
/// does not survive the crossing. `holds` is [`Requested::Lossy`]'s fragment, so
/// a field and a reader say the same thing about the same value.
pub(super) fn lossy(want: &str, holds: &str) -> String {
    format!(
        "the column holds {holds}, so reading it as {want} would not be the same value — `rule:core-classes/db-one-api` \
         § 6 converts losslessly or throws"
    )
}

/// Releases every reference in `values` — [`nvs_runtime::construct`]'s "an
/// argument is consumed whether or not the constructor ran", owed by every path
/// out of [`hydrate`] that does not reach it.
pub(super) fn release_all(values: &[Value]) {
    for value in values {
        #[expect(
            unsafe_code,
            reason = "each entry is either `null` or a value this frame took a \
                      reference to in `hydrate`"
        )]
        unsafe {
            value.release();
        }
    }
}

/// One row of a [`ROWS`], borrowed — see [`result_rows`] for the refusal.
pub(super) fn row_at(
    rows: &NvsArray,
    slot: usize,
    member: &str,
) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let held = rows
        .value_at(slot)
        .expect("next_slot only names live entries");
    let array = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{ROWS_NAME}::{member} found tag {} where a row should be",
            held.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// The columns of the [`ROW`] one of its members was called on, borrowed —
/// [`result_rows`]'s twin, and its refusal is the same paste error.
pub(super) fn row_columns(
    args: &[Value],
    member: &str,
) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let receiver = crate::instance::receiver(args[0], &ROW, member)?;
    let held = crate::instance::slot(receiver, COLUMNS_AT);
    let array = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{ROW_NAME}::{member} found tag {} in its `{COLUMNS_SLOT}` slot",
            held.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// A column-name argument as the bytes an array is keyed by.
///
/// # Errors
///
/// A [`Fault::fatal`], because every row that takes one declares it `string` and
/// a non-text argument is refused at `E0401` first.
pub(super) fn column_name<'a>(
    value: &'a Value,
    class: &str,
    member: &str,
) -> Result<&'a [u8], Fault> {
    value.as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{class}::{member} expected a `string` column name, got tag {}",
            value.tag_byte()
        ))
    })
}

/// Spec § 18's "An unknown column name throws", with the names that would have
/// worked — the one thing a caller holding a misspelling wants next.
pub(super) fn unknown_column(class: &str, member: &str, name: &[u8], columns: &NvsArray) -> Fault {
    let known: Vec<String> = columns
        .keys()
        .iter()
        .map(|key| String::from_utf8_lossy(key).into_owned())
        .collect();
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{class}::{member}: no column is named `{}` — this row has {}",
            String::from_utf8_lossy(name),
            if known.is_empty() {
                "none at all".to_owned()
            } else {
                known.join(", ")
            }
        ),
    )
}

/// The value at one column *position*, counted from zero over the server's own
/// description, or `None` for a position the row does not reach — the `int` arm
/// of [`nvs_core_db_rows_column`]'s `int|string` key.
pub(super) fn column_at(row: &NvsArray, index: i64) -> Option<Value> {
    let index = usize::try_from(index).ok()?;
    let mut from = 0usize;
    for _ in 0..index {
        from = row.next_slot(from)? + 1;
    }
    row.next_slot(from).and_then(|slot| row.value_at(slot))
}

/// The column one of [`ROW`]'s eleven typed readers was asked for, borrowed and
/// paired with the name it was asked by, or `None` where the column is SQL
/// `NULL` — which is the `?T` every one of them answers.
///
/// # Errors
///
/// The [`unknown_column`] throw, and [`row_columns`]'s and [`column_name`]'s
/// fatals.
pub(super) fn typed_column<'a>(
    args: &'a [Value],
    member: &str,
) -> Result<(&'a [u8], Option<Value>), Fault> {
    let columns = row_columns(args, member)?;
    let name = column_name(&args[1], ROW_NAME, member)?;
    let value = columns
        .get(name)
        .ok_or_else(|| unknown_column(ROW_NAME, member, name, &columns))?;
    Ok((name, (value.tag() != Some(Tag::Null)).then_some(value)))
}

/// A typed reader's refusal for a column it will not convert — `rule:core-classes/db-column-types`'s
/// "lossless conversion or throws", said with what the column actually is.
pub(super) fn wrong_column_type(member: &str, name: &[u8], value: Value, want: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{ROW_NAME}::{member}: the column `{}` came back as {} and this reader answers {want} \
             only — `rule:core-classes/db-column-types` converts losslessly or throws, and `->get()` plus `as` is the \
             universal path",
            String::from_utf8_lossy(name),
            value.tag().map_or_else(
                || format!("tag {}", value.tag_byte()),
                |tag| tag.describe().to_owned()
            )
        ),
    )
}

/// The other half of that refusal: the column *is* an integer, and the reader
/// asked for is the one of `int`/`uint` it does not fit.
pub(super) fn column_out_of_range(member: &str, name: &[u8], holds: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{ROW_NAME}::{member}: the column `{}` holds {holds}, so reading it as `{member}` \
             would not be the same value — `rule:core-classes/db-column-types` converts losslessly or throws",
            String::from_utf8_lossy(name)
        ),
    )
}

/// What one of `rule:core-classes/db-column-types`'s *requests*
/// makes of the value a column's natural type already produced.
///
/// Three answers rather than two, because the two refusals are different
/// questions and a caller says so in different words: [`Self::Lossy`] is the
/// right family and a value that does not survive the crossing, while
/// [`Self::Mismatched`] is a family with no crossing to consider at all. A
/// `DECIMAL` asked for `float` is the second and not the first — `rule:types/decimal` keeps
/// those apart by construction, so there is no value of one that is a value of
/// the other.
pub(super) enum Requested<T> {
    /// § 6's "losslessly".
    Is(T),
    /// The right family, the wrong value, carrying the fragment that says which
    /// — the number, and why it does not cross. Built here so the reader's
    /// sentence and the `#[Db\Derive]` field's quote one wording.
    Lossy(String),
    /// The wrong family.
    Mismatched,
}

/// § 6's `bool` request: the one crossing in the map that is neither a column's
/// natural type nor a refusal.
///
/// `BOOLEAN` and `BIT(1)` arrive as `Tag::Bool` already. MySQL and MariaDB have
/// neither, and § 9 reads their `TINYINT(1)` as `int` because a display width is
/// not a type and nothing on the wire separates a flag column from a small
/// integer — so the *request* is what decides, and § 6 says outright that it
/// decides this one. `0` and `1` are the whole of what a flag column holds; a
/// stored `7` throws rather than reading as PHP's `true`, which is the half of
/// this rule that keeps the conversion lossless.
pub(super) fn requested_bool(value: Value) -> Requested<bool> {
    if let Some(flag) = value.as_bool() {
        return Requested::Is(flag);
    }
    let number = match (value.as_int(), value.as_uint()) {
        (Some(signed), _) => i128::from(signed),
        (_, Some(unsigned)) => i128::from(unsigned),
        _ => return Requested::Mismatched,
    };
    match number {
        0 => Requested::Is(false),
        1 => Requested::Is(true),
        _ => Requested::Lossy(format!("{number}, which is neither `0` nor `1`")),
    }
}

/// § 6's `int` request, which crosses from the other half of `rule:types/arithmetic`'s one
/// integer and stops where `int` does.
pub(super) fn requested_int(value: Value) -> Requested<i64> {
    if let Some(signed) = value.as_int() {
        return Requested::Is(signed);
    }
    let Some(unsigned) = value.as_uint() else {
        return Requested::Mismatched;
    };
    i64::try_from(unsigned).map_or_else(
        |_| Requested::Lossy(format!("{unsigned}, which is past `int`'s ceiling")),
        Requested::Is,
    )
}

/// § 6's `uint` request — [`requested_int`]'s twin, and what `BIGINT UNSIGNED`
/// needs: PHP overflows that column to a `float` and stops comparing equal to
/// itself.
pub(super) fn requested_uint(value: Value) -> Requested<u64> {
    if let Some(unsigned) = value.as_uint() {
        return Requested::Is(unsigned);
    }
    let Some(signed) = value.as_int() else {
        return Requested::Mismatched;
    };
    u64::try_from(signed).map_or_else(
        |_| Requested::Lossy(format!("{signed}, which is below `uint`'s floor")),
        Requested::Is,
    )
}

nvs_runtime::nvs_helper! {
    /// `$rows->all(): array<Db\Row>` — every row at once, replacing
    /// `PDO::fetchAll` and the fetch-mode argument that chose its shape.
    ///
    /// One object per row and no second array: a [`ROW`]'s slot takes a
    /// reference to the row [`ROWS`] already holds ([`COLUMNS_SLOT`]), so what
    /// this spends over a result already in memory is one small object each.
    fn nvs_core_db_rows_all(ctx, args: [1]) {
        let rows = result_rows(args, "all")?;
        let class = rows_class(args, "all")?;
        let mut all = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = rows.next_slot(from) {
            let row = rows
                .value_at(slot)
                .expect("next_slot only names live entries");
            all.append(row_object(ctx, row, class, "all")?);
            from = slot + 1;
        }
        Ok(Value::array(all))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<Row>::iterate(): Iterator<Row>` — a cursor over one [`ROW`]
    /// per row this value is holding.
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed,
    /// which is why this body releases it and [`nvs_core_db_rows_all`] does
    /// not. [`crate::cursor`]'s module docs own both halves of that.
    ///
    /// The objects are built here rather than shared with a previous `all()`,
    /// because there need not have been one — and it costs no more than `all()`
    /// does for the same reason: a [`ROW`]'s slot takes a reference to the row
    /// [`ROWS`] already holds, so a `foreach` over a thousand rows allocates a
    /// thousand small objects and not a second thousand arrays. The snapshot
    /// every § 9 collection's `iterate()` has to take is free here as well:
    /// [`ROWS`] has no mutating member, so the array was already frozen when
    /// [`nvs_core_db_connection_query`] built it.
    fn nvs_core_db_rows_iterate(ctx, args: [1]) {
        let cursor = (|| {
            let rows = result_rows(args, nvs_runtime::sequence::ITERATE)?;
            let class = rows_class(args, nvs_runtime::sequence::ITERATE)?;
            let mut all = NvsArray::new();
            let mut from = 0usize;
            while let Some(slot) = rows.next_slot(from) {
                let row = rows
                    .value_at(slot)
                    .expect("next_slot only names live entries");
                all.append(row_object(ctx, row, class, nvs_runtime::sequence::ITERATE)?);
                from = slot + 1;
            }
            Ok(crate::cursor::over(all))
        })();
        crate::cursor::consume(args[0]);
        cursor
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->first(): ?Db\Row` — the first row, or `null` for none.
    ///
    /// `null` rather than a throw, and rather than PHP's `false`: `rule:core-api/shape-rules` R5
    /// makes `?T` the only absence spelling, and a `select` that matched
    /// nothing is an answer rather than a failure. There is no cursor a second
    /// call would move past, either — this is the first row every time, which
    /// is what makes it safe to write in a condition.
    fn nvs_core_db_rows_first(ctx, args: [1]) {
        let rows = result_rows(args, "first")?;
        let Some(slot) = rows.next_slot(0) else {
            return Ok(Value::null());
        };
        let class = rows_class(args, "first")?;
        let row = rows
            .value_at(slot)
            .expect("next_slot only names live entries");
        row_object(ctx, row, class, "first")
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->value(): mixed` — the first column of the first row, replacing
    /// `PDOStatement::fetchColumn`.
    ///
    /// **An empty result is `null`, which a NULL column is too**, and the two
    /// are not told apart here. The declared type is § 18's `mixed`, so there
    /// is no `?T` to put the absence in that the value itself could not
    /// already be; a caller that has to distinguish them asks `count()`, which
    /// is exact. The alternative — throwing on an empty result — would make
    /// the commonest use, a `select count(*)`, the one shape that has to be
    /// wrapped in a `try`.
    fn nvs_core_db_rows_value(_ctx, args: [1]) {
        let rows = result_rows(args, "value")?;
        let Some(slot) = rows.next_slot(0) else {
            return Ok(Value::null());
        };
        let row = row_at(&rows, slot, "value")?;
        Ok(match row.next_slot(0) {
            Some(at) => owned(
                row.value_at(at)
                    .expect("next_slot only names live entries"),
            ),
            None => Value::null(),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->column(int|string $key): array<mixed>` — one column from every
    /// row, replacing `PDO::fetchAll(PDO::FETCH_COLUMN)`.
    ///
    /// The key is checked against each row rather than once, because the check
    /// *is* the lookup: rows all carry the same columns, so the first row
    /// decides and the rest cost a hash lookup each. An empty result therefore
    /// answers an empty array for a key that names nothing — it describes no
    /// columns for the key to be wrong about, and `columns()` is the member
    /// that answers what a statement described.
    fn nvs_core_db_rows_column(_ctx, args: [2]) {
        let rows = result_rows(args, "column")?;
        let mut taken = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = rows.next_slot(from) {
            let row = row_at(&rows, slot, "column")?;
            let found = if let Some(index) = args[1].as_int() {
                column_at(&row, index).ok_or_else(|| {
                    Fault::thrown_as(
                        ThrownClass::Logic,
                        format!(
                            "{ROWS_NAME}::column: there is no column at position {index} — this \
                             row has {}, counted from zero",
                            row.count()
                        ),
                    )
                })?
            } else {
                let name = column_name(&args[1], ROWS_NAME, "column")?;
                row.get(name)
                    .ok_or_else(|| unknown_column(ROWS_NAME, "column", name, &row))?
            };
            taken.append(owned(found));
            from = slot + 1;
        }
        Ok(Value::array(taken))
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->count(): uint` — how many rows there are, replacing
    /// `PDOStatement::rowCount` on a select.
    ///
    /// Exact, and that is § 4's buffered default paying for itself: every row
    /// was read before `query` answered, so this is a length rather than the
    /// driver-dependent guess `rowCount` is on a select.
    fn nvs_core_db_rows_count(_ctx, args: [1]) {
        let rows = result_rows(args, "count")?;
        let held = rows.count();
        let count = u64::try_from(held).map_err(|_| {
            Fault::fatal(format!("{ROWS_NAME}::count: {held} rows do not fit a `uint`"))
        })?;
        Ok(Value::uint(count))
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->columns(): array<Db\Column>` — what the statement described,
    /// replacing `PDOStatement::getColumnMeta` and `mysqli_fetch_fields`.
    ///
    /// A reader over [`ROWS_COLUMNS_SLOT`] and nothing more: the objects were
    /// built when the result was ([`described_columns`]), so this hands that
    /// array on under a second reference exactly as a [`ROW`] takes one of the
    /// row it reads. Two calls answer the same columns rather than two
    /// descriptions of them, and a result set that matched no rows answers the
    /// same thing a matching one would.
    fn nvs_core_db_rows_columns(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &ROWS, "columns")?;
        Ok(owned(crate::instance::slot(receiver, ROWS_COLUMNS_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$column->name(): string` — the label the server described this column
    /// with, replacing `getColumnMeta`'s `name` key.
    fn nvs_core_db_column_name(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &COLUMN, "name")?;
        Ok(owned(crate::instance::slot(receiver, LABEL_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$column->type(): Db\ColumnType` — what the column was declared as, as
    /// the case [`COLUMN_TYPE`] registers rather than the vendor type name
    /// `getColumnMeta` answers.
    ///
    /// The slot already holds the ordinal an enum is at runtime, written there
    /// by [`column_type_value`], so nothing is classified here: a description
    /// is of the statement and a statement is described once.
    fn nvs_core_db_column_type(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &COLUMN, "type")?;
        Ok(crate::instance::slot(receiver, DECLARED_AT))
    }
}

nvs_runtime::nvs_helper! {
    /// `$column->nullable(): bool` — whether the column may hold NULL, which
    /// on this driver is always `true` and [`COLUMN_NULLABLE_DOC`] is where a
    /// program's author reads why.
    fn nvs_core_db_column_nullable(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &COLUMN, "nullable")?;
        Ok(crate::instance::slot(receiver, NULLABLE_AT))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->has(string $name): bool` — whether the row carries this column.
    ///
    /// A NULL column is present, which is the whole point of asking: the
    /// typed readers answer `null` for both "this column is NULL" and nothing
    /// else, so this is where "there is no such column" is told from it.
    fn nvs_core_db_row_has(_ctx, args: [2]) {
        let columns = row_columns(args, "has")?;
        let name = column_name(&args[1], ROW_NAME, "has")?;
        Ok(Value::bool(columns.has_key(name)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->get(string $name): mixed` — one column, whatever `rule:core-classes/db-column-types`'s
    /// type map made of it, and the universal path the typed readers narrow.
    fn nvs_core_db_row_get(_ctx, args: [2]) {
        let columns = row_columns(args, "get")?;
        let name = column_name(&args[1], ROW_NAME, "get")?;
        let found = columns
            .get(name)
            .ok_or_else(|| unknown_column(ROW_NAME, "get", name, &columns))?;
        Ok(owned(found))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->toArray(): array<string, mixed>` — the whole row, replacing
    /// `FETCH_ASSOC`.
    ///
    /// The slot's own array under a second reference rather than a copy: an
    /// Novis array is a value with copy-on-write, so a caller that writes to
    /// what it got here separates it and the row is untouched, and a caller
    /// that only reads pays nothing at all.
    fn nvs_core_db_row_to_array(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &ROW, "toArray")?;
        Ok(owned(crate::instance::slot(receiver, COLUMNS_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->string(string $name): ?string` — the text family alone, which
    /// [`ROW`]'s own docs state the rule for.
    fn nvs_core_db_row_string(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "string")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if value.as_str_bytes().is_none() {
            return Err(wrong_column_type("string", name, value, "`string`"));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->bytes(string $name): ?bytes` — [`nvs_core_db_row_string`]'s twin
    /// on `rule:types/bytes`'s other side.
    fn nvs_core_db_row_bytes(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "bytes")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if value.as_bytes().is_none() {
            return Err(wrong_column_type("bytes", name, value, "`bytes`"));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->int(string $name): ?int` — the signed half of `rule:types/arithmetic`'s one
    /// integer, and one of the two readers that cross.
    fn nvs_core_db_row_int(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "int")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        match requested_int(value) {
            Requested::Is(number) => Ok(Value::int(number)),
            Requested::Lossy(holds) => Err(column_out_of_range("int", name, &holds)),
            Requested::Mismatched => Err(wrong_column_type("int", name, value, "an integer")),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->uint(string $name): ?uint` — [`nvs_core_db_row_int`]'s unsigned
    /// twin, and what `BIGINT UNSIGNED` needs: PHP overflows that column to a
    /// `float` and stops comparing equal to itself.
    fn nvs_core_db_row_uint(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "uint")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        match requested_uint(value) {
            Requested::Is(number) => Ok(Value::uint(number)),
            Requested::Lossy(holds) => Err(column_out_of_range("uint", name, &holds)),
            Requested::Mismatched => Err(wrong_column_type("uint", name, value, "an integer")),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->float(string $name): ?float` — `FLOAT`, `REAL` and `DOUBLE`.
    ///
    /// A `DECIMAL` is refused rather than widened: that conversion is the one
    /// this whole type map exists to stop happening by accident.
    fn nvs_core_db_row_float(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "float")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        value
            .as_float()
            .map(Value::float)
            .ok_or_else(|| wrong_column_type("float", name, value, "`float`"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->bool(string $name): ?bool` — `BOOLEAN` and `BIT(1)`, and on
    /// request `TINYINT(1)` as well.
    ///
    /// The one reader whose answer is not the column's natural type: MySQL and
    /// MariaDB have no boolean column at all, so § 6 makes the *request* what
    /// converts a `0`/`1` integer here. [`requested_bool`] owns the rule and the
    /// stored `7` it refuses.
    fn nvs_core_db_row_bool(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "bool")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        match requested_bool(value) {
            Requested::Is(flag) => Ok(Value::bool(flag)),
            Requested::Lossy(holds) => Err(column_out_of_range("bool", name, &holds)),
            Requested::Mismatched => Err(wrong_column_type("bool", name, value, "`bool`")),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->decimal(string $name): ?decimal` — `rule:types/decimal`'s exact scalar, where
    /// PHP hands back a string and leaves the parsing to the caller.
    fn nvs_core_db_row_decimal(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "decimal")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        value
            .as_decimal()
            .map(Value::decimal)
            .ok_or_else(|| wrong_column_type("decimal", name, value, "`decimal`"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->instant(string $name): ?Core\Time\Instant` — `TIMESTAMPTZ` and
    /// `datetimeoffset`.
    ///
    /// One of the four [`ROW`]'s docs name: the value is the `Core\Time\Instant`
    /// [`column_value`] built out of the column, so this member is the lookup
    /// and the class check and nothing else.
    fn nvs_core_db_row_instant(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "instant")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if !crate::instance::is_instance(value, &crate::time::INSTANT) {
            return Err(wrong_column_type(
                "instant",
                name,
                value,
                "a `Core\\Time\\Instant`",
            ));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->date(string $name): ?Core\Time\Date` — a `DATE`, and one of
    /// [`nvs_core_db_row_instant`]'s four.
    fn nvs_core_db_row_date(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "date")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if !crate::instance::is_instance(value, &crate::time::DATE) {
            return Err(wrong_column_type("date", name, value, "a `Core\\Time\\Date`"));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->time(string $name): ?Core\Time\TimeOfDay` — a `TIME`, and one of
    /// [`nvs_core_db_row_instant`]'s four.
    fn nvs_core_db_row_time(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "time")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if !crate::instance::is_instance(value, &crate::time::TIME_OF_DAY) {
            return Err(wrong_column_type(
                "time",
                name,
                value,
                "a `Core\\Time\\TimeOfDay`",
            ));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->uuid(string $name): ?Core\Uuid` — a native `UUID` column, and the
    /// last of [`nvs_core_db_row_instant`]'s four.
    fn nvs_core_db_row_uuid(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "uuid")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if !crate::instance::is_instance(value, &crate::uuid::CLASS) {
            return Err(wrong_column_type("uuid", name, value, "a `Core\\Uuid`"));
        }
        Ok(owned(value))
    }
}

/// One of a [`WRITE`]'s three counts, read back out of its slot.
///
/// No reference is taken, unlike [`owned`]'s readers: every one of these slots
/// holds a `uint` or a `null`, and neither owns anything to retain.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot holding any other tag. All three are written
/// by [`nvs_core_db_connection_execute`] and by nothing else, so that is a
/// paste error in this crate rather than anything a program can cause.
pub(super) fn write_count(args: &[Value], member: &str, at: usize) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &WRITE, member)?;
    let held = crate::instance::slot(receiver, at);
    if held.as_uint().is_none() && held.tag() != Some(Tag::Null) {
        return Err(Fault::fatal(format!(
            "{WRITE_NAME}::{member} found tag {} in its `{}` slot",
            held.tag_byte(),
            WRITE.slots[at]
        )));
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `$write->affected(): uint` — how many rows the statement affected, and
    /// `0` where its kind reports no count at all.
    fn nvs_core_db_write_affected(_ctx, args: [1]) {
        write_count(args, "affected", AFFECTED_AT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$write->changed(): ?uint` — the same count as the server reported it,
    /// whose `null` is what [`nvs_core_db_write_affected`] folds to `0`.
    fn nvs_core_db_write_changed(_ctx, args: [1]) {
        write_count(args, "changed", CHANGED_AT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$write->lastId(): ?uint` — `rule:core-classes/db-statement-members`'s id, which on PostgreSQL is
    /// whatever a `RETURNING` clause handed back and belongs to this write
    /// rather than to the connection.
    fn nvs_core_db_write_last_id(_ctx, args: [1]) {
        write_count(args, "lastId", LAST_ID_AT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::{Ctx, Decimal, OutputSink};

    /// A described column is the three slots [`COLUMN`] declares, in the order
    /// its readers name — a paste error [`crate::instance::build`]'s arity
    /// assertion cannot catch, since all three are one class's.
    #[test]
    fn a_column_holds_its_label_its_type_and_its_nullability() {
        assert_eq!(COLUMN.slots, [LABEL_SLOT, DECLARED_SLOT, NULLABLE_SLOT]);
        assert_eq!(COLUMN.slot(LABEL_SLOT), LABEL_AT);
        assert_eq!(COLUMN.slot(DECLARED_SLOT), DECLARED_AT);
        assert_eq!(COLUMN.slot(NULLABLE_SLOT), NULLABLE_AT);
        assert_eq!(ROWS.slot(ROWS_COLUMNS_SLOT), ROWS_COLUMNS_AT);
    }

    /// Drops the one reference a member handed this frame, which is what a
    /// compiled caller owes for every value it was given.
    fn released(value: Value) {
        #[expect(
            unsafe_code,
            reason = "the reference released here is the one this frame was \
                      handed, and nothing else holds it"
        )]
        unsafe {
            value.release();
        }
    }

    /// **`Core\Db\Column::name` answers the label the server described**, which
    /// is the alias wherever the select list wrote one — [`COLUMN_LABEL_DOC`]'s
    /// sentence, and the half of it a type-level case cannot reach.
    ///
    /// Asserted against a real engine on purpose. Every other `Core\Db` case in
    /// this crate asks what a member *declares*, and a label is not declared by
    /// anything here: it is what SQLite wrote into its own description of a
    /// prepared statement. A claim about it that never prepared one would be a
    /// claim about this crate's guess at the engine.
    ///
    /// Two claims, because either alone looks right: the engine describes the
    /// alias rather than the column, and the member hands that description on
    /// unchanged. The third column is the bound on the other side — one entry
    /// per column and never per label, so a description carrying two columns of
    /// one name keeps both.
    ///
    /// The `.nvst` half runs the same statements through a program and is where
    /// the member is reached the way a caller reaches it.
    // covers: Core\Db\Column::name
    #[test]
    fn a_column_label_is_the_alias_a_real_statement_described() {
        let block = nvs_config::tree::Database {
            driver: Some(String::from("sqlite")),
            path: Some(String::from(":memory:")),
            ..nvs_config::tree::Database::default()
        };
        let target = nvs_db::SqliteTarget::resolve(&block).expect("a `sqlite` block resolves");
        let conn = nvs_db::sqlite::open(&target).expect("an in-memory database opens");
        conn.query(
            "create table notes (id integer primary key, text text not null)",
            Vec::new(),
        )
        .expect("the schema is applied");

        // 1. What the engine described. `id` is renamed, `text` is not, and the
        //    third column carries a label the first column already has.
        let described: Vec<nvs_db::SqliteColumn> = conn
            .query("select id as reference, text, id from notes", Vec::new())
            .expect("the statement prepares")
            .columns()
            .to_vec();
        let labels: Vec<&str> = described
            .iter()
            .map(|column| column.name.as_str())
            .collect();
        assert_eq!(
            labels,
            ["reference", "text", "id"],
            "an alias is what the server describes, and two columns may share one label"
        );

        // 2. What the member answers, over a column built exactly as
        //    `sqlite_described_columns` builds one. The other two slots are
        //    never read here, which is the point: a label is one slot.
        let mut ctx = Ctx::buffered();
        for expected in labels {
            let column = crate::instance::build(
                &COLUMN,
                [
                    Value::str(NvsStr::new(expected.as_bytes())),
                    Value::int(0),
                    Value::bool(true),
                ],
            );
            let answered = nvs_runtime::call(nvs_core_db_column_name, &mut ctx, &[column])
                .expect("`name` reads one slot and has nothing to throw");
            assert_eq!(
                answered.as_text(),
                Some(expected),
                "`name` answered something other than the label it was described with"
            );
            released(answered);
            released(column);
        }
    }

    /// **`Core\Db\Column::type` answers what the column was *declared* as**, and
    /// on this backend that is keyed off the schema's own type name rather than
    /// off the storage class a cell arrived in — [`COLUMN_DECLARED_DOC`]'s
    /// sentence, over a schema a real engine parsed.
    ///
    /// Asserted against a real engine for [`a_column_label_is_the_alias_a_real_statement_described`]'s
    /// reason, and the declarations below are chosen so that a map written one
    /// arm short prints plausibly: `datetime` is answered before `date` and
    /// `date` before `time`, a `json` column is `Text` on a backend with no JSON
    /// type of its own, and a column the statement works out was declared
    /// nowhere and is `Other`.
    ///
    /// The second half is the member: the ordinal [`column_type_value`] wrote
    /// into the slot is what a program matches on, and the member hands that on
    /// without classifying anything a second time.
    // covers: Core\Db\Column::type
    #[test]
    fn a_column_case_is_what_a_real_schema_declared() {
        let block = nvs_config::tree::Database {
            driver: Some(String::from("sqlite")),
            path: Some(String::from(":memory:")),
            ..nvs_config::tree::Database::default()
        };
        let target = nvs_db::SqliteTarget::resolve(&block).expect("a `sqlite` block resolves");
        let conn = nvs_db::sqlite::open(&target).expect("an in-memory database opens");
        conn.query(
            "create table notes (id integer, title text, weight real, paidAt datetime, \
             due date, active boolean, picture blob, price decimal(10, 2), badge uuid, \
             payload json)",
            Vec::new(),
        )
        .expect("the schema is applied");

        // 1. The declaration decides, over every family § 9 names that this
        //    backend can carry.
        let described: Vec<nvs_db::SqliteColumn> = conn
            .query(
                "select id, title, weight, paidAt, due, active, picture, price, badge, \
                 payload, weight * 2 as doubled from notes",
                Vec::new(),
            )
            .expect("the statement prepares")
            .columns()
            .to_vec();
        let cases: Vec<&str> = described
            .iter()
            .map(|column| column_type_case(column.column_type()))
            .collect();
        assert_eq!(
            cases,
            [
                "Int", "Text", "Float", "DateTime", "Date", "Bool", "Bytes", "Decimal", "Uuid",
                "Text", "Other",
            ],
            "a declared type named a case this backend does not give it"
        );

        // 2. And the member answers the ordinal that case is at runtime, which
        //    is the only thing a program compares.
        let mut ctx = Ctx::buffered();
        for column in &described {
            let declared = column_type_value(column.column_type());
            let built = crate::instance::build(
                &COLUMN,
                [
                    Value::str(NvsStr::new(column.name.as_bytes())),
                    declared,
                    Value::bool(true),
                ],
            );
            let answered = nvs_runtime::call(nvs_core_db_column_type, &mut ctx, &[built])
                .expect("`type` reads one slot and has nothing to throw");
            assert_eq!(
                answered.as_int(),
                declared.as_int(),
                "`type` answered an ordinal other than the case the column was described with"
            );
            released(built);
        }
    }

    /// **`Core\Db\Column::nullable` answers `true` for every column a real
    /// statement described**, whatever that column was declared — the whole of
    /// [`COLUMN_NULLABLE_DOC`]'s sentence, and the only claim a program may
    /// make about this member today.
    ///
    /// Asserted against a real engine for
    /// [`a_column_label_is_the_alias_a_real_statement_described`]'s reason, and
    /// over four columns a driver keying off the declaration would have to
    /// disagree about: a primary key, a `not null` column, a column declared
    /// nothing at all, and one the statement works out. A stepped SQLite
    /// statement carries no nullability, so there is nothing to key off and all
    /// four are the same answer.
    ///
    /// Two claims, because either alone looks right: [`sqlite_described_columns`]
    /// is what writes the slot, and the member hands that slot on rather than
    /// deciding anything a second time. The count is the bound beside them —
    /// one entry per column, so a description losing a column would print an
    /// answer for every entry it kept.
    // covers: Core\Db\Column::nullable
    #[test]
    fn every_described_column_may_hold_null_whatever_it_was_declared() {
        let block = nvs_config::tree::Database {
            driver: Some(String::from("sqlite")),
            path: Some(String::from(":memory:")),
            ..nvs_config::tree::Database::default()
        };
        let target = nvs_db::SqliteTarget::resolve(&block).expect("a `sqlite` block resolves");
        let conn = nvs_db::sqlite::open(&target).expect("an in-memory database opens");
        conn.query(
            "create table notes (id integer primary key, text text not null, note text)",
            Vec::new(),
        )
        .expect("the schema is applied");

        // 1. The description this crate builds from what the engine described.
        let columns: Vec<nvs_db::SqliteColumn> = conn
            .query(
                "select id, text, note, length(text) as size from notes",
                Vec::new(),
            )
            .expect("the statement prepares")
            .columns()
            .to_vec();
        let described = sqlite_described_columns(&columns);
        assert_eq!(
            described.count(),
            4,
            "one entry per column of the select list"
        );

        // 2. And the member over each of them, which is the only half a program
        //    can see. The element is borrowed from the array, so the frame owes
        //    nothing back here and the answer is the slot itself.
        let mut ctx = Ctx::buffered();
        for index in 0..4 {
            let column = described
                .get_index(index)
                .expect("a column this loop is walking is one the array carries");
            let answered = nvs_runtime::call(nvs_core_db_column_nullable, &mut ctx, &[column])
                .expect("`nullable` reads one slot and has nothing to throw");
            assert_eq!(
                answered.as_bool(),
                Some(true),
                "a stepped statement describes no nullability, so every column may hold NULL"
            );
        }
    }

    /// One [`Requested`] answer as a word, so a case below reads as the sentence
    /// `rule:core-classes/db-column-types` writes rather than as a `match` arm.
    fn answered<T: std::fmt::Debug>(requested: &Requested<T>) -> String {
        match requested {
            Requested::Is(value) => format!("{value:?}"),
            Requested::Lossy(holds) => format!("throws — {holds}"),
            Requested::Mismatched => "throws — not that family".to_owned(),
        }
    }

    /// `rule:core-classes/db-column-types`'s first named crossing:
    /// **`TINYINT(1)` is naturally `int` and reads as `bool` on request, with a
    /// stored `7` throwing.**
    ///
    /// Both halves of that sentence, because either alone is a member that
    /// looks right. A reader that refused the crossing outright would be
    /// correct about the `7` and unusable against MySQL, which has no boolean
    /// column for § 9 to map — `nvs_db::mysql`'s own type-map test pins
    /// `MYSQL_TYPE_TINY` as [`nvs_db::ColumnType::Int`] for that reason. A
    /// reader that took PHP's cast instead would be usable and would read a
    /// `7` as `true`, which is the lossy conversion § 6 exists to refuse.
    ///
    /// The bound is asserted on both sides at once: `0` and `1` are the last
    /// accepted values and `2` is the first refused one, with `-1` the other
    /// end — a member stopping one entry early prints plausibly against either
    /// half alone. `7` is § 6's own number and is in here under its own name.
    ///
    /// Asked of [`requested_bool`] rather than of `$row->bool()`, because that
    /// function is where the rule lives *and* is what `queryAs<T>`'s `bool`
    /// field reaches through [`converted`]: the two surfaces are asserted to
    /// agree below rather than tested twice.
    // covers: Core\Db\Row::bool
    #[test]
    fn tinyint_one_reads_int_and_bool_and_throws_for_a_stored_seven() {
        // § 9 first: a `TINYINT(1)` is an `int` column, so the value a row
        // holds for one is an `int` and `->int()` reads it unchanged.
        for stored in [0_i64, 1, 7, -1] {
            let held = Value::int(stored);
            assert_eq!(
                answered(&requested_int(held)),
                format!("{stored}"),
                "§ 9 gives `TINYINT(1)` the `int` row, whatever it stores"
            );
        }

        // § 6 second: the request converts, and only where it is lossless.
        for (stored, expected) in [
            (0_i64, "false"),
            (1, "true"),
            (7, "throws — 7, which is neither `0` nor `1`"),
            (2, "throws — 2, which is neither `0` nor `1`"),
            (-1, "throws — -1, which is neither `0` nor `1`"),
        ] {
            assert_eq!(
                answered(&requested_bool(Value::int(stored))),
                expected,
                "a `TINYINT(1)` storing {stored}, read as `bool`"
            );
            // The same column on a server that declared it `UNSIGNED`, which is
            // § 9's `uint` row and the same question.
            if let Ok(unsigned) = u64::try_from(stored) {
                assert_eq!(
                    answered(&requested_bool(Value::uint(unsigned))),
                    expected,
                    "an unsigned `TINYINT(1)` storing {stored}, read as `bool`"
                );
            }
        }

        // And a real `BOOLEAN`/`BIT(1)`, which needs no crossing at all.
        for flag in [false, true] {
            assert_eq!(
                answered(&requested_bool(Value::bool(flag))),
                format!("{flag}")
            );
        }

        // The two surfaces § 6 states the rule for once: a `Db\Row` reader and
        // a `#[Db\Derive]` field. A field is checked here through `converted`,
        // which is the whole of what `queryAs<T>` asks; that they route through
        // one function is the assertion, since a second copy would agree on the
        // day it was written and on nothing afterwards.
        assert!(
            matches!(
                converted(nvs_runtime::CodecTy::Bool, None, None, Value::int(1)),
                Ok(value) if value.as_bool() == Some(true)
            ),
            "a `bool` field over a `TINYINT(1)` holding 1 hydrates"
        );
        let refused = converted(nvs_runtime::CodecTy::Bool, None, None, Value::int(7))
            .expect_err("a `bool` field over a stored 7 is § 6's refusal");
        assert!(
            refused.contains("7, which is neither `0` nor `1`"),
            "the field quotes the reader's own wording: {refused}"
        );
    }

    /// **`Core\Db\Row::date` lends the day the row already holds**, and takes
    /// an instance of any other class for a refusal — the member is the lookup
    /// and the class check, and it builds nothing.
    ///
    /// Identity is the assertion a program cannot make: a reader that parsed
    /// the column a second time, or copied the object, would answer a day equal
    /// to this one on every case a `.nvst` can write. So what is compared here
    /// is the address, and the count of holders either side of the read — a
    /// member that answered without lending would free the row's own day under
    /// it on the first release.
    ///
    /// The refusal is asserted against `Core\Time\TimeOfDay`, which is the
    /// neighbour a check written as "is this one of the time classes" would let
    /// through: both are instances, both came out of a text column, and only
    /// the class tells them apart.
    // covers: Core\Db\Row::date
    #[test]
    fn a_day_is_lent_by_the_row_and_no_other_class_is_taken_for_one() {
        let mut ctx = Ctx::buffered();
        let day = crate::time::date_at(2026, 9, 3).expect("the third of September is a day");
        let clock =
            crate::time::time_of_day_at(9, 30, 0, 0).expect("half past nine is a clock reading");
        let mut columns = NvsArray::new();
        columns.set(NvsStr::new(b"born"), day);
        columns.set(NvsStr::new(b"opens"), clock);
        columns.set(NvsStr::new(b"empty"), Value::null());
        let row = crate::instance::build(&ROW, [Value::array(columns)]);

        // 1. The day the row holds, lent rather than rebuilt: one address, and
        //    one holder more for as long as the caller has it.
        let held = day.obj_ptr().expect("a `Core\\Time\\Date` is an object");
        #[expect(
            unsafe_code,
            reason = "the day is alive for this whole test — the row holds it — \
                      so reading its header is reading a live object"
        )]
        let before = unsafe { nvs_runtime::NvsObj::refcount_of(held) };
        let name = Value::str(NvsStr::new(b"born"));
        let answered = nvs_runtime::call(nvs_core_db_row_date, &mut ctx, &[row, name])
            .expect("a `DATE` column is what `date` reads");
        assert_eq!(
            answered.obj_ptr(),
            Some(held),
            "`date` answered a different object from the one the row holds"
        );
        #[expect(
            unsafe_code,
            reason = "the same live object, read once the reader has answered"
        )]
        let lent = unsafe { nvs_runtime::NvsObj::refcount_of(held) };
        assert_eq!(
            lent,
            before + 1,
            "the answer is a reference the caller owes"
        );
        released(answered);
        released(name);

        // 2. The neighbouring class, which is an instance and is not a day.
        let other = Value::str(NvsStr::new(b"opens"));
        nvs_runtime::call(nvs_core_db_row_date, &mut ctx, &[row, other])
            .expect_err("a `TIME` column has no `date` reading");
        let message = ctx
            .take_pending()
            .expect("a refusal is a throw, and it carries a sentence");
        assert!(
            message.contains("opens") && message.contains("Date"),
            "the refusal names the column and the class it answers: {message}"
        );
        released(other);

        // 3. And the column with no value, which is an absence rather than a
        //    day the reader had to invent.
        let absent = Value::str(NvsStr::new(b"empty"));
        let nothing = nvs_runtime::call(nvs_core_db_row_date, &mut ctx, &[row, absent])
            .expect("a NULL column is read rather than refused");
        assert_eq!(nothing.tag(), Some(Tag::Null));
        released(absent);

        released(row);
    }

    /// **`Core\Db\Row::bytes` and `Core\Db\Row::string` do not span each
    /// other**, which is `rule:types/bytes` at the one boundary a database
    /// crosses: a `BLOB` column has no `string` reading and a `TEXT` column has
    /// no `bytes` one.
    ///
    /// Asserted as an agreement between the two readers rather than one answer
    /// each, because a reader that spanned would look right against every
    /// column whose octets happen to be text — which is most of them. The
    /// stored value below is `00 ff 80`, which no encoding reads, so a reader
    /// handing it over as a `string` would be handing over a `string` that
    /// breaks `rule:types/bytes`'s UTF-8 invariant.
    ///
    /// The refusal names the column, since that is the half a caller acts on,
    /// and it is read off the member rather than off [`wrong_column_type`]: the
    /// sentence is what a program catches.
    // covers: Core\Db\Row::bytes
    #[test]
    fn a_blob_column_and_a_text_column_do_not_read_as_each_other() {
        let mut ctx = Ctx::buffered();
        let mut columns = NvsArray::new();
        columns.set(
            NvsStr::new(b"photo"),
            Value::bytes(NvsStr::new(&[0x00, 0xff, 0x80])),
        );
        columns.set(NvsStr::new(b"label"), Value::str(NvsStr::new(b"a name")));
        columns.set(NvsStr::new(b"empty"), Value::null());
        let row = crate::instance::build(&ROW, [Value::array(columns)]);

        // 1. Each reader over its own column, and the octets come back as they
        //    were stored rather than as whatever they could be decoded into.
        let name = Value::str(NvsStr::new(b"photo"));
        let answered = nvs_runtime::call(nvs_core_db_row_bytes, &mut ctx, &[row, name])
            .expect("a `BLOB` column is what `bytes` reads");
        assert_eq!(
            answered.as_bytes(),
            Some(&[0x00, 0xff, 0x80][..]),
            "`bytes` answered something other than the octets the column holds"
        );
        released(answered);
        released(name);

        // 2. The refusals, both directions. Neither reader is a way into the
        //    other's column, and each names the column it was asked about.
        for (reader, column, member) in [
            (
                nvs_core_db_row_bytes as unsafe extern "C" fn(_, _, _) -> i32,
                "label",
                "bytes",
            ),
            (nvs_core_db_row_string, "photo", "string"),
        ] {
            let asked = Value::str(NvsStr::new(column.as_bytes()));
            nvs_runtime::call(reader, &mut ctx, &[row, asked])
                .expect_err("§ 6 keeps `bytes` and `string` apart in both directions");
            let message = ctx
                .take_pending()
                .expect("a refusal is a throw, and it carries a sentence");
            assert!(
                message.contains(column) && message.contains(member),
                "`{member}` over the `{column}` column said: {message}"
            );
            released(asked);
        }

        // 3. And the column with no value, which is an absence in both readers
        //    and never an empty buffer — the one answer that is not a refusal
        //    and not the octets.
        let absent = Value::str(NvsStr::new(b"empty"));
        let nothing = nvs_runtime::call(nvs_core_db_row_bytes, &mut ctx, &[row, absent])
            .expect("a NULL column is read rather than refused");
        assert_eq!(
            nothing.tag(),
            Some(Tag::Null),
            "a NULL `BLOB` column is `null` and not an empty buffer"
        );
        released(absent);

        released(row);
    }

    /// **`Core\Db\Row::decimal` and `Core\Db\Row::float` do not read each
    /// other's columns**, which is `rule:types/decimal` at the one boundary a
    /// database crosses: an exact number and a binary one are both numbers to
    /// the engine, and only the declared column type tells them apart.
    ///
    /// Asserted as an agreement between the two readers rather than one answer
    /// each, because a reader that spanned would look right against every value
    /// both types can hold — which is most of the ones anybody stores. The
    /// number below is `19.950`, which no `float` holds exactly, so a reader
    /// widening it would hand over a price a hundredth of a cent away from the
    /// stored one and go on printing plausibly.
    ///
    /// The scale is the half a `.nvst` cannot assert. `rule:types/decimal`
    /// carries scale for rendering and leaves equality alone, so `19.950` and
    /// `19.95` compare equal in a program and a reader that reduced the value
    /// would pass every case one can write; here the comparison is the
    /// [`nvs_runtime::Decimal`] itself, whose derived equality is the sign, the
    /// mantissa and the scale together.
    // covers: Core\Db\Row::decimal
    #[test]
    fn an_exact_column_and_a_float_column_do_not_read_as_each_other() {
        let mut ctx = Ctx::buffered();
        let stored = nvs_runtime::Decimal::parse("19.950").expect("`19.950` is an exact number");
        let mut columns = NvsArray::new();
        columns.set(NvsStr::new(b"amount"), Value::decimal(stored));
        columns.set(NvsStr::new(b"rate"), Value::float(19.95));
        columns.set(NvsStr::new(b"empty"), Value::null());
        let row = crate::instance::build(&ROW, [Value::array(columns)]);

        // 1. The exact number, with the digits and the scale it was stored
        //    with, and nothing allocated to carry them.
        let name = Value::str(NvsStr::new(b"amount"));
        let answered = nvs_runtime::call(nvs_core_db_row_decimal, &mut ctx, &[row, name])
            .expect("a `DECIMAL` column is what `decimal` reads");
        assert_eq!(
            answered.as_decimal(),
            Some(stored),
            "`decimal` answered a number that renders differently from the stored one"
        );
        assert_eq!(
            answered.obj_ptr(),
            None,
            "a `decimal` is a scalar, so the answer is no object and owes no release"
        );
        released(answered);
        released(name);

        // 2. The refusals, both directions. Neither reader is a way into the
        //    other's column, and each names the column it was asked about.
        for (reader, column, member) in [
            (
                nvs_core_db_row_decimal as unsafe extern "C" fn(_, _, _) -> i32,
                "rate",
                "decimal",
            ),
            (nvs_core_db_row_float, "amount", "float"),
        ] {
            let asked = Value::str(NvsStr::new(column.as_bytes()));
            nvs_runtime::call(reader, &mut ctx, &[row, asked])
                .expect_err("§ 6 keeps `decimal` and `float` apart in both directions");
            let message = ctx
                .take_pending()
                .expect("a refusal is a throw, and it carries a sentence");
            assert!(
                message.contains(column) && message.contains(member),
                "`{member}` over the `{column}` column said: {message}"
            );
            released(asked);
        }

        // 3. And the column with no value, which is an absence rather than the
        //    zero a number column makes so easy to answer instead.
        let absent = Value::str(NvsStr::new(b"empty"));
        let nothing = nvs_runtime::call(nvs_core_db_row_decimal, &mut ctx, &[row, absent])
            .expect("a NULL column is read rather than refused");
        assert_eq!(
            nothing.tag(),
            Some(Tag::Null),
            "a NULL `DECIMAL` column is `null` and not a zero"
        );
        released(absent);

        released(row);
    }

    /// **Reading `Core\Db\Row::time` again lends the same clock reading**, so a
    /// row read a million times holds one of them and not a million.
    ///
    /// That is the invariant this member's attack is written around and cannot
    /// assert: a program watching memory sees a number that stays flat, where
    /// the count of holders is the thing that actually has to stay flat. Here
    /// three reads are held at once and the count is read at each end — a
    /// member that copied would answer three addresses, and one that lent
    /// without counting would leave the row's own reading freed under it.
    ///
    /// The refusal is asserted against `Core\Time\Date`, which is the neighbour
    /// a check written as "is this one of the time classes" would let through:
    /// both are instances, both came out of a text column, and only the class
    /// tells them apart.
    // covers: Core\Db\Row::time
    #[test]
    fn three_reads_of_one_clock_reading_lend_it_three_times_and_copy_it_none() {
        let mut ctx = Ctx::buffered();
        let clock =
            crate::time::time_of_day_at(9, 30, 0, 0).expect("half past nine is a clock reading");
        let day = crate::time::date_at(2026, 9, 3).expect("the third of September is a day");
        let mut columns = NvsArray::new();
        columns.set(NvsStr::new(b"opens"), clock);
        columns.set(NvsStr::new(b"born"), day);
        columns.set(NvsStr::new(b"empty"), Value::null());
        let row = crate::instance::build(&ROW, [Value::array(columns)]);

        let held = clock
            .obj_ptr()
            .expect("a `Core\\Time\\TimeOfDay` is an object");
        #[expect(
            unsafe_code,
            reason = "the reading is alive for this whole test — the row holds it — \
                      so reading its header is reading a live object"
        )]
        let before = unsafe { nvs_runtime::NvsObj::refcount_of(held) };

        // 1. Three reads, all held at once. One address, and one holder more
        //    for each answer the caller has not released yet.
        let mut answers = Vec::new();
        for _ in 0..3 {
            let name = Value::str(NvsStr::new(b"opens"));
            let answered = nvs_runtime::call(nvs_core_db_row_time, &mut ctx, &[row, name])
                .expect("a `TIME` column is what `time` reads");
            assert_eq!(
                answered.obj_ptr(),
                Some(held),
                "`time` answered a different object from the one the row holds"
            );
            answers.push(answered);
            released(name);
        }
        #[expect(
            unsafe_code,
            reason = "the same live object, read once three answers are outstanding"
        )]
        let lent = unsafe { nvs_runtime::NvsObj::refcount_of(held) };
        assert_eq!(
            lent,
            before + 3,
            "three answers are three references the caller owes"
        );
        for answered in answers {
            released(answered);
        }
        #[expect(
            unsafe_code,
            reason = "the row still holds the reading, so the object is live after every release"
        )]
        let given_back = unsafe { nvs_runtime::NvsObj::refcount_of(held) };
        assert_eq!(
            given_back, before,
            "releasing every answer leaves the row holding its own reading"
        );

        // 2. The neighbouring class, which is an instance and is not a clock
        //    reading.
        let other = Value::str(NvsStr::new(b"born"));
        nvs_runtime::call(nvs_core_db_row_time, &mut ctx, &[row, other])
            .expect_err("a `DATE` column has no `time` reading");
        let message = ctx
            .take_pending()
            .expect("a refusal is a throw, and it carries a sentence");
        assert!(
            message.contains("born") && message.contains("TimeOfDay"),
            "the refusal names the column and the class it answers: {message}"
        );
        released(other);

        // 3. And the column with no value, which is an absence rather than
        //    midnight, the reading a member inventing one would pick.
        let absent = Value::str(NvsStr::new(b"empty"));
        let nothing = nvs_runtime::call(nvs_core_db_row_time, &mut ctx, &[row, absent])
            .expect("a NULL column is read rather than refused");
        assert_eq!(nothing.tag(), Some(Tag::Null));
        released(absent);

        released(row);
    }

    /// **`Core\Db\Row::instant` answers the moment a row holds**, and refuses
    /// the zone-*less* reading that is the only thing a column like that one's
    /// carries on the engine every proof program here runs against.
    ///
    /// This is the one place the answer is reached at all. SQLite folds every
    /// `TIMESTAMP` spelling into `ColumnType::DateTime`, so this member's
    /// example, attack and bench are skip entries in
    /// `tools/data/dossier-policy.toml` and the `.nvst` case beside them can
    /// pin the refusals and nothing else. A row built here holds whatever it is
    /// given, so the `TIMESTAMPTZ` column the member is written for is one
    /// [`NvsArray::set`] away.
    ///
    /// `Core\Time\DateTime` is the refusal worth asserting: it is an instance,
    /// it came out of a column spelled almost like this one's, and only the
    /// class tells the two apart. The answer is read back through its own
    /// rendering, so a member handing over the right class carrying some other
    /// moment fails here rather than passing on the class check alone.
    // covers: Core\Db\Row::instant
    #[test]
    fn instant_answers_the_moment_a_row_holds_and_refuses_a_zoneless_reading() {
        let mut ctx = Ctx::buffered();
        let moment = crate::time::instant_from_iso("2026-09-03T14:05:00Z")
            .expect("an RFC 3339 rendering is a moment");
        let zoneless = crate::time::datetime_of_text("2026-09-03 14:05:00", 0)
            .expect("a `datetime` cell is a reading with no zone");
        let mut columns = NvsArray::new();
        columns.set(NvsStr::new(b"at"), moment);
        columns.set(NvsStr::new(b"seen"), zoneless);
        columns.set(NvsStr::new(b"empty"), Value::null());
        let row = crate::instance::build(&ROW, [Value::array(columns)]);

        // 1. The answer: the row's own object, carrying the moment it was
        //    built with.
        let held = moment
            .obj_ptr()
            .expect("a `Core\\Time\\Instant` is an object");
        let name = Value::str(NvsStr::new(b"at"));
        let answered = nvs_runtime::call(nvs_core_db_row_instant, &mut ctx, &[row, name])
            .expect("a `TIMESTAMPTZ` column is what `instant` reads");
        assert_eq!(
            answered.obj_ptr(),
            Some(held),
            "`instant` answered a different object from the one the row holds"
        );
        assert_eq!(
            crate::time::instant_iso(answered).as_deref(),
            Some("2026-09-03T14:05:00Z"),
            "the moment answered is the moment the row holds"
        );
        released(answered);
        released(name);

        // 2. The neighbouring class, which is an instance and carries no zone.
        let other = Value::str(NvsStr::new(b"seen"));
        nvs_runtime::call(nvs_core_db_row_instant, &mut ctx, &[row, other])
            .expect_err("a reading with no zone is not a moment");
        let message = ctx
            .take_pending()
            .expect("a refusal is a throw, and it carries a sentence");
        assert!(
            message.contains("seen") && message.contains("Instant"),
            "the refusal names the column and the class it answers: {message}"
        );
        released(other);

        // 3. And the column with no value, which is an absence rather than the
        //    epoch, the moment a member inventing one would pick.
        let absent = Value::str(NvsStr::new(b"empty"));
        let nothing = nvs_runtime::call(nvs_core_db_row_instant, &mut ctx, &[row, absent])
            .expect("a NULL column is read rather than refused");
        assert_eq!(nothing.tag(), Some(Tag::Null));
        released(absent);

        released(row);
    }

    /// **`Core\Db\Row::uuid` answers an id and refuses every other class a
    /// column can hold**, which is the whole of what this member does past the
    /// lookup itself.
    ///
    /// One question asked of the four `Core`-owned classes a column is read as
    /// — an id, a moment, a calendar day and a clock reading — so a member
    /// checking "is this an instance" rather than "is this an id" fails here
    /// while answering plausibly row by row. No program reaches this shape: an
    /// engine hands a row the classes its own column types fold to, and no
    /// engine folds to all four in one table, where a row built here holds
    /// whatever it is given.
    ///
    /// The answer is asserted to be the row's own object, so a member building
    /// a second id out of the first fails rather than passing on the class
    /// check alone.
    // covers: Core\Db\Row::uuid
    #[test]
    fn uuid_answers_an_id_and_refuses_the_other_three_classes_a_column_holds() {
        let mut ctx = Ctx::buffered();
        let id = crate::uuid::of_text("3f2504e0-4f89-41d3-9a0c-0305e82c3301")
            .expect("a canonical rendering is an id");
        let moment = crate::time::instant_from_iso("2026-09-03T14:05:00Z")
            .expect("an RFC 3339 rendering is a moment");
        let day = crate::time::date_at(2026, 9, 3).expect("the third of September is a day");
        let clock =
            crate::time::time_of_day_at(9, 30, 0, 0).expect("half past nine is a clock reading");
        let mut columns = NvsArray::new();
        columns.set(NvsStr::new(b"id"), id);
        columns.set(NvsStr::new(b"at"), moment);
        columns.set(NvsStr::new(b"born"), day);
        columns.set(NvsStr::new(b"opens"), clock);
        columns.set(NvsStr::new(b"empty"), Value::null());
        let row = crate::instance::build(&ROW, [Value::array(columns)]);

        // 1. The one column of the four that is an id, answered as the object
        //    the row itself holds.
        let name = Value::str(NvsStr::new(b"id"));
        let answered = nvs_runtime::call(nvs_core_db_row_uuid, &mut ctx, &[row, name])
            .expect("a `UUID` column is what `uuid` reads");
        assert_eq!(
            answered.obj_ptr(),
            id.obj_ptr(),
            "`uuid` answered an id the row does not hold"
        );
        released(answered);
        released(name);

        // 2. The other three, each an instance and none of them an id.
        for column in [b"at".as_slice(), b"born".as_slice(), b"opens".as_slice()] {
            let label = core::str::from_utf8(column).expect("these column names are ASCII");
            let other = Value::str(NvsStr::new(column));
            nvs_runtime::call(nvs_core_db_row_uuid, &mut ctx, &[row, other])
                .expect_err("a reading of another class is not an id");
            let message = ctx
                .take_pending()
                .expect("a refusal is a throw, and it carries a sentence");
            assert!(
                message.contains(label) && message.contains("Uuid"),
                "the refusal names the column and the class it answers: {message}"
            );
            released(other);
        }

        // 3. And the column with no value, which is an absence rather than the
        //    nil id, the value a member inventing one would pick.
        let absent = Value::str(NvsStr::new(b"empty"));
        let nothing = nvs_runtime::call(nvs_core_db_row_uuid, &mut ctx, &[row, absent])
            .expect("a NULL column is read rather than refused");
        assert_eq!(nothing.tag(), Some(Tag::Null));
        released(absent);

        released(row);
    }

    /// **`Core\Db\Row::string` answers the row's own text rather than a copy
    /// of it**, so reading one column a million times costs the one allocation
    /// the row already made.
    ///
    /// Asserted on the address of the payload, which is the only thing that
    /// separates handing a value over from rebuilding it. A reader that copied
    /// the characters would answer the right text on every row and still turn
    /// a listing reading one column per row into a listing allocating one
    /// string per row — the cost `benches/members/core/Db-Row/string.nvs`
    /// declares as zero. No program can see the difference, which is why this
    /// half is pinned from Rust and the column types are pinned from Novis.
    // covers: Core\Db\Row::string
    #[test]
    fn string_answers_the_rows_own_text_rather_than_a_copy_of_it() {
        let mut ctx = Ctx::buffered();
        let stored = Value::str(NvsStr::new("Amélie".as_bytes()));
        let mut columns = NvsArray::new();
        columns.set(NvsStr::new(b"name"), stored);
        columns.set(NvsStr::new(b"empty"), Value::null());
        let row = crate::instance::build(&ROW, [Value::array(columns)]);
        let held = stored
            .as_str_bytes()
            .map(<[u8]>::as_ptr)
            .expect("the column was built holding a text");

        // 1. Four reads of the one column, each answering the allocation the
        //    row itself holds.
        let name = Value::str(NvsStr::new(b"name"));
        for _ in 0..4 {
            let answered = nvs_runtime::call(nvs_core_db_row_string, &mut ctx, &[row, name])
                .expect("a `TEXT` column is what `string` reads");
            assert_eq!(
                answered.as_str_bytes(),
                Some("Amélie".as_bytes()),
                "`string` answered something other than the text the column holds"
            );
            assert_eq!(
                answered.as_str_bytes().map(<[u8]>::as_ptr),
                Some(held),
                "`string` built a second copy of the text the row already holds"
            );
            released(answered);
        }
        released(name);

        // 2. And the column with no value, which is an absence rather than the
        //    empty text, the value a member inventing one would pick.
        let absent = Value::str(NvsStr::new(b"empty"));
        let nothing = nvs_runtime::call(nvs_core_db_row_string, &mut ctx, &[row, absent])
            .expect("a NULL column is read rather than refused");
        assert_eq!(nothing.tag(), Some(Tag::Null));
        released(absent);

        released(row);
    }

    /// **`Core\Db\Row::int` answers the whole signed range exactly, and never
    /// through a float**, so both ends of a `BIGINT` column come back as the
    /// numbers they were stored as.
    ///
    /// `i64::MAX` is where that is decided: the nearest `f64` to it is
    /// `9223372036854775808`, one past the column's own ceiling, so a reader
    /// converting through a double answers a number the column does not hold
    /// and stops comparing equal to itself. The float column below holds
    /// exactly that double and is refused rather than read as the ceiling,
    /// which is the same claim from the other side. No engine hands a row both
    /// of those at once, so the pair is built here.
    // covers: Core\Db\Row::int
    #[test]
    fn int_answers_the_whole_signed_range_and_never_through_a_float() {
        /// The four values a signed column is read at, which are its two ends
        /// and the two either side of zero.
        const ENDS: [(&[u8], i64); 4] = [
            (b"smallest", i64::MIN),
            (b"below", -1),
            (b"none", 0),
            (b"largest", i64::MAX),
        ];

        let mut ctx = Ctx::buffered();
        let mut columns = NvsArray::new();
        for (label, stored) in ENDS {
            columns.set(NvsStr::new(label), Value::int(stored));
        }
        columns.set(NvsStr::new(b"near"), Value::float(9223372036854775808.0));
        let row = crate::instance::build(&ROW, [Value::array(columns)]);

        // 1. Each end of the range, answered as the number the column holds.
        for (label, stored) in ENDS {
            let name = Value::str(NvsStr::new(label));
            let answered = nvs_runtime::call(nvs_core_db_row_int, &mut ctx, &[row, name])
                .expect("an integer column is what `int` reads");
            assert_eq!(
                answered.as_int(),
                Some(stored),
                "`int` answered something other than the number the column holds"
            );
            released(name);
        }

        // 2. And the nearest double to the ceiling, which is a fractional
        //    column however round its value is.
        let near = Value::str(NvsStr::new(b"near"));
        nvs_runtime::call(nvs_core_db_row_int, &mut ctx, &[row, near])
            .expect_err("a `REAL` column is not an integer, whatever it holds");
        let message = ctx
            .take_pending()
            .expect("a refusal is a throw, and it carries a sentence");
        assert!(
            message.contains("near"),
            "the refusal names the column it was asked about: {message}"
        );
        released(near);

        released(row);
    }

    /// **`Core\Db\Row::uint` tells a negative value apart from a column that
    /// is no integer at all**, which are the two refusals it has and the one
    /// distinction a caller acts on.
    ///
    /// Both arrive as a `LogicError`, so a program catching them sees one
    /// thing and the sentence is the whole difference: the first says what the
    /// column holds and where the reader stops, and the second says what the
    /// column is. A reader answering [`wrong_column_type`] for a negative
    /// number would send a caller looking for a column of another type, when
    /// the column is right and `Core\Db\Row::int` is the reader for it.
    // covers: Core\Db\Row::uint
    #[test]
    fn uint_tells_a_negative_value_apart_from_a_column_that_is_no_integer() {
        let mut ctx = Ctx::buffered();
        let mut columns = NvsArray::new();
        columns.set(NvsStr::new(b"size"), Value::int(18432));
        columns.set(NvsStr::new(b"owed"), Value::int(-1));
        columns.set(NvsStr::new(b"label"), Value::str(NvsStr::new(b"18432")));
        let row = crate::instance::build(&ROW, [Value::array(columns)]);

        // 1. The column this reader answers, which is a signed one that is not
        //    negative.
        let name = Value::str(NvsStr::new(b"size"));
        let answered = nvs_runtime::call(nvs_core_db_row_uint, &mut ctx, &[row, name])
            .expect("a signed column that is not negative has a `uint` reading");
        assert_eq!(answered.as_uint(), Some(18432));
        released(name);

        // 2. The two refusals, read off the member as a program catches them.
        let mut said = Vec::new();
        for column in [b"owed".as_slice(), b"label".as_slice()] {
            let asked = Value::str(NvsStr::new(column));
            nvs_runtime::call(nvs_core_db_row_uint, &mut ctx, &[row, asked])
                .expect_err("neither a negative number nor a text is a `uint`");
            said.push(
                ctx.take_pending()
                    .expect("a refusal is a throw, and it carries a sentence"),
            );
            released(asked);
        }
        assert!(
            said[0].contains("-1") && said[0].contains("floor"),
            "the negative column's refusal says what it holds and where `uint` stops: {}",
            said[0]
        );
        assert!(
            said[1].contains("an integer") && !said[1].contains("floor"),
            "the text column's refusal says what the column is: {}",
            said[1]
        );

        released(row);
    }

    /// § 6's second named crossing: **a `BIGINT UNSIGNED` past `i64::MAX` reads
    /// as `uint` and throws for `int`.**
    ///
    /// The bound on both sides, at the one value where it falls: `i64::MAX`
    /// itself crosses and `i64::MAX + 1` does not. A driver reading the column
    /// through PHP's `int` loses that value to a `float` and stops comparing
    /// equal to itself, which is the defect § 9's `uint` row exists for — and a
    /// range check written one off would pass every test that named only the
    /// obvious `u64::MAX`.
    ///
    /// The other direction is the same rule and is asserted beside it, since
    /// `int` and `uint` are `rule:types/arithmetic`'s one integer read two ways: a
    /// negative `BIGINT` has no `uint` reading, and `0` is the bound there.
    #[test]
    fn bigint_unsigned_past_i64_max_reads_uint_and_throws_for_int() {
        /// Where `int` stops and `uint` keeps going, which is the one value
        /// this bound falls at.
        const CEILING: u64 = i64::MAX.cast_unsigned();

        for (stored, as_uint, as_int) in [
            (0_u64, "0", "0"),
            (CEILING, "9223372036854775807", "9223372036854775807"),
            (
                CEILING + 1,
                "9223372036854775808",
                "throws — 9223372036854775808, which is past `int`'s ceiling",
            ),
            (
                u64::MAX,
                "18446744073709551615",
                "throws — 18446744073709551615, which is past `int`'s ceiling",
            ),
        ] {
            let held = Value::uint(stored);
            assert_eq!(
                answered(&requested_uint(held)),
                as_uint,
                "a `BIGINT UNSIGNED` holding {stored} is `uint`'s own row"
            );
            assert_eq!(
                answered(&requested_int(held)),
                as_int,
                "the same column asked for `int`"
            );
        }

        for (stored, as_uint) in [
            (0_i64, "0"),
            (-1, "throws — -1, which is below `uint`'s floor"),
            (
                i64::MIN,
                "throws — -9223372036854775808, which is below `uint`'s floor",
            ),
        ] {
            assert_eq!(
                answered(&requested_uint(Value::int(stored))),
                as_uint,
                "a signed `BIGINT` holding {stored}, asked for `uint`"
            );
        }

        // Neither reading is a way into a column of another family: § 6's
        // crossings are between `int` and `uint` and nowhere else.
        for held in [
            Value::float(1.0),
            Value::decimal(Decimal::parse("1").expect("`1` is a decimal")),
            Value::bool(true),
        ] {
            assert!(
                matches!(requested_int(held), Requested::Mismatched),
                "{held:?} is not an integer column"
            );
            assert!(matches!(requested_uint(held), Requested::Mismatched));
        }
    }

    /// § 6's third named crossing, which is the one that is not a crossing: **a
    /// `DECIMAL` refuses a `float` field**, since
    /// `rule:types/decimal` keeps the two
    /// apart.
    ///
    /// This is the ADR's own § *Context* defect at the hydration boundary. PDO
    /// hands a `DECIMAL` back as a string on every driver it has, and the PHP
    /// code that follows compares a price with `==` and gets away with it until
    /// a value stops surviving the `float` it is silently coerced through. So
    /// the refusal is asserted at a value where the widening is *invisible* —
    /// `0.1` has no exact `float` and `1` has one — because a check written
    /// against a value that already fails to round-trip would pass over a
    /// codec that widened whenever it could.
    ///
    /// Both directions, since `rule:types/decimal` keeps them apart in both: a `FLOAT`
    /// column has no `decimal` field either, and `->decimal()` is the reader
    /// the exact column has.
    #[test]
    fn a_decimal_into_a_float_field_throws() {
        use nvs_runtime::CodecTy;

        for text in ["1", "0.1", "-12345678901234567890.12", "0"] {
            let exact = Decimal::parse(text).expect("a decimal literal");
            let refused = converted(CodecTy::Float, None, None, Value::decimal(exact))
                .expect_err("a `float` field over a `DECIMAL` is refused whatever it holds");
            assert!(
                refused.contains("`float`"),
                "the refusal names the field's declared type: {refused}"
            );

            // The column's own field type still hydrates it, so what is being
            // pinned is the crossing and not the column.
            assert!(
                converted(CodecTy::Mixed, None, None, Value::decimal(exact)).is_ok(),
                "`mixed` takes whatever the column held"
            );
        }

        // The other side of `rule:types/decimal`'s wall, and the reason this is a
        // `Mismatched` rather than a range: there is no `DECIMAL` a `float`
        // field takes and no `FLOAT` a `decimal` field takes, at any value.
        let refused = converted(CodecTy::Float, None, None, Value::int(1))
            .expect_err("§ 6 has no int-widens-to-float crossing either");
        assert!(refused.contains("`float`"), "{refused}");
    }

    /// One [`nvs_runtime::CodecField`], with the properties this file's cases
    /// never vary spelled once.
    fn codec_field(key: &str, param: usize, ty: nvs_runtime::CodecTy) -> nvs_runtime::CodecField {
        nvs_runtime::CodecField {
            key: key.to_owned(),
            slot: param,
            param,
            ty,
            element: None,
            class: None,
            cases: None,
            shape: None,
            nullable: false,
            required: true,
            default: None,
        }
    }

    /// `rule:core-classes/db-column-types`'s refusal for
    /// `queryAs<T>`: a wrong type, a missing column or a NULL in a field
    /// declared non-nullable throws **naming every offending column, not the
    /// first**.
    ///
    /// The three conditions are named in one sentence of the ADR and they reach
    /// [`hydrate`] by three different routes — a value the field's declared type
    /// refuses, a key the row has no entry for at all, and a `Tag::Null` that
    /// only a `?T` field takes — so a codec that accumulated on one route and
    /// returned early on another passes any case that asks about one of them.
    /// Asked here as a **count**: three fields are wrong and three issues come
    /// back, which is the assertion a per-condition case cannot make.
    ///
    /// Naming the column is the item rather than a nicety. § 6 has field names
    /// match column names exactly and `AS` as the way to rename, so at a table
    /// of forty columns the path is the only thing separating "one of these did
    /// not match" from a fix — [ADR 0071 § 5](/docs/decisions/0071.md)
    /// is where the `issues` list this reads back is specified, and the throw is
    /// a `ParseError` for the reason [`hydrate`]'s own docs give.
    ///
    /// The member this is the Rust side of is `queryAs`: hydration is the whole
    /// of what it adds to `query`, and [`hydrate`] is where a row becomes the
    /// class the call site wrote. The `.nvst` half runs the same construction
    /// through a program, where the refusal is caught by name.
    // covers: Core\Db\Connection::queryAs
    #[test]
    fn query_as_throws_naming_the_column_for_a_mismatch_a_missing_column_and_a_null() {
        use nvs_runtime::CodecTy;

        // A descriptor is identified by its address, so the table outlives the
        // test rather than being moved — `allocation_policy.rs`'s `closure_of`
        // is the same shape and the same reason.
        let table: &'static mut nvs_runtime::ClassTable =
            Box::leak(Box::new(nvs_runtime::ClassTable::new()));
        let id = table.define("Account", &["id", "name", "at"], &[]);
        table.set_db_codec(
            id,
            vec![
                codec_field("id", 0, CodecTy::Int),
                codec_field("name", 1, CodecTy::Str),
                codec_field("at", 2, CodecTy::Str),
            ],
            3,
            vec![std::ptr::null(); 3],
        );
        let class = table.desc(id);

        // The row three of whose columns are wrong in three different ways, and
        // the fourth — `id` is present and is a string where the field declares
        // `int`; `name` is absent outright; `at` is SQL NULL against a field
        // that is not `?T`. Nothing here is right, which is the point: a codec
        // reporting the first would answer one of the three.
        let mut row = NvsArray::new();
        row.set(NvsStr::new(b"id"), Value::str(NvsStr::new(b"7")));
        row.set(NvsStr::new(b"at"), Value::null());

        #[expect(unsafe_code, reason = "the leaked table keeps the descriptor live")]
        let refused = unsafe {
            hydrate(&mut Ctx::new(OutputSink::Sink), class, &row).expect_err(
                "a row with three offending columns is § 6's throw and never reaches `new`",
            )
        };

        let Fault::ThrownWithSlots(ThrownClass::Parse, message, slots) = refused else {
            panic!(
                "§ 6's mismatch is `rule:core-classes/derive-reports-every-field`'s `ParseError` carrying `issues`"
            )
        };
        assert!(
            message.contains("3 column(s) of `Account`"),
            "the summary counts what the list carries: {message}"
        );

        let [(slot, issues)] = *slots else {
            panic!("one slot, and it is `issues`")
        };
        assert_eq!(slot, nvs_runtime::ISSUES_SLOT);
        let list = crate::arr::borrowed(issues.array_ptr().expect("`issues` is an `array<Issue>`"));
        assert_eq!(
            list.count(),
            3,
            "every offending column at once, which is what a form needs to \
             report all four bad fields rather than the first"
        );

        // `rule:core-classes/derive-reports-every-field`'s `path` is the column, and the order is the field
        // declaration order — read off `crate::issue::FIELDS`' slot order
        // rather than guessed, since that agreement is the one that would fail
        // silently.
        let paths: Vec<String> = (0..3)
            .map(|index| {
                let issue = list
                    .get_index(index)
                    .expect("every position of the list holds an issue");
                let object = issue.obj_ptr().expect("an issue is a shape value");
                let path = crate::instance::slot(object, 1);
                String::from_utf8_lossy(path.as_str_bytes().expect("`path` is a string"))
                    .into_owned()
            })
            .collect();
        assert_eq!(paths, ["id", "name", "at"]);

        #[expect(
            unsafe_code,
            reason = "this frame owns the reference the throw handed over"
        )]
        unsafe {
            issues.release();
        }
    }

    /// `rule:core-api/required-optional-and-nullable`'s first column at the row
    /// door: a column the result does not carry is an issue only where the
    /// field's constructor parameter declares no default, and a field that
    /// declares one is filled from the constant
    /// [`nvs_runtime::CodecField::default`] carries.
    ///
    /// Asked of one row in which **both** columns are absent, and asserted as a
    /// count: a hydration reading presence off the row alone reports two, and
    /// one reading it off the type — as a `?T` would — reports two as well,
    /// while either still looks right on whichever field is asked about on its
    /// own.
    #[test]
    fn an_absent_column_is_an_issue_only_where_the_parameter_declares_no_default() {
        use nvs_runtime::{CodecTy, FieldDefault};

        // Leaked for the sibling case's reason: a descriptor is identified by
        // its address, so the table outlives the test rather than moving.
        let table: &'static mut nvs_runtime::ClassTable =
            Box::leak(Box::new(nvs_runtime::ClassTable::new()));
        let id = table.define("Account", &["id", "name"], &[]);
        let mut defaulted = codec_field("name", 1, CodecTy::Str);
        defaulted.required = false;
        defaulted.default = Some(FieldDefault::Str("anon".to_owned()));
        table.set_db_codec(
            id,
            vec![codec_field("id", 0, CodecTy::Int), defaulted],
            2,
            vec![std::ptr::null(); 2],
        );
        let class = table.desc(id);

        let row = NvsArray::new();
        #[expect(unsafe_code, reason = "the leaked table keeps the descriptor live")]
        let refused = unsafe {
            hydrate(&mut Ctx::new(OutputSink::Sink), class, &row)
                .expect_err("`id` declares no default, so its column cannot be missing")
        };

        let Fault::ThrownWithSlots(ThrownClass::Parse, message, slots) = refused else {
            panic!("a missing required column is § 6's `ParseError` carrying `issues`")
        };
        assert!(
            message.contains("1 column(s) of `Account`"),
            "`name` filled and `id` did not: {message}"
        );
        let [(_, issues)] = *slots else {
            panic!("one slot, and it is `issues`")
        };
        let list = crate::arr::borrowed(issues.array_ptr().expect("`issues` is an `array<Issue>`"));
        assert_eq!(
            list.count(),
            1,
            "the absent defaulted column is filled rather than reported, and the \
             absent required one is reported rather than filled"
        );
        let issue = list.get_index(0).expect("the one issue");
        let object = issue.obj_ptr().expect("an issue is a shape value");
        let path = crate::instance::slot(object, 1);
        assert_eq!(
            path.as_str_bytes().expect("`path` is a string"),
            b"id",
            "the column named is the one with no default"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the reference the throw handed over"
        )]
        unsafe {
            issues.release();
        }
    }

    /// Stands in for the compiled `static fromRow(Db\Row $row): static` a class
    /// writes itself, with the ABI a compiled method has: slot 0 is the called
    /// class and slot 1 the row, and what it answers is read **out of the row**
    /// so that no path which failed to hand one over can produce it.
    ///
    /// It releases its parameter because a compiled Novis function does —
    /// [`nvs_runtime::call_static_on`] retained every slot before the jump —
    /// and retains what it hands back for the same reason.
    #[expect(
        unsafe_code,
        reason = "a method's slot array, its ownership convention and the out \
                  parameter are all `nvs_runtime::abi`'s calling convention, \
                  which a stand-in for a compiled member has to meet exactly"
    )]
    unsafe extern "C" fn from_row_stub(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        let row = unsafe { *args.add(1) };
        let object = row
            .obj_ptr()
            .expect("`fromRow` is handed the `Core\\Db\\Row` every other reader gets");
        let columns = crate::arr::borrowed(
            crate::instance::slot(object, COLUMNS_AT)
                .array_ptr()
                .expect("a row's one slot holds its columns"),
        );
        let id = columns.get(b"id").expect("the row carries an `id` column");
        unsafe { id.retain() };
        unsafe { *out = id };
        unsafe { row.release() };
        nvs_runtime::OK
    }

    /// `rule:core-classes/derive-generates-what-is-missing`'s other door for a
    /// row: `Core\Db\Codec` declares `fromRow` alone, so a class that writes it
    /// carries no `#[Db\Derive]` — the two together are `E0757` — and records
    /// no column mapping at all.
    ///
    /// So the assertion is that an **empty** `db_codec` reaches the member the
    /// class wrote rather than the refusal it used to be, and that the member
    /// is handed the same `Core\Db\Row` a `query` would have answered with:
    /// the value that comes back is one only a call that received the row could
    /// have built. [ADR 0067 § 6](/docs/decisions/0067.md) admits the
    /// hand-written body beside the derived one, and `check_row_sites` lets the
    /// call site through for exactly this class.
    #[test]
    fn query_as_calls_a_hand_written_from_row() {
        let table: &'static mut nvs_runtime::ClassTable =
            Box::leak(Box::new(nvs_runtime::ClassTable::new()));
        let id = table.define("Account", &["id"], &[]);
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: FROM_ROW.to_owned(),
                code: from_row_stub as nvs_runtime::NvsFn as *const u8,
                arity: 1,
                // Nothing on this route reads either of the two: an argument
                // list a native member built is checked where it is built, and
                // `nvs_runtime::call_static_bound`'s tag comparison is for the
                // other caller, whose list came out of a program's own map.
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        let class = table.desc(id);

        let mut row = NvsArray::new();
        row.set(NvsStr::new(b"id"), Value::int(7));

        #[expect(unsafe_code, reason = "the leaked table keeps the descriptor live")]
        let built = unsafe { hydrate(&mut Ctx::new(OutputSink::Sink), class, &row) }
            .expect("a class declaring `fromRow` hydrates through it");
        assert_eq!(
            built.as_int(),
            Some(7),
            "the member answered out of the row it was handed"
        );

        // The class that took neither door is still the backstop refusal, and
        // it now names both of them.
        let bare = table.define("Bare", &["id"], &[]);
        #[expect(unsafe_code, reason = "the leaked table keeps the descriptor live")]
        let refused = unsafe { hydrate(&mut Ctx::new(OutputSink::Sink), table.desc(bare), &row) }
            .expect_err("no mapping and no `fromRow` is the program's own mistake");
        let Fault::Thrown(ThrownClass::Logic, message) = refused else {
            panic!("a class with neither half is a `LogicError`")
        };
        assert!(message.contains(FROM_ROW), "{message}");
        assert!(message.contains("E0806"), "{message}");
    }
}
