//! The response stream's vocabulary: which tokens this driver reads, what each
//! one carries, and how a server's error number normalises to a
//! [`DbErrorKind`].
//!
//! The types here are what a parse produces; [`mod@super::stream`] is what does
//! the parsing. Keeping the two apart is what lets [`kind_of`]'s table — the
//! numbers [ADR 0067 § 10](/docs/decisions/0067.md) maps to
//! throwable classes — be read without the parser around it.

use super::*;

/// `ERROR`: the server refused something, and said which condition in a number
/// [`kind_of`] normalises.
pub(super) const TOKEN_ERROR: u8 = 0xAA;
/// `INFO`: the same layout with nothing refused. A login collects several of
/// them for a change of database context alone, so a reader that did not know
/// the token would fail on an ordinary success.
pub(super) const TOKEN_INFO: u8 = 0xAB;
/// `LOGINACK`: the login succeeded, and this is what the server is.
pub(super) const TOKEN_LOGIN_ACK: u8 = 0xAD;
/// `ENVCHANGE`: one session property moved, and both values are given.
pub(super) const TOKEN_ENV_CHANGE: u8 = 0xE3;
/// `COLMETADATA`: the shape of the rows that follow, one [`TdsColumn`] each.
pub(super) const TOKEN_COL_METADATA: u8 = 0x81;
/// `ROW`: one row, every column's value in `COLMETADATA`'s order.
///
/// Read by [`TdsRows`] and refused by [`Tokens`], which is not a gap in the
/// second: a row is the one token whose extent cannot be known without the
/// column list, so a reader holding a message and nothing else has no way to
/// step over one.
pub(super) const TOKEN_ROW: u8 = 0xD1;
/// `NBCROW`: a [`TOKEN_ROW`] whose null columns are a bitmap in front of the
/// values instead of a length each.
pub(super) const TOKEN_NBC_ROW: u8 = 0xD2;
/// `RETURNSTATUS`: the integer a stored procedure returned, which only an RPC
/// can produce.
///
/// `sp_prepexec` answers `0` for a statement it prepared and executed, and a
/// non-zero value for one it refused — which the `ERROR` beside it has already
/// said, in a sentence [`kind_of`] can normalise. So this is read to step over
/// it rather than acted on.
pub(super) const TOKEN_RETURN_STATUS: u8 = 0x79;
/// `RETURNVALUE`: one of a procedure's output parameters, coming back.
///
/// § 1's whole reason for calling `sp_prepexec` rather than `sp_executesql`:
/// the handle the server allocated for the statement arrives in one of these,
/// and the statement cache's key maps to it.
pub(super) const TOKEN_RETURN_VALUE: u8 = 0xAC;
/// `ORDER`: which columns the result set that follows is ordered by.
///
/// Sent for any statement carrying an `ORDER BY`, which every catalog query in
/// [`crate::catalog`] does — so a reader that refused it could not introspect a
/// SQL Server database at all. Read to **step over**: the ordinals it carries
/// say what the query already said, and nothing above this module asks a result
/// set what it was sorted by. That is why it produces no [`Token`] rather than a
/// variant every caller would match and ignore.
pub(super) const TOKEN_ORDER: u8 = 0xA9;
/// `DONE`: the end of one statement's answer.
pub(super) const TOKEN_DONE: u8 = 0xFD;
/// `DONEPROC`: `DONE` for a stored procedure, which § 13's `sp_reset_connection`
/// and § 1's `sp_prepexec` both are.
pub(super) const TOKEN_DONE_PROC: u8 = 0xFE;
/// `DONEINPROC`: `DONE` for one statement *inside* a procedure, and the one of
/// the three that is never the last token of a message.
pub(super) const TOKEN_DONE_IN_PROC: u8 = 0xFF;

/// `DONE`'s `Status`: another set of results follows this one.
pub(super) const DONE_MORE: u16 = 0x0001;
/// `DONE`'s `Status`: the statement this ends did not complete.
pub(super) const DONE_ERROR: u16 = 0x0002;
/// `DONE`'s `Status`: the row count means something. Without it the field is a
/// number the server did not intend to report, which is not the same as zero.
pub(super) const DONE_COUNT: u16 = 0x0010;

