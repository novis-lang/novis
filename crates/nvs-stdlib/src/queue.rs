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
//! **A `secret` never reaches the row, and nothing here is what refuses it.** [`payload_of`] encodes
//! `$args` with the encoder `Core\Json::encode` uses, so a durable row a worker process decodes
//! later is `rule:security/secret-sinks-refuse`'s serialiser sink arriving one member further on —
//! and it is refused where the call is written, by
//! `nvs_types::expr::quals::reject_secret_enqueued_argument`, which reports that sink's own code. A
//! declared type could not have done it: `args` is `mixed`, which admits every qualifier there is,
//! and the bag reaches the checker as the shape the registry declared rather than the one the call
//! wrote.
//!
//! **What it spends:** one statement per member call, on a connection the request either already
//! held or now holds for the rest of it, plus one JSON encoding of `$args` sized by the payload the
//! caller wrote. Nothing is held between calls, except the four counters `stats` answers with for
//! as long as its caller keeps the record.
//!
//! # Known gaps
//!
//! 1. **`limits` and `grants` stay undeclared until an isolate enforces them**, and that is a
//!    narrower gap than the spelling it used to be. **The spelling is settled.** An option's type is
//!    never a [`CoreTy::Shape`] — `rule:core-api/shape-parameter`, held over every registered row by
//!    `a_shape_is_only_ever_a_whole_parameter` — and `rule:concurrency/queue-four-members` puts both
//!    of these inside the one trailing bag, so neither is ever the whole parameter `Core\Db::open`'s
//!    settings literal is; that is the answer to "may an option carry a `{…}`", and it is no. What is
//!    left is two ordinary options: `grants` a list of capability names in the spelling `nvs.toml`
//!    grants them under, and `limits` its sub-caps one option each, which is what ADR 0084 § 1's
//!    `{…}` was standing in for.
//!
//!    **What they wait on is enforcement.** A job runs as a root isolate
//!    (`rule:concurrency/a-job-runs-as-a-root-isolate`) and the isolate half applies neither: the
//!    spawn's own `limits:` and `grants:` are checked where they are written
//!    (`nvs_types::expr::isolate`) and no sub-cap or narrowing reaches a child, so a row recording
//!    either would be
//!    `rule:concurrency/an-upgrades-options-are-spawn-scripts`'s accepted-and-dropped narrowing,
//!    which hands the job the authority its request meant to give up. Undeclared *is* the refusal
//!    here: a bag reports a key it does not declare
//!    (`rule:core-api/shape-reuses-the-option-diagnostics`), so `{grants: …}` is a diagnostic today
//!    and stays one until the narrowing
//!    `rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` asks for is applied to the
//!    isolate the worker starts.
//!    — owner: m8-db-queue
//! 2. **`key`'s "at most one pending job per key" is enforced by the statement, and by the unique
//!    key only where the schema has been applied.** [`INSERT_POSTGRES`]'s `existing` arm reads the table
//!    inside the same statement that writes it, which is correct against every other `push` on a
//!    *serialized* transaction and racy against a concurrent one at `read committed`. [`schema`]
//!    carries the constraint under the name `nvs_jobs_dedupe`, a plain unique key over the
//!    `dedupe_pending` column every statement here maintains, and the statement is race-free
//!    against a schema carrying it without changing shape — so what is left of this gap is a
//!    deployment that never ran `nvs queue migrate`, which is the one case the key is absent in.
//!    Decided: Refuse to serve a queue whose schema is behind, checked at boot — Never racy and costs
//!    no request time; a deployment that skipped migrate fails to start.
//!    — owner: unowned-closures
//! 3. **`stats` counts the four things § 6 names and no fifth**, and a fifth would be a column in
//!    § 2's schema before it is a member here. The sharp edge is a dead-lettered job's own
//!    attempts: § 6 *moves* that row to [`DEAD_TABLE`], whose columns this module deliberately does
//!    not decide beyond `id` and `queue`, so [`COUNTS_POSTGRES`] sums `attempts` over [`JOBS_TABLE`] alone
//!    and counts the depth separately rather than inventing a column for the sum to reach.
//!    Decided: Yes: add the column through the `nvs queue migrate` converge and a fifth stats counter —
//!    More observability; a schema change and a spec § 6 amendment.
//!    — owner: unowned-closures

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc, ErrorDoc,
    MethodDoc, ParamDoc, Qual,
};

/// `Core\Queue`'s fully-qualified name.
pub(crate) const NAME: &str = r"Core\Queue";

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

/// `delete`'s, which a capability denial names as well as a refusal from the server does.
const DELETE_OF: &str = r"Core\Queue::delete";

/// `purge`'s, which names the same denial for the queue the call itself wrote.
const PURGE_OF: &str = r"Core\Queue::purge";

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
/// One number rather than one per column, because two of the columns that carry it hold the *same
/// value*:
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
/// nothing on any of the five: four read a unique key's nulls as distinct outright, and SQL Server
/// reads two nulls as equal but never sees this key as a constraint at all —
/// `rule:core-classes/a-unique-key-reads-nulls-as-distinct` is the filtered index `nvs_db::ddl`
/// writes in the constraint's place there. So the guarantee gap 3 names is the same one on every
/// backend `Core\Queue` runs a statement against, and it is one sentence rather than four.
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
/// **`tag` is a second column and never a second meaning for `dedupe_key`.**
/// `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them` is why: a key admits at most one
/// pending job under a value and a tag names many, so folding the two would cap a batch at one job
/// at the enqueue that created it. It has no `_pending` twin — nothing releases a tag, because a
/// tag is not a lock — and nothing on the request path reads it. `nvs_jobs_tag` is there for
/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `purge` alone, which is what makes
/// the pair one column and one index rather than a feature.
///
/// **A live queue converges onto it by one `Safe` step per table and one `Locking` index build.**
/// A nullable column with no default is a catalog write on all four dialects, which is the whole
/// reason `tag` is declared that way; the index over `(queue, tag)` is built over every row that is
/// already there, and `rule:core-classes/schema-plan` grades a build no v1 emitter runs
/// concurrently as `Locking`. So `nvs queue migrate` takes the columns unasked and the index with
/// `--including-risky`, and a deployment that is not ready for the build has the column regardless.
///
/// **What it spends:** two indexed [`KEY_WIDTH`]-wide columns per job row. `dedupe_pending` is what
/// a partial index costs nothing for — priority 5 spent to buy one spelling everywhere the queue
/// runs instead of two spellings on two backends — and `tag` is one more, written once by `push`
/// and read by no statement a request or a worker runs.
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
    // Indexed text is bounded and payload text is not: `queue`, `tag` and the two dedupe columns
    // are read by a key, while `script`, `args` and `errors` are a path and two JSON documents that
    // no index ever covers.
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
            // Beside `dedupe_key` and ahead of it, so the pair the enqueue writes one value into
            // stays adjacent: a tag is `dedupe_key`'s kind of column — written once by `push` and
            // read by a filter — and has no `_pending` twin, because nothing releases it.
            named("tag", short()).null(),
            named("dedupe_key", short()).null(),
            named("dedupe_pending", short()).null(),
            named("created_at", big()),
            named("claimed_at", big()).null(),
        ],
    )
    .and_then(|table| table.primary_key(&["id"]))
    .and_then(|table| table.unique("nvs_jobs_dedupe", &["dedupe_pending"]))
    .and_then(|table| table.index("nvs_jobs_due", &["queue", "state", "run_at"]))
    .and_then(|table| table.index("nvs_jobs_tag", &["queue", "tag"]))
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
            named("tag", short()).null(),
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
/// no parameter at all; a repeated bound slot in [`INSERT_MYSQL`]'s array is what it costs the
/// other one.
///
/// **`tag` is bound last and written to one column**, because it is the newest of the job's own
/// values and `crates/nvs-cli/src/worker.rs` reads the claim's columns by position — a value added
/// anywhere but the end of a list here is a column list a session has to check against every other
/// statement. It has no second column for the reason
/// `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them` gives: nothing releases a tag.
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
      dedupe_pending, created_at, tag) \
     select $2::text, $3::text, $4::text, $5::smallint, 0, $6::int, $7::bigint, $8::bigint, \
            $1::text, $1::text, $9::bigint, $10::text \
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
     returning id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, tag, \
     dedupe_key, created_at\
 ) insert into nvs_dead_jobs \
 (id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, tag, dedupe_key, \
 created_at, failed_at, errors) \
 select id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, tag, dedupe_key, \
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
/// claim, half a push, half a move or half a removal. The transaction is therefore not the caller's
/// convenience: it is what these two texts *mean*, and it is the one thing a second backend cannot
/// omit.
#[derive(Debug)]
pub struct Split {
    /// The statement run first: the one the rest of the pair is decided from.
    ///
    /// **Something has to carry the pair's meaning across the gap between two round trips that a
    /// single statement did not have, and which mechanism that is belongs to the dialect.** Where
    /// the backend has row locks and concurrent writers to need them from, this statement takes
    /// them and § 4's mutual exclusion rests on its `for update skip locked` ([`CLAIM_MYSQL`]).
    /// Where the backend has one writer, the transaction around the pair is already the exclusion
    /// and this statement carries no clause at all ([`CLAIM_SQLITE`]).
    pub first: &'static str,
    /// The statement run second, in the same transaction.
    ///
    /// Keyed by the row [`Self::first`] answered where the pair carries one, and run whatever that
    /// answered where the two texts are one member's two arms rather than a read and a write
    /// ([`DELETE_SQLITE`]). Either way what the transaction buys is the same: the pair is one
    /// moment, and nothing decided in the first statement can have been undone by the time the
    /// second runs.
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
           dedupe_pending, created_at, tag) \
           values (?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?)",
};

/// [`INSERT_MYSQL`] on the backend with one writer, which is that pair with the locking clause
/// removed.
///
/// **The read carries no `for update`, and what replaces it is the transaction itself.** This pair
/// runs inside `nvs_db::sqlite::SqliteConn::begin_immediate`'s transaction — [`CLAIM_SQLITE`] owns
/// why every [`Split`] on this backend does — so the write lock is taken before the `select` runs
/// and no second connection can insert a row carrying this key between the read and the insert.
/// That is what [`INSERT_MYSQL`]'s row lock buys where there are concurrent writers to need one,
/// and it is why a `for update` here would be a syntax error rather than a missing safeguard.
///
/// **The ordering is what makes it safe, and that is the immediate transaction's whole point.** A
/// deferred transaction reading first would hold a shared lock and then ask to upgrade it for the
/// insert — the upgrade SQLite refuses without honouring the busy timeout — so this text run
/// outside an immediate transaction fails under exactly the concurrency it exists to survive,
/// rather than merely guaranteeing less.
///
/// Everything else transcribes. The read is keyed on `dedupe_pending` for [`INSERT_MYSQL`]'s
/// reason, the insert names that column as a tenth bound slot because a `?` is bound by position
/// and cannot be named twice, and a deduped push answers the pending job's id from [`Split::first`]
/// with [`Split::then`] never running at all.
pub const INSERT_SQLITE: Split = Split {
    first: "select id from nvs_jobs where dedupe_pending = ? limit 1",
    then: "insert into nvs_jobs \
           (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
           dedupe_pending, created_at, tag) \
           values (?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?)",
};

/// The same enqueue in T-SQL, as a [`Split`] for [`INSERT_MYSQL`]'s reason and one of its own.
///
/// **A common table expression cannot carry a data-modifying statement here**, so
/// [`INSERT_POSTGRES`]'s single text has no transcription at all: the dedupe read and the insert are
/// two statements in one transaction, exactly as the second dialect runs them, and the transaction
/// is what makes them one moment.
///
/// **`updlock, holdlock` is what `for update` is over a row that does not exist yet.** The dedupe
/// read is a read of an *absence* — nothing is pending under this key — and an ordinary lock has
/// nothing to take, so a second enqueue of the same key would pass the same read and reach the same
/// insert. `holdlock` is serializable range locking over `nvs_jobs_dedupe`, which locks the gap the
/// key would occupy, and [`INSERT_MYSQL`]'s doc owns why that cost is paid on a keyed push alone.
///
/// **`output inserted.id` rather than a second read**, because this driver's answer to
/// "what id did that insert take" is a row the insert itself returns: `scope_identity()` is another
/// statement on a connection this member is already holding inside a transaction, and an identity
/// read that is a statement apart from its insert is a thing a session has to keep right forever.
/// Every bound value is declared `nvarchar` (`nvs_db::tds`'s § 5), so each one the column table
/// types as a number is cast where it is written — the same job [`INSERT_POSTGRES`]'s `::` casts do.
pub const INSERT_SQLSERVER: Split = Split {
    first: "select top 1 id from nvs_jobs with (updlock, holdlock) where dedupe_pending = @p1",
    then: "insert into nvs_jobs \
           (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, dedupe_key, \
           dedupe_pending, created_at, tag) \
           output inserted.id \
           values (@p1, @p2, @p3, cast(@p4 as smallint), 0, cast(@p5 as int), \
           cast(@p6 as bigint), cast(@p7 as bigint), @p8, @p9, cast(@p10 as bigint), @p11)",
};

/// [`CLAIM_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged.
///
/// **The `select` answers [`CLAIM_POSTGRES`]'s `returning` list, in its order**, so a worker reads
/// the same column at the same ordinal whichever dialect it claimed with — the six slots
/// `crates/nvs-cli/src/worker.rs` names by position, and
/// `all_three_dialects_answer_a_claim_with_the_same_columns` is what holds them together.
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

/// [`CLAIM_MYSQL`]'s two statements in SQLite's dialect, and it is
/// `rule:concurrency/claiming-is-one-statement`'s immediate transaction that makes them a claim.
///
/// The columns are [`CLAIM_POSTGRES`]'s `returning` list in its order, `attempts + 1 as attempts`
/// included and for [`CLAIM_MYSQL`]'s reason — the `update` has not run when the `select` answers,
/// so the column is read as the value it is about to have.
/// `all_three_dialects_answer_a_claim_with_the_same_columns` is what holds all three lists together.
///
/// **There is no locking clause, and the mutual exclusion is stronger rather than weaker.** SQLite
/// has one writer: inside a `nvs_db::sqlite::SqliteConn::begin_immediate` transaction no second
/// connection is writing at all, so two workers cannot come back with one row — which is what
/// `for update skip locked` buys on a backend that has row locks and concurrent writers to need
/// them from. A `skip locked` here would be a syntax error, and the reason not to reach for one is
/// that there is nothing left for it to do. `crates/nvs-db/src/sqlite.rs`'s `begin_immediate` owns
/// why the transaction has to be an immediate one: a deferred transaction that reads and then
/// writes asks to upgrade a shared lock, and SQLite refuses an upgrade without honouring the busy
/// timeout, so the pair below would fail under exactly the concurrency it exists to survive.
///
/// The `update` is keyed by `id` for [`CLAIM_MYSQL`]'s reason as well: [`Split::first`] has named
/// the row, and there is no CTE to reach back into.
pub const CLAIM_SQLITE: Split = Split {
    first: "select id, script, args, attempts + 1 as attempts, max_attempts, backoff_ms \
            from nvs_jobs \
            where queue = ? \
            and ((state = 0 and run_at <= ?) or (state = 1 and claimed_at <= ?)) \
            order by run_at, id limit 1",
    then: "update nvs_jobs set state = 1, attempts = attempts + 1, claimed_at = ?, \
           dedupe_pending = null where id = ?",
};

/// `rule:concurrency/claiming-is-one-statement`'s claim in T-SQL, and one statement rather than a
/// [`Split`] — which is the one place this dialect follows [`CLAIM_POSTGRES`] where the enqueue
/// beside it could not.
///
/// **A common table expression is updatable here even though it cannot insert**, so the shape
/// [`CLAIM_POSTGRES`] takes survives the crossing: the reader picks the one due row and the update
/// marks it, in a single statement the server cannot be interrupted inside. What crosses with it is
/// the ordering — `order by run_at, id` needs a `top` to be legal inside a CTE, and one row is what
/// a claim takes anyway.
///
/// **`updlock, readpast, rowlock` is `for update skip locked` in this dialect's spelling**, and the
/// three hints are one decision: `updlock` takes the lock the update is about to need, `readpast`
/// steps over a row another worker already holds rather than queueing behind it, and `rowlock`
/// keeps the engine from escalating to a page and stepping over rows nobody holds. Without
/// `readpast` every worker in a fleet serialises on the oldest due row, which is the failure
/// `rule:concurrency/claiming-is-one-statement` exists to refuse.
///
/// **`output inserted.<column>` answers the list a `returning` does**, and the columns are the ones
/// `crates/nvs-cli/src/worker.rs` reads by position — `inserted` is the row *after* the update, so
/// `attempts` is the incremented count, which is what the other dialects answer too. The CTE names
/// every column the update writes or the output reads, because `inserted` over an updated CTE
/// carries that CTE's columns and no others.
pub const CLAIM_SQLSERVER: &str = "with due as (\
     select top 1 id, script, args, state, attempts, max_attempts, backoff_ms, run_at, claimed_at, \
     dedupe_pending from nvs_jobs with (updlock, readpast, rowlock) \
     where queue = @p1 \
     and ((state = 0 and run_at <= cast(@p2 as bigint)) \
     or (state = 1 and claimed_at <= cast(@p3 as bigint))) \
     order by run_at, id\
 ) update due set state = 1, attempts = attempts + 1, claimed_at = cast(@p2 as bigint), \
   dedupe_pending = null \
   output inserted.id, inserted.script, inserted.args, inserted.attempts, \
   inserted.max_attempts, inserted.backoff_ms";

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
            (id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, tag, \
            dedupe_key, created_at, failed_at, errors) \
            select id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, tag, \
            dedupe_key, created_at, ?, ? from nvs_jobs \
            where id = ? and claimed_at = ?",
    then: "delete from nvs_jobs where id = ? and claimed_at = ?",
};

/// [`DEAD_LETTER_MYSQL`]'s pair, which SQLite runs unchanged.
///
/// **The text is that constant rather than a copy of it, and this is the first of four.** Two
/// literals that must stay identical are two chances to drift, and the module already answers that
/// the same way for MariaDB: what a backend runs unchanged it is given, not sent a transcription
/// of. What differs between these two backends here is the send path and not the SQL — a [`Split`]
/// is bound as owned values on this driver and as encoded wire bytes on that one — and the type of
/// a binding is not a dialect. The day a change belongs to one of them alone is the day this
/// becomes its own literal, and naming the alias is what makes that day's edit a visible one.
///
/// The copy runs before the delete and both halves are keyed on the lease, for
/// [`DEAD_LETTER_MYSQL`]'s reasons. What holds the two as one moment here is the immediate
/// transaction every [`Split`] on this backend runs inside ([`CLAIM_SQLITE`]).
pub const DEAD_LETTER_SQLITE: Split = DEAD_LETTER_MYSQL;

