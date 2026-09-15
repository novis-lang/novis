//! [`TdsRows`] — one result set, read a row packet at a time.
//!
//! The stream owns the connection until it ends, which is [ADR 0067
//! § 4](/docs/decisions/0067.md)'s single-statement rule made
//! structural: the `State` returns to `Idle` when the `DONE` token arrives and
//! not before. Every token between the rows goes to [`Tokens`] rather than
//! being read a second time here.

use super::*;

/// A statement's result, and the rows still to come out of it.
///
/// [`crate::MySqlRows`]' shape, and it is the same borrow for the same reason:
/// the handle holds the wire and the busy state, so the connection is unusable
/// for anything else until the stream ends — [ADR 0067
/// § 4](/docs/decisions/0067.md)'s one-statement-at-a-time rule
/// enforced by the type system rather than by a check every caller has to
/// remember. A statement with no result set answers one of these too, already
/// ended: [`TdsRows::columns`] is empty and [`TdsRows::next_row`] is `None` on
/// the first call.
///
/// Everything about the answer that is not the borrow is [`TdsCursor`], which
/// owns why that is a split rather than its fields inlined here.
pub struct TdsRows<'a, S: Read + Write = NvsTls<Tunnel<NvsTcp>>> {
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    /// § 1's cache, for the one moment reading an answer touches it, and `None`
    /// for an answer with no plan to file — [`PendingPlan`] owns why the key
    /// travels on the cursor while the cache is handed to the step that files
    /// it.
    cache: Option<&'a mut StatementCache<TdsPlan>>,
    reading: TdsCursor,
}

/// A statement's read state: what the answer described, how far this side has
/// got through it, what ended it, and the event it is being timed by —
/// everything a row needs that is not the wire.
///
/// [`crate::MySqlCursor`]'s twin, and a type of its own for that one's reason:
/// `rule:core-classes/db-statement-members`'s rows are reached two ways and only one of them can
/// hold a borrow. The **buffered** members drain their rows inside the call
/// that started the statement, so [`TdsRows`] keeps this beside a borrow of the
/// connection and the borrow checker is what refuses a second statement.
/// **`Core\Db\Connection::stream`** hands a walk back to the program and is
/// advanced by a *later* call, with nothing of the connection borrowed in
/// between; a borrow cannot span that, so its copy of this state is parked on
/// the connection ([`TdsConn::stream`]) and [`State::Streaming`] is what
/// refuses the second statement there.
///
/// Both drive [`next_row_of`], which is the one place in this driver a row
/// token is read, so the two paths cannot disagree about what ends an answer or
/// about what the tokens between the rows said on the way past.
///
/// # What it holds that the other drivers' do not
///
/// **The remainder.** MySQL and PostgreSQL align a row with a packet, so their
/// cursors own nothing between calls; a TDS row is cut wherever the packet size
/// lands, so this one buffers what a packet carried past the token it was
/// parsing and drops what it has read off the front on the next refill. The
/// buffer is therefore about a packet plus the token in hand, and it is
/// deliberately *not* the answer: a `PLP` value's chunks are copied into the
/// row as they arrive rather than accumulated here, so a `varbinary(max)`
/// column costs its own size and not its size twice.
#[derive(Debug)]
pub(crate) struct TdsCursor {
    /// `COLMETADATA`'s columns, in the order their values arrive, and empty for
    /// a statement that returned no result set.
    columns: Vec<TdsColumn>,
    /// Bytes of the token stream that have arrived and are not parsed yet.
    buffer: Vec<u8>,
    /// How much of [`TdsCursor::buffer`] is behind the reader. The prefix is
    /// dropped at the next refill rather than at every read, which is what
    /// keeps a row of many small columns from being a `drain` per column.
    at: usize,
    /// Whether the packet carrying [`Status::EOM`] has been read, so no further
    /// packet belongs to this answer.
    last: bool,
    /// Rows handed back so far.
    rows: u64,
    /// The last `DONE` that set [`Done::counted`], which is the count a
    /// statement with no result set reports.
    counted: Option<u64>,
    /// Whether the stream has ended, by any of the ways it can: a `DONE`, the
    /// server's refusal, or a decode this driver will not continue past.
    ended: bool,
    /// The last `RETURNVALUE` the answer carried, which for § 1's `sp_prepexec`
    /// is the handle the server allocated.
    ///
    /// The *last* rather than all of them because a procedure this driver calls
    /// declares one output parameter, and a `Vec` for a list that is one long
    /// would be an allocation on every statement. It arrives near the end of
    /// the stream, after the rows, so a caller reads it once [`next_row_of`]
    /// has answered `None` — see [`TdsCursor::returned`].
    returned: Option<ReturnValue>,
    /// [ADR 0067 § 1](/docs/decisions/0067.md)'s cache key this
    /// answer's handle belongs under, for a `sp_prepexec` whose plan is to be
    /// kept; `None` for every other answer, which is most of them.
    ///
    /// The walk carries it rather than the caller filing it afterwards —
    /// [`PendingPlan`] owns why.
    filing: Option<PendingPlan>,
    /// [ADR 0067 § 11](/docs/decisions/0067.md)'s trace event for this
    /// statement, opened when the request went out and ended by whatever ends
    /// the stream — [`crate::MySqlCursor`]' field, for [`crate::span`]'s
    /// reasons.
    span: QuerySpan,
}

