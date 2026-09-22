//! From the values a program holds to the statement a driver runs, and from a
//! server's failure back to a class the program can catch.
//!
//! Everything here is the boundary's shape rather than any one driver's: a
//! parameter is a [`Bound`] whatever the wire will make of it, and a failure is
//! a [`nvs_db::DbErrorKind`] before it is a class name. That second ordering is
//! what lets five drivers disagree about error codes and still agree about
//! [ADR 0067 § 10](/docs/decisions/0067.md)'s kinds.

use super::*;

/// The request-table key and the `[db.<name>]` block behind a
/// `Core\Db\Connection` receiver.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not an object or whose handle slot
/// holds the wrong tag, on `Core\IO\File`'s reading of the identical pair: both
/// slots are written by [`nvs_core_db_connect`] and by nothing else, so either
/// is a paste error in this crate rather than anything a program can cause.
pub(super) fn connection_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
    let receiver = crate::instance::receiver(value, &CONNECTION, member)?;
    let key = crate::instance::slot(receiver, HANDLE_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{CONNECTION_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                CONNECTION.slots[HANDLE_AT]
            ))
        })?;
    Ok((key, crate::instance::slot(receiver, BLOCK_AT)))
}

/// The same pair off a [`TRANSACTION`], plus `rule:core-classes/db-transactions`'s first hazard.
///
/// The scope check is here rather than in each member because it is the
/// interface's rule and not any one member's: a `$tx` that escaped its
/// `transaction()` call still names a live connection, and running its
/// statement outside the transaction — silently, on whatever the connection is
/// doing now — is the failure § 7 closes by name.
///
/// # Errors
///
/// A thrown `LogicError` for a transaction whose call has returned. A
/// [`Fault::fatal`] for a slot of the wrong tag, as [`connection_of`].
pub(super) fn transaction_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
    let receiver = crate::instance::receiver(value, &TRANSACTION, member)?;
    if crate::instance::slot(receiver, SCOPE_AT).as_bool() != Some(true) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{TRANSACTION_NAME}::{member}: transaction scope has ended"),
        ));
    }
    let key = crate::instance::slot(receiver, HANDLE_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{TRANSACTION_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                TRANSACTION.slots[HANDLE_AT]
            ))
        })?;
    Ok((key, crate::instance::slot(receiver, BLOCK_AT)))
}

/// The connection a `Core\Db\Queryable` member runs on, off **either** receiver
/// the interface has.
///
/// This is the whole of `rule:classes/no-traits`'s delegation at runtime. `Transaction
/// implements Queryable by $connection` gives the two classes one set of rows
/// under one set of symbols ([`TRANSACTION`] says why), so the helper behind a
/// row is handed whichever receiver the call site wrote and asks here which one
/// it got — rather than four forwarding bodies that would each be a second
/// place for the statement path to be written.
///
/// The order is deliberate: a [`TRANSACTION`] is asked about first because it
/// is the receiver carrying an extra rule, and [`connection_of`] is the
/// fallthrough that also produces the `Fault::fatal` for anything that is
/// neither.
pub(super) fn handle_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
    if crate::instance::is_instance(value, &TRANSACTION) {
        return transaction_of(value, member);
    }
    connection_of(value, member)
}

/// One element of `$params`, as both halves of the statement path need it: what
/// it does to the SQL text, and the values the bind reads out of it.
///
/// The two are one walk's answers and are carried together because the rewriter
/// is told the arity **before** anything is encoded — `rule:core-classes/db-parameters`'s expansion
/// changes the text, and § 1's statement cache keys on the text it changed it
/// to, so an `inList`'s width has to be known a round trip early.
pub(super) struct Bound {
    /// The rewriter's view of this element: one marker, or § 5's run of them.
    binding: nvs_db::Binding,
    /// What binds at those markers — one value, or the whole run in the array's
    /// own order. **Keys are not read**, which is `inList`'s own rule: a marker
    /// binds positions inside one placeholder and not names.
    ///
    /// Borrowed rather than retained, as every value read out of an argument in
    /// this crate is: the caller's `$params` array owns them and outlives the
    /// call.
    values: Vec<Value>,
}

/// One element of `$params`, read as either an ordinary value or `rule:core-classes/db-parameters`'s expansion marker.
pub(super) fn bound_of(value: Value) -> Bound {
    if !crate::instance::is_instance(value, &IN_LIST) {
        return Bound {
            binding: nvs_db::Binding::One,
            values: vec![value],
        };
    }
    // Its one slot is written by `nvs_core_db_in_list` and by nothing else, and
    // that member refuses an empty list — so this is an array, and the arity
    // below is at least one.
    let held = crate::instance::slot(
        value.obj_ptr().expect("an `InList` is an object"),
        VALUES_AT,
    );
    let values = held
        .array_ptr()
        .map(|array| {
            let list = crate::arr::borrowed(array);
            let mut held = Vec::with_capacity(list.count());
            let mut from = 0usize;
            while let Some(slot) = list.next_slot(from) {
                from = slot + 1;
                held.push(
                    list.value_at(slot)
                        .expect("next_slot only names live entries"),
                );
            }
            held
        })
        .unwrap_or_default();
    Bound {
        binding: nvs_db::Binding::List(values.len()),
        values,
    }
}

