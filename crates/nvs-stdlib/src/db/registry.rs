//! What `Core\Db` declares: all but one of its classes, its enums, and a
//! reference card for each of them and for every member.
//!
//! The one left out is `Core\Db\Stream`, which is declared beside its own walk
//! in [`mod@super::stream`] rather than here: it carries no member, and what
//! there is to say about it is the walk.
//!
//! Rows, not behaviour. Every symbol named here is defined by a sibling, and
//! the join between the two is checked when the crate links rather than by
//! anything in this file — [`crate::registry`]'s own doc owns why that is the
//! check. It stays one module because a card sits directly after the row it
//! describes, per [docs/agent/conventions.md](/docs/agent/conventions.md).

use super::*;

/// Spec § 18's `Db\Settings` — the two arms `open` takes, in the spec's own
/// order, per `rule:core-api/shape-parameter`.
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
/// a [`Qual::Sink`] because `rule:core-classes/db-capabilities` makes an address one and gives it no
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
pub(super) const SETTINGS: &[&[CoreField]] = &[
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
            ty: CoreTy::Path(Qual::Sink),
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

/// `Core\Db`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Opens a connection to a database. The name of a connection in the server's \
            configuration is enough, or the program can give the settings itself.",
};

/// `Core\Db\Connection`'s class card — `rule:core-api/reference-card`.
const CONNECTION_CARD: ClassDoc = ClassDoc {
    short: "An open connection to one database. It runs SQL queries and statements, and it \
            starts transactions.",
};

/// `Core\Db\Transaction`'s class card — `rule:core-api/reference-card`.
const TRANSACTION_CARD: ClassDoc = ClassDoc {
    short: "Runs SQL statements inside one transaction. Either all of their changes are saved, \
            or none of them are.",
};

/// `Core\Db\Rows`'s class card — `rule:core-api/reference-card`.
const ROWS_CARD: ClassDoc = ClassDoc {
    short: "The whole result of a query, read into memory at once. A `foreach` loop gives its \
            rows one by one.",
};

/// `Core\Db\Row`'s class card — `rule:core-api/reference-card`.
const ROW_CARD: ClassDoc = ClassDoc {
    short: "One row of a query result. Its methods read a column by name and return it as a \
            given type.",
};

/// `Core\Db\Write`'s class card — `rule:core-api/reference-card`.
const WRITE_CARD: ClassDoc = ClassDoc {
    short: "The result of a statement that changes data: how many rows it changed, and the id \
            of an inserted row.",
};

/// `Core\Db\Column`'s class card — `rule:core-api/reference-card`.
const COLUMN_CARD: ClassDoc = ClassDoc {
    short: "Describes one column of a query result: its name, its type, and whether it can \
            contain `null`.",
};

/// `Core\Db\InList`'s class card — `rule:core-api/reference-card`.
const IN_LIST_CARD: ClassDoc = ClassDoc {
    short: "A list of values for a SQL `IN (...)` condition. You can use it only as a bound \
            parameter of a query.",
};

/// `Core\Db\Schema`'s class card — `rule:core-api/reference-card`.
const SCHEMA_CARD: ClassDoc = ClassDoc {
    short: "Describes the tables a database should have. You can compare it with a real \
            database and apply the changes that make them match.",
};

/// `Core\Db\Plan`'s class card — `rule:core-api/reference-card`.
const PLAN_CARD: ClassDoc = ClassDoc {
    short: "Every difference between a schema and a database, as a list of steps. A program \
            can read the steps before any of them runs.",
};

/// `Core\Db\Plan\Step`'s class card — `rule:core-api/reference-card`.
const STEP_CARD: ClassDoc = ClassDoc {
    short: "One difference in a plan: how risky the change is, the reason for that grade, and \
            the SQL that makes the change.",
};

/// Spec § 18's `Core\Db` — `connect`, `open`, and the two connectionless entry
/// points, in the spec's own order.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
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
                        // off, which is `rule:core-classes/db-connection-is-named`'s `{shared: false}`.
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
                    // `connect`'s default, for `connect`'s reason — `rule:core-classes/db-connection-is-named` memoizes by default and the option only turns it off.
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
/// owed (this module's gap 4), and `stream` and `streamAs` are owed whole.**
/// Beyond the interface, § 18's own three rows land here: `close`, `driver`
/// and `isOpen`, the second and third of which are the two readonly properties
/// a connection can answer without a round trip. `serverVersion` is the third
/// and is owed, because no driver keeps the server's own version string —
/// this module's gap 3 is the inventory.
///
/// **`close` is the only one of these on this class alone.** A
/// [`TRANSACTION`] delegates `Core\Db\Queryable` to its connection and nothing
/// else, and closing the connection out from under the `transaction()` call
/// that is still running is not something § 18 gives a spelling for.
/// `rule:classes/no-traits` makes `Transaction` delegate the interface to its connection, so
/// every one of them is declared once — here — and [`TRANSACTION`] is where the
/// forwarding lands.
/// `rule:core-classes/db-statement-members`'s `{timeout?: Duration}`, the one option every statement member
/// carries.
///
/// One constant rather than ten copies of it: § 4 gives `query`, `queryAs`,
/// `execute`, `executeMany` and `stream` the same bag, and [`TRANSACTION`]
/// declares all five a second time — ten spellings of one option are ten places
/// for a default to drift apart. What the instant reaches is
/// `nvs_db::Connection::set_deadline` — the socket on the four drivers with a
/// wire, and the lock wait on SQLite — so the option means one thing across the
/// roster rather than one thing per backend.
///
/// **`chunk` is deliberately not here.** `stream`'s second option in § 18 has to
/// reach the `Execute` that asks for a row count, and this driver's walk asks for
/// one row; [`crate::db`] § *`stream` declares no `chunk`, and the portal is
/// why* is where that stays recorded.
const STATEMENT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "timeout",
    ty: CoreTy::Instance(crate::time::DURATION_NAME),
    // `connect`'s default and for `connect`'s reason: no `Duration` means
    // "unbounded", so the absence of the option is the absence itself.
    default: Const::Null,
}];

