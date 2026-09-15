//! [ADR 0067 § 9](/docs/decisions/0067.md)'s type table: what a
//! column declares itself to be, and what one cell becomes as a value.
//!
//! Three drivers arrive with three column types and three date/time shapes and
//! § 9 is one table, so each gets an arm and they meet at [`Value`]. The
//! `civil_of` trio is where that lands for `Core\Time`: a driver crate cannot
//! allocate an instance of a class it does not declare, so it hands over
//! components and this crate builds the instance.

use super::*;

/// The row description as spec § 18's `array<Column>`: one [`COLUMN`] per
/// column, in the server's own order and never keyed by label — `select a, a`
/// describes two columns under one name, and a keyed array would answer one.
///
/// **One object per column, built whether or not the program asks.** What that
/// spends is bounded by the statement's `select` list rather than by its
/// result, so it is a few objects beside the one-array-per-row the decode above
/// already allocates; the alternative — keeping the labels and the OIDs in a
/// pair of arrays and building the objects in `columns()` — buys nothing back
/// on the path that never calls it and costs a second representation of the
/// same fact on the path that does.
pub(super) fn described_columns(columns: &[nvs_db::PgColumn]) -> NvsArray {
    let mut described = NvsArray::new();
    for column in columns {
        described.append(crate::instance::build(
            &COLUMN,
            [
                Value::str(NvsStr::new(column.name.as_bytes())),
                column_type_value(column.column_type()),
                // `rule:core-classes/db-column-types`'s own answer, stated at the one place that can
                // state it: [`COLUMN_NULLABLE_DOC`] is where a program's author
                // reads what the `true` means.
                Value::bool(true),
            ],
        ));
    }
    described
}

/// The same description for a MySQL result set: [`described_columns`]'s twin,
/// building the same [`COLUMN`] objects out of the other driver's metadata.
///
/// **A twin and not one function over both**, because the two descriptions have
/// no type in common. A [`nvs_db::PgColumn`] is `nvs-db`'s own row description,
/// carrying its label as a `String` and its § 9 row as a field; a MySQL column
/// definition is `mysql_common`'s `Column`, whose label is octets in the packet
/// and whose § 9 row is read off its type and its `UNSIGNED` flag together. So
/// the two loops share their shape and not one line of their bodies, and a
/// trait over the pair would be a third name for two fields.
///
/// **The type is asked of the result set rather than read off the definition**,
/// because which § 9 row a definition names is `nvs-db`'s reading and not this
/// module's — the same division [`nvs_db::mysql::scalar`] draws for a value.
/// The label is taken as octets for [`described_columns`]'s reason: a column
/// name is a key in the row array, and a lossy decode would rename a column
/// rather than refuse it.
pub(super) fn mysql_described_columns(rows: &nvs_db::MySqlRows<'_>) -> NvsArray {
    let mut described = NvsArray::new();
    for (index, column) in rows.columns().iter().enumerate() {
        let column_type = rows
            .column_type(index)
            .expect("a column this loop is walking is one the result set described");
        described.append(crate::instance::build(
            &COLUMN,
            [
                Value::str(NvsStr::new(column.name_ref())),
                column_type_value(column_type),
                // As [`described_columns`]: § 9's own answer, and
                // [`COLUMN_NULLABLE_DOC`] is where it is written down.
                Value::bool(true),
            ],
        ));
    }
    described
}

/// The same description for a SQL Server result set: [`described_columns`]'
/// third twin, over `COLMETADATA`.
///
/// A twin for [`mysql_described_columns`]' reason, and one decision shorter
/// than it: a `nvs_db::tds::TdsColumn` carries its label as a `String` the
/// driver already decoded out of UCS-2, so there is no packet-octets-versus-
/// text question to answer here at all.
pub(super) fn tds_described_columns(rows: &nvs_db::tds::TdsRows<'_>) -> NvsArray {
    let mut described = NvsArray::new();
    for (index, column) in rows.columns().iter().enumerate() {
        let column_type = rows
            .column_type(index)
            .expect("a column this loop is walking is one the result set described");
        described.append(crate::instance::build(
            &COLUMN,
            [
                Value::str(NvsStr::new(column.name.as_bytes())),
                column_type_value(column_type),
                // As [`described_columns`]: § 9's own answer, and
                // [`COLUMN_NULLABLE_DOC`] is where it is written down. The
                // server's own `Flags` bit is beside it on this driver —
                // `nvs_db::tds::TdsColumn::nullable` — and says something
                // narrower, which that method's doc owns.
                Value::bool(true),
            ],
        ));
    }
    described
}

