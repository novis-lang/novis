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
//! [`JOBS_TABLE`], [`DEAD_TABLE`] and the statements beside them are the one home for what those
//! tables' columns are, and the migrate command will be read off them rather than the other way
//! round. Two choices in it are worth their
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
//! 4. **`stats` is owed** — § 1's roster is four members and three are here. It
//!    additionally waits on a decision rather than on work: it answers a *record* of counters, and
//!    a `Core` instance's properties are unreachable from a program ([`CoreTy::Instance`]'s own
//!    rule), so it is either a shape parameter's twin — gap 1's blocker — or a class whose counters
//!    are members, which is `Core\Db\Write::lastId`'s shape.
//! 5. **PostgreSQL only**, as [`crate::db`]'s gap 2 is: the other four drivers have no statement path
//!    yet, so [`postgres_of`] refuses them by name rather than writing a row nothing would claim.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc, ErrorDoc,
    MethodDoc, ParamDoc, Qual,
};

/// `Core\Queue`'s fully-qualified name.
const NAME: &str = r"Core\Queue";

/// `Core\Queue\Id`'s, as [`CoreTy::Instance`] spells it.
pub(crate) const ID_NAME: &str = r"Core\Queue\Id";

/// `Core\Queue\State`'s, as [`CoreTy::Enum`] spells it.
pub(crate) const STATE_NAME: &str = r"Core\Queue\State";

/// `push`'s name in a refusal, written once so every message spells it the same way.
const PUSH: &str = r"Core\Queue::push";

/// `status`'s, the same way.
const STATUS_OF: &str = r"Core\Queue::status";

/// `cancel`'s.
const CANCEL_OF: &str = r"Core\Queue::cancel";

/// The table § 2's `nvs queue migrate` creates and this module writes into.
///
/// Unqualified on purpose: the block's own `search_path` — the operator's, in root-owned
/// configuration — decides which schema it lands in, exactly as it decides for every statement the
/// application itself writes. A name this module qualified would be a second answer to a question
/// [`nvs_config::db`] has already given.
const JOBS_TABLE: &str = "nvs_jobs";

/// § 6's dead-letter table, unqualified for [`JOBS_TABLE`]'s reason.
///
/// **Only two of its columns are decided here**, and they are the two [`STATUS`] reads: a job keeps
/// the `id` and the `queue` it had in [`JOBS_TABLE`], so a `Core\Queue\Id` handed out before the job
/// exhausted its attempts still names it afterwards. What else the row carries — § 6's payload,
/// every attempt's error and its timing — belongs to the member that writes one, which is the
/// worker, and deciding it here would be deciding it twice.
const DEAD_TABLE: &str = "nvs_dead_jobs";

/// `Core\Queue\State::Pending`'s ordinal, which is what a freshly pushed row's `state` is.
///
/// Written as the number rather than read off [`STATE`] because the SQL beside it cannot read the
/// enum either — a statement's `state = 0` is a literal in a string — so one spelling of the rule
/// covering both is worth more than two half-rules. `queue_statements_agree_with_the_state_enum` is
/// that spelling: it holds this constant and the ordinals inside [`INSERT`] and [`STATUS`] to
/// [`STATE`]'s own cases.
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

/// ADR 0084 §§ 1 and 6's `status`, as one statement over both of § 2's tables.
///
/// **Two tables and not one**, because § 6 *moves* a job that has exhausted its attempts into the
/// dead-letter table rather than deleting it, and a caller asking what became of its job is owed
/// `Dead` rather than a refusal. The `union all` is [`INSERT`]'s shape for [`INSERT`]'s reason: two
/// statements would be two moments, and one is what makes the answer a fact about a single instant.
/// A job cannot be in both arms at once, because moving it is one transaction of the worker's.
///
/// The `3` in the second arm is `Core\Queue\State::Dead`'s ordinal, written as a literal because no
/// `const` can reach inside a SQL string; `queue_statements_agree_with_the_state_enum` is what holds
/// it to [`STATE`]. That arm also costs nothing under a design where a dead job stays in
/// [`JOBS_TABLE`] with its state written instead of moving — the first arm is tried first and the
/// `limit 1` takes it — so the statement is correct either way and the worker is free to choose.
const STATUS: &str = "select state from nvs_jobs \
    where id = $1::bigint and queue = $2::text \
    union all \
    select 3 from nvs_dead_jobs \
    where id = $1::bigint and queue = $2::text \
    limit 1";

