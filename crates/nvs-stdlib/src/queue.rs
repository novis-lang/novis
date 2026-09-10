//! `Core\Queue` — `rule:concurrency/enqueue-commits-with-your-write`'s durable background
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
//! is no name for a capability to be about, so `rule:concurrency/queue-four-members` states none and this module invents one.
//!
//! **The schema is this module's, and `nvs queue migrate` reads it.** § 2 makes the runtime own one
//! jobs table and one dead-letter table, created by an explicit operator command — DDL is an
//! injection sink and never issued from a request — so `push` writes into a table it does not create.
//! [`schema`] is that command's whole schema — one `nvs_db::Schema` value, written in no dialect at
//! all, and the one home for what those tables' columns are; `nvs-cli`'s `queue` module converges a
//! database onto it and decides nothing about it. Two choices in it are worth their
//! sentence: every instant is a `bigint` of epoch milliseconds rather than a timestamp, because § 2
//! supports all five of `rule:core-classes/db-one-api`'s backends and five timestamp dialects is exactly the cost a
//! runtime-owned table should not carry; and `state` is the ordinal `Core\Queue\State` already is at
//! runtime (`rule:enums/closed-integer-type`), so the enum and the column
//! are one representation and not two.
//!
//! **Every text this module sends has been parsed by a real server of both framed drivers.**
//! [`runs`] is the roster of drivers a statement exists for, and it is narrower than [`crate::db`]
//! § *Every driver reaches every member, and all five are pooled* on purpose: each of the three it
//! names reaches § 2's schema, § 4's claim and § 6's move as [`Split`]s, § 5's three readers as
//! ordinary second spellings, and [`queue_connection`] as the seam that borrows the connection as
//! whichever dialect it speaks. What says those texts are ones a *server* accepts rather than ones
//! this module agrees with itself about is `crates/nvs-stdlib/tests/queue.rs`: § 2's schema, § 4's
//! claim with its `skip locked`, § 6's two write-backs and its move, and § 5's readers all run
//! against a real MySQL and a real MariaDB there, beside the PostgreSQL cases they were written
//! from — down to the two constructs nothing else in the roster spells, an `update` whose whole
//! answer is the affected count and `count(case when … then 1 end)` beside the `cast(… as signed)`
//! over the `sum` MySQL answers as a `decimal`.
//!
//! **What it spends:** one statement per member call, on a connection the request either already
//! held or now holds for the rest of it, plus one JSON encoding of `$args` sized by the payload the
//! caller wrote. Nothing is held between calls, except the four counters `stats` answers with for
//! as long as its caller keeps the record.
//!
//! # Known gaps
//!
//! 1. **`limits` and `grants` are not declared**, because § 1 makes each of them an *option* whose
//!    value is itself a `{…}`, and a shape is only ever a whole parameter: a [`CoreOption`]'s type is
//!    never a bag and a [`crate::registry::CoreField`]'s is never a [`CoreTy::Shape`], so the one
//!    trailing bag an optioned member has leaves nowhere to write either of them. `Core\Db::open`
//!    does not wait on this any more — its settings literal *is* a whole parameter and spells a
//!    shape — so what is left to decide is whether an option may carry one at all, and where the
//!    narrowing `rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` asks for is read
//!    off the context that enqueued the job.
//!    — owner: unowned-sweep
//! 2. **`$args` is `mixed` and so does not refuse a `secret`**, which § 1 asks for. A durable row is
//!    an output and `rule:security/secret-qualifier`'s five sinks are the shape of the eventual answer; `CoreTy::Mixed`
//!    carries no qualifier, so saying it needs a spelling the registry has not got.
//!    — owner: unowned-sweep
//! 3. **`key`'s "at most one pending job per key" is enforced by the statement, and by the unique
//!    key only where the schema has been applied.** [`INSERT_POSTGRES`]'s `existing` arm reads the table
//!    inside the same statement that writes it, which is correct against every other `push` on a
//!    *serialized* transaction and racy against a concurrent one at `read committed`. [`schema`]
//!    carries the constraint under the name `nvs_jobs_dedupe`, a plain unique key over the
//!    `dedupe_pending` column every statement here maintains, and the statement is race-free
//!    against a schema carrying it without changing shape — so what is left of this gap is a
//!    deployment that never ran `nvs queue migrate`, which is the one case the key is absent in.
//!    — owner: unowned
//! 4. **`stats` counts the four things § 6 names and no fifth**, and a fifth would be a column in
//!    § 2's schema before it is a member here. The sharp edge is a dead-lettered job's own
//!    attempts: § 6 *moves* that row to [`DEAD_TABLE`], whose columns this module deliberately does
//!    not decide beyond `id` and `queue`, so [`COUNTS_POSTGRES`] sums `attempts` over [`JOBS_TABLE`] alone
//!    and counts the depth separately rather than inventing a column for the sum to reach.
//!    — owner: unowned
//! 5. **Two of the five backends have no statement here at all**, and the text is this module's to
//!    write rather than [`crate::db`]'s: `Core\Db` reaches all five, so each of the two opens a
//!    connection that works and has nothing of § 4's or § 6's to send over it. SQLite is one
//!    dialect away. SQL Server is a dialect *and* the vocabulary behind it, because
//!    `rule:core-classes/queue-storage-is-a-table` orders the filtered index its nulls need before a
//!    fourth dialect is written — [`no_dialect`] carries both sentences, and is where an operator
//!    reads which of the two they are waiting on.
//!    — owner: gap-zero

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
/// **Two of its columns are all this module reads, and [`schema`] is where every one of them is
/// written down.** A job keeps the `id` and the `queue` it had in [`JOBS_TABLE`], so a
/// `Core\Queue\Id` handed out before the job exhausted its attempts still names it afterwards, and
/// that pair is the whole of what [`STATUS_POSTGRES`] and [`COUNTS_POSTGRES`] ask of the table. What else the row
/// carries — § 6's payload, every attempt's error and its timing — is decided by [`schema`] and not
/// by the worker that will write one: a column has to exist before anything can move a row into it,
/// so the schema is the earlier of the two decisions and the only one there is room for.
const DEAD_TABLE: &str = "nvs_dead_jobs";

/// `Core\Queue\State::Pending`'s ordinal, which is what a freshly pushed row's `state` is.
///
/// Written as the number rather than read off [`STATE`] because the SQL beside it cannot read the
/// enum either — a statement's `state = 0` is a literal in a string — so one spelling of the rule
/// covering both is worth more than two half-rules. `queue_statements_agree_with_the_state_enum` is
/// that spelling: it holds this constant and the ordinals inside [`INSERT_POSTGRES`] and [`STATUS_POSTGRES`] to
/// [`STATE`]'s own cases.
const PENDING: i16 = 0;

/// One statement of [`schema`]'s DDL, under the name an operator sees it by.
///
/// A **label** rather than a table name, because a dialect may build a table's indexes as statements
/// of their own, and the question an operator reading `nvs queue migrate` has is which of § 2's
/// *two* tables a statement belongs to. So the label is that table's role in the ADR — `jobs` or
/// `dead_letter` — and what those tables are actually called stays [`JOBS_TABLE`]'s and
/// [`DEAD_TABLE`]'s business for the reason those two constants give.
///
/// **The statement is owned rather than borrowed**, because there is one schema value and four
/// dialects: no dialect's text exists until a driver asks [`migration`] for it, and a `&'static str`
/// would be exactly the transcription of the value that the value exists to make unnecessary.
#[derive(Debug)]
pub struct Migration {
    /// Which of § 2's two tables this statement builds.
    pub label: &'static str,
    /// The statement, carrying no separator: a driver is handed one statement at a time, and the
    /// `;` belongs to whatever is printing them for a human instead.
    pub sql: String,
}

/// [`schema`]'s two tables as the statements that build them in `driver`'s dialect, in order.
///
/// **Total, where a list per dialect could not be.** Every driver has an answer because the schema
/// is one value and `nvs_db::ddl` emits it in all four dialects, so the question an operator's
/// backend faces is no longer whether anybody wrote its list. The statements are emitted on demand
/// rather than held as constants for the same reason: a constant per dialect is a transcription of
/// the value, and a transcription is the thing that drifts.
///
/// **This is the offline half of `nvs queue migrate`**, which prints the schema for a database it
/// does not open — the create-from-nothing reading of a value that says what the tables *should
/// be*. The applying half converges instead: `nvs_db::ddl` writes no `if not exists`, and
/// `rule:core-classes/schema-converges`'s plan is what makes running the command twice safe, by
/// being computed against the database as it is rather than by a construct that pretends a second
/// run is the first.
///
/// Every statement the emitter writes names one of § 2's two tables, which is what makes the label
/// total; `every_statement_of_the_schema_names_one_of_the_two_tables` is that assertion.
#[must_use]
pub fn migration(driver: nvs_db::Driver) -> Vec<Migration> {
    nvs_db::ddl::create_schema(&schema(), nvs_db::Dialect::of(driver))
        .into_iter()
        .map(|sql| Migration {
            label: if sql.contains(DEAD_TABLE) {
                "dead_letter"
            } else {
                "jobs"
            },
            sql,
        })
        .collect()
}

/// How wide every indexed text column of § 2's schema is.
///
/// One number rather than one per column, because the two that carry it hold the *same value*:
/// [`schema`]'s `dedupe_pending` is `dedupe_key` while the job is pending, so a narrower one of the
/// pair would refuse a key the other admitted. 255 is what fits InnoDB's 3,072-byte key limit at
/// `utf8mb4`'s four bytes a character with room for the rest of the `nvs_jobs_due` key, and it is
/// the bound the whole vocabulary is held to rather than MySQL's alone: a schema value is one
/// spelling for four dialects, so the tightest backend's limit is the value's limit.
const KEY_WIDTH: u32 = 255;

/// `rule:core-classes/queue-storage-is-a-table`'s two tables as one [`nvs_db::Schema`] value, which
/// is the one home for what the queue's tables are.
///
/// **One value and four dialects.** The columns are decided here and the spelling is
/// `nvs_db::ddl`'s, so every backend `rule:core-classes/db-one-api` names has this schema rather
/// than the two that someone wrote a `create table` for — and a third dialect is the emitter's
/// business rather than a third transcription of one column list. [`migration`] is that emission,
/// and `nvs schema plan` converges a live database onto the same value.
///
/// **`dedupe_pending` is the construct this value had to decide.** The vocabulary holds neither a
/// partial unique index (`… where state = 0`) nor a stored generated column, and both are out of v1
/// for the reason `rule:core-classes/schema-plan` gives — no portable spelling. What is portable is
/// the column those two constructs each *derive*: a plain `dedupe_pending` that the statements
/// maintain, holding `dedupe_key` while the job is pending and `null` once it is not, with a plain
/// unique key over it. That is `where state = 0` said on the other side, and a null collides with
/// nothing on the four backends whose unique keys read nulls as distinct — so the guarantee gap 3
/// names is the same one on every backend `Core\Queue` can run a statement against.
/// **SQL Server reads two nulls as equal** and so admits one released row rather than any number of
/// them; it has no queue statements at all ([`no_dialect`]), and the day it gains them the answer is
/// the filtered index `rule:core-classes/schema-plan` keeps out of v1, which is what the vocabulary
/// would have to grow first.
///
/// **A nullable column rather than a `not null` one with a sentinel**, which is what SQL Server
/// would otherwise want: a `not null` unique column needs a distinct value per released row, so
/// every push and every state transition would carry a generated token — and, decisively, such a
/// column cannot be *added* to a table that already has rows at all, while a nullable one converges
/// onto a live queue as a `Safe` step. A schema that no existing deployment can reach is not a
/// schema.
///
/// Three things in it are decisions rather than transcription:
///
/// - **Every instant is a `bigint` of epoch milliseconds** and never a timestamp — this module's own
///   doc owns why, and it is the one-value argument above applied to the type map.
/// - **`claimed_at` is when the claim was taken, not when it expires.** § 4's visibility timeout is
///   `[queue] visibility` measured from it, so the bound stays in configuration where
///   `rule:config/the-config-is-an-immutable-snapshot`'s reload can move
///   it; a stored deadline would freeze the superseded bound onto every job already claimed.
/// - **The dead-letter row is the job's own columns plus `failed_at` and `errors`**, where `errors`
///   is the JSON array § 6 asks for — an entry carrying when an attempt ran and what it threw.
///   **It is one entry deep, and that entry is the attempt that exhausted the job**, because the
///   jobs table has nowhere to keep what an earlier attempt threw: [`RETRY_POSTGRES`] arms a row
///   for the next attempt and keeps the count and nothing else. Recording all of them would be a
///   text column on `nvs_jobs` appended to on every failure — a row rewritten once per attempt,
///   carrying a value only the exhausted job ever reads, on the table § 4's claim contends over —
///   so the array is § 6's shape at the depth this schema pays for, and [`dead_errors`] is where
///   that trade is written down. There is no `state`: a row is `Dead` by being in that table, which
///   is exactly what [`STATUS_POSTGRES`]'s second arm asserts by answering the ordinal as a literal.
///
/// **What it spends:** one indexed [`KEY_WIDTH`]-wide column per job row, which is what a partial
/// index costs nothing for — priority 5 spent to buy one spelling everywhere the queue runs
/// instead of two spellings on two backends.
///
/// Every identifier below is a literal this module wrote, so a refusal from the builders is a bug
/// in this function rather than bad input, and the `expect` says which.
#[must_use]
pub fn schema() -> nvs_db::Schema {
    use nvs_db::schema::{Column, IntWidth, ScalarType, Table};

    fn named(name: &str, ty: ScalarType) -> Column {
        Column::new(name, ty).expect("the queue's own column names are bare identifiers")
    }
    let big = || ScalarType::Int(IntWidth::Big);
    let int = || ScalarType::Int(IntWidth::Normal);
    // Indexed text is bounded and payload text is not: `queue` and the two dedupe columns are read
    // by a key, while `script`, `args` and `errors` are a path and two JSON documents that no index
    // ever covers.
    let short = || ScalarType::Text {
        max: Some(KEY_WIDTH),
    };
    let long = || ScalarType::Text { max: None };

    let jobs = Table::new(
        JOBS_TABLE,
        vec![
            named("id", big())
                .identity()
                .expect("`id` is an integer, which is what an identity column may be"),
            named("queue", short()),
            named("script", long()),
            named("args", long()).null(),
            named("state", ScalarType::Int(IntWidth::Small)),
            named("attempts", int()),
            named("max_attempts", int()),
            named("backoff_ms", big()),
            named("run_at", big()),
            named("dedupe_key", short()).null(),
            named("dedupe_pending", short()).null(),
            named("created_at", big()),
            named("claimed_at", big()).null(),
        ],
    )
    .and_then(|table| table.primary_key(&["id"]))
    .and_then(|table| table.unique("nvs_jobs_dedupe", &["dedupe_pending"]))
    .and_then(|table| table.index("nvs_jobs_due", &["queue", "state", "run_at"]))
    .expect("the jobs table names its own columns in its own keys");

    let dead = Table::new(
        DEAD_TABLE,
        vec![
            named("id", big()),
            named("queue", short()),
            named("script", long()),
            named("args", long()).null(),
            named("attempts", int()),
            named("max_attempts", int()),
            named("backoff_ms", big()),
            named("run_at", big()),
            named("dedupe_key", short()).null(),
            named("created_at", big()),
            named("failed_at", big()),
            named("errors", long()),
        ],
    )
    .and_then(|table| table.primary_key(&["id"]))
    .and_then(|table| table.index("nvs_dead_jobs_queue", &["queue"]))
    .expect("the dead-letter table names its own columns in its own keys");

    nvs_db::Schema::new(vec![jobs, dead]).expect("two tables, named apart")
}

