//! `Core\Queue` — [ADR 0084](../../../docs/adr/0084-durable-background-jobs.md)'s durable background
//! job, over the `[db.<name>]` block a `[queue] connection` names.
//!
//! **A job is a row, and that is the whole design.** § 3 states the property everything else is
//! arranged around: `push` runs on the queue's *own* connection, so an enqueue inside
//! `Core\Db::connect("main")->transaction(…)` — where `[queue] connection = "main"` — commits with the
//! write that caused it or with neither. There is no outbox and no window in which the order exists
//! and the receipt job does not. That property is not implemented here so much as *not broken* here:
//! [`nvs_runtime::Ctx::memoized_connection`] already answers with the connection the request opened
//! under that name, an open `BEGIN` is that connection's state, and so the insert below is inside the
//! transaction without either half knowing about the other. Reaching the connection any other way —
//! a second handshake, a dedicated pool of the queue's own — would silently take the property away,
//! which is why [`db::open_named`](crate::db::open_named) is shared with `Core\Db::connect` rather
//! than reimplemented.
//!
//! **The connection is the operator's and never the program's, so `push` asks for no capability.**
//! `Core\Db::connect` needs `db.connect` for its name because the *program* writes that name and a
//! grant is what stops a request choosing its own database. Nothing here takes a connection name at
//! all: `[queue] connection` is root-owned configuration, resolved and proven to name a real block at
//! boot ([`nvs_config::queue::queue_for`]), and `$queue` is a column value rather than a block. There
//! is no name for a capability to be about, so ADR 0084 § 1 states none and this module invents one.
//!
//! **The schema is this module's until `nvs queue migrate` exists.** § 2 makes the runtime own one
//! jobs table and one dead-letter table, created by an explicit operator command — DDL is an
//! injection sink and never issued from a request — so `push` writes into a table it does not create.
//! [`JOBS_TABLE`] and [`INSERT`] are the one home for what that table's columns are, and the migrate
//! command will be read off them rather than the other way round. Two choices in it are worth their
//! sentence: every instant is a `bigint` of epoch milliseconds rather than a timestamp, because § 2
//! supports all five of ADR 0067's backends and five timestamp dialects is exactly the cost a
//! runtime-owned table should not carry; and `state` is the ordinal `Core\Queue\State` already is at
//! runtime ([ADR 0010](../../../docs/adr/0010-enums-are-a-value-type.md)), so the enum and the column
//! are one representation and not two.
//!
//! **What it spends:** one statement per `push`, on a connection the request either already held or
//! now holds for the rest of it, plus one JSON encoding of `$args` sized by the payload the caller
//! wrote. Nothing is held between calls.
//!
//! # Known gaps
//!
//! 1. **`limits` and `grants` are not declared**, because both are § 1's `{…}` — a *shape* parameter,
//!    which this registry still cannot spell. That is the same blocker `Core\Db::open` waits on
//!    ([`crate::db`]'s own gaps), and the two lift together.
//! 2. **`$args` is `mixed` and so does not refuse a `secret`**, which § 1 asks for. A durable row is
//!    an output and ADR 0033's five sinks are the shape of the eventual answer; `CoreTy::Mixed`
//!    carries no qualifier, so saying it needs a spelling the registry has not got.
//! 3. **`key`'s "at most one pending job per key" is enforced by the statement and not yet by an
//!    index.** [`INSERT`]'s `existing` arm reads the table inside the same statement that writes it,
//!    which is correct against every other `push` on a *serialized* transaction and racy against a
//!    concurrent one at `read committed`. The partial unique index over `(dedupe_key) where state =
//!    0` is `nvs queue migrate`'s to create, and when it exists the statement below becomes race-free
//!    without changing shape.
//! 4. **`cancel`, `status` and `stats` are owed** — § 1's roster is four members and this is one.
//! 5. **PostgreSQL only**, as [`crate::db`]'s gap 2 is: the other four drivers have no statement path
//!    yet, so [`postgres_of`] refuses them by name rather than writing a row nothing would claim.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// `Core\Queue`'s fully-qualified name.
const NAME: &str = r"Core\Queue";