/// `ENVCHANGE` type 1: the database this session is in.
pub(super) const ENV_DATABASE: u8 = 1;
/// `ENVCHANGE` type 2: the session's language.
pub(super) const ENV_LANGUAGE: u8 = 2;
/// `ENVCHANGE` type 3: the session's character set, which TDS 7.4 does not use.
pub(super) const ENV_CHARSET: u8 = 3;
/// `ENVCHANGE` type 4: the packet size, as decimal digits rather than a number.
pub(super) const ENV_PACKET_SIZE: u8 = 4;
/// `ENVCHANGE` type 8: a transaction began, and the value is the descriptor
/// every request after it has to name it by.
pub(super) const ENV_BEGIN_TRANSACTION: u8 = 8;
/// `ENVCHANGE` type 9: a transaction committed.
pub(super) const ENV_COMMIT_TRANSACTION: u8 = 9;
/// `ENVCHANGE` type 10: a transaction rolled back, which is type 9 as far as
/// this driver is concerned — either way there is no open transaction left for
/// the next request to enlist in.
pub(super) const ENV_ROLLBACK_TRANSACTION: u8 = 10;

/// The severity at which SQL Server ends the connection rather than the
/// statement.
///
/// Documented as such by the server and not a driver convention: at 20 and
/// above the server closes the socket, so the read after one is an end of file
/// whatever this driver decides. [`kind_of`] answers
/// [`DbErrorKind::ConnectionLost`] for any unnamed number at this class for
/// that reason — the connection really is gone, and § 13's pool must destroy it
/// rather than reset it.
pub(super) const FATAL_CLASS: u8 = 20;

/// The § 8 kind a SQL Server error number means.
///
/// **Keyed on the number alone**, where [`crate::pg`]'s table is keyed on the
/// `SQLSTATE` and [`crate::mysql`]'s reads one as a fallback: TDS has no
/// `SQLSTATE` field at all. The five characters PDO reports for this backend
/// are ODBC's invention, mapped from the number by the driver, so a table keyed
/// on them here would be keyed on a value this driver had to make up first.
///
/// The rows of it worth arguing:
///
/// - **`547` is both a foreign key and a `CHECK`**, and SQL Server merges them
///   into one number on purpose — only the message text separates them, and
///   matching on message text is the thing § 8 exists to stop. It normalises as
///   [`DbErrorKind::ForeignKeyViolation`], the far commoner reading, and
///   § 8's `driverCode` still carries the number for a caller that needs the
///   difference. [`DbErrorKind::CheckViolation`] is therefore **unreachable on
///   this backend**, which is exactly what § 8 means when it says some
///   boundaries are driver-dependent.
/// - **`4060` is a `Permission`**, not a `Syntax` the way MySQL's "unknown
///   database" is. SQL Server deliberately answers a database that does not
///   exist and one this login may not open with the same refusal, so the only
///   half that is always true of it is that the login could not open it. It is
///   also the refusal [`OPT1_INIT_DB_FATAL`] asks for: with the other reading
///   of that bit this arrives as a warning and the connection opens somewhere
///   else.
///
/// Everything unnamed is [`DbErrorKind::Other`] rather than a guess, as on the
/// other drivers, except that a class of [`FATAL_CLASS`] or more is
/// [`DbErrorKind::ConnectionLost`] whatever the number: at that severity the
/// server has already closed the socket.
pub(crate) fn kind_of(number: u32, class: u8) -> DbErrorKind {
    match number {
        // A unique index and a primary key, which are two numbers for one
        // condition.
        2601 | 2627 => DbErrorKind::UniqueViolation,
        547 => DbErrorKind::ForeignKeyViolation,
        515 => DbErrorKind::NotNullViolation,
        // The deadlock victim: SQL Server rolled this transaction back whole,
        // so § 7's callable re-runs from nothing.
        1205 => DbErrorKind::Deadlock,
        // Snapshot isolation could not serialise this transaction against a
        // concurrent one, and aborted it.
        3960 | 3961 => DbErrorKind::SerializationFailure,
        // A lock wait that ran out, which is `Timeout` and **not** `Deadlock`
        // for `crate::mysql`'s reason: nothing was rolled back, so re-running
        // the callable would run its earlier statements again inside a
        // transaction that is still open.
        1222 => DbErrorKind::Timeout,
        // A malformed statement, and the "no such thing" numbers § 8 makes one
        // kind with it: an undefined object and a syntax error are the same bug
        // to a caller.
        102 | 156 | 207 | 208 | 2812 | 4104 => DbErrorKind::Syntax,
        // Denied: to the server, to a database, to an object, to a column, or
        // at the login itself.
        229 | 230 | 262 | 300 | 916 | 4060 | 18456 => DbErrorKind::Permission,
        _ if class >= FATAL_CLASS => DbErrorKind::ConnectionLost,
        _ => DbErrorKind::Other,
    }
}

