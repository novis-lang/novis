//! What this driver's cases build with: a scripted stream standing in for a
//! server, and the token, column and row builders every module's tests are
//! written against.
//!
//! It is one module rather than a copy per file because the builders are the
//! protocol's shapes, not any one module's — a `COLMETADATA` written two ways
//! would let a row case and a type case disagree about what the server sent.

use super::*;

/// A stream that answers with a canned transcript and keeps what was
/// written.
///
/// Unlike [`crate::mysql`]'s `Peer` this one does not script a server: at
/// this layer nothing depends on what the previous message was, so a fixed
/// transcript asserts everything, and what a scripted server would add is
/// the sequencing the slices above this one bring.
///
/// `chunk` is the point of it: it caps what one `read` answers, so the
/// partial-arrival path — the one a real socket takes and a `Cursor` never
/// does — is exercised by setting it to 1.
pub(super) struct Script {
    pub(super) inbound: Vec<u8>,
    pub(super) read: usize,
    pub(super) chunk: usize,
    pub(super) sent: Vec<u8>,
}

impl Script {
    pub(super) fn answering(inbound: Vec<u8>, chunk: usize) -> Script {
        Script {
            inbound,
            read: 0,
            chunk,
            sent: Vec::new(),
        }
    }

    pub(super) fn silent() -> Script {
        Script::answering(Vec::new(), READ_CHUNK)
    }
}

impl Write for Script {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.sent.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Script {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = &self.inbound[self.read..];
        let take = left.len().min(buf.len()).min(self.chunk);
        buf[..take].copy_from_slice(&left[..take]);
        self.read += take;
        Ok(take)
    }
}

/// One packet as a server would write it, for the read-side cases.
pub(super) fn packet(kind: PacketType, status: Status, id: u8, payload: &[u8]) -> Vec<u8> {
    let length = u16::try_from(HEADER + payload.len()).expect("a test packet fits");
    let mut out = vec![kind.byte(), status.bits()];
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&[0, 0, id, 0]);
    out.extend_from_slice(payload);
    out
}

/// A `[db.<name>]` block with every field a SQL Server connection reads, as
/// an operator writes it and as `nvs_config` hands it over.
pub(super) fn block() -> Database {
    Database {
        driver: Some("mssql".to_owned()),
        host: Some("mssql.test".to_owned()),
        user: Some("sa".to_owned()),
        password: Some("hunter2".to_owned()),
        database: Some("novis_test".to_owned()),
        time_zone: Some("+02:00".to_owned()),
        ..Database::default()
    }
}

/// Which `Option<String>` a field name selects, so the fields that resolve
/// identically are asserted in one loop rather than in a copy each.
pub(super) fn written_field<'a>(block: &'a mut Database, field: &str) -> &'a mut Option<String> {
    match field {
        "host" => &mut block.host,
        "user" => &mut block.user,
        "database" => &mut block.database,
        other => unreachable!("no case names {other}"),
    }
}

/// A PRELOGIN response carrying one `ENCRYPTION` option, as a server writes
/// it: the entry, the terminator, then the byte the entry points at.
pub(super) fn prelogin_answer(encryption: u8) -> Vec<u8> {
    let mut payload = vec![PL_ENCRYPTION];
    // One entry, then the terminator, so the byte is at offset 6.
    payload.extend_from_slice(&(PL_ENTRY + 1).to_be_bytes());
    payload.extend_from_slice(&PL_ENCRYPTION_LEN.to_be_bytes());
    payload.push(PL_TERMINATOR);
    payload.push(encryption);
    packet(PacketType::TabularResult, Status::EOM, 1, &payload)
}

/// Where in LOGIN7's fixed header each offset/length pair sits, by the name
/// MS-TDS gives it.
///
/// `login7_request` writes the pairs in order and never names a position,
/// which is the cheap way to build one and the expensive way to read one
/// back: a pair written a slot out of place is a login the *server*
/// refuses, with a message about the field it landed in. Asserting through
/// this table is what makes that fail here instead.
pub(super) const PAIRS: [(&str, usize); 12] = [
    ("HostName", 36),
    ("UserName", 40),
    ("Password", 44),
    ("AppName", 48),
    ("ServerName", 52),
    ("Unused", 56),
    ("CltIntName", 60),
    ("Language", 64),
    ("Database", 68),
    // `ClientID`'s six bytes sit between these two.
    ("SSPI", 78),
    ("AtchDBFile", 82),
    ("ChangePassword", 86),
];