/// One step of a walk: a cursor, and a fresh borrow of everything on the
/// connection it takes to advance.
///
/// The other drivers hand `(wire, state, cursor)` to a single function and are
/// done, because a row is a packet there and reading one is a call. A TDS row
/// is cut wherever the packet size lands, so reading one is a dozen small
/// readers — a byte, a length, a chunk of a `PLP` value — each of which needs
/// the wire *and* the buffer the last packet overshot into. Grouping them here
/// is what keeps every one of those from carrying the same three parameters,
/// and the group is **built per step and dropped with it**, which is what lets
/// a parked cursor be advanced by a call that borrows the connection afresh.
///
/// **A token reader it hands the between-row tokens to.** [`Tokens`] parses
/// every token in a response but the two row ones, and reusing it is what keeps
/// `ERROR`, `INFO`, `ENVCHANGE`, `COLMETADATA` and `DONE` written once. It
/// parses a slice, so it is offered the buffer and asked again with more of it
/// when the token was not all there — [`Walk::token`] owns why that is cheaper
/// than it looks.
struct Walk<'a, S: Read + Write> {
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    /// § 1's cache, where the answer being read is one whose plan is still to
    /// be filed. `None` for a walk with nothing to file — a cache hit, a drain,
    /// or a read a test scripted.
    cache: Option<&'a mut StatementCache<TdsPlan>>,
    reading: &'a mut TdsCursor,
}

/// [ADR 0067 § 1](/docs/decisions/0067.md)'s cache key a walk's plan
/// is filed under, once the `RETURNVALUE` carrying the handle arrives.
///
/// **The key travels with the walk and the cache does not**, which is the one
/// thing parking a read costs this driver: a cursor parked on a connection
/// outlives every borrow of it, so a `&mut StatementCache` held here would be a
/// borrow nothing could give it. The cache is handed to the step instead
/// ([`Walk::cache`]), and [`Walk::end`] is where the two meet.
///
/// The walk carries the key at all because `sp_prepexec`'s handle is a
/// `RETURNVALUE` that arrives *after* the rows, and a statement with no result
/// set has already ended by the time [`open_result`] returns — so a caller
/// filing it afterwards would file nothing on exactly the statements a cache is
/// worth the most on.
#[derive(Debug)]
pub(super) struct PendingPlan {
    /// § 1's key, first half: the statement as written.
    pub(super) sql: String,
    /// § 1's key, second half: how many markers § 5's rewrite left in it.
    pub(super) arity: usize,
    /// The `@params` the plan is being compiled against — [`TdsPlan::declared`]
    /// owns why a plan is not usable without it.
    pub(super) declared: Rc<str>,
}

impl<S: Read + Write> std::fmt::Debug for TdsRows<'_, S> {
    /// The shape of the result and where the wire is, and nothing that arrived.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdsRows")
            .field("columns", &self.reading.columns.len())
            .field("state", &self.state.get())
            .finish_non_exhaustive()
    }
}

impl TdsCursor {
    /// The result set's columns, empty for a statement that returned none —
    /// [`TdsRows::columns`] and [`TdsConn::stream_columns`] are both this.
    pub(crate) fn columns(&self) -> &[TdsColumn] {
        &self.columns
    }

    /// Column `index`'s Novis type, per [`TdsColumn::column_type`], or `None`
    /// where the result set has no such column.
    pub(crate) fn column_type(&self, index: usize) -> Option<ColumnType> {
        self.columns.get(index).map(TdsColumn::column_type)
    }

    /// [ADR 0067 § 11](/docs/decisions/0067.md)'s trace event for the
    /// statement this walks.
    pub(crate) fn span(&self) -> &QuerySpan {
        &self.span
    }

    /// Names the `[db.<name>]` block the statement ran on — [`QuerySpan::name`]
    /// owns why the driver cannot work it out for itself.
    pub(crate) fn name_connection(&mut self, connection: &str) {
        self.span.name(connection);
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s affected-row count,
    /// once the stream has ended.
    ///
    /// [`crate::MySqlCursor`]'s two numbers under one name — the rows that came
    /// back for a statement with a result set, and what the server counted for
    /// one without — with one difference this backend forces: **`None` is two
    /// facts here rather than one.** A stream still running has no count yet,
    /// and a `DONE` that did not set [`Done::counted`] reported none at all,
    /// which [`Done::counted`]'s own doc explains is not the same as zero. The
    /// caller that must tell them apart has already been told which:
    /// [`next_row_of`] answered `None`.
    ///
    /// There is no `lastId` beside it. SQL Server puts no generated key in the
    /// token stream at all — `SCOPE_IDENTITY()` is a statement a caller writes —
    /// so this driver has nothing to answer with, and PostgreSQL's `RETURNING`
    /// is the same absence.
    pub(crate) fn affected(&self) -> Option<u64> {
        if !self.ended {
            return None;
        }
        if self.columns.is_empty() {
            self.counted
        } else {
            Some(self.rows)
        }
    }

    /// The output parameter the procedure came back with, once the stream has
    /// reached it.
    ///
    /// For § 1's `sp_prepexec` that is the statement handle, and it arrives
    /// **after** the rows: the server sends `RETURNVALUE` next to the
    /// `RETURNSTATUS` that ends the procedure, so the handle is filed when the
    /// stream has ended and not when it opened.  A statement that was never a
    /// procedure call answers `None`.
    pub(crate) fn returned(&self) -> Option<&ReturnValue> {
        self.returned.as_ref()
    }
}

impl<'a, S: Read + Write> TdsRows<'a, S> {
    /// The buffered shape of an answer already opened: the read state beside a
    /// borrow of the connection it came off.
    ///
    /// `cache` is `Some` for the answer whose plan [`Walk::end`] is still to
    /// file, and that is the whole of why the borrow is here rather than the
    /// cursor holding one.
    pub(super) fn over(
        wire: &'a mut Wire<S>,
        state: &'a Cell<State>,
        cache: Option<&'a mut StatementCache<TdsPlan>>,
        reading: TdsCursor,
    ) -> TdsRows<'a, S> {
        TdsRows {
            wire,
            state,
            cache,
            reading,
        }
    }
}