/// An `ERROR` or `INFO` token: one thing the server has to say.
///
/// The same fields carry both, which is the protocol's doing and not a
/// convenience taken here — [`Token::Error`] and [`Token::Info`] are what say
/// whether anything was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerMessage {
    /// The server's own error number, which is § 8's `driverCode` and the key
    /// [`kind_of`] reads.
    pub number: u32,
    /// Which of the places that raise this number raised it. Not normalised by
    /// anything: it is what a support article asks for.
    pub state: u8,
    /// The severity. Above 10 is a refusal, and [`FATAL_CLASS`] or more is one
    /// that took the connection with it.
    pub class: u8,
    /// The server's own sentence, in the language its login default chose.
    pub message: String,
    /// Which server said it, as that server knows its own name.
    pub server: String,
    /// The procedure it was raised in, empty for a statement sent directly.
    pub procedure: String,
    /// The line within that procedure or batch.
    pub line: u32,
}

impl ServerMessage {
    /// Whether this ended the connection as well as the statement.
    #[must_use]
    pub const fn is_fatal(&self) -> bool {
        self.class >= FATAL_CLASS
    }

    /// § 8's kind for this message, from [`kind_of`].
    #[must_use]
    pub fn kind(&self) -> DbErrorKind {
        kind_of(self.number, self.class)
    }
}

/// A `LOGINACK` token: the login succeeded, and this is what answered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAck {
    /// The TDS version the server will speak, which is the one thing here a
    /// driver could act on. It should be the [`TDS_VERSION`] LOGIN7 asked for;
    /// a server entitled to answer with a lower one is a server this driver
    /// would have to have a second dialect for.
    pub tds_version: u32,
    /// What the server calls its own program — `Microsoft SQL Server`.
    pub program: String,
    /// Its major, minor and build numbers, which is the version an operator
    /// reads.
    pub version: (u8, u8, u16),
}

impl LoginAck {
    /// The version `Core\Db\Connection::serverVersion` answers on this driver:
    /// `major.minor.build`, decimal and unpadded — `16.0.4035`.
    ///
    /// [ADR 0187 § 2](/docs/decisions/0187.md) fixes that spelling here because
    /// SQL Server is the one backend sending numbers where the others send a
    /// string, and a driver inventing a rendering of its own would leave a
    /// program comparing versions across drivers comparing two shapes.
    #[must_use]
    pub fn server_version(&self) -> String {
        let (major, minor, build) = self.version;
        format!("{major}.{minor}.{build}")
    }
}