/// The offset and the character count one pair holds.
pub(super) fn pair(message: &[u8], at: usize) -> (usize, usize) {
    (
        usize::from(u16::from_le_bytes([message[at], message[at + 1]])),
        usize::from(u16::from_le_bytes([message[at + 2], message[at + 3]])),
    )
}

/// The characters one pair points at, read back as a `String`.
pub(super) fn field_at(message: &[u8], at: usize) -> String {
    let (offset, characters) = pair(message, at);
    let units: Vec<u16> = message[offset..offset + characters * 2]
        .chunks_exact(2)
        .map(|two| u16::from_le_bytes([two[0], two[1]]))
        .collect();
    String::from_utf16(&units).expect("this driver writes back what it was handed")
}

/// The `[db.<name>]` block above, as the target LOGIN7 is built from.
pub(super) fn login7(block: &Database) -> Vec<u8> {
    let target = TdsTarget::resolve(block).expect("a complete block resolves");
    login7_request(&target, DEFAULT_PACKET_SIZE).expect("every field of it fits")
}

/// A `B_VARCHAR` as a server writes one.
pub(super) fn b_varchar(text: &str) -> Vec<u8> {
    let mut out = vec![u8::try_from(text.encode_utf16().count()).expect("a short field")];
    out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    out
}

/// A token, given its type byte and the body its `u16` length covers.
pub(super) fn token(kind: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![kind];
    out.extend_from_slice(
        &u16::try_from(body.len())
            .expect("a short token")
            .to_le_bytes(),
    );
    out.extend_from_slice(body);
    out
}

/// An `ERROR` or `INFO` token as a server writes one.
pub(super) fn message_token(kind: u8, number: u32, class: u8, text: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&number.to_le_bytes());
    body.push(1);
    body.push(class);
    body.extend_from_slice(
        &u16::try_from(text.encode_utf16().count())
            .expect("a short sentence")
            .to_le_bytes(),
    );
    body.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    body.extend_from_slice(&b_varchar("mssql.test"));
    body.extend_from_slice(&b_varchar(""));
    body.extend_from_slice(&7u32.to_le_bytes());
    token(kind, &body)
}

/// An `ENVCHANGE` token over two `B_VARCHAR` halves.
pub(super) fn env_token(kind: u8, to: &str, from: &str) -> Vec<u8> {
    let mut body = vec![kind];
    body.extend_from_slice(&b_varchar(to));
    body.extend_from_slice(&b_varchar(from));
    token(TOKEN_ENV_CHANGE, &body)
}

/// A `LOGINACK` as SQL Server 2022 writes one — **including the byte order
/// of its version**, which is the opposite of the one LOGIN7 asked in and is
/// what a real 2022 server puts on the wire.
pub(super) fn login_ack_token() -> Vec<u8> {
    let mut body = vec![1];
    body.extend_from_slice(&TDS_VERSION.to_be_bytes());
    body.extend_from_slice(&b_varchar("Microsoft SQL Server"));
    body.extend_from_slice(&[16, 0]);
    body.extend_from_slice(&4035u16.to_be_bytes());
    token(TOKEN_LOGIN_ACK, &body)
}

/// A `DONE`, which carries no length at all.
pub(super) fn done_token(status: u16, rows: u64) -> Vec<u8> {
    let mut out = vec![TOKEN_DONE];
    out.extend_from_slice(&status.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&rows.to_le_bytes());
    out
}

/// A `DONEINPROC` or a `DONEPROC`, which are the same twelve bytes under
/// another type byte.
pub(super) fn done_kind(kind: u8, status: u16, rows: u64) -> Vec<u8> {
    let mut out = done_token(status, rows);
    out[0] = kind;
    out
}

/// Every token of a message, or the refusal one of them was.
pub(super) fn tokens(payload: &[u8]) -> io::Result<Vec<Token>> {
    let mut reader = Tokens::over(payload);
    let mut out = Vec::new();
    while let Some(token) = reader.next_token()? {
        out.push(token);
    }
    Ok(out)
}

