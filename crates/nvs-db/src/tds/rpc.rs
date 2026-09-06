//! The requests this driver sends: `sp_prepexec`, `sp_execute`, `sp_unprepare`
//! and a bare batch, with the parameter encoding they carry.
//!
//! Every parameter goes out as one `nvarchar` and the server casts it, which is
//! what makes a declaration a function of the statement rather than of the
//! values — [`mod@super`]'s doc owns the one type that does not fit and why
//! closing that gap is a cache-key question. [`all_headers`] is the transaction
//! descriptor every request after a `BEGIN` must carry.

use super::*;

/// `sp_prepexec`'s procedure id — MS-TDS § 2.2.6.6's `Sp_PrepExec`.
///
/// Called by number rather than by name. The id is the shorter form of
/// `NameLenProcID` and it saves the server the lookup as well as the bytes,
/// which is the whole difference between the two spellings.
pub(super) const PROC_SP_PREPEXEC: u16 = 13;

/// `sp_execute`'s procedure id — MS-TDS § 2.2.6.6's `Sp_Execute`.
///
/// [ADR 0067 § 1](/docs/decisions/0067.md)'s cached re-execution: the
/// handle [`PROC_SP_PREPEXEC`] answered with and the values, and no SQL on the
/// wire at all.
pub(super) const PROC_SP_EXECUTE: u16 = 12;

/// `sp_unprepare`'s procedure id — MS-TDS § 2.2.6.6's `Sp_Unprepare`.
///
/// What an eviction sends, so the server never holds more plans than the
/// block's `statement_cache` allows.
pub(super) const PROC_SP_UNPREPARE: u16 = 15;

/// The batch [`Status::RESET_CONNECTION`] rides.
///
/// The bit resets the session *before* the message carrying it is processed, so
/// there has to be a message — and this is the smallest well-formed one whose
/// answer proves the reset landed. It deliberately sets nothing: anything this
/// batch did would be session state the reset had just finished removing.
pub(super) const RESET_STATEMENT: &str = "select 1";

/// [ADR 0067 § 7](/docs/decisions/0067.md)'s outermost transaction, in
/// T-SQL's spelling.
///
/// `BEGIN TRANSACTION` and not `START TRANSACTION`, which SQL Server does not
/// accept, and carrying no options at all: the level is a session setting sent
/// separately ([`isolation_command`]) and there is no read-only transaction
/// here for an option to ask for ([`begin`]).
pub(super) const BEGIN_TRANSACTION: &str = "BEGIN TRANSACTION";

/// § 7's outermost commit.
pub(super) const COMMIT_TRANSACTION: &str = "COMMIT TRANSACTION";

/// § 7's outermost rollback.
pub(super) const ROLLBACK_TRANSACTION: &str = "ROLLBACK TRANSACTION";

/// The level a session sits at when nothing has moved it, and what [`begin`],
/// [`commit`] and [`roll_back`] put back.
///
/// SQL Server's own default for a login, so a connection this driver never
/// asked for a level on is already here and is never sent it — the restore is
/// owed only by a session an isolation level moved.
pub(super) const DEFAULT_ISOLATION: &str = "SET TRANSACTION ISOLATION LEVEL READ COMMITTED";

/// `NameLenProcID`'s first field when what follows is a procedure *id* rather
/// than a name.
pub(super) const PROC_ID_SWITCH: u16 = 0xFFFF;

/// `ALL_HEADERS`'s `TotalLength`, which counts itself: its own four bytes plus
/// the eighteen of the one header every request here carries.
pub(super) const ALL_HEADERS_BYTES: u32 = 22;

/// The transaction-descriptor header's own length, likewise counting itself.
pub(super) const TRANSACTION_HEADER_BYTES: u32 = 18;

/// `HeaderType` for that header, which is the only one of the three a request
/// may carry that TDS 7.2 and later require.
pub(super) const HEADER_TRANSACTION: u16 = 0x0002;

/// The descriptor of *no* transaction: what every request carries until a
/// `BEGIN TRANSACTION` is answered, and what a commit or a rollback puts back.
pub(super) const NO_TRANSACTION: u64 = 0;

/// `StatusFlags` for a parameter the server may write back —
/// `sp_prepexec`'s `@handle`, and nothing else this driver sends.
pub(super) const PARAM_BY_REF: u8 = 0x01;

/// `INTN`'s declared width for the one integer parameter this driver sends.
pub(super) const INTN_BYTES: u8 = 4;

