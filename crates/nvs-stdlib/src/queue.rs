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
//! **The schema is this module's, and `nvs queue migrate` reads it.** § 2 makes the runtime own one
//! jobs table and one dead-letter table, created by an explicit operator command — DDL is an
//! injection sink and never issued from a request — so `push` writes into a table it does not create.
//! [`MIGRATION`] is that command's whole schema and the one home for what those tables' columns are;
//! `nvs-cli`'s `queue` module runs it and decides nothing about it. Two choices in it are worth their
//! sentence: every instant is a `bigint` of epoch milliseconds rather than a timestamp, because § 2
//! supports all five of ADR 0067's backends and five timestamp dialects is exactly the cost a
//! runtime-owned table should not carry; and `state` is the ordinal `Core\Queue\State` already is at
//! runtime ([ADR 0010](../../../docs/adr/0010-enums-are-a-value-type.md)), so the enum and the column
//! are one representation and not two.
//!
//! **What it spends:** one statement per member call, on a connection the request either already
//! held or now holds for the rest of it, plus one JSON encoding of `$args` sized by the payload the
//! caller wrote. Nothing is held between calls, except the four counters `stats` answers with for
//! as long as its caller keeps the record.
//!
//! # Known gaps
//!
//! 1. **`limits` and `grants` are not declared**, because both are § 1's `{…}` — a *shape* parameter,
//!    which this registry still cannot spell. That is the same blocker `Core\Db::open` waits on
//!    ([`crate::db`]'s own gaps), and the two lift together.
//! 2. **`$args` is `mixed` and so does not refuse a `secret`**, which § 1 asks for. A durable row is
//!    an output and ADR 0033's five sinks are the shape of the eventual answer; `CoreTy::Mixed`
//!    carries no qualifier, so saying it needs a spelling the registry has not got.
//! 3. **`key`'s "at most one pending job per key" is enforced by the statement, and by the index
//!    only where the migration has been applied.** [`INSERT`]'s `existing` arm reads the table
//!    inside the same statement that writes it, which is correct against every other `push` on a
//!    *serialized* transaction and racy against a concurrent one at `read committed`. The partial
//!    unique index over `(dedupe_key) where state = 0` is [`MIGRATION`]'s `jobs.dedupe`, and the
//!    statement is race-free against a schema carrying it without changing shape — so what is left
//!    of this gap is a deployment that never ran `nvs queue migrate`, which is the one case the
//!    index is absent in.
//! 4. **`stats` counts the four things § 6 names and no fifth**, and a fifth would be a column in
//!    § 2's schema before it is a member here. The sharp edge is a dead-lettered job's own
//!    attempts: § 6 *moves* that row to [`DEAD_TABLE`], whose columns this module deliberately does
//!    not decide beyond `id` and `queue`, so [`COUNTS`] sums `attempts` over [`JOBS_TABLE`] alone
//!    and counts the depth separately rather than inventing a column for the sum to reach.
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

/// `Core\Queue\Stats`'s, as [`CoreTy::Instance`] spells it.
pub(crate) const STATS_NAME: &str = r"Core\Queue\Stats";

/// `push`'s name in a refusal, written once so every message spells it the same way.
const PUSH: &str = r"Core\Queue::push";

/// `status`'s, the same way.
const STATUS_OF: &str = r"Core\Queue::status";

/// `cancel`'s.
const CANCEL_OF: &str = r"Core\Queue::cancel";

/// `stats`'s.
const STATS_OF: &str = r"Core\Queue::stats";

/// The table § 2's `nvs queue migrate` creates and this module writes into.
///
/// Unqualified on purpose: the block's own `search_path` — the operator's, in root-owned
/// configuration — decides which schema it lands in, exactly as it decides for every statement the
/// application itself writes. A name this module qualified would be a second answer to a question
/// [`nvs_config::db`] has already given.
const JOBS_TABLE: &str = "nvs_jobs";

/// § 6's dead-letter table, unqualified for [`JOBS_TABLE`]'s reason.
///
/// **Two of its columns are all this module reads, and [`MIGRATION`] is where every one of them is
/// written down.** A job keeps the `id` and the `queue` it had in [`JOBS_TABLE`], so a
/// `Core\Queue\Id` handed out before the job exhausted its attempts still names it afterwards, and
/// that pair is the whole of what [`STATUS`] and [`COUNTS`] ask of the table. What else the row
/// carries — § 6's payload, every attempt's error and its timing — is decided by the DDL below and
/// not by the worker that will write one: a column has to exist before anything can move a row into
/// it, so the migration is the earlier of the two decisions and the only one there is room for.
const DEAD_TABLE: &str = "nvs_dead_jobs";

/// `Core\Queue\State::Pending`'s ordinal, which is what a freshly pushed row's `state` is.
///
/// Written as the number rather than read off [`STATE`] because the SQL beside it cannot read the
/// enum either — a statement's `state = 0` is a literal in a string — so one spelling of the rule
/// covering both is worth more than two half-rules. `queue_statements_agree_with_the_state_enum` is
/// that spelling: it holds this constant and the ordinals inside [`INSERT`] and [`STATUS`] to
/// [`STATE`]'s own cases.
const PENDING: i16 = 0;

/// One statement of § 2's schema, under the name an operator sees it by.
///
/// A **label** rather than a table name, because three of the five statements below are indexes on a
/// table an earlier one created, and the question an operator reading `nvs queue migrate` has is
/// which of § 2's *two* tables a statement belongs to. So the label is that table's role in the ADR
/// — `jobs` or `dead_letter` — dotted with what the statement adds when it is not the `create table`
/// itself, and what those tables are actually called stays [`JOBS_TABLE`]'s and [`DEAD_TABLE`]'s
/// business for the reason those two constants give.
#[derive(Debug)]
pub struct Migration {
    /// Which of § 2's two tables this statement builds, dotted with what it adds to it.
    pub label: &'static str,
    /// The statement, carrying no separator: a driver is handed one statement at a time, and the
    /// `;` belongs to whatever is printing them for a human instead.
    pub sql: &'static str,
}

