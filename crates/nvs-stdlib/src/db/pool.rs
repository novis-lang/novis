//! The connection a request runs on: warming one, waiting for a slot when every
//! one is taken, and the dialect and encoder its driver renders in.
//!
//! [`nvs_runtime::pool`] owns the pool; what is decided here is only what a
//! `Core\Db` call needs from a lease. [`rendering_for`] is the one table that
//! pairs a driver with how its statements are written, so a placeholder style
//! and a parameter encoding cannot drift apart.

use super::*;

/// A connection out of this core's pool under `lease`'s key, reset and ready to
/// run a statement — `rule:security/db-pool-reset-is-a-boundary`'s acquire, where the reset is the gate.
///
/// The lease is what the caller already holds a `max` slot on, and drawing
/// against it is how § 13's ceiling counts a warm connection the same as a
/// fresh one — `nvs_runtime::pool`'s *What `max` counts* owns that rule.
///
/// **A failed reset destroys the connection.** Every driver's `reset` takes
/// `self` by value — `nvs_db::PgConn::reset`, `nvs_db::MySqlConn::reset` and
/// `nvs_db::TdsConn::reset` — and hands the connection back only on the path
/// where every one of § 13's commands succeeded, so a connection that could not
/// be proven clean is closed before this returns and there is no shape in which
/// one request reads another's session state. That is also why the caller
/// cannot tell a failed reset from an empty pool: both are `None`, and both
/// mean open a fresh connection, which is what a request did before there was a
/// pool at all.
///
/// **The three resets are not one reset**, and § 13 says so: PostgreSQL's keeps
/// § 1's statement cache, MySQL's `COM_RESET_CONNECTION` drops it, and SQL
/// Server's `sp_reset_connection` is "the same shape as MySQL's" in that
/// section's own words and drops it too — so the connection each arm hands back
/// is warm in a different amount. None of that is a choice this function makes:
/// each driver's own `reset` is where its section's property is met.
///
/// **The SQLite arm is reset like the rest of them now**, and § 13's per-backend
/// list is where its shortness is argued: a file handle has no session state to
/// leak, so rolling back whatever transaction is open is the whole of it, and
/// `nvs_db::SqliteConn::reset` asks the engine rather than this driver's own
/// depth count so that a transaction a caller opened in its own § 4 statement
/// text is closed too. Every arm is spelled rather than left to a `_` so that a
/// sixth driver arrives as a build failure instead of as a connection silently
/// thrown away. MariaDB's arm is MySQL's: `COM_RESET_CONNECTION` is one
/// protocol's command and § 13 says of both that it drops the prepared
/// statements with the session state.
///
/// **The SQL Server arm is reached now**: both openers call
/// `nvs_db::TdsConn::connect`, so a released connection is one this function is
/// asked for. It was written before either did, for the reason § 13 gives: a
/// driver that becomes openable while this function still answers `None` for it
/// is a pool that quietly stops pooling, which nothing observable would report.
pub(super) fn warm_connection(lease: &nvs_runtime::pool::Lease) -> Option<nvs_db::Connection> {
    let held = nvs_runtime::pool::take(lease, std::time::Instant::now())?;
    let mut connection = held.into_any().downcast::<nvs_db::Connection>().ok()?;
    // § 4's statement deadline belongs to the statement that named it, and this
    // connection is carrying whatever the last one on it did. The reset below is
    // the one exchange no program's clock may bound — a `RESET ALL` given up on
    // because a previous request asked for a 50ms query would destroy a healthy
    // connection and read, from here, as a pool that quietly stopped pooling.
    connection.set_deadline(None).ok()?;
    match *connection {
        nvs_db::Connection::Postgres(postgres) => {
            Some(nvs_db::Connection::Postgres(postgres.reset().ok()?))
        }
        nvs_db::Connection::MySql(mysql) => Some(nvs_db::Connection::MySql(mysql.reset().ok()?)),
        nvs_db::Connection::MariaDb(maria) => {
            Some(nvs_db::Connection::MariaDb(maria.reset().ok()?))
        }
        nvs_db::Connection::SqlServer(tds) => {
            Some(nvs_db::Connection::SqlServer(tds.reset().ok()?))
        }
        nvs_db::Connection::Sqlite(sqlite) => {
            Some(nvs_db::Connection::Sqlite(sqlite.reset().ok()?))
        }
    }
}