/// `rule:concurrency/queue-four-members`'s `push`, as one statement.
///
/// **One statement rather than a check and an insert**, because two would be two moments and § 3's
/// property is about there being one. The `existing` arm is `key`'s dedupe, read off the
/// `dedupe_pending` column this same statement writes, and it costs nothing at all when `$1` is
/// `null`: `dedupe_pending = null` is never true, so the arm is empty and the `not exists` guard
/// admits the insert. That is why there is no second spelling of this statement for the commoner
/// call that passes no key — a branch here would be a second SQL text for § 1's statement cache to
/// hold and a second thing to keep right.
///
/// **`$1` is written into two columns and bound once.** `dedupe_key` is what the job was pushed
/// under and stays on the row for as long as the row does; `dedupe_pending` is
/// `rule:core-classes/queue-storage-is-a-table`'s guarantee, the column [`schema`] puts a plain
/// unique key over, and it holds that key only while the job is pending — [`CLAIM_POSTGRES`],
/// [`SUCCEEDED_POSTGRES`] and [`CANCEL_POSTGRES`] clear it and [`RETRY_POSTGRES`] restores it from
/// `dedupe_key`. A `$n` may be named as often as a statement likes, so the pair costs this dialect
/// no parameter at all; [`INSERT_MYSQL`]'s tenth bound slot is what it costs the other one.
///
/// The trailing `union all` is what makes the answer one row in both cases: a deduped push answers
/// with the pending job's own id, which is what a caller that wanted "at most one" asked for.
/// `pub` for the reason [`CLAIM_POSTGRES`] is, with a different caller: `crates/nvs-stdlib/tests/queue.rs`
/// pushes a job with this statement and then claims and dead-letters it against a real server, and a
/// test target is another crate. A copy of the text there would assert over the copy.
///
/// **PostgreSQL's dialect, and [`INSERT_MYSQL`] is the same push where a `returning` cannot be
/// had** — that constant's doc owns where the two part, and [`Split`]'s owns what a backend that
/// cannot answer in one statement does instead.
pub const INSERT_POSTGRES: &str = "with existing as (\
     select id from nvs_jobs where dedupe_pending = $1::text limit 1\
 ), inserted as (\
     insert into nvs_jobs \
     (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
      dedupe_pending, created_at) \
     select $2::text, $3::text, $4::text, $5::smallint, 0, $6::int, $7::bigint, $8::bigint, \
            $1::text, $1::text, $9::bigint \
     where not exists (select 1 from existing) \
     returning id\
 ) select id from inserted union all select id from existing limit 1";

/// `rule:concurrency/claiming-is-one-statement`'s claim, as the one statement that finds a job and marks it in the same moment.
///
/// **`for update skip locked` is the whole of the mutual exclusion**, and it is why a fleet needs no
/// protocol of ours: two workers running this against one server cannot come back with the same row,
/// because the second one's lock attempt steps over what the first is holding instead of queueing
/// behind it. § 4 names the other backends' spellings — `readpast`, and SQLite's immediate
/// transaction — and each brings its own text, for the reason [`Split`]'s doc gives.
///
/// **The `update` is in the same statement as the `select`**, as a CTE, because two statements would
/// be two moments: the lock the first took is released by its own commit before the second could
/// arrive, and the row it found would be free in between. That is [`INSERT_POSTGRES`]'s reasoning applied to
/// the read side.
///
/// **Two arms, and the second is § 4's visibility timeout.** A pending row is claimable once its
/// `run_at` has passed; a claimed one is claimable again when nothing has finished it within
/// `[queue] visibility` of the claim. `$3` is that cutoff — the instant `visibility` before now,
/// computed by the caller — rather than a bound written into this text, because [`schema`]'s
/// `claimed_at` records when the claim was *taken* precisely so that
/// `rule:config/the-config-is-an-immutable-snapshot`'s reload can move the
/// bound under jobs that are already claimed.
///
/// **`attempts` is incremented by the claim and not by the failure that follows it.** § 6's bound
/// has to hold for the worker that dies reporting nothing at all, and an attempt counted only when a
/// job reports its own failure retries forever on exactly the failure mode the timeout above exists
/// for. It is also what makes [`COUNTS_POSTGRES`]'s third counter answer during an attempt rather than after
/// it.
///
/// **Keyed on one queue**, as every other statement here is and as [`schema`]'s `nvs_jobs_due` index
/// is built for: `(queue, state, run_at)` is read leftmost-first, so a claim naming no queue would
/// scan what this one seeks. Which queues one worker asks about is [`QUEUES_POSTGRES`]'s question,
/// asked one statement earlier and against the same two arms.
///
/// **The claim clears `dedupe_pending`**, which is the moment the job stops being pending in the
/// sense [`schema`]'s unique key means it: a second push of the same key is admitted from here on,
/// and [`RETRY_POSTGRES`] is what puts the key back when an attempt did not return.
///
/// The `returning` list is what running a job needs and nothing else: `queue` is `$1` and the row's
/// other columns are the schema's business.
///
/// `pub` because the worker that claims with it lives in `nvs-cli` — the crate that owns the
/// scheduler a worker is a task on — and § 2's schema has one home, which is here beside the
/// `insert` that writes the columns this reads back.
///
/// **PostgreSQL's dialect, and [`CLAIM_MYSQL`] is § 4's claim where a data-modifying CTE cannot be
/// had** — the same two arms, the same lock, and the same six columns in the same order, taken by
/// two statements inside one transaction rather than by one.
pub const CLAIM_POSTGRES: &str = "with due as (\
     select id from nvs_jobs \
     where queue = $1::text \
     and ((state = 0 and run_at <= $2::bigint) or (state = 1 and claimed_at <= $3::bigint)) \
     order by run_at, id limit 1 \
     for update skip locked\
 ) update nvs_jobs set state = 1, attempts = attempts + 1, claimed_at = $2::bigint, \
   dedupe_pending = null \
   where id in (select id from due) \
   returning id, script, args, attempts, max_attempts, backoff_ms";

/// `rule:core-classes/queue-storage-is-a-table`'s unanswered question — *which* queues a worker asks about — answered by the table
/// rather than by a key.
///
/// **§ 2's block names no roster and this module does not invent one.** `connection`, `workers`,
/// `max_attempts` and `visibility` are the whole of what an operator writes, so a worker's queues
/// cannot come from configuration without adding a fifth key that ADR would then have to mean. The
/// honest reading of a block that says nothing is *every queue*, and a queue exists exactly when a
/// row names it: `push` writes the name as a column value and no queue is declared anywhere else,
/// which is what makes the table the only place the roster could be read from.
///
/// **The two arms are [`CLAIM_POSTGRES`]'s, so the roster is due work and not every name the table has ever
/// held.** A queue whose rows are all finished, cancelled or claimed-and-still-within-visibility
/// answers nothing here, so a worker spends no claim on it. `$1` and `$2` are that statement's `$2`
/// and `$3` — now, and the instant `[queue] visibility` before it.
///
/// **What it costs, and the gap it leaves.** `distinct` over `(queue, state, run_at)` is a scan
/// PostgreSQL will not turn into a skip-scan, so this is O(due rows) per idle turn rather than
/// O(queues). That is the right trade while the alternative is a configuration key: a deployment
/// whose due backlog is large enough for it to matter is one that wants a roster written down, and
/// the roster is where this should move when § 2 grows one.
///
/// **PostgreSQL's dialect, and [`QUEUES_MYSQL`] is the same question asked in the other one** — one
/// whole text each, per [`Split`]'s doc, since nothing here rests on a construct only PostgreSQL
/// has.
pub const QUEUES_POSTGRES: &str = "select distinct queue from nvs_jobs \
    where (state = 0 and run_at <= $1::bigint) or (state = 1 and claimed_at <= $2::bigint)";

/// `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s write-back for an attempt that returned, and [`CLAIM_POSTGRES`]'s other half.
///
/// **Keyed on the lease and not only on the id.** `claimed_at` is the instant the worker's own
/// claim wrote, so a write-back whose row has since been handed to another worker by § 4's
/// visibility timeout matches nothing and changes nothing — which is the only reading of
/// at-least-once that does not let a slow worker's late acknowledgement cancel the attempt that
/// replaced it. A statement matching no row is therefore an ordinary outcome here rather than an
/// error, and the affected count is what says which happened.
///
/// `claimed_at` is cleared with the state for the same reason [`CLAIM_POSTGRES`] sets it: it means *this
/// claim*, and a finished job holds none. `dedupe_pending` is cleared beside it, and it costs
/// nothing to clear a column the claim already cleared: this row is being rewritten either way, and
/// a statement that maintains [`schema`]'s column about its own row is one that holds the guarantee
/// without resting on which statement ran before it.
///
/// The `2` is `Core\Queue\State::Succeeded`'s ordinal, a literal for [`PENDING`]'s reason and held
/// to the enum by `queue_statements_agree_with_the_state_enum`, in both dialects.
///
/// `pub` for [`CLAIM_POSTGRES`]'s reason: the worker that writes it lives in `nvs-cli`, and § 2's
/// schema has one home.
///
/// **PostgreSQL's dialect, and [`SUCCEEDED_MYSQL`] is the other one**, binding the same two values
/// in the same order.
pub const SUCCEEDED_POSTGRES: &str = "update nvs_jobs set state = 2, claimed_at = null, \
    dedupe_pending = null \
    where id = $1::bigint and claimed_at = $2::bigint";

/// § 6's other write-back: the attempt did not return, and the job is armed for the next one.
///
/// Back to `Pending` — the `0` is that ordinal — with `run_at` pushed out to what [`retry_at`]
/// computed, which is why the delay is a bound parameter rather than arithmetic in the statement:
/// § 6's ladder is exponential *and jittered*, and neither the previous rungs nor the jitter is
/// something SQL should be deciding on a row it is already updating.
///
/// Keyed on the lease exactly as [`SUCCEEDED_POSTGRES`] is, and for the same reason.
///
/// **`dedupe_pending` comes back from `dedupe_key`**, because the row is pending again and
/// [`schema`]'s column says exactly that. It is the one statement that restores it, and it is where
/// the guarantee could be refused rather than silently lost: a key pushed again while this job was
/// claimed already holds the unique key, and this update is then rejected as an ordinary
/// [`insert_refused`] — which is the same answer the partial index gave when the row went back to
/// `state = 0` underneath it.
///
/// **PostgreSQL's dialect, and [`RETRY_MYSQL`] is the other one** — the same three values, in an
/// order that dialect's placeholders force rather than choose, which that constant's doc owns.
pub const RETRY_POSTGRES: &str = "update nvs_jobs set state = 0, run_at = $3::bigint, \
    claimed_at = null, dedupe_pending = dedupe_key \
    where id = $1::bigint and claimed_at = $2::bigint";

/// § 6's third write-back and the floor under the other two: the attempt was the job's last, so the
/// row leaves [`JOBS_TABLE`] for [`DEAD_TABLE`] instead of being armed again.
///
/// **One statement, because the move is one moment.** The `delete` is a data-modifying CTE whose
/// `returning` list is what the `insert` selects from, so there is no instant in which the job is in
/// both tables or in neither — which is exactly the claim [`STATUS_POSTGRES`]'s doc makes about its two arms,
/// held here rather than by a transaction a worker would otherwise have to open around two
/// statements and keep right on every path out of them.
///
/// **Keyed on the lease exactly as [`SUCCEEDED_POSTGRES`] and [`RETRY_POSTGRES`] are**, and for the
/// same reason: a worker that overran § 4's visibility window matches no row here, so it cannot
/// dead-letter a job the claim that replaced it is still running.
///
/// The columns are listed rather than `select *`-ed because the two tables are deliberately not one
/// shape: the job keeps its `id` and its `queue` ([`DEAD_TABLE`]'s doc says why), leaves `state` and
/// `claimed_at` behind — a row is `Dead` by being here and nothing holds it — and gains `failed_at`
/// and the `errors` array [`dead_errors`] builds.
///
/// **PostgreSQL's dialect, and [`DEAD_LETTER_MYSQL`] is § 6's move where a `delete … returning`
/// cannot be had** — the same two tables and the same lease, copied and then removed inside one
/// transaction rather than removed and then copied inside one statement.
pub const DEAD_LETTER_POSTGRES: &str = "with moved as (\
     delete from nvs_jobs where id = $1::bigint and claimed_at = $2::bigint \
     returning id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
     created_at\
 ) insert into nvs_dead_jobs \
 (id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, dedupe_key, created_at, \
 failed_at, errors) \
 select id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
 created_at, $3::bigint, $4::text from moved";