pub(crate) const CONNECTION: CoreClass = CoreClass {
    name: CONNECTION_NAME,
    doc: Some(&CONNECTION_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "query",
            names: &["sql", "params"],
            params: &[
                // § 4's Q column, and `rule:security/sink-predicate`'s whole injection story: the
                // statement text is the sink, so a `tainted` value cannot reach
                // it at all and the bound parameters below accept one freely.
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            // § 18's `Rows<Row>` — [`ROWS`] at the one concrete argument an
            // unhydrated result set has, and `queryAs<T>` is the same class at
            // whatever the call site wrote. A [`CoreTy::InstanceAt`] and not a
            // [`CoreTy::Instance`]: the bare spelling would intern the class at
            // its *own* `T`, a variable no call site of `query` ever binds.
            // Written out on both classes rather than named once, because
            // `bun nv gaps` attributes a case to the class a member answers
            // by reading this very line.
            return_ty: CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Instance(ROW_NAME)]),
            symbol: "nvs_core_db_connection_query",
            doc: Some(&QUERY_DOC),
        },
        CoreMethod {
            name: "queryAs",
            names: &["sql", "params"],
            // `query`'s parameters exactly, bag and all: § 4 makes this the same
            // statement, read the same way and bounded the same way, and the
            // only difference is what each row becomes.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
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
            // The same [`CoreTy`]s `query` above declares, for the same
            // reasons — § 4's Q column marks both members' statement text a sink,
            // and a write is exactly where a `tainted` value most wants to reach
            // one.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
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
                // § 4's bag bounds the *batch*: the sets are one statement run N
                // times over one connection, and there is no per-set answer for
                // a per-set clock to belong to.
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            // A bare `uint` and not a [`WRITE`]: § 4 gives the batch a sum and
            // no second return to hand rows or a `lastId` back through, because
            // there is no one execution for either to belong to.
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_db_connection_execute_many",
            doc: Some(&EXECUTE_MANY_DOC),
        },
        CoreMethod {
            name: "stream",
            names: &["sql", "params"],
            // `query`'s two exactly: § 4 gives this member the same statement
            // and the same binding rule, and the only difference is where the
            // rows are when it answers. § 18's `{timeout?, chunk?: uint}` is
            // here at its `timeout` alone: on this member the deadline stays
            // filed while the portal is open, so it bounds every `advance()` up
            // to the last row rather than the call that opens the walk.
            // [`crate::db`] § *`stream` declares no `chunk`, and the portal is
            // why* keeps that decision — a size that reached no read would be
            // an option that parsed and did nothing.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            // § 18's `Iterable<Db\Row>`, spelled as the class that *is* the
            // walk — [`CoreTy::Iterated`] is parameter position only, so
            // `Core\IO\Lines`' spelling is the one available here. The element
            // rides as that class's own type argument, exactly as it does on
            // the [`ROWS`] the member above answers: `Core\Db\Stream` is one
            // generic walk at two arguments, `Db\Row` here and the call site's
            // own `T` on `streamAs` below, because a second class would be a
            // second iteration protocol saying the same thing.
            return_ty: CoreTy::InstanceAt(STREAM_NAME, &[CoreTy::Instance(ROW_NAME)]),
            symbol: "nvs_core_db_connection_stream",
            doc: Some(&STREAM_DOC),
        },
        CoreMethod {
            name: "streamAs",
            names: &["sql", "params"],
            // `stream`'s parameters exactly, for the reason `queryAs` takes
            // `query`'s: § 4 makes this the same statement over the same
            // portal, bound the same way and bounded by the same `timeout`,
            // and the only difference is what each row becomes.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            // § 18's `Iterable<T>`, which is the walk above at the type its
            // call site wrote. The [`CoreTy::Written`] is what makes the member
            // generic, exactly as `queryAs`'s is: `CoreMethod::written` finds
            // the `T` here and nowhere else, so a call naming no type argument
            // is `E0442`.
            return_ty: CoreTy::InstanceAt(STREAM_NAME, &[CoreTy::Written("T")]),
            symbol: "nvs_core_db_connection_stream_as",
            doc: Some(&STREAM_AS_DOC),
        },
        TRANSACTION_ROW,
        CoreMethod {
            name: "close",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_db_connection_close",
            doc: Some(&CLOSE_DOC),
        },
        CoreMethod {
            name: "driver",
            names: &[],
            params: &[],
            defaults: &[],
            // § 18's `Driver`, which is the same enum a `Core\Db::open`
            // settings literal names first — [`DRIVER`]. A connection is the
            // one place the answer is a fact rather than a request, since a
            // `connect` reads it out of the block the operator wrote.
            return_ty: CoreTy::Enum(DRIVER_NAME),
            symbol: "nvs_core_db_connection_driver",
            doc: Some(&DRIVER_MEMBER_DOC),
        },
        CoreMethod {
            name: "serverVersion",
            names: &[],
            params: &[],
            defaults: &[],
            // Unqualified, as § 18's table writes it and for [`COLUMN`]'s
            // reason: the server described itself during the handshake, before
            // this connection had carried a statement at all, so it is not one
            // of § 9's `tainted` reads — those are the values a request can put
            // bytes into.
            return_ty: CoreTy::Str,
            symbol: "nvs_core_db_connection_server_version",
            doc: Some(&SERVER_VERSION_DOC),
        },
        CoreMethod {
            name: "isOpen",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_db_connection_is_open",
            doc: Some(&IS_OPEN_DOC),
        },
    ],
    slots: &[HANDLE_SLOT, CONNECTION_NAME_SLOT],
    constants: &[],
};

/// The [`CONNECTION`] rows that are **not** `Core\Db\Queryable`'s, and so are
/// not delegated to a [`TRANSACTION`] — spec § 18's second table, which gives
/// `Connection` members "beyond `Queryable`".
///
/// It exists so that the delegation sweep in
/// [`crate::db::transaction`]'s `a_transaction_is_a_callable_and_transaction_is_a_queryable`
/// stays a sweep. That case compares the two rosters row for row, which is what
/// catches a member added to one and not the other; a `close` that is *meant*
/// to be on one alone would have to weaken it to a member-by-member check, so
/// the exception is named here instead and the case asserts this list from both
/// ends — every name on it is a row [`CONNECTION`] has and [`TRANSACTION`] does
/// not. A row that stops being either fails there rather than going stale.
///
/// `stream` and `streamAs` will not join it: § 18 puts both on `Queryable`, so
/// each lands on both classes.
///
/// [`QUERYABLE`] reads its members off the same list: every [`CONNECTION`]
/// row not named here.
pub(super) const BEYOND_QUERYABLE: &[&str] = &["close", "driver", "serverVersion", "isOpen"];

/// `rule:core-classes/db-transactions`' `Core\Db\Queryable`: what a function
/// declares when it runs its statements on a connection and a transaction
/// alike.
///
/// [`CONNECTION`] and [`TRANSACTION`] already carry its members under one
/// symbol each, and `bind::handle_of` is where a helper asks which receiver it was
/// handed. So the interface is that shared roster given a name, and costs
/// nothing at a call.
pub(crate) const QUERYABLE: CoreInterface = CoreInterface::new(
    QUERYABLE_NAME,
    &[CONNECTION_NAME, TRANSACTION_NAME],
    &CONNECTION,
    BEYOND_QUERYABLE,
);