/// An `ENVCHANGE` token: one session property, before and after.
///
/// The variants are the ones TDS 7.4 spells as text, plus the one it spells as
/// bytes that this driver cannot do without: the transaction descriptor, which
/// [`EnvChange::Transaction`] owns. Everything else — collation, the routing
/// answer an Azure failover sends — is [`EnvChange::Other`] carrying its type
/// byte, because the value halves of those are bytes rather than characters and
/// nothing here has a use for them yet. They are skipped by the token's own
/// declared length, so an unread type never costs the reader its place in the
/// stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvChange {
    /// The database this session is in. Novis reads it because § 13's pool key
    /// is a promise about which database a pooled connection is in, and a `USE`
    /// is the one thing that could break it.
    Database {
        /// What it was.
        from: String,
        /// What it now is.
        to: String,
    },
    /// The session's language.
    Language {
        /// What it was.
        from: String,
        /// What it now is.
        to: String,
    },
    /// The session's character set, which a TDS 7.4 server does not send.
    Charset {
        /// What it was.
        from: String,
        /// What it now is.
        to: String,
    },
    /// The packet size, which is the answer to what LOGIN7 asked for and is
    /// what [`Codec::set_packet_size`] takes.
    PacketSize {
        /// The size in force from the *next* packet onwards.
        to: u16,
    },
    /// The transaction this session's requests must now enlist in, which the
    /// server sends on a begin, a commit and a rollback alike.
    ///
    /// **Not optional to read.** Once a transaction is open, a request whose
    /// `ALL_HEADERS` names a different descriptor than this one is refused with
    /// driver code 3989 — *new request is not allowed to start because it
    /// should come with valid transaction descriptor* — so a driver that
    /// skipped this token could open a transaction and never run a statement
    /// inside it. It is the one `ENVCHANGE` whose value is bytes that this
    /// driver has a use for, and [`Wire::descriptor`] is where the answer is
    /// kept.
    Transaction {
        /// The descriptor now in force, and **zero for none**: a commit and a
        /// rollback send an empty value, which is the protocol's way of saying
        /// there is no transaction left to name.
        to: u64,
    },
    /// A type this driver does not read, by its type byte.
    Other(u8),
}

/// One token of a response stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// The server refused something: [`ServerMessage::kind`] is § 8's kind.
    Error(ServerMessage),
    /// The server said something and refused nothing.
    Info(ServerMessage),
    /// The login succeeded.
    LoginAck(LoginAck),
    /// A session property moved.
    Env(EnvChange),
    /// The shape of the rows that follow, in the order the server sends their
    /// values.
    ///
    /// Empty where the server answered `0xFFFF` — *no metadata*, which is what
    /// a statement with no result set at all sends. A reader has nothing to
    /// distinguish that from a result set of no columns, which no server sends,
    /// so the two are one case here rather than a variant that is never taken.
    Columns(Vec<TdsColumn>),
    /// A procedure returned, and this is what it returned —
    /// [`TOKEN_RETURN_STATUS`] owns why nothing acts on the number.
    ReturnStatus(i32),
    /// One of a procedure's output parameters came back.
    ReturnValue(ReturnValue),
    /// The result set that follows is ordered — [`TOKEN_ORDER`] owns why
    /// nothing reads which columns it is ordered by.
    ///
    /// A variant carrying nothing rather than no variant at all: the walk has
    /// to *stop* on this token, because what follows it is a row, and a row is
    /// the one token [`Tokens`] cannot step over.
    Order,
    /// A statement's answer ended.
    Done(Done),
}

/// A `RETURNVALUE` token: one output parameter of the procedure that was
/// called, named as the procedure declares it.
///
/// The value is owned rather than borrowed, unlike every other token's bytes,
/// because [`TdsRows`] parses its tokens out of a buffer it then refills over —
/// and this is the one token whose *payload* a caller keeps past the parse. It
/// is at most a handle: § 1 calls one procedure and asks for one integer back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnValue {
    /// Which parameter of the call this is, one-based, as the server counted
    /// it. Read for a caller that asked for more than one output parameter;
    /// § 1 asks for a single one and reads [`ReturnValue::as_i32`] instead.
    pub ordinal: u16,
    /// The parameter's name as the procedure declares it — `@handle` for
    /// `sp_prepexec`'s. A caller that sent the parameter unnamed still gets
    /// this, since it is the *procedure's* spelling and not the request's.
    pub name: String,
    /// What the server said the value's type is.
    pub type_info: TypeInfo,
    /// The value's bytes, or `None` for SQL `NULL` — which is what an output
    /// parameter a procedure never assigned comes back as.
    pub value: Option<Vec<u8>>,
}

