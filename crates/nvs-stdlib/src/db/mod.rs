//! `Core\Db` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 18's entry points, over [`nvs_db`]'s wire half.
//!
//! `rule:core-classes/db-one-api` is authoritative for every
//! semantic and § 18 for every signature. What belongs here is what this side
//! of the boundary decides for itself: what an `InList` *is* once it is a value
//! a program holds, what `quoteIdentifier` can honestly promise from a class
//! that has no connection in front of it, what bounds a statement's `timeout`,
//! and which of § 18's options this module declines to declare.
//!
//! # `inList` is a carrier, and the expansion stays on the wire
//!
//! `rule:core-classes/db-parameters` makes `Core\Db::inList($values)` the explicit marker that a
//! bound parameter expands into a parenthesised run of placeholders, because
//! automatic expansion would make the *SQL text* depend on a runtime value's
//! type. The marker has to survive as a value from the call that builds it to
//! the bind that consumes it, so it is a `Core\Db\InList` — one
//! [`crate::instance`] object over one slot holding the array itself.
//!
//! **It holds the array and computes nothing.** The rendering already exists
//! once, as `nvs_db::sql`'s `expand`, which is where it has to be: the markers
//! are `$1` on PostgreSQL and `@p1` on SQL Server, so only a driver knows what
//! this expands to, and § 1's statement cache keys on the arity that expansion
//! produces. A second rendering here would be a second answer to a question the
//! rewriter already answers, and it would have to guess the dialect.
//!
//! So the only thing this side decides is the refusal. `inList([])` throws
//! `LogicError` at the *call* rather than at the bind, which is § 5's rule and
//! is also the earlier of the two places it can be caught — the caller who
//! wrote the empty list is on the stack, and the branch § 5 asks them to write
//! is the one they are standing in.
//!
//! # `quoteIdentifier` validates, and deliberately adds no delimiter
//!
//! § 18 puts this member on `Core\Db`, which means it runs with **no
//! connection and therefore no dialect**. The five backends do not agree on how
//! an identifier is delimited — `"name"` on PostgreSQL and SQLite, backticks on
//! MySQL and MariaDB, `"name"` or `[name]` on SQL Server — so a delimiter
//! chosen here is wrong on at least two of them, and a *wrong* delimiter is not
//! a cosmetic problem: on MySQL a double-quoted identifier is a string literal,
//! so the answer would parse and mean something else.
//!
//! What laundering owes `rule:security/sink-predicate`
//! is that the result cannot end the identifier and start something else.
//! A strict character class — a letter or `_`, then letters, digits or `_` —
//! guarantees that under *every* dialect at once, which no quoting scheme does:
//! the accepted alphabet contains no quote, no backtick, no bracket, no space,
//! no dot, no semicolon and no comment opener, so there is nothing left in the
//! answer for any parser to act on. That is why this member returns its
//! argument unchanged and refuses everything else, rather than escaping.
//!
//! The cost is stated rather than hidden. A name that genuinely needs
//! delimiting — a reserved word, a name with a space, a mixed-case name on
//! PostgreSQL — is **refused**, not mangled, and the caller sees a `LogicError`
//! naming the name. A `$` is refused too, though four of the five backends
//! accept one inside an identifier: it is the one character in the plausible
//! set that also opens PostgreSQL's dollar-quoting, and the rule is worth more
//! than the character.
//!
//! There is no dotted `schema.table` form for the same reason there is no
//! delimiter: a dot is a separator the caller writes between two calls, so the
//! member stays "one identifier in, one identifier out" and cannot be handed
//! something whose halves it did not each check.
//!
//! **A delimiting quoter belongs on `Connection` if one is ever wanted**, and
//! not on this class, because a connection is the only place a dialect exists.
//! § 18 asks for no such member, and the refusal above is why one here could
//! not be the answer either: `Core\Db` would have to guess the delimiter, and
//! a wrong one parses as something else.
//!
//! # A statement's `timeout` is a deadline on the socket
//!
//! `rule:core-classes/db-statement-members` gives `query`, `queryAs`,
//! `execute`, `executeMany` and `stream` a `{timeout?: Duration}`, and says
//! nothing about what bounds it. There were two candidates and this module is
//! where the choice is recorded, because it is a property of the implementation
//! rather than of the surface: **it is a deadline on the connection's socket**,
//! filed through [`nvs_db::Connection::set_deadline`] exactly where
//! [`nvs_db::PgConn::connect`]'s handshake deadline already goes.
//!
//! The alternative was the server's own `statement_timeout` — a `SET` before the
//! statement, cancelled after it — and it loses on all three counts that matter
//! here:
//!
//! - **It does not bound the failure a timeout exists for.** A server-side
//!   cancel is a message the server has to be answering to act on. The wait a
//!   request has to survive is the one where it is not: a network partition, a
//!   host that froze, a connection the middle of the network dropped. The socket
//!   deadline bounds that case and the server-side one is silent through it.
//! - **It is a second statement on a connection § 4 allows one on**, and a round
//!   trip per statement to set and another to put back. § 1's cache exists to
//!   make a statement one round trip; this would make every bounded one three.
//! - **It is spelled four ways and missing once.** PostgreSQL and MySQL have a
//!   session variable with different semantics, SQL Server's `LOCK_TIMEOUT`
//!   bounds something else entirely, and SQLite has none — so the portable
//!   option would mean four different things and, on one backend, nothing.
//!
//! The socket deadline holds on the four drivers with a wire. **SQLite is the
//! one exception and it is answered rather than excused**: it has no socket, and
//! the only thing a statement of its waits *for* is the write lock, so its arm
//! is `sqlite3_busy_timeout` — `nvs_db::sqlite`'s `set_busy_timeout` owns that
//! reasoning. Either way the promise is the same one: the call answers, one way
//! or the other, by the instant the caller named.
//!
//! Three consequences worth stating where a reader meets them:
//!
//! - **The bound is the connection's, not the call's**, so every statement path
//!   files its own — [`bound_connection`] is the one door, and it files a `None`
//!   for a call that named no timeout so that a statement cannot inherit the
//!   bound of the one before it. [`warm_connection`] lifts it again before
//!   § 13's reset, which is the one exchange no program's clock may bound.
//! - **A statement that runs out of time throws `IOError` and spends the
//!   connection.** It was given up on part way through a message, so the wire is
//!   poisoned by `rule:core-classes/db-connection-busy-state`'s own rule and the connection does not rejoin the
//!   pool. That is the honest cost of the bound and the reference cards say so.
//! - **On `stream` the deadline bounds the walk**, not the call that opens it:
//!   it stays filed while the portal is open, so it covers every `advance()` up
//!   to the last row — which is the wait a streaming caller actually takes.
//!
//! `rule:observability/a-slow-query-is-logged-past-a-threshold`'s `slow_query` is the neighbour that is neither of these: it
//! *reports* a statement that took too long and never stops one.
//!
//! # `stream` declares no `chunk`, and the portal is why
//!
//! **That is a refusal rather than an unlanded option.** § 4's other option is
//! in the spec signature and deliberately in neither of that member's registry
//! rows, because there is no read for a size to reach. `nvs_db`'s `open_portal`
//! writes its `Execute` with a row count of **`0` — every row** — and the walk
//! then reads one `DataRow` off the wire per `advance()`, which is why that
//! driver never has to answer a `PortalSuspended` at all. So a streamed result
//! set already crosses on one round trip while the client holds a single row:
//! § 4's constant memory at the best case the protocol has.
//!
//! A chunk size could only turn that one `Execute` into one per chunk, each
//! resuming a suspended portal — **latency spent (AGENTS.md's priority 3) to
//! buy nothing**, since the memory the option exists to bound is already one
//! row. An option that parsed and did nothing would be worse than its absence,
//! which the compiler can at least report. What would reopen this is a driver
//! whose protocol delivers a result set eagerly rather than as a readable
//! stream, and none of the five is one. Its sibling `{timeout?: Duration}` is
//! no longer here — the section above is where that landed and what it decided.
//!
//! # Every driver reaches every member, and all five are pooled
//!
//! **Five drivers open, and everything past the handshake follows on every one
//! of them.** `connect` branches on the block's `driver` —
//! `rule:core-classes/db-connection-is-named` — so a `postgres` block, a
//! `mysql` block, a `mariadb` block and an `mssql` one each reach their own
//! target, their own default port and their own `nvs_db::Connection` variant,
//! and `open` branches the same ways on the settings hash's own `driver`. A
//! block naming `sqlite` reaches `nvs_db::sqlite::open` off its own arm of
//! § 2's discriminated union, which has a file where the other four have an
//! address: `connect` resolves no host for it and `open` asks `fs.read` and
//! `fs.write` of the path instead of
//! `rule:http-server/allow-url-pins-the-address`'s address table.
//!
//! Past the handshake the list is shorter than that. Binding is whole:
//! [`rendering_of`] pairs § 5's dialect with § 9's encoder off the connection's
//! own [`nvs_db::Driver`], so a MySQL statement is rewritten to `?` and bound
//! as MySQL reads a parameter. Sending is not: [`queried_rows`] branches on the
//! connection and [`mysql_rows`] drains a binary result set through § 9's
//! decode, so `query`, `queryAs`, `execute` and `executeMany` answer on either
//! driver, § 11's event included. § 7's `transaction` does too, over
//! [`Transacting`] — the drivers' commands differ and `nvs_db::mysql`'s `begin`
//! owns how, but the five points this module asks them at do not. MySQL and
//! MariaDB reach all of it through one body rather than two: [`Framed`] is that
//! seam, and its doc is where "its own driver above the framing, not inside
//! it" is argued.
//!
//! **SQL Server reaches every member the other three do**:
//! `nvs_db::TdsConn::connect` is reached from both openers, [`rendering_for`]
//! binds a parameter through `nvs_db::tds::encode`, and the one `sp_prepexec`
//! behind `nvs_db::TdsConn::query` answers every member built on it —
//! [`tds_rows`] drains the token stream for `query` and `queryAs`,
//! [`tds_write`] drains it for `execute`'s count, and
//! `nvs_db::tds::execute_many` runs § 4's batch as that same send once per
//! parameter set — one `sp_prepexec`, an `sp_execute` after it. § 7 is
//! [`Transacting`]'s fourth arm over `nvs_db::tds`'s own commands, T-SQL
//! spelling a savepoint `SAVE TRANSACTION`, having no `RELEASE` for a nested
//! commit to send and no read-only transaction to offer at all. What
//! [`crate::queue`]'s four members run on is narrower than this section and is
//! that module's gap 4: `nvs_stdlib::queue::runs` is the roster, and a driver
//! missing from it is missing a text rather than a send path.
//!
//! **SQLite reaches every member the other four do**: [`rendering_for`] binds a
//! parameter through `nvs_db::sqlite::encode` as a storage class rather than as
//! octets ([`Binds`] is that split), and [`sqlite_rows`], [`sqlite_write`] and
//! `nvs_db::SqliteConn::execute_many` answer `query`, `queryAs`, `execute` and
//! `executeMany` over it. § 7 is [`Transacting`]'s fifth arm, over
//! `nvs_db::sqlite`'s own `begin`, `commit` and `roll_back` — commands sent by
//! a call on the blocking pool rather than over a wire, accepting every
//! isolation level because this backend is always serializable and refusing
//! `readOnly` because it has no read-only transaction to open. **So there is no
//! driver gap left, and [`transacting`] is total**: every `nvs_db::Connection`
//! variant has an arm, which is why the refusal that used to name a roster of
//! drivers is gone rather than left with an empty list to render. **§ 9's
//! declared-type map runs on this driver like it does on the other four**, in
//! [`sqlite_column_value`] — a cell in a column the schema declared `date`,
//! `datetime`, `time`, `uuid`, `decimal` or `boolean` is that, and one the
//! declaration does not describe throws — so nothing downstream of a row learns
//! that SQLite has five storage classes and no date.
//!
//! **Every driver has a reset behind it, and all five are pooled.**
//! `rule:security/db-pool-reset-is-a-boundary`'s pool is on disk as
//! [`nvs_runtime::pool`], a connection is *released* to it at teardown under
//! the ticket `Core\Db::connect` files, and [`warm_connection`] takes one back
//! out behind that section's reset. The five resets are not one reset and § 13
//! says so per backend; SQLite's is the shortest of them, a file handle having
//! no session state to leak, and a rollback of whatever transaction is open is
//! the whole of it.
//!
//! # Known gaps
//!
//! 1. **An `open` describing an endpoint no block describes still takes the
//!    default bounds.** The operator's half of `rule:security/db-pool-reset-is-a-boundary` has landed for the
//!    rest: [`settings_bounds`] reads the `[db.<name>.pool]` of the block whose
//!    settings hash *is* this connection's — [`block_settings_key`] builds each
//!    block's key through [`settings_key`] itself, so the pool and the memo
//!    cannot come to disagree about what "the same connection" is — and the
//!    unscoped `[db] pool = false` reaches every connection the process opens,
//!    which is the audited deployment's requirement and the one thing a
//!    per-block key could never state. What is left is narrower: a literal
//!    naming an endpoint an operator wrote no block for, and a literal that
//!    differs from the block in any hashed field — a written `port` where the
//!    block left the server's default implicit — is a second key and so a
//!    second pool, at `PoolBounds::DEFAULT`. Where bounds for a key only the
//!    program knows would be *written* is an `rule:security/db-pool-reset-is-a-boundary` question and not a
//!    shape this module may pick on its own.
//!
//!    `rule:core-api/shape-arms-are-disjoint`'s *exactly one arm accepts it* **is** the checker's rule —
//!    `nvs_types::expr::args`' `select_arm` — so a `host` written beside
//!    `Driver::Sqlite` is the compile error § 18 says it is, and a key only one
//!    arm requires is required of the call that selected that arm. What reaches
//!    [`settings_driver`] is therefore a literal one arm has already accepted,
//!    which is why it reads the discriminant before it reads anything else and
//!    why every slot it then reads is filled.
//!    Decided: Nowhere: the defaults apply, and a deployment that wants bounds writes a block — No new
//!    surface; mismatched literals silently get defaults unless a first-use notice is added.
//!    — owner: unowned-closures
//! 2. **`Db\DbError` is not in spec § 10's error tree, so it declares no
//!    `issues`.** A per-column refusal is thrown as a `ParseError` naming the
//!    columns instead, because that is the class the property is declared on,
//!    and what has to be decided is whether `DbError` joins the tree — § 10
//!    giving it that property too — or the split stands. What the class *does*
//!    declare is all five of § 18's values.
//!    A refusal the server itself made is thrown as
//!    `nvs_runtime::ThrownClass::DbError` ([`statement_failure`]), so a `catch`
//!    can name the database instead of `RuntimeError` — which it still is,
//!    being its parent, so nothing written against the old class stops
//!    working. `kind` is a slot and it carries **the kind the server chose**:
//!    `nvs_hir::errors::OWN_PROPERTIES` declares it, `nvs_types::error_lib`
//!    types it as the registered enum [`ERROR_KIND`], the synthesized
//!    constructor seeds `Other` for a `DbError` a program built itself, and
//!    [`statement_failure`] writes the driver's own classification over it
//!    through `nvs_runtime::Fault::thrown_with_slots`, and the raw `sqlState`
//!    and `constraint` beside it where a server worded the refusal — the second
//!    only where the condition names one, since most do not. `sql` is the
//!    statement as the caller spelled it, absent only where the member had no
//!    caller-written statement to name — § 7's `BEGIN` and `COMMIT`.
//!    `driverCode` is the vendor integer a server sends beside its `SQLSTATE` —
//!    MySQL's `1062` — and stays `null` on PostgreSQL, whose `SQLSTATE` is its
//!    only code. A failure of the *wire* rather than of the
//!    statement stays an `IOError`: § 8's class is the server's answer, not the
//!    socket's.
//!    Decided: Split stands: a shape mismatch is a ParseError, as for Json::decodeAs — One class for
//!    'data does not fit the type' across Json and Db, and no spec change.
//!    — owner: unowned-closures
//! 3. **`query`, `queryAs`, `execute`, `executeMany`, `stream` and
//!    `transaction` are what has landed of `Core\Db\Queryable`** (gap 4 is what
//!    `queryAs` still owes). **`stream` lands on PostgreSQL, MySQL and
//!    MariaDB**, and what is left is the wire half rather than this one: § 4's
//!    read needs the statement left open with the read state parked off the
//!    borrow — `nvs_db::PgCursor`, and `nvs_db::MySqlCursor` for the two drivers
//!    that share one row loop — and SQLite and SQL Server have no such state, so
//!    [`mod@stream`]'s member throws a `RuntimeError` naming `query` on those two
//!    rather than buffering behind the caller's back. Buffering would be the worse answer twice over: it
//!    breaks the member's one promise, constant memory, and it breaks § 4's
//!    *uniform* connection-busy rule, which is there so that a program written
//!    against one driver runs on all five. `streamAs` is owed whole and is that
//!    class at a written type, exactly as `queryAs` is `query`'s. Of § 18's four
//!    rows beyond the
//!    interface, `close`, `driver` and `isOpen` land — the first over
//!    [`nvs_runtime::Ctx::close_open_connection`], which is § 13's release
//!    reached early for one connection — and **`serverVersion` is owed for a
//!    reason that is not this crate's**: no driver keeps the server's own
//!    version string. `nvs_db::mysql` parses one into a `(u16, u16, u16)` for
//!    its own capability decisions and the other four keep nothing, so the
//!    member cannot be written until PostgreSQL's `server_version`
//!    `ParameterStatus`, MariaDB's greeting, TDS's `LOGINACK` and SQLite's
//!    library version are each held on the connection. On
//!    the result side [`ROWS`] owes nothing: all six of § 18's members are
//!    registered, `columns()` among them. What that member cannot answer is
//!    one field rather than a member — [`COLUMN_NULLABLE_DOC`] states it — and
//!    it is a property of the PostgreSQL wire and not a gap in this module.
//!    — owner: m8-db-queue
//! 4. **`queryAs<T>` hydrates, and one of its refusals is still said per row
//!    rather than while compiling.** [`hydrate`] is the walk over
//!    [`nvs_runtime::ClassDesc::db_codec`] and it lands. Three of the ways the
//!    call is answered *no* are properties of the call site or of the class,
//!    and `nvs_types::derive`'s `check_row_sites` says all three as `E0806`
//!    while compiling: a `queryAs<array<C>>`, whose list form asks for the
//!    plural twice; a `T` carrying no `#[Db\Derive]` codec; and a mapping that
//!    fills fewer parameters than the constructor declares. What is left is a
//!    property of a *field* — a declared type the derive pass erased to
//!    [`nvs_runtime::CodecTy::Opaque`], a `decimal`, a `bytes` or an inline
//!    shape — and [`hydrate`] refuses it per row, because the erasure is
//!    `nvs_types::derive`'s own gap and there is no wire type behind it to
//!    decode into. The run-time refusals the compiling pass now covers stay
//!    under it as the backstop for a class built by hand, and each says so.
//!    Two smaller ones ride with the erased field: a constructor parameter no
//!    codec field fills is a fatal rather than `rule:core-classes/derive-field-list`'s default, for
//!    `crate::json`'s reason, and the refusals carry § 5's `issues` on a
//!    `ParseError` because `Db\DbError` has no `issues` slot to carry them —
//!    gap 2's other half.
//!    — owner: m8-db-queue