/// `Core\Db\Connection::close`'s reference card — `rule:core-api/reference-card`.
const CLOSE_DOC: MethodDoc = MethodDoc {
    short: "Releases the connection to this core's pool, ahead of the request that opened it. \
            Every other member of this connection then throws; `isOpen` answers `false`, and a \
            second `close` does nothing.",
    params: &[],
    ret: "Nothing. A connection is released for its effect on the pool, and the pool is not \
          something a program holds.",
    errors: &[],
};

/// `Core\Db\Connection::driver`'s reference card — `rule:core-api/reference-card`.
const DRIVER_MEMBER_DOC: MethodDoc = MethodDoc {
    short: "Which backend this connection speaks to, as the `Core\\Db\\Driver` case the \
            `[db.<name>]` block or the `open` settings named.",
    params: &[],
    ret: "The connection's own `Core\\Db\\Driver` case.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The connection has been closed.",
    }],
};

/// `Core\Db\Connection::serverVersion`'s reference card — `rule:core-api/reference-card`.
const SERVER_VERSION_DOC: MethodDoc = MethodDoc {
    short: "The version the other end reported of itself, in its own words: PostgreSQL's \
            `server_version`, MySQL's and MariaDB's greeting banner, `major.minor.build` from SQL \
            Server's login answer, and SQLite's library version. The handshake delivered it, so \
            reading it costs no statement — a `stream` may ask while it holds the connection.",
    params: &[],
    ret: "The version, never empty and never re-worded — a distribution's suffix is part of what \
          is on the other end.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The connection has been closed.",
    }],
};

/// `Core\Db\Connection::isOpen`'s reference card — `rule:core-api/reference-card`.
const IS_OPEN_DOC: MethodDoc = MethodDoc {
    short: "Whether this connection is still usable — `true` until `close`, and `false` after \
            it. It is the one member a closed connection still answers.",
    params: &[],
    ret: "`true` for a connection a statement may still run on, `false` for one `close` has \
          released.",
    errors: &[],
};