impl ReturnValue {
    /// The value as a four-byte little-endian integer, or `None` where it is
    /// null or is not four bytes wide.
    ///
    /// The one shape § 1 reads: `sp_prepexec` hands its handle back as an
    /// `INTN` of four bytes. Narrower is not silently widened — a server that
    /// answered a two-byte handle is one this driver has no account of, and
    /// reading it as a number would be inventing the two bytes it did not send.
    #[must_use]
    pub fn as_i32(&self) -> Option<i32> {
        let bytes: [u8; 4] = self.value.as_deref()?.try_into().ok()?;
        Some(i32::from_le_bytes(bytes))
    }
}

/// A `DONE`, `DONEPROC` or `DONEINPROC` token: one statement's answer ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Done {
    /// The status bits, read through the methods below rather than matched on
    /// directly.
    pub status: u16,
    /// Which command this ends, which no caller here reads: the token stream is
    /// already in order.
    pub command: u16,
    /// Whether this was a `DONEINPROC` — one statement *inside* a procedure
    /// ending, rather than the answer.
    ///
    /// Not a status bit, which is why it is a field beside them rather than a
    /// method over `status`: the three ends are three token *bytes*, and only
    /// this one is guaranteed to have something behind it. A reader that
    /// stopped on it would end an `sp_prepexec` answer before the
    /// `RETURNVALUE` carrying § 1's handle, which is sent after the rows.
    pub in_proc: bool,
    /// The row count, meaningful only where [`Done::counted`] says so.
    pub rows: u64,
}

impl Done {
    /// Whether another statement's answer follows this one in the same stream.
    #[must_use]
    pub const fn more(self) -> bool {
        self.status & DONE_MORE != 0
    }

    /// Whether the statement this ends failed.
    ///
    /// The bit is the *only* place a failure is reported for a statement whose
    /// `ERROR` token a caller chose not to keep — a reader that watched for
    /// `ERROR` alone and then trusted the `DONE` would report a rolled-back
    /// batch as a success.
    #[must_use]
    pub const fn failed(self) -> bool {
        self.status & DONE_ERROR != 0
    }