impl<S: Read + Write> TdsRows<'_, S> {
    /// The result set's columns, empty for a statement that returned none.
    #[must_use]
    pub fn columns(&self) -> &[TdsColumn] {
        self.reading.columns()
    }

    /// Column `index`'s Novis type, per [`TdsColumn::column_type`], or `None`
    /// where the result set has no such column.
    #[must_use]
    pub fn column_type(&self, index: usize) -> Option<ColumnType> {
        self.reading.column_type(index)
    }

    /// [ADR 0067 § 11](/docs/decisions/0067.md)'s trace event for this
    /// statement.
    ///
    /// Borrowed rather than taken, for [`crate::PgRows::span`]'s reason.
    #[must_use]
    pub fn span(&self) -> &QuerySpan {
        self.reading.span()
    }

    /// Names the `[db.<name>]` block this statement ran on, for the layer that
    /// resolved it — [`QuerySpan::name`] owns why the driver cannot.
    pub fn name_connection(&mut self, connection: &str) {
        self.reading.name_connection(connection);
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s affected-row count,
    /// once the stream has ended — [`TdsCursor::affected`], including what its
    /// `None` means.
    #[must_use]
    pub fn affected(&self) -> Option<u64> {
        self.reading.affected()
    }

    /// The output parameter the procedure came back with, once the stream has
    /// reached it — [`TdsCursor::returned`].
    #[must_use]
    pub fn returned(&self) -> Option<&ReturnValue> {
        self.reading.returned()
    }

    /// The next row, or `None` once the stream has ended.
    ///
    /// Deliberately not `Iterator::next`, for [`crate::PgRows::next_row`]'s
    /// reason: every call can fail, and an `Option` would have to swallow it.
    /// The call [`TdsConn::stream_next_row`] advances a parked walk with is the
    /// same [`next_row_of`] over the same cursor.
    ///
    /// # Errors
    ///
    /// As [`next_row_of`].
    pub fn next_row(&mut self) -> io::Result<Option<TdsRow>> {
        next_row_of(
            self.wire,
            self.state,
            self.cache.as_deref_mut(),
            &mut self.reading,
        )
    }
}

/// One row of a walk, or `None` once the answer has ended —
/// [`TdsRows::next_row`] and [`TdsConn::stream_next_row`] are both this.
///
/// Free, and generic in the stream, for [`crate::pg::next_row_of`]'s two
/// reasons: the parked walk has to be advanced through a borrow taken per row,
/// and a [`TdsConn`]'s own wire is at the default type parameter, so anything
/// reachable only through an inherent method on it would need a socket and a
/// certificate to reach at all.
///
/// Ending the stream is what returns the connection to [`State::Idle`].
///
/// # Errors
///
/// The server's own error, which still ends the stream cleanly and leaves the
/// connection idle; `InvalidData` for a token stream that does not add up
/// against the columns, for a second result set, and for an answer that ended
/// without a `DONE`, all of which poison the connection; and whatever the
/// stream reported.
pub(crate) fn next_row_of<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: Option<&mut StatementCache<TdsPlan>>,
    reading: &mut TdsCursor,
) -> io::Result<Option<TdsRow>> {
    // The state is the only bookkeeping: anything that ended this stream has
    // already left `State::Streaming`.
    if state.get() != State::Streaming {
        return Ok(None);
    }
    let stepped = Walk {
        wire,
        state,
        cache,
        reading,
    }
    .step();
    match stepped {
        Ok(row) => Ok(row),
        Err(e) => Err(poison_on_read(state, e)),
    }
}

/// Abandons a parked walk and forgets what it read — [`TdsConn::end_stream`],
/// and § 13's reset on its way past.
///
/// **The walk is drained first**, because the rows are on their way whether or
/// not anybody reads them and this protocol gives the next request nothing to
/// resynchronise on: every TDS request has exactly one answer, so an answer
/// left half-read is one the next statement reads as its own. A read that fails
/// on the way poisons the connection through the same helper every other read
/// here uses, and the loop ends because that leaves [`State::Streaming`].
///
/// The cache is offered for the same drain's sake: an answer read to its end
/// carries the `RETURNVALUE` § 1's plan is filed under, and a walk the program
/// abandoned has compiled a plan on the server that nothing else will ever name
/// again. Dropping the read state is the other half — a result set's shape and
/// its counts are one request's, and `rule:security/db-pool-reset-is-a-boundary` is why the next
/// one must not be able to read them.
pub(crate) fn end_stream_of<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: Option<&mut StatementCache<TdsPlan>>,
    reading: &mut Option<TdsCursor>,
) {
    let mut cache = cache;
    if let Some(walk) = reading.as_mut() {
        while state.get() == State::Streaming {
            if next_row_of(wire, state, cache.as_deref_mut(), walk).is_err() {
                break;
            }
        }
    }
    *reading = None;
}

