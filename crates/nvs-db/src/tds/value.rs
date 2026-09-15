//! One column's bytes, decoded against the type its `COLMETADATA` declared.
//!
//! [`decode_column`] mints the [`Value`] for the rows that are values;
//! [`scalar`] answers a [`TdsScalar`] for the three that are `Core\Time`
//! instances, which stay components for `nvs-stdlib` to build — this crate
//! cannot allocate an instance of a class it does not declare. [`TdsDate`] and
//! [`TdsTime`] are that boundary's shape.
//!
//! Nothing here guesses: a value that is not the width its column declared is
//! refused, because the alternative is reading the *next* column's bytes as
//! this one's tail.

use super::*;
use nvs_runtime::NotDecimal;

/// One row, already read out of the packets it was framed in.
///
/// **Eager, and one allocation for the values rather than one each.**
/// [`crate::PgRow`] is lazy because a `DataRow` is a run of length-prefixed
/// bodies inside one packet it already owns; a TDS row is neither — a value's
/// width comes from its column's declared type, so finding column five means
/// measuring columns zero to four, and the bytes may have arrived in three
/// packets that no longer exist. Once that walk is unavoidable, keeping the
/// result is free and re-walking per column is what would cost.
///
/// The values are the server's bytes, undecoded: [ADR 0067
/// § 9](/docs/decisions/0067.md)'s table is applied by `nvs-stdlib`
/// against [`TdsColumn::type_info`], which is the boundary
/// [`crate::PgColumn::decode`] sits on for the other driver.
pub struct TdsRow {
    /// Every present value's bytes, end to end.
    pub(super) bytes: Vec<u8>,
    /// Where each column's value is in [`TdsRow::bytes`], and `None` for a
    /// column that was `NULL`.
    pub(super) values: Vec<Option<Range<usize>>>,
}

impl std::fmt::Debug for TdsRow {
    /// How many columns, and none of their values: a row in flight is one
    /// request's data — the rule [`Wire`]'s own rendering follows.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdsRow")
            .field("columns", &self.values.len())
            .finish_non_exhaustive()
    }
}

impl TdsRow {
    /// How many columns this row has, which is the result set's column count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the row has no columns at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Column `index`'s bytes.
    ///
    /// **The two `None`s are two different facts**, which is why they are
    /// nested rather than flattened: the outer one is a column the result set
    /// does not have, and the inner one is SQL `NULL` — § 9's `?T`, and the
    /// only one of the two a program can see.
    #[must_use]
    pub fn column(&self, index: usize) -> Option<Option<&[u8]>> {
        self.values
            .get(index)
            .map(|range| range.clone().map(|range| &self.bytes[range]))
    }
}

/// The civil date a `date`, `datetime`, `datetime2` or `datetimeoffset` column
/// carried, as the three fields a `Core\Time\Date` is built from.
///
/// [`crate::MySqlDate`]'s twin, and separate from it for the reason that type
/// is separate from [`crate::PgDate`]: a driver's decoded column is that
/// driver's, and this crate cannot allocate a `Core\Time\Date` at all — only
/// `nvs-stdlib` can, which is why [`TdsScalar::into_value`] answers `None` for
/// every row that is one.
///
/// Every field is in range by construction: the wire carries a day count and
/// [`civil_from_days`] is the only thing that writes one of these.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TdsDate {
    /// The year, `1` to `9999` — the range SQL Server's `date` spans.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
}

impl std::fmt::Debug for TdsDate {
    /// The type and none of the fields, for the reason [`TdsRow`]'s own
    /// rendering gives: a decoded column is one request's data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TdsDate")
    }
}

/// The civil time a `time`, `datetime`, `datetime2` or `datetimeoffset` column
/// carried, as the four fields a `Core\Time\TimeOfDay` is built from.
///
/// [`crate::MySqlTime`]'s twin with one difference: MySQL's `TIME` doubles as
/// an interval and needs a bound asserted on both sides of a day, while SQL
/// Server's `time` is a clock reading and nothing else, so this is always one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TdsTime {
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// The second, 0 to 59. SQL Server has no leap second.
    pub second: u8,
    /// The nanosecond within the second. `datetime2(7)` stores 100ns ticks, so
    /// the last two digits are always zero; the field counts nanoseconds
    /// because that is what every `Core\Time` type holds.
    pub nanosecond: u32,
}

impl std::fmt::Debug for TdsTime {
    /// The type and none of the fields — [`TdsDate`]'s rule.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TdsTime")
    }
}