/// ADR 0084 § 2's schema, in the order `nvs queue migrate` runs it.
///
/// **This is the one home for what the queue's tables are**, and the command reads it rather than
/// carrying a copy: every column below is one a statement in this module binds or reads, and
/// `the_ddl_creates_every_column_the_statements_name` holds the two lists together — a column
/// renamed here and nowhere else fails that test rather than a deployment.
///
/// **PostgreSQL's dialect, because it is the only driver with a statement path at all** (gap 5). A
/// second backend brings its own list rather than a dialect switch inside these strings: the
/// identity column, the partial index and `if not exists` are each spelt differently across § 2's
/// five, and a string with three holes in it has stopped being a statement.
///
/// **`if not exists` on every one, because § 2 says *created and upgraded*.** Running the command
/// twice is not an error and running it against a half-built schema completes it, which is what
/// makes it an operator's ordinary answer to "is this deployment's queue ready" rather than a
/// one-shot they have to remember having run.
///
/// Three things in it are decisions rather than transcription:
///
/// - **Every instant is a `bigint` of epoch milliseconds** and never a timestamp — this module's own
///   doc owns why, and it is the dialect argument above applied to the type map.
/// - **`claimed_at` is when the claim was taken, not when it expires.** § 4's visibility timeout is
///   `[queue] visibility` measured from it, so the bound stays in configuration where
///   [ADR 0078](../../../docs/adr/0078-config-reload-and-control-socket.md) § 1's reload can move
///   it; a stored deadline would freeze the superseded bound onto every job already claimed.
/// - **The dead-letter row is the job's own columns plus `failed_at` and `errors`**, where `errors`
///   is the JSON array § 6 asks for — an entry carrying when an attempt ran and what it threw.
///   **It is one entry deep, and that entry is the attempt that exhausted the job**, because the
///   jobs table above has nowhere to keep what an earlier attempt threw: [`RETRY`] arms a failed
///   row for the next attempt and keeps the count and nothing else. Recording all of them would be
///   a text column on `nvs_jobs` appended to on every failure — a row rewritten once per attempt,
///   carrying a value only the exhausted job ever reads, on the table § 4's claim contends over —
///   so the array is § 6's shape at the depth this schema pays for, and [`dead_errors`] is where
///   that trade is written down. There is no `state`: a row is `Dead` by being in that table, which
///   is exactly what [`STATUS`]'s second arm asserts by answering the ordinal as a literal.
pub const MIGRATION: &[Migration] = &[
    Migration {
        label: "jobs",
        sql: "create table if not exists nvs_jobs (\
              id bigint generated always as identity primary key, \
              queue text not null, \
              script text not null, \
              args text, \
              state smallint not null, \
              attempts int not null, \
              max_attempts int not null, \
              backoff_ms bigint not null, \
              run_at bigint not null, \
              dedupe_key text, \
              created_at bigint not null, \
              claimed_at bigint)",
    },
    Migration {
        // Gap 3's index, and the one statement here that changes what a member *means*: with it in
        // place `INSERT`'s `existing` arm is race-free against a concurrent push at `read
        // committed`, because the second insert is refused by the index rather than admitted by a
        // guard that read the table a moment earlier.
        label: "jobs.dedupe",
        sql: "create unique index if not exists nvs_jobs_dedupe \
              on nvs_jobs (dedupe_key) where state = 0",
    },
    Migration {
        // § 4's claim order, as the index the claim statement will read: the oldest due job of one
        // queue that nothing holds. A row with no `dedupe_key` is indexed here and not above,
        // which is the ordinary case and the reason these are two indexes.
        label: "jobs.due",
        sql: "create index if not exists nvs_jobs_due on nvs_jobs (queue, state, run_at)",
    },
    Migration {
        label: "dead_letter",
        sql: "create table if not exists nvs_dead_jobs (\
              id bigint primary key, \
              queue text not null, \
              script text not null, \
              args text, \
              attempts int not null, \
              max_attempts int not null, \
              backoff_ms bigint not null, \
              run_at bigint not null, \
              dedupe_key text, \
              created_at bigint not null, \
              failed_at bigint not null, \
              errors text not null)",
    },
    Migration {
        // `COUNTS`'s fourth counter is a scalar subquery over this table, keyed on the queue and on
        // nothing else, so the depth of one queue's dead letters costs a lookup rather than a scan
        // of every queue's.
        label: "dead_letter.queue",
        sql: "create index if not exists nvs_dead_jobs_queue on nvs_dead_jobs (queue)",
    },
];

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
/// `pub` for the reason [`CLAIM`] is, with a different caller: `crates/nvs-stdlib/tests/queue.rs`
/// pushes a job with this statement and then claims and dead-letters it against a real server, and a
/// test target is another crate. A copy of the text there would assert over the copy.
pub const INSERT: &str = "with existing as (\
     select id from nvs_jobs where dedupe_key = $1::text and state = 0 limit 1\
 ), inserted as (\
     insert into nvs_jobs \
     (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, dedupe_key, created_at) \
     select $2::text, $3::text, $4::text, $5::smallint, 0, $6::int, $7::bigint, $8::bigint, \
            $1::text, $9::bigint \
     where not exists (select 1 from existing) \
     returning id\
 ) select id from inserted union all select id from existing limit 1";

/// ADR 0084 § 4's claim, as the one statement that finds a job and marks it in the same moment.
///
/// **`for update skip locked` is the whole of the mutual exclusion**, and it is why a fleet needs no
/// protocol of ours: two workers running this against one server cannot come back with the same row,
/// because the second one's lock attempt steps over what the first is holding instead of queueing
/// behind it. § 4 names the other backends' spellings — `readpast`, and SQLite's immediate
/// transaction — and each brings its own text for [`MIGRATION`]'s reason.
///
/// **The `update` is in the same statement as the `select`**, as a CTE, because two statements would
/// be two moments: the lock the first took is released by its own commit before the second could
/// arrive, and the row it found would be free in between. That is [`INSERT`]'s reasoning applied to
/// the read side.
///
/// **Two arms, and the second is § 4's visibility timeout.** A pending row is claimable once its
/// `run_at` has passed; a claimed one is claimable again when nothing has finished it within
/// `[queue] visibility` of the claim. `$3` is that cutoff — the instant `visibility` before now,
/// computed by the caller — rather than a bound written into this text, because [`MIGRATION`]'s
/// `claimed_at` records when the claim was *taken* precisely so that
/// [ADR 0078](../../../docs/adr/0078-config-reload-and-control-socket.md) § 1's reload can move the
/// bound under jobs that are already claimed.
///
/// **`attempts` is incremented by the claim and not by the failure that follows it.** § 6's bound
/// has to hold for the worker that dies reporting nothing at all, and an attempt counted only when a
/// job reports its own failure retries forever on exactly the failure mode the timeout above exists
/// for. It is also what makes [`COUNTS`]'s third counter answer during an attempt rather than after
/// it.
///
/// **Keyed on one queue**, as every other statement here is and as [`MIGRATION`]'s `jobs.due` index
/// is built for: `(queue, state, run_at)` is read leftmost-first, so a claim naming no queue would
/// scan what this one seeks. Which queues one worker asks about is [`QUEUES`]'s question, asked one
/// statement earlier and against the same two arms.
///
/// The `returning` list is what running a job needs and nothing else: `queue` is `$1` and the row's
/// other columns are the migration's business.
///
/// `pub` because the worker that claims with it lives in `nvs-cli` — the crate that owns the
/// scheduler a worker is a task on — and § 2's schema has one home, which is here beside the
/// `insert` that writes the columns this reads back.
pub const CLAIM: &str = "with due as (\
     select id from nvs_jobs \
     where queue = $1::text \
     and ((state = 0 and run_at <= $2::bigint) or (state = 1 and claimed_at <= $3::bigint)) \
     order by run_at, id limit 1 \
     for update skip locked\
 ) update nvs_jobs set state = 1, attempts = attempts + 1, claimed_at = $2::bigint \
   where id in (select id from due) \
   returning id, script, args, attempts, max_attempts, backoff_ms";