/// One `TabularResult` message carrying `payload`, as a server writes it.
pub(super) fn answer(payload: &[u8]) -> Vec<u8> {
    packet(PacketType::TabularResult, Status::EOM, 1, payload)
}

/// A wire over a server that answers one login and says nothing else.
pub(super) fn logging_in(payload: &[u8]) -> Wire<Script> {
    Wire::new(Script::answering(answer(payload), READ_CHUNK))
}

/// A `US_VARCHAR` as a server writes one.
pub(super) fn us_varchar(text: &str) -> Vec<u8> {
    let mut out = u16::try_from(text.encode_utf16().count())
        .expect("a short field")
        .to_le_bytes()
        .to_vec();
    out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    out
}

/// A `COLLATION` as a server writes one — `Latin1_General_CI_AS` — which
/// this driver keeps and does not read apart.
pub(super) const COLLATION: [u8; 5] = [0x09, 0x04, 0xD0, 0x00, 0x34];

/// The declared maximum of every large-object type.
pub(super) const MAX_LOB: u32 = 0x7FFF_FFFF;

/// One column of a `COLMETADATA`, from its `TYPE_INFO` outwards. A `text`,
/// `ntext` or `image` column's `TableName` belongs on the end of
/// `type_info`, which is where the wire puts it.
pub(super) fn column(type_info: &[u8], name: &str, flags: u16) -> Vec<u8> {
    let mut out = 0u32.to_le_bytes().to_vec();
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(type_info);
    out.extend_from_slice(&b_varchar(name));
    out
}

/// A `COLMETADATA` token over the columns `column` built.
pub(super) fn col_metadata(columns: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![TOKEN_COL_METADATA];
    out.extend_from_slice(
        &u16::try_from(columns.len())
            .expect("a few columns")
            .to_le_bytes(),
    );
    for one in columns {
        out.extend_from_slice(one);
    }
    out
}

/// A `USHORTLEN` character type's `TYPE_INFO`, which carries a collation.
pub(super) fn char_type(id: u8, declared: u16) -> Vec<u8> {
    let mut out = vec![id];
    out.extend_from_slice(&declared.to_le_bytes());
    out.extend_from_slice(&COLLATION);
    out
}

/// A `USHORTLEN` binary type's, which does not.
pub(super) fn binary_type(id: u8, declared: u16) -> Vec<u8> {
    let mut out = vec![id];
    out.extend_from_slice(&declared.to_le_bytes());
    out
}

/// A `LONGLEN` type's, with the collation and the `TableName` for the three
/// types that carry them.
pub(super) fn long_type(id: u8, declared: u32) -> Vec<u8> {
    let mut out = vec![id];
    out.extend_from_slice(&declared.to_le_bytes());
    if matches!(id, TY_TEXT | TY_NTEXT) {
        out.extend_from_slice(&COLLATION);
    }
    if matches!(id, TY_TEXT | TY_NTEXT | TY_IMAGE) {
        out.push(1);
        out.extend_from_slice(&us_varchar("notes"));
    }
    out
}

/// A CLR type's `TYPE_INFO`: `geometry`, as SQL Server describes one.
pub(super) fn udt_type() -> Vec<u8> {
    let mut out = vec![TY_UDT];
    out.extend_from_slice(&NO_LENGTH.to_le_bytes());
    out.extend_from_slice(&b_varchar("novis_test"));
    out.extend_from_slice(&b_varchar("sys"));
    out.extend_from_slice(&b_varchar("geometry"));
    out.extend_from_slice(&us_varchar(
        "Microsoft.SqlServer.Types.SqlGeometry, Microsoft.SqlServer.Types",
    ));
    out
}

/// The one column a `TYPE_INFO` describes, or the refusal reading it was.
pub(super) fn one_column(type_info: &[u8]) -> io::Result<TdsColumn> {
    let payload = col_metadata(&[column(type_info, "c", COLUMN_NULLABLE)]);
    let mut read = tokens(&payload)?;
    let Some(Token::Columns(mut columns)) = read.pop() else {
        panic!("a COLMETADATA reads back as one");
    };
    Ok(columns.pop().expect("the one column that was described"))
}