/// `Core\Queue\Id`'s, as [`CoreTy::Instance`] spells it.
pub(crate) const ID_NAME: &str = r"Core\Queue\Id";

/// `push`'s name in a refusal, written once so every message spells it the same way.
const PUSH: &str = r"Core\Queue::push";

/// The table § 2's `nvs queue migrate` creates and this module writes into.
///
/// Unqualified on purpose: the block's own `search_path` — the operator's, in root-owned
/// configuration — decides which schema it lands in, exactly as it decides for every statement the
/// application itself writes. A name this module qualified would be a second answer to a question
/// [`nvs_config::db`] has already given.
const JOBS_TABLE: &str = "nvs_jobs";

/// `Core\Queue\State::Pending`'s ordinal, which is what a freshly pushed row's `state` is.
///
/// Written as the number rather than read off the enum because the enum is § 1's remaining work and
/// this is the one case that cannot wait for it: a row nothing can claim is not an enqueue. The
/// member that declares `Core\Queue\State` owes an assertion that its first case is still this.
const PENDING: i16 = 0;

/// ADR 0084 § 1's `push`, as one statement.
///
/// **One statement rather than a check and an insert**, because two would be two moments and § 3's
/// property is about there being one. The `existing` arm is `key`'s dedupe and costs nothing at all
/// when `$1` is `null`: `dedupe_key = null` is never true, so the arm is empty and the `not exists`
/// guard admits the insert. That is why there is no second spelling of this statement for the
/// commoner call that passes no key — a branch here would be a second SQL text for § 1's statement
/// cache to hold and a second thing to keep right.
///
/// The trailing `union all` is what makes the answer one row in both cases: a deduped push answers
/// with the pending job's own id, which is what a caller that wanted "at most one" asked for.
const INSERT: &str = "with existing as (\
     select id from nvs_jobs where dedupe_key = $1::text and state = 0 limit 1\
 ), inserted as (\
     insert into nvs_jobs \
     (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, dedupe_key, created_at) \
     select $2::text, $3::text, $4::text, $5::smallint, 0, $6::int, $7::bigint, $8::bigint, \
            $1::text, $9::bigint \
     where not exists (select 1 from existing) \
     returning id\
 ) select id from inserted union all select id from existing limit 1";

/// A [`ID`]'s first slot: the primary key the insert returned.
const ID_SLOT: &str = "id";

/// Its second: the queue the job is in, so a `cancel` or a `status` written against this value knows
/// which queue to look in without asking the row again.
const ID_QUEUE_SLOT: &str = "queue";

/// `$script`'s argument slot.
const SCRIPT_ARG: usize = 0;

/// `{args: …}`'s. Every option below is flattened to one slot in the order [`CLASS`] declares them.
const ARGS_ARG: usize = 1;

/// `{queue: …}`'s. See [`ARGS_ARG`].
const QUEUE_ARG: usize = 2;

/// `{runAt: …}`'s. See [`ARGS_ARG`].
const RUN_AT_ARG: usize = 3;

/// `{maxAttempts: …}`'s. See [`ARGS_ARG`].
const MAX_ATTEMPTS_ARG: usize = 4;

/// `{backoff: …}`'s. See [`ARGS_ARG`].
const BACKOFF_ARG: usize = 5;

/// `{key: …}`'s. See [`ARGS_ARG`].
const KEY_ARG: usize = 6;

