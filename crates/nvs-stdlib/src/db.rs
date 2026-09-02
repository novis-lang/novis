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
//! 1. **`open` is not here, and what is missing is a type rather than a body.**
//!    § 18 writes `open(Db\Settings $settings, {shared?: bool})`, and
//!    `Db\Settings` is a *discriminated union of two shapes* over ADR 0047's
//!    enum-case types — the SQLite arm has a `path` and no `host`. The registry
//!    has no [`CoreTy`] for a shape **parameter** at all: `CoreTy::Options` is
//!    a trailing bag, flattened to one ABI argument per option and optional by
//!    construction, and no row in this crate has ever declared a fixed-key
//!    shape argument. So `open` is blocked on a registry type rather than on
//!    anything about databases, and adding one decides how every future shape
//!    parameter is passed — which is a language-surface question and not this
//!    module's to answer in passing.
//! 2. **Two drivers open, and only PostgreSQL runs a statement.** `connect`
//!    branches on the block's `driver` — ADR 0067 § 2 — so a `postgres` block
//!    and a `mysql` block each reach their own target, their own default port
//!    and their own `nvs_db::Connection` variant. A block naming any of the
//!    other three is still refused by `nvs_db::PgTarget::resolve` with the
//!    message that names the driver it is, which is the honest answer while
//!    those variants have no connect path behind them. Past the handshake the
//!    list is shorter than that: [`postgres_of`] is what a statement goes
//!    through, so a MySQL connection opens, pools and resets, and refuses
//!    every member that would run something on it. § 9's own half of that path
//!    is already here — [`mysql_column_value`] and [`mysql_described_columns`]
//!    read a MySQL row and its description — and both carry an
//!    `#[expect(dead_code)]` that the branch calling them takes off by itself.
//! 3. **Only the two drivers that open are pooled.** ADR 0067 § 13's pool is
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
//!    `driverCode` is
//!    declared and stays `null` on PostgreSQL, whose `SQLSTATE` is its only
//!    code. A failure of the *wire* rather than of the
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
//! 9. **`transaction` re-runs the closure with no wait between attempts**, and
//!    that is a deliberate narrowing of § 7 rather than an omission. Everything
//!    else the rule needs is here — [`ISOLATION`] and `readOnly` reach the
//!    `BEGIN`, [`nvs_db::PgConn::depth`] says which call is the outermost one,
//!    and [`nvs_db::DbErrorKind::is_retryable`] names the two failures that may
//!    be re-run, that being the half only a driver can answer. What is missing
//!    is the *backoff*: § 7 specifies exponential backoff with jitter that
//!    **suspends the coroutine**, and `nvs-runtime`'s own known gap 3 is that a
//!    helper cannot suspend yet.
//!
//!    Of the two ways to wait without a yielder, neither is § 7's and the
//!    cheaper one is no wait at all. Sleeping on the core is out — that is the
//!    one thing § 7 forbids. Sleeping on `nvs_host::blocking` keeps the core
//!    free but parks a pool worker for the whole backoff, and a retry storm is
//!    by definition many requests waiting at once: the conflicts that make
//!    retries fire are correlated, so the pool every other request needs for
//!    real blocking work — a `Core\Process` wait, a file read — is drained by
//!    calls doing nothing. That is priority 3 paid across the whole core to buy
//!    one request's politeness, and it is the bargain § 7 explicitly did not
//!    strike. Retrying immediately spends nothing shared: PostgreSQL reports a
//!    deadlock or a serialization failure only once it has already resolved the
//!    conflict, so the second attempt is not spinning against a lock still
//!    held. What it loses is the de-correlation the jitter bought, and the
//!    bound on that is `retries` itself, which defaults to 0.
//!
//!    **A conflict raised by a statement *inside* the closure is retried on the
//!    same four conditions**, and only the shape it arrives in differs. The
//!    commit's refusal is an `io::Error` carrying its own
//!    [`nvs_db::ServerError`]; the closure's is a pending exception on the
//!    context, so its kind is read back off that object's
//!    [`nvs_runtime::KIND_SLOT`] and turned into a
//!    [`nvs_db::DbErrorKind`] by [`error_kind_of`]. Both then ask
//!    [`nvs_db::DbErrorKind::is_retryable`], which is the one place the rule
//!    lives.

use std::net::{SocketAddr, ToSocketAddrs as _};

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc, ErrorDoc,
    MethodDoc, ParamDoc, Qual,
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