/// ADR 0084 § 1's `cancel`, as one conditional update.
///
/// **`and state = 0` is the whole of the member's semantics, and it is in the statement rather than
/// in a check before it.** A job is cancellable only while it is pending, so reading its state and
/// then writing it would be two moments with a worker's claim free to land between them — and the
/// answer this hands back would then be about the first moment. Written this way the database
/// decides, one row is affected or none is, and `returning id` is how many.
///
/// **An update and not a delete**, because a program that cancels a job and then asks `status`
/// about it is owed an answer rather than a refusal; [`STATE`]'s `Cancelled` case is the answer.
/// The `4` is that case's ordinal, held to the enum by
/// `queue_statements_agree_with_the_state_enum` for the reason [`STATUS`]'s `3` is.
const CANCEL: &str = "update nvs_jobs set state = 4 \
    where id = $1::bigint and queue = $2::text and state = 0 \
    returning id";

/// A [`ID`]'s first slot: the primary key the insert returned.
const ID_SLOT: &str = "id";

/// Its second: the queue the job is in, so a `cancel` or a `status` written against this value knows
/// which queue to look in without asking the row again.
const ID_QUEUE_SLOT: &str = "queue";

/// [`ID_SLOT`]'s index, which is what [`crate::instance::slot`] takes.
const ID_AT: usize = 0;

/// [`ID_QUEUE_SLOT`]'s.
const ID_QUEUE_AT: usize = 1;

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

/// ADR 0084 § 1's `Core\Queue` — `push`, `status` and `cancel`, with `stats` owed (gap 4).
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
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
        },
        // The receipt in, a state out, and nothing between them that a program has to
        // spell: § 6's at-least-once delivery is why this is a state to be read rather
        // than a completion to be handed, and [`STATE`] is where that reading lives.
        CoreMethod {
            name: "status",
            names: &["job"],
            // The receipt itself and not the row's number, which is the whole of why
            // `push` answers a class: `status` has to know the queue as well.
            params: &[CoreTy::Instance(ID_NAME)],
            defaults: &[],
            return_ty: CoreTy::Enum(STATE_NAME),
            symbol: "nvs_core_queue_status",
            doc: Some(&STATUS_DOC),
        },
        // The same receipt, and an answer that is a `bool` although § 1 annotates no
        // return at all: what a caller of this member needs to know is whether it got
        // there first, and there is no other way for it to find out. [`CANCEL`] owns
        // why that race is the ordinary case rather than an unlucky one.
        CoreMethod {
            name: "cancel",
            names: &["job"],
            params: &[CoreTy::Instance(ID_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_queue_cancel",
            doc: Some(&CANCEL_DOC),
        },
    ],
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

/// `Core\Queue::status`'s reference card — ADR 0117.
const STATUS_DOC: MethodDoc = MethodDoc {
    short: "Reports what has become of one job, as a `Core\\Queue\\State` case. Delivery is \
            at-least-once, which is why this is a state a program reads rather than a completion it \
            is handed: a job may run twice, so \"it ran\" is a fact about the row.",
    params: &[ParamDoc {
        name: "job",
        desc: "The receipt `push` answered with, which names both the row and the queue it is in.",
        shape: &[],
    }],
    ret: "`Pending` while it waits — including while a `runAt` or a retry's backoff has not \
          elapsed — `Claimed` while a worker holds it, `Succeeded` once it has run, and `Dead` once \
          it has exhausted its attempts.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This deployment writes no `[queue]` block, so nothing says which database the \
                   job would be in; or the queue's connection names a driver that cannot yet run a \
                   statement; or neither table holds the job, which means it was enqueued by \
                   another deployment or removed by hand.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The queue's connection did not open, or the query was refused by the server — \
                   most often because `nvs queue migrate` has not created the tables.",
        },
    ],
};