/// The widest `nvarchar` that is not a `MAX` one, in characters. A value past
/// it is declared and sent as `nvarchar(max)`, which arrives in `PLP` chunks.
pub(super) const NVARCHAR_CHARS: u16 = 4000;

/// A parameter's `COLLATION`, all zeroes, which is TDS's spelling of *the
/// server's own default*.
///
/// The field decides how a value **compares**, never how it is carried:
/// `nvarchar` is UCS-2 whatever the collation says. Sending anything else would
/// be this driver deciding a sort order for someone else's database.
pub(super) const NO_COLLATION: [u8; 5] = [0; 5];

/// [ADR 0067 § 1](/docs/decisions/0067.md)'s prepare and execute in
/// one message: `sp_prepexec`, carrying § 5's rewritten SQL and its parameters
/// as the procedure's own arguments.
///
/// **`sp_prepexec` rather than `sp_executesql` because § 1 keeps a statement
/// cache.** Both send the SQL and the parameters in one round trip; only this
/// one hands back a handle, and the handle is what a later `sp_execute` costs
/// nothing to run. The first execution of a statement is therefore one round
/// trip here — where § 1 records two for MySQL — and a cached one is one as
/// well, with no SQL on the wire at all.
///
/// **Every parameter goes out as `nvarchar` and the server casts it**, which is
/// [`crate::mysql::execute`]'s `MYSQL_TYPE_VAR_STRING` account reached through a
/// different protocol: the values arrive here already encoded as text by the
/// layer that knows what they are, and no byte of one is ever parsed as SQL —
/// § 1's no-emulated-prepares rule holds as a property of this function, since
/// `sql` is a separate argument to the procedure and never a string a value is
/// spliced into. The `@params` declaration beside it is what makes the cast the
/// *server's* decision rather than a guess: it names each marker's type, and
/// [`Dialect::marker`] is asked for the names so that the declaration and the
/// rewritten SQL cannot drift apart.
///
/// **A parameter that is not UTF-8 is refused rather than reinterpreted.** § 9's
/// `bytes` maps to `varbinary`, and `nvarchar` → `varbinary` on SQL Server is a
/// reinterpretation of UCS-2 code units rather than a parse — so a binary
/// parameter needs an encoding of its own, which is the encoder's slice and not
/// this one's. Refusing is what keeps that gap visible instead of silently
/// storing the wrong bytes.
///
/// # Errors
///
/// `InvalidInput` for a parameter that is not UTF-8, and for one whose UCS-2
/// form is past [`MAX_MESSAGE`].
pub fn sp_prepexec_request(
    sql: &str,
    params: &[Option<&[u8]>],
    descriptor: u64,
) -> io::Result<Vec<u8>> {
    let bound = bind(params)?;
    prepexec_request(sql, &bound, declarations(&bound).as_deref(), descriptor)
}

/// Every bound value as the UCS-2 [`text_param`] writes, or the refusal that
/// names the marker one of them was bound at.
///
/// Done once per statement and before anything is written, so a refusal costs
/// no bytes on the wire and neither of § 1's two request shapes has to repeat
/// the conversion to decide which of them is being sent.
///
/// # Errors
///
/// `InvalidInput` for a value that is not UTF-8 — [`text_of`]'s refusal.
pub(super) fn bind(params: &[Option<&[u8]>]) -> io::Result<Vec<Option<Vec<u8>>>> {
    let mut bound = Vec::with_capacity(params.len());
    for (index, value) in params.iter().enumerate() {
        bound.push(match value {
            None => None,
            Some(bytes) => Some(ucs2_of(text_of(bytes, index + 1)?)),
        });
    }
    Ok(bound)
}