/// `rule:core-classes/db-transactions`'s `{isolation?, readOnly?, retries?}` — the bag
/// [`TRANSACTION_ROW`] declares last, per `rule:core-api/shape-rules` R2.
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
pub(super) const TRANSACTION_OPTIONS: &[CoreOption] = &[
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

/// `rule:core-classes/db-transactions`'s `transaction`, written once because it is declared once: the
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
pub(super) const TRANSACTION_ROW: CoreMethod = CoreMethod {
    name: "transaction",
    names: &["fn"],
    // Written, as `rule:types/callable-signature` has every callback spell what
    // it receives: what this one is handed is a [`TRANSACTION`], and a closure
    // declaring no parameter at all still satisfies the row under
    // `rule:types/callable-arity`'s prefix match — which is § 7's R9 allowance,
    // now stated where it is checked rather than left to
    // `nvs_runtime::call_callable`'s trim.
    params: &[
        CoreTy::CallableSig(&[CoreTy::Instance(TRANSACTION_NAME)], &CoreTy::Var("T")),
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
/// under the same symbols**, which is `rule:classes/no-traits`'s
/// delegation with no second body to drift from the first: `query`, `execute`,
/// `executeMany` and `transaction` resolve to [`CONNECTION`]'s helpers, which
/// reach the connection through [`handle_of`] and so accept either receiver.
/// The alternative — four forwarding bodies — is four places for a rule to be
/// stated twice, and `rule:core-api/shape-rules` R17 is the same objection to two spellings of one
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
    doc: Some(&TRANSACTION_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "query",
            names: &["sql", "params"],
            // [`CONNECTION`]'s parameters, bag included and for the reason its
            // symbol is shared: one body reads either handle, so a bag declared
            // on one class and not the other would be one arity at two spellings.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            // § 18's `Rows<Row>` — [`ROWS`] at the one concrete argument an
            // unhydrated result set has, and `queryAs<T>` is the same class at
            // whatever the call site wrote. A [`CoreTy::InstanceAt`] and not a
            // [`CoreTy::Instance`]: the bare spelling would intern the class at
            // its *own* `T`, a variable no call site of `query` ever binds.
            // Written out on both classes rather than named once, because
            // `bun nv gaps` attributes a case to the class a member answers
            // by reading this very line.
            return_ty: CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Instance(ROW_NAME)]),
            symbol: "nvs_core_db_connection_query",
            doc: Some(&QUERY_DOC),
        },
        CoreMethod {
            name: "queryAs",
            names: &["sql", "params"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            // [`CONNECTION`]'s row, written out for the reason its `query`
            // sibling is: this is the class `bun nv gaps` attributes a case
            // to. The symbol is the connection's too — `rule:classes/no-traits`'s delegation
            // is one body reached through either handle, and [`handle_of`] is
            // what reads the two of them the same way.
            return_ty: CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Written("T")]),
            symbol: "nvs_core_db_connection_query_as",
            doc: Some(&QUERY_AS_DOC),
        },
        CoreMethod {
            name: "execute",
            names: &["sql", "params"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
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
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_db_connection_execute_many",
            doc: Some(&EXECUTE_MANY_DOC),
        },
        CoreMethod {
            name: "stream",
            names: &["sql", "params"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            // [`CONNECTION`]'s row under [`CONNECTION`]'s symbol, for its
            // `queryAs` sibling's reason — and § 4's connection-busy rule is
            // what makes the delegation worth stating twice here: a stream
            // opened inside a transaction holds the very connection the
            // `COMMIT` has to go out on, so the `LogicError` a second statement
            // meets is the same one either receiver produces.
            return_ty: CoreTy::InstanceAt(STREAM_NAME, &[CoreTy::Instance(ROW_NAME)]),
            symbol: "nvs_core_db_connection_stream",
            doc: Some(&STREAM_DOC),
        },
        CoreMethod {
            name: "streamAs",
            names: &["sql", "params"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Mixed),
                CoreTy::Options(STATEMENT_OPTIONS),
            ],
            defaults: &[],
            // The row above's delegation at a written type, and the second
            // reason it is spelled out here rather than inherited:
            // `crate::registry::WRITTEN_CLASS_MEMBERS` is keyed by the
            // *declaring* class, so a `$tx->streamAs<Person>(…)` that this row
            // did not exist for would lose the class its call site wrote.
            return_ty: CoreTy::InstanceAt(STREAM_NAME, &[CoreTy::Written("T")]),
            symbol: "nvs_core_db_connection_stream_as",
            doc: Some(&STREAM_AS_DOC),
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

/// Spec § 18's `Driver` — `rule:core-classes/db-one-api`'s five backends, as the registry half of
/// [`nvs_db::Driver`].
///
/// **The two halves are one enum and the wire one is authoritative.** This
/// table is what a program writes into a `Db\Settings` literal;
/// `nvs_db::Driver` is what a connection reports and what
/// `nvs_db::Driver::from_config_name` reads a `[db.<name>]` block's `driver`
/// as. Nothing about a backend is decided here.
///
/// **The cases are what make `Db\Settings` a discriminated union**, and they
/// are the whole of the mechanism: `rule:core-api/shape-arms-are-disjoint` selects an arm by asking which
/// one accepts the literal, and `rule:types/single-value-types`'s enum-case types make
/// `Driver::Sqlite` and the other four disjoint sets. No field is declared to
/// be a discriminant, here or anywhere.
///
/// **The values are declaration ordinals and mean nothing else.** They are
/// § 18's own order, so `MySql` is 0 and `SqlServer` is 4, and they are not a
/// rank — writing them out rather than leaning on `rule:enums/declaration`'s
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

/// [`DRIVER`]'s reference card — `rule:core-api/reference-card`.
pub(super) const DRIVER_DOC: EnumDoc = EnumDoc {
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
/// implements**, which is `rule:core-classes/db-capabilities`'s third closed hole: PHP's `pdo_pgsql`
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

/// [`TLS`]'s reference card — `rule:core-api/reference-card`.
pub(super) const TLS_DOC: EnumDoc = EnumDoc {
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

/// Spec § 18's `Isolation` — `rule:core-classes/db-transactions`'s five levels, as the registry half
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
/// Writing them out rather than leaning on `rule:enums/declaration`'s auto-increment is
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

/// [`ISOLATION`]'s reference card — `rule:core-api/reference-card`.
pub(super) const ISOLATION_DOC: EnumDoc = EnumDoc {
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

/// `rule:core-classes/db-error`'s `ErrorKind` — the
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
/// [0063 § 4](/docs/decisions/0063.md) fixes, for
/// boundaries that are driver-dependent anyway. What normalising does not
/// reach stays readable as the raw `sqlState`, `constraint` and `driverCode`
/// beside it.
///
/// **The values are declaration ordinals and mean nothing else.** They are
/// § 8's own order, so `UniqueViolation` is 0 and `Other` is 10, but they are
/// not a rank a program may compare: a kind is a set and not a scale, which is
/// why `nvs_db::DbErrorKind` derives no `Ord` either. Writing them out rather
/// than leaning on `rule:enums/declaration`'s auto-increment is [`CoreEnum::cases`]' rule
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

/// [`ERROR_KIND`]'s reference card — `rule:core-api/reference-card`.
pub(super) const ERROR_KIND_DOC: EnumDoc = EnumDoc {
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
/// case of its own although `rule:core-classes/db-column-types`
/// decodes a `JSON` column to the same `tainted string` a `TEXT` one decodes
/// to, and why there is no array case at all — § 9 reads a PostgreSQL array as
/// `array<T>` and MySQL's `SET` as `array<string>`, and both *describe* as
/// `Other`. [`ROWS`] records the same split from the reader's side.
///
/// **The values are declaration ordinals and mean nothing else.** They are the
/// spec's own order at `docs/spec/01-core-library.md:1223`, so `Int` is 0 and
/// `Other` is 13, but they are not a rank a program may compare: these are a
/// set and not a scale, which is why `nvs_db::ColumnType` derives no `Ord`
/// either. Writing them out rather than leaning on `rule:enums/declaration`'s
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

/// [`COLUMN_TYPE`]'s reference card — `rule:core-api/reference-card`.
pub(super) const COLUMN_TYPE_DOC: EnumDoc = EnumDoc {
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
/// `rule:iteration/foreach-subjects` takes exactly three subjects and this is the second of them
/// rather than a fourth, so `foreach ($rows as Row $row)` and `all()` are one
/// walk over one array of rows: neither copies what the other already holds.
///
/// **Buffered is `rule:core-classes/db-statement-members`'s default and this is what it spends**: a result
/// set is held whole, per request, and the connection is free the moment
/// `query` returns. § 4 chose that over the alternative because
/// `rule:programs/memory-priority` ranks memory
/// last and because a cursor breaks the commonest loop in web programming on a
/// connection-busy rule; `stream` is the member for a result set that does not
/// fit, and it is the one that holds the connection.
pub(crate) const ROWS: CoreClass = CoreClass {
    name: ROWS_NAME,
    doc: Some(&ROWS_CARD),
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
/// `rule:core-classes/db-column-types` puts in place of `FETCH_ASSOC`, `FETCH_NUM` and `FETCH_OBJ`.
///
/// **The three orderings PHP makes a fetch mode of are one shape here.** A row
/// is a string-keyed array of its columns and nothing else, so there is no
/// numeric twin to ask for and no object twin either: `get`/`toArray` are the
/// associative reading, the typed readers below are what an object reading was
/// wanted for, and `queryAs<T>` — `rule:core-classes/derive-attribute`'s `#[Db\Derive]` — is where a real
/// class comes from. A fetch-mode argument would be `rule:core-api/shape-rules`
/// R11's flag deciding what a member returns, which is the thing that section
/// removes.
///
/// **The eleven typed readers convert losslessly or throw, and the rule is one
/// sentence: a reader answers its own tag, and `int`/`uint` are the single
/// crossing** — `rule:types/arithmetic`
/// makes those two views of one integer, so a `BIGINT` read as `uint` is the
/// same value and a negative one throws rather than wrapping. Everything else
/// refuses: `->float` on a `NUMERIC` is not the rounding PHP does silently, and
/// `->string` on a `BYTEA` is not the re-interpretation
/// `rule:types/bytes` keeps apart. The
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
    doc: Some(&ROW_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "has",
            names: &["name"],
            // § 18's own annotation on this row, and what every other name
            // parameter in this class carries too: a column name is a lookup
            // key, so what comes back carries the qualifiers `rule:core-classes/db-column-types` gives
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
/// with, and the whole of what `rule:core-classes/db-statement-members` puts in place of `rowCount` on a
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
    doc: Some(&WRITE_CARD),
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
/// and the whole of what `rule:core-classes/db-one-api` puts in place of `getColumnMeta` and
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
/// differ per driver, which is the shape `rule:core-api/shape-rules`
/// R11 removes.
pub(crate) const COLUMN: CoreClass = CoreClass {
    name: COLUMN_NAME,
    doc: Some(&COLUMN_CARD),
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
    doc: Some(&IN_LIST_CARD),
    methods: &[],
    instance: &[],
    slots: &[VALUES_SLOT],
    constants: &[],
};

/// `rule:core-classes/schema-is-a-value`'s `Core\Db\Schema` — a database schema as a value, in the array form
/// that is its canonical spelling.
///
/// **Two members here and no builders.** § 1 gives a schema three
/// interchangeable spellings and no privileged one, and the array form is the
/// one the other two are defined against — so `fromArray` is the door and
/// `toArray` is the window, and a typed builder surface would be a second way
/// to say what this one already says. § 9's `planAgainst`, `applySafe` and
/// `applyIncludingRisky` are the members that do something with the value; they
/// need a connection and this class needs none.
pub(crate) const SCHEMA: CoreClass = CoreClass {
    name: SCHEMA_NAME,
    doc: Some(&SCHEMA_CARD),
    methods: &[CoreMethod {
        name: "fromArray",
        names: &["array"],
        params: &[CoreTy::Array(&CoreTy::Mixed)],
        defaults: &[],
        return_ty: CoreTy::Instance(SCHEMA_NAME),
        symbol: "nvs_core_db_schema_from_array",
        doc: Some(&SCHEMA_FROM_ARRAY_DOC),
    }],
    instance: &[
        CoreMethod {
            name: "toArray",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Mixed),
            symbol: "nvs_core_db_schema_to_array",
            doc: Some(&SCHEMA_TO_ARRAY_DOC),
        },
        CoreMethod {
            name: "planAgainst",
            names: &["connection"],
            params: &[CoreTy::Instance(CONNECTION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(PLAN_NAME),
            symbol: "nvs_core_db_schema_plan_against",
            doc: Some(&SCHEMA_PLAN_AGAINST_DOC),
        },
        CoreMethod {
            name: "applySafe",
            names: &["connection"],
            params: &[CoreTy::Instance(CONNECTION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_db_schema_apply_safe",
            doc: Some(&SCHEMA_APPLY_SAFE_DOC),
        },
        CoreMethod {
            name: "applyIncludingRisky",
            names: &["connection"],
            params: &[CoreTy::Instance(CONNECTION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_db_schema_apply_including_risky",
            doc: Some(&SCHEMA_APPLY_RISKY_DOC),
        },
    ],
    slots: &[SCHEMA_ARRAY_SLOT],
    constants: &[],
};

/// `Core\Db\Schema::fromArray`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_FROM_ARRAY_DOC: MethodDoc = MethodDoc {
    short: "Reads a schema out of its canonical array form — the same form `toArray` writes, a \
            file holds and `nvs schema dump` prints. Every rule the vocabulary has is checked \
            here, so a schema value that exists is one all five backends can be asked for.",
    params: &[ParamDoc {
        name: "array",
        desc: "The schema, as `[\"tables\" => [...]]`. A table is `name`, `columns`, and \
               optionally `primary_key`, `unique` and `indexes`; a column is `name` and `type`, \
               with `null`, `identity` and a one-key `default` where it has them. A key that is \
               left out is the empty list or `false`.",
        shape: &[],
    }],
    ret: "A `Core\\Db\\Schema` holding the **normalized** form: tables in name order, columns in \
          declaration order, every optional key filled in. So `toArray` answers the same array \
          for every spelling of one schema, which is what makes two schemas comparable.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "A key is missing or holds the wrong kind of value, a type spelling is outside the \
               vocabulary, an identifier is not a plain identifier, or the schema reads but is \
               not coherent — a table with no columns, a second identity column, an identity \
               column outside the primary key, or an index over a column that is not there.",
    }],
};

/// `Core\Db\Schema::toArray`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_TO_ARRAY_DOC: MethodDoc = MethodDoc {
    short: "The schema in its canonical array form — what a program saves to a file, hands to \
            `Core\\Json::encode`, or compares against another schema.",
    params: &[],
    ret: "The array `fromArray` would read back as the same schema. Tables come in name order and \
          columns in the order they were declared, since a `create table` reproduces it; \
          constraints and indexes come in name order, since nothing observable depends on the \
          order they were added in.",
    errors: &[],
};

/// `Core\Db\Schema::planAgainst`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_PLAN_AGAINST_DOC: MethodDoc = MethodDoc {
    short: "Reads the database this connection reaches and answers every difference between it and \
            this schema, in the order the differences must be closed. A plan is a document: \
            nothing is changed by computing one, and an empty plan is what convergence looks like.",
    params: &[ParamDoc {
        name: "connection",
        desc: "The database to compare against. Planning is an ordinary read and needs only the \
               `db.connect` this connection was opened with.",
        shape: &[],
    }],
    ret: "The plan, whose `steps()` are graded `Safe`, `Locking` or `Destructive` and each carry \
          the complete SQL that makes them. A table or column the database has and this schema \
          does not is a **report**: it is in the plan, with the SQL that would remove it, and no \
          `apply` will ever run it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The database holds a type, a default or an identifier the schema vocabulary cannot \
               name, so no plan against it would be total.",
    }],
};

/// `Core\Db\Schema::applySafe`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_APPLY_SAFE_DOC: MethodDoc = MethodDoc {
    short: "Plans against this connection and runs the plan, provided every step it would run is \
            graded `Safe`. Needs the `db.schema` capability for the connection's block: issuing \
            DDL is a privileged act and reaching the database is not enough on its own.",
    params: &[ParamDoc {
        name: "connection",
        desc: "The database to converge. Its `[db.<name>]` block is the name `db.schema` is \
               granted for, so a connection from `Core\\Db::open` cannot be applied to.",
        shape: &[],
    }],
    ret: "Nothing. The database matches the schema afterwards, apart from what the schema does \
          not declare — which is reported and left alone.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The plan holds a step that is not `Safe`, named in the message; the connection \
                   has no block to grant `db.schema` for; or `planAgainst`'s own refusal.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The `db.schema` capability is not granted for this connection's block.",
        },
    ],
};

/// `Core\Db\Schema::applyIncludingRisky`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_APPLY_RISKY_DOC: MethodDoc = MethodDoc {
    short: "`applySafe`, without the grade check: runs every step of the plan including the ones \
            that can hold a long lock, rewrite a table or fail on rows that already exist. Named \
            so that a reviewer reading the call site sees the claim being made.",
    params: &[ParamDoc {
        name: "connection",
        desc: "The database to converge, as `applySafe` takes it and under the same `db.schema` \
               grant.",
        shape: &[],
    }],
    ret: "Nothing. A reported drop is still not run — accepting risk is not accepting data loss, \
          and the SQL for one is on the step for a program that wants it.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The connection has no block to grant `db.schema` for, or `planAgainst`'s own \
                   refusal.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The `db.schema` capability is not granted for this connection's block.",
        },
    ],
};

/// `rule:core-classes/schema-plan`'s plan — every difference between a schema value and a database, as a
/// document a program walks.
///
/// **One member, because a plan is a list and nothing else.** Counting by
/// grade, finding the first refusal and rendering the document are all things a
/// program writes over `steps()` in three lines, and a member for each would be
/// a surface that has to be kept agreeing with a `foreach` anyone can write.
/// [`mod@super::plan`]'s module doc owns what a step holds and why it is copied
/// out rather than computed on demand.
pub(crate) const PLAN: CoreClass = CoreClass {
    name: PLAN_NAME,
    doc: Some(&PLAN_CARD),
    methods: &[],
    instance: &[CoreMethod {
        name: "steps",
        names: &[],
        params: &[],
        defaults: &[],
        return_ty: CoreTy::Array(&CoreTy::Instance(STEP_NAME)),
        symbol: "nvs_core_db_plan_steps",
        doc: Some(&PLAN_STEPS_DOC),
    }],
    slots: &[PLAN_STEPS_SLOT],
    constants: &[],
};

/// `Core\Db\Plan::steps`'s reference card — `rule:core-api/reference-card`.
const PLAN_STEPS_DOC: MethodDoc = MethodDoc {
    short: "Every step of the plan, in the order they must run, reports included.",
    params: &[],
    ret: "The steps. An empty array is convergence: the database already matches the schema. A \
          step whose `isRefused()` is `true` is one no `apply` will run.",
    errors: &[],
};

/// `rule:core-classes/schema-plan`'s step — one difference, its grade, the sentence explaining the
/// grade, and the SQL that makes it.
///
/// **Four readers over four slots and no `change()`.** § 2's vocabulary is
/// closed and a [`nvs_db::Change`] is closed with it, so exposing the change
/// itself would mean a Novis class per variant — a surface that grows with the
/// vocabulary and that no caller of this member needs, since what a program
/// does with a step is read its grade, print its reason, or run its SQL.
pub(crate) const STEP: CoreClass = CoreClass {
    name: STEP_NAME,
    doc: Some(&STEP_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "grade",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(GRADE_NAME),
            symbol: "nvs_core_db_plan_step_grade",
            doc: Some(&STEP_GRADE_DOC),
        },
        CoreMethod {
            name: "reason",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_db_plan_step_reason",
            doc: Some(&STEP_REASON_DOC),
        },
        CoreMethod {
            name: "sql",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_db_plan_step_sql",
            doc: Some(&STEP_SQL_DOC),
        },
        CoreMethod {
            name: "isRefused",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_db_plan_step_is_refused",
            doc: Some(&STEP_IS_REFUSED_DOC),
        },
    ],
    slots: &[
        STEP_GRADE_SLOT,
        STEP_REASON_SLOT,
        STEP_SQL_SLOT,
        STEP_REPORT_SLOT,
    ],
    constants: &[],
};

