//! The statement path: [ADR 0067 § 1](/docs/decisions/0067.md)'s
//! plan cache, the transaction commands, and the session reset.
//!
//! [`TdsPlan`] is why the cache holds more than a handle. A plan was prepared
//! against a *declaration* — the widths [`mod@super::rpc`] wrote — so a hit
//! whose declaration is narrower than this call needs is unprepared and
//! prepared again rather than reused, which is the one place this driver's
//! cache differs from [`crate::mysql`]'s.

use super::*;

/// One plan this connection has the server holding, as [ADR 0067
/// § 1](/docs/decisions/0067.md)'s cache records it.
///
/// The handle alone would be [`crate::mysql::Prepared`]'s twin. It is not
/// enough here, and the second field is why.
#[derive(Debug, Clone)]
pub struct TdsPlan {
    /// The number `sp_execute` and `sp_unprepare` name — the `@handle`
    /// `sp_prepexec` wrote back.
    pub handle: i32,
    /// The `@params` declaration this plan was compiled against, empty for a
    /// statement that binds nothing.
    ///
    /// **§ 1's key is one component short on this protocol.** [`declarations`]
    /// widens a marker to `nvarchar(max)` for a value past [`NVARCHAR_CHARS`]
    /// and declares it `varbinary` for a value [`encode`] marked binary, so one
    /// statement run first with a short text value and then with a long one or
    /// with a `bytes` wants two different plans under a key — the SQL text and
    /// the arity — that cannot tell them apart. A long value bound against a
    /// plan declared narrow is *truncated* by SQL Server rather than refused,
    /// which is silent data loss, so this is carried and compared and a
    /// mismatch is a miss that unprepares the plan it did not fit. The key
    /// itself is left alone: § 1
    /// states it once, for every driver, and this is one driver's reason to
    /// reject a hit rather than another way to spell the key.
    ///
    /// `Rc<str>` because [`StatementCache::lookup`] clones the handle on every
    /// hit, and a hit is the path the cache exists for.
    pub(super) declared: Rc<str>,
}

/// [ADR 0067 §§ 1 and 4](/docs/decisions/0067.md)'s one statement,
/// end to end: the RPC out, and the token stream that answers it.
///
/// [`crate::mysql::start_statement`]'s shape and its reasons — free and generic
/// in the stream so a unit test can script a server for it, and taking the busy
/// state by reference so § 4's one-statement-at-a-time rule is enforced here
/// rather than by each caller remembering to.
///
/// **§ 1's cache decides which of two requests goes out.** A hit sends
/// [`execute_request`] — the handle and the values, no SQL — and files nothing,
/// since the plan is already recorded. A miss sends [`prepexec_request`] and
/// hands the walk a [`PendingPlan`], because the handle it will be recorded
/// under arrives in a `RETURNVALUE` after the rows. An eviction and a rejected
/// hit both send [`unprepare_request`] *first*, so the server never holds more
/// plans than `statement_cache` allows, not even for the length of one round
/// trip — `crate::mysql`'s `cached_statement` ordering, for its reason.
///
/// The answer is the read state alone, which is what lets `rule:core-classes/db-streaming`'s
/// `stream` park it on the connection ([`TdsConn::stream`]);
/// [`start_statement`] is this with a borrow around it.
///
/// # Errors
///
/// `InvalidInput` for a statement written to a connection that is not idle and
/// for [`bind`]'s and [`text_param`]'s refusals — none of which touches the
/// wire, so none poisons the connection; otherwise as [`read_shape`], including
/// for an eviction's own answer. A write that failed part-way leaves the
/// connection [`State::Poisoned`], because a half-written packet is not a
/// boundary anything can be found from.
pub(crate) fn open_result<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: &mut StatementCache<TdsPlan>,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<TdsCursor> {
    if !state.get().may_start_statement() {
        return Err(crate::pg::second_statement(state));
    }

    // `rule:observability/a-query-is-a-trace-event`'s span, opened before the request goes out and handed `sql`
    // and never `params` — `crate::span`'s module doc owns why that is a
    // signature rather than a rule.
    let span = QuerySpan::opened(Driver::SqlServer, sql);
    let bound = bind(params)?;
    let declared = declarations(&bound);
    let declared: Rc<str> = Rc::from(declared.as_deref().unwrap_or_default());

    // § 7's open transaction, read once: a statement is one request or two, and
    // the descriptor cannot move between them — nothing here sends a `BEGIN`,
    // and the eviction below runs on the same transaction the execution does.
    let descriptor = wire.descriptor();

    let mut stale = None;
    match cache.lookup(sql, params.len()) {
        Some(plan) if plan.declared == declared => {
            let request = execute_request(plan.handle, &bound, descriptor)?;
            send_request(wire, state, PacketType::Rpc, Status::NORMAL, &request)?;
            return read_shape(wire, state, None, span, None);
        }
        // A hit whose plan was compiled against a different declaration — see
        // `TdsPlan::declared`. The entry is dropped rather than shadowed so the
        // cache never holds two under one key, and the plan is unprepared
        // because nothing else will ever name it again.
        Some(plan) => {
            cache.forget(sql, params.len());
            stale = Some(plan.handle);
        }
        None => {}
    }

    // Built before anything is written, so a value this driver will not send
    // costs neither an eviction nor a byte on the wire.
    let request = prepexec_request(sql, &bound, cache_declaration(&declared), descriptor)?;
    if let Some(handle) = stale.or_else(|| cache.make_room().map(|plan| plan.handle)) {
        send_request(
            wire,
            state,
            PacketType::Rpc,
            Status::NORMAL,
            &unprepare_request(handle, descriptor),
        )?;
        drain(wire, state)?;
    }
    send_request(wire, state, PacketType::Rpc, Status::NORMAL, &request)?;
    read_shape(
        wire,
        state,
        Some(cache),
        span,
        Some(PendingPlan {
            sql: sql.to_owned(),
            arity: params.len(),
            declared,
        }),
    )
}

/// [`open_result`], with the read state lent out beside a borrow of the
/// connection: `rule:core-classes/db-statement-members`'s buffered members, which is every one of
/// them but `stream`.
///
/// The cache travels into the borrow because the plan is filed when the answer
/// *ends*, which on this protocol is after the rows — [`PendingPlan`] owns that
/// whole ordering.
///
/// # Errors
///
/// As [`open_result`].
pub fn start_statement<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    cache: &'a mut StatementCache<TdsPlan>,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<TdsRows<'a, S>> {
    let reading = open_result(wire, state, cache, sql, params)?;
    Ok(TdsRows::over(wire, state, Some(cache), reading))
}

/// The `@params` a declaration string stands for: `None` where it is empty,
/// which is the statement that binds nothing and which the procedure reads as a
/// plan with no parameters where an empty string would not.
pub(super) fn cache_declaration(declared: &Rc<str>) -> Option<&str> {
    (!declared.is_empty()).then(|| &**declared)
}