/// A request's `ALL_HEADERS` and `NameLenProcID`, up to the first argument.
///
/// Every RPC body goes through here, which is why `descriptor` is taken here
/// rather than defaulted: a request shape added later cannot forget the header
/// § 7 needs, because it cannot be built without naming the transaction.
pub(super) fn rpc_header(proc_id: u16, descriptor: u64) -> Vec<u8> {
    let mut out = Vec::new();
    all_headers(&mut out, descriptor);
    out.extend_from_slice(&PROC_ID_SWITCH.to_le_bytes());
    out.extend_from_slice(&proc_id.to_le_bytes());
    // `OptionFlags`: neither `fWithRecomp` nor the two metadata ones. A
    // recompile on every execution is the opposite of what § 1's cache is for.
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// [`sp_prepexec_request`] over values already bound and a declaration already
/// derived, which is what [`start_statement`] is holding by the time it knows
/// this is the shape to send.
pub(super) fn prepexec_request(
    sql: &str,
    bound: &[Option<Vec<u8>>],
    declared: Option<&str>,
    descriptor: u64,
) -> io::Result<Vec<u8>> {
    let mut out = rpc_header(PROC_SP_PREPEXEC, descriptor);
    // `@handle`, sent null and by reference: the server allocates the plan and
    // writes the number back in the `RETURNVALUE` [`TdsRows::returned`] keeps.
    int_param(&mut out, true, None);
    let declaration = declared.map(ucs2_of);
    text_param(&mut out, declaration.as_deref(), "@params")?;
    text_param(&mut out, Some(&ucs2_of(sql)), "@stmt")?;
    values(&mut out, bound)?;
    Ok(out)
}

/// § 1's cached re-execution: `sp_execute`, naming the plan the server already
/// holds and carrying the same values in the same order.
///
/// **No SQL and no `@params`.** The plan already knows both, which is the whole
/// of what a hit buys on this protocol — a four-byte handle where the first
/// execution carried the statement. The declaration the plan was compiled
/// against is therefore not this call's to change, which is why
/// [`TdsPlan::declared`] is compared before one is sent.
///
/// # Errors
///
/// As [`text_param`], for the same values.
pub(super) fn execute_request(
    handle: i32,
    bound: &[Option<Vec<u8>>],
    descriptor: u64,
) -> io::Result<Vec<u8>> {
    let mut out = rpc_header(PROC_SP_EXECUTE, descriptor);
    // By value rather than by reference: this one is read and never written
    // back, so there is nothing for the server to return.
    int_param(&mut out, false, Some(handle));
    values(&mut out, bound)?;
    Ok(out)
}

/// § 1's eviction: `sp_unprepare`, dropping one plan the server is holding.
///
/// Sent as a message of its own and read to its `DONEPROC`, unlike MySQL's
/// `COM_STMT_CLOSE`, which answers nothing. Every TDS request has an answer, so
/// one left unread would be taken as the *next* statement's — this is the
/// protocol's difference and not a choice, and it is why an eviction costs a
/// round trip here and none there.
pub(super) fn unprepare_request(handle: i32, descriptor: u64) -> Vec<u8> {
    let mut out = rpc_header(PROC_SP_UNPREPARE, descriptor);
    int_param(&mut out, false, Some(handle));
    out
}

/// The `SQL_BATCH` message one text goes out as, carrying no headers of its own
/// beyond the transaction descriptor every request needs.
///
/// Two callers and both of them this driver's own text: § 13's
/// [`RESET_STATEMENT`] and § 7's commands. **No caller's SQL ever reaches this
/// function** — a program's statement is an `sp_prepexec` with its values as
/// parameters ([`start_statement`]), which is § 1's no-emulated-prepares rule,
/// and the only thing composed into a batch here is a savepoint name this
/// module minted.
pub(super) fn batch_request(sql: &str, descriptor: u64) -> Vec<u8> {
    let mut out = Vec::new();
    all_headers(&mut out, descriptor);
    out.extend_from_slice(&ucs2_of(sql));
    out
}

/// One argument per marker, in § 5's order, for either of the two procedures
/// that take them.
///
/// # Errors
///
/// As [`text_param`].
pub(super) fn values(out: &mut Vec<u8>, bound: &[Option<Vec<u8>>]) -> io::Result<()> {
    for (index, value) in bound.iter().enumerate() {
        text_param(out, value.as_deref(), &Dialect::SqlServer.marker(index + 1))?;
    }
    Ok(())
}

/// One bound value as text, or the refusal that names the marker it was bound
/// at.
pub(super) fn text_of(value: &[u8], marker: usize) -> io::Result<&str> {
    std::str::from_utf8(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the value bound at {} is not UTF-8, and this driver sends every parameter as \
                 `nvarchar` — `rule:core-classes/db-column-types`'s `bytes` needs an encoding of its own here rather \
                 than a reinterpretation of these bytes as UCS-2",
                Dialect::SqlServer.marker(marker)
            ),
        )
    })
}