/// ADR 0084 § 1's `Core\Queue` — `push`, with `cancel`, `status` and `stats` owed (gap 4).
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
        name: "push",
        names: &["script"],
        params: &[
            // A **sink**, and for ADR 0044's reason rather than ADR 0024's usual one: the argument
            // selects which file a worker will execute, so a `tainted` one would let a request pick
            // the program that runs on its behalf.
            CoreTy::Text(Qual::Sink),
            CoreTy::Options(&[
                CoreOption {
                    name: "args",
                    ty: CoreTy::Mixed,
                    default: Const::Null,
                },
                CoreOption {
                    name: "queue",
                    // **Neutral, so a queue name may come from a request.** It is a bound
                    // parameter and reaches no statement text, and what it answers with —
                    // an opaque id — carries none of it. What a `tainted` one can do is
                    // choose which named queue the job waits in, which is a routing
                    // decision over data and not an instruction: a name no worker is
                    // configured for is claimed by nobody, which is the same outcome as
                    // not pushing at all.
                    ty: CoreTy::Text(Qual::Neutral),
                    default: Const::Str("default"),
                },
                CoreOption {
                    name: "runAt",
                    ty: CoreTy::Instance(crate::time::INSTANT_NAME),
                    default: Const::Null,
                },
                CoreOption {
                    name: "maxAttempts",
                    ty: CoreTy::Uint,
                    // Not given is not zero: the bound falls back to `[queue] max_attempts`, which
                    // `nvs_config::queue` has already refused a `0` for.
                    default: Const::Null,
                },
                CoreOption {
                    name: "backoff",
                    ty: CoreTy::Instance(crate::time::DURATION_NAME),
                    default: Const::Null,
                },
                CoreOption {
                    name: "key",
                    // Neutral for `queue`'s reason and more plainly: a dedupe key is
                    // compared against a column and read by nothing else, so the
                    // commonest key there is — one derived from the request that caused
                    // the job — is exactly the one a `string` would have refused.
                    ty: CoreTy::Text(Qual::Neutral),
                    default: Const::Null,
                },
            ]),
        ],
        defaults: &[],
        return_ty: CoreTy::Instance(ID_NAME),
        symbol: "nvs_core_queue_push",
        doc: Some(&PUSH_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Queue::push`'s reference card — ADR 0117.
const PUSH_DOC: MethodDoc = MethodDoc {
    short: "Enqueues `$script` to run in the background, as a row in the database `[queue] \
            connection` names. Inside a transaction on that same connection the enqueue commits \
            with the write that caused it, or with neither — which is the whole reason a job is a \
            table row and not a message to a broker.",
    params: &[
        ParamDoc {
            name: "script",
            desc: "The file a worker runs, as `spawn script` names one. A path and not a class or \
                   a closure, so the job carries no captured state across the boundary.",
            shape: &[],
        },
        ParamDoc {
            name: "args",
            desc: "The payload, copied by value into the row and decoded on the other side into \
                   the job's declared types. A reference is never carried, because the worker is \
                   a separate isolate and usually a separate process.",
            shape: &[],
        },
        ParamDoc {
            name: "queue",
            desc: "The named queue the job goes in. Workers claim from the queues they are \
                   configured for, so this is how work is separated by rate rather than by kind.",
            shape: &[],
        },
        ParamDoc {
            name: "runAt",
            desc: "The earliest moment a worker may claim it. Left out, that moment is now.",
            shape: &[],
        },
        ParamDoc {
            name: "maxAttempts",
            desc: "How many attempts this job gets before it is dead-lettered. Left out, `[queue] \
                   max_attempts`. Always finite: there is no spelling that retries forever.",
            shape: &[],
        },
        ParamDoc {
            name: "backoff",
            desc: "The base delay for the exponential backoff between attempts, jittered by the \
                   worker. Left out, the worker's own default.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "A dedupe key: while a job with this key is still pending, a second push with it \
                   enqueues nothing and answers the pending job's own id.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Queue\\Id` naming the row, which `cancel` and `status` are asked about. For a \
          push deduped by `key`, the id of the job already pending under it.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This deployment writes no `[queue]` block, so nothing says which database a job \
                   would live in; or the queue's connection names a driver that cannot yet run a \
                   statement.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`maxAttempts` is `0`, which asks for a job that is dead-lettered by the enqueue \
                   that created it; or `backoff` is negative.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The queue's connection did not open, or the insert was refused by the server — \
                   most often because `nvs queue migrate` has not created the table.",
        },
    ],
};