impl<S: Read + Write> Walk<'_, S> {
    /// Reads tokens until a row arrives or the answer ends.
    ///
    /// The refusal is collected rather than returned where it is found: an
    /// `ERROR` is followed by the `DONE` that ends the statement, and a reader
    /// that returned at the `ERROR` would leave those bytes on the wire and
    /// poison a connection the server had left perfectly usable. Rows that
    /// arrive after one are read and dropped for the same reason — the wire has
    /// to be walked past them either way, and there is nobody to hand them to
    /// once the statement has failed.
    fn step(&mut self) -> io::Result<Option<TdsRow>> {
        let mut refusal = None;
        loop {
            match self.peek()? {
                Some(kind @ (TOKEN_ROW | TOKEN_NBC_ROW)) => {
                    self.reading.at += 1;
                    let row = self.row(kind == TOKEN_NBC_ROW)?;
                    if refusal.is_some() {
                        continue;
                    }
                    self.reading.rows += 1;
                    self.reading.span.row();
                    return Ok(Some(row));
                }
                Some(_) => match self.token()? {
                    // § 7's descriptor, which the next request has to carry —
                    // [`EnvChange::Transaction`] owns what a missed one costs.
                    Some(Token::Env(EnvChange::Transaction { to })) => self.wire.set_descriptor(to),
                    Some(
                        Token::Info(_) | Token::Env(_) | Token::ReturnStatus(_) | Token::Order,
                    ) => {}
                    Some(Token::ReturnValue(returned)) => {
                        self.reading.returned = Some(returned);
                    }
                    Some(Token::Error(message)) => {
                        if refusal.is_none() {
                            refusal = Some(message);
                        }
                    }
                    Some(Token::Done(done)) => {
                        if done.counted() {
                            self.reading.counted = Some(done.rows);
                        }
                        // `DONEINPROC` is never the last token of an answer —
                        // § 1's `sp_prepexec` sends the `RETURNVALUE` carrying
                        // its handle after one — and a `DONE` that says more
                        // results follow is a batch this surface has nowhere to
                        // put: the second `COLMETADATA` below is where that is
                        // refused, once the tokens between here and it have been
                        // read rather than abandoned mid-packet.
                        if done.more() || done.in_proc {
                            continue;
                        }
                        self.end(refusal.is_some())?;
                        return match refusal {
                            Some(message) => Err(server_refusal(&message)),
                            None => Ok(None),
                        };
                    }
                    Some(Token::Columns(_)) => {
                        return Err(malformed(String::from(
                            "the server has a second result set for this statement, and `rule:core-classes/db-one-api` \
                             § 4's one-statement-at-a-time surface has nowhere to put it",
                        )));
                    }
                    Some(Token::LoginAck(_)) => {
                        return Err(malformed(String::from(
                            "a TDS statement was answered with a LOGINACK, which is a stream out \
                             of sync rather than a login",
                        )));
                    }
                    None => return Err(no_done()),
                },
                None => return Err(no_done()),
            }
        }
    }

    /// Ends the stream: freezes the span, and hands the connection back where
    /// the answer really did end at a packet boundary.
    ///
    /// The check is the whole of what makes the connection reusable. A `DONE`
    /// with bytes behind it, or one in a packet that never carried
    /// [`Status::EOM`], is an answer this driver has read only part of, and
    /// returning that wire to the pool is one request reading another's.
    ///
    /// # Errors
    ///
    /// `InvalidData` where the answer did not end where the `DONE` said it did.
    /// [`TdsRows::next_row`] poisons the connection on it.
    fn end(&mut self, refused: bool) -> io::Result<()> {
        self.reading.ended = true;
        // No affected count on a refusal, as `crate::MySqlRows::next_row` does
        // it: § 11 gives a span no success field to lose, so a refused statement
        // reports the rows that did arrive and the error is the caller's own
        // return value.
        let affected = if refused {
            None
        } else {
            self.reading.affected()
        };
        self.reading.span.finished(affected);
        // § 1's cache is filed here and nowhere else, because here is the first
        // moment the handle exists: `sp_prepexec` writes it into a `RETURNVALUE`
        // that arrives after the rows, so a caller filing it would have to be
        // told to read `returned()` and would be free to forget. A refusal files
        // nothing — the server compiled no plan there is a number for — and
        // neither does a stream that ended any other way, which leaves that plan
        // alive on the server until the connection closes rather than filed
        // under a handle this side never read.
        let handle = self.reading.returned.as_ref().and_then(ReturnValue::as_i32);
        let filing = self.reading.filing.take().filter(|_| !refused);
        if let (Some(filing), Some(handle), Some(cache)) =
            (filing, handle, self.cache.as_deref_mut())
        {
            let plan = TdsPlan {
                handle,
                declared: filing.declared,
            };
            cache.commit(&filing.sql, filing.arity, plan);
        }
        if !self.reading.last || self.reading.at < self.reading.buffer.len() {
            return Err(malformed(format!(
                "a TDS answer carried {} byte(s) after the DONE that ended it",
                self.reading.buffer.len() - self.reading.at
            )));
        }
        self.state.set(State::Idle);
        Ok(())
    }

    /// The next token's type byte, without consuming it, or `None` where the
    /// answer has ended.
    fn peek(&mut self) -> io::Result<Option<u8>> {
        while self.reading.at == self.reading.buffer.len() {
            if self.reading.last {
                return Ok(None);
            }
            self.fill()?;
        }
        Ok(Some(self.reading.buffer[self.reading.at]))
    }

    /// The next token, parsed by [`Tokens`] over the buffer this reader filled.
    ///
    /// **A failed parse is retried with more bytes rather than reported**, until
    /// the packet carrying [`Status::EOM`] has arrived. `Tokens` reads a slice
    /// and cannot say whether it ran out of bytes or ran into a bad one, and
    /// telling the two apart from the outside would mean a second table of
    /// every token's extent — `COLMETADATA`, which is the one a result set
    /// opens with, does not carry one at all. Retrying costs a re-parse of a
    /// token that is at most tens of kilobytes and only while it is incomplete;
    /// a malformed token is refused with the same message one packet later, and
    /// [`MAX_MESSAGE`] in [`TdsRows::fill`] is what stops a stream that never
    /// parses from growing without one.
    ///
    /// Rows never reach here: [`TdsRows::step`] reads the type byte first, and
    /// a row is the one token whose extent needs the column list.
    fn token(&mut self) -> io::Result<Option<Token>> {
        loop {
            let (parsed, consumed) = {
                let mut tokens = Tokens::over(&self.reading.buffer[self.reading.at..]);
                let parsed = tokens.next_token();
                (parsed, tokens.consumed())
            };
            match parsed {
                Ok(Some(token)) => {
                    self.reading.at += consumed;
                    return Ok(Some(token));
                }
                Ok(None) if self.reading.last => return Ok(None),
                Err(e) if self.reading.last => return Err(e),
                _ => self.fill()?,
            }
        }
    }

    /// One row's values, against the columns that measure them.
    ///
    /// `nbc` is [`TOKEN_NBC_ROW`]: a bitmap of one bit per column, least
    /// significant bit first, and a set bit is a column that is `NULL` and
    /// carries no bytes at all. It is not an optimisation this reader may
    /// decline — the server sends whichever form it likes and the two are read
    /// differently — and it is the only place a null arrives without the type's
    /// own null form.
    fn row(&mut self, nbc: bool) -> io::Result<TdsRow> {
        let mut nulls = Vec::new();
        if nbc {
            let bitmap = self.reading.columns.len().div_ceil(8);
            self.copy(bitmap, &mut nulls)?;
        }

        let mut bytes = Vec::new();
        let mut values = Vec::with_capacity(self.reading.columns.len());
        for index in 0..self.reading.columns.len() {
            if nbc && nulls[index / 8] & (1 << (index % 8)) != 0 {
                values.push(None);
                continue;
            }
            // Copied out because `TypeInfo` is `Copy` and the read below is a
            // `&mut self`: the alternative is a clone of the whole column list
            // per row.
            let info = self.reading.columns[index].type_info;
            let start = bytes.len();
            let present = self.value(index, info, &mut bytes)?;
            values.push(present.then_some(start..bytes.len()));
        }
        Ok(TdsRow { bytes, values })
    }

    /// One column's value, appended to `bytes`, and whether it was there at all
    /// — `false` is SQL `NULL`.
    ///
    /// [`Length`] is the whole of the type knowledge here: this reads what the
    /// column said its values are measured by and never what they mean, which
    /// is [ADR 0067 § 9](/docs/decisions/0067.md)'s decode and belongs
    /// to the crate that can allocate a `Core\Time\DateTime`.
    fn value(&mut self, index: usize, info: TypeInfo, bytes: &mut Vec<u8>) -> io::Result<bool> {
        // A `NULLTYPE` column carries no value and has no value to carry: the
        // one thing `SELECT NULL` can be is null, and reading it as a present
        // value of no bytes would make § 9's `?T` depend on the type byte a
        // layer up.
        if info.id == TY_NULL {
            return Ok(false);
        }
        match info.length {
            Length::Fixed(width) => {
                self.copy(width, bytes)?;
                Ok(true)
            }
            Length::Byte(most) => {
                let length = self.byte()?;
                if length == 0 {
                    return Ok(false);
                }
                self.bounded(u32::from(length), u32::from(most), index, info)?;
                self.copy(usize::from(length), bytes)?;
                Ok(true)
            }
            Length::Short(most) => {
                let length = self.short()?;
                if length == NO_LENGTH {
                    return Ok(false);
                }
                self.bounded(u32::from(length), u32::from(most), index, info)?;
                self.copy(usize::from(length), bytes)?;
                Ok(true)
            }
            Length::Long(most) => self.long_value(index, info, most, bytes),
            Length::Partial => self.partial_value(index, info, bytes),
        }
    }

    /// A `LONGLEN` value: the three deprecated types' text pointer, then the
    /// four-byte length [`Length::Long`] describes.
    fn long_value(
        &mut self,
        index: usize,
        info: TypeInfo,
        most: u32,
        bytes: &mut Vec<u8>,
    ) -> io::Result<bool> {
        if matches!(info.id, TY_TEXT | TY_NTEXT | TY_IMAGE) {
            let pointer = self.byte()?;
            if pointer == 0 {
                return Ok(false);
            }
            self.skip(usize::from(pointer) + TEXT_TIMESTAMP)?;
        }
        let length = self.long()?;
        // Zero is `NULL` only for `sql_variant`, which the type byte decides:
        // an empty `text` is a value, and it arrived through a pointer that was
        // there.
        if length == NO_LENGTH_LONG || (length == 0 && info.id == TY_VARIANT) {
            return Ok(false);
        }
        self.bounded(length, most, index, info)?;
        self.copy(as_usize(length)?, bytes)?;
        Ok(true)
    }

    /// A `PLP` value: a declared total or a sentinel, then chunks until an empty
    /// one.
    ///
    /// The chunks are copied straight into the row, so the buffer never holds
    /// the value and a `varbinary(max)` costs its own size once. The declared
    /// total is checked against what arrived rather than trusted: it is the
    /// server's word about bytes that had not been sent yet, and a value that
    /// disagrees with it is a stream this driver cannot prove the position of.
    fn partial_value(
        &mut self,
        index: usize,
        info: TypeInfo,
        bytes: &mut Vec<u8>,
    ) -> io::Result<bool> {
        let total = self.quad()?;
        if total == PLP_NULL {
            return Ok(false);
        }
        let start = bytes.len();
        if total != PLP_UNKNOWN {
            bytes.reserve(as_usize(total)?.min(PLP_RESERVE));
        }
        loop {
            let chunk = self.long()?;
            if chunk == 0 {
                break;
            }
            self.copy(as_usize(chunk)?, bytes)?;
        }
        let read = (bytes.len() - start) as u64;
        if total != PLP_UNKNOWN && read != total {
            return Err(malformed(format!(
                "a TDS PLP value for column {index} (type 0x{:02X}) declared {total} byte(s) and \
                 its chunks carried {read}",
                info.id
            )));
        }
        Ok(true)
    }

    /// Refuses a value wider than the column it belongs to.
    ///
    /// The declared width is the server's own description of the column two
    /// tokens earlier, so a value past it is the reader having lost its place
    /// in the stream — the one error worth catching before it is read as the
    /// next column's length.
    fn bounded(&self, length: u32, most: u32, index: usize, info: TypeInfo) -> io::Result<()> {
        if length > most {
            return Err(malformed(format!(
                "a TDS row's column {index} (type 0x{:02X}) carried {length} byte(s), past the \
                 {most} its COLMETADATA declared",
                info.id
            )));
        }
        Ok(())
    }

    /// Reads one more packet of this answer into the buffer, dropping what is
    /// already behind the reader.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a packet that is not a tabular result, for an answer
    /// whose last packet left a token unfinished, and for a single token past
    /// [`MAX_MESSAGE`] — the same ceiling [`Wire::read_message`] holds a whole
    /// message to, and here it bounds one token rather than the answer, since a
    /// row's values leave the buffer as they arrive.
    fn fill(&mut self) -> io::Result<()> {
        if self.reading.last {
            return Err(malformed(String::from(
                "a TDS answer's last packet ended in the middle of a token",
            )));
        }
        let packet = self.wire.read_packet()?;
        if packet.kind != PacketType::TabularResult {
            return Err(malformed(format!(
                "a TDS answer continued with a packet of type 0x{:02X}, which is not a tabular \
                 result",
                packet.kind.byte()
            )));
        }
        self.reading.last = packet.status.contains(Status::EOM);
        if self.reading.at > 0 {
            self.reading.buffer.drain(..self.reading.at);
            self.reading.at = 0;
        }
        if self.reading.buffer.len() + packet.payload.len() > MAX_MESSAGE {
            return Err(malformed(format!(
                "a TDS token grew past this driver's {MAX_MESSAGE}-byte ceiling"
            )));
        }
        self.reading.buffer.extend_from_slice(&packet.payload);
        Ok(())
    }

    /// Ensures the buffer holds `n` bytes the reader has not read.
    ///
    /// Only ever asked for a field, never for a value: a value is copied out
    /// with [`TdsRows::copy`], which spans packets instead of demanding they be
    /// contiguous — a `PLP` chunk is four bytes of length away from being
    /// two gigabytes.
    fn need(&mut self, n: usize) -> io::Result<()> {
        while self.reading.buffer.len() - self.reading.at < n {
            self.fill()?;
        }
        Ok(())
    }

    /// One byte of a token's own fields.
    fn byte(&mut self) -> io::Result<u8> {
        self.need(1)?;
        let byte = self.reading.buffer[self.reading.at];
        self.reading.at += 1;
        Ok(byte)
    }

    /// Two bytes, little-endian, as everything inside a payload is.
    fn short(&mut self) -> io::Result<u16> {
        self.need(2)?;
        let at = self.reading.at;
        let short = u16::from_le_bytes([self.reading.buffer[at], self.reading.buffer[at + 1]]);
        self.reading.at += 2;
        Ok(short)
    }

    /// Four bytes, little-endian.
    fn long(&mut self) -> io::Result<u32> {
        self.need(4)?;
        let at = self.reading.at;
        let long = u32::from_le_bytes(
            self.reading.buffer[at..at + 4]
                .try_into()
                .expect("four bytes, as asked for"),
        );
        self.reading.at += 4;
        Ok(long)
    }

    /// Eight bytes, little-endian: a `PLP` value's declared total.
    fn quad(&mut self) -> io::Result<u64> {
        self.need(8)?;
        let at = self.reading.at;
        let quad = u64::from_le_bytes(
            self.reading.buffer[at..at + 8]
                .try_into()
                .expect("eight bytes, as asked for"),
        );
        self.reading.at += 8;
        Ok(quad)
    }

    /// Copies `n` bytes out of the stream, reading packets as it needs them.
    fn copy(&mut self, n: usize, out: &mut Vec<u8>) -> io::Result<()> {
        let mut left = n;
        while left > 0 {
            if self.reading.at == self.reading.buffer.len() {
                self.fill()?;
                continue;
            }
            let take = left.min(self.reading.buffer.len() - self.reading.at);
            let at = self.reading.at;
            out.extend_from_slice(&self.reading.buffer[at..at + take]);
            self.reading.at += take;
            left -= take;
        }
        Ok(())
    }

    /// Steps over `n` bytes, reading packets as it needs them.
    fn skip(&mut self, n: usize) -> io::Result<()> {
        let mut left = n;
        while left > 0 {
            if self.reading.at == self.reading.buffer.len() {
                self.fill()?;
                continue;
            }
            let take = left.min(self.reading.buffer.len() - self.reading.at);
            self.reading.at += take;
            left -= take;
        }
        Ok(())
    }
}