/// Text as UCS-2LE, which is the one form TDS carries a character value in.
pub(super) fn ucs2_of(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// Whether a value needs the `MAX` form — the one decision `@params` and the
/// parameter itself have to agree on, so it is asked once and here.
pub(super) fn is_wide(ucs2: &[u8]) -> bool {
    ucs2.len() > usize::from(NVARCHAR_CHARS) * 2
}

/// A request's `ALL_HEADERS`: the transaction descriptor TDS 7.2 and later
/// require, and nothing else.
///
/// `descriptor` is [`Wire::descriptor`] — the transaction this request enlists
/// in, and zero is *none*. It is a parameter rather than a constant because the
/// server refuses a request that names the wrong one: once a `BEGIN
/// TRANSACTION` has been answered, every later request on the session carries
/// the eight octets that answer came with, and one still writing zero is
/// refused with code 3989 before it runs.
pub(super) fn all_headers(out: &mut Vec<u8>, descriptor: u64) {
    out.extend_from_slice(&ALL_HEADERS_BYTES.to_le_bytes());
    out.extend_from_slice(&TRANSACTION_HEADER_BYTES.to_le_bytes());
    out.extend_from_slice(&HEADER_TRANSACTION.to_le_bytes());
    out.extend_from_slice(&descriptor.to_le_bytes());
    // `OutstandingRequestCount`: one, which is all § 4 ever allows in flight.
    out.extend_from_slice(&1u32.to_le_bytes());
}

/// `sp_prepexec`'s `@params`: § 5's markers with the type each is sent as, or
/// `None` for a statement that binds nothing — which the procedure reads as a
/// plan with no parameters, and an empty string would not.
pub(super) fn declarations(bound: &[Option<Vec<u8>>]) -> Option<String> {
    if bound.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (index, value) in bound.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&Dialect::SqlServer.marker(index + 1));
        // A null is declared narrow: it carries no characters, so nothing about
        // it wants the `MAX` form, and the plan a later execution of the same
        // statement reuses is the one compiled against the narrow declaration.
        match value {
            Some(ucs2) if is_wide(ucs2) => out.push_str(" nvarchar(max)"),
            _ => out.push_str(&format!(" nvarchar({NVARCHAR_CHARS})")),
        }
    }
    Some(out)
}

/// A parameter's `ParamMetaData` up to its `TYPE_INFO`.
///
/// The name is always empty. A procedure's arguments are read positionally when
/// they are unnamed, and naming them would put SQL Server's own spelling of
/// `sp_prepexec`'s parameters in this driver — a thing a release is free to
/// change and this driver would have no way to notice.
pub(super) fn param_header(out: &mut Vec<u8>, by_ref: bool) {
    out.push(0);
    out.push(if by_ref { PARAM_BY_REF } else { 0 });
}

/// An `INTN` argument: four bytes, or `NULL` at a length of zero.
pub(super) fn int_param(out: &mut Vec<u8>, by_ref: bool, value: Option<i32>) {
    param_header(out, by_ref);
    out.push(TY_INTN);
    out.push(INTN_BYTES);
    match value {
        None => out.push(0),
        Some(number) => {
            out.push(INTN_BYTES);
            out.extend_from_slice(&number.to_le_bytes());
        }
    }
}

/// An `NVARCHAR` argument, in whichever of its two forms the value fits:
/// `USHORTLEN` up to [`NVARCHAR_CHARS`], and `PLP` past it.
///
/// `what` is the marker or procedure parameter the value was bound at, for the
/// refusal alone — the request itself sends no names.
///
/// # Errors
///
/// `InvalidInput` for a value past [`MAX_MESSAGE`], which is the ceiling this
/// driver reads a message to and therefore the widest one it is willing to
/// write.
pub(super) fn text_param(out: &mut Vec<u8>, value: Option<&[u8]>, what: &str) -> io::Result<()> {
    param_header(out, false);
    out.push(TY_NVARCHAR);

    let wide = value.is_some_and(is_wide);
    if wide {
        out.extend_from_slice(&NO_LENGTH.to_le_bytes());
    } else {
        out.extend_from_slice(&(NVARCHAR_CHARS * 2).to_le_bytes());
    }
    out.extend_from_slice(&NO_COLLATION);

    let Some(ucs2) = value else {
        // The null of whichever form the type declared, which are two different
        // sentinels rather than one.
        if wide {
            out.extend_from_slice(&PLP_NULL.to_le_bytes());
        } else {
            out.extend_from_slice(&NO_LENGTH.to_le_bytes());
        }
        return Ok(());
    };
    if ucs2.len() > MAX_MESSAGE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the value bound at {what} is {} byte(s) of UCS-2, past the {MAX_MESSAGE} a TDS \
                 message holds",
                ucs2.len()
            ),
        ));
    }
    let length = u32::try_from(ucs2.len()).expect("checked against MAX_MESSAGE above");
    if wide {
        // One chunk and its terminator. The value is in hand, so the declared
        // total is the truth rather than `PLP_UNKNOWN`, and a server reading it
        // can size its buffer once.
        out.extend_from_slice(&u64::from(length).to_le_bytes());
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(ucs2);
        out.extend_from_slice(&0u32.to_le_bytes());
    } else {
        let short = u16::try_from(length).expect("narrower than NVARCHAR_CHARS characters");
        out.extend_from_slice(&short.to_le_bytes());
        out.extend_from_slice(ucs2);
    }
    Ok(())
}