use std::net::{SocketAddr, ToSocketAddrs as _};

use rand::RngExt as _;

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreField, CoreMethod, CoreOption, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual, ShapeKeyDoc,
};

mod bind;
mod check;
mod column;
mod execute;
mod open;
mod plan;
mod pool;
mod registry;
mod row;
mod schema;
mod span;
mod stream;
mod transaction;

// A glob re-export takes each item at its own visibility, so what these lines
// decide is only the ceiling: `pub` for the modules holding a helper symbol or
// § 4's compile-time check, `pub(crate)` for the rows and the two types this
// crate reads elsewhere, and plain `use` where everything is `pub(super)` and
// the name is wanted only inside `db`.
use self::bind::*;
pub use self::check::*;
use self::column::*;
pub use self::execute::*;
pub use self::open::*;
pub(crate) use self::plan::*;
pub(crate) use self::pool::*;
pub(crate) use self::registry::*;
pub use self::row::*;
pub use self::schema::*;
pub(crate) use self::span::*;
pub use self::stream::*;
pub use self::transaction::*;

/// The class name, once, for the messages and the rows that all name it —
/// including `registry::CAPABILITIES`', which is why it is `pub(crate)`.
pub(crate) const NAME: &str = r"Core\Db";

/// The carrier class's fully-qualified name, as
/// [`CoreTy::Instance`] spells it.
///
/// `pub(crate)` for the one reader outside this module: `registry`'s
/// `a_class_with_slots_has_instance_members_and_the_reverse` names every class
/// that is a handle — slots and no members — and this is one of them.
pub(crate) const IN_LIST_NAME: &str = r"Core\Db\InList";