/// A statement one backend spells as two, and the pair a caller runs inside **one transaction**.
///
/// **The pair is one moment or it is nothing**, which is why it is a type rather than two loose
/// constants beside each other. [`CLAIM_POSTGRES`]'s own doc argues the property: a claim's `select`
/// and its `update` cannot be two commits, because the lock the first took is released by its own
/// commit before the second could arrive and the row it found would be free in between. PostgreSQL
/// reaches that with a data-modifying CTE and pays one round trip; a backend that has no such
/// construct reaches the *same* property with an explicit transaction around these two and pays
/// four. So the difference between the dialects is latency and nothing else — priority 3 spent
/// where the construct is not available, rather than a weaker guarantee sold as a dialect.
///
/// A caller that ran [`Self::first`] and committed without [`Self::then`] has published half a
/// claim, half a push or half a move. The transaction is therefore not the caller's convenience: it
/// is what these two texts *mean*, and it is the one thing a second backend cannot omit.
#[derive(Debug)]
pub struct Split {
    /// The statement run first: the one that reads, and whose row the second one acts on.
    ///
    /// It takes the row's locks — `for update`, and `skip locked` where § 4's mutual exclusion
    /// rests on it — because a lock is what carries the pair's meaning across the gap between two
    /// round trips that a single statement did not have.
    pub first: &'static str,
    /// The statement run second, in the same transaction, keyed by what [`Self::first`] answered.
    pub then: &'static str,
}

/// [`INSERT_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged for
/// [`schema`]'s reason: MariaDB does answer `insert … returning`, and a second text spelling one
/// statement two ways is the drift the one-home rule exists to prevent.
///
/// **The dedupe read is keyed on `dedupe_pending`, not on `dedupe_key`**, which is what both
/// dialects do: [`schema`]'s column holds the key while the job is pending and `null` once it is
/// not, so `dedupe_pending = ?` *is* `dedupe_key = ? and state = 0` said in the form a plain unique
/// index can answer. Written the other way this `select … for update` would scan `nvs_jobs` and
/// lock every row it passed — a table lock in all but name, on the one statement every enqueue
/// runs.
///
/// **The insert names `dedupe_pending` as a tenth bound slot**, where [`INSERT_POSTGRES`] simply
/// names `$1` twice: a `?` is bound by the position it occupies in the text and cannot be named
/// again, so the key is encoded once and bound twice, which is what the doubled `dedupe` in the
/// caller's array is. Ten slots for nine values is the whole of what this dialect pays for the
/// column the other one gets for nothing.
///
/// **The insert answers no id, and the caller reads one off the write's own OK packet**
/// ([`nvs_db::MySqlRows::last_id`]) rather than from a second query. That value is the connection's
/// own state and not the server's, so nothing another session did can be read there, and a
/// `select last_insert_id()` would be a third round trip for a number the second one already
/// carried.
///
/// **A deduped push answers the pending job's id from [`Split::first`], and [`Split::then`] does
/// not run at all** — [`INSERT_POSTGRES`]'s trailing `union all` moved out of the SQL and into
/// the caller, not a second meaning. Gap 3 reads exactly as it does there: two concurrent pushes of
/// an unseen key at `read committed` both find nothing, and the second insert is refused by
/// `nvs_jobs_dedupe` rather than admitted by a guard that read the table a moment earlier.
///
/// **A push with no key runs [`Split::then`] alone**, and [`push_in_two`] owns why that is not
/// merely two round trips saved: `dedupe_pending = null` is a range scan here where
/// [`INSERT_POSTGRES`]'s `existing` arm is an empty CTE.
pub const INSERT_MYSQL: Split = Split {
    first: "select id from nvs_jobs where dedupe_pending = ? limit 1 for update",
    then: "insert into nvs_jobs \
           (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
           dedupe_pending, created_at) \
           values (?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?)",
};

/// [`CLAIM_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged.
///
/// **The `select` answers [`CLAIM_POSTGRES`]'s `returning` list, in its order**, so a worker reads
/// the same column at the same ordinal whichever dialect it claimed with — the six slots
/// `crates/nvs-cli/src/worker.rs` names by position, and
/// `both_dialects_answer_a_claim_with_the_same_columns` is what holds them together.
/// `attempts + 1 as attempts` is what makes that true across the split: PostgreSQL's `returning`
/// runs after its own `update` and so reads the incremented value, while here the `update` has not
/// run yet, so the column is read as the value it is about to have. A claim answering the
/// pre-increment count would stop § 6's ladder one rung short of `max_attempts`.
///
/// **`skip locked` is on the `select` and the mutual exclusion is unchanged**: the same row lock,
/// asked for by the same clause, held now by an explicit transaction instead of by a single
/// statement — two workers still cannot come back with one row. MySQL has had it since 8.0 and
/// MariaDB since 10.6, which is the floor [`schema`]'s tables already sit on: `for update skip
/// locked` is a row lock, and InnoDB is the engine that has them.
///
/// The `update` is keyed by `id` rather than by a subquery, because [`Split::first`] has already
/// named the row and holds its lock: PostgreSQL's `where id in (select id from due)` exists to
/// reach its own CTE, and there is no CTE here to reach.
pub const CLAIM_MYSQL: Split = Split {
    first: "select id, script, args, attempts + 1 as attempts, max_attempts, backoff_ms \
            from nvs_jobs \
            where queue = ? \
            and ((state = 0 and run_at <= ?) or (state = 1 and claimed_at <= ?)) \
            order by run_at, id limit 1 \
            for update skip locked",
    then: "update nvs_jobs set state = 1, attempts = attempts + 1, claimed_at = ?, \
           dedupe_pending = null where id = ?",
};

/// [`DEAD_LETTER_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged.
///
/// **The copy runs before the delete, which is the reverse of PostgreSQL's order**, and it is the
/// only order there is once the `delete` cannot answer with what it removed: the columns have to be
/// read while they still exist. § 6's property is unchanged — the row is in both tables or in
/// neither — and here it is the transaction that holds it where one statement held it by
/// construction.
///
/// **Both halves are keyed on the lease**, exactly as [`DEAD_LETTER_POSTGRES`] is and for its
/// reason: a worker that overran § 4's visibility window matches no row in either half, so it can
/// neither copy nor delete a job the claim that replaced it is still running. Keying only the copy
/// would take a row out from under its new owner.
///
/// The `insert … select` reads `nvs_jobs` and writes `nvs_dead_jobs`, which are two tables, so
/// nothing here meets MySQL's refusal to read the table a statement is writing.
pub const DEAD_LETTER_MYSQL: Split = Split {
    first: "insert into nvs_dead_jobs \
            (id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
            created_at, failed_at, errors) \
            select id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, \
            dedupe_key, created_at, ?, ? from nvs_jobs \
            where id = ? and claimed_at = ?",
    then: "delete from nvs_jobs where id = ? and claimed_at = ?",
};

/// [`QUEUES_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged.
///
/// **Not a [`Split`], for [`STATUS_MYSQL`]'s reason**: a `select distinct` over one table with two
/// arms is the same statement in both dialects, so what changes is the placeholder spelling and the
/// casts PostgreSQL needs to type a text-format parameter at all. Its two values are that
/// statement's two, in its order — the instant now, and the instant `[queue] visibility` before it.
///
/// The scan [`QUEUES_POSTGRES`]'s doc costs out is the same scan here, for the same reason: MySQL
/// will not answer `distinct` off the leading column of `jobs.due` without walking the due rows
/// either, and § 2's roster is where both dialects stop paying for it.
pub const QUEUES_MYSQL: &str = "select distinct queue from nvs_jobs \
    where (state = 0 and run_at <= ?) or (state = 1 and claimed_at <= ?)";

/// [`SUCCEEDED_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged.
///
/// **The lease keying survives the transcription intact**, which is the whole of what makes this
/// the same write-back rather than a weaker one: `id` and `claimed_at` are bound here in the order
/// they are `$1` and `$2` there, so a worker that overran § 4's visibility window matches no row in
/// either dialect and cannot cancel the attempt that replaced it.
///
/// A statement matching no row therefore stays an ordinary outcome rather than an error, and the
/// affected count is what says which happened — the same reading [`CANCEL_MYSQL`]'s doc makes of a
/// dialect that has no `returning` to answer with.
pub const SUCCEEDED_MYSQL: &str = "update nvs_jobs set state = 2, claimed_at = null, \
    dedupe_pending = null \
    where id = ? and claimed_at = ?";

/// [`RETRY_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged.
///
/// **Its three values go out in a different order from [`RETRY_POSTGRES`]'s, and that is forced
/// rather than chosen.** A `$n` is named where its value is wanted and may be named anywhere; a `?`
/// is bound by the position it occupies in the text. The `run_at` this writes is in the `set`
/// clause, which is left of the `where`, so this binds `run_at`, `id`, `claimed_at` where
/// PostgreSQL binds `id`, `claimed_at`, `run_at`. Reordering PostgreSQL's `$n`s to match would be
/// editing a landed text to make a new one resemble it, and the caller is where the two orders are
/// reconciled anyway — once, beside the connection it already had to branch on.
///
/// The delay stays a bound value for [`RETRY_POSTGRES`]'s reason: § 6's ladder is exponential and
/// jittered, and neither is something SQL should be deciding on a row it is already updating. The
/// `0` is `Core\Queue\State::Pending`'s ordinal, held to the enum beside its twin by
/// `queue_statements_agree_with_the_state_enum`.
pub const RETRY_MYSQL: &str = "update nvs_jobs set state = 0, run_at = ?, claimed_at = null, \
    dedupe_pending = dedupe_key \
    where id = ? and claimed_at = ?";

/// § 6's `errors` array, as [`DEAD_LETTER_POSTGRES`] binds it: one entry, the attempt that exhausted the job.
///
/// [`schema`]'s own doc owns *why* the array is this deep and not deeper, and it is the one home
/// for that trade. What is decided here is the entry's shape: `at` is when the attempt started,
/// which is the lease the move is keyed on, so the row says how long the last attempt ran for
/// against `failed_at` beside it, and `class` and `message` are what the isolate answered with —
/// data rather than an exception object, per
/// `rule:security/isolate-shares-nothing`.
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

/// The ceiling `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` asks for and names no number for.
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
/// one after it doubles until [`RETRY_CAP_MS`]. `attempts` is the column [`CLAIM_POSTGRES`] returns, which
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

/// `rule:concurrency/queue-four-members` and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s `status`, as one statement over both of § 2's tables.
///
/// **Two tables and not one**, because § 6 *moves* a job that has exhausted its attempts into the
/// dead-letter table rather than deleting it, and a caller asking what became of its job is owed
/// `Dead` rather than a refusal. The `union all` is [`INSERT_POSTGRES`]'s shape for [`INSERT_POSTGRES`]'s reason: two
/// statements would be two moments, and one is what makes the answer a fact about a single instant.
/// A job cannot be in both arms at once, because moving it is one transaction of the worker's.
///
/// The `3` in the second arm is `Core\Queue\State::Dead`'s ordinal, written as a literal because no
/// `const` can reach inside a SQL string; `queue_statements_agree_with_the_state_enum` is what holds
/// it to [`STATE`]. That arm also costs nothing under a design where a dead job stays in
/// [`JOBS_TABLE`] with its state written instead of moving — the first arm is tried first and the
/// `limit 1` takes it — so the statement is correct either way and the worker is free to choose.
///
/// **Public for the reason the statements a worker sends are**, and it is the same reason twice
/// over: nothing outside this module *runs* a `status` — [`crate::queue`]'s member is the only
/// caller — but `crates/nvs-stdlib/tests/queue.rs` sends both spellings to a real server, and a
/// statement no server has ever parsed is exactly what that target exists to catch.
pub const STATUS_POSTGRES: &str = "select state from nvs_jobs \
    where id = $1::bigint and queue = $2::text \
    union all \
    select 3 from nvs_dead_jobs \
    where id = $1::bigint and queue = $2::text \
    limit 1";

/// [`STATUS_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged for [`INSERT_MYSQL`]'s
/// reason.
///
/// **Not a [`Split`], and that is the difference between this member and § 4's three**: nothing
/// here rests on a construct MySQL lacks — a `union all` of two `select`s is the same statement in
/// both dialects — so what changes is the placeholder spelling and the casts PostgreSQL needs to
/// type a text-format parameter at all.
///
/// **Four placeholders where PostgreSQL has two.** `$1` can be named as often as a statement likes
/// and a `?` cannot, so the id and the queue each go out twice: the same two values bound twice,
/// not two more arguments for a caller to get wrong. [`counted_row`] is where that pair is doubled,
/// once, rather than in each of the two members that send it.
pub const STATUS_MYSQL: &str = "select state from nvs_jobs \
    where id = ? and queue = ? \
    union all \
    select 3 from nvs_dead_jobs \
    where id = ? and queue = ? \
    limit 1";

/// `rule:concurrency/queue-four-members`'s `cancel`, as one conditional update.
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
/// `queue_statements_agree_with_the_state_enum` for the reason [`STATUS_POSTGRES`]'s `3` is.
///
/// **`dedupe_pending` is cleared here and nowhere else on this path**, because this is the one
/// transition out of `Pending` that no claim precedes: a cancelled job never reaches
/// [`SUCCEEDED_POSTGRES`], so leaving the column set would hold a key against a job nothing will
/// ever run.
const CANCEL_POSTGRES: &str = "update nvs_jobs set state = 4, dedupe_pending = null \
    where id = $1::bigint and queue = $2::text and state = 0 \
    returning id";

/// [`CANCEL_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged.
///
/// **No `returning`, and the member's answer is the affected count instead** — not a weaker
/// reading of the same question. `set state = 4 where … and state = 0` changes every row it
/// matches, so the count of rows the server says it changed and the count PostgreSQL returns are
/// one number, and [`Counted::touched`] is where the two spellings are read as one fact. MariaDB
/// answers `returning` for an `insert` and a `delete` and not for an `update`, so this is the text
/// both drivers run rather than a MySQL-only concession.
pub const CANCEL_MYSQL: &str = "update nvs_jobs set state = 4, dedupe_pending = null \
    where id = ? and queue = ? and state = 0";