/// ADR 0084 § 2's unanswered question — *which* queues a worker asks about — answered by the table
/// rather than by a key.
///
/// **§ 2's block names no roster and this module does not invent one.** `connection`, `workers`,
/// `max_attempts` and `visibility` are the whole of what an operator writes, so a worker's queues
/// cannot come from configuration without adding a fifth key that ADR would then have to mean. The
/// honest reading of a block that says nothing is *every queue*, and a queue exists exactly when a
/// row names it: `push` writes the name as a column value and no queue is declared anywhere else,
/// which is what makes the table the only place the roster could be read from.
///
/// **The two arms are [`CLAIM`]'s, so the roster is due work and not every name the table has ever
/// held.** A queue whose rows are all finished, cancelled or claimed-and-still-within-visibility
/// answers nothing here, so a worker spends no claim on it. `$1` and `$2` are that statement's `$2`
/// and `$3` — now, and the instant `[queue] visibility` before it.
///
/// **What it costs, and the gap it leaves.** `distinct` over `(queue, state, run_at)` is a scan
/// PostgreSQL will not turn into a skip-scan, so this is O(due rows) per idle turn rather than
/// O(queues). That is the right trade while the alternative is a configuration key: a deployment
/// whose due backlog is large enough for it to matter is one that wants a roster written down, and
/// the roster is where this should move when § 2 grows one.
pub const QUEUES: &str = "select distinct queue from nvs_jobs \
    where (state = 0 and run_at <= $1::bigint) or (state = 1 and claimed_at <= $2::bigint)";

/// ADR 0084 § 6's write-back for an attempt that returned, and [`CLAIM`]'s other half.
///
/// **Keyed on the lease and not only on the id.** `claimed_at` is the instant the worker's own
/// claim wrote, so a write-back whose row has since been handed to another worker by § 4's
/// visibility timeout matches nothing and changes nothing — which is the only reading of
/// at-least-once that does not let a slow worker's late acknowledgement cancel the attempt that
/// replaced it. A statement matching no row is therefore an ordinary outcome here rather than an
/// error, and the affected count is what says which happened.
///
/// `claimed_at` is cleared with the state for the same reason [`CLAIM`] sets it: it means *this
/// claim*, and a finished job holds none.
///
/// The `2` is `Core\Queue\State::Succeeded`'s ordinal, a literal for [`PENDING`]'s reason and held
/// to the enum by `queue_statements_agree_with_the_state_enum`.
///
/// `pub` for [`CLAIM`]'s reason: the worker that writes it lives in `nvs-cli`, and § 2's schema has
/// one home.
pub const SUCCEEDED: &str = "update nvs_jobs set state = 2, claimed_at = null \
    where id = $1::bigint and claimed_at = $2::bigint";

/// § 6's other write-back: the attempt did not return, and the job is armed for the next one.
///
/// Back to `Pending` — the `0` is that ordinal — with `run_at` pushed out to what [`retry_at`]
/// computed, which is why the delay is a bound parameter rather than arithmetic in the statement:
/// § 6's ladder is exponential *and jittered*, and neither the previous rungs nor the jitter is
/// something SQL should be deciding on a row it is already updating.
///
/// Keyed on the lease exactly as [`SUCCEEDED`] is, and for the same reason.
pub const RETRY: &str = "update nvs_jobs set state = 0, run_at = $3::bigint, claimed_at = null \
    where id = $1::bigint and claimed_at = $2::bigint";

/// § 6's third write-back and the floor under the other two: the attempt was the job's last, so the
/// row leaves [`JOBS_TABLE`] for [`DEAD_TABLE`] instead of being armed again.
///
/// **One statement, because the move is one moment.** The `delete` is a data-modifying CTE whose
/// `returning` list is what the `insert` selects from, so there is no instant in which the job is in
/// both tables or in neither — which is exactly the claim [`STATUS`]'s doc makes about its two arms,
/// held here rather than by a transaction a worker would otherwise have to open around two
/// statements and keep right on every path out of them.
///
/// **Keyed on the lease exactly as [`SUCCEEDED`] and [`RETRY`] are**, and for the same reason: a
/// worker that overran § 4's visibility window matches no row here, so it cannot dead-letter a job
/// the claim that replaced it is still running.
///
/// The columns are listed rather than `select *`-ed because the two tables are deliberately not one
/// shape: the job keeps its `id` and its `queue` ([`DEAD_TABLE`]'s doc says why), leaves `state` and
/// `claimed_at` behind — a row is `Dead` by being here and nothing holds it — and gains `failed_at`
/// and the `errors` array [`dead_errors`] builds.
pub const DEAD_LETTER: &str = "with moved as (\
     delete from nvs_jobs where id = $1::bigint and claimed_at = $2::bigint \
     returning id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
     created_at\
 ) insert into nvs_dead_jobs \
 (id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, dedupe_key, created_at, \
 failed_at, errors) \
 select id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
 created_at, $3::bigint, $4::text from moved";

/// § 6's `errors` array, as [`DEAD_LETTER`] binds it: one entry, the attempt that exhausted the job.
///
/// [`MIGRATION`]'s own doc owns *why* the array is this deep and not deeper, and it is the one home
/// for that trade. What is decided here is the entry's shape: `at` is when the attempt started,
/// which is the lease the move is keyed on, so the row says how long the last attempt ran for
/// against `failed_at` beside it, and `class` and `message` are what the isolate answered with —
/// data rather than an exception object, per
/// [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md).
///
/// Built through `serde_json` rather than formatted, because a thrown message is arbitrary text and
/// a hand-rolled array is one unescaped quote away from a column no reader can parse.
pub fn dead_errors(at: i64, class: &str, message: &str) -> String {
    let mut entry = serde_json::Map::new();
    entry.insert("at".to_string(), at.into());
    entry.insert("class".to_string(), class.into());
    entry.insert("message".to_string(), message.into());
    serde_json::Value::Array(vec![serde_json::Value::Object(entry)]).to_string()
}