/// `Core\Db\Plan\Step::grade`'s reference card — `rule:core-api/reference-card`.
const STEP_GRADE_DOC: MethodDoc = MethodDoc {
    short: "What this step can cost, at worst.",
    params: &[],
    ret: "`Safe`, `Locking` or `Destructive`. A grade is the worst case rather than the likely \
          one: an emitter with no rule for a case grades up, because over-reporting risk costs a \
          confirmation and under-reporting it costs an outage.",
    errors: &[],
};

/// `Core\Db\Plan\Step::reason`'s reference card — `rule:core-api/reference-card`.
const STEP_REASON_DOC: MethodDoc = MethodDoc {
    short: "Why the step is graded the way it is, in one sentence an operator reads.",
    params: &[],
    ret: "The sentence, written by the emitter that produced the SQL — so it names the backend's \
          own reason, such as a rewrite this server performs and another does not.",
    errors: &[],
};

/// `Core\Db\Plan\Step::sql`'s reference card — `rule:core-api/reference-card`.
const STEP_SQL_DOC: MethodDoc = MethodDoc {
    short: "The complete, terminated, dialect-correct SQL this step is. Never elided, including \
            for a step no `apply` will run: a deployment whose application credentials cannot \
            issue DDL hands the plan to a DBA, and a summary would be useless there.",
    params: &[],
    ret: "The statements, newline-joined. Most steps are one statement and can be handed \
          straight to `Core\\Db::execute`; SQLite's table rebuild is four, which is an operator's \
          to paste rather than a program's to run.",
    errors: &[],
};