/// A `TYPE_INFO` for every type byte this driver reads, with the length
/// family MS-TDS gives it and the § 9 row it classifies as. The sweeps below
/// share it, which is what makes them ask about one table.
pub(super) fn every_type_info() -> Vec<(Vec<u8>, Length, ColumnType)> {
    vec![
        (vec![TY_NULL], Length::Fixed(0), ColumnType::Other),
        (vec![TY_INT1], Length::Fixed(1), ColumnType::Int),
        (vec![TY_INT2], Length::Fixed(2), ColumnType::Int),
        (vec![TY_INT4], Length::Fixed(4), ColumnType::Int),
        (vec![TY_INT8], Length::Fixed(8), ColumnType::Int),
        (vec![TY_INTN, 4], Length::Byte(4), ColumnType::Int),
        (vec![TY_BIT], Length::Fixed(1), ColumnType::Bool),
        (vec![TY_BITN, 1], Length::Byte(1), ColumnType::Bool),
        (vec![TY_FLT4], Length::Fixed(4), ColumnType::Float),
        (vec![TY_FLT8], Length::Fixed(8), ColumnType::Float),
        (vec![TY_FLTN, 8], Length::Byte(8), ColumnType::Float),
        (vec![TY_MONEY], Length::Fixed(8), ColumnType::Decimal),
        (vec![TY_MONEY4], Length::Fixed(4), ColumnType::Decimal),
        (vec![TY_MONEYN, 8], Length::Byte(8), ColumnType::Decimal),
        (
            vec![TY_DECIMALN, 17, 38, 4],
            Length::Byte(17),
            ColumnType::Decimal,
        ),
        (
            vec![TY_NUMERICN, 9, 18, 2],
            Length::Byte(9),
            ColumnType::Decimal,
        ),
        (
            char_type(TY_BIGCHAR, 10),
            Length::Short(10),
            ColumnType::Text,
        ),
        (
            char_type(TY_BIGVARCHAR, 8000),
            Length::Short(8000),
            ColumnType::Text,
        ),
        (
            char_type(TY_BIGVARCHAR, NO_LENGTH),
            Length::Partial,
            ColumnType::Text,
        ),
        (char_type(TY_NCHAR, 20), Length::Short(20), ColumnType::Text),
        (
            char_type(TY_NVARCHAR, 100),
            Length::Short(100),
            ColumnType::Text,
        ),
        (
            char_type(TY_NVARCHAR, NO_LENGTH),
            Length::Partial,
            ColumnType::Text,
        ),
        (
            binary_type(TY_BIGBINARY, 16),
            Length::Short(16),
            ColumnType::Bytes,
        ),
        (
            binary_type(TY_BIGVARBINARY, 900),
            Length::Short(900),
            ColumnType::Bytes,
        ),
        (
            binary_type(TY_BIGVARBINARY, NO_LENGTH),
            Length::Partial,
            ColumnType::Bytes,
        ),
        (
            long_type(TY_TEXT, MAX_LOB),
            Length::Long(MAX_LOB),
            ColumnType::Text,
        ),
        (
            long_type(TY_NTEXT, MAX_LOB),
            Length::Long(MAX_LOB),
            ColumnType::Text,
        ),
        (
            long_type(TY_IMAGE, MAX_LOB),
            Length::Long(MAX_LOB),
            ColumnType::Bytes,
        ),
        (
            long_type(TY_VARIANT, 8009),
            Length::Long(8009),
            ColumnType::Other,
        ),
        (vec![TY_GUID, 16], Length::Byte(16), ColumnType::Uuid),
        (vec![TY_DATEN], Length::Byte(3), ColumnType::Date),
        (vec![TY_TIMEN, 7], Length::Byte(5), ColumnType::Time),
        (vec![TY_DATETIME4], Length::Fixed(4), ColumnType::DateTime),
        (vec![TY_DATETIME], Length::Fixed(8), ColumnType::DateTime),
        (vec![TY_DATETIMEN, 8], Length::Byte(8), ColumnType::DateTime),
        (
            vec![TY_DATETIME2N, 7],
            Length::Byte(8),
            ColumnType::DateTime,
        ),
        (
            vec![TY_DATETIMEOFFSETN, 7],
            Length::Byte(10),
            ColumnType::Instant,
        ),
        (vec![TY_XML, 0], Length::Partial, ColumnType::Other),
        (udt_type(), Length::Partial, ColumnType::Other),
    ]
}