/// The ceiling ADR 0084 § 6 asks for and names no number for.
///
/// Five minutes, and the two directions it is chosen between: a cap long enough to be worth having
/// spares a queue nothing once the outage it is waiting out is over, and a cap short enough to
/// retry promptly costs a persistently failing job one attempt every cap rather than a doubling
/// sequence that reaches days. Five minutes is the longest delay an operator watching a recovered
/// dependency would still call prompt, and the rung a one-second base reaches on its ninth attempt
/// — past `[queue] max_attempts`'s own default, so an ordinary job never meets it at all.
const RETRY_CAP_MS: i64 = 300_000;

/// When a job whose attempt failed becomes due again: § 6's exponential backoff, jittered and
/// capped, over the base `push` recorded on the row.
///
/// **Exponential in the attempts already made**, so the base is the *first* retry's delay and each
/// one after it doubles until [`RETRY_CAP_MS`]. `attempts` is the column [`CLAIM`] returns, which
/// the claim itself has already incremented, so the first failed attempt arrives here as `1` and
/// waits one base.
///
/// **Jittered by the job's own id rather than by a clock or a random source.** § 6 asks for jitter
/// because the failure that matters is the shared one — a hundred jobs against an endpoint that
/// went down retry at the instant it comes back, and land on it together. Spreading them needs
/// their delays to *differ*, not to be unpredictable, and the id is the one thing they do not
/// share; deriving the spread from it keeps this a pure function, which is what lets
/// `a_retry_is_exponential_jittered_and_capped` assert the ladder rather than sample it. The result
/// lies in the top half of the rung — `[full/2, full]`, the "equal jitter" shape — so the ladder
/// still grows with every attempt instead of a late rung landing before an early one.
pub fn retry_at(now: i64, attempts: i64, backoff_ms: i64, id: i64) -> i64 {
    // Clamped before the shift rather than after: 30 rungs is already past the cap for any base a
    // `push` could write, and a shift by 64 is undefined rather than saturating.
    let rungs = u32::try_from(attempts.saturating_sub(1))
        .unwrap_or(0)
        .min(30);
    let full = backoff_ms
        .max(0)
        .saturating_mul(1_i64 << rungs)
        .min(RETRY_CAP_MS);
    let half = full / 2;
    let span = u64::try_from(half).unwrap_or(0).saturating_add(1);
    let spread = i64::try_from(jitter(id, attempts) % span).unwrap_or(0);
    now.saturating_add(half).saturating_add(spread)
}

/// The spread [`retry_at`] takes off one job's id, as SplitMix64's finalizer.
///
/// A mixing function and not a hash of anything: what it owes is that two adjacent ids land far
/// apart, which the shift-multiply-shift sequence gives and `id % span` — the obvious spelling —
/// does not, since a queue's ids are consecutive and would then retry in the order they were
/// pushed. The attempt is mixed in with it so that two jobs colliding on one rung do not collide on
/// the next.
fn jitter(id: i64, attempts: i64) -> u64 {
    let mut z = id
        .unsigned_abs()
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(attempts.unsigned_abs());
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

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

/// ADR 0084 §§ 1 and 6's `stats`, as one aggregate over one queue.
///
/// **Named for what it reads rather than for the member**, because [`STATS`] is the class that
/// member answers with and two constants cannot both be `STATS`.
///
/// **One row and not four**, which is the whole reason `stats` answers a record instead of
/// answering a number four times: an aggregate with no `group by` is exactly one row however empty
/// the table is, so the four counters describe one instant rather than four of them with a worker's
/// claim free to land in between. That is [`STATUS`]'s reading of § 2 applied to a whole queue.
///
/// **The `0` and the `1` are [`STATE`]'s `Pending` and `Claimed` ordinals**, literals for
/// [`PENDING`]'s reason — no `const` reaches inside a SQL string — and held to the enum by
/// `queue_statements_agree_with_the_state_enum`. The dead-letter depth is a scalar subquery rather
/// than a fifth arm of the aggregate because it counts rows of the *other* table; § 6 moves an
/// exhausted job there, and [`DEAD_TABLE`]'s doc owns why only `id` and `queue` are readable on it,
/// which is also why `attempts` sums [`JOBS_TABLE`] alone.
///
/// Every column is cast to `bigint` so the four decode the same way whatever widths `nvs queue
/// migrate` gives their columns, and `filter` is PostgreSQL's spelling — gap 5 is why that costs
/// nothing yet, since a second driver needs its own text for [`INSERT`]'s `returning` regardless.
const COUNTS: &str = "select \
    (count(*) filter (where state = 0))::bigint, \
    (count(*) filter (where state = 1))::bigint, \
    (coalesce(sum(attempts), 0))::bigint, \
    (select count(*) from nvs_dead_jobs where queue = $1::text)::bigint \
    from nvs_jobs where queue = $1::text";

/// A [`ID`]'s first slot: the primary key the insert returned.
const ID_SLOT: &str = "id";

/// Its second: the queue the job is in, so a `cancel` or a `status` written against this value knows
/// which queue to look in without asking the row again.
const ID_QUEUE_SLOT: &str = "queue";

/// [`ID_SLOT`]'s index, which is what [`crate::instance::slot`] takes.
const ID_AT: usize = 0;

/// [`ID_QUEUE_SLOT`]'s.
const ID_QUEUE_AT: usize = 1;

/// A [`STATS`]'s first slot: how many of the queue's jobs are waiting for a worker.
///
/// The four slot names are the four member names, which is not decoration:
/// `every_stats_counter_reads_the_slot_its_member_is_named_for` asserts it, because a swapped pair
/// still type-checks, still runs, and answers the wrong number.
const STATS_PENDING_SLOT: &str = "pending";

/// Its second: how many a worker currently holds.
const STATS_CLAIMED_SLOT: &str = "claimed";

/// Its third: how many attempts the jobs still in [`JOBS_TABLE`] have used between them.
const STATS_ATTEMPTS_SLOT: &str = "attempts";

/// Its fourth: how many of the queue's jobs are in [`DEAD_TABLE`].
const STATS_DEAD_SLOT: &str = "deadLettered";

/// [`STATS_PENDING_SLOT`]'s index, and [`COUNTS`]'s first column.
const STATS_PENDING_AT: usize = 0;

/// [`STATS_CLAIMED_SLOT`]'s.
const STATS_CLAIMED_AT: usize = 1;

/// [`STATS_ATTEMPTS_SLOT`]'s.
const STATS_ATTEMPTS_AT: usize = 2;

/// [`STATS_DEAD_SLOT`]'s.
const STATS_DEAD_AT: usize = 3;

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

/// ADR 0084 § 1's `Core\Queue` — all four of `push`, `status`, `cancel` and `stats`.
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
        // The one member of the four asked about a *queue* rather than about a job,
        // because what § 6 wants watched is a population and not a row. It answers a
        // class for the reason [`STATS`] gives, which is the same rule [`ID`] rests
        // on read the other way round.
        CoreMethod {
            name: "stats",
            names: &["queue"],
            // Neutral for the reason `push`'s own `queue` option is: a queue name is
            // compared against a column and read by nothing else.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(STATS_NAME),
            symbol: "nvs_core_queue_stats",
            doc: Some(&STATS_DOC),
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
                   worker. Left out, one second — the row records a delay either way, since there \
                   is no spelling of a job that retries at once.",
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

/// `Core\Queue::stats`'s reference card — ADR 0117.
const STATS_DOC: MethodDoc = MethodDoc {
    short: "Counts one named queue: what is waiting, what a worker holds, how many attempts the \
            queue's jobs have used, and how deep its dead-letter table is. The four are read \
            together, so they describe one instant rather than four.",
    params: &[ParamDoc {
        name: "queue",
        desc: "The queue to count, as `push`'s own `queue` option names one. Queues are separate \
               populations by design, so there is no spelling that totals them.",
        shape: &[],
    }],
    ret: "A `Core\\Queue\\Stats`, whose four counters are members — `$stats->pending()` and not \
          `$stats->pending`, because a `Core`-owned instance has no property a program can reach.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This deployment writes no `[queue]` block, so nothing says which database the \
                   jobs would be in; or the queue's connection names a driver that cannot yet run \
                   a statement.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The queue's connection did not open, or the query was refused by the server — \
                   most often because `nvs queue migrate` has not created the tables.",
        },
    ],
};

