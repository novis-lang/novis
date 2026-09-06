//! `TYPE_INFO`: what a column declares itself to be, how wide its values are,
//! and which [ADR 0067 § 9](/docs/decisions/0067.md) row it maps
//! to.
//!
//! A TDS value carries no width of its own — the width comes from the column,
//! which arrived in `COLMETADATA` before the first row — so [`Length`] is the
//! type the row reader measures with and [`TdsColumn`] is what it measures
//! against. A type byte this driver does not read is refused here, named by its
//! byte, rather than guessed at in a value.

use super::*;

/// `COLMETADATA`'s count, and `TYPE_INFO`'s two-byte length, when what is meant
/// is *there is none*: no result set at all in the first, `MAX` — a value that
/// arrives in [`Length::Partial`]'s chunks — in the second.
pub(super) const NO_LENGTH: u16 = 0xFFFF;

/// A [`Length::Long`] value's length, when what is meant is `NULL`.
pub(super) const NO_LENGTH_LONG: u32 = 0xFFFF_FFFF;

/// The eight bytes of row timestamp a `text`, `ntext` or `image` value carries
/// between its text pointer and its length. Read to step over: it says when the
/// row was last written, which nothing here asks.
pub(super) const TEXT_TIMESTAMP: usize = 8;

/// A `PLP` value's declared total, when what is meant is `NULL`.
pub(super) const PLP_NULL: u64 = 0xFFFF_FFFF_FFFF_FFFF;

/// A `PLP` value's declared total, when the server does not know it yet: the
/// chunks say how long it was, and the terminator is the only end.
pub(super) const PLP_UNKNOWN: u64 = 0xFFFF_FFFF_FFFF_FFFE;

/// How much of a [`PLP_UNKNOWN`]-free value is reserved up front.
///
/// The declared total is the server's word about bytes that are not on the wire
/// yet, the same position [`Tokens::columns`]' own cap is in: reserving two
/// gigabytes because a `varbinary(max)` said so is a per-row allocation an
/// answer of one packet can ask for.
pub(super) const PLP_RESERVE: usize = 64 * 1024;

/// `COLMETADATA`'s `Flags`: the column may be null. The other fifteen bits are
/// what the server would say about updatability, identity and sparseness, and
/// nothing in this driver asks.
pub(super) const COLUMN_NULLABLE: u16 = 0x0001;