/// `Core\Db\Plan\Step::isRefused`'s reference card — `rule:core-api/reference-card`.
const STEP_IS_REFUSED_DOC: MethodDoc = MethodDoc {
    short: "Whether this step is a report — a table, column or key the database has and the \
            schema does not name.",
    params: &[],
    ret: "`true` for a step no `apply` will ever run. Absence never destroys, because a database \
          an application shares with a queue, a reporting view and whatever an operator put there \
          is the ordinary case. A program that does want the drop runs this step's own `sql()`.",
    errors: &[],
};

/// `rule:core-classes/schema-plan`'s three grades — what a step can cost, at worst.
///
/// **Three and not two**, because "cannot lose data" and "cannot take the site
/// down for an hour" are different promises: merging them either refuses an
/// index a `Safe` deployment wants or applies a table rewrite it did not ask
/// for. The values are declaration ordinals and are ordered — `Safe` is the
/// smallest — which is [`nvs_db::Grade`]'s own ordering and what "an unknown
/// grade grades up" is an operation over.
pub(crate) const GRADE: CoreEnum = CoreEnum {
    name: GRADE_NAME,
    cases: &[("Safe", 0), ("Locking", 1), ("Destructive", 2)],
    doc: Some(&GRADE_DOC),
};

/// [`GRADE`]'s reference card — `rule:core-api/reference-card`.
const GRADE_DOC: EnumDoc = EnumDoc {
    short: "What one step of a schema plan can cost at worst, so that a deployment can run the \
            half it is willing to run unattended.",
    cases: &[
        CaseDoc {
            name: "Safe",
            desc: "Cannot lose data, cannot fail on rows that already exist, and cannot hold a \
                   long lock.",
        },
        CaseDoc {
            name: "Locking",
            desc: "Cannot lose data, but can fail on existing rows or block writes for a long \
                   time — a unique key over data that already collides, `NOT NULL` on a populated \
                   column, a type change that rewrites the table.",
        },
        CaseDoc {
            name: "Destructive",
            desc: "Can lose data. Every drop is here, and so is SQLite's create-copy-drop-rename \
                   rebuild, which is a data copy however it is spelled.",
        },
    ],
};