    /// Whether [`Done::rows`] is a count the server meant to report.
    ///
    /// Without the bit the field is whatever the server left there, which is
    /// not the same as zero: `execute` answering 0 for a statement that
    /// affected rows and never counted them would be a wrong number rather than
    /// a missing one.
    #[must_use]
    pub const fn counted(self) -> bool {
        self.status & DONE_COUNT != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    #[test]
    fn a_login_answer_reads_back_as_the_tokens_the_server_wrote() {
        let mut payload = env_token(ENV_DATABASE, "novis_test", "master");
        payload.extend_from_slice(&message_token(
            TOKEN_INFO,
            5701,
            0,
            "Changed database context to 'novis_test'.",
        ));
        payload.extend_from_slice(&env_token(ENV_PACKET_SIZE, "8192", "4096"));
        payload.extend_from_slice(&login_ack_token());
        payload.extend_from_slice(&done_token(DONE_COUNT, 0));

        let read = tokens(&payload).expect("a login answer this driver can read");
        assert_eq!(read.len(), 5);
        assert_eq!(
            read[0],
            Token::Env(EnvChange::Database {
                from: "master".to_owned(),
                to: "novis_test".to_owned(),
            }),
            "OptionFlags1's fUseDB is what asks for this token"
        );
        let Token::Info(info) = &read[1] else {
            panic!("the second token is an INFO");
        };
        assert_eq!(info.number, 5701);
        assert_eq!(info.server, "mssql.test");
        assert_eq!(info.line, 7);
        assert!(!info.is_fatal());
        assert_eq!(read[2], Token::Env(EnvChange::PacketSize { to: 8192 }));
        assert_eq!(
            read[3],
            Token::LoginAck(LoginAck {
                tds_version: TDS_VERSION,
                program: "Microsoft SQL Server".to_owned(),
                version: (16, 0, 4035),
            }),
            "the version LOGIN7 asked for is the one that comes back"
        );
        let Token::Done(done) = &read[4] else {
            panic!("the last token is a DONE");
        };
        assert!(done.counted() && !done.more() && !done.failed());
    }

    /// § 8's kinds, over the numbers this backend raises them as.
    ///
    /// One question of every row rather than a case each, so a table that grew
    /// a row meaning something else fails here: the numbers are the whole of
    /// what a caller branches on, since TDS sends no `SQLSTATE` to fall back to.
    #[test]
    fn a_sql_server_error_normalises_by_its_number_and_a_fatal_class_is_a_lost_connection() {
        for (number, kind) in [
            (2601, DbErrorKind::UniqueViolation),
            (2627, DbErrorKind::UniqueViolation),
            (547, DbErrorKind::ForeignKeyViolation),
            (515, DbErrorKind::NotNullViolation),
            (1205, DbErrorKind::Deadlock),
            (3960, DbErrorKind::SerializationFailure),
            (1222, DbErrorKind::Timeout),
            (208, DbErrorKind::Syntax),
            (4060, DbErrorKind::Permission),
            (18456, DbErrorKind::Permission),
            (8152, DbErrorKind::Other),
        ] {
            assert_eq!(kind_of(number, 16), kind, "error {number}");
        }

        assert!(
            kind_of(1205, 13).is_retryable() && kind_of(3960, 16).is_retryable(),
            "§ 7 re-runs a callable over these two and nothing else"
        );
        assert!(
            !kind_of(1222, 16).is_retryable(),
            "a lock timeout rolled nothing back, so the callable's earlier statements are still \
             in the transaction"
        );
        assert_eq!(
            kind_of(4001, FATAL_CLASS),
            DbErrorKind::ConnectionLost,
            "at this severity the server has already closed the socket"
        );
        assert_eq!(kind_of(4001, FATAL_CLASS - 1), DbErrorKind::Other);
    }

    #[test]
    fn an_error_token_carries_what_the_server_said_and_which_kind_it_is() {
        let payload = message_token(
            TOKEN_ERROR,
            2627,
            14,
            "Violation of PRIMARY KEY constraint 'PK_orders'.",
        );
        let read = tokens(&payload).expect("an error token is a token like any other");
        let [Token::Error(error)] = read.as_slice() else {
            panic!("one ERROR token");
        };
        assert_eq!(error.number, 2627);
        assert_eq!(error.state, 1);
        assert_eq!(error.kind(), DbErrorKind::UniqueViolation);
        assert!(error.message.contains("PK_orders"));
        assert!(
            !error.is_fatal(),
            "a constraint refuses a statement, not the connection"
        );
    }

    /// An unread `ENVCHANGE` type costs the reader nothing, because the token's
    /// own length is what skips it.
    #[test]
    fn an_envchange_this_driver_does_not_read_is_skipped_by_its_length() {
        // Type 17, the routing answer an Azure failover sends, whose two halves
        // are bytes rather than characters — so a reader that walked the value
        // halves would be reading a routing address as a character count.
        let mut payload = token(TOKEN_ENV_CHANGE, &[17, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0]);
        payload.extend_from_slice(&done_token(0, 0));

        let read = tokens(&payload).expect("an unread type is not a malformed message");
        assert_eq!(read[0], Token::Env(EnvChange::Other(17)));
        assert_eq!(read.len(), 2, "the DONE after it is still found");
    }

    /// § 7's descriptor is read off the token that carries it, on all three of
    /// the types that carry one.
    ///
    /// The begin is where the eight octets arrive; the commit and the rollback
    /// send an empty new value, and reading that as *no transaction* is what
    /// puts the connection back where the next statement can run outside a
    /// transaction at all.
    #[test]
    fn a_transaction_envchange_carries_the_descriptor_a_begin_opened() {
        let opened = tokens(&token(
            TOKEN_ENV_CHANGE,
            &[ENV_BEGIN_TRANSACTION, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0],
        ))
        .expect("a begin names the transaction it opened");
        assert_eq!(
            opened[0],
            Token::Env(EnvChange::Transaction {
                to: 0x0807_0605_0403_0201
            }),
            "the octets are echoed back in `ALL_HEADERS` and never read into fields"
        );

        for ended in [ENV_COMMIT_TRANSACTION, ENV_ROLLBACK_TRANSACTION] {
            // The empty new value, then the descriptor as the *old* one, which
            // this driver skips by the token's own length.
            let read = tokens(&token(
                TOKEN_ENV_CHANGE,
                &[ended, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8],
            ))
            .expect("an ended transaction names none");
            assert_eq!(
                read[0],
                Token::Env(EnvChange::Transaction { to: NO_TRANSACTION }),
                "type {ended}"
            );
        }

        // A width this driver cannot echo is refused where it arrived, rather
        // than carried into every later request as a descriptor naming nothing.
        let refused = tokens(&token(
            TOKEN_ENV_CHANGE,
            &[ENV_BEGIN_TRANSACTION, 4, 1, 2, 3, 4, 0],
        ))
        .expect_err("a descriptor that is not eight octets");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("4 octets"), "{refused}");
    }