/// A [`nvs_db::ColumnType`] as the [`COLUMN_TYPE`] case a program matches on,
/// which at runtime is that case's ordinal
/// (`rule:enums/closed-integer-type`).
///
/// **The ordinal is looked up rather than written a second time.** The two
/// halves of the enum are one enum and [`COLUMN_TYPE`]'s doc says which half is
/// authoritative; a `match` answering numbers here would be a third place the
/// fourteen cases are written down, and the one that goes wrong silently. What
/// is written here is the *name* correspondence, which is the only thing this
/// crate knows that neither half does — and `every_column_type_case_is_named`
/// holds it total in both directions.
pub(super) fn column_type_value(of: nvs_db::ColumnType) -> Value {
    let case = column_type_case(of);
    let (_, ordinal) = COLUMN_TYPE
        .cases
        .iter()
        .find(|(name, _)| *name == case)
        .expect("every `nvs_db::ColumnType` names a case `COLUMN_TYPE` registers");
    Value::int(*ordinal)
}

/// The [`COLUMN_TYPE`] case one [`nvs_db::ColumnType`] is, by name.
///
/// Exhaustive on purpose — a variant added over there arrives here as a
/// non-exhaustive `match` rather than as a column that describes wrongly.
pub(super) fn column_type_case(of: nvs_db::ColumnType) -> &'static str {
    match of {
        nvs_db::ColumnType::Int => "Int",
        nvs_db::ColumnType::Uint => "Uint",
        nvs_db::ColumnType::Float => "Float",
        nvs_db::ColumnType::Decimal => "Decimal",
        nvs_db::ColumnType::Text => "Text",
        nvs_db::ColumnType::Bytes => "Bytes",
        nvs_db::ColumnType::Bool => "Bool",
        nvs_db::ColumnType::Date => "Date",
        nvs_db::ColumnType::Time => "Time",
        nvs_db::ColumnType::DateTime => "DateTime",
        nvs_db::ColumnType::Instant => "Instant",
        nvs_db::ColumnType::Uuid => "Uuid",
        nvs_db::ColumnType::Json => "Json",
        nvs_db::ColumnType::Other => "Other",
    }
}

/// One column's Novis value: `rule:core-classes/db-column-types`'s whole type map, with the five rows
/// whose Novis type is a class instance built here.
///
/// This is the second half of one decode and not a second decode. `nvs-db`
/// reads every column to a [`nvs_db::PgScalar`] and mints a [`Value`] for the
/// rows that are values; the five that are class instances arrive as the
/// components the server rendered, because a `Core\Time\Date` is an instance
/// of a class *this* crate declares and that one cannot allocate — see
/// [`nvs_db::PgDate`]. So nothing here parses a body, and the only thing left
/// that can go wrong is a rendered value no `Core\Time` type has.
///
/// `zone` is the connection's declared zone, § 9's answer for the one row that
/// carries no offset of its own.
///
/// An array is this function again per element, so an `array<Core\Uuid>` and
/// an `array<array<Core\Time\Date>>` need nothing of their own. Each is built
/// into an [`NvsArray`] that a later element's refusal drops — releasing what
/// it already holds — which is the rule the row itself is built under.
///
/// # Errors
///
/// [`unrepresentable_column`] for a value with no Novis representation, and a
/// [`Fault::fatal`] for a row `nvs-db` answers no value for and this function
/// does not build, which is a variant added there with no arm here.
pub(super) fn column_value(
    scalar: nvs_db::PgScalar<'_>,
    zone: i32,
    named: &str,
    column: &str,
) -> Result<Value, Fault> {
    let refused = |row| unrepresentable_column(named, column, row);
    Ok(match scalar {
        nvs_db::PgScalar::Date(date) => {
            crate::time::date_at(date.year, date.month, date.day).ok_or_else(|| refused("date"))?
        }
        nvs_db::PgScalar::Time(time) => {
            crate::time::time_of_day_at(time.hour, time.minute, time.second, time.nanosecond)
                .ok_or_else(|| refused("time of day"))?
        }
        nvs_db::PgScalar::Timestamp { date, time } => {
            crate::time::datetime_at(&civil_of(date, time), zone)
                .ok_or_else(|| refused("date and time"))?
        }
        nvs_db::PgScalar::Instant { date, time, offset } => {
            crate::time::instant_at(&civil_of(date, time), offset)
                .ok_or_else(|| refused("date and time"))?
        }
        nvs_db::PgScalar::Uuid(octets) => crate::uuid::of_octets(octets),
        nvs_db::PgScalar::Array(items) => {
            let mut array = NvsArray::new();
            for item in items {
                array.append(column_value(item, zone, named, column)?);
            }
            Value::array(array)
        }
        row => row.into_value().ok_or_else(|| {
            Fault::fatal(format!(
                "{named}: `nvs-db` answered no value for the column `{column}`, and this decoder \
                 builds no instance for it either"
            ))
        })?,
    })
}