/// A statement path failure, worded by which half of it went wrong.
///
/// The three-way split is this crate's to make, because `nvs-db` builds no
/// fault at all (its `Cargo.toml` § 1 is the rule) and answers in
/// [`std::io::ErrorKind`]s instead. `InvalidInput` is every mistake **in the
/// call** — the rewriter's refusals, a value with no bound form, a second
/// statement on a streaming connection — so it is a `LogicError` the caller
/// fixes by writing the call differently. `Other` is the server's own refusal,
/// carrying its `SQLSTATE` and message. Everything left is the wire.
///
/// The middle one is `Core\Db\DbError` — `rule:core-classes/db-error`'s single class for every refusal the server made, sitting beside
/// `Core\Db\RolledBack` in spec § 10's tree so that a `catch` can tell a
/// refusal the program did not choose from one it did. Nothing about the
/// message changes with the class, and § 8 requires it to carry no bound value.
///
/// The `Core\Db\DbError` it builds carries § 8's `kind`:
/// `nvs_db::ServerError::of(refused)` reads the driver's own classification
/// back out of the error this function is handed, and
/// [`Fault::thrown_with_slot`] writes it into
/// [`nvs_runtime::KIND_SLOT`] as the throw is recorded. A refusal the driver
/// answered with no [`nvs_db::ServerError`] behind it reads as `Other`, which
/// is what § 8 defines that case to be — the condition a code table does not
/// name — so the property is written on every path and never `null`.
///
/// **`sql` is the statement the caller wrote and not [`Statement::sql`]'s
/// rewrite of it**, and `None` for a member with no caller-written statement to
/// name. § 8 lets the text ride the throw because it is developer-authored,
/// which the rewritten form is only at one remove: that form spells its markers
/// the way one driver wants them — `$1` here, `?` on MySQL — so carrying it
/// would make a property of a deliberately *normalised* error read differently
/// per driver, which is the thing § 8's `kind` exists to stop. § 7's `BEGIN`,
/// `COMMIT` and `SAVEPOINT` pass `None` for the other half of the same reason:
/// that text is this runtime's, no program asked for it by name, and `?string`
/// already has an absent case that costs no slot.
pub(super) fn statement_failure(
    named: &str,
    block: &Value,
    sql: Option<&str>,
    refused: &std::io::Error,
) -> Fault {
    let name = block.as_text().unwrap_or("?");
    match refused.kind() {
        std::io::ErrorKind::InvalidInput => {
            Fault::thrown_as(ThrownClass::Logic, format!("{named}: {refused}"))
        }
        std::io::ErrorKind::Other => {
            let message = format!("{named}: `[db.{name}]` refused the statement: {refused}");
            // Built once for both arms below: whether the driver classified the
            // refusal says nothing about whether there was a statement behind
            // it, so `sql` is not the server's half of the error and does not
            // follow the server's.
            let wrote = sql.map(|text| {
                (
                    nvs_runtime::SQL_SLOT,
                    Value::str(NvsStr::new(text.as_bytes())),
                )
            });
            match nvs_db::ServerError::of(refused) {
                // The raw codes ride beside the kind normalised from them, so an
                // application that § 8's eleven conditions do not cover reads
                // what the server actually said without the driver having to
                // widen that enum. `driverCode` joins the `SQLSTATE` only on a
                // backend that sends a vendor integer as well — MySQL does and
                // PostgreSQL does not, and `nvs_db::ServerError` owns why. It is
                // never the `SQLSTATE` again under a second name.
                //
                // And the `SQLSTATE` itself joins only where there is one:
                // § 8 spells it `?string` because TDS has no such field at all,
                // and an empty `sql_state` is how that driver says so — the
                // field's own doc owns the reading. Writing it anyway would put
                // `""` where a program tests for `null`.
                Some(server) => {
                    let mut slots = vec![(nvs_runtime::KIND_SLOT, error_kind_value(server.kind))];
                    if !server.sql_state.is_empty() {
                        slots.push((
                            nvs_runtime::SQL_STATE_SLOT,
                            Value::str(NvsStr::new(server.sql_state.as_bytes())),
                        ));
                    }
                    if let Some(code) = server.driver_code {
                        slots.push((nvs_runtime::DRIVER_CODE_SLOT, Value::int(i64::from(code))));
                    }
                    // `constraint` joins them only where the condition named
                    // one, which most conditions do not. An unwritten slot
                    // already reads `null`, so the absent case costs no value
                    // here and no branch in the program that reads it — the
                    // same reason `driverCode` above is written only where a
                    // server sends one.
                    if let Some(constraint) = &server.constraint {
                        slots.push((
                            nvs_runtime::CONSTRAINT_SLOT,
                            Value::str(NvsStr::new(constraint.as_bytes())),
                        ));
                    }
                    slots.extend(wrote);
                    Fault::thrown_with_slots(ThrownClass::DbError, message, slots)
                }
                None => {
                    let mut slots = vec![(
                        nvs_runtime::KIND_SLOT,
                        error_kind_value(nvs_db::DbErrorKind::Other),
                    )];
                    slots.extend(wrote);
                    Fault::thrown_with_slots(ThrownClass::DbError, message, slots)
                }
            }
        }
        _ => Fault::thrown_as(
            ThrownClass::Io,
            format!("{named}: `[db.{name}]` failed while the statement was running: {refused}"),
        ),
    }
}

/// A [`nvs_db::DbErrorKind`] as the [`ERROR_KIND`] case a program matches on,
/// which at runtime is that case's ordinal
/// (`rule:enums/closed-integer-type`) — so a
/// `match ($e->kind) { Core\Db\ErrorKind::Deadlock => … }` reads what the
/// server itself said.
///
/// [`column_type_value`]'s rule, for its reason: the ordinal is looked up
/// rather than written a second time, and what is spelled here is only the
/// *name* correspondence neither half of the enum knows.
/// `every_db_error_kind_case_is_named` holds it total in both directions, which
/// is also what makes the `expect` unreachable.
pub(super) fn error_kind_value(of: nvs_db::DbErrorKind) -> Value {
    let case = error_kind_case(of);
    let (_, ordinal) = ERROR_KIND
        .cases
        .iter()
        .find(|(name, _)| *name == case)
        .expect("every `nvs_db::DbErrorKind` names a case `ERROR_KIND` registers");
    Value::int(*ordinal)
}

/// The [`nvs_db::DbErrorKind`] a `Core\Db\ErrorKind` value is, or `None` for
/// anything that is not one of its ordinals — [`error_kind_value`] read
/// backwards, which is how § 7's retry rule asks what a *thrown* `Db\DbError`
/// carries when there is no [`nvs_db::ServerError`] left to ask.
///
/// Inverted through the forward function rather than written as a second
/// `match`: the name correspondence exists once, in [`error_kind_case`], and a
/// table spelled out again here would be free to disagree with it. The scan is
/// over eleven entries on a failure path.
pub(super) fn error_kind_of(value: Value) -> Option<nvs_db::DbErrorKind> {
    let ordinal = value.as_int()?;
    EVERY_ERROR_KIND
        .into_iter()
        .find(|kind| error_kind_value(*kind).as_int() == Some(ordinal))
}

/// Every [`nvs_db::DbErrorKind`], in [`ERROR_KIND`]'s own order.
///
/// Written out because that enum carries no roster of its own, and guarded
/// rather than trusted: `every_db_error_kind_case_is_named` maps this list
/// through [`error_kind_case`] and compares it against the registered cases,
/// so a variant left out here is a case name with nothing producing it.
pub(super) const EVERY_ERROR_KIND: [nvs_db::DbErrorKind; 11] = [
    nvs_db::DbErrorKind::UniqueViolation,
    nvs_db::DbErrorKind::ForeignKeyViolation,
    nvs_db::DbErrorKind::NotNullViolation,
    nvs_db::DbErrorKind::CheckViolation,
    nvs_db::DbErrorKind::Deadlock,
    nvs_db::DbErrorKind::SerializationFailure,
    nvs_db::DbErrorKind::ConnectionLost,
    nvs_db::DbErrorKind::Timeout,
    nvs_db::DbErrorKind::Syntax,
    nvs_db::DbErrorKind::Permission,
    nvs_db::DbErrorKind::Other,
];