/// `Core\Queue\Stats::pending`'s reference card — ADR 0117.
const STATS_PENDING_DOC: MethodDoc = MethodDoc {
    short: "How many of the queue's jobs are waiting for a worker — including those whose `runAt` \
            is still in the future and those between attempts with a backoff still to elapse, \
            because `Core\\Queue\\State::Pending` is one state and not three.",
    params: &[],
    ret: "A `uint`, and `0` both for a queue nothing was ever pushed to and for one that has \
          drained.",
    errors: &[],
};

/// `Core\Queue\Stats::claimed`'s reference card — ADR 0117.
const STATS_CLAIMED_DOC: MethodDoc = MethodDoc {
    short: "How many of the queue's jobs a worker currently holds. Work in flight rather than work \
            committed to: a worker that dies returns its job to `Pending` when the visibility \
            timeout expires.",
    params: &[],
    ret: "A `uint`, read against the fleet's configured concurrency — a queue sitting at that \
          ceiling is saturated rather than stuck.",
    errors: &[],
};

/// `Core\Queue\Stats::attempts`'s reference card — ADR 0117.
const STATS_ATTEMPTS_DOC: MethodDoc = MethodDoc {
    short: "How many attempts the queue's jobs have used between them. Climbing while `pending` \
            does not is what a queue whose jobs keep failing and being retried looks like.",
    params: &[],
    ret: "A `uint`, summed over the jobs table alone: a job that exhausted its attempts has moved \
          to the dead-letter table, and `deadLettered` is what counts it there.",
    errors: &[],
};

/// `Core\Queue\Stats::deadLettered`'s reference card — ADR 0117.
const STATS_DEAD_LETTERED_DOC: MethodDoc = MethodDoc {
    short: "How many of the queue's jobs exhausted their attempts and are in the dead-letter \
            table. The counter worth alerting on: an unwatched dead-letter table is the classic \
            way a queue silently loses work.",
    params: &[],
    ret: "A `uint` that only rises, since nothing the runtime does ever removes a dead-lettered \
          job — emptying that table is an operator's act.",
    errors: &[],
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

/// § 1's `stats`, as the record it answers with — ADR 0084 §§ 1 and 6.
///
/// **The counters are members rather than a shape's fields**, which is where this departs from
/// § 1's originally unannotated `::stats(string $queue)` and has to: a `Core`-owned instance has no
/// property a program can reach ([`CoreTy::Instance`] is the home of that rule), so `$stats->pending`
/// would resolve a class, find no member, and reach `nvs-ir` with nothing to call. The other answer
/// — a shape returned by value — needs a spelling this registry has not got, which is gap 1's
/// blocker and not a thing worth waiting for. `Core\Db\Write` is the same shape for the same
/// reason, and ADR 0084 § 1 now carries the annotation so there is one home for it.
///
/// **Four counters, because § 6 names four things to watch**: what is waiting, what is held, how
/// much has been attempted, and how deep the dead-letter table is. [`COUNTS`] is the one home for
/// which four and for why a fifth is a schema change first.
///
/// The four slots are filled once, by [`nvs_core_queue_stats`], out of a single row — which is the
/// whole reason a member answers a record instead of answering a number four times.
pub(crate) const STATS: CoreClass = CoreClass {
    name: STATS_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "pending",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_queue_stats_pending",
            doc: Some(&STATS_PENDING_DOC),
        },
        CoreMethod {
            name: "claimed",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_queue_stats_claimed",
            doc: Some(&STATS_CLAIMED_DOC),
        },
        CoreMethod {
            name: "attempts",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_queue_stats_attempts",
            doc: Some(&STATS_ATTEMPTS_DOC),
        },
        CoreMethod {
            name: "deadLettered",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_queue_stats_dead_lettered",
            doc: Some(&STATS_DEAD_LETTERED_DOC),
        },
    ],
    slots: &[
        STATS_PENDING_SLOT,
        STATS_CLAIMED_SLOT,
        STATS_ATTEMPTS_SLOT,
        STATS_DEAD_SLOT,
    ],
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
#[must_use]
pub fn now_millis() -> i64 {
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

/// The base delay a `push` that wrote no `{backoff: …}` agreed to, in milliseconds.
///
/// [ADR 0084](../../../docs/adr/0084-durable-background-jobs.md) § 6 asks for exponential backoff
/// with jitter and a cap and names no number, and § 2's `[queue]` block has no key for one — the
/// base is a property of the *job*, which is why § 1 puts it on `push`'s options shape beside
/// `maxAttempts` and not in the deployment's block. So the default lives here, and it is not
/// nothing: [`MIGRATION`]'s `backoff_ms` is `not null`, so there is no row that means "retry at
/// once", and a job whose first attempt failed against a database or an endpoint would otherwise
/// make its second one at the same instant. One second is long enough for that not to be a second
/// failure of the same outage and short enough to be invisible on a queue that is merely busy.
const DEFAULT_BACKOFF_MS: i64 = 1_000;

/// `{backoff: …}` as milliseconds, or [`DEFAULT_BACKOFF_MS`] for the call that left it out.
///
/// # Errors
///
/// A thrown `LogicError` for a negative duration — a backoff that runs the next attempt before the
/// one that failed.
fn backoff_of(args: &[Value]) -> Result<i64, Fault> {
    if matches!(args[BACKOFF_ARG].tag(), Some(Tag::Null)) {
        return Ok(DEFAULT_BACKOFF_MS);
    }
    let nanos = crate::time::nanos_of(args, BACKOFF_ARG, "push")?;
    if nanos < 0 {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{PUSH}: `backoff` cannot be negative, and this one is {nanos}ns"),
        ));
    }
    Ok(nanos / 1_000_000)
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