/// The civil fields a `TIMESTAMP` or a `TIMESTAMPTZ` was rendered with, in the
/// shape [`crate::time`]'s two seams read.
pub(super) fn civil_of(date: nvs_db::PgDate, time: nvs_db::PgTime) -> crate::time::Civil {
    crate::time::Civil {
        year: date.year,
        month: date.month,
        day: date.day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        nanosecond: time.nanosecond,
    }
}

/// One MySQL column's Novis value: [`column_value`]'s twin over the other
/// driver's scalar.
///
/// The division is the same one, for the same reason: `nvs-db` reads a column
/// to a [`nvs_db::MySqlScalar`] and mints a [`Value`] for the rows that are
/// values, and § 9's structured rows arrive as the components the server sent,
/// because the classes they become are declared in *this* crate and that one
/// cannot allocate an instance — [`nvs_db::MySqlDate`] owns that half.
///
/// **Three rows and not five**, which is why this is a shorter `match` rather
/// than a copy of [`column_value`]'s. MySQL has no `UUID` column type — § 9
/// sends its `BINARY(16)` to the `bytes` row and MariaDB, which does have the
/// type, is its own driver — and no array type either, so the recursion a
/// PostgreSQL `array<T>` needs has nothing here to recur over. What is left is
/// `DATE`, `TIME` and the zone-less `DATETIME`/`TIMESTAMP` pair.
///
/// `zone` is the connection's declared zone: § 9's answer for the row that
/// carries no offset of its own, and the offset the connection told the server
/// at connect so that `CURRENT_TIMESTAMP` agrees with what is read back here.
///
/// # Errors
///
/// [`unrepresentable_column`] for a value no `Core\Time` type has — on this
/// driver the zero date, which its own decoder deliberately does not check —
/// and a [`Fault::fatal`] for a row `nvs-db` answers no value for and this
/// function does not build, which is a variant added there with no arm here.
pub(super) fn mysql_column_value(
    scalar: nvs_db::MySqlScalar<'_>,
    zone: i32,
    named: &str,
    column: &str,
) -> Result<Value, Fault> {
    let refused = |row| unrepresentable_column(named, column, row);
    Ok(match scalar {
        nvs_db::MySqlScalar::Date(date) => {
            crate::time::date_at(date.year, date.month, date.day).ok_or_else(|| refused("date"))?
        }
        nvs_db::MySqlScalar::Time(time) => {
            crate::time::time_of_day_at(time.hour, time.minute, time.second, time.nanosecond)
                .ok_or_else(|| refused("time of day"))?
        }
        nvs_db::MySqlScalar::DateTime { date, time } => {
            crate::time::datetime_at(&mysql_civil_of(date, time), zone)
                .ok_or_else(|| refused("date and time"))?
        }
        row => row.into_value().ok_or_else(|| {
            Fault::fatal(format!(
                "{named}: `nvs-db` answered no value for the column `{column}`, and this decoder \
                 builds no instance for it either"
            ))
        })?,
    })
}