/// [ADR 0067 § 4](/docs/decisions/0067.md)'s `executeMany` on this
/// protocol: one plan, one execution per set, and the affected counts summed.
///
/// [`crate::mysql::execute_many`]'s loop, and every rule that function argues
/// holds here for the same reason — an execution is its own transaction, so a
/// refusal leaves the writes before it standing and the sets after it are still
/// attempted; the **first** error is what the batch reports; and a wire failure
/// is the one thing that ends it early, a poisoned connection having no packet
/// boundary left for the next set to be written at.
///
/// **The bulk command § 4 refuses has no spelling here at all.** What makes the
/// batch cheaper than N `execute` calls on this driver is § 1's cache and
/// nothing else: the first set is an `sp_prepexec` and every set after it an
/// `sp_execute` naming the plan that prepare filed, which is the one prepare
/// and N executions § 4 asks for, arrived at through [`start_statement`] rather
/// than through a second request shape.
///
/// An empty `sets` is § 4's no-op answering `0`, with the busy check still
/// ahead of it for [`crate::pg`]'s reason: § 4's refusal is a property of the
/// connection and not of the payload.
///
/// # Errors
///
/// `InvalidInput` for a batch written to a connection that is not idle, and for
/// a `sets` whose members do not all bind the same number of parameters — one
/// prepare has one parameter count, and it is what § 1's cache is keyed on
/// beside the SQL text. Otherwise the first error any execution drew, or the
/// wire failure that stopped the batch.
pub fn execute_many<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: &mut StatementCache<TdsPlan>,
    sql: &str,
    sets: &[&[Option<&[u8]>]],
) -> io::Result<u64> {
    if !state.get().may_start_statement() {
        return Err(crate::pg::second_statement(state));
    }

    let Some(first) = sets.first() else {
        return Ok(0);
    };
    let arity = first.len();
    if let Some(odd) = sets.iter().find(|set| set.len() != arity) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "one executeMany bound {arity} parameters in its first set and {} in another, and \
                 `rule:core-classes/db-statement-members`'s one prepare has one parameter count",
                odd.len()
            ),
        ));
    }

    let mut affected = 0_u64;
    let mut refused: Option<io::Error> = None;
    for set in sets {
        match execute_one(wire, state, cache, sql, set) {
            Ok(count) => affected += count,
            Err(e) => {
                if state.get() == State::Poisoned {
                    return Err(e);
                }
                refused.get_or_insert(e);
            }
        }
    }

    match refused {
        Some(e) => Err(e),
        None => Ok(affected),
    }
}

/// One of [`execute_many`]'s sets, drained, and what it changed.
///
/// [`crate::mysql`]'s helper of the same name, split out for its reason: a
/// [`TdsRows`] borrows the wire, so the count has to be read before the stream
/// is dropped and the batch's accounting is a `match` on one result rather than
/// a stream held across the next iteration.
///
/// A set that answered with rows contributes the rows it produced, which is the
/// number [`TdsRows::affected`] already reports for one — § 4 gives the batch
/// one sum and that method is where the two numbers became one. A `DONE` that
/// counted nothing reads as `0` here, the distinction it draws being one a sum
/// has nothing to do with.
///
/// # Errors
///
/// As [`start_statement`] and [`TdsRows::next_row`].
pub(super) fn execute_one<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: &mut StatementCache<TdsPlan>,
    sql: &str,
    set: &[Option<&[u8]>],
) -> io::Result<u64> {
    let mut rows = start_statement(wire, state, cache, sql, set)?;
    while rows.next_row()?.is_some() {}
    Ok(rows.affected().unwrap_or(0))
}

/// [ADR 0067 § 7](/docs/decisions/0067.md)'s `BEGIN TRANSACTION`, or
/// the `SAVE TRANSACTION` a nested `transaction()` is.
///
/// [`crate::mysql::begin`]'s shape and its depth accounting, with T-SQL's
/// spellings and the facts that are this backend's alone.
///
/// **SQL Server has no read-only transaction at all**, so `read_only` is
/// refused at any depth rather than dropped. Every other backend § 7 reaches
/// enforces the option, and a driver that accepted it here and opened an
/// ordinary writable transaction would hand a program the word without the
/// guarantee — the one failure mode a `readOnly` exists to prevent.
///
/// **`SET TRANSACTION ISOLATION LEVEL` is session-scoped here**, where MySQL's
/// applies to the next transaction and PostgreSQL's rides the `BEGIN`. A level
/// therefore outlives the transaction that asked for it and would become the
/// level of every later statement on the connection, so putting it back is this
/// driver's own work: [`TdsConn::isolation_moved`](crate::conn::TdsConn) records
/// that a restore is owed, [`commit`] and [`roll_back`] pay it when the
/// outermost level closes, and an outermost `begin` asking for no level pays it
/// first. The second is not redundant — a `COMMIT TRANSACTION` the server
/// refused ends the transaction with the restore still owed — and § 13's
/// `sp_reset_connection` covers only the *pool*, not a second `transaction()`
/// in the same request.
///
/// The flag is set **before** the `SET` is written rather than after it lands:
/// what it records is that a restore may be owed, and a `SET` that failed on
/// the wire has not been proven not to have reached the server.
///
/// # Errors
///
/// `InvalidInput` for a `read_only` at any depth and for a nested call asking
/// for an isolation level, otherwise as [`simple_command`]. The depth moves only
/// after a command the server accepted, so a refused begin leaves the connection
/// at the level it had. A `BEGIN TRANSACTION` refused after its `SET` landed
/// poisons the connection, as [`crate::mysql::begin`] does and for a sharper
/// reason: the session is sitting at a level no transaction is going to end.
pub(crate) fn begin<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    depth: &Cell<u32>,
    moved: &Cell<bool>,
    isolation: Option<Isolation>,
    read_only: bool,
) -> io::Result<QuerySpan> {
    if read_only {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a transaction asked to be read-only and SQL Server has no read-only transaction: \
             drop the option, or take the guarantee where this backend really offers one — a \
             login without write permission, or a read-only replica named by its own \
             `[db.<name>]` block",
        ));
    }

    let open = depth.get();
    if open > 0 {
        if isolation.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "a transaction nested {open} deep asked for its own isolation level, and SQL \
                     Server settles one for the whole session: ask for it on the outermost \
                     `transaction()`, or give this one a `{{shared: false}}` connection of its own"
                ),
            ));
        }
        let command = format!("SAVE TRANSACTION {}", crate::pg::savepoint_name(open));
        let span = simple_command(wire, state, &command)?;
        depth.set(open + 1);
        return Ok(span);
    }

    let Some(level) = isolation else {
        // The restore a refused outermost commit left owed, before this
        // transaction inherits a level nobody asked it for.
        restore_isolation(wire, state, moved)?;
        let span = simple_command(wire, state, BEGIN_TRANSACTION)?;
        depth.set(1);
        return Ok(span);
    };

    let set = isolation_command(level);
    let mut span = QuerySpan::opened(Driver::SqlServer, &format!("{set}; {BEGIN_TRANSACTION}"));
    moved.set(true);
    batch_command(wire, state, set)?;
    if let Err(refused) = batch_command(wire, state, BEGIN_TRANSACTION) {
        state.set(State::Poisoned);
        return Err(refused);
    }
    span.finished(None);
    depth.set(1);
    Ok(span)
}