/// `NULLTYPE`: a column with no type, which is what `SELECT NULL` has.
pub(super) const TY_NULL: u8 = 0x1F;
/// `INT1TYPE`: `tinyint`, and the one integer SQL Server does not sign.
pub(super) const TY_INT1: u8 = 0x30;
/// `BITTYPE`: `bit`.
pub(super) const TY_BIT: u8 = 0x32;
/// `INT2TYPE`: `smallint`.
pub(super) const TY_INT2: u8 = 0x34;
/// `INT4TYPE`: `int`.
pub(super) const TY_INT4: u8 = 0x38;
/// `DATETIM4TYPE`: `smalldatetime`.
pub(super) const TY_DATETIME4: u8 = 0x3A;
/// `FLT4TYPE`: `real`.
pub(super) const TY_FLT4: u8 = 0x3B;
/// `MONEYTYPE`: `money`, four decimal places in a scaled 64-bit integer.
pub(super) const TY_MONEY: u8 = 0x3C;
/// `DATETIMETYPE`: `datetime`.
pub(super) const TY_DATETIME: u8 = 0x3D;
/// `FLT8TYPE`: `float`.
pub(super) const TY_FLT8: u8 = 0x3E;
/// `MONEY4TYPE`: `smallmoney`.
pub(super) const TY_MONEY4: u8 = 0x7A;
/// `INT8TYPE`: `bigint`.
pub(super) const TY_INT8: u8 = 0x7F;
/// `GUIDTYPE`: `uniqueidentifier`.
pub(super) const TY_GUID: u8 = 0x24;
/// `INTNTYPE`: any of the four integers where the column is nullable, its width
/// in the declared length rather than in the type byte.
pub(super) const TY_INTN: u8 = 0x26;
/// `BITNTYPE`: a nullable `bit`.
pub(super) const TY_BITN: u8 = 0x68;
/// `DECIMALNTYPE`: `decimal`, carrying its own precision and scale.
pub(super) const TY_DECIMALN: u8 = 0x6A;
/// `NUMERICNTYPE`: `numeric`, which SQL Server stores identically.
pub(super) const TY_NUMERICN: u8 = 0x6C;
/// `FLTNTYPE`: a nullable `real` or `float`.
pub(super) const TY_FLTN: u8 = 0x6D;
/// `MONEYNTYPE`: a nullable `money` or `smallmoney`.
pub(super) const TY_MONEYN: u8 = 0x6E;
/// `DATETIMNTYPE`: a nullable `datetime` or `smalldatetime`.
pub(super) const TY_DATETIMEN: u8 = 0x6F;
/// `DATENTYPE`: `date`, whose `TYPE_INFO` carries nothing at all — three bytes
/// is the only width it has.
pub(super) const TY_DATEN: u8 = 0x28;
/// `TIMENTYPE`: `time`, whose `TYPE_INFO` carries a scale and no length.
pub(super) const TY_TIMEN: u8 = 0x29;
/// `DATETIME2NTYPE`: `datetime2`, a `TY_TIMEN` with three bytes of date after
/// it.
pub(super) const TY_DATETIME2N: u8 = 0x2A;
/// `DATETIMEOFFSETNTYPE`: `datetimeoffset`, a `TY_DATETIME2N` with two bytes of
/// offset after it — the one SQL Server type that carries its own zone.
pub(super) const TY_DATETIMEOFFSETN: u8 = 0x2B;
/// `BIGVARBINTYPE`: `varbinary(n)`, or `varbinary(max)` at [`NO_LENGTH`].
pub(super) const TY_BIGVARBINARY: u8 = 0xA5;
/// `BIGVARCHRTYPE`: `varchar(n)`, or `varchar(max)` at [`NO_LENGTH`].
pub(super) const TY_BIGVARCHAR: u8 = 0xA7;
/// `BIGBINARYTYPE`: `binary(n)`.
pub(super) const TY_BIGBINARY: u8 = 0xAD;
/// `BIGCHARTYPE`: `char(n)`.
pub(super) const TY_BIGCHAR: u8 = 0xAF;
/// `NVARCHARTYPE`: `nvarchar(n)`, or `nvarchar(max)` at [`NO_LENGTH`].
pub(super) const TY_NVARCHAR: u8 = 0xE7;
/// `NCHARTYPE`: `nchar(n)`.
pub(super) const TY_NCHAR: u8 = 0xEF;
/// `IMAGETYPE`: `image`, deprecated since 2005 and still on disk everywhere.
pub(super) const TY_IMAGE: u8 = 0x22;
/// `TEXTTYPE`: `text`, likewise.
pub(super) const TY_TEXT: u8 = 0x23;
/// `NTEXTTYPE`: `ntext`, likewise.
pub(super) const TY_NTEXT: u8 = 0x63;
/// `SSVARIANTTYPE`: `sql_variant`, a value carrying its own type description in
/// front of itself.
pub(super) const TY_VARIANT: u8 = 0x62;
/// `UDTTYPE`: a CLR type, which is what `geometry`, `geography` and
/// `hierarchyid` arrive as.
pub(super) const TY_UDT: u8 = 0xF0;
/// `XMLTYPE`: `xml`, optionally naming the schema collection it is bound to.
pub(super) const TY_XML: u8 = 0xF1;

/// The three bytes a `date` occupies, and the date half of a `datetime2`.
pub(super) const DATE_BYTES: u8 = 3;
/// The two bytes of minute offset a `datetimeoffset` carries after its
/// `datetime2` half.
pub(super) const OFFSET_BYTES: u8 = 2;