// ---- `ROW`, `NBCROW` and PLP over packets: `rule:core-classes/db-statement-members` and `rule:core-classes/db-column-types` ----------

/// A wire over a server that answers with these packets in order, only the
/// last of them ending the message.
pub(super) fn answering(packets: &[Vec<u8>]) -> Wire<Script> {
    let mut inbound = Vec::new();
    for (index, payload) in packets.iter().enumerate() {
        let last = index + 1 == packets.len();
        inbound.extend_from_slice(&packet(
            PacketType::TabularResult,
            if last { Status::EOM } else { Status::NORMAL },
            u8::try_from(index + 1).expect("a few packets"),
            payload,
        ));
    }
    Wire::new(Script::answering(inbound, READ_CHUNK))
}

/// One payload cut into packets of `at` bytes, which is what a real server
/// does at whatever offset its packet size lands on.
pub(super) fn chunked(payload: &[u8], at: usize) -> Vec<Vec<u8>> {
    payload.chunks(at).map(<[u8]>::to_vec).collect()
}

/// The span every case here opens, since none of them is about § 11.
pub(super) fn span() -> QuerySpan {
    QuerySpan::opened(Driver::SqlServer, "select 1")
}

/// A `ROW` token over values already written the way their columns measure
/// them.
pub(super) fn row_token(values: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![TOKEN_ROW];
    for value in values {
        out.extend_from_slice(value);
    }
    out
}

/// An `NBCROW`: the null bitmap `nulls` describes, then the values of the
/// columns it left off.
pub(super) fn nbc_row_token(nulls: &[bool], values: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![TOKEN_NBC_ROW];
    let mut bitmap = vec![0u8; nulls.len().div_ceil(8)];
    for (index, null) in nulls.iter().enumerate() {
        if *null {
            bitmap[index / 8] |= 1 << (index % 8);
        }
    }
    out.extend_from_slice(&bitmap);
    for value in values {
        out.extend_from_slice(value);
    }
    out
}

/// A `BYTELEN` value, and its `NULL` at a length of zero.
pub(super) fn byte_value(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![u8::try_from(bytes.len()).expect("a short value")];
    out.extend_from_slice(bytes);
    out
}

/// A `USHORTLEN` value.
pub(super) fn short_value(bytes: &[u8]) -> Vec<u8> {
    let mut out = u16::try_from(bytes.len())
        .expect("a short value")
        .to_le_bytes()
        .to_vec();
    out.extend_from_slice(bytes);
    out
}

/// A `text`, `ntext` or `image` value: a text pointer, the row timestamp
/// nothing reads, and then the length.
pub(super) fn long_value(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![2, 0xAB, 0xCD];
    out.extend_from_slice(&[0; TEXT_TIMESTAMP]);
    out.extend_from_slice(
        &u32::try_from(bytes.len())
            .expect("a short value")
            .to_le_bytes(),
    );
    out.extend_from_slice(bytes);
    out
}