/// § 1's receipt: the row `push` wrote, and the queue it is in.
///
/// A class rather than a bare `uint` because the two members that take one — `cancel` and `status` —
/// have to know the queue as well, and a caller holding an integer would have to carry it beside.
/// It has no members of its own; it is a name for a pair, which is what ADR 0063 gives an opaque
/// handle.
pub(crate) const ID: CoreClass = CoreClass {
    name: ID_NAME,
    methods: &[],
    instance: &[],
    slots: &[ID_SLOT, ID_QUEUE_SLOT],
    constants: &[],
};

/// Milliseconds since the epoch, now.
///
/// Saturating rather than fallible: a clock before 1970 is not a condition an enqueue should refuse
/// over, and the row it would write is `run_at` in the past, which means *claimable immediately* —
/// the same answer the caller asked for.
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or(0)
}

/// One of a `Core\Time\Instant`'s two slots as a number, whichever integer tag it carries.
///
/// The slots are written by [`crate::time`] and read here, and this tolerates either tag rather than
/// asserting one, because which of `int` and `uint` a component is held under is that module's
/// business and not a fact this one should pin from outside.
fn units_of(value: Value) -> Option<i64> {
    value
        .as_int()
        .or_else(|| value.as_uint().and_then(|held| i64::try_from(held).ok()))
}

/// `{runAt: …}` as epoch milliseconds, or `None` for the call that left it out.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not an integer, which the class's own layout rules out.
fn run_at_of(args: &[Value]) -> Result<Option<i64>, Fault> {
    if matches!(args[RUN_AT_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let receiver = crate::instance::receiver(args[RUN_AT_ARG], &crate::time::INSTANT, "push")?;
    let seconds = units_of(crate::instance::slot(
        receiver,
        crate::time::INSTANT_SECONDS_SLOT,
    ));
    let nanos = units_of(crate::instance::slot(
        receiver,
        crate::time::INSTANT_NANOS_SLOT,
    ));
    let (Some(seconds), Some(nanos)) = (seconds, nanos) else {
        return Err(Fault::fatal(format!(
            "{PUSH}: `runAt` is a `Core\\Time\\Instant` whose components are not both integers"
        )));
    };
    // Milliseconds, so the row is one column and every backend reads it the same way — the module
    // doc owns why an instant is a `bigint` here at all. Saturating at both ends, since an instant
    // far enough out to overflow is one no worker will ever reach either way.
    Ok(Some(
        seconds
            .saturating_mul(1_000)
            .saturating_add(nanos / 1_000_000),
    ))
}

/// `{backoff: …}` as milliseconds, or `None` for the call that left it out.
///
/// # Errors
///
/// A thrown `LogicError` for a negative duration — a backoff that runs the next attempt before the
/// one that failed.
fn backoff_of(args: &[Value]) -> Result<Option<i64>, Fault> {
    if matches!(args[BACKOFF_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let nanos = crate::time::nanos_of(args, BACKOFF_ARG, "push")?;
    if nanos < 0 {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{PUSH}: `backoff` cannot be negative, and this one is {nanos}ns"),
        ));
    }
    Ok(Some(nanos / 1_000_000))
}

/// `{maxAttempts: …}`, or the `[queue] max_attempts` the operator configured.
///
/// # Errors
///
/// A thrown `LogicError` for `0`. The configured value cannot be `0` — `nvs_config::queue` refuses
/// one at boot as `E0617` — so this only ever fires for a call site that wrote it.
fn max_attempts_of(args: &[Value], configured: u32) -> Result<u32, Fault> {
    if matches!(args[MAX_ATTEMPTS_ARG].tag(), Some(Tag::Null)) {
        return Ok(configured);
    }
    let written = args[MAX_ATTEMPTS_ARG].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "{PUSH}: expected a `uint` for `maxAttempts`, got tag {}",
            args[MAX_ATTEMPTS_ARG].tag_byte()
        ))
    })?;
    if written == 0 {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{PUSH}: `maxAttempts` of 0 asks for a job that is dead-lettered by the enqueue \
                 that created it — ADR 0084 § 6 makes attempts finite, not optional"
            ),
        ));
    }
    Ok(u32::try_from(written).unwrap_or(u32::MAX))
}