/// One bound parameter as the octets [`text_param`] carries it in:
/// [`crate::encode`]'s and [`crate::mysql::encode`]'s opposite number on this
/// protocol, and a third text rendering rather than a copy of either.
///
/// **Every parameter goes out as one `nvarchar` and the server casts it to
/// whatever the statement compares it against** — [`start_statement`] says why
/// that is a fact about `sp_prepexec`'s `@params` and not an escaping decision —
/// so this renders text exactly as the other two do, and differs from them
/// wherever T-SQL reads a literal differently:
///
/// - A `bool` is `1`/`0`, MySQL's rendering rather than PostgreSQL's: `bit` is
///   a numeric type here and the cast of `'t'` is an error, not a `false`.
/// - A non-finite `float` is refused, for the reason MySQL's is. T-SQL has no
///   `Infinity` or `NaN` literal, and `float` holds neither value.
/// - A `bytes` is refused, and that is this driver's one gap against
///   [ADR 0067 § 9](/docs/decisions/0067.md)'s table rather than a
///   rendering choice: `varbinary` has no text input form a cast recovers —
///   `'0x61'` casts to the four characters and not to the octet — so a `bytes`
///   needs a parameter of its own type, which is a marker this driver does not
///   write yet. [`text_param`] refuses the same value one layer down for the
///   same reason, so the gap is closed in one place or in neither.
///
/// A `Core\Db\InList` never reaches here for [`crate::encode`]'s reason: § 5's
/// marker has expanded into one bound value per element by the time a statement
/// has its bind list.
///
/// # Errors
///
/// `InvalidInput` for a value with no form to send — a `bytes`, an array, an
/// object, a closure, and the three non-finite floats — where the whole answer
/// is the tag and never the value, for the reason [`malformed`] gives.
pub fn encode(value: Value) -> io::Result<Option<Vec<u8>>> {
    let rendered = match value.tag() {
        Some(Tag::Null) => return Ok(None),
        Some(Tag::Bool) => String::from(if value.as_bool() == Some(true) {
            "1"
        } else {
            "0"
        }),
        Some(Tag::Int) => value.as_int().unwrap_or_default().to_string(),
        // No unsigned type exists here, so a `uint` past `bigint` renders as
        // digits and the server reads it as a `numeric`. A column that cannot
        // hold it refuses it, which is the answer a silent wrap would hide.
        Some(Tag::Uint) => value.as_uint().unwrap_or_default().to_string(),
        Some(Tag::Float) => {
            let float = value.as_float().unwrap_or_default();
            if !float.is_finite() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a `float` that is not finite has no T-SQL literal and no `float` value — \
                     the three of them are what this driver cannot bind, and a program that \
                     stores one writes its own text column",
                ));
            }
            // Rust's shortest round-tripping form, which `float`'s own string
            // cast reads back to the same bits.
            float.to_string()
        }
        // Exact on both sides, as the other two drivers' is: `rule:types/decimal`'s
        // `decimal` renders as digits and a point, which is `decimal`'s own
        // input form, so nothing rounds here the way binding a `float` would.
        Some(Tag::Decimal) => value
            .as_decimal()
            .map(|exact| exact.to_string())
            .unwrap_or_default(),
        // A `string` is UTF-8 by `rule:types/bytes`; `text_param` widens it to UCS-2 on
        // the way out, so the octets go out as they are.
        Some(Tag::Str) => {
            return Ok(Some(value.as_str_bytes().unwrap_or_default().to_vec()));
        }
        Some(Tag::Bytes) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a `bytes` has no SQL Server parameter yet: every value this driver binds goes \
                 out as an `nvarchar`, and `varbinary` has no text form a cast recovers",
            ));
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "a value of tag {} has no form this driver can bind",
                    value.tag_byte()
                ),
            ));
        }
    };
    Ok(Some(rendered.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    #[test]
    fn an_rpc_names_sp_prepexec_and_carries_a_handle_a_declaration_the_sql_and_every_value() {
        let request = sp_prepexec_request(
            "select * from t where a = @p1 and b = @p2",
            &[Some(b"7"), None],
            NO_TRANSACTION,
        )
        .expect("two bound values");
        let (proc_id, params) = sent_rpc(&request);

        assert_eq!(proc_id, PROC_SP_PREPEXEC);
        assert_eq!(
            params.len(),
            5,
            "the handle, @params, @stmt, and two values"
        );
        assert_eq!(
            params[0],
            SentParam {
                by_ref: true,
                type_id: TY_INTN,
                declared: u16::from(INTN_BYTES),
                text: None,
            },
            "the handle goes out null and by reference, for the server to fill in"
        );
        assert!(params[1..].iter().all(|param| !param.by_ref));
        assert_eq!(
            params[1].text.as_deref(),
            Some("@p1 nvarchar(4000),@p2 nvarchar(4000)")
        );
        assert_eq!(
            params[2].text.as_deref(),
            Some("select * from t where a = @p1 and b = @p2"),
            "the SQL is an argument to the procedure and never a string a value \
             was spliced into"
        );
        assert_eq!(params[3].text.as_deref(), Some("7"));
        assert_eq!(params[4].text, None, "a bound null is the type's own null");
    }

    /// A statement that binds nothing declares nothing, and `@params` is null
    /// rather than empty: an empty declaration is a plan with a parameter list
    /// the server then finds no parameters for.
    #[test]
    fn a_statement_that_binds_nothing_sends_a_null_declaration_and_no_values() {
        let request =
            sp_prepexec_request("select 1", &[], NO_TRANSACTION).expect("no bound values");
        let (_, params) = sent_rpc(&request);

        assert_eq!(params.len(), 3);
        assert_eq!(params[1].text, None, "@params");
        assert_eq!(params[2].text.as_deref(), Some("select 1"));
    }

    /// Agreement, over § 5's own rewriter: the names `@params` declares are the
    /// names the rewritten SQL uses, because both ask [`Dialect::marker`].
    #[test]
    fn a_declarations_markers_are_the_ones_section_fives_rewriter_wrote() {
        let statement = crate::sql::rewrite(
            "insert into t values (?, ?, ?)",
            crate::sql::Params::Positional(&[
                crate::sql::Binding::One,
                crate::sql::Binding::One,
                crate::sql::Binding::One,
            ]),
            Dialect::SqlServer,
        )
        .expect("three positional markers");

        let request = sp_prepexec_request(
            &statement.sql,
            &[Some(b"a"), Some(b"b"), Some(b"c")],
            NO_TRANSACTION,
        )
        .expect("three bound values");
        let (_, params) = sent_rpc(&request);
        let declared = params[1].text.clone().expect("a declaration");

        assert_eq!(statement.arity(), 3);
        for marker in declared.split(',') {
            let name = marker.split(' ').next().expect("a name and a type");
            assert!(
                statement.sql.contains(name),
                "{name} is declared and never written: {}",
                statement.sql
            );
        }
    }

    /// Both sides of the bound `sp_prepexec` is told about: 4,000 characters is
    /// the widest `nvarchar` that is not a `MAX` one, and 4,001 is not a
    /// narrower one.
    #[test]
    fn a_value_past_four_thousand_characters_is_declared_and_sent_as_max() {
        for (chars, declared, spelling) in [
            (
                usize::from(NVARCHAR_CHARS),
                NVARCHAR_CHARS * 2,
                "nvarchar(4000)",
            ),
            (usize::from(NVARCHAR_CHARS) + 1, NO_LENGTH, "nvarchar(max)"),
        ] {
            let value = "x".repeat(chars);
            let request =
                sp_prepexec_request("select @p1", &[Some(value.as_bytes())], NO_TRANSACTION)
                    .expect("one value");
            let (_, params) = sent_rpc(&request);

            assert_eq!(params[1].text.as_deref(), Some(&*format!("@p1 {spelling}")));
            assert_eq!(params[3].declared, declared, "{chars} character(s)");
            assert_eq!(params[3].text.as_deref(), Some(&*value));
        }
    }

    /// A null takes the null of whichever form its declaration named, and the
    /// two are different sentinels rather than one.
    #[test]
    fn a_null_value_is_the_narrow_forms_sentinel_and_a_max_one_is_plps() {
        let request = sp_prepexec_request("select @p1", &[None], NO_TRANSACTION).expect("one null");
        let (_, params) = sent_rpc(&request);
        assert_eq!(params[3].declared, NVARCHAR_CHARS * 2);
        assert_eq!(params[3].text, None);

        // The `MAX` half of the same rule, asserted where a value forces it:
        // `@params` and `@stmt` take the same path as a bound value.
        let long = "y".repeat(usize::from(NVARCHAR_CHARS) + 1);
        let request = sp_prepexec_request(&long, &[], NO_TRANSACTION).expect("a long statement");
        let (_, params) = sent_rpc(&request);
        assert_eq!(params[2].declared, NO_LENGTH);
        assert_eq!(params[2].text.as_deref(), Some(&*long));
    }

    #[test]
    fn a_parameter_that_is_not_utf8_is_refused_by_the_marker_it_was_bound_at() {
        let refused = sp_prepexec_request(
            "select @p1, @p2",
            &[Some(b"fine"), Some(&[0xFF, 0xFE])],
            NO_TRANSACTION,
        )
        .expect_err("this driver has no binary parameter yet");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(refused.to_string().contains("@p2"), "{refused}");
    }

    #[test]
    fn a_procedures_answer_is_read_past_its_two_tokens_and_the_handle_is_kept() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        // Where `sp_prepexec` really puts them: the inner statement's own
        // `DONEINPROC`, then the handle and the status, then the `DONEPROC`
        // that ends the procedure. The `DONEINPROC` is deliberately written
        // without `DONE_MORE` — its *type byte* is what says something follows.
        payload.extend_from_slice(&done_kind(TOKEN_DONE_IN_PROC, DONE_COUNT, 1));
        payload.extend_from_slice(&return_value_token("@handle", Some(9)));
        payload.extend_from_slice(&return_status_token(0));
        payload.extend_from_slice(&done_kind(TOKEN_DONE_PROC, 0, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        assert_eq!(rows.returned(), None, "the handle arrives after the rows");

        let read = drain_rows(&mut rows).expect("one row, then the procedure's own end");
        assert_eq!(read.len(), 1);
        let returned = rows.returned().expect("sp_prepexec's handle");
        assert_eq!(returned.name, "@handle");
        assert_eq!(returned.ordinal, 1);
        assert_eq!(returned.as_i32(), Some(9));
        assert_eq!(state.get(), State::Idle);
    }

    /// The same two tokens on the other side of the `COLMETADATA`, which is
    /// [`TdsRows::shape`] rather than [`TdsRows::step`]: a procedure that
    /// assigned its output parameter before selecting anything sends them
    /// there, and a reader that only knew one of the two paths would refuse it.
    #[test]
    fn the_two_procedure_tokens_are_read_before_the_columns_as_well() {
        let mut payload = return_status_token(0);
        payload.extend_from_slice(&return_value_token("@handle", Some(4)));
        payload.extend_from_slice(&four_columns());
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        assert_eq!(
            rows.returned().and_then(ReturnValue::as_i32),
            Some(4),
            "the handle was there before the columns were"
        );
        assert_eq!(drain_rows(&mut rows).expect("one row").len(), 1);
    }

    /// A null output parameter is `None` and not a zero handle: a procedure
    /// that never assigned one has not allocated a plan number 0.
    #[test]
    fn an_unassigned_output_parameter_is_null_rather_than_a_handle() {
        let mut payload = return_value_token("@handle", None);
        payload.extend_from_slice(&done_token(0, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let rows = read_rows(&mut wire, &state, span()).expect("a statement with no result set");
        let returned = rows.returned().expect("the token was there");
        assert_eq!(returned.value, None);
        assert_eq!(returned.as_i32(), None);
    }

    #[test]
    fn a_statement_goes_out_as_one_rpc_message_and_its_rows_come_back() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_kind(TOKEN_DONE_IN_PROC, DONE_COUNT, 1));
        payload.extend_from_slice(&return_value_token("@handle", Some(3)));
        payload.extend_from_slice(&done_kind(TOKEN_DONE_PROC, 0, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);
        let mut rows = start_statement(
            &mut wire,
            &state,
            &mut cache,
            "select * from t where a = @p1",
            &[Some(b"7")],
        )
        .expect("a described result set");
        assert_eq!(drain_rows(&mut rows).expect("one row").len(), 1);
        assert_eq!(rows.returned().and_then(ReturnValue::as_i32), Some(3));

        let sent = wire.peer().sent.clone();
        assert_eq!(sent[0], PacketType::Rpc.byte());
        assert_eq!(sent[1], Status::EOM.bits(), "one packet, and it ends there");
        assert_eq!(
            usize::from(u16::from_be_bytes([sent[2], sent[3]])),
            sent.len()
        );
        let (proc_id, params) = sent_rpc(&sent[HEADER..]);
        assert_eq!(proc_id, PROC_SP_PREPEXEC);
        assert_eq!(params[3].text.as_deref(), Some("7"));
    }

    /// § 4's one statement at a time, held here rather than by every caller —
    /// and a request this driver would not build never reaches the wire, so the
    /// connection it was refused on is still usable.
    #[test]
    fn a_statement_on_a_busy_connection_is_refused_and_a_bad_parameter_leaves_it_idle() {
        let mut wire = Wire::new(Script::silent());
        let busy = Cell::new(State::Streaming);
        let mut cache = plans(2);
        let refused = start_statement(&mut wire, &busy, &mut cache, "select 1", &[])
            .expect_err("a second statement on one connection");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);

        let idle = Cell::new(State::Idle);
        let refused = start_statement(&mut wire, &idle, &mut cache, "select @p1", &[Some(&[0xFF])])
            .expect_err("a parameter that is not UTF-8");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(idle.get(), State::Idle, "nothing was written");
        assert!(wire.peer().sent.is_empty());
    }

    /// A bound parameter renders as *this* server reads it, and the rows where
    /// that differs from another driver are asserted against that driver's own
    /// answer rather than on their own.
    ///
    /// `t` is not a `bit` here and `Infinity` is not a `float`, so an encoder
    /// written by copying PostgreSQL's passes every assertion that only reads
    /// this one's output: the agreement is the bug, which is why those rows say
    /// `assert_ne!`. The `bytes` row is the same shape and is a *gap* rather
    /// than a rendering — [`encode`]'s doc owns why — so it is pinned as a
    /// refusal beside a driver that binds the same value, and the day this
    /// driver grows a `varbinary` marker this line is what fails.
    #[test]
    fn a_bound_parameter_renders_as_t_sql_reads_it_and_a_bytes_is_refused() {
        let sent = [
            (Value::bool(true), b"1".to_vec()),
            (Value::bool(false), b"0".to_vec()),
            (Value::int(-7), b"-7".to_vec()),
            (Value::uint(u64::MAX), u64::MAX.to_string().into_bytes()),
            (Value::float(1.5), b"1.5".to_vec()),
            (
                Value::str(nvs_runtime::NvsStr::new("é".as_bytes())),
                "é".as_bytes().to_vec(),
            ),
        ];
        for (value, octets) in sent {
            assert_eq!(
                encode(value).expect("a value § 9 binds"),
                Some(octets),
                "bound as SQL Server reads it"
            );
        }

        // A bitmap bit on the other drivers and a null marker here, and neither
        // is a rendering.
        assert_eq!(encode(Value::null()).expect("SQL NULL"), None);

        assert_ne!(
            encode(Value::bool(true)).expect("a `bit` this driver binds"),
            crate::encode(Value::bool(true)).expect("PostgreSQL binds it too"),
            "`t` is not a `bit`, so the two drivers must not agree here"
        );

        // The values PostgreSQL binds and this driver refuses: the three floats
        // no `float` column holds, and § 9's `bytes` row, which is this
        // driver's one gap against the table.
        let octets = Value::bytes(nvs_runtime::NvsStr::new(&[0x00, 0x61, 0xFF]));
        let refused = encode(octets).expect_err("no `nvarchar` carries these octets");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(crate::encode(octets).is_ok());
        for outside in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let refused = encode(Value::float(outside)).expect_err("no `float` holds it");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
            assert!(crate::encode(Value::float(outside)).is_ok());
        }
    }
}