/// The one slot an [`IN_LIST`] holds: the array `inList` was given, kept whole
/// so the bind that expands it counts the same elements the caller passed.
const VALUES_SLOT: &str = "values";

/// `Core\Db\Connection`'s fully-qualified name, as [`CoreTy::Instance`] spells
/// it.
///
/// `pub(crate)` for `registry`'s handle roster, for [`IN_LIST_NAME`]'s reason.
pub(crate) const CONNECTION_NAME: &str = r"Core\Db\Connection";

/// A [`CONNECTION`]'s first slot: the key its connection is filed under in the
/// request's own table.
const HANDLE_SLOT: &str = "handle";

/// Its second: the `[db.<name>]` block it was opened by, so a refusal can name
/// the connection without holding it.
const CONNECTION_NAME_SLOT: &str = "name";

/// Where [`HANDLE_SLOT`] sits, for the members that read it back.
const HANDLE_AT: usize = 0;

/// Where [`CONNECTION_NAME_SLOT`] sits. See [`HANDLE_AT`].
const BLOCK_AT: usize = 1;

/// `Core\Db\Transaction`'s fully-qualified name, as [`CoreTy::Instance`] spells
/// it.
///
/// `pub(crate)` for `registry`'s handle roster, for [`IN_LIST_NAME`]'s reason.
pub(crate) const TRANSACTION_NAME: &str = r"Core\Db\Transaction";