/// `rule:concurrency/queue-four-members` and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s `stats`, as one aggregate over one queue.
///
/// **Named for what it reads rather than for the member**, because [`STATS`] is the class that
/// member answers with and two constants cannot both be `STATS`.
///
/// **One row and not four**, which is the whole reason `stats` answers a record instead of
/// answering a number four times: an aggregate with no `group by` is exactly one row however empty
/// the table is, so the four counters describe one instant rather than four of them with a worker's
/// claim free to land in between. That is [`STATUS_POSTGRES`]'s reading of § 2 applied to a whole queue.
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
/// nothing yet, since a second driver needs its own text for [`INSERT_POSTGRES`]'s `returning` regardless.
const COUNTS_POSTGRES: &str = "select \
    (count(*) filter (where state = 0))::bigint, \
    (count(*) filter (where state = 1))::bigint, \
    (coalesce(sum(attempts), 0))::bigint, \
    (select count(*) from nvs_dead_jobs where queue = $1::text)::bigint \
    from nvs_jobs where queue = $1::text";

/// [`COUNTS_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged.
///
/// **`count(case when … then 1 end)` is the aggregate filter PostgreSQL spells `filter (where …)`**,
/// and it counts rather than sums for the empty queue: `count` ignores the `null` the `case` falls
/// through to and answers `0` over no rows at all, where a `sum` of ones would answer `null` and
/// § 6 means zero. That is the same reading [`COUNTS_POSTGRES`]'s `coalesce` makes of its third
/// counter.
///
/// **The third counter is cast and the first two are not**, which is § 9 rather than an
/// inconsistency: MySQL answers `sum` over an integer column as a `decimal`, so the cast is what
/// keeps all four columns one type for one reader, while `count` is already a `bigint`.
pub const COUNTS_MYSQL: &str = "select \
    count(case when state = 0 then 1 end), \
    count(case when state = 1 then 1 end), \
    cast(coalesce(sum(attempts), 0) as signed), \
    (select count(*) from nvs_dead_jobs where queue = ?) \
    from nvs_jobs where queue = ?";

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

/// [`STATS_PENDING_SLOT`]'s index, and [`COUNTS_POSTGRES`]'s first column.
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

/// `rule:concurrency/queue-four-members`'s `Core\Queue` — all four of `push`, `status`, `cancel` and `stats`.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "push",
            names: &["script"],
            params: &[
                // A **sink**, and for `rule:core-classes/process-is-argv-only`'s reason rather than `rule:security/tainted-qualifier`'s usual one: the argument
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
        // there first, and there is no other way for it to find out. [`CANCEL_POSTGRES`] owns
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

/// `Core\Queue::push`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Queue::status`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Queue::cancel`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Queue::stats`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Queue\Stats::pending`'s reference card — `rule:core-api/reference-card`.
const STATS_PENDING_DOC: MethodDoc = MethodDoc {
    short: "How many of the queue's jobs are waiting for a worker — including those whose `runAt` \
            is still in the future and those between attempts with a backoff still to elapse, \
            because `Core\\Queue\\State::Pending` is one state and not three.",
    params: &[],
    ret: "A `uint`, and `0` both for a queue nothing was ever pushed to and for one that has \
          drained.",
    errors: &[],
};

/// `Core\Queue\Stats::claimed`'s reference card — `rule:core-api/reference-card`.
const STATS_CLAIMED_DOC: MethodDoc = MethodDoc {
    short: "How many of the queue's jobs a worker currently holds. Work in flight rather than work \
            committed to: a worker that dies returns its job to `Pending` when the visibility \
            timeout expires.",
    params: &[],
    ret: "A `uint`, read against the fleet's configured concurrency — a queue sitting at that \
          ceiling is saturated rather than stuck.",
    errors: &[],
};

/// `Core\Queue\Stats::attempts`'s reference card — `rule:core-api/reference-card`.
const STATS_ATTEMPTS_DOC: MethodDoc = MethodDoc {
    short: "How many attempts the queue's jobs have used between them. Climbing while `pending` \
            does not is what a queue whose jobs keep failing and being retried looks like.",
    params: &[],
    ret: "A `uint`, summed over the jobs table alone: a job that exhausted its attempts has moved \
          to the dead-letter table, and `deadLettered` is what counts it there.",
    errors: &[],
};

/// `Core\Queue\Stats::deadLettered`'s reference card — `rule:core-api/reference-card`.
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
/// It has no members of its own; it is a name for a pair, which is what `rule:core-api/shape-rules` gives an opaque
/// handle.
pub(crate) const ID: CoreClass = CoreClass {
    name: ID_NAME,
    methods: &[],
    instance: &[],
    slots: &[ID_SLOT, ID_QUEUE_SLOT],
    constants: &[],
};

/// § 1's `stats`, as the record it answers with — `rule:concurrency/queue-four-members` and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`.
///
/// **The counters are members rather than a shape's fields**, which is where this departs from
/// § 1's originally unannotated `::stats(string $queue)` and has to: a `Core`-owned instance has no
/// property a program can reach ([`CoreTy::Instance`] is the home of that rule), so `$stats->pending`
/// would resolve a class, find no member, and reach `nvs-ir` with nothing to call. The other answer
/// — a shape returned by value — needs a spelling this registry has not got, which is gap 1's
/// blocker and not a thing worth waiting for. `Core\Db\Write` is the same shape for the same
/// reason, and `rule:concurrency/queue-four-members` now carries the annotation so there is one home for it.
///
/// **Four counters, because § 6 names four things to watch**: what is waiting, what is held, how
/// much has been attempted, and how deep the dead-letter table is. [`COUNTS_POSTGRES`] is the one home for
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

/// `rule:concurrency/claiming-is-one-statement` and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s job lifecycle, as § 1's `Core\Queue\State`.
///
/// **§ 1 is the home of the roster and of why there is no `Failed`**; this is the home of what a
/// case *is* here, which is a stored number. Each case is a resting state of a row rather than an
/// event: a job whose `runAt` has not come round is `Pending`, because waiting for a worker and
/// waiting for a clock are one thing to everything that reads this column, and a case separating
/// them would be a distinction no claim statement makes. `Cancelled` is what a cancel *is* on a
/// table nobody polls twice — [`CANCEL_POSTGRES`] writes it in place of `Pending`, and every claim statement
/// reads `Pending`, so a job leaves the queue by changing one column.
///
/// **The values are declaration ordinals and mean nothing else**, as they are for every enum here
/// ([`CoreEnum::cases`]' rule) — but these ones are additionally *stored*: § 2's jobs table writes
/// this ordinal in its `state` column, so the enum and the column are one representation and not
/// two, and each case's number is part of the schema `nvs queue migrate` will create. Renumbering
/// one is therefore a migration and not an edit. `queue_statements_agree_with_the_state_enum` pins
/// them for that reason: [`PENDING`] and the ordinals written inside [`INSERT_POSTGRES`] and [`STATUS_POSTGRES`] are
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

/// [`STATE`]'s reference card — `rule:core-api/reference-card`.
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
/// `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` asks for exponential backoff
/// with jitter and a cap and names no number, and § 2's `[queue]` block has no key for one — the
/// base is a property of the *job*, which is why § 1 puts it on `push`'s options shape beside
/// `maxAttempts` and not in the deployment's block. So the default lives here, and it is not
/// nothing: [`schema`]'s `backoff_ms` is `not null`, so there is no row that means "retry at
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
                 that created it — `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` makes attempts finite, not optional"
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
                 database a job would live in — `rule:core-classes/queue-storage-is-a-table` is the block, and `connection` is the \
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

/// Whether `Core\Queue`'s four members run over a driver at all, which is this module's roster and
/// not `nvs-db`'s.
///
/// **A driver § 2 has a schema for is not yet a driver § 4 has statements for**, and that gap is the
/// whole of what this answers. [`migration`] emits [`schema`] for all five, so the wider question —
/// can a statement be sent at all — is `rule:core-classes/db-drivers-are-an-enum`'s and is answered
/// one crate down; the narrower one is here, because the texts are here.
///
/// One predicate rather than one list per reader: [`queue_connection`] is the seam itself and hands
/// back a borrow, so it cannot be written as a `bool`, but the roster it implements is also what
/// [`no_dialect`]'s last arm is the complement of and what
/// `crates/nvs-stdlib/tests/queue.rs`'s leg gate skips a matrix run on. The match is exhaustive for
/// the same reason [`no_dialect`]'s is: a sixth driver is a build failure rather than a silent
/// `false`.
#[must_use]
pub fn runs(driver: nvs_db::Driver) -> bool {
    match driver {
        nvs_db::Driver::Postgres | nvs_db::Driver::MySql | nvs_db::Driver::MariaDb => true,
        nvs_db::Driver::SqlServer | nvs_db::Driver::Sqlite => false,
    }
}

/// The queue's connection, as the dialect the member's statements are written in.
///
/// **Two arms and not five, because that is how many dialects this module has** — § 2's two
/// migration lists, and § 4's statements once as PostgreSQL's single-statement text and once as a
/// [`Split`]. The second arm is [`crate::db::Framed`] rather than a pair of its own: MySQL and
/// MariaDB are one send path and one dialect here, and that constant's doc owns why a driver
/// difference that is only the type of the borrow is flattened at the call sites. [`runs`] is the
/// same roster as a predicate, for the readers that need to ask without holding a connection.
///
/// # Errors
///
/// A thrown `RuntimeError` by way of [`no_dialect`] for a driver this module has no dialect for at
/// all. A [`Fault::fatal`] for a key the request's own table does not hold, which is this crate's
/// paste error rather than a program's.
fn queue_connection<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
    block: &str,
    member: &str,
) -> Result<Queued<'a>, Fault> {
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
    match connection {
        nvs_db::Connection::Postgres(postgres) => Ok(Queued::Postgres(postgres)),
        nvs_db::Connection::MySql(mysql) => Ok(Queued::Framed(crate::db::Framed::MySql(mysql))),
        nvs_db::Connection::MariaDb(maria) => Ok(Queued::Framed(crate::db::Framed::MariaDb(maria))),
        other => Err(no_dialect(member, block, other.driver())),
    }
}

/// The queue's connection, borrowed as the driver whose dialect the member holds.
///
/// [`crate::db`]'s `filed_connection` one module over, and separate rather than shared because that
/// one narrows to § 7's three commands and this one to § 4's statements — the same downcast asked
/// two different questions.
enum Queued<'a> {
    /// [`INSERT_POSTGRES`], [`CLAIM_POSTGRES`] and [`DEAD_LETTER_POSTGRES`], each answering in one
    /// statement.
    Postgres(&'a mut nvs_db::PgConn),
    /// [`INSERT_MYSQL`], [`CLAIM_MYSQL`] and [`DEAD_LETTER_MYSQL`], each a [`Split`] the caller
    /// runs inside one transaction — and MariaDB runs every one of them unchanged.
    Framed(crate::db::Framed<'a>),
}

/// The refusal a connection this module cannot run its statements over earns.
///
/// **What is missing is the statements, and never § 2's schema.** [`migration`] emits [`schema`] in
/// whichever dialect a driver speaks, so `nvs queue migrate` converges the two tables on all five
/// backends and an operator reading this sentence has already been able to build them. What a
/// driver arriving here lacks is § 4's and § 6's texts: [`INSERT_POSTGRES`] and [`INSERT_MYSQL`] are
/// the two this module holds, and [`queue_connection`] is the seam that reaches them.
///
/// **The two arms name two different gaps**, which is why they are two sentences rather than one.
/// SQLite is one text away — its unique keys read nulls as distinct, so the shape [`schema`]
/// converges to is already the guarantee `rule:core-classes/queue-storage-is-a-table` states. SQL
/// Server is two, and the order is that rule's own: two nulls are equal there, so `dedupe_pending`'s
/// plain unique key admits one released row rather than any number of them, and the vocabulary grows
/// the filtered index `rule:core-classes/schema-plan` keeps out of v1 before a fourth dialect is
/// written against it. An operator told that statements are the only thing left would be waiting on
/// half of what SQL Server needs.
///
/// Every arm is spelled rather than left to a `_`, so a sixth driver arrives as a build failure here
/// instead of as whichever sentence happens to be written last. [`runs`] is the roster the last arm
/// is the complement of, and the test below holds the two together.
fn no_dialect(member: &str, block: &str, driver: nvs_db::Driver) -> Fault {
    let missing = match driver {
        nvs_db::Driver::SqlServer => {
            "and the queue has no statements for it yet — `nvs queue migrate` converges \
             `rule:core-classes/queue-storage-is-a-table`'s tables here, but this backend reads two nulls as equal, so \
             `dedupe_pending`'s unique key needs a filtered index the schema vocabulary does not \
             hold yet and § 4's statements are written after it"
        }
        nvs_db::Driver::Sqlite => {
            "and the queue has no statements for it yet — `nvs queue migrate` converges \
             `rule:core-classes/queue-storage-is-a-table`'s tables here, and § 4's statements are written for PostgreSQL and \
             MySQL only"
        }
        // Unreachable: [`queue_connection`] matches all three of these out before it asks.
        nvs_db::Driver::Postgres | nvs_db::Driver::MySql | nvs_db::Driver::MariaDb => {
            "and it is one of the three the queue does run — this is a bug"
        }
    };
    Fault::thrown(format!(
        "{member}: `[db.{block}]` is a {} connection, {missing}",
        driver.display_name()
    ))
}

nvs_runtime::nvs_helper! {
    /// `Core\Queue::push(string $script, {…}): Queue\Id` — `rule:concurrency/queue-four-members` and `rule:concurrency/enqueue-commits-with-your-write`.
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
        // Encoded once and bound twice, because the two dialects want the same nine values in two
        // orders: [`INSERT_POSTGRES`] names the dedupe key first, since `$1` is read by all three
        // of the places it appears, and [`INSERT_MYSQL`]'s insert names it in column order like
        // every other value — twice, since it writes it to two columns and a `?` cannot repeat.
        // One array per order over one set of buffers, rather than a second encoding of the same
        // integers.
        let dedupe = key.map(String::into_bytes);
        let queued = queue.clone().into_bytes();
        let scripted = script.into_bytes();
        let payloaded = payload.map(String::into_bytes);
        let state = PENDING.to_string().into_bytes();
        let attempts = max_attempts.to_string().into_bytes();
        let backing = backoff.to_string().into_bytes();
        let due = run_at.unwrap_or(now).to_string().into_bytes();
        let created = now.to_string().into_bytes();

        let block = configured.connection.clone();
        // `rule:observability/a-query-is-a-trace-event`'s event belongs to a *statement*, not to `Core\Db`, so this one files it
        // on the same terms as that class's own — read before the connection takes the context,
        // for the reason [`crate::db::QueryWatch`] gives. The split dialect runs up to four
        // statements, so what a driver hands back is a *list* of spans, filed once the connection
        // has let the context go.
        let mut spans = Spans::of(ctx, &block);
        let id = match queue_connection(ctx, handle, &block, PUSH)? {
            Queued::Postgres(postgres) => {
                let bound: [Option<&[u8]>; 9] = [
                    dedupe.as_deref(),
                    Some(&queued),
                    Some(&scripted),
                    payloaded.as_deref(),
                    Some(&state),
                    Some(&attempts),
                    Some(&backing),
                    Some(&due),
                    Some(&created),
                ];
                push_in_one(postgres, &bound, &block, &mut spans)?
            }
            Queued::Framed(framed) => {
                let bound: [Option<&[u8]>; 10] = [
                    Some(&queued),
                    Some(&scripted),
                    payloaded.as_deref(),
                    Some(&state),
                    Some(&attempts),
                    Some(&backing),
                    Some(&due),
                    // `dedupe_key` and `dedupe_pending`: one value in two columns, which
                    // `INSERT_MYSQL`'s doc owns and `INSERT_POSTGRES` spells as `$1` twice.
                    dedupe.as_deref(),
                    dedupe.as_deref(),
                    Some(&created),
                ];
                push_in_two(framed, dedupe.as_deref(), &bound, &block, &mut spans)?
            }
        };
        spans.file(ctx);
        Ok(crate::instance::build(
            &ID,
            [Value::uint(id), Value::str(NvsStr::new(queue.as_bytes()))],
        ))
    }
}