/// `Core\Queue::cancel`'s reference card — ADR 0117.
const CANCEL_DOC: MethodDoc = MethodDoc {
    short: "Takes one job out of the queue, if it is still waiting. A job a worker has already \
            claimed is running now and is not stopped: cancelling is a change to a row, and there \
            is no protocol for interrupting work in flight.",
    params: &[ParamDoc {
        name: "job",
        desc: "The receipt `push` answered with, which names both the row and the queue it is in.",
        shape: &[],
    }],
    ret: "`true` if this call is what took the job out of the queue, and `false` if there was \
          nothing pending left to take — because a worker claimed it first, because it has already \
          run, or because an earlier `cancel` got there. `status` then answers `Cancelled`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This deployment writes no `[queue]` block, so nothing says which database the \
                   job would be in; or the queue's connection names a driver that cannot yet run a \
                   statement.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The queue's connection did not open, or the update was refused by the server — \
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

/// ADR 0084 §§ 4 and 6's job lifecycle, as § 1's `Core\Queue\State`.
///
/// **§ 1 is the home of the roster and of why there is no `Failed`**; this is the home of what a
/// case *is* here, which is a stored number. Each case is a resting state of a row rather than an
/// event: a job whose `runAt` has not come round is `Pending`, because waiting for a worker and
/// waiting for a clock are one thing to everything that reads this column, and a case separating
/// them would be a distinction no claim statement makes. `Cancelled` is what a cancel *is* on a
/// table nobody polls twice — [`CANCEL`] writes it in place of `Pending`, and every claim statement
/// reads `Pending`, so a job leaves the queue by changing one column.
///
/// **The values are declaration ordinals and mean nothing else**, as they are for every enum here
/// ([`CoreEnum::cases`]' rule) — but these ones are additionally *stored*: § 2's jobs table writes
/// this ordinal in its `state` column, so the enum and the column are one representation and not
/// two, and each case's number is part of the schema `nvs queue migrate` will create. Renumbering
/// one is therefore a migration and not an edit. `queue_statements_agree_with_the_state_enum` pins
/// them for that reason: [`PENDING`] and the ordinals written inside [`INSERT`] and [`STATUS`] are
/// uses of this table that no `const` can reach.
pub(crate) const STATE: CoreEnum = CoreEnum {
    name: STATE_NAME,
    cases: &[
        ("Pending", 0),
        ("Claimed", 1),
        ("Succeeded", 2),
        ("Dead", 3),
        ("Cancelled", 4),
    ],
    doc: Some(&STATE_DOC),
};

/// [`STATE`]'s reference card — ADR 0117.
const STATE_DOC: EnumDoc = EnumDoc {
    short: "What has become of a background job, as `Core\\Queue::status` answers it. Five states \
            and no `Failed`, because a failed attempt is retried: it returns the job to `Pending` \
            rather than ending it.",
    cases: &[
        CaseDoc {
            name: "Pending",
            desc: "Waiting for a worker to claim it — including while its `runAt` is still in the \
                   future, and between attempts while its backoff elapses.",
        },
        CaseDoc {
            name: "Claimed",
            desc: "A worker holds it, under the visibility timeout that returns it to `Pending` if \
                   that worker dies.",
        },
        CaseDoc {
            name: "Succeeded",
            desc: "It ran to completion. Delivery is at-least-once, so this says the work happened \
                   and not that it happened exactly once.",
        },
        CaseDoc {
            name: "Dead",
            desc: "It exhausted its attempts and is in the dead-letter table, with its payload and \
                   every attempt's error. Nothing the runtime does ever removes it from there.",
        },
        CaseDoc {
            name: "Cancelled",
            desc: "`cancel` reached it while it was still pending, so no worker ever will. A job \
                   already claimed cannot arrive here — cancelling does not stop work in flight.",
        },
    ],
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

/// The `[queue]` block this deployment resolved at boot, and the bounds it carries.
///
/// One reading of § 2's block for every member that needs it, for the reason
/// [`crate::db::open_named`] is one reading of `[db.<name>]`: what `[queue] connection` names is a
/// single question, and a second answer to it here would be a second place for it to be answered
/// differently.
///
/// # Errors
///
/// A thrown `RuntimeError` for a deployment that configures no `[queue]` block at all, which is a
/// real tree and not a broken one — a deployment says it runs no jobs by leaving the block out. A
/// [`Fault::fatal`] for a block that does not resolve here, because `nvs_config::resolve` ran this
/// same pass at boot and a tree that failed it never started a request; reaching that arm means the
/// two passes disagree, which is this crate's bug rather than anything a program can catch.
fn configured_queue(
    ctx: &mut nvs_runtime::Ctx,
    member: &str,
) -> Result<nvs_config::queue::QueueBounds, Fault> {
    // The snapshot is cloned rather than borrowed for `Core\Db::connect`'s reason: the tree, the
    // bounds read out of it and the `ctx` that files the connection are all live at once. It is an
    // `Arc` shared by every request on the core, so the clone is one refcount.
    let snapshot = ctx
        .config()
        .map(|config| std::sync::Arc::clone(config.snapshot()))
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{member}: this program is running with no configuration at all, so there is no \
                 `[queue]` block to say which database a job would live in"
            ))
        })?;
    nvs_config::queue::queue_for(&snapshot.config, &BTreeMap::new())
        .map_err(|_| {
            Fault::fatal(format!(
                "{member}: the `[queue]` block did not resolve here, and boot accepted it"
            ))
        })?
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{member}: this deployment writes no `[queue]` block, so nothing names the \
                 database a job would live in — ADR 0084 § 2 is the block, and `connection` is the \
                 field"
            ))
        })
}