/// One column's value as the row of [ADR 0067
/// § 9](/docs/decisions/0067.md)'s table it landed on, before
/// anything a `Core` class needs is allocated.
///
/// [`crate::PgScalar`]'s and [`crate::MySqlScalar`]'s opposite number, and the
/// two differences from both are the protocol's. There is no `UInt` row —
/// `tinyint` is SQL Server's only unsigned integer and it fits an `int` with
/// room to spare, which [`TdsColumn::column_type`] already says — and there is
/// no `Array` row, because arrays are PostgreSQL's alone.
///
/// **Every value here is binary**, where PostgreSQL's arrive as the server's
/// text rendering: TDS describes a type structurally and sends its octets, so
/// this is where the widths, the epochs and the little-endian orders live. That
/// is also why the borrow is narrower than [`crate::PgScalar`]'s — a `bytes` is
/// the row's own octets rather than something unescaped into a buffer.
pub enum TdsScalar<'a> {
    /// SQL `NULL`: the row of § 9's table that makes every column `?T`.
    Null,
    /// `bit`.
    Bool(bool),
    /// `tinyint`/`smallint`/`int`/`bigint`.
    Int(i64),
    /// `real`/`float`.
    Float(f64),
    /// `decimal`/`numeric`, and `money`/`smallmoney` — which § 9's table puts
    /// on this row too, and which are a scaled integer on the wire rather than
    /// a `numeric` with a sign byte.
    Decimal(Decimal),
    /// A `tainted string`'s text, already proven well-formed UTF-8.
    ///
    /// Owned for the six `N` types and for `xml`, whose wire form is UCS-2 and
    /// so cannot be borrowed as UTF-8 at all; borrowed out of the row body for
    /// `char`, `varchar` and `text`, whose octets already are their text.
    Text(Cow<'a, str>),
    /// A `tainted bytes`'s octets — `binary`, `varbinary` and `image` — as the
    /// row body holds them.
    Bytes(&'a [u8]),
    /// `date`, which is a `Core\Time\Date`.
    Date(TdsDate),
    /// `time`, which is a `Core\Time\TimeOfDay`.
    Time(TdsTime),
    /// `datetime`, `smalldatetime` and `datetime2`: § 9's zone-less row, a
    /// `Core\Time\DateTime` in the zone [`TdsTarget::time_zone`] declared.
    DateTime {
        /// The civil date, as stored.
        date: TdsDate,
        /// The civil time, as stored.
        time: TdsTime,
    },
    /// `datetimeoffset`, which is a `Core\Time\Instant`: the civil fields *at
    /// the stored offset*, plus what turns them into a point in time.
    ///
    /// The wire carries the civil fields in UTC and the offset beside them, so
    /// [`scalar`] shifts them before they arrive here — the fields mean what
    /// [`crate::PgScalar::Instant`]'s mean, which is what lets one reader in
    /// `nvs-stdlib` serve both drivers.
    Instant {
        /// The civil date, at `offset`.
        date: TdsDate,
        /// The civil time, at `offset`.
        time: TdsTime,
        /// Seconds east of UTC — the sign every `Core\Time` type uses.
        offset: i32,
    },
    /// `uniqueidentifier`, as its sixteen octets in the order the text spells
    /// them, which is **not** the order the wire carries them in.
    Uuid([u8; 16]),
}

impl std::fmt::Debug for TdsScalar<'_> {
    /// Which row of § 9's table this landed on, and never the value: a column
    /// in flight is one request's data, which is the rule [`TdsRow`]'s own
    /// rendering holds.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TdsScalar::Null => "null",
            TdsScalar::Bool(_) => "bool",
            TdsScalar::Int(_) => "int",
            TdsScalar::Float(_) => "float",
            TdsScalar::Decimal(_) => "decimal",
            TdsScalar::Text(_) => "string",
            TdsScalar::Bytes(_) => "bytes",
            TdsScalar::Date(_) => "date",
            TdsScalar::Time(_) => "time",
            TdsScalar::DateTime { .. } => "datetime",
            TdsScalar::Instant { .. } => "instant",
            TdsScalar::Uuid(_) => "uuid",
        })
    }
}

impl TdsScalar<'_> {
    /// The Novis value, taking on the one reference a `string` or a `bytes`
    /// costs and nothing at all for the rest.
    ///
    /// `None` for § 9's five structured rows, whose Novis type is a class
    /// instance this crate cannot allocate at all — [`TdsDate`] owns why. A
    /// caller that wants the whole table matches those five variants first and
    /// reaches this for everything left, exactly as it does on the other two
    /// drivers.
    pub fn into_value(self) -> Option<Value> {
        Some(match self {
            TdsScalar::Null => Value::null(),
            TdsScalar::Bool(value) => Value::bool(value),
            TdsScalar::Int(value) => Value::int(value),
            TdsScalar::Float(value) => Value::float(value),
            TdsScalar::Decimal(value) => Value::decimal(value),
            TdsScalar::Text(text) => Value::str(NvsStr::new(text.as_bytes())),
            TdsScalar::Bytes(octets) => Value::bytes(NvsStr::new(octets)),
            _ => return None,
        })
    }
}

/// One column's value as the Novis value § 9's table names, or `None` for the
/// rows that are class instances.
///
/// [`crate::mysql::decode`]'s twin, and free rather than a method for the same
/// reason: the subject of the match is [`TdsColumn::type_info`], and the value
/// only fills in what the type already decided. It carries the longer name
/// because [`decode`] is already this module's *packet* reader — the framing is
/// written here where the other drivers borrow theirs, which is the one thing
/// that makes this crate's names collide at all.
///
/// # Errors
///
/// As [`scalar`].
pub fn decode_column(column: &TdsColumn, value: Option<&[u8]>) -> io::Result<Option<Value>> {
    Ok(scalar(column, value)?.into_value())
}