/// `{args: …}` as the JSON the row holds, or `None` for a payload that was not given.
///
/// [`crate::json`]'s own encoder and not a second one, so a `float` or a nested array is spelled in
/// a job's payload exactly as `Core\Json::encode` spells it — the same reason that type is
/// `pub(crate)` for [`crate::log`].
///
/// # Errors
///
/// A thrown `RuntimeError` for a payload JSON cannot hold, which is what the encoder refuses: an
/// infinite `float`, or nesting past its depth bound.
fn payload_of(args: &[Value]) -> Result<Option<String>, Fault> {
    if matches!(args[ARGS_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    serde_json::to_string(&crate::json::Encodable::document(args[ARGS_ARG]))
        .map(Some)
        .map_err(|refused| Fault::thrown(format!("{PUSH}: `args` cannot be stored: {refused}")))
}

/// The queue's PostgreSQL connection, by the key it is filed under.
///
/// [`crate::db`]'s `postgres_of` one module over, and separate rather than shared because the two
/// name their connection differently: that one has the caller's own `Core\Db\Connection` to quote
/// back, and this one has the block name out of configuration, which is a `&str` and not a `Value`.
///
/// # Errors
///
/// A thrown `RuntimeError` for a driver with no statement path yet ([`crate::db`]'s gap 2). A
/// [`Fault::fatal`] for a key the request's own table does not hold, which is this crate's paste
/// error rather than a program's.
fn postgres_of<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
    block: &str,
) -> Result<&'a mut nvs_db::PgConn, Fault> {
    let filed = ctx.open_connection_mut(key).ok_or_else(|| {
        Fault::fatal(format!(
            "{PUSH}: no connection is filed under the key {key}"
        ))
    })?;
    let connection = filed
        .as_any_mut()
        .downcast_mut::<nvs_db::Connection>()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{PUSH}: the connection filed under the key {key} is not `nvs-db`'s"
            ))
        })?;
    let driver = connection.driver();
    let nvs_db::Connection::Postgres(postgres) = connection else {
        return Err(Fault::thrown(format!(
            "{PUSH}: `[db.{block}]` is a {driver:?} connection, and only PostgreSQL runs a \
             statement so far"
        )));
    };
    Ok(postgres)
}