/// A `PLP` value: a declared total, the chunks, and the empty one that ends
/// them.
///
/// A `NULL` is the sentinel alone — MS-TDS § 2.2.5.2.3 gives it no chunks to
/// terminate — which is exactly the four bytes a reader that returned early
/// would leave on the wire.
pub(super) fn plp_value(total: u64, chunks: &[&[u8]]) -> Vec<u8> {
    let mut out = total.to_le_bytes().to_vec();
    if total == PLP_NULL {
        return out;
    }
    for chunk in chunks {
        out.extend_from_slice(
            &u32::try_from(chunk.len())
                .expect("a short chunk")
                .to_le_bytes(),
        );
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

/// The four columns the cases below read rows against, one per length
/// family: `int`, `varchar(10)`, a nullable `int` and `text`.
pub(super) fn four_columns() -> Vec<u8> {
    col_metadata(&[
        column(&[TY_INT4], "id", 0),
        column(&char_type(TY_BIGVARCHAR, 10), "name", COLUMN_NULLABLE),
        column(&[TY_INTN, 4], "score", COLUMN_NULLABLE),
        column(&long_type(TY_TEXT, MAX_LOB), "notes", COLUMN_NULLABLE),
    ])
}

/// One row of [`four_columns`], with the third column null.
pub(super) fn four_values() -> Vec<u8> {
    row_token(&[
        7i32.to_le_bytes().to_vec(),
        short_value(b"abc"),
        byte_value(&[]),
        long_value(b"hello"),
    ])
}

/// Every row of a stream, or the refusal it ended with.
pub(super) fn drain_rows(rows: &mut TdsRows<'_, Script>) -> io::Result<Vec<TdsRow>> {
    let mut out = Vec::new();
    while let Some(row) = rows.next_row()? {
        out.push(row);
    }
    Ok(out)
}

// ---- The RPC out, and the two tokens a procedure answers with: §§ 1 and 5 -

/// One argument of an RPC, read back off the request that carried it.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct SentParam {
    pub(super) by_ref: bool,
    pub(super) type_id: u8,
    /// The width the `TYPE_INFO` declared: `NO_LENGTH` is the `MAX` form.
    pub(super) declared: u16,
    /// The value as text, and `None` for the null of whichever form and for
    /// every argument that went out binary.
    pub(super) text: Option<String>,
    /// The value as the octets it was written as, for a `varbinary` argument
    /// alone: the form that has no text to read it back as.
    pub(super) octets: Option<Vec<u8>>,
}

/// Walks a request the way a server does: past its headers, then one
/// argument at a time until the message ends.
///
/// Deliberately a walk rather than a table of offsets — a parameter's extent
/// depends on the one before it, so an offset asserted by hand would pass on
/// a request whose *earlier* fields were the wrong width.
pub(super) fn sent_rpc(request: &[u8]) -> (u16, Vec<SentParam>) {
    assert_eq!(
        u32::from_le_bytes(request[0..4].try_into().unwrap()),
        ALL_HEADERS_BYTES
    );
    assert_eq!(
        u32::from_le_bytes(request[4..8].try_into().unwrap()),
        TRANSACTION_HEADER_BYTES
    );
    assert_eq!(
        u16::from_le_bytes([request[8], request[9]]),
        HEADER_TRANSACTION
    );
    assert_eq!(request[10..18], [0; 8], "no transaction descriptor");
    assert_eq!(u32::from_le_bytes(request[18..22].try_into().unwrap()), 1);
    assert_eq!(
        u16::from_le_bytes([request[22], request[23]]),
        PROC_ID_SWITCH
    );
    let proc_id = u16::from_le_bytes([request[24], request[25]]);
    assert_eq!(u16::from_le_bytes([request[26], request[27]]), 0);

    let mut at = usize::try_from(ALL_HEADERS_BYTES).expect("twenty-two") + 6;
    let mut params = Vec::new();
    while at < request.len() {
        assert_eq!(request[at], 0, "every argument goes out unnamed");
        let by_ref = request[at + 1] == PARAM_BY_REF;
        let type_id = request[at + 2];
        at += 3;
        let (declared, text, octets) = match type_id {
            TY_INTN => {
                let declared = u16::from(request[at]);
                let length = usize::from(request[at + 1]);
                at += 2 + length;
                let text = (length > 0).then(|| {
                    let bytes = &request[at - length..at];
                    i32::from_le_bytes(bytes.try_into().unwrap()).to_string()
                });
                (declared, text, None)
            }
            TY_NVARCHAR => {
                let declared = u16::from_le_bytes([request[at], request[at + 1]]);
                assert_eq!(request[at + 2..at + 7], NO_COLLATION);
                at += 7;
                let bytes = variable_body(request, &mut at, declared);
                let text = bytes.map(|bytes| {
                    let units: Vec<u16> = bytes
                        .chunks_exact(2)
                        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                        .collect();
                    String::from_utf16(&units).expect("what we wrote")
                });
                (declared, text, None)
            }
            // The same two body shapes with no `COLLATION` in front of them,
            // which is the whole difference on the way out as well.
            TY_BIGVARBINARY => {
                let declared = u16::from_le_bytes([request[at], request[at + 1]]);
                at += 2;
                let octets = variable_body(request, &mut at, declared);
                (declared, None, octets)
            }
            other => panic!("this driver sends no argument of type 0x{other:02X}"),
        };
        params.push(SentParam {
            by_ref,
            type_id,
            declared,
            text,
            octets,
        });
    }
    (proc_id, params)
}

/// One argument's body, read the way its `TYPE_INFO` declared it: `PLP` chunks
/// to the terminator at [`NO_LENGTH`], a `USHORTLEN` run otherwise, and `None`
/// for the null of whichever of the two.
///
/// Shared by the character and binary arms of [`sent_rpc`] because the two write
/// the same two shapes — a walk written twice would be two chances to assert a
/// value's extent wrongly.
fn variable_body(request: &[u8], at: &mut usize, declared: u16) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    if declared == NO_LENGTH {
        let total = u64::from_le_bytes(request[*at..*at + 8].try_into().unwrap());
        *at += 8;
        if total == PLP_NULL {
            return None;
        }
        loop {
            let chunk = usize::try_from(u32::from_le_bytes(
                request[*at..*at + 4].try_into().unwrap(),
            ))
            .unwrap();
            *at += 4;
            if chunk == 0 {
                break;
            }
            bytes.extend_from_slice(&request[*at..*at + chunk]);
            *at += chunk;
        }
        assert_eq!(
            u64::try_from(bytes.len()).unwrap(),
            total,
            "the declared total is the truth, not a sentinel"
        );
    } else {
        let length = usize::from(u16::from_le_bytes([request[*at], request[*at + 1]]));
        *at += 2;
        if length == usize::from(NO_LENGTH) {
            return None;
        }
        bytes.extend_from_slice(&request[*at..*at + length]);
        *at += length;
    }
    Some(bytes)
}