/// Waits for a slot under `ticket`'s key — `rule:security/db-pool-reset-is-a-boundary`'s `acquire` — or
/// throws because the wait ran out.
///
/// Reached only once `nvs_runtime::pool::admit` has already said the key is
/// full, so the first thing it does is join the line: looking before queueing
/// is the one ordering that can miss a hand-over, and that module's `queue`
/// owns why.
///
/// **Which bound wins: whichever comes first, and the refusal says which.**
/// `acquire` is the operator's, written in `[db.<name>.pool]` and sized against
/// the server's own connection limit; `timeout` is this call's, already an
/// instant by the time `connect` reaches here and covering the call as a whole
/// rather than the handshake alone. Neither is a budget the other may spend: a
/// program that asked for an answer within two seconds does not get five
/// because the pool was allowed to wait that long, and a pool told to wait one
/// second does not wait thirty because its caller was patient. Both are
/// ceilings, so the earlier instant is the deadline — and because the handshake
/// below is measured against the same `timeout` instant, a wait that ate most
/// of it leaves the rest for opening, which is what a caller asking for a whole
/// answer by an instant meant.
///
/// **`acquire = 0` never parks.** § 13 makes it legal and defines it as
/// refusing rather than queueing, and so does a call with no task beneath it: a
/// `nvs run` of a CLI program is one task, so there is no peer that could free
/// a slot and waiting could only be this core standing still —
/// `rule:http-server/a-core-is-never-blocked-on-a-syscall`
/// 's tier-B failure.
///
/// **The wording of that refusal is the caller's**, handed in as `full` and
/// completed with the clause saying which ending it was. The two members have
/// nothing to say in common there: `connect` names a block and the
/// `[db.<name>.pool] max` an operator can raise, and `open` has a hash of its
/// own settings — whose `max` is [`settings_bounds`]'s answer, so the sentence
/// it can honestly write is about the block describing that same endpoint, if a
/// deployment wrote one. What this function owns is the waiting, which *is* the
/// same for both.
///
/// # Errors
///
/// A thrown `IOError` for the ceiling reached, which is where every ending but
/// one lands: it is the refusal that stood beside the handshake that "did not
/// open" before there was any waiting, on the same reading — what a program can
/// do about either is the same, and § 8's `Db\DbError` is for a refusal the
/// *server* made, which this is not. The one other ending is [`Ctx::cancel`]'s
/// status for a task cancelled while it waited, which no `catch` sees.
pub(super) fn wait_for_slot(
    ctx: &mut nvs_runtime::Ctx,
    ticket: nvs_runtime::pool::Ticket,
    full: &dyn Fn(&str) -> Fault,
    timeout: Option<std::time::Instant>,
) -> Result<nvs_runtime::pool::Lease, Fault> {
    let acquire = std::time::Instant::now().checked_add(ticket.bounds.acquire);
    let Some(acquire) = acquire.filter(|_| !ticket.bounds.acquire.is_zero()) else {
        return Err(full(
            "`acquire` is `0`, so a request that arrives at that ceiling is refused rather than \
             queued behind one",
        ));
    };
    let (until, bound) = match timeout {
        Some(timeout) if timeout < acquire => (timeout, "this call's own `timeout`"),
        _ => (acquire, "`acquire`"),
    };
    // In hand before the pool is looked at again, which is `Host::waker`'s own
    // rule: a handle taken after the look could be registered by a peer that
    // has already released, and that is the one way this becomes a hang.
    let waker = nvs_runtime::host::with_current(|host| host.waker()).flatten();
    let Some(waker) = waker else {
        return Err(full(
            "there is no scheduler on this thread for a request to wait on, so `acquire` would \
             be this core standing still rather than a queue",
        ));
    };
    let mut waiting = nvs_runtime::pool::queue(ticket, waker);
    loop {
        if let Some(lease) = waiting.slot() {
            return Ok(lease);
        }
        // The clock, not the wake, is what ends the wait: a wake is a hint the
        // seam does not promise means anything, so the deadline is read here
        // where it is a fact.
        if std::time::Instant::now() >= until {
            return Err(full(&format!(
                "no connection came free before {bound} was up"
            )));
        }
        if let Some(nvs_runtime::host::Woken::Cancelled) =
            nvs_runtime::host::with_current(|host| host.park(Some(until)))
        {
            return Err(ctx.cancel());
        }
    }
}