nvs_runtime::nvs_helper! {
    /// `Core\Queue::push(string $script, {…}): Queue\Id` — ADR 0084 §§ 1 and 3.
    ///
    /// **The order of the work is the point, not an accident of writing.** Every argument is read
    /// and judged before the connection is reached, so a call that wrote `maxAttempts: 0` refuses
    /// without opening anything — the same ordering `Core\Db::connect` states between its options
    /// and its grant, and for the same reason: a refusal about the caller's own arguments should
    /// not depend on whether a database was reachable.
    ///
    /// **The connection is reached through the memo first**, which is the whole of § 3. If this
    /// request has already opened `[queue] connection` — because it is the application's own
    /// database, which § 2 calls the recommended configuration — then this insert runs on that
    /// connection, inside whatever transaction it is in, and commits with it. If it has not, the
    /// insert is its own committed statement, and § 3 says exactly that.
    ///
    /// **What it spends:** one statement, plus the connection if the request had not already opened
    /// one — which is then held for the rest of the request like any other, and pooled after it.
    fn nvs_core_queue_push(ctx, args: [7]) {
        // Unreachable from source: the row types this parameter `string`, so a non-text argument is
        // refused at `E0401` first — `Core\Db::connect`'s guard states the same judgement.
        let script = args[SCRIPT_ARG]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{PUSH}: expected a `string` script, got tag {}",
                    args[SCRIPT_ARG].tag_byte()
                ))
            })?
            .to_owned();
        let queue = args[QUEUE_ARG]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{PUSH}: expected a `string` for `queue`, got tag {}",
                    args[QUEUE_ARG].tag_byte()
                ))
            })?
            .to_owned();
        let key = args[KEY_ARG].as_text().map(str::to_owned);
        let payload = payload_of(args)?;
        let run_at = run_at_of(args)?;
        let backoff = backoff_of(args)?;

        // The snapshot is cloned rather than borrowed for `Core\Db::connect`'s reason: the tree, the
        // bounds read out of it and the `ctx` that files the connection are all live at once. It is
        // an `Arc` shared by every request on the core, so the clone is one refcount.
        let snapshot = ctx
            .config()
            .map(|config| std::sync::Arc::clone(config.snapshot()))
            .ok_or_else(|| {
                Fault::thrown(format!(
                    "{PUSH}: this program is running with no configuration at all, so there is no \
                     `[queue]` block to say which database a job would live in"
                ))
            })?;
        // The refusal arm cannot fire: `nvs_config::resolve` runs this same pass at boot and a tree
        // that failed it never started a request. It is a `fatal` rather than a `thrown` for that
        // reason — reaching it means the two passes disagree, which is this crate's bug and not a
        // condition a program can produce or catch.
        let configured = nvs_config::queue::queue_for(&snapshot.config, &BTreeMap::new())
            .map_err(|_| {
                Fault::fatal(format!(
                    "{PUSH}: the `[queue]` block did not resolve here, and boot accepted it"
                ))
            })?
            .ok_or_else(|| {
                Fault::thrown(format!(
                    "{PUSH}: this deployment writes no `[queue]` block, so nothing names the \
                     database a job would live in — ADR 0084 § 2 is the block, and `connection` is \
                     the field"
                ))
            })?;
        let max_attempts = max_attempts_of(args, configured.max_attempts)?;

        // Shared, and that is § 3 rather than an economy: `{shared: false}` would open a second
        // connection, outside whatever transaction the request has open on the first, and the
        // enqueue would commit on its own. The memo is what makes the property hold.
        let handle = crate::db::open_named(ctx, &configured.connection, true, None, PUSH)?;
        let now = now_millis();
        let sending: [Option<Vec<u8>>; 9] = [
            key.map(String::into_bytes),
            Some(queue.clone().into_bytes()),
            Some(script.into_bytes()),
            payload.map(String::into_bytes),
            Some(PENDING.to_string().into_bytes()),
            Some(max_attempts.to_string().into_bytes()),
            backoff.map(|millis| millis.to_string().into_bytes()),
            Some(run_at.unwrap_or(now).to_string().into_bytes()),
            Some(now.to_string().into_bytes()),
        ];
        let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();

        let block = configured.connection.clone();
        let postgres = postgres_of(ctx, handle, &block)?;
        let mut answered = postgres.query(INSERT, &bound).map_err(|refused| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{PUSH}: the insert into `{JOBS_TABLE}` on `[db.{block}]` was refused: \
                     {refused} — `nvs queue migrate` is what creates that table"
                ),
            )
        })?;
        // Every row is read before the id is asked for, exactly as `Core\Db\Connection::execute`
        // does it: the connection has to be back at a message boundary before this returns, or the
        // next statement on it — the caller's own, inside the same transaction — meets a busy one.
        while answered
            .next_row()
            .map_err(|refused| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{PUSH}: reading the id back from `{JOBS_TABLE}` failed: {refused}"),
                )
            })?
            .is_some()
        {}
        // ADR 0067 § 4's `lastId`, which on PostgreSQL is what the `returning` clause handed back —
        // so the decoding is the driver's and this member parses nothing. `None` would mean the
        // statement's `union all` answered neither an insert nor a pending duplicate, which it
        // cannot: the `existing` arm is the only thing the insert stands down for.
        let id = answered.last_id().ok_or_else(|| {
            Fault::fatal(format!(
                "{PUSH}: the insert into `{JOBS_TABLE}` answered no id at all"
            ))
        })?;
        Ok(crate::instance::build(
            &ID,
            [Value::uint(id), Value::str(NvsStr::new(queue.as_bytes()))],
        ))
    }
}

/// The address of one of *this* module's symbols, or `None` for a symbol that belongs to another
/// domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_queue_push" => (nvs_core_queue_push as *const ()).cast(),
        _ => return None,
    })
}