/// Spec § 18's `Core\Db` — `connect` and the two connectionless entry points,
/// in the spec's own order. `open` joins them once a shape *parameter* is
/// expressible in this registry; this module's known gaps own that.
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
/// a conflict the driver says may be re-run — with no wait between attempts,
/// since § 7's backoff suspends the coroutine and `nvs-runtime` has no yielder.
/// This module's known gap 9 is that narrowing and why it is the safe half of
/// it; the declared default of 0 is what every call that does not ask gets.
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
            desc: "The host does not resolve, or the connection, the TLS handshake or the login \
                   itself failed. A refusal the server worded carries its own message.",
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
    short: "One column as `bool` — `BOOLEAN` and `BIT(1)`. MySQL's and MariaDB's `TINYINT(1)` is \
            naturally an `int` and is read by `->int`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The truth value, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not boolean — a `0`/`1` \
               integer is not silently one.",
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
    let lease = match nvs_runtime::pool::admit(ticket.clone()) {
        Some(lease) => lease,
        None => wait_for_slot(ctx, ticket, name, deadline)?,
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
                // The raw code rides beside the kind normalised from it, so an
                // application that § 8's eleven conditions do not cover reads
                // what the server actually said without the driver having to
                // widen that enum. `driverCode` is left `null` here rather than
                // filled with the `SQLSTATE` again — `nvs_db::ServerError` owns
                // why PostgreSQL has no second code.
                Some(server) => {
                    let mut slots = vec![
                        (nvs_runtime::KIND_SLOT, error_kind_value(server.kind)),
                        (
                            nvs_runtime::SQL_STATE_SLOT,
                            Value::str(NvsStr::new(server.sql_state.as_bytes())),
                        ),
                    ];
                    // `constraint` joins them only where the condition named
                    // one, which most conditions do not. An unwritten slot
                    // already reads `null`, so the absent case costs no value
                    // here and no branch in the program that reads it — the
                    // same reason `driverCode` above is written nowhere at all.
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
/// Both spellings of the member's name are passed because two things want
/// different ones: `member` is the bare name [`connection_of`] builds
/// `Class::member` out of, and `named` is the whole spelling the messages here
/// already hold a class in. The argument slots are read the same way for each
/// member, since § 18 gives both the identical two parameters.
///
/// # Errors
///
/// A thrown `LogicError` for a `$params` keyed both ways at once, and whatever
/// [`statement_failure`] makes of the rewriter's and the encoder's refusals. A
/// [`Fault::fatal`] for an argument of the wrong tag, which the registry row
/// refuses first.
fn statement_of(args: &[Value], member: &str, named: &str) -> Result<Statement, Fault> {
    let (key, block) = handle_of(args[0], member)?;
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
    let rewritten = nvs_db::rewrite(sql, spelling, nvs_db::Dialect::PostgreSql)
        .map_err(|refused| statement_failure(named, &block, Some(sql), &refused))?;

    let bounds: Vec<&Bound> = if keys.is_empty() {
        positional.iter().collect()
    } else {
        keys.iter().map(|(_, bound)| bound).collect()
    };
    let mut rendered: Vec<Option<Vec<u8>>> = Vec::with_capacity(rewritten.binds.len());
    for source in &rewritten.binds {
        rendered.push(
            nvs_db::encode(bounds[source.arg].values[source.element])
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
/// [`statement_of`] throws for any one of them. A [`Fault::fatal`] for an
/// argument of the wrong tag, which the registry row refuses first.
fn batch_of(args: &[Value], member: &str, named: &str) -> Result<Batch, Fault> {
    let (key, block) = handle_of(args[0], member)?;
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
        let one = statement_of(&[args[0], args[1], set], member, named)?;
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
/// **A failed reset destroys the connection.** Both `nvs_db::PgConn::reset` and
/// `nvs_db::MySqlConn::reset` take `self` by value and hand it back only on the
/// path where every one of § 13's commands succeeded, so a connection that
/// could not be proven clean is closed before this returns and there is no
/// shape in which one request reads another's session state. That is also why
/// the caller cannot tell a failed reset from an empty pool: both are `None`,
/// and both mean open a fresh connection, which is what a request did before
/// there was a pool at all.
///
/// **The two resets are not the same reset**, and § 13 says so: PostgreSQL's
/// keeps § 1's statement cache and MySQL's `COM_RESET_CONNECTION` drops it, so
/// the connection each arm hands back is warm in a different amount. Neither is
/// a choice this function makes — each driver's own `reset` is where its
/// section's property is met.
///
/// A connection filed by any other driver is dropped here for the same reason —
/// `nvs_db::Connection`'s other three variants have no reset behind them yet,
/// so they are not poolable and this is the one place that is enforced.
fn warm_connection(lease: &nvs_runtime::pool::Lease) -> Option<nvs_db::Connection> {
    let held = nvs_runtime::pool::take(lease, std::time::Instant::now())?;
    let connection = held.into_any().downcast::<nvs_db::Connection>().ok()?;
    match *connection {
        nvs_db::Connection::Postgres(postgres) => {
            Some(nvs_db::Connection::Postgres(postgres.reset().ok()?))
        }
        nvs_db::Connection::MySql(mysql) => Some(nvs_db::Connection::MySql(mysql.reset().ok()?)),
        _ => None,
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
    name: &str,
    timeout: Option<std::time::Instant>,
) -> Result<nvs_runtime::pool::Lease, Fault> {
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

/// The connection a [`Statement`] or a [`Batch`] names, as the one driver that
/// runs a statement so far.
///
/// The key and the block are passed rather than either of those types, because
/// they are the only two fields it reads and a batch is not a statement — the
/// alternative is a `Statement` built with an empty `binds` purely to reach
/// this, which would be a shape nothing else in this module means.
///
/// # Errors
///
/// A thrown `RuntimeError` for a block naming another driver — this module's
/// known gap 2 — and a [`Fault::fatal`] for a key the request's own table does
/// not hold, which is this crate's paste error rather than a program's.
fn postgres_of<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
    block: &Value,
    named: &str,
) -> Result<&'a mut nvs_db::PgConn, Fault> {
    let filed = ctx.open_connection_mut(key).ok_or_else(|| {
        Fault::fatal(format!(
            "{named}: no connection is filed under the key {key}"
        ))
    })?;
    let connection = filed
        .as_any_mut()
        .downcast_mut::<nvs_db::Connection>()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{named}: the connection filed under the key {key} is not `nvs-db`'s"
            ))
        })?;
    let driver = connection.driver();
    let nvs_db::Connection::Postgres(postgres) = connection else {
        return Err(Fault::thrown(format!(
            "{named}: `[db.{}]` is a {driver:?} connection, and only PostgreSQL runs a \
             statement so far — this module's known gap 2 is the list",
            block.as_text().unwrap_or("?")
        )));
    };
    Ok(postgres)
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
/// [`statement_of`]'s and [`postgres_of`]'s refusals, [`statement_failure`] for
/// anything the server refused, and [`column_value`]'s for a column whose value
/// has no Novis representation.
fn queried_rows(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    member: &str,
    named: &str,
) -> Result<Answered, Fault> {
    let statement = statement_of(args, member, named)?;
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
    let postgres = postgres_of(ctx, statement.key, &statement.block, named)?;
    // Read before the statement borrows the connection, and once for the whole
    // result: § 9's zone-less `TIMESTAMP` is decoded in the zone this
    // connection declared, and that is a property of the connection rather
    // than of the row.
    let zone = postgres.time_zone();
    let mut answered = postgres
        .query(&statement.sql, &sending)
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
    // the rows it actually got, and after the last read of `answered`, which is
    // what ends the borrow on the context.
    let taken = watch.taken(answered.span());
    // Explicit because `PgRows` has a `Drop` — it releases the statement — so
    // its borrow of the context runs to the end of the scope unless the stream
    // is dropped here, and the context is what the event is filed on.
    drop(answered);
    watch.file(ctx, taken);
    Ok(Answered {
        rows,
        columns: described,
    })
}

/// What is reading this statement's span — [ADR 0041](../../../../docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md)
/// § 1's trace, ADR 0067 § 11's `slow_query` line, both or neither — asked
/// **before** a statement borrows the context.
///
/// A statement holds `ctx` mutably for as long as its rows do
/// ([`postgres_of`]), so neither can be read at the point the event is filed.
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
pub(crate) fn name_span(rows: &mut nvs_db::PgRows<'_>, block: Option<&str>) {
    if let Some(name) = block {
        rows.name_connection(name);
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
#[expect(
    dead_code,
    reason = "§ 9's decode is landed ahead of the read path that calls it — `queried_rows` \
              still goes through `postgres_of`, which is this module's known gap 2"
)]
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
#[expect(
    dead_code,
    reason = "§ 9's decode is landed ahead of the read path that calls it — `queried_rows` \
              still goes through `postgres_of`, which is this module's known gap 2"
)]
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
#[expect(
    dead_code,
    reason = "as `mysql_column_value`, whose only caller this is"
)]
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
        let statement = statement_of(args, "execute", EXECUTE)?;
        // As `query`, and for the reason given there: a refusal names the
        // caller's own text rather than the rewrite of it that reached the wire.
        let source = args[1].as_text();
        let sending: Vec<Option<&[u8]>> =
            statement.binds.iter().map(|one| one.as_deref()).collect();
        // As `query`, and for the reason [`QueryWatch`] gives.
        let watch = QueryWatch::of(ctx, &statement.block);
        let postgres = postgres_of(ctx, statement.key, &statement.block, EXECUTE)?;
        let mut answered = postgres
            .query(&statement.sql, &sending)
            .map_err(|refused| statement_failure(EXECUTE, &statement.block, source, &refused))?;
        name_span(&mut answered, statement.block.as_text());
        while answered
            .next_row()
            .map_err(|refused| statement_failure(EXECUTE, &statement.block, source, &refused))?
            .is_some()
        {}

        let changed = answered.affected();
        let last_id = answered.last_id();
        // § 11's event is a *statement's*, not a reader's: a write files one on
        // the same terms as `query`, carrying the affected count `finished`
        // froze on the span above.
        let taken = watch.taken(answered.span());
        // As `query`, and for the same borrow reason given there.
        drop(answered);
        watch.file(ctx, taken);
        Ok(crate::instance::build(
            &WRITE,
            [
                Value::uint(changed.unwrap_or(0)),
                changed.map_or_else(Value::null, Value::uint),
                last_id.map_or_else(Value::null, Value::uint),
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
    /// [`nvs_db::PgConn::execute_many`] answers with a count and lends no
    /// `PgRows` out, so there is no handle a driver-built span could ride on
    /// and be read off afterwards — the span is opened here, around the same
    /// round trips, and finished with the batch's sum as its affected count and
    /// no rows at all, which is what a batch contributes to a trace.
    /// `nvs_db::QuerySpan` owns the field set and why a bound value is not in
    /// it, and `Ctx::record_query` owns why what crosses is a rendering.
    ///
    /// **What it answers is a `uint` and not a [`WRITE`].** § 4 gives the batch
    /// a sum, because `changed` and `lastId` would each have to pick one
    /// execution to be about — and the sum is what a caller writing the loop by
    /// hand would have accumulated anyway. The batch is also **not** a
    /// transaction: each execution carries its own `Sync`
    /// ([`nvs_db::PgConn::execute_many`] is where that is argued), so a failure
    /// part way through leaves the writes before it standing, and `transaction`
    /// is the member that asks for all or nothing.
    fn nvs_core_db_connection_execute_many(ctx, args: [3]) {
        let batch = batch_of(args, "executeMany", EXECUTE_MANY)?;
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
        let postgres = postgres_of(ctx, batch.key, &batch.block, EXECUTE_MANY)?;
        // § 11's span, opened where the driver opens `execute`'s: after the
        // connection is in hand, so the duration is the statement's wait and
        // not the pool's. It carries the rewritten text, which is what reaches
        // the wire and what a driver-opened span would have been handed.
        let mut span = nvs_db::QuerySpan::opened(nvs_db::Driver::Postgres, &batch.sql);
        // One statement over many parameter sets, so the batch has exactly the
        // one text to name and it is the caller's, as `execute`'s is.
        let written = postgres
            .execute_many(&batch.sql, &sets)
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
    /// [`nvs_runtime::Ctx::pending_slot`]. The wait between attempts is what
    /// this module's known gap 9 still holds.
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
        let mut left = args[RETRIES_ARG].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{TRANSACTION_MEMBER} expected a `uint` for `retries`, got tag {}",
                args[RETRIES_ARG].tag_byte()
            ))
        })?;

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
            let outermost = postgres_of(ctx, key, &block, TRANSACTION_MEMBER)?.depth() == 0;
            // § 11's event covers § 7's own commands as well as the statements
            // inside them: a trace that showed the closure's writes but not the
            // `BEGIN` and the `COMMIT` around them would put the transaction's
            // whole cost on its last statement. The driver answers with the
            // span because only it knows whether the depth made this a
            // `SAVEPOINT` — [`nvs_db::PgConn::begin`] owns that.
            let opened = postgres_of(ctx, key, &block, TRANSACTION_MEMBER)?
                .begin(isolation, read_only)
                .map_err(|refused| {
                    statement_failure(TRANSACTION_MEMBER, &block, None, &refused)
                })?;
            file_span(ctx, watch, &block, opened);

            // The block name is handed on rather than looked up again: a
            // transaction refuses under the same `[db.<name>]` its connection
            // does, and the slot is the only place that name lives.
            let scope = crate::instance::build(
                &TRANSACTION,
                [
                    Value::uint(key),
                    owned(block),
                    Value::bool(true),
                    Value::null(),
                ],
            );
            let outcome = nvs_runtime::call_closure(ctx, args[1], &[scope]);

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
                    let undone = match postgres_of(ctx, key, &block, TRANSACTION_MEMBER) {
                        Ok(postgres) => postgres.roll_back().ok(),
                        Err(_) => None,
                    };
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
                    left -= 1;
                    continue;
                }
            };

            // The driver's own error rather than the `Fault` it renders to: the
            // retry rule branches on § 8's kind, which only [`nvs_db`] can put
            // there and only this shape still carries.
            let closed = match postgres_of(ctx, key, &block, TRANSACTION_MEMBER) {
                Ok(postgres) => {
                    if abandoned.is_some() {
                        postgres.roll_back()
                    } else {
                        postgres.commit()
                    }
                }
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
            // rolled back, and a conflict the driver says may be re-run. The
            // wait § 7 also asks for is this module's known gap 9.
            let conflicted = nvs_db::ServerError::of(&refused)
                .is_some_and(|server| server.kind.is_retryable());
            if outermost && left > 0 && abandoned.is_none() && conflicted {
                left -= 1;
                continue;
            }
            return Err(statement_failure(TRANSACTION_MEMBER, &block, None, &refused));
        }
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
        CodecTy::Bool => match held.tag() {
            Some(Tag::Bool) => Ok(held),
            _ => Err(wanted("`bool`", held)),
        },
        CodecTy::Int => match held.as_uint() {
            Some(unsigned) => i64::try_from(unsigned).map(Value::int).map_err(|_| {
                format!("the column holds {unsigned}, which is not an `int` — ADR 0067 § 6")
            }),
            None if held.tag() == Some(Tag::Int) => Ok(held),
            None => Err(wanted("`int`", held)),
        },
        CodecTy::Uint => match held.as_int() {
            Some(signed) => u64::try_from(signed).map(Value::uint).map_err(|_| {
                format!("the column holds {signed}, which is not a `uint` — ADR 0067 § 6")
            }),
            None if held.tag() == Some(Tag::Uint) => Ok(held),
            None => Err(wanted("`uint`", held)),
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
             would not be the same number — ADR 0067 § 6 converts losslessly or throws",
            String::from_utf8_lossy(name)
        ),
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
        if let Some(number) = value.as_int() {
            return Ok(Value::int(number));
        }
        let Some(number) = value.as_uint() else {
            return Err(wrong_column_type("int", name, value, "an integer"));
        };
        i64::try_from(number)
            .map(Value::int)
            .map_err(|_| column_out_of_range("int", name, "a value past `int`'s ceiling"))
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
        if let Some(number) = value.as_uint() {
            return Ok(Value::uint(number));
        }
        let Some(number) = value.as_int() else {
            return Err(wrong_column_type("uint", name, value, "an integer"));
        };
        u64::try_from(number)
            .map(Value::uint)
            .map_err(|_| column_out_of_range("uint", name, "a negative value"))
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
    /// `$row->bool(string $name): ?bool` — `BOOLEAN` and `BIT(1)`.
    fn nvs_core_db_row_bool(_ctx, args: [2]) {
        let (name, found) = typed_column(args, "bool")?;
        let Some(value) = found else {
            return Ok(Value::null());
        };
        value
            .as_bool()
            .map(Value::bool)
            .ok_or_else(|| wrong_column_type("bool", name, value, "`bool`"))
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
    use nvs_runtime::{Ctx, OutputSink, call};

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
}
