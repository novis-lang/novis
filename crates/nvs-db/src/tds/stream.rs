//! [`Tokens`] — the parser that walks a response, and the login exchange
//! written on top of it.
//!
//! A token is routinely cut in half by a packet boundary, so this parses a
//! *slice* and reports how much of it was consumed: a caller that ran out
//! offers the same buffer again with more in it. That is why it is not an
//! `Iterator` — every step can fail, and a `for` loop over
//! `Iterator<Item = Result<_>>` would walk past a malformed token by ignoring
//! the item it was handed.

use super::*;

/// A reader over one message's tokens.
///
/// Sans-IO like the rest of this module: it borrows a payload and never touches
/// a stream, so every shape below is tested without a socket. The payload it is
/// given is one whole message — [`Wire::read_message`] — because a token is cut
/// at whatever offset the packet size lands on and this reader holds no
/// remainder of its own. The row path is the one that cannot afford that, and
/// it is [`TdsRows`], which holds the remainder itself and hands *this* reader
/// the tokens between the rows — see [`Tokens::consumed`].
///
/// Not an `Iterator`: every step can fail, and an `Iterator<Item = Result<_>>`
/// would let a `for` loop walk past a malformed token by ignoring the item it
/// was handed.
#[derive(Debug)]
pub struct Tokens<'a> {
    payload: &'a [u8],
    at: usize,
}