/// [`decode`]'s whole decision, before anything is allocated, and every row of
/// § 9's table rather than the scalar half.
///
/// `nvs-stdlib` calls this one: it is the only crate that can turn a
/// [`TdsDate`] and its three siblings into the `Core\Time` instances the table
/// names.
///
/// **The column decides the row and the value only fills it in**, which is why
/// this takes both: `intn`, `fltn`, `moneyn` and `datetimen` are one type byte
/// covering two or four widths, and the width is read off the value's own
/// length because that is where TDS puts it. The one thing read off the value
/// instead is `NULL`, which every column may be.
///
/// **Three rows are computed rather than copied**, and each is a place a reader
/// that trusted the obvious layout would be quietly wrong:
///
/// - `money` is **not** a little-endian `i64`. Its high four bytes come first,
///   which is the one big-endian field in the protocol, and the value is scaled
///   by ten thousand.
/// - `uniqueidentifier`'s first three groups are little-endian on the wire and
///   big-endian in the text form every other system spells it in, so the
///   octets are reordered here — a straight copy answers a different UUID, not
///   a malformed one.
/// - `datetimeoffset` stores its civil fields **in UTC** with the offset
///   beside them, unlike `timestamptz`, so the offset is applied here and
///   [`TdsScalar::Instant`] carries local fields on both drivers.
///
/// A non-Unicode `char`/`varchar`/`text` is read as UTF-8 and refused when it
/// is not. SQL Server has no session charset to force — `rule:core-classes/db-capabilities`'s
/// guarantee is `utf8mb4` on one protocol and nothing on this one — so the
/// column's collation decides its code page, and transcoding an arbitrary one
/// would be a character table this crate does not carry. A UTF-8 collation
/// (SQL Server 2019 and later) reads; anything else is a refusal naming the
/// column rather than an `rule:types/bytes` string that is not UTF-8.
///
/// # Errors
///
/// `InvalidData` for a value its column's own type cannot be read out of: a
/// width the type does not have, a day or tick count outside the calendar,
/// octets that are not the UCS-2 or UTF-8 the column claimed, and the two types
/// whose octets carry their own type description — `sql_variant` and a CLR
/// type — which have no reading at all here. The message names the column and
/// never the value, per [ADR 0067 § 8](/docs/decisions/0067.md).
pub fn scalar<'a>(column: &TdsColumn, value: Option<&'a [u8]>) -> io::Result<TdsScalar<'a>> {
    let Some(bytes) = value else {
        return Ok(TdsScalar::Null);
    };
    let name = column.name.as_str();
    let info = column.type_info;
    Ok(match info.id {
        TY_NULL => TdsScalar::Null,
        TY_BIT | TY_BITN => TdsScalar::Bool(octets::<1>(bytes, name, "a `bit`")?[0] != 0),
        // `tinyint` is the unsigned one, and it is the reason § 9's `uint` row
        // is not reachable on this driver: every value of it fits an `int`.
        TY_INT1 => TdsScalar::Int(i64::from(octets::<1>(bytes, name, "a `tinyint`")?[0])),
        TY_INT2 => TdsScalar::Int(i64::from(i16::from_le_bytes(octets(
            bytes,
            name,
            "a `smallint`",
        )?))),
        TY_INT4 => TdsScalar::Int(i64::from(i32::from_le_bytes(octets(
            bytes, name, "an `int`",
        )?))),
        TY_INT8 => TdsScalar::Int(i64::from_le_bytes(octets(bytes, name, "a `bigint`")?)),
        TY_INTN => TdsScalar::Int(match bytes.len() {
            1 => i64::from(bytes[0]),
            2 => i64::from(i16::from_le_bytes(octets(bytes, name, "a `smallint`")?)),
            4 => i64::from(i32::from_le_bytes(octets(bytes, name, "an `int`")?)),
            8 => i64::from_le_bytes(octets(bytes, name, "a `bigint`")?),
            width => return Err(width_of(name, "an integer", width)),
        }),
        TY_FLT4 => TdsScalar::Float(f64::from(f32::from_le_bytes(octets(
            bytes, name, "a `real`",
        )?))),
        TY_FLT8 => TdsScalar::Float(f64::from_le_bytes(octets(bytes, name, "a `float`")?)),
        TY_FLTN => TdsScalar::Float(match bytes.len() {
            4 => f64::from(f32::from_le_bytes(octets(bytes, name, "a `real`")?)),
            8 => f64::from_le_bytes(octets(bytes, name, "a `float`")?),
            width => return Err(width_of(name, "a float", width)),
        }),
        TY_MONEY => TdsScalar::Decimal(money(bytes, name)?),
        TY_MONEY4 => TdsScalar::Decimal(small_money(bytes, name)?),
        TY_MONEYN => TdsScalar::Decimal(match bytes.len() {
            4 => small_money(bytes, name)?,
            8 => money(bytes, name)?,
            width => return Err(width_of(name, "a money", width)),
        }),
        TY_DECIMALN | TY_NUMERICN => TdsScalar::Decimal(numeric(bytes, name, info.scale)?),
        TY_GUID => TdsScalar::Uuid(guid(bytes, name)?),
        TY_DATEN => TdsScalar::Date(date_of(bytes, name)?),
        TY_TIMEN => TdsScalar::Time(time_of(bytes, name, info.scale)?),
        TY_DATETIME2N => {
            let (date, time) = datetime2(bytes, name, info.scale)?;
            TdsScalar::DateTime { date, time }
        }
        TY_DATETIMEOFFSETN => offset_datetime(bytes, name, info.scale)?,
        TY_DATETIME => {
            let (date, time) = datetime(bytes, name)?;
            TdsScalar::DateTime { date, time }
        }
        TY_DATETIME4 => {
            let (date, time) = small_datetime(bytes, name)?;
            TdsScalar::DateTime { date, time }
        }
        TY_DATETIMEN => {
            let (date, time) = match bytes.len() {
                4 => small_datetime(bytes, name)?,
                8 => datetime(bytes, name)?,
                width => return Err(width_of(name, "a datetime", width)),
            };
            TdsScalar::DateTime { date, time }
        }
        TY_NCHAR | TY_NVARCHAR | TY_NTEXT | TY_XML => {
            TdsScalar::Text(Cow::Owned(wide_text(bytes, name)?))
        }
        TY_BIGBINARY | TY_BIGVARBINARY | TY_IMAGE => TdsScalar::Bytes(bytes),
        // `sql_variant` carries a type header and a CLR type carries a
        // serialized object; both would render as text that is not the value,
        // which is the one answer worse than no answer.
        TY_VARIANT | TY_UDT => {
            return Err(malformed(format!(
                "column `{name}` is a `sql_variant` or a CLR type, which this driver has no \
                 reading for — cast it in the statement to a type `rule:core-classes/db-column-types`'s table names"
            )));
        }
        // `char`, `varchar`, `text` — and every type byte this driver has no
        // row for, which `TdsColumn::column_type` already answers `Other` for
        // and § 9's last row reads as a `tainted string`.
        _ => TdsScalar::Text(Cow::Borrowed(narrow_text(bytes, name)?)),
    })
}