/// A [`TRANSACTION`]'s third slot: whether the `transaction()` call that built
/// it is still running.
///
/// `rule:core-classes/db-transactions`'s first hazard, and the reason a [`TRANSACTION`] carries state
/// at all — one is passable down a call stack, so a program can hold one past
/// the call that owned it and every member has to say no.
const SCOPE_SLOT: &str = "open";

/// Its fourth: the reason `rollBack` was given, `null` until it is given one.
///
/// **§ 7's rollback-only flag and its message are one slot**, because they are
/// one fact: `rollBack` never sets the flag without a reason, and the reason is
/// what the `Core\Db\RolledBack` it throws carries. Two slots would be a state
/// this class can hold and § 7 cannot describe.
const REASON_SLOT: &str = "reason";

/// Where [`SCOPE_SLOT`] sits, for the guard every member of that class runs
/// first.
const SCOPE_AT: usize = 2;

/// Where [`REASON_SLOT`] sits. See [`SCOPE_AT`].
const REASON_AT: usize = 3;

/// `Core\Db\Rows`'s fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const ROWS_NAME: &str = r"Core\Db\Rows";

/// The symbol behind `Iterable<Row>::iterate()`, reached by name through this
/// class's method table rather than as a registered member — [`crate::cursor`]
/// and [`crate::instance`]'s dispatch roster own that protocol.
pub(crate) const ROWS_ITERATE_SYMBOL: &str = "nvs_core_db_rows_iterate";