/// How one column's values carry their own length, which is the whole of what
/// a row reader needs from a type it does not otherwise understand.
///
/// MS-TDS states this as four families — `FIXEDLENTYPE`, `BYTELEN`, `USHORTLEN`
/// and `LONGLEN` — plus `PARTLEN` for the `MAX` types, and the families are not
/// derivable from the type byte in any shorter way than the table
/// [`Tokens::type_info`] writes out. Every variant but [`Length::Fixed`] has a
/// null form, which is why a fixed-width nullable column arrives as its `…N`
/// type instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// Exactly this many bytes, with no length in front of them and no null
    /// form: the type byte is the width.
    Fixed(usize),
    /// One byte of length, at most this many, and `0` is `NULL`.
    Byte(u8),
    /// Two bytes of length, at most this many, and [`NO_LENGTH`] is `NULL`.
    Short(u16),
    /// Four bytes of length, at most this many, and [`NO_LENGTH_LONG`] is
    /// `NULL`.
    ///
    /// `text`, `ntext` and `image` put a text pointer and a timestamp in front
    /// of that length and are `NULL` where the pointer is empty; `sql_variant`
    /// does not — it is `NULL` at a length of **zero** as well, since a
    /// `sql_variant` carries its own type description and a value of no bytes
    /// at all is not one. The row reader tells them apart by [`TypeInfo::id`],
    /// because nothing else about the two is different.
    Long(u32),
    /// `PLP`: an eight-byte total length or an unknown-length sentinel, then
    /// chunks until an empty one. Every `MAX` type, `xml` and every CLR type.
    Partial,
}

/// One column's `TYPE_INFO`: what the server said its type is, in the server's
/// own vocabulary.
///
/// Deliberately not a Novis type — [`TdsColumn::column_type`] is the only thing
/// here that has an opinion about that, and [ADR 0067
/// § 9](/docs/decisions/0067.md)'s decode into a value belongs to
/// `nvs-stdlib`, which is the crate that can allocate a `Core\Time\DateTime`.
/// The fields are all four things a `TYPE_INFO` can carry, and a type that
/// carries none of them leaves them at their zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeInfo {
    /// The type byte, which is the identity of the type and the only thing
    /// [`TdsColumn::column_type`] reads.
    pub id: u8,
    /// How a value of this type is measured on the wire.
    pub length: Length,
    /// A `decimal` or `numeric` column's declared precision, and `0` for every
    /// other type.
    pub precision: u8,
    /// A `decimal`/`numeric` column's declared scale, or the fractional-second
    /// digits of a `time`, `datetime2` or `datetimeoffset` — the same field
    /// because the wire spells them with one byte in the same place.
    pub scale: u8,
    /// The five raw bytes of the column's `COLLATION`, for the six character
    /// types that carry one: an LCID and flags in four little-endian bytes,
    /// then a sort id.
    ///
    /// Kept raw and unparsed because the one thing a reader will ever want from
    /// it is the code page of a non-Unicode column, and that question belongs
    /// to the row path rather than here.
    pub collation: Option<[u8; 5]>,
}

/// One column of a result set, as `COLMETADATA` described it.
///
/// The shape [`crate::PgColumn`] has, for the reason its doc gives: a driver's
/// column carries the *server's* type description and never a Novis type. What
/// differs is that TDS describes a type structurally rather than by a catalog
/// id, so the description is [`TypeInfo`] and not an integer this driver would
/// have to hold a table for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TdsColumn {
    /// The column's label, as the server wrote it.
    pub name: String,
    /// The user-defined type id, `0` for every built-in type. This driver reads
    /// it only to skip it: a UDT's own name is in [`TypeInfo`].
    pub user_type: u32,
    /// The `Flags` field, read through [`TdsColumn::nullable`] rather than
    /// matched on.
    pub flags: u16,
    /// What the server said the column's type is.
    pub type_info: TypeInfo,
}

impl TdsColumn {
    /// Whether the server said this column may be null.
    ///
    /// Not what makes a value optional to a program: every column reads back as
    /// `?T` because a value can be `NULL` whatever the schema says, and a
    /// server computes this bit for an expression rather than reading it off a
    /// table.
    #[must_use]
    pub const fn nullable(&self) -> bool {
        self.flags & COLUMN_NULLABLE != 0
    }