/// Reads a statement's answer as far as its shape: the tokens in front of the
/// first row, and the `COLMETADATA` that says what a row is.
///
/// A free function generic in the stream for [`login`]'s reason: a method on
/// [`TdsConn`] could only be reached through a real socket and a real
/// certificate. The request is already on the wire when this is called — which
/// of § 1's two ways it was written is the caller's business, and this reads the
/// same tokens either way.
///
/// **The answer borrows nothing**, which is what makes it serve both halves of
/// `rule:core-classes/db-statement-members`: [`open_result`] parks the cursor on the connection for
/// `Core\Db\Connection::stream`, and [`read_rows`] puts it beside a borrow for
/// every buffered member.
///
/// # Errors
///
/// An `Other` carrying a [`ServerError`] for a statement the server refused;
/// `InvalidData` for an answer that is not a token stream and for one that ends
/// without either a `COLMETADATA` or a `DONE`; and whatever the stream reported.
/// Everything but the refusal poisons the connection.
pub(super) fn read_shape<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: Option<&mut StatementCache<TdsPlan>>,
    span: QuerySpan,
    filing: Option<PendingPlan>,
) -> io::Result<TdsCursor> {
    state.set(State::Streaming);
    let mut reading = TdsCursor {
        columns: Vec::new(),
        buffer: Vec::new(),
        at: 0,
        last: false,
        rows: 0,
        counted: None,
        ended: false,
        returned: None,
        filing,
        span,
    };
    let shaped = Walk {
        wire,
        state,
        cache,
        reading: &mut reading,
    }
    .shape();
    match shaped {
        Ok(()) => Ok(reading),
        Err(e) => Err(poison_on_read(state, e)),
    }
}