/// A `RETURNVALUE` as a server writes one: no length in front of it, so its
/// extent is the `TYPE_INFO` and nothing else.
pub(super) fn return_value_token(name: &str, handle: Option<i32>) -> Vec<u8> {
    let mut out = vec![TOKEN_RETURN_VALUE, 1, 0];
    out.extend_from_slice(&b_varchar(name));
    // `Status` — an RPC's output parameter — then `UserType` and `Flags`.
    out.push(0x01);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.push(TY_INTN);
    out.push(INTN_BYTES);
    match handle {
        None => out.push(0),
        Some(number) => {
            out.push(INTN_BYTES);
            out.extend_from_slice(&number.to_le_bytes());
        }
    }
    out
}

/// A `RETURNSTATUS`: four bytes and no length either.
pub(super) fn return_status_token(status: i32) -> Vec<u8> {
    let mut out = vec![TOKEN_RETURN_STATUS];
    out.extend_from_slice(&status.to_le_bytes());
    out
}

/// A described column of `id`, at `scale` for the four types that declare
/// one, named so a refusal has something to name.
pub(super) fn described(id: u8, scale: u8) -> TdsColumn {
    TdsColumn {
        name: String::from("c"),
        user_type: 0,
        flags: 0,
        type_info: TypeInfo {
            id,
            length: Length::Fixed(0),
            precision: 0,
            scale,
            collation: None,
        },
    }
}

/// An empty § 1 cache of `capacity` plans.
pub(super) fn plans(capacity: usize) -> StatementCache<TdsPlan> {
    StatementCache::new(capacity)
}

/// One `Wire` answering a *sequence* of requests, each answer a message of
/// its own — where [`answering`] builds one answer out of many packets.
pub(super) fn answering_each(answers: &[Vec<u8>]) -> Wire<Script> {
    let mut inbound = Vec::new();
    for payload in answers {
        inbound.extend_from_slice(&packet(PacketType::TabularResult, Status::EOM, 1, payload));
    }
    Wire::new(Script::answering(inbound, READ_CHUNK))
}