/// The [`ERROR_KIND`] case one [`nvs_db::DbErrorKind`] is, by name.
///
/// Exhaustive on purpose — a variant added over there arrives here as a
/// non-exhaustive `match` rather than as a refusal that classifies wrongly.
pub(super) fn error_kind_case(of: nvs_db::DbErrorKind) -> &'static str {
    match of {
        nvs_db::DbErrorKind::UniqueViolation => "UniqueViolation",
        nvs_db::DbErrorKind::ForeignKeyViolation => "ForeignKeyViolation",
        nvs_db::DbErrorKind::NotNullViolation => "NotNullViolation",
        nvs_db::DbErrorKind::CheckViolation => "CheckViolation",
        nvs_db::DbErrorKind::Deadlock => "Deadlock",
        nvs_db::DbErrorKind::SerializationFailure => "SerializationFailure",
        nvs_db::DbErrorKind::ConnectionLost => "ConnectionLost",
        nvs_db::DbErrorKind::Timeout => "Timeout",
        nvs_db::DbErrorKind::Syntax => "Syntax",
        nvs_db::DbErrorKind::Permission => "Permission",
        nvs_db::DbErrorKind::Other => "Other",
    }
}

/// The refusal for a column of one of `rule:core-classes/db-column-types`'s five class-typed rows
/// holding a value the `Core\Time` type it maps to has no representation for.
///
/// § 9's last paragraph is the rule: a structured column that does not parse
/// throws rather than reading back as something else. Three values reach it in
/// practice — PostgreSQL's `TIME` of `24:00:00`, which is a reading
/// `Core\Time\TimeOfDay` deliberately does not have, MySQL's zero date
/// `0000-00-00`, which its own driver hands over unchecked because the calendar
/// is over here ([`nvs_db::MySqlDate`]), and a year outside the calendar
/// `Core\Time`'s types count. `crate::time`'s seams own every one of those
/// bounds and answer `None`; naming the column is this side's half, since that
/// is what the program's next act needs.
pub(super) fn unrepresentable_column(named: &str, column: &str, row: &str) -> Fault {
    Fault::thrown(format!(
        "{named}: the column `{column}` holds a {row} that no `Core\\Time` type has a value for \
         — PostgreSQL's `24:00:00`, MySQL's zero date and a year outside the calendar \
         `Core\\Time\\Date` counts are the three — and a cast to text in the statement reads one \
         back as the server rendered it"
    ))
}

/// A statement the wire is ready for: which connection it goes to, § 5's
/// rewritten text, and its values encoded in the order that text asks for them.
///
/// The two members that send one differ **only in what they do with the
/// answer**. `rule:core-classes/db-statement-members` gives `query` and `execute` one signature and one
/// binding rule, so everything up to the send is [`statement_of`] and the
/// members are the two ways of reading a stream that has already started —
/// which is also why a write's values are checked exactly as a read's are, with
/// no second path for a caller to find a difference in.
pub(super) struct Statement {
    /// The key its connection is filed under in the request's own table.
    pub(super) key: u64,
    /// The `[db.<name>]` block it was opened by, so a refusal can name the
    /// connection without holding it.
    pub(super) block: Value,
    /// § 5's rewritten text, in the driver's own placeholder spelling. A
    /// refusal names the caller's own spelling instead — [`statement_failure`]'s
    /// `sql` parameter — because this one is a property of the driver.
    pub(super) sql: String,
    /// One entry per marker that text holds, in the **statement's** order and
    /// never the array's.
    pub(super) binds: Binds,
}

/// One statement's bound values, in whichever form its driver takes them.
///
/// [`Encoder`]'s split, one step later: the four drivers with a wire read a
/// parameter as octets, and SQLite takes a [`nvs_db::SqliteValue`] by value —
/// `nvs_db::sqlite`'s module doc owns why that one is owned rather than
/// borrowed.
///
/// **One tag for the whole vector and not one per element**, because what it
/// says is a property of the *statement*: every bind of one statement is its own
/// connection's driver's, [`rendering_of`] having read that driver off the
/// connection the send then matches on. A per-element tag would cost a
/// discriminant per parameter and would be able to describe a mixture nothing
/// can send.
pub(super) enum Binds {
    /// PostgreSQL, MySQL, MariaDB and SQL Server: `None` where the bound value
    /// is `null`, which is how all four spell it on the wire.
    Wire(Vec<Option<Vec<u8>>>),
    /// SQLite: one storage class per marker, `SqliteValue::Null` among them.
    Sqlite(Vec<nvs_db::SqliteValue>),
}

impl Binds {
    /// The wire drivers' view: one borrowed slice per marker.
    ///
    /// Empty for a SQLite statement, which no caller reads rather than a wrong
    /// answer any caller could act on: the four arms that take this are reached
    /// by matching the same connection [`rendering_of`] chose the encoder off,
    /// so a statement bound one way and sent the other is a bug in this file and
    /// not a state a program can reach.
    pub(super) fn wire(&self) -> Vec<Option<&[u8]>> {
        match self {
            Binds::Wire(rendered) => rendered.iter().map(|one| one.as_deref()).collect(),
            Binds::Sqlite(_) => Vec::new(),
        }
    }

    /// SQLite's view, cloned because `nvs_db::SqliteConn::query` takes its
    /// parameters owned — one `Vec` per send, which for a batch is one per set
    /// and is what its own `execute_many` is handed.
    pub(super) fn sqlite(&self) -> Vec<nvs_db::SqliteValue> {
        match self {
            Binds::Sqlite(values) => values.clone(),
            Binds::Wire(_) => Vec::new(),
        }
    }
}