/// The `SET TRANSACTION ISOLATION LEVEL` one of § 7's levels renders to.
///
/// **Nothing collapses here, and this is the backend [`Isolation::Snapshot`] is
/// named after**: SQL Server implements it as a level of its own rather than as
/// a spelling of `REPEATABLE READ`, which is what the other row-versioning
/// drivers fold it onto. A database with `ALLOW_SNAPSHOT_ISOLATION` off refuses
/// the command, and that refusal is § 7's "throwing where a driver lacks the
/// level" arriving as the server's own error rather than as a guess this driver
/// made about the database's settings.
pub(super) fn isolation_command(level: Isolation) -> &'static str {
    match level {
        Isolation::ReadUncommitted => "SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED",
        Isolation::ReadCommitted => DEFAULT_ISOLATION,
        Isolation::RepeatableRead => "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ",
        Isolation::Snapshot => "SET TRANSACTION ISOLATION LEVEL SNAPSHOT",
        Isolation::Serializable => "SET TRANSACTION ISOLATION LEVEL SERIALIZABLE",
    }
}

/// Puts the session's isolation level back at [`DEFAULT_ISOLATION`], where a
/// transaction that asked for one moved it.
///
/// Costs nothing at all when no transaction asked: the flag is false and no
/// message goes out, which is every transaction a program did not give an
/// `{isolation}` to. [`begin`] owns why the flag exists and when it is paid.
///
/// # Errors
///
/// As [`batch_command`]. The flag survives a failure, so the restore is still
/// owed and the next outermost [`begin`] attempts it again.
pub(super) fn restore_isolation<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    moved: &Cell<bool>,
) -> io::Result<()> {
    if !moved.get() {
        return Ok(());
    }
    batch_command(wire, state, DEFAULT_ISOLATION)?;
    moved.set(false);
    Ok(())
}

/// § 7's `COMMIT TRANSACTION`, or the nested commit that has nothing to send.
///
/// **T-SQL has no `RELEASE SAVEPOINT`**, and there is nothing to send in its
/// place: a `COMMIT TRANSACTION` inside a nested level would commit the *whole*
/// transaction, `@@TRANCOUNT` being 1 for a nesting this driver spells as
/// `SAVE TRANSACTION`. So a nested commit moves the depth and costs no round
/// trip, and the savepoint it leaves behind is released by the outermost commit
/// along with every other one. Its span therefore carries no SQL text — § 11's
/// event says a commit happened and that nothing went out, which is what
/// happened, and naming a command the wire never saw would be the lie a trace
/// exists to prevent.
///
/// # Errors
///
/// `InvalidInput` for a connection in no transaction, otherwise as
/// [`simple_command`]. **The count follows the connection where the server took
/// it**, [`crate::mysql::commit`]'s rule and its reasoning in full: an outermost
/// commit the server refused has already rolled the transaction back, so the
/// depth goes to 0 and § 7's `{retries: n}` opens the next attempt with a
/// `BEGIN TRANSACTION` rather than a `SAVE TRANSACTION` against nothing. That
/// path leaves the isolation restore owed rather than sending it, so the error
/// the caller sees is the server's own — [`begin`] pays it instead.
pub(crate) fn commit<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    depth: &Cell<u32>,
    moved: &Cell<bool>,
) -> io::Result<QuerySpan> {
    let open = crate::pg::open_transaction(depth, "commit")?;
    if open > 1 {
        depth.set(open - 1);
        let mut span = QuerySpan::opened(Driver::SqlServer, "");
        span.finished(None);
        return Ok(span);
    }

    let span = match simple_command(wire, state, COMMIT_TRANSACTION) {
        Ok(span) => span,
        Err(refused) => {
            if ServerError::of(&refused).is_some() {
                depth.set(0);
            }
            return Err(refused);
        }
    };
    depth.set(0);
    restore_isolation(wire, state, moved)?;
    Ok(span)
}

/// § 7's `ROLLBACK TRANSACTION`, or the `ROLLBACK TRANSACTION <name>` that
/// undoes a nested level.
///
/// **One command where [`crate::pg`]'s nested rollback is two**, for
/// [`crate::mysql::roll_back`]'s reason and under T-SQL's own savepoint rule: a
/// second `SAVE TRANSACTION` of a name already used is what a later
/// `ROLLBACK TRANSACTION` of that name returns to, so re-opening a level reuses
/// `nvs_1` rather than adding to it, and there is no release to pay for. There
/// is no `RELEASE` in this dialect to pay it with either — see [`commit`].
///
/// # Errors
///
/// `InvalidInput` for a connection in no transaction, otherwise as
/// [`simple_command`]. A refused rollback leaves the depth where it was: the
/// level is still open as far as the server is concerned, and the level above it
/// rolls back over this one anyway. The isolation restore rides the outermost
/// rollback exactly as it rides the outermost commit.
pub(crate) fn roll_back<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    depth: &Cell<u32>,
    moved: &Cell<bool>,
) -> io::Result<QuerySpan> {
    let open = crate::pg::open_transaction(depth, "roll back")?;
    let command: Cow<'_, str> = if open == 1 {
        Cow::Borrowed(ROLLBACK_TRANSACTION)
    } else {
        Cow::Owned(format!(
            "{ROLLBACK_TRANSACTION} {}",
            crate::pg::savepoint_name(open - 1)
        ))
    };

    let span = simple_command(wire, state, &command)?;
    depth.set(open - 1);
    if open == 1 {
        restore_isolation(wire, state, moved)?;
    }
    Ok(span)
}

/// One of § 7's commands, sent as a `SQL_BATCH` message, with
/// [ADR 0067 § 11](/docs/decisions/0067.md)'s span around it.
///
/// **A batch rather than § 1's prepared statements**, which is where the two
/// halves of § 1 stop pulling together — [`crate::mysql::simple_command`]'s
/// reasoning, and one more that is this protocol's: an `sp_prepexec` of
/// `BEGIN TRANSACTION` would file a plan in a cache sized for the request's real
/// statements, to run a command of two words that binds nothing.
///
/// The span carries no count. § 7's commands change no rows themselves, and
/// `affected` says "this statement reported a count" rather than "it reported
/// zero".
///
/// # Errors
///
/// As [`batch_command`].
pub(super) fn simple_command<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    sql: &str,
) -> io::Result<QuerySpan> {
    let mut span = QuerySpan::opened(Driver::SqlServer, sql);
    batch_command(wire, state, sql)?;
    span.finished(None);
    Ok(span)
}