    /// A field or a token that runs off the end says so, rather than reading
    /// whatever followed it.
    #[test]
    fn a_token_that_runs_past_the_end_of_the_message_is_refused() {
        let whole = message_token(TOKEN_ERROR, 2627, 14, "Violation");
        for cut in 1..whole.len() {
            let refused = tokens(&whole[..cut]).expect_err("half a token is not a token");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidData, "cut at {cut}");
        }

        // A length that promises more than the message holds is the same
        // refusal one step earlier, and it is the one a `take` per field would
        // miss: every field of this token is present.
        let mut lying = whole.clone();
        lying[1] = 0xFF;
        let refused = tokens(&lying).expect_err("a token cannot be longer than its message");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);

        let refused = tokens(&[0x60, 0, 0]).expect_err("no TDS token is 0x60");
        assert!(
            refused.to_string().contains("0x60"),
            "an unread token names the byte, which is the whole of what it can say"
        );
    }

    /// `ORDER` carries nothing a caller reads, and the walk resumes at the byte
    /// after its ordinals.
    ///
    /// Its extent comes from its own length field, so a reader that assumed a
    /// fixed one would land inside the next token and refuse an ordinary
    /// answer. Every catalog query orders its rows, so this is the token
    /// between `COLMETADATA` and the first row of every introspection on this
    /// backend — and the walk has to stop on it rather than swallow it, because
    /// what follows is a row and [`Tokens`] cannot step over one.
    #[test]
    fn an_order_token_carries_nothing_and_the_walk_resumes_after_it() {
        // Two ordinals, four bytes: the shape an `ORDER BY` over two columns
        // arrives in.
        let mut payload = vec![TOKEN_ORDER, 4, 0, 1, 0, 2, 0];
        payload.extend_from_slice(&done());
        let read = tokens(&payload).expect("an ordered result set is an ordinary answer");
        assert_eq!(read.len(), 2, "the walk lost a token: {read:?}");
        assert_eq!(read[0], Token::Order);
        assert!(
            matches!(read[1], Token::Done(_)),
            "the walk resumed inside the ordinals: {read:?}"
        );

        // A length that promises more than the message holds is refused with
        // the message every other token's is, rather than silently ending the
        // walk.
        let refused = tokens(&[TOKEN_ORDER, 8, 0, 1, 0]).expect_err("the ordinals are not there");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
    }

    /// The packet size arrives as digits, and a server that wrote something
    /// else is not answered with a plausible size.
    #[test]
    fn a_packet_size_envchange_is_the_number_the_server_wrote() {
        let read = tokens(&env_token(ENV_PACKET_SIZE, "16384", "4096"))
            .expect("digits are what this token carries");
        assert_eq!(read[0], Token::Env(EnvChange::PacketSize { to: 16384 }));

        let refused = tokens(&env_token(ENV_PACKET_SIZE, "large", "4096"))
            .expect_err("a size that is not a number");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
    }
}