/// `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s move in T-SQL, as the pair the
/// second dialect runs rather than the `delete … output … into` this one could spell.
///
/// **The copy is written from the table it is read from**, which is the whole of why the pair is
/// safe to split: both halves are keyed on the lease as well as on the id, so a row another worker
/// has since re-claimed matches neither, and the transaction around them is what makes the insert
/// and the delete one moment. A `delete … output deleted.* into nvs_dead_jobs` would be one
/// statement and is refused for a plainer reason than elegance: the two columns the dead-letter row
/// adds are not columns of the row being deleted, so the shape that reads as one statement is one
/// statement with two of its values smuggled through an `output` expression list, and the pair
/// [`DEAD_LETTER_MYSQL`] already proves is the same work said once.
pub const DEAD_LETTER_SQLSERVER: Split = Split {
    first: "insert into nvs_dead_jobs \
            (id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, tag, \
            dedupe_key, created_at, failed_at, errors) \
            select id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, tag, \
            dedupe_key, created_at, cast(@p1 as bigint), @p2 from nvs_jobs \
            where id = cast(@p3 as bigint) and claimed_at = cast(@p4 as bigint)",
    then: "delete from nvs_jobs where id = cast(@p1 as bigint) \
           and claimed_at = cast(@p2 as bigint)",
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

/// [`QUEUES_MYSQL`], which SQLite runs unchanged — one `select distinct` over one table with two
/// arms, in a placeholder spelling both backends share. [`DEAD_LETTER_SQLITE`] owns why an alias
/// and not a copy.
pub const QUEUES_SQLITE: &str = QUEUES_MYSQL;

/// The same roster in T-SQL: [`QUEUES_POSTGRES`] with this dialect's markers, and a cast at each of
/// them because a bound value arrives declared `nvarchar` and both columns it is compared against
/// are `bigint`.
pub const QUEUES_SQLSERVER: &str = "select distinct queue from nvs_jobs \
    where (state = 0 and run_at <= cast(@p1 as bigint)) \
    or (state = 1 and claimed_at <= cast(@p2 as bigint))";

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

/// [`SUCCEEDED_MYSQL`], which SQLite runs unchanged — one keyed `update` and no construct behind
/// it, so the lease keying survives with the text. [`DEAD_LETTER_SQLITE`] owns why an alias and not
/// a copy.
///
/// It is one statement, so it needs no transaction of its own: what stage 2's rule requires a
/// transaction for is a pair.
pub const SUCCEEDED_SQLITE: &str = SUCCEEDED_MYSQL;

/// The same write-back in T-SQL, keyed on the lease as every dialect's is: a worker that lost its
/// row to the visibility timeout writes nothing, because `claimed_at` moved under it.
pub const SUCCEEDED_SQLSERVER: &str = "update nvs_jobs set state = 2, claimed_at = null, \
    dedupe_pending = null \
    where id = cast(@p1 as bigint) and claimed_at = cast(@p2 as bigint)";

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

/// [`RETRY_MYSQL`], which SQLite runs unchanged — including its binding order, `run_at`, `id`,
/// `claimed_at`, which the `set` clause standing left of the `where` forces in any dialect binding
/// by position. [`DEAD_LETTER_SQLITE`] owns why an alias and not a copy.
///
/// The `0` is `Core\Queue\State::Pending`'s ordinal here as well, and it is the same literal, so
/// `queue_statements_agree_with_the_state_enum` holds it against the enum without a third list to
/// read.
pub const RETRY_SQLITE: &str = RETRY_MYSQL;

/// The same re-arming in T-SQL, and it takes [`RETRY_POSTGRES`]'s numbering rather than
/// [`RETRY_MYSQL`]'s: a marker here carries its own number, so the values go out in the order the
/// keyed dialect sends them and the new `run_at` is the third of them rather than the first.
pub const RETRY_SQLSERVER: &str = "update nvs_jobs set state = 0, run_at = cast(@p3 as bigint), \
    claimed_at = null, dedupe_pending = dedupe_key \
    where id = cast(@p1 as bigint) and claimed_at = cast(@p2 as bigint)";

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

/// [`STATUS_MYSQL`], which SQLite runs unchanged — a `union all` of two `select`s under one `limit`,
/// in a placeholder spelling both backends share. [`DEAD_LETTER_SQLITE`] owns why an alias and not a
/// copy.
///
/// **Its four placeholders stay four**, because a `?` is bound by the position it occupies here as
/// well: the id and the queue each go out twice, and [`counted_row`] doubles that pair once for every
/// backend that spells a parameter this way. The `limit` binds to the compound rather than to its
/// second arm, which is what makes the member's one answer one row — § 6 moves a job, so the two
/// arms never hold it at the same moment anyway.
pub const STATUS_SQLITE: &str = STATUS_MYSQL;

/// The same reader in T-SQL, with the bound of the union outside it.
///
/// `select top 1 … union all …` bounds the first arm alone in this dialect, where `limit` after the
/// last arm bounds the whole of it in the other two, so the union is a derived table and the `top`
/// is over that. The arm over the dead-letter table answers the `Dead` ordinal as a literal for
/// [`STATUS_POSTGRES`]'s reason: a row is dead by being in that table, and the union's first arm is
/// what names the column both arms are read as.
pub const STATUS_SQLSERVER: &str = "select top 1 state from (\
     select state from nvs_jobs \
     where id = cast(@p1 as bigint) and queue = @p2 \
     union all \
     select 3 from nvs_dead_jobs \
     where id = cast(@p1 as bigint) and queue = @p2\
 ) as found";

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

/// [`CANCEL_MYSQL`], which SQLite runs unchanged — one conditional `update` whose `and state = 0`
/// carries the whole of the member's semantics. [`DEAD_LETTER_SQLITE`] owns why an alias and not a
/// copy.
///
/// **The answer is the affected count here as well, and that is what makes the alias whole rather
/// than a text that merely parses.** [`Counted::touched`] reads a changed count and a returned row as
/// one fact, so a member on this backend answers the number this text already produces. A `returning
/// id` of its own would be a third literal saying what [`Counted`] can already read, kept in step
/// with the two beside it for nothing.
///
/// It is one statement, so it needs no transaction of its own: the reading is the `where`, which is
/// [`CANCEL_POSTGRES`]'s reason unchanged rather than a property of this backend.
pub const CANCEL_SQLITE: &str = CANCEL_MYSQL;

/// The same cancel in T-SQL, decided by the statement and read off the affected count — this driver
/// has an `output` clause to answer with and does not use it, because what the member owes is
/// whether one row moved and every dialect but the first already answers that with a count.
pub const CANCEL_SQLSERVER: &str = "update nvs_jobs set state = 4, dedupe_pending = null \
    where id = cast(@p1 as bigint) and queue = @p2 and state = 0";

/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `delete`, as one statement over both
/// of § 2's tables.
///
/// **`and state <> 1` is the member's whole refusal, and it is in the `where` rather than in a check
/// above it.** A claimed job is not removable at all — a worker holds a lease on that row and there
/// is no protocol for interrupting work in flight — and reading the state and then deleting would be
/// two moments with a claim free to land in between, so the answer would be about the first one.
/// Written this way the database decides, one row comes back or none does, and that is what the
/// member's `bool` means. The `1` is [`STATE`]'s `Claimed` ordinal, a literal for [`PENDING`]'s
/// reason and held to the enum by `queue_statements_agree_with_the_state_enum`.
///
/// **`<>` and not a list of the states that may go**, because the states that may go are every one
/// but that one: a caller naming the receipt removes a job that is pending, succeeded or cancelled
/// alike. A positive list would have to be extended by every case [`STATE`] ever grows, and it would
/// fail silently — a row in a state the list had forgotten answers `false`, which reads as a job
/// something else had already removed.
///
/// **Both tables in one statement, as [`STATUS_POSTGRES`] reads both**, because a `Queue\Id` names a
/// job across the move § 6 makes and a member that stopped working the moment a job exhausted its
/// attempts would be a receipt that expires without saying so. Running the two arms together costs
/// nothing: § 6 *moves* a row, so an id is in one of the tables and never in both, and a
/// data-modifying CTE runs whatever the outer `select` reads — which is what makes this one moment
/// rather than an attempt and a second attempt.
///
/// **The dead-letter arm names no state**, and not because a dead job is exempt from the rule above:
/// [`DEAD_TABLE`] has no `state` column at all, since being in that table is what `Dead` *is*. That
/// is the same reading [`STATUS_POSTGRES`] makes when it answers the ordinal there as a literal.
///
/// **Public for the reason the statements a worker sends are**: nothing outside this module runs a
/// `delete`, and `crates/nvs-stdlib/tests/queue.rs` sends both spellings to a real server, where a
/// statement no server has ever parsed is exactly what that target exists to catch.
pub const DELETE_POSTGRES: &str = "with gone as (\
     delete from nvs_jobs \
     where id = $1::bigint and queue = $2::text and state <> 1 \
     returning id\
 ), buried as (\
     delete from nvs_dead_jobs \
     where id = $1::bigint and queue = $2::text \
     returning id\
 ) select id from gone union all select id from buried limit 1";

/// [`DELETE_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged for [`INSERT_MYSQL`]'s
/// reason.
///
/// **A multi-table delete and not a [`Split`]**, which is this dialect's answer to the
/// data-modifying CTE it does not have: `delete j, d from …` names both of § 2's tables as targets
/// of one statement, so the two arms are decided in one moment exactly as
/// [`DELETE_POSTGRES`]'s are. A pair of ordinary deletes would be two moments needing a transaction
/// to mean what one statement means by itself, and the second of them could remove a job that the
/// first had just refused as claimed — § 6 moves an exhausted job while a caller stands between the
/// two round trips, and the one row this member exists to leave alone is gone.
///
/// **The receipt arrives as a one-row derived table, and that is what makes the two arms
/// independent.** A multi-table delete removes out of whatever its join produced, so driving it from
/// `nvs_jobs` would answer nothing at all for a job § 6 had already moved: there would be no left
/// side to hang the dead-letter row on. `r` is a row that exists whatever the tables hold, and the
/// two `left join`s attach whichever one holds the receipt.
///
/// **The state predicate is on the join and not in a `where`**, which is the same refusal put where
/// this shape has room for it. A trailing `where j.state <> 1` is read after the join and drops the
/// driving row for a claimed job, taking the dead-letter arm down with it; on the `on` clause it
/// fails to attach `j` alone and `d` is still judged on its own terms.
///
/// **Two placeholders where [`STATUS_MYSQL`] needs four**, and `r` is why. A `?` is a position that
/// cannot be named twice, so every other framed statement here binds a value once per mention;
/// naming the pair once in the derived table and reading `r.jid` afterwards is what a `$1` does on
/// the other dialect.
pub const DELETE_MYSQL: &str = "delete j, d \
    from (select ? as jid, ? as qname) r \
    left join nvs_jobs j on j.id = r.jid and j.queue = r.qname and j.state <> 1 \
    left join nvs_dead_jobs d on d.id = r.jid and d.queue = r.qname";

/// [`DELETE_POSTGRES`] on SQLite, which is the one member here with no single-statement shape: two
/// deletes, one per table, inside stage 2's transaction.
///
/// **Neither other dialect's construct exists on this backend.** [`DELETE_POSTGRES`] is a
/// data-modifying CTE, which SQLite has no spelling for at all, and [`DELETE_MYSQL`] is a
/// multi-table `delete j, d`, which it has no spelling for either. What is left is one `delete` per
/// table, and a pair is a [`Split`] — the type exists for exactly this, and its doc owns why the
/// transaction is what the two texts *mean* rather than a caller's convenience.
///
/// **This is the one [`Split`] whose second statement is not keyed on the first's row.** A claim, a
/// push and a dead-letter move each read something the next statement then acts on; here both texts
/// are the member's own arms, so [`Split::then`] runs whatever [`Split::first`] answered and the
/// member's `bool` is either arm having removed a row. At most one of them can: § 6 *moves* a job,
/// so an id is in one of § 2's tables and never in both, which is the same fact
/// [`DELETE_POSTGRES`]'s `union all` rests on.
///
/// **The transaction is load-bearing even so, and § 6 is what makes it so.** The first arm refuses a
/// claimed job; the dead-letter table has no state to refuse anything by. Run as two moments, a
/// worker that exhausts that job's attempts between them moves the row it just refused into the
/// second arm's reach, and the one row this member exists to leave alone is removed by the call that
/// had already declined to remove it. That is [`DELETE_MYSQL`]'s argument for a multi-table delete,
/// reaching the same property with the mechanism this backend does have.
///
/// **The answer is the two affected counts and not a `returning`**, for [`CANCEL_SQLITE`]'s reason:
/// [`Counted::touched`] reads a count as the same fact a returned row states, and a text that
/// answered a row would be a spelling the other two dialects' texts do not need.
///
/// `state <> 1` is [`STATE`]'s `Claimed` ordinal, a literal for [`PENDING`]'s reason and held to the
/// enum by `queue_statements_agree_with_the_state_enum`; the dead-letter arm names no state for
/// [`PURGE_DEAD_POSTGRES`]'s reason. Each arm binds its own receipt, so the pair takes the id and
/// the queue twice — two statements, two positions each, which is what a dialect binding by position
/// costs a member that reads both tables.
pub const DELETE_SQLITE: Split = Split {
    first: "delete from nvs_jobs where id = ? and queue = ? and state <> 1",
    then: "delete from nvs_dead_jobs where id = ? and queue = ?",
};

/// The same receipt-shaped delete in T-SQL, as the pair [`DELETE_SQLITE`] is: a CTE cannot delete
/// here, and the multi-table `delete j, d` [`DELETE_MYSQL`] spells is MySQL's alone, so what is
/// left is one statement per table inside one transaction. Both halves bind the same two values,
/// and the arm over the dead-letter table names no ordinal at all for [`DELETE_SQLITE`]'s reason.
pub const DELETE_SQLSERVER: Split = Split {
    first: "delete from nvs_jobs where id = cast(@p1 as bigint) and queue = @p2 and state <> 1",
    then: "delete from nvs_dead_jobs where id = cast(@p1 as bigint) and queue = @p2",
};

/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `purge` over [`JOBS_TABLE`]: the
/// state set the call selected, narrowed by an optional tag and an optional age, and never more rows
/// than the call's `limit`.
///
/// **The bound is the rule and this spelling is not.** ADR 0153 § 4 is why there is a `limit` at
/// all — the first `purge` a deployment runs is against the table that has been growing since it was
/// deployed, and an unbounded `delete` there holds a lock on the connection the application enqueues
/// through for as long as it takes. PostgreSQL has no `delete … limit`, so the bound is carried by a
/// subquery that selects the ids and the outer statement removes those; [`PURGE_MYSQL`] writes the
/// same bound as the `limit` its own dialect takes on a single-table delete. Which shape a dialect
/// takes is a session's choice; dropping the bound is the one answer that is not available.
///
/// **`order by id` and not `order by created_at`**, though the age is what the caller thinks in.
/// `id` is [`schema`]'s identity column, so rows are numbered in the order they were enqueued and
/// that is `created_at`'s order without a sort — over the one table in the runtime that grows
/// without bound, a sort is the whole cost of the call. Ordering at all is what makes § 4's
/// `while (purge(…) > 0) {}` drain from the oldest end rather than from wherever the scan happened
/// to start.
///
/// **The default set is two literals here rather than a set the member binds.** A `purge` naming no
/// state removes what has finished — `Succeeded` and `Cancelled` — and writing that as `state in (2,
/// 4)` makes it a property of the statement rather than of the argument handling in front of it.
/// `Dead` and `Pending` are opt-in for the reasons the rule gives, and a call naming either binds
/// `$2` instead; [`PURGE_DEAD_POSTGRES`] is where `Dead` goes, since that selection is a different
/// table. The ordinals are literals for [`PENDING`]'s reason and are held to [`STATE`] by
/// `queue_statements_agree_with_the_state_enum`.
///
/// **`state <> 1` is carried here as well as in front of it.** `purge` refuses `State::Claimed` at
/// the call, which is where a caller finds out; the text refusing it too is what keeps *a claimed
/// job is not removable* a property of the system rather than of one layer of it, and it is the same
/// reading [`DELETE_POSTGRES`] makes. A `limit` spent on rows the statement then declines would be
/// the alternative, and it is worse in the only case that matters.
///
/// **`before` reads `created_at`, which is the job's age.** It is the one instant every row in
/// either of § 2's tables carries and nothing afterwards rewrites: `run_at` moves with § 6's retry
/// ladder, so a sweep keyed on it would remove a row for a reason the caller never named, and
/// `failed_at` is on [`DEAD_TABLE`] alone, so `before` would ask a different question depending on
/// which of the two statements ran.
///
/// **`limit` is the one option with no null arm.** § 4's default is a number rather than an absence,
/// so the member always binds one and there is nothing here to branch on; the other two are absent
/// far more often than they are named, and `$n is null or …` is how a single text serves both
/// without a second spelling for § 1's statement cache to hold.
///
/// **Public for the reason the statements a worker sends are**: nothing outside this module runs a
/// `purge`, and `crates/nvs-stdlib/tests/queue.rs` sends both spellings to a real server, where a
/// statement no server has ever parsed is exactly what that target exists to catch.
pub const PURGE_POSTGRES: &str = "delete from nvs_jobs where id in (\
     select id from nvs_jobs \
     where queue = $1::text and state <> 1 \
     and (($2::smallint is null and state in (2, 4)) or state = $2::smallint) \
     and ($3::text is null or tag = $3::text) \
     and ($4::bigint is null or created_at < $4::bigint) \
     order by id limit $5::bigint\
 )";

/// [`PURGE_POSTGRES`]'s `state: Dead` selection, which is a different table and therefore a
/// different statement.
///
/// **It names no state**, and not because a dead job is exempt from the selection: [`DEAD_TABLE`]
/// has no `state` column at all, since being in that table is what `Dead` *is*. That is the same
/// reading [`DELETE_POSTGRES`]'s dead-letter arm and [`STATUS_POSTGRES`]'s literal `3` make.
///
/// **The tag and the age are asked of this table in the same words**, because § 6 moves a row
/// carrying both columns: a group tagged at enqueue is still that group after the job exhausted its
/// attempts, which is what makes a purge of a batch's dead rows expressible at all.
pub const PURGE_DEAD_POSTGRES: &str = "delete from nvs_dead_jobs where id in (\
     select id from nvs_dead_jobs \
     where queue = $1::text \
     and ($2::text is null or tag = $2::text) \
     and ($3::bigint is null or created_at < $3::bigint) \
     order by id limit $4::bigint\
 )";

/// [`PURGE_POSTGRES`] in MySQL's dialect, which MariaDB runs unchanged for [`INSERT_MYSQL`]'s
/// reason.
///
/// **The bound needs no subquery here**, because a single-table `delete` in this dialect takes
/// `order by … limit` itself. That is the same bound and not a weaker one, and it is why neither of
/// these is a [`Split`]: one statement removes what it selected, so a caller's loop counts rows the
/// server actually removed rather than rows something else had listed a round trip earlier.
///
/// **Eight placeholders where [`PURGE_POSTGRES`] binds five**, for [`STATUS_MYSQL`]'s reason: a `$n`
/// may be named as often as a statement likes and a `?` is a position that cannot, so the two
/// null-checked options cost two slots each. The order is the order they are read in — queue, the
/// state twice, the tag twice, the age twice, then the bound.
pub const PURGE_MYSQL: &str = "delete from nvs_jobs \
    where queue = ? and state <> 1 \
    and ((? is null and state in (2, 4)) or state = ?) \
    and (? is null or tag = ?) \
    and (? is null or created_at < ?) \
    order by id limit ?";

/// [`PURGE_DEAD_POSTGRES`] in MySQL's dialect, carrying [`PURGE_MYSQL`]'s bound and that constant's
/// placeholder arithmetic — six slots for the four [`PURGE_DEAD_POSTGRES`] binds.
pub const PURGE_DEAD_MYSQL: &str = "delete from nvs_dead_jobs \
    where queue = ? \
    and (? is null or tag = ?) \
    and (? is null or created_at < ?) \
    order by id limit ?";

/// [`PURGE_POSTGRES`]'s shape with [`PURGE_MYSQL`]'s placeholders, which is what SQLite runs: the
/// bound carried by a subquery, and the binds in the order a dialect naming positions reads them.
///
/// **The bound is the rule and this spelling is not, and here the spelling is forced.** `delete …
/// order by … limit` — the shape [`PURGE_MYSQL`] takes — parses on SQLite only when the library was
/// compiled with `SQLITE_ENABLE_UPDATE_DELETE_LIMIT`, which `libsqlite3-sys`'s bundled build does
/// not set. A statement that parses only under a non-default build of a dependency is a statement
/// that stops parsing the day the dependency moves, so the bound is carried the way
/// [`PURGE_POSTGRES`] carries it, in a subquery that selects the ids the outer statement removes.
/// Dropping the bound is the one answer that is not available: ADR 0153 § 4 is why there is a
/// `limit` at all.
///
/// **Eight placeholders in [`PURGE_MYSQL`]'s order**, so the value array a caller builds for that
/// dialect is the one this text takes as well — the two null-checked options cost two slots each for
/// [`STATUS_MYSQL`]'s reason, and the order is queue, the state twice, the tag twice, the age twice,
/// then the bound.
///
/// `order by id` carries over with the subquery for [`PURGE_POSTGRES`]'s reason — it is
/// `created_at`'s order without a sort, over the one table in the runtime that grows without
/// bound — and the default set is that constant's two literals, held to [`STATE`] by
/// `queue_statements_agree_with_the_state_enum`.
pub const PURGE_SQLITE: &str = "delete from nvs_jobs where id in (\
     select id from nvs_jobs \
     where queue = ? and state <> 1 \
     and ((? is null and state in (2, 4)) or state = ?) \
     and (? is null or tag = ?) \
     and (? is null or created_at < ?) \
     order by id limit ?\
 )";

/// The same bounded purge in T-SQL: [`PURGE_POSTGRES`]'s subquery with `top` where that one writes
/// `limit`, which is the only difference the shape has. `top` takes an expression in parentheses,
/// so the bound is still the caller's and still bound rather than pasted.
///
/// The optional filters are one marker each rather than the pair [`PURGE_MYSQL`] needs, because a
/// marker here carries its own number: `@p2 is null` and `state = cast(@p2 as smallint)` are the
/// same value read twice, which is what makes this dialect's bound array
/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s five values rather than nine.
pub const PURGE_SQLSERVER: &str = "delete from nvs_jobs where id in (\
     select top (cast(@p5 as bigint)) id from nvs_jobs \
     where queue = @p1 and state <> 1 \
     and ((@p2 is null and state in (2, 4)) or state = cast(@p2 as smallint)) \
     and (@p3 is null or tag = @p3) \
     and (@p4 is null or created_at < cast(@p4 as bigint)) \
     order by id\
 )";

/// [`PURGE_SQLITE`]'s `state: Dead` selection, which is a different table and therefore a different
/// statement, carrying that constant's bound and [`PURGE_DEAD_MYSQL`]'s six slots for the four
/// values [`PURGE_DEAD_POSTGRES`] binds.
///
/// It names no state for [`PURGE_DEAD_POSTGRES`]'s reason, and asks the tag and the age of this
/// table in the same words for that constant's.
pub const PURGE_DEAD_SQLITE: &str = "delete from nvs_dead_jobs where id in (\
     select id from nvs_dead_jobs \
     where queue = ? \
     and (? is null or tag = ?) \
     and (? is null or created_at < ?) \
     order by id limit ?\
 )";

/// The purge's other table in T-SQL, and what it does not carry is the point: there is no state to
/// filter on, because being in this table is what `Dead` is.
pub const PURGE_DEAD_SQLSERVER: &str = "delete from nvs_dead_jobs where id in (\
     select top (cast(@p4 as bigint)) id from nvs_dead_jobs \
     where queue = @p1 \
     and (@p2 is null or tag = @p2) \
     and (@p3 is null or created_at < cast(@p3 as bigint)) \
     order by id\
 )";

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

/// [`COUNTS_MYSQL`], which SQLite runs unchanged — the same four counters, the same two ordinals,
/// and the same scalar subquery over the dead-letter table. [`DEAD_LETTER_SQLITE`] owns why an alias
/// and not a copy.
///
/// **`count(case when … then 1 end)` is the portable spelling and that is why the alias is MySQL's
/// rather than PostgreSQL's** — what rules [`COUNTS_POSTGRES`]'s text out here is its `::bigint`
/// casts and its `$1`, neither of which is about the aggregate filter. The empty queue answers `0`
/// for the same reason it does on MySQL: `count` ignores the `null` the `case` falls through to and
/// answers over no rows at all.
///
/// **The third counter's cast is inert on this backend rather than absent from it.** MySQL needs it
/// because `sum` over an integer column comes back a `decimal`; SQLite answers that `sum` as an
/// integer already, and it reads a type name it does not have by its own affinity rules — `signed`
/// carries none of the spellings that name a text, blob or real affinity, so the cast is numeric,
/// and a numeric cast of an integer is that integer. An inert cast is not a dialect difference,
/// which is what keeps this one text rather than two.
pub const COUNTS_SQLITE: &str = COUNTS_MYSQL;

/// The same four counters in T-SQL, each widened to the type the other three answer in.
///
/// **`count` is `int` on this backend alone**, and `sum` over an `int` column is an `int` that
/// overflows at a depth a busy queue reaches, so the widening is a correctness fix rather than a
/// tidiness one: `attempts` is cast before it is summed, and the counters after it so that all four
/// values arrive as one type whichever backend answered. `rule:core-classes/db-one-api`'s one API
/// is what that buys — a program reading `stats` reads the same value everywhere.
pub const COUNTS_SQLSERVER: &str = "select \
    cast(count(case when state = 0 then 1 end) as bigint), \
    cast(count(case when state = 1 then 1 end) as bigint), \
    coalesce(sum(cast(attempts as bigint)), 0), \
    (select cast(count(*) as bigint) from nvs_dead_jobs where queue = @p1) \
    from nvs_jobs where queue = @p1";

/// Every statement the queue sends on one driver, as `(member, sql)`, each [`Split`] flattened to
/// its two halves.
///
/// **One roster rather than a list per case that wants one**, which is the same argument the
/// statements themselves make: a member added to the queue is covered by every property asserted
/// over this list on the day it lands, rather than on the day somebody remembers to extend a
/// literal. The match is exhaustive for [`runs`]'s reason — a sixth driver is a build failure here
/// instead of a roster that silently answers for four of five.
///
/// `pub` for the reason [`CLAIM_POSTGRES`] is: `crates/nvs-stdlib/tests/queue.rs` asks this crate
/// what it sends, and a test target is another crate. What it answers is the text alone, so a
/// reader cannot mistake it for a way to send one.
#[must_use]
pub fn texts(driver: nvs_db::Driver) -> Vec<(&'static str, &'static str)> {
    fn flattened(
        pairs: &[(&'static str, Split)],
        singles: &[(&'static str, &'static str)],
    ) -> Vec<(&'static str, &'static str)> {
        let mut texts: Vec<(&'static str, &'static str)> = Vec::new();
        for (member, split) in pairs {
            texts.push((*member, split.first));
            texts.push((*member, split.then));
        }
        texts.extend_from_slice(singles);
        texts
    }
    match driver {
        nvs_db::Driver::Postgres => flattened(
            &[],
            &[
                ("push", INSERT_POSTGRES),
                ("claim", CLAIM_POSTGRES),
                ("move", DEAD_LETTER_POSTGRES),
                ("delete", DELETE_POSTGRES),
                ("status", STATUS_POSTGRES),
                ("cancel", CANCEL_POSTGRES),
                ("stats", COUNTS_POSTGRES),
                ("roster", QUEUES_POSTGRES),
                ("succeeded", SUCCEEDED_POSTGRES),
                ("retry", RETRY_POSTGRES),
                ("purge", PURGE_POSTGRES),
                ("purge dead", PURGE_DEAD_POSTGRES),
            ],
        ),
        nvs_db::Driver::MySql | nvs_db::Driver::MariaDb => flattened(
            &[
                ("push", INSERT_MYSQL),
                ("claim", CLAIM_MYSQL),
                ("move", DEAD_LETTER_MYSQL),
            ],
            &[
                ("delete", DELETE_MYSQL),
                ("status", STATUS_MYSQL),
                ("cancel", CANCEL_MYSQL),
                ("stats", COUNTS_MYSQL),
                ("roster", QUEUES_MYSQL),
                ("succeeded", SUCCEEDED_MYSQL),
                ("retry", RETRY_MYSQL),
                ("purge", PURGE_MYSQL),
                ("purge dead", PURGE_DEAD_MYSQL),
            ],
        ),
        nvs_db::Driver::Sqlite => flattened(
            &[
                ("push", INSERT_SQLITE),
                ("claim", CLAIM_SQLITE),
                ("move", DEAD_LETTER_SQLITE),
                ("delete", DELETE_SQLITE),
            ],
            &[
                ("status", STATUS_SQLITE),
                ("cancel", CANCEL_SQLITE),
                ("stats", COUNTS_SQLITE),
                ("roster", QUEUES_SQLITE),
                ("succeeded", SUCCEEDED_SQLITE),
                ("retry", RETRY_SQLITE),
                ("purge", PURGE_SQLITE),
                ("purge dead", PURGE_DEAD_SQLITE),
            ],
        ),
        nvs_db::Driver::SqlServer => flattened(
            &[
                ("push", INSERT_SQLSERVER),
                ("move", DEAD_LETTER_SQLSERVER),
                ("delete", DELETE_SQLSERVER),
            ],
            &[
                ("claim", CLAIM_SQLSERVER),
                ("status", STATUS_SQLSERVER),
                ("cancel", CANCEL_SQLSERVER),
                ("stats", COUNTS_SQLSERVER),
                ("roster", QUEUES_SQLSERVER),
                ("succeeded", SUCCEEDED_SQLSERVER),
                ("retry", RETRY_SQLSERVER),
                ("purge", PURGE_SQLSERVER),
                ("purge dead", PURGE_DEAD_SQLSERVER),
            ],
        ),
    }
}

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

/// `{tag: …}`'s. See [`ARGS_ARG`].
const TAG_ARG: usize = 7;

/// `purge`'s queue name, which is the whole of its positional half.
const PURGE_QUEUE_ARG: usize = 0;

/// `purge`'s `{state: …}`.
const PURGE_STATE_ARG: usize = 1;

/// `purge`'s `{tag: …}`.
const PURGE_TAG_ARG: usize = 2;

/// `purge`'s `{before: …}`.
const PURGE_BEFORE_ARG: usize = 3;

/// `purge`'s `{limit: …}`.
const PURGE_LIMIT_ARG: usize = 4;

/// How many rows a `purge` that wrote no `{limit: …}` removes.
///
/// ADR 0153 § 4 makes the bound finite with nothing written and names no number, so the number is
/// this module's: it is what one statement may hold a lock for on the connection the application
/// enqueues through without the enqueues behind it noticing, and it is large enough that draining a
/// table which has been growing since the deployment is a loop of calls rather than a conversation.
/// A caller with a maintenance window of its own writes its own bound; what it cannot write is no
/// bound at all, which is `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`.
const DEFAULT_PURGE_LIMIT: u64 = 1_000;

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
                        // `mixed` admits a written `null` without spelling it, so the omission
                        // fill is the never-written marker rather than a null — otherwise the
                        // two arrive as one argument
                        // (`rule:core-api/a-nullable-field-omits-as-the-never-written-marker`).
                        // Both still mean *no payload* here: `rule:core-api/a-written-null-removes`
                        // makes a written `null` a removal, and there is nothing to remove but
                        // the payload. [`payload_of`] reads the pair.
                        //
                        // `mixed` admits a `secret` without spelling it either, and that one
                        // is refused rather than carried: the module doc above names the
                        // call-site rule, because no type written in this cell could.
                        default: Const::NeverWritten,
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
                    CoreOption {
                        name: "tag",
                        // Neutral for `key`'s reason exactly: a tag is compared against a column
                        // and read by nothing else, and the commonest one there is — a tenant or a
                        // batch named by the request that created the work — is the one a `string`
                        // would have refused.
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
        // Beside the four rather than among them, which is
        // `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s own reading:
        // this is the operator's half of the class, and the first member here to
        // take a capability at all. It is asked with the same receipt `cancel` is
        // and answers the same `bool` for the same reason — a job a worker holds
        // is not removable, and finding that out is the ordinary case rather than
        // an exceptional one.
        CoreMethod {
            name: "delete",
            names: &["job"],
            params: &[CoreTy::Instance(ID_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_queue_delete",
            doc: Some(&DELETE_DOC),
        },
        // The other half of the operator's pair, taking the same grant for ADR 0153 § 5's
        // reason: a capability answers *may this program remove queue records*, and one row
        // at a time in a loop is that same answer at a different rate. Its subject is a
        // queue's name rather than a receipt, which makes it `stats`' twin rather than
        // `cancel`'s — the member asked about a population — and what it answers is the count
        // § 4's drain loop reads.
        CoreMethod {
            name: "purge",
            names: &["queue"],
            params: &[
                // **Neutral, by `rule:security/sink-predicate` and for `push`'s `queue`
                // option's reason**: the name is a bound parameter the wire protocol frames
                // and a column compares, never text a parser executes. What keeps a request
                // from choosing which queue is swept is the grant this member is scoped by,
                // and a qualifier here would be a second answer to that question that the
                // operator cannot see.
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(&[
                    CoreOption {
                        name: "state",
                        ty: CoreTy::Enum(STATE_NAME),
                        // Absent rather than a default case, because the default is a *set* of
                        // two and no enum case names a set: [`PURGE_POSTGRES`] writes it as
                        // its own two literals and [`purge_state_of`] reads `Tag::Null` for
                        // the call that named none.
                        default: Const::Null,
                    },
                    CoreOption {
                        name: "tag",
                        // Neutral for `push`'s `tag`'s reason exactly, and it is that column:
                        // a group named by the request that created the work is the commonest
                        // tag there is, and it selects among rows the grant already covers.
                        ty: CoreTy::Text(Qual::Neutral),
                        default: Const::Null,
                    },
                    CoreOption {
                        name: "before",
                        ty: CoreTy::Instance(crate::time::INSTANT_NAME),
                        default: Const::Null,
                    },
                    CoreOption {
                        name: "limit",
                        // The one option whose omission is a number rather than an absence,
                        // which is [`DEFAULT_PURGE_LIMIT`]'s whole subject.
                        ty: CoreTy::Uint,
                        default: Const::Uint(DEFAULT_PURGE_LIMIT),
                    },
                ]),
            ],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_queue_purge",
            doc: Some(&PURGE_DOC),
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
        ParamDoc {
            name: "tag",
            desc: "A group name: any number of jobs may carry one, nothing dedupes on it, and \
                   `purge` is the only thing that reads it. Grouping is decided here, at the \
                   enqueue, because nothing can later group rows that were never grouped.",
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

/// `Core\Queue::delete`'s reference card — `rule:core-api/reference-card`.
const DELETE_DOC: MethodDoc = MethodDoc {
    short: "Removes one job's row, wherever the receipt finds it — the jobs table, or the \
            dead-letter table a job moved to when it exhausted its attempts. A job a worker is \
            running now is left alone: there is no protocol for interrupting work in flight, and \
            removing the row under it would let the job run to completion reporting into nothing. \
            Needs the `queue.purge` capability for the queue the receipt names.",
    params: &[ParamDoc {
        name: "job",
        desc: "The receipt `push` answered with, which names both the row and the queue it is in. \
               It keeps naming the job across the move to the dead-letter table, so a receipt does \
               not expire when a job fails for the last time.",
        shape: &[],
    }],
    ret: "`true` if this call is what removed the row, and `false` if there was nothing to \
          remove — because a worker is holding it, or because it was never in this queue, or \
          because an earlier `delete` got there. Unlike `cancel`, nothing is left for `status` to \
          answer about afterwards: the row is gone, not changed.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This deployment grants no `queue.purge` for the queue the receipt names, which \
                   is the answer until an operator writes one; or it writes no `[queue]` block, so \
                   nothing says which database the job would be in; or the queue's connection \
                   names a driver that cannot yet run a statement.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The queue's connection did not open, or the delete was refused by the server — \
                   most often because `nvs queue migrate` has not created the tables.",
        },
    ],
};

/// `Core\Queue::purge`'s reference card — `rule:core-api/reference-card`.
const PURGE_DOC: MethodDoc = MethodDoc {
    short: "Removes a queue's finished jobs — `Core\\Queue\\State::Succeeded` and \
            `Core\\Queue\\State::Cancelled`, which is the set a call naming no state selects — and \
            answers how many rows went. `Core\\Queue\\State::Dead` and \
            `Core\\Queue\\State::Pending` are reached only by a call that names one of them, \
            because each is a record something else would otherwise lose silently, and \
            `Core\\Queue\\State::Claimed` is not reachable at all. Bounded with nothing written, \
            so a table that has been growing since the deployment is drained by calling this until \
            it answers `0`. Needs the `queue.purge` capability for the queue it names.",
    params: &[
        ParamDoc {
            name: "queue",
            desc: "The queue to sweep, matched exactly: the name `push` wrote in its `{queue: …}` \
                   option, and the name the grant is scoped on.",
            shape: &[],
        },
        ParamDoc {
            name: "state",
            desc: "One state to remove in place of the default set. `Dead` reads the dead-letter \
                   table instead of the jobs table, `Pending` removes work that has not run yet, \
                   and `Claimed` throws — a worker is running that job, and there is no protocol \
                   for interrupting work in flight.",
            shape: &[],
        },
        ParamDoc {
            name: "tag",
            desc: "Only the jobs `push` tagged with this group name. Grouping is decided at the \
                   enqueue, so nothing written here can group rows that were never grouped.",
            shape: &[],
        },
        ParamDoc {
            name: "before",
            desc: "Only the jobs enqueued before this instant, which is a job's age rather than \
                   its next attempt: the retry ladder moves `runAt` and never the row's age.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "How many rows at most, oldest first. It is finite with nothing written, \
                   because an unbounded delete over the one table that grows without bound holds \
                   a lock on the connection the application enqueues through for as long as it \
                   takes.",
            shape: &[],
        },
    ],
    ret: "How many rows this call removed, and `0` when nothing matched — so \
          `while (Core\\Queue::purge('email') > 0) {}` is the loop that drains a large table, and \
          a count rather than a `bool` is what lets it terminate.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call named `Core\\Queue\\State::Claimed`, which is work a worker holds: \
                   removing that row would leave the job running to completion with nothing to \
                   report into.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "This deployment grants no `queue.purge` for the queue named, which is the \
                   answer until an operator writes one; or it writes no `[queue]` block, so \
                   nothing says which database the jobs would be in; or the queue's connection \
                   names a driver that cannot yet run a statement.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The queue's connection did not open, or the delete was refused by the server — \
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
/// Two tags mean `None` and they are two different requests that arrive at the same place: an
/// omitted `args` is the never-written marker
/// (`rule:core-api/a-nullable-field-omits-as-the-never-written-marker`) and a written `{args: null}`
/// is `rule:core-api/a-written-null-removes`'s removal, which for this option is the payload
/// itself. The distinction the marker buys is spent by `Core\Uri::with`, not here; what it buys
/// here is that the row can be declared `mixed` at all.
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
    if matches!(args[ARGS_ARG].tag(), Some(Tag::Null | Tag::Unset)) {
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
/// **It answers `true` for every driver, and that is a claim rather than a constant.** § 4 has a
/// text in each of the four dialects ([`texts`]) and [`queue_connection`] has an arm that sends
/// each of them, so there is no backend `Core\Queue` opens a connection to and then refuses; the
/// wider question — can a statement be sent at all — is
/// `rule:core-classes/db-drivers-are-an-enum`'s and is answered one crate down.
///
/// **A predicate and not a `bool` constant**, because the roster is what a reader that cannot hold
/// a connection asks: `crates/nvs-stdlib/tests/queue.rs`'s leg gate is the one that does, and
/// `queue_runs_on_every_driver` beside it is what holds this answer to the seam. The match is
/// exhaustive for [`queue_connection`]'s reason — a sixth driver is a build failure here rather
/// than a silent `true` for a dialect nobody wrote.
#[must_use]
pub fn runs(driver: nvs_db::Driver) -> bool {
    match driver {
        nvs_db::Driver::Postgres
        | nvs_db::Driver::MySql
        | nvs_db::Driver::MariaDb
        | nvs_db::Driver::Sqlite
        | nvs_db::Driver::SqlServer => true,
    }
}

/// The queue's connection, as the send path the member's statements go out over.
///
/// **Four arms where the module has four dialects**, and one of them is a *binding* rather than a
/// spelling: § 4's statements are PostgreSQL's single texts, MySQL's — some of them [`Split`]s —
/// and T-SQL's, and SQLite runs the second set, [`STATUS_SQLITE`] and its siblings being those
/// constants aliased. What it does not share is how a value reaches the statement, and [`Sent`] is
/// where that is argued. The MySQL and MariaDB arm is [`crate::db::Framed`] for the opposite
/// reason, and that type's doc owns why a driver difference that is only the type of the borrow is
/// flattened at the call sites. [`runs`] is the same roster as a predicate, for the readers that
/// need to ask without holding a connection.
///
/// Every driver is spelled rather than left to a `_`, so a sixth arrives as a build failure here
/// instead of as a refusal written wherever the last session happened to leave one.
///
/// # Errors
///
/// A [`Fault::fatal`] for a key the request's own table does not hold, which is this crate's paste
/// error rather than a program's.
fn queue_connection<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
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
        nvs_db::Connection::Sqlite(sqlite) => Ok(Queued::Sqlite(sqlite)),
        nvs_db::Connection::SqlServer(tds) => Ok(Queued::SqlServer(tds)),
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
    /// The same texts, and a different way of reaching them: [`INSERT_SQLITE`] is its own literal,
    /// [`STATUS_SQLITE`] and the rest are MySQL's aliased, and every one of them binds owned
    /// [`nvs_db::SqliteValue`]s rather than encoded octets ([`Sent`]). The [`Split`]s among them run
    /// inside the immediate transaction `rule:concurrency/claiming-is-one-statement` names.
    Sqlite(&'a mut nvs_db::SqliteConn),
    /// [`INSERT_SQLSERVER`], [`STATUS_SQLSERVER`] and the rest of § 4's T-SQL, binding the array
    /// PostgreSQL binds: a `@pN` carries its own number, so a value the text reads twice goes out
    /// once, where a `?` is a position and cannot. What this dialect has instead of a
    /// data-modifying common table expression is a transaction, so the two [`Split`]s among its
    /// texts — the enqueue's dedupe read and [`DELETE_SQLSERVER`] — run inside one.
    SqlServer(&'a mut nvs_db::TdsConn),
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
    fn nvs_core_queue_push(ctx, args: [8]) {
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
        let tag = args[TAG_ARG].as_text().map(str::to_owned);
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
        // Encoded once and bound twice, because the two dialects want the same values in two
        // orders: [`INSERT_POSTGRES`] names the dedupe key first, since `$1` is read by all three
        // of the places it appears, and [`INSERT_MYSQL`]'s insert names it in column order like
        // every other value — twice, since it writes it to two columns and a `?` cannot repeat.
        // One array per order over one set of buffers, rather than a second encoding of the same
        // integers. A text parameter's octets are the string's own on both dialects, so the four
        // strings are borrowed and only the numbers are rendered; SQLite takes neither form, and
        // [`Sent`] is where its own values are built.
        let dedupe = key.as_deref().map(str::as_bytes);
        let tagged = tag.as_deref().map(str::as_bytes);
        let queued = queue.as_bytes();
        let scripted = script.as_bytes();
        let payloaded = payload.as_deref().map(str::as_bytes);
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
        // [`INSERT_MYSQL`]'s eleven slots, which [`INSERT_SQLSERVER`] takes unchanged: the two
        // texts write the same columns in the same order, so the array is one binding rather than
        // one per dialect that happens to agree.
        let eleven: [Option<&[u8]>; 11] = [
            Some(queued),
            Some(scripted),
            payloaded,
            Some(&state),
            Some(&attempts),
            Some(&backing),
            Some(&due),
            // `dedupe_key` and `dedupe_pending`: one value in two columns, which `INSERT_MYSQL`'s
            // doc owns and `INSERT_POSTGRES` spells as `$1` twice.
            dedupe,
            dedupe,
            Some(&created),
            tagged,
        ];
        let id = match queue_connection(ctx, handle, PUSH)? {
            Queued::Postgres(postgres) => {
                let bound: [Option<&[u8]>; 10] = [
                    dedupe,
                    Some(queued),
                    Some(scripted),
                    payloaded,
                    Some(&state),
                    Some(&attempts),
                    Some(&backing),
                    Some(&due),
                    Some(&created),
                    tagged,
                ];
                push_in_one(postgres, &bound, &block, &mut spans)?
            }
            Queued::Framed(framed) => {
                push_in_two(framed, dedupe, &eleven, &block, &mut spans)?
            }
            // The same eleven slots, because `@p1` through `@p11` are written over the same
            // columns in the same order — [`INSERT_SQLSERVER`] owns why the pair is a transaction
            // here and why each slot a column types as a number is cast where it is bound.
            Queued::SqlServer(tds) => push_in_tds(tds, dedupe, &eleven, &block, &mut spans)?,
            // [`INSERT_SQLITE`]'s eleven slots, which are the framed dialect's in the framed
            // dialect's order — the same value in `dedupe_key` and `dedupe_pending` among them.
            Queued::Sqlite(sqlite) => {
                let keyed = sqlite_text(key.as_deref());
                let values = vec![
                    sqlite_text(Some(&queue)),
                    sqlite_text(Some(&script)),
                    sqlite_text(payload.as_deref()),
                    nvs_db::SqliteValue::Int(i64::from(PENDING)),
                    nvs_db::SqliteValue::Int(i64::from(max_attempts)),
                    nvs_db::SqliteValue::Int(backoff),
                    nvs_db::SqliteValue::Int(run_at.unwrap_or(now)),
                    keyed.clone(),
                    keyed,
                    nvs_db::SqliteValue::Int(now),
                    sqlite_text(tag.as_deref()),
                ];
                push_in_sqlite(sqlite, key.as_deref(), values, &block, &mut spans)?
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

/// [`push_in_two`] in T-SQL, where the pair is two statements for a reason of the dialect's own.
///
/// **A common table expression cannot modify data here**, so there is no transcription of
/// [`INSERT_POSTGRES`]'s single statement at all and the dedupe read is its own round trip —
/// [`INSERT_SQLSERVER`] owns that, and owns why the read takes `updlock, holdlock` rather than the
/// row lock a `for update` would have nothing to take. The transaction is therefore what the pair
/// *means*, exactly as on the framed dialect.
///
/// **The id comes off the insert's own row.** `output inserted.id` answers what a `RETURNING` does,
/// which is why nothing here reads a `last_id`: `nvs_db::tds::TdsRows` has none, and
/// `scope_identity()` would be a third statement whose agreement with its insert a session has to
/// keep right forever.
///
/// A push naming no key opens nothing, for [`push_in_two`]'s reason.
///
/// # Errors
///
/// [`insert_refused`] for anything the server refused, including the transaction commands, and a
/// [`Fault::fatal`] for an insert whose `output` clause answered no row.
fn push_in_tds(
    tds: &mut nvs_db::TdsConn,
    dedupe: Option<&[u8]>,
    bound: &[Option<&[u8]>],
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    let Some(key) = dedupe else {
        return tds_inserted(tds, bound, block, spans);
    };
    let opened = tds
        .begin(None, false)
        .map_err(|refused| insert_refused(block, &refused))?;
    spans.named(block, opened);
    let outcome = match tds_id(tds, INSERT_SQLSERVER.first, &[Some(key)], block, spans) {
        // [`INSERT_POSTGRES`]'s trailing `union all` moved into the caller, as [`deduped`] moves
        // it: a pending job under this key is the answer, and the insert stands down.
        Ok(Some(pending)) => Ok(pending),
        Ok(None) => tds_inserted(tds, bound, block, spans),
        Err(failed) => Err(failed),
    };
    match outcome {
        Ok(id) => {
            let closed = tds
                .commit()
                .map_err(|refused| insert_refused(block, &refused))?;
            spans.named(block, closed);
            Ok(id)
        }
        Err(failed) => {
            // Best effort, and the enqueue's own refusal is what the caller hears, for
            // [`push_in_two`]'s reason.
            let _undone = tds.roll_back();
            Err(failed)
        }
    }
}

/// [`INSERT_SQLSERVER::then`](INSERT_SQLSERVER): the insert, and the id its `output` clause hands
/// back in the same round trip.
///
/// # Errors
///
/// [`insert_refused`] for anything the server refused, and a [`Fault::fatal`] for an insert that
/// answered no row — [`schema`] declares the identity column that makes that impossible.
fn tds_inserted(
    tds: &mut nvs_db::TdsConn,
    bound: &[Option<&[u8]>],
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    tds_id(tds, INSERT_SQLSERVER.then, bound, block, spans)?.ok_or_else(|| {
        Fault::fatal(format!(
            "{PUSH}: the insert into `{JOBS_TABLE}` answered no `output inserted.id` row at all"
        ))
    })
}

/// The first row's first column as the identity [`schema`] declares it, or `None` for a statement
/// that answered no row.
///
/// One reader for both halves of [`INSERT_SQLSERVER`], because both ask the same question of the
/// same column: the dedupe read answers the pending job's id and the insert answers the new one's,
/// and what a caller does about an absence is the only thing that differs.
///
/// # Errors
///
/// [`insert_refused`] for anything the server refused, and a [`Fault::fatal`] for an `id` that came
/// back as anything but an integer, which is a table some other writer created.
fn tds_id(
    tds: &mut nvs_db::TdsConn,
    sql: &str,
    bound: &[Option<&[u8]>],
    block: &str,
    spans: &mut Spans,
) -> Result<Option<u64>, Fault> {
    let mut answered = tds
        .query(sql, bound)
        .map_err(|refused| insert_refused(block, &refused))?;
    crate::db::name_span(&mut answered, Some(block));
    // Described before the first row, for [`deduped`]'s reason: this driver measures a value by the
    // column it arrived under, and the definitions are lent out of a shared borrow while the rows
    // are read out of a mutable one.
    let described: Vec<nvs_db::tds::TdsColumn> = answered.columns().to_vec();
    let mut first: Option<u64> = None;
    while let Some(row) = answered
        .next_row()
        .map_err(|refused| insert_refused(block, &refused))?
    {
        // Every row is read whatever the first one said: the connection owes the transaction around
        // it a message boundary, and each of these statements answers at most one row anyway.
        if first.is_some() {
            continue;
        }
        let Some(column) = described.first() else {
            return Err(Fault::fatal(format!(
                "{PUSH}: the row read back from `{JOBS_TABLE}` has no first column"
            )));
        };
        // Unreachable from source: `nvs-db` decodes one value per described column, so a row is
        // exactly as wide as the result set said.
        let body = row.column(0).ok_or_else(|| {
            Fault::fatal(format!(
                "{PUSH}: the row read back from `{JOBS_TABLE}` has no first column"
            ))
        })?;
        let scalar =
            nvs_db::tds::scalar(column, body).map_err(|refused| insert_refused(block, &refused))?;
        first = match scalar {
            nvs_db::tds::TdsScalar::Int(id) => u64::try_from(id).ok(),
            other => {
                return Err(Fault::fatal(format!(
                    "{PUSH}: `{JOBS_TABLE}`.`id` read back as {other:?}, not an integer"
                )));
            }
        };
    }
    spans.note(answered.span());
    Ok(first)
}

/// [`push_in_two`] on the backend whose parameters are values, and whose transaction is the mutual
/// exclusion itself.
///
/// **The transaction is what makes the read and the insert one moment**, and [`INSERT_SQLITE`] owns
/// why a deferred one would fail under exactly the concurrency the pair exists to survive:
/// [`sqlite_opened`] is which level this opens and why.
///
/// A push naming no key opens nothing, for [`push_in_two`]'s reason: there is no read for the
/// insert to be keyed on, so one statement is the whole enqueue.
///
/// # Errors
///
/// [`insert_refused`] for anything SQLite refused, including the transaction commands, and a
/// [`Fault::fatal`] for a pending row whose id is not an integer.
fn push_in_sqlite(
    sqlite: &mut nvs_db::SqliteConn,
    dedupe: Option<&str>,
    values: Vec<nvs_db::SqliteValue>,
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    let Some(key) = dedupe else {
        return sqlite_inserted(sqlite, values, block, spans);
    };
    let refused = |failed: &dyn std::fmt::Display| insert_refused(block, failed);
    sqlite_opened(sqlite, block, &refused, spans)?;
    match sqlite_deduped(sqlite, key, values, block, spans) {
        Ok(id) => {
            let closed = sqlite
                .commit()
                .map_err(|refused| insert_refused(block, &refused))?;
            spans.named(block, closed);
            Ok(id)
        }
        Err(failed) => {
            // Best effort, and the enqueue's own refusal is what the caller hears, for
            // [`push_in_two`]'s reason.
            let _undone = sqlite.roll_back();
            Err(failed)
        }
    }
}

/// The transaction a [`Split`] runs inside, opened at whichever level the request left the
/// connection at.
///
/// **An immediate transaction where there is none, and a savepoint inside one.**
/// `nvs_db::SqliteConn::begin_immediate` takes the write lock up front, which is what
/// `rule:concurrency/claiming-is-one-statement` rests on here, and it is an outermost transaction by
/// construction: that method refuses a nested level rather than turning it into a savepoint, because
/// a savepoint holds whatever lock the level around it took. A member called inside the request's
/// own transaction therefore opens the nested level — the write lock is already held by then, the
/// pair is still undone as one by a rollback to it, and
/// `rule:concurrency/enqueue-commits-with-your-write` still commits it with the caller's own write.
///
/// # Errors
///
/// Whatever `refused` makes of a refusal.
fn sqlite_opened(
    sqlite: &nvs_db::SqliteConn,
    block: &str,
    refused: &dyn Fn(&dyn std::fmt::Display) -> Fault,
    spans: &mut Spans,
) -> Result<(), Fault> {
    let opened = if sqlite.depth() == 0 {
        sqlite.begin_immediate()
    } else {
        sqlite.begin(None, false)
    }
    .map_err(|failed| refused(&failed))?;
    spans.named(block, opened);
    Ok(())
}

/// [`deduped`] on SQLite: the pending key read first, and the insert only if nothing holds it.
///
/// # Errors
///
/// [`insert_refused`]'s, and a [`Fault::fatal`] for a `first` whose row answered no integer id —
/// [`deduped`]'s reading of the same column.
fn sqlite_deduped(
    sqlite: &mut nvs_db::SqliteConn,
    key: &str,
    values: Vec<nvs_db::SqliteValue>,
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    // Scoped, because `nvs_db::SqliteRows` borrows the connection until it is dropped and the
    // insert below is the second statement of this transaction.
    let pending = {
        let mut span = nvs_db::QuerySpan::opened(nvs_db::Driver::Sqlite, INSERT_SQLITE.first);
        let mut answered = sqlite
            .query(
                INSERT_SQLITE.first,
                vec![nvs_db::SqliteValue::Text(key.to_owned())],
            )
            .map_err(|refused| insert_refused(block, &refused))?;
        let mut pending: Option<u64> = None;
        while let Some(row) = answered.next_row() {
            span.row();
            if pending.is_some() {
                continue;
            }
            let Some(nvs_db::SqliteValue::Int(id)) = row.first() else {
                return Err(Fault::fatal(format!(
                    "{PUSH}: `{JOBS_TABLE}`.`id` came back as something other than an integer, and \
                     this module's own schema is what declares it one"
                )));
            };
            pending = u64::try_from(*id).ok();
        }
        span.finished(None);
        spans.named(block, span);
        pending
    };
    if let Some(already) = pending {
        return Ok(already);
    }
    sqlite_inserted(sqlite, values, block, spans)
}

/// [`INSERT_SQLITE`]'s insert, and the id the connection reports for it.
///
/// **The id is `sqlite3_last_insert_rowid` and not a `returning` clause**, for [`CANCEL_SQLITE`]'s
/// reason: this driver already answers what the text would have to spell, and a clause of its own
/// would be a difference between [`INSERT_SQLITE`] and the statement it is otherwise a
/// transcription of.
///
/// # Errors
///
/// [`insert_refused`]'s, and a [`Fault::fatal`] for an insert the connection reports no row id for.
fn sqlite_inserted(
    sqlite: &nvs_db::SqliteConn,
    values: Vec<nvs_db::SqliteValue>,
    block: &str,
    spans: &mut Spans,
) -> Result<u64, Fault> {
    let mut span = nvs_db::QuerySpan::opened(nvs_db::Driver::Sqlite, INSERT_SQLITE.then);
    let mut answered = sqlite
        .query(INSERT_SQLITE.then, values)
        .map_err(|refused| insert_refused(block, &refused))?;
    // Drained rather than skipped, for `Core\Db`'s reason: the result set is what holds the
    // connection until it is done with, and this transaction has a commit after it.
    while answered.next_row().is_some() {}
    let landed = answered.last_insert_id();
    span.finished(Some(answered.affected()));
    spans.named(block, span);
    u64::try_from(landed)
        .ok()
        .filter(|id| *id != 0)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{PUSH}: the insert into `{JOBS_TABLE}` reported no row id, and `0` is how this \
                 driver spells no row having been inserted on this connection at all"
            ))
        })
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
/// **Each send path's text and its own binding arrive together**, because a dialect is not only its
/// SQL: [`STATUS_MYSQL`] binds four parameters where [`STATUS_POSTGRES`] binds two, and a signature
/// taking one binding for both would make that impossible to say. T-SQL is handed the array the
/// first dialect binds, which is that same fact in the other direction — a numbered marker is read
/// as often as the text likes. SQLite's arrives as a thunk and not as a value, so a member holding
/// a `String` and an `i64` builds [`Sent`]'s owned values on the connection that takes them and
/// nowhere else. What it is *not* is a rewrite — `nvs_db::sql::rewrite` renders one spelling into
/// another, and these are four statements.
///
/// `columns` is how many of the row's columns to read, so `cancel` asks for none and reads
/// [`Counted::touched`] alone.
///
/// # Errors
///
/// Whatever `refused` makes of a server's refusal — one wording per member, since what an operator
/// does about it depends on what was being asked — and a [`Fault::fatal`] for a row narrower than
/// the result set described it, which is a `nvs-db` bug rather than a program's.
#[expect(
    clippy::too_many_arguments,
    reason = "a text and its own binding per send path, plus the row width, the block, the \
              member's own refusal and § 11's spans"
)]
fn counted_row(
    queued: Queued<'_>,
    postgres: (&str, &[Option<&[u8]>]),
    framed: (&str, &[Option<&[u8]>]),
    sqlserver: (Tds, &[Option<&[u8]>]),
    sqlite: &dyn Fn() -> Sent,
    columns: usize,
    block: &str,
    refused: &dyn Fn(&dyn std::fmt::Display) -> Fault,
    spans: &mut Spans,
) -> Result<Counted, Fault> {
    match queued {
        Queued::Sqlite(connection) => {
            sqlite_counted(connection, sqlite(), columns, block, refused, spans)
        }
        Queued::SqlServer(connection) => tds_counted(
            connection,
            sqlserver.0,
            sqlserver.1,
            columns,
            block,
            refused,
            spans,
        ),
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

/// § 4's statements as SQL Server takes them: one text, or the pair a dialect with no
/// data-modifying common table expression writes instead.
///
/// **No binding of its own**, where [`Sent`] carries one: this driver's markers are numbered, so
/// both halves of the one pair bind the array their member already built for PostgreSQL, and a
/// variant holding two would be describing a difference the dialect does not have.
enum Tds {
    /// One statement, whose row and count are the member's whole answer.
    One(&'static str),
    /// A [`Split`]'s two statements inside one transaction, whose counts add up —
    /// [`DELETE_SQLSERVER`], the one member with no single-statement shape on this backend.
    Pair(Split),
}

/// [`counted_row`]'s SQL Server arm, and the one that may open a transaction of its own.
///
/// **A [`Tds::Pair`] is one moment because of the transaction and not because of the text**, which
/// is [`DELETE_SQLSERVER`]'s own argument for why the pair may not be run as two — a receipt that
/// deleted from one table and not the other would leave `status` answering about a job `delete`
/// reported removed. The level is whatever the request left the connection at, for
/// [`push_in_tds`]'s reason: inside the caller's own transaction `begin` is a `SAVE TRANSACTION`,
/// so the pair is still undone as one and still commits with the write around it.
///
/// # Errors
///
/// Whatever `refused` makes of a refusal, including the transaction commands'.
fn tds_counted(
    tds: &mut nvs_db::TdsConn,
    sent: Tds,
    bound: &[Option<&[u8]>],
    columns: usize,
    block: &str,
    refused: &dyn Fn(&dyn std::fmt::Display) -> Fault,
    spans: &mut Spans,
) -> Result<Counted, Fault> {
    let split = match sent {
        Tds::One(sql) => return tds_statement(tds, sql, bound, columns, block, refused, spans),
        Tds::Pair(split) => split,
    };
    let opened = tds.begin(None, false).map_err(|failed| refused(&failed))?;
    spans.named(block, opened);
    let both = match tds_statement(tds, split.first, bound, 0, block, refused, spans) {
        Ok(head) => {
            tds_statement(tds, split.then, bound, 0, block, refused, spans).map(|tail| Counted {
                row: None,
                affected: Some(
                    head.affected
                        .unwrap_or(0)
                        .saturating_add(tail.affected.unwrap_or(0)),
                ),
            })
        }
        Err(failed) => Err(failed),
    };
    match both {
        Ok(counted) => {
            let closed = tds.commit().map_err(|failed| refused(&failed))?;
            spans.named(block, closed);
            Ok(counted)
        }
        Err(failed) => {
            // Best effort, and the member's own refusal is what the caller hears — [`push_in_two`]
            // states the reading, and a poisoned connection is `nvs-db`'s to close either way.
            let _undone = tds.roll_back();
            Err(failed)
        }
    }
}

/// One T-SQL statement, drained, and the first row's requested columns as integers.
///
/// # Errors
///
/// Whatever `refused` makes of the server's refusal, and a [`Fault::fatal`] for a row narrower than
/// the result set described it, which is [`counted_row`]'s reading of that as a `nvs-db` bug.
fn tds_statement(
    tds: &mut nvs_db::TdsConn,
    sql: &str,
    bound: &[Option<&[u8]>],
    columns: usize,
    block: &str,
    refused: &dyn Fn(&dyn std::fmt::Display) -> Fault,
    spans: &mut Spans,
) -> Result<Counted, Fault> {
    let mut answered = tds.query(sql, bound).map_err(|failed| refused(&failed))?;
    crate::db::name_span(&mut answered, Some(block));
    // Cloned before the first row, for [`counted_row`]'s two other wire arms' reason: a value is
    // read against the definition it arrived under, and the definitions outlive the borrow the rows
    // are read through.
    let described: Vec<nvs_db::tds::TdsColumn> = answered.columns().to_vec();
    let mut row: Option<Vec<Option<i64>>> = None;
    // Every row is read before the answer is judged, for the PostgreSQL arm's reason: the
    // connection has to be back at a message boundary before this returns, or the next statement on
    // it — the caller's own, or this member's second half — meets a busy one.
    while let Some(reading) = answered.next_row().map_err(|failed| refused(&failed))? {
        if row.is_some() {
            continue;
        }
        let mut read = Vec::with_capacity(columns);
        for (at, column) in described.iter().enumerate().take(columns) {
            // Unreachable from source and fatal for the framed arm's reason: `nvs-db` decodes one
            // value per described column, so a row is exactly as wide as this loop.
            let body = reading.column(at).ok_or_else(|| {
                Fault::fatal(format!(
                    "the row has no column {at}, where the result set described {}",
                    described.len()
                ))
            })?;
            let scalar = nvs_db::tds::scalar(column, body).map_err(|failed| refused(&failed))?;
            read.push(match scalar {
                nvs_db::tds::TdsScalar::Int(number) => Some(number),
                _ => None,
            });
        }
        row = Some(read);
    }
    let affected = answered.affected();
    spans.note(answered.span());
    Ok(Counted { row, affected })
}

/// § 4's statements as SQLite takes them: the text, and the values typed rather than rendered.
///
/// **A form of its own and not a third `&[Option<&[u8]>]`**, because this driver binds a *value*
/// where the others bind a rendering of one — `nvs_db::SqliteConn::query` takes owned
/// [`nvs_db::SqliteValue`]s and `crates/nvs-db/src/sqlite.rs`'s `encode` is that conversion for
/// `Core\Db`. A `?` still binds by position, so the order and the count are [`STATUS_MYSQL`]'s and
/// the placeholder arithmetic beside it is unchanged.
/// A receipt's id as the storage class SQLite holds it in.
///
/// **Saturating rather than refusing**, where `crates/nvs-db/src/sqlite.rs`'s `encode` refuses: that
/// one is answering for a value a program wrote, and this is § 2's `integer primary key`, which this
/// backend cannot have issued past [`i64::MAX`]. A receipt that large names no row, so the statement
/// it goes into answers nothing — which is the member's own *no such job* answer rather than a
/// second wording of it.
fn sqlite_id(id: u64) -> nvs_db::SqliteValue {
    nvs_db::SqliteValue::Int(i64::try_from(id).unwrap_or(i64::MAX))
}

/// A queue, a tag or a payload as SQLite holds it: `TEXT`, or the `NULL` an absent option is.
fn sqlite_text(text: Option<&str>) -> nvs_db::SqliteValue {
    text.map_or(nvs_db::SqliteValue::Null, |held| {
        nvs_db::SqliteValue::Text(held.to_owned())
    })
}

enum Sent {
    /// One statement, whose row and count are the member's whole answer.
    One(&'static str, Vec<nvs_db::SqliteValue>),
    /// A [`Split`]'s two statements and the values each of them binds, whose counts add up —
    /// [`DELETE_SQLITE`], the one member with no single-statement shape on this backend.
    Pair(Split, Vec<nvs_db::SqliteValue>, Vec<nvs_db::SqliteValue>),
}

/// [`counted_row`]'s SQLite arm, and the one that may open a transaction of its own.
///
/// **A [`Sent::Pair`] is one moment because of the transaction and not because of the text**, which
/// is `rule:concurrency/claiming-is-one-statement`'s mechanism on this backend and
/// [`DELETE_SQLITE`]'s own argument for why the pair may not be run as two. Which transaction that
/// is, and why a deferred one would fail under exactly the concurrency it exists to survive, is
/// [`sqlite_opened`]'s.
///
/// # Errors
///
/// Whatever `refused` makes of a refusal, including the transaction commands'.
fn sqlite_counted(
    sqlite: &mut nvs_db::SqliteConn,
    sent: Sent,
    columns: usize,
    block: &str,
    refused: &dyn Fn(&dyn std::fmt::Display) -> Fault,
    spans: &mut Spans,
) -> Result<Counted, Fault> {
    let (split, first, then) = match sent {
        Sent::One(sql, values) => {
            return sqlite_statement(sqlite, sql, values, columns, block, refused, spans);
        }
        Sent::Pair(split, first, then) => (split, first, then),
    };
    sqlite_opened(sqlite, block, refused, spans)?;
    let both = match sqlite_statement(sqlite, split.first, first, 0, block, refused, spans) {
        Ok(head) => {
            sqlite_statement(sqlite, split.then, then, 0, block, refused, spans).map(|tail| {
                Counted {
                    row: None,
                    affected: Some(
                        head.affected
                            .unwrap_or(0)
                            .saturating_add(tail.affected.unwrap_or(0)),
                    ),
                }
            })
        }
        Err(failed) => Err(failed),
    };
    match both {
        Ok(counted) => {
            let closed = sqlite.commit().map_err(|failed| refused(&failed))?;
            spans.named(block, closed);
            Ok(counted)
        }
        Err(failed) => {
            // Best effort, and the member's own refusal is what the caller hears — [`push_in_two`]
            // states the reading, and a poisoned connection is `nvs-db`'s to close either way.
            let _undone = sqlite.roll_back();
            Err(failed)
        }
    }
}

/// One SQLite statement, drained, with § 11's span opened here rather than by the driver.
///
/// **The count is a write's and never a read's.** `nvs_db::SqliteRows::affected` answers what the
/// last data-changing statement on this *connection* reported, so a `select` would read whatever
/// wrote before it; a member asking for columns is asking for a row, and [`Counted::touched`] is
/// not what it reads. `crates/nvs-stdlib/src/db/execute.rs`'s two SQLite walks split the same way
/// and for the same reason.
///
/// The span is this module's on this backend alone, for that module's reason: there is no round
/// trip for a driver-side one to time, and what it does time is the handoff to `nvs_host::blocking`
/// and back.
///
/// # Errors
///
/// Whatever `refused` makes of a refusal.
fn sqlite_statement(
    sqlite: &nvs_db::SqliteConn,
    sql: &str,
    values: Vec<nvs_db::SqliteValue>,
    columns: usize,
    block: &str,
    refused: &dyn Fn(&dyn std::fmt::Display) -> Fault,
    spans: &mut Spans,
) -> Result<Counted, Fault> {
    let mut span = nvs_db::QuerySpan::opened(nvs_db::Driver::Sqlite, sql);
    let mut answered = sqlite
        .query(sql, values)
        .map_err(|failed| refused(&failed))?;
    let mut row: Option<Vec<Option<i64>>> = None;
    // Every row is read for [`counted_row`]'s reason, and here the set is in hand before the first
    // one is looked at: this driver steps the whole result on the blocking pool.
    while let Some(reading) = answered.next_row() {
        span.row();
        if row.is_some() {
            continue;
        }
        row = Some(
            reading
                .into_iter()
                .take(columns)
                .map(|cell| match cell {
                    nvs_db::SqliteValue::Int(number) => Some(number),
                    _ => None,
                })
                .collect(),
        );
    }
    let affected = (columns == 0).then(|| answered.affected());
    span.finished(affected);
    spans.named(block, span);
    Ok(Counted { row, affected })
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
            queue_connection(ctx, handle, STATUS_OF)?,
            (STATUS_POSTGRES, &bound),
            // The same two values a second time, which is [`STATUS_MYSQL`]'s whole difference: a
            // `?` cannot be named twice where a `$1` can.
            (STATUS_MYSQL, &twice),
            // The array the first dialect binds, for the other half of that difference: a `@p1`
            // carries its own number, so the id and the queue go out once each however often
            // [`STATUS_SQLSERVER`]'s two arms read them.
            (Tds::One(STATUS_SQLSERVER), &bound),
            // That text and that order, bound as values rather than as their octets.
            &|| {
                Sent::One(
                    STATUS_SQLITE,
                    vec![
                        sqlite_id(id),
                        sqlite_text(Some(&queue)),
                        sqlite_id(id),
                        sqlite_text(Some(&queue)),
                    ],
                )
            },
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
            queue_connection(ctx, handle, CANCEL_OF)?,
            (CANCEL_POSTGRES, &bound),
            (CANCEL_MYSQL, &bound),
            (Tds::One(CANCEL_SQLSERVER), &bound),
            &|| Sent::One(CANCEL_SQLITE, vec![sqlite_id(id), sqlite_text(Some(&queue))]),
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
    /// `Core\Queue::delete(Queue\Id $job): bool` — `rule:concurrency/queue-deletion-is-explicit-and-bounded`.
    ///
    /// **[`nvs_core_queue_cancel`]'s twin, down to the `bool`**, and the difference between them is
    /// the whole of what the two members are for: `cancel` changes a state and leaves a row `status`
    /// can still answer about, and this removes the row. ADR 0153 § 2's table is the one home for
    /// which states each of them reaches.
    ///
    /// **The grant is asked first, before the queue's block is read and before anything is
    /// opened.** `rule:security/capability-check-at-the-door` puts the check at the door, and the
    /// ordering here is [`nvs_runtime::capability::pin_host`]'s: a caller outside the grant is
    /// refused whether or not the deployment configures a queue at all, so the difference between
    /// the two refusals cannot be read as a probe for what this program is wired to.
    ///
    /// **The queue it is asked about is the receipt's own**, which is what makes a scope possible
    /// here at all: a `Queue\Id` carries the queue beside the row number ([`ID`]), so the name the
    /// grant is matched against is one the program wrote at its own `push` rather than one this
    /// member had to be handed. There is no static half to this — the name arrives at run time —
    /// and `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s other member is where a
    /// written one is refused while checking.
    ///
    /// **Both tables in one statement, and the claimed row refused inside it.** [`DELETE_POSTGRES`]
    /// owns why the two arms are one moment rather than two, and why the state test is in the text
    /// instead of in a read above it: a job § 6 moved between the two round trips of a
    /// read-then-delete is exactly the row this member exists to leave alone.
    ///
    /// **What it spends:** one statement, on the connection the request either already held or now
    /// holds for the rest of it — so a delete inside a transaction on that connection is undone
    /// with it if that transaction rolls back, exactly as § 3's enqueue commits with it.
    fn nvs_core_queue_delete(ctx, args: [1]) {
        let (id, queue) = job_of(args[0], DELETE_OF)?;
        nvs_runtime::capability::require(
            ctx,
            nvs_config::Cap::QueuePurge,
            nvs_config::capability::Scope::Name(&queue),
            DELETE_OF,
        )?;
        let block = configured_queue(ctx, DELETE_OF)?.connection;
        // Shared, for `push`'s reason: removing on a second connection would be removing from
        // outside whatever transaction the request has open on the first, and a rolled-back
        // request would have destroyed the row anyway.
        let handle = crate::db::open_named(ctx, &block, true, None, DELETE_OF)?;
        let sending: [Option<Vec<u8>>; 2] = [
            Some(id.to_string().into_bytes()),
            Some(queue.clone().into_bytes()),
        ];
        let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
        let refused_by_server = |refused: &dyn std::fmt::Display| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{DELETE_OF}: removing job {id} from `{JOBS_TABLE}` and `{DEAD_TABLE}` on \
                     `[db.{block}]` was refused: {refused} — `nvs queue migrate` is what creates \
                     those tables"
                ),
            )
        };
        // `rule:observability/a-query-is-a-trace-event`'s event, as `push` files it and for the reason given there.
        let mut spans = Spans::of(ctx, &block);
        // No column is read, for [`nvs_core_queue_cancel`]'s reason: [`DELETE_POSTGRES`]'s
        // `returning id` is there to make the removal observable and the caller already holds the
        // id, so [`Counted::touched`] is the whole answer — and it is what [`DELETE_MYSQL`] gives
        // on the framed dialect, which answers a count and no row at all.
        let counted = counted_row(
            queue_connection(ctx, handle, DELETE_OF)?,
            (DELETE_POSTGRES, &bound),
            (DELETE_MYSQL, &bound),
            // A pair here too, and both halves bind the one receipt because the markers are
            // numbered: [`DELETE_SQLSERVER`] owns why neither other dialect's construct exists in
            // T-SQL, and the transaction around the two is what makes their counts one answer.
            (Tds::Pair(DELETE_SQLSERVER), &bound),
            // A pair on this backend for [`DELETE_SQLITE`]'s own reason, and each arm binds the
            // receipt for itself because a `?` is a position: the two counts add up to the same
            // `bool` a single statement answers with.
            &|| {
                let receipt = vec![sqlite_id(id), sqlite_text(Some(&queue))];
                Sent::Pair(DELETE_SQLITE, receipt.clone(), receipt)
            },
            0,
            &block,
            &refused_by_server,
            &mut spans,
        )?;
        // As `push`: taken while the rows still lend the span out, filed once they have let the
        // context go. A delete that found a claimed job sent a statement like any other.
        spans.file(ctx);
        Ok(Value::bool(counted.touched()))
    }
}

/// Which of § 6's two tables a `purge`'s `{state: …}` selects, and the ordinal that table's
/// statement binds for it.
///
/// A call names at most one state, and § 6 puts a dead-lettered job in a table of its own — so the
/// table and the ordinal are one answer rather than two the member would have to keep agreeing
/// about.
#[derive(Clone, Copy)]
enum Selection {
    /// [`JOBS_TABLE`], with the ordinal the call named — or [`None`] for the call that named no
    /// state, whose set [`PURGE_POSTGRES`] writes as two literals of its own.
    Jobs(Option<i64>),
    /// [`DEAD_TABLE`], which is what `State::Dead` selects: being in that table is what the case
    /// *is*, so [`PURGE_DEAD_POSTGRES`] binds no ordinal at all.
    Dead,
}

/// `{state: …}` as the table it selects and the ordinal that table's statement binds.
///
/// **`State::Claimed` is refused here as well as in [`PURGE_POSTGRES`]'s own text**, and the two
/// are not one check written twice: this is where a caller finds out, naming the case it wrote,
/// and the text refusing it is what keeps *a claimed job is not removable* a property of the system
/// rather than of the layer in front of it.
///
/// # Errors
///
/// A thrown `LogicError` for `State::Claimed`. A [`Fault::fatal`] for a slot holding anything but
/// one of [`STATE`]'s ordinals, which the row typing the option as that enum rules out.
fn purge_state_of(args: &[Value]) -> Result<Selection, Fault> {
    if matches!(args[PURGE_STATE_ARG].tag(), Some(Tag::Null)) {
        return Ok(Selection::Jobs(None));
    }
    let ordinal = args[PURGE_STATE_ARG].as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "{PURGE_OF}: expected a `{STATE_NAME}` case for `state`, got tag {}",
            args[PURGE_STATE_ARG].tag_byte()
        ))
    })?;
    // Named out of [`STATE`] rather than compared against a number written here: the ordinals are
    // the column's, so a literal in front of the statement would be a third place they are spelled.
    let (case, _) = STATE
        .cases
        .iter()
        .find(|(_, value)| *value == ordinal)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{PURGE_OF}: `state` holds {ordinal}, which names no `{STATE_NAME}` case"
            ))
        })?;
    match *case {
        "Claimed" => Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{PURGE_OF}: `{STATE_NAME}::Claimed` names work a worker is running now, and there \
                 is no protocol for interrupting work in flight — removing the row under it would \
                 leave the job running to completion with nothing to report into"
            ),
        )),
        "Dead" => Ok(Selection::Dead),
        _ => Ok(Selection::Jobs(Some(ordinal))),
    }
}