/// One text sent as a `SQL_BATCH` message and its answer read to the end.
///
/// Reading to the end is not optional here for [`drain`]'s reason: every TDS
/// request has an answer, and one left on the wire is read as the next
/// statement's.
///
/// # Errors
///
/// `InvalidInput` for a command written to a connection that is not idle — § 4's
/// rule, which is a property of the connection and not of what is being sent —
/// otherwise as [`read_rows`]. A write that failed part-way leaves the
/// connection [`State::Poisoned`], because a half-written packet is not a
/// boundary anything can be found from.
pub(super) fn batch_command<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    sql: &str,
) -> io::Result<()> {
    if !state.get().may_start_statement() {
        return Err(crate::pg::second_statement(state));
    }
    let request = batch_request(sql, wire.descriptor());
    send_request(wire, state, PacketType::SqlBatch, Status::NORMAL, &request)?;
    drain(wire, state)
}

/// [ADR 0067 § 13](/docs/decisions/0067.md)'s reset, and the cache it
/// takes with it.
///
/// **`sp_reset_connection` as [`Status::RESET_CONNECTION`] on a message of its
/// own**, which is MS-TDS's own spelling of that procedure and is why nothing
/// here names it: the bit resets the session before the message carrying it is
/// processed, and [`RESET_STATEMENT`] is the smallest well-formed message there
/// is to carry it. Its answer is what proves the reset landed, which is what
/// § 13 asks of a reset and what a bit riding the *next* request could not give
/// — that request already belongs to the program the connection was handed to.
///
/// [`crate::mysql::reset_session`]'s twin with one half missing and one half
/// this backend alone owes. TDS has no session time zone to send again,
/// [`TdsTarget::time_zone`] owns why. The cache is emptied here rather than by
/// whoever pools the connection, for that function's reason — the plans are
/// gone from the server the moment this answers, and a cache still naming them
/// would bind the next request against handles this session no longer has.
///
/// **The isolation level is put back by this driver, because the procedure does
/// not put it back.** That is measured rather than assumed: the § 13 case in
/// `crates/nvs-db/tests/handshake.rs` reads `sys.dm_exec_sessions` after a reset
/// and finds the level the last `transaction()` asked for still in force. § 13
/// states the reset as a *property* — after it, no session state the next
/// request could observe — and names `sp_reset_connection` as the means; where
/// the means falls short of the property, the property is what has to hold. The
/// restore costs a round trip only for a session an isolation level actually
/// moved, which is what [`TdsConn::isolation_moved`](crate::conn::TdsConn)
/// records.
///
/// Free and generic in the stream for this crate's usual reason: a
/// `Wire<NvsTls<Tunnel<NvsTcp>>>` needs a socket and a certificate that no unit
/// test has.
///
/// # Errors
///
/// As [`read_rows`], for the batch the bit rode in on. § 13 destroys the
/// connection on any of them, so what the cache holds on that path is nobody's
/// business.
pub fn reset_session<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: &mut StatementCache<TdsPlan>,
    moved: &Cell<bool>,
) -> io::Result<()> {
    // The one request that names no transaction however deep the session was.
    // [`Status::RESET_CONNECTION`] is honoured *before* the request it rides is
    // processed, so whatever § 7 had open is already rolled back by the time
    // this message's `ALL_HEADERS` is read, and a descriptor naming it would
    // name a transaction the server no longer has. Cleared before the request
    // is built rather than after the answer, so a reset that failed leaves
    // nothing behind either: § 13 destroys the connection on any error here.
    wire.set_descriptor(NO_TRANSACTION);
    send_request(
        wire,
        state,
        PacketType::SqlBatch,
        Status::RESET_CONNECTION,
        &batch_request(RESET_STATEMENT, NO_TRANSACTION),
    )?;
    drain(wire, state)?;
    cache.clear();
    // The one piece of session state `sp_reset_connection` leaves standing, and
    // the server is what says so: `crates/nvs-db/tests/handshake.rs`'s
    // `a_mssql_reset_from_inside_a_transaction_leaves_none_and_the_logins_level`
    // reads `sys.dm_exec_sessions` after a reset and finds the level the last
    // `transaction()` asked for still in force. § 13 states the reset as a
    // *property* rather than as a command list for exactly this reason — a
    // pooled connection whose level one request moved would silently run the
    // next request's statements at it — so the restore this driver already owes
    // ([`begin`]) is paid here as well, and only where one is owed.
    restore_isolation(wire, state, moved)
}

/// Writes one request and says where a failed write leaves the connection.
///
/// # Errors
///
/// Whatever the write reported, with the connection [`State::Poisoned`]: a
/// half-written packet is not a boundary anything can be found from.
pub(super) fn send_request<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    kind: PacketType,
    status: Status,
    request: &[u8],
) -> io::Result<()> {
    state.set(State::Executing);
    if let Err(e) = wire.send(kind, status, request) {
        state.set(State::Poisoned);
        return Err(e);
    }
    Ok(())
}

/// Reads an answer this driver sent for its own reasons to its end, and throws
/// it away.
///
/// The span it opens is dropped with the stream on purpose: § 11's trace event
/// is one statement a *program* ran, and neither an eviction nor a reset is
/// one. Reading to the end is not optional — a TDS answer left on the wire is
/// read as the next statement's.
///
/// # Errors
///
/// As [`read_rows`] and [`TdsRows::next_row`].
pub(super) fn drain<S: Read + Write>(wire: &mut Wire<S>, state: &Cell<State>) -> io::Result<()> {
    let span = QuerySpan::opened(Driver::SqlServer, "");
    let mut reading = read_shape(wire, state, None, span, None)?;
    while next_row_of(wire, state, None, &mut reading)?.is_some() {}
    Ok(())
}

/// The refusal for an answer the server stopped sending without ending it.
pub(super) fn no_done() -> io::Error {
    malformed(String::from(
        "a TDS answer ended without the DONE that says which statement finished and what it \
         counted",
    ))
}

/// A length the protocol writes as a `u32` or a `u64`, as an index into memory.
///
/// Fallible because both are wider than a `usize` on a 32-bit host, and a
/// `PLP` chunk is four bytes of the server's choosing: the cast that cannot
/// fail on this project's targets is still the cast that truncates on one of
/// them, and a truncated length reads the next value as part of this one.
pub(super) fn as_usize(length: impl TryInto<usize>) -> io::Result<usize> {
    length.try_into().map_err(|_| {
        malformed(String::from(
            "a TDS value declared a length this host cannot hold in memory",
        ))
    })
}