/// A value of exactly `N` octets, or a refusal naming the column and the width
/// it actually had.
pub(super) fn octets<const N: usize>(
    bytes: &[u8],
    column: &str,
    what: &str,
) -> io::Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| width_of(column, what, bytes.len()))
}

/// The refusal every width mismatch above reports, worded once.
pub(super) fn width_of(column: &str, what: &str, width: usize) -> io::Error {
    malformed(format!(
        "column `{column}` is {what} the server sent in {width} byte(s), which is not a width \
         that type has"
    ))
}

/// A `money`: the high four bytes first — the protocol's one big-endian field —
/// and a value scaled by ten thousand.
pub(super) fn money(bytes: &[u8], column: &str) -> io::Result<Decimal> {
    let raw: [u8; 8] = octets(bytes, column, "a `money`")?;
    let high = i64::from(i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]));
    let low = i64::from(u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]));
    exact(column, (high << 32) | low, MONEY_SCALE)
}

/// A `smallmoney`, which is the same scale in four little-endian bytes.
pub(super) fn small_money(bytes: &[u8], column: &str) -> io::Result<Decimal> {
    let units = i64::from(i32::from_le_bytes(octets(bytes, column, "a `smallmoney`")?));
    exact(column, units, MONEY_SCALE)
}

/// A `decimal`/`numeric`: a sign byte, then the magnitude in up to sixteen
/// little-endian bytes, at the scale the column declared.
pub(super) fn numeric(bytes: &[u8], column: &str, scale: u8) -> io::Result<Decimal> {
    let Some((sign, magnitude)) = bytes.split_first() else {
        return Err(width_of(column, "a `decimal`", 0));
    };
    if magnitude.len() > NUMERIC_BYTES {
        return Err(width_of(column, "a `decimal`", bytes.len()));
    }
    let mut units: u128 = 0;
    for (index, byte) in magnitude.iter().enumerate() {
        units |= u128::from(*byte) << (index * 8);
    }
    // The sign byte is `1` for positive on this wire, unlike every other
    // sign-magnitude encoding in this crate.
    parse_exact(column, *sign == 0, units, scale)
}

/// `units` at `scale` as a [`Decimal`], for the two money types whose magnitude
/// is already signed.
pub(super) fn exact(column: &str, units: i64, scale: u8) -> io::Result<Decimal> {
    parse_exact(column, units < 0, u128::from(units.unsigned_abs()), scale)
}

/// The one place a scaled integer becomes a [`Decimal`]: digits, a point placed
/// `scale` from the right, and `rule:types/decimal`'s own parser reading it back.
///
/// Text rather than a constructor because that is the only exact route into a
/// `Decimal` this crate has, and it is the route the other two drivers take
/// from the server's own rendering — so all three agree by construction.
pub(super) fn parse_exact(
    column: &str,
    negative: bool,
    units: u128,
    scale: u8,
) -> io::Result<Decimal> {
    let digits = units.to_string();
    let places = usize::from(scale);
    let mut text = String::with_capacity(digits.len() + places + 3);
    if negative && units != 0 {
        text.push('-');
    }
    if places == 0 {
        text.push_str(&digits);
    } else {
        let padded = format!("{digits:0>width$}", width = places + 1);
        let (whole, fraction) = padded.split_at(padded.len() - places);
        text.push_str(whole);
        text.push('.');
        text.push_str(fraction);
    }
    Decimal::read(&text).map_err(|why| match why {
        NotDecimal::PastRange => malformed(format!(
            "column `{column}` carries a number with more precision than a `decimal` holds — 96 \
             mantissa bits and a scale of 28 — so the column's own type is what narrows"
        )),
        // The text above is a decimal literal by construction, so this arm is
        // the loop that built it having gone wrong rather than anything the
        // server sent. `crate::pg::PgColumn::decimal` is the same split where
        // the server's own rendering is what arrives, and owns why there are
        // two refusals.
        NotDecimal::Unreadable => malformed(format!(
            "column `{column}` holds a scaled integer this driver did not render as a decimal"
        )),
    })
}