/// The refusal an enqueue the server would not run earns, in one wording for both dialects.
///
/// The table is named rather than the statement, because what an operator does about it is the
/// same whichever dialect was sent and whichever of a [`Split`]'s two halves came back: the
/// migration has not been applied here.
fn insert_refused(block: &str, refused: &dyn std::fmt::Display) -> Fault {
    Fault::thrown_as(
        ThrownClass::Io,
        format!(
            "{PUSH}: the insert into `{JOBS_TABLE}` on `[db.{block}]` was refused: {refused} — \
             `nvs queue migrate` is what creates that table"
        ),
    )
}

/// [`INSERT_POSTGRES`]: § 1's enqueue as the one statement that dedupes and inserts, and the id it
/// answers with.
///
/// # Errors
///
/// [`insert_refused`] for anything the server refused, and a [`Fault::fatal`] for a statement that
/// answered no id — which the `union all` cannot do, the `existing` arm being the only thing the
/// insert stands down for.
fn push_in_one(
    postgres: &mut nvs_db::PgConn,
    bound: &[Option<&[u8]>],
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    let mut answered = postgres
        .query(INSERT_POSTGRES, bound)
        .map_err(|refused| insert_refused(block, &refused))?;
    crate::db::name_span(&mut answered, Some(block));
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
    // `rule:core-classes/db-statement-members`'s `lastId`, which on PostgreSQL is what the `returning` clause handed back —
    // so the decoding is the driver's and this member parses nothing.
    let last = answered.last_id();
    // Taken after the drain, so the span carries what the caller waited for, and filed after
    // the rows have let the context go — `Core\Db`'s reader does both in the same order, and
    // a `PgRows` holds `ctx` until it is dropped. Taken before the id is judged: a statement
    // the server ran is one a trace should show, whatever this member then makes of it.
    spans.note(answered.span());
    last.ok_or_else(|| {
        Fault::fatal(format!(
            "{PUSH}: the insert into `{JOBS_TABLE}` answered no id at all"
        ))
    })
}

/// [`INSERT_MYSQL`]: the same enqueue as the pair one transaction runs, and the id it answers with.
///
/// **A push with no `key` runs the insert alone, outside a transaction of this member's own.**
/// [`Split::first`] is the dedupe read and there is nothing to dedupe against, so the pair is one
/// statement — and that is not only an economy of two round trips. `dedupe_pending = ?` with a
/// `null` parameter is never true, but it is also not an equality InnoDB can answer from the
/// unique index: the `select … for update` degenerates to a range scan and gap-locks what it
/// passes, on the one statement every enqueue runs. PostgreSQL's `existing` arm costs nothing in
/// the same case because it is a planner's arm inside a single statement rather than a lock taken
/// across two.
///
/// **A keyed push holds § 3's property with a transaction where PostgreSQL held it by
/// construction**, which is what [`Split`] means. Where the request already has one open on this
/// connection — § 2's recommended configuration, and the whole point of the memo — `begin` is a
/// `SAVEPOINT` and `commit` its release, so the enqueue still commits with the write that caused
/// it and never on its own.
///
/// **A refusal rolls back before it propagates.** The caller's transaction is the one thing this
/// member must not leave changed on its way out: an enqueue that failed inside a savepoint and
/// left it open would fail the caller's next statement instead, blaming the wrong write.
///
/// # Errors
///
/// [`insert_refused`] for anything the server refused, including the transaction commands, and a
/// [`Fault::fatal`] for an insert whose OK packet carried no `AUTO_INCREMENT` value.
fn push_in_two(
    mut framed: crate::db::Framed<'_>,
    dedupe: Option<&[u8]>,
    bound: &[Option<&[u8]>],
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    let Some(key) = dedupe else {
        return inserted(&mut framed, bound, block, spans);
    };
    let opened = framed
        .begin(None, false)
        .map_err(|refused| insert_refused(block, &refused))?;
    spans.named(block, opened);
    let outcome = deduped(&mut framed, key, bound, block, spans);
    match outcome {
        Ok(id) => {
            let closed = framed
                .commit()
                .map_err(|refused| insert_refused(block, &refused))?;
            spans.named(block, closed);
            Ok(id)
        }
        Err(failed) => {
            // Best effort, and the enqueue's own refusal is what the caller hears: a rollback that
            // fails has poisoned the connection in `nvs-db` already, and a second message about it
            // would replace the one that says what the caller did wrong.
            let _undone = framed.roll_back();
            Err(failed)
        }
    }
}

/// [`Split::first`] and then [`Split::then`], inside the transaction [`push_in_two`] opened.
///
/// # Errors
///
/// [`insert_refused`]'s, and a [`Fault::fatal`] for a `first` whose row answered no integer id —
/// which is the column [`schema`] declares as its identity, so anything else is
/// a table some other writer created.
fn deduped(
    framed: &mut crate::db::Framed<'_>,
    key: &[u8],
    bound: &[Option<&[u8]>],
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    let reading: [Option<&[u8]>; 1] = [Some(key)];
    let mut answered = framed
        .query(INSERT_MYSQL.first, &reading)
        .map_err(|refused| insert_refused(block, &refused))?;
    crate::db::name_span(&mut answered, Some(block));
    // Described before the first row, for `Core\Db`'s reason: a value is read against the
    // definition it arrived under, and the definitions are lent out of a shared borrow while the
    // rows are read out of a mutable one.
    let columns = answered.columns().to_vec();
    let mut pending: Option<u64> = None;
    while let Some(row) = answered
        .next_row()
        .map_err(|refused| insert_refused(block, &refused))?
    {
        // `limit 1`, so the guard is about the shape of the loop and not about a second row —
        // every row is still read, because draining is what ends the statement on this driver.
        if pending.is_some() {
            continue;
        }
        let body = row.value(0).ok_or_else(|| {
            Fault::fatal(format!(
                "{PUSH}: the row read back from `{JOBS_TABLE}` has no first column"
            ))
        })?;
        let scalar = nvs_db::mysql::scalar(&columns[0], body)
            .map_err(|refused| insert_refused(block, &refused))?;
        pending = match scalar {
            nvs_db::MySqlScalar::Int(id) => u64::try_from(id).ok(),
            nvs_db::MySqlScalar::UInt(id) => Some(id),
            other => {
                return Err(Fault::fatal(format!(
                    "{PUSH}: `{JOBS_TABLE}`.`id` read back as {other:?}, not an integer"
                )));
            }
        };
    }
    spans.note(answered.span());
    drop(answered);
    // [`INSERT_POSTGRES`]'s trailing `union all` moved into the caller: a pending job under this
    // key is the answer, and the insert stands down rather than being refused by the index.
    match pending {
        Some(id) => Ok(id),
        None => inserted(framed, bound, block, spans),
    }
}

/// [`Split::then`]: the insert itself, and § 4's `lastId` off the write's own OK packet.
///
/// # Errors
///
/// [`insert_refused`] for anything the server refused, and a [`Fault::fatal`] for an insert that
/// generated no `AUTO_INCREMENT` value — [`MySqlRows::last_id`](nvs_db::MySqlRows::last_id) says
/// `0` for none, and [`schema`] declares the column that makes it impossible.
fn inserted(
    framed: &mut crate::db::Framed<'_>,
    bound: &[Option<&[u8]>],
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    let mut answered = framed
        .query(INSERT_MYSQL.then, bound)
        .map_err(|refused| insert_refused(block, &refused))?;
    crate::db::name_span(&mut answered, Some(block));
    // An insert answers no result set, but draining is what ends the statement on this driver and
    // what lets `last_id` be read at all — `Core\Db\Connection::execute` does the same.
    while answered
        .next_row()
        .map_err(|refused| insert_refused(block, &refused))?
        .is_some()
    {}
    let last = answered.last_id().filter(|id| *id != 0);
    spans.note(answered.span());
    drop(answered);
    last.ok_or_else(|| {
        Fault::fatal(format!(
            "{PUSH}: the insert into `{JOBS_TABLE}` answered no id at all"
        ))
    })
}

/// `rule:observability/a-query-is-a-trace-event`'s events one member is holding until the connection lets the context go, and
/// what is reading them.
///
/// **A member's statements are a list here where `Core\Db`'s are one**, which is what a [`Split`]
/// costs the trace: an enqueue on MySQL is up to four statements — the transaction's two commands
/// and the pair itself — and § 11 describes statements rather than members, so each of them files
/// its own event. The two fields travel together everywhere because neither is usable without the
/// other: a span nothing is reading is never taken, and a taken span cannot be filed until the
/// rows have let go of `ctx`.
struct Spans {
    /// What § 11 and `rule:observability/trace-events-carry-a-kind`'s trace are asking for, read before the first statement goes out —
    /// [`crate::db::QueryWatch`] owns why it cannot be read at the point the event is filed.
    watch: crate::db::QueryWatch,
    /// One entry per statement that ran, in the order they ran.
    taken: Vec<(String, std::time::Duration)>,
}

impl Spans {
    /// What the context and the queue's block say, before the first statement goes out.
    fn of(ctx: &nvs_runtime::Ctx, block: &str) -> Spans {
        Spans {
            watch: crate::db::QueryWatch::named(ctx, Some(block)),
            taken: Vec::new(),
        }
    }

    /// A running statement's span, taken while its rows still lend it out.
    fn note(&mut self, span: &nvs_db::QuerySpan) {
        if let Some(one) = self.watch.taken(span) {
            self.taken.push(one);
        }
    }

    /// The same, for § 7's three commands, which have no rows to hang a span on and so carry the
    /// block name here — `crate::db`'s `file_span` is this rule for a caller holding a `Value`.
    fn named(&mut self, block: &str, mut span: nvs_db::QuerySpan) {
        span.name(block);
        self.note(&span);
    }

    /// Files every event held, once the connection has let the context go.
    fn file(self, ctx: &mut nvs_runtime::Ctx) {
        let Spans { watch, taken } = self;
        for one in taken {
            watch.file(ctx, Some(one));
        }
    }
}

/// What one of § 1's three reading members got back: its row, as the integers § 2's schema
/// declares, and what the server said the statement touched.
///
/// **All three read integers and nothing else** — a state ordinal, a cancel's yes-or-no, four
/// counters — which is what lets [`counted_row`] answer for every one of them over both dialects.
/// A reader that decoded § 9's whole type map would be `Core\Db`'s, and that class has one.
struct Counted {
    /// The first row's requested columns, or `None` for a statement that answered no row at all.
    /// The inner `Option` is "that column was an integer", judged after the drain rather than
    /// inside it, because the connection owes the caller a message boundary either way.
    row: Option<Vec<Option<i64>>>,
    /// `rule:core-classes/db-statement-members`'s affected count, which both drivers define as the rows a write changed or the
    /// rows a read answered with.
    affected: Option<u64>,
}

impl Counted {
    /// Whether the statement touched a row, in whichever way its dialect says so.
    ///
    /// [`CANCEL_POSTGRES`] says it with a returned row and [`CANCEL_MYSQL`] with the count on its
    /// OK packet; a member asking "did this happen" wants the same answer from both, and neither
    /// driver invents one — an `update` that matched nothing answers no row *and* zero.
    fn touched(&self) -> bool {
        self.row.is_some() || self.affected.is_some_and(|rows| rows > 0)
    }
}