impl<'a> Tokens<'a> {
    /// A reader over one message's payload.
    #[must_use]
    pub const fn over(payload: &'a [u8]) -> Tokens<'a> {
        Tokens { payload, at: 0 }
    }

    /// The next token, or `None` where the message has ended.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a token this driver does not read yet — naming the
    /// byte, since that is the whole of what a reader can say about it — and
    /// for any token whose fields run past the end of the message.
    ///
    /// [`TOKEN_ORDER`] is read for its extent alone and answers
    /// [`Token::Order`], which every caller ignores.
    pub fn next_token(&mut self) -> io::Result<Option<Token>> {
        let Some(&kind) = self.payload.get(self.at) else {
            return Ok(None);
        };
        self.at += 1;

        let token = match kind {
            TOKEN_ERROR => Token::Error(self.server_message("ERROR")?),
            TOKEN_INFO => Token::Info(self.server_message("INFO")?),
            TOKEN_LOGIN_ACK => Token::LoginAck(self.login_ack()?),
            TOKEN_ENV_CHANGE => Token::Env(self.env_change()?),
            TOKEN_COL_METADATA => Token::Columns(self.columns()?),
            TOKEN_RETURN_STATUS => Token::ReturnStatus(self.signed("RETURNSTATUS")?),
            TOKEN_RETURN_VALUE => Token::ReturnValue(self.return_value()?),
            TOKEN_ORDER => {
                // Positioned by its own length rather than read column by
                // column, which is [`Tokens::bounded`]'s whole reason: nothing
                // here wants the ordinals, and the length is the one field that
                // says where the rows begin.
                self.at = self.bounded("ORDER")?;
                Token::Order
            }
            TOKEN_DONE | TOKEN_DONE_PROC | TOKEN_DONE_IN_PROC => {
                Token::Done(self.done(kind == TOKEN_DONE_IN_PROC)?)
            }
            other => {
                return Err(malformed(format!(
                    "a TDS response carried token 0x{other:02X}, which this driver does not read"
                )));
            }
        };
        Ok(Some(token))
    }

    /// How far the payload has been read, which is how many bytes the tokens
    /// answered so far occupied.
    ///
    /// [`TdsRows`] is the caller: it parses a token out of a buffer it filled
    /// from packets and needs to know what to drop off the front, and the token
    /// itself says nothing about its own extent — some carry a length, `DONE`
    /// is fixed, and `COLMETADATA` is neither.
    #[must_use]
    pub const fn consumed(&self) -> usize {
        self.at
    }

    /// `n` bytes, or the refusal that says which field ran off the end.
    fn take(&mut self, n: usize, what: &'static str) -> io::Result<&'a [u8]> {
        let Some(bytes) = self.payload.get(self.at..self.at + n) else {
            return Err(malformed(format!(
                "a TDS token's {what} wanted {n} byte(s) at offset {}, past the end of a {}-byte \
                 message",
                self.at,
                self.payload.len()
            )));
        };
        self.at += n;
        Ok(bytes)
    }

    /// One byte.
    fn byte(&mut self, what: &'static str) -> io::Result<u8> {
        Ok(self.take(1, what)?[0])
    }

    /// A little-endian `u16`.
    fn short(&mut self, what: &'static str) -> io::Result<u16> {
        let bytes = self.take(2, what)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// A little-endian `u32`.
    fn long(&mut self, what: &'static str) -> io::Result<u32> {
        let bytes = self.take(4, what)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// A little-endian `i32`.
    ///
    /// `RETURNSTATUS` is the one signed field this driver reads: a procedure
    /// returns whatever integer it likes, and the convention is that a negative
    /// one is a failure.
    fn signed(&mut self, what: &'static str) -> io::Result<i32> {
        let bytes = self.take(4, what)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// `characters` UCS-2LE characters, as a `String`.
    ///
    /// Lossy on an unpaired surrogate, which is the one thing a UCS-2 field can
    /// hold that Novis's own strings cannot (`rule:types/bytes`
    /// makes a `string` valid UTF-8). Refusing a message because the server's
    /// *prose* was ill-formed would turn a reportable error into an
    /// unreportable one.
    fn characters(&mut self, characters: usize, what: &'static str) -> io::Result<String> {
        let bytes = self.take(characters * 2, what)?;
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|two| u16::from_le_bytes([two[0], two[1]]))
            .collect();
        Ok(String::from_utf16_lossy(&units))
    }

    /// A `B_VARCHAR`: a one-byte character count, then the characters.
    fn b_varchar(&mut self, what: &'static str) -> io::Result<String> {
        let characters = usize::from(self.byte(what)?);
        self.characters(characters, what)
    }

    /// A `US_VARCHAR`: a two-byte character count, then the characters.
    fn us_varchar(&mut self, what: &'static str) -> io::Result<String> {
        let characters = usize::from(self.short(what)?);
        self.characters(characters, what)
    }

    /// A length-prefixed token's end, from the `u16` length that opens it.
    ///
    /// Every variable-length token below is parsed field by field and then
    /// *positioned* by this, rather than trusted to have consumed exactly the
    /// right number of bytes. That is what makes a field MS-TDS adds to the end
    /// of a token in a later version a thing this driver skips rather than
    /// reads as the next token's type byte.
    fn bounded(&mut self, what: &'static str) -> io::Result<usize> {
        let length = usize::from(self.short(what)?);
        let end = self.at + length;
        if end > self.payload.len() {
            return Err(malformed(format!(
                "a TDS {what} token claims {length} byte(s) at offset {}, past the end of a \
                 {}-byte message",
                self.at,
                self.payload.len()
            )));
        }
        Ok(end)
    }

    /// An `ERROR` or `INFO` token, which have one layout.
    fn server_message(&mut self, what: &'static str) -> io::Result<ServerMessage> {
        let end = self.bounded(what)?;
        let message = ServerMessage {
            number: self.long(what)?,
            state: self.byte(what)?,
            class: self.byte(what)?,
            message: self.us_varchar(what)?,
            server: self.b_varchar(what)?,
            procedure: self.b_varchar(what)?,
            line: self.long(what)?,
        };
        self.finish(end, what)?;
        Ok(message)
    }

    /// A `LOGINACK` token.
    fn login_ack(&mut self) -> io::Result<LoginAck> {
        let end = self.bounded("LOGINACK")?;
        // `Interface`, which says which of the two SQL dialects the server will
        // accept and is the same answer for every server this driver can talk
        // to at all.
        self.byte("LOGINACK")?;
        let ack = LoginAck {
            // **The reverse of the same field in LOGIN7**, which is the whole of
            // why this is four `byte` reads and not a `long`. A request writes
            // `TDS_VERSION` little-endian ([`login7_request`]) and a server
            // answers it high half first, so reading it the way it was written
            // yields `0x04000074` — a number no server speaks and, since
            // [`login`] compares it, a connection refused at the last step of a
            // handshake that otherwise worked.
            tds_version: u32::from_be_bytes([
                self.byte("LOGINACK")?,
                self.byte("LOGINACK")?,
                self.byte("LOGINACK")?,
                self.byte("LOGINACK")?,
            ]),
            program: self.b_varchar("LOGINACK")?,
            version: (
                self.byte("LOGINACK")?,
                self.byte("LOGINACK")?,
                // The build number is two bytes written high half first, the
                // other number here that is not little-endian.
                u16::from_be_bytes([self.byte("LOGINACK")?, self.byte("LOGINACK")?]),
            ),
        };
        self.finish(end, "LOGINACK")?;
        Ok(ack)
    }

    /// An `ENVCHANGE` token: the type, then a new value and an old one whose
    /// shape depends on it.
    fn env_change(&mut self) -> io::Result<EnvChange> {
        let end = self.bounded("ENVCHANGE")?;
        let kind = self.byte("ENVCHANGE")?;
        let change = match kind {
            ENV_DATABASE | ENV_LANGUAGE | ENV_CHARSET | ENV_PACKET_SIZE => {
                let to = self.b_varchar("ENVCHANGE")?;
                let from = self.b_varchar("ENVCHANGE")?;
                match kind {
                    ENV_DATABASE => EnvChange::Database { from, to },
                    ENV_LANGUAGE => EnvChange::Language { from, to },
                    ENV_CHARSET => EnvChange::Charset { from, to },
                    _ => EnvChange::PacketSize {
                        to: to.parse().map_err(|_| {
                            malformed(format!(
                                "a TDS ENVCHANGE answered the packet size with {to:?}, which is \
                                 not a size"
                            ))
                        })?,
                    },
                }
            }
            ENV_BEGIN_TRANSACTION | ENV_COMMIT_TRANSACTION | ENV_ROLLBACK_TRANSACTION => {
                EnvChange::Transaction {
                    to: self.transaction_descriptor()?,
                }
            }
            other => EnvChange::Other(other),
        };
        // Deliberately unconditional: an unread type is skipped by the token's
        // own length rather than by walking value halves this driver has no
        // parser for.
        self.at = end;
        Ok(change)
    }

    /// The `B_VARBYTE` new value of a transaction `ENVCHANGE`: eight octets for
    /// a begin, and empty for a commit or a rollback.
    ///
    /// The eight are opaque — this driver never reads a field out of them, only
    /// writes them back in the next request's `ALL_HEADERS` — so they are taken
    /// little-endian here and written little-endian there and the number in
    /// between is a handle rather than a quantity.
    ///
    /// **Any other length is refused rather than read as zero.** A descriptor
    /// this driver could not read is one every later request would name wrongly,
    /// and the server answers that with code 3989 on a statement the program
    /// wrote; a refusal at the token names the real fault at the message that
    /// carried it.
    fn transaction_descriptor(&mut self) -> io::Result<u64> {
        let octets = usize::from(self.byte("ENVCHANGE")?);
        match octets {
            0 => Ok(NO_TRANSACTION),
            8 => Ok(u64::from_le_bytes(
                self.take(8, "ENVCHANGE")?.try_into().expect("eight bytes"),
            )),
            other => Err(malformed(format!(
                "a TDS ENVCHANGE described an open transaction with {other} octets, and the \
                 descriptor every request after a BEGIN has to carry is the eight the protocol \
                 defines"
            ))),
        }
    }

    /// A `DONE`, `DONEPROC` or `DONEINPROC` token: twelve fixed bytes with no
    /// length in front of them.
    fn done(&mut self, in_proc: bool) -> io::Result<Done> {
        let status = self.short("DONE")?;
        let command = self.short("DONE")?;
        let rows = self.take(8, "DONE")?;
        Ok(Done {
            status,
            command,
            in_proc,
            rows: u64::from_le_bytes(rows.try_into().expect("eight bytes")),
        })
    }

    /// A `COLMETADATA` token: a column count, then that many descriptions in
    /// the order the values will arrive.
    ///
    /// The one token here that carries neither a length nor a fixed size, so a
    /// field that runs off the end is caught by [`Tokens::take`] rather than by
    /// [`Tokens::finish`]. That costs nothing here and is what lets the row
    /// path reuse this reader unchanged over a buffer it filled from packets:
    /// this parses bytes, and where they came from is not its question.
    ///
    /// The `CekTable` a column-encryption login would put between the count and
    /// the first column is deliberately not read. LOGIN7 does not ask for that
    /// feature ([`login7_request`]), so the field is never sent, and a parser for a
    /// shape this driver cannot receive is untestable by construction.
    fn columns(&mut self) -> io::Result<Vec<TdsColumn>> {
        let count = self.short("COLMETADATA")?;
        if count == NO_LENGTH {
            return Ok(Vec::new());
        }
        // Capped, because the count is the server's word and the columns are
        // not on the wire yet: SQL Server's own ceiling on a `SELECT` is 4,096,
        // and a claim past it grows the vector instead of preallocating for it.
        let mut columns = Vec::with_capacity(usize::from(count).min(4096));
        for _ in 0..count {
            let user_type = self.long("COLMETADATA")?;
            let flags = self.short("COLMETADATA")?;
            let type_info = self.type_info()?;
            if matches!(type_info.id, TY_TEXT | TY_NTEXT | TY_IMAGE) {
                // The `TableName` only those three carry: a part count, then
                // that many `US_VARCHAR`s naming the table the value is stored
                // in. Read to skip it — a text pointer is what fetches the
                // value — and field by field, since it is variable.
                let parts = self.byte("COLMETADATA")?;
                for _ in 0..parts {
                    self.us_varchar("COLMETADATA")?;
                }
            }
            columns.push(TdsColumn {
                name: self.b_varchar("COLMETADATA")?,
                user_type,
                flags,
                type_info,
            });
        }
        Ok(columns)
    }

    /// A `RETURNVALUE`: which output parameter it is, what type the server gave
    /// it, and its bytes.
    ///
    /// The `Flags` field is read and dropped. It describes the *column* an
    /// output parameter would have if it were one — nullable, updateable — and
    /// a value that came back has already answered the only question here,
    /// which is whether it is null.
    fn return_value(&mut self) -> io::Result<ReturnValue> {
        const WHAT: &str = "RETURNVALUE";

        let ordinal = self.short(WHAT)?;
        let name = self.b_varchar(WHAT)?;
        // `Status`: 0x01 for an RPC's output parameter and 0x02 for a
        // user-defined function's return value. Nothing here calls a UDF, and a
        // caller that did would read the same bytes either way.
        self.byte(WHAT)?;
        self.long(WHAT)?;
        self.short(WHAT)?;
        let type_info = self.type_info()?;
        let value = self.value(type_info)?;
        Ok(ReturnValue {
            ordinal,
            name,
            type_info,
            value,
        })
    }

    /// One value, measured by its own `TYPE_INFO`, or `None` for SQL `NULL`.
    ///
    /// [`TdsRows::value`]'s table over a payload that is all there, rather than
    /// over a buffer that refills — which is the whole of the difference, and
    /// the reason the two are not one function. That one copies into a row's
    /// bytes as chunks arrive so a `PLP` value never sits in the buffer twice;
    /// this one is reading a message [`Wire::read_message`] has already
    /// assembled, so there is no second copy to avoid. It is also the narrower
    /// job: a `RETURNVALUE` is the only token that reaches here, and § 1 asks
    /// for a four-byte handle.
    fn value(&mut self, info: TypeInfo) -> io::Result<Option<Vec<u8>>> {
        const WHAT: &str = "value";

        if info.id == TY_NULL {
            return Ok(None);
        }
        let length = match info.length {
            Length::Fixed(width) => width,
            Length::Byte(_) => match self.byte(WHAT)? {
                0 => return Ok(None),
                length => usize::from(length),
            },
            Length::Short(_) => match self.short(WHAT)? {
                NO_LENGTH => return Ok(None),
                length => usize::from(length),
            },
            Length::Long(_) => {
                if matches!(info.id, TY_TEXT | TY_NTEXT | TY_IMAGE) {
                    let pointer = self.byte(WHAT)?;
                    if pointer == 0 {
                        return Ok(None);
                    }
                    self.take(usize::from(pointer) + TEXT_TIMESTAMP, WHAT)?;
                }
                let declared = self.long(WHAT)?;
                if declared == NO_LENGTH_LONG || (declared == 0 && info.id == TY_VARIANT) {
                    return Ok(None);
                }
                as_usize(declared)?
            }
            Length::Partial => return self.partial(),
        };
        Ok(Some(self.take(length, WHAT)?.to_vec()))
    }

    /// A `PLP` value: a declared total or a sentinel, then chunks until an empty
    /// one, joined.
    ///
    /// The total is checked against what the chunks carried for
    /// [`TdsRows::partial_value`]'s reason — it is the server's word about bytes
    /// it had not sent yet, and a value that disagrees with it is a message this
    /// reader cannot prove its position in.
    fn partial(&mut self) -> io::Result<Option<Vec<u8>>> {
        const WHAT: &str = "PLP value";

        let bytes = self.take(8, WHAT)?;
        let total = u64::from_le_bytes(bytes.try_into().expect("eight bytes, as asked for"));
        if total == PLP_NULL {
            return Ok(None);
        }
        let mut out = Vec::new();
        loop {
            let chunk = self.long(WHAT)?;
            if chunk == 0 {
                break;
            }
            out.extend_from_slice(self.take(as_usize(chunk)?, WHAT)?);
        }
        let read = out.len() as u64;
        if total != PLP_UNKNOWN && read != total {
            return Err(malformed(format!(
                "a TDS PLP value declared {total} byte(s) and its chunks carried {read}"
            )));
        }
        Ok(Some(out))
    }

    /// A `TYPE_INFO`: the type byte, then whatever that type declares about
    /// itself.
    ///
    /// MS-TDS § 2.2.5.4.1's tables written as a `match`, and there is no
    /// shorter form of them: which [`Length`] family a type belongs to is a
    /// property of the byte and not of any range it falls in — `0x28` is
    /// measured by a byte while `0x27` two below it is a type from another
    /// decade.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a type byte this driver does not read, naming it. TDS
    /// 4.2's `CHAR` (0x2F), `VARCHAR` (0x27), `BINARY` (0x2D), `VARBINARY`
    /// (0x25), `DECIMAL` (0x37) and `NUMERIC` (0x3F) are among them on purpose:
    /// a 7.4 server sends the `BIG…`/`…N` spelling in every case, so an arm for
    /// one would be a guess at a layout this driver can never observe. Also for
    /// a fractional-second scale past the seven digits SQL Server's three time
    /// types hold, which is a width this reader would otherwise have to invent.
    fn type_info(&mut self) -> io::Result<TypeInfo> {
        const WHAT: &str = "TYPE_INFO";

        let id = self.byte(WHAT)?;
        let mut info = TypeInfo {
            id,
            length: Length::Fixed(0),
            precision: 0,
            scale: 0,
            collation: None,
        };
        match id {
            // A column with no type carries no value either, and the zero above
            // is already that.
            TY_NULL => {}
            TY_INT1 | TY_BIT => info.length = Length::Fixed(1),
            TY_INT2 => info.length = Length::Fixed(2),
            TY_INT4 | TY_DATETIME4 | TY_FLT4 | TY_MONEY4 => info.length = Length::Fixed(4),
            TY_MONEY | TY_DATETIME | TY_FLT8 | TY_INT8 => info.length = Length::Fixed(8),
            TY_GUID | TY_INTN | TY_BITN | TY_FLTN | TY_MONEYN | TY_DATETIMEN => {
                info.length = Length::Byte(self.byte(WHAT)?);
            }
            TY_DECIMALN | TY_NUMERICN => {
                info.length = Length::Byte(self.byte(WHAT)?);
                info.precision = self.byte(WHAT)?;
                info.scale = self.byte(WHAT)?;
            }
            // `date` declares nothing at all: three bytes is the only width it
            // has, and it is still measured by a byte because a null one is
            // measured as none.
            TY_DATEN => info.length = Length::Byte(DATE_BYTES),
            TY_TIMEN | TY_DATETIME2N | TY_DATETIMEOFFSETN => {
                let scale = self.byte(WHAT)?;
                info.scale = scale;
                let seconds = match scale {
                    0..=2 => 3,
                    3..=4 => 4,
                    5..=7 => 5,
                    past => {
                        return Err(malformed(format!(
                            "a TDS TYPE_INFO gave type 0x{id:02X} a scale of {past}, past the \
                             seven fractional-second digits SQL Server holds"
                        )));
                    }
                };
                info.length = Length::Byte(match id {
                    TY_TIMEN => seconds,
                    TY_DATETIME2N => seconds + DATE_BYTES,
                    _ => seconds + DATE_BYTES + OFFSET_BYTES,
                });
            }
            TY_BIGBINARY | TY_BIGVARBINARY => info.length = self.declared_length(WHAT)?,
            TY_BIGCHAR | TY_BIGVARCHAR | TY_NCHAR | TY_NVARCHAR => {
                info.length = self.declared_length(WHAT)?;
                info.collation = Some(self.collation(WHAT)?);
            }
            TY_IMAGE | TY_VARIANT => info.length = Length::Long(self.long(WHAT)?),
            TY_TEXT | TY_NTEXT => {
                info.length = Length::Long(self.long(WHAT)?);
                info.collation = Some(self.collation(WHAT)?);
            }
            TY_XML => {
                // `SchemaPresent`, then the three names a bound column carries.
                // Read to skip: an `xml` value arrives the same way either way.
                if self.byte(WHAT)? != 0 {
                    self.b_varchar(WHAT)?;
                    self.b_varchar(WHAT)?;
                    self.us_varchar(WHAT)?;
                }
                info.length = Length::Partial;
            }
            TY_UDT => {
                // The declared maximum, then the four names a CLR type is
                // identified by — none of which changes how its bytes are read.
                self.short(WHAT)?;
                self.b_varchar(WHAT)?;
                self.b_varchar(WHAT)?;
                self.b_varchar(WHAT)?;
                self.us_varchar(WHAT)?;
                info.length = Length::Partial;
            }
            other => {
                return Err(malformed(format!(
                    "a TDS TYPE_INFO described a value as type 0x{other:02X}, which this driver \
                     does not read"
                )));
            }
        }
        Ok(info)
    }

    /// A `USHORTLEN` type's declared width, which is [`Length::Partial`] where
    /// the type is a `MAX` one.
    fn declared_length(&mut self, what: &'static str) -> io::Result<Length> {
        let declared = self.short(what)?;
        Ok(if declared == NO_LENGTH {
            Length::Partial
        } else {
            Length::Short(declared)
        })
    }

    /// A `COLLATION`, kept as the five bytes the server wrote.
    fn collation(&mut self, what: &'static str) -> io::Result<[u8; 5]> {
        Ok(self
            .take(5, what)?
            .try_into()
            .expect("five bytes, as asked for"))
    }

    /// Positions the reader at a token's declared end, refusing one whose
    /// fields already read past it.
    fn finish(&mut self, end: usize, what: &'static str) -> io::Result<()> {
        if self.at > end {
            return Err(malformed(format!(
                "a TDS {what} token's fields read {} byte(s) past the length it declared",
                self.at - end
            )));
        }
        self.at = end;
        Ok(())
    }
}

/// What [`ServerError::backend`] calls this one, and what its rendered sentence
/// therefore opens with.
///
/// The `[db.<name>]` block's own spelling, so an operator reading a refusal and
/// an operator reading the configuration that produced it are reading one word
/// — and deliberately not [`Driver::matrix_name`], which is a harness key
/// [`ServerError::backend`]'s own doc refuses to tie this to.
pub(super) const BACKEND: &str = "sqlserver";

/// The server's refusal, as the `io::Error` every entry point here answers
/// with.
///
/// [`crate::mysql`]'s `server_refusal` and [`crate::pg`]'s `server_error` with
/// this backend's two absences. TDS sends **no `SQLSTATE`**, so
/// [`ServerError::sql_state`] is empty — that field's own doc owns what the
/// empty string means, and inventing ODBC's five characters here would fill the
/// slot a program reads with a value this driver made up, which is what
/// [`kind_of`] refuses to key its table on for the same reason. And its
/// severity is a number rather than a word, so [`ServerMessage::is_fatal`] is
/// what picks between the two words [`ServerError::severity`] is documented to
/// carry.
///
/// `constraint` is `None` on every refusal this backend words: SQL Server names
/// the index or the key inside the sentence and in no field of its own, and
/// § 8's rule against matching on message text is exactly the rule that stops
/// this driver taking it from there.
pub(crate) fn server_refusal(message: &ServerMessage) -> io::Error {
    io::Error::other(ServerError {
        kind: message.kind(),
        sql_state: String::new(),
        severity: String::from(if message.is_fatal() { "FATAL" } else { "ERROR" }),
        message: message.message.clone(),
        constraint: None,
        driver_code: Some(message.number),
        backend: BACKEND,
    })
}

/// LOGIN7 out over an encrypted wire, and the token stream that answers it read
/// to its end.
///
/// A free function generic in the stream for [`negotiate_tls`]'s reason: a
/// method on [`TdsConn`] could only be reached through a real socket and a real
/// certificate, and this is the half worth a unit test.
///
/// **The whole answer is read before anything acts on it.** The packet size an
/// `ENVCHANGE` announces is in force from the packet *after* the one that
/// carried it, and a refused login sends its `ERROR` before the `DONE` that
/// ends the message, so a reader that applied either mid-stream would be acting
/// on a message it had not finished. The first `ERROR` is the one kept: what
/// follows it is the server describing the consequences of the first, and the
/// `DONE` that ends a failed login says only that something failed.
///
/// The [`LoginAck`] is answered because its version is what the connection
/// keeps for `Core\Db\Connection::serverVersion`
/// ([`LoginAck::server_version`]). Its dialect is not the caller's to act on
/// and is checked here: a server answering a version other than
/// [`TDS_VERSION`] is one this driver would need a second set of readers for,
/// and reading its tokens as 7.4's would be guessing at the bytes rather than
/// refusing them.
///
/// # Errors
///
/// [`login7_request`]'s `InvalidInput` for a field past [`MAX_FIELD_CHARS`]; an
/// `Other` carrying a [`ServerError`] for the login the server refused —
/// [`DbErrorKind::Permission`] for the usual one, since `18456` is what a wrong
/// password, an unknown login and a database this login may not open all
/// arrive as; `InvalidData` for an answer that is not a token stream, for one
/// that neither accepts nor refuses, for a dialect this driver does not speak
/// and for a packet size outside [`Codec::set_packet_size`]'s range; and
/// whatever the stream reported.
pub fn login<S: Read + Write>(wire: &mut Wire<S>, target: &TdsTarget<'_>) -> io::Result<LoginAck> {
    let request = login7_request(target, wire.framing().packet_size())?;
    wire.send(PacketType::Login7, Status::NORMAL, &request)?;

    let answer = wire.read_message()?;
    if answer.kind != PacketType::TabularResult {
        return Err(malformed(format!(
            "a LOGIN7 was answered with a packet of type 0x{:02X}",
            answer.kind.byte()
        )));
    }

    let mut tokens = Tokens::over(&answer.payload);
    let mut refusal = None;
    let mut ack = None;
    let mut packet_size = None;
    while let Some(token) = tokens.next_token()? {
        match token {
            Token::Error(message) if refusal.is_none() => refusal = Some(message),
            Token::LoginAck(answered) => ack = Some(answered),
            Token::Env(EnvChange::PacketSize { to }) => packet_size = Some(to),
            // Exhaustive rather than a wildcard, so a token this driver learns
            // to read is a decision here and not a silent omission. A login
            // answers with no result set, so `Columns` is one of them.
            Token::Error(_)
            | Token::Info(_)
            | Token::Env(_)
            | Token::Columns(_)
            | Token::ReturnStatus(_)
            | Token::ReturnValue(_)
            | Token::Order
            | Token::Done(_) => {}
        }
    }

    if let Some(message) = refusal {
        return Err(server_refusal(&message));
    }
    let Some(ack) = ack else {
        return Err(malformed(String::from(
            "a LOGIN7 was answered without either a LOGINACK or an ERROR",
        )));
    };
    if ack.tds_version != TDS_VERSION {
        return Err(malformed(format!(
            "the server answered a TDS 7.4 login speaking 0x{:08X}, which this driver has no readers for",
            ack.tds_version
        )));
    }
    if let Some(size) = packet_size {
        wire.codec().set_packet_size(size)?;
    }
    Ok(ack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    #[test]
    fn a_login_sends_login7_and_the_servers_answer_settles_the_framing() {
        let block = block();
        let target = TdsTarget::resolve(&block).expect("a complete block resolves");

        let mut payload = env_token(ENV_DATABASE, "novis_test", "master");
        payload.extend_from_slice(&env_token(ENV_PACKET_SIZE, "8192", "4096"));
        payload.extend_from_slice(&login_ack_token());
        payload.extend_from_slice(&done_token(0, 0));
        let mut wire = logging_in(&payload);

        let ack = login(&mut wire, &target).expect("a login this server accepted");
        assert_eq!(ack.tds_version, TDS_VERSION);
        assert_eq!(ack.version, (16, 0, 4035));

        assert_eq!(
            wire.peer().sent,
            packet(PacketType::Login7, Status::EOM, 1, &login7(&block)),
            "what went out is the message `login7_request` built, framed whole"
        );
        assert_eq!(
            wire.framing().packet_size(),
            8192,
            "the ENVCHANGE is the only place a server says what size it will \
             actually use, so the framing follows it rather than what LOGIN7 asked"
        );
    }

    /// § 8's fields for a refusal this backend words, all of them at once.
    ///
    /// Both absences are asserted here rather than left to a reader's
    /// assumption: TDS sends no `SQLSTATE`, and no constraint name outside the
    /// sentence. A driver that filled either from ODBC's table or from the
    /// message text would still answer the right kind and would fail here.
    #[test]
    fn a_refused_login_is_the_servers_own_refusal_with_its_number_and_no_sqlstate() {
        let block = block();
        let target = TdsTarget::resolve(&block).expect("a complete block resolves");

        let mut payload = message_token(TOKEN_ERROR, 18456, 14, "Login failed for user 'novis'.");
        payload.extend_from_slice(&done_token(DONE_ERROR, 0));
        let mut wire = logging_in(&payload);

        let refused = login(&mut wire, &target).expect_err("this login was refused");
        let server = ServerError::of(&refused).expect("a refusal the server worded");
        assert_eq!(server.kind, DbErrorKind::Permission);
        assert_eq!(
            server.driver_code,
            Some(18456),
            "the number is the whole of what a caller branches on past § 8's kind"
        );
        assert_eq!(server.sql_state, "", "TDS has no such field to fill");
        assert_eq!(server.severity, "ERROR");
        assert_eq!(server.constraint, None);
        assert_eq!(
            refused.to_string(),
            "sqlserver ERROR: Login failed for user 'novis'.",
            "an absent SQLSTATE is omitted from the sentence, not rendered empty"
        );

        // A `THROW` past a `u16`, which is why this field is wider than one:
        // the whole user-defined range sits above that width, so a narrower
        // field would answer `None` for every error an application raised
        // itself.
        let mut raised = message_token(TOKEN_ERROR, 90_001, 16, "the application said no");
        raised.extend_from_slice(&done_token(DONE_ERROR, 0));
        let mut wire = logging_in(&raised);
        let refused = login(&mut wire, &target).expect_err("this login was refused");
        let server = ServerError::of(&refused).expect("a refusal the server worded");
        assert_eq!(server.driver_code, Some(90_001));
        assert_eq!(server.kind, DbErrorKind::Other);

        // At `FATAL_CLASS` the server has already closed the socket, so § 13's
        // pool must destroy this connection rather than reset it — which is
        // what the kind says and what the severity has to agree with.
        let mut fatal = message_token(TOKEN_ERROR, 9001, 21, "the log is not available");
        fatal.extend_from_slice(&done_token(DONE_ERROR, 0));
        let mut wire = logging_in(&fatal);
        let refused = login(&mut wire, &target).expect_err("this login was refused");
        let server = ServerError::of(&refused).expect("a refusal the server worded");
        assert_eq!(server.kind, DbErrorKind::ConnectionLost);
        assert_eq!(server.severity, "FATAL");
    }

    /// The version a connection keeps is the one `LOGINACK` sent, spelled the
    /// one way [ADR 0187 § 2](/docs/decisions/0187.md) fixes it, and bought
    /// with no statement: the login is the whole exchange.
    ///
    /// The fixture is what a SQL Server 2022 answers, down to the byte order of
    /// the build number, so a driver reading the triple the way LOGIN7 wrote it
    /// answers a version no server has.
    #[test]
    fn tds_keeps_the_version_its_loginack_sent() {
        let block = block();
        let target = TdsTarget::resolve(&block).expect("a complete block resolves");

        let mut accepted = login_ack_token();
        accepted.extend_from_slice(&done_token(0, 0));
        let mut wire = logging_in(&accepted);
        let ack = login(&mut wire, &target).expect("this login was accepted");

        assert_eq!(ack.server_version(), "16.0.4035");
        assert_eq!(ack.program, "Microsoft SQL Server");
    }

    /// The answers that are neither an acceptance nor a refusal.
    ///
    /// Each is a stream this driver could read *something* out of and must not:
    /// a login that ended with no verdict, a server speaking another dialect,
    /// and an answer that is not a token stream at all. The first is the one a
    /// reader trusting `LOGINACK`'s absence to mean failure would get wrong.
    #[test]
    fn a_login_answer_that_decides_nothing_is_refused_rather_than_assumed() {
        let block = block();
        let target = TdsTarget::resolve(&block).expect("a complete block resolves");

        let mut nothing = message_token(TOKEN_INFO, 5701, 0, "Changed database context.");
        nothing.extend_from_slice(&done_token(0, 0));
        let mut wire = logging_in(&nothing);
        let refused = login(&mut wire, &target).expect_err("nothing here accepted the login");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("LOGINACK"), "{refused}");
        assert!(
            ServerError::of(&refused).is_none(),
            "the server refused nothing, so this is a protocol failure and not \
             a condition § 8 normalises"
        );

        // TDS 7.3, which a server before SQL Server 2012 answers with. Its
        // token stream is close enough to read wrongly and not close enough to
        // read right.
        // The version is the four bytes after the token byte, its length and
        // the interface byte — `login_ack_token` is what says so, including the
        // high-half-first order this field alone arrives in.
        let mut older = login_ack_token();
        older[4..8].copy_from_slice(&0x730B_0003u32.to_be_bytes());
        older.extend_from_slice(&done_token(0, 0));
        let mut wire = logging_in(&older);
        let refused = login(&mut wire, &target).expect_err("this driver speaks 7.4 alone");
        assert!(refused.to_string().contains("0x730B0003"), "{refused}");

        // The server answering a LOGIN7 with a PRELOGIN: the stream is out of
        // sync, and the tokens would be read out of whatever arrived instead.
        let mut wire = Wire::new(Script::answering(
            packet(PacketType::PreLogin, Status::EOM, 1, &[PL_TERMINATOR]),
            READ_CHUNK,
        ));
        let refused = login(&mut wire, &target).expect_err("0x12 answers nothing here");
        assert!(refused.to_string().contains("type 0x12"), "{refused}");
    }
}