/// `{tag: …}` as the group name a statement binds, or [`None`] for the call that left it out.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not text, which the option's declared type rules out.
fn purge_tag_of(args: &[Value]) -> Result<Option<String>, Fault> {
    if matches!(args[PURGE_TAG_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    Ok(Some(
        args[PURGE_TAG_ARG]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{PURGE_OF}: expected a `string` for `tag`, got tag {}",
                    args[PURGE_TAG_ARG].tag_byte()
                ))
            })?
            .to_owned(),
    ))
}

/// `{before: …}` as epoch milliseconds, or [`None`] for the call that left it out.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `Core\Time\Instant`, which the option's declared
/// type rules out.
fn purge_before_of(args: &[Value]) -> Result<Option<i64>, Fault> {
    if matches!(args[PURGE_BEFORE_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let at = crate::time::instant_of(args, PURGE_BEFORE_ARG, "purge")?;
    // Milliseconds, and saturating at both ends, for [`run_at_of`]'s reason: the column is a
    // `bigint` of them, and an age far enough out to overflow selects the same rows either way.
    Ok(Some(at.as_second().saturating_mul(1_000).saturating_add(
        i64::from(at.subsec_nanosecond()) / 1_000_000,
    )))
}

/// Each send path's text for a selection, and the table they remove from.
///
/// A function rather than a `match` inside the member, because *which table a selection reads* is
/// the half of `rule:concurrency/queue-deletion-is-explicit-and-bounded` that a statement's own
/// text cannot state: [`PURGE_DEAD_POSTGRES`] naming [`DEAD_TABLE`] says nothing about which call
/// is routed to it.
fn purge_texts(
    selection: Selection,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match selection {
        Selection::Jobs(_) => (
            PURGE_POSTGRES,
            PURGE_MYSQL,
            PURGE_SQLSERVER,
            PURGE_SQLITE,
            JOBS_TABLE,
        ),
        Selection::Dead => (
            PURGE_DEAD_POSTGRES,
            PURGE_DEAD_MYSQL,
            PURGE_DEAD_SQLSERVER,
            PURGE_DEAD_SQLITE,
            DEAD_TABLE,
        ),
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Queue::purge(string $queue, {state?, tag?, before?, limit?}): uint` —
    /// `rule:concurrency/queue-deletion-is-explicit-and-bounded`, ADR 0153 § 2 and § 4.
    ///
    /// **`stats`' subject and `delete`'s grant.** It is asked about a queue rather than about a
    /// job, because what a retention sweep names is a population; and it takes the same
    /// `queue.purge`, because a capability answers *may this program remove queue records* and one
    /// row at a time in a loop is that answer at a different rate.
    ///
    /// **The grant is asked before the `[queue]` block is read and before anything opens**, so an
    /// ungranted caller cannot read the difference between a deployment that configured no queue
    /// and one that configured a queue it may not sweep.
    ///
    /// **One statement per call, and which one is [`purge_state_of`]'s whole answer.** § 6 puts a
    /// dead-lettered job in another table, so `state: Dead` is a different text rather than another
    /// arm of the same one; every other selection is [`PURGE_POSTGRES`] with the ordinal the call
    /// named, or without one for the default set that statement writes as two literals.
    ///
    /// **What it spends:** one statement, on the connection the request either already held or now
    /// holds for the rest of it — so a purge inside a transaction on that connection is undone with
    /// it if that transaction rolls back, exactly as § 3's enqueue commits with it. Nothing is held
    /// between calls and no row is read: the count is the statement's own completion, and how many
    /// rows it may remove is [`DEFAULT_PURGE_LIMIT`] until a caller writes its own bound.
    fn nvs_core_queue_purge(ctx, args: [5]) {
        // Unreachable from source: the row types this parameter `string`, so a non-text argument is
        // refused at `E0401` first — [`nvs_core_queue_stats`]'s guard states the same judgement.
        let queue = args[PURGE_QUEUE_ARG]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{PURGE_OF}: expected a `string` queue, got tag {}",
                    args[PURGE_QUEUE_ARG].tag_byte()
                ))
            })?
            .to_owned();
        nvs_runtime::capability::require(
            ctx,
            nvs_config::Cap::QueuePurge,
            nvs_config::capability::Scope::Name(&queue),
            PURGE_OF,
        )?;
        let selection = purge_state_of(args)?;
        let tag = purge_tag_of(args)?;
        let before = purge_before_of(args)?;
        // Unreachable from source for the queue name's reason, and an omitting call site
        // materializes the row's own default rather than an absence.
        let limit = args[PURGE_LIMIT_ARG].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{PURGE_OF}: expected a `uint` for `limit`, got tag {}",
                args[PURGE_LIMIT_ARG].tag_byte()
            ))
        })?;
        let block = configured_queue(ctx, PURGE_OF)?.connection;
        // Shared, for `delete`'s reason: removing on a second connection would be removing from
        // outside whatever transaction the request has open on the first.
        let handle = crate::db::open_named(ctx, &block, true, None, PURGE_OF)?;
        let (postgres, framed_text, sqlserver_text, sqlite_sql, table) = purge_texts(selection);
        let queue_sent = queue.clone().into_bytes();
        // Cloned as the queue is, because the values below are one rendering of this option and the
        // SQLite binding is the option itself.
        let tag_sent = tag.clone().map(String::into_bytes);
        let before_sent = before.map(|at| at.to_string().into_bytes());
        let limit_sent = limit.to_string().into_bytes();
        let sending: Vec<Option<Vec<u8>>> = match selection {
            Selection::Jobs(state) => vec![
                Some(queue_sent),
                state.map(|ordinal| ordinal.to_string().into_bytes()),
                tag_sent,
                before_sent,
                Some(limit_sent),
            ],
            Selection::Dead => vec![Some(queue_sent), tag_sent, before_sent, Some(limit_sent)],
        };
        let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
        // [`PURGE_MYSQL`]'s placeholder arithmetic, which is [`STATUS_MYSQL`]'s: a `$n` may be
        // named as often as a statement likes and a `?` is a position that cannot, so every option
        // the text asks about twice is bound twice, in the order it reads them.
        let repeated: &[usize] = match selection {
            Selection::Jobs(_) => &[0, 1, 1, 2, 2, 3, 3, 4],
            Selection::Dead => &[0, 1, 1, 2, 2, 3],
        };
        let framed: Vec<Option<&[u8]>> = repeated.iter().map(|at| bound[*at]).collect();
        let refused_by_server = |refused: &dyn std::fmt::Display| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{PURGE_OF}: removing up to {limit} of the `{queue}` queue's rows from \
                     `{table}` on `[db.{block}]` was refused: {refused} — `nvs queue migrate` is \
                     what creates that table"
                ),
            )
        };
        // `rule:observability/a-query-is-a-trace-event`'s event, as `push` files it and for the reason given there.
        let mut spans = Spans::of(ctx, &block);
        // No column is read, for [`nvs_core_queue_delete`]'s reason: a bounded delete says how many
        // rows it removed on the tag that completes it in either dialect, and that count is the
        // whole of what this member has to answer.
        let counted = counted_row(
            queue_connection(ctx, handle, PURGE_OF)?,
            (postgres, &bound),
            (framed_text, &framed),
            // The first dialect's five values over this dialect's text, because an optional filter
            // read twice is one numbered marker here — [`PURGE_SQLSERVER`] owns that arithmetic.
            (Tds::One(sqlserver_text), &bound),
            // The framed dialect's slots over the framed dialect's text, as values: the `repeated`
            // list above is the same arithmetic, so what changes is what a slot holds.
            &|| {
                let queued = sqlite_text(Some(&queue));
                let tagged = sqlite_text(tag.as_deref());
                let aged = before.map_or(nvs_db::SqliteValue::Null, nvs_db::SqliteValue::Int);
                let bounded = nvs_db::SqliteValue::Int(i64::try_from(limit).unwrap_or(i64::MAX));
                let base = match selection {
                    Selection::Jobs(state) => vec![
                        queued,
                        state.map_or(nvs_db::SqliteValue::Null, nvs_db::SqliteValue::Int),
                        tagged,
                        aged,
                        bounded,
                    ],
                    Selection::Dead => vec![queued, tagged, aged, bounded],
                };
                Sent::One(sqlite_sql, repeated.iter().map(|at| base[*at].clone()).collect())
            },
            0,
            &block,
            &refused_by_server,
            &mut spans,
        )?;
        // As `push`: taken while the rows still lend the span out, filed once they have let the
        // context go.
        spans.file(ctx);
        // A statement that matched nothing answers zero rather than no count at all, which is what
        // makes § 4's drain loop terminate.
        Ok(Value::uint(counted.affected.unwrap_or(0)))
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
            queue_connection(ctx, handle, STATS_OF)?,
            (COUNTS_POSTGRES, &bound),
            // The queue a second time, for [`STATUS_MYSQL`]'s reason: the dead-letter subquery and
            // the aggregate's own `where` each bind their own `?`.
            (COUNTS_MYSQL, &twice),
            // The queue once, because `@p1` is read by both the aggregate's `where` and the
            // dead-letter subquery beside it.
            (Tds::One(COUNTS_SQLSERVER), &bound),
            // That text, and the queue twice for its reason.
            &|| {
                Sent::One(
                    COUNTS_SQLITE,
                    vec![sqlite_text(Some(&queue)), sqlite_text(Some(&queue))],
                )
            },
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
        "nvs_core_queue_delete" => (nvs_core_queue_delete as *const ()).cast(),
        "nvs_core_queue_purge" => (nvs_core_queue_purge as *const ()).cast(),
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
        CANCEL_MYSQL, CANCEL_POSTGRES, CANCEL_SQLITE, CANCEL_SQLSERVER, CLAIM_MYSQL,
        CLAIM_POSTGRES, CLAIM_SQLITE, CLAIM_SQLSERVER, CLASS, COUNTS_MYSQL, COUNTS_POSTGRES,
        COUNTS_SQLITE, DEAD_LETTER_MYSQL, DEAD_LETTER_POSTGRES, DEAD_LETTER_SQLSERVER, DEAD_TABLE,
        DEFAULT_PURGE_LIMIT, DELETE_MYSQL, DELETE_POSTGRES, DELETE_SQLITE, Fault, INSERT_MYSQL,
        INSERT_POSTGRES, INSERT_SQLSERVER, JOBS_TABLE, PENDING, PURGE_DEAD_MYSQL,
        PURGE_DEAD_POSTGRES, PURGE_DEAD_SQLITE, PURGE_MYSQL, PURGE_POSTGRES, PURGE_SQLITE,
        PURGE_STATE_ARG, QUEUES_MYSQL, QUEUES_POSTGRES, RETRY_CAP_MS, RETRY_MYSQL, RETRY_POSTGRES,
        RETRY_SQLSERVER, STATE, STATS, STATS_ATTEMPTS_AT, STATS_ATTEMPTS_SLOT, STATS_CLAIMED_AT,
        STATS_CLAIMED_SLOT, STATS_DEAD_AT, STATS_DEAD_SLOT, STATS_PENDING_AT, STATS_PENDING_SLOT,
        STATUS_MYSQL, STATUS_POSTGRES, STATUS_SQLITE, STATUS_SQLSERVER, SUCCEEDED_MYSQL,
        SUCCEEDED_POSTGRES, SUCCEEDED_SQLSERVER, Selection, Split, ThrownClass, Value, dead_errors,
        migration, purge_state_of, purge_texts, retry_at,
    };
    use super::{NAME, PURGE_DOC, PUSH_DOC};
    use crate::registry::{CAPABILITIES, Const, CoreTy};

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
            ("DELETE_POSTGRES", DELETE_POSTGRES),
            ("PURGE_POSTGRES", PURGE_POSTGRES),
            ("PURGE_DEAD_POSTGRES", PURGE_DEAD_POSTGRES),
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
    /// Asked of [`super::texts`] rather than of the three that happen to be interesting, so a
    /// statement added to this dialect is covered on the day it lands. The readers and the worker's
    /// own statements are not [`Split`]s — nothing in them rests on a construct MySQL lacks — but
    /// they are the same second dialect and owe the same check, and one roster is what says so
    /// without a second list here.
    #[test]
    fn the_mysql_statements_spell_nothing_only_postgresql_has() {
        for (member, sql) in super::texts(nvs_db::Driver::MySql) {
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

    /// The third dialect's texts are held to what a transcription of another one would spell, and
    /// the list is [`DELETE_POSTGRES`]'s own vocabulary plus the clause [`CLAIM_POSTGRES`] carries:
    /// a text spelling any of them came from the first dialect rather than from this one.
    ///
    /// Two of them this backend cannot parse — `::` is a cast operator it does not have, and `for
    /// update` a locking clause it does not need, since the exclusion is the immediate transaction
    /// (`rule:concurrency/claiming-is-one-statement`). The other three it would accept and no
    /// statement here reaches for: `$1` is a name this driver binds nothing into, because a
    /// [`Split`] is sent as owned values by position; an insert's id is
    /// `sqlite3_last_insert_rowid` and a delete's answer is its affected count, both for
    /// [`CANCEL_SQLITE`]'s reason, so nothing needs a `returning`; and the CTE [`DELETE_POSTGRES`]
    /// carries its two tables in is [`DELETE_SQLITE`]'s pair here.
    ///
    /// Asked of [`super::texts`] rather than of the three that are interesting, so a statement
    /// added to this backend is covered on the day it lands. The second half is what the name says
    /// outright: every text binds by position, and one that binds nothing is not the statement it
    /// replaces.
    #[test]
    fn no_sqlite_statement_binds_another_dialects_placeholder() {
        for (member, sql) in super::texts(nvs_db::Driver::Sqlite) {
            for absent in ["$1", "::", "returning", "with ", "for update"] {
                assert!(
                    !sql.contains(absent),
                    "{member}'s SQLite text spells `{absent}`, which is another dialect's: {sql}"
                );
            }
            assert!(
                sql.contains('?'),
                "{member}'s SQLite text binds nothing, so it is not the statement it replaces"
            );
        }
    }

    /// `delete … limit` is a build option, and this backend's roster asks for none.
    ///
    /// `libsqlite3-sys`'s bundled build does not set `SQLITE_ENABLE_UPDATE_DELETE_LIMIT`, and
    /// turning it on is the answer this goal refuses: a statement that parses only under a
    /// non-default build stops parsing the day the dependency moves, where
    /// [`PURGE_POSTGRES`]'s `where id in (select id … order by id limit ?)` is a shape every
    /// backend already has. So the bound is inside a subquery here, and this is what says it stayed
    /// there.
    ///
    /// **A `limit` inside parentheses is the `select`'s and one after the last of them is the
    /// `delete`'s**, which is the whole of the distinction: [`PURGE_SQLITE`] spells the word and is
    /// correct, and a text that moved it out of the subquery spells the same word and would be
    /// refused by the engine. The second half asserts the bound is still there at all, because a
    /// purge that lost it satisfies the first half by having nothing to move.
    #[test]
    fn no_sqlite_statement_asks_for_a_delete_limit() {
        for (member, sql) in super::texts(nvs_db::Driver::Sqlite) {
            let Some(at) = sql.find("delete from") else {
                continue;
            };
            let tail = sql[at..].rsplit(')').next().unwrap_or(&sql[at..]);
            assert!(
                !tail.contains("limit"),
                "{member}'s SQLite text bounds its `delete` with a `limit`, which parses only \
                 under `SQLITE_ENABLE_UPDATE_DELETE_LIMIT` and this build turns nothing on"
            );
        }

        for (member, sql) in [("purge", PURGE_SQLITE), ("purge dead", PURGE_DEAD_SQLITE)] {
            assert!(
                sql.contains("limit ?"),
                "{member} is still bounded, and on this backend the bound is in the subquery"
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
    /// Asked of the pairs and not of the [`Split`]s, whose two halves divide one PostgreSQL text's
    /// placeholders between them and so answer a different question. The delete is out for a
    /// nearer reason: [`DELETE_MYSQL`] names its pair once in a derived table where
    /// [`DELETE_POSTGRES`] names each of `$1` and `$2` twice, so the two counts disagree by
    /// design and that constant's doc is where it is written down.
    #[test]
    fn each_second_text_binds_a_value_wherever_its_first_names_one() {
        for (member, postgres, mysql) in [
            ("status", STATUS_POSTGRES, STATUS_MYSQL),
            ("cancel", CANCEL_POSTGRES, CANCEL_MYSQL),
            ("stats", COUNTS_POSTGRES, COUNTS_MYSQL),
            ("roster", QUEUES_POSTGRES, QUEUES_MYSQL),
            ("succeeded", SUCCEEDED_POSTGRES, SUCCEEDED_MYSQL),
            ("retry", RETRY_POSTGRES, RETRY_MYSQL),
            ("purge", PURGE_POSTGRES, PURGE_MYSQL),
            ("purge dead", PURGE_DEAD_POSTGRES, PURGE_DEAD_MYSQL),
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

    /// A worker reads a claimed job's columns **by ordinal**, so the dialects owe each other more
    /// than a column set here: the same names in the same order. Nothing else would notice them
    /// diverging — a claim that swapped `attempts` and `max_attempts` still runs, still answers six
    /// values, and puts § 6's ladder on the wrong number.
    ///
    /// The alias is stripped rather than matched, because `attempts + 1 as attempts` is exactly the
    /// difference the split forces ([`CLAIM_MYSQL`]'s doc owns why) and it is not a difference in
    /// what the column *is*.
    ///
    /// One case over every dialect rather than a case each: what is asserted is that they **agree**,
    /// and a per-dialect case can only assert what one of them answered.
    #[test]
    fn all_three_dialects_answer_a_claim_with_the_same_columns() {
        fn named(list: &str) -> Vec<&str> {
            list.split(',')
                .map(|one| one.trim().rsplit(" as ").next().unwrap_or(one).trim())
                .collect()
        }
        fn read_by(claim: &Split, dialect: &str) -> Vec<&'static str> {
            named(
                claim
                    .first
                    .strip_prefix("select ")
                    .and_then(|rest| rest.split_once(" from "))
                    .unwrap_or_else(|| {
                        panic!("{dialect}'s claim reads its columns before its table")
                    })
                    .0,
            )
        }
        let returned = CLAIM_POSTGRES
            .split_once("returning ")
            .expect("PostgreSQL's claim answers with a `returning` list")
            .1;
        let selected = read_by(&CLAIM_MYSQL, "MySQL");
        assert_eq!(
            named(returned),
            selected,
            "a worker reads these by position, so the dialects answer one list or none of them does"
        );
        assert_eq!(
            read_by(&CLAIM_SQLITE, "SQLite"),
            selected,
            "a worker reads these by position, so the dialects answer one list or none of them does"
        );
    }

    /// The fourth dialect answers that same list, read off the clause it names its columns in.
    ///
    /// A case of its own rather than a fourth entry above, because the list is somewhere else in
    /// the statement: a [`Split`] names its columns in the `select` its first half reads them with
    /// and PostgreSQL in a `returning`, while T-SQL has neither over an update and writes
    /// `output inserted.<column>` instead. What is asserted is the same thing either way —
    /// `crates/nvs-cli/src/worker.rs` reads a claimed job's columns by position, so every dialect
    /// answers one list or none of them does.
    #[test]
    fn the_sql_server_claim_answers_the_columns_the_other_dialects_do() {
        fn named(list: &str) -> Vec<&str> {
            list.split(',')
                .map(|one| one.trim().rsplit('.').next().unwrap_or(one).trim())
                .collect()
        }
        let returned = CLAIM_POSTGRES
            .split_once("returning ")
            .expect("PostgreSQL's claim answers with a `returning` list")
            .1;
        let output = CLAIM_SQLSERVER
            .split_once("output ")
            .expect("SQL Server's claim answers with an `output` list")
            .1;
        assert_eq!(
            named(output),
            named(returned),
            "a worker reads these by position, so the dialects answer one list or none of them does"
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

    /// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s one arm that is not a policy
    /// choice: a claimed job is refused by the statement that would otherwise remove it.
    ///
    /// **What this asserts is where the predicate sits, which no assertion about an answer can
    /// reach.** A `delete` that read the state and then removed the row would answer `false` for a
    /// job that was claimed at the moment it looked, and would still be the two moments ADR 0153 § 2
    /// says are not survivable — the claim it is racing lands in the gap, and the row goes out from
    /// under a worker that then reports into a void. The one thing a text can be held to is that the
    /// statement carrying the refusal is the statement doing the removing.
    ///
    /// **The two dialects hold it in two places**, and the framed one is the one worth pinning: a
    /// trailing `where` there is read after the join and would take the dead-letter arm with it.
    #[test]
    fn the_delete_statement_excludes_the_claimed_state_in_its_own_where_clause() {
        let arm = |sql: &'static str, from: &str, to: &str| {
            sql.split_once(from)
                .unwrap_or_else(|| panic!("`{from}` is one of the statement's arms"))
                .1
                .split_once(to)
                .unwrap_or_else(|| panic!("that arm ends at `{to}`"))
                .0
        };

        let jobs = arm(DELETE_POSTGRES, "delete from nvs_jobs", "returning");
        assert!(
            jobs.contains("state <> 1"),
            "the refusal is inside the delete it refuses, and not in a read above it: {jobs}"
        );
        let buried = arm(DELETE_POSTGRES, "delete from nvs_dead_jobs", "returning");
        assert!(
            !buried.contains("state"),
            "`DEAD_TABLE` has no `state` column: being in that table is what `Dead` is: {buried}"
        );

        assert!(
            !DELETE_MYSQL.contains(" where "),
            "a `where` is read after the join here, and a claimed job would drop the row the \
             dead-letter arm hangs on"
        );
        let attached = arm(DELETE_MYSQL, "left join nvs_jobs j on ", "left join");
        assert!(
            attached.contains("j.state <> 1"),
            "the refusal is on the join that attaches the jobs table, so it fails to attach that \
             row alone: {attached}"
        );

        assert!(
            !DELETE_POSTGRES.contains("state = ") && !DELETE_MYSQL.contains("state = "),
            "the removable states are every one but `Claimed`, so neither delete names one it \
             will take"
        );
    }

    /// § 4's bound, asserted where a machine with no containers still sees it.
    ///
    /// A server case asserts the bound a call *named*; this asserts that there is no way to send
    /// one of these without a bound at all, which is what
    /// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` is about and what a
    /// `purge` over several million rows costs when it is missing. The two dialects spell it in
    /// different places — PostgreSQL bounds the subquery the ids come from and MySQL bounds the
    /// delete itself — so what is held here is the ordering and the placeholder each text ends
    /// with, which is the whole of the bound either way.
    #[test]
    fn every_purge_is_bounded_and_takes_the_oldest_rows_first() {
        for (name, purge, bound) in [
            (
                "PURGE_POSTGRES",
                PURGE_POSTGRES,
                "order by id limit $5::bigint",
            ),
            (
                "PURGE_DEAD_POSTGRES",
                PURGE_DEAD_POSTGRES,
                "order by id limit $4::bigint",
            ),
            ("PURGE_MYSQL", PURGE_MYSQL, "order by id limit ?"),
            ("PURGE_DEAD_MYSQL", PURGE_DEAD_MYSQL, "order by id limit ?"),
        ] {
            assert!(
                purge.contains(bound),
                "{name} does not name `{bound}`, and a sweep of the one table that grows without \
                 bound holds a lock for as long as it takes: {purge}"
            );
        }
    }

    /// A `Value` array shaped like the one [`purge_state_of`] reads its `{state: …}` out of.
    ///
    /// Only that slot is written. The other three options have helpers of their own, and a text
    /// `Value` would carry a reference this array has nothing to release it with.
    fn purge_reading(state: Value) -> [Value; 5] {
        let mut args = [Value::null(); 5];
        args[PURGE_STATE_ARG] = state;
        args
    }

    /// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s one arm that is not a policy
    /// choice, asserted over the whole enum rather than over the case that is interesting: whatever
    /// a call names, the text this member would send cannot reach a row a worker holds.
    ///
    /// **A sweep and not a line**, because the option is a closed enum: a sixth case added to
    /// [`STATE`] arrives here as a selection nobody classified, and this is what says so. Each case
    /// is either refused before a statement is chosen at all, or lands on a text carrying the
    /// refusal itself — [`PURGE_POSTGRES`]'s `state <> 1`, or [`DEAD_TABLE`], where a claimed row
    /// cannot be at all: § 6 moves a job there only once it has stopped being attempted.
    #[test]
    fn purge_has_no_option_that_reaches_a_claimed_row() {
        for (case, ordinal) in STATE.cases {
            let Ok(selection) = purge_state_of(&purge_reading(Value::int(*ordinal))) else {
                assert_eq!(
                    *case, "Claimed",
                    "the refused case is the one a worker is holding, and it is the only one"
                );
                continue;
            };
            let (postgres, framed, sqlserver, sqlite, table) = purge_texts(selection);
            for (dialect, sql) in [
                ("postgres", postgres),
                ("mysql", framed),
                ("sqlserver", sqlserver),
                ("sqlite", sqlite),
            ] {
                assert!(
                    sql.contains("state <> 1") || table == DEAD_TABLE,
                    "{dialect}: `{case}` selects a text that could remove a claimed row: {sql}"
                );
            }
        }
    }

    /// The refusal a caller meets, which is the half a statement's text cannot carry: the message
    /// is where a program finds out *which* case it wrote is the one with no removal in it.
    ///
    /// Thrown rather than fatal, and `LogicError` rather than the bare `RuntimeError` a
    /// [`Fault::thrown`] would be: naming `Claimed` is a call site that asked for something the
    /// runtime will not do, which is `push`'s `maxAttempts: 0` one member over.
    #[test]
    fn purge_naming_the_claimed_state_throws_logic_error_and_names_the_case() {
        let claimed = STATE
            .cases
            .iter()
            .find(|(name, _)| *name == "Claimed")
            .expect("`Claimed` is one of the enum's cases")
            .1;
        let refused = purge_state_of(&purge_reading(Value::int(claimed)))
            .err()
            .expect("a purge naming the claimed state removes nothing and says so");
        let Fault::Thrown(class, message) = refused else {
            panic!("a call site error is thrown for a program to catch, not a fatal");
        };
        assert!(
            matches!(class, ThrownClass::Logic),
            "the class is the one `rule:core-api/shape-rules` gives an argument a member refuses"
        );
        assert!(
            message.contains("Claimed"),
            "the message names the case that was written: {message}"
        );
    }

    /// ADR 0153 § 2's default set, asserted where the member *reads* it rather than where the
    /// statement writes it.
    ///
    /// [`queue_statements_agree_with_the_state_enum`] holds the two ordinals inside
    /// [`PURGE_POSTGRES`]'s own text; this holds the reading in front of them. A call naming no
    /// state binds no ordinal at all, so the set is the statement's and cannot be one this layer
    /// quietly widened — and each of `Dead` and `Pending` is reachable only by the call that names
    /// it, which is what keeps *nothing is discarded silently* a property of every deployment
    /// rather than of the runtime alone.
    #[test]
    fn purge_with_no_state_takes_succeeded_and_cancelled_and_neither_dead_nor_pending() {
        assert!(
            matches!(
                purge_state_of(&purge_reading(Value::null())),
                Ok(Selection::Jobs(None))
            ),
            "a call naming no state binds no ordinal, so the statement's own two literals are the \
             whole of the default set"
        );
        for (case, ordinal) in STATE.cases {
            if *case == "Claimed" {
                continue;
            }
            let named = purge_state_of(&purge_reading(Value::int(*ordinal)))
                .expect("every case but the claimed one selects something");
            let reached = match named {
                Selection::Jobs(state) => state == Some(*ordinal),
                // Being in the other table is what `Dead` is, so its selection carries no ordinal.
                Selection::Dead => *case == "Dead",
            };
            assert!(
                reached,
                "`{case}` is reached by the call that names it, and the nameless call above \
                 reached none of them"
            );
        }
    }

    /// § 6's other table, asserted as the member's own routing: one case reads it, and it is the
    /// one being in that table *is*.
    ///
    /// What the statements' texts can say is that [`PURGE_DEAD_POSTGRES`] names [`DEAD_TABLE`] and
    /// no ordinal; what they cannot say is which call arrives there. A selection sending `Pending`
    /// to the dead-letter table would remove the record that work was lost while every statement
    /// in the module still read correctly on its own line.
    #[test]
    fn purge_naming_dead_reads_the_dead_letter_table_and_no_other_selection_does() {
        let mut reading = 0;
        for (case, ordinal) in STATE.cases {
            let Ok(selection) = purge_state_of(&purge_reading(Value::int(*ordinal))) else {
                continue;
            };
            let (postgres, framed, sqlserver, sqlite, table) = purge_texts(selection);
            if *case == "Dead" {
                reading += 1;
                for (dialect, sql) in [
                    ("postgres", postgres),
                    ("mysql", framed),
                    ("sqlserver", sqlserver),
                    ("sqlite", sqlite),
                ] {
                    assert!(
                        table == DEAD_TABLE
                            && sql.contains(DEAD_TABLE)
                            && !sql.contains(&format!("{JOBS_TABLE} ")),
                        "{dialect}: `Dead` removes from the table § 6 moved the row into, and \
                         from that one alone: {sql}"
                    );
                }
            } else {
                assert_eq!(
                    table, JOBS_TABLE,
                    "`{case}` is a state of a row in the jobs table, and is removed from there"
                );
            }
        }
        assert_eq!(
            reading, 1,
            "exactly one of the enum's cases reads the dead-letter table"
        );
    }

    /// § 4's bound and § 1's answer, asserted on the registry row rather than on a text.
    ///
    /// [`every_purge_is_bounded_and_takes_the_oldest_rows_first`] holds that no statement can be
    /// *sent* without a bound; this holds that no call can be *written* without one, which is a
    /// different claim: the option's default is a number, so an omitting call site materializes
    /// [`DEFAULT_PURGE_LIMIT`] rather than an absence some later reading would have to invent a
    /// bound for. The count beside it is the other half of the same sentence — a `bool` there
    /// would leave § 4's drain loop with nothing to terminate on.
    #[test]
    fn purge_is_bounded_with_nothing_written_and_answers_the_count_it_removed() {
        let row = CLASS
            .methods
            .iter()
            .find(|member| member.name == "purge")
            .expect("`purge` is one of the class's members");
        assert!(
            matches!(row.return_ty, CoreTy::Uint),
            "the answer is how many rows went, which is what a caller's loop reads"
        );
        let CoreTy::Options(options) = &row.params[1] else {
            panic!("the filter is one trailing options shape, as `push`'s is");
        };
        let limit = options
            .iter()
            .find(|option| option.name == "limit")
            .expect("the bound is one of the shape's fields");
        assert!(
            matches!(limit.default, Const::Uint(DEFAULT_PURGE_LIMIT)),
            "an omitting call site materializes the bound itself, not an absence"
        );
        const {
            assert!(
                DEFAULT_PURGE_LIMIT > 0,
                "a bound of zero is a member that removes nothing and a loop that never drains"
            );
        }
    }

    /// `rule:concurrency/queue-four-members`' four and ADR 0153 § 5's two, as one roster: what the
    /// class declares, and which of its members a grant is asked about.
    ///
    /// **The six are asserted together, because the claim is about the boundary between them.** A
    /// grant appearing on `push` would put the request path behind a list an operator has to keep,
    /// and a grant missing from either removal member would leave the record that work existed
    /// removable by anything that can reach a queue name. Written as an if-and-only-if over every
    /// member rather than as two lines, so a seventh member added to the class fails here until
    /// somebody decides which side of that boundary it is on.
    #[test]
    fn core_queue_declares_delete_and_purge_beside_its_four_and_both_declare_the_capability() {
        let declared: Vec<&str> = CLASS.methods.iter().map(|member| member.name).collect();
        assert_eq!(
            declared,
            ["push", "status", "cancel", "stats", "delete", "purge"],
            "the operator's two sit beside the four a request asks, and not among them"
        );
        for member in CLASS.methods {
            let (_, _, grant) = CAPABILITIES
                .iter()
                .find(|(class, name, _)| *class == NAME && *name == member.name)
                .unwrap_or_else(|| panic!("`{}` declares what it may reach", member.name));
            assert_eq!(
                matches!(grant, Some(nvs_config::Cap::QueuePurge)),
                matches!(member.name, "delete" | "purge"),
                "`{}` is asked for `queue.purge` exactly when it is a member that removes rows",
                member.name
            );
        }
    }

    /// `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them` as a reader of the reference
    /// meets it: the two options are opposites at the point they touch, so the card that documents
    /// one documents the other beside it and says which does which.
    ///
    /// **The adjacency is the assertion**, not decoration. A caller reaching for "delete this
    /// batch" finds `key` first — it is the older option — and a card that explained dedupe
    /// without the group name next to it is how a 500-job batch gets tagged with a key and becomes
    /// a 1-job batch at the enqueue that created it.
    #[test]
    fn pushs_reference_card_documents_tag_beside_key_and_says_which_one_groups() {
        let at = |name: &str| {
            PUSH_DOC
                .params
                .iter()
                .position(|param| param.name == name)
                .unwrap_or_else(|| panic!("`{name}` is one of the options the card documents"))
        };
        assert_eq!(
            at("tag"),
            at("key") + 1,
            "the two are read together, so the card documents them together"
        );
        let desc = |name: &str| PUSH_DOC.params[at(name)].desc;
        assert!(
            desc("tag").contains("group") && !desc("key").contains("group"),
            "the card says which of the two names a group: {}",
            desc("tag")
        );
        assert!(
            desc("key").contains("dedupe"),
            "and which admits one pending job: {}",
            desc("key")
        );
    }

    /// ADR 0153 § 2 as the reference states it, which is the one place a caller reads what a
    /// `purge` naming nothing removes.
    ///
    /// **The default set and the two opt-ins have to be on the card and not only in the rule**,
    /// because the failure they guard against is silent: a caller who believes the default sweeps
    /// the dead-letter table writes a retention job that deletes the record an incident review
    /// reads, and nothing at the call site would have told it otherwise. The count is on the same
    /// card for the same reason — it is what `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s
    /// drain loop reads, and a caller that never learns of it writes one call and believes the
    /// table is empty.
    #[test]
    fn purges_reference_card_names_the_default_set_the_two_opt_ins_and_the_count() {
        for case in ["Succeeded", "Cancelled", "Dead", "Pending"] {
            assert!(
                PURGE_DOC.short.contains(case),
                "the card names `{case}`, which is a state a caller has to know the answer about"
            );
        }
        let state = PURGE_DOC
            .params
            .iter()
            .find(|param| param.name == "state")
            .expect("the option that selects them is documented");
        assert!(
            state.desc.contains("Dead") && state.desc.contains("Pending"),
            "the two opt-ins are explained where they are written: {}",
            state.desc
        );
        assert!(
            PURGE_DOC.ret.contains("removed") && PURGE_DOC.ret.contains('0'),
            "the answer is the count, and the `0` is what ends the loop: {}",
            PURGE_DOC.ret
        );
        let refused = PURGE_DOC
            .errors
            .iter()
            .find(|thrown| thrown.error == "LogicError")
            .expect("the one state with no removal in it throws, and the card says so");
        assert!(
            refused.desc.contains("Claimed"),
            "the throw names the case that was written: {}",
            refused.desc
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
            "every `status`'s dead-letter arm spells this `3`"
        );
        for (dialect, status) in [
            ("postgres", STATUS_POSTGRES),
            ("mysql", STATUS_MYSQL),
            ("sqlite", STATUS_SQLITE),
            ("sqlserver", STATUS_SQLSERVER),
        ] {
            assert!(
                status.contains("select 3 from nvs_dead_jobs"),
                "{dialect}: `status` answers the ordinal above for a dead-lettered job"
            );
            assert!(
                status.contains(JOBS_TABLE) && status.contains(DEAD_TABLE),
                "{dialect}: `status` reads both of § 2's tables"
            );
        }
        assert_eq!(
            case("Cancelled"),
            4,
            "every `cancel` writes this ordinal in place of `Pending`"
        );
        for (dialect, cancel) in [
            ("postgres", CANCEL_POSTGRES),
            ("mysql", CANCEL_MYSQL),
            ("sqlite", CANCEL_SQLITE),
            ("sqlserver", CANCEL_SQLSERVER),
        ] {
            assert!(
                cancel.contains("set state = 4") && cancel.contains("and state = 0"),
                "{dialect}: `cancel` moves a job from the ordinal above to the one before it, and \
                 only that one"
            );
        }
        assert_eq!(
            case("Claimed"),
            1,
            "`DELETE_POSTGRES` refuses this ordinal, and `COUNTS_POSTGRES` counts it"
        );
        assert!(
            DELETE_POSTGRES.contains("state <> 1")
                && DELETE_MYSQL.contains("j.state <> 1")
                && DELETE_SQLITE.first.contains("state <> 1"),
            "every delete leaves the ordinal above where it is, which is the one arm of \
             `rule:concurrency/queue-deletion-is-explicit-and-bounded` that is not a policy choice"
        );
        assert!(
            DELETE_POSTGRES.contains(JOBS_TABLE)
                && DELETE_POSTGRES.contains(DEAD_TABLE)
                && DELETE_MYSQL.contains(JOBS_TABLE)
                && DELETE_MYSQL.contains(DEAD_TABLE)
                && DELETE_SQLITE.first.contains(JOBS_TABLE)
                && DELETE_SQLITE.then.contains(DEAD_TABLE),
            "a receipt names a job across § 6's move, so every delete reaches both of § 2's tables"
        );
        assert!(
            !DELETE_SQLITE.then.contains("state"),
            "the arm over the other table names no ordinal at all, because being in that table is \
             what `Dead` is"
        );
        // `purge` selects a set where `delete` names a receipt, so the ordinals are in the text
        // twice over: the set a call selected nothing for, and the one arm no call may select.
        for (dialect, purge) in [
            ("postgres", PURGE_POSTGRES),
            ("mysql", PURGE_MYSQL),
            ("sqlite", PURGE_SQLITE),
        ] {
            assert!(
                purge.contains(&format!(
                    "state in ({}, {})",
                    case("Succeeded"),
                    case("Cancelled")
                )),
                "{dialect}: a purge naming no state removes what has finished, which is those two \
                 ordinals and no others"
            );
            assert!(
                purge.contains("state <> 1"),
                "{dialect}: `purge` has no options that reach a claimed job, and the text refuses \
                 the ordinal as well as the member does"
            );
            assert!(
                !purge.contains(DEAD_TABLE),
                "{dialect}: `State::Dead` is the other table's whole selection, so the statement \
                 every other selection sends never reaches it"
            );
        }
        for (dialect, purge) in [
            ("postgres", PURGE_DEAD_POSTGRES),
            ("mysql", PURGE_DEAD_MYSQL),
            ("sqlite", PURGE_DEAD_SQLITE),
        ] {
            assert!(
                !purge.contains("state"),
                "{dialect}: being in the dead-letter table is what `Dead` is, so its purge names \
                 no ordinal at all"
            );
            assert!(
                purge.contains(DEAD_TABLE) && !purge.contains(JOBS_TABLE),
                "{dialect}: `State::Dead` selects the table § 6 moved the row into, and only that \
                 one"
            );
        }
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
        for (dialect, counts) in [("mysql", COUNTS_MYSQL), ("sqlite", COUNTS_SQLITE)] {
            assert!(
                counts.contains("case when state = 0") && counts.contains("case when state = 1"),
                "{dialect}: `stats` counts by the two ordinals above, in the spelling a dialect \
                 without `filter` has for a filtered count"
            );
        }
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
        for (dialect, counts) in [
            ("postgres", COUNTS_POSTGRES),
            ("mysql", COUNTS_MYSQL),
            ("sqlite", COUNTS_SQLITE),
        ] {
            assert!(
                counts.contains(JOBS_TABLE) && counts.contains(DEAD_TABLE),
                "{dialect}: `stats` reads both of § 2's tables, taking the depth from the second"
            );
        }
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
            (
                "sqlserver",
                INSERT_SQLSERVER.then,
                CLAIM_SQLSERVER,
                SUCCEEDED_SQLSERVER,
                RETRY_SQLSERVER,
                CANCEL_SQLSERVER,
                DEAD_LETTER_SQLSERVER.first,
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

    /// `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`, asked of the statements as the
    /// exact mirror of the test above: what the dedupe column owes every transition, the tag owes
    /// none of.
    ///
    /// **The two columns are opposites at the point they touch**, so the assertion that carries the
    /// rule is not that `tag` is written — it is that nothing ever assigns it again. A `tag = null`
    /// in the claim would make every group a group of one pending job the moment a worker took the
    /// first of them, and it is the transition beside it in every one of these statements, so it is
    /// the edit a session lands by symmetry. The dead-letter is the one statement that names the
    /// column, and there it is copied rather than cleared: a purge by tag has to reach the rows
    /// that failed, which is where the record of lost work is.
    #[test]
    fn nothing_releases_a_tag_the_way_a_claim_releases_a_dedupe_key() {
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
                push.contains("tag"),
                "{dialect}: the push is the one statement that writes a tag, since grouping is \
                 decided at the enqueue"
            );
            for (name, statement) in [
                ("claim", claim),
                ("write-back", succeeded),
                ("cancel", cancel),
                ("retry", retry),
            ] {
                assert!(
                    !statement.contains("tag"),
                    "{dialect}: the {name} names `tag`, so something releases a group the way a \
                     claim releases a key — and a tag is not a lock"
                );
            }
            assert!(
                moving.contains("tag"),
                "{dialect}: a dead-lettered row leaves `{JOBS_TABLE}` carrying its tag, or a purge \
                 by tag cannot reach the rows that failed"
            );
        }
    }

    /// The half of the rule that is about the *request path*: a tag costs a claim nothing, because
    /// no statement a worker runs to find or re-arm a job reads it.
    ///
    /// Asserted as absence over the whole text rather than over a clause, since the ways to make a
    /// tag cost the claim something are not one construct — a `select` list that carries it, an
    /// `order by` that reads it, a `where` that filters on it. `nvs_jobs_tag` is written for
    /// `purge` alone, and an index the claim's plan reaches for is a second thing every enqueue is
    /// paying to maintain.
    #[test]
    fn the_claim_and_retry_statements_read_no_tag_at_all() {
        for (dialect, statements) in [
            ("postgres", vec![CLAIM_POSTGRES, RETRY_POSTGRES]),
            (
                "mysql",
                vec![CLAIM_MYSQL.first, CLAIM_MYSQL.then, RETRY_MYSQL],
            ),
        ] {
            for statement in statements {
                assert!(
                    !statement.contains("tag"),
                    "{dialect}: a statement on the request path reads `tag`: {statement}"
                );
            }
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

    /// `rule:core-classes/queue-storage-is-a-table`: the dedupe constraint the queue asks for has
    /// **one** spelling, and every dialect emits that one key over that one column.
    ///
    /// This is the retirement's own open question asserted rather than argued. The two lists reach
    /// gap 3's guarantee by two constructs the vocabulary refuses — a partial index and a stored
    /// generated column — so the assertion that matters is not that the constraint exists but that
    /// what carries it is a plain column and a plain unique key, asked for once and emitted by
    /// exactly one statement per dialect. A queue that grew its own spelling again fails here.
    ///
    /// **SQL Server's statement is a filtered `CREATE UNIQUE INDEX` and the other three write a
    /// clause inside the `CREATE TABLE`**, which is `nvs_db::ddl` reading
    /// `rule:core-classes/a-unique-key-reads-nulls-as-distinct` for every schema on that backend
    /// and not this one asking for anything. It is the difference between a queue that works around
    /// a dialect and a dialect brought into line: a `UNIQUE` constraint there would read this
    /// column's nulls as equal and cap the whole queue at one released job, so the predicate is
    /// what makes `dedupe_pending` mean the same thing on all five.
    #[test]
    fn the_queues_dedupe_constraint_is_one_column_in_every_dialect() {
        let schema = super::schema();
        for driver in nvs_db::Driver::ALL.iter().copied() {
            let dialect = nvs_db::Dialect::of(driver);
            let statements = nvs_db::ddl::create_schema(&schema, dialect);
            let jobs = statements
                .iter()
                .find(|statement| statement.contains(JOBS_TABLE))
                .expect("the jobs table is created in every dialect");
            for construct in ["dedupe_key", "dedupe_pending"] {
                assert!(
                    jobs.contains(construct),
                    "{}'s `{JOBS_TABLE}` does not declare `{construct}`",
                    driver.display_name()
                );
            }

            let key: Vec<&String> = statements
                .iter()
                .filter(|statement| statement.contains("nvs_jobs_dedupe"))
                .collect();
            assert_eq!(
                key.len(),
                1,
                "{} emits `nvs_jobs_dedupe` other than once",
                driver.display_name()
            );
            assert_eq!(
                key[0].starts_with("CREATE UNIQUE INDEX"),
                dialect == nvs_db::Dialect::SqlServer,
                "{} spells the dedupe key as `{}`",
                driver.display_name(),
                key[0]
            );
            assert!(
                !key[0].contains("where state") && !key[0].contains("GENERATED ALWAYS AS ("),
                "{} reached gap 3's guarantee by a construct the vocabulary does not hold",
                driver.display_name()
            );
        }
    }

    /// `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`: `tag` is nullable on both
    /// tables, indexed with the queue it belongs to, and has no `_pending` twin.
    ///
    /// The absence is the half worth asserting. A column *named* `tag` is what a session writing
    /// this from the rule's first sentence would land; what the rule actually decides is that
    /// nothing releases it, so a `tag_pending` maintained by the claim — the shape `dedupe_key`
    /// already has beside it, and therefore the shape a later session is most likely to copy —
    /// would cap a group at one pending job while every other assertion here still passed.
    #[test]
    fn the_schema_declares_tag_nullable_on_both_tables_and_indexes_it_with_its_queue() {
        let schema = super::schema();
        for table in schema.tables() {
            let tag = table
                .columns()
                .iter()
                .find(|column| column.name().as_str() == "tag")
                .unwrap_or_else(|| panic!("`{}` declares no `tag`", table.name()));
            assert!(
                tag.is_nullable(),
                "`{}`'s `tag` is not nullable, so no live queue can converge onto it",
                table.name()
            );
            assert_eq!(
                tag.ty(),
                &nvs_db::schema::ScalarType::Text {
                    max: Some(super::KEY_WIDTH)
                },
                "`{}`'s `tag` is not the width every indexed text column of this schema is",
                table.name()
            );
            assert!(
                table
                    .columns()
                    .iter()
                    .all(|column| column.name().as_str() != "tag_pending"),
                "`{}` releases a tag, and a tag is not a lock",
                table.name()
            );
        }
        let jobs = schema
            .tables()
            .iter()
            .find(|table| table.name().as_str() == JOBS_TABLE)
            .expect("the jobs table is one of the two");
        let tagged = jobs
            .indexes()
            .iter()
            .find(|key| key.name().as_str() == "nvs_jobs_tag")
            .expect("`tag` is indexed for `purge` to select on");
        let columns: Vec<&str> = tagged
            .columns()
            .iter()
            .map(nvs_db::schema::Ident::as_str)
            .collect();
        assert_eq!(
            columns,
            ["queue", "tag"],
            "a tag is scoped to its queue, so the queue leads the key"
        );
        assert!(
            jobs.unique_keys()
                .iter()
                .all(|key| key.columns().iter().all(|column| column.as_str() != "tag")),
            "a unique key over `tag` admits one job per group, which is `key`'s job and not this one"
        );
    }

    /// The converge this column had to be declared for: a database on the pre-`tag` schema takes
    /// one **`Safe`** add per table, and the index is the one step that is not.
    ///
    /// Asked of the live value with `tag` filtered back out of it, rather than of a hand-written
    /// pair, because what is being proved is that *this* schema is reachable from a deployment
    /// already running — a transcription of the columns here would prove it of a schema nobody has.
    /// The index is asserted rather than hidden: it is built over every row already there and
    /// `rule:core-classes/schema-plan` grades a build no v1 emitter runs concurrently as `Locking`,
    /// so `nvs queue migrate` takes the columns unasked and the index with `--including-risky`.
    #[test]
    fn the_pre_tag_queue_schema_converges_by_one_safe_column_add_per_table() {
        use nvs_db::schema::{Ident, Table};

        fn without_tag(table: &Table) -> Table {
            let columns = table
                .columns()
                .iter()
                .filter(|column| column.name().as_str() != "tag")
                .cloned()
                .collect();
            let mut built = Table::new(table.name().as_str(), columns)
                .expect("a table minus one column is still a table");
            let key: Vec<&str> = table
                .primary_key_columns()
                .iter()
                .map(Ident::as_str)
                .collect();
            built = built
                .primary_key(&key)
                .expect("the primary key names no column this dropped");
            for unique in table.unique_keys() {
                let over: Vec<&str> = unique.columns().iter().map(Ident::as_str).collect();
                built = built
                    .unique(unique.name().as_str(), &over)
                    .expect("the unique keys name no column this dropped");
            }
            for index in table.indexes() {
                if index.name().as_str() == "nvs_jobs_tag" {
                    continue;
                }
                let over: Vec<&str> = index.columns().iter().map(Ident::as_str).collect();
                built = built
                    .index(index.name().as_str(), &over)
                    .expect("the other indexes name no column this dropped");
            }
            built
        }

        let want = super::schema();
        let have = nvs_db::Schema::new(want.tables().iter().map(without_tag).collect())
            .expect("the same two tables, named apart");
        for driver in nvs_db::Driver::ALL.iter().copied() {
            let dialect = nvs_db::Dialect::of(driver);
            let plan = nvs_db::diff(&want, &have, dialect);
            let added: Vec<&nvs_db::Step> = plan
                .steps()
                .iter()
                .filter(|step| matches!(step.change(), nvs_db::Change::AddColumn { .. }))
                .collect();
            assert_eq!(
                added.len(),
                2,
                "{} plans {:?} where the difference is one column on each table",
                driver.display_name(),
                plan.steps()
                    .iter()
                    .map(|step| step.change().to_string())
                    .collect::<Vec<_>>()
            );
            for step in added {
                assert_eq!(
                    step.grade(),
                    nvs_db::Grade::Safe,
                    "{} grades `{}` above the catalog write a nullable column with no default is",
                    driver.display_name(),
                    step.change()
                );
            }
            let refused = plan
                .first_refused()
                .expect("the index is built over every row already there");
            assert_eq!(
                refused.change().to_string(),
                format!("add index nvs_jobs_tag on {JOBS_TABLE}"),
                "{} refuses a step that is not the index build",
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

    /// The module doc's gap 1, asserted as the absence it now is.
    ///
    /// A declared option that `push` recorded in the row and no isolate applied is
    /// `rule:concurrency/an-upgrades-options-are-spawn-scripts`'s accepted-and-dropped narrowing:
    /// the job keeps the authority the enqueuing request meant to give up, which is a priority-1
    /// failure rather than a missing feature. The isolate half applies neither today, so
    /// undeclared is the matching refusal — a bag reports a key it does not declare
    /// (`rule:core-api/shape-reuses-the-option-diagnostics`), and this reads the rows a call site
    /// resolves that diagnostic off.
    ///
    /// The bag count rides along because it is where both land once the narrowing is applied:
    /// `rule:concurrency/queue-four-members` gives `push` one trailing options shape and no second
    /// place to put a knob.
    #[test]
    fn limits_and_grants_are_refused_by_name_until_an_isolate_enforces_them() {
        let push = super::CLASS
            .methods
            .iter()
            .find(|method| method.name == "push")
            .expect("`Core\\Queue` declares `push`");
        let mut bags = 0;
        for param in push.params {
            let crate::registry::CoreTy::Options(options) = param else {
                continue;
            };
            bags += 1;
            for option in *options {
                assert!(
                    !matches!(option.name, "limits" | "grants"),
                    "`push` declares `{}`, so a narrowing written at the call site is accepted and \
                     dropped — the isolate the worker starts applies neither",
                    option.name
                );
            }
        }
        assert_eq!(
            bags, 1,
            "`push` carries the one trailing bag `rule:concurrency/queue-four-members` gives it, \
             which is where both options land once an isolate applies them"
        );
    }

    /// The `Core\Db::open` half of the same blocker, read against this one.
    ///
    /// Both members wanted a `{…}` an option could carry, and `rule:core-api/shape-parameter`
    /// answers them once: a shape is only ever a whole parameter. `open` took that answer — its
    /// settings literal *is* the parameter — and `push` cannot, because
    /// `rule:concurrency/queue-four-members` puts its knobs in the one trailing bag. So the two
    /// agree by there being no third spelling, which is asserted over every registered row rather
    /// than over these two: an option carrying a shape anywhere would mean one half had settled it
    /// differently, and `Core\Queue` would be the class that read it that way.
    #[test]
    fn the_core_db_open_half_of_the_same_blocker_and_this_one_agree_on_the_spelling() {
        let db = crate::registry::CLASSES
            .iter()
            .find(|class| class.name == crate::db::NAME)
            .expect("`Core\\Db` is registered");
        let open = db
            .methods
            .iter()
            .find(|method| method.name == "open")
            .expect("`Core\\Db` declares `open`");
        assert!(
            open.params
                .iter()
                .any(|param| matches!(param, crate::registry::CoreTy::Shape(_))),
            "`Core\\Db::open`'s settings literal is the whole parameter that lifted its half of the \
             blocker, and this test reads the other half against it"
        );
        for class in crate::registry::CLASSES {
            for method in class.methods.iter().chain(class.instance) {
                for param in method.params {
                    let crate::registry::CoreTy::Options(options) = param else {
                        continue;
                    };
                    for option in *options {
                        assert!(
                            !matches!(option.ty, crate::registry::CoreTy::Shape(_)),
                            "{}::{}'s `{}` carries a shape, so the two halves of one blocker \
                             settled it differently",
                            class.name,
                            method.name,
                            option.name
                        );
                    }
                }
            }
        }
    }
}
