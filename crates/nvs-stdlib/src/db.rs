//! `Core\Db` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 18's entry points, over [`nvs_db`]'s wire half.
//!
//! [ADR 0067](../../../../docs/adr/0067-core-db.md) is authoritative for every
//! semantic and § 18 for every signature. What belongs here is the two
//! decisions this side of the boundary owns: what an `InList` *is* once it is a
//! value a program holds, and what `quoteIdentifier` can honestly promise from
//! a class that has no connection in front of it.
//!
//! # `inList` is a carrier, and the expansion stays on the wire
//!
//! ADR 0067 § 5 makes `Core\Db::inList($values)` the explicit marker that a
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
//! What laundering owes [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
//! § 4 is that the result cannot end the identifier and start something else.
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
//! # Known gaps
//!
//! 1. **An `open` pool's bounds are the defaults, and nothing an operator
//!    writes can change them.** ADR 0067 § 13's key for `open` is live —
//!    [`settings_key`] is the hash, [`nvs_runtime::pool::Ticket::for_settings`]
//!    is the ticket, and a released connection rejoins this core's pool under it
//!    exactly as a `connect`ed one does. What has no spelling is the operator's
//!    half: `max`, `idle`, `lifetime` and `acquire` are `PoolBounds::DEFAULT`
//!    because a settings literal has no `[db.<name>.pool]` table to read them
//!    from, and § 13's `pool = false` is written *per block* and so cannot reach
//!    an `open` at all. The deployment that notices is the audited one that
//!    needs every connection to map to one request: it can switch off every
//!    block an operator wrote and not the connections a program opens for
//!    itself. Where those bounds would be written, for a key only the program
//!    knows, is an ADR 0067 § 13 question and not a shape this module may pick
//!    on its own.
//!
//!    ADR 0135 § 2's *exactly one arm accepts it* **is** the checker's rule —
//!    `nvs_types::expr::args`' `select_arm` — so a `host` written beside
//!    `Driver::Sqlite` is the compile error § 18 says it is, and a key only one
//!    arm requires is required of the call that selected that arm. What reaches
//!    [`settings_driver`] is therefore a literal one arm has already accepted,
//!    which is why it reads the discriminant before it reads anything else and
//!    why every slot it then reads is filled.
//! 2. **Three drivers open, and everything past the handshake follows.**
//!    `connect`
//!    branches on the block's `driver` — ADR 0067 § 2 — so a `postgres` block,
//!    a `mysql` block and a `mariadb` block each reach their own target, their
//!    own default port and their own `nvs_db::Connection` variant, and `open`
//!    branches the same three ways on the settings hash's own `driver`. A block
//!    naming either of the other two is still refused by
//!    `nvs_db::PgTarget::resolve` with the
//!    message that names the driver it is, which is the honest answer while
//!    those variants have no connect path behind them. Past the handshake the
//!    list is shorter than that. Binding is whole: [`rendering_of`] pairs § 5's
//!    dialect with § 9's encoder off the connection's own [`nvs_db::Driver`],
//!    so a MySQL statement is rewritten to `?` and bound as MySQL reads a
//!    parameter. Sending is not: [`queried_rows`] branches on the connection
//!    and [`mysql_rows`] drains a binary result set through § 9's decode, so
//!    `query`, `queryAs`, `execute` and `executeMany` answer on either driver,
//!    § 11's event included. § 7's `transaction` does too, over [`Transacting`]
//!    — the drivers' commands differ and `nvs_db::mysql`'s `begin` owns how,
//!    but the five points this module asks them at do not. MySQL and MariaDB
//!    reach all of it through one body rather than two: [`Framed`] is that
//!    seam, and its doc is where "its own driver above the framing, not inside
//!    it" is argued. **SQL Server reads, once one is open**: [`rendering_for`]
//!    binds a parameter through `nvs_db::tds::encode` and [`tds_rows`] drains
//!    the token stream, so `query` and `queryAs` answer on it — but `connect`
//!    and `open` have no arm that calls `nvs_db::TdsConn::connect`, so nothing
//!    in a program can hold one yet and that arm is what stands between this
//!    and § 4 running on the driver. `execute` has the same driver method
//!    behind it as `query` and no arm here either; § 4's batch and § 7's
//!    commands have no `nvs_db::TdsConn` primitive at all. What is still
//!    PostgreSQL-only is [`crate::queue`]'s four members. Known gap 2 above the
//!    handshake is therefore a roster per member rather than one list, which is
//!    what [`driverless`] takes.
//! 3. **A driver with a reset behind it is pooled, and SQLite is the one
//!    without.** ADR 0067 § 13's pool is
//!    on disk as [`nvs_runtime::pool`], a connection is *released* to it at
//!    teardown under the ticket `Core\Db::connect` files, and
//!    [`warm_connection`] takes one back out behind that section's reset. A
//!    connection filed by any other driver is dropped there rather than reset,
//!    because a reset nobody has written is not a reset that failed: § 13
//!    makes the reset a security boundary, and the only safe reading of a
//!    missing one is that the connection is not poolable.
//! 4. **`Db\DbError` declares all five of § 18's values.**
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
//! 5. **`query`, `queryAs`, `execute`, `executeMany` and `transaction` are what
//!    has landed of `Core\Db\Queryable`** (gap 8 is what `queryAs` still owes).
//!    `stream` and `streamAs` are owed whole, and so are
//!    `close` and § 18's three readonly properties on `Connection`. On
//!    the result side [`ROWS`] owes nothing: all six of § 18's members are
//!    registered, `columns()` among them. What that member cannot answer is
//!    one field rather than a member — [`COLUMN_NULLABLE_DOC`] states it — and
//!    it is a property of the PostgreSQL wire and not a gap in this module.
//! 6. **Neither `query` nor `execute` declares a `{timeout?: Duration}`.**
//!    § 4's option is in both spec signatures and is deliberately in neither
//!    registry row, for one reason on both: a deadline
//!    on a statement has to reach the socket the way
//!    [`nvs_db::PgConn::connect`]'s does, and there is no seam for one on the
//!    statement path yet. An option that parsed and did nothing would be worse
//!    than its absence, which the compiler can at least report.
//! 7. **A delimiting quoter, if one is ever wanted, belongs on `Connection`**
//!    and not here — that is the only place a dialect exists. § 18 does not ask
//!    for one, and this module's second decision above is why adding it to
//!    `Core\Db` cannot be the answer.
//! 8. **`queryAs<T>` hydrates, and three of its refusals are at run time that
//!    should be at compile time.** [`hydrate`] is the walk over
//!    [`nvs_runtime::ClassDesc::db_codec`] and it lands; what is owed is where
//!    the *no* is said. A `T` carrying no `#[Db\Derive]` codec and a
//!    `queryAs<array<C>>` whose list form means nothing here are both
//!    properties of the call site alone, and a field whose declared type the
//!    derive pass erased to [`nvs_runtime::CodecTy::Opaque`] — a `decimal`, a
//!    `bytes`, an inline shape — is a property of the class alone; all three
//!    are refused per row instead. Both bands the checker would take a code
//!    from (`E04xx`, `E07xx`) are full, so they are the helper's until a band
//!    is opened. Two smaller ones ride with them: a constructor parameter no
//!    codec field fills is a fatal rather than ADR 0071 § 3's default, for
//!    `crate::json`'s reason, and the refusals carry § 5's `issues` on a
//!    `ParseError` because `Db\DbError` has no `issues` slot to carry them —
//!    gap 4's other half, spec § 10 giving it that property too.

use std::net::{SocketAddr, ToSocketAddrs as _};

use rand::RngExt as _;

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreField, CoreMethod, CoreOption, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual, ShapeKeyDoc,
};

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
/// ADR 0067 § 7's first hazard, and the reason a [`TRANSACTION`] carries state
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

/// The one slot a [`ROW`] holds: that row's own columns, string-keyed and in
/// the server's order — **the very array [`ROWS_SLOT`] already holds one of per
/// row**, handed on under a second reference rather than copied, so `all()` over
/// a thousand rows allocates a thousand objects and not a second thousand
/// arrays.
const COLUMNS_SLOT: &str = "columns";

/// Where [`COLUMNS_SLOT`] sits. See [`ROWS_AT`].
const COLUMNS_AT: usize = 0;

/// `Core\Db\Write`'s fully-qualified name, as [`CoreTy::Instance`] spells it.
const WRITE_NAME: &str = r"Core\Db\Write";

/// A [`WRITE`]'s first slot: how many rows the statement affected, as a `uint`
/// and never absent — [`CHANGED_SLOT`] is where the absence is kept.
const AFFECTED_SLOT: &str = "affected";

/// Its second: the same count as the server actually reported it, `null` for a
/// command whose tag carries no count at all.
const CHANGED_SLOT: &str = "changed";

/// Its third: ADR 0067 § 4's `lastId`, `null` for a statement that returned no
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
/// enum *is* at runtime ([ADR 0010](../../../../docs/adr/0010-enums-are-a-value-type.md)).
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

/// Whether a literal query holds the one statement [ADR 0067
/// § 1](../../../docs/adr/0067-core-db.md) prepares — § 10's "a refused second
/// statement", over [`nvs_db::sql::holds_a_second_statement`].
///
/// All four dialects, for [`check_literal_query`]'s reason and with the same
/// direction: a `;` that only one of them reads as a separator is a `;` this
/// pass says nothing about.
///
/// # Errors
///
/// One message, since there is only one thing this can find.
pub fn check_single_statement(sql: &str) -> Result<(), String> {
    let split = [
        nvs_db::sql::Dialect::PostgreSql,
        nvs_db::sql::Dialect::MySql,
        nvs_db::sql::Dialect::Sqlite,
        nvs_db::sql::Dialect::SqlServer,
    ]
    .into_iter()
    .all(|dialect| nvs_db::sql::holds_a_second_statement(sql, dialect));
    if split {
        return Err(
            "a statement text holds one statement, and this holds two — every statement is \
             prepared, and a prepared statement is one command on every backend"
                .to_owned(),
        );
    }
    Ok(())
}

/// A literal params array as far as the *compiler* can read one — [ADR 0067
/// § 10](../../../docs/adr/0067-core-db.md)'s "a literal params array", which is
/// the only shape it promises anything about.
///
/// § 5's two spellings are exclusive, so this is two cases and not a pair: a
/// list-keyed array is positional and a string-keyed one is named. An array the
/// checker cannot read whole — a spread, a variable, a mixture of keyed and
/// unkeyed elements — is not one of these at all, and the call is left to run
/// time.
#[derive(Debug, Clone, Copy)]
pub enum LiteralParams<'a> {
    /// A list-keyed array literal, and how many elements it has.
    Positional(usize),
    /// A string-keyed array literal, its keys in written order.
    Named(&'a [&'a str]),
}

/// Whether a literal query agrees with the literal params array written beside
/// it — [ADR 0067 § 10](../../../docs/adr/0067-core-db.md)'s "placeholder count
/// against a literal params array, positional-vs-named consistency".
///
/// It is [`nvs_db::sql::rewrite`] itself and deliberately not a second reader of
/// the same grammar: ADR 0057 § 4 asks that this pass produce an earlier answer
/// and never a different one, which is a property of *asking the runtime's own
/// question* rather than of two scanners being kept in step. The message handed
/// back is the one the first call would have thrown.
///
/// **A refusal must hold on every dialect.** The scan is dialect-shaped — a `?`
/// inside backticks is a placeholder on PostgreSQL and text on MySQL — and a
/// call site does not name the driver its `[db.<name>]` block will resolve to.
/// So all four are asked and the first acceptance ends it, which is the
/// soundness direction ADR 0057 § 4 fixes: a refusal that would be a guess is
/// not made. Where they all refuse, PostgreSQL's wording is the one reported,
/// since two dialects can refuse the same query over different placeholder
/// counts.
///
/// # Errors
///
/// The rewriter's own message, where every dialect refused.
pub fn check_literal_query(sql: &str, params: LiteralParams<'_>) -> Result<(), String> {
    let positional;
    let named;
    let params = match params {
        LiteralParams::Positional(count) => {
            positional = vec![nvs_db::sql::Binding::One; count];
            nvs_db::sql::Params::Positional(&positional)
        }
        LiteralParams::Named(keys) => {
            named = keys
                .iter()
                .map(|key| (*key, nvs_db::sql::Binding::One))
                .collect::<Vec<_>>();
            nvs_db::sql::Params::Named(&named)
        }
    };
    let mut stated = String::new();
    for dialect in [
        nvs_db::sql::Dialect::PostgreSql,
        nvs_db::sql::Dialect::MySql,
        nvs_db::sql::Dialect::Sqlite,
        nvs_db::sql::Dialect::SqlServer,
    ] {
        match nvs_db::sql::rewrite(sql, params, dialect) {
            Ok(_) => return Ok(()),
            Err(refused) if stated.is_empty() => stated = refused.to_string(),
            Err(_) => {}
        }
    }
    Err(stated)
}

/// Spec § 18's `Db\Settings` — the two arms `open` takes, in the spec's own
/// order, per [ADR 0135](../../../docs/adr/0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md)
/// § 1.
///
/// **The arms are separated by their `driver` and by nothing else that is
/// declared.** § 2's arm selection is "exactly one arm accepts it", and ADR
/// 0047's enum-case types are what make these two disjoint: a literal naming
/// `Driver::Sqlite` cannot satisfy the server arm's required `driver`, and one
/// naming any other case cannot satisfy the SQLite arm's. Nothing here says
/// `driver` *is* a discriminant, because saying so would be a second, weaker
/// spelling of the disjointness the types already carry, and
/// `a_shapes_arms_are_pairwise_disjoint` proves it from the types rather than
/// from a declaration.
///
/// **Every qualifier classification lands on a field**, § 3's rule: `host` is
/// a [`Qual::Sink`] because ADR 0067 § 3 makes an address one and gives it no
/// launderer, `path` is one because a program-supplied SQLite file is a path
/// sink, `database` and `user` accept `tainted` freely as length-prefixed
/// protocol fields, and `password` is `secret tainted string`. The parameter
/// as a whole classifies nothing.
///
/// **The merged list is twelve slots long**, § 3's ABI: these ten in order,
/// then the SQLite arm's `path` — `driver`, `timeZone` and `timeout` are
/// already spoken for — then the trailing bag's `shared`. [`DRIVER_ARG`] and
/// the eleven consts under it are that list as slot indices, and they are the
/// only place the numbers are written.
const SETTINGS: &[&[CoreField]] = &[
    &[
        CoreField {
            name: "driver",
            // The four backends that take a host. Written as the union of the
            // cases rather than as the bare enum, because the bare enum would
            // admit `Driver::Sqlite` here and the two arms would stop being
            // disjoint — the arm below is the one that spells that case.
            ty: CoreTy::Union(&[
                CoreTy::EnumCase(DRIVER_NAME, "MySql"),
                CoreTy::EnumCase(DRIVER_NAME, "MariaDb"),
                CoreTy::EnumCase(DRIVER_NAME, "Postgres"),
                CoreTy::EnumCase(DRIVER_NAME, "SqlServer"),
            ]),
            default: None,
        },
        CoreField {
            name: "host",
            ty: CoreTy::Text(Qual::Sink),
            default: None,
        },
        CoreField {
            name: "port",
            // `Const::Null` and not a number: which port an absent one means
            // is the *driver's*, and there is no one answer to write here —
            // 5432 and 3306 are different servers. The helper reads the
            // driver first and falls back to its own default.
            ty: CoreTy::Uint,
            default: Some(Const::Null),
        },
        CoreField {
            name: "database",
            ty: CoreTy::TaintedStr,
            default: None,
        },
        CoreField {
            name: "user",
            ty: CoreTy::TaintedStr,
            default: None,
        },
        CoreField {
            name: "password",
            ty: CoreTy::SecretTaintedStr,
            default: None,
        },
        CoreField {
            name: "tls",
            ty: CoreTy::Enum(TLS_NAME),
            default: Some(Const::Null),
        },
        CoreField {
            name: "timeZone",
            ty: CoreTy::Instance(crate::time::ZONE_NAME),
            default: Some(Const::Null),
        },
        CoreField {
            name: "timeout",
            ty: CoreTy::Instance(crate::time::DURATION_NAME),
            default: Some(Const::Null),
        },
        CoreField {
            name: "statementCache",
            ty: CoreTy::Uint,
            default: Some(Const::Null),
        },
    ],
    &[
        CoreField {
            name: "driver",
            ty: CoreTy::EnumCase(DRIVER_NAME, "Sqlite"),
            default: None,
        },
        CoreField {
            name: "path",
            ty: CoreTy::Text(Qual::Sink),
            default: None,
        },
        CoreField {
            name: "timeZone",
            ty: CoreTy::Instance(crate::time::ZONE_NAME),
            default: Some(Const::Null),
        },
        CoreField {
            name: "timeout",
            ty: CoreTy::Instance(crate::time::DURATION_NAME),
            default: Some(Const::Null),
        },
    ],
];

/// Spec § 18's `Core\Db` — `connect`, `open`, and the two connectionless entry
/// points, in the spec's own order.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "connect",
            names: &["name"],
            params: &[
                // § 18's Q column: the name is a **sink**, because it selects
                // which operator-written credential this program opens with,
                // and a `tainted` one would let a request pick the database.
                CoreTy::Text(Qual::Sink),
                CoreTy::Options(&[
                    CoreOption {
                        name: "shared",
                        ty: CoreTy::Bool,
                        // Memoized is the default and the option only turns it
                        // off, which is ADR 0067 § 2's `{shared: false}`.
                        default: Const::Bool(true),
                    },
                    CoreOption {
                        name: "timeout",
                        ty: CoreTy::Instance(crate::time::DURATION_NAME),
                        default: Const::Null,
                    },
                ]),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(CONNECTION_NAME),
            symbol: "nvs_core_db_connect",
            doc: Some(&CONNECT_DOC),
        },
        CoreMethod {
            name: "open",
            names: &["settings"],
            params: &[
                // § 18's Q column reads **sink (host)**, and that is where the
                // classification sits: on [`SETTINGS`]' own field, not here.
                CoreTy::Shape(SETTINGS),
                CoreTy::Options(&[CoreOption {
                    name: "shared",
                    ty: CoreTy::Bool,
                    // `connect`'s default, for `connect`'s reason — ADR 0067
                    // § 2 memoizes by default and the option only turns it off.
                    default: Const::Bool(true),
                }]),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(CONNECTION_NAME),
            symbol: "nvs_core_db_open",
            doc: Some(&OPEN_DOC),
        },
        CoreMethod {
            name: "inList",
            names: &["values"],
            params: &[CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Instance(IN_LIST_NAME),
            symbol: "nvs_core_db_in_list",
            doc: Some(&IN_LIST_DOC),
        },
        CoreMethod {
            name: "quoteIdentifier",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_db_quote_identifier",
            doc: Some(&QUOTE_IDENTIFIER_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// Spec § 18's `Core\Db\Connection` — the object `connect` answers with.
///
/// The slots are the pair every handle in this crate carries — the key into the
/// request's own table ([`nvs_runtime::Ctx::hold_open_connection`]) and the name
/// it was opened by, which is what a refusal can name without reaching for the
/// connection it is refusing about.
///
/// **`query`, `execute`, `executeMany` and `transaction` are
/// `Core\Db\Queryable`'s landed members, `queryAs` is declared with its body
/// owed (this module's gap 8), and the rest are owed whole**: `stream` and
/// `streamAs`, plus `close` and § 18's three readonly properties.
/// ADR 0043 makes `Transaction` delegate the interface to its connection, so
/// every one of them is declared once — here — and [`TRANSACTION`] is where the
/// forwarding lands.
pub(crate) const CONNECTION: CoreClass = CoreClass {
    name: CONNECTION_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "query",
            names: &["sql", "params"],
            params: &[
                // § 4's Q column, and ADR 0024 § 4's whole injection story: the
                // statement text is the sink, so a `tainted` value cannot reach
                // it at all and the bound parameters below accept one freely.
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
            ],
            defaults: &[],
            // § 18's `Rows<Row>` — [`ROWS`] at the one concrete argument an
            // unhydrated result set has, and `queryAs<T>` is the same class at
            // whatever the call site wrote. A [`CoreTy::InstanceAt`] and not a
            // [`CoreTy::Instance`]: the bare spelling would intern the class at
            // its *own* `T`, a variable no call site of `query` ever binds.
            // Written out on both classes rather than named once, because
            // `tools/gaps.py` attributes a case to the class a member answers
            // by reading this very line.
            return_ty: CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Instance(ROW_NAME)]),
            symbol: "nvs_core_db_connection_query",
            doc: Some(&QUERY_DOC),
        },
        CoreMethod {
            name: "queryAs",
            names: &["sql", "params"],
            // `query`'s two parameters exactly: § 4 makes this the same
            // statement, read the same way, and the only difference is what
            // each row becomes.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            // § 18's `Rows<T>` at the `T` the call site wrote — the whole
            // reason [`CoreTy::Written`] descends into a
            // [`CoreTy::InstanceAt`]. It is also what makes this member
            // generic: `CoreMethod::written` finds the `T` here and nowhere
            // else, so a call naming no type argument is `E0442`.
            return_ty: CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Written("T")]),
            symbol: "nvs_core_db_connection_query_as",
            doc: Some(&QUERY_AS_DOC),
        },
        CoreMethod {
            name: "execute",
            names: &["sql", "params"],
            // The same two [`CoreTy`]s `query` above declares, for the same
            // reasons — § 4's Q column marks both members' statement text a sink,
            // and a write is exactly where a `tainted` value most wants to reach
            // one.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Instance(WRITE_NAME),
            symbol: "nvs_core_db_connection_execute",
            doc: Some(&EXECUTE_DOC),
        },
        CoreMethod {
            name: "executeMany",
            names: &["sql", "sets"],
            params: &[
                // The same sink, and it is the member's whole point that there
                // is only one of them: § 4's batch is one statement run many
                // times, so the text is written once and cannot pick up a
                // `tainted` fragment per set.
                CoreTy::Text(Qual::Sink),
                // § 18's `array<array<mixed>>`. The outer array is the sets and
                // the inner one is `query`'s own `$params`, which is why each
                // set is read by [`statement_of`] and gains no binding rule of
                // its own.
                CoreTy::Array(&CoreTy::Array(&CoreTy::Mixed)),
            ],
            defaults: &[],
            // A bare `uint` and not a [`WRITE`]: § 4 gives the batch a sum and
            // no second return to hand rows or a `lastId` back through, because
            // there is no one execution for either to belong to.
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_db_connection_execute_many",
            doc: Some(&EXECUTE_MANY_DOC),
        },
        TRANSACTION_ROW,
    ],
    slots: &[HANDLE_SLOT, CONNECTION_NAME_SLOT],
    constants: &[],
};

/// ADR 0067 § 7's `{isolation?, readOnly?, retries?}` — the bag
/// [`TRANSACTION_ROW`] declares last, per ADR 0063 R2.
///
/// **Every default is § 7's own and each is a decision rather than a
/// placeholder.** `isolation` is [`Const::Null`] and so *absent*, which is the
/// only default that does not silently overrule the level the operator set on
/// the server: the transaction runs at the connection's own. `readOnly` is
/// false because a transaction is asked for by code that writes. `retries` is 0
/// because § 7 says so out loud — a closure may have side effects that are not
/// the database's, and re-running one that sends mail is worse than surfacing
/// the conflict to the caller who can decide.
///
/// **`isolation` is the enum, not its ordinal.** Typing it
/// [`CoreTy::Enum`] is what makes a level [`ISOLATION`] does not list an
/// `E0401` at the call site rather than an integer arriving in the helper for
/// it to re-check; [`isolation_of`] is the one place the two rosters are
/// matched.
const TRANSACTION_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "isolation",
        ty: CoreTy::Enum(ISOLATION_NAME),
        default: Const::Null,
    },
    CoreOption {
        name: "readOnly",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "retries",
        ty: CoreTy::Uint,
        default: Const::Uint(0),
    },
];

/// ADR 0067 § 7's `transaction`, written once because it is declared once: the
/// row is `Core\Db\Queryable`'s and both [`CONNECTION`] and [`TRANSACTION`]
/// carry it, a nested call on the second being the savepoint § 7 asks for.
///
/// **Two of [`TRANSACTION_OPTIONS`]' three reach the `BEGIN` and the third is
/// the loop around it.** [`nvs_db::PgConn::begin`] takes the level and the
/// read-only flag and renders the command from them, and refuses a *nested*
/// call that carries either rather than running it at the outer transaction's
/// level. `retries` is read by the helper instead, which re-runs the closure on
/// a conflict the driver says may be re-run, after the wait § 7 asks for.
/// [`retry_backoff`] draws that wait exponentially and with full jitter and
/// [`wait_between_attempts`] gives the core back for it; the declared default
/// of 0 is what every call that does not ask gets.
const TRANSACTION_ROW: CoreMethod = CoreMethod {
    name: "transaction",
    names: &["fn"],
    // Opaque, as ADR 0031 § 4 keeps every `callable`: what this one is handed
    // is a [`TRANSACTION`] and what it may declare is zero parameters or one,
    // and neither is sayable here — `nvs_runtime::call_closure` trims to the
    // arity the closure recorded, which is § 7's R9 allowance.
    params: &[
        CoreTy::CallableTo("T"),
        CoreTy::Options(TRANSACTION_OPTIONS),
    ],
    defaults: &[],
    // § 7's `: T`. The member's answer *is* the closure's, so the transaction
    // is scenery around a call that computes whatever it was going to compute
    // — the same binding `Core\Cli::live` performs, and the reason a
    // transaction can wrap an existing expression without retyping it.
    return_ty: CoreTy::Var("T"),
    symbol: "nvs_core_db_connection_transaction",
    doc: Some(&TRANSACTION_DOC),
};

/// Spec § 18's `Core\Db\Transaction` — what § 7's closure is handed, and the
/// only place a transaction is nameable.
///
/// **`implements Queryable by $connection` is spelled here as the same rows
/// under the same symbols**, which is [ADR 0043](../../../../docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)'s
/// delegation with no second body to drift from the first: `query`, `execute`,
/// `executeMany` and `transaction` resolve to [`CONNECTION`]'s helpers, which
/// reach the connection through [`handle_of`] and so accept either receiver.
/// The alternative — four forwarding bodies — is four places for a rule to be
/// stated twice, and ADR 0063 R17 is the same objection to two spellings of one
/// operation.
///
/// **The first two slots are [`CONNECTION`]'s, in the same positions and under
/// the same names**, and that is load-bearing rather than tidy: it is what lets
/// one statement path read either handle. The two after it are this class's own
/// — § 7's two hazards, one slot each.
///
/// **What it does not hold is the connection object.** A transaction is a key
/// into the request's own table exactly as a connection is, so an escaped
/// `$tx` keeps nothing alive and cannot outlive the request that opened it;
/// [`SCOPE_SLOT`] is what makes the escape a `LogicError` rather than a use of
/// a connection that has moved on.
pub(crate) const TRANSACTION: CoreClass = CoreClass {
    name: TRANSACTION_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "query",
            names: &["sql", "params"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            // § 18's `Rows<Row>` — [`ROWS`] at the one concrete argument an
            // unhydrated result set has, and `queryAs<T>` is the same class at
            // whatever the call site wrote. A [`CoreTy::InstanceAt`] and not a
            // [`CoreTy::Instance`]: the bare spelling would intern the class at
            // its *own* `T`, a variable no call site of `query` ever binds.
            // Written out on both classes rather than named once, because
            // `tools/gaps.py` attributes a case to the class a member answers
            // by reading this very line.
            return_ty: CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Instance(ROW_NAME)]),
            symbol: "nvs_core_db_connection_query",
            doc: Some(&QUERY_DOC),
        },
        CoreMethod {
            name: "queryAs",
            names: &["sql", "params"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            // [`CONNECTION`]'s row, written out for the reason its `query`
            // sibling is: this is the class `tools/gaps.py` attributes a case
            // to. The symbol is the connection's too — ADR 0043's delegation
            // is one body reached through either handle, and [`handle_of`] is
            // what reads the two of them the same way.
            return_ty: CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Written("T")]),
            symbol: "nvs_core_db_connection_query_as",
            doc: Some(&QUERY_AS_DOC),
        },
        CoreMethod {
            name: "execute",
            names: &["sql", "params"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Instance(WRITE_NAME),
            symbol: "nvs_core_db_connection_execute",
            doc: Some(&EXECUTE_DOC),
        },
        CoreMethod {
            name: "executeMany",
            names: &["sql", "sets"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Array(&CoreTy::Mixed)),
            ],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_db_connection_execute_many",
            doc: Some(&EXECUTE_MANY_DOC),
        },
        TRANSACTION_ROW,
        CoreMethod {
            name: "rollBack",
            names: &["reason"],
            // Neutral: a reason is prose for a human and reaches no statement,
            // so a `tainted` one — "the cart holds ${item}, which is gone" —
            // is exactly the string a program has to hand and refusing it
            // would push callers to launder text that is never a sink's.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            // § 7: it always throws, so there is no value to answer with. The
            // `void` is the honest half of "sets a flag *and* throws".
            return_ty: CoreTy::Void,
            symbol: "nvs_core_db_transaction_roll_back",
            doc: Some(&ROLL_BACK_DOC),
        },
    ],
    slots: &[HANDLE_SLOT, CONNECTION_NAME_SLOT, SCOPE_SLOT, REASON_SLOT],
    constants: &[],
};

/// [`DRIVER`]'s fully-qualified name, written once so the two arms of
/// [`SETTINGS`] and every message quoting one cannot drift apart.
pub(crate) const DRIVER_NAME: &str = r"Core\Db\Driver";

/// Spec § 18's `Driver` — ADR 0067's five backends, as the registry half of
/// [`nvs_db::Driver`].
///
/// **The two halves are one enum and the wire one is authoritative.** This
/// table is what a program writes into a `Db\Settings` literal;
/// `nvs_db::Driver` is what a connection reports and what
/// `nvs_db::Driver::from_config_name` reads a `[db.<name>]` block's `driver`
/// as. Nothing about a backend is decided here.
///
/// **The cases are what make `Db\Settings` a discriminated union**, and they
/// are the whole of the mechanism: ADR 0135 § 2 selects an arm by asking which
/// one accepts the literal, and ADR 0047's enum-case types make
/// `Driver::Sqlite` and the other four disjoint sets. No field is declared to
/// be a discriminant, here or anywhere.
///
/// **The values are declaration ordinals and mean nothing else.** They are
/// § 18's own order, so `MySql` is 0 and `SqlServer` is 4, and they are not a
/// rank — writing them out rather than leaning on ADR 0010 § 1's
/// auto-increment is [`CoreEnum::cases`]' rule for every enum here.
pub(crate) const DRIVER: CoreEnum = CoreEnum {
    name: DRIVER_NAME,
    cases: &[
        ("MySql", 0),
        ("MariaDb", 1),
        ("Postgres", 2),
        ("Sqlite", 3),
        ("SqlServer", 4),
    ],
    doc: Some(&DRIVER_DOC),
};

/// [`DRIVER`]'s reference card — ADR 0117.
const DRIVER_DOC: EnumDoc = EnumDoc {
    short: "Which backend a connection speaks to. It is what a `Core\\Db::open` settings literal \
            names first, and naming it is what decides which of the two shapes the rest of that \
            literal has to be — a server takes a `host`, SQLite takes a `path`.",
    cases: &[
        CaseDoc {
            name: "MySql",
            desc: "MySQL, over its own client protocol.",
        },
        CaseDoc {
            name: "MariaDb",
            desc: "MariaDB, which is its own driver and not a MySQL flag — its authentication \
                   roster and its error codes are its own.",
        },
        CaseDoc {
            name: "Postgres",
            desc: "PostgreSQL, over the extended-query protocol.",
        },
        CaseDoc {
            name: "Sqlite",
            desc: "SQLite, over a file named by `path` rather than a host.",
        },
        CaseDoc {
            name: "SqlServer",
            desc: "Microsoft SQL Server, over TDS.",
        },
    ],
};

/// [`TLS`]'s fully-qualified name, written once for the same reason
/// [`DRIVER_NAME`] is.
pub(crate) const TLS_NAME: &str = r"Core\Db\Tls";

/// Spec § 18's `Tls` — how much of the server's certificate a TCP connection
/// checks.
///
/// **`VerifyFull` is the default and it is the only mode this runtime
/// implements**, which is ADR 0067 § 3's third closed hole: PHP's `pdo_pgsql`
/// defaults to `sslmode=prefer` and connects in plaintext whenever the server
/// says so, and none of § 3's three defaults is configurable to the unsafe
/// value. The other three cases are declared because § 18 declares them and a
/// program that writes one is refused *at the call*, naming what it asked for
/// — which is an honest answer, where accepting `Disabled` and quietly
/// verifying anyway would be a second, silent one. [`settings_tls`] is where
/// that refusal is worded.
///
/// What a deployment *can* change is whose certificates are believed, and that
/// is a different field: a `tls_ca_file` on the block, or the compiled-in
/// Mozilla anchor set — [`nvs_db::PgTarget::tls_ca_file`] owns that reading.
///
/// **The values are declaration ordinals**, § 18's own order, weakest first.
pub(crate) const TLS: CoreEnum = CoreEnum {
    name: TLS_NAME,
    cases: &[
        ("Disabled", 0),
        ("Required", 1),
        ("VerifyCa", 2),
        ("VerifyFull", 3),
    ],
    doc: Some(&TLS_DOC),
};

/// [`TLS`]'s reference card — ADR 0117.
const TLS_DOC: EnumDoc = EnumDoc {
    short: "How much of a server's identity a TCP connection establishes before it sends a \
            credential. `VerifyFull` is what every connection does and what a settings literal \
            that names nothing gets; the weaker three are refused rather than honoured, because \
            a connection that verified less than it promised is the hole this enum exists to \
            close.",
    cases: &[
        CaseDoc {
            name: "Disabled",
            desc: "No TLS at all. Refused.",
        },
        CaseDoc {
            name: "Required",
            desc: "TLS, with the certificate unchecked. Refused.",
        },
        CaseDoc {
            name: "VerifyCa",
            desc: "The certificate must chain to a trusted anchor, but its name is not checked. \
                   Refused.",
        },
        CaseDoc {
            name: "VerifyFull",
            desc: "The certificate must chain to a trusted anchor and must be issued for the \
                   host that was written. The default, and the only mode a connection runs in.",
        },
    ],
};

/// [`ISOLATION`]'s fully-qualified name, written once so the row, the option
/// that will take it and every message quoting it cannot drift apart.
pub(crate) const ISOLATION_NAME: &str = r"Core\Db\Isolation";

/// Spec § 18's `Isolation` — ADR 0067 § 7's five levels, as the registry half
/// of [`nvs_db::Isolation`].
///
/// **The two halves are one enum and the wire one is authoritative.** This
/// table is what a program writes; `nvs_db::Isolation` is what a driver renders,
/// and its doc comment owns both the rule that a driver lacking a level throws
/// rather than running the closure at a weaker one, and the reason a driver may
/// render a level as a *stronger* guarantee without throwing. Nothing about a
/// level is decided here — there is no second place for it to be decided
/// differently.
///
/// **The values are declaration ordinals and mean nothing else.** They are § 7's
/// own order, so `ReadUncommitted` is 0, but they are not a rank a program may
/// compare: `Snapshot` and the two levels either side of it are not one chain on
/// every backend, which is why `nvs_db::Isolation` derives no `Ord` either.
/// Writing them out rather than leaning on ADR 0010 § 1's auto-increment is
/// [`CoreEnum::cases`]' rule for every enum here.
pub(crate) const ISOLATION: CoreEnum = CoreEnum {
    name: ISOLATION_NAME,
    cases: &[
        ("ReadUncommitted", 0),
        ("ReadCommitted", 1),
        ("RepeatableRead", 2),
        ("Snapshot", 3),
        ("Serializable", 4),
    ],
    doc: Some(&ISOLATION_DOC),
};

/// [`ISOLATION`]'s reference card — ADR 0117.
const ISOLATION_DOC: EnumDoc = EnumDoc {
    short: "What a transaction is allowed to see of the work running beside it — the `isolation` \
            option `transaction` takes, and the connection's own level when it is absent. A driver \
            that cannot offer the level asked for throws rather than running the closure at a \
            weaker one.",
    cases: &[
        CaseDoc {
            name: "ReadUncommitted",
            desc: "A statement may read rows another transaction has written and not committed, \
                   on a backend that implements the level at all.",
        },
        CaseDoc {
            name: "ReadCommitted",
            desc: "A statement sees the rows committed before that statement began.",
        },
        CaseDoc {
            name: "RepeatableRead",
            desc: "Every statement in the transaction sees one snapshot of committed rows.",
        },
        CaseDoc {
            name: "Snapshot",
            desc: "The transaction reads from one snapshot taken when it began, and writes \
                   conflict rather than block — SQL Server's own level, and what the \
                   row-versioning backends call `REPEATABLE READ`.",
        },
        CaseDoc {
            name: "Serializable",
            desc: "Concurrent transactions produce a result some serial order of them would have \
                   produced, and a transaction that cannot is rolled back for the caller to retry.",
        },
    ],
};

/// [`ERROR_KIND`]'s fully-qualified name, written once so the row, the property
/// that answers with it and every message quoting it cannot drift apart.
///
/// `nvs_types::error_lib` spells it a second time, because that crate seeds
/// `Core\Db\DbError::$kind`'s type and cannot reach a `pub(crate)` const here;
/// `the_kind_property_names_a_registered_enum` is what holds the two spellings
/// together rather than this sentence.
pub(crate) const ERROR_KIND_NAME: &str = r"Core\Db\ErrorKind";

/// [ADR 0067](../../../docs/adr/0067-core-db.md) § 8's `ErrorKind` — the
/// eleven conditions an application branches on, as the registry half of
/// [`nvs_db::DbErrorKind`].
///
/// **The two halves are one enum and the wire one is authoritative.** This
/// table is what a program matches on; `nvs_db::DbErrorKind` is what a driver
/// maps its own codes onto, and its doc comment owns why the set is normalised
/// at all — PDO exposes a `SQLSTATE` and a vendor integer, so real PHP matches
/// on `"Duplicate entry"` or hard-codes `1062`. `nvs_db::DbErrorKind::is_retryable`
/// owns which two of these § 7's `{retries: n}` re-runs a closure over, and
/// nothing about that rule is decided here.
///
/// **A class per condition was rejected** — § 8's own *Alternatives*: ten more
/// types in the deliberately small closed exception set
/// [0063 § 4](../../../docs/adr/0063-core-api-conventions.md) fixes, for
/// boundaries that are driver-dependent anyway. What normalising does not
/// reach stays readable as the raw `sqlState`, `constraint` and `driverCode`
/// beside it.
///
/// **The values are declaration ordinals and mean nothing else.** They are
/// § 8's own order, so `UniqueViolation` is 0 and `Other` is 10, but they are
/// not a rank a program may compare: a kind is a set and not a scale, which is
/// why `nvs_db::DbErrorKind` derives no `Ord` either. Writing them out rather
/// than leaning on ADR 0010 § 1's auto-increment is [`CoreEnum::cases`]' rule
/// for every enum here.
pub(crate) const ERROR_KIND: CoreEnum = CoreEnum {
    name: ERROR_KIND_NAME,
    cases: &[
        ("UniqueViolation", 0),
        ("ForeignKeyViolation", 1),
        ("NotNullViolation", 2),
        ("CheckViolation", 3),
        ("Deadlock", 4),
        ("SerializationFailure", 5),
        ("ConnectionLost", 6),
        ("Timeout", 7),
        ("Syntax", 8),
        ("Permission", 9),
        ("Other", 10),
    ],
    doc: Some(&ERROR_KIND_DOC),
};

/// [`ERROR_KIND`]'s reference card — ADR 0117.
const ERROR_KIND_DOC: EnumDoc = EnumDoc {
    short: "Why the server refused a statement, normalised across the drivers so that a program \
            branches on the condition rather than on a vendor code. `Core\\Db\\DbError::$kind` \
            answers with one of these, and the raw `SQLSTATE` beside it covers what normalising \
            does not reach.",
    cases: &[
        CaseDoc {
            name: "UniqueViolation",
            desc: "A row with this key already exists.",
        },
        CaseDoc {
            name: "ForeignKeyViolation",
            desc: "A referenced row does not exist, or a referencing one still does.",
        },
        CaseDoc {
            name: "NotNullViolation",
            desc: "A column that may not be null was written null.",
        },
        CaseDoc {
            name: "CheckViolation",
            desc: "A `CHECK` constraint refused the row.",
        },
        CaseDoc {
            name: "Deadlock",
            desc: "Two transactions each hold what the other waits for, and the server aborted \
                   this one to break it. SQLite's `SQLITE_BUSY` and `SQLITE_LOCKED` arrive here \
                   too, so that a retry works there as well.",
        },
        CaseDoc {
            name: "SerializationFailure",
            desc: "The transaction could not be serialised against a concurrent one and was \
                   aborted — the ordinary outcome under `REPEATABLE READ` or stronger.",
        },
        CaseDoc {
            name: "ConnectionLost",
            desc: "The connection is gone, or the server is going away.",
        },
        CaseDoc {
            name: "Timeout",
            desc: "A statement or an idle transaction ran past a bound and was cancelled.",
        },
        CaseDoc {
            name: "Syntax",
            desc: "The statement is not something the server will run: a syntax error, an \
                   undefined table, a type it cannot resolve.",
        },
        CaseDoc {
            name: "Permission",
            desc: "The role may not do this.",
        },
        CaseDoc {
            name: "Other",
            desc: "Anything the driver's own table does not name, including a condition one \
                   backend has and the others do not. It is also what a `Core\\Db\\DbError` \
                   constructed by hand carries, no server having classified it.",
        },
    ],
};

/// [`COLUMN_TYPE`]'s fully-qualified name, written once so the row, the member
/// that answers with it and every message quoting it cannot drift apart.
pub(crate) const COLUMN_TYPE_NAME: &str = r"Core\Db\ColumnType";

/// Spec § 18's `ColumnType` — the fourteen types a result column can be
/// declared as, as the registry half of [`nvs_db::ColumnType`].
///
/// **The two halves are one enum and the wire one is authoritative.** This
/// table is what a program matches on; `nvs_db::ColumnType` is what a driver
/// classifies a column into, and its doc comment owns the rule every
/// description below is written to: **a case says what the column was
/// *declared* as, never what a read of it produces.** That is why `Json` is a
/// case of its own although [ADR 0067](../../../docs/adr/0067-core-db.md) § 9
/// decodes a `JSON` column to the same `tainted string` a `TEXT` one decodes
/// to, and why there is no array case at all — § 9 reads a PostgreSQL array as
/// `array<T>` and MySQL's `SET` as `array<string>`, and both *describe* as
/// `Other`. [`ROWS`] records the same split from the reader's side.
///
/// **The values are declaration ordinals and mean nothing else.** They are the
/// spec's own order at `docs/spec/01-core-library.md:1223`, so `Int` is 0 and
/// `Other` is 13, but they are not a rank a program may compare: these are a
/// set and not a scale, which is why `nvs_db::ColumnType` derives no `Ord`
/// either. Writing them out rather than leaning on ADR 0010 § 1's
/// auto-increment is [`CoreEnum::cases`]' rule for every enum here.
pub(crate) const COLUMN_TYPE: CoreEnum = CoreEnum {
    name: COLUMN_TYPE_NAME,
    cases: &[
        ("Int", 0),
        ("Uint", 1),
        ("Float", 2),
        ("Decimal", 3),
        ("Text", 4),
        ("Bytes", 5),
        ("Bool", 6),
        ("Date", 7),
        ("Time", 8),
        ("DateTime", 9),
        ("Instant", 10),
        ("Uuid", 11),
        ("Json", 12),
        ("Other", 13),
    ],
    doc: Some(&COLUMN_TYPE_DOC),
};

/// [`COLUMN_TYPE`]'s reference card — ADR 0117.
const COLUMN_TYPE_DOC: EnumDoc = EnumDoc {
    short: "What a result column was declared as, which is a description of the column and not a \
            summary of the value a read of it produces: a `JSON` column and a `TEXT` one both read \
            back as `tainted string` and are told apart here, wherever the backend has a type of \
            its own to tell them apart by.",
    cases: &[
        CaseDoc {
            name: "Int",
            desc: "A signed integer column — `SMALLINT`, `INTEGER` or `BIGINT`. MySQL's and \
                   MariaDB's `TINYINT(1)` is one of these rather than a `Bool`.",
        },
        CaseDoc {
            name: "Uint",
            desc: "An unsigned integer column: an `UNSIGNED` integer on MySQL and MariaDB, an \
                   `oid` on PostgreSQL.",
        },
        CaseDoc {
            name: "Float",
            desc: "`FLOAT`, `REAL` or `DOUBLE`.",
        },
        CaseDoc {
            name: "Decimal",
            desc: "An exact numeric column — `DECIMAL`, `NUMERIC` or `MONEY`.",
        },
        CaseDoc {
            name: "Text",
            desc: "A text-family column: `CHAR`, `VARCHAR`, `TEXT` or `ENUM`, and a JSON column \
                   on a backend where JSON is an aliased text type rather than a type of its own.",
        },
        CaseDoc {
            name: "Bytes",
            desc: "A binary column — `BINARY`, `BLOB` or `BYTEA`, including the `BINARY(16)` a \
                   MySQL schema stores a UUID in.",
        },
        CaseDoc {
            name: "Bool",
            desc: "`BOOLEAN`, and `BIT(1)`.",
        },
        CaseDoc {
            name: "Date",
            desc: "A `DATE`, which a read answers with a `Core\\Time\\Date`.",
        },
        CaseDoc {
            name: "Time",
            desc: "A `TIME`, which a read answers with a `Core\\Time\\TimeOfDay`.",
        },
        CaseDoc {
            name: "DateTime",
            desc: "A zone-less `DATETIME` or `TIMESTAMP`, which a read answers with a \
                   `Core\\Time\\DateTime` in the zone the connection declared.",
        },
        CaseDoc {
            name: "Instant",
            desc: "A column carrying its own offset — `TIMESTAMPTZ`, or SQL Server's \
                   `datetimeoffset` — which a read answers with a `Core\\Time\\Instant`.",
        },
        CaseDoc {
            name: "Uuid",
            desc: "A `UUID` or a `uniqueidentifier`, which a read answers with a `Core\\Uuid`.",
        },
        CaseDoc {
            name: "Json",
            desc: "A column the backend types as JSON. The value still reads back as a `tainted \
                   string`, since JSON is never decoded for you; a backend that has no JSON type \
                   of its own reports the column as `Text` instead.",
        },
        CaseDoc {
            name: "Other",
            desc: "Every other column: one with no Novis type of its own, and every array. This \
                   is the total case rather than a failure, so a column list describes every \
                   column a server can send.",
        },
    ],
};

/// Spec § 18's `Core\Db\Rows` — what a buffered statement answers with.
///
/// What the slot holds is the whole of what the members read, decided here
/// rather than left to them: **every row, already decoded, each as a
/// string-keyed array of its columns** — which is `Row::toArray`'s own shape, so
/// every member below is a reader over it and never a second decoder.
///
/// **All six of § 18's members, and the sixth reads a second slot.**
/// `columns()` answers what the *statement* described rather than anything a
/// row holds, so [`ROWS_COLUMNS_SLOT`] is filled beside the rows at query time:
/// [`nvs_db::PgRows`] lends its row description out of the borrow the rows are
/// read from, so it is captured there or not at all. What a program
/// matches the answer on is [`COLUMN_TYPE`], whose own doc owns why a
/// description of a column is not a summary of the value a read of it
/// produces.
///
/// **It is generic at `T`, and § 18's `Rows` and `Rows<T>` are this one class.**
/// The roster row is in [`crate::registry::GENERIC_CLASSES`], `all`/`first`
/// answer `array<T>`/`?T`, and what fixes the variable is the member that
/// produced the receiver: `query` answers `Rows<Row>` and `queryAs<T>` the
/// same class at the call site's own `T`. Two classes would have been two
/// rosters of identical readers, and the unhydrated one is not a different
/// thing from the hydrated one — it is the case where `T` is a `Core\Db\Row`.
///
/// **It is `Iterable<T>`, so a `foreach` walks it directly** — the row in
/// [`crate::registry::ITERABLES`] is what lets the checker compile one, and
/// [`nvs_core_db_rows_iterate`] is what the receiver answers the protocol with.
/// ADR 0053 § 3 takes exactly three subjects and this is the second of them
/// rather than a fourth, so `foreach ($rows as Row $row)` and `all()` are one
/// walk over one array of rows: neither copies what the other already holds.
///
/// **Buffered is ADR 0067 § 4's default and this is what it spends**: a result
/// set is held whole, per request, and the connection is free the moment
/// `query` returns. § 4 chose that over the alternative because
/// [ADR 0004](../../../../docs/adr/0004-memory-for-simplicity.md) ranks memory
/// last and because a cursor breaks the commonest loop in web programming on a
/// connection-busy rule; `stream` is the member for a result set that does not
/// fit, and it is the one that holds the connection.
pub(crate) const ROWS: CoreClass = CoreClass {
    name: ROWS_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "all",
            names: &[],
            params: &[],
            defaults: &[],
            // § 18's `Rows<T>` row: `all` answers the class's own type
            // variable, which `query`'s `Rows<Row>` fixes at a `Core\Db\Row`
            // and `queryAs<T>` at whatever the call site wrote. Substituted in
            // by `nvs_types::expr::args::substitute_receiver_args`, exactly as
            // `Core\ObjectSet`'s members' `T` is.
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_db_rows_all",
            doc: Some(&ROWS_ALL_DOC),
        },
        CoreMethod {
            name: "first",
            names: &[],
            params: &[],
            defaults: &[],
            // `?T`, for the reason `all` answers `array<T>` above.
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "nvs_core_db_rows_first",
            doc: Some(&ROWS_FIRST_DOC),
        },
        CoreMethod {
            name: "value",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_db_rows_value",
            doc: Some(&ROWS_VALUE_DOC),
        },
        CoreMethod {
            name: "column",
            names: &["key"],
            // § 18's `int|string $key`: a number is the column's *position* in
            // the server's own description and a name is its label. Neutral,
            // for the reason every name parameter in [`ROW`] is — the values
            // this answers with carry the database's qualifiers and never the
            // key's.
            params: &[CoreTy::Union(&[CoreTy::Int, CoreTy::Text(Qual::Neutral)])],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Mixed),
            symbol: "nvs_core_db_rows_column",
            doc: Some(&ROWS_COLUMN_DOC),
        },
        CoreMethod {
            name: "count",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_db_rows_count",
            doc: Some(&ROWS_COUNT_DOC),
        },
        CoreMethod {
            name: "columns",
            names: &[],
            params: &[],
            defaults: &[],
            // § 18's `array<Column>` and deliberately not `array<T>`: a
            // description belongs to the statement, so `queryAs<Person>`
            // describes the same columns `query` does and the class variable
            // has nothing to say about it.
            return_ty: CoreTy::Array(&CoreTy::Instance(COLUMN_NAME)),
            symbol: "nvs_core_db_rows_columns",
            doc: Some(&ROWS_COLUMNS_DOC),
        },
    ],
    slots: &[ROWS_SLOT, ROWS_CLASS_SLOT, ROWS_COLUMNS_SLOT],
    constants: &[],
};

/// Spec § 18's `Core\Db\Row` — one row of a [`ROWS`], and the whole of what
/// ADR 0067 § 6 puts in place of `FETCH_ASSOC`, `FETCH_NUM` and `FETCH_OBJ`.
///
/// **The three orderings PHP makes a fetch mode of are one shape here.** A row
/// is a string-keyed array of its columns and nothing else, so there is no
/// numeric twin to ask for and no object twin either: `get`/`toArray` are the
/// associative reading, the typed readers below are what an object reading was
/// wanted for, and `queryAs<T>` — ADR 0071's `#[Db\Derive]` — is where a real
/// class comes from. A fetch-mode argument would be [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md)
/// R11's flag deciding what a member returns, which is the thing that section
/// removes.
///
/// **The eleven typed readers convert losslessly or throw, and the rule is one
/// sentence: a reader answers its own tag, and `int`/`uint` are the single
/// crossing** — [ADR 0007](../../../../docs/adr/0007-static-type-system.md) § 4
/// makes those two views of one integer, so a `BIGINT` read as `uint` is the
/// same value and a negative one throws rather than wrapping. Everything else
/// refuses: `->float` on a `NUMERIC` is not the rounding PHP does silently, and
/// `->string` on a `BYTEA` is not the re-interpretation
/// [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) keeps apart. The
/// universal path § 18 names — `->get()` plus `as` — is what a program that
/// means a conversion writes.
///
/// **Four of the eleven answer with an instance**: `instant`, `date`, `time`
/// and `uuid` read back the `Core\Time`/`Core\Uuid` value [`column_value`]
/// built out of § 9's five structured columns. They were written as the lookups
/// they always would be while that decoder still refused those columns, which
/// is why landing it changed the decoder and not one line of this class.
///
/// **The fifth structured row has no reader of its own**: a zone-less
/// `TIMESTAMP` is a `Core\Time\DateTime`, and § 6's roster of eleven — which
/// this list is exactly — names no `dateTime`. It is reached through `get`,
/// which is § 18's universal path and what that roster means by leaving it
/// out.
pub(crate) const ROW: CoreClass = CoreClass {
    name: ROW_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "has",
            names: &["name"],
            // § 18's own annotation on this row, and what every other name
            // parameter in this class carries too: a column name is a lookup
            // key, so what comes back carries the qualifiers ADR 0067 § 9 gives
            // the *column* and never the name's. `Qual::Sink` would be the
            // wrong word — nothing here executes the name.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_db_row_has",
            doc: Some(&ROW_HAS_DOC),
        },
        CoreMethod {
            name: "get",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_db_row_get",
            doc: Some(&ROW_GET_DOC),
        },
        CoreMethod {
            name: "toArray",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Mixed),
            symbol: "nvs_core_db_row_to_array",
            doc: Some(&ROW_TO_ARRAY_DOC),
        },
        CoreMethod {
            name: "string",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_db_row_string",
            doc: Some(&ROW_STRING_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Bytes),
            symbol: "nvs_core_db_row_bytes",
            doc: Some(&ROW_BYTES_DOC),
        },
        CoreMethod {
            name: "int",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Int),
            symbol: "nvs_core_db_row_int",
            doc: Some(&ROW_INT_DOC),
        },
        CoreMethod {
            name: "uint",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Uint),
            symbol: "nvs_core_db_row_uint",
            doc: Some(&ROW_UINT_DOC),
        },
        CoreMethod {
            name: "float",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Float),
            symbol: "nvs_core_db_row_float",
            doc: Some(&ROW_FLOAT_DOC),
        },
        CoreMethod {
            name: "bool",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Bool),
            symbol: "nvs_core_db_row_bool",
            doc: Some(&ROW_BOOL_DOC),
        },
        CoreMethod {
            name: "decimal",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Decimal),
            symbol: "nvs_core_db_row_decimal",
            doc: Some(&ROW_DECIMAL_DOC),
        },
        CoreMethod {
            name: "instant",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::time::INSTANT_NAME)),
            symbol: "nvs_core_db_row_instant",
            doc: Some(&ROW_INSTANT_DOC),
        },
        CoreMethod {
            name: "date",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::time::DATE_NAME)),
            symbol: "nvs_core_db_row_date",
            doc: Some(&ROW_DATE_DOC),
        },
        CoreMethod {
            name: "time",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::time::TIME_OF_DAY_NAME)),
            symbol: "nvs_core_db_row_time",
            doc: Some(&ROW_TIME_DOC),
        },
        CoreMethod {
            name: "uuid",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::uuid::NAME)),
            symbol: "nvs_core_db_row_uuid",
            doc: Some(&ROW_UUID_DOC),
        },
    ],
    slots: &[COLUMNS_SLOT],
    constants: &[],
};

/// Spec § 18's `Core\Db\Write` — what a statement that answers no rows answers
/// with, and the whole of what ADR 0067 § 4 puts in place of `rowCount` on a
/// write, `lastInsertId` and `mysqli_info`.
///
/// **Three readers rather than § 18's three readonly properties**, which is
/// where this departs from that table and has to: a `Core`-owned instance has
/// no property a program can reach ([`CoreTy::Instance`] is the home of that
/// rule), so `$w->affected` would resolve a class, find no member and reach
/// `nvs-ir` with nothing to call. `Core\RateLimit\Decision` is the same shape
/// for the same reason and `Core\Http\Response::status` is the precedent.
///
/// **`lastId` belongs to the write and not to the connection**, which is the
/// one design difference worth the class existing: `mysqli_insert_id` reads a
/// *session* value, so an unrelated statement in between makes it stale, and
/// there is no session state here for that hazard to live in.
/// [`nvs_db::PgRows::last_id`] owns what PostgreSQL reads it out of — a
/// `RETURNING` clause, since that protocol has no last-insert-id at all.
///
/// The three slots are filled once, by
/// [`nvs_core_db_connection_execute`], from a stream that has already ended:
/// both counts are `CommandComplete`'s and neither exists until it has
/// arrived.
pub(crate) const WRITE: CoreClass = CoreClass {
    name: WRITE_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "affected",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_db_write_affected",
            doc: Some(&WRITE_AFFECTED_DOC),
        },
        CoreMethod {
            name: "changed",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Uint),
            symbol: "nvs_core_db_write_changed",
            doc: Some(&WRITE_CHANGED_DOC),
        },
        CoreMethod {
            name: "lastId",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Uint),
            symbol: "nvs_core_db_write_last_id",
            doc: Some(&WRITE_LAST_ID_DOC),
        },
    ],
    slots: &[AFFECTED_SLOT, CHANGED_SLOT, LAST_ID_SLOT],
    constants: &[],
};

/// Spec § 18's `Core\Db\Column` — one column of what a statement described,
/// and the whole of what ADR 0067 puts in place of `getColumnMeta` and
/// `mysqli_fetch_field`.
///
/// **A description belongs to the statement and not to a row**, which is why
/// one of these is reached through [`ROWS`] and never through [`ROW`]: a
/// `select` that matched nothing still described the columns it would have
/// answered, and that is most of what the class is for — a caller rendering a
/// table has its headings before it knows whether there is anything under
/// them.
///
/// **Three readers rather than § 18's three properties**, for the reason
/// [`WRITE`]'s own docs give: a `Core`-owned instance has no property a program
/// can reach.
///
/// **What `getColumnMeta` also carried is deliberately absent.** No vendor type
/// name, no `pdo_type`, no `len`, no `precision`, no table name and no flag
/// list. A vendor type name is the string every backend spells differently,
/// which is the thing [`COLUMN_TYPE`] exists to replace; the rest is either a
/// property of the wire encoding rather than of the column, or a second catalog
/// round trip per statement — and PHP's own answer for it is an array whose keys
/// differ per driver, which is the shape [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md)
/// R11 removes.
pub(crate) const COLUMN: CoreClass = CoreClass {
    name: COLUMN_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            // Unqualified, and § 18's table writes it that way: a label is
            // described by the server out of the statement *this* program
            // wrote, so it is not one of § 9's `tainted` reads — those are the
            // column's values, which is what a request can put bytes into.
            return_ty: CoreTy::Str,
            symbol: "nvs_core_db_column_name",
            doc: Some(&COLUMN_LABEL_DOC),
        },
        CoreMethod {
            name: "type",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(COLUMN_TYPE_NAME),
            symbol: "nvs_core_db_column_type",
            doc: Some(&COLUMN_DECLARED_DOC),
        },
        CoreMethod {
            name: "nullable",
            names: &[],
            params: &[],
            defaults: &[],
            // `bool` and not `?bool`, which is the decision worth naming: an
            // absence would be a third answer every caller has to branch on to
            // learn nothing, and [`COLUMN_NULLABLE_DOC`] says instead what the
            // one answer this driver can give means.
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_db_column_nullable",
            doc: Some(&COLUMN_NULLABLE_DOC),
        },
    ],
    slots: &[LABEL_SLOT, DECLARED_SLOT, NULLABLE_SLOT],
    constants: &[],
};

/// Spec § 18's `Core\Db\InList` — opaque, produced by one member and read by
/// the bind. No members at all, which is that table's own "accepted only as a
/// bound parameter".
pub(crate) const IN_LIST: CoreClass = CoreClass {
    name: IN_LIST_NAME,
    methods: &[],
    instance: &[],
    slots: &[VALUES_SLOT],
    constants: &[],
};

/// `Core\Db::connect`'s reference card — ADR 0117.
const CONNECT_DOC: MethodDoc = MethodDoc {
    short: "Opens the connection an operator named in a `[db.<name>]` block of `nvs.toml`, and \
            answers the same one again for the rest of the request — `new PDO`, `pg_connect` and \
            `mysqli_connect`, with the credential out of the program and in root-owned \
            configuration. Needs the `db.connect` capability for that name.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The block to open, matched exactly: `\"main\"` is `[db.main]`. Two blocks that \
                   configure the same server are two connections, because an operator who wrote \
                   two meant two.",
            shape: &[],
        },
        ParamDoc {
            name: "shared",
            desc: "Whether this call may answer with the connection an earlier one already \
                   opened. `false` opens a dedicated connection instead — what a write that must \
                   survive a rollback, a session-scoped lock or a second statement alongside a \
                   `stream` needs.",
            shape: &[],
        },
        ParamDoc {
            name: "timeout",
            desc: "How long the handshake may take, including name resolution and TLS. Left out, \
                   the connection is bounded by the server and the network alone.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Db\\Connection`. The same call twice in one request answers the same object \
          unless `shared` is `false`, and the connection is closed when the request ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`db.connect` does not grant `$name`, no `[db.<name>]` block of that name \
                   exists, or the block cannot be read as a connection — a missing `driver`, a \
                   field belonging to another driver, or a `time_zone` that is not an offset.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The host does not resolve, the connection, the TLS handshake or the login \
                   itself failed, or this core already holds `[db.<name>.pool] max` connections \
                   under that name and none came free within `acquire`. A refusal the server \
                   worded carries its own message.",
        },
    ],
};

/// `Core\Db::open`'s reference card — ADR 0117.
///
/// `shape` is filled here and empty on every other card in this module,
/// because this is the one parameter that is a written shape rather than a
/// value: [`SETTINGS`]' merged key list, in the order the ABI flattens it.
const OPEN_DOC: MethodDoc = MethodDoc {
    short: "Opens a connection to a server the program itself names, for the case a `[db.<name>]` \
            block cannot cover — a tenant whose database is a row in another one, or an \
            administration tool a human types a host into. Needs the `db.open` capability for \
            that host, and unlike `connect` the address is checked against the denied ranges in \
            full.",
    params: &[
        ParamDoc {
            name: "settings",
            desc: "Everything the connection is made of. It is one of two shapes and the \
                   `driver` decides which: four of the five backends take a host, and SQLite \
                   takes a file path instead.",
            shape: &[
                ShapeKeyDoc {
                    key: "driver",
                    ty: "Driver",
                    desc: "Which backend this is, and so which of the two shapes the rest of \
                           the literal has to be.",
                },
                ShapeKeyDoc {
                    key: "host",
                    ty: "string",
                    desc: "The server to open, and the name its certificate is checked \
                           against. It is a sink with no launderer: no check on a string can \
                           establish that a host is safe to send a credential to.",
                },
                ShapeKeyDoc {
                    key: "port",
                    ty: "uint",
                    desc: "The port to open. Left out, the driver's own — 5432 for PostgreSQL, \
                           3306 for MySQL and MariaDB.",
                },
                ShapeKeyDoc {
                    key: "database",
                    ty: "tainted string",
                    desc: "The database or schema to attach to. `tainted` is accepted: it is a \
                           length-prefixed protocol field and never parsed text.",
                },
                ShapeKeyDoc {
                    key: "user",
                    ty: "tainted string",
                    desc: "The role to log in as, accepted `tainted` for the same reason.",
                },
                ShapeKeyDoc {
                    key: "password",
                    ty: "secret tainted string",
                    desc: "The role's password. It is `secret`, so it cannot reach a log line, \
                           a message or a trace.",
                },
                ShapeKeyDoc {
                    key: "tls",
                    ty: "Tls",
                    desc: "How much of the certificate is checked. Only `VerifyFull` runs, and \
                           it is what an absent key means; the weaker three are refused.",
                },
                ShapeKeyDoc {
                    key: "timeZone",
                    ty: "Core\\Time\\Zone",
                    desc: "The zone a column with no zone of its own is read in, and the one \
                           the server is told to use. UTC where it is absent.",
                },
                ShapeKeyDoc {
                    key: "timeout",
                    ty: "Core\\Time\\Duration",
                    desc: "How long the handshake may take, resolution and TLS included.",
                },
                ShapeKeyDoc {
                    key: "statementCache",
                    ty: "uint",
                    desc: "How many prepared statements this connection may keep on the \
                           server. `0` turns the cache off.",
                },
                ShapeKeyDoc {
                    key: "path",
                    ty: "string",
                    desc: "SQLite's file, in place of a host. It is a path sink, and reaching \
                           it needs `fs.read` and `fs.write` as well.",
                },
            ],
        },
        ParamDoc {
            name: "shared",
            desc: "Whether this call may answer with the connection an earlier one opened from \
                   the same settings. `false` opens a dedicated connection instead.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Db\\Connection`, closed when the request ends. Two calls with settings that \
          agree in every field answer the same object unless `shared` is `false`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`db.open` does not grant the host, the address it resolves to is a private \
                   range that `net.internal` does not except, the settings do not describe a \
                   connection this build can open, or `tls` asks for a mode weaker than \
                   `VerifyFull`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The host does not resolve, the connection, the TLS handshake or the login \
                   itself failed, or this core already holds its `max` of connections to those \
                   settings and none came free while waiting.",
        },
    ],
};

/// `Core\Db::inList`'s reference card — ADR 0117.
const IN_LIST_DOC: MethodDoc = MethodDoc {
    short: "Marks `$values` as a run of bound values rather than one, so the placeholder it is \
            bound to expands into a parenthesised list of that many — the `IN (?, ?, ?)` every \
            PHP program builds with `implode` and `array_fill`. Nothing else in a statement \
            expands, which is what keeps the SQL text independent of what a value turned out \
            to be.",
    params: &[ParamDoc {
        name: "values",
        desc: "The values to bind, one placeholder each, in the array's own order. Keys are not \
               read: a marker binds positions inside one placeholder, not names.",
        shape: &[],
    }],
    ret: "A `Core\\Db\\InList` to bind to a single placeholder. It has no members and is accepted \
          nowhere else; two lists of different lengths bound to the same SQL are two entries in \
          the statement cache, because expansion changes the statement's arity.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$values` is empty, which means \"match nothing\" inside `IN` and \"match \
               everything\" inside `NOT IN` — the caller branches instead.",
    }],
};

/// `Core\Db::quoteIdentifier`'s reference card — ADR 0117.
const QUOTE_IDENTIFIER_DOC: MethodDoc = MethodDoc {
    short: "Checks that `$name` is a bare SQL identifier — a letter or `_`, then letters, digits \
            or `_` — and answers it unchanged and no longer `tainted`, so it can be written into \
            the text of a statement. Replaces escaping a table or column name by hand, which is \
            what `mysqli_real_escape_string` was doing there.",
    params: &[ParamDoc {
        name: "name",
        desc: "The identifier to check. It is answered exactly as given: nothing is escaped, \
               truncated or lower-cased, because a name this accepts needs none of it.",
        shape: &[],
    }],
    ret: "The same text, without the `tainted` qualifier. It carries no delimiter — `Core\\Db` \
          has no connection and so no dialect, and the five backends disagree on what a \
          delimiter is.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$name` is empty, starts with a digit, or holds any character outside letters, \
               digits and `_` — including a name that would need delimiting to be legal.",
    }],
};

/// `Core\Db\Connection::query`'s reference card — ADR 0117.
const QUERY_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement with its values bound, and reads every row it answers into memory \
            before returning — `PDO::prepare` plus `execute` plus `fetchAll` in one call, with no \
            `prepare` step because every statement is prepared. The connection is free again the \
            moment this returns; `stream` is the one that holds it.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, with a `?` for each value or a `:name` for each — never a value \
                   written into the text. It is a sink, so a `tainted` string is refused while \
                   compiling and there is no escaper to launder one with.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind: list-keyed for `?` and string-keyed for `:name`, one array \
                   and never both spellings. A `Core\\Db::inList` element expands into a run of \
                   placeholders at its own position, and nothing else expands.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Db\\Rows<Core\\Db\\Row>` holding every row the statement answered, in the \
          server's order. A statement that answers none — an `update`, a `create table` — is an \
          empty one rather than a refusal.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong rather than the database: the placeholders and the array \
                   disagree in spelling or in number, a `:name` names no element, an element is a \
                   value with no bound form — an array, an object that is not an `inList` — or a \
                   statement is already streaming on this connection.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server refused the statement — a syntax error, a constraint, a permission \
                   — carrying its own `SQLSTATE` and message, or a column came back in a type \
                   this driver does not read back yet.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the statement was in flight, which leaves it \
                   unusable for the rest of the request.",
        },
    ],
};

/// `Core\Db\Connection::queryAs`'s reference card — ADR 0117.
const QUERY_AS_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement exactly as `query` does and answers its rows as the class written at \
            the call site — `PDO::FETCH_CLASS` and the hand-written hydration loop, with the \
            mapping generated from the class's own declared properties by `#[Db\\Derive]` rather \
            than matched up by hand.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, bound exactly as `query` binds it: a `?` or a `:name` per value, \
                   never a value written into the text, and a sink either way.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind, under `query`'s own rule — one array, list-keyed for `?` \
                   and string-keyed for `:name`.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Db\\Rows<T>` holding one `T` per row, in the server's order. A row is built \
          when it is handed out — by `all`, by `first` or by a `foreach` — so a result that is \
          only counted constructs nothing.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The type argument is an `array<...>`, which a result set already is one row \
                   per row of; or `T` carries no `#[Db\\Derive]`, so there is no column mapping \
                   to build it from. Both are properties of the call site and would be \
                   compile-time diagnostics if either type band had a code left.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "A row did not match `T`: a column missing, a column of another type than the \
                   field declares, a SQL NULL in a field that is not `?T`, or a field whose \
                   declared type has no column mapping at all. Every bad column of the row is \
                   reported at once, in `issues`, each `path` the column's name.",
        },
    ],
};

/// `Core\Db\Connection::execute`'s reference card — ADR 0117.
const EXECUTE_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement that answers counts rather than rows — an `insert`, an `update`, a \
            `delete`, a `create table` — and answers what it did: `PDO::exec`, \
            `PDOStatement::execute` and `lastInsertId` in one call, with the values bound the same \
            way `query` binds them.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, with a `?` for each value or a `:name` for each — never a value \
                   written into the text. It is a sink, so a `tainted` string is refused while \
                   compiling and there is no escaper to launder one with.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind: list-keyed for `?` and string-keyed for `:name`, one array \
                   and never both spellings — `query`'s rule exactly, since both members bind \
                   through the same rewriter.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Db\\Write` carrying how many rows were affected, that count as the server \
          reported it, and the id a `RETURNING` clause handed back. Rows the statement did answer \
          are read to the end and discarded, so the connection is free when this returns; `query` \
          is the member that keeps them.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong rather than the database: the placeholders and the array \
                   disagree in spelling or in number, a `:name` names no element, an element is a \
                   value with no bound form — an array, an object that is not an `inList` — or a \
                   statement is already streaming on this connection.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server refused the statement — a syntax error, a constraint, a permission \
                   — carrying its own `SQLSTATE` and message.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the statement was in flight, which leaves it \
                   unusable for the rest of the request.",
        },
    ],
};

/// `Core\Db\Connection::executeMany`'s reference card — ADR 0117.
const EXECUTE_MANY_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement once per set of values and answers how many rows the whole batch \
            wrote — the loop around `PDOStatement::execute` that every driver writes by hand, with \
            one prepare and one round trip instead of one of each per set.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, written once and bound once per set. It is a sink exactly as \
                   `execute`'s is, and the batch gives it no second spelling: there is one text \
                   for every set.",
            shape: &[],
        },
        ParamDoc {
            name: "sets",
            desc: "One `$params` array per execution, each keyed the way `execute` requires and \
                   all of them binding the same number of values — a set whose `inList` is a \
                   different width is a different statement, not another row of this one.",
            shape: &[],
        },
    ],
    ret: "The sum of what each execution reported, with a command whose tag carries no count \
          contributing nothing. An empty `$sets` writes nothing and answers `0`. Rows a \
          `RETURNING` clause produced are discarded, and there is no `lastId`: neither has one \
          execution to belong to.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong rather than the database: a set is keyed both ways at once, \
                   two sets do not agree on how many values the statement binds, an element has no \
                   bound form, or a statement is already streaming on this connection.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server refused an execution — a syntax error, a constraint, a permission. \
                   Each execution is its own transaction, so the writes before the failing one \
                   stand; `transaction` is how a caller asks for all or nothing.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the batch was in flight, which leaves it unusable \
                   for the rest of the request.",
        },
    ],
};

/// `Core\Db\Queryable::transaction`'s reference card — ADR 0117.
const TRANSACTION_DOC: MethodDoc = MethodDoc {
    short: "Runs `$fn` inside a transaction and answers whatever it answered: returning commits, \
            throwing rolls back and propagates. Replaces `beginTransaction`/`commit`/`rollBack` \
            and every savepoint member with the one shape that cannot be left open by an early \
            return.",
    params: &[
        ParamDoc {
            name: "fn",
            desc: "The work. It is handed a `Core\\Db\\Transaction`, which has the same query \
                   surface the connection has, and may declare that parameter or no parameter at \
                   all.",
            shape: &[],
        },
        ParamDoc {
            name: "isolation",
            desc: "What this transaction may see of the work running beside it. Left out, it runs \
                   at the level the server was configured with. A nested call may not ask for one \
                   at all — the level belongs to the whole transaction, not to a savepoint inside \
                   it.",
            shape: &[],
        },
        ParamDoc {
            name: "readOnly",
            desc: "Refuses writes for the length of the transaction, which lets the server plan \
                   for a reader. False by default, and a nested call may not ask for it for the \
                   reason `isolation` may not.",
            shape: &[],
        },
        ParamDoc {
            name: "retries",
            desc: "How many times a deadlock or a serialization failure the commit reports may \
                   re-run `$fn`, outermost transactions only. Zero by default, because a closure \
                   with side effects should not be re-run without being asked for; nothing else \
                   is ever retried, there is no wait between attempts, and a conflict a statement \
                   inside `$fn` raised is thrown rather than re-run.",
            shape: &[],
        },
    ],
    ret: "What `$fn` returned, after the commit. A nested call on the same connection is a \
          savepoint, so a function that wraps its own writes stays callable from inside a \
          caller's transaction.",
    errors: &[
        ErrorDoc {
            error: "Core\\Db\\RolledBack",
            desc: "`$fn` called `rollBack`. It travels out of this call whether or not anything \
                   inside caught it, because the decision is a flag on the transaction and not \
                   the exception's own journey.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "A statement inside the closure was refused for the way it was written, the \
                   transaction was reached after the call that owned it returned, or a nested \
                   call asked for its own `isolation` or `readOnly`.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server refused the `BEGIN`, or refused the `COMMIT` after the closure \
                   returned — a serialization failure or a deferred constraint. The work is not \
                   committed either way.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the transaction was open, which leaves it unusable \
                   for the rest of the request.",
        },
    ],
};

/// `Core\Db\Transaction::rollBack`'s reference card — ADR 0117.
const ROLL_BACK_DOC: MethodDoc = MethodDoc {
    short: "Gives up on this transaction: records `$reason`, and throws `Core\\Db\\RolledBack` \
            carrying it. There is no way to ask for a rollback and carry on inside the same \
            transaction, which is the difference from a `setRollbackOnly` every layer has to \
            remember to check.",
    params: &[ParamDoc {
        name: "reason",
        desc: "Why the work is being abandoned. It becomes the thrown `RolledBack`'s `reason` \
               property and its message, so it is written for whoever reads the failure.",
        shape: &[],
    }],
    ret: "Nothing — this member always throws.",
    errors: &[
        ErrorDoc {
            error: "Core\\Db\\RolledBack",
            desc: "Always. It propagates out of the owning `transaction()` call even if something \
                   between here and there catches it, because the owning frame acts on the \
                   recorded reason rather than on seeing the throw.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "The transaction was reached after the `transaction()` call that owned it \
                   returned, so there is no longer a scope to roll back.",
        },
    ],
};

/// `Core\Db\Rows::all`'s reference card — ADR 0117.
const ROWS_ALL_DOC: MethodDoc = MethodDoc {
    short: "Every row of the result, in the server's order — `PDO::fetchAll` without a fetch-mode \
            argument to choose the shape with.",
    params: &[],
    ret: "An `array<T>`, empty for a statement that answered no rows. `T` is the result set's own \
          type argument: a `Core\\Db\\Row` for `query`, and the hydrated class for `queryAs<T>`. \
          The rows are the ones already read, so this costs one object each and no second decode.",
    errors: &[],
};

/// `Core\Db\Rows::first`'s reference card — ADR 0117.
const ROWS_FIRST_DOC: MethodDoc = MethodDoc {
    short: "The first row, or `null` where there is none — `PDO::fetch`, without its `false` and \
            without a cursor that a second call would move.",
    params: &[],
    ret: "A `T` — the result set's own type argument, as `all` describes — or `null` for an empty \
          result. `?T` is the absence spelling everywhere in `Core`, and a query that matched \
          nothing is an answer rather than a failure to throw about.",
    errors: &[],
};

/// `Core\Db\Rows::value`'s reference card — ADR 0117.
const ROWS_VALUE_DOC: MethodDoc = MethodDoc {
    short: "The first column of the first row — `PDO::fetchColumn`, and the shape a `select \
            count(*)` is read with.",
    params: &[],
    ret: "That column's value, or `null` where the result has no rows at all — which is the same \
          `null` a NULL column reads as, since the declared type is `mixed`. A caller that must \
          tell the two apart asks `count()` first.",
    errors: &[],
};

/// `Core\Db\Rows::column`'s reference card — ADR 0117.
const ROWS_COLUMN_DOC: MethodDoc = MethodDoc {
    short: "One column's value from every row, in the server's order — `PDO::fetchAll` under \
            `FETCH_COLUMN`, with the column named rather than a mode flag.",
    params: &[ParamDoc {
        name: "key",
        desc: "The column: an `int` is its position in the server's own description, counted from \
               zero, and a `string` is its label.",
        shape: &[],
    }],
    ret: "An `array<mixed>` with one entry per row, empty for a result with no rows.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "No column has that name, or the position is negative or past the last column. An \
               empty result answers an empty array instead, since it describes no columns to be \
               wrong about.",
    }],
};

/// `Core\Db\Rows::count`'s reference card — ADR 0117.
const ROWS_COUNT_DOC: MethodDoc = MethodDoc {
    short: "How many rows the statement answered — `PDOStatement::rowCount` on a select, which is \
            the use of that member this replaces. A write's count is `Core\\Db\\Write::affected`.",
    params: &[],
    ret: "The number of rows held, which is exact because § 4's default read all of them before \
          `query` returned.",
    errors: &[],
};

/// `Core\Db\Rows::columns`'s reference card — ADR 0117.
const ROWS_COLUMNS_DOC: MethodDoc = MethodDoc {
    short: "What the statement described, one `Core\\Db\\Column` per column and in the server's \
            own order — `PDOStatement::getColumnMeta` asked once for the whole row description \
            rather than once per column, and `mysqli_fetch_fields`.",
    params: &[],
    ret: "The columns. An empty result set has them too: a `select` that matched nothing still \
          described what it would have answered, which is what makes this readable before the \
          rows are.",
    errors: &[],
};

/// `Core\Db\Column::name`'s reference card — ADR 0117.
const COLUMN_LABEL_DOC: MethodDoc = MethodDoc {
    short: "The column's label, as the server described it — `getColumnMeta`'s `name`. It is the \
            alias wherever the `select` list wrote one, because an alias is what the server \
            describes.",
    params: &[],
    ret: "The label, and not a key: `select a, a` describes two columns under one label, so the \
          answer is read by position in the array `columns()` handed back.",
    errors: &[],
};

/// `Core\Db\Column::type`'s reference card — ADR 0117.
const COLUMN_DECLARED_DOC: MethodDoc = MethodDoc {
    short: "What the column was declared as, as a `Core\\Db\\ColumnType` case rather than the \
            vendor type name `getColumnMeta` hands back — so a program branches on something the \
            backends agree about.",
    params: &[],
    ret: "The case. It describes the column rather than summarising the value a read of it \
          produces — a `JSON` column and a `TEXT` one both read back as `tainted string` and are \
          told apart here — and a type with no Novis type of its own is `Other`, which includes \
          every array and, on PostgreSQL, every `ENUM`.",
    errors: &[],
};

/// `Core\Db\Column::nullable`'s reference card — ADR 0117.
const COLUMN_NULLABLE_DOC: MethodDoc = MethodDoc {
    short: "Whether the column may hold NULL. On PostgreSQL this is always `true`, because a row \
            description carries no NOT NULL flag: the only way to learn it is a catalog query per \
            statement, and this driver makes none.",
    params: &[],
    ret: "`true` on every column this driver describes, so a program reading it treats every \
          column as nullable — which is what the typed readers of `Core\\Db\\Row` already do, \
          each answering `?T`. A backend whose description carries the flag answers it here \
          instead.",
    errors: &[],
};

/// `Core\Db\Row::has`'s reference card — ADR 0117.
const ROW_HAS_DOC: MethodDoc = MethodDoc {
    short: "Reports whether the row has a column with this name, so that a reader that would throw \
            on an unknown one can be asked first.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it. Accepted `tainted`, because this \
               answers rather than throws.",
        shape: &[],
    }],
    ret: "`true` for a column the row carries, whatever its value — a NULL column is present. \
          `false` otherwise.",
    errors: &[],
};

/// `Core\Db\Row::get`'s reference card — ADR 0117.
const ROW_GET_DOC: MethodDoc = MethodDoc {
    short: "One column's value, whatever the SQL-to-Novis type map made of it — the universal \
            read, which a program narrows with `as` where the typed readers do not fit.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The value, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name.",
    }],
};

/// `Core\Db\Row::toArray`'s reference card — ADR 0117.
const ROW_TO_ARRAY_DOC: MethodDoc = MethodDoc {
    short: "The whole row as a string-keyed array, in the server's column order — `FETCH_ASSOC`, \
            which is the only one of PHP's three fetch shapes that survives.",
    params: &[],
    ret: "An `array<mixed>` keyed by column label, a NULL column being a `null` entry that is \
          present rather than absent.",
    errors: &[],
};

/// `Core\Db\Row::string`'s reference card — ADR 0117.
const ROW_STRING_DOC: MethodDoc = MethodDoc {
    short: "One column as `string`, for the text family alone — `CHAR`, `VARCHAR`, `TEXT`, `ENUM` \
            and `JSON`, each of which reads back as a `tainted string`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The text, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not text — a `BYTEA` is \
               `bytes` and is read by `->bytes`, and a number is not re-rendered here.",
    }],
};

/// `Core\Db\Row::bytes`'s reference card — ADR 0117.
const ROW_BYTES_DOC: MethodDoc = MethodDoc {
    short: "One column as `bytes` — `BINARY`, `BLOB` and `BYTEA`, which have no text form at all \
            and are a separate type from `string`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The octets, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is text rather than `bytes` — \
               the two are separate types and this reader does not span them.",
    }],
};

/// `Core\Db\Row::int`'s reference card — ADR 0117.
const ROW_INT_DOC: MethodDoc = MethodDoc {
    short: "One column as `int` — `SMALLINT`, `INT` and `BIGINT`, and an unsigned column whose \
            value fits.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The integer, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, the column is not an integer at all, or it is \
               an unsigned value past `int`'s ceiling — which PHP would hand back as a `float` \
               that no longer equals it.",
    }],
};

/// `Core\Db\Row::uint`'s reference card — ADR 0117.
const ROW_UINT_DOC: MethodDoc = MethodDoc {
    short: "One column as `uint` — MySQL's and MariaDB's `… UNSIGNED`, and a signed column that is \
            not negative.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The integer, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, the column is not an integer at all, or its \
               value is negative — which would wrap rather than convert.",
    }],
};

/// `Core\Db\Row::float`'s reference card — ADR 0117.
const ROW_FLOAT_DOC: MethodDoc = MethodDoc {
    short: "One column as `float` — `FLOAT`, `REAL` and `DOUBLE`, and nothing else.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The number, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not a floating-point one — a \
               `DECIMAL` is exact and is read by `->decimal`, since binary floating point is where \
               money stops adding up.",
    }],
};

/// `Core\Db\Row::bool`'s reference card — ADR 0117.
const ROW_BOOL_DOC: MethodDoc = MethodDoc {
    short: "One column as `bool` — `BOOLEAN` and `BIT(1)`, and an integer column holding `0` or \
            `1`, which is how MySQL's and MariaDB's `TINYINT(1)` is read: it is naturally an \
            `int`, and asking for a `bool` is what converts it.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The truth value, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, the column is neither boolean nor an \
               integer, or it is an integer holding something other than `0` or `1` — a stored \
               `7` is a refusal here rather than `true`.",
    }],
};

/// `Core\Db\Row::decimal`'s reference card — ADR 0117.
const ROW_DECIMAL_DOC: MethodDoc = MethodDoc {
    short: "One column as `decimal` — `DECIMAL`, `NUMERIC` and `MONEY`, exact, where PHP hands \
            back a string to parse.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The exact number, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not an exact numeric one.",
    }],
};

/// `Core\Db\Row::instant`'s reference card — ADR 0117.
const ROW_INSTANT_DOC: MethodDoc = MethodDoc {
    short: "One column as a `Core\\Time\\Instant` — `TIMESTAMPTZ` and `datetimeoffset`, the two \
            that carry their own zone.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The instant, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not a zone-carrying \
               timestamp — a zone-less one is a `Core\\Time\\DateTime` in the connection's \
               declared zone.",
    }],
};

/// `Core\Db\Row::date`'s reference card — ADR 0117.
const ROW_DATE_DOC: MethodDoc = MethodDoc {
    short: "One column as a `Core\\Time\\Date` — a `DATE`, which is a calendar day and carries no \
            time at all.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The day, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not a `DATE`.",
    }],
};

/// `Core\Db\Row::time`'s reference card — ADR 0117.
const ROW_TIME_DOC: MethodDoc = MethodDoc {
    short: "One column as a `Core\\Time\\TimeOfDay` — a `TIME`, which is a clock reading with no \
            day behind it.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The time of day, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not a `TIME`.",
    }],
};

/// `Core\Db\Row::uuid`'s reference card — ADR 0117.
const ROW_UUID_DOC: MethodDoc = MethodDoc {
    short: "One column as a `Core\\Uuid` — PostgreSQL's `UUID`, SQL Server's `uniqueidentifier` \
            and MariaDB 10.7+'s `UUID`. MySQL stores one as `BINARY(16)`, which stays `bytes`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The identifier, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not a native UUID one — a \
               `BINARY(16)` is `bytes` and a text rendering is a `string`.",
    }],
};

/// `Core\Db\Write::affected`'s reference card — ADR 0117.
const WRITE_AFFECTED_DOC: MethodDoc = MethodDoc {
    short: "How many rows the statement affected — `PDOStatement::rowCount` on a write, without \
            its documented unreliability on a select, because a select does not answer with one \
            of these at all.",
    params: &[],
    ret: "A `uint`, and `0` for a statement that affected none as well as for one whose kind has \
          no count to report — a `create table`. `changed` is where those two are told apart.",
    errors: &[],
};

/// `Core\Db\Write::changed`'s reference card — ADR 0117.
const WRITE_CHANGED_DOC: MethodDoc = MethodDoc {
    short: "The same count as the server itself reported it, whose `null` is the one thing \
            `affected` cannot say: this statement's kind carries no row count at all.",
    params: &[],
    ret: "A `?uint`, equal to `affected` wherever it is not `null`. On PostgreSQL the distinction \
          MySQL draws between rows matched and rows altered has nothing in the protocol to read \
          it out of, so inventing a second count that always equalled the first would be a \
          difference callers wrote code against.",
    errors: &[],
};

/// `Core\Db\Write::lastId`'s reference card — ADR 0117.
const WRITE_LAST_ID_DOC: MethodDoc = MethodDoc {
    short: "The key the statement handed back, read off the write that produced it rather than \
            off the connection — `lastInsertId` and `mysqli_insert_id` without their \
            stale-after-an-unrelated-statement hazard.",
    params: &[],
    ret: "A `?uint`: the first column of the last row the statement returned, where that column \
          was declared an integer, and `null` otherwise. On PostgreSQL that means a `RETURNING` \
          clause — the protocol has no last-insert-id of its own, and an `insert`'s tag carries \
          an OID that is `0` on every supported server.",
    errors: &[],
};

/// `Core\Db::connect`, as its own refusals spell it.
const CONNECT: &str = r"Core\Db::connect";

/// `Core\Db::open`, as its own refusals spell it — including the two ADR 0067
/// § 3 gives it and gives `connect` no equivalent of.
const OPEN: &str = r"Core\Db::open";

/// `Core\Db\Connection::query`, as its own refusals spell it.
const QUERY: &str = r"Core\Db\Connection::query";

/// `Core\Db\Connection::queryAs`, as its own refusals spell it — under
/// [`ADR 0043`](../../../../docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)'s
/// delegation, a call through a `Core\Db\Transaction` names the connection's
/// member here exactly as [`QUERY`] does.
const QUERY_AS: &str = r"Core\Db\Connection::queryAs";

/// `Core\Db\Connection::execute`, as its own refusals spell it. Both halves are
/// passed together to everything on the statement path — the short name for
/// [`connection_of`], which builds `Class::member` itself, and this one for the
/// messages that already hold a class.
const EXECUTE: &str = r"Core\Db\Connection::execute";

/// `Core\Db\Connection::executeMany`, as its own refusals spell it. See
/// [`EXECUTE`] for why both spellings travel together.
const EXECUTE_MANY: &str = r"Core\Db\Connection::executeMany";

/// `transaction`'s own name for a refusal, spelled on the connection because
/// that is the class that declares the row — a nested call on a
/// [`TRANSACTION`] reaches the same helper and so names the same member, which
/// is what delegating rather than re-declaring means.
const TRANSACTION_MEMBER: &str = r"Core\Db\Connection::transaction";

/// `rollBack`'s, which is the one member of that class with a body of its own.
const ROLL_BACK: &str = r"Core\Db\Transaction::rollBack";

/// The ABI slot each of `connect`'s two options arrives in — the row's one
/// positional parameter, then the bag flattened in declaration order.
const SHARED_ARG: usize = 1;
/// See [`SHARED_ARG`].
const TIMEOUT_ARG: usize = 2;

/// The ABI slot each of `transaction`'s options arrives in — the receiver, then
/// the row's one positional parameter, then [`TRANSACTION_OPTIONS`] flattened
/// in declaration order.
///
const ISOLATION_ARG: usize = 2;
/// See [`ISOLATION_ARG`].
const READ_ONLY_ARG: usize = 3;
/// See [`ISOLATION_ARG`].
const RETRIES_ARG: usize = 4;

/// The instant the handshake must be done by, or `None` for a call that named
/// no `timeout`.
///
/// # Errors
///
/// A thrown `RuntimeError` for a duration that is zero or negative, on
/// `Core\Http`'s reading: ADR 0074 § 5 has no spelling for an unbounded wait,
/// and a zero one is that spelling said quietly. A [`Fault::fatal`] for a slot
/// that is neither a `Duration` nor `Tag::Null`, which the row's type rules out.
fn deadline_of(args: &[Value]) -> Result<Option<std::time::Instant>, Fault> {
    if matches!(args[TIMEOUT_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let nanos = crate::time::nanos_of(args, TIMEOUT_ARG, "timeout")?;
    if nanos <= 0 {
        return Err(Fault::thrown(format!(
            "{CONNECT}: `timeout` must be a positive duration, and this one is {nanos}ns"
        )));
    }
    Ok(Some(
        std::time::Instant::now() + std::time::Duration::from_nanos(nanos.unsigned_abs()),
    ))
}

/// Where a block's `host` and `port` are, as one address.
///
/// **Pinned here and asked nothing else**, which is ADR 0067 § 3: the endpoint
/// was written into root-owned configuration by the same authority that granted
/// `db.connect`, so it is pre-approved and is *not* additionally checked against
/// [ADR 0058](../../../../docs/adr/0058-outbound-request-policy.md) § 3's denied
/// ranges — where every database on a container network or a `10/8` estate
/// lives. `Core\Db::open`'s host is program-supplied and stays subject to that
/// policy in full, which is the whole difference between the two members.
///
/// The name is resolved once and the resolved address is what the socket is
/// opened to, so nothing re-resolves between the check and the connection. What
/// the certificate is checked against stays the written host, which is the
/// target's own `host` and not this.
///
/// `default_port` is the driver's, and it is a parameter because 5432 and 3306
/// are different servers: the caller has already decided which handshake goes
/// out, and it is the only one that knows what a block writing no `port` meant.
///
/// # Errors
///
/// A thrown `IOError` for a host that resolves to nothing.
fn address_of(
    host: &str,
    port: Option<u16>,
    default_port: u16,
    name: &str,
) -> Result<SocketAddr, Fault> {
    let port = port.unwrap_or(default_port);
    let bare = host
        .strip_prefix('[')
        .and_then(|held| held.strip_suffix(']'))
        .unwrap_or(host);
    if let Ok(literal) = bare.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(literal, port));
    }
    (host, port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut found| found.next())
        .ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{CONNECT}: `[db.{name}]` names the host `{host}`, which resolves to no \
                         address"
                ),
            )
        })
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::connect(string $name, {shared?: bool, timeout?: Duration}): Db\Connection`
    /// — ADR 0067 § 2's named connection, memoized for the request.
    ///
    /// **The grant is asked first**, before the configuration is read at all,
    /// which is `Core\Mail::send`'s ordering and is the same property: a
    /// program with no grant learns nothing about which blocks a deployment
    /// wrote. A name outside the grant and a name with no block behind it are
    /// two different sentences, and only the second is reachable by a program
    /// the operator already trusted with that name.
    ///
    /// **The block is read from the boot snapshot and never through
    /// `Request::get`.** Every other reader of a `[…]` block in this crate goes
    /// through the per-request view, because a directive an operator marked
    /// `Runtime` can be moved by `Core\Config::set`; a credential is not one of
    /// those, and reading one through a table a program can write to would let
    /// a request choose the server its own query is answered by.
    ///
    /// **What it spends:** one connection — a socket, a TLS session and § 1's
    /// statement cache — per distinct name a request opens, held by the request
    /// and closed with it. A second `connect("main")` spends nothing at all,
    /// which is what § 2's memoization is for; `{shared: false}` opts out of
    /// that and is charged again.
    fn nvs_core_db_connect(ctx, args: [3]) {
        // Unreachable from source: parameter 0 is a `string` in `CLASS` above,
        // so a non-text argument is refused at `E0401` first — the same
        // judgement `quoteIdentifier`'s guard states. Owned, because the
        // capability check and the table below both want `ctx` back.
        let name = args[0]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Db::connect expected a `string` name, got tag {}",
                    args[0].tag_byte()
                ))
            })?
            .to_owned();
        // Unreachable from source for the same reason: the option is declared
        // `bool` and defaults to one, so the slot is never anything else.
        let shared = args[SHARED_ARG].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Db::connect expected a `bool` for `shared`, got tag {}",
                args[SHARED_ARG].tag_byte()
            ))
        })?;
        let deadline = deadline_of(args)?;

        nvs_runtime::capability::require(
            ctx,
            nvs_config::Cap::DbConnect,
            nvs_config::capability::Scope::Name(&name),
            CONNECT,
        )?;

        let key = open_named(ctx, &name, shared, deadline, CONNECT)?;
        let block = Value::str(NvsStr::new(name.as_bytes()));
        Ok(crate::instance::build(&CONNECTION, [Value::uint(key), block]))
    }
}

/// Opens `[db.<name>]` for this request and files it, answering the key it is filed
/// under — `Core\Db::connect`'s body from the memo down, and every ADR 0067 § 13
/// decision with it.
///
/// **Separate from the member because it has a second caller, and that caller is a
/// property rather than a convenience.** [`crate::queue`]'s `push` has to run on the
/// connection this request already holds under this name, because ADR 0084 § 3's
/// transactional enqueue *is* "the same connection". A second implementation would be
/// a second pool key, a second memo and a second reset — three places for § 13's
/// bounds to drift apart, and one silent way to lose § 3.
///
/// **The capability check is deliberately not in here.** `connect` asks `db.connect`
/// about a name the *program* wrote, which is what stops a request choosing its own
/// database; a queue's name comes out of root-owned configuration and no program can
/// influence it. Moving the check in would demand a grant for a name nobody chose —
/// [`crate::queue`]'s module doc is where that reading lives.
///
/// `named` is the member a refusal is worded for, so one failure reads as
/// `Core\Db::connect` or as `Core\Queue::push` depending on who asked.
///
/// # Errors
///
/// A thrown `RuntimeError` for a name no `[db.<name>]` block covers, or a block that
/// cannot be read as a connection. An `IOError` for a host that does not resolve, or
/// a connection, TLS handshake or login that failed. A `Db\DbError` for a pool whose
/// `acquire` bound expired with no slot free.
pub(crate) fn open_named(
    ctx: &mut nvs_runtime::Ctx,
    name: &str,
    shared: bool,
    deadline: Option<std::time::Instant>,
    named: &str,
) -> Result<u64, Fault> {
    let held = if shared {
        ctx.memoized_connection(name)
    } else {
        None
    };
    if let Some(key) = held {
        return Ok(key);
    }

    // The snapshot is cloned rather than borrowed because the block, the
    // target that borrows it and the `ctx` that files the connection are
    // all live at once. It is an `Arc` and a boot generation is shared by
    // every request on the core, so the clone is one refcount.
    let snapshot = ctx
        .config()
        .map(|config| std::sync::Arc::clone(config.snapshot()))
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{named}: this program is running with no configuration at all, so there is \
                     no `[db.{name}]` block to open"
            ))
        })?;
    let block = snapshot.config.db.get(name).ok_or_else(|| {
        Fault::thrown(format!(
            "{named}: nothing sets `[db.{name}]` up, so the name resolves to a block an \
             operator has not written yet"
        ))
    })?;
    // ADR 0067 § 13's ticket. The bounds were validated at boot by
    // `nvs_config::db::validate`, so the refusal below cannot fire; if it
    // ever did, `OFF` is the answer that closes this connection with the
    // request rather than pooling it under bounds nobody could resolve.
    let bounds = nvs_config::db::pool_for(name, block, &std::collections::BTreeMap::new())
        .unwrap_or(nvs_config::db::PoolBounds::OFF);
    let ticket = nvs_runtime::pool::Ticket::for_block(&snapshot, name, bounds);
    // § 13's ceiling, taken before the handshake so that `max` bounds every
    // connection this core has live under the key and not only the ones the
    // pool itself supplied. The slot goes back when the request ends and on
    // every failure path between here and there — `Lease`'s own `Drop` is
    // what makes that true, so no path below has to remember it.
    //
    // The clone is the price of asking without queueing first: a slot is
    // there on all but the busiest request, and paying one short `String`
    // for the case that has to wait beats a queue registration — a boxed
    // wake and a `Vec` push — on every `connect` that never waits at all.
    let max = ticket.bounds.max;
    let full = |waited: &str| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{CONNECT}: `[db.{name}]` already holds its `max` of {max} connections on this \
                 core, and {waited} — raise `[db.{name}.pool] max`, or hold fewer connections \
                 open at once"
            ),
        )
    };
    let lease = match nvs_runtime::pool::admit(ticket.clone()) {
        Some(lease) => lease,
        None => wait_for_slot(ctx, ticket, &full, deadline)?,
    };

    // § 13's acquire: this core's pool first, and what comes out of it is
    // reset before this request may use it. `warm_connection` is where a
    // failed reset destroys the connection, and `None` from it is
    // indistinguishable here from an empty pool — either way the fall-back
    // is the handshake below, which is what a request did before there was
    // a pool at all.
    let pooled = bounds.enabled.then(|| warm_connection(&lease)).flatten();
    // One wording for both handshakes: which driver could not reach its server
    // is the block's business, and what a program can do about either is the
    // same thing.
    let opening = |address: SocketAddr, err: &std::io::Error| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!("{named}: `[db.{name}]` at {address} did not open: {err}"),
        )
    };
    let written = block.driver.as_deref().unwrap_or("");
    let driver = nvs_db::Driver::from_config_name(written);
    let opened = match pooled {
        Some(warm) => warm,
        // ADR 0067 § 2's `driver` decides which handshake goes out, and it is
        // read here rather than inside a driver: the two openers share nothing
        // but this shape — their own target, their own default port, their own
        // `Connection` variant — and one resolver answering for both is the
        // trait ADR 0132 § 5 declines to write.
        //
        // Every other spelling goes to PostgreSQL, including the block that
        // writes no `driver` at all and the one whose `driver` no backend
        // answers to: `PgTarget::resolve` is where each of those refusals is
        // worded, and it names what was written rather than what it wanted.
        None => match driver {
            Some(nvs_db::Driver::MySql) => {
                let target = nvs_db::MySqlTarget::resolve(block).map_err(|refused| {
                    Fault::thrown(format!("{named}: {}", refused.refusal(name)))
                })?;
                let address =
                    address_of(target.host, block.port, nvs_db::mysql::DEFAULT_PORT, name)?;
                let conn = nvs_db::MySqlConn::connect(address, &target, deadline)
                    .map_err(|err| opening(address, &err))?;
                nvs_db::Connection::MySql(conn)
            }
            Some(nvs_db::Driver::MariaDb) => {
                let target = nvs_db::MariaTarget::resolve(block).map_err(|refused| {
                    Fault::thrown(format!("{named}: {}", refused.refusal(name)))
                })?;
                let address =
                    address_of(target.host, block.port, nvs_db::maria::DEFAULT_PORT, name)?;
                let conn = nvs_db::MariaConn::connect(address, &target, deadline)
                    .map_err(|err| opening(address, &err))?;
                nvs_db::Connection::MariaDb(conn)
            }
            _ => {
                let target = nvs_db::PgTarget::resolve(block).map_err(|refused| {
                    Fault::thrown(format!("{named}: {}", refused.refusal(name)))
                })?;
                let address = address_of(target.host, block.port, nvs_db::pg::DEFAULT_PORT, name)?;
                let conn = nvs_db::PgConn::connect(address, &target, deadline)
                    .map_err(|err| opening(address, &err))?;
                nvs_db::Connection::Postgres(conn)
            }
        },
    };
    // The lease is filed even for `{shared: false}`, whose `None` memo is
    // the slot beside it: that option bypasses memoization *within* the
    // request and never pooling across requests — § 13 says so in as many
    // words.
    let key = ctx.hold_open_connection(
        shared.then(|| name.to_owned()),
        Some(lease),
        Box::new(opened),
    );
    Ok(key)
}

/// ADR 0135 § 3's merged field list of [`SETTINGS`], as ABI slots: the server
/// arm's ten fields in order, then the SQLite arm's `path` — `driver`,
/// `timeZone` and `timeout` are already spoken for — then the trailing bag's
/// `shared`. Twelve, which is what `nvs_core_db_open` declares.
///
/// A slot belonging to an arm the caller did not write arrives as `Tag::Null`,
/// which is why the reads below check the driver first and then look only at
/// the slots that arm declares.
const DRIVER_ARG: usize = 0;
/// See [`DRIVER_ARG`].
const HOST_ARG: usize = 1;
/// See [`DRIVER_ARG`].
const PORT_ARG: usize = 2;
/// See [`DRIVER_ARG`].
const DATABASE_ARG: usize = 3;
/// See [`DRIVER_ARG`].
const USER_ARG: usize = 4;
/// See [`DRIVER_ARG`].
const PASSWORD_ARG: usize = 5;
/// See [`DRIVER_ARG`].
const TLS_ARG: usize = 6;
/// See [`DRIVER_ARG`].
const TIME_ZONE_ARG: usize = 7;
/// See [`DRIVER_ARG`].
const OPEN_TIMEOUT_ARG: usize = 8;
/// See [`DRIVER_ARG`].
const STATEMENT_CACHE_ARG: usize = 9;
/// See [`DRIVER_ARG`].
const PATH_ARG: usize = 10;
/// See [`DRIVER_ARG`].
const OPEN_SHARED_ARG: usize = 11;

/// The backend a `driver` slot names.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not one of [`DRIVER`]'s ordinals,
/// which the shape's own type rules out — [`isolation_of`] states the same
/// judgement at length.
fn settings_driver(value: &Value) -> Result<nvs_db::Driver, Fault> {
    match value.as_int() {
        Some(0) => Ok(nvs_db::Driver::MySql),
        Some(1) => Ok(nvs_db::Driver::MariaDb),
        Some(2) => Ok(nvs_db::Driver::Postgres),
        Some(3) => Ok(nvs_db::Driver::Sqlite),
        Some(4) => Ok(nvs_db::Driver::SqlServer),
        _ => Err(Fault::fatal(format!(
            "{OPEN} expected a `{DRIVER_NAME}` case for `driver`, got tag {}",
            value.tag_byte()
        ))),
    }
}

/// What a written `tls` key means, which is either "the only mode there is" or
/// a refusal naming what was asked for.
///
/// # Errors
///
/// A thrown `RuntimeError` for any of [`TLS`]'s weaker three. That enum's doc
/// owns why they are refused rather than honoured; the short of it is that
/// ADR 0067 § 3's TLS default is not configurable to the unsafe value, and a
/// connection that verified more than it was asked to would be a promise made
/// quietly.
fn settings_tls(value: &Value) -> Result<(), Fault> {
    let asked = match value.as_int() {
        None => return Ok(()),
        Some(3) => return Ok(()),
        Some(0) => "Disabled",
        Some(1) => "Required",
        Some(2) => "VerifyCa",
        Some(_) => {
            return Err(Fault::fatal(format!(
                "{OPEN} expected a `{TLS_NAME}` case for `tls`, got tag {}",
                value.tag_byte()
            )));
        }
    };
    Err(Fault::thrown(format!(
        "{OPEN}: `tls` asks for `Tls::{asked}`, and this runtime opens every TCP connection at \
         `Tls::VerifyFull` — ADR 0067 § 3 has no spelling for verifying less. A private \
         certificate authority is a `tls_ca_file` on a `[db.<name>]` block, which changes whose \
         certificates are believed and not whether they are checked"
    )))
}

/// A `uint` slot as its number, or `None` for the [`Const::Null`] an omitting
/// call site passed.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is neither, which the shape's own type
/// rules out.
fn settings_uint(value: &Value, key: &str) -> Result<Option<u64>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    value.as_uint().map(Some).ok_or_else(|| {
        Fault::fatal(format!(
            "{OPEN} expected a `uint` for `{key}`, got tag {}",
            value.tag_byte()
        ))
    })
}

/// A text slot the *written arm* requires.
///
/// **Thrown and not [`Fault::fatal`], which the two members either side of it
/// would be.** `E0402` refuses a literal that omits a key the merged list
/// requires, and the merged list can only require a key **every** arm does:
/// `driver` is one, `host` is not, because the SQLite arm does not declare it.
/// So a missing `host` reaches here as a `Tag::Null`, and until ADR 0135 § 2's
/// arm selection lands — this module's known gap 1 — it is a program error and
/// a catchable one, rather than an impossible state worth a `FATAL`.
///
/// # Errors
///
/// A thrown `RuntimeError` naming the key the written `driver` needed.
fn settings_text<'a>(args: &'a [Value], at: usize, key: &str) -> Result<&'a str, Fault> {
    args[at].as_text().ok_or_else(|| {
        Fault::thrown(format!(
            "{OPEN}: this `driver` needs a `{key}`, and the settings do not give one"
        ))
    })
}

/// The memo key one settings literal opens under — ADR 0067 § 2's "a hash of
/// every settings field", where `connect`'s key is the name an operator wrote.
///
/// **It cannot collide with a `connect` key**, which is the one property this
/// spelling has to have: both members file into the same per-request table, and
/// a `[db.<name>]` block's name is a configuration key and so can never begin
/// with a NUL byte. Two calls whose fields all agree share a connection, which
/// is what § 2 promises, and two that differ anywhere — including in the
/// password — do not.
///
/// **It is § 13's pool key as well as the memo**, which decides what it hashes:
/// every field that says what the connection *is* once it is open. The endpoint
/// and the credentials are the obvious ones; the declared zone and the
/// statement cache's size are the two that are only obvious from the pool's
/// side, because a drawn connection carries both from the request that opened
/// it — § 13's reset restores the zone rather than re-reading it, as a startup
/// parameter on PostgreSQL and as `set_session_time_zone` on MySQL — and a
/// second request that declared a different one would silently be answered in
/// the first one's. `timeout` and `tls` are deliberately not among them: the
/// first bounds the act of opening rather than the connection it produces, and
/// the second has one accepted value, so hashing either would only split a pool
/// two requests could have shared.
fn settings_key(
    fields: &[&str],
    port: Option<u64>,
    zone: i32,
    statement_cache: Option<u64>,
) -> String {
    use std::hash::{Hash as _, Hasher as _};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for field in fields {
        field.hash(&mut hasher);
    }
    port.hash(&mut hasher);
    zone.hash(&mut hasher);
    statement_cache.hash(&mut hasher);
    format!("\u{0}open:{:016x}", hasher.finish())
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::open(Db\Settings $settings, {shared?: bool}): Db\Connection`
    /// — ADR 0067 § 2's connection the *program* describes.
    ///
    /// **The whole difference from `connect` is which authority wrote the
    /// endpoint**, and § 3 turns that into two checks this body makes and that
    /// one does not. `db.open` is asked about the host rather than a block
    /// name, because a host is what a settings literal chooses; and the
    /// address it resolves to is then put through
    /// [ADR 0058](../../../../docs/adr/0058-outbound-request-policy.md) § 3's
    /// denied ranges in full, which is the check a `connect`-named endpoint is
    /// deliberately exempt from — [`address_of`]'s doc owns that asymmetry from
    /// the other side.
    ///
    /// **The settings become a `[db.<name>]` block and are resolved as one.**
    /// The two drivers' resolvers already own every refusal a set of fields can
    /// earn — a field belonging to another driver, a blank password, a zone
    /// that is not an offset — and re-deciding any of it here would be a second
    /// answer to a question ADR 0067 § 2 has one of. What this body decides is
    /// only what the config path has no equivalent of: the two checks above,
    /// and the `tls` key, which no block has.
    ///
    /// **What it spends:** one connection per distinct set of settings a
    /// request opens, and § 13's pool keeps up to `idle` of them per key on this
    /// core between the requests that use them. A settings literal has no
    /// `[db.<name>.pool]` table to size that with, so it takes
    /// `PoolBounds::DEFAULT` — the same bounds a block that writes no `pool` key
    /// takes, which is what ADR 0074's *finite with nothing configured* already
    /// means one layer down, and the only other candidate (`OFF`) is § 13
    /// declining to pool `open` at all, which that section spends a bullet
    /// requiring. Two consequences an operator has to be told rather than
    /// discover: the ceiling on the database is `cores × max` *per distinct
    /// settings hash* and that key space is the program's rather than the
    /// config's, and `pool = false` cannot reach an `open` because that switch
    /// is written per block and this has none.
    fn nvs_core_db_open(ctx, args: [12]) {
        let driver = settings_driver(&args[DRIVER_ARG])?;
        // The SQLite arm, whole: its `path` is the field that says the caller
        // wrote it, and there is no SQLite driver to hand it to — known gap 2's
        // roster, worded here rather than through `driverless` because that one
        // names the block a program did not write.
        if driver == nvs_db::Driver::Sqlite {
            let path = settings_text(args, PATH_ARG, "path")?;
            return Err(Fault::thrown(format!(
                "{OPEN}: `{}` is a driver this build has no connection path for yet, so the \
                 settings naming `{path}` cannot be opened",
                driver.display_name()
            )));
        }

        let host = settings_text(args, HOST_ARG, "host")?;
        let user = settings_text(args, USER_ARG, "user")?;
        let database = settings_text(args, DATABASE_ARG, "database")?;
        let password = settings_text(args, PASSWORD_ARG, "password")?;
        let port = settings_uint(&args[PORT_ARG], "port")?;
        let statement_cache = settings_uint(&args[STATEMENT_CACHE_ARG], "statementCache")?;
        settings_tls(&args[TLS_ARG])?;
        let shared = args[OPEN_SHARED_ARG].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "{OPEN} expected a `bool` for `shared`, got tag {}",
                args[OPEN_SHARED_ARG].tag_byte()
            ))
        })?;

        // § 3's grant, asked before anything is resolved or opened: a host
        // outside it is refused whether or not it exists.
        nvs_runtime::capability::require(
            ctx,
            nvs_config::Cap::DbOpen,
            nvs_config::capability::Scope::Host(host),
            OPEN,
        )?;
        // And § 3's other half — ADR 0058's table, which is what a
        // program-supplied address is subject to and an operator-written one is
        // not. The name is resolved once, here, and the socket below opens to
        // exactly the address that was checked.
        let pinned = nvs_runtime::capability::pinned_address(ctx, host, OPEN)?;

        // § 9's declared zone, resolved here rather than beside the target that
        // carries it because it is one of the fields the key below hashes: a
        // pooled connection is in the zone the request that opened it declared,
        // and § 13's reset restores that zone rather than re-reading it. It is
        // set on the resolved target rather than written into the block, because
        // the block's field is an offset *spelling* and this arrives as a
        // `Core\Time\Zone`: rendering it to text for the resolver to parse back
        // would be two conversions and one more place for the two to disagree.
        let zone = if matches!(args[TIME_ZONE_ARG].tag(), Some(Tag::Null)) {
            0
        } else {
            crate::time::zone_offset_now(args, TIME_ZONE_ARG, "open")?
        };
        let deadline = open_deadline(args)?;
        let memo = settings_key(
            &[host, user, database, password, driver.matrix_name()],
            port,
            zone,
            statement_cache,
        );
        if shared && let Some(key) = ctx.memoized_connection(&memo) {
            return Ok(crate::instance::build(
                &CONNECTION,
                [Value::uint(key), Value::str(NvsStr::new(host.as_bytes()))],
            ));
        }

        // § 13's ticket, under the key § 2 hashes rather than the block name
        // `connect` keys on — `Ticket::for_settings` owns why only one of the
        // two is scoped to a configuration generation, and this member's own
        // doc owns why the bounds are the defaults.
        let ticket = nvs_runtime::pool::Ticket::for_settings(
            memo.clone(),
            nvs_config::db::PoolBounds::DEFAULT,
        );
        let max = ticket.bounds.max;
        let full = |waited: &str| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{OPEN}: these settings already hold their `max` of {max} connections to \
                     {host} on this core, and {waited} — a settings literal is keyed on its own \
                     fields and takes bounds no `[db.<name>.pool]` table can raise, so open \
                     fewer of them at once"
                ),
            )
        };
        let lease = match nvs_runtime::pool::admit(ticket.clone()) {
            Some(lease) => lease,
            None => wait_for_slot(ctx, ticket, &full, deadline)?,
        };
        // § 13's acquire, and `{shared: false}` reaches it too: that option
        // bypasses the memo above and never the pool, which § 13 says in as
        // many words. `warm_connection` is where a failed reset destroys the
        // connection, so a `None` here is indistinguishable from an empty pool
        // and falls through to the handshake either way.
        let pooled = lease
            .bounds()
            .enabled
            .then(|| warm_connection(&lease))
            .flatten();
        let opened = match pooled {
            Some(warm) => warm,
            // The block the two resolvers read, built only on the path that
            // needs it. Every field is one the settings literal wrote, so what
            // comes back out is the same target a `[db.<name>]` block of the
            // same content would resolve to — including its refusals, which is
            // the point of going through them.
            None => {
                let block = nvs_config::tree::Database {
                    driver: Some(driver.matrix_name().to_owned()),
                    host: Some(host.to_owned()),
                    port: port.and_then(|written| u16::try_from(written).ok()),
                    user: Some(user.to_owned()),
                    password: Some(password.to_owned()),
                    database: Some(database.to_owned()),
                    statement_cache: statement_cache.and_then(|held| u32::try_from(held).ok()),
                    ..nvs_config::tree::Database::default()
                };
                let refused = |refusal: nvs_db::BlockError<'_>| {
                    Fault::thrown(format!("{OPEN}: {}", refusal.refusal("<settings>")))
                };
                let opening = |address: SocketAddr, err: &std::io::Error| {
                    Fault::thrown_as(
                        ThrownClass::Io,
                        format!("{OPEN}: {address} did not open: {err}"),
                    )
                };
                match driver {
                    nvs_db::Driver::MySql => {
                        let mut target = nvs_db::MySqlTarget::resolve(&block).map_err(refused)?;
                        target.time_zone = zone;
                        let address =
                            SocketAddr::new(pinned, port_of(port, nvs_db::mysql::DEFAULT_PORT));
                        let conn = nvs_db::MySqlConn::connect(address, &target, deadline)
                            .map_err(|err| opening(address, &err))?;
                        nvs_db::Connection::MySql(conn)
                    }
                    nvs_db::Driver::MariaDb => {
                        let mut target = nvs_db::MariaTarget::resolve(&block).map_err(refused)?;
                        target.time_zone = zone;
                        let address =
                            SocketAddr::new(pinned, port_of(port, nvs_db::maria::DEFAULT_PORT));
                        let conn = nvs_db::MariaConn::connect(address, &target, deadline)
                            .map_err(|err| opening(address, &err))?;
                        nvs_db::Connection::MariaDb(conn)
                    }
                    nvs_db::Driver::Postgres => {
                        let mut target = nvs_db::PgTarget::resolve(&block).map_err(refused)?;
                        target.time_zone = zone;
                        let address =
                            SocketAddr::new(pinned, port_of(port, nvs_db::pg::DEFAULT_PORT));
                        let conn = nvs_db::PgConn::connect(address, &target, deadline)
                            .map_err(|err| opening(address, &err))?;
                        nvs_db::Connection::Postgres(conn)
                    }
                    // SQL Server, which binds and decodes but has no handshake
                    // here — known gap 2's list, and the same refusal a block
                    // naming it earns. SQLite left above, at its own field.
                    other => {
                        return Err(Fault::thrown(format!(
                            "{OPEN}: `{}` is a driver this build has no connection path for yet, \
                             so the settings naming it cannot be opened",
                            other.display_name()
                        )));
                    }
                }
            }
        };
        let key =
            ctx.hold_open_connection(shared.then_some(memo), Some(lease), Box::new(opened));
        Ok(crate::instance::build(
            &CONNECTION,
            [Value::uint(key), Value::str(NvsStr::new(host.as_bytes()))],
        ))
    }
}

/// Which port a settings literal reaches: the one it wrote, or the driver's.
///
/// [`address_of`]'s rule, applied to the member that resolved its host
/// somewhere else: a written port that does not fit a `u16` cannot be a port at
/// all, so it falls back rather than truncating into one.
fn port_of(written: Option<u64>, default_port: u16) -> u16 {
    written
        .and_then(|held| u16::try_from(held).ok())
        .unwrap_or(default_port)
}

/// [`deadline_of`]'s twin for `open`, whose `timeout` is a *shape field* and so
/// arrives in a slot of its own.
///
/// # Errors
///
/// A thrown `RuntimeError` for a duration that is zero or negative, and a
/// [`Fault::fatal`] for a slot that is neither a `Duration` nor `Tag::Null`.
fn open_deadline(args: &[Value]) -> Result<Option<std::time::Instant>, Fault> {
    if matches!(args[OPEN_TIMEOUT_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let nanos = crate::time::nanos_of(args, OPEN_TIMEOUT_ARG, "timeout")?;
    if nanos <= 0 {
        return Err(Fault::thrown(format!(
            "{OPEN}: `timeout` must be a positive duration, and this one is {nanos}ns"
        )));
    }
    Ok(Some(
        std::time::Instant::now() + std::time::Duration::from_nanos(nanos.unsigned_abs()),
    ))
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::inList(array<mixed> $values): Db\InList` — ADR 0067 § 5's
    /// explicit expansion marker.
    ///
    /// The body is the refusal and a carrier around the argument: what the
    /// marker expands *to* is the driver's, for the reason this module's docs
    /// give. The array is retained rather than copied, so a list of a thousand
    /// ids costs one reference and not a second array.
    fn nvs_core_db_in_list(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is `array<mixed>` in `CLASS`
        // above, so a non-container argument is refused at `E0401` before any
        // of this runs. The guard is what makes the count below sound.
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Db::inList expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for the length of this call"
        )]
        let count = unsafe { nvs_runtime::nvs_array_count(array) };
        if count == 0 {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "Core\\Db::inList() was given an empty list, and ADR 0067 § 5 refuses one: an \
                 empty list matches nothing inside `IN` and everything inside `NOT IN`, the \
                 rewriter cannot tell which it is in, and the caller has to branch"
                    .to_owned(),
            ));
        }
        let values = args[0];
        #[expect(
            unsafe_code,
            reason = "the array is owned by the caller's argument slot, which \
                      outlives this call, so the slot this builds needs a \
                      reference of its own"
        )]
        unsafe {
            values.retain();
        }
        Ok(crate::instance::build(&IN_LIST, [values]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::quoteIdentifier(tainted string $name): string` — ADR 0024
    /// § 4's launderer for the statement-text sink.
    ///
    /// It validates rather than escapes, and this module's docs are the whole
    /// argument for why: with no connection there is no dialect, and the
    /// character class below is the one answer that is safe under all five at
    /// once.
    fn nvs_core_db_quote_identifier(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is a `string` in `CLASS` above,
        // so a non-text argument is refused at `E0401` first — the same
        // judgement `Core\Arr::count`'s guard states.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Db::quoteIdentifier expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        if !is_bare_identifier(name) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "Core\\Db::quoteIdentifier() takes a bare identifier — a letter or `_`, then \
                     letters, digits or `_` — and `{name}` is not one. `Core\\Db` has no \
                     connection and so no dialect to delimit for, so a name that needs \
                     delimiting is refused rather than quoted for the wrong backend"
                ),
            ));
        }
        Ok(Value::str(NvsStr::new(name.as_bytes())))
    }
}

/// Whether `name` is an identifier every backend reads the same way, and which
/// carries nothing into the statement text it is written into.
///
/// ASCII on purpose. Every backend also accepts some set of non-ASCII letters,
/// and no two of those sets are the same — `char::is_alphabetic` would accept a
/// name PostgreSQL takes and SQL Server folds differently, which is exactly the
/// dialect dependence this member exists to avoid having.
fn is_bare_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// The request-table key and the `[db.<name>]` block behind a
/// `Core\Db\Connection` receiver.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not an object or whose handle slot
/// holds the wrong tag, on `Core\IO\File`'s reading of the identical pair: both
/// slots are written by [`nvs_core_db_connect`] and by nothing else, so either
/// is a paste error in this crate rather than anything a program can cause.
fn connection_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
    let receiver = crate::instance::receiver(value, &CONNECTION, member)?;
    let key = crate::instance::slot(receiver, HANDLE_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{CONNECTION_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                CONNECTION.slots[HANDLE_AT]
            ))
        })?;
    Ok((key, crate::instance::slot(receiver, BLOCK_AT)))
}

/// The same pair off a [`TRANSACTION`], plus ADR 0067 § 7's first hazard.
///
/// The scope check is here rather than in each member because it is the
/// interface's rule and not any one member's: a `$tx` that escaped its
/// `transaction()` call still names a live connection, and running its
/// statement outside the transaction — silently, on whatever the connection is
/// doing now — is the failure § 7 closes by name.
///
/// # Errors
///
/// A thrown `LogicError` for a transaction whose call has returned. A
/// [`Fault::fatal`] for a slot of the wrong tag, as [`connection_of`].
fn transaction_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
    let receiver = crate::instance::receiver(value, &TRANSACTION, member)?;
    if crate::instance::slot(receiver, SCOPE_AT).as_bool() != Some(true) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{TRANSACTION_NAME}::{member}: transaction scope has ended"),
        ));
    }
    let key = crate::instance::slot(receiver, HANDLE_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{TRANSACTION_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                TRANSACTION.slots[HANDLE_AT]
            ))
        })?;
    Ok((key, crate::instance::slot(receiver, BLOCK_AT)))
}

/// The connection a `Core\Db\Queryable` member runs on, off **either** receiver
/// the interface has.
///
/// This is the whole of ADR 0043's delegation at runtime. `Transaction
/// implements Queryable by $connection` gives the two classes one set of rows
/// under one set of symbols ([`TRANSACTION`] says why), so the helper behind a
/// row is handed whichever receiver the call site wrote and asks here which one
/// it got — rather than four forwarding bodies that would each be a second
/// place for the statement path to be written.
///
/// The order is deliberate: a [`TRANSACTION`] is asked about first because it
/// is the receiver carrying an extra rule, and [`connection_of`] is the
/// fallthrough that also produces the `Fault::fatal` for anything that is
/// neither.
fn handle_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
    if crate::instance::is_instance(value, &TRANSACTION) {
        return transaction_of(value, member);
    }
    connection_of(value, member)
}

/// One element of `$params`, as both halves of the statement path need it: what
/// it does to the SQL text, and the values the bind reads out of it.
///
/// The two are one walk's answers and are carried together because the rewriter
/// is told the arity **before** anything is encoded — ADR 0067 § 5's expansion
/// changes the text, and § 1's statement cache keys on the text it changed it
/// to, so an `inList`'s width has to be known a round trip early.
struct Bound {
    /// The rewriter's view of this element: one marker, or § 5's run of them.
    binding: nvs_db::Binding,
    /// What binds at those markers — one value, or the whole run in the array's
    /// own order. **Keys are not read**, which is `inList`'s own rule: a marker
    /// binds positions inside one placeholder and not names.
    ///
    /// Borrowed rather than retained, as every value read out of an argument in
    /// this crate is: the caller's `$params` array owns them and outlives the
    /// call.
    values: Vec<Value>,
}

/// One element of `$params`, read as either an ordinary value or ADR 0067
/// § 5's expansion marker.
fn bound_of(value: Value) -> Bound {
    if !crate::instance::is_instance(value, &IN_LIST) {
        return Bound {
            binding: nvs_db::Binding::One,
            values: vec![value],
        };
    }
    // Its one slot is written by `nvs_core_db_in_list` and by nothing else, and
    // that member refuses an empty list — so this is an array, and the arity
    // below is at least one.
    let held = crate::instance::slot(
        value.obj_ptr().expect("an `InList` is an object"),
        VALUES_AT,
    );
    let values = held
        .array_ptr()
        .map(|array| {
            let list = crate::arr::borrowed(array);
            let mut held = Vec::with_capacity(list.count());
            let mut from = 0usize;
            while let Some(slot) = list.next_slot(from) {
                from = slot + 1;
                held.push(
                    list.value_at(slot)
                        .expect("next_slot only names live entries"),
                );
            }
            held
        })
        .unwrap_or_default();
    Bound {
        binding: nvs_db::Binding::List(values.len()),
        values,
    }
}

/// A statement path failure, worded by which half of it went wrong.
///
/// The three-way split is this crate's to make, because `nvs-db` builds no
/// fault at all (its `Cargo.toml` § 1 is the rule) and answers in
/// [`std::io::ErrorKind`]s instead. `InvalidInput` is every mistake **in the
/// call** — the rewriter's refusals, a value with no bound form, a second
/// statement on a streaming connection — so it is a `LogicError` the caller
/// fixes by writing the call differently. `Other` is the server's own refusal,
/// carrying its `SQLSTATE` and message. Everything left is the wire.
///
/// The middle one is `Core\Db\DbError` — [ADR 0067](../../../docs/adr/0067-core-db.md)
/// § 8's single class for every refusal the server made, sitting beside
/// `Core\Db\RolledBack` in spec § 10's tree so that a `catch` can tell a
/// refusal the program did not choose from one it did. Nothing about the
/// message changes with the class, and § 8 requires it to carry no bound value.
///
/// The `Core\Db\DbError` it builds carries § 8's `kind`:
/// `nvs_db::ServerError::of(refused)` reads the driver's own classification
/// back out of the error this function is handed, and
/// [`Fault::thrown_with_slot`] writes it into
/// [`nvs_runtime::KIND_SLOT`] as the throw is recorded. A refusal the driver
/// answered with no [`nvs_db::ServerError`] behind it reads as `Other`, which
/// is what § 8 defines that case to be — the condition a code table does not
/// name — so the property is written on every path and never `null`.
///
/// **`sql` is the statement the caller wrote and not [`Statement::sql`]'s
/// rewrite of it**, and `None` for a member with no caller-written statement to
/// name. § 8 lets the text ride the throw because it is developer-authored,
/// which the rewritten form is only at one remove: that form spells its markers
/// the way one driver wants them — `$1` here, `?` on MySQL — so carrying it
/// would make a property of a deliberately *normalised* error read differently
/// per driver, which is the thing § 8's `kind` exists to stop. § 7's `BEGIN`,
/// `COMMIT` and `SAVEPOINT` pass `None` for the other half of the same reason:
/// that text is this runtime's, no program asked for it by name, and `?string`
/// already has an absent case that costs no slot.
fn statement_failure(
    named: &str,
    block: &Value,
    sql: Option<&str>,
    refused: &std::io::Error,
) -> Fault {
    let name = block.as_text().unwrap_or("?");
    match refused.kind() {
        std::io::ErrorKind::InvalidInput => {
            Fault::thrown_as(ThrownClass::Logic, format!("{named}: {refused}"))
        }
        std::io::ErrorKind::Other => {
            let message = format!("{named}: `[db.{name}]` refused the statement: {refused}");
            // Built once for both arms below: whether the driver classified the
            // refusal says nothing about whether there was a statement behind
            // it, so `sql` is not the server's half of the error and does not
            // follow the server's.
            let wrote = sql.map(|text| {
                (
                    nvs_runtime::SQL_SLOT,
                    Value::str(NvsStr::new(text.as_bytes())),
                )
            });
            match nvs_db::ServerError::of(refused) {
                // The raw codes ride beside the kind normalised from them, so an
                // application that § 8's eleven conditions do not cover reads
                // what the server actually said without the driver having to
                // widen that enum. `driverCode` joins the `SQLSTATE` only on a
                // backend that sends a vendor integer as well — MySQL does and
                // PostgreSQL does not, and `nvs_db::ServerError` owns why. It is
                // never the `SQLSTATE` again under a second name.
                //
                // And the `SQLSTATE` itself joins only where there is one:
                // § 8 spells it `?string` because TDS has no such field at all,
                // and an empty `sql_state` is how that driver says so — the
                // field's own doc owns the reading. Writing it anyway would put
                // `""` where a program tests for `null`.
                Some(server) => {
                    let mut slots = vec![(nvs_runtime::KIND_SLOT, error_kind_value(server.kind))];
                    if !server.sql_state.is_empty() {
                        slots.push((
                            nvs_runtime::SQL_STATE_SLOT,
                            Value::str(NvsStr::new(server.sql_state.as_bytes())),
                        ));
                    }
                    if let Some(code) = server.driver_code {
                        slots.push((nvs_runtime::DRIVER_CODE_SLOT, Value::int(i64::from(code))));
                    }
                    // `constraint` joins them only where the condition named
                    // one, which most conditions do not. An unwritten slot
                    // already reads `null`, so the absent case costs no value
                    // here and no branch in the program that reads it — the
                    // same reason `driverCode` above is written only where a
                    // server sends one.
                    if let Some(constraint) = &server.constraint {
                        slots.push((
                            nvs_runtime::CONSTRAINT_SLOT,
                            Value::str(NvsStr::new(constraint.as_bytes())),
                        ));
                    }
                    slots.extend(wrote);
                    Fault::thrown_with_slots(ThrownClass::DbError, message, slots)
                }
                None => {
                    let mut slots = vec![(
                        nvs_runtime::KIND_SLOT,
                        error_kind_value(nvs_db::DbErrorKind::Other),
                    )];
                    slots.extend(wrote);
                    Fault::thrown_with_slots(ThrownClass::DbError, message, slots)
                }
            }
        }
        _ => Fault::thrown_as(
            ThrownClass::Io,
            format!("{named}: `[db.{name}]` failed while the statement was running: {refused}"),
        ),
    }
}

/// A [`nvs_db::DbErrorKind`] as the [`ERROR_KIND`] case a program matches on,
/// which at runtime is that case's ordinal
/// ([ADR 0010](../../../docs/adr/0010-enums-are-a-value-type.md)) — so a
/// `match ($e->kind) { Core\Db\ErrorKind::Deadlock => … }` reads what the
/// server itself said.
///
/// [`column_type_value`]'s rule, for its reason: the ordinal is looked up
/// rather than written a second time, and what is spelled here is only the
/// *name* correspondence neither half of the enum knows.
/// `every_db_error_kind_case_is_named` holds it total in both directions, which
/// is also what makes the `expect` unreachable.
fn error_kind_value(of: nvs_db::DbErrorKind) -> Value {
    let case = error_kind_case(of);
    let (_, ordinal) = ERROR_KIND
        .cases
        .iter()
        .find(|(name, _)| *name == case)
        .expect("every `nvs_db::DbErrorKind` names a case `ERROR_KIND` registers");
    Value::int(*ordinal)
}

/// The [`nvs_db::DbErrorKind`] a `Core\Db\ErrorKind` value is, or `None` for
/// anything that is not one of its ordinals — [`error_kind_value`] read
/// backwards, which is how § 7's retry rule asks what a *thrown* `Db\DbError`
/// carries when there is no [`nvs_db::ServerError`] left to ask.
///
/// Inverted through the forward function rather than written as a second
/// `match`: the name correspondence exists once, in [`error_kind_case`], and a
/// table spelled out again here would be free to disagree with it. The scan is
/// over eleven entries on a failure path.
fn error_kind_of(value: Value) -> Option<nvs_db::DbErrorKind> {
    let ordinal = value.as_int()?;
    EVERY_ERROR_KIND
        .into_iter()
        .find(|kind| error_kind_value(*kind).as_int() == Some(ordinal))
}

/// Every [`nvs_db::DbErrorKind`], in [`ERROR_KIND`]'s own order.
///
/// Written out because that enum carries no roster of its own, and guarded
/// rather than trusted: `every_db_error_kind_case_is_named` maps this list
/// through [`error_kind_case`] and compares it against the registered cases,
/// so a variant left out here is a case name with nothing producing it.
const EVERY_ERROR_KIND: [nvs_db::DbErrorKind; 11] = [
    nvs_db::DbErrorKind::UniqueViolation,
    nvs_db::DbErrorKind::ForeignKeyViolation,
    nvs_db::DbErrorKind::NotNullViolation,
    nvs_db::DbErrorKind::CheckViolation,
    nvs_db::DbErrorKind::Deadlock,
    nvs_db::DbErrorKind::SerializationFailure,
    nvs_db::DbErrorKind::ConnectionLost,
    nvs_db::DbErrorKind::Timeout,
    nvs_db::DbErrorKind::Syntax,
    nvs_db::DbErrorKind::Permission,
    nvs_db::DbErrorKind::Other,
];

/// The [`ERROR_KIND`] case one [`nvs_db::DbErrorKind`] is, by name.
///
/// Exhaustive on purpose — a variant added over there arrives here as a
/// non-exhaustive `match` rather than as a refusal that classifies wrongly.
fn error_kind_case(of: nvs_db::DbErrorKind) -> &'static str {
    match of {
        nvs_db::DbErrorKind::UniqueViolation => "UniqueViolation",
        nvs_db::DbErrorKind::ForeignKeyViolation => "ForeignKeyViolation",
        nvs_db::DbErrorKind::NotNullViolation => "NotNullViolation",
        nvs_db::DbErrorKind::CheckViolation => "CheckViolation",
        nvs_db::DbErrorKind::Deadlock => "Deadlock",
        nvs_db::DbErrorKind::SerializationFailure => "SerializationFailure",
        nvs_db::DbErrorKind::ConnectionLost => "ConnectionLost",
        nvs_db::DbErrorKind::Timeout => "Timeout",
        nvs_db::DbErrorKind::Syntax => "Syntax",
        nvs_db::DbErrorKind::Permission => "Permission",
        nvs_db::DbErrorKind::Other => "Other",
    }
}

/// The refusal for a column of one of ADR 0067 § 9's five class-typed rows
/// holding a value the `Core\Time` type it maps to has no representation for.
///
/// § 9's last paragraph is the rule: a structured column that does not parse
/// throws rather than reading back as something else. Three values reach it in
/// practice — PostgreSQL's `TIME` of `24:00:00`, which is a reading
/// `Core\Time\TimeOfDay` deliberately does not have, MySQL's zero date
/// `0000-00-00`, which its own driver hands over unchecked because the calendar
/// is over here ([`nvs_db::MySqlDate`]), and a year outside the calendar
/// `Core\Time`'s types count. `crate::time`'s seams own every one of those
/// bounds and answer `None`; naming the column is this side's half, since that
/// is what the program's next act needs.
fn unrepresentable_column(named: &str, column: &str, row: &str) -> Fault {
    Fault::thrown(format!(
        "{named}: the column `{column}` holds a {row} that no `Core\\Time` type has a value for \
         — PostgreSQL's `24:00:00`, MySQL's zero date and a year outside the calendar \
         `Core\\Time\\Date` counts are the three — and a cast to text in the statement reads one \
         back as the server rendered it"
    ))
}

/// A statement the wire is ready for: which connection it goes to, § 5's
/// rewritten text, and its values encoded in the order that text asks for them.
///
/// The two members that send one differ **only in what they do with the
/// answer**. ADR 0067 § 4 gives `query` and `execute` one signature and one
/// binding rule, so everything up to the send is [`statement_of`] and the
/// members are the two ways of reading a stream that has already started —
/// which is also why a write's values are checked exactly as a read's are, with
/// no second path for a caller to find a difference in.
struct Statement {
    /// The key its connection is filed under in the request's own table.
    key: u64,
    /// The `[db.<name>]` block it was opened by, so a refusal can name the
    /// connection without holding it.
    block: Value,
    /// § 5's rewritten text, in the driver's own placeholder spelling. A
    /// refusal names the caller's own spelling instead — [`statement_failure`]'s
    /// `sql` parameter — because this one is a property of the driver.
    sql: String,
    /// One entry per marker that text holds, in the **statement's** order and
    /// never the array's — `None` where the bound value is `null`.
    binds: Vec<Option<Vec<u8>>>,
}

/// Everything ADR 0067 §§ 4 and 5 do to a call before it reaches the socket:
/// § 18's `$params` rule, the rewrite, and the encoding.
///
/// **Both halves are the receiver's own driver's**, which is the one thing a
/// caller cannot state: [`rendering_of`] reads it off the connection filed
/// under the receiver's key, and § 5's rewrite and § 9's encoding follow it
/// together. The context is borrowed for that lookup alone and released before
/// the caller reaches [`transacting`], so a member still binds and sends inside
/// one borrow each.
///
/// Both spellings of the member's name are passed because two things want
/// different ones: `member` is the bare name [`connection_of`] builds
/// `Class::member` out of, and `named` is the whole spelling the messages here
/// already hold a class in. The argument slots are read the same way for each
/// member, since § 18 gives both the identical two parameters.
///
/// # Errors
///
/// [`rendering_of`]'s throw for a driver with no statement path yet, a thrown
/// `LogicError` for a `$params` keyed both ways at once, and whatever
/// [`statement_failure`] makes of the rewriter's and the encoder's refusals. A
/// [`Fault::fatal`] for an argument of the wrong tag, which the registry row
/// refuses first.
fn statement_of(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    member: &str,
    named: &str,
) -> Result<Statement, Fault> {
    let (key, block) = handle_of(args[0], member)?;
    let (dialect, encode) = rendering_of(ctx, key, &block, named)?;
    statement_in(dialect, encode, key, block, args, named)
}

/// [`statement_of`] with the connection already asked about, so a batch asks
/// once for every set it binds rather than once per set.
///
/// The key and the block are passed in for that reason and not carried back out
/// of [`handle_of`] again: they are the receiver's, and every set of a batch has
/// the same one.
///
/// # Errors
///
/// [`statement_of`]'s, less the lookup it has already done.
fn statement_in(
    dialect: nvs_db::Dialect,
    encode: Encoder,
    key: u64,
    block: Value,
    args: &[Value],
    named: &str,
) -> Result<Statement, Fault> {
    // Unreachable from source: parameter 0 is a `string` in `CONNECTION`
    // above, so a non-text argument is refused at `E0401` first — the same
    // judgement `Core\Db::quoteIdentifier`'s guard states.
    let sql = args[1].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{named} expected a `string` statement, got tag {}",
            args[1].tag_byte()
        ))
    })?;
    // Unreachable for that reason too: the row declares `array<mixed>`.
    let params = args[2].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{named} expected {:?} for its parameters, got tag {}",
            Tag::Array,
            args[2].tag_byte()
        ))
    })?;

    // § 18: "list-keyed for `?`, string-keyed for `:name`, mixing throws".
    // The refusal is here rather than in the rewriter because an array is
    // the only thing that can be both, and the rewriter is handed one form.
    let held = crate::arr::borrowed(params);
    let mut positional: Vec<Bound> = Vec::new();
    let mut keys: Vec<(String, Bound)> = Vec::new();
    let mut from = 0usize;
    while let Some(slot) = held.next_slot(from) {
        from = slot + 1;
        let value = held
            .value_at(slot)
            .expect("next_slot only names live entries");
        let bound = bound_of(value);
        match held
            .slot_key(slot)
            .expect("next_slot only names live entries")
        {
            nvs_runtime::SlotKey::Index(_) if keys.is_empty() => positional.push(bound),
            nvs_runtime::SlotKey::Str(name) if positional.is_empty() => {
                // A key is a Novis `string` and so is UTF-8 by ADR 0009;
                // the lossy read is the spelling that needs no unreachable
                // arm to say so.
                keys.push((String::from_utf8_lossy(name.as_bytes()).into_owned(), bound));
            }
            _ => {
                return Err(Fault::thrown_as(
                    ThrownClass::Logic,
                    format!(
                        "{named}: `$params` is keyed both ways at once, and a statement is \
                         written one way or the other — a list-keyed array binds `?` in \
                         order, a string-keyed one binds `:name`"
                    ),
                ));
            }
        }
    }

    let arities: Vec<nvs_db::Binding> = positional.iter().map(|bound| bound.binding).collect();
    let keyed: Vec<(&str, nvs_db::Binding)> = keys
        .iter()
        .map(|(name, bound)| (name.as_str(), bound.binding))
        .collect();
    let spelling = if keys.is_empty() {
        nvs_db::Params::Positional(&arities)
    } else {
        nvs_db::Params::Named(&keyed)
    };
    let rewritten = nvs_db::rewrite(sql, spelling, dialect)
        .map_err(|refused| statement_failure(named, &block, Some(sql), &refused))?;

    let bounds: Vec<&Bound> = if keys.is_empty() {
        positional.iter().collect()
    } else {
        keys.iter().map(|(_, bound)| bound).collect()
    };
    let mut rendered: Vec<Option<Vec<u8>>> = Vec::with_capacity(rewritten.binds.len());
    for source in &rewritten.binds {
        rendered.push(
            encode(bounds[source.arg].values[source.element])
                .map_err(|refused| statement_failure(named, &block, Some(sql), &refused))?,
        );
    }

    Ok(Statement {
        key,
        block,
        sql: rewritten.sql,
        binds: rendered,
    })
}

/// ADR 0067 § 4's batch: one statement the wire is ready for, and one encoded
/// set of values per execution it is about to get.
///
/// It is deliberately not a `Vec<Statement>`. Every set rewrites to the *same*
/// text or the batch is refused, so the text is held once here — which is also
/// the invariant, written into the shape rather than left as a rule
/// [`batch_of`] has to be trusted to have checked.
struct Batch {
    /// The key its connection is filed under in the request's own table.
    key: u64,
    /// The `[db.<name>]` block it was opened by, so a refusal can name the
    /// connection without holding it.
    block: Value,
    /// § 5's rewritten text, which every set agreed on — or, for an empty
    /// `$sets`, the caller's own text unrewritten, which never reaches the wire
    /// because [`nvs_db::PgConn::execute_many`] answers `0` before it prepares
    /// anything.
    sql: String,
    /// One encoded set per execution, each in the **statement's** order, as
    /// [`Statement::binds`] is.
    binds: Vec<Vec<Option<Vec<u8>>>>,
}

/// § 18's `$sets`, read as one [`statement_of`] per set with the expansions
/// checked to agree.
///
/// **Each set is a whole `$params`**, so it gains § 18's keying rule, § 5's
/// rewrite and § 9's encoding from the member that already owns them — a set
/// cannot be bound by a weaker rule than the one `execute` would have applied
/// to it on its own.
///
/// **The sets must rewrite to one text, and that is a stronger check than the
/// arity one it looks like.** § 1's cache is keyed on the SQL *plus its
/// expansion arity* and § 5 expands an `inList` into as many markers as it has
/// elements, so two sets whose `inList`s differ in width are two prepared
/// statements — the driver's own `execute_many` refuses them on the count, and
/// this refuses them on the text, which is the thing the count stands for and
/// can name in the message.
///
/// # Errors
///
/// A thrown `LogicError` for two sets that do not rewrite alike, plus whatever
/// [`statement_in`] throws for any one of them and [`rendering_of`]'s throw for
/// a driver with no statement path yet. A [`Fault::fatal`] for an argument of
/// the wrong tag, which the registry row refuses first.
fn batch_of(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    member: &str,
    named: &str,
) -> Result<Batch, Fault> {
    let (key, block) = handle_of(args[0], member)?;
    // Once for the batch: every set binds to the same connection, so asking per
    // set would be the same answer read `$sets` times.
    let (dialect, encode) = rendering_of(ctx, key, &block, named)?;
    // Unreachable from source for both, as in `statement_of`: the row declares
    // a `string` and an `array<array<mixed>>`, so `E0401` refuses either tag
    // first.
    let sql = args[1].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{named} expected a `string` statement, got tag {}",
            args[1].tag_byte()
        ))
    })?;
    let given = args[2].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{named} expected {:?} for its parameter sets, got tag {}",
            Tag::Array,
            args[2].tag_byte()
        ))
    })?;

    let held = crate::arr::borrowed(given);
    let mut text: Option<String> = None;
    let mut binds: Vec<Vec<Option<Vec<u8>>>> = Vec::with_capacity(held.count());
    let mut from = 0usize;
    while let Some(slot) = held.next_slot(from) {
        from = slot + 1;
        let set = held
            .value_at(slot)
            .expect("next_slot only names live entries");
        let at = binds.len();
        // The inner type's turn to be unreachable: the element is an
        // `array<mixed>` by the row, and `statement_of` would call a non-array
        // one a wrong parameter tag without saying which set it was.
        if set.array_ptr().is_none() {
            return Err(Fault::fatal(format!(
                "{named} expected {:?} for the set at {at}, got tag {}",
                Tag::Array,
                set.tag_byte()
            )));
        }
        let one = statement_in(dialect, encode, key, block, &[args[0], args[1], set], named)?;
        match &text {
            None => text = Some(one.sql),
            Some(first) if *first == one.sql => {}
            Some(first) => {
                return Err(Fault::thrown_as(
                    ThrownClass::Logic,
                    format!(
                        "{named}: the set at {at} binds `{}` where the first set binds `{first}`, \
                         and ADR 0067 § 1's cache is keyed on the statement's expansion — so two \
                         sets whose `inList`s differ in width are two statements and not one \
                         batch, and each of them wants its own call",
                        one.sql
                    ),
                ));
            }
        }
        binds.push(one.binds);
    }

    Ok(Batch {
        key,
        block,
        sql: text.unwrap_or_else(|| sql.to_owned()),
        binds,
    })
}

/// A connection out of this core's pool under `lease`'s key, reset and ready to
/// run a statement — ADR 0067 § 13's acquire, where the reset is the gate.
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
/// The SQLite arm is dropped here rather than reset, because a reset nobody has
/// written is not a reset that failed — § 13 makes it a security boundary, and
/// the only safe reading of a missing one is that the connection is not
/// poolable. It is spelled rather than left to a `_` so that a sixth driver
/// arrives as a build failure instead of as a connection silently thrown away.
/// MariaDB's arm is MySQL's: `COM_RESET_CONNECTION` is one protocol's command
/// and § 13 says of both that it drops the prepared statements with the session
/// state.
///
/// **Nothing reaches the SQL Server arm yet**, because `connect` and `open`
/// have no handshake for that driver — the module doc's known gap 2 is the
/// list. The arm is here rather than after it for the reason § 13 gives: a
/// driver that becomes openable while this function still answers `None` for it
/// is a pool that quietly stops pooling, which nothing observable would report.
fn warm_connection(lease: &nvs_runtime::pool::Lease) -> Option<nvs_db::Connection> {
    let held = nvs_runtime::pool::take(lease, std::time::Instant::now())?;
    let connection = held.into_any().downcast::<nvs_db::Connection>().ok()?;
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
        nvs_db::Connection::Sqlite(_) => None,
    }
}

/// Waits for a slot under `ticket`'s key — ADR 0067 § 13's `acquire` — or
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
/// [ADR 0106](../../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
/// § 6's tier-B failure.
///
/// **The wording of that refusal is the caller's**, handed in as `full` and
/// completed with the clause saying which ending it was. The two members have
/// nothing to say in common there: `connect` names a block and the
/// `[db.<name>.pool] max` an operator can raise, and `open` has neither — a
/// settings literal is keyed on its own hash and takes bounds nothing can
/// configure. What this function owns is the waiting, which *is* the same for
/// both.
///
/// # Errors
///
/// A thrown `IOError` for the ceiling reached, which is where every ending but
/// one lands: it is the refusal that stood beside the handshake that "did not
/// open" before there was any waiting, on the same reading — what a program can
/// do about either is the same, and § 8's `Db\DbError` is for a refusal the
/// *server* made, which this is not. The one other ending is [`Ctx::cancel`]'s
/// status for a task cancelled while it waited, which no `catch` sees.
fn wait_for_slot(
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
/// protocol.
///
/// A pointer rather than a `match` at the two call sites because § 5's dialect
/// and § 9's encoding are **one** choice: a statement rewritten for one
/// protocol and bound for another is refused by nothing here — `?` and `$1` are
/// both valid text, `t` and `1` are both valid bytes — and fails at the server
/// or, worse, binds the wrong value. [`rendering_for`] is the single place the
/// pair is made.
type Encoder = fn(Value) -> std::io::Result<Option<Vec<u8>>>;

/// ADR 0067 § 5's dialect and § 9's encoder for one driver, or `None` for a
/// driver with no statement path yet — this module's known gap 2.
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
/// `pub(crate)` for its `None`, which is this crate's one roster of the drivers
/// nothing binds for at all. It is **not** [`crate::queue`]'s roster any more:
/// ADR 0084 § 2's schema is written for three drivers and this binds for four,
/// so that module's `no_dialect` splits on its own `migration` instead. The two
/// agreed only while the lists were equal, and SQL Server is where they parted.
pub(crate) fn rendering_for(driver: nvs_db::Driver) -> Option<(nvs_db::Dialect, Encoder)> {
    let encode: Encoder = match driver {
        nvs_db::Driver::Postgres => nvs_db::encode,
        nvs_db::Driver::MySql | nvs_db::Driver::MariaDb => nvs_db::mysql::encode,
        nvs_db::Driver::SqlServer => nvs_db::tds::encode,
        nvs_db::Driver::Sqlite => return None,
    };
    Some((nvs_db::Dialect::of(driver), encode))
}

/// [`rendering_for`] the connection filed under `key`, which is how a statement
/// is written in its own connection's dialect rather than in one this module
/// picked.
///
/// It is asked **before** anything is rewritten, where [`driverless`] is asked
/// after everything is bound — the two refusals therefore name different
/// halves of gap 2, and a driver that can bind but not send says so at the
/// send.
///
/// # Errors
///
/// A thrown `RuntimeError` for a driver nothing binds for yet, and
/// [`filed_connection`]'s [`Fault::fatal`]s for a table this crate filled
/// wrongly.
fn rendering_of(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    block: &Value,
    named: &str,
) -> Result<(nvs_db::Dialect, Encoder), Fault> {
    let driver = filed_connection(ctx, key, named)?.driver();
    rendering_for(driver).ok_or_else(|| {
        Fault::thrown(format!(
            "{named}: `[db.{}]` is a {driver:?} connection, and no statement is written in its \
             dialect yet — this module's known gap 2 is the list",
            block.as_text().unwrap_or("?")
        ))
    })
}

/// The `nvs-db` connection filed under `key`, whichever driver it is.
///
/// The one downcast in this module: [`transacting`] narrows it further and
/// [`rendering_of`] only reads its driver, and either written on its own is a
/// second place holding the two `Fault::fatal`s that say the request's own
/// table is wrong.
///
/// # Errors
///
/// A [`Fault::fatal`] for a key the request's table does not hold, and another
/// for an entry that is not this crate's — both this crate's paste error rather
/// than a program's.
fn filed_connection<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
    named: &str,
) -> Result<&'a mut nvs_db::Connection, Fault> {
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

/// A connection ADR 0067 § 7's commands are written for, borrowed as one thing.
///
/// The drivers spell a transaction differently — `nvs_db::mysql`'s `begin`
/// owns the differences, from `START TRANSACTION` down to the release a nested
/// rollback does not owe — but they answer the same four questions, and
/// `transaction` asks them at five points around a closure it does not control.
/// An enum here rather than a trait in `nvs-db`: which commands a backend sends
/// is exactly what this goal's ADR slot refuses to flatten, and what this needs
/// is the *call sites* flattened rather than the drivers.
///
/// The key and the block are passed to [`transacting`] rather than a
/// [`Statement`] or a [`Batch`], because they are the only two fields it reads
/// and a batch is not a statement — the alternative is a `Statement` built with
/// an empty `binds` purely to reach this, which would be a shape nothing else
/// in this module means.
enum Transacting<'a> {
    /// § 7 over the extended-query protocol's simple `Query`.
    Postgres(&'a mut nvs_db::PgConn),
    /// § 7 over `COM_QUERY`, with an isolation level as a command of its own.
    MySql(&'a mut nvs_db::MySqlConn),
    /// The same commands over the same framing — `nvs_db::mysql`'s `begin`,
    /// `commit` and `roll_back` are what `nvs_db::MariaConn` delegates to, as
    /// [`Framed`] says of the send path.
    MariaDb(&'a mut nvs_db::MariaConn),
}

impl Transacting<'_> {
    /// How many of § 7's levels are open — 0 outside a transaction.
    fn depth(&self) -> u32 {
        match self {
            Transacting::Postgres(postgres) => postgres.depth(),
            Transacting::MySql(mysql) => mysql.depth(),
            Transacting::MariaDb(maria) => maria.depth(),
        }
    }

    /// § 7's outermost `BEGIN`, or the `SAVEPOINT` a nested call opens.
    ///
    /// # Errors
    ///
    /// As the driver's own `begin`, including the refusal of a nested call that
    /// asked for either option.
    fn begin(
        &mut self,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Transacting::Postgres(postgres) => postgres.begin(isolation, read_only),
            Transacting::MySql(mysql) => mysql.begin(isolation, read_only),
            Transacting::MariaDb(maria) => maria.begin(isolation, read_only),
        }
    }

    /// § 7's `COMMIT`, or the release that closes a nested level.
    ///
    /// # Errors
    ///
    /// As the driver's own `commit`.
    fn commit(&mut self) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Transacting::Postgres(postgres) => postgres.commit(),
            Transacting::MySql(mysql) => mysql.commit(),
            Transacting::MariaDb(maria) => maria.commit(),
        }
    }

    /// § 7's `ROLLBACK`, or the undo of a nested level.
    ///
    /// # Errors
    ///
    /// As the driver's own `roll_back`.
    fn roll_back(&mut self) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Transacting::Postgres(postgres) => postgres.roll_back(),
            Transacting::MySql(mysql) => mysql.roll_back(),
            Transacting::MariaDb(maria) => maria.roll_back(),
        }
    }
}

/// The connection filed under `key`, as the driver § 7's commands run on.
///
/// # Errors
///
/// A thrown `RuntimeError` for a block naming a driver with no transaction
/// behind it — [`driverless`]'s wording, because it is the same known gap 2 the
/// statement members refuse under — and a [`Fault::fatal`] for a key the
/// request's own table does not hold, which is this crate's paste error rather
/// than a program's.
fn transacting<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
    block: &Value,
    named: &str,
) -> Result<Transacting<'a>, Fault> {
    match filed_connection(ctx, key, named)? {
        nvs_db::Connection::Postgres(postgres) => Ok(Transacting::Postgres(postgres)),
        nvs_db::Connection::MySql(mysql) => Ok(Transacting::MySql(mysql)),
        nvs_db::Connection::MariaDb(maria) => Ok(Transacting::MariaDb(maria)),
        other => Err(driverless(named, block, other.driver(), BEYOND_READING)),
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Connection::query(string $sql, array<mixed> $params): Db\Rows`
    /// — ADR 0067 § 4's buffered statement, and the first member of
    /// `Core\Db\Queryable` to land.
    ///
    /// **Every row is read before this returns**, which is § 4's default and
    /// the whole difference between this and `stream`: the connection is at a
    /// message boundary again by the time the caller has its answer, so the
    /// loop that reads rows and writes per row — the commonest one in web
    /// programming — needs no second connection and cannot meet § 4's
    /// connection-busy rule at runtime. [`ROWS`] is where what that spends is
    /// written down.
    ///
    /// **The bind order is the rewriter's and not the array's.** § 5's
    /// `rewrite` answers one `Source` per marker, and a repeated `:name` on a
    /// numbered dialect is one marker read twice — so the values go out in the
    /// order the *statement* asks for them, which is why nothing here counts
    /// placeholders itself and why an `inList`'s expansion needs no second
    /// pass.
    fn nvs_core_db_connection_query(ctx, args: [3]) {
        let answered = queried_rows(ctx, args, "query", QUERY)?;
        Ok(crate::instance::build(
            &ROWS,
            [
                Value::array(answered.rows),
                Value::null(),
                Value::array(answered.columns),
            ],
        ))
    }
}

/// What one statement answered, in the two shapes a [`ROWS`] holds it in.
///
/// The pair rather than the rows alone because they come out of one borrow and
/// are wanted at one place: [`ROWS_COLUMNS_SLOT`] says why the description is
/// built at query time, and the alternative — handing back a `Vec<PgColumn>`
/// for the caller to build objects from — would put half of that at each of
/// [`nvs_core_db_connection_query`] and
/// [`nvs_core_db_connection_query_as`] instead of neither.
struct Answered {
    /// Every row, as [`ROWS_SLOT`] holds them.
    rows: NvsArray,
    /// One [`COLUMN`] per described column, as [`ROWS_COLUMNS_SLOT`] holds
    /// them.
    columns: NvsArray,
}

/// One statement's rows and the columns it described, as the two arrays a
/// [`ROWS`] holds — the whole of what `query` and `queryAs` share, which is
/// everything except which class the result carries.
///
/// **`args` starts at the receiver**, so `queryAs` hands over the slice past
/// [`crate::registry::WRITTEN_CLASS_MEMBERS`]' two leading constants and both
/// members read one shape here. Nothing about the statement differs between
/// them: § 4 gives them one signature and one binding rule, and hydration is a
/// property of the result rather than of the wire.
///
/// # Errors
///
/// [`statement_of`]'s refusals, a thrown `RuntimeError` for a driver with no
/// send path yet, [`statement_failure`] for anything the server refused, and
/// [`column_value`]'s or [`mysql_column_value`]'s for a column whose value has
/// no Novis representation.
fn queried_rows(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    member: &str,
    named: &str,
) -> Result<Answered, Fault> {
    let statement = statement_of(ctx, args, member, named)?;
    // § 18's `$sql` argument read a second time rather than [`Statement::sql`]:
    // what a refusal names is the text the program wrote, where that field is
    // § 5's rewrite of it. The tag is already known good — `statement_of`
    // refused anything else above — so the `None` arm here is unreachable and
    // costs no message of its own.
    let source = args[1].as_text();
    let sending: Vec<Option<&[u8]>> = statement.binds.iter().map(|one| one.as_deref()).collect();
    // Read before the statement takes the context, because it holds it for as
    // long as the rows do — see [`QueryWatch`] for the rest.
    let watch = QueryWatch::of(ctx, &statement.block);
    // The event is filed after the match and not inside it, because a driver's
    // rows borrow the connection and the connection borrows the context — so
    // the arm that read the span is still holding the thing the span is filed
    // on. Each arm hands back what [`QueryWatch::taken`] took, which is `None`
    // where nothing is reading rather than where a driver has no span.
    let (answered, taken) = match filed_connection(ctx, statement.key, named)? {
        nvs_db::Connection::Postgres(postgres) => {
            postgres_rows(postgres, &statement, &sending, source, watch, named)?
        }
        nvs_db::Connection::MySql(mysql) => mysql_rows(
            Framed::MySql(mysql),
            &statement,
            &sending,
            source,
            watch,
            named,
        )?,
        nvs_db::Connection::MariaDb(maria) => mysql_rows(
            Framed::MariaDb(maria),
            &statement,
            &sending,
            source,
            watch,
            named,
        )?,
        nvs_db::Connection::SqlServer(tds) => {
            tds_rows(tds, &statement, &sending, source, watch, named)?
        }
        other => return Err(driverless(named, &statement.block, other.driver(), READING)),
    };
    watch.file(ctx, taken);
    Ok(answered)
}

/// Known gap 2's roster for the two members that only read a result set —
/// § 4's `query` and `queryAs`, both of them [`queried_rows`].
///
/// **A roster per member and not one list**, which is the whole of what SQL
/// Server changed here: it runs [`tds_rows`] and reaches no other send member,
/// so a single sentence would either tell an operator calling `query` that the
/// driver cannot or tell one calling `transaction` that it can. Each is a
/// message an operator would act on wrongly.
const READING: &[nvs_db::Driver] = &[
    nvs_db::Driver::Postgres,
    nvs_db::Driver::MySql,
    nvs_db::Driver::MariaDb,
    nvs_db::Driver::SqlServer,
];

/// Known gap 2's roster for the three members that are more than one read —
/// `execute`'s two counts, § 4's `executeMany` and § 7's `transaction`.
///
/// It is [`READING`] less SQL Server, and the reason is `nvs_db::TdsConn`'s own
/// surface rather than a decision taken here: it has `query` and a reset and
/// nothing else, so there is no `execute_many` and no `begin` for an arm to
/// reach. `execute` is the one of the three that could be written against the
/// method already there — `nvs_db::tds::TdsConn::query`'s doc says `execute` is
/// that same method — and it has no arm yet.
const BEYOND_READING: &[nvs_db::Driver] = &[
    nvs_db::Driver::Postgres,
    nvs_db::Driver::MySql,
    nvs_db::Driver::MariaDb,
];

/// The refusal a connection whose driver has no path to `named` draws — this
/// module's known gap 2, worded once.
///
/// Four members reach it ([`queried_rows`], `execute`, `executeMany` and
/// § 7's `transaction`, through [`transacting`]) and a message per member would
/// be four sentences to keep agreeing as the lists shorten. It names the driver
/// the block actually resolved to, because "this one is not supported" without
/// saying which is what an operator cannot act on.
///
/// **`reaching` is the caller's own roster**, [`READING`] or [`BEYOND_READING`],
/// because the four members stopped agreeing when SQL Server gained one arm and
/// not four. It is rendered by [`named_drivers`] rather than written into the
/// sentence, so the list an operator reads is the list a `match` arm below
/// actually has.
fn driverless(
    named: &str,
    block: &Value,
    driver: nvs_db::Driver,
    reaching: &[nvs_db::Driver],
) -> Fault {
    Fault::thrown(format!(
        "{named}: `[db.{}]` is a {} connection, and only {} run this member so far — this \
         module's known gap 2 is the list",
        block.as_text().unwrap_or("?"),
        driver.display_name(),
        named_drivers(reaching)
    ))
}

/// A roster as the sentence [`driverless`] builds reads it: `A`, `A and B`,
/// `A, B and C`.
///
/// Spelled out rather than joined with commas throughout, because an operator
/// reads this and a trailing `, ` list reads as a truncation.
fn named_drivers(drivers: &[nvs_db::Driver]) -> String {
    let mut sentence = String::new();
    for (index, driver) in drivers.iter().enumerate() {
        if index > 0 {
            sentence.push_str(if index + 1 == drivers.len() {
                " and "
            } else {
                ", "
            });
        }
        sentence.push_str(driver.display_name());
    }
    sentence
}

/// ADR 0067 § 4's `Write`, as the driver answered it and before it becomes the
/// instance.
///
/// Two `Option`s and not two numbers: § 4 gives both fields `?uint`, and the
/// absence is a different fact from a zero on both drivers — a command that
/// carries no affected count at all, and a statement that generated no id.
struct Written {
    /// The server's own affected count, `None` for a command that carries none.
    changed: Option<u64>,
    /// § 4's `lastId`, `None` where the statement generated no id.
    last_id: Option<u64>,
}

/// [`queried_rows`] over the PostgreSQL driver: the extended-query stream, § 9's
/// decode of every row, and ADR 0067 § 11's span taken off the rows before they
/// are dropped.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused, and [`column_value`]'s
/// for a column whose value has no Novis representation.
fn postgres_rows(
    postgres: &mut nvs_db::PgConn,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Answered, Option<(String, std::time::Duration)>), Fault> {
    // Read before the statement borrows the connection, and once for the whole
    // result: § 9's zone-less `TIMESTAMP` is decoded in the zone this
    // connection declared, and that is a property of the connection rather
    // than of the row.
    let zone = postgres.time_zone();
    let mut answered = postgres
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    // Taken before the first row: a `PgRows` lends its columns and its rows
    // out of one borrow, and the rows are read with it held mutably.
    let columns: Vec<nvs_db::PgColumn> = answered.columns().to_vec();
    let described = described_columns(&columns);

    let mut rows = NvsArray::new();
    loop {
        let Some(row) = answered
            .next_row()
            .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
        else {
            break;
        };
        // Built whole before it joins the result, so that a column this
        // driver cannot read back releases the row it was half way through
        // rather than leaving it in one — `NvsArray`'s own `Drop`.
        let mut one = NvsArray::new();
        for (index, column) in columns.iter().enumerate() {
            let body = row
                .column(index)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            let scalar = column
                .scalar(body)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            let value = column_value(scalar, zone, named, &column.name)?;
            one.set(NvsStr::new(column.name.as_bytes()), value);
        }
        rows.append(Value::array(one));
    }
    // After the drain, so the span carries the duration the caller waited and
    // the rows it actually got.
    let taken = watch.taken(answered.span());
    Ok((
        Answered {
            rows,
            columns: described,
        },
        taken,
    ))
}

/// A connection `nvs_db::mysql`'s statement path is written for, borrowed as
/// one thing.
///
/// **MariaDB is its own driver above the framing, not inside it.** ADR 0067 § 2
/// is emphatic that treating it as a MySQL flag is a design error, and
/// `nvs_db::maria` obeys that where it counts — its own targets, its own
/// authentication roster, its own § 8 code table. What it does not duplicate is
/// the wire: `nvs_db::MariaConn::query` is a two-line delegation into the same
/// `nvs_db::mysql::start_statement` that `nvs_db::MySqlConn::query` is, and it
/// hands back the same `nvs_db::MySqlRows`. So the only difference this module
/// can observe between the two on the send path is the *type of the borrow*,
/// and the three send members would otherwise each grow a second arm that
/// copies the first line for line.
///
/// An enum rather than a trait, for [`Transacting`]'s reason and no other: what
/// wants flattening is the call sites. Where the two drivers genuinely part —
/// MariaDB's `RETURNING`, which MySQL does not have — the arm belongs on the
/// connection in `nvs-db`, and nothing about it reaches here.
///
/// **`pub(crate)` because [`crate::queue`] sends over the same two drivers**, and
/// for [`QueryWatch`]'s reason: ADR 0084's members drive a result set themselves
/// rather than through this class's, so a second borrow-flattening enum over
/// there would be this one with the same two arms. It carries § 7's three
/// commands as well as the send, which [`Transacting`] also spells — the two are
/// not one type because that one covers PostgreSQL, whose arm the queue's splits
/// must not reach: a `Split` is what a driver *without* the single-statement
/// construct runs, and PostgreSQL runs the single statement instead.
pub(crate) enum Framed<'a> {
    /// § 1's two round trips as MySQL frames them.
    MySql(&'a mut nvs_db::MySqlConn),
    /// The same two, framed as MariaDB and authenticated by its own roster.
    MariaDb(&'a mut nvs_db::MariaConn),
}

impl Framed<'_> {
    /// § 9's zone a zone-less `DATETIME` off this connection is read in, as
    /// seconds east of UTC.
    fn time_zone(&self) -> i32 {
        match self {
            Framed::MySql(mysql) => mysql.time_zone(),
            Framed::MariaDb(maria) => maria.time_zone(),
        }
    }

    /// ADR 0067 § 1's round trips for one statement, and the rows it answers
    /// with.
    ///
    /// # Errors
    ///
    /// As the driver's own `query`, which on both is
    /// `nvs_db::mysql::start_statement`'s.
    pub(crate) fn query(
        &mut self,
        sql: &str,
        params: &[Option<&[u8]>],
    ) -> std::io::Result<nvs_db::MySqlRows<'_>> {
        match self {
            Framed::MySql(mysql) => mysql.query(sql, params),
            Framed::MariaDb(maria) => maria.query(sql, params),
        }
    }

    /// § 7's `START TRANSACTION`, or the `SAVEPOINT` a nested one is.
    ///
    /// # Errors
    ///
    /// As the driver's own `begin`.
    pub(crate) fn begin(
        &mut self,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.begin(isolation, read_only),
            Framed::MariaDb(maria) => maria.begin(isolation, read_only),
        }
    }

    /// § 7's `COMMIT`, or the release that closes a nested level.
    ///
    /// # Errors
    ///
    /// As the driver's own `commit`.
    pub(crate) fn commit(&mut self) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.commit(),
            Framed::MariaDb(maria) => maria.commit(),
        }
    }

    /// § 7's `ROLLBACK`, or the undo of a nested level.
    ///
    /// # Errors
    ///
    /// As the driver's own `roll_back`.
    pub(crate) fn roll_back(&mut self) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.roll_back(),
            Framed::MariaDb(maria) => maria.roll_back(),
        }
    }
}

/// [`queried_rows`] over the two drivers [`Framed`] covers: ADR 0067 § 1's
/// `COM_STMT_EXECUTE`, and § 9's decode of the binary rows it answers with.
///
/// **The same shape as [`postgres_rows`] and deliberately not shared with it.**
/// The two drivers agree on what a row *is* — a keyed array under the labels the
/// result set described — and on nothing else in the walk: the description is
/// read off the stream here and off a cloned `PgColumn` there, a value is a
/// `MyValue` read against its own definition rather than a body the column
/// decodes, and the two `Scalar` enums are two sets of rows because MySQL has no
/// `UUID` and no array type. A trait over that would be four abstract methods
/// standing for eight concrete lines.
///
/// **§ 11's event is the one thing the two do share**, down to the line: a
/// `MySqlRows` opens its own span exactly as a `PgRows` does, so the pair this
/// hands back is [`postgres_rows`]' pair and a trace reads across the two
/// drivers without a field being spelled twice.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused, [`mysql_column_value`]'s
/// for a column whose value has no Novis representation, and a [`Fault::fatal`]
/// for a row narrower than the definitions it was decoded against, which is a
/// `nvs-db` bug rather than a program's.
fn mysql_rows(
    mut framed: Framed<'_>,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Answered, Option<(String, std::time::Duration)>), Fault> {
    // As [`postgres_rows`], and § 9's zone rule is the connection's on both
    // drivers.
    let zone = framed.time_zone();
    let mut answered = framed
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    // Described before the first row, because the description is read out of a
    // shared borrow of the stream and the rows out of a mutable one — the same
    // ordering `postgres_rows` gets by cloning its columns, and here the clone
    // is needed anyway: `nvs_db::mysql::scalar` reads a value against the
    // definition it arrived under.
    let described = mysql_described_columns(&answered);
    let columns = answered.columns().to_vec();

    let mut rows = NvsArray::new();
    while let Some(row) = answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
    {
        // Built whole before it joins the result, for [`postgres_rows`]' reason.
        let mut one = NvsArray::new();
        for (index, column) in columns.iter().enumerate() {
            // Unreachable: `nvs-db` decodes one value per definition, so a row
            // is exactly as wide as this loop. It is a `fatal` rather than a
            // refusal because a narrower row is that crate disagreeing with
            // itself and not something a statement can ask for.
            let body = row.value(index).ok_or_else(|| {
                Fault::fatal(format!(
                    "{named}: the row has no column {index}, where the result set described {}",
                    columns.len()
                ))
            })?;
            let scalar = nvs_db::mysql::scalar(column, body)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            // A label is bytes on this driver and text on the other, and § 9
            // reads both as UTF-8: the lossy decode is for the message only,
            // where the key keeps the octets the server sent.
            let label = String::from_utf8_lossy(column.name_ref());
            let value = mysql_column_value(scalar, zone, named, &label)?;
            one.set(NvsStr::new(column.name_ref()), value);
        }
        rows.append(Value::array(one));
    }
    // After the drain, as [`postgres_rows`]: the terminator is what freezes the
    // duration, and the rows counted are the ones that came back.
    let taken = watch.taken(answered.span());
    Ok((
        Answered {
            rows,
            columns: described,
        },
        taken,
    ))
}

/// [`queried_rows`] over the SQL Server driver: ADR 0067 § 1's `sp_prepexec`,
/// and § 9's decode of the token stream it answers with.
///
/// **[`mysql_rows`]' shape a third time**, and that function's doc argues at
/// length why the three are not one walk. What differs here is smaller than
/// what differs between the other two: a value is the row's own octets read
/// against the column `COLMETADATA` described, which is `nvs_db::tds::scalar`'s
/// reading rather than this module's, and a label is a `String` because TDS
/// carries it as UCS-2 and the driver has already decoded it — so the key is
/// that text's octets and no lossy decode stands between the two.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused, [`tds_column_value`]'s
/// for a column whose value has no Novis representation, and a [`Fault::fatal`]
/// for a row narrower than the columns it was decoded against, which is a
/// `nvs-db` bug rather than a program's.
fn tds_rows(
    tds: &mut nvs_db::TdsConn,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Answered, Option<(String, std::time::Duration)>), Fault> {
    // As the other two drivers, and § 9's zone is a property of the connection
    // on all three. What is this driver's own is that no server was told:
    // `nvs_db::tds::TdsTarget::time_zone` owns why SQL Server has nowhere to be.
    let zone = tds.time_zone();
    let mut answered = tds
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    // Cloned before the first row, for [`postgres_rows`]' reason: the columns
    // are lent out of a shared borrow and the rows out of a mutable one.
    let columns: Vec<nvs_db::tds::TdsColumn> = answered.columns().to_vec();
    let described = tds_described_columns(&answered);

    let mut rows = NvsArray::new();
    while let Some(row) = answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
    {
        // Built whole before it joins the result, for [`postgres_rows`]' reason.
        let mut one = NvsArray::new();
        for (index, column) in columns.iter().enumerate() {
            // Unreachable and fatal for [`mysql_rows`]' reason: `nvs-db` reads
            // one value per described column, so a row is exactly as wide as
            // this loop.
            let body = row.column(index).ok_or_else(|| {
                Fault::fatal(format!(
                    "{named}: the row has no column {index}, where the result set described {}",
                    columns.len()
                ))
            })?;
            let scalar = nvs_db::tds::scalar(column, body)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            let value = tds_column_value(scalar, zone, named, &column.name)?;
            one.set(NvsStr::new(column.name.as_bytes()), value);
        }
        rows.append(Value::array(one));
    }
    // After the drain, as the other two: the `DONE` token is what freezes the
    // duration and the count the span carries.
    let taken = watch.taken(answered.span());
    Ok((
        Answered {
            rows,
            columns: described,
        },
        taken,
    ))
}

/// `execute` over the PostgreSQL driver: the same stream [`postgres_rows`]
/// drains, read for its counts rather than its rows.
///
/// **The rows are drained and discarded, not skipped.** § 4 gives `execute` no
/// way to hand a `RETURNING` clause's rows back, and the completion tag that
/// carries the affected count is on the far side of them — so a member that
/// walked away would leave the connection mid-stream and would have no count to
/// answer with either.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused.
fn postgres_write(
    postgres: &mut nvs_db::PgConn,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Written, Option<(String, std::time::Duration)>), Fault> {
    let mut answered = postgres
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    while answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
        .is_some()
    {}

    let written = Written {
        changed: answered.affected(),
        last_id: answered.last_id(),
    };
    // § 11's event is a *statement's*, not a reader's: a write files one on the
    // same terms as `query`, carrying the affected count `finished` froze on
    // the span above.
    let taken = watch.taken(answered.span());
    Ok((written, taken))
}

/// `execute` over the MySQL driver: [`postgres_write`]'s shape, and § 4's two
/// counts read out of the status packet rather than out of a completion tag.
///
/// **`lastId` is where the two drivers differ and § 4 does not.** MySQL answers
/// `0` for a statement that generated no `AUTO_INCREMENT` value, and § 4's field
/// is `?uint` — so the zero is mapped to null here rather than handed to a
/// caller who would have to know to read it as absence. PostgreSQL reaches the
/// same answer by having no `RETURNING` id to read at all.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused.
fn mysql_write(
    mut framed: Framed<'_>,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Written, Option<(String, std::time::Duration)>), Fault> {
    let mut answered = framed
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    // A write answers no result set, but a `CALL` does — and draining is what
    // ends the statement on this driver, as [`postgres_write`]'s does there.
    while answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
        .is_some()
    {}

    let written = Written {
        changed: answered.affected(),
        last_id: answered.last_id().filter(|id| *id != 0),
    };
    let taken = watch.taken(answered.span());
    Ok((written, taken))
}

/// What is reading this statement's span — [ADR 0041](../../../../docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md)
/// § 1's trace, ADR 0067 § 11's `slow_query` line, both or neither — asked
/// **before** a statement borrows the context.
///
/// A statement holds `ctx` mutably for as long as its rows do
/// ([`transacting`]), so neither can be read at the point the event is filed.
/// Reading them early also means a request that turns tracing on midway through
/// a statement does not get half an event — the span is either filed whole or
/// not at all, unlike a call site's pair, which ADR 0018 deliberately lets
/// straddle a change.
///
/// **The two readers are one type because they read one span.** § 11 gives the
/// threshold the same facts the trace event carries, so a statement renders its
/// span at most once however many readers there are — and a second rendering is
/// the only way the two could ever describe one statement differently.
///
/// **`pub(crate)` because § 11 is about statements and not about `Core\Db`.**
/// [`crate::queue`]'s four members drive a `PgRows` themselves rather than
/// through this class's own, and a trace that showed every statement but
/// theirs would be describing a request that never happened.
#[derive(Clone, Copy)]
pub(crate) struct QueryWatch {
    /// ADR 0041 § 1's trace is recording this request.
    traced: bool,
    /// § 11's threshold, for the block that wrote one.
    slow: Option<std::time::Duration>,
}

impl QueryWatch {
    /// What the context and the connection's block say, before the statement
    /// goes out.
    fn of(ctx: &nvs_runtime::Ctx, block: &Value) -> QueryWatch {
        QueryWatch::named(ctx, block.as_text())
    }

    /// The same, for a caller that holds the block's *name* rather than the
    /// `Value` § 2's `Connection` carries it as.
    ///
    /// [`crate::queue`] is that caller: its connection is named by ADR 0084
    /// § 2's `[queue] connection` and reached by key, so there is no `Value` to
    /// read a name out of. `None` is § 2's unnamed `open` and means the same
    /// thing here as there — nothing to look a threshold up under.
    pub(crate) fn named(ctx: &nvs_runtime::Ctx, block: Option<&str>) -> QueryWatch {
        QueryWatch {
            traced: ctx.debug_flags().contains(nvs_runtime::DebugFlags::TRACE),
            slow: block.and_then(|name| slow_query_of(ctx, name)),
        }
    }

    /// The span's facts, taken while the statement still lends them out.
    ///
    /// `None` when nothing is reading, and then the rendering and the clock are
    /// not paid for at all — which is every request on a deployment that has
    /// asked for neither.
    pub(crate) fn taken(self, span: &nvs_db::QuerySpan) -> Option<(String, std::time::Duration)> {
        (self.traced || self.slow.is_some()).then(|| (span.to_string(), span.duration()))
    }

    /// Files what [`QueryWatch::taken`] took, once the statement has let the
    /// context go.
    pub(crate) fn file(
        self,
        ctx: &mut nvs_runtime::Ctx,
        taken: Option<(String, std::time::Duration)>,
    ) {
        let Some((line, took)) = taken else {
            return;
        };
        if self.traced {
            ctx.record_query(&line);
        }
        if self.slow.is_some_and(|threshold| took >= threshold) {
            slow_query_record(ctx, &line);
        }
    }
}

/// ADR 0067 § 11's threshold for the `[db.<name>]` block a statement is running
/// on, or `None` for a statement nothing is timing.
///
/// Three cases answer `None` and they are one answer: the block wrote no
/// threshold, the connection has no block at all (§ 2's `open`, whose settings
/// the program wrote and no operator named — [`QueryWatch::named`] answers that
/// one before this is reached), and a value that would not parse —
/// which `nvs_config::db::validate` refused at boot, so it is unreachable here.
/// Off is the right answer to all three: a threshold nobody can read is not a
/// reason to fail a statement, and § 11's output is inert until asked for.
fn slow_query_of(ctx: &nvs_runtime::Ctx, name: &str) -> Option<std::time::Duration> {
    let snapshot = ctx.config()?.snapshot();
    let written = snapshot.config.db.get(name)?;
    nvs_config::db::slow_query_for(name, written, &std::collections::BTreeMap::new())
        .ok()
        .flatten()
}

/// § 11's slow-query line: the span ADR 0041's trace event carries, written to
/// `Core\Log` as one record.
///
/// **The message is the span's own rendering and not a bag of fields**, which is
/// what "the same facts" costs here: `nvs_db::QuerySpan`'s `Display` is the one
/// home of § 11's field set, and a field-shaped second spelling of it in this
/// module would be the copy that goes stale the day a driver adds one. `Warn`
/// because a threshold is written by an operator asking to be told, and it is
/// the quietest level a log pipeline is not configured to drop.
///
/// A record that cannot be written is dropped rather than retried or thrown —
/// `Core\Log::write`'s own rule, and a statement that already ran is not failed
/// by the line describing it.
fn slow_query_record(ctx: &mut nvs_runtime::Ctx, line: &str) {
    let mut record = nvs_render::Record::at(nvs_render::Level::Warn);
    record.envelope.message = Some(nvs_render::Rendered::new(line));
    let _dropped = ctx.write_log_record(&record, nvs_runtime::LogChannel::Output);
}

/// Files ADR 0067 § 11's event for a statement that never lent a `PgRows` out —
/// § 7's three commands, and the batch `executeMany` is.
///
/// The block goes on here rather than through [`name_span`], which takes the
/// rows those statements never have; the rule it applies is the same one and
/// `nvs_db::QuerySpan::name` owns it. The span arrives finished — the driver
/// froze it when its command came back, or the caller did with its own count —
/// so this is only the naming, the taking and the filing, in the order
/// [`QueryWatch`] requires.
fn file_span(
    ctx: &mut nvs_runtime::Ctx,
    watch: QueryWatch,
    block: &Value,
    mut span: nvs_db::QuerySpan,
) {
    if let Some(name) = block.as_text() {
        span.name(name);
    }
    let taken = watch.taken(&span);
    watch.file(ctx, taken);
}

/// Puts the `[db.<name>]` block on a running statement's span.
///
/// `nvs_db::QuerySpan::name` owns why the driver cannot do this itself. The
/// block is the `Statement`'s own, so a `connect`'d connection names itself and
/// ADR 0067 § 2's unnamed `open` — which has no block at all — leaves the field
/// empty rather than carrying a made-up name.
///
/// It takes the name and not the `Value`, so [`crate::queue`]'s statements —
/// whose block is `[queue] connection`'s name — put it on their spans through
/// this one rule rather than a second spelling of it.
pub(crate) fn name_span<R: NamesConnection>(rows: &mut R, block: Option<&str>) {
    if let Some(name) = block {
        rows.name_connection(name);
    }
}

/// A running statement's handle, on whichever driver — joined by the one method
/// [`name_span`] needs of it.
///
/// **One method and not a result-set trait**, which [`mysql_rows`] argues at
/// length for the walk above: the two handles agree on nothing else, and what is
/// shared here is the *rule* that a block names its own span, not the reading of
/// a row. Written as a trait rather than as two calls so a third driver's
/// statement cannot quietly file a nameless span.
pub(crate) trait NamesConnection {
    /// Puts `connection` on this statement's span.
    fn name_connection(&mut self, connection: &str);
}

impl NamesConnection for nvs_db::PgRows<'_> {
    fn name_connection(&mut self, connection: &str) {
        nvs_db::PgRows::name_connection(self, connection);
    }
}

impl NamesConnection for nvs_db::MySqlRows<'_> {
    fn name_connection(&mut self, connection: &str) {
        nvs_db::MySqlRows::name_connection(self, connection);
    }
}

impl NamesConnection for nvs_db::tds::TdsRows<'_> {
    fn name_connection(&mut self, connection: &str) {
        nvs_db::tds::TdsRows::name_connection(self, connection);
    }
}

/// The row description as spec § 18's `array<Column>`: one [`COLUMN`] per
/// column, in the server's own order and never keyed by label — `select a, a`
/// describes two columns under one name, and a keyed array would answer one.
///
/// **One object per column, built whether or not the program asks.** What that
/// spends is bounded by the statement's `select` list rather than by its
/// result, so it is a few objects beside the one-array-per-row the decode above
/// already allocates; the alternative — keeping the labels and the OIDs in a
/// pair of arrays and building the objects in `columns()` — buys nothing back
/// on the path that never calls it and costs a second representation of the
/// same fact on the path that does.
fn described_columns(columns: &[nvs_db::PgColumn]) -> NvsArray {
    let mut described = NvsArray::new();
    for column in columns {
        described.append(crate::instance::build(
            &COLUMN,
            [
                Value::str(NvsStr::new(column.name.as_bytes())),
                column_type_value(column.column_type()),
                // ADR 0067 § 9's own answer, stated at the one place that can
                // state it: [`COLUMN_NULLABLE_DOC`] is where a program's author
                // reads what the `true` means.
                Value::bool(true),
            ],
        ));
    }
    described
}

/// The same description for a MySQL result set: [`described_columns`]'s twin,
/// building the same [`COLUMN`] objects out of the other driver's metadata.
///
/// **A twin and not one function over both**, because the two descriptions have
/// no type in common. A [`nvs_db::PgColumn`] is `nvs-db`'s own row description,
/// carrying its label as a `String` and its § 9 row as a field; a MySQL column
/// definition is `mysql_common`'s `Column`, whose label is octets in the packet
/// and whose § 9 row is read off its type and its `UNSIGNED` flag together. So
/// the two loops share their shape and not one line of their bodies, and a
/// trait over the pair would be a third name for two fields.
///
/// **The type is asked of the result set rather than read off the definition**,
/// because which § 9 row a definition names is `nvs-db`'s reading and not this
/// module's — the same division [`nvs_db::mysql::scalar`] draws for a value.
/// The label is taken as octets for [`described_columns`]'s reason: a column
/// name is a key in the row array, and a lossy decode would rename a column
/// rather than refuse it.
fn mysql_described_columns(rows: &nvs_db::MySqlRows<'_>) -> NvsArray {
    let mut described = NvsArray::new();
    for (index, column) in rows.columns().iter().enumerate() {
        let column_type = rows
            .column_type(index)
            .expect("a column this loop is walking is one the result set described");
        described.append(crate::instance::build(
            &COLUMN,
            [
                Value::str(NvsStr::new(column.name_ref())),
                column_type_value(column_type),
                // As [`described_columns`]: § 9's own answer, and
                // [`COLUMN_NULLABLE_DOC`] is where it is written down.
                Value::bool(true),
            ],
        ));
    }
    described
}

/// The same description for a SQL Server result set: [`described_columns`]'
/// third twin, over `COLMETADATA`.
///
/// A twin for [`mysql_described_columns`]' reason, and one decision shorter
/// than it: a `nvs_db::tds::TdsColumn` carries its label as a `String` the
/// driver already decoded out of UCS-2, so there is no packet-octets-versus-
/// text question to answer here at all.
fn tds_described_columns(rows: &nvs_db::tds::TdsRows<'_>) -> NvsArray {
    let mut described = NvsArray::new();
    for (index, column) in rows.columns().iter().enumerate() {
        let column_type = rows
            .column_type(index)
            .expect("a column this loop is walking is one the result set described");
        described.append(crate::instance::build(
            &COLUMN,
            [
                Value::str(NvsStr::new(column.name.as_bytes())),
                column_type_value(column_type),
                // As [`described_columns`]: § 9's own answer, and
                // [`COLUMN_NULLABLE_DOC`] is where it is written down. The
                // server's own `Flags` bit is beside it on this driver —
                // `nvs_db::tds::TdsColumn::nullable` — and says something
                // narrower, which that method's doc owns.
                Value::bool(true),
            ],
        ));
    }
    described
}

/// A [`nvs_db::ColumnType`] as the [`COLUMN_TYPE`] case a program matches on,
/// which at runtime is that case's ordinal
/// ([ADR 0010](../../../../docs/adr/0010-enums-are-a-value-type.md)).
///
/// **The ordinal is looked up rather than written a second time.** The two
/// halves of the enum are one enum and [`COLUMN_TYPE`]'s doc says which half is
/// authoritative; a `match` answering numbers here would be a third place the
/// fourteen cases are written down, and the one that goes wrong silently. What
/// is written here is the *name* correspondence, which is the only thing this
/// crate knows that neither half does — and `every_column_type_case_is_named`
/// holds it total in both directions.
fn column_type_value(of: nvs_db::ColumnType) -> Value {
    let case = column_type_case(of);
    let (_, ordinal) = COLUMN_TYPE
        .cases
        .iter()
        .find(|(name, _)| *name == case)
        .expect("every `nvs_db::ColumnType` names a case `COLUMN_TYPE` registers");
    Value::int(*ordinal)
}

/// The [`COLUMN_TYPE`] case one [`nvs_db::ColumnType`] is, by name.
///
/// Exhaustive on purpose — a variant added over there arrives here as a
/// non-exhaustive `match` rather than as a column that describes wrongly.
fn column_type_case(of: nvs_db::ColumnType) -> &'static str {
    match of {
        nvs_db::ColumnType::Int => "Int",
        nvs_db::ColumnType::Uint => "Uint",
        nvs_db::ColumnType::Float => "Float",
        nvs_db::ColumnType::Decimal => "Decimal",
        nvs_db::ColumnType::Text => "Text",
        nvs_db::ColumnType::Bytes => "Bytes",
        nvs_db::ColumnType::Bool => "Bool",
        nvs_db::ColumnType::Date => "Date",
        nvs_db::ColumnType::Time => "Time",
        nvs_db::ColumnType::DateTime => "DateTime",
        nvs_db::ColumnType::Instant => "Instant",
        nvs_db::ColumnType::Uuid => "Uuid",
        nvs_db::ColumnType::Json => "Json",
        nvs_db::ColumnType::Other => "Other",
    }
}

/// One column's Novis value: ADR 0067 § 9's whole type map, with the five rows
/// whose Novis type is a class instance built here.
///
/// This is the second half of one decode and not a second decode. `nvs-db`
/// reads every column to a [`nvs_db::PgScalar`] and mints a [`Value`] for the
/// rows that are values; the five that are class instances arrive as the
/// components the server rendered, because a `Core\Time\Date` is an instance
/// of a class *this* crate declares and that one cannot allocate — see
/// [`nvs_db::PgDate`]. So nothing here parses a body, and the only thing left
/// that can go wrong is a rendered value no `Core\Time` type has.
///
/// `zone` is the connection's declared zone, § 9's answer for the one row that
/// carries no offset of its own.
///
/// An array is this function again per element, so an `array<Core\Uuid>` and
/// an `array<array<Core\Time\Date>>` need nothing of their own. Each is built
/// into an [`NvsArray`] that a later element's refusal drops — releasing what
/// it already holds — which is the rule the row itself is built under.
///
/// # Errors
///
/// [`unrepresentable_column`] for a value with no Novis representation, and a
/// [`Fault::fatal`] for a row `nvs-db` answers no value for and this function
/// does not build, which is a variant added there with no arm here.
fn column_value(
    scalar: nvs_db::PgScalar<'_>,
    zone: i32,
    named: &str,
    column: &str,
) -> Result<Value, Fault> {
    let refused = |row| unrepresentable_column(named, column, row);
    Ok(match scalar {
        nvs_db::PgScalar::Date(date) => {
            crate::time::date_at(date.year, date.month, date.day).ok_or_else(|| refused("date"))?
        }
        nvs_db::PgScalar::Time(time) => {
            crate::time::time_of_day_at(time.hour, time.minute, time.second, time.nanosecond)
                .ok_or_else(|| refused("time of day"))?
        }
        nvs_db::PgScalar::Timestamp { date, time } => {
            crate::time::datetime_at(&civil_of(date, time), zone)
                .ok_or_else(|| refused("date and time"))?
        }
        nvs_db::PgScalar::Instant { date, time, offset } => {
            crate::time::instant_at(&civil_of(date, time), offset)
                .ok_or_else(|| refused("date and time"))?
        }
        nvs_db::PgScalar::Uuid(octets) => crate::uuid::of_octets(octets),
        nvs_db::PgScalar::Array(items) => {
            let mut array = NvsArray::new();
            for item in items {
                array.append(column_value(item, zone, named, column)?);
            }
            Value::array(array)
        }
        row => row.into_value().ok_or_else(|| {
            Fault::fatal(format!(
                "{named}: `nvs-db` answered no value for the column `{column}`, and this decoder \
                 builds no instance for it either"
            ))
        })?,
    })
}

/// The civil fields a `TIMESTAMP` or a `TIMESTAMPTZ` was rendered with, in the
/// shape [`crate::time`]'s two seams read.
fn civil_of(date: nvs_db::PgDate, time: nvs_db::PgTime) -> crate::time::Civil {
    crate::time::Civil {
        year: date.year,
        month: date.month,
        day: date.day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        nanosecond: time.nanosecond,
    }
}

/// One MySQL column's Novis value: [`column_value`]'s twin over the other
/// driver's scalar.
///
/// The division is the same one, for the same reason: `nvs-db` reads a column
/// to a [`nvs_db::MySqlScalar`] and mints a [`Value`] for the rows that are
/// values, and § 9's structured rows arrive as the components the server sent,
/// because the classes they become are declared in *this* crate and that one
/// cannot allocate an instance — [`nvs_db::MySqlDate`] owns that half.
///
/// **Three rows and not five**, which is why this is a shorter `match` rather
/// than a copy of [`column_value`]'s. MySQL has no `UUID` column type — § 9
/// sends its `BINARY(16)` to the `bytes` row and MariaDB, which does have the
/// type, is its own driver — and no array type either, so the recursion a
/// PostgreSQL `array<T>` needs has nothing here to recur over. What is left is
/// `DATE`, `TIME` and the zone-less `DATETIME`/`TIMESTAMP` pair.
///
/// `zone` is the connection's declared zone: § 9's answer for the row that
/// carries no offset of its own, and the offset the connection told the server
/// at connect so that `CURRENT_TIMESTAMP` agrees with what is read back here.
///
/// # Errors
///
/// [`unrepresentable_column`] for a value no `Core\Time` type has — on this
/// driver the zero date, which its own decoder deliberately does not check —
/// and a [`Fault::fatal`] for a row `nvs-db` answers no value for and this
/// function does not build, which is a variant added there with no arm here.
fn mysql_column_value(
    scalar: nvs_db::MySqlScalar<'_>,
    zone: i32,
    named: &str,
    column: &str,
) -> Result<Value, Fault> {
    let refused = |row| unrepresentable_column(named, column, row);
    Ok(match scalar {
        nvs_db::MySqlScalar::Date(date) => {
            crate::time::date_at(date.year, date.month, date.day).ok_or_else(|| refused("date"))?
        }
        nvs_db::MySqlScalar::Time(time) => {
            crate::time::time_of_day_at(time.hour, time.minute, time.second, time.nanosecond)
                .ok_or_else(|| refused("time of day"))?
        }
        nvs_db::MySqlScalar::DateTime { date, time } => {
            crate::time::datetime_at(&mysql_civil_of(date, time), zone)
                .ok_or_else(|| refused("date and time"))?
        }
        row => row.into_value().ok_or_else(|| {
            Fault::fatal(format!(
                "{named}: `nvs-db` answered no value for the column `{column}`, and this decoder \
                 builds no instance for it either"
            ))
        })?,
    })
}

/// The civil fields a `DATETIME` or a `TIMESTAMP` was sent with, in the shape
/// [`crate::time`]'s seams read — [`civil_of`] for the other driver.
///
/// Two functions over two field-identical structs, because the structs belong
/// to two protocols: PostgreSQL's components are parsed out of a rendering and
/// MySQL's arrive as integers, and one type standing for both would say they
/// are the same fact when only their shape is the same.
fn mysql_civil_of(date: nvs_db::MySqlDate, time: nvs_db::MySqlTime) -> crate::time::Civil {
    crate::time::Civil {
        year: date.year,
        month: date.month,
        day: date.day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        nanosecond: time.nanosecond,
    }
}

/// One SQL Server column's Novis value: [`column_value`]'s twin over the third
/// driver's scalar.
///
/// The division is the same one for the same reason, and **all five of § 9's
/// structured rows are here** where MySQL reaches three: this driver has
/// `date`, `time`, the zone-less `datetime`/`datetime2` pair, `datetimeoffset`
/// and a `uniqueidentifier` type of its own. What it has no row for is the
/// array, which is PostgreSQL's alone, so nothing here recurs.
///
/// **Nothing is computed here, and three of these would be wrong if it were.**
/// `nvs_db::tds::scalar`'s doc names the rows whose octets do not mean what
/// they look like — `money`'s leading high word, `uniqueidentifier`'s
/// endianness, and `datetimeoffset`'s civil fields being stored in UTC — and
/// all three arrive already in their answered form. An `Instant` off this
/// driver therefore carries local fields beside its offset exactly as
/// PostgreSQL's does, which is what lets [`crate::time`]'s two seams serve both
/// without a driver argument.
///
/// `zone` is the connection's declared zone, § 9's answer for the row that
/// carries no offset of its own — declared and never negotiated on this driver,
/// unlike the other two, because there is no server-side session variable to
/// send it to.
///
/// # Errors
///
/// [`unrepresentable_column`] for a value no `Core\Time` type has, and a
/// [`Fault::fatal`] for a row `nvs-db` answers no value for and this function
/// does not build, which is a variant added there with no arm here.
fn tds_column_value(
    scalar: nvs_db::tds::TdsScalar<'_>,
    zone: i32,
    named: &str,
    column: &str,
) -> Result<Value, Fault> {
    let refused = |row| unrepresentable_column(named, column, row);
    Ok(match scalar {
        nvs_db::tds::TdsScalar::Date(date) => {
            crate::time::date_at(date.year, date.month, date.day).ok_or_else(|| refused("date"))?
        }
        nvs_db::tds::TdsScalar::Time(time) => {
            crate::time::time_of_day_at(time.hour, time.minute, time.second, time.nanosecond)
                .ok_or_else(|| refused("time of day"))?
        }
        nvs_db::tds::TdsScalar::DateTime { date, time } => {
            crate::time::datetime_at(&tds_civil_of(date, time), zone)
                .ok_or_else(|| refused("date and time"))?
        }
        nvs_db::tds::TdsScalar::Instant { date, time, offset } => {
            crate::time::instant_at(&tds_civil_of(date, time), offset)
                .ok_or_else(|| refused("date and time"))?
        }
        nvs_db::tds::TdsScalar::Uuid(octets) => crate::uuid::of_octets(octets),
        row => row.into_value().ok_or_else(|| {
            Fault::fatal(format!(
                "{named}: `nvs-db` answered no value for the column `{column}`, and this decoder \
                 builds no instance for it either"
            ))
        })?,
    })
}

/// The civil fields a `datetime`, `datetime2` or `datetimeoffset` carried, in
/// the shape [`crate::time`]'s seams read.
///
/// [`civil_of`]'s and [`mysql_civil_of`]'s third, separate from both for the
/// reason those two are separate from each other: the fields are the same three
/// plus four and the *facts* are three protocols', one parsed out of a text
/// rendering and two read off the wire in different units.
fn tds_civil_of(date: nvs_db::tds::TdsDate, time: nvs_db::tds::TdsTime) -> crate::time::Civil {
    crate::time::Civil {
        year: date.year,
        month: date.month,
        day: date.day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        nanosecond: time.nanosecond,
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Connection::queryAs<T>(string $sql, array<mixed> $params):
    /// Db\Rows<T>` — ADR 0067 § 4's statement over § 18's hydrating result.
    ///
    /// **Arguments 0 and 1 are the class written at the call site and whether
    /// it was written as `array<...>` of one**, not values, and the receiver is
    /// argument 2: `crate::registry::WRITTEN_CLASS_MEMBERS` owns that ABI and
    /// this is its first *instance* member. So the arity here is two more than
    /// `query`'s, which is otherwise the same call.
    ///
    /// **The statement is [`queried_rows`], unchanged**: § 4 gives `query` and
    /// this member one signature and one binding rule, so what differs is the
    /// class the result carries and nothing on the wire. That class goes into
    /// [`ROWS_CLASS_SLOT`] and is read only when a row is handed out, so a
    /// caller that just counts pays for no construction.
    ///
    /// **The construction is [`hydrate`]'s**, reached through [`row_object`]
    /// when `all`, `first` or a `foreach` asks for a row — so this body's own
    /// refusals are the two that are about the *call site* rather than about a
    /// row, and they are raised before the statement goes out.
    fn nvs_core_db_connection_query_as(ctx, args: [5]) {
        // Unreachable from source, exactly as `Core\Json::decodeAs`'s own
        // reading of these two slots is: `nvs_ir::lower` writes the descriptor
        // and the flag out of the type argument at the call site, and a call
        // naming none is `E0442` before any of this runs.
        if args[0].as_class_desc().is_none() {
            return Err(Fault::fatal(format!(
                "internal error: `{QUERY_AS}` was called with no class in argument 0"
            )));
        }
        // Unreachable from source for the same reason and refused by the same
        // `E0442`: slot 1 is the `ConstBool` the lowering emits beside the
        // descriptor, so a call that has one has the other.
        let list = args[1].as_bool().ok_or_else(|| Fault::fatal(format!(
            "internal error: `{QUERY_AS}` was called with no list flag in argument 1"
        )))?;
        // Refused before the statement goes out, because it cannot mean
        // anything downstream: `Core\Json::decodeAs`'s list form is a document
        // that *is* a JSON array, and a result set is already one row per row.
        // A compile-time home would be better and gap 8 says why there is none.
        if list {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{QUERY_AS}: `array<...>` is not a type argument this member takes — a result \
                     set is already one `{ROWS_NAME}` of one row each, so write \
                     `queryAs<Person>(…)` and read the list off the result"
                ),
            ));
        }
        // The receiver and the two value parameters, past the pair
        // `WRITTEN_CLASS_MEMBERS` puts ahead of everything.
        let answered = queried_rows(ctx, &args[2..], "queryAs", QUERY_AS)?;
        // `args[0]` carries no reference — a descriptor rides in the payload
        // half of an otherwise-`null` value — so the slot takes it as it is.
        Ok(crate::instance::build(
            &ROWS,
            [
                Value::array(answered.rows),
                args[0],
                Value::array(answered.columns),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Connection::execute(string $sql, array<mixed> $params):
    /// Db\Write` — ADR 0067 § 4's counting half of the same statement path.
    ///
    /// **The difference from [`nvs_core_db_connection_query`] is what becomes
    /// of the rows, and nothing else.** The statement goes out the same way,
    /// through the same [`statement_of`], because § 4 gives the two members one
    /// signature and one binding rule — so `execute` is not a second, weaker
    /// path a caller could reach a difference through.
    ///
    /// **The rows are read to the end and dropped**, which is not a waste: a
    /// PostgreSQL statement is a stream either way, ending it is what returns
    /// the connection to idle, and `lastId` is taken as each row goes past
    /// ([`nvs_db::PgRows::last_id`]) — so the drain is also what finds it.
    /// Nothing is decoded, so an `insert … returning` costs no column work at
    /// all here, which is the one thing this member does differently with the
    /// same stream `query` reads.
    ///
    /// **Both counts come off `CommandComplete`**, so neither exists until that
    /// stream has ended, and the pair is § 4's own: `affected` folds a command
    /// whose tag carries no count at all — a `create table` — to `0`, and
    /// `changed` keeps the absence, which is the only thing the two say
    /// differently on this driver.
    fn nvs_core_db_connection_execute(ctx, args: [3]) {
        let statement = statement_of(ctx, args, "execute", EXECUTE)?;
        // As `query`, and for the reason given there: a refusal names the
        // caller's own text rather than the rewrite of it that reached the wire.
        let source = args[1].as_text();
        let sending: Vec<Option<&[u8]>> =
            statement.binds.iter().map(|one| one.as_deref()).collect();
        // As `query`, and for the reason [`QueryWatch`] gives.
        let watch = QueryWatch::of(ctx, &statement.block);
        // Branched as [`queried_rows`] is, and the arms hand the event back for
        // the same borrow reason: the rows hold the connection, which holds the
        // context the span is filed on.
        let (written, taken) = match filed_connection(ctx, statement.key, EXECUTE)? {
            nvs_db::Connection::Postgres(postgres) => {
                postgres_write(postgres, &statement, &sending, source, watch, EXECUTE)?
            }
            nvs_db::Connection::MySql(mysql) => {
                mysql_write(Framed::MySql(mysql), &statement, &sending, source, watch, EXECUTE)?
            }
            nvs_db::Connection::MariaDb(maria) => mysql_write(
                Framed::MariaDb(maria),
                &statement,
                &sending,
                source,
                watch,
                EXECUTE,
            )?,
            other => {
                return Err(driverless(
                    EXECUTE,
                    &statement.block,
                    other.driver(),
                    BEYOND_READING,
                ));
            }
        };
        watch.file(ctx, taken);
        Ok(crate::instance::build(
            &WRITE,
            [
                Value::uint(written.changed.unwrap_or(0)),
                written.changed.map_or_else(Value::null, Value::uint),
                written.last_id.map_or_else(Value::null, Value::uint),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Connection::executeMany(string $sql, array<array<mixed>> $sets):
    /// uint` — ADR 0067 § 4's batch, and § 1's reason for having no `Statement`
    /// object at all.
    ///
    /// **This is the member a `prepare` handle would have existed for.** § 1
    /// removes the handle because the per-connection cache already buys what it
    /// bought, and names the batch as the one case that would otherwise still
    /// want one — so a loop of `execute` calls and this member differ in round
    /// trips and in nothing else a caller can see.
    ///
    /// **Every set is bound by [`statement_of`], and they must agree**; that
    /// rule and why it is checked on the rewritten text rather than on a count
    /// are [`batch_of`]'s.
    ///
    /// **The batch opens and files ADR 0067 § 11's span itself**, which is the
    /// one thing it does that `execute` leaves to the driver.
    /// Either driver's `execute_many` answers with a count and lends no row
    /// handle out, so there is no handle a driver-built span could ride on and
    /// be read off afterwards — the span is opened here, around the same round
    /// trips, and finished with the batch's sum as its affected count and no
    /// rows at all, which is what a batch contributes to a trace.
    /// `nvs_db::QuerySpan` owns the field set and why a bound value is not in
    /// it, and `Ctx::record_query` owns why what crosses is a rendering.
    ///
    /// **What it answers is a `uint` and not a [`WRITE`].** § 4 gives the batch
    /// a sum, because `changed` and `lastId` would each have to pick one
    /// execution to be about — and the sum is what a caller writing the loop by
    /// hand would have accumulated anyway. The batch is also **not** a
    /// transaction: a failure part way through leaves the writes before it
    /// standing and the sets after it still attempted, and `transaction` is the
    /// member that asks for all or nothing. The two drivers reach that one
    /// observable from opposite ends of their protocols — a `Sync` per execution
    /// on PostgreSQL, a command per execution on MySQL — and each
    /// `execute_many`'s own doc argues its half.
    fn nvs_core_db_connection_execute_many(ctx, args: [3]) {
        let batch = batch_of(ctx, args, "executeMany", EXECUTE_MANY)?;
        // Two hops rather than one: the driver borrows each set as a slice, so
        // the per-set `Vec` has to outlive the slice taken of it.
        let sending: Vec<Vec<Option<&[u8]>>> = batch
            .binds
            .iter()
            .map(|set| set.iter().map(|one| one.as_deref()).collect())
            .collect();
        let sets: Vec<&[Option<&[u8]>]> = sending.iter().map(Vec::as_slice).collect();

        // As `execute`, and for the reason [`QueryWatch`] gives.
        let watch = QueryWatch::of(ctx, &batch.block);
        let connection = filed_connection(ctx, batch.key, EXECUTE_MANY)?;
        let driver = connection.driver();
        // § 11's span, opened where the driver opens `execute`'s: after the
        // connection is in hand, so the duration is the statement's wait and
        // not the pool's. It carries the rewritten text, which is what reaches
        // the wire and what a driver-opened span would have been handed, and the
        // connection's own driver, so a trace reads the batch beside the
        // statements around it rather than as PostgreSQL's whatever ran it.
        let mut span = nvs_db::QuerySpan::opened(driver, &batch.sql);
        // One statement over many parameter sets, so the batch has exactly the
        // one text to name and it is the caller's, as `execute`'s is. What the
        // two drivers do with the sets differs and what a caller observes does
        // not — `nvs_db::mysql`'s own `execute_many` is where that is argued.
        let written = match connection {
            nvs_db::Connection::Postgres(postgres) => postgres.execute_many(&batch.sql, &sets),
            nvs_db::Connection::MySql(mysql) => mysql.execute_many(&batch.sql, &sets),
            // Not through [`Framed`]: that seam exists to stop a *body* being
            // written twice, and this arm is the whole body. § 4 runs N
            // executions on both drivers and `nvs_db::mysql::execute_many` is
            // the one that runs them.
            nvs_db::Connection::MariaDb(maria) => maria.execute_many(&batch.sql, &sets),
            other => {
                return Err(driverless(
                    EXECUTE_MANY,
                    &batch.block,
                    other.driver(),
                    BEYOND_READING,
                ));
            }
        }
        .map_err(|refused| {
            statement_failure(EXECUTE_MANY, &batch.block, args[1].as_text(), &refused)
        })?;
        // § 4's sum is the batch's affected count, and the span's rows stay at
        // zero: nothing was handed back, and a batch that inserted a thousand
        // rows reporting a thousand rows *returned* would read as a select.
        span.finished(Some(written));
        file_span(ctx, watch, &batch.block, span);
        Ok(Value::uint(written))
    }
}

/// The [`ISOLATION`] case an `{isolation: …}` option arrived as, or `None` for
/// the call that left it out.
///
/// **The two rosters are matched by ordinal and written out**, not cast:
/// [`ISOLATION`] states § 7's order for the program and [`nvs_db::Isolation`]
/// states it again for the driver, in different crates for different readers,
/// and a cast between them would answer the wrong level the first time either
/// gained a case. `Core\Cli`'s `stream_of` is the same judgement.
///
/// `None` is the option's declared default and not a failure: § 7's absent
/// `isolation` runs at the level the connection already has, which
/// [`nvs_db::PgConn::begin`] renders by leaving the clause off the `BEGIN`.
fn isolation_of(value: &Value, member: &str) -> Result<Option<nvs_db::Isolation>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    match value.as_int() {
        Some(0) => Ok(Some(nvs_db::Isolation::ReadUncommitted)),
        Some(1) => Ok(Some(nvs_db::Isolation::ReadCommitted)),
        Some(2) => Ok(Some(nvs_db::Isolation::RepeatableRead)),
        Some(3) => Ok(Some(nvs_db::Isolation::Snapshot)),
        Some(4) => Ok(Some(nvs_db::Isolation::Serializable)),
        // Unreachable from source: the option is `CoreTy::Enum(ISOLATION_NAME)`
        // in [`TRANSACTION_OPTIONS`], so anything that is not one of the five
        // cases is `E0401` at the checker and compiled code writes the ordinal
        // itself. `Core\Arr::sort`'s `order` states the same reasoning in full.
        _ => Err(Fault::fatal(format!(
            "{member} expected a `{ISOLATION_NAME}` case for `isolation`, got tag {} value {:?}",
            value.tag_byte(),
            value.as_int()
        ))),
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Queryable::transaction(callable $fn, {isolation?, readOnly?,
    /// retries?}): T` — ADR 0067 § 7's whole shape, and the only way to open a
    /// transaction on this surface.
    ///
    /// **The closure form is what removes the failure mode**, which § 7 argues
    /// and this body implements: there is no point between the `BEGIN` and the
    /// `COMMIT` at which a program can walk away, because the scope is a call
    /// and an early `return` inside it is still a return *through* here. With
    /// no destructors there is nothing an object-scoped transaction could hook
    /// its rollback to, so the closure is not the tidier of two options — it is
    /// the one that can be made to hold.
    ///
    /// **Committing is what returning does, and there is no member for it.**
    /// The three outcomes are decided here rather than by the closure: it
    /// returned and nothing asked for a rollback, so the work commits and its
    /// answer is this call's; it threw, so the work rolls back and its
    /// exception travels on unchanged; or it recorded a reason through
    /// [`nvs_core_db_transaction_roll_back`], so the work rolls back and
    /// `Core\Db\RolledBack` is raised here.
    ///
    /// **The third case is read off the transaction and not off the throw**,
    /// which is the point of § 7's flag: an intervening `catch (Throwable)`
    /// swallows the signal, the closure returns normally, and this frame still
    /// refuses to commit. That is the guarantee Doctrine's `setRollbackOnly`
    /// asks every layer to cooperate on.
    ///
    /// **A nested call is a savepoint**, and nothing here says so: the depth
    /// lives on the connection and [`nvs_db::PgConn::begin`] picks the command
    /// from it, so a library that wraps its own writes composes with a caller's
    /// transaction without either of them knowing.
    ///
    /// **`isolation` and `readOnly` are handed straight to that same call**,
    /// which is where both the rendering and the one refusal live: a nested
    /// call carrying either is an `InvalidInput` there and so a `LogicError`
    /// here, because PostgreSQL settles both for the whole transaction and
    /// running the closure at the outer one's level would be quietly weaker
    /// than what its author wrote. Reading the two here and deciding nothing
    /// with them is deliberate — the level a given backend can offer is the
    /// driver's question, and [`TRANSACTION_OPTIONS`] owns what their defaults
    /// mean.
    ///
    /// **`{retries: n}` goes around the whole block and not inside it.** Each
    /// attempt gets its own `BEGIN` and its own scope object, because the one
    /// above is closed and discarded on every path already — a re-run that
    /// reused either would be handing the closure a `$tx` that is refusing.
    /// **Either conflict re-runs it**: the commit's own refusal, and one a
    /// statement inside the closure raised, which arrives as a pending
    /// `Core\Db\DbError` instead and is read through
    /// [`nvs_runtime::Ctx::pending_slot`]. [`wait_between_attempts`] is the
    /// wait § 7 puts between two of them. The loop itself is
    /// [`transacted`], which is handed its connection rather than reading one
    /// back out of the context — see [`Attempts`] for why that seam is where
    /// it is.
    ///
    /// **Rolling back after a throw discards its own failure.** The exception
    /// the closure raised is what the request is about, and a connection whose
    /// `ROLLBACK` was refused is one § 13's reset destroys rather than pools —
    /// so replacing the program's exception with the driver's would lose the
    /// only half a caller can act on.
    fn nvs_core_db_connection_transaction(ctx, args: [5]) {
        let (key, block) = handle_of(args[0], "transaction")?;
        let isolation = isolation_of(&args[ISOLATION_ARG], TRANSACTION_MEMBER)?;
        // Unreachable from source for `connect`'s reason: the option is
        // declared `bool` and defaults to one, so the slot is never anything
        // else.
        let read_only = args[READ_ONLY_ARG].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "{TRANSACTION_MEMBER} expected a `bool` for `readOnly`, got tag {}",
                args[READ_ONLY_ARG].tag_byte()
            ))
        })?;
        // As `readOnly`: the option is declared `uint` and defaults to 0.
        let retries = args[RETRIES_ARG].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{TRANSACTION_MEMBER} expected a `uint` for `retries`, got tag {}",
                args[RETRIES_ARG].tag_byte()
            ))
        })?;

        transacted(
            ctx,
            &mut Filed { key, block },
            &Attempted {
                key,
                block,
                closure: args[1],
                isolation,
                read_only,
                retries,
            },
        )
    }
}

/// § 7's four questions to the connection an attempt runs on, as something
/// [`transacted`] is *handed* rather than reads back out of the context.
///
/// **The retry loop is the half of § 7 no driver implements.** A `BEGIN`, a
/// `SAVEPOINT` and a `ROLLBACK` are `nvs-db`'s and are asserted there; nothing
/// in a driver is ever asked to *re-run* anything, so the loop is the one § 7
/// behaviour no `-p nvs-db` case can reach. This trait is what makes it
/// reachable from a `-p nvs-stdlib` one instead: [`Filed`] is the only
/// implementation the runtime ever builds, and a test scripts the
/// conflict-then-commit pair a real deadlock produces without a server in front
/// of it — which it could not otherwise do, [`Transacting`]'s two arms both
/// being connections no test can construct.
///
/// **Every method is handed the context rather than borrowing out of it.** The
/// loop calls Novis code between the `BEGIN` and the `COMMIT`, and that needs
/// the same `&mut Ctx` the connection is filed in, so a borrow held across the
/// closure cannot exist. That is why this is four questions asked one at a
/// time rather than one that answers a [`Transacting`], and it is the same
/// reason the sites below re-read the connection at each of them.
///
/// **Two error channels, which is what the nested `Result` is.** The outer
/// failure is *this request has no such connection* — already a [`Fault`],
/// from [`transacting`] — and the inner one is the server's own refusal, which
/// § 7's retry rule reads a [`nvs_db::DbErrorKind`] off through
/// [`nvs_db::ServerError`] and which only [`std::io::Error`] still carries.
/// Collapsing the two would leave the loop deciding a retry off a rendered
/// message.
trait Attempts {
    /// How many of § 7's levels are open, before this attempt's `BEGIN`.
    ///
    /// # Errors
    ///
    /// As [`transacting`].
    fn depth(&mut self, ctx: &mut nvs_runtime::Ctx) -> Result<u32, Fault>;

    /// § 7's outermost `BEGIN`, or the `SAVEPOINT` a nested call opens.
    ///
    /// # Errors
    ///
    /// As [`transacting`]; the driver's own refusal is the inner `Err`.
    fn begin(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault>;

    /// § 7's `COMMIT`, or the release that closes a nested level.
    ///
    /// # Errors
    ///
    /// As [`Self::begin`].
    fn commit(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault>;

    /// § 7's `ROLLBACK`, or the undo of a nested level.
    ///
    /// # Errors
    ///
    /// As [`Self::begin`].
    fn roll_back(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault>;
}

/// [`Attempts`] over the connection this request has filed under `key` — the
/// only implementation outside this module's own tests.
///
/// Both fields are the caller's, borrowed for the length of one [`transacted`]
/// call: the [`Value`] carries no reference of its own, exactly as
/// [`Attempted`]'s two do.
struct Filed {
    /// § 2's key, as [`handle_of`] read it off the receiver.
    key: u64,
    /// The `[db.<name>]` block, for [`transacting`]'s refusal to name.
    block: Value,
}

impl Attempts for Filed {
    fn depth(&mut self, ctx: &mut nvs_runtime::Ctx) -> Result<u32, Fault> {
        Ok(transacting(ctx, self.key, &self.block, TRANSACTION_MEMBER)?.depth())
    }

    fn begin(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
        Ok(
            transacting(ctx, self.key, &self.block, TRANSACTION_MEMBER)?
                .begin(isolation, read_only),
        )
    }

    fn commit(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
        Ok(transacting(ctx, self.key, &self.block, TRANSACTION_MEMBER)?.commit())
    }

    fn roll_back(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
        Ok(transacting(ctx, self.key, &self.block, TRANSACTION_MEMBER)?.roll_back())
    }
}

/// § 7's call, as [`nvs_core_db_connection_transaction`] read it off its
/// arguments.
///
/// One struct rather than five more parameters on [`transacted`], which takes
/// the connection separately: splitting those two halves is what the hoist is
/// for, and a seven-argument function would be the shape it was written to
/// avoid. Neither [`Value`] carries a reference of its own — both are the
/// helper's own arguments, live for the whole call.
struct Attempted {
    /// § 2's key, as the scope object carries it on to a delegated member.
    key: u64,
    /// The `[db.<name>]` block this transaction refuses under.
    block: Value,
    /// § 7's `$fn`, run once per attempt.
    closure: Value,
    /// § 7's `{isolation}`, decided by the driver and refused when nested.
    isolation: Option<nvs_db::Isolation>,
    /// § 7's `{readOnly}`, on the same terms.
    read_only: bool,
    /// § 7's `{retries: n}` — how many re-runs a conflict may still have.
    retries: u64,
}

/// The first rung of § 7's backoff, and the base every rung after it doubles
/// from.
///
/// § 7 names no number and `{retries: n}` has no key for one, which is the
/// right shape rather than an omission: the wait is not spent letting a lock
/// clear. A server reports a deadlock or a serialization failure only once it
/// has already *resolved* the conflict — PostgreSQL detects a deadlock by
/// breaking it — so what the wait buys is the de-correlation of two requests
/// that conflicted with each other and would otherwise re-run into each other
/// on the same schedule. Ten milliseconds is a spread wide enough for that and
/// narrow enough that a request retried once does not notice it.
const RETRY_BACKOFF: std::time::Duration = std::time::Duration::from_millis(10);

/// The ceiling one rung may reach, however many retries were asked for.
///
/// `retries` is a `uint`, so the rung is capped here or a program naming sixty
/// of them draws its wait out of a range measured in years. A second is already
/// past the point where waiting longer buys anything — it is the whole latency
/// budget of the request the retry exists to save.
const RETRY_BACKOFF_CAP: std::time::Duration = std::time::Duration::from_secs(1);

/// § 7's wait before the retry with `taken` of them already spent: full jitter
/// over an exponential rung, capped at [`RETRY_BACKOFF_CAP`].
///
/// **Full jitter — uniform in `[0, base × 2^taken]` — and not the rung
/// itself**, which is `crate::http::transport`'s shape for
/// [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 6 and
/// holds here for a sharper reason: two requests that deadlocked against each
/// other were, by construction, running at the same time, so an unjittered
/// backoff hands them the same next instant and they collide again. Spreading
/// them is the whole of what the wait is for.
///
/// Unlike `crate::queue`'s ladder, which mixes a job's own id, this draws from
/// a random source, because a transaction has nothing to mix: the two
/// conflicting requests share their statements and their timing and differ in
/// nothing this function can read. `rand` is already this crate's, for the
/// jitter above, so the draw costs no dependency and reopens no question under
/// [ADR 0051](../../../docs/adr/0051-standard-library-tiers.md) § 4.
fn retry_backoff(taken: u32) -> std::time::Duration {
    // Clamped before the shift rather than after: sixteen rungs is already past
    // the cap for this base, and a shift by 32 is undefined rather than
    // saturating.
    let ceiling = RETRY_BACKOFF
        .saturating_mul(1_u32 << taken.min(16))
        .min(RETRY_BACKOFF_CAP);
    let nanos = u64::try_from(ceiling.as_nanos()).unwrap_or(u64::MAX);
    std::time::Duration::from_nanos(rand::rng().random_range(0..=nanos))
}

/// § 7's suspension between two attempts — the coroutine waits and the core
/// does not.
///
/// [`nvs_runtime::host::Host::sleep`] and deliberately not the bounded park
/// [`wait_for_slot`] takes, on that trait's own distinction between the two: a
/// backoff is a wait **for the clock**, with no state a peer could change to
/// end it early, and nothing that could wake it would mean anything — a
/// connection coming free says nothing about a deadlock that is already broken.
///
/// With no host on the thread the wait still happens, blocking, for
/// `Core\Time::sleep`'s reason: there is no scheduler under the call, so there
/// is no neighbour for
/// [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
/// § 6's tier-B failure to have as its victim.
///
/// # Errors
///
/// [`nvs_runtime::Ctx::cancel`]'s status for a task cancelled mid-wait, which
/// no `catch` sees. The attempt this waits after was already rolled back by the
/// conflict that ended it, so there is nothing left open to close.
fn wait_between_attempts(ctx: &mut nvs_runtime::Ctx, taken: u32) -> Result<(), Fault> {
    let waited = retry_backoff(taken);
    match nvs_runtime::host::with_current(|host| host.sleep(waited)) {
        Some(nvs_runtime::host::Woken::Cancelled) => Err(ctx.cancel()),
        Some(nvs_runtime::host::Woken::Elapsed) => Ok(()),
        None => {
            std::thread::sleep(waited);
            Ok(())
        }
    }
}

/// § 7's whole transaction, from the `BEGIN` to the answer, over a connection
/// it is handed.
///
/// Hoisted out of [`nvs_core_db_connection_transaction`] so that the retry rule
/// — four conditions over two conflict channels, and the one part of § 7 that
/// belongs to no driver — has a caller other than a member that can only reach
/// a real server. [`Attempts`] is where what that buys is written down.
///
/// **The two channels differ only in the shape a conflict arrives in.** The
/// commit's refusal is an `io::Error` carrying its own [`nvs_db::ServerError`];
/// a conflict a statement *inside* the closure raised is a pending exception on
/// the context instead, so its kind is read back off that object's
/// [`nvs_runtime::KIND_SLOT`] and turned into a [`nvs_db::DbErrorKind`] by
/// [`error_kind_of`]. Both then ask [`nvs_db::DbErrorKind::is_retryable`],
/// which is the one place the rule lives, and both wait
/// [`wait_between_attempts`] before the re-run.
///
/// # Errors
///
/// The closure's own failure, re-raised exactly as it left it where no attempt
/// is left to spend; `Core\Db\RolledBack` where the closure abandoned the
/// scope; and [`statement_failure`]'s rendering of a refusal by the `BEGIN` or
/// by the command that closes the level.
fn transacted(
    ctx: &mut nvs_runtime::Ctx,
    attempts: &mut impl Attempts,
    call: &Attempted,
) -> Result<Value, Fault> {
    let block = call.block;
    let mut left = call.retries;

    // § 11's readers, once for the whole call: every command below runs on
    // the one connection, and a retry does not change what is watching.
    // Read here for [`QueryWatch`]'s reason — the connection holds the
    // context for as long as each command does.
    let watch = QueryWatch::of(ctx, &block);

    loop {
        // § 7 retries **outermost transactions only**, and the depth before
        // the `BEGIN` is the only thing that says which this call is —
        // re-running a nested closure would re-run it inside an outer
        // transaction the conflict has already aborted.
        let outermost = attempts.depth(ctx)? == 0;
        // § 11's event covers § 7's own commands as well as the statements
        // inside them: a trace that showed the closure's writes but not the
        // `BEGIN` and the `COMMIT` around them would put the transaction's
        // whole cost on its last statement. The driver answers with the
        // span because only it knows whether the depth made this a
        // `SAVEPOINT` — [`nvs_db::PgConn::begin`] and its MySQL twin own
        // that, and the second of them spends two round trips where an
        // isolation level was asked for.
        let opened = attempts
            .begin(ctx, call.isolation, call.read_only)?
            .map_err(|refused| statement_failure(TRANSACTION_MEMBER, &block, None, &refused))?;
        file_span(ctx, watch, &block, opened);

        // The block name is handed on rather than looked up again: a
        // transaction refuses under the same `[db.<name>]` its connection
        // does, and the slot is the only place that name lives.
        let scope = crate::instance::build(
            &TRANSACTION,
            [
                Value::uint(call.key),
                owned(block),
                Value::bool(true),
                Value::null(),
            ],
        );
        let outcome = nvs_runtime::call_closure(ctx, call.closure, &[scope]);

        // Closed before the outcome is acted on, so that a `$tx` the closure
        // stored somewhere is already refusing by the time this call returns
        // — and closed on every path, which is why it is not inside a
        // branch. An attempt that retries gets its own scope object below,
        // for the same reason it gets its own `BEGIN`.
        let receiver = crate::instance::receiver(scope, &TRANSACTION, "transaction")?;
        crate::instance::set_slot(receiver, SCOPE_AT, Value::bool(false));
        let held = crate::instance::slot(receiver, REASON_AT);
        let abandoned = held.as_text().map(str::to_owned);
        discard(scope);

        let answered = match outcome {
            Ok(value) => value,
            Err(fault) => {
                // The closure's own conflict, under the same four
                // conditions the commit's is. It is a `Fault::Pending`
                // here, so the kind is read off the still-pending object
                // rather than off an `io::Error` this path never has —
                // borrowing it, because a failure that turns out not to be
                // retryable is re-raised exactly as the closure left it.
                let conflicted = ctx
                    .pending_slot(ThrownClass::DbError.name(), nvs_runtime::KIND_SLOT)
                    .and_then(error_kind_of)
                    .is_some_and(nvs_db::DbErrorKind::is_retryable);
                let retry = outermost && left > 0 && abandoned.is_none() && conflicted;
                // Best effort as before, and filed on the path where it
                // worked: an undo the server ran is a statement the trace
                // owes an entry, and one it refused leaves no span to file.
                let undone = attempts.roll_back(ctx).ok().and_then(Result::ok);
                if let Some(span) = undone {
                    file_span(ctx, watch, &block, span);
                }
                if !retry {
                    return Err(fault);
                }
                // Cleared before the next attempt: the retry is this
                // frame's decision that the throw did not happen as far as
                // the caller is concerned, and a pending failure left on
                // the context would surface against whatever ran next.
                drop(ctx.take_thrown());
                let taken = u32::try_from(call.retries.saturating_sub(left)).unwrap_or(u32::MAX);
                left -= 1;
                // After the rollback and after the context is clear, so a
                // cancellation arriving mid-wait ends a call with nothing open
                // and nothing pending.
                wait_between_attempts(ctx, taken)?;
                continue;
            }
        };

        // The driver's own error rather than the `Fault` it renders to: the
        // retry rule branches on § 8's kind, which only [`nvs_db`] can put
        // there and only this shape still carries.
        let closed = if abandoned.is_some() {
            attempts.roll_back(ctx)
        } else {
            attempts.commit(ctx)
        };
        let closed = match closed {
            Ok(closed) => closed,
            Err(fault) => {
                discard(answered);
                return Err(fault);
            }
        };

        // On two of the three paths the closure's answer is not this call's,
        // and this frame owns the only reference to it.
        let refused = match closed {
            Ok(span) => {
                // The command that closed the level, whichever it was, and
                // filed before this call returns rather than after — the
                // answer below leaves by three different paths.
                file_span(ctx, watch, &block, span);
                return match abandoned {
                    Some(reason) => {
                        discard(answered);
                        Err(Fault::thrown_as(ThrownClass::DbRolledBack, reason))
                    }
                    None => Ok(answered),
                };
            }
            Err(refused) => refused,
        };
        discard(answered);

        // § 7's `{retries: n}`, and the four conditions are all of it: an
        // outermost transaction, an attempt left, nothing that asked to be
        // rolled back, and a conflict the driver says may be re-run.
        let conflicted =
            nvs_db::ServerError::of(&refused).is_some_and(|server| server.kind.is_retryable());
        if outermost && left > 0 && abandoned.is_none() && conflicted {
            // The rung is how many re-runs this call has already spent, so the
            // first wait is one base and each one after it doubles. Read before
            // the decrement, where it is still 0.
            let taken = u32::try_from(call.retries.saturating_sub(left)).unwrap_or(u32::MAX);
            left -= 1;
            wait_between_attempts(ctx, taken)?;
            continue;
        }
        return Err(statement_failure(
            TRANSACTION_MEMBER,
            &block,
            None,
            &refused,
        ));
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Transaction::rollBack(string $reason): void` — ADR 0067 § 7's
    /// second hazard, closed by doing both things at once.
    ///
    /// **The flag is what the owning frame acts on and the throw is what the
    /// program sees**, and neither alone is enough. A throw on its own is
    /// catchable, so a `catch (Throwable)` between here and
    /// [`nvs_core_db_connection_transaction`] could leave the work committed;
    /// a flag on its own is `setRollbackOnly`, which every layer has to
    /// remember to check and one of them will not. Recording the reason in the
    /// receiver's own slot is what makes the decision survive the catch.
    ///
    /// **There is deliberately no way to roll back and carry on.** § 7 gives
    /// the member a `void` return because it always throws: a program that
    /// wants the writes it has made kept has not asked for a transaction, and
    /// one that wants to try again puts the retry outside the closure, where
    /// the second attempt gets its own `BEGIN`.
    fn nvs_core_db_transaction_roll_back(_ctx, args: [2]) {
        // Through the same guard every delegated member runs, and for the same
        // reason: a `$tx` that outlived its call has no scope to abandon.
        transaction_of(args[0], "rollBack")?;
        let receiver = crate::instance::receiver(args[0], &TRANSACTION, "rollBack")?;
        // Unreachable from source: parameter 0 is a `string` in [`TRANSACTION`]
        // above, so `E0401` refuses anything else before this runs.
        let reason = args[1]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{ROLL_BACK} expected a `string` reason, got tag {}",
                    args[1].tag_byte()
                ))
            })?
            .to_owned();
        crate::instance::set_slot(receiver, REASON_AT, owned(args[1]));
        Err(Fault::thrown_as(ThrownClass::DbRolledBack, reason))
    }
}

/// Drops a reference this frame owns, for a value it is not handing back.
///
/// The mirror of [`owned`], and it exists for one member: `transaction` builds
/// the scope object and receives the closure's answer, and on the paths where
/// the transaction did not commit neither of them reaches Novis code at all.
fn discard(value: Value) {
    #[expect(
        unsafe_code,
        reason = "the reference released here is one this frame took — from \
                  `instance::build`, or from `call_closure`, which hands back a \
                  value the caller owns"
    )]
    unsafe {
        value.release();
    }
}

/// A value read out of an array or a slot, with a reference of the caller's
/// own.
///
/// Every read below borrows — [`NvsArray::get`], `value_at` and
/// [`crate::instance::slot`] all hand back a reference the holder still owns —
/// so this is the one place the second one is taken, rather than an `unsafe`
/// block at each of the members that hands a held value out.
fn owned(value: Value) -> Value {
    #[expect(
        unsafe_code,
        reason = "the entry is owned by an array the receiver holds, which \
                  outlives this call, so the value handed back needs a \
                  reference of its own"
    )]
    unsafe {
        value.retain();
    }
    value
}

/// The rows one of [`ROWS`]'s members reads, borrowed from its receiver.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot holding anything but an array: the slot is
/// written by [`nvs_core_db_connection_query`] and
/// [`nvs_core_db_connection_query_as`] out of one [`queried_rows`] and by
/// nothing else, so that is a paste error in this crate rather than anything a
/// program can cause.
fn result_rows(args: &[Value], member: &str) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let receiver = crate::instance::receiver(args[0], &ROWS, member)?;
    let held = crate::instance::slot(receiver, ROWS_AT);
    let array = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{ROWS_NAME}::{member} found tag {} in its `{ROWS_SLOT}` slot",
            held.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// The class a [`ROWS`] hydrates its rows into, or `None` where they stay
/// [`ROW`]s — [`ROWS_CLASS_SLOT`], read by the three members that hand a row
/// out and by nothing else.
///
/// # Errors
///
/// [`crate::instance::receiver`]'s, for a receiver of the wrong class. The slot
/// itself cannot refuse: a value that is not a descriptor is the `null`
/// [`nvs_core_db_connection_query`] wrote.
fn rows_class(
    args: &[Value],
    member: &str,
) -> Result<Option<*const nvs_runtime::ClassDesc>, Fault> {
    let receiver = crate::instance::receiver(args[0], &ROWS, member)?;
    Ok(crate::instance::slot(receiver, ROWS_CLASS_AT).as_class_desc())
}

/// One row of a [`ROWS`] as the object its member answers with: a [`ROW`] over
/// the very array the receiver holds, or — where [`rows_class`] named one — an
/// instance of the class `queryAs<T>`'s call site wrote.
///
/// # Errors
///
/// [`hydrate`]'s, for the second shape, and a [`Fault::fatal`] for a row slot
/// holding anything but an array, which is [`row_at`]'s paste error.
fn row_object(
    ctx: &mut nvs_runtime::Ctx,
    row: Value,
    class: Option<*const nvs_runtime::ClassDesc>,
    member: &str,
) -> Result<Value, Fault> {
    let Some(class) = class else {
        return Ok(crate::instance::build(&ROW, [owned(row)]));
    };
    let held = row.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{ROWS_NAME}::{member} found tag {} where a row should be",
            row.tag_byte()
        ))
    })?;
    let columns = crate::arr::borrowed(held);
    #[expect(
        unsafe_code,
        reason = "the descriptor came out of a `ClassDescConst` the compiled unit \
                  owns, written into this receiver's own slot by \
                  `nvs_core_db_connection_query_as`, so it outlives this call"
    )]
    unsafe {
        hydrate(ctx, class, &columns)
    }
}

/// One row built into the class `queryAs<T>`'s call site wrote — ADR 0071 § 5's
/// accumulate-then-construct, over a row whose columns ADR 0067 § 9's type map
/// has already decoded.
///
/// **Every field is a check and not a parse**, which is the whole difference
/// from [`crate::json`]'s walk over the same [`nvs_runtime::CodecField`] list:
/// a column arrives as the Novis value § 9 names for its SQL type, so what is
/// left is whether that value is the one the field declares — and § 6's
/// "losslessly or throws" is what decides the two integer types against each
/// other, exactly as [`ROW`]'s own typed readers do.
///
/// **§ 5's `path` is the column name**, which that section says outright for
/// the `Db` half, so a list element's position rides in its message rather
/// than in a dotted path.
///
/// # Errors
///
/// A `ParseError` carrying every bad column at once — `ParseError` rather than
/// § 8's `DbError` because this module's known gap 4 is that the latter is not
/// in spec § 10's tree, and because `issues` is a property only the former
/// declares. A class carrying no `#[Db\Derive]` is a `LogicError` instead: it
/// is the program's mistake rather than the row's, and gap 8 owns why it is not
/// the compile-time diagnostic it should be.
///
/// # Safety
///
/// `class` must refer to a live descriptor whose method table `nvs-codegen` has
/// filled.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn hydrate(
    ctx: &mut nvs_runtime::Ctx,
    class: *const nvs_runtime::ClassDesc,
    row: &NvsArray,
) -> Result<Value, Fault> {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let desc = unsafe { &*class };
    let fields = desc.db_codec();
    if fields.is_empty() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{QUERY_AS}: `{}` carries no `#[Db\\Derive]`, so there is no column mapping to \
                 build one from — ADR 0071 § 1's opt-in is that attribute, and this is the \
                 refusal a compile-time diagnostic would be better at (`nvs_stdlib::db`'s known \
                 gap 8)",
                desc.name()
            ),
        ));
    }
    let mut ctor_args = vec![Value::null(); desc.ctor_arity()];
    let mut filled = vec![false; desc.ctor_arity()];
    let mut issues: Vec<(String, String)> = Vec::new();
    for field in fields {
        let Some(held) = row.get(field.key.as_bytes()) else {
            issues.push((
                field.key.clone(),
                format!(
                    "the result has no column `{}` — a `#[Db\\Field(name: \"…\")]` is how a field \
                     reads one under another name",
                    field.key
                ),
            ));
            continue;
        };
        match hydrated(field, held) {
            Ok(value) => match ctor_args.get_mut(field.param) {
                Some(slot) => {
                    *slot = owned(value);
                    filled[field.param] = true;
                }
                None => {
                    release_all(&ctor_args);
                    // Unreachable from source with no diagnostic to name:
                    // `field.param` and `desc.ctor_arity()` are two readings of
                    // one class's own constructor, both written while compiling
                    // that class.
                    return Err(Fault::fatal(format!(
                        "internal error: `{}`'s `{}` field names constructor parameter {} of {}",
                        desc.name(),
                        field.key,
                        field.param,
                        desc.ctor_arity()
                    )));
                }
            },
            Err(why) => issues.push((field.key.clone(), why)),
        }
    }

    if !issues.is_empty() {
        release_all(&ctor_args);
        return Err(Fault::thrown_with_issues(
            ThrownClass::Parse,
            format!(
                "{QUERY_AS}: {} column(s) of `{}` did not match the row",
                issues.len(),
                desc.name()
            ),
            crate::issue::list(
                issues
                    .iter()
                    .map(|(path, message)| (path.as_str(), message.as_str())),
            ),
        ));
    }
    // ADR 0071 § 3's skipped field with a constructor default, exactly as
    // `Core\Json::decodeAs` meets it: the default is a constant the *call site*
    // emits and there is no call site here, so this is loud rather than a
    // `null` that would be right for one declaration in ten.
    if let Some(index) = filled.iter().position(|done| !done) {
        release_all(&ctor_args);
        return Err(Fault::fatal(format!(
            "{QUERY_AS}: `{}`'s constructor parameter {index} is not a codec field, and a \
             skipped field's default is `nvs_stdlib::db`'s own known gap 8",
            desc.name()
        )));
    }
    #[expect(
        unsafe_code,
        reason = "the same live descriptor, and every argument is one this frame \
                  owns and hands over"
    )]
    unsafe {
        nvs_runtime::construct(ctx, class, &ctor_args)
    }
}

/// One column as the value one [`nvs_runtime::CodecField`] takes, borrowed from
/// the row — or § 5's message for why it is not that value.
///
/// The reference is *not* taken here: [`hydrate`] does that with [`owned`] on
/// the one value it keeps, so a list's element checks below cost nothing and
/// leak nothing.
fn hydrated(field: &nvs_runtime::CodecField, held: Value) -> Result<Value, String> {
    if held.tag() == Some(Tag::Null) {
        return if field.nullable {
            Ok(held)
        } else {
            Err(
                "the column is SQL NULL and the field is not declared `?T` — ADR 0067 § 9 reads a \
                 NULL back as `null` whatever the column's type is"
                    .to_owned(),
            )
        };
    }
    let nvs_runtime::CodecTy::List = field.ty else {
        return converted(field.ty, field.cases.as_ref(), field.class.as_deref(), held);
    };
    let Some(element) = field.element else {
        return Err(
            "this field is a list whose element type the derive pass did not record".to_owned(),
        );
    };
    let held_ptr = held
        .array_ptr()
        .ok_or_else(|| wanted("an `array<T>` column", held))?;
    let elements = crate::arr::borrowed(held_ptr);
    let mut from = 0usize;
    while let Some(slot) = elements.next_slot(from) {
        let one = elements
            .value_at(slot)
            .expect("next_slot only names live entries");
        // A NULL element is taken as it comes: a list field's element carries
        // no nullability of its own on `CodecField`, and PostgreSQL's array
        // types all admit one.
        if one.tag() != Some(Tag::Null) {
            converted(element, field.cases.as_ref(), field.class.as_deref(), one)
                .map_err(|why| format!("element {slot}: {why}"))?;
        }
        from = slot + 1;
    }
    Ok(held)
}

/// One value against one wire type: itself where it already is that type, the
/// same number under the other integer tag where ADR 0067 § 6's "losslessly or
/// throws" allows it, and § 5's message otherwise.
///
/// `class` is the rendered name the *declaration* carried, where `ty` is a
/// [`nvs_runtime::CodecTy::Class`]: the field's own class for a scalar field
/// and the element's for a list's element, which is exactly how
/// [`nvs_runtime::CodecField::class`] holds it — so both callers hand over the
/// same field's, and neither has to know which of the two it is.
///
/// Never a heap value it did not receive, so nothing here allocates or takes a
/// reference — see [`hydrated`].
fn converted(
    ty: nvs_runtime::CodecTy,
    cases: Option<&nvs_runtime::EnumCases>,
    class: Option<&str>,
    held: Value,
) -> Result<Value, String> {
    use nvs_runtime::CodecTy;

    match ty {
        // ADR 0007's `mixed`: whatever the column held, unchecked.
        CodecTy::Mixed => Ok(held),
        // § 6's three crossings, and a field asks for one in exactly the words
        // a `Db\Row` reader does: the helpers below are the rule's one home, so
        // `queryAs<T>` cannot drift from `->bool()` the way two copies would.
        CodecTy::Bool => match requested_bool(held) {
            Requested::Is(flag) => Ok(Value::bool(flag)),
            Requested::Lossy(holds) => Err(lossy("`bool`", &holds)),
            Requested::Mismatched => Err(wanted("`bool`", held)),
        },
        CodecTy::Int => match requested_int(held) {
            Requested::Is(number) => Ok(Value::int(number)),
            Requested::Lossy(holds) => Err(lossy("`int`", &holds)),
            Requested::Mismatched => Err(wanted("`int`", held)),
        },
        CodecTy::Uint => match requested_uint(held) {
            Requested::Is(number) => Ok(Value::uint(number)),
            Requested::Lossy(holds) => Err(lossy("`uint`", &holds)),
            Requested::Mismatched => Err(wanted("`uint`", held)),
        },
        CodecTy::Float => match held.tag() {
            Some(Tag::Float) => Ok(held),
            _ => Err(wanted("`float`", held)),
        },
        CodecTy::Str => match held.tag() {
            Some(Tag::Str) => Ok(held),
            _ => Err(wanted("`string`", held)),
        },
        // ADR 0010 § 6: a case *is* the integer behind it by the time it is a
        // `Value`, so this is a membership test and not a construction.
        CodecTy::Enum => {
            let Some(cases) = cases else {
                return Err(
                    "this field is an enum whose cases the derive pass did not record".to_owned(),
                );
            };
            let backing = held
                .as_int()
                .map(i128::from)
                .or_else(|| held.as_uint().map(i128::from))
                .ok_or_else(|| wanted("an enum's backing integer", held))?;
            if cases.values.binary_search(&backing).is_err() {
                return Err(format!(
                    "the column holds {backing}, which this enum declares no case for"
                ));
            }
            Ok(if cases.unsigned {
                #[expect(
                    clippy::cast_sign_loss,
                    clippy::cast_possible_truncation,
                    reason = "the value is one of the declared cases, which a `uint`-backed \
                              enum's are all `u64`"
                )]
                Value::uint(backing as u64)
            } else {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the value is one of the declared cases, which an `int`-backed \
                              enum's are all `i64`"
                )]
                Value::int(backing as i64)
            })
        }
        // A row is a flat list of columns and `nvs_types::derive`'s own
        // `db_reachable` maps none of them to a nested class, so this arm is
        // § 9's five value types and nothing else. The instance is built long
        // before hydration reads it — [`column_value`] is where a driver's
        // components become a `Core\Time` or a `Core\Uuid` — so all this arm
        // asks is the question every other one asks: is it what the field
        // declared.
        //
        // By rendered name rather than by descriptor pointer, because the two
        // sides are written at different times: the label is what
        // `nvs_types::derive` read off the declaration, and the descriptor is
        // the one `crate::instance` gave the value the driver's components
        // built. ADR 0051 keeps the `Core\` prefix for Tier 0, so no program
        // can declare a second class answering to one of these five names.
        CodecTy::Class => {
            let Some(declared) = class else {
                return Err(
                    "this field is a class whose name the derive pass did not record".to_owned(),
                );
            };
            let Some(object) = held.obj_ptr() else {
                return Err(wanted(&format!("`{declared}`"), held));
            };
            #[expect(
                unsafe_code,
                reason = "the value argument owns a reference to a live allocation, \
                          so it is live for the length of this call"
            )]
            let found = unsafe { &*nvs_runtime::NvsObj::class_of(object) };
            if found.name() == declared {
                Ok(held)
            } else {
                Err(format!(
                    "the column came back as a `{}` and this field declares `{declared}` — ADR \
                     0067 § 9's type map is what each column reads back as",
                    found.name()
                ))
            }
        }
        // `nvs_types::derive` erases `decimal`, `bytes` and every inline shape
        // to this, and its own gap 1 owns the erasure.
        CodecTy::Opaque => Err(
            "this field's declared type is one the derive pass has no wire \
                                type for — a `decimal`, a `bytes` or an inline shape"
                .to_owned(),
        ),
        // Unreachable: [`hydrated`] takes the list arm before this is called,
        // and a list's element is never itself a list (`CodecTy::List`).
        CodecTy::List => Err("a list of lists is not a column type".to_owned()),
    }
}

/// § 5's message for a column that came back as something else, said with what
/// it actually is — [`wrong_column_type`]'s shape, for the walk that has a
/// field's declared type in hand rather than a reader's name.
fn wanted(want: &str, held: Value) -> String {
    format!(
        "the column came back as {} and this field declares {want} — ADR 0067 § 9's type map is \
         what each column reads back as",
        held.tag().map_or_else(
            || format!("tag {}", held.tag_byte()),
            |tag| tag.describe().to_owned()
        )
    )
}

/// [`wanted`]'s other half, and [`column_out_of_range`]'s counterpart on this
/// side: the column is of the family the field declares and holds a value that
/// does not survive the crossing. `holds` is [`Requested::Lossy`]'s fragment, so
/// a field and a reader say the same thing about the same value.
fn lossy(want: &str, holds: &str) -> String {
    format!(
        "the column holds {holds}, so reading it as {want} would not be the same value — ADR 0067 \
         § 6 converts losslessly or throws"
    )
}

/// Releases every reference in `values` — [`nvs_runtime::construct`]'s "an
/// argument is consumed whether or not the constructor ran", owed by every path
/// out of [`hydrate`] that does not reach it.
fn release_all(values: &[Value]) {
    for value in values {
        #[expect(
            unsafe_code,
            reason = "each entry is either `null` or a value this frame took a \
                      reference to in `hydrate`"
        )]
        unsafe {
            value.release();
        }
    }
}

/// One row of a [`ROWS`], borrowed — see [`result_rows`] for the refusal.
fn row_at(
    rows: &NvsArray,
    slot: usize,
    member: &str,
) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let held = rows
        .value_at(slot)
        .expect("next_slot only names live entries");
    let array = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{ROWS_NAME}::{member} found tag {} where a row should be",
            held.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// The columns of the [`ROW`] one of its members was called on, borrowed —
/// [`result_rows`]'s twin, and its refusal is the same paste error.
fn row_columns(args: &[Value], member: &str) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let receiver = crate::instance::receiver(args[0], &ROW, member)?;
    let held = crate::instance::slot(receiver, COLUMNS_AT);
    let array = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{ROW_NAME}::{member} found tag {} in its `{COLUMNS_SLOT}` slot",
            held.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// A column-name argument as the bytes an array is keyed by.
///
/// # Errors
///
/// A [`Fault::fatal`], because every row that takes one declares it `string` and
/// a non-text argument is refused at `E0401` first.
fn column_name<'a>(value: &'a Value, class: &str, member: &str) -> Result<&'a [u8], Fault> {
    value.as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{class}::{member} expected a `string` column name, got tag {}",
            value.tag_byte()
        ))
    })
}

/// Spec § 18's "An unknown column name throws", with the names that would have
/// worked — the one thing a caller holding a misspelling wants next.
fn unknown_column(class: &str, member: &str, name: &[u8], columns: &NvsArray) -> Fault {
    let known: Vec<String> = columns
        .keys()
        .iter()
        .map(|key| String::from_utf8_lossy(key).into_owned())
        .collect();
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{class}::{member}: no column is named `{}` — this row has {}",
            String::from_utf8_lossy(name),
            if known.is_empty() {
                "none at all".to_owned()
            } else {
                known.join(", ")
            }
        ),
    )
}

/// The value at one column *position*, counted from zero over the server's own
/// description, or `None` for a position the row does not reach — the `int` arm
/// of [`nvs_core_db_rows_column`]'s `int|string` key.
fn column_at(row: &NvsArray, index: i64) -> Option<Value> {
    let index = usize::try_from(index).ok()?;
    let mut from = 0usize;
    for _ in 0..index {
        from = row.next_slot(from)? + 1;
    }
    row.next_slot(from).and_then(|slot| row.value_at(slot))
}

/// The column one of [`ROW`]'s eleven typed readers was asked for, borrowed and
/// paired with the name it was asked by, or `None` where the column is SQL
/// `NULL` — which is the `?T` every one of them answers.
///
/// # Errors
///
/// The [`unknown_column`] throw, and [`row_columns`]'s and [`column_name`]'s
/// fatals.
fn typed_column<'a>(args: &'a [Value], member: &str) -> Result<(&'a [u8], Option<Value>), Fault> {
    let columns = row_columns(args, member)?;
    let name = column_name(&args[1], ROW_NAME, member)?;
    let value = columns
        .get(name)
        .ok_or_else(|| unknown_column(ROW_NAME, member, name, &columns))?;
    Ok((name, (value.tag() != Some(Tag::Null)).then_some(value)))
}

/// A typed reader's refusal for a column it will not convert — ADR 0067 § 6's
/// "lossless conversion or throws", said with what the column actually is.
fn wrong_column_type(member: &str, name: &[u8], value: Value, want: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{ROW_NAME}::{member}: the column `{}` came back as {} and this reader answers {want} \
             only — ADR 0067 § 6 converts losslessly or throws, and `->get()` plus `as` is the \
             universal path",
            String::from_utf8_lossy(name),
            value.tag().map_or_else(
                || format!("tag {}", value.tag_byte()),
                |tag| tag.describe().to_owned()
            )
        ),
    )
}

/// The other half of that refusal: the column *is* an integer, and the reader
/// asked for is the one of `int`/`uint` it does not fit.
fn column_out_of_range(member: &str, name: &[u8], holds: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{ROW_NAME}::{member}: the column `{}` holds {holds}, so reading it as `{member}` \
             would not be the same value — ADR 0067 § 6 converts losslessly or throws",
            String::from_utf8_lossy(name)
        ),
    )
}

/// What one of [ADR 0067](../../../docs/adr/0067-core-db.md) § 6's *requests*
/// makes of the value a column's natural type already produced.
///
/// Three answers rather than two, because the two refusals are different
/// questions and a caller says so in different words: [`Self::Lossy`] is the
/// right family and a value that does not survive the crossing, while
/// [`Self::Mismatched`] is a family with no crossing to consider at all. A
/// `DECIMAL` asked for `float` is the second and not the first — ADR 0054 keeps
/// those apart by construction, so there is no value of one that is a value of
/// the other.
enum Requested<T> {
    /// § 6's "losslessly".
    Is(T),
    /// The right family, the wrong value, carrying the fragment that says which
    /// — the number, and why it does not cross. Built here so the reader's
    /// sentence and the `#[Db\Derive]` field's quote one wording.
    Lossy(String),
    /// The wrong family.
    Mismatched,
}

/// § 6's `bool` request: the one crossing in the map that is neither a column's
/// natural type nor a refusal.
///
/// `BOOLEAN` and `BIT(1)` arrive as `Tag::Bool` already. MySQL and MariaDB have
/// neither, and § 9 reads their `TINYINT(1)` as `int` because a display width is
/// not a type and nothing on the wire separates a flag column from a small
/// integer — so the *request* is what decides, and § 6 says outright that it
/// decides this one. `0` and `1` are the whole of what a flag column holds; a
/// stored `7` throws rather than reading as PHP's `true`, which is the half of
/// this rule that keeps the conversion lossless.
fn requested_bool(value: Value) -> Requested<bool> {
    if let Some(flag) = value.as_bool() {
        return Requested::Is(flag);
    }
    let number = match (value.as_int(), value.as_uint()) {
        (Some(signed), _) => i128::from(signed),
        (_, Some(unsigned)) => i128::from(unsigned),
        _ => return Requested::Mismatched,
    };
    match number {
        0 => Requested::Is(false),
        1 => Requested::Is(true),
        _ => Requested::Lossy(format!("{number}, which is neither `0` nor `1`")),
    }
}

/// § 6's `int` request, which crosses from the other half of ADR 0007 § 4's one
/// integer and stops where `int` does.
fn requested_int(value: Value) -> Requested<i64> {
    if let Some(signed) = value.as_int() {
        return Requested::Is(signed);
    }
    let Some(unsigned) = value.as_uint() else {
        return Requested::Mismatched;
    };
    i64::try_from(unsigned).map_or_else(
        |_| Requested::Lossy(format!("{unsigned}, which is past `int`'s ceiling")),
        Requested::Is,
    )
}

/// § 6's `uint` request — [`requested_int`]'s twin, and what `BIGINT UNSIGNED`
/// needs: PHP overflows that column to a `float` and stops comparing equal to
/// itself.
fn requested_uint(value: Value) -> Requested<u64> {
    if let Some(unsigned) = value.as_uint() {
        return Requested::Is(unsigned);
    }
    let Some(signed) = value.as_int() else {
        return Requested::Mismatched;
    };
    u64::try_from(signed).map_or_else(
        |_| Requested::Lossy(format!("{signed}, which is below `uint`'s floor")),
        Requested::Is,
    )
}

nvs_runtime::nvs_helper! {
    /// `$rows->all(): array<Db\Row>` — every row at once, replacing
    /// `PDO::fetchAll` and the fetch-mode argument that chose its shape.
    ///
    /// One object per row and no second array: a [`ROW`]'s slot takes a
    /// reference to the row [`ROWS`] already holds ([`COLUMNS_SLOT`]), so what
    /// this spends over a result already in memory is one small object each.
    fn nvs_core_db_rows_all(ctx, args: [1]) {
        let rows = result_rows(args, "all")?;
        let class = rows_class(args, "all")?;
        let mut all = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = rows.next_slot(from) {
            let row = rows
                .value_at(slot)
                .expect("next_slot only names live entries");
            all.append(row_object(ctx, row, class, "all")?);
            from = slot + 1;
        }
        Ok(Value::array(all))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<Row>::iterate(): Iterator<Row>` — a cursor over one [`ROW`]
    /// per row this value is holding.
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed,
    /// which is why this body releases it and [`nvs_core_db_rows_all`] does
    /// not. [`crate::cursor`]'s module docs own both halves of that.
    ///
    /// The objects are built here rather than shared with a previous `all()`,
    /// because there need not have been one — and it costs no more than `all()`
    /// does for the same reason: a [`ROW`]'s slot takes a reference to the row
    /// [`ROWS`] already holds, so a `foreach` over a thousand rows allocates a
    /// thousand small objects and not a second thousand arrays. The snapshot
    /// every § 9 collection's `iterate()` has to take is free here as well:
    /// [`ROWS`] has no mutating member, so the array was already frozen when
    /// [`nvs_core_db_connection_query`] built it.
    fn nvs_core_db_rows_iterate(ctx, args: [1]) {
        let cursor = (|| {
            let rows = result_rows(args, nvs_runtime::sequence::ITERATE)?;
            let class = rows_class(args, nvs_runtime::sequence::ITERATE)?;
            let mut all = NvsArray::new();
            let mut from = 0usize;
            while let Some(slot) = rows.next_slot(from) {
                let row = rows
                    .value_at(slot)
                    .expect("next_slot only names live entries");
                all.append(row_object(ctx, row, class, nvs_runtime::sequence::ITERATE)?);
                from = slot + 1;
            }
            Ok(crate::cursor::over(all))
        })();
        crate::cursor::consume(args[0]);
        cursor
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->first(): ?Db\Row` — the first row, or `null` for none.
    ///
    /// `null` rather than a throw, and rather than PHP's `false`: ADR 0063 R5
    /// makes `?T` the only absence spelling, and a `select` that matched
    /// nothing is an answer rather than a failure. There is no cursor a second
    /// call would move past, either — this is the first row every time, which
    /// is what makes it safe to write in a condition.
    fn nvs_core_db_rows_first(ctx, args: [1]) {
        let rows = result_rows(args, "first")?;
        let Some(slot) = rows.next_slot(0) else {
            return Ok(Value::null());
        };
        let class = rows_class(args, "first")?;
        let row = rows
            .value_at(slot)
            .expect("next_slot only names live entries");
        row_object(ctx, row, class, "first")
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->value(): mixed` — the first column of the first row, replacing
    /// `PDOStatement::fetchColumn`.
    ///
    /// **An empty result is `null`, which a NULL column is too**, and the two
    /// are not told apart here. The declared type is § 18's `mixed`, so there
    /// is no `?T` to put the absence in that the value itself could not
    /// already be; a caller that has to distinguish them asks `count()`, which
    /// is exact. The alternative — throwing on an empty result — would make
    /// the commonest use, a `select count(*)`, the one shape that has to be
    /// wrapped in a `try`.
    fn nvs_core_db_rows_value(_ctx, args: [1]) {
        let rows = result_rows(args, "value")?;
        let Some(slot) = rows.next_slot(0) else {
            return Ok(Value::null());
        };
        let row = row_at(&rows, slot, "value")?;
        Ok(match row.next_slot(0) {
            Some(at) => owned(
                row.value_at(at)
                    .expect("next_slot only names live entries"),
            ),
            None => Value::null(),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->column(int|string $key): array<mixed>` — one column from every
    /// row, replacing `PDO::fetchAll(PDO::FETCH_COLUMN)`.
    ///
    /// The key is checked against each row rather than once, because the check
    /// *is* the lookup: rows all carry the same columns, so the first row
    /// decides and the rest cost a hash lookup each. An empty result therefore
    /// answers an empty array for a key that names nothing — it describes no
    /// columns for the key to be wrong about, and `columns()` is the member
    /// that answers what a statement described.
    fn nvs_core_db_rows_column(_ctx, args: [2]) {
        let rows = result_rows(args, "column")?;
        let mut taken = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = rows.next_slot(from) {
            let row = row_at(&rows, slot, "column")?;
            let found = if let Some(index) = args[1].as_int() {
                column_at(&row, index).ok_or_else(|| {
                    Fault::thrown_as(
                        ThrownClass::Logic,
                        format!(
                            "{ROWS_NAME}::column: there is no column at position {index} — this \
                             row has {}, counted from zero",
                            row.count()
                        ),
                    )
                })?
            } else {
                let name = column_name(&args[1], ROWS_NAME, "column")?;
                row.get(name)
                    .ok_or_else(|| unknown_column(ROWS_NAME, "column", name, &row))?
            };
            taken.append(owned(found));
            from = slot + 1;
        }
        Ok(Value::array(taken))
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->count(): uint` — how many rows there are, replacing
    /// `PDOStatement::rowCount` on a select.
    ///
    /// Exact, and that is § 4's buffered default paying for itself: every row
    /// was read before `query` answered, so this is a length rather than the
    /// driver-dependent guess `rowCount` is on a select.
    fn nvs_core_db_rows_count(_ctx, args: [1]) {
        let rows = result_rows(args, "count")?;
        let held = rows.count();
        let count = u64::try_from(held).map_err(|_| {
            Fault::fatal(format!("{ROWS_NAME}::count: {held} rows do not fit a `uint`"))
        })?;
        Ok(Value::uint(count))
    }
}

nvs_runtime::nvs_helper! {
    /// `$rows->columns(): array<Db\Column>` — what the statement described,
    /// replacing `PDOStatement::getColumnMeta` and `mysqli_fetch_fields`.
    ///
    /// A reader over [`ROWS_COLUMNS_SLOT`] and nothing more: the objects were
    /// built when the result was ([`described_columns`]), so this hands that
    /// array on under a second reference exactly as a [`ROW`] takes one of the
    /// row it reads. Two calls answer the same columns rather than two
    /// descriptions of them, and a result set that matched no rows answers the
    /// same thing a matching one would.
    fn nvs_core_db_rows_columns(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &ROWS, "columns")?;
        Ok(owned(crate::instance::slot(receiver, ROWS_COLUMNS_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$column->name(): string` — the label the server described this column
    /// with, replacing `getColumnMeta`'s `name` key.
    fn nvs_core_db_column_name(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &COLUMN, "name")?;
        Ok(owned(crate::instance::slot(receiver, LABEL_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$column->type(): Db\ColumnType` — what the column was declared as, as
    /// the case [`COLUMN_TYPE`] registers rather than the vendor type name
    /// `getColumnMeta` answers.
    ///
    /// The slot already holds the ordinal an enum is at runtime, written there
    /// by [`column_type_value`], so nothing is classified here: a description
    /// is of the statement and a statement is described once.
    fn nvs_core_db_column_type(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &COLUMN, "type")?;
        Ok(crate::instance::slot(receiver, DECLARED_AT))
    }
}

nvs_runtime::nvs_helper! {
    /// `$column->nullable(): bool` — whether the column may hold NULL, which
    /// on this driver is always `true` and [`COLUMN_NULLABLE_DOC`] is where a
    /// program's author reads why.
    fn nvs_core_db_column_nullable(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &COLUMN, "nullable")?;
        Ok(crate::instance::slot(receiver, NULLABLE_AT))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->has(string $name): bool` — whether the row carries this column.
    ///
    /// A NULL column is present, which is the whole point of asking: the
    /// typed readers answer `null` for both "this column is NULL" and nothing
    /// else, so this is where "there is no such column" is told from it.
    fn nvs_core_db_row_has(_ctx, args: [2]) {
        let columns = row_columns(args, "has")?;
        let name = column_name(&args[1], ROW_NAME, "has")?;
        Ok(Value::bool(columns.has_key(name)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->get(string $name): mixed` — one column, whatever ADR 0067 § 9's
    /// type map made of it, and the universal path the typed readers narrow.
    fn nvs_core_db_row_get(_ctx, args: [2]) {
        let columns = row_columns(args, "get")?;
        let name = column_name(&args[1], ROW_NAME, "get")?;
        let found = columns
            .get(name)
            .ok_or_else(|| unknown_column(ROW_NAME, "get", name, &columns))?;
        Ok(owned(found))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->toArray(): array<string, mixed>` — the whole row, replacing
    /// `FETCH_ASSOC`.
    ///
    /// The slot's own array under a second reference rather than a copy: an
    /// Novis array is a value with copy-on-write, so a caller that writes to
    /// what it got here separates it and the row is untouched, and a caller
    /// that only reads pays nothing at all.
    fn nvs_core_db_row_to_array(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &ROW, "toArray")?;
        Ok(owned(crate::instance::slot(receiver, COLUMNS_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->string(string $name): ?string` — the text family alone, which
    /// [`ROW`]'s own docs state the rule for.
    fn nvs_core_db_row_string(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "string")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if value.as_str_bytes().is_none() {
            return Err(wrong_column_type("string", name, value, "`string`"));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->bytes(string $name): ?bytes` — [`nvs_core_db_row_string`]'s twin
    /// on ADR 0009's other side.
    fn nvs_core_db_row_bytes(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "bytes")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if value.as_bytes().is_none() {
            return Err(wrong_column_type("bytes", name, value, "`bytes`"));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->int(string $name): ?int` — the signed half of ADR 0007 § 4's one
    /// integer, and one of the two readers that cross.
    fn nvs_core_db_row_int(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "int")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        match requested_int(value) {
            Requested::Is(number) => Ok(Value::int(number)),
            Requested::Lossy(holds) => Err(column_out_of_range("int", name, &holds)),
            Requested::Mismatched => Err(wrong_column_type("int", name, value, "an integer")),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->uint(string $name): ?uint` — [`nvs_core_db_row_int`]'s unsigned
    /// twin, and what `BIGINT UNSIGNED` needs: PHP overflows that column to a
    /// `float` and stops comparing equal to itself.
    fn nvs_core_db_row_uint(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "uint")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        match requested_uint(value) {
            Requested::Is(number) => Ok(Value::uint(number)),
            Requested::Lossy(holds) => Err(column_out_of_range("uint", name, &holds)),
            Requested::Mismatched => Err(wrong_column_type("uint", name, value, "an integer")),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->float(string $name): ?float` — `FLOAT`, `REAL` and `DOUBLE`.
    ///
    /// A `DECIMAL` is refused rather than widened: that conversion is the one
    /// this whole type map exists to stop happening by accident.
    fn nvs_core_db_row_float(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "float")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        value
            .as_float()
            .map(Value::float)
            .ok_or_else(|| wrong_column_type("float", name, value, "`float`"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->bool(string $name): ?bool` — `BOOLEAN` and `BIT(1)`, and on
    /// request `TINYINT(1)` as well.
    ///
    /// The one reader whose answer is not the column's natural type: MySQL and
    /// MariaDB have no boolean column at all, so § 6 makes the *request* what
    /// converts a `0`/`1` integer here. [`requested_bool`] owns the rule and the
    /// stored `7` it refuses.
    fn nvs_core_db_row_bool(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "bool")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        match requested_bool(value) {
            Requested::Is(flag) => Ok(Value::bool(flag)),
            Requested::Lossy(holds) => Err(column_out_of_range("bool", name, &holds)),
            Requested::Mismatched => Err(wrong_column_type("bool", name, value, "`bool`")),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->decimal(string $name): ?decimal` — ADR 0054's exact scalar, where
    /// PHP hands back a string and leaves the parsing to the caller.
    fn nvs_core_db_row_decimal(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "decimal")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        value
            .as_decimal()
            .map(Value::decimal)
            .ok_or_else(|| wrong_column_type("decimal", name, value, "`decimal`"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->instant(string $name): ?Core\Time\Instant` — `TIMESTAMPTZ` and
    /// `datetimeoffset`.
    ///
    /// One of the four [`ROW`]'s docs name: the value is the `Core\Time\Instant`
    /// [`column_value`] built out of the column, so this member is the lookup
    /// and the class check and nothing else.
    fn nvs_core_db_row_instant(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "instant")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if !crate::instance::is_instance(value, &crate::time::INSTANT) {
            return Err(wrong_column_type(
                "instant",
                name,
                value,
                "a `Core\\Time\\Instant`",
            ));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->date(string $name): ?Core\Time\Date` — a `DATE`, and one of
    /// [`nvs_core_db_row_instant`]'s four.
    fn nvs_core_db_row_date(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "date")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if !crate::instance::is_instance(value, &crate::time::DATE) {
            return Err(wrong_column_type("date", name, value, "a `Core\\Time\\Date`"));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->time(string $name): ?Core\Time\TimeOfDay` — a `TIME`, and one of
    /// [`nvs_core_db_row_instant`]'s four.
    fn nvs_core_db_row_time(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "time")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if !crate::instance::is_instance(value, &crate::time::TIME_OF_DAY) {
            return Err(wrong_column_type(
                "time",
                name,
                value,
                "a `Core\\Time\\TimeOfDay`",
            ));
        }
        Ok(owned(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$row->uuid(string $name): ?Core\Uuid` — a native `UUID` column, and the
    /// last of [`nvs_core_db_row_instant`]'s four.
    fn nvs_core_db_row_uuid(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "uuid")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        if !crate::instance::is_instance(value, &crate::uuid::CLASS) {
            return Err(wrong_column_type("uuid", name, value, "a `Core\\Uuid`"));
        }
        Ok(owned(value))
    }
}

/// One of a [`WRITE`]'s three counts, read back out of its slot.
///
/// No reference is taken, unlike [`owned`]'s readers: every one of these slots
/// holds a `uint` or a `null`, and neither owns anything to retain.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot holding any other tag. All three are written
/// by [`nvs_core_db_connection_execute`] and by nothing else, so that is a
/// paste error in this crate rather than anything a program can cause.
fn write_count(args: &[Value], member: &str, at: usize) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &WRITE, member)?;
    let held = crate::instance::slot(receiver, at);
    if held.as_uint().is_none() && held.tag() != Some(Tag::Null) {
        return Err(Fault::fatal(format!(
            "{WRITE_NAME}::{member} found tag {} in its `{}` slot",
            held.tag_byte(),
            WRITE.slots[at]
        )));
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `$write->affected(): uint` — how many rows the statement affected, and
    /// `0` where its kind reports no count at all.
    fn nvs_core_db_write_affected(_ctx, args: [1]) {
        write_count(args, "affected", AFFECTED_AT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$write->changed(): ?uint` — the same count as the server reported it,
    /// whose `null` is what [`nvs_core_db_write_affected`] folds to `0`.
    fn nvs_core_db_write_changed(_ctx, args: [1]) {
        write_count(args, "changed", CHANGED_AT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$write->lastId(): ?uint` — ADR 0067 § 4's id, which on PostgreSQL is
    /// whatever a `RETURNING` clause handed back and belongs to this write
    /// rather than to the connection.
    fn nvs_core_db_write_last_id(_ctx, args: [1]) {
        write_count(args, "lastId", LAST_ID_AT)
    }
}

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
        // `transaction` under this symbol too, which is what ADR 0043's
        // delegation is here — see [`TRANSACTION`].
        "nvs_core_db_connection_transaction" => {
            (nvs_core_db_connection_transaction as *const ()).cast()
        }
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
        "nvs_core_db_write_affected" => (nvs_core_db_write_affected as *const ()).cast(),
        "nvs_core_db_write_changed" => (nvs_core_db_write_changed as *const ()).cast(),
        "nvs_core_db_write_last_id" => (nvs_core_db_write_last_id as *const ()).cast(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::{Ctx, Decimal, OutputSink, call};

    /// ADR 0067 § 13's key for `open` is § 2's memo key, so what that hash
    /// separates is what two requests are refused a shared connection over.
    ///
    /// **Asserted as a sweep that every field moves the key**, rather than as
    /// one pair per field: a key that dropped a field would still answer
    /// plausibly on every other pair, and the two the pool added — the declared
    /// zone and the statement cache's size — are exactly the ones a reader
    /// checking "did it hash the credentials?" would not miss. The count is the
    /// assertion, so a field added to the hash without a case here fails it.
    #[test]
    fn every_settings_field_the_pool_keys_on_moves_the_key() {
        let base = settings_key(&["h", "u", "d", "p", "postgres"], Some(5432), 0, Some(64));
        let variants = [
            settings_key(
                &["other", "u", "d", "p", "postgres"],
                Some(5432),
                0,
                Some(64),
            ),
            settings_key(
                &["h", "other", "d", "p", "postgres"],
                Some(5432),
                0,
                Some(64),
            ),
            settings_key(
                &["h", "u", "other", "p", "postgres"],
                Some(5432),
                0,
                Some(64),
            ),
            settings_key(
                &["h", "u", "d", "other", "postgres"],
                Some(5432),
                0,
                Some(64),
            ),
            settings_key(&["h", "u", "d", "p", "mysql"], Some(5432), 0, Some(64)),
            settings_key(&["h", "u", "d", "p", "postgres"], Some(3306), 0, Some(64)),
            settings_key(&["h", "u", "d", "p", "postgres"], None, 0, Some(64)),
            settings_key(
                &["h", "u", "d", "p", "postgres"],
                Some(5432),
                2 * 3600,
                Some(64),
            ),
            settings_key(&["h", "u", "d", "p", "postgres"], Some(5432), 0, Some(32)),
            settings_key(&["h", "u", "d", "p", "postgres"], Some(5432), 0, None),
        ];
        let mut keys: Vec<&str> = variants.iter().map(String::as_str).collect();
        keys.push(&base);
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), variants.len() + 1, "{keys:?}");
        assert_eq!(
            base,
            settings_key(&["h", "u", "d", "p", "postgres"], Some(5432), 0, Some(64))
        );
        // The one property § 2 needs of the spelling: a `connect` key is a
        // block's name and a name is a configuration key, so no settings hash
        // can be one.
        assert!(base.starts_with('\u{0}'));
    }

    /// ADR 0067 § 5's rewrite and § 9's encoding are **one** choice per driver,
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
            let Some((dialect, encode)) = rendering_for(driver) else {
                continue;
            };
            assert_eq!(
                dialect,
                nvs_db::Dialect::of(driver),
                "{driver:?} rewrites in the dialect `Dialect::of` gives it, or in none at all"
            );
            let rendered = encode(Value::bool(true)).expect("`true` renders on every driver");
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
    }

    /// The roster [`rendering_for`] answers for at all, pinned whole — the test
    /// above says nothing about a driver it answers `None` for, and that half is
    /// this module's known gap 2.
    #[test]
    fn only_a_driver_with_a_statement_path_is_bound() {
        let bound: Vec<nvs_db::Driver> = nvs_db::Driver::ALL
            .into_iter()
            .filter(|driver| rendering_for(*driver).is_some())
            .collect();
        assert_eq!(
            bound,
            vec![
                nvs_db::Driver::Postgres,
                nvs_db::Driver::MySql,
                nvs_db::Driver::MariaDb,
                nvs_db::Driver::SqlServer,
            ],
            "this module's known gap 2 names the drivers a statement is written for, and a driver \
             that gains an encoder belongs in both places"
        );
    }

    /// [`driverless`]'s refusal names every driver that reaches the member it is
    /// about, and names no driver that does not.
    ///
    /// **An agreement test rather than a wording one.** Known gap 2's rosters
    /// live in places that cannot see each other — the match arms of
    /// [`queried_rows`], `execute` and `executeMany`, § 5's encoder roster that
    /// the test above pins, and the two `const`s [`driverless`] renders — and
    /// the way it breaks is a roster going stale while the arms grow. What an
    /// operator then reads is "this build cannot do that" about a driver that
    /// just did, which is the one thing that message exists to prevent.
    ///
    /// **Asked of both rosters, because they parted.** [`READING`] gained SQL
    /// Server with [`tds_rows`] and [`BEYOND_READING`] did not, so the driver
    /// the refusal is *about* is SQLite here: it is the one driver outside both,
    /// which is what makes "named exactly when it reaches" askable of every
    /// other driver in one loop.
    #[test]
    fn the_refusal_names_every_driver_that_sends() {
        let block = Value::str(NvsStr::new(b"main"));
        for roster in [READING, BEYOND_READING] {
            let refused = format!(
                "{:?}",
                driverless(QUERY, &block, nvs_db::Driver::Sqlite, roster)
            );
            assert!(
                refused.contains(nvs_db::Driver::Sqlite.display_name()),
                "a refusal an operator can act on names the driver the block resolved to: {refused}"
            );
            for driver in nvs_db::Driver::ALL {
                if driver == nvs_db::Driver::Sqlite {
                    continue;
                }
                assert_eq!(
                    roster.contains(&driver),
                    refused.contains(driver.display_name()),
                    "{driver:?} is named by this refusal exactly when it reaches the member: \
                     {refused}"
                );
            }
        }
        for driver in READING {
            assert!(
                rendering_for(*driver).is_some(),
                "{driver:?} runs a statement, so § 5 has to render one for it"
            );
        }
        assert!(
            BEYOND_READING.iter().all(|driver| READING.contains(driver)),
            "a driver that reaches `execute` or § 7 reaches `query` first"
        );
    }

    /// The two halves of one enum name the same fourteen cases —
    /// [`COLUMN_TYPE`]'s doc is where "the wire one is authoritative" is
    /// written, and this is what keeps the registry half from drifting off it.
    ///
    /// Both directions, because they fail differently: a case
    /// [`column_type_case`] never names is a value a program can match on and
    /// never receive, while a name it produces that [`COLUMN_TYPE`] does not
    /// register is [`column_type_value`]'s `expect` firing on a real query —
    /// the one of the two that reaches a request.
    #[test]
    fn every_column_type_case_is_named() {
        let described: Vec<&'static str> = [
            nvs_db::ColumnType::Int,
            nvs_db::ColumnType::Uint,
            nvs_db::ColumnType::Float,
            nvs_db::ColumnType::Decimal,
            nvs_db::ColumnType::Text,
            nvs_db::ColumnType::Bytes,
            nvs_db::ColumnType::Bool,
            nvs_db::ColumnType::Date,
            nvs_db::ColumnType::Time,
            nvs_db::ColumnType::DateTime,
            nvs_db::ColumnType::Instant,
            nvs_db::ColumnType::Uuid,
            nvs_db::ColumnType::Json,
            nvs_db::ColumnType::Other,
        ]
        .into_iter()
        .map(column_type_case)
        .collect();
        let registered: Vec<&'static str> =
            COLUMN_TYPE.cases.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            described, registered,
            "`nvs_db::ColumnType` and `{COLUMN_TYPE_NAME}` are one enum, in the spec's own \
             order — a case added to either belongs in both, and in the same place"
        );
    }

    /// The same agreement for ADR 0067 § 8's `ErrorKind`, and it fails the same
    /// two ways: a case [`error_kind_case`] never names is a condition a
    /// program can `match` on and never receive, and a name it produces that
    /// [`ERROR_KIND`] does not register is [`error_kind_value`]'s `expect`
    /// firing inside [`statement_failure`] — on the throw path of a real
    /// refusal, which is the one of the two that reaches a request.
    ///
    /// Reading the list off [`EVERY_ERROR_KIND`] is what also guards *that*:
    /// a variant added to the driver's enum and to [`error_kind_case`] but not
    /// to the roster leaves a registered case with nothing describing it, and
    /// the comparison below is where that shows up.
    #[test]
    fn every_db_error_kind_case_is_named() {
        let described: Vec<&'static str> =
            EVERY_ERROR_KIND.into_iter().map(error_kind_case).collect();
        let registered: Vec<&'static str> =
            ERROR_KIND.cases.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            described, registered,
            "`nvs_db::DbErrorKind` and `{ERROR_KIND_NAME}` are one enum, in § 8's own order — a \
             case added to either belongs in both, and in the same place"
        );
    }

    /// § 7's retry reads a kind back out of a thrown `Db\DbError`, so the two
    /// halves of that round trip have to agree for every case — including the
    /// two [`nvs_db::DbErrorKind::is_retryable`] names, which is the only
    /// answer the loop acts on.
    #[test]
    fn a_kind_written_into_a_throw_reads_back_as_itself() {
        for kind in EVERY_ERROR_KIND {
            assert_eq!(
                error_kind_of(error_kind_value(kind)),
                Some(kind),
                "`{}` did not survive the round trip",
                error_kind_case(kind)
            );
        }
        assert_eq!(error_kind_of(Value::null()), None);
        let past_the_end = i64::try_from(EVERY_ERROR_KIND.len()).expect("eleven cases");
        assert_eq!(
            error_kind_of(Value::int(past_the_end)),
            None,
            "one past the last ordinal is no case at all"
        );
    }

    /// § 8's `driverCode` reaches the throw where the server sent one, and is
    /// left unwritten where it did not.
    ///
    /// **Both sides, because the drivers disagree and the slot cannot be
    /// decided by the class.** MySQL words a refusal with a vendor integer
    /// beside its `SQLSTATE`; PostgreSQL's `SQLSTATE` is its only code, so a
    /// slot filled unconditionally would invent one for it — the failure this
    /// asserts against — and a slot never filled loses the other's, which is
    /// what a program hard-coding `1062` reads. `nvs_db::ServerError` owns why
    /// only one of them has it.
    #[test]
    fn a_driver_code_reaches_the_throw_only_where_the_server_sent_one() {
        let block = Value::str(NvsStr::new(b"main"));
        let refusal = |driver_code| {
            std::io::Error::other(nvs_db::ServerError {
                kind: nvs_db::DbErrorKind::UniqueViolation,
                sql_state: String::from("23000"),
                severity: String::from("ERROR"),
                message: String::from("duplicate"),
                constraint: None,
                driver_code,
                backend: "mysql",
            })
        };
        let code_in = |fault| match fault {
            Fault::ThrownWithSlots(ThrownClass::DbError, _, slots) => slots
                .iter()
                .find(|(slot, _)| *slot == nvs_runtime::DRIVER_CODE_SLOT)
                .map(|(_, value)| value.as_int()),
            other => panic!("§ 8 makes a server's refusal a `Db\\DbError`: {other:?}"),
        };

        assert_eq!(
            code_in(statement_failure(QUERY, &block, None, &refusal(Some(1062)))),
            Some(Some(1062)),
            "MySQL's own integer is what an application reads when § 8's kind is \
             not specific enough for it"
        );
        assert_eq!(
            code_in(statement_failure(QUERY, &block, None, &refusal(None))),
            None,
            "and an unwritten slot already reads `null`, which is the whole of \
             what PostgreSQL has to say here"
        );
    }

    /// A described column is the three slots [`COLUMN`] declares, in the order
    /// its readers name — a paste error [`crate::instance::build`]'s arity
    /// assertion cannot catch, since all three are one class's.
    #[test]
    fn a_column_holds_its_label_its_type_and_its_nullability() {
        assert_eq!(COLUMN.slots, [LABEL_SLOT, DECLARED_SLOT, NULLABLE_SLOT]);
        assert_eq!(COLUMN.slot(LABEL_SLOT), LABEL_AT);
        assert_eq!(COLUMN.slot(DECLARED_SLOT), DECLARED_AT);
        assert_eq!(COLUMN.slot(NULLABLE_SLOT), NULLABLE_AT);
        assert_eq!(ROWS.slot(ROWS_COLUMNS_SLOT), ROWS_COLUMNS_AT);
    }

    /// ADR 0067 § 7's two halves, and the second is the one a forwarding body
    /// would pass while still drifting.
    ///
    /// **A transaction is a closure**: [`TRANSACTION_ROW`] takes one
    /// `callable` and answers at *its* `T`, which is what lets a transaction
    /// wrap an existing expression without retyping it. Its consequence is
    /// asserted as an absence — § 7 removes `commit`, `rollBack` and
    /// `inTransaction` from the connection outright, and a pair a program can
    /// leave half-done is exactly what nesting made unnecessary.
    ///
    /// **`Transaction implements Queryable by $connection`** is asserted over
    /// [`CONNECTION`]'s whole roster rather than member by member, so a member
    /// added there fails here until [`TRANSACTION`] carries it: the rows must
    /// be *identical*, symbol included, since ADR 0043's delegation is one
    /// body reached through either handle and a second body would agree on
    /// name and arity on the day it was written and on nothing afterwards.
    /// `stream` and `streamAs` are owed on both and so are outside the sweep
    /// by construction.
    #[test]
    fn a_transaction_is_a_closure_and_transaction_is_a_queryable() {
        assert_eq!(TRANSACTION_ROW.name, "transaction");
        // § 7's `$fn`, and R9's allowance that the closure may declare no
        // parameter at all is why the arity is not sayable in the row.
        assert_eq!(TRANSACTION_ROW.names, ["fn"]);
        assert_eq!(
            format!("{:?}", TRANSACTION_ROW.params),
            format!(
                "{:?}",
                [
                    CoreTy::CallableTo("T"),
                    CoreTy::Options(TRANSACTION_OPTIONS)
                ]
            ),
            "§ 7's `transaction` takes the closure and R2's one trailing bag"
        );
        // § 7's three options, in its order, at its defaults — asserted as the
        // whole bag rather than one lookup each, so an option added without
        // being specified fails here too. The defaults are the half a call site
        // never writes and so the half nothing else would catch: `isolation`
        // absent leaves the server's own level standing, and `retries` at 0 is
        // § 7's argument that a side-effecting closure is not re-run unasked.
        assert_eq!(
            TRANSACTION_OPTIONS
                .iter()
                .map(|option| (option.name, format!("{:?}", option.default)))
                .collect::<Vec<_>>(),
            [
                ("isolation", format!("{:?}", Const::Null)),
                ("readOnly", format!("{:?}", Const::Bool(false))),
                ("retries", format!("{:?}", Const::Uint(0))),
            ]
        );
        assert_eq!(
            format!("{:?}", TRANSACTION_ROW.return_ty),
            format!("{:?}", CoreTy::Var("T")),
            "§ 7's `: T` — the member's answer is the closure's own"
        );

        // "no `commit()`, no `rollBack()` on the connection and no
        // `inTransaction()`": nesting removed the reason each existed, so
        // their absence is the decision rather than an unlanded row.
        for member in CONNECTION.members() {
            assert!(
                !matches!(
                    member.name,
                    "begin" | "commit" | "rollBack" | "savepoint" | "inTransaction"
                ),
                "`{}` declares `{}` — § 7 replaced that surface with a closure",
                CONNECTION.name,
                member.name
            );
        }

        let declared: Vec<String> = CONNECTION
            .instance
            .iter()
            .map(|row| format!("{row:?}"))
            .collect();
        let forwarded: Vec<String> = TRANSACTION
            .instance
            .iter()
            .filter(|row| CONNECTION.instance.iter().any(|had| had.name == row.name))
            .map(|row| format!("{row:?}"))
            .collect();
        assert_eq!(
            declared, forwarded,
            "`{}` is `{}`'s query surface delegated, not restated: every row \
             agrees down to its symbol and its position",
            TRANSACTION.name, CONNECTION.name
        );

        // What the delegation is allowed to add, in full: § 7's own hazard,
        // and it is on the transaction because that is what holds the flag.
        let own: Vec<&str> = TRANSACTION
            .instance
            .iter()
            .map(|row| row.name)
            .filter(|name| !CONNECTION.instance.iter().any(|had| had.name == *name))
            .collect();
        assert_eq!(own, ["rollBack"]);

        // Load-bearing rather than tidy, per [`TRANSACTION`]'s own doc: one
        // statement path reads either handle, which is what makes the shared
        // symbols above reachable at all.
        assert_eq!(
            &TRANSACTION.slots[..CONNECTION.slots.len()],
            CONNECTION.slots
        );
    }

    /// ADR 0067 § 7's second hazard, asserted at the seam where Doctrine's
    /// `setRollbackOnly()` loses: the decision is on the receiver, so throwing
    /// it away does not undo it.
    ///
    /// **Dropping the [`Fault`] is what an intervening `catch (Throwable)`
    /// does**, and this case does exactly that between the raise and the read
    /// — [`nvs_core_db_transaction_roll_back`] records the reason in
    /// [`REASON_SLOT`] *and* throws, and
    /// [`nvs_core_db_connection_transaction`] reads that slot rather than the
    /// exception it did not catch. A member that only threw would pass every
    /// assertion here but the last one.
    ///
    /// **The read happens after the scope is closed**, which is the order the
    /// owning frame runs in: the `$tx` is refusing further work by then, and
    /// the decision it holds still has to be legible. Both sides are named,
    /// since a transaction nobody abandoned reads back as the same `null` a
    /// broken recording would.
    #[test]
    fn roll_back_survives_an_intervening_catch_of_throwable() {
        let scope = crate::instance::build(
            &TRANSACTION,
            [
                Value::uint(1),
                Value::str(NvsStr::new(b"main")),
                Value::bool(true),
                Value::null(),
            ],
        );
        let receiver =
            crate::instance::receiver(scope, &TRANSACTION, "rollBack").expect("a built instance");
        assert!(
            crate::instance::slot(receiver, REASON_AT)
                .as_text()
                .is_none(),
            "a transaction nobody abandoned carries no reason, which is the \
             `commit` half of the same read"
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let reason = Value::str(NvsStr::new(b"the cart is gone"));
        if call(
            nvs_core_db_transaction_roll_back,
            &mut ctx,
            &[scope, reason],
        )
        .is_ok()
        {
            panic!("§ 7 gives `rollBack` a `void` return because it always throws");
        }
        assert_eq!(
            ctx.pending_class().as_deref(),
            Some(r"Core\Db\RolledBack"),
            "§ 7's signal is the class it names, not a bare `RuntimeError`"
        );
        assert_eq!(ctx.pending().as_deref(), Some("the cart is gone"));

        // The catch, and this is the whole of one: clearing the pending
        // exception is what a `catch (Throwable)` frame does. Everything below
        // runs in the state `setRollbackOnly()` cannot recover from.
        assert!(ctx.take_pending().is_some());
        assert!(ctx.pending().is_none());

        crate::instance::set_slot(receiver, SCOPE_AT, Value::bool(false));
        assert_eq!(
            crate::instance::slot(receiver, REASON_AT).as_text(),
            Some("the cart is gone"),
            "the flag outlives both the exception and the scope — it is what \
             the owning frame rolls back on"
        );

        // And the scope guard is the first hazard, still closed: a `$tx` the
        // closure stored cannot abandon a transaction that has moved on.
        assert!(matches!(
            transaction_of(scope, "rollBack"),
            Err(Fault::Thrown(ThrownClass::Logic, _))
        ));

        discard(reason);
        discard(scope);
    }

    /// [`Attempts`] with no server behind it: every command succeeds, and the
    /// case reads back the order they were asked in.
    ///
    /// Nothing here is scripted to *fail*, because the conflict § 7 retries on
    /// arrives from the closure rather than from the connection — the half no
    /// driver could induce, and the whole reason [`transacted`] takes its
    /// connection as an argument. The depth is 0 so every attempt is
    /// outermost, which is one of the four conditions the retry rule reads.
    struct Scripted {
        /// Every command [`transacted`] asked for, in order.
        asked: Vec<&'static str>,
    }

    impl Scripted {
        /// One command, recorded and answered with a span a trace could file.
        fn ran(
            &mut self,
            command: &'static str,
        ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
            self.asked.push(command);
            Ok(Ok(nvs_db::QuerySpan::opened(
                nvs_db::Driver::Postgres,
                command,
            )))
        }
    }

    impl Attempts for Scripted {
        fn depth(&mut self, _ctx: &mut Ctx) -> Result<u32, Fault> {
            Ok(0)
        }

        fn begin(
            &mut self,
            _ctx: &mut Ctx,
            _isolation: Option<nvs_db::Isolation>,
            _read_only: bool,
        ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
            self.ran("BEGIN")
        }

        fn commit(&mut self, _ctx: &mut Ctx) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
            self.ran("COMMIT")
        }

        fn roll_back(
            &mut self,
            _ctx: &mut Ctx,
        ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
            self.ran("ROLLBACK")
        }
    }

    thread_local! {
        /// How many times [`conflicts_once`] has been entered on this thread.
        ///
        /// A thread local rather than a field on [`Scripted`]: the callback is
        /// reached through an `extern "C"` address and has no `self` to read.
        static ATTEMPTS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }

    /// § 7's closure as a deadlock makes it behave: the first attempt throws a
    /// retryable `Core\Db\DbError`, and every one after it returns.
    ///
    /// **The throw is built by [`statement_failure`] off a real
    /// [`nvs_db::ServerError`]** rather than assembled here, so what the loop
    /// reads back through [`nvs_runtime::Ctx::pending_slot`] is the object a
    /// PostgreSQL `40P01` actually produces —
    /// `a_kind_written_into_a_throw_reads_back_as_itself` holds the other half
    /// of that round trip.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes exactly these two live values, each \
                  retained for this callee to release, and `run_helper` \
                  discharges the rest of the helper ABI's pointer contract"
    )]
    unsafe extern "C" fn conflicts_once(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe {
            nvs_runtime::run_helper(ctx, args, 2, out, |_ctx, args| {
                // The exit sweep a compiled callee owes: the receiver and the
                // one declared parameter, both retained on the way in.
                for slot in args {
                    discard(*slot);
                }
                if ATTEMPTS.with(|entered| entered.replace(entered.get() + 1)) > 0 {
                    return Ok(Value::int(2));
                }
                let block = Value::str(NvsStr::new(b"main"));
                let deadlocked = statement_failure(
                    TRANSACTION_MEMBER,
                    &block,
                    None,
                    &std::io::Error::other(nvs_db::ServerError {
                        kind: nvs_db::DbErrorKind::Deadlock,
                        sql_state: String::from("40P01"),
                        severity: String::from("ERROR"),
                        message: String::from("deadlock detected"),
                        constraint: None,
                        driver_code: None,
                        backend: "postgresql",
                    }),
                );
                discard(block);
                Err(deadlocked)
            })
        }
    }

    /// A `callable` whose `invoke` is `invoke` and which declares one
    /// parameter — the `$tx` § 7 hands its closure.
    ///
    /// `nvs_runtime::call_closure` reads exactly two things off a closure
    /// value, so this is a whole one: the arity in its own slot, and the
    /// address in the class's `CLOSURE_INVOKE` row. The table is leaked
    /// because a descriptor's *address* is its identity and it must outlive
    /// every instance made from it, which is `crate::instance`'s own rule; the
    /// test process exiting is what reclaims it.
    fn closure_of(invoke: nvs_runtime::NvsFn) -> Value {
        let mut table = nvs_runtime::ClassTable::new();
        let id = table.define("{closure}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
                code: invoke as *const u8,
                // Read off the object's own slots below rather than off this
                // row — see `nvs_runtime::MethodRow`.
                arity: 0,
                param_tags: 0,
                public: true,
                native: false,
            }],
        );
        let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
        object.set_field(nvs_runtime::CLOSURE_ARITY_SLOT, Value::int(1));
        object.set_field(
            nvs_runtime::CLOSURE_PARAM_TAGS_SLOT,
            Value::int(i64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY)),
        );
        Value::object(object)
    }

    /// ADR 0067 § 7's `{retries: n}`, at the seam that belongs to no driver: a
    /// deadlock inside the closure re-runs it, and the second attempt's answer
    /// is the call's.
    ///
    /// **The connection is handed in rather than filed**, which is what
    /// [`Attempts`] exists for. [`transacting`] downcasts to a real
    /// `nvs_db::PgConn` and no `-p nvs-stdlib` test can build one, so until
    /// [`transacted`] took its connection as an argument the retry rule — the
    /// one half of § 7 a driver is never asked to implement — had no caller a
    /// test could reach.
    ///
    /// **The conflict is induced the way a server induces one**: the closure
    /// throws what [`statement_failure`] renders a `40P01` into, so the loop's
    /// decision goes through [`nvs_runtime::Ctx::pending_slot`] and § 8's
    /// normalised kind exactly as it does in a request.
    ///
    /// **Counting the attempts is not enough, so the commands are read back in
    /// order.** A loop that re-ran the closure but left the aborted attempt
    /// open, or that opened no second `BEGIN`, would pass a count alone. The
    /// pending failure is asserted *gone* for the same reason: a retry the
    /// caller is never told about must leave nothing for the next member to
    /// trip over.
    ///
    /// **Both sides of the bound**, since a loop that always retried would
    /// pass the first half — § 7's default is 0, and at 0 the same conflict
    /// reaches the caller with the closure run once.
    #[test]
    fn retries_recover_an_induced_deadlock() {
        let block = Value::str(NvsStr::new(b"main"));
        let closure = closure_of(conflicts_once);
        let attempted = |retries| Attempted {
            key: 1,
            block,
            closure,
            isolation: None,
            read_only: false,
            retries,
        };

        // § 8's class has to be *resolvable* or the retry cannot happen at
        // all: `pending_slot` answers `None` for a context with no exception
        // class installed, so a loop reading the kind off the throw would see
        // no conflict and re-raise. Spec § 10's root shape and the one
        // subclass this case throws, arriving the one way a context takes a
        // table — the playbook's `Ctx::class_desc` bullet.
        const THROWABLE: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = nvs_runtime::ClassTable::new();
        let root = classes.define("RuntimeError", &THROWABLE, &[]);
        classes.define(
            ThrownClass::DbError.name(),
            &[
                "message",
                "previous",
                "backtrace",
                "location",
                "kind",
                "sqlState",
                "driverCode",
                "constraint",
                "sql",
            ],
            &[root],
        );

        ATTEMPTS.with(|entered| entered.set(0));
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(nvs_runtime::ErrorClass::new(
            std::rc::Rc::new(classes),
            root,
        ));
        let mut recovered = Scripted { asked: Vec::new() };
        let answered = transacted(&mut ctx, &mut recovered, &attempted(1))
            .expect("§ 7 re-runs a deadlocked closure, and the second attempt commits");
        assert_eq!(
            answered.as_int(),
            Some(2),
            "the call answers the *retried* attempt's return, not the conflict"
        );
        assert_eq!(
            ATTEMPTS.with(std::cell::Cell::get),
            2,
            "one attempt per `BEGIN`, and § 7 spends the one retry it was given"
        );
        assert_eq!(
            recovered.asked,
            ["BEGIN", "ROLLBACK", "BEGIN", "COMMIT"],
            "each attempt gets its own `BEGIN`, and the aborted one is undone \
             before the next opens"
        );
        assert!(
            ctx.pending().is_none(),
            "the retry is this frame's decision that the throw did not happen, \
             so nothing may be left pending for the next member"
        );

        ATTEMPTS.with(|entered| entered.set(0));
        let mut refused = Scripted { asked: Vec::new() };
        let raised = transacted(&mut ctx, &mut refused, &attempted(0))
            .expect_err("§ 7's default is 0, and a conflict with no attempt left is the caller's");
        assert!(
            matches!(raised, Fault::Pending(_)),
            "the closure's own failure travels on unchanged: {raised:?}"
        );
        assert_eq!(
            ctx.pending_class().as_deref(),
            Some(r"Core\Db\DbError"),
            "§ 8's class, still pending exactly as the closure left it"
        );
        assert_eq!(ATTEMPTS.with(std::cell::Cell::get), 1);
        assert_eq!(
            refused.asked,
            ["BEGIN", "ROLLBACK"],
            "the attempt is still rolled back — what 0 removes is the re-run, \
             not the undo"
        );

        drop(ctx.take_thrown());
        discard(closure);
        discard(block);
    }

    /// ADR 0067 § 7's backoff, asserted as bounds over the whole ladder rather
    /// than as numbers: the draw is random, so there is no value to name.
    ///
    /// **Both halves are asserted because either alone passes for the wrong
    /// reason.** A `retry_backoff` answering its ceiling every time grows and
    /// caps correctly and loses the entire point — two requests that
    /// deadlocked against each other are handed one next instant and collide
    /// again — while one drawing over a fixed range is jittered and retries a
    /// hopeless conflict at the same rate forever.
    #[test]
    fn the_backoff_is_exponential_jittered_and_capped() {
        for taken in 0..24_u32 {
            let ceiling = RETRY_BACKOFF
                .saturating_mul(1_u32 << taken.min(16))
                .min(RETRY_BACKOFF_CAP);
            for _ in 0..64 {
                let drawn = retry_backoff(taken);
                assert!(
                    drawn <= ceiling,
                    "rung {taken} drew past the ceiling full jitter is uniform under"
                );
                assert!(
                    drawn <= RETRY_BACKOFF_CAP,
                    "rung {taken} drew past the cap a `uint` of retries is bounded by"
                );
            }
        }

        let drawn: std::collections::BTreeSet<_> = (0..64).map(|_| retry_backoff(4)).collect();
        assert!(drawn.len() > 1, "an unjittered backoff draws one value");

        // The ceiling doubles per rung, which no single draw shows, so it is
        // asserted on the maximum of enough of them to reach past the rung
        // below: 256 draws all landing in rung 0's tenth of rung 3's range is
        // not a flake anyone will see.
        let low = (0..256).map(|_| retry_backoff(0)).max().expect("256 draws");
        let high = (0..256).map(|_| retry_backoff(3)).max().expect("256 draws");
        assert!(high > low, "rung 3's range did not reach past rung 0's");
    }

    /// ADR 0067 § 2's memoization, asserted where it is written: a second
    /// `Core\Db::connect("main")` in one request answers the connection the
    /// first one opened, and performs no handshake of its own.
    ///
    /// **The first call is stood in for by its own last statement.** What sits
    /// between a `connect` and its memo entry is a socket, a TLS session and a
    /// login — a server, which no `-p nvs-stdlib` test has — and this case is
    /// about what the *second* call does. So the context starts where the
    /// first call leaves it: one connection filed under `main`, by the
    /// [`nvs_runtime::Ctx::hold_open_connection`] that is the line
    /// [`open_named`] ends on.
    ///
    /// **"No handshake" is asserted by making one impossible.** This context
    /// carries no configuration at all, so every path past the memo refuses
    /// before it can read a `[db.main]` block, let alone open a socket. The
    /// shared call answering a key is therefore proof it returned at
    /// [`nvs_runtime::Ctx::memoized_connection`] and nowhere later, and the
    /// `{shared: false}` call reaching that refusal on the same context is the
    /// other half of the same evidence — § 2's opt-out is charged again, as
    /// [`CONNECT_DOC`] tells a caller it is.
    ///
    /// **One connection, counted rather than read off the key.** A memoized
    /// call that answered the right key and *also* filed a connection would
    /// pass every assertion above while leaving the request holding two, so
    /// the next name filed has to land at the slot after the first.
    #[test]
    fn a_named_connection_is_memoized_for_the_request() {
        /// A connection that has already been opened, which is the whole of
        /// what this case needs one to be: the memo answers with a key and
        /// never asks what the key holds.
        #[derive(Debug)]
        struct Opened;

        impl nvs_runtime::HeldConnection for Opened {
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }

        // The default the two bare calls in § 2's sentence are made under:
        // memoization is what a program gets for writing no option at all, and
        // `{shared: false}` is the only way out of it.
        let connect = CLASS
            .methods
            .iter()
            .find(|row| row.name == "connect")
            .expect("spec § 18's `connect` is this class's own row");
        let Some(CoreTy::Options(options)) = connect.params.last() else {
            panic!("R2's one trailing options bag is `connect`'s last parameter")
        };
        let shared = options
            .iter()
            .find(|option| option.name == "shared")
            .expect("§ 2's opt-out is an option of that bag");
        assert_eq!(
            format!("{:?}", shared.default),
            format!("{:?}", Const::Bool(true)),
            "a `connect` that writes no option is a shared one, which is what \
             makes the pair below the ordinary case rather than an opt-in"
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let first = ctx.hold_open_connection(Some("main".to_owned()), None, Box::new(Opened));

        let second = open_named(&mut ctx, "main", true, None, CONNECT)
            .expect("§ 2's memo answers before anything reads a configuration");
        assert_eq!(
            first, second,
            "two `connect(\"main\")` calls in one request name one connection"
        );

        let refused = open_named(&mut ctx, "main", false, None, CONNECT).expect_err(
            "`{shared: false}` bypasses the memo, and there is no `[db.main]` behind it",
        );
        let Fault::Thrown(ThrownClass::Runtime, message) = refused else {
            panic!("a name with no configuration behind it refuses as a plain `RuntimeError`")
        };
        assert!(
            message.contains("no configuration at all"),
            "the unshared call got past the memo and refused for want of a \
             block — which is what the shared one did not do: {message}"
        );

        let other = ctx.hold_open_connection(Some("reports".to_owned()), None, Box::new(Opened));
        assert_eq!(
            other,
            first + 1,
            "the memoized call filed nothing, so the next name takes the very \
             next slot — one connection, not two under one key"
        );
        assert_eq!(
            ctx.memoized_connection("main"),
            Some(first),
            "and the name still resolves to the connection the first call opened"
        );
    }

    /// ADR 0067 § 3's asymmetry, asserted as the **contrast** it is: the very
    /// loopback address ADR 0058 § 3's door refuses is the address
    /// `Core\Db::connect` opens to, on a deployment that grants `db.connect`
    /// and writes nothing under `[capabilities.net]` at all.
    ///
    /// On its own, "`connect` reached `127.0.0.1`" is not the claim — an
    /// address nobody denies prints the same line. So the door is asked first,
    /// twice, and both of its refusals are pinned: with no `net.connect` the
    /// host never reaches the range table, and with `net.connect` but no
    /// `net.internal` the range table is what refuses. Only then is
    /// [`address_of`] asked, and it answers.
    ///
    /// The deepest half is a signature rather than an assertion, and is worth
    /// saying because no `assert!` can reach it: [`address_of`] takes no
    /// [`Ctx`], so there is no configuration in front of that path to consult.
    /// The pre-approval is structural, and adding a check to it later would
    /// have to change what the function is handed first.
    #[test]
    fn a_connect_named_private_endpoint_needs_no_net_connect_grant() {
        const HOST: &str = "127.0.0.1";

        // What the operator wrote: one block may be opened by name, and this
        // deployment reaches no host and excepts no address.
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_config(crate::tests::granting(
            "[capabilities.db]\nconnect = [\"main\"]\n",
        ));

        // § 3's two capabilities answer different questions: the one this path
        // asks, and the neighbour it does not.
        nvs_runtime::capability::require(
            &ctx,
            nvs_config::Cap::DbConnect,
            nvs_config::capability::Scope::Name("main"),
            CONNECT,
        )
        .expect("`db.connect` grants the block by name, and this deployment granted it");
        let ungranted = nvs_runtime::capability::require(
            &ctx,
            nvs_config::Cap::NetConnect,
            nvs_config::capability::Scope::Host(HOST),
            CONNECT,
        )
        .expect_err("and nothing here grants `net.connect` for any host at all");

        // ADR 0058's door on that same deployment, refusing the host before it
        // is resolved — the first of the two ways a database on loopback would
        // be unreachable if a named endpoint went through it.
        let by_door = nvs_runtime::capability::pin_host(&ctx, HOST, CONNECT)
            .expect_err("the door asks `net.connect` first — ADR 0058 § 3");
        assert_eq!(
            format!("{by_door:?}"),
            format!("{ungranted:?}"),
            "the door's first question is the capability's own, unchanged"
        );

        // And the second way: buy the host back, and § 3's range table is what
        // refuses. That is the check ADR 0067 § 3 says a named endpoint is not
        // additionally put through — where every database on `10/8`, a
        // container network or loopback lives.
        let mut reachable = Ctx::new(OutputSink::Sink);
        reachable.set_config(crate::tests::granting(
            "[capabilities.db]\nconnect = [\"main\"]\n\
             [capabilities.net]\nconnect = [\"127.0.0.1\"]\n",
        ));
        let by_range = nvs_runtime::capability::pin_host(&reachable, HOST, CONNECT)
            .expect_err("loopback is the first range § 3 denies");
        assert!(
            format!("{by_range:?}").contains("net.internal"),
            "the range half names the key that would except it: {by_range:?}"
        );

        // The path `Core\Db::connect` actually takes, on the deployment that
        // granted neither `net` key: it answers the address both refusals above
        // just named.
        let pinned = address_of(HOST, Some(5432), nvs_db::pg::DEFAULT_PORT, "main")
            .expect("a `connect`-named endpoint is pre-approved — ADR 0067 § 3");
        assert_eq!(
            pinned,
            SocketAddr::from(([127, 0, 0, 1], 5432)),
            "and it is the written host's own address, resolved once"
        );
    }

    /// ADR 0067 § 3's other side, and the same address: a target
    /// `Core\Db::open` was *granted* is still refused when it resolves into one
    /// of ADR 0058 § 3's denied ranges, because a program-supplied address
    /// stays subject to that policy in full.
    ///
    /// The claim is the pair, not either half. `db.open` granting the host is
    /// asserted first, so the refusal that follows cannot be read as an
    /// ungranted one; and [`address_of`] — the path `connect` takes to the very
    /// same address — is asked last and answers, so the refusal cannot be read
    /// as "this runtime will not open loopback". What separates them is which
    /// authority wrote the endpoint, and that is the whole of § 3.
    ///
    /// The deepest half is again a signature: `pinned_address` takes the
    /// [`Ctx`], so the policy has a deployment's `net.internal` in front of it,
    /// where [`address_of`] has nothing in front of it at all.
    #[test]
    fn a_db_open_target_in_a_denied_range_fails() {
        const HOST: &str = "127.0.0.1";

        // What the operator wrote: this program may open a database at that
        // host, and nothing excepts any address from § 3's table.
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_config(crate::tests::granting(
            "[capabilities.db]\nopen = [\"127.0.0.1\"]\n",
        ));

        nvs_runtime::capability::require(
            &ctx,
            nvs_config::Cap::DbOpen,
            nvs_config::capability::Scope::Host(HOST),
            OPEN,
        )
        .expect("`db.open` grants the host by name, and this deployment granted it");

        let by_range = nvs_runtime::capability::pinned_address(&ctx, HOST, OPEN)
            .expect_err("a granted host is not a permitted address — ADR 0067 § 3");
        let said = format!("{by_range:?}");
        assert!(
            // `::open` and not [`OPEN`] itself: this is the `Debug` rendering
            // and it escapes the class's own backslash.
            said.contains("net.internal") && said.contains("::open"),
            "the refusal names the key that would except the range, and the member that \
             asked: {said}"
        );

        // And the same address down `connect`'s path, on a deployment that
        // grants no `net` key either: pre-approved, because an operator wrote
        // the endpoint into root-owned configuration.
        let pinned = address_of(HOST, Some(5432), nvs_db::pg::DEFAULT_PORT, "main")
            .expect("a `connect`-named endpoint is pre-approved — ADR 0067 § 3");
        assert_eq!(
            pinned,
            SocketAddr::from(([127, 0, 0, 1], 5432)),
            "so the two members differ in the check and not in the address"
        );
    }

    /// A block that writes no `port` reaches **its own** driver's port — ADR
    /// 0067 § 2, where the `driver` field is what the rest of the block is read
    /// against.
    ///
    /// The claim is the disagreement rather than either number: [`address_of`]
    /// cannot see the block, so a default read off one driver would send a
    /// `mysql` block's handshake to 5432 and fail as a connection refused,
    /// which reads like a server that is down and not like a bug here. Both
    /// constants are asked for by the same expression the branch in
    /// [`open_named`] uses, so a third driver landing with a port of its own
    /// cannot quietly inherit one of these two.
    #[test]
    fn a_block_with_no_port_reaches_its_own_drivers_default() {
        const HOST: &str = "127.0.0.1";

        assert_ne!(
            nvs_db::pg::DEFAULT_PORT,
            nvs_db::mysql::DEFAULT_PORT,
            "the two servers do not listen in the same place, which is why the port is a \
             parameter at all"
        );
        for (default, expected) in [
            (nvs_db::pg::DEFAULT_PORT, 5432),
            (nvs_db::mysql::DEFAULT_PORT, 3306),
        ] {
            let pinned = address_of(HOST, None, default, "main")
                .expect("a literal host resolves to itself with no name service at all");
            assert_eq!(
                pinned,
                SocketAddr::from(([127, 0, 0, 1], expected)),
                "a block writing no `port` means the port its driver listens on"
            );
        }
    }

    /// One [`Requested`] answer as a word, so a case below reads as the sentence
    /// ADR 0067 § 6 writes rather than as a `match` arm.
    fn answered<T: std::fmt::Debug>(requested: &Requested<T>) -> String {
        match requested {
            Requested::Is(value) => format!("{value:?}"),
            Requested::Lossy(holds) => format!("throws — {holds}"),
            Requested::Mismatched => "throws — not that family".to_owned(),
        }
    }

    /// [ADR 0067](../../../docs/adr/0067-core-db.md) § 6's first named crossing:
    /// **`TINYINT(1)` is naturally `int` and reads as `bool` on request, with a
    /// stored `7` throwing.**
    ///
    /// Both halves of that sentence, because either alone is a member that
    /// looks right. A reader that refused the crossing outright would be
    /// correct about the `7` and unusable against MySQL, which has no boolean
    /// column for § 9 to map — `nvs_db::mysql`'s own type-map test pins
    /// `MYSQL_TYPE_TINY` as [`nvs_db::ColumnType::Int`] for that reason. A
    /// reader that took PHP's cast instead would be usable and would read a
    /// `7` as `true`, which is the lossy conversion § 6 exists to refuse.
    ///
    /// The bound is asserted on both sides at once: `0` and `1` are the last
    /// accepted values and `2` is the first refused one, with `-1` the other
    /// end — a member stopping one entry early prints plausibly against either
    /// half alone. `7` is § 6's own number and is in here under its own name.
    ///
    /// Asked of [`requested_bool`] rather than of `$row->bool()`, because that
    /// function is where the rule lives *and* is what `queryAs<T>`'s `bool`
    /// field reaches through [`converted`]: the two surfaces are asserted to
    /// agree below rather than tested twice.
    #[test]
    fn tinyint_one_reads_int_and_bool_and_throws_for_a_stored_seven() {
        // § 9 first: a `TINYINT(1)` is an `int` column, so the value a row
        // holds for one is an `int` and `->int()` reads it unchanged.
        for stored in [0_i64, 1, 7, -1] {
            let held = Value::int(stored);
            assert_eq!(
                answered(&requested_int(held)),
                format!("{stored}"),
                "§ 9 gives `TINYINT(1)` the `int` row, whatever it stores"
            );
        }

        // § 6 second: the request converts, and only where it is lossless.
        for (stored, expected) in [
            (0_i64, "false"),
            (1, "true"),
            (7, "throws — 7, which is neither `0` nor `1`"),
            (2, "throws — 2, which is neither `0` nor `1`"),
            (-1, "throws — -1, which is neither `0` nor `1`"),
        ] {
            assert_eq!(
                answered(&requested_bool(Value::int(stored))),
                expected,
                "a `TINYINT(1)` storing {stored}, read as `bool`"
            );
            // The same column on a server that declared it `UNSIGNED`, which is
            // § 9's `uint` row and the same question.
            if let Ok(unsigned) = u64::try_from(stored) {
                assert_eq!(
                    answered(&requested_bool(Value::uint(unsigned))),
                    expected,
                    "an unsigned `TINYINT(1)` storing {stored}, read as `bool`"
                );
            }
        }

        // And a real `BOOLEAN`/`BIT(1)`, which needs no crossing at all.
        for flag in [false, true] {
            assert_eq!(
                answered(&requested_bool(Value::bool(flag))),
                format!("{flag}")
            );
        }

        // The two surfaces § 6 states the rule for once: a `Db\Row` reader and
        // a `#[Db\Derive]` field. A field is checked here through `converted`,
        // which is the whole of what `queryAs<T>` asks; that they route through
        // one function is the assertion, since a second copy would agree on the
        // day it was written and on nothing afterwards.
        assert!(
            matches!(
                converted(nvs_runtime::CodecTy::Bool, None, None, Value::int(1)),
                Ok(value) if value.as_bool() == Some(true)
            ),
            "a `bool` field over a `TINYINT(1)` holding 1 hydrates"
        );
        let refused = converted(nvs_runtime::CodecTy::Bool, None, None, Value::int(7))
            .expect_err("a `bool` field over a stored 7 is § 6's refusal");
        assert!(
            refused.contains("7, which is neither `0` nor `1`"),
            "the field quotes the reader's own wording: {refused}"
        );
    }

    /// § 6's second named crossing: **a `BIGINT UNSIGNED` past `i64::MAX` reads
    /// as `uint` and throws for `int`.**
    ///
    /// The bound on both sides, at the one value where it falls: `i64::MAX`
    /// itself crosses and `i64::MAX + 1` does not. A driver reading the column
    /// through PHP's `int` loses that value to a `float` and stops comparing
    /// equal to itself, which is the defect § 9's `uint` row exists for — and a
    /// range check written one off would pass every test that named only the
    /// obvious `u64::MAX`.
    ///
    /// The other direction is the same rule and is asserted beside it, since
    /// `int` and `uint` are ADR 0007 § 4's one integer read two ways: a
    /// negative `BIGINT` has no `uint` reading, and `0` is the bound there.
    #[test]
    fn bigint_unsigned_past_i64_max_reads_uint_and_throws_for_int() {
        /// Where `int` stops and `uint` keeps going, which is the one value
        /// this bound falls at.
        const CEILING: u64 = i64::MAX.cast_unsigned();

        for (stored, as_uint, as_int) in [
            (0_u64, "0", "0"),
            (CEILING, "9223372036854775807", "9223372036854775807"),
            (
                CEILING + 1,
                "9223372036854775808",
                "throws — 9223372036854775808, which is past `int`'s ceiling",
            ),
            (
                u64::MAX,
                "18446744073709551615",
                "throws — 18446744073709551615, which is past `int`'s ceiling",
            ),
        ] {
            let held = Value::uint(stored);
            assert_eq!(
                answered(&requested_uint(held)),
                as_uint,
                "a `BIGINT UNSIGNED` holding {stored} is `uint`'s own row"
            );
            assert_eq!(
                answered(&requested_int(held)),
                as_int,
                "the same column asked for `int`"
            );
        }

        for (stored, as_uint) in [
            (0_i64, "0"),
            (-1, "throws — -1, which is below `uint`'s floor"),
            (
                i64::MIN,
                "throws — -9223372036854775808, which is below `uint`'s floor",
            ),
        ] {
            assert_eq!(
                answered(&requested_uint(Value::int(stored))),
                as_uint,
                "a signed `BIGINT` holding {stored}, asked for `uint`"
            );
        }

        // Neither reading is a way into a column of another family: § 6's
        // crossings are between `int` and `uint` and nowhere else.
        for held in [
            Value::float(1.0),
            Value::decimal(Decimal::parse("1").expect("`1` is a decimal")),
            Value::bool(true),
        ] {
            assert!(
                matches!(requested_int(held), Requested::Mismatched),
                "{held:?} is not an integer column"
            );
            assert!(matches!(requested_uint(held), Requested::Mismatched));
        }
    }

    /// § 6's third named crossing, which is the one that is not a crossing: **a
    /// `DECIMAL` refuses a `float` field**, since
    /// [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md) keeps the two
    /// apart.
    ///
    /// This is the ADR's own § *Context* defect at the hydration boundary. PDO
    /// hands a `DECIMAL` back as a string on every driver it has, and the PHP
    /// code that follows compares a price with `==` and gets away with it until
    /// a value stops surviving the `float` it is silently coerced through. So
    /// the refusal is asserted at a value where the widening is *invisible* —
    /// `0.1` has no exact `float` and `1` has one — because a check written
    /// against a value that already fails to round-trip would pass over a
    /// codec that widened whenever it could.
    ///
    /// Both directions, since ADR 0054 keeps them apart in both: a `FLOAT`
    /// column has no `decimal` field either, and `->decimal()` is the reader
    /// the exact column has.
    #[test]
    fn a_decimal_into_a_float_field_throws() {
        use nvs_runtime::CodecTy;

        for text in ["1", "0.1", "-12345678901234567890.12", "0"] {
            let exact = Decimal::parse(text).expect("a decimal literal");
            let refused = converted(CodecTy::Float, None, None, Value::decimal(exact))
                .expect_err("a `float` field over a `DECIMAL` is refused whatever it holds");
            assert!(
                refused.contains("`float`"),
                "the refusal names the field's declared type: {refused}"
            );

            // The column's own field type still hydrates it, so what is being
            // pinned is the crossing and not the column.
            assert!(
                converted(CodecTy::Mixed, None, None, Value::decimal(exact)).is_ok(),
                "`mixed` takes whatever the column held"
            );
        }

        // The other side of ADR 0054's wall, and the reason this is a
        // `Mismatched` rather than a range: there is no `DECIMAL` a `float`
        // field takes and no `FLOAT` a `decimal` field takes, at any value.
        let refused = converted(CodecTy::Float, None, None, Value::int(1))
            .expect_err("§ 6 has no int-widens-to-float crossing either");
        assert!(refused.contains("`float`"), "{refused}");
    }

    /// One [`nvs_runtime::CodecField`], with the six properties this file's
    /// cases never vary spelled once.
    fn codec_field(key: &str, param: usize, ty: nvs_runtime::CodecTy) -> nvs_runtime::CodecField {
        nvs_runtime::CodecField {
            key: key.to_owned(),
            slot: param,
            param,
            ty,
            element: None,
            class: None,
            cases: None,
            nullable: false,
        }
    }

    /// [ADR 0067](../../../docs/adr/0067-core-db.md) § 6's refusal for
    /// `queryAs<T>`: a wrong type, a missing column or a NULL in a field
    /// declared non-nullable throws **naming every offending column, not the
    /// first**.
    ///
    /// The three conditions are named in one sentence of the ADR and they reach
    /// [`hydrate`] by three different routes — a value the field's declared type
    /// refuses, a key the row has no entry for at all, and a `Tag::Null` that
    /// only a `?T` field takes — so a codec that accumulated on one route and
    /// returned early on another passes any case that asks about one of them.
    /// Asked here as a **count**: three fields are wrong and three issues come
    /// back, which is the assertion a per-condition case cannot make.
    ///
    /// Naming the column is the item rather than a nicety. § 6 has field names
    /// match column names exactly and `AS` as the way to rename, so at a table
    /// of forty columns the path is the only thing separating "one of these did
    /// not match" from a fix — [ADR 0071 § 5](../../../docs/adr/0071-derived-codecs.md)
    /// is where the `issues` list this reads back is specified, and the throw is
    /// a `ParseError` for the reason [`hydrate`]'s own docs give.
    #[test]
    fn query_as_throws_naming_the_column_for_a_mismatch_a_missing_column_and_a_null() {
        use nvs_runtime::CodecTy;

        // A descriptor is identified by its address, so the table outlives the
        // test rather than being moved — `allocation_policy.rs`'s `closure_of`
        // is the same shape and the same reason.
        let table: &'static mut nvs_runtime::ClassTable =
            Box::leak(Box::new(nvs_runtime::ClassTable::new()));
        let id = table.define("Account", &["id", "name", "at"], &[]);
        table.set_db_codec(
            id,
            vec![
                codec_field("id", 0, CodecTy::Int),
                codec_field("name", 1, CodecTy::Str),
                codec_field("at", 2, CodecTy::Str),
            ],
            3,
            vec![std::ptr::null(); 3],
        );
        let class = table.desc(id);

        // The row three of whose columns are wrong in three different ways, and
        // the fourth — `id` is present and is a string where the field declares
        // `int`; `name` is absent outright; `at` is SQL NULL against a field
        // that is not `?T`. Nothing here is right, which is the point: a codec
        // reporting the first would answer one of the three.
        let mut row = NvsArray::new();
        row.set(NvsStr::new(b"id"), Value::str(NvsStr::new(b"7")));
        row.set(NvsStr::new(b"at"), Value::null());

        #[expect(unsafe_code, reason = "the leaked table keeps the descriptor live")]
        let refused = unsafe {
            hydrate(&mut Ctx::new(OutputSink::Sink), class, &row).expect_err(
                "a row with three offending columns is § 6's throw and never reaches `new`",
            )
        };

        let Fault::ThrownWithSlots(ThrownClass::Parse, message, slots) = refused else {
            panic!("§ 6's mismatch is ADR 0071 § 5's `ParseError` carrying `issues`")
        };
        assert!(
            message.contains("3 column(s) of `Account`"),
            "the summary counts what the list carries: {message}"
        );

        let [(slot, issues)] = *slots else {
            panic!("one slot, and it is `issues`")
        };
        assert_eq!(slot, nvs_runtime::ISSUES_SLOT);
        let list = crate::arr::borrowed(issues.array_ptr().expect("`issues` is an `array<Issue>`"));
        assert_eq!(
            list.count(),
            3,
            "every offending column at once, which is what a form needs to \
             report all four bad fields rather than the first"
        );

        // ADR 0071 § 5's `path` is the column, and the order is the field
        // declaration order — read off `crate::issue::FIELDS`' slot order
        // rather than guessed, since that agreement is the one that would fail
        // silently.
        let paths: Vec<String> = (0..3)
            .map(|index| {
                let issue = list
                    .get_index(index)
                    .expect("every position of the list holds an issue");
                let object = issue.obj_ptr().expect("an issue is a shape value");
                let path = crate::instance::slot(object, 1);
                String::from_utf8_lossy(path.as_str_bytes().expect("`path` is a string"))
                    .into_owned()
            })
            .collect();
        assert_eq!(paths, ["id", "name", "at"]);

        #[expect(
            unsafe_code,
            reason = "this frame owns the reference the throw handed over"
        )]
        unsafe {
            issues.release();
        }
    }
}