/// Every message the driver flushed, reassembled from the packets it wrote.
///
/// `Script` records one stream of bytes, and § 1's cache is asserted on the
/// *sequence* of requests rather than on any one of them: which procedure
/// went out, in what order, is the whole of what a hit and an eviction are.
/// Messages and not packets, because a `nvarchar(max)` value is longer than
/// the negotiated packet size and a request counted in packets would make a
/// long parameter look like three requests. The status kept is the **first**
/// packet's, which is where [`Status::RESET_CONNECTION`] rides.
pub(super) fn flushed(sent: &[u8]) -> Vec<(PacketType, Status, Vec<u8>)> {
    let mut out: Vec<(PacketType, Status, Vec<u8>)> = Vec::new();
    let mut at = 0;
    let mut open = false;
    while at < sent.len() {
        let length = usize::from(u16::from_be_bytes([sent[at + 2], sent[at + 3]]));
        let kind = PacketType::from_byte(sent[at]).expect("a type this driver writes");
        let status = Status(sent[at + 1]);
        let body = &sent[at + HEADER..at + length];
        if open {
            out.last_mut()
                .expect("a message these bytes continue")
                .2
                .extend_from_slice(body);
        } else {
            out.push((kind, status, body.to_vec()));
        }
        open = !status.contains(Status::EOM);
        at += length;
    }
    out
}

/// `sp_prepexec`'s answer for a statement with no result set: the inner
/// statement's own end, then the handle and the status, then the procedure's.
pub(super) fn prepexec_answer(handle: i32) -> Vec<u8> {
    let mut out = done_kind(TOKEN_DONE_IN_PROC, DONE_COUNT, 1);
    out.extend_from_slice(&return_value_token("@handle", Some(handle)));
    out.extend_from_slice(&return_status_token(0));
    out.extend_from_slice(&done_kind(TOKEN_DONE_PROC, 0, 0));
    out
}

/// What `sp_unprepare` answers: a status and the end of the procedure.
pub(super) fn procedure_answer() -> Vec<u8> {
    let mut out = return_status_token(0);
    out.extend_from_slice(&done_kind(TOKEN_DONE_PROC, 0, 0));
    out
}

/// The T-SQL one flushed `SQL_BATCH` message carried, past the `ALL_HEADERS`
/// every request writes.
pub(super) fn batch_text(body: &[u8]) -> String {
    let text = &body[ALL_HEADERS_BYTES as usize..];
    let units: Vec<u16> = text
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).expect("UCS-2 this driver wrote")
}

/// Every batch text the driver flushed, in order.
pub(super) fn batches(sent: &[u8]) -> Vec<String> {
    flushed(sent)
        .iter()
        .map(|(kind, _, body)| {
            assert_eq!(*kind, PacketType::SqlBatch, "§ 7's commands are batches");
            batch_text(body)
        })
        .collect()
}

/// A `DONE` for a command that counted nothing, which is what every one of
/// § 7's answers with.
pub(super) fn done() -> Vec<u8> {
    done_token(0, 0)
}

/// One answer for a transaction that began, committed or rolled back:
/// the `ENVCHANGE` and the `DONE` after it.
pub(super) fn transaction_answer(kind: u8, descriptor: Option<u64>) -> Vec<u8> {
    let mut value = Vec::new();
    match descriptor {
        // A begin: the eight octets as the new value, and no old one.
        Some(open) => {
            value.push(8);
            value.extend_from_slice(&open.to_le_bytes());
            value.push(0);
        }
        // A commit or a rollback: an empty new value, and the descriptor
        // that ended as the old one.
        None => {
            value.push(0);
            value.push(8);
            value.extend_from_slice(&1_u64.to_le_bytes());
        }
    }
    let mut body = vec![kind];
    body.extend_from_slice(&value);
    let mut answer = token(TOKEN_ENV_CHANGE, &body);
    answer.extend_from_slice(&done_token(0, 0));
    answer
}

/// The transaction a flushed request's `ALL_HEADERS` enlisted it in.
pub(super) fn header_descriptor(body: &[u8]) -> u64 {
    // `TotalLength`, then the one header's own length and type, then the
    // eight octets — the layout [`all_headers`] writes.
    let at = 4 + 4 + 2;
    u64::from_le_bytes(body[at..at + 8].try_into().expect("eight bytes"))
}