/// The other half of [`payload_of`]: the `args` column of a claimed row, as the value the job's
/// isolate is handed.
///
/// **One reference is handed over**, which is exactly what [`nvs_host::Isolate::new`] consumes, so
/// the caller passes this straight into the isolate and owes no release on the ordinary path.
///
/// `None` for a document that does not parse. That cannot be a row this module wrote — the encoder
/// above produces one document per row and the column is written by nothing else — so it is a row
/// some other writer put in the table, and a job whose payload is not the payload it was enqueued
/// with is not run with a guess at what was meant. The depth bound is
/// [`crate::json::DEFAULT_MAX_DEPTH`], the same one the encoder refused past.
///
/// `pub` because the worker that runs a claimed job lives in `nvs-cli` — the crate that owns the
/// scheduler a worker is a task on — while § 3's payload encoding is this module's, and a column's
/// two halves belong beside each other.
#[must_use]
pub fn payload(text: &str) -> Option<Value> {
    let max = u32::try_from(crate::json::DEFAULT_MAX_DEPTH).unwrap_or(u32::MAX);
    crate::json::read(text, max).ok()
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
            Some(backoff.to_string().into_bytes()),
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

nvs_runtime::nvs_helper! {
    /// `Core\Queue::stats(string $queue): Queue\Stats` — ADR 0084 §§ 1 and 6.
    ///
    /// **Asked about a queue and not about a job**, which is what makes it the odd member of § 1's
    /// four: the other three take the receipt [`ID`] is, because they are about one row, and this
    /// one is about a population an operator watches.
    ///
    /// **One statement, so the four counters are one fact.** [`COUNTS`] owns why an aggregate with
    /// no `group by` is the shape, and why the dead-letter depth is a scalar subquery beside it
    /// rather than a second query: four queries would be four instants, and a caller comparing
    /// `pending` against `claimed` across them would be comparing two different queues.
    ///
    /// **What it spends:** one statement, on the connection the request either already held or now
    /// holds for the rest of it, plus one four-slot record. [`nvs_core_queue_push`]'s reading of
    /// § 3 applies in the other direction — a `stats` inside a transaction on that connection
    /// counts that transaction's own enqueues, because it is the same connection and not a second.
    fn nvs_core_queue_stats(ctx, args: [1]) {
        // Unreachable from source: the row types this parameter `string`, so a non-text argument is
        // refused at `E0401` first — [`nvs_core_queue_push`]'s guard states the same judgement.
        let queue = args[0]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{STATS_OF}: expected a `string` queue, got tag {}",
                    args[0].tag_byte()
                ))
            })?
            .to_owned();
        let block = configured_queue(ctx, STATS_OF)?.connection;
        // Shared, for `push`'s reason: counting on a second connection would be counting from
        // outside whatever transaction the request has open on the first, so a program that
        // enqueues and then asks would be told its own job does not exist.
        let handle = crate::db::open_named(ctx, &block, true, None, STATS_OF)?;
        let sending: [Option<Vec<u8>>; 1] = [Some(queue.clone().into_bytes())];
        let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
        let refused_by_server = |refused: &dyn std::fmt::Display| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{STATS_OF}: counting the `{queue}` queue across `{JOBS_TABLE}` and \
                     `{DEAD_TABLE}` on `[db.{block}]` was refused: {refused} — `nvs queue migrate` \
                     is what creates those tables"
                ),
            )
        };
        let postgres = postgres_of(ctx, handle, &block, STATS_OF)?;
        let mut answered = postgres
            .query(COUNTS, &bound)
            .map_err(|refused| refused_by_server(&refused))?;
        // Taken before the first row, as `status` takes them: a `PgRows` lends its columns and its
        // rows out of one borrow, and the rows are read with it held mutably.
        let columns: Vec<nvs_db::PgColumn> = answered.columns().to_vec();
        // Every row is drained before the answer is judged, exactly as `push` and `status` drain:
        // the connection owes the caller a message boundary before the next statement on it, which
        // may be the caller's own inside the same transaction. An aggregate with no `group by` is
        // one row, so the guard below is about the shape of the loop and not about a second row.
        let mut read: Option<[i64; 4]> = None;
        loop {
            let Some(row) = answered
                .next_row()
                .map_err(|refused| refused_by_server(&refused))?
            else {
                break;
            };
            if read.is_some() {
                continue;
            }
            let mut counted = [0i64; 4];
            for (at, held) in counted.iter_mut().enumerate() {
                let body = row
                    .column(at)
                    .map_err(|refused| refused_by_server(&refused))?;
                let scalar = columns[at]
                    .scalar(body)
                    .map_err(|refused| refused_by_server(&refused))?;
                let nvs_db::PgScalar::Int(count) = scalar else {
                    return Err(Fault::fatal(format!(
                        "{STATS_OF}: the `{}` counter came back as something other than an \
                         integer, and every one of `COUNTS`'s four columns is cast to `bigint` \
                         here",
                        STATS.slots[at]
                    )));
                };
                *held = count;
            }
            read = Some(counted);
        }
        let counted = read.ok_or_else(|| {
            Fault::fatal(format!(
                "{STATS_OF}: the aggregate over `{JOBS_TABLE}` answered no row at all, and one \
                 with no `group by` answers exactly one however empty the table is"
            ))
        })?;
        // Saturating at zero rather than refusing: a negative count is not something the server
        // can produce from a `count` or from a sum of non-negative attempts, so the alternative is
        // a refusal nothing can reach.
        Ok(crate::instance::build(
            &STATS,
            counted.map(|one| Value::uint(u64::try_from(one).unwrap_or(0))),
        ))
    }
}