    /// The column's declared type, as spec § 18's `ColumnType` names it — what
    /// `Core\Db\Rows::columns` answers for this column.
    ///
    /// It reads the type byte and nothing else, which is the whole difference
    /// from [`crate::PgColumn::column_type`]: PostgreSQL has one OID for `bit`
    /// and `bit varying` and needs the modifier to find [ADR 0067
    /// § 9](/docs/decisions/0067.md)'s `BIT(1)` row, while SQL
    /// Server's `bit` *is* one bit — `BIT(n>1)` is not a type it has — so the
    /// byte is the answer and the width is never consulted.
    ///
    /// Two more of § 9's rows fall out of the same table. `uniqueidentifier` is
    /// [`ColumnType::Uuid`] rather than [`ColumnType::Bytes`], unlike MySQL's
    /// `BINARY(16)`, because it is a type of its own here. And nothing answers
    /// [`ColumnType::Json`]: SQL Server through 2022 stores JSON in an
    /// `nvarchar` with a `CHECK` constraint, so a JSON column *is* a text
    /// column and § 9's rule that JSON is never auto-decoded is what makes that
    /// the honest answer rather than a lost one.
    ///
    /// [`ColumnType::Uint`] is likewise unreachable: `tinyint` is the only
    /// unsigned integer SQL Server has and it fits an `int` with room to spare,
    /// so § 9's `uint` row is MySQL's and PostgreSQL's alone.
    #[must_use]
    pub const fn column_type(&self) -> ColumnType {
        match self.type_info.id {
            TY_INT1 | TY_INT2 | TY_INT4 | TY_INT8 | TY_INTN => ColumnType::Int,
            TY_BIT | TY_BITN => ColumnType::Bool,
            TY_FLT4 | TY_FLT8 | TY_FLTN => ColumnType::Float,
            TY_MONEY | TY_MONEY4 | TY_MONEYN | TY_DECIMALN | TY_NUMERICN => ColumnType::Decimal,
            TY_BIGCHAR | TY_BIGVARCHAR | TY_NCHAR | TY_NVARCHAR | TY_TEXT | TY_NTEXT => {
                ColumnType::Text
            }
            TY_BIGBINARY | TY_BIGVARBINARY | TY_IMAGE => ColumnType::Bytes,
            TY_GUID => ColumnType::Uuid,
            TY_DATEN => ColumnType::Date,
            TY_TIMEN => ColumnType::Time,
            TY_DATETIME | TY_DATETIME4 | TY_DATETIMEN | TY_DATETIME2N => ColumnType::DateTime,
            TY_DATETIMEOFFSETN => ColumnType::Instant,
            // `sql_variant`, `xml`, every CLR type — `geometry` among them —
            // and the column that has no type at all: § 9's last row, which
            // reads as a `tainted string`.
            _ => ColumnType::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    /// Every type this driver reads is measured the way MS-TDS § 2.2.5.4.1
    /// says, and the count is asserted rather than the rows: a type byte that
    /// gained an arm without gaining a row here fails the last line.
    #[test]
    fn every_type_info_is_measured_the_way_ms_tds_measures_it() {
        let table = every_type_info();
        for (bytes, length, _) in &table {
            let column = one_column(bytes)
                .unwrap_or_else(|refused| panic!("type 0x{:02X}: {refused}", bytes[0]));
            assert_eq!(column.type_info.length, *length, "type 0x{:02X}", bytes[0]);
            assert_eq!(column.type_info.id, bytes[0]);
            assert_eq!(
                column.name, "c",
                "the name is after the TYPE_INFO, so a type read short takes the name with it"
            );
        }

        for family in [
            Length::Fixed(0),
            Length::Byte(0),
            Length::Short(0),
            Length::Long(0),
            Length::Partial,
        ] {
            let sort = std::mem::discriminant(&family);
            assert!(
                table
                    .iter()
                    .any(|(_, length, _)| std::mem::discriminant(length) == sort),
                "no type in the table is measured as {family:?}, so that family is untested"
            );
        }

        let ids: std::collections::HashSet<u8> =
            table.iter().map(|(bytes, _, _)| bytes[0]).collect();
        assert_eq!(
            ids.len(),
            36,
            "every type byte `Tokens::type_info` has an arm for is one row here"
        );
    }

    /// § 9's rows this backend can reach, asserted as a set — so a type that
    /// classified as a plausible neighbour fails here even where its own row
    /// still passes.
    #[test]
    fn every_column_type_this_backend_reaches_is_a_row_of_section_nine() {
        let mut seen: std::collections::HashSet<ColumnType> = std::collections::HashSet::new();
        for (bytes, _, expected) in &every_type_info() {
            let column = one_column(bytes)
                .unwrap_or_else(|refused| panic!("type 0x{:02X}: {refused}", bytes[0]));
            assert_eq!(column.column_type(), *expected, "type 0x{:02X}", bytes[0]);
            seen.insert(*expected);
        }

        assert_eq!(seen.len(), 12, "twelve of § 9's fourteen rows, and these:");
        assert!(
            !seen.contains(&ColumnType::Uint),
            "`tinyint` is the only unsigned integer here and it fits an `int`"
        );
        assert!(
            !seen.contains(&ColumnType::Json),
            "SQL Server stores JSON in an `nvarchar`, so a JSON column is a text column"
        );
    }

    /// A result set's shape is its columns in the order their values will
    /// arrive, and the fields around the `TYPE_INFO` come back with them.
    #[test]
    fn a_colmetadata_describes_its_columns_in_the_order_the_values_arrive() {
        let mut payload = col_metadata(&[
            column(&[TY_INT4], "id", 0),
            column(&char_type(TY_NVARCHAR, 100), "name", COLUMN_NULLABLE),
            column(&long_type(TY_TEXT, MAX_LOB), "note", COLUMN_NULLABLE),
        ]);
        payload.extend_from_slice(&done_token(DONE_COUNT, 3));

        let read = tokens(&payload).expect("a result set this driver can describe");
        assert_eq!(read.len(), 2, "the DONE after the columns is still found");
        let Token::Columns(columns) = &read[0] else {
            panic!("the first token describes the columns");
        };
        assert_eq!(
            columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["id", "name", "note"],
            "`note` is the one that proves the TableName is skipped: a reader that \
             did not would name the column after the table"
        );
        assert!(!columns[0].nullable(), "the server said this one cannot be");
        assert!(columns[1].nullable());
        assert_eq!(columns[1].type_info.collation, Some(COLLATION));
        assert_eq!(
            columns[0].type_info.collation, None,
            "an `int` has no collation to carry"
        );
        assert_eq!(columns[2].column_type(), ColumnType::Text);
    }

    /// TDS 4.2's spellings are refused by their byte rather than parsed on a
    /// guess at a layout a 7.4 server never sends.
    #[test]
    fn a_column_type_this_driver_does_not_read_is_named_by_its_byte() {
        for legacy in [0x2Fu8, 0x27, 0x2D, 0x25, 0x37, 0x3F] {
            let refused = one_column(&[legacy, 8]).expect_err("a 4.2 type is not read here");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
            assert!(
                refused.to_string().contains(&format!("0x{legacy:02X}")),
                "{refused}"
            );
        }
    }

    /// Both sides of the bound: seven fractional-second digits is the widest
    /// `time` SQL Server has, and eight is not a narrower one.
    #[test]
    fn a_time_scale_is_read_up_to_seven_digits_and_refused_past_them() {
        for (scale, width) in [(0u8, 3u8), (2, 3), (3, 4), (4, 4), (5, 5), (7, 5)] {
            let column = one_column(&[TY_TIMEN, scale]).expect("a scale SQL Server has");
            assert_eq!(
                column.type_info.length,
                Length::Byte(width),
                "scale {scale}"
            );
            assert_eq!(column.type_info.scale, scale);
        }

        let refused = one_column(&[TY_DATETIME2N, 8]).expect_err("eight digits is not a time");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("scale of 8"), "{refused}");
    }

    /// `0xFFFF` columns is the server saying the statement had no result set at
    /// all, and it is not a count that reads the tokens after it as columns.
    #[test]
    fn a_statement_with_no_result_set_answers_no_columns() {
        let mut payload = vec![TOKEN_COL_METADATA];
        payload.extend_from_slice(&NO_LENGTH.to_le_bytes());
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let read = tokens(&payload).expect("no metadata is not malformed metadata");
        assert_eq!(read[0], Token::Columns(Vec::new()));
        assert_eq!(read.len(), 2, "the DONE is the next token, not a column");
    }

    /// `COLMETADATA` carries no length of its own, so the fields running out is
    /// the only thing between a lying count and a reader walking into the
    /// tokens after it.
    #[test]
    fn a_colmetadata_that_promises_more_columns_than_it_holds_is_refused() {
        let mut payload = vec![TOKEN_COL_METADATA];
        payload.extend_from_slice(&3u16.to_le_bytes());
        payload.extend_from_slice(&column(&[TY_INT4], "id", 0));
        payload.extend_from_slice(&done_token(0, 0));

        let refused = tokens(&payload).expect_err("three columns were promised and one sent");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
    }
}