/// A `uniqueidentifier`, reordered out of the wire's mixed-endian layout into
/// the order the text form spells.
pub(super) fn guid(bytes: &[u8], column: &str) -> io::Result<[u8; 16]> {
    let raw: [u8; 16] = octets(bytes, column, "a `uniqueidentifier`")?;
    Ok([
        raw[3], raw[2], raw[1], raw[0], raw[5], raw[4], raw[7], raw[6], raw[8], raw[9], raw[10],
        raw[11], raw[12], raw[13], raw[14], raw[15],
    ])
}

/// A `date`: three little-endian bytes of days since `0001-01-01`.
pub(super) fn date_of(bytes: &[u8], column: &str) -> io::Result<TdsDate> {
    let raw: [u8; 3] = octets(bytes, column, "a `date`")?;
    let days = i64::from(u32::from_le_bytes([raw[0], raw[1], raw[2], 0]));
    civil_from_days(column, days - DAYS_0001_TO_1970)
}

/// A `time(n)`: three, four or five little-endian bytes counting `10^-scale`
/// seconds since midnight.
pub(super) fn time_of(bytes: &[u8], column: &str, scale: u8) -> io::Result<TdsTime> {
    civil_from_nanos(column, scaled_ticks(bytes, column, scale)?)
}

/// A `datetime2(n)`: the `time(n)` first, then the `date`.
pub(super) fn datetime2(bytes: &[u8], column: &str, scale: u8) -> io::Result<(TdsDate, TdsTime)> {
    let split = time_bytes(scale);
    if bytes.len() != split + usize::from(DATE_BYTES) {
        return Err(width_of(column, "a `datetime2`", bytes.len()));
    }
    let time = time_of(&bytes[..split], column, scale)?;
    let date = date_of(&bytes[split..], column)?;
    Ok((date, time))
}

/// A `datetimeoffset(n)`: a `datetime2(n)` **in UTC**, then the offset in
/// signed little-endian minutes.
///
/// The shift is made here rather than left to the caller because
/// [`TdsScalar::Instant`] means the same thing on both drivers that answer one,
/// and PostgreSQL's `timestamptz` arrives already rendered at the session zone.
pub(super) fn offset_datetime<'a>(
    bytes: &[u8],
    column: &str,
    scale: u8,
) -> io::Result<TdsScalar<'a>> {
    let split = bytes.len().saturating_sub(usize::from(OFFSET_BYTES));
    if bytes.len() != time_bytes(scale) + usize::from(DATE_BYTES) + usize::from(OFFSET_BYTES) {
        return Err(width_of(column, "a `datetimeoffset`", bytes.len()));
    }
    let minutes = i64::from(i16::from_le_bytes(octets(
        &bytes[split..],
        column,
        "a `datetimeoffset`",
    )?));
    let (date, time) = datetime2(&bytes[..split], column, scale)?;
    let shifted = days_from_civil(date) * NANOS_PER_DAY
        + i64::try_from(nanos_of(time)).expect("a time of day is under a day of nanoseconds")
        + minutes * 60 * NANOS_PER_SECOND;
    let days = shifted.div_euclid(NANOS_PER_DAY);
    let within = shifted.rem_euclid(NANOS_PER_DAY);
    Ok(TdsScalar::Instant {
        date: civil_from_days(column, days)?,
        time: civil_from_nanos(
            column,
            u64::try_from(within).expect("a Euclidean remainder is not negative"),
        )?,
        offset: i32::try_from(minutes * 60).expect("an offset in minutes fits seconds"),
    })
}

/// A `datetime`: four signed little-endian bytes of days since `1900-01-01`,
/// then four unsigned ones counting three-hundredths of a second since
/// midnight.
pub(super) fn datetime(bytes: &[u8], column: &str) -> io::Result<(TdsDate, TdsTime)> {
    let raw: [u8; 8] = octets(bytes, column, "a `datetime`")?;
    let days = i64::from(i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]));
    let ticks = u64::from(u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]));
    // A tick is a three-hundredth of a second, so the nanoseconds are a third
    // of ten million per tick — the reason this column's milliseconds only ever
    // end in 0, 3 and 7.
    let time = civil_from_nanos(column, ticks * 10_000_000 / 3)?;
    Ok((civil_from_days(column, days - DAYS_1900_TO_1970)?, time))
}

/// A `smalldatetime`: two unsigned little-endian bytes of days since
/// `1900-01-01`, then two of minutes since midnight.
pub(super) fn small_datetime(bytes: &[u8], column: &str) -> io::Result<(TdsDate, TdsTime)> {
    let raw: [u8; 4] = octets(bytes, column, "a `smalldatetime`")?;
    let days = i64::from(u16::from_le_bytes([raw[0], raw[1]]));
    let minutes = u64::from(u16::from_le_bytes([raw[2], raw[3]]));
    let time = civil_from_nanos(
        column,
        minutes * 60 * u64::try_from(NANOS_PER_SECOND).expect("a positive constant"),
    )?;
    Ok((civil_from_days(column, days - DAYS_1900_TO_1970)?, time))
}