/// A [`ROWS`]'s first slot: every row the statement answered, in the server's
/// order, each one a string-keyed array of its own columns.
const ROWS_SLOT: &str = "rows";

/// Where [`ROWS_SLOT`] sits, for the six members that read it back.
const ROWS_AT: usize = 0;

/// Its second: the class each row hydrates into, or `null` where the rows stay
/// `Core\Db\Row`s. [`nvs_core_db_connection_query`] writes the `null` and
/// [`nvs_core_db_connection_query_as`] the descriptor its call site named
/// ([`crate::registry::WRITTEN_CLASS_MEMBERS`]).
///
/// **It is read lazily, by the three members that hand a row out and by no
/// other** — so `value()`, `column()` and `count()` read the columns they
/// always did whichever member built the receiver, and a `queryAs<T>` whose
/// caller only counts pays for no construction at all. A descriptor rides in
/// the payload half of an otherwise-`null` value
/// ([`nvs_runtime::Value::class_desc`]), so this slot sweeps as the `null` it
/// is and holds no reference either way.
const ROWS_CLASS_SLOT: &str = "class";

/// Where [`ROWS_CLASS_SLOT`] sits. See [`ROWS_AT`].
const ROWS_CLASS_AT: usize = 1;

/// Its third: what the statement described, one [`COLUMN`] per column and in
/// the server's own order.
///
/// **Built with the result rather than on demand**, which is the opposite of
/// [`ROWS_CLASS_SLOT`] above and differs because the work does: a description
/// is per *statement* and bounded by the `select` list, while hydration is per
/// row and bounded by the traffic. It is also the only moment the material
/// exists — [`nvs_db::PgRows`] lends its row description out of the same borrow
/// the rows are read from — so keeping it for later would mean copying it
/// anyway, and copying it into anything but the objects it becomes would be
/// copying it twice.
const ROWS_COLUMNS_SLOT: &str = "columns";