/// One of [`STATS`]'s four counters, read out of the slot [`nvs_core_queue_stats`] filled.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not a `Core\Queue\Stats`, or for a slot holding
/// anything but a `uint`: every slot is written by [`nvs_core_queue_stats`] and by nothing else, so
/// either is a paste error in this crate rather than anything a program can produce —
/// `Core\Db\Write`'s `write_count` states the same reading of the identical pair.
fn counter(args: &[Value], member: &str, at: usize) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &STATS, member)?;
    let held = crate::instance::slot(receiver, at);
    if held.as_uint().is_none() {
        return Err(Fault::fatal(format!(
            "{STATS_NAME}::{member} found tag {} in its `{}` slot",
            held.tag_byte(),
            STATS.slots[at]
        )));
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `$stats->pending(): uint` — how many of the queue's jobs are waiting, as
    /// [`STATS_PENDING_DOC`] states the reading of `Pending` a program gets here.
    fn nvs_core_queue_stats_pending(_ctx, args: [1]) {
        counter(args, "pending", STATS_PENDING_AT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$stats->claimed(): uint` — how many a worker holds right now.
    fn nvs_core_queue_stats_claimed(_ctx, args: [1]) {
        counter(args, "claimed", STATS_CLAIMED_AT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$stats->attempts(): uint` — the attempts the queue's own rows have used, which
    /// [`COUNTS`] sums over [`JOBS_TABLE`] alone for [`DEAD_TABLE`]'s reason.
    fn nvs_core_queue_stats_attempts(_ctx, args: [1]) {
        counter(args, "attempts", STATS_ATTEMPTS_AT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$stats->deadLettered(): uint` — § 6's dead-letter depth, the counter that section names
    /// outright as the one an unwatched deployment loses work behind.
    fn nvs_core_queue_stats_dead_lettered(_ctx, args: [1]) {
        counter(args, "deadLettered", STATS_DEAD_AT)
    }
}

/// The address of one of *this* module's symbols, or `None` for a symbol that belongs to another
/// domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_queue_push" => (nvs_core_queue_push as *const ()).cast(),
        "nvs_core_queue_status" => (nvs_core_queue_status as *const ()).cast(),
        "nvs_core_queue_cancel" => (nvs_core_queue_cancel as *const ()).cast(),
        "nvs_core_queue_stats" => (nvs_core_queue_stats as *const ()).cast(),
        "nvs_core_queue_stats_pending" => (nvs_core_queue_stats_pending as *const ()).cast(),
        "nvs_core_queue_stats_claimed" => (nvs_core_queue_stats_claimed as *const ()).cast(),
        "nvs_core_queue_stats_attempts" => (nvs_core_queue_stats_attempts as *const ()).cast(),
        "nvs_core_queue_stats_dead_lettered" => {
            (nvs_core_queue_stats_dead_lettered as *const ()).cast()
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CANCEL, CLAIM, COUNTS, DEAD_LETTER, DEAD_TABLE, INSERT, JOBS_TABLE, MIGRATION, PENDING,
        QUEUES, RETRY, RETRY_CAP_MS, STATE, STATS, STATS_ATTEMPTS_AT, STATS_ATTEMPTS_SLOT,
        STATS_CLAIMED_AT, STATS_CLAIMED_SLOT, STATS_DEAD_AT, STATS_DEAD_SLOT, STATS_PENDING_AT,
        STATS_PENDING_SLOT, STATUS, SUCCEEDED, dead_errors, retry_at,
    };

    /// The statement with that label, or the test fails naming it: every assertion below is about
    /// one of § 2's two tables, and a label typed differently in the DDL than in the command's
    /// output contract is exactly the drift this file is holding.
    fn labelled(label: &str) -> &'static str {
        MIGRATION
            .iter()
            .find(|one| one.label == label)
            .unwrap_or_else(|| panic!("`MIGRATION` carries no `{label}` statement"))
            .sql
    }

    /// The `errors` column is text a reader has to parse, and what a job threw is arbitrary text —
    /// so the one thing this owes is that a message able to end the array early does not. Asserted
    /// by parsing the answer back rather than by comparing it to a spelling, since the escaping is
    /// `serde_json`'s business and only the shape is this module's.
    #[test]
    fn a_dead_letter_row_carries_the_exhausting_attempts_error() {
        let written = dead_errors(
            1_700_000_000_123,
            "RuntimeError",
            "the \"endpoint\" \\ refused",
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&written).expect("`dead_errors` writes a JSON document");
        let entries = parsed.as_array().expect("§ 6's `errors` is an array");
        assert_eq!(
            entries.len(),
            1,
            "one entry, which is `MIGRATION`'s doc's decision and the depth `nvs_jobs` pays for"
        );
        assert_eq!(entries[0]["at"], 1_700_000_000_123_i64);
        assert_eq!(entries[0]["class"], "RuntimeError");
        assert_eq!(
            entries[0]["message"], "the \"endpoint\" \\ refused",
            "the message crosses the column unchanged, quotes and all"
        );
    }

    /// [`MIGRATION`] is the only place the queue's columns exist and the statements above are their
    /// only readers — two lists in one file, with nothing but this test between them. A column
    /// renamed in the DDL and nowhere else still compiles, still migrates, and fails on the first
    /// `push` against a database an operator has already built.
    #[test]
    fn the_ddl_creates_every_column_the_statements_name() {
        let jobs = labelled("jobs");
        let dead = labelled("dead_letter");
        assert!(
            jobs.contains(JOBS_TABLE) && dead.contains(DEAD_TABLE),
            "each `create table` builds the table its own constant names"
        );

        // `INSERT`'s parenthesised column list is the widest claim any statement makes about
        // `nvs_jobs`: every other one reads a subset of it.
        let list = INSERT
            .split_once(&format!("insert into {JOBS_TABLE} ("))
            .and_then(|(_, rest)| rest.split_once(')'))
            .expect("`INSERT` names its columns as one parenthesised list")
            .0;
        for column in list.split(',').map(str::trim) {
            assert!(
                jobs.contains(&format!("{column} ")),
                "the `jobs` DDL creates `{column}`, which `INSERT` binds"
            );
        }
        for column in ["state ", "attempts ", "queue "] {
            assert!(
                jobs.contains(column),
                "the `jobs` DDL creates the column `STATUS` and `COUNTS` read as `{column}`"
            );
        }
        // `CLAIM` is the one statement that reads a column no `INSERT` writes: a claim's own
        // instant, which is where § 4's visibility timeout is measured from.
        assert!(
            jobs.contains("claimed_at "),
            "the `jobs` DDL creates `claimed_at`, which `CLAIM` writes and reads back"
        );
        assert!(
            CLAIM.contains("for update skip locked"),
            "§ 4's mutual exclusion is the database's, and this is the spelling that asks for it"
        );
        assert!(
            labelled("jobs.due").contains("(queue, state, run_at)"),
            "`CLAIM` seeks by queue, then state, then due-ness, which is the order of this index"
        );
        // The roster is read off the same table and the same column `CLAIM` is then keyed on, which
        // is the whole of why § 2 needs no fifth key to name a worker's queues.
        assert!(
            QUEUES.contains(JOBS_TABLE) && QUEUES.contains("distinct queue"),
            "`QUEUES` reads the roster off the column `INSERT` writes the queue name into"
        );
        for column in ["id ", "queue "] {
            assert!(
                dead.contains(column),
                "the `dead_letter` DDL creates `{column}`, which is what `DEAD_TABLE`'s doc says \
                 this module reads of it"
            );
        }
        // § 6's move is the only statement that writes `DEAD_TABLE`, so its column list is the
        // widest claim anything makes about that table — the reading `INSERT`'s list gets above,
        // against the other DDL.
        let moved = DEAD_LETTER
            .split_once(&format!("insert into {DEAD_TABLE} ("))
            .and_then(|(_, rest)| rest.split_once(')'))
            .expect("`DEAD_LETTER` names its columns as one parenthesised list")
            .0;
        for column in moved.split(',').map(str::trim) {
            assert!(
                dead.contains(&format!("{column} ")),
                "the `dead_letter` DDL creates `{column}`, which `DEAD_LETTER` writes"
            );
        }
        assert!(
            DEAD_LETTER.contains(&format!("delete from {JOBS_TABLE} "))
                && DEAD_LETTER.contains("claimed_at = $2::bigint"),
            "the move takes the row out of the jobs table keyed on the lease, as `SUCCEEDED` is"
        );

        assert!(
            labelled("jobs.dedupe").contains(&format!("where state = {PENDING}")),
            "gap 3's index covers pending rows by `PENDING`'s own ordinal, as `INSERT` does"
        );
        for step in MIGRATION {
            let table = step
                .label
                .split_once('.')
                .map_or(step.label, |(head, _)| head);
            assert!(
                matches!(table, "jobs" | "dead_letter"),
                "`{}` is labelled under one of § 2's two tables, which is what `nvs queue \
                 migrate`'s output promises",
                step.label
            );
        }
    }

    /// [`retry_at`] is ADR 0084 § 6's ladder and this is what makes it one: the delay doubles, it
    /// stops at [`RETRY_CAP_MS`], and two jobs on the same rung are not due at the same instant.
    ///
    /// Asserted as bounds over the whole ladder rather than as numbers, because the jitter has no
    /// number to assert — what it owes is a *spread inside its rung*, which is exactly what the
    /// containment check below says and what a fixed expectation could not.
    #[test]
    fn a_retry_is_exponential_jittered_and_capped() {
        // The three properties § 6 names, each asserted over the whole ladder rather than on one
        // rung: a delay read off a single call would pass for a function that had lost the shift.
        for attempt in 1..=12_i64 {
            let full = 1_000_i64
                .saturating_mul(1 << (attempt - 1))
                .min(RETRY_CAP_MS);
            for id in 1..=64 {
                let due = retry_at(0, attempt, 1_000, id);
                assert!(
                    (full / 2..=full).contains(&due),
                    "attempt {attempt} of job {id} is due at {due}, outside its rung's own half"
                );
            }
        }

        // A rung never lands before the one under it, whichever way the two were jittered, which is
        // what the top-half spread buys over a `[0, full]` one. Only up to the cap: past it the two
        // rungs are the same rung, and the jitter is then free to order them either way — which is
        // the cap doing its job rather than the ladder failing.
        for attempt in 1..=8_i64 {
            assert!(
                retry_at(0, attempt + 1, 1_000, 7) >= retry_at(0, attempt, 1_000, 11),
                "attempt {attempt}'s jitter reached past the rung above it"
            );
        }

        // The cap, from far past it: an attempt count no `max_attempts` would allow still answers a
        // delay rather than an overflow, and a base that would overflow the shift by itself does
        // too.
        assert!((RETRY_CAP_MS / 2..=RETRY_CAP_MS).contains(&retry_at(0, 40, 1_000, 3)));
        assert!((RETRY_CAP_MS / 2..=RETRY_CAP_MS).contains(&retry_at(0, 3, i64::MAX, 3)));

        // And the jitter is a spread and not a constant: the whole point is that jobs failing
        // together do not come back together.
        let first = retry_at(0, 4, 1_000, 1);
        assert!(
            (2..=200).any(|id| retry_at(0, 4, 1_000, id) != first),
            "every job on one rung is due at the same instant, so nothing was jittered"
        );
    }

    /// The statements in this module write and read [`STATE`]'s ordinals as SQL literals, which no
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
        assert_eq!(
            case("Succeeded"),
            2,
            "`SUCCEEDED` writes this ordinal for an attempt that returned"
        );
        assert!(
            SUCCEEDED.contains("set state = 2"),
            "the write-back for a job that ran spells the ordinal above"
        );
        assert!(
            RETRY.contains("set state = 0"),
            "a retried job goes back to `Pending`'s own ordinal, which is what makes it claimable"
        );
        assert!(
            COUNTS.contains("filter (where state = 0)"),
            "`COUNTS` counts waiting jobs by `Pending`'s own ordinal"
        );
        assert_eq!(
            case("Claimed"),
            1,
            "`COUNTS`'s second counter spells this `1`"
        );
        assert!(
            CLAIM.contains("set state = 1") && CLAIM.contains("(state = 1 and claimed_at"),
            "`CLAIM` writes the ordinal above and is what the visibility timeout takes back"
        );
        assert!(
            CLAIM.contains("state = 0 and run_at"),
            "`CLAIM`'s first arm takes pending rows by `Pending`'s own ordinal"
        );
        assert!(
            QUEUES.contains("state = 0 and run_at") && QUEUES.contains("state = 1 and claimed_at"),
            "`QUEUES` asks `CLAIM`'s two arms, so a roster entry is a queue with due work in it"
        );
        assert!(
            COUNTS.contains("filter (where state = 1)"),
            "`COUNTS` counts jobs a worker holds by the ordinal above"
        );
        assert!(
            COUNTS.contains(JOBS_TABLE) && COUNTS.contains(DEAD_TABLE),
            "`COUNTS` reads both of § 2's tables, taking the depth from the second"
        );
    }

    /// Three separate places say what order [`STATS`]'s counters are in — [`COUNTS`]'s select
    /// list, the slot roster, and the `*_AT` index each reader passes — and only the first is
    /// beyond a test's reach. Nothing else would notice the other two disagreeing: a swapped pair
    /// still type-checks, still runs, and answers the wrong number.
    #[test]
    fn every_stats_counter_reads_the_slot_its_member_is_named_for() {
        let members: Vec<&str> = STATS.instance.iter().map(|one| one.name).collect();
        assert_eq!(
            members, STATS.slots,
            "each counter is named for the slot it reads, in `COUNTS`'s column order"
        );
        for (at, slot) in [
            (STATS_PENDING_AT, STATS_PENDING_SLOT),
            (STATS_CLAIMED_AT, STATS_CLAIMED_SLOT),
            (STATS_ATTEMPTS_AT, STATS_ATTEMPTS_SLOT),
            (STATS_DEAD_AT, STATS_DEAD_SLOT),
        ] {
            assert_eq!(
                STATS.slots[at], slot,
                "the index `{slot}`'s reader passes is the slot of that name"
            );
        }
    }
}