/// The civil fields a `DATETIME` or a `TIMESTAMP` was sent with, in the shape
/// [`crate::time`]'s seams read — [`civil_of`] for the other driver.
///
/// Two functions over two field-identical structs, because the structs belong
/// to two protocols: PostgreSQL's components are parsed out of a rendering and
/// MySQL's arrive as integers, and one type standing for both would say they
/// are the same fact when only their shape is the same.
pub(super) fn mysql_civil_of(
    date: nvs_db::MySqlDate,
    time: nvs_db::MySqlTime,
) -> crate::time::Civil {
    crate::time::Civil {
        year: date.year,
        month: date.month,
        day: date.day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        nanosecond: time.nanosecond,
    }
}

/// One SQL Server column's Novis value: [`column_value`]'s twin over the third
/// driver's scalar.
///
/// The division is the same one for the same reason, and **all five of § 9's
/// structured rows are here** where MySQL reaches three: this driver has
/// `date`, `time`, the zone-less `datetime`/`datetime2` pair, `datetimeoffset`
/// and a `uniqueidentifier` type of its own. What it has no row for is the
/// array, which is PostgreSQL's alone, so nothing here recurs.
///
/// **Nothing is computed here, and three of these would be wrong if it were.**
/// `nvs_db::tds::scalar`'s doc names the rows whose octets do not mean what
/// they look like — `money`'s leading high word, `uniqueidentifier`'s
/// endianness, and `datetimeoffset`'s civil fields being stored in UTC — and
/// all three arrive already in their answered form. An `Instant` off this
/// driver therefore carries local fields beside its offset exactly as
/// PostgreSQL's does, which is what lets [`crate::time`]'s two seams serve both
/// without a driver argument.
///
/// `zone` is the connection's declared zone, § 9's answer for the row that
/// carries no offset of its own — declared and never negotiated on this driver,
/// unlike the other two, because there is no server-side session variable to
/// send it to.
///
/// # Errors
///
/// [`unrepresentable_column`] for a value no `Core\Time` type has, and a
/// [`Fault::fatal`] for a row `nvs-db` answers no value for and this function
/// does not build, which is a variant added there with no arm here.
pub(super) fn tds_column_value(
    scalar: nvs_db::tds::TdsScalar<'_>,
    zone: i32,
    named: &str,
    column: &str,
) -> Result<Value, Fault> {
    let refused = |row| unrepresentable_column(named, column, row);
    Ok(match scalar {
        nvs_db::tds::TdsScalar::Date(date) => {
            crate::time::date_at(date.year, date.month, date.day).ok_or_else(|| refused("date"))?
        }
        nvs_db::tds::TdsScalar::Time(time) => {
            crate::time::time_of_day_at(time.hour, time.minute, time.second, time.nanosecond)
                .ok_or_else(|| refused("time of day"))?
        }
        nvs_db::tds::TdsScalar::DateTime { date, time } => {
            crate::time::datetime_at(&tds_civil_of(date, time), zone)
                .ok_or_else(|| refused("date and time"))?
        }
        nvs_db::tds::TdsScalar::Instant { date, time, offset } => {
            crate::time::instant_at(&tds_civil_of(date, time), offset)
                .ok_or_else(|| refused("date and time"))?
        }
        nvs_db::tds::TdsScalar::Uuid(octets) => crate::uuid::of_octets(octets),
        row => row.into_value().ok_or_else(|| {
            Fault::fatal(format!(
                "{named}: `nvs-db` answered no value for the column `{column}`, and this decoder \
                 builds no instance for it either"
            ))
        })?,
    })
}