/// How one driver's bound values are rendered — [`nvs_db::encode`] for
/// PostgreSQL, [`nvs_db::mysql::encode`] for the two that speak MySQL's
/// protocol, `nvs_db::sqlite::encode` for the one with no protocol at all.
///
/// **An enum and not one `fn` pointer, because the fifth driver does not answer
/// the same shape.** Four protocols carry a parameter as octets and SQLite
/// carries a *value*, so there is no return type the five share that is not
/// either a second encoding of SQLite's or a loss of the four's borrowing.
/// [`Binds`] is the same split on the result, and both are made in
/// [`rendering_for`].
///
/// A pointer rather than a `match` at the two call sites because § 5's dialect
/// and § 9's encoding are **one** choice: a statement rewritten for one
/// protocol and bound for another is refused by nothing here — `?` and `$1` are
/// both valid text, `t` and `1` are both valid bytes — and fails at the server
/// or, worse, binds the wrong value. [`rendering_for`] is the single place the
/// pair is made.
#[derive(Clone, Copy)]
pub(crate) enum Encoder {
    /// The four drivers with a wire: § 9's value as the octets that protocol
    /// reads a parameter in, `None` for `null`.
    Wire(fn(Value) -> std::io::Result<Option<Vec<u8>>>),
    /// SQLite: § 9's value as one of five storage classes, `NULL` among them.
    Sqlite(fn(Value) -> std::io::Result<nvs_db::SqliteValue>),
}

/// `rule:core-classes/db-parameters`'s dialect and § 9's encoder for one driver.
///
/// Pure and separate from [`rendering_of`] so the pairing is testable with no
/// connection in hand: `a_driver_is_bound_in_its_own_dialect` is what holds it
/// to [`nvs_db::Dialect::of`], which is the rewriter's own answer for the same
/// question.
///
/// **MariaDB renders as MySQL does, and that is not a shortcut**: the encoder
/// follows the *protocol*, which the two share whole, where
/// [`nvs_db::Driver`] separates them for the auth plugins and error tables ADR
/// 0067 keeps them apart for. `nvs_db::Dialect` has already made the same call
/// for the text.
///
/// **Total, which is [`crate::db`] § *Every driver reaches every member, and
/// all five are pooled*'s binding half.** It answered `Option`
/// while SQLite had no encoder and was this crate's roster of the drivers
/// nothing binds for at all; every driver `nvs-db` has written now binds, so
/// there is no absent case left to carry. It was never [`crate::queue`]'s
/// roster: `rule:core-classes/queue-storage-is-a-table`'s schema is written for three drivers and this binds for
/// five, so that module's `no_dialect` splits on its own `migration` instead.
pub(crate) fn rendering_for(driver: nvs_db::Driver) -> (nvs_db::Dialect, Encoder) {
    let encode = match driver {
        nvs_db::Driver::Postgres => Encoder::Wire(nvs_db::encode),
        nvs_db::Driver::MySql | nvs_db::Driver::MariaDb => Encoder::Wire(nvs_db::mysql::encode),
        nvs_db::Driver::SqlServer => Encoder::Wire(nvs_db::tds::encode),
        nvs_db::Driver::Sqlite => Encoder::Sqlite(nvs_db::sqlite::encode),
    };
    (nvs_db::Dialect::of(driver), encode)
}