/// How many bytes a `time`, and so the time half of a `datetime2` or a
/// `datetimeoffset`, takes at `scale`.
pub(super) fn time_bytes(scale: u8) -> usize {
    match scale {
        0..=2 => 3,
        3..=4 => 4,
        _ => 5,
    }
}

/// A scaled tick count as nanoseconds, from the three, four or five bytes
/// [`time_bytes`] says the scale takes.
pub(super) fn scaled_ticks(bytes: &[u8], column: &str, scale: u8) -> io::Result<u64> {
    if bytes.len() != time_bytes(scale) || scale > MAX_TIME_SCALE {
        return Err(width_of(column, "a `time`", bytes.len()));
    }
    let mut ticks: u64 = 0;
    for (index, byte) in bytes.iter().enumerate() {
        ticks |= u64::from(*byte) << (index * 8);
    }
    // 10^(9 - scale) nanoseconds per tick, which is exact for every scale the
    // type has.
    Ok(ticks * 10u64.pow(u32::from(MAX_NANO_DIGITS - scale)))
}

/// A time of day as the nanoseconds since midnight it counts.
pub(super) fn nanos_of(time: TdsTime) -> u64 {
    (u64::from(time.hour) * 3600 + u64::from(time.minute) * 60 + u64::from(time.second))
        * 1_000_000_000
        + u64::from(time.nanosecond)
}

/// Nanoseconds since midnight as a clock reading, refusing a count that is not
/// one.
pub(super) fn civil_from_nanos(column: &str, nanos: u64) -> io::Result<TdsTime> {
    let day = u64::try_from(NANOS_PER_DAY).expect("a positive constant");
    if nanos >= day {
        return Err(malformed(format!(
            "column `{column}` carries a time of day past midnight, which no `time` column holds"
        )));
    }
    let seconds = nanos / 1_000_000_000;
    Ok(TdsTime {
        hour: u8::try_from(seconds / 3600).expect("under a day"),
        minute: u8::try_from((seconds / 60) % 60).expect("under an hour"),
        second: u8::try_from(seconds % 60).expect("under a minute"),
        nanosecond: u32::try_from(nanos % 1_000_000_000).expect("under a second"),
    })
}

/// Days since `1970-01-01` as a civil date — Howard Hinnant's `civil_from_days`,
/// which is exact for every day the proleptic Gregorian calendar has.
pub(super) fn civil_from_days(column: &str, days: i64) -> io::Result<TdsDate> {
    let shifted = days + 719_468;
    let era = (if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    }) / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year + i64::from(month <= 2);
    Ok(TdsDate {
        year: i32::try_from(year).map_err(|_| out_of_calendar(column))?,
        month: u8::try_from(month).map_err(|_| out_of_calendar(column))?,
        day: u8::try_from(day).map_err(|_| out_of_calendar(column))?,
    })
}

/// [`civil_from_days`]' inverse — Hinnant's `days_from_civil` — which only the
/// `datetimeoffset` shift needs.
pub(super) fn days_from_civil(date: TdsDate) -> i64 {
    let year = i64::from(date.year) - i64::from(date.month <= 2);
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(date.month);
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(date.day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The refusal a day count outside the calendar draws, worded once.
pub(super) fn out_of_calendar(column: &str) -> io::Error {
    malformed(format!(
        "column `{column}` carries a day count outside the calendar `Core\\Time\\Date` spans"
    ))
}

/// A UCS-2 value as the UTF-8 an `rule:types/bytes` `string` is.
pub(super) fn wide_text(bytes: &[u8], column: &str) -> io::Result<String> {
    if !bytes.len().is_multiple_of(2) {
        return Err(malformed(format!(
            "column `{column}` is a Unicode column whose value is an odd number of octets, so it \
             is not UCS-2 at all"
        )));
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).map_err(|_| {
        malformed(format!(
            "column `{column}` carries UCS-2 that is not valid UTF-16, so it has no `string`"
        ))
    })
}

/// A non-Unicode value as the UTF-8 an `rule:types/bytes` `string` is, or the refusal
/// [`scalar`]'s doc argues for.
pub(super) fn narrow_text<'a>(bytes: &'a [u8], column: &str) -> io::Result<&'a str> {
    std::str::from_utf8(bytes).map_err(|_| {
        malformed(format!(
            "column `{column}` is not a Unicode column and its collation is not UTF-8, which this \
             driver has no character table to transcode — declare the column `nvarchar` or give it \
             a UTF-8 collation"
        ))
    })
}

/// Days from `0001-01-01` to `1970-01-01`, which is where `date` counts from.
pub(super) const DAYS_0001_TO_1970: i64 = 719_162;

/// Days from `1900-01-01` to `1970-01-01`, which is where `datetime` and
/// `smalldatetime` count from.
pub(super) const DAYS_1900_TO_1970: i64 = 25_567;

/// Nanoseconds in a second, as the signed type the offset shift works in.
pub(super) const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// Nanoseconds in a day.
pub(super) const NANOS_PER_DAY: i64 = 86_400 * NANOS_PER_SECOND;