/// Where [`ROWS_COLUMNS_SLOT`] sits. See [`ROWS_AT`].
const ROWS_COLUMNS_AT: usize = 2;

/// `Core\Db\Row`'s fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const ROW_NAME: &str = r"Core\Db\Row";

/// `Core\Db\Schema`'s fully-qualified name, as [`CoreTy::Instance`] spells it —
/// `rule:core-classes/schema-is-a-value`'s value.
pub(crate) const SCHEMA_NAME: &str = r"Core\Db\Schema";

/// The one slot a [`SCHEMA`] holds: that schema's canonical array form,
/// normalized when the value was built. [`mod@schema`]'s own doc is why the
/// array is the whole of the state.
const SCHEMA_ARRAY_SLOT: &str = "array";

/// Where [`SCHEMA_ARRAY_SLOT`] sits, for the members that read it back.
const SCHEMA_ARRAY_AT: usize = 0;

/// `Core\Db\Plan`'s fully-qualified name, as [`CoreTy::Instance`] spells it —
/// `rule:core-classes/schema-plan`'s document, and what `planAgainst` answers.
pub(crate) const PLAN_NAME: &str = r"Core\Db\Plan";

/// The one slot a [`PLAN`] holds: its steps, in the order they must run.
const PLAN_STEPS_SLOT: &str = "steps";

/// Where [`PLAN_STEPS_SLOT`] sits.
const PLAN_STEPS_AT: usize = 0;

/// `Core\Db\Plan\Step`'s fully-qualified name — one difference, with the reason
/// it costs what it costs. A class under a class's name, exactly as
/// `Core\Queue\Id` is.
pub(crate) const STEP_NAME: &str = r"Core\Db\Plan\Step";

/// A [`STEP`]'s grade, as the [`GRADE`] case value `rule:enums/closed-integer-type` makes an enum.
const STEP_GRADE_SLOT: &str = "grade";

/// Where [`STEP_GRADE_SLOT`] sits.
const STEP_GRADE_AT: usize = 0;

/// A [`STEP`]'s one-sentence reason — § 6's, written by the emitter that graded
/// it.
const STEP_REASON_SLOT: &str = "reason";

/// Where [`STEP_REASON_SLOT`] sits.
const STEP_REASON_AT: usize = 1;

/// A [`STEP`]'s complete SQL, newline-joined — [`mod@plan`]'s doc owns why the
/// list is flattened here and not in `nvs-db`.
const STEP_SQL_SLOT: &str = "sql";

/// Where [`STEP_SQL_SLOT`] sits.
const STEP_SQL_AT: usize = 2;

/// Whether a [`STEP`] is § 7's report, carried and never applied.
const STEP_REPORT_SLOT: &str = "report";

/// Where [`STEP_REPORT_SLOT`] sits.
const STEP_REPORT_AT: usize = 3;

/// [`GRADE`]'s fully-qualified name, written once so the enum, the slot that
/// holds one of its cases and every message quoting it cannot drift apart.
pub(crate) const GRADE_NAME: &str = r"Core\Db\Plan\Grade";

/// The one slot a [`ROW`] holds: that row's own columns, string-keyed and in
/// the server's order — **the very array [`ROWS_SLOT`] already holds one of per
/// row**, handed on under a second reference rather than copied, so `all()` over
/// a thousand rows allocates a thousand objects and not a second thousand
/// arrays.
const COLUMNS_SLOT: &str = "columns";

/// Where [`COLUMNS_SLOT`] sits. See [`ROWS_AT`].
const COLUMNS_AT: usize = 0;

/// `Core\Db\Stream`'s fully-qualified name, as [`CoreTy::Instance`] spells it —
/// spec § 18's `Iterable<Db\Row>`, which needs a class name because
/// [`CoreTy::Iterated`] is parameter position only.
///
/// `pub(crate)` for `registry`'s handle roster and its `ITERABLES` row, for
/// [`IN_LIST_NAME`]'s reason.
pub(crate) const STREAM_NAME: &str = r"Core\Db\Stream";