/// `Core\Db::connect`'s reference card — `rule:core-api/reference-card`.
pub(super) const CONNECT_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db::open`'s reference card — `rule:core-api/reference-card`.
///
/// `shape` is filled here and empty on every other card in this module,
/// because this is the one parameter that is a written shape rather than a
/// value: [`SETTINGS`]' merged key list, in the order the ABI flattens it.
pub(super) const OPEN_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db::inList`'s reference card — `rule:core-api/reference-card`.
pub(super) const IN_LIST_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db::quoteIdentifier`'s reference card — `rule:core-api/reference-card`.
pub(super) const QUOTE_IDENTIFIER_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Connection::query`'s reference card — `rule:core-api/reference-card`.
pub(super) const QUERY_DOC: MethodDoc = MethodDoc {
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
        TIMEOUT_PARAM_DOC,
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
            desc: "The connection failed while the statement was in flight — or `timeout` passed \
                   with it still in flight — which leaves it unusable for the rest of the \
                   request.",
        },
    ],
};

/// The `timeout` option's entry, shared by the four buffered statement members.
///
/// One constant for the same reason [`STATEMENT_OPTIONS`] is one: the option is
/// one option, and a card per member repeating it in its own words is four
/// descriptions free to drift. `stream` writes its own because the bound means
/// something different there — it covers the walk rather than the call.
const TIMEOUT_PARAM_DOC: ParamDoc = ParamDoc {
    name: "timeout",
    desc: "How long this statement may take. It bounds the whole exchange — the prepare, the \
           execution and every row of the answer — and not one read of it: when it passes the \
           statement gives up with an `IOError` and the connection is spent, since it was given \
           up on part way through a message. Omitted, the statement waits as long as the server \
           takes.",
    shape: &[],
};

/// `Core\Db\Connection::queryAs`'s reference card — `rule:core-api/reference-card`.
pub(super) const QUERY_AS_DOC: MethodDoc = MethodDoc {
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
        TIMEOUT_PARAM_DOC,
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

/// `Core\Db\Connection::execute`'s reference card — `rule:core-api/reference-card`.
pub(super) const EXECUTE_DOC: MethodDoc = MethodDoc {
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
        TIMEOUT_PARAM_DOC,
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
            desc: "The connection failed while the statement was in flight — or `timeout` passed \
                   with it still in flight — which leaves it unusable for the rest of the \
                   request.",
        },
    ],
};

/// `Core\Db\Connection::executeMany`'s reference card — `rule:core-api/reference-card`.
pub(super) const EXECUTE_MANY_DOC: MethodDoc = MethodDoc {
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
        TIMEOUT_PARAM_DOC,
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
            desc: "The connection failed while the batch was in flight — or `timeout` passed with \
                   it still in flight — which leaves it unusable for the rest of the request.",
        },
    ],
};

/// `Core\Db\Queryable::stream`'s reference card — `rule:core-api/reference-card`.
pub(super) const STREAM_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement and walks its rows one at a time, holding the connection open until \
            the walk ends — `MYSQLI_USE_RESULT` and `PDO::CURSOR_*`, with the cursor answered as \
            something a `foreach` reads directly. Memory is constant in the number of rows, which \
            is the whole reason to write this rather than `query`.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, bound exactly as `query` binds it: a `?` or a `:name` per value, \
                   never a value written into the text, and a sink either way.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind, read exactly as `query` reads them — list-keyed for `?`, \
                   string-keyed for `:name`, one array and never both spellings.",
            shape: &[],
        },
        ParamDoc {
            name: "timeout",
            desc: "How long the whole walk may take. The bound stays on the connection while the \
                   cursor is open, so it covers every row read and not just the call that opens \
                   the walk; the statement gives up with an `IOError` when it passes, and the \
                   connection is spent. Omitted, the walk waits as long as the server takes.",
            shape: &[],
        },
    ],
    ret: "A walk over the statement's rows, each one a `Core\\Db\\Row`, in the server's order. \
          Nothing has been read when this returns and the connection is busy from here: no second \
          statement runs on it until the walk reaches its end, so a loop that writes per row needs \
          a second connection (`{shared: false}`) or `query`'s buffered read instead.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong rather than the database: the placeholders and the array \
                   disagree in spelling or in number, a `:name` names no element, an element is a \
                   value with no bound form, the connection has been closed, or a statement is \
                   already streaming on it.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server refused the statement, or refused it part way through the walk, \
                   carrying its own `SQLSTATE` and message — or a column came back in a type this \
                   driver does not read back yet.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the statement or one of its rows was in flight — \
                   or `timeout` passed with the walk still open — which leaves it unusable for \
                   the rest of the request.",
        },
    ],
};

/// `Core\Db\Queryable::streamAs`'s reference card — `rule:core-api/reference-card`.
pub(super) const STREAM_AS_DOC: MethodDoc = MethodDoc {
    short: "Walks a result set a row at a time exactly as `stream` does and builds each row into \
            the class written at the call site — `queryAs`'s hydration over `stream`'s constant \
            memory, which is the pair `MYSQLI_USE_RESULT` and `PDO::FETCH_CLASS` only ever did one \
            at a time.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, bound exactly as `query` binds it: a `?` or a `:name` per value, \
                   never a value written into the text, and a sink either way.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind, read exactly as `query` reads them — list-keyed for `?`, \
                   string-keyed for `:name`, one array and never both spellings.",
            shape: &[],
        },
        ParamDoc {
            name: "timeout",
            desc: "How long the whole walk may take, under `stream`'s own rule: the bound stays on \
                   the connection while the cursor is open, so it covers every row read and not \
                   just the call that opens the walk. Omitted, the walk waits as long as the \
                   server takes.",
            shape: &[],
        },
    ],
    ret: "A walk over the statement's rows, each one built into `T`, in the server's order. \
          Nothing has been read when this returns and the connection is busy from here, exactly \
          as `stream` leaves it — and one `T` is held at a time, which is the whole difference \
          from `queryAs`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong rather than the database: the placeholders and the array \
                   disagree in spelling or in number, a `:name` names no element, an element is a \
                   value with no bound form, the connection has been closed, or a statement is \
                   already streaming on it. A `T` that carries no `#[Db\\Derive]` is refused here \
                   too, as the backstop under the compile-time diagnostic that already names it.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "A row did not match `T`: a column missing, a column of another type than the \
                   field declares, a SQL NULL in a field that is not `?T`, or a field whose \
                   declared type has no column mapping at all. Every bad column of that row is \
                   reported at once, in `issues`, each `path` the column's name — and the walk \
                   ends there, since the connection is left holding an unread portal.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server refused the statement, or refused it part way through the walk, \
                   carrying its own `SQLSTATE` and message — or a column came back in a type this \
                   driver does not read back yet.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the statement or one of its rows was in flight — \
                   or `timeout` passed with the walk still open — which leaves it unusable for \
                   the rest of the request.",
        },
    ],
};