/// Everything `rule:core-classes/db-statement-members` and `rule:core-classes/db-parameters` do to a call before it reaches the socket:
/// § 18's `$params` rule, the rewrite, and the encoding.
///
/// **Both halves are the receiver's own driver's**, which is the one thing a
/// caller cannot state: [`rendering_of`] reads it off the connection filed
/// under the receiver's key, and § 5's rewrite and § 9's encoding follow it
/// together. The context is borrowed for that lookup alone and released before
/// the caller reaches [`transacting`], so a member still binds and sends inside
/// one borrow each.
///
/// Both spellings of the member's name are passed because two things want
/// different ones: `member` is the bare name [`connection_of`] builds
/// `Class::member` out of, and `named` is the whole spelling the messages here
/// already hold a class in. The argument slots are read the same way for each
/// member, since § 18 gives both the identical two parameters.
///
/// # Errors
///
/// [`rendering_of`]'s throw for a driver with no statement path yet, a thrown
/// `LogicError` for a `$params` keyed both ways at once, and whatever
/// [`statement_failure`] makes of the rewriter's and the encoder's refusals. A
/// [`Fault::fatal`] for an argument of the wrong tag, which the registry row
/// refuses first.
pub(super) fn statement_of(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    member: &str,
    named: &str,
) -> Result<Statement, Fault> {
    let (key, block) = handle_of(args[0], member)?;
    let (dialect, encode) = rendering_of(ctx, key, &block, named)?;
    statement_in(dialect, encode, key, block, args, named)
}

/// [`statement_of`] with the connection already asked about, so a batch asks
/// once for every set it binds rather than once per set.
///
/// The key and the block are passed in for that reason and not carried back out
/// of [`handle_of`] again: they are the receiver's, and every set of a batch has
/// the same one.
///
/// # Errors
///
/// [`statement_of`]'s, less the lookup it has already done.
pub(super) fn statement_in(
    dialect: nvs_db::Dialect,
    encode: Encoder,
    key: u64,
    block: Value,
    args: &[Value],
    named: &str,
) -> Result<Statement, Fault> {
    // Unreachable from source: parameter 0 is a `string` in `CONNECTION`
    // above, so a non-text argument is refused at `E0401` first — the same
    // judgement `Core\Db::quoteIdentifier`'s guard states.
    let sql = args[1].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{named} expected a `string` statement, got tag {}",
            args[1].tag_byte()
        ))
    })?;
    // Unreachable for that reason too: the row declares `array<mixed>`.
    let params = args[2].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{named} expected {:?} for its parameters, got tag {}",
            Tag::Array,
            args[2].tag_byte()
        ))
    })?;

    // § 18: "list-keyed for `?`, string-keyed for `:name`, mixing throws".
    // The refusal is here rather than in the rewriter because an array is
    // the only thing that can be both, and the rewriter is handed one form.
    let held = crate::arr::borrowed(params);
    let mut positional: Vec<Bound> = Vec::new();
    let mut keys: Vec<(String, Bound)> = Vec::new();
    let mut from = 0usize;
    while let Some(slot) = held.next_slot(from) {
        from = slot + 1;
        let value = held
            .value_at(slot)
            .expect("next_slot only names live entries");
        let bound = bound_of(value);
        match held
            .slot_key(slot)
            .expect("next_slot only names live entries")
        {
            nvs_runtime::SlotKey::Index(_) if keys.is_empty() => positional.push(bound),
            nvs_runtime::SlotKey::Str(name) if positional.is_empty() => {
                // A key is a Novis `string` and so is UTF-8 by `rule:types/bytes`;
                // the lossy read is the spelling that needs no unreachable
                // arm to say so.
                keys.push((String::from_utf8_lossy(name.as_bytes()).into_owned(), bound));
            }
            _ => {
                return Err(Fault::thrown_as(
                    ThrownClass::Logic,
                    format!(
                        "{named}: `$params` is keyed both ways at once, and a statement is \
                         written one way or the other — a list-keyed array binds `?` in \
                         order, a string-keyed one binds `:name`"
                    ),
                ));
            }
        }
    }

    let arities: Vec<nvs_db::Binding> = positional.iter().map(|bound| bound.binding).collect();
    let keyed: Vec<(&str, nvs_db::Binding)> = keys
        .iter()
        .map(|(name, bound)| (name.as_str(), bound.binding))
        .collect();
    let spelling = if keys.is_empty() {
        nvs_db::Params::Positional(&arities)
    } else {
        nvs_db::Params::Named(&keyed)
    };
    let rewritten = nvs_db::rewrite(sql, spelling, dialect)
        .map_err(|refused| statement_failure(named, &block, Some(sql), &refused))?;

    let bounds: Vec<&Bound> = if keys.is_empty() {
        positional.iter().collect()
    } else {
        keys.iter().map(|(_, bound)| bound).collect()
    };
    // A loop per shape rather than one loop with a `match` inside it: the rule
    // is identical and only the vector's type is not, and the refusal a value
    // draws is [`statement_failure`]'s on either side.
    let binds = match encode {
        Encoder::Wire(encode) => {
            let mut rendered: Vec<Option<Vec<u8>>> = Vec::with_capacity(rewritten.binds.len());
            for source in &rewritten.binds {
                rendered
                    .push(encode(bounds[source.arg].values[source.element]).map_err(
                        |refused| statement_failure(named, &block, Some(sql), &refused),
                    )?);
            }
            Binds::Wire(rendered)
        }
        Encoder::Sqlite(encode) => {
            let mut rendered: Vec<nvs_db::SqliteValue> = Vec::with_capacity(rewritten.binds.len());
            for source in &rewritten.binds {
                rendered
                    .push(encode(bounds[source.arg].values[source.element]).map_err(
                        |refused| statement_failure(named, &block, Some(sql), &refused),
                    )?);
            }
            Binds::Sqlite(rendered)
        }
    };

    Ok(Statement {
        key,
        block,
        sql: rewritten.sql,
        binds,
    })
}

/// `rule:core-classes/db-statement-members`'s batch: one statement the wire is ready for, and one encoded
/// set of values per execution it is about to get.
///
/// It is deliberately not a `Vec<Statement>`. Every set rewrites to the *same*
/// text or the batch is refused, so the text is held once here — which is also
/// the invariant, written into the shape rather than left as a rule
/// [`batch_of`] has to be trusted to have checked.
pub(super) struct Batch {
    /// The key its connection is filed under in the request's own table.
    pub(super) key: u64,
    /// The `[db.<name>]` block it was opened by, so a refusal can name the
    /// connection without holding it.
    pub(super) block: Value,
    /// § 5's rewritten text, which every set agreed on — or, for an empty
    /// `$sets`, the caller's own text unrewritten, which never reaches the wire
    /// because [`nvs_db::PgConn::execute_many`] answers `0` before it prepares
    /// anything.
    pub(super) sql: String,
    /// One encoded set per execution, each in the **statement's** order, as
    /// [`Statement::binds`] is — and each in its driver's own shape, since every
    /// set of one batch went through the one [`Encoder`] its connection chose.
    pub(super) binds: Vec<Binds>,
}