/// Where a failed read leaves the connection.
///
/// [`crate::mysql`]'s `poison_on_write` and its split, which its own doc owns: a
/// [`ServerError`] arrived whole and left the wire at a boundary, so the next
/// statement on it is fine, and anything else reached here with the stream in a
/// position nothing has proven.
pub(super) fn poison_on_read(state: &Cell<State>, error: io::Error) -> io::Error {
    if ServerError::of(&error).is_some() {
        state.set(State::Idle);
    } else {
        state.set(State::Poisoned);
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    /// `rule:security/db-pool-reset-is-a-boundary`'s reset on this backend, both halves of it: the session is
    /// reset through `sp_reset_connection` — which MS-TDS spells as a header bit
    /// rather than as a call — and § 1's cache is emptied with it, because the
    /// reset drops the server's prepared statements.
    ///
    /// The asymmetry § 13 names, from the other side: PostgreSQL's reset is a
    /// command list picked so the cache *survives*, and this one has no such
    /// choice to make. A cache that survived here would bind the next request
    /// against plan handles this session no longer has — a wrong answer rather
    /// than a slow one.
    #[test]
    fn mssql_resets_through_sp_reset_connection_and_loses_its_cache() {
        const SQL: &str = "select a";

        let mut wire = answering_each(&[
            prepexec_answer(9),
            done_token(DONE_COUNT, 1),
            prepexec_answer(11),
        ]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);

        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[])
                .expect("a statement the server answered"),
        );
        assert_eq!(cache.len(), 1);

        // No `transaction()` moved the level here, so the reset owes no restore
        // and sends the one message this case is counting.
        let moved = Cell::new(false);
        reset_session(&mut wire, &state, &mut cache, &moved)
            .expect("a reset the server acknowledged");
        assert!(
            cache.is_empty(),
            "§ 13: the reset drops every prepared statement, so a cache that \
             kept one is naming a plan the server does not have"
        );
        assert_eq!(
            state.get(),
            State::Idle,
            "the reset left the wire at a boundary"
        );

        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[])
                .expect("a statement the server answered"),
        );

        let sent = flushed(&wire.peer().sent);
        assert_eq!(sent.len(), 3);
        assert_eq!(sent[0].0, PacketType::Rpc);
        assert_eq!(sent[1].0, PacketType::SqlBatch);
        assert!(
            sent[1].1.contains(Status::RESET_CONNECTION),
            "the reset is the header bit, which is MS-TDS's own sp_reset_connection"
        );
        assert_eq!(sent[2].0, PacketType::Rpc);
        assert_eq!(
            sent_rpc(&sent[2].2).0,
            PROC_SP_PREPEXEC,
            "the same statement after a reset costs the prepare again"
        );
    }

    /// § 1's "cached re-executions cost one round trip", on this protocol: the
    /// second execution names the plan and carries the values, and no byte of
    /// the statement is on the wire.
    #[test]
    fn a_cached_statement_is_sent_as_sp_execute_with_no_sql() {
        const SQL: &str = "select a from t where b = @p1";

        let mut wire = answering_each(&[prepexec_answer(9), done_token(DONE_COUNT, 1)]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);

        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[Some(b"7")])
                .expect("the first execution, which prepares"),
        );
        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[Some(b"8")])
                .expect("the second, which does not"),
        );

        let sent = flushed(&wire.peer().sent);
        assert_eq!(
            sent.len(),
            2,
            "one message each, and the eviction sent none"
        );
        assert_eq!(sent_rpc(&sent[0].2).0, PROC_SP_PREPEXEC);

        let (proc_id, params) = sent_rpc(&sent[1].2);
        assert_eq!(proc_id, PROC_SP_EXECUTE);
        assert_eq!(
            params.len(),
            2,
            "the handle and the one value — no SQL, no @params"
        );
        assert_eq!(params[0].type_id, TY_INTN);
        assert!(
            !params[0].by_ref,
            "the handle is read here, never written back"
        );
        assert_eq!(params[0].text.as_deref(), Some("9"));
        assert_eq!(params[1].text.as_deref(), Some("8"));
        assert_eq!(cache.len(), 1, "one plan, executed twice");
    }

    /// § 1's capacity is what the *server* holds, so the eviction goes out
    /// before the prepare that needed the room and not after it.
    #[test]
    fn an_eviction_unprepares_before_the_prepare_that_needed_the_room() {
        let mut wire = answering_each(&[
            prepexec_answer(9),
            prepexec_answer(10),
            procedure_answer(),
            prepexec_answer(11),
        ]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);

        for sql in ["select 1", "select 2", "select 3"] {
            drop(
                start_statement(&mut wire, &state, &mut cache, sql, &[])
                    .expect("a statement the server answered"),
            );
        }

        let sent = flushed(&wire.peer().sent);
        assert_eq!(sent.len(), 4);
        let (proc_id, params) = sent_rpc(&sent[2].2);
        assert_eq!(proc_id, PROC_SP_UNPREPARE);
        assert_eq!(
            params[0].text.as_deref(),
            Some("9"),
            "the plan unprepared is the least recently used one, not the new one"
        );
        assert_eq!(sent_rpc(&sent[3].2).0, PROC_SP_PREPEXEC);
        assert_eq!(cache.len(), 2, "never more plans than the block allowed");
    }

    /// The reason [`TdsPlan`] carries more than a handle: a plan compiled
    /// against `nvarchar(4000)` would *truncate* a longer value rather than
    /// refuse it, so the hit is rejected, the plan unprepared and a new one
    /// compiled against the declaration this execution needs.
    #[test]
    fn a_hit_whose_plan_was_declared_narrower_is_unprepared_rather_than_truncating() {
        const SQL: &str = "select a from t where b = @p1";
        let long = "x".repeat(usize::from(NVARCHAR_CHARS) + 1);

        let mut wire =
            answering_each(&[prepexec_answer(9), procedure_answer(), prepexec_answer(10)]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(4);

        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[Some(b"7")])
                .expect("a short value, declared narrow"),
        );
        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[Some(long.as_bytes())])
                .expect("a long one, which the narrow plan cannot hold"),
        );

        let sent = flushed(&wire.peer().sent);
        assert_eq!(
            sent.len(),
            3,
            "the hit was rejected, so this one prepares too"
        );
        assert_eq!(sent_rpc(&sent[1].2).0, PROC_SP_UNPREPARE);
        let (proc_id, params) = sent_rpc(&sent[2].2);
        assert_eq!(proc_id, PROC_SP_PREPEXEC);
        assert!(
            params[1]
                .text
                .as_deref()
                .expect("a declaration")
                .contains("nvarchar(max)"),
            "the new plan is compiled against the declaration the value needs"
        );
        assert_eq!(cache.len(), 1, "the stale entry is dropped, not shadowed");
    }

    /// The same comparison over the other thing a declaration varies with: a
    /// `bytes` declares its marker `varbinary`, so one statement binding one at
    /// `@p1` and then at `@p2` wants two plans under the one key § 1 gives it.
    ///
    /// The key is deliberately not widened to carry a type per marker —
    /// [`TdsPlan::declared`] is one driver's reason to reject a hit, where the
    /// key is every driver's.
    #[test]
    fn one_statement_binding_a_bytes_at_two_markers_is_two_plans_under_one_key() {
        const SQL: &str = "insert into t values (@p1, @p2)";
        let binary = [BINARY_MARK, 0x00, 0xFF];

        let mut wire =
            answering_each(&[prepexec_answer(9), procedure_answer(), prepexec_answer(10)]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(4);

        drop(
            start_statement(
                &mut wire,
                &state,
                &mut cache,
                SQL,
                &[Some(&binary), Some(b"a")],
            )
            .expect("the `bytes` at the first marker"),
        );
        drop(
            start_statement(
                &mut wire,
                &state,
                &mut cache,
                SQL,
                &[Some(b"a"), Some(&binary)],
            )
            .expect("the same statement with the forms the other way round"),
        );

        let sent = flushed(&wire.peer().sent);
        assert_eq!(sent.len(), 3, "the hit was rejected, so this one prepares");
        assert_eq!(sent_rpc(&sent[1].2).0, PROC_SP_UNPREPARE);
        let (_, params) = sent_rpc(&sent[2].2);
        assert_eq!(
            params[1].text.as_deref(),
            Some("@p1 nvarchar(4000),@p2 varbinary(8000)"),
            "the second plan is compiled against the forms the second call binds"
        );
        assert_eq!(cache.len(), 1, "the stale entry is dropped, not shadowed");
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s batch on this
    /// protocol: one prepare, one execution per set, and the sum of what each
    /// one counted.
    ///
    /// **The prepare is asserted to happen once**, because that is the whole of
    /// what this member buys over the loop of `execute` calls a caller could
    /// write by hand — the sum would be the same either way, and so would every
    /// row it wrote. § 1's cache is what makes the second set an `sp_execute`,
    /// which is why the batch needs no request shape of its own here.
    ///
    /// The empty list and the sets that disagree are asserted beside it: both
    /// are answers § 4 gives before anything reaches the wire, so a batch that
    /// wrote first would still look right from the count alone.
    #[test]
    fn a_batch_is_one_prepare_an_execution_per_set_and_the_counts_summed() {
        const SQL: &str = "insert into t (a) values (@p1)";

        let mut wire = answering_each(&[
            prepexec_answer(9),
            done_token(DONE_COUNT, 2),
            done_token(DONE_COUNT, 3),
        ]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);

        assert_eq!(
            execute_many(&mut wire, &state, &mut cache, SQL, &[]).expect("§ 4's no-op"),
            0,
            "an empty set list is answered without a round trip"
        );
        assert!(wire.peer().sent.is_empty());

        let bound: [[Option<&[u8]>; 1]; 3] = [[Some(b"x")], [Some(b"y")], [Some(b"z")]];
        let sets: [&[Option<&[u8]>]; 3] = [&bound[0], &bound[1], &bound[2]];
        assert_eq!(
            execute_many(&mut wire, &state, &mut cache, SQL, &sets)
                .expect("a batch the server took"),
            6,
            "§ 4's answer is the sum of what the executions counted"
        );

        let sent = flushed(&wire.peer().sent);
        assert_eq!(sent.len(), 3, "one message per set, and no second prepare");
        assert_eq!(sent_rpc(&sent[0].2).0, PROC_SP_PREPEXEC);
        assert_eq!(sent_rpc(&sent[1].2).0, PROC_SP_EXECUTE);
        assert_eq!(sent_rpc(&sent[2].2).0, PROC_SP_EXECUTE);
        assert_eq!(cache.len(), 1, "one plan, executed three times");

        let odd: [&[Option<&[u8]>]; 2] = [&bound[0], &[]];
        let refused = execute_many(&mut wire, &state, &mut cache, SQL, &odd)
            .expect_err("one prepare has one parameter count");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            flushed(&wire.peer().sent).len(),
            3,
            "the sets that disagree are refused before the first of them is written"
        );
    }

    /// The half of § 4's semantics that costs the round trips: a refused set
    /// does not end the batch, the sets behind it are still attempted, and the
    /// **first** refusal is what the batch answers with.
    ///
    /// [`crate::mysql`]'s
    /// `a_refused_set_does_not_end_the_batch_and_the_first_refusal_is_reported`
    /// asks this of the driver that has a bulk command to refuse. Here there is
    /// none to refuse, so what this pins is that the loop was not quietly
    /// shortened into "stop at the first error" — which no count and no sum
    /// would show, both of them being discarded with the throw.
    #[test]
    fn a_refused_set_does_not_end_a_batch_and_the_first_refusal_is_reported() {
        const SQL: &str = "insert into t (a) values (@p1)";

        let mut first = message_token(TOKEN_ERROR, 2627, 14, "Violation of PRIMARY KEY constraint");
        first.extend_from_slice(&done_token(DONE_ERROR, 0));
        let mut second = message_token(TOKEN_ERROR, 515, 16, "Cannot insert the value NULL");
        second.extend_from_slice(&done_token(DONE_ERROR, 0));

        let mut wire = answering_each(&[prepexec_answer(9), first, second]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);

        let bound: [[Option<&[u8]>; 1]; 3] = [[Some(b"x")], [Some(b"y")], [Some(b"z")]];
        let sets: [&[Option<&[u8]>]; 3] = [&bound[0], &bound[1], &bound[2]];
        let refused = execute_many(&mut wire, &state, &mut cache, SQL, &sets)
            .expect_err("two sets were refused");

        let error = ServerError::of(&refused).expect("the server's own refusal");
        assert_eq!(
            error.driver_code,
            Some(2627),
            "the batch reports the first refusal and not the last: {refused}"
        );
        assert_eq!(
            flushed(&wire.peer().sent).len(),
            3,
            "the third set was written after the second was refused"
        );
        assert_eq!(
            state.get(),
            State::Idle,
            "a set the server refused leaves the connection at a boundary, so the batch \
             could carry on and § 13 can still pool it"
        );
    }

    /// [ADR 0067 § 7](/docs/decisions/0067.md)'s nesting in T-SQL's
    /// own vocabulary, and the one command in it that does not exist.
    ///
    /// The spellings are the driver's whole contribution here — `BEGIN`, not
    /// `START TRANSACTION`; `SAVE TRANSACTION`, not `SAVEPOINT`;
    /// `ROLLBACK TRANSACTION <name>`, not `ROLLBACK TO SAVEPOINT` — and a
    /// dialect error in any of them is a runtime refusal from the server that no
    /// type checks. The claim that cannot be read off a spelling is the message
    /// that is *not there*: T-SQL has no `RELEASE SAVEPOINT`, and a
    /// nested commit that sent `COMMIT TRANSACTION` in its place would commit
    /// the whole transaction while the depth still said two levels were open.
    #[test]
    fn a_transaction_nests_as_t_sql_spells_it_and_a_nested_commit_sends_nothing() {
        let mut wire = answering_each(&[done(), done(), done(), done(), done()]);
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let moved = Cell::new(false);

        let refused = commit(&mut wire, &state, &depth, &moved)
            .expect_err("a connection in no transaction has nothing to commit");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);

        for level in 1..=3 {
            begin(&mut wire, &state, &depth, &moved, None, false).expect("a level the server took");
            assert_eq!(depth.get(), level);
        }
        roll_back(&mut wire, &state, &depth, &moved).expect("the innermost level, undone");
        assert_eq!(depth.get(), 2);
        let span = commit(&mut wire, &state, &depth, &moved).expect("a nested commit");
        assert_eq!(depth.get(), 1);
        assert!(
            span.sql().is_empty(),
            "§ 11's event for a commit that sent nothing names no statement"
        );
        commit(&mut wire, &state, &depth, &moved).expect("the outermost commit");
        assert_eq!(depth.get(), 0);

        assert_eq!(
            batches(&wire.peer().sent),
            [
                "BEGIN TRANSACTION",
                "SAVE TRANSACTION nvs_1",
                "SAVE TRANSACTION nvs_2",
                "ROLLBACK TRANSACTION nvs_2",
                "COMMIT TRANSACTION",
            ],
            "the nested commit is the message that is not here"
        );
    }

    /// § 7's descriptor rides every request after the `BEGIN` that opened it —
    /// the batch commands this driver writes itself and the RPC a program's
    /// statement goes out as, alike.
    ///
    /// **The claim is the server's own refusal.** Once a transaction is open, a
    /// request whose `ALL_HEADERS` still names zero is answered with driver code
    /// 3989 — *new request is not allowed to start because it should come with
    /// valid transaction descriptor* — rather than run, so a driver that opened
    /// a transaction and kept writing zeroes could never run a statement inside
    /// one. The scripted peer cannot refuse anything, answering whatever was
    /// written to it, which is why this asserts on the header bytes that went
    /// out and `crates/nvs-db/tests/handshake.rs` asks a real server.
    ///
    /// The first request names none on purpose: the `BEGIN` is written before
    /// there is a transaction to enlist in, and the descriptor arrives in its
    /// answer.
    #[test]
    fn every_request_after_a_begin_carries_the_descriptor_the_server_sent() {
        const DESCRIPTOR: u64 = 0x0807_0605_0403_0201;

        let mut wire = answering_each(&[
            transaction_answer(ENV_BEGIN_TRANSACTION, Some(DESCRIPTOR)),
            prepexec_answer(3),
            transaction_answer(ENV_COMMIT_TRANSACTION, None),
        ]);
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let moved = Cell::new(false);
        let mut cache = plans(2);

        begin(&mut wire, &state, &depth, &moved, None, false)
            .expect("a transaction the server took");
        assert_eq!(
            wire.descriptor(),
            DESCRIPTOR,
            "the ENVCHANGE a begin answers with is read rather than skipped"
        );

        drop(
            start_statement(&mut wire, &state, &mut cache, "select a", &[])
                .expect("a statement inside the transaction"),
        );
        commit(&mut wire, &state, &depth, &moved).expect("the outermost commit");
        assert_eq!(
            wire.descriptor(),
            NO_TRANSACTION,
            "a commit ends the transaction the next request would otherwise name"
        );

        let carried: Vec<u64> = flushed(&wire.peer().sent)
            .iter()
            .map(|(_, _, body)| header_descriptor(body))
            .collect();
        assert_eq!(
            carried,
            [NO_TRANSACTION, DESCRIPTOR, DESCRIPTOR],
            "the BEGIN opens the transaction the statement and the COMMIT enlist in"
        );
    }

    /// § 13's reset is the one request that names no transaction however deep
    /// the session was, because the bit it carries has already ended one.
    ///
    /// [`Status::RESET_CONNECTION`] is honoured *before* the request it rides is
    /// processed, so a descriptor naming the rolled-back transaction would name
    /// one the server no longer has — and the connection this hands back to the
    /// pool must be one the next request can open its own transaction on.
    #[test]
    fn a_reset_from_inside_a_transaction_names_none_and_forgets_the_descriptor() {
        let mut wire = answering_each(&[
            transaction_answer(ENV_BEGIN_TRANSACTION, Some(0x0102_0304_0506_0708)),
            done(),
        ]);
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let moved = Cell::new(false);
        let mut cache = plans(2);

        begin(&mut wire, &state, &depth, &moved, None, false)
            .expect("a transaction the server took");
        reset_session(&mut wire, &state, &mut cache, &moved)
            .expect("a reset the server acknowledged");

        assert_eq!(wire.descriptor(), NO_TRANSACTION);
        let sent = flushed(&wire.peer().sent);
        let (_, status, body) = sent.last().expect("the reset went out");
        assert_ne!(
            status.bits() & Status::RESET_CONNECTION.bits(),
            0,
            "the bit is what makes this message the reset"
        );
        assert_eq!(header_descriptor(body), NO_TRANSACTION);
    }

    /// § 7's `{isolation}` on the one backend where it is a **session** setting:
    /// it is put back when the outermost transaction ends, and a transaction
    /// that asked for nothing pays for none of it.
    ///
    /// The restore is what the other drivers do not need and what
    /// `sp_reset_connection` does not cover — § 13 resets a connection on its way
    /// back to the pool, and a second `transaction()` in the *same request* never
    /// goes near it. Without the restore here, that second transaction would
    /// silently run `SERIALIZABLE`.
    #[test]
    fn a_session_isolation_level_is_put_back_when_the_outermost_transaction_ends() {
        let mut wire = answering_each(&[done(), done(), done(), done(), done(), done()]);
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let moved = Cell::new(false);

        let span = begin(
            &mut wire,
            &state,
            &depth,
            &moved,
            Some(Isolation::Serializable),
            false,
        )
        .expect("a level the server accepted");
        assert!(moved.get(), "the session no longer sits at its default");
        assert!(
            span.sql().contains("SERIALIZABLE") && span.sql().contains(BEGIN_TRANSACTION),
            "§ 11's event names both round trips the level cost: {}",
            span.sql()
        );

        commit(&mut wire, &state, &depth, &moved).expect("the outermost commit");
        assert!(!moved.get(), "the restore is paid, so none is owed");

        begin(&mut wire, &state, &depth, &moved, None, false).expect("a second transaction");
        roll_back(&mut wire, &state, &depth, &moved).expect("undone");

        assert_eq!(
            batches(&wire.peer().sent),
            [
                "SET TRANSACTION ISOLATION LEVEL SERIALIZABLE",
                "BEGIN TRANSACTION",
                "COMMIT TRANSACTION",
                DEFAULT_ISOLATION,
                "BEGIN TRANSACTION",
                "ROLLBACK TRANSACTION",
            ],
            "the second transaction inherits nothing and costs no restore of its own"
        );
    }

    /// The refusals § 7 owes a program on this backend, none of which touches
    /// the wire.
    ///
    /// **`readOnly` is refused rather than dropped**: SQL Server has no
    /// read-only transaction, and every other backend § 7 reaches enforces the
    /// option — a driver that accepted it and opened an ordinary writable
    /// transaction would give a program the word without the guarantee. The
    /// nested one is [`crate::mysql::begin`]'s, for a reason this backend states
    /// more strongly: the level is not the nested transaction's to set, being the
    /// whole session's.
    #[test]
    fn there_is_no_read_only_transaction_here_and_a_nested_one_may_not_set_the_level() {
        let mut wire = answering_each(&[done()]);
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let moved = Cell::new(false);

        let refused = begin(&mut wire, &state, &depth, &moved, None, true)
            .expect_err("SQL Server has no read-only transaction");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(
            wire.peer().sent.is_empty(),
            "the option is refused before anything opens"
        );
        assert_eq!(depth.get(), 0);

        begin(&mut wire, &state, &depth, &moved, None, false).expect("an ordinary transaction");
        let refused = begin(
            &mut wire,
            &state,
            &depth,
            &moved,
            Some(Isolation::Snapshot),
            false,
        )
        .expect_err("a nested level is the session's, not this transaction's");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            batches(&wire.peer().sent),
            ["BEGIN TRANSACTION"],
            "the refused nesting wrote nothing and opened no level"
        );
        assert_eq!(depth.get(), 1, "a refused begin leaves the depth alone");
    }

    /// The path the restore would otherwise fall through: a commit the *server*
    /// refused ends the transaction with the level still moved.
    ///
    /// The depth follows the connection where the server took it —
    /// [`crate::mysql::commit`]'s rule — so § 7's `{retries: n}` opens the next
    /// attempt with a `BEGIN TRANSACTION`. What this pins is that the restore
    /// rides that begin instead of the commit that could not carry it: the error
    /// the caller sees stays the server's own, and the next transaction still
    /// runs at the level it asked for.
    #[test]
    fn a_commit_the_server_refused_leaves_the_restore_owed_and_the_next_begin_pays_it() {
        let mut conflict = message_token(TOKEN_ERROR, 1205, 13, "Transaction was deadlocked");
        conflict.extend_from_slice(&done_token(DONE_ERROR, 0));
        let mut wire = answering_each(&[done(), done(), conflict, done(), done()]);
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let moved = Cell::new(false);

        begin(
            &mut wire,
            &state,
            &depth,
            &moved,
            Some(Isolation::Snapshot),
            false,
        )
        .expect("a level the server accepted");

        let refused = commit(&mut wire, &state, &depth, &moved).expect_err("the server refused it");
        assert_eq!(
            ServerError::of(&refused)
                .expect("the server's own refusal")
                .driver_code,
            Some(1205),
            "the caller sees the server's error and not a restore's"
        );
        assert_eq!(depth.get(), 0, "the server rolled the transaction back");
        assert!(moved.get(), "and nothing put the level back");

        begin(&mut wire, &state, &depth, &moved, None, false).expect("the next transaction");
        assert!(!moved.get());
        assert_eq!(
            batches(&wire.peer().sent),
            [
                "SET TRANSACTION ISOLATION LEVEL SNAPSHOT",
                "BEGIN TRANSACTION",
                "COMMIT TRANSACTION",
                DEFAULT_ISOLATION,
                "BEGIN TRANSACTION",
            ],
        );
    }

    /// The parked form of a walk, which is the whole point of splitting the read
    /// state off the borrow: the cursor is a local of its own, and every row is
    /// read through a **fresh** borrow of the wire, the state and § 1's cache.
    ///
    /// The compiler is half the assertion, as it is in `crate::mysql`'s
    /// `mysql_stream_parks_its_read_and_answers_one_row_per_step`. A [`TdsRows`]
    /// cannot express this shape at all — its borrow would have to span the
    /// calls between the rows — and that is exactly what a `Core\Db\Connection`
    /// holding a walk across `advance()` calls needs, the connection going back
    /// to the request in between. The other half is the run: one row per step,
    /// in order, the connection busy for as long as rows remain, and the `DONE`
    /// leaving it idle with § 1's plan filed on the way past.
    #[test]
    fn tds_stream_parks_its_read_and_answers_one_row_per_step() {
        const SQL: &str = "select v";
        let mut answer = col_metadata(&[column(&[TY_INT4], "v", 0)]);
        answer.extend_from_slice(&row_token(&[7i32.to_le_bytes().to_vec()]));
        answer.extend_from_slice(&row_token(&[9i32.to_le_bytes().to_vec()]));
        // The tokens a `sp_prepexec` ends with, the handle among them: a walk
        // that is nobody's borrow still files § 1's plan, and the step that
        // reads the `RETURNVALUE` is where.
        answer.extend_from_slice(&prepexec_answer(3));

        let mut wire = answering_each(&[answer]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(4);

        let mut reading = open_result(&mut wire, &state, &mut cache, SQL, &[])
            .expect("a result set the server described");

        assert_eq!(state.get(), State::Streaming);
        assert_eq!(reading.columns().len(), 1);

        let mut seen = Vec::new();
        while let Some(row) = next_row_of(&mut wire, &state, Some(&mut cache), &mut reading)
            .expect("the walk advanced")
        {
            // Read a step at a time and not drained into a buffer: the rows
            // still to come are still on the wire, which is what
            // `rule:core-classes/db-streaming`'s constant memory means and why the connection is
            // busy between the steps.
            assert_eq!(state.get(), State::Streaming);
            seen.push(row.column(0).flatten().map(<[u8]>::to_vec));
        }

        assert_eq!(
            seen,
            [
                Some(7i32.to_le_bytes().to_vec()),
                Some(9i32.to_le_bytes().to_vec()),
            ]
        );
        assert_eq!(
            state.get(),
            State::Idle,
            "a drained walk left the connection unpoolable"
        );
        assert_eq!(reading.affected(), Some(2));
        assert_eq!(
            cache.lookup(SQL, 0).map(|plan| plan.handle),
            Some(3),
            "the parked walk filed no plan under § 1's key"
        );
    }
}