/// One statement, on whichever dialect the connection speaks, and the first row's integers.
///
/// **The two texts and their two bindings arrive together**, because a dialect is not only its
/// SQL: [`STATUS_MYSQL`] binds four parameters where [`STATUS_POSTGRES`] binds two, and a signature
/// taking one binding for both would make that impossible to say. What it is *not* is a rewrite —
/// `nvs_db::sql::rewrite` renders one spelling into another, and these are two statements.
///
/// `columns` is how many of the row's columns to read, so `cancel` asks for none and reads
/// [`Counted::touched`] alone.
///
/// # Errors
///
/// Whatever `refused` makes of a server's refusal — one wording per member, since what an operator
/// does about it depends on what was being asked — and a [`Fault::fatal`] for a row narrower than
/// the result set described it, which is a `nvs-db` bug rather than a program's.
fn counted_row(
    queued: Queued<'_>,
    postgres: (&str, &[Option<&[u8]>]),
    framed: (&str, &[Option<&[u8]>]),
    columns: usize,
    block: &str,
    refused: &dyn Fn(&dyn std::fmt::Display) -> Fault,
    spans: &mut Spans,
) -> Result<Counted, Fault> {
    match queued {
        Queued::Postgres(connection) => {
            let mut answered = connection
                .query(postgres.0, postgres.1)
                .map_err(|failed| refused(&failed))?;
            crate::db::name_span(&mut answered, Some(block));
            // Described before the first row, as `Core\Db`'s own reader describes it: a `PgRows`
            // lends its columns and its rows out of one borrow, and the rows are read with it held
            // mutably.
            let described: Vec<nvs_db::PgColumn> = answered.columns().to_vec();
            let mut row: Option<Vec<Option<i64>>> = None;
            // Every row is read before the answer is judged: the connection has to be back at a
            // message boundary before this returns, or the next statement on it — the caller's
            // own, inside the same transaction — meets a busy one. Each of the three statements
            // answers at most one row, so the guard is about the shape of the loop rather than
            // about a second row.
            while let Some(reading) = answered.next_row().map_err(|failed| refused(&failed))? {
                if row.is_some() {
                    continue;
                }
                let mut read = Vec::with_capacity(columns);
                for (at, column) in described.iter().enumerate().take(columns) {
                    let body = reading.column(at).map_err(|failed| refused(&failed))?;
                    let scalar = column.scalar(body).map_err(|failed| refused(&failed))?;
                    read.push(match scalar {
                        nvs_db::PgScalar::Int(number) => Some(number),
                        _ => None,
                    });
                }
                row = Some(read);
            }
            let affected = answered.affected();
            spans.note(answered.span());
            Ok(Counted { row, affected })
        }
        Queued::Framed(mut connection) => {
            let mut answered = connection
                .query(framed.0, framed.1)
                .map_err(|failed| refused(&failed))?;
            crate::db::name_span(&mut answered, Some(block));
            // Cloned for the arm above's reason and for one more: this driver reads a value
            // against the definition it arrived under, so the definitions outlive the borrow the
            // rows are read through.
            let described = answered.columns().to_vec();
            let mut row: Option<Vec<Option<i64>>> = None;
            while let Some(reading) = answered.next_row().map_err(|failed| refused(&failed))? {
                if row.is_some() {
                    continue;
                }
                let mut read = Vec::with_capacity(columns);
                for (at, column) in described.iter().enumerate().take(columns) {
                    // Unreachable from source: `nvs-db` decodes one value per definition, so a
                    // row is exactly as wide as the result set said, and the statement is this
                    // module's own text rather than anything a program wrote. `Core\Db`'s walk
                    // states the same reading — it is a `fatal` because a narrower row is that
                    // crate disagreeing with itself and not something a statement can ask for.
                    let body = reading.value(at).ok_or_else(|| {
                        Fault::fatal(format!(
                            "the row has no column {at}, where the result set described {}",
                            described.len()
                        ))
                    })?;
                    let scalar =
                        nvs_db::mysql::scalar(column, body).map_err(|failed| refused(&failed))?;
                    read.push(match scalar {
                        nvs_db::MySqlScalar::Int(number) => Some(number),
                        // § 9's `uint` row: MySQL answers `count` as `bigint unsigned`, so the
                        // two counters that are not cast land here rather than above.
                        nvs_db::MySqlScalar::UInt(number) => i64::try_from(number).ok(),
                        _ => None,
                    });
                }
                row = Some(read);
            }
            let affected = answered.affected();
            spans.note(answered.span());
            Ok(Counted { row, affected })
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Queue::status(Queue\Id $job): Queue\State` — `rule:concurrency/queue-four-members` and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`.
    ///
    /// **The receipt is the whole argument**, because it carries the queue as well as the row: a
    /// member taking a bare id would have to be told the queue beside it or search every one, and
    /// the first is what [`ID`] exists to spare the caller.
    ///
    /// **One statement over both of § 2's tables**, so the answer describes one instant.
    /// [`STATUS_POSTGRES`] owns why that is two arms rather than two queries, and the reading it rests on is
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
        let twice: Vec<Option<&[u8]>> = bound.iter().chain(bound.iter()).copied().collect();
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
        // `rule:observability/a-query-is-a-trace-event`'s event, as `push` files it and for the reason given there.
        let mut spans = Spans::of(ctx, &block);
        let counted = counted_row(
            queue_connection(ctx, handle, &block, STATUS_OF)?,
            (STATUS_POSTGRES, &bound),
            // The same two values a second time, which is [`STATUS_MYSQL`]'s whole difference: a
            // `?` cannot be named twice where a `$1` can.
            (STATUS_MYSQL, &twice),
            1,
            &block,
            &refused_by_server,
            &mut spans,
        )?;
        // Filed as `push` files it, and before the answer is judged for the same reason: the
        // statement ran either way, and a trace showing it is what § 11 asks for.
        spans.file(ctx);
        let ordinal = counted
            .row
            .and_then(|read| read.first().copied())
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
    /// `Core\Queue::cancel(Queue\Id $job): bool` — `rule:concurrency/queue-four-members`.
    ///
    /// **It answers a `bool` although § 1 annotates no return**, and that is a decision rather than
    /// a liberty: the member's own semantics are a race it can lose — § 4 lets a worker claim the
    /// job at any moment, and it is the *ordinary* outcome for a job cancelled late, not an unlucky
    /// one — so a caller has no other way to learn whether the work is still going to happen. A
    /// `void` spelling would make "cancelled" and "too late" look identical at the call site, and
    /// throwing for the second would make the commonest race an exception. `rule:concurrency/queue-four-members` carries the
    /// annotation now, so there is one home for it.
    ///
    /// **The state test is in [`CANCEL_POSTGRES`] and not here**, which is why nothing in this body reads
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
        // `rule:observability/a-query-is-a-trace-event`'s event, as `push` files it and for the reason given there.
        let mut spans = Spans::of(ctx, &block);
        // No column is read: `returning id` is in [`CANCEL_POSTGRES`] to make the affected count
        // observable and nothing reads the id, since the caller already holds it —
        // [`Counted::touched`] is that reading, and the one [`CANCEL_MYSQL`] answers without a
        // row at all.
        let counted = counted_row(
            queue_connection(ctx, handle, &block, CANCEL_OF)?,
            (CANCEL_POSTGRES, &bound),
            (CANCEL_MYSQL, &bound),
            0,
            &block,
            &refused_by_server,
            &mut spans,
        )?;
        // As `push`: taken while the rows still lend the span out, filed once they have let the
        // context go. A cancel that lost § 4's race is a statement like any other and files one.
        spans.file(ctx);
        Ok(Value::bool(counted.touched()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Queue::stats(string $queue): Queue\Stats` — `rule:concurrency/queue-four-members` and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`.
    ///
    /// **Asked about a queue and not about a job**, which is what makes it the odd member of § 1's
    /// four: the other three take the receipt [`ID`] is, because they are about one row, and this
    /// one is about a population an operator watches.
    ///
    /// **One statement, so the four counters are one fact.** [`COUNTS_POSTGRES`] owns why an aggregate with
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
        let twice: Vec<Option<&[u8]>> = bound.iter().chain(bound.iter()).copied().collect();
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
        // `rule:observability/a-query-is-a-trace-event`'s event, as `push` files it and for the reason given there.
        let mut spans = Spans::of(ctx, &block);
        let read = counted_row(
            queue_connection(ctx, handle, &block, STATS_OF)?,
            (COUNTS_POSTGRES, &bound),
            // The queue a second time, for [`STATUS_MYSQL`]'s reason: the dead-letter subquery and
            // the aggregate's own `where` each bind their own `?`.
            (COUNTS_MYSQL, &twice),
            4,
            &block,
            &refused_by_server,
            &mut spans,
        )?;
        // As `push`, and before the row is judged for the reason `status` gives.
        spans.file(ctx);
        let row = read.row.ok_or_else(|| {
            Fault::fatal(format!(
                "{STATS_OF}: the aggregate over `{JOBS_TABLE}` answered no row at all, and one \
                 with no `group by` answers exactly one however empty the table is"
            ))
        })?;
        let mut counted = [0i64; 4];
        for (at, held) in counted.iter_mut().enumerate() {
            *held = row.get(at).copied().flatten().ok_or_else(|| {
                Fault::fatal(format!(
                    "{STATS_OF}: the `{}` counter came back as something other than an integer, \
                     and both dialects declare all four columns a `bigint` here",
                    STATS.slots[at]
                ))
            })?;
        }
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
    /// [`COUNTS_POSTGRES`] sums over [`JOBS_TABLE`] alone for [`DEAD_TABLE`]'s reason.
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
        CANCEL_MYSQL, CANCEL_POSTGRES, CLAIM_MYSQL, CLAIM_POSTGRES, COUNTS_MYSQL, COUNTS_POSTGRES,
        DEAD_LETTER_MYSQL, DEAD_LETTER_POSTGRES, DEAD_TABLE, INSERT_MYSQL, INSERT_POSTGRES,
        JOBS_TABLE, PENDING, PUSH, QUEUES_MYSQL, QUEUES_POSTGRES, RETRY_CAP_MS, RETRY_MYSQL,
        RETRY_POSTGRES, STATE, STATS, STATS_ATTEMPTS_AT, STATS_ATTEMPTS_SLOT, STATS_CLAIMED_AT,
        STATS_CLAIMED_SLOT, STATS_DEAD_AT, STATS_DEAD_SLOT, STATS_PENDING_AT, STATS_PENDING_SLOT,
        STATUS_MYSQL, STATUS_POSTGRES, SUCCEEDED_MYSQL, SUCCEEDED_POSTGRES, dead_errors, migration,
        no_dialect, retry_at,
    };

    /// An agreement test rather than a wording one, in `the_refusal_names_every_driver_that_sends`'s
    /// shape one module over: what this file must not do is tell an operator to fix the wrong thing.
    /// The roster behind it is [`crate::db::rendering_for`]'s `None` rather than a second list of
    /// five drivers here — so a driver gaining a statement path in that module moves this refusal
    /// with it, and a sixth arriving fails the build in [`no_dialect`] before it reaches this test.
    ///
    /// **Which gap the sentence names is the thing under test**, and there are two of them. Every
    /// driver has § 2's schema — [`migration`] emits it in whichever dialect the driver speaks — so
    /// no refusal may send an operator off to build tables `nvs queue migrate` already converges,
    /// and the only thing left to wait on is § 4's statements. For SQL Server there is a second
    /// thing behind them, which `rule:core-classes/queue-storage-is-a-table` orders: two nulls are
    /// equal there, so the vocabulary grows a filtered index before the fourth dialect is written.
    /// The sentence has to say so, because an operator who reads only the first half will expect
    /// the queue with the next release.
    #[test]
    fn the_queues_refusal_is_only_ever_about_a_driver_that_cannot_send() {
        for driver in nvs_db::Driver::ALL {
            let refused = format!("{:?}", no_dialect(PUSH, "main", driver));
            assert!(
                refused.contains(driver.display_name()),
                "a refusal an operator can act on names the driver the block resolved to: {refused}"
            );
            // The roster is `runs` and not a second list here, so a fourth backend moves this test
            // with the seam rather than after it.
            let runs = super::runs(driver);
            assert_eq!(
                runs,
                refused.contains("this is a bug"),
                "{driver:?} runs all four members, so nothing should be refusing it: {refused}"
            );
            if runs {
                continue;
            }
            assert!(
                refused.contains("the queue has no statements for it yet")
                    && refused.contains("`nvs queue migrate` converges"),
                "{driver:?} has § 2's schema, so what it waits on is § 4's statements and the \
                 refusal may not claim otherwise: {refused}"
            );
            assert_eq!(
                driver == nvs_db::Driver::SqlServer,
                refused.contains("filtered index"),
                "only SQL Server waits on the vocabulary as well as on the statements: {refused}"
            );
        }
    }

    /// The other direction of the test above, which is not the same assertion: a `?` in a
    /// PostgreSQL text binds nothing and the server cannot say so usefully — it reads the character
    /// as an operator and answers a syntax error naming a statement no `.nvst` case can see. The
    /// two dialects are two whole texts precisely because no rewrite stands between them
    /// ([`Split`]'s doc), so each list owes the check that it is written in its own dialect
    /// throughout rather than in most places.
    #[test]
    fn no_postgresql_statement_binds_the_other_dialects_placeholder() {
        for (name, sql) in [
            ("INSERT_POSTGRES", INSERT_POSTGRES),
            ("CLAIM_POSTGRES", CLAIM_POSTGRES),
            ("DEAD_LETTER_POSTGRES", DEAD_LETTER_POSTGRES),
            ("STATUS_POSTGRES", STATUS_POSTGRES),
            ("CANCEL_POSTGRES", CANCEL_POSTGRES),
            ("COUNTS_POSTGRES", COUNTS_POSTGRES),
            ("QUEUES_POSTGRES", QUEUES_POSTGRES),
            ("SUCCEEDED_POSTGRES", SUCCEEDED_POSTGRES),
            ("RETRY_POSTGRES", RETRY_POSTGRES),
        ] {
            assert!(
                !sql.contains('?'),
                "`{name}` is PostgreSQL's, so every parameter in it is a `$n`: {sql}"
            );
        }
    }

    /// Everything one dialect's DDL says about one of § 2's two tables, joined into one text.
    ///
    /// **A table's share of an emission, not a statement**, because how many statements a table
    /// takes is itself the thing the dialects disagree about: MySQL declares an index inside the
    /// `CREATE TABLE` and the other three write one of their own. Joining them means every
    /// assertion below asks what the table *has* rather than which statement said so, which is the
    /// only form in which one assertion can hold every dialect.
    fn table_ddl(driver: nvs_db::Driver, table: &str) -> String {
        let text = nvs_db::ddl::create_schema(&super::schema(), nvs_db::Dialect::of(driver))
            .into_iter()
            .filter(|statement| statement.contains(table))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            !text.is_empty(),
            "{} builds no `{table}`",
            driver.display_name()
        );
        text
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
            "one entry, which is `schema`'s doc's decision and the depth `nvs_jobs` pays for"
        );
        assert_eq!(entries[0]["at"], 1_700_000_000_123_i64);
        assert_eq!(entries[0]["class"], "RuntimeError");
        assert_eq!(
            entries[0]["message"], "the \"endpoint\" \\ refused",
            "the message crosses the column unchanged, quotes and all"
        );
    }

    /// [`migration`] and [`crate::db::rendering_for`] answer about one roster each, and this is the
    /// assertion that the queue's is inside `Core\Db`'s: § 2's schema is only ever sent over a
    /// driver a statement can be bound for.
    ///
    /// **Asked as containment rather than as a list**, because a list here would be the third copy
    /// of the same roster and would go stale silently — a driver gaining a `Dialect` in `nvs-db`
    /// while this file kept its own answer is precisely the drift that would print PostgreSQL's DDL
    /// for a server that cannot run it.
    ///
    /// **One direction and not two**: `Core\Db` binds for all five drivers and § 2's schema is one
    /// value emitted in every dialect, so what is left to hold is the pairing itself — a driver
    /// whose rewriter disagreed with the dialect its schema is emitted in would send the right
    /// columns in the wrong SQL. [`no_dialect`] is where an operator is told that a driver runs no
    /// statement yet, which is a different question from having a schema.
    #[test]
    fn the_schema_has_a_dialect_for_every_driver_that_can_be_sent_one() {
        for driver in nvs_db::Driver::ALL {
            // "Is there an encoder at all" became unconditional when SQLite
            // gained one, so what is left to hold is the pairing: a schema
            // written in one dialect and sent through another's rewriter is
            // the failure this direction was ever about.
            assert_eq!(
                crate::db::rendering_for(driver).0,
                nvs_db::Dialect::of(driver),
                "{driver:?} has § 2's schema, so `Core\\Db` owes it a statement in the dialect \
                 that schema is written in"
            );
        }
        // MariaDB and MySQL are one dialect and so one emission, which is what makes MariaDB's
        // schema MySQL's own rather than a copy of it that something has to keep in step.
        let sql_of = |driver: nvs_db::Driver| -> Vec<String> {
            migration(driver).into_iter().map(|step| step.sql).collect()
        };
        assert_eq!(
            sql_of(nvs_db::Driver::MariaDb),
            sql_of(nvs_db::Driver::MySql),
            "MariaDB's schema is MySQL's own, statement for statement"
        );
    }

    /// [`schema`] is the only place the queue's columns exist and the statements above are its only
    /// readers — one value and one roster of statements in one file, with nothing but this test
    /// between them. A column renamed in the value and nowhere else still compiles, still migrates,
    /// and fails on the first `push` against a database an operator has already built.
    ///
    /// **Every assertion runs against both dialects the statements are written in**, which is what
    /// keeps them one schema rather than two: the constructs around the columns are the emitter's
    /// and differ freely, and the columns themselves may not. A column added to PostgreSQL's push
    /// alone fails here on MySQL's.
    #[test]
    fn the_ddl_creates_every_column_the_statements_name() {
        // `INSERT_POSTGRES`'s parenthesised column list is the widest claim any statement makes about
        // `nvs_jobs`, and § 6's move is the same for `nvs_dead_jobs`: every other statement reads a
        // subset of one of the two. Both are cut once and asked of each dialect in turn.
        let written = |statement: &'static str, table: &str| {
            statement
                .split_once(&format!("insert into {table} ("))
                .and_then(|(_, rest)| rest.split_once(')'))
                .expect("the statement names its columns as one parenthesised list")
                .0
        };
        for (driver, push, moving) in [
            (
                nvs_db::Driver::Postgres,
                INSERT_POSTGRES,
                DEAD_LETTER_POSTGRES,
            ),
            (
                nvs_db::Driver::MySql,
                INSERT_MYSQL.then,
                DEAD_LETTER_MYSQL.first,
            ),
        ] {
            // Each dialect is asked about its *own* two statements: the columns are one value and
            // the SQL around them is the emitter's, so a column added to one dialect's push and
            // not to the value fails here rather than against a server an operator has already
            // built.
            let dialect = driver.display_name();
            let inserted = written(push, JOBS_TABLE);
            let moved = written(moving, DEAD_TABLE);
            let jobs = table_ddl(driver, JOBS_TABLE);
            let dead = table_ddl(driver, DEAD_TABLE);
            assert!(
                jobs.contains(JOBS_TABLE) && dead.contains(DEAD_TABLE),
                "{dialect}: each `create table` builds the table its own constant names"
            );

            for column in inserted.split(',').map(str::trim) {
                assert!(
                    jobs.contains(&format!("{column} ")),
                    "{dialect}: the `jobs` DDL creates `{column}`, which `INSERT_POSTGRES` binds"
                );
            }
            for column in ["state ", "attempts ", "queue "] {
                assert!(
                    jobs.contains(column),
                    "{dialect}: the `jobs` DDL creates the column `STATUS_POSTGRES` and `COUNTS_POSTGRES` read as \
                     `{column}`"
                );
            }
            // `CLAIM_POSTGRES` is the one statement that reads a column no `INSERT_POSTGRES` writes: a claim's own
            // instant, which is where § 4's visibility timeout is measured from.
            assert!(
                jobs.contains("claimed_at "),
                "{dialect}: the `jobs` DDL creates `claimed_at`, which `CLAIM_POSTGRES` writes and reads back"
            );
            assert!(
                jobs.contains("(queue, state, run_at)"),
                "{dialect}: `CLAIM_POSTGRES` seeks by queue, then state, then due-ness, which is the order \
                 of this index"
            );
            // Gap 3's constraint, asked as what it must *be*: one uniqueness rule, named
            // `nvs_jobs_dedupe`, over the column the statements maintain. Which rows it covers is
            // not the DDL's business any more — `dedupe_pending` is null for every row that is not
            // pending — and a constraint over `dedupe_key` instead would refuse a second push of a
            // key whose first job is long done.
            assert!(
                jobs.contains("UNIQUE")
                    && jobs.contains("nvs_jobs_dedupe")
                    && jobs.contains("dedupe_pending"),
                "{dialect}: gap 3's constraint is unique over `dedupe_pending`, which is the column \
                 `INSERT_POSTGRES` writes the key into while the job is pending"
            );

            for column in ["id ", "queue "] {
                assert!(
                    dead.contains(column),
                    "{dialect}: the `dead_letter` DDL creates `{column}`, which is what \
                     `DEAD_TABLE`'s doc says this module reads of it"
                );
            }
            for column in moved.split(',').map(str::trim) {
                assert!(
                    dead.contains(&format!("{column} ")),
                    "{dialect}: the `dead_letter` DDL creates `{column}`, which `DEAD_LETTER_POSTGRES` writes"
                );
            }
        }

        // § 4's mutual exclusion is the database's in both dialects, and asked of each because the
        // clause moved: PostgreSQL's sits inside the CTE its `update` reads, MySQL's on the `select`
        // half of a `Split`, and a claim that lost it in either would take a row a second worker is
        // already running.
        for (dialect, claim) in [("postgres", CLAIM_POSTGRES), ("mysql", CLAIM_MYSQL.first)] {
            assert!(
                claim.contains("for update skip locked"),
                "{dialect}: this is the spelling that asks the server for § 4's exclusion"
            );
        }
        // The roster is read off the same table and the same column a claim is then keyed on, which
        // is the whole of why § 2 needs no fifth key to name a worker's queues. Asked of both
        // dialects, because both spell it themselves: the roster rests on no construct MySQL lacks,
        // so nothing but this would notice one of the two texts reading a different column.
        for (dialect, roster) in [("postgres", QUEUES_POSTGRES), ("mysql", QUEUES_MYSQL)] {
            assert!(
                roster.contains(JOBS_TABLE) && roster.contains("distinct queue"),
                "{dialect}: the roster is read off the column the push writes the queue name into"
            );
        }
        // The lease is what both moves are keyed on, in the two places each dialect spells it: one
        // statement on PostgreSQL, and the copy *and* the delete on MySQL — where keying only the
        // first would take a row out from under the worker that claimed it next.
        assert!(
            DEAD_LETTER_POSTGRES.contains(&format!("delete from {JOBS_TABLE} "))
                && DEAD_LETTER_POSTGRES.contains("claimed_at = $2::bigint"),
            "the move takes the row out of the jobs table keyed on the lease, as `SUCCEEDED_POSTGRES` is"
        );
        assert!(
            DEAD_LETTER_MYSQL
                .then
                .contains(&format!("delete from {JOBS_TABLE} ")),
            "MySQL's move is the half that empties the jobs table, and the copy runs before it"
        );
        for half in [DEAD_LETTER_MYSQL.first, DEAD_LETTER_MYSQL.then] {
            assert!(
                half.contains("claimed_at = ?"),
                "both halves of MySQL's move are keyed on the lease, not just the one that reads"
            );
        }
    }

    /// The second dialect's statements are held to what MySQL *cannot* spell, which is the whole
    /// reason they exist: a text carrying any of these four would have been a transcription of
    /// PostgreSQL's rather than a dialect, and would fail on the first server it reached.
    ///
    /// Asserted as absence over every half of every [`Split`] rather than on the three that happen
    /// to be interesting, so a fourth statement added to this set is covered on the day it lands.
    #[test]
    fn the_mysql_statements_spell_nothing_only_postgresql_has() {
        let mut texts: Vec<(&str, &str)> = Vec::new();
        for (member, split) in [
            ("push", INSERT_MYSQL),
            ("claim", CLAIM_MYSQL),
            ("move", DEAD_LETTER_MYSQL),
        ] {
            texts.push((member, split.first));
            texts.push((member, split.then));
        }
        // § 5's three readers are not [`Split`]s — nothing in them rests on a construct MySQL
        // lacks — but they are the same second dialect and owe the same check.
        texts.push(("status", STATUS_MYSQL));
        texts.push(("cancel", CANCEL_MYSQL));
        texts.push(("stats", COUNTS_MYSQL));
        // §§ 4 and 6's three worker statements, which are not [`Split`]s either and owe the check
        // for the same reason the readers above do.
        texts.push(("roster", QUEUES_MYSQL));
        texts.push(("succeeded", SUCCEEDED_MYSQL));
        texts.push(("retry", RETRY_MYSQL));
        for (member, sql) in texts {
            for absent in ["returning", "::", "$1", "with ", "filter (where"] {
                assert!(
                    !sql.contains(absent),
                    "{member}'s MySQL text spells `{absent}`, which MySQL has no reading for"
                );
            }
            assert!(
                sql.contains('?'),
                "{member}'s MySQL text binds nothing, so it is not the statement it replaces"
            );
        }
    }

    /// A second text that is not a [`Split`] binds exactly as many values as its first names, which
    /// is the one thing a transcription loses silently. The two counts are not equal by
    /// construction: a `$n` may be named as often as a statement likes and a `?` may not, so
    /// [`STATUS_MYSQL`] carries four placeholders for two values and [`COUNTS_MYSQL`] two for one.
    /// A text that dropped or doubled one still parses on the server and binds a value into the
    /// wrong column.
    ///
    /// Asked of the six pairs and not of the [`Split`]s, whose two halves divide one PostgreSQL
    /// text's placeholders between them and so answer a different question.
    #[test]
    fn each_second_text_binds_a_value_wherever_its_first_names_one() {
        for (member, postgres, mysql) in [
            ("status", STATUS_POSTGRES, STATUS_MYSQL),
            ("cancel", CANCEL_POSTGRES, CANCEL_MYSQL),
            ("stats", COUNTS_POSTGRES, COUNTS_MYSQL),
            ("roster", QUEUES_POSTGRES, QUEUES_MYSQL),
            ("succeeded", SUCCEEDED_POSTGRES, SUCCEEDED_MYSQL),
            ("retry", RETRY_POSTGRES, RETRY_MYSQL),
        ] {
            assert_eq!(
                mysql.matches('?').count(),
                postgres.matches('$').count(),
                "{member}'s MySQL text binds a different number of values than its PostgreSQL twin \
                 names, so one list of arguments cannot be right about both"
            );
        }
        // The one place the two orders differ, asserted rather than left to a doc comment: MySQL's
        // retry writes `run_at` in its `set` clause, which is left of the `where`, so its first
        // value is the delay where PostgreSQL's first is the id. A caller sending PostgreSQL's
        // order into this text would push every job's next attempt out to its own id.
        assert!(
            RETRY_MYSQL
                .split_once("run_at = ?")
                .is_some_and(|(before, _)| !before.contains('?')),
            "MySQL's retry binds the delay before the lease it is keyed on"
        );
    }

    /// A worker reads a claimed job's columns **by ordinal**, so the two dialects owe each other
    /// more than a column set here: the same names in the same order. Nothing else would notice
    /// them diverging — a claim that swapped `attempts` and `max_attempts` still runs, still
    /// answers six values, and puts § 6's ladder on the wrong number.
    ///
    /// The alias is stripped rather than matched, because `attempts + 1 as attempts` is exactly the
    /// difference the split forces ([`CLAIM_MYSQL`]'s doc owns why) and it is not a difference in
    /// what the column *is*.
    #[test]
    fn both_dialects_answer_a_claim_with_the_same_columns() {
        fn named(list: &str) -> Vec<&str> {
            list.split(',')
                .map(|one| one.trim().rsplit(" as ").next().unwrap_or(one).trim())
                .collect()
        }
        let returned = CLAIM_POSTGRES
            .split_once("returning ")
            .expect("PostgreSQL's claim answers with a `returning` list")
            .1;
        let selected = CLAIM_MYSQL
            .first
            .strip_prefix("select ")
            .and_then(|rest| rest.split_once(" from "))
            .expect("MySQL's claim reads its columns before its table")
            .0;
        assert_eq!(
            named(returned),
            named(selected),
            "a worker reads these by position, so the two dialects answer one list or neither does"
        );
    }

    /// [`retry_at`] is `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s ladder and this is what makes it one: the delay doubles, it
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
        assert_eq!(
            case("Pending"),
            0,
            "`CLAIM_POSTGRES`'s first arm spells this `0`"
        );
        assert!(
            CLAIM_POSTGRES.contains("state = 0") && CLAIM_MYSQL.first.contains("state = 0"),
            "both claims seek pending rows by the ordinal above"
        );
        // Neither push spells the ordinal at all: `dedupe_pending` carries what `state = 0` used to
        // say, in the form a plain unique key can answer, and the statements around it are what
        // maintain it. This is the assertion that both dialects go through that column.
        assert!(
            INSERT_POSTGRES.contains("dedupe_pending = $1::text")
                && INSERT_MYSQL.first.contains("dedupe_pending = ?"),
            "both dedupe arms read the column that carries pending-ness rather than the ordinal"
        );
        assert_eq!(
            case("Dead"),
            3,
            "`STATUS_POSTGRES`'s dead-letter arm spells this `3`"
        );
        assert!(
            STATUS_POSTGRES.contains("select 3 from nvs_dead_jobs"),
            "`STATUS_POSTGRES` answers the ordinal above for a dead-lettered job"
        );
        assert!(
            STATUS_POSTGRES.contains(JOBS_TABLE) && STATUS_POSTGRES.contains(DEAD_TABLE),
            "`STATUS_POSTGRES` reads both of § 2's tables"
        );
        assert!(
            STATUS_MYSQL.contains("select 3 from nvs_dead_jobs")
                && STATUS_MYSQL.contains(JOBS_TABLE)
                && STATUS_MYSQL.contains(DEAD_TABLE),
            "`STATUS_MYSQL` answers the same ordinal over the same two tables"
        );
        assert_eq!(
            case("Cancelled"),
            4,
            "`CANCEL_POSTGRES` writes this ordinal in place of `Pending`"
        );
        assert!(
            CANCEL_POSTGRES.contains("set state = 4") && CANCEL_POSTGRES.contains("and state = 0"),
            "`CANCEL_POSTGRES` moves a job from the ordinal above to the one before it, and only that one"
        );
        assert!(
            CANCEL_MYSQL.contains("set state = 4") && CANCEL_MYSQL.contains("and state = 0"),
            "`CANCEL_MYSQL` moves a job between the same two ordinals"
        );
        assert_eq!(
            case("Succeeded"),
            2,
            "`SUCCEEDED_POSTGRES` writes this ordinal for an attempt that returned"
        );
        // Both dialects for each write-back, because each spells the ordinal in its own text: a
        // second text that transcribed the statement and not the number would run everywhere and
        // finish a job into a state nothing reads.
        for (dialect, succeeded) in [("postgres", SUCCEEDED_POSTGRES), ("mysql", SUCCEEDED_MYSQL)] {
            assert!(
                succeeded.contains("set state = 2"),
                "{dialect}: the write-back for a job that ran spells the ordinal above"
            );
        }
        for (dialect, retry) in [("postgres", RETRY_POSTGRES), ("mysql", RETRY_MYSQL)] {
            assert!(
                retry.contains("set state = 0"),
                "{dialect}: a retried job goes back to `Pending`'s own ordinal, which is what makes \
                 it claimable"
            );
        }
        assert!(
            COUNTS_POSTGRES.contains("filter (where state = 0)"),
            "`COUNTS_POSTGRES` counts waiting jobs by `Pending`'s own ordinal"
        );
        assert!(
            COUNTS_MYSQL.contains("case when state = 0")
                && COUNTS_MYSQL.contains("case when state = 1"),
            "`COUNTS_MYSQL` counts by the same two ordinals, in the spelling MySQL has for a \
             filtered count"
        );
        assert_eq!(
            case("Claimed"),
            1,
            "`COUNTS_POSTGRES`'s second counter spells this `1`"
        );
        // Both dialects, because both spell the ordinals themselves: the arms are in the half that
        // reads and the write is in the half that takes, which is the one place the split moved an
        // ordinal from one text to another.
        for (dialect, takes, arms) in [
            ("postgres", CLAIM_POSTGRES, CLAIM_POSTGRES),
            ("mysql", CLAIM_MYSQL.then, CLAIM_MYSQL.first),
        ] {
            assert!(
                takes.contains("set state = 1") && arms.contains("(state = 1 and claimed_at"),
                "{dialect}: a claim writes the ordinal above, and the timeout takes it back"
            );
            assert!(
                arms.contains("state = 0 and run_at"),
                "{dialect}: a claim's first arm takes pending rows by `Pending`'s own ordinal"
            );
        }
        for (dialect, roster) in [("postgres", QUEUES_POSTGRES), ("mysql", QUEUES_MYSQL)] {
            assert!(
                roster.contains("state = 0 and run_at")
                    && roster.contains("state = 1 and claimed_at"),
                "{dialect}: the roster asks the claim's two arms, so a roster entry is a queue with \
                 due work in it"
            );
        }
        assert!(
            COUNTS_POSTGRES.contains("filter (where state = 1)"),
            "`COUNTS_POSTGRES` counts jobs a worker holds by the ordinal above"
        );
        assert!(
            COUNTS_POSTGRES.contains(JOBS_TABLE) && COUNTS_POSTGRES.contains(DEAD_TABLE),
            "`COUNTS_POSTGRES` reads both of § 2's tables, taking the depth from the second"
        );
    }

    /// `rule:core-classes/queue-storage-is-a-table`'s guarantee is a plain column under a plain
    /// unique key, which means the statements are what maintain it — so this asks every statement
    /// that moves a job in or out of `Pending`, not the one that reads the column.
    ///
    /// **An agreement test, and that is the shape the guarantee needs.** A dialect that named
    /// `dedupe_pending` in its push and nowhere else would hold a key against a job long since
    /// finished, and the push is the one statement that would still look right on its own line.
    /// What both dialects owe is the same four facts: the insert writes it, the three transitions
    /// out of `Pending` clear it, the retry restores it from `dedupe_key`, and the dead-letter
    /// names it nowhere because the row leaves [`JOBS_TABLE`] entirely.
    #[test]
    fn every_transition_out_of_pending_maintains_the_dedupe_column() {
        for (dialect, push, claim, succeeded, retry, cancel, moving) in [
            (
                "postgres",
                INSERT_POSTGRES,
                CLAIM_POSTGRES,
                SUCCEEDED_POSTGRES,
                RETRY_POSTGRES,
                CANCEL_POSTGRES,
                DEAD_LETTER_POSTGRES,
            ),
            (
                "mysql",
                INSERT_MYSQL.then,
                CLAIM_MYSQL.then,
                SUCCEEDED_MYSQL,
                RETRY_MYSQL,
                CANCEL_MYSQL,
                DEAD_LETTER_MYSQL.first,
            ),
        ] {
            assert!(
                push.contains("dedupe_pending"),
                "{dialect}: the push writes the column `nvs_jobs_dedupe` is built over"
            );
            for (name, statement) in [
                ("claim", claim),
                ("write-back", succeeded),
                ("cancel", cancel),
            ] {
                assert!(
                    statement.contains("dedupe_pending = null"),
                    "{dialect}: the {name} leaves the job not pending, so it clears the column and \
                     a second push of the key is admitted"
                );
            }
            assert!(
                retry.contains("dedupe_pending = dedupe_key"),
                "{dialect}: the retry arms the job again, so the key it was pushed under is \
                 pending again too"
            );
            assert!(
                !moving.contains("dedupe_pending"),
                "{dialect}: a dead-lettered row leaves `{JOBS_TABLE}`, so nothing has to release \
                 its key and `{DEAD_TABLE}` has no such column"
            );
        }
    }

    /// Three separate places say what order [`STATS`]'s counters are in — [`COUNTS_POSTGRES`]'s select
    /// list, the slot roster, and the `*_AT` index each reader passes — and only the first is
    /// beyond a test's reach. Nothing else would notice the other two disagreeing: a swapped pair
    /// still type-checks, still runs, and answers the wrong number.
    #[test]
    fn every_stats_counter_reads_the_slot_its_member_is_named_for() {
        let members: Vec<&str> = STATS.instance.iter().map(|one| one.name).collect();
        assert_eq!(
            members, STATS.slots,
            "each counter is named for the slot it reads, in `COUNTS_POSTGRES`'s column order"
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

    /// `rule:core-classes/queue-storage-is-a-table`: every driver has a schema, where two of them
    /// had no dialect at all.
    ///
    /// The lists this replaces are keyed on the dialect, so [`migration`] answers `None` for SQL
    /// Server and SQLite and `nvs queue migrate` refuses those two backends before it opens
    /// anything. A schema *value* is keyed on nothing: the dialect is `nvs_db::ddl`'s to choose, so
    /// the question this asks is the one the retirement makes answerable — not "which drivers have
    /// a list" but "does the one value emit both tables in every dialect", which is what makes the
    /// roster of supported backends stop being a roster.
    #[test]
    fn every_driver_has_a_queue_schema_and_none_answers_none() {
        let schema = super::schema();
        for driver in nvs_db::Driver::ALL.iter().copied() {
            let statements = nvs_db::ddl::create_schema(&schema, nvs_db::Dialect::of(driver));
            assert!(
                !statements.is_empty(),
                "{} emits no statement for a schema that has two tables",
                driver.display_name()
            );
            for table in [JOBS_TABLE, DEAD_TABLE] {
                assert!(
                    statements.iter().any(|statement| statement.contains(table)),
                    "{} builds no `{table}`",
                    driver.display_name()
                );
            }
        }
    }

    /// `rule:core-classes/queue-storage-is-a-table`: the dedupe constraint has **one** spelling, and
    /// it is the same one in all four dialects.
    ///
    /// This is the retirement's own open question asserted rather than argued. The two lists reach
    /// gap 3's guarantee by two constructs the vocabulary refuses — a partial index and a stored
    /// generated column — so the assertion that matters is not that the constraint exists but that
    /// what carries it is a plain column and a plain unique key, present in the `CREATE TABLE`
    /// itself on every backend. A dialect that grew its own spelling again fails here.
    #[test]
    fn the_queues_dedupe_constraint_is_one_column_in_every_dialect() {
        let schema = super::schema();
        for driver in nvs_db::Driver::ALL.iter().copied() {
            let jobs = nvs_db::ddl::create_schema(&schema, nvs_db::Dialect::of(driver))
                .into_iter()
                .find(|statement| statement.contains(JOBS_TABLE))
                .expect("the jobs table is created in every dialect");
            for construct in ["dedupe_key", "dedupe_pending", "nvs_jobs_dedupe"] {
                assert!(
                    jobs.contains(construct),
                    "{}'s `{JOBS_TABLE}` does not declare `{construct}`",
                    driver.display_name()
                );
            }
            assert!(
                !jobs.contains("where state") && !jobs.contains("GENERATED ALWAYS AS ("),
                "{} reached gap 3's guarantee by a construct the vocabulary does not hold",
                driver.display_name()
            );
        }
    }

    /// `rule:core-classes/queue-storage-is-a-table`: the queue's schema is one value, and no
    /// dialect holds a list of its own for a column to be added to.
    ///
    /// **Asked as sameness rather than as absence.** A second home for what the queue's columns are
    /// is not a constant a test could look for; it is a dialect answering the question differently.
    /// So the assertion is that every dialect declares the same columns in the value's own order —
    /// four emissions of one value cannot disagree, and two hand-written lists eventually always
    /// do. It is also what makes SQL Server and SQLite first-class here rather than the backends
    /// whose list nobody wrote.
    #[test]
    fn the_queues_schema_is_one_value_and_no_dialect_list() {
        let schema = super::schema();
        for (table, name) in schema.tables().iter().zip([JOBS_TABLE, DEAD_TABLE]) {
            let columns: Vec<String> = table
                .columns()
                .iter()
                .map(|column| column.name().to_string())
                .collect();
            for driver in nvs_db::Driver::ALL.iter().copied() {
                let ddl = table_ddl(driver, name);
                // Walked from where the last column was found, so this is the value's *order* and
                // not its set: a dialect that declared the same columns in another order would be
                // a second answer to the same question, which is the thing being refused.
                let mut at = 0;
                for column in &columns {
                    let found = ddl[at..].find(&format!("{column} ")).unwrap_or_else(|| {
                        panic!(
                            "{}'s `{name}` does not declare `{column}` where the one schema value \
                             puts it",
                            driver.display_name()
                        )
                    });
                    at += found + column.len();
                }
            }
        }
    }

    /// [`migration`]'s label is total, and it is only total because every statement the emitter
    /// writes names one of § 2's two tables.
    ///
    /// The label is what `nvs queue migrate` prints and what an operator reads a refusal against, so
    /// a statement filed under the wrong table names the wrong half of the schema. Asked of every
    /// driver, because how many statements a table takes is the dialect's own business: MySQL
    /// declares an index inside the `CREATE TABLE` and the other three write one of their own.
    #[test]
    fn every_statement_of_the_schema_names_one_of_the_two_tables() {
        for driver in nvs_db::Driver::ALL.iter().copied() {
            let steps = migration(driver);
            assert!(
                !steps.is_empty(),
                "{} emits no statement for a schema that has two tables",
                driver.display_name()
            );
            for step in &steps {
                let dead = step.sql.contains(DEAD_TABLE);
                assert!(
                    dead || step.sql.contains(JOBS_TABLE),
                    "{}: `{}` names neither of § 2's two tables",
                    driver.display_name(),
                    step.sql
                );
                assert_eq!(
                    step.label,
                    if dead { "dead_letter" } else { "jobs" },
                    "{}: a statement is labelled for the table it builds",
                    driver.display_name()
                );
            }
            assert_eq!(
                steps.first().map(|step| step.label),
                Some("jobs"),
                "{}: the jobs table is built first, which is the schema value's own order",
                driver.display_name()
            );
        }
    }
}