/// The symbol behind `Iterable<Db\Row>::iterate()` — [`ROWS_ITERATE_SYMBOL`]'s
/// twin, and see [`mod@stream`] for why this class carries the other two names
/// as well rather than handing a [`crate::cursor`] back.
pub(crate) const STREAM_ITERATE_SYMBOL: &str = "nvs_core_db_stream_iterate";

/// The symbol behind `Iterator<Db\Row>::advance()`.
pub(crate) const STREAM_ADVANCE_SYMBOL: &str = "nvs_core_db_stream_advance";

/// The symbol behind `Iterator<Db\Row>::current()`.
pub(crate) const STREAM_CURRENT_SYMBOL: &str = "nvs_core_db_stream_current";

/// A [`STREAM`]'s third slot: the row the last `advance()` read, `null` before
/// the first one and after the last.
///
/// **Its first two are [`CONNECTION`]'s, in the same positions and under the
/// same names**, for [`TRANSACTION`]'s reason — a walk reaches its connection
/// through the same key any other member does.
const STREAM_ROW_SLOT: &str = "row";

/// Where [`STREAM_ROW_SLOT`] sits. See [`HANDLE_AT`].
const STREAM_ROW_AT: usize = 2;

/// `Core\Db\Write`'s fully-qualified name, as [`CoreTy::Instance`] spells it.
const WRITE_NAME: &str = r"Core\Db\Write";

/// A [`WRITE`]'s first slot: how many rows the statement affected, as a `uint`
/// and never absent — [`CHANGED_SLOT`] is where the absence is kept.
const AFFECTED_SLOT: &str = "affected";

/// Its second: the same count as the server actually reported it, `null` for a
/// command whose tag carries no count at all.
const CHANGED_SLOT: &str = "changed";

/// Its third: `rule:core-classes/db-statement-members`'s `lastId`, `null` for a statement that returned no
/// integer first column — which is every statement without a `RETURNING`
/// clause.
const LAST_ID_SLOT: &str = "lastId";

/// Where [`AFFECTED_SLOT`] sits, for the reader that answers it.
const AFFECTED_AT: usize = 0;

/// Where [`CHANGED_SLOT`] sits. See [`AFFECTED_AT`].
const CHANGED_AT: usize = 1;

/// Where [`LAST_ID_SLOT`] sits. See [`AFFECTED_AT`].
const LAST_ID_AT: usize = 2;

/// `Core\Db\Column`'s fully-qualified name, as [`CoreTy::Instance`] spells it.
const COLUMN_NAME: &str = r"Core\Db\Column";

/// A [`COLUMN`]'s first slot: the label the server described the column with.
const LABEL_SLOT: &str = "name";

/// Its second: [`COLUMN_TYPE`]'s case for that column, held as the ordinal an
/// enum *is* at runtime (`rule:enums/closed-integer-type`).
/// [`column_type_value`] is where a [`nvs_db::ColumnType`] becomes one.
const DECLARED_SLOT: &str = "type";

/// Its third: whether the column may hold NULL, which on this driver is always
/// `true` — [`COLUMN_NULLABLE_DOC`] owns why. A slot rather than a constant
/// because a backend whose description carries the flag fills it here without
/// the member moving.
const NULLABLE_SLOT: &str = "nullable";

/// Where [`LABEL_SLOT`] sits, for the reader that answers it.
const LABEL_AT: usize = 0;

/// Where [`DECLARED_SLOT`] sits. See [`LABEL_AT`].
const DECLARED_AT: usize = 1;

/// Where [`NULLABLE_SLOT`] sits. See [`LABEL_AT`].
const NULLABLE_AT: usize = 2;