/// [`read_shape`] with the read state lent out beside a borrow of the
/// connection: the shape every buffered member of `rule:core-classes/db-statement-members` wants,
/// and the one `Core\Db\Connection::stream` is the single caller that cannot
/// use.
///
/// # Errors
///
/// As [`read_shape`].
pub fn read_rows<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    span: QuerySpan,
) -> io::Result<TdsRows<'a, S>> {
    let reading = read_shape(wire, state, None, span, None)?;
    Ok(TdsRows::over(wire, state, None, reading))
}

impl<S: Read + Write> Walk<'_, S> {
    /// The tokens up to and including the `COLMETADATA`, or the `DONE` of a
    /// statement that has no result set to describe.
    fn shape(&mut self) -> io::Result<()> {
        let mut refusal = None;
        loop {
            match self.token()? {
                Some(Token::Columns(columns)) => {
                    self.reading.columns = columns;
                    return Ok(());
                }
                // [`TdsRows::step`]'s arm, and this is where a § 7 command's
                // own answer is read: a `BEGIN TRANSACTION` has no result set,
                // so its `ENVCHANGE` arrives before the `DONE` here.
                Some(Token::Env(EnvChange::Transaction { to })) => self.wire.set_descriptor(to),
                Some(Token::Info(_) | Token::Env(_) | Token::ReturnStatus(_) | Token::Order) => {}
                Some(Token::ReturnValue(returned)) => self.reading.returned = Some(returned),
                Some(Token::Error(message)) => {
                    if refusal.is_none() {
                        refusal = Some(message);
                    }
                }
                Some(Token::Done(done)) => {
                    if done.counted() {
                        self.reading.counted = Some(done.rows);
                    }
                    // [`TdsRows::step`]'s test, for its reasons.
                    if done.more() || done.in_proc {
                        continue;
                    }
                    self.end(refusal.is_some())?;
                    return match refusal {
                        Some(message) => Err(server_refusal(&message)),
                        None => Ok(()),
                    };
                }
                Some(Token::LoginAck(_)) => {
                    return Err(malformed(String::from(
                        "a TDS statement was answered with a LOGINACK, which is a stream out of \
                         sync rather than a login",
                    )));
                }
                None => return Err(no_done()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    #[test]
    fn a_row_is_read_back_as_the_values_its_columns_measure() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        assert_eq!(rows.columns().len(), 4);
        assert_eq!(rows.column_type(0), Some(ColumnType::Int));
        assert_eq!(rows.column_type(4), None);
        assert_eq!(rows.affected(), None, "the stream has not ended yet");

        let read = drain_rows(&mut rows).expect("one row and a DONE");
        assert_eq!(read.len(), 1);
        let row = &read[0];
        assert_eq!(row.len(), 4);
        assert_eq!(row.column(0), Some(Some(&7i32.to_le_bytes()[..])));
        assert_eq!(row.column(1), Some(Some(&b"abc"[..])));
        assert_eq!(
            row.column(2),
            Some(None),
            "a BYTELEN length of zero is NULL"
        );
        assert_eq!(row.column(3), Some(Some(&b"hello"[..])));
        assert_eq!(row.column(4), None, "the row has four columns");

        // A result set's `affected` is the rows that came back, which is the
        // number `crate::MySqlRows::affected` answers for the same statement.
        assert_eq!(rows.affected(), Some(1));
        assert_eq!(rows.span().rows(), 1);
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_row_cut_at_any_offset_is_read_back_as_one_row() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        // Three bytes a packet cuts every field of every token, the COLMETADATA
        // and the DONE included — the case a reader holding no remainder of its
        // own cannot answer at all.
        let mut wire = answering(&chunked(&payload, 3));
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain_rows(&mut rows).expect("one row and a DONE");

        assert_eq!(read.len(), 1);
        assert_eq!(read[0].column(0), Some(Some(&7i32.to_le_bytes()[..])));
        assert_eq!(read[0].column(1), Some(Some(&b"abc"[..])));
        assert_eq!(read[0].column(3), Some(Some(&b"hello"[..])));
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn an_nbcrows_bitmap_is_the_null_of_a_column_that_sent_no_bytes() {
        let mut payload = four_columns();
        // Columns 1 and 3 are in the bitmap and carry nothing at all, where the
        // ROW form would have sent each of them a length.
        payload.extend_from_slice(&nbc_row_token(
            &[false, true, false, true],
            &[7i32.to_le_bytes().to_vec(), byte_value(&[9, 0, 0, 0])],
        ));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain_rows(&mut rows).expect("one row and a DONE");

        assert_eq!(read[0].column(0), Some(Some(&7i32.to_le_bytes()[..])));
        assert_eq!(read[0].column(1), Some(None));
        assert_eq!(read[0].column(2), Some(Some(&[9, 0, 0, 0][..])));
        assert_eq!(read[0].column(3), Some(None));
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_plp_value_is_its_chunks_joined_and_the_empty_one_ends_them() {
        let columns = col_metadata(&[
            column(
                &char_type(TY_BIGVARCHAR, NO_LENGTH),
                "known",
                COLUMN_NULLABLE,
            ),
            column(
                &binary_type(TY_BIGVARBINARY, NO_LENGTH),
                "unknown",
                COLUMN_NULLABLE,
            ),
            column(&char_type(TY_NVARCHAR, NO_LENGTH), "empty", COLUMN_NULLABLE),
            column(
                &binary_type(TY_BIGVARBINARY, NO_LENGTH),
                "absent",
                COLUMN_NULLABLE,
            ),
        ]);
        let mut payload = columns;
        payload.extend_from_slice(&row_token(&[
            plp_value(6, &[b"abc", b"def"]),
            plp_value(PLP_UNKNOWN, &[b"one", b"two", b"three"]),
            plp_value(0, &[]),
            plp_value(PLP_NULL, &[]),
        ]));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&chunked(&payload, 7));
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain_rows(&mut rows).expect("one row and a DONE");

        assert_eq!(read[0].column(0), Some(Some(&b"abcdef"[..])));
        assert_eq!(read[0].column(1), Some(Some(&b"onetwothree"[..])));
        assert_eq!(
            read[0].column(2),
            Some(Some(&b""[..])),
            "a value of no bytes is not a value that was not there"
        );
        assert_eq!(read[0].column(3), Some(None));
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_plp_value_that_disagrees_with_its_declared_total_is_refused() {
        let mut payload = col_metadata(&[column(
            &char_type(TY_BIGVARCHAR, NO_LENGTH),
            "text",
            COLUMN_NULLABLE,
        )]);
        payload.extend_from_slice(&row_token(&[plp_value(9, &[b"abc"])]));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain_rows(&mut rows).expect_err("nine bytes were promised and three sent");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert_eq!(state.get(), State::Poisoned);
    }

    #[test]
    fn a_nulltype_column_is_null_and_carries_nothing() {
        let mut payload = col_metadata(&[
            column(&[TY_NULL], "nothing", COLUMN_NULLABLE),
            column(&[TY_INT4], "id", 0),
        ]);
        payload.extend_from_slice(&row_token(&[Vec::new(), 7i32.to_le_bytes().to_vec()]));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain_rows(&mut rows).expect("one row and a DONE");

        assert_eq!(read[0].column(0), Some(None));
        assert_eq!(read[0].column(1), Some(Some(&7i32.to_le_bytes()[..])));
    }

    #[test]
    fn a_statement_with_no_result_set_is_already_ended_and_reports_what_it_counted() {
        let mut payload = env_token(ENV_DATABASE, "novis_test", "master");
        payload.extend_from_slice(&message_token(
            TOKEN_INFO,
            5701,
            0,
            "Changed database context.",
        ));
        payload.extend_from_slice(&done_token(DONE_COUNT, 4));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("an answer with no result set");

        assert!(rows.columns().is_empty());
        assert_eq!(
            rows.affected(),
            Some(4),
            "the server's own count, not the rows"
        );
        assert!(
            rows.next_row().expect("an ended stream").is_none(),
            "there is nothing to read and the wire is at a boundary"
        );
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_done_that_counted_nothing_reports_no_count_rather_than_zero() {
        let mut wire = answering(&[done_token(0, 99)]);
        let state = Cell::new(State::Executing);
        let rows = read_rows(&mut wire, &state, span()).expect("an answer with no result set");

        assert_eq!(
            rows.affected(),
            None,
            "the row count without the count bit is a number the server did not mean"
        );
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_refused_statement_is_read_to_its_done_and_leaves_the_connection_idle() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&message_token(
            TOKEN_ERROR,
            8134,
            16,
            "Divide by zero error encountered.",
        ));
        payload.extend_from_slice(&done_token(DONE_ERROR, 0));

        let mut wire = answering(&chunked(&payload, 11));
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain_rows(&mut rows).expect_err("the server refused the statement");

        let error = ServerError::of(&refused).expect("the server's own refusal");
        assert_eq!(error.driver_code, Some(8134));
        assert_eq!(error.kind, DbErrorKind::Other);
        assert_eq!(
            state.get(),
            State::Idle,
            "the ERROR's DONE was read, so the wire is at a boundary"
        );
    }

    #[test]
    fn a_second_result_set_is_refused_rather_than_left_on_the_wire() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_token(DONE_MORE | DONE_COUNT, 1));
        payload.extend_from_slice(&four_columns());
        payload.extend_from_slice(&done_token(DONE_COUNT, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain_rows(&mut rows)
            .expect_err("`rule:core-classes/db-statement-members` has nowhere to put the second");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("second result set"));
        assert_eq!(state.get(), State::Poisoned);
    }

    #[test]
    fn an_answer_whose_last_packet_ends_mid_token_is_refused() {
        let mut payload = four_columns();
        let mut cut = four_values();
        cut.truncate(cut.len() - 3);
        payload.extend_from_slice(&cut);

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain_rows(&mut rows).expect_err("the row's last value never arrived");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert_eq!(state.get(), State::Poisoned);
    }

    #[test]
    fn a_value_wider_than_its_column_declared_is_refused_before_it_is_read() {
        let mut payload = col_metadata(&[column(
            &char_type(TY_BIGVARCHAR, 4),
            "name",
            COLUMN_NULLABLE,
        )]);
        payload.extend_from_slice(&row_token(&[short_value(b"abcdefgh")]));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain_rows(&mut rows).expect_err("eight bytes in a varchar(4)");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(
            refused
                .to_string()
                .contains("past the 4 its COLMETADATA declared")
        );
        assert_eq!(state.get(), State::Poisoned);
    }

    #[test]
    fn every_row_of_a_stream_is_counted_once_on_its_span() {
        let mut payload = four_columns();
        for _ in 0..3 {
            payload.extend_from_slice(&four_values());
        }
        payload.extend_from_slice(&done_token(DONE_COUNT, 3));

        let mut wire = answering(&chunked(&payload, 16));
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain_rows(&mut rows).expect("three rows and a DONE");

        assert_eq!(read.len(), 3);
        assert_eq!(rows.span().rows(), 3);
        assert_eq!(rows.affected(), Some(3));
        assert!(rows.span().duration() > std::time::Duration::ZERO);
    }

    #[test]
    fn a_statement_the_server_refused_before_describing_anything_is_the_servers_error() {
        let mut payload = message_token(TOKEN_ERROR, 208, 16, "Invalid object name 'nope'.");
        payload.extend_from_slice(&done_token(DONE_ERROR, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let refused = read_rows(&mut wire, &state, span()).expect_err("the server refused it");

        let error = ServerError::of(&refused).expect("the server's own refusal");
        assert_eq!(error.driver_code, Some(208));
        assert!(error.sql_state.is_empty(), "TDS sends no SQLSTATE");
        assert_eq!(state.get(), State::Idle);
    }
}