/// [`rendering_for`] the connection filed under `key`, which is how a statement
/// is written in its own connection's dialect rather than in one this module
/// picked.
///
/// It no longer refuses anything, and neither does anything downstream of it:
/// every driver binds, every driver sends, and § 7's [`transacting`] — the last
/// place a `sqlite` block was turned away — is total.
///
/// The `block` is still taken because the lookup can fail for a reason that is
/// this crate's own, and a message naming the connection is what says which one.
///
/// # Errors
///
/// [`filed_connection`]'s [`Fault::fatal`]s for a table this crate filled
/// wrongly.
pub(super) fn rendering_of(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    _block: &Value,
    named: &str,
) -> Result<(nvs_db::Dialect, Encoder), Fault> {
    let driver = filed_connection(ctx, key, named)?.driver();
    Ok(rendering_for(driver))
}

/// The `nvs-db` connection filed under `key`, whichever driver it is.
///
/// The one downcast in this module: [`transacting`] narrows it further and
/// [`rendering_of`] only reads its driver, and either written on its own is a
/// second place holding the two `Fault::fatal`s that say the request's own
/// table is wrong.
///
/// **This is where spec § 18's `close` is enforced for every other member.**
/// A closed handle still names its entry — [`nvs_runtime::Ctx::connection_is_filed`]
/// is what says so — and every member that needs the connection refuses here
/// rather than each carrying its own guard, because "the connection is gone" is
/// one fact and reaching it is what every one of them has in common.
///
/// # Errors
///
/// A thrown `LogicError` for a connection the program has already closed. A
/// [`Fault::fatal`] for a key the request's table never held, and another for
/// an entry that is not this crate's — both this crate's paste error rather
/// than a program's.
pub(super) fn filed_connection<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
    named: &str,
) -> Result<&'a mut nvs_db::Connection, Fault> {
    if ctx.open_connection_mut(key).is_none() && ctx.connection_is_filed(key) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{named}: the connection has been closed"),
        ));
    }
    let filed = ctx.open_connection_mut(key).ok_or_else(|| {
        Fault::fatal(format!(
            "{named}: no connection is filed under the key {key}"
        ))
    })?;
    filed
        .as_any_mut()
        .downcast_mut::<nvs_db::Connection>()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{named}: the connection filed under the key {key} is not `nvs-db`'s"
            ))
        })
}

/// `rule:security/db-pool-reset-is-a-boundary`'s bounds for a connection a *program* described: the
/// `[db.<name>.pool]` table of the block whose settings hash is `memo` when a
/// deployment wrote one, and [`PoolBounds::DEFAULT`] when it did not.
///
/// **A settings object names no block, and that is the whole difficulty.**
/// `Core\Db::connect` asks `nvs_config::db::bounds_for` by the name an operator
/// wrote; this path has only the hash § 2 built out of the settings themselves,
/// so the block is found by building each block's own key and comparing —
/// [`super::open::block_settings_key`] owns that and owns why it is a key
/// comparison rather than a field-by-field one.
///
/// **A program that opens a configured endpoint by hand gets that endpoint's
/// bounds**, which is the rule this exists for: those two connections are the
/// same connection to the same server under the same credentials, they already
/// share a pool because § 2's key is the pool's key, and a pool with two
/// answers for `max` would be a ceiling an operator sized and did not get. A
/// settings object naming an endpoint no block describes keeps the defaults, because
/// there is no table to read and § 13 requires the bounds to be finite anyway.
///
/// **It costs one key per configured block per `open`, and only on the miss.**
/// The memoized-connection check runs first and answers every `open` after the
/// request's own first one, so this is reached once per distinct settings
/// object per request, over a `[db]` table an operator hand-wrote — a handful
/// of `DefaultHasher` runs over short strings. Caching the mapping on the
/// snapshot would spend a per-generation table to save that, which `rule:programs/memory-priority`'s
/// ordering does not buy: the latency is not on the request path's hot part,
/// and the memory would be O(blocks) per generation held for the lifetime of a
/// reload.
///
/// A program running with no configuration at all takes the defaults: there is
/// no tree to read a switch out of, which is not the same as a deployment that
/// wrote one.
pub(super) fn settings_bounds(ctx: &nvs_runtime::Ctx, memo: &str) -> nvs_config::db::PoolBounds {
    let Some(config) = ctx.config() else {
        return nvs_config::db::PoolBounds::DEFAULT;
    };
    bounds_for_settings(&config.snapshot().config, memo)
}