/// The row and the queue one `Core\Queue\Id` names.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not an id, or whose slots hold the wrong tags: both slots
/// are written by [`nvs_core_queue_push`] and by nothing else, so either is a paste error in this
/// crate rather than anything a program can produce — `Core\Db`'s `connection_of` states the same
/// reading of the identical pair.
fn job_of(value: Value, member: &str) -> Result<(u64, String), Fault> {
    let held = value.obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{member}: expected a `{ID_NAME}`, got tag {}",
            value.tag_byte()
        ))
    })?;
    let id = crate::instance::slot(held, ID_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member}: expected {:?} in the `{ID_SLOT}` slot of a `{ID_NAME}`",
                Tag::Uint
            ))
        })?;
    let queue = crate::instance::slot(held, ID_QUEUE_AT)
        .as_text()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member}: expected text in the `{ID_QUEUE_SLOT}` slot of a `{ID_NAME}`"
            ))
        })?
        .to_owned();
    Ok((id, queue))
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
    member: &str,
) -> Result<&'a mut nvs_db::PgConn, Fault> {
    let filed = ctx.open_connection_mut(key).ok_or_else(|| {
        Fault::fatal(format!(
            "{member}: no connection is filed under the key {key}"
        ))
    })?;
    let connection = filed
        .as_any_mut()
        .downcast_mut::<nvs_db::Connection>()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member}: the connection filed under the key {key} is not `nvs-db`'s"
            ))
        })?;
    let driver = connection.driver();
    let nvs_db::Connection::Postgres(postgres) = connection else {
        return Err(Fault::thrown(format!(
            "{member}: `[db.{block}]` is a {driver:?} connection, and only PostgreSQL runs a \
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

        let configured = configured_queue(ctx, PUSH)?;
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
        let postgres = postgres_of(ctx, handle, &block, PUSH)?;
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

nvs_runtime::nvs_helper! {
    /// `Core\Queue::status(Queue\Id $job): Queue\State` — ADR 0084 §§ 1 and 6.
    ///
    /// **The receipt is the whole argument**, because it carries the queue as well as the row: a
    /// member taking a bare id would have to be told the queue beside it or search every one, and
    /// the first is what [`ID`] exists to spare the caller.
    ///
    /// **One statement over both of § 2's tables**, so the answer describes one instant.
    /// [`STATUS`] owns why that is two arms rather than two queries, and the reading it rests on is
    /// § 6's: a job that ran out of attempts is moved rather than deleted, so there is a row to
    /// answer from until an operator removes one.
    ///
    /// **What it spends:** one statement, on the connection the request either already held or now
    /// holds for the rest of it. [`nvs_core_queue_push`]'s reading of § 3 applies unchanged in the
    /// other direction — a `status` inside a transaction on that connection sees that
    /// transaction's own enqueues, because it is the same connection and not a second one.
    fn nvs_core_queue_status(ctx, args: [1]) {
        let (id, queue) = job_of(args[0], STATUS_OF)?;
        let block = configured_queue(ctx, STATUS_OF)?.connection;
        // Shared, for `push`'s reason: reading a job's state on a second connection would be
        // reading it from outside whatever transaction the request has open on the first.
        let handle = crate::db::open_named(ctx, &block, true, None, STATUS_OF)?;
        let sending: [Option<Vec<u8>>; 2] = [
            Some(id.to_string().into_bytes()),
            Some(queue.clone().into_bytes()),
        ];
        let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
        let refused_by_server = |refused: &dyn std::fmt::Display| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{STATUS_OF}: reading job {id} back from `{JOBS_TABLE}` and `{DEAD_TABLE}` on \
                     `[db.{block}]` was refused: {refused} — `nvs queue migrate` is what creates \
                     those tables"
                ),
            )
        };
        let postgres = postgres_of(ctx, handle, &block, STATUS_OF)?;
        let mut answered = postgres
            .query(STATUS, &bound)
            .map_err(|refused| refused_by_server(&refused))?;
        // Taken before the first row, as `Core\Db`'s own reader takes it: a `PgRows` lends its
        // columns and its rows out of one borrow, and the rows are read with it held mutably.
        let columns: Vec<nvs_db::PgColumn> = answered.columns().to_vec();
        // Every row is read before the answer is judged, exactly as `push` reads its id back: the
        // connection has to be back at a message boundary before this returns, or the caller's
        // next statement — inside the same transaction — meets a busy one. `limit 1` makes that
        // one row, so the guard below is about the shape of the loop and not about a second row.
        // The outer `Option` is "there was a row", the inner one "its column was an integer",
        // both judged after the drain rather than inside it.
        let mut read: Option<Option<i64>> = None;
        loop {
            let Some(row) = answered
                .next_row()
                .map_err(|refused| refused_by_server(&refused))?
            else {
                break;
            };
            if read.is_none() {
                let body = row.column(0).map_err(|refused| refused_by_server(&refused))?;
                let scalar = columns[0]
                    .scalar(body)
                    .map_err(|refused| refused_by_server(&refused))?;
                read = Some(match scalar {
                    nvs_db::PgScalar::Int(ordinal) => Some(ordinal),
                    _ => None,
                });
            }
        }
        let ordinal = read
            .ok_or_else(|| {
                Fault::thrown(format!(
                    "{STATUS_OF}: no job {id} is in the `{queue}` queue on `[db.{block}]`, in \
                     either `{JOBS_TABLE}` or `{DEAD_TABLE}` — a receipt this deployment issued \
                     names a row in one of them, so this one was issued by another deployment or \
                     the row was removed by hand"
                ))
            })?
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{STATUS_OF}: `{JOBS_TABLE}`.`state` came back as something other than an \
                     integer, and this module's own schema is what declares it one"
                ))
            })?;
        // Matched against the enum rather than cast to it, because the column is written by a
        // worker and read here: a number no case names is a table this build does not understand,
        // which is a deployment running two versions of the schema and not a program's error.
        let (_, case) = STATE
            .cases
            .iter()
            .find(|(_, value)| *value == ordinal)
            .ok_or_else(|| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!(
                        "{STATUS_OF}: job {id} is in state {ordinal}, which no `{STATE_NAME}` case \
                         names — `nvs queue migrate` and this runtime are at different versions"
                    ),
                )
            })?;
        Ok(Value::int(*case))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Queue::cancel(Queue\Id $job): bool` — ADR 0084 § 1.
    ///
    /// **It answers a `bool` although § 1 annotates no return**, and that is a decision rather than
    /// a liberty: the member's own semantics are a race it can lose — § 4 lets a worker claim the
    /// job at any moment, and it is the *ordinary* outcome for a job cancelled late, not an unlucky
    /// one — so a caller has no other way to learn whether the work is still going to happen. A
    /// `void` spelling would make "cancelled" and "too late" look identical at the call site, and
    /// throwing for the second would make the commonest race an exception. ADR 0084 § 1 carries the
    /// annotation now, so there is one home for it.
    ///
    /// **The state test is in [`CANCEL`] and not here**, which is why nothing in this body reads
    /// the job's state first. The whole member is one statement for the reason `push` is: two would
    /// be two moments and the answer would be about the earlier one.
    ///
    /// **What it spends:** one statement, on the connection the request either already held or now
    /// holds for the rest of it — so a cancel inside a transaction on that connection is undone
    /// with it if that transaction rolls back, exactly as § 3's enqueue commits with it.
    fn nvs_core_queue_cancel(ctx, args: [1]) {
        let (id, queue) = job_of(args[0], CANCEL_OF)?;
        let block = configured_queue(ctx, CANCEL_OF)?.connection;
        // Shared, for `push`'s reason: cancelling on a second connection would be cancelling from
        // outside whatever transaction the request has open on the first, and a rolled-back
        // request would have taken a job out of the queue anyway.
        let handle = crate::db::open_named(ctx, &block, true, None, CANCEL_OF)?;
        let sending: [Option<Vec<u8>>; 2] = [
            Some(id.to_string().into_bytes()),
            Some(queue.clone().into_bytes()),
        ];
        let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
        let refused_by_server = |refused: &dyn std::fmt::Display| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{CANCEL_OF}: taking job {id} out of `{JOBS_TABLE}` on `[db.{block}]` was \
                     refused: {refused} — `nvs queue migrate` is what creates that table"
                ),
            )
        };
        let postgres = postgres_of(ctx, handle, &block, CANCEL_OF)?;
        let mut answered = postgres
            .query(CANCEL, &bound)
            .map_err(|refused| refused_by_server(&refused))?;
        // The rows are counted rather than read: `returning id` is here to make the affected count
        // observable and nothing reads the id, since the caller already holds it. Draining is what
        // `push` and `status` drain for — the connection owes the caller a message boundary before
        // the next statement on it, which may be the caller's own inside the same transaction.
        let mut cancelled = false;
        while answered
            .next_row()
            .map_err(|refused| refused_by_server(&refused))?
            .is_some()
        {
            cancelled = true;
        }
        Ok(Value::bool(cancelled))
    }
}

/// The address of one of *this* module's symbols, or `None` for a symbol that belongs to another
/// domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_queue_push" => (nvs_core_queue_push as *const ()).cast(),
        "nvs_core_queue_status" => (nvs_core_queue_status as *const ()).cast(),
        "nvs_core_queue_cancel" => (nvs_core_queue_cancel as *const ()).cast(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{CANCEL, DEAD_TABLE, INSERT, JOBS_TABLE, PENDING, STATE, STATUS};

    /// The two statements above write and read [`STATE`]'s ordinals as SQL literals, which no
    /// `const` can reach into. This is the assertion [`PENDING`]'s doc comment owes: the enum a
    /// program compares against and the column a worker claims from are one representation, and
    /// nothing else would notice them drifting apart.
    #[test]
    fn queue_statements_agree_with_the_state_enum() {
        let case = |name: &str| {
            STATE
                .cases
                .iter()
                .find(|(spelt, _)| *spelt == name)
                .unwrap_or_else(|| panic!("`{}` declares no `{name}` case", STATE.name))
                .1
        };
        assert_eq!(
            i64::from(PENDING),
            case("Pending"),
            "a pushed row's `state` is what `status` reads back as `Pending`"
        );
        assert_eq!(case("Pending"), 0, "`INSERT`'s dedupe arm spells this `0`");
        assert!(
            INSERT.contains("state = 0"),
            "`INSERT` reads pending rows by the ordinal above"
        );
        assert_eq!(
            case("Dead"),
            3,
            "`STATUS`'s dead-letter arm spells this `3`"
        );
        assert!(
            STATUS.contains("select 3 from nvs_dead_jobs"),
            "`STATUS` answers the ordinal above for a dead-lettered job"
        );
        assert!(
            STATUS.contains(JOBS_TABLE) && STATUS.contains(DEAD_TABLE),
            "`STATUS` reads both of § 2's tables"
        );
        assert_eq!(
            case("Cancelled"),
            4,
            "`CANCEL` writes this ordinal in place of `Pending`"
        );
        assert!(
            CANCEL.contains("set state = 4") && CANCEL.contains("and state = 0"),
            "`CANCEL` moves a job from the ordinal above to the one before it, and only that one"
        );
    }
}