/// § 18's `$sets`, read as one [`statement_of`] per set with the expansions
/// checked to agree.
///
/// **Each set is a whole `$params`**, so it gains § 18's keying rule, § 5's
/// rewrite and § 9's encoding from the member that already owns them — a set
/// cannot be bound by a weaker rule than the one `execute` would have applied
/// to it on its own.
///
/// **The sets must rewrite to one text, and that is a stronger check than the
/// arity one it looks like.** § 1's cache is keyed on the SQL *plus its
/// expansion arity* and § 5 expands an `inList` into as many markers as it has
/// elements, so two sets whose `inList`s differ in width are two prepared
/// statements — the driver's own `execute_many` refuses them on the count, and
/// this refuses them on the text, which is the thing the count stands for and
/// can name in the message.
///
/// # Errors
///
/// A thrown `LogicError` for two sets that do not rewrite alike, plus whatever
/// [`statement_in`] throws for any one of them and [`rendering_of`]'s throw for
/// a driver with no statement path yet. A [`Fault::fatal`] for an argument of
/// the wrong tag, which the registry row refuses first.
pub(super) fn batch_of(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    member: &str,
    named: &str,
) -> Result<Batch, Fault> {
    let (key, block) = handle_of(args[0], member)?;
    // Once for the batch: every set binds to the same connection, so asking per
    // set would be the same answer read `$sets` times.
    let (dialect, encode) = rendering_of(ctx, key, &block, named)?;
    // Unreachable from source for both, as in `statement_of`: the row declares
    // a `string` and an `array<array<mixed>>`, so `E0401` refuses either tag
    // first.
    let sql = args[1].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{named} expected a `string` statement, got tag {}",
            args[1].tag_byte()
        ))
    })?;
    let given = args[2].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{named} expected {:?} for its parameter sets, got tag {}",
            Tag::Array,
            args[2].tag_byte()
        ))
    })?;

    let held = crate::arr::borrowed(given);
    let mut text: Option<String> = None;
    let mut binds: Vec<Binds> = Vec::with_capacity(held.count());
    let mut from = 0usize;
    while let Some(slot) = held.next_slot(from) {
        from = slot + 1;
        let set = held
            .value_at(slot)
            .expect("next_slot only names live entries");
        let at = binds.len();
        // The inner type's turn to be unreachable: the element is an
        // `array<mixed>` by the row, and `statement_of` would call a non-array
        // one a wrong parameter tag without saying which set it was.
        if set.array_ptr().is_none() {
            return Err(Fault::fatal(format!(
                "{named} expected {:?} for the set at {at}, got tag {}",
                Tag::Array,
                set.tag_byte()
            )));
        }
        let one = statement_in(dialect, encode, key, block, &[args[0], args[1], set], named)?;
        match &text {
            None => text = Some(one.sql),
            Some(first) if *first == one.sql => {}
            Some(first) => {
                return Err(Fault::thrown_as(
                    ThrownClass::Logic,
                    format!(
                        "{named}: the set at {at} binds `{}` where the first set binds `{first}`, \
                         and `rule:core-classes/db-one-api`'s cache is keyed on the statement's expansion — so two \
                         sets whose `inList`s differ in width are two statements and not one \
                         batch, and each of them wants its own call",
                        one.sql
                    ),
                ));
            }
        }
        binds.push(one.binds);
    }

    Ok(Batch {
        key,
        block,
        sql: text.unwrap_or_else(|| sql.to_owned()),
        binds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same agreement for `rule:core-classes/db-error`'s `ErrorKind`, and it fails the same
    /// two ways: a case [`error_kind_case`] never names is a condition a
    /// program can `match` on and never receive, and a name it produces that
    /// [`ERROR_KIND`] does not register is [`error_kind_value`]'s `expect`
    /// firing inside [`statement_failure`] — on the throw path of a real
    /// refusal, which is the one of the two that reaches a request.
    ///
    /// Reading the list off [`EVERY_ERROR_KIND`] is what also guards *that*:
    /// a variant added to the driver's enum and to [`error_kind_case`] but not
    /// to the roster leaves a registered case with nothing describing it, and
    /// the comparison below is where that shows up.
    #[test]
    fn every_db_error_kind_case_is_named() {
        let described: Vec<&'static str> =
            EVERY_ERROR_KIND.into_iter().map(error_kind_case).collect();
        let registered: Vec<&'static str> =
            ERROR_KIND.cases.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            described, registered,
            "`nvs_db::DbErrorKind` and `{ERROR_KIND_NAME}` are one enum, in § 8's own order — a \
             case added to either belongs in both, and in the same place"
        );
    }

    /// § 7's retry reads a kind back out of a thrown `Db\DbError`, so the two
    /// halves of that round trip have to agree for every case — including the
    /// two [`nvs_db::DbErrorKind::is_retryable`] names, which is the only
    /// answer the loop acts on.
    #[test]
    fn a_kind_written_into_a_throw_reads_back_as_itself() {
        for kind in EVERY_ERROR_KIND {
            assert_eq!(
                error_kind_of(error_kind_value(kind)),
                Some(kind),
                "`{}` did not survive the round trip",
                error_kind_case(kind)
            );
        }
        assert_eq!(error_kind_of(Value::null()), None);
        let past_the_end = i64::try_from(EVERY_ERROR_KIND.len()).expect("eleven cases");
        assert_eq!(
            error_kind_of(Value::int(past_the_end)),
            None,
            "one past the last ordinal is no case at all"
        );
    }

    /// § 8's `driverCode` reaches the throw where the server sent one, and is
    /// left unwritten where it did not.
    ///
    /// **Both sides, because the drivers disagree and the slot cannot be
    /// decided by the class.** MySQL words a refusal with a vendor integer
    /// beside its `SQLSTATE`; PostgreSQL's `SQLSTATE` is its only code, so a
    /// slot filled unconditionally would invent one for it — the failure this
    /// asserts against — and a slot never filled loses the other's, which is
    /// what a program hard-coding `1062` reads. `nvs_db::ServerError` owns why
    /// only one of them has it.
    #[test]
    fn a_driver_code_reaches_the_throw_only_where_the_server_sent_one() {
        let block = Value::str(NvsStr::new(b"main"));
        let refusal = |driver_code| {
            std::io::Error::other(nvs_db::ServerError {
                kind: nvs_db::DbErrorKind::UniqueViolation,
                sql_state: String::from("23000"),
                severity: String::from("ERROR"),
                message: String::from("duplicate"),
                constraint: None,
                driver_code,
                backend: "mysql",
            })
        };
        let code_in = |fault| match fault {
            Fault::ThrownWithSlots(ThrownClass::DbError, _, slots) => slots
                .iter()
                .find(|(slot, _)| *slot == nvs_runtime::DRIVER_CODE_SLOT)
                .map(|(_, value)| value.as_int()),
            other => panic!("§ 8 makes a server's refusal a `Db\\DbError`: {other:?}"),
        };

        assert_eq!(
            code_in(statement_failure(QUERY, &block, None, &refusal(Some(1062)))),
            Some(Some(1062)),
            "MySQL's own integer is what an application reads when § 8's kind is \
             not specific enough for it"
        );
        assert_eq!(
            code_in(statement_failure(QUERY, &block, None, &refusal(None))),
            None,
            "and an unwritten slot already reads `null`, which is the whole of \
             what PostgreSQL has to say here"
        );
    }

    /// `rule:core-classes/db-transactions`'s first hazard, on the member most likely to meet
    /// it: a `$tx` its closure carried out of the `transaction()` call still
    /// names a live connection, so a write through it would run outside every
    /// transaction and on whatever that connection is doing now.
    ///
    /// **Asserted through [`statement_of`] rather than [`transaction_of`]**,
    /// because the guard is only worth anything where it sits in the statement
    /// path: the context here has no connection filed under the key, so
    /// [`rendering_of`] is a `Fault::fatal` waiting one line further on, and a
    /// thrown `LogicError` means [`handle_of`] refused first. The two receivers
    /// share one body and one symbol — [`TRANSACTION`] says why — so this is
    /// the only thing that tells `Core\Db\Transaction::execute` apart from the
    /// connection's, and the message it carries is the class it was reached
    /// through rather than the `named` constant, which is the connection's on
    /// both paths.
    // covers: Core\Db\Transaction::execute
    #[test]
    fn a_write_through_an_escaped_transaction_is_refused_before_it_reaches_a_connection() {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        // The last two slots are this class's own: the scope flag § 7 clears
        // when the call returns, and the reason a `rollBack` would have left.
        let escaped = |open: bool| {
            crate::instance::build(
                &TRANSACTION,
                [
                    Value::uint(0),
                    Value::str(NvsStr::new(b"main")),
                    Value::bool(open),
                    Value::null(),
                ],
            )
        };
        // Only the receiver is read on either path below, the refusal landing
        // before the statement is bound at all.
        let args = |receiver| {
            [
                receiver,
                Value::str(NvsStr::new(b"insert into t (name) values (?)")),
                Value::null(),
                Value::null(),
            ]
        };

        let refused = statement_of(&mut ctx, &args(escaped(false)), "execute", EXECUTE)
            .err()
            .expect("a transaction whose call has returned refuses every statement");
        match refused {
            Fault::Thrown(ThrownClass::Logic, message) => assert!(
                message.contains(TRANSACTION_NAME) && message.contains("execute"),
                "§ 7's refusal names the handle the write was attempted through: {message}"
            ),
            other => panic!("an escaped transaction is a `LogicError`, not {other:?}"),
        }

        // And the other side of the bound: the same instance with its scope
        // still open is past the guard, so what stops it is the missing
        // connection rather than anything this member decided.
        let past = statement_of(&mut ctx, &args(escaped(true)), "execute", EXECUTE)
            .err()
            .expect("no connection is filed under the key this fixture wrote");
        assert!(
            !matches!(past, Fault::Thrown(ThrownClass::Logic, _)),
            "an open transaction reaches the connection lookup: {past:?}"
        );
    }

    /// § 4's empty batch is where `rule:core-classes/db-transactions`'s scope guard is easiest to
    /// lose: a member that answered `0` for a `$sets` holding nothing, before
    /// it had read its receiver at all, would pass every other assertion in
    /// this tree and still let a program reach a transaction its owning call
    /// had already returned from.
    ///
    /// **The rule is the interface's and not one member's, so the batch path
    /// owes what the single-statement path owes.** [`batch_of`] and
    /// [`statement_of`] read a receiver through the one [`handle_of`], and
    /// what this asserts is that the two **agree** — a batch that grew a
    /// receiver reading of its own would answer plausibly here and disagree
    /// with its neighbour.
    // covers: Core\Db\Transaction::executeMany
    #[test]
    fn an_empty_batch_through_an_escaped_transaction_is_refused_as_a_statement_is() {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        let escaped = || {
            crate::instance::build(
                &TRANSACTION,
                [
                    Value::uint(0),
                    Value::str(NvsStr::new(b"main")),
                    Value::bool(false),
                    Value::null(),
                ],
            )
        };
        // A real empty array and not a `null`, because the whole question is
        // what a batch with no work in it does before it reaches a connection.
        let sets = Value::array(NvsArray::new());
        let args = |receiver| {
            [
                receiver,
                Value::str(NvsStr::new(b"insert into t (name) values (?)")),
                sets,
                Value::null(),
            ]
        };

        let refusal = |fault| match fault {
            Fault::Thrown(ThrownClass::Logic, message) => message,
            other => panic!("§ 7 makes an escaped transaction a `LogicError`, not {other:?}"),
        };
        let batched = refusal(
            batch_of(&mut ctx, &args(escaped()), "executeMany", EXECUTE_MANY)
                .err()
                .expect("an empty batch reads its receiver before it answers `0`"),
        );
        assert!(
            batched.contains(TRANSACTION_NAME) && batched.contains("executeMany"),
            "the refusal names the handle the batch was attempted through: {batched}"
        );

        let single = refusal(
            statement_of(&mut ctx, &args(escaped()), "execute", EXECUTE)
                .err()
                .expect("the single-statement path refuses the same receiver"),
        );
        assert_eq!(
            batched.replace("executeMany", "execute"),
            single,
            "one interface, one guard: the two paths differ in the member they name and in \
             nothing else"
        );

        #[expect(
            unsafe_code,
            reason = "the reference released here is the one this frame built, and \
                      neither path above retains its arguments"
        )]
        unsafe {
            sets.release();
        }
    }

    /// `rule:core-classes/db-transactions`'s scope guard on the read path, and where in that path it
    /// answers: [`crate::db::execute::queried_rows`] reads its receiver through
    /// [`statement_of`] **before** it reads the statement's own `timeout`, so a
    /// transaction whose call has returned is refused for the handle rather
    /// than for anything the options said.
    ///
    /// **The order is the claim, so both halves of it are asserted.** The
    /// arguments below carry a zero `timeout`, which
    /// [`crate::db::execute::statement_deadline`] refuses on its own — asserted
    /// here first, because without it the `LogicError` further down proves only
    /// that *something* refused. A read path that asked its options first would
    /// answer that refusal instead, and a program holding an escaped `$tx`
    /// would be told to fix its duration.
    ///
    /// Reached through `queried_rows` rather than `statement_of`, which is the
    /// only thing that makes this the *read* path's assertion: § 18's buffered
    /// read has a preamble of its own, and a guard reached only from the member
    /// bodies below it is one a later caller can skip.
    // covers: Core\Db\Transaction::query
    #[test]
    fn a_read_through_an_escaped_transaction_is_refused_before_its_options_are_read() {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        // The last two slots are this class's own: the scope flag § 7 clears
        // when the call returns, and the reason a `rollBack` would have left.
        let handle = |open: bool| {
            crate::instance::build(
                &TRANSACTION,
                [
                    Value::uint(0),
                    Value::str(NvsStr::new(b"main")),
                    Value::bool(open),
                    Value::null(),
                ],
            )
        };
        // A zero is the shortest way to a `timeout` that refuses, and
        // `rule:http-server/no-spelling-for-an-unbounded-wait` is why it is not read as "no bound".
        let zero = crate::time::duration_of(0);
        let args = |receiver| {
            [
                receiver,
                Value::str(NvsStr::new(b"select id from t")),
                Value::null(),
                zero,
            ]
        };

        assert!(
            matches!(
                crate::db::execute::statement_deadline(&args(handle(true)), QUERY),
                Err(Fault::Thrown(..))
            ),
            "the options this case carries have to be refusable, or the order below proves nothing"
        );

        let refused =
            crate::db::execute::queried_rows(&mut ctx, &args(handle(false)), "query", QUERY)
                .err()
                .expect("a transaction whose call has returned refuses every statement");
        match refused {
            Fault::Thrown(ThrownClass::Logic, message) => assert_eq!(
                message,
                format!("{TRANSACTION_NAME}::query: transaction scope has ended"),
                "§ 7's refusal names the handle the read was attempted through, and says nothing \
                 about the `timeout` it never got to"
            ),
            other => panic!("an escaped transaction is a `LogicError`, not {other:?}"),
        }

        // And the other side of the bound: the same arguments with the scope
        // still open are past the guard, so what stops them is the missing
        // connection rather than anything this member decided.
        let past = crate::db::execute::queried_rows(&mut ctx, &args(handle(true)), "query", QUERY)
            .err()
            .expect("no connection is filed under the key this fixture wrote");
        assert!(
            !matches!(past, Fault::Thrown(ThrownClass::Logic, _)),
            "an open transaction reaches the connection lookup: {past:?}"
        );

        #[expect(
            unsafe_code,
            reason = "the duration released here is the one this frame built, and \
                      no path above retains its arguments"
        )]
        unsafe {
            zero.release();
        }
    }

    /// `crate::registry::WRITTEN_CLASS_MEMBERS` is keyed by the class a call is
    /// *resolved to*, so `rule:classes/no-traits`'s delegation puts each written member on it
    /// twice — and `queryAs` is where a missing second row is felt first. That
    /// roster is what the lowering reads to put a descriptor and its two
    /// companions ahead of the receiver, and the member body slices `args[3..]`
    /// because of them: a `Core\Db\Transaction` row nobody wrote would send the
    /// same helper four arguments where it reads seven, losing the class the
    /// call site named.
    ///
    /// **Asserted as agreement over the whole interface rather than for the one
    /// member**, because a single row is what goes missing: a member both
    /// classes declare is written on both of them or on neither, and a `streamAs`
    /// added to one spelling alone fails here rather than at a call site.
    // covers: Core\Db\Transaction::queryAs
    #[test]
    fn a_written_member_is_on_the_roster_under_both_of_its_receivers() {
        let shared: Vec<&str> = TRANSACTION
            .instance
            .iter()
            .map(|member| member.name)
            .filter(|name| CONNECTION.instance.iter().any(|twin| twin.name == *name))
            .collect();
        assert!(
            shared.contains(&"queryAs"),
            "the two receivers share the member this case is about: {shared:?}"
        );

        let written = |class: &str| -> Vec<&str> {
            shared
                .iter()
                .copied()
                .filter(|name| crate::registry::takes_written_class(class, name))
                .collect()
        };
        assert_eq!(
            written(TRANSACTION_NAME),
            written(CONNECTION_NAME),
            "one interface, one roster: a member written at its call site is written through \
             either handle"
        );
        assert!(
            written(TRANSACTION_NAME).contains(&"queryAs"),
            "`queryAs` reads its receiver past three leading constants that only this roster puts \
             there"
        );
    }

    /// `rule:classes/no-traits`'s delegation is *one symbol* per shared member, and `stream` is
    /// where a second body would be felt first: the walk parks a portal on the
    /// connection, so § 4's connection-busy `LogicError` is thrown by the
    /// connection's own member and names it — `Core\Db\Connection::execute`,
    /// even where the program wrote `$tx->execute(…)`. A forwarding body of the
    /// transaction's own would give that one rule two messages and two places to
    /// decide it.
    ///
    /// **Asserted as agreement over every shared member rather than for
    /// `stream` alone**, because one row is what drifts: a member both classes
    /// declare resolves to one helper or the rules underneath it are stated
    /// twice, and a row that grew a symbol of its own fails here rather than at
    /// the call site that meets the second message.
    // covers: Core\Db\Transaction::stream
    #[test]
    fn every_member_both_db_handles_declare_resolves_to_the_connections_own_symbol() {
        let paired: Vec<(&str, &str, &str)> = TRANSACTION
            .instance
            .iter()
            .filter_map(|member| {
                CONNECTION
                    .instance
                    .iter()
                    .find(|twin| twin.name == member.name)
                    .map(|twin| (member.name, member.symbol, twin.symbol))
            })
            .collect();
        assert!(
            paired.iter().any(|(name, ..)| *name == "stream"),
            "the two receivers share the member this case is about: {paired:?}"
        );

        let apart: Vec<&str> = paired
            .iter()
            .filter(|(_, mine, theirs)| mine != theirs)
            .map(|(name, ..)| *name)
            .collect();
        assert!(
            apart.is_empty(),
            "a shared member resolving to a body of its own states § 4's rules twice: {apart:?}"
        );
    }

    /// Reduces a return type to the container it names and what one step of the
    /// walk over it yields, so the four rows below compare as two words rather
    /// than as [`CoreTy`]s — which carry an `f64` and are deliberately not
    /// comparable.
    fn walked(ty: &CoreTy) -> Option<(&'static str, &'static str)> {
        match ty {
            CoreTy::InstanceAt(container, [CoreTy::Written(_)]) => Some((container, "written")),
            CoreTy::InstanceAt(container, [CoreTy::Instance(row)]) if *row == ROW_NAME => {
                Some((container, "row"))
            }
            _ => None,
        }
    }

    /// `streamAs` is where two axes cross — `queryAs`'s hydration and `stream`'s
    /// constant memory — so what it owes is that neither axis moved the other.
    /// A `…As` member answers the container its plain twin answers, at the class
    /// the call site wrote instead of at [`ROW_NAME`]; the pair is the whole
    /// claim, and the container half is what a member cannot decide on its own.
    ///
    /// **Asserted as agreement over every such pair on both receivers rather
    /// than for `streamAs` alone.** A member that grew a `Rows` return under a
    /// streaming name, or a plain `Row` under an `As` name, still reads
    /// correctly on its own line and is a different member from the one its name
    /// promises; it fails here. Compared against the *twin's* container rather
    /// than against a written-down name, so adding a sixth `…As` member needs no
    /// edit and is covered the day it lands.
    // covers: Core\Db\Transaction::streamAs
    #[test]
    fn every_as_member_answers_its_plain_twins_container_at_the_written_class() {
        let mut checked: Vec<&str> = Vec::new();
        for table in [&TRANSACTION, &CONNECTION] {
            for member in table.instance {
                let Some(plain) = member.name.strip_suffix("As") else {
                    continue;
                };
                let twin = table
                    .instance
                    .iter()
                    .find(|other| other.name == plain)
                    .unwrap_or_else(|| {
                        panic!(
                            "`{}` on `{}` has no `{plain}` twin",
                            member.name, table.name
                        )
                    });
                assert_eq!(
                    walked(&member.return_ty),
                    Some((
                        walked(&twin.return_ty).expect("the twin walks rows").0,
                        "written"
                    )),
                    "`{}` on `{}` does not answer `{plain}`'s container at the written class",
                    member.name,
                    table.name
                );
                checked.push(member.name);
            }
        }
        assert!(
            checked.contains(&"streamAs") && checked.contains(&"queryAs"),
            "both hydrating members are on the sweep this case is about: {checked:?}"
        );
    }

    /// Reduces a callable parameter to the two names the case below compares:
    /// the class it hands its closure, and the variable that closure answers.
    fn handed_over(ty: &CoreTy) -> Option<(&'static str, &'static str)> {
        match *ty {
            CoreTy::CallableSig(&[CoreTy::Instance(given)], &CoreTy::Var(answered)) => {
                Some((given, answered))
            }
            _ => None,
        }
    }

    /// The variable a return type names, for the same reason.
    fn answered(ty: &CoreTy) -> Option<&'static str> {
        match *ty {
            CoreTy::Var(name) => Some(name),
            _ => None,
        }
    }

    /// A nested `transaction` is the same row as the outermost one, so what
    /// makes § 7's nesting compose is that neither what the closure is handed
    /// nor what the member answers depends on the receiver: the closure is given
    /// a [`TRANSACTION`], and the member answers the very variable that closure
    /// declared. A library opening a transaction for its own writes then reads
    /// the same types inside a caller's transaction as outside one.
    ///
    /// **Asserted as the identity between the callable's answer and the row's,
    /// on every receiver that declares the row.** Either half alone is satisfied
    /// by the wrong shape: a row answering a variable of its own typechecks
    /// against itself and still loses the closure's type at the call site, and
    /// one handing over a [`CONNECTION`] would give a step the four members
    /// § 7 keeps off a transaction. The row is declared once and carried twice,
    /// so this is one decision, and the sweep over both receivers is what says
    /// so.
    // covers: Core\Db\Transaction::transaction
    #[test]
    fn a_nested_transaction_hands_over_a_transaction_and_answers_its_closures_own_variable() {
        let mut swept: Vec<&str> = Vec::new();
        for table in [&TRANSACTION, &CONNECTION] {
            let row = table
                .instance
                .iter()
                .find(|member| member.name == "transaction")
                .unwrap_or_else(|| panic!("`{}` declares § 7's `transaction`", table.name));
            let (given, closure) = handed_over(&row.params[0]).unwrap_or_else(|| {
                panic!(
                    "`{}::transaction` takes the closure it runs first, at one written instance",
                    table.name
                )
            });
            let returned = answered(&row.return_ty).unwrap_or_else(|| {
                panic!(
                    "`{}::transaction` answers a type variable and not a concrete type",
                    table.name
                )
            });
            assert_eq!(
                given, TRANSACTION_NAME,
                "`{}::transaction` hands its closure `{given}`, which is a receiver § 7 keeps four \
                 members off",
                table.name
            );
            assert_eq!(
                closure, returned,
                "`{}::transaction` answers `{returned}` where its closure answers `{closure}`, so \
                 a nested call loses the type its call site wrote",
                table.name
            );
            swept.push(table.name);
        }
        assert_eq!(
            swept.len(),
            2,
            "both receivers carry the row this case is about: {swept:?}"
        );
    }

    /// `rollBack` is the one member of [`TRANSACTION`] that takes text and runs
    /// no statement with it, so it is where `rule:security/sink-predicate`'s predicate is
    /// decided rather than copied: a reason is prose for a human, reaches no
    /// parser, and a `tainted` one — "the cart holds ${item}, which is gone" —
    /// is exactly the string a program has to hand.
    ///
    /// **Asserted as the biconditional over every text parameter the class
    /// takes**, because either half alone is satisfied by the wrong table. A
    /// `reason` that drifted to `Sink` would push callers to launder text no
    /// sink ever reads, and a `sql` that drifted off `Sink` would take a tainted
    /// statement — and the two are one decision made per parameter, so what
    /// guards them is one rule and not two lists.
    // covers: Core\Db\Transaction::rollBack
    #[test]
    fn a_transaction_takes_text_as_a_sink_exactly_where_that_text_is_the_statement() {
        let mut seen: Vec<(&str, bool, bool)> = Vec::new();
        for member in TRANSACTION.instance {
            for (name, ty) in member.names.iter().zip(member.params) {
                let CoreTy::Text(qual) = ty else {
                    continue;
                };
                seen.push((member.name, *name == "sql", matches!(qual, Qual::Sink)));
            }
        }

        let wrong: Vec<&(&str, bool, bool)> = seen
            .iter()
            .filter(|(_, is_statement, is_sink)| is_statement != is_sink)
            .collect();
        assert!(
            wrong.is_empty(),
            "a text parameter is a sink exactly where it is the statement: {wrong:?}"
        );
        assert!(
            seen.contains(&("rollBack", false, false)),
            "`rollBack`'s reason is the text this case is about: {seen:?}"
        );
        assert!(
            seen.iter()
                .filter(|(_, is_statement, _)| *is_statement)
                .count()
                >= 4,
            "every statement-running member is on the sweep: {seen:?}"
        );
    }
}