/// `money` and `smallmoney` are scaled by ten thousand, whatever they are
/// displayed at.
pub(super) const MONEY_SCALE: u8 = 4;

/// The most fractional-second digits `time`, `datetime2` and `datetimeoffset`
/// carry, which is also how many a nanosecond has.
pub(super) const MAX_TIME_SCALE: u8 = 7;

/// Decimal digits in a nanosecond, which is what a scale is subtracted from to
/// size one tick.
pub(super) const MAX_NANO_DIGITS: u8 = 9;

/// The widest magnitude a `decimal` or `numeric` carries, in bytes.
pub(super) const NUMERIC_BYTES: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    /// [`scalar`] and [`TdsColumn::column_type`] answer the same row of § 9's
    /// table for every type this driver reads, and [`TdsScalar::into_value`]
    /// answers `None` for exactly the five rows that are class instances.
    ///
    /// The *agreement* shape rather than a row of expected values: the two
    /// functions are two independent tables over one type byte, and a decoder
    /// that grew its own opinion — a `uniqueidentifier` read as bytes, a
    /// `datetimeoffset` read as a `datetime2` — looks right on its own line and
    /// fails here. `Core\Db\Rows::columns` publishes `column_type`'s answer
    /// while `Row`'s readers get `scalar`'s, so the two disagreeing is a
    /// program told one thing and handed another.
    #[test]
    fn every_column_decodes_to_the_row_its_column_type_already_published() {
        let sent: Vec<(u8, u8, Vec<u8>)> = vec![
            (TY_BIT, 0, vec![1]),
            (TY_BITN, 0, vec![0]),
            (TY_INT1, 0, vec![7]),
            (TY_INT2, 0, vec![0xF9, 0xFF]),
            (TY_INT4, 0, 7i32.to_le_bytes().to_vec()),
            (TY_INT8, 0, (-7i64).to_le_bytes().to_vec()),
            (TY_INTN, 0, 7i32.to_le_bytes().to_vec()),
            (TY_FLT4, 0, 1.5f32.to_le_bytes().to_vec()),
            (TY_FLT8, 0, 1.5f64.to_le_bytes().to_vec()),
            (TY_FLTN, 0, 1.5f32.to_le_bytes().to_vec()),
            (TY_MONEY, 0, vec![0, 0, 0, 0, 0x10, 0x27, 0, 0]),
            (TY_MONEY4, 0, 10_000i32.to_le_bytes().to_vec()),
            (TY_MONEYN, 0, 10_000i32.to_le_bytes().to_vec()),
            (TY_DECIMALN, 2, vec![1, 0xB2, 0x0C, 0, 0]),
            (TY_NUMERICN, 2, vec![0, 0xB2, 0x0C, 0, 0]),
            (TY_GUID, 0, (1..=16).collect()),
            (TY_DATEN, 0, vec![0x45, 0x46, 0x0B]),
            (TY_TIMEN, 0, vec![0x08, 0x07, 0x00]),
            (TY_DATETIME2N, 0, vec![0x08, 0x07, 0x00, 0x45, 0x46, 0x0B]),
            (
                TY_DATETIMEOFFSETN,
                0,
                vec![0x08, 0x07, 0x00, 0x45, 0x46, 0x0B, 0xC4, 0xFF],
            ),
            (TY_DATETIME, 0, vec![0, 0, 0, 0, 0, 0, 0, 0]),
            (TY_DATETIME4, 0, vec![0, 0, 0, 0]),
            (TY_DATETIMEN, 0, vec![0, 0, 0, 0]),
            (TY_NVARCHAR, 0, ucs2_of("é")),
            (TY_NCHAR, 0, ucs2_of("ada")),
            (TY_NTEXT, 0, ucs2_of("ada")),
            (TY_XML, 0, ucs2_of("<a/>")),
            (TY_BIGVARCHAR, 0, b"ada".to_vec()),
            (TY_BIGCHAR, 0, b"ada".to_vec()),
            (TY_TEXT, 0, b"ada".to_vec()),
            (TY_BIGVARBINARY, 0, vec![0x00, 0x61, 0xFF]),
            (TY_BIGBINARY, 0, vec![0x00, 0x61, 0xFF]),
            (TY_IMAGE, 0, vec![0x00, 0x61, 0xFF]),
        ];
        for (id, scale, octets) in sent {
            let column = described(id, scale);
            let published = column.column_type();
            let decoded = scalar(&column, Some(&octets)).expect("a value § 9 has a row for");
            assert_eq!(
                format!("{decoded:?}"),
                match published {
                    ColumnType::Bool => "bool",
                    ColumnType::Int => "int",
                    ColumnType::Float => "float",
                    ColumnType::Decimal => "decimal",
                    ColumnType::Bytes => "bytes",
                    ColumnType::Uuid => "uuid",
                    ColumnType::Date => "date",
                    ColumnType::Time => "time",
                    ColumnType::DateTime => "datetime",
                    ColumnType::Instant => "instant",
                    // § 9's last row, which is where `xml` lands.
                    ColumnType::Text | ColumnType::Other => "string",
                    other => panic!("no SQL Server column answers {other:?}"),
                },
                "type {id:#04X} decodes to a different row than `column_type` published"
            );

            let structured = matches!(
                published,
                ColumnType::Date
                    | ColumnType::Time
                    | ColumnType::DateTime
                    | ColumnType::Instant
                    | ColumnType::Uuid
            );
            assert_eq!(
                decoded.into_value().is_none(),
                structured,
                "type {id:#04X} disagrees about whether its row is a class instance"
            );

            // Every column is `?T`, whatever its type says: the null arrives
            // without any of the type's own bytes.
            assert!(matches!(
                scalar(&described(id, scale), None).expect("SQL NULL"),
                TdsScalar::Null
            ));
        }
    }

    /// The three columns whose wire layout is not the obvious one, each
    /// asserted against the reading a decoder that copied the bytes straight
    /// through would have produced.
    ///
    /// All three are *silent* when got wrong — a wrong number, a different
    /// UUID, an hour that is off — so an assertion on the right answer alone
    /// says nothing about whether the wrong one was ever a possibility. That is
    /// why each half is here.
    #[test]
    fn money_a_guid_and_an_offset_are_not_read_the_way_their_bytes_are_laid_out() {
        // `money` puts its high four bytes first, and nothing else in TDS does.
        let raw = [0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
        let TdsScalar::Decimal(exact) =
            scalar(&described(TY_MONEY, 0), Some(&raw)).expect("a `money` § 9 maps to `decimal`")
        else {
            panic!("a `money` is not a `decimal`");
        };
        let straight = i64::from_le_bytes(raw);
        assert_eq!(exact.to_string(), "858993.4593");
        assert_ne!(
            exact.to_string(),
            format!("{}.{:04}", straight / 10_000, straight % 10_000),
            "the high half came second, which is the reading this column does not have"
        );

        // A `uniqueidentifier`'s first three groups are little-endian on the
        // wire and big-endian in every text form of a UUID.
        let raw: Vec<u8> = (1..=16).collect();
        let TdsScalar::Uuid(octets) =
            scalar(&described(TY_GUID, 0), Some(&raw)).expect("a `uniqueidentifier`")
        else {
            panic!("a `uniqueidentifier` is not a uuid");
        };
        assert_eq!(
            octets,
            [4, 3, 2, 1, 6, 5, 8, 7, 9, 10, 11, 12, 13, 14, 15, 16]
        );
        assert_ne!(octets.as_slice(), raw.as_slice());

        // A `datetimeoffset` stores UTC and the offset beside it, so the civil
        // fields move — and this one moves across a year boundary, which is
        // where a shift that only touched the clock stays plausible.
        let raw = [0x08, 0x07, 0x00, 0x45, 0x46, 0x0B, 0xC4, 0xFF];
        let TdsScalar::Instant { date, time, offset } =
            scalar(&described(TY_DATETIMEOFFSETN, 0), Some(&raw)).expect("a `datetimeoffset`")
        else {
            panic!("a `datetimeoffset` is not an instant");
        };
        assert_eq!((date.year, date.month, date.day), (2023, 12, 31));
        assert_eq!((time.hour, time.minute, time.second), (23, 30, 0));
        assert_eq!(offset, -3600);

        // The same bytes without the offset, which is what a `datetime2` reader
        // pointed at them would have said.
        let TdsScalar::DateTime { date, .. } =
            scalar(&described(TY_DATETIME2N, 0), Some(&raw[..6])).expect("a `datetime2`")
        else {
            panic!("a `datetime2` is not a datetime");
        };
        assert_eq!((date.year, date.month, date.day), (2024, 1, 1));
    }

    /// What a column cannot be read as, on both sides of every boundary
    /// [`scalar`] draws.
    ///
    /// The refusals are the whole of `rule:types/bytes`'s guarantee on this driver: SQL
    /// Server has no session charset to force, so a `varchar` in a code page
    /// that is not UTF-8 is the one place a `string` could arrive that is not
    /// UTF-8 at all, and it does not.
    #[test]
    fn a_column_that_is_not_its_declared_type_is_refused_and_never_guessed() {
        // A width the type does not have, against the width it does.
        let column = described(TY_INTN, 0);
        assert!(scalar(&column, Some(&[0, 0, 0])).is_err());
        assert!(scalar(&column, Some(&[0, 0, 0, 0])).is_ok());

        // `rule:types/bytes`: a non-UTF-8 collation has no `string`, and the same octets
        // in the Unicode column beside it do.
        let narrow = scalar(&described(TY_BIGVARCHAR, 0), Some(&[0xE9]))
            .expect_err("a code page this driver cannot transcode");
        assert_eq!(narrow.kind(), io::ErrorKind::InvalidData);
        assert!(narrow.to_string().contains('c'), "the column is named");
        assert!(matches!(
            scalar(&described(TY_NVARCHAR, 0), Some(&ucs2_of("é"))).expect("a Unicode column"),
            TdsScalar::Text(_)
        ));
        // An odd number of octets is not UCS-2 at all, whatever it decodes to.
        assert!(scalar(&described(TY_NVARCHAR, 0), Some(&[0x61])).is_err());

        // The two types whose octets describe themselves, which is the one
        // place this driver refuses a row `column_type` calls `Other`.
        for opaque in [TY_VARIANT, TY_UDT] {
            let refused = scalar(&described(opaque, 0), Some(&[0x01]))
                .expect_err("a value carrying its own type");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        }
    }
}