/// Where [`VALUES_SLOT`] sits inside an [`IN_LIST`], for the bind that expands
/// it.
const VALUES_AT: usize = 0;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_db_connect" => (nvs_core_db_connect as *const ()).cast(),
        "nvs_core_db_open" => (nvs_core_db_open as *const ()).cast(),
        "nvs_core_db_in_list" => (nvs_core_db_in_list as *const ()).cast(),
        "nvs_core_db_quote_identifier" => (nvs_core_db_quote_identifier as *const ()).cast(),
        "nvs_core_db_connection_query" => (nvs_core_db_connection_query as *const ()).cast(),
        "nvs_core_db_connection_query_as" => (nvs_core_db_connection_query_as as *const ()).cast(),
        "nvs_core_db_connection_execute" => (nvs_core_db_connection_execute as *const ()).cast(),
        "nvs_core_db_connection_execute_many" => {
            (nvs_core_db_connection_execute_many as *const ()).cast()
        }
        // One arm for both classes' rows: `Core\Db\Transaction` declares
        // `transaction` under this symbol too, which is what `rule:classes/no-traits`'s
        // delegation is here — see [`TRANSACTION`].
        "nvs_core_db_connection_transaction" => {
            (nvs_core_db_connection_transaction as *const ()).cast()
        }
        // One arm for both classes' rows again, for the transaction arm's
        // reason: § 18 puts `stream` on `Core\Db\Queryable`.
        "nvs_core_db_connection_stream" => (nvs_core_db_connection_stream as *const ()).cast(),
        STREAM_ITERATE_SYMBOL => (nvs_core_db_stream_iterate as *const ()).cast(),
        STREAM_ADVANCE_SYMBOL => (nvs_core_db_stream_advance as *const ()).cast(),
        STREAM_CURRENT_SYMBOL => (nvs_core_db_stream_current as *const ()).cast(),
        "nvs_core_db_connection_close" => (nvs_core_db_connection_close as *const ()).cast(),
        "nvs_core_db_connection_driver" => (nvs_core_db_connection_driver as *const ()).cast(),
        "nvs_core_db_connection_is_open" => (nvs_core_db_connection_is_open as *const ()).cast(),
        "nvs_core_db_transaction_roll_back" => {
            (nvs_core_db_transaction_roll_back as *const ()).cast()
        }
        "nvs_core_db_rows_all" => (nvs_core_db_rows_all as *const ()).cast(),
        ROWS_ITERATE_SYMBOL => (nvs_core_db_rows_iterate as *const ()).cast(),
        "nvs_core_db_rows_first" => (nvs_core_db_rows_first as *const ()).cast(),
        "nvs_core_db_rows_value" => (nvs_core_db_rows_value as *const ()).cast(),
        "nvs_core_db_rows_column" => (nvs_core_db_rows_column as *const ()).cast(),
        "nvs_core_db_rows_count" => (nvs_core_db_rows_count as *const ()).cast(),
        "nvs_core_db_rows_columns" => (nvs_core_db_rows_columns as *const ()).cast(),
        "nvs_core_db_column_name" => (nvs_core_db_column_name as *const ()).cast(),
        "nvs_core_db_column_type" => (nvs_core_db_column_type as *const ()).cast(),
        "nvs_core_db_column_nullable" => (nvs_core_db_column_nullable as *const ()).cast(),
        "nvs_core_db_row_has" => (nvs_core_db_row_has as *const ()).cast(),
        "nvs_core_db_row_get" => (nvs_core_db_row_get as *const ()).cast(),
        "nvs_core_db_row_to_array" => (nvs_core_db_row_to_array as *const ()).cast(),
        "nvs_core_db_row_string" => (nvs_core_db_row_string as *const ()).cast(),
        "nvs_core_db_row_bytes" => (nvs_core_db_row_bytes as *const ()).cast(),
        "nvs_core_db_row_int" => (nvs_core_db_row_int as *const ()).cast(),
        "nvs_core_db_row_uint" => (nvs_core_db_row_uint as *const ()).cast(),
        "nvs_core_db_row_float" => (nvs_core_db_row_float as *const ()).cast(),
        "nvs_core_db_row_bool" => (nvs_core_db_row_bool as *const ()).cast(),
        "nvs_core_db_row_decimal" => (nvs_core_db_row_decimal as *const ()).cast(),
        "nvs_core_db_row_instant" => (nvs_core_db_row_instant as *const ()).cast(),
        "nvs_core_db_row_date" => (nvs_core_db_row_date as *const ()).cast(),
        "nvs_core_db_row_time" => (nvs_core_db_row_time as *const ()).cast(),
        "nvs_core_db_row_uuid" => (nvs_core_db_row_uuid as *const ()).cast(),
        "nvs_core_db_schema_from_array" => (nvs_core_db_schema_from_array as *const ()).cast(),
        "nvs_core_db_schema_to_array" => (nvs_core_db_schema_to_array as *const ()).cast(),
        "nvs_core_db_schema_plan_against" => (nvs_core_db_schema_plan_against as *const ()).cast(),
        "nvs_core_db_schema_apply_safe" => (nvs_core_db_schema_apply_safe as *const ()).cast(),
        "nvs_core_db_schema_apply_including_risky" => {
            (nvs_core_db_schema_apply_including_risky as *const ()).cast()
        }
        "nvs_core_db_plan_steps" => (nvs_core_db_plan_steps as *const ()).cast(),
        "nvs_core_db_plan_step_grade" => (nvs_core_db_plan_step_grade as *const ()).cast(),
        "nvs_core_db_plan_step_reason" => (nvs_core_db_plan_step_reason as *const ()).cast(),
        "nvs_core_db_plan_step_sql" => (nvs_core_db_plan_step_sql as *const ()).cast(),
        "nvs_core_db_plan_step_is_refused" => {
            (nvs_core_db_plan_step_is_refused as *const ()).cast()
        }
        "nvs_core_db_write_affected" => (nvs_core_db_write_affected as *const ()).cast(),
        "nvs_core_db_write_changed" => (nvs_core_db_write_changed as *const ()).cast(),
        "nvs_core_db_write_last_id" => (nvs_core_db_write_last_id as *const ()).cast(),
        _ => return None,
    })
}