/// The civil fields a `datetime`, `datetime2` or `datetimeoffset` carried, in
/// the shape [`crate::time`]'s seams read.
///
/// [`civil_of`]'s and [`mysql_civil_of`]'s third, separate from both for the
/// reason those two are separate from each other: the fields are the same three
/// plus four and the *facts* are three protocols', one parsed out of a text
/// rendering and two read off the wire in different units.
pub(super) fn tds_civil_of(
    date: nvs_db::tds::TdsDate,
    time: nvs_db::tds::TdsTime,
) -> crate::time::Civil {
    crate::time::Civil {
        year: date.year,
        month: date.month,
        day: date.day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        nanosecond: time.nanosecond,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two halves of one enum name the same fourteen cases —
    /// [`COLUMN_TYPE`]'s doc is where "the wire one is authoritative" is
    /// written, and this is what keeps the registry half from drifting off it.
    ///
    /// Both directions, because they fail differently: a case
    /// [`column_type_case`] never names is a value a program can match on and
    /// never receive, while a name it produces that [`COLUMN_TYPE`] does not
    /// register is [`column_type_value`]'s `expect` firing on a real query —
    /// the one of the two that reaches a request.
    #[test]
    fn every_column_type_case_is_named() {
        let described: Vec<&'static str> = [
            nvs_db::ColumnType::Int,
            nvs_db::ColumnType::Uint,
            nvs_db::ColumnType::Float,
            nvs_db::ColumnType::Decimal,
            nvs_db::ColumnType::Text,
            nvs_db::ColumnType::Bytes,
            nvs_db::ColumnType::Bool,
            nvs_db::ColumnType::Date,
            nvs_db::ColumnType::Time,
            nvs_db::ColumnType::DateTime,
            nvs_db::ColumnType::Instant,
            nvs_db::ColumnType::Uuid,
            nvs_db::ColumnType::Json,
            nvs_db::ColumnType::Other,
        ]
        .into_iter()
        .map(column_type_case)
        .collect();
        let registered: Vec<&'static str> =
            COLUMN_TYPE.cases.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            described, registered,
            "`nvs_db::ColumnType` and `{COLUMN_TYPE_NAME}` are one enum, in the spec's own \
             order — a case added to either belongs in both, and in the same place"
        );
    }

    /// `rule:types/decimal`'s bound, at the door a database can push a wider
    /// number through: a `NUMERIC(30,10)` column holds numbers a `decimal`
    /// does not, and reading one **throws naming the column**.
    ///
    /// **Asserted here rather than only against a server**, because what a
    /// truncating driver would produce is a plausible number: a case that
    /// needs a container is a case that does not run on the machine the
    /// change is written on, and this refusal is one wrong `if` away at any
    /// time. `a_numeric_30_10_postgres_column_throws_on_read_rather_than_truncating`
    /// in `tests/db_stream.rs` is the same assertion against a real server,
    /// where the rendering is the server's own rather than this test's.
    ///
    /// **The two refusals are told apart**, which is the whole point of
    /// `nvs_runtime::Decimal::read`: the operator reading the first one fixes
    /// a column type, and the one reading the second is looking at a driver
    /// that read a body it was not sent — or at PostgreSQL's `NaN`, which this
    /// type has no value for. One message for both sends the first operator
    /// hunting corruption that is not there.
    ///
    /// The in-range value is asserted digit for digit beside them, because a
    /// refusal that fired on everything would pass both halves above.
    #[test]
    fn a_numeric_30_10_value_past_decimals_range_throws_rather_than_truncating() {
        // `nvs-db`'s OID table is private to that crate; `NUMERIC` is 1700 and
        // is one of the fixed numbers PostgreSQL's own catalog assigns.
        let total = nvs_db::PgColumn {
            name: String::from("total"),
            type_oid: 1700,
            type_modifier: -1,
        };
        let read = |body: &'static str| total.scalar(Some(body.as_bytes()));

        let kept =
            read("1234567890.1234567890").expect("a `NUMERIC(30,10)` value a `decimal` holds");
        match kept {
            nvs_db::PgScalar::Decimal(number) => assert_eq!(
                number.to_string(),
                "1234567890.1234567890",
                "a value inside the bound keeps every digit the server wrote, scale included"
            ),
            other => panic!("§ 9 reads a `NUMERIC` as a `decimal`, and read {other:?}"),
        }

        let past = read("12345678901234567890.1234567890")
            .expect_err("a value past 96 mantissa bits is refused rather than narrowed")
            .to_string();
        assert!(
            past.contains("total") && past.contains("narrows"),
            "the refusal names the column and what fixes it, and said: {past}"
        );
        assert!(
            !past.contains("1234567890"),
            "no refusal carries the value — `PgColumn::malformed`'s reason — and said: {past}"
        );

        let unreadable = read("NaN")
            .expect_err("a `NUMERIC`'s `NaN` has no `decimal` value")
            .to_string();
        assert!(
            unreadable.contains("total") && !unreadable.contains("narrows"),
            "a rendering that is not a number at all is the other refusal, and said: {unreadable}"
        );
    }
}