/// [`settings_bounds`] over the tree alone, which is the half a test can drive:
/// nothing below this line needs a connection, a socket or a driver.
///
/// The refusal cannot fire — `nvs_config::db::validate` proved every block's
/// bounds at boot — and `OFF` is what it answers if it ever did, for
/// `nvs_core_db_connect`'s reason: a connection closed with the request beats
/// one pooled under bounds nobody could resolve.
fn bounds_for_settings(
    config: &nvs_config::tree::Config,
    memo: &str,
) -> nvs_config::db::PoolBounds {
    let named = config
        .db
        .blocks
        .iter()
        .find(|(_, block)| super::open::block_settings_key(block).as_deref() == Some(memo))
        .map(|(name, _)| name.as_str());
    nvs_config::db::bounds_for(config, named, &std::collections::BTreeMap::new())
        .unwrap_or(nvs_config::db::PoolBounds::OFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:core-classes/db-parameters`'s rewrite and § 9's encoding are **one** choice per driver,
    /// and the pairing is what a statement bound half one way fails on — at the
    /// server if it is lucky, since `?` and `$1` are both valid text and `t` and
    /// `1` are both valid bytes.
    ///
    /// The dialect is asserted against [`nvs_db::Dialect::of`] rather than
    /// against a list spelled out here, because that is the rewriter's own
    /// answer to the same question and a second list is the thing that drifts.
    /// `bool` is the value the two encoders first disagree about, so it is what
    /// catches a pair put together the wrong way round.
    #[test]
    fn a_driver_is_bound_in_its_own_dialect() {
        for driver in nvs_db::Driver::ALL {
            let (dialect, encode) = rendering_for(driver);
            assert_eq!(
                dialect,
                nvs_db::Dialect::of(driver),
                "{driver:?} rewrites in the dialect `Dialect::of` gives it"
            );
            match encode {
                Encoder::Wire(encode) => {
                    let rendered =
                        encode(Value::bool(true)).expect("`true` renders on every driver");
                    let expected: &[u8] = if dialect == nvs_db::Dialect::PostgreSql {
                        b"t"
                    } else {
                        b"1"
                    };
                    assert_eq!(
                        rendered.as_deref(),
                        Some(expected),
                        "{driver:?} is paired with another protocol's encoder"
                    );
                }
                Encoder::Sqlite(encode) => {
                    assert_eq!(
                        dialect,
                        nvs_db::Dialect::Sqlite,
                        "{driver:?} takes storage classes, so it is the one with no wire"
                    );
                    assert_eq!(
                        encode(Value::bool(true)).expect("`true` binds off the wire too"),
                        nvs_db::SqliteValue::Int(1),
                        "{driver:?} is paired with another backend's encoder"
                    );
                }
            }
        }
    }

    /// Every driver binds, and the *shape* of a bind follows the protocol rather
    /// than a roster kept here.
    ///
    /// [`rendering_for`] used to answer `None` for a driver nothing bound for,
    /// and this test pinned that list; the list is empty now, so what is left to
    /// hold is the other half — that exactly the driver with no wire takes a
    /// value where the four with one take octets. A fifth wire driver added
    /// with SQLite's encoder would pass the test above and fail this one.
    #[test]
    fn a_parameter_is_octets_on_every_driver_but_the_one_with_no_wire() {
        let owned: Vec<nvs_db::Driver> = nvs_db::Driver::ALL
            .into_iter()
            .filter(|driver| matches!(rendering_for(*driver).1, Encoder::Sqlite(_)))
            .collect();
        assert_eq!(
            owned,
            vec![nvs_db::Driver::Sqlite],
            "a bound value is a storage class exactly where there is no protocol to render it for"
        );
    }

    /// One `[db.main]` block, written out as an operator would, with bounds
    /// that are nothing like [`nvs_config::db::PoolBounds::DEFAULT`] — so a
    /// path that quietly kept the defaults answers `16` where the table says
    /// `3`, and every assertion below can name which number it got.
    ///
    /// `pool` is left for the caller to append, because the two cases below
    /// differ only in where the switch is written.
    const BLOCK: &str = "[db.main]\ndriver = \"postgres\"\nhost = \"db.internal\"\n\
                         port = 6432\nuser = \"app\"\npassword = \"s3cret\"\n\
                         database = \"shop\"\n";

    /// The memo key a `Core\Db::open` writing [`BLOCK`]'s own fields opens
    /// under — built here through [`crate::db::open::settings_key`], which is
    /// the member's own call and not a copy of it.
    fn memo_for(password: &str) -> String {
        crate::db::open::settings_key(
            &["db.internal", "app", "shop", password, "postgres"],
            Some(6432),
            0,
            None,
        )
    }

    /// `rule:security/db-pool-reset-is-a-boundary`'s bounds for a connection a *program* described, which is
    /// the half the ADR left to be found: `connect` looks its block up by the
    /// name an operator wrote, and `open` has only § 2's settings hash.
    ///
    /// The claim is asserted **on both sides**, because a body that read the
    /// table for every connection would pass the first half alone. A settings object
    /// whose fields are the block's takes the block's `max` of `3`; the same
    /// object with one credential changed is a different connection to the
    /// same server, matches no block, and takes the defaults — `16`, which is
    /// what every path here answered before the lookup existed.
    #[test]
    fn an_open_reads_its_pool_bounds_from_the_blocks_pool_table() {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        ctx.set_config(crate::tests::granting(&format!(
            "{BLOCK}\n[db.main.pool]\nmax = 3\nidle = 1\n"
        )));

        let mine = settings_bounds(&ctx, &memo_for("s3cret"));
        assert!(
            mine.enabled,
            "a block that writes bounds is a block that wants a pool"
        );
        assert_eq!(
            (mine.max, mine.idle),
            (3, 1),
            "the settings hash names `[db.main]`, so `[db.main.pool]` is what bounds it"
        );

        let stranger = settings_bounds(&ctx, &memo_for("another-password"));
        assert_eq!(
            stranger,
            nvs_config::db::PoolBounds::DEFAULT,
            "and settings no block describes have no table to read, so § 13's defaults bound them"
        );
    }

    /// § 13's `pool = false`, written **unscoped**: the audited deployment's
    /// requirement is that every connection the process opens maps to one
    /// request, and a per-block switch cannot say that about a connection a
    /// program described for itself.
    ///
    /// So all three of the ways a connection is bounded are asked here — the
    /// object that matches the block, the object that matches nothing, and
    /// the `connect` by name that [`nvs_config::db::bounds_for`] answers — and
    /// the switch reaches all three. The middle one is the case no per-block
    /// spelling could ever have covered.
    #[test]
    fn an_unscoped_pool_false_reaches_a_program_opened_connection() {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        let snapshot = crate::tests::granting(&format!(
            "[db]\npool = false\n{BLOCK}\n[db.main.pool]\nmax = 3\n"
        ));
        assert_eq!(
            snapshot.config.db.blocks.len(),
            1,
            "`pool` is the table's own key and not a block, so `[db.main]` is still the only one"
        );
        ctx.set_config(std::sync::Arc::clone(&snapshot));

        for memo in [memo_for("s3cret"), memo_for("another-password")] {
            assert!(
                !settings_bounds(&ctx, &memo).enabled,
                "the switch is unscoped, so it reaches a settings object whether or not a block \
                 describes the same endpoint"
            );
        }
        let named = nvs_config::db::bounds_for(
            &snapshot.config,
            Some("main"),
            &std::collections::BTreeMap::new(),
        )
        .expect("`max = 3` is a bound this tree accepts");
        assert!(
            !named.enabled,
            "and it outranks the block's own table, which is what `connect` reads"
        );
    }
}