/// `Core\Db\Queryable::transaction`'s reference card — `rule:core-api/reference-card`.
pub(super) const TRANSACTION_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Transaction::rollBack`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROLL_BACK_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Rows::all`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_ALL_DOC: MethodDoc = MethodDoc {
    short: "Every row of the result, in the server's order — `PDO::fetchAll` without a fetch-mode \
            argument to choose the shape with.",
    params: &[],
    ret: "An `array<T>`, empty for a statement that answered no rows. `T` is the result set's own \
          type argument: a `Core\\Db\\Row` for `query`, and the hydrated class for `queryAs<T>`. \
          The rows are the ones already read, so this costs one object each and no second decode.",
    errors: &[],
};

/// `Core\Db\Rows::first`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_FIRST_DOC: MethodDoc = MethodDoc {
    short: "The first row, or `null` where there is none — `PDO::fetch`, without its `false` and \
            without a cursor that a second call would move.",
    params: &[],
    ret: "A `T` — the result set's own type argument, as `all` describes — or `null` for an empty \
          result. `?T` is the absence spelling everywhere in `Core`, and a query that matched \
          nothing is an answer rather than a failure to throw about.",
    errors: &[],
};

/// `Core\Db\Rows::value`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_VALUE_DOC: MethodDoc = MethodDoc {
    short: "The first column of the first row — `PDO::fetchColumn`, and the shape a `select \
            count(*)` is read with.",
    params: &[],
    ret: "That column's value, or `null` where the result has no rows at all — which is the same \
          `null` a NULL column reads as, since the declared type is `mixed`. A caller that must \
          tell the two apart asks `count()` first.",
    errors: &[],
};

/// `Core\Db\Rows::column`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_COLUMN_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Rows::count`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_COUNT_DOC: MethodDoc = MethodDoc {
    short: "How many rows the statement answered — `PDOStatement::rowCount` on a select, which is \
            the use of that member this replaces. A write's count is `Core\\Db\\Write::affected`.",
    params: &[],
    ret: "The number of rows held, which is exact because § 4's default read all of them before \
          `query` returned.",
    errors: &[],
};

/// `Core\Db\Rows::columns`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_COLUMNS_DOC: MethodDoc = MethodDoc {
    short: "What the statement described, one `Core\\Db\\Column` per column and in the server's \
            own order — `PDOStatement::getColumnMeta` asked once for the whole row description \
            rather than once per column, and `mysqli_fetch_fields`.",
    params: &[],
    ret: "The columns. An empty result set has them too: a `select` that matched nothing still \
          described what it would have answered, which is what makes this readable before the \
          rows are.",
    errors: &[],
};

/// `Core\Db\Column::name`'s reference card — `rule:core-api/reference-card`.
pub(super) const COLUMN_LABEL_DOC: MethodDoc = MethodDoc {
    short: "The column's label, as the server described it — `getColumnMeta`'s `name`. It is the \
            alias wherever the `select` list wrote one, because an alias is what the server \
            describes.",
    params: &[],
    ret: "The label, and not a key: `select a, a` describes two columns under one label, so the \
          answer is read by position in the array `columns()` handed back.",
    errors: &[],
};

/// `Core\Db\Column::type`'s reference card — `rule:core-api/reference-card`.
pub(super) const COLUMN_DECLARED_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Column::nullable`'s reference card — `rule:core-api/reference-card`.
pub(super) const COLUMN_NULLABLE_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::has`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_HAS_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::get`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_GET_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::toArray`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_TO_ARRAY_DOC: MethodDoc = MethodDoc {
    short: "The whole row as a string-keyed array, in the server's column order — `FETCH_ASSOC`, \
            which is the only one of PHP's three fetch shapes that survives.",
    params: &[],
    ret: "An `array<mixed>` keyed by column label, a NULL column being a `null` entry that is \
          present rather than absent.",
    errors: &[],
};

/// `Core\Db\Row::string`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_STRING_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::bytes`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_BYTES_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::int`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_INT_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::uint`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_UINT_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::float`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_FLOAT_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::bool`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_BOOL_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::decimal`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_DECIMAL_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::instant`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_INSTANT_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::date`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_DATE_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::time`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_TIME_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Row::uuid`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_UUID_DOC: MethodDoc = MethodDoc {
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

/// `Core\Db\Write::affected`'s reference card — `rule:core-api/reference-card`.
pub(super) const WRITE_AFFECTED_DOC: MethodDoc = MethodDoc {
    short: "How many rows the statement affected — `PDOStatement::rowCount` on a write, without \
            its documented unreliability on a select, because a select does not answer with one \
            of these at all.",
    params: &[],
    ret: "A `uint`, and `0` for a statement that affected none as well as for one whose kind has \
          no count to report — a `create table`. `changed` is where those two are told apart.",
    errors: &[],
};

/// `Core\Db\Write::changed`'s reference card — `rule:core-api/reference-card`.
pub(super) const WRITE_CHANGED_DOC: MethodDoc = MethodDoc {
    short: "The same count as the server itself reported it, whose `null` is the one thing \
            `affected` cannot say: this statement's kind carries no row count at all.",
    params: &[],
    ret: "A `?uint`, equal to `affected` wherever it is not `null`. On PostgreSQL the distinction \
          MySQL draws between rows matched and rows altered has nothing in the protocol to read \
          it out of, so inventing a second count that always equalled the first would be a \
          difference callers wrote code against.",
    errors: &[],
};

/// `Core\Db\Write::lastId`'s reference card — `rule:core-api/reference-card`.
pub(super) const WRITE_LAST_ID_DOC: MethodDoc = MethodDoc {
    short: "The key the statement handed back, read off the write that produced it rather than \
            off the connection — `lastInsertId` and `mysqli_insert_id` without their \
            stale-after-an-unrelated-statement hazard.",
    params: &[],
    ret: "A `?uint`: the key this statement generated, and `null` where it generated none — a \
          statement that inserted no row answers `null`. PostgreSQL reads the key out of a \
          `RETURNING` clause, since that protocol has no last-insert-id of its own: the first \
          column of the last row returned, where that column was declared an integer. MySQL \
          reads the `AUTO_INCREMENT` value out of the write's own status packet, and SQLite the \
          rowid of a row this statement inserted itself, which may be `0`. SQL Server answers \
          `null` always, because its token stream carries no generated key.",
    errors: &[],
};
