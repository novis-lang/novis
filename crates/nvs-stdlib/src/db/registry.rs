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

/// `Core\Db\Settings` — the two arms `open` takes, per
/// `rule:core-api/shape-parameter`.
///
/// **The arms are separated by their `driver` and by nothing else that is
/// declared.** § 2's arm selection is "exactly one arm accepts it", and enum-case
/// types (`rule:types/enum-case-type`) are what make these two disjoint: a settings object naming
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

/// `Core\Db` — `connect`, `open`, and the two connectionless entry points.
///
/// `rule:core-classes/db-one-api` owns every semantic behind it. `open` is the
/// one entry point still missing, and [`crate::db`]'s known gaps own why: what
/// it needs is a `CoreTy` for a shape parameter, not a body.
// rank: 1490
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "connect",
            names: &["name"],
            params: &[
                // The name is a **sink**, because it selects
                // which operator-written credential this program opens with,
                // and a `tainted` one would let a request pick the database.
                CoreTy::Text(Qual::Sink),
                CoreTy::Options(&[
                    CoreOption {
                        name: "shared",
                        ty: CoreTy::Bool,
                        // Memoized is the default and the option only turns it
                        // off, which is `rule:core-classes/db-connection-is-named`'s `{ shared: false }`.
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
                // The settings are a **sink** for the host, and that
                // classification sits on [`SETTINGS`]' own field, not here.
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

/// `Core\Db\Connection` — the object `connect` answers with.
///
/// The slots are the pair every handle in this crate carries — the key into the
/// request's own table ([`nvs_runtime::Ctx::hold_open_connection`]) and the name
/// it was opened by, which is what a refusal can name without reaching for the
/// connection it is refusing about.
///
/// **`query`, `queryAs`, `execute`, `executeMany`, `stream`, `streamAs` and
/// `transaction` are `Core\Db\Queryable`'s members, and every one is declared
/// here.** Beyond the interface, four rows of its own land here too: `close`,
/// `driver`, `serverVersion` and `isOpen`. The last three answer without a
/// round trip: `nvs_db::Connection::server_version` reads a string the
/// connection already has.
///
/// **`close` is the only one of these on this class alone.** A
/// [`TRANSACTION`] delegates `Core\Db\Queryable` to its connection and nothing
/// else, and closing the connection out from under the `transaction()` call
/// that is still running is not something Novis gives a syntax for.
/// `rule:classes/no-traits` makes `Transaction` delegate the interface to its connection, so
/// every one of them is declared once — here — and [`TRANSACTION`] is where the
/// forwarding lands.
/// `rule:core-classes/db-statement-members`'s `{ timeout?: Duration }`, the one option every statement member
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
/// **`chunk` is deliberately not here.** A `chunk` option on `stream` would have
/// to reach the `Execute` that asks for a row count, and this driver's walk asks for
/// one row; [`crate::db`] § *`stream` declares no `chunk`, and the portal is
/// why* is where that stays recorded.
const STATEMENT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "timeout",
    ty: CoreTy::Instance(crate::time::DURATION_NAME),
    // `connect`'s default and for `connect`'s reason: no `Duration` means
    // "unbounded", so the absence of the option is the absence itself.
    default: Const::Null,
}];

/// What `connect` answers with, and `Core\Db\Queryable`'s own home: `rule:classes/no-traits`
/// has `Transaction` delegate the interface to its connection, so the seven
/// members are declared here once. `query`, `execute`, `executeMany` and
/// `transaction` are the four that have landed.
// rank: 1500
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
            // `Rows<Row>` — [`ROWS`] at the one concrete argument an
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
            // `Rows<T>` at the `T` the call site wrote — the whole
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
                // `array<array<mixed>>`. The outer array is the sets and
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
            // rows are when it answers. The options bag is `{timeout?}`
            // alone: on this member the deadline stays
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
            // `Iterable<Db\Row>`, spelled as the class that *is* the
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
            // `Iterable<T>`, which is the walk above at the type its
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
            // `Core\Db\Driver`, which is the same enum a `Core\Db::open`
            // settings object names first — [`DRIVER`]. A connection is the
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
            // Unqualified, for [`COLUMN`]'s
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
/// not delegated to a [`TRANSACTION`]: the `Connection` members beyond
/// `Queryable`.
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
/// `stream` and `streamAs` will not join it: both are `Queryable` members, so
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
    short: "Returns the connection to the pool before the request that opened it ends. After \
            that, every other method of this connection throws an error. `isOpen` returns \
            `false`, and a second `close` does nothing.",
    params: &[],
    ret: "Nothing. The only effect is that the connection goes back to the pool, and a program \
          cannot use the pool directly.",
    errors: &[],
};

/// `Core\Db\Connection::driver`'s reference card — `rule:core-api/reference-card`.
const DRIVER_MEMBER_DOC: MethodDoc = MethodDoc {
    short: "Returns the database this connection uses. This is the `Core\\Db\\Driver` case that \
            the `[db.<name>]` block or the `open` settings named.",
    params: &[],
    ret: "The connection's own `Core\\Db\\Driver` case.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The connection has been closed.",
    }],
};

/// `Core\Db\Connection::serverVersion`'s reference card — `rule:core-api/reference-card`.
const SERVER_VERSION_DOC: MethodDoc = MethodDoc {
    short: "Returns the version the database server reported, exactly as it reported it. This \
            is PostgreSQL's `server_version`, the greeting banner of MySQL and MariaDB, \
            `major.minor.build` from the SQL Server login reply, or SQLite's library version. \
            The server sent it when the connection opened, so reading it runs no statement. \
            You can call it while a `stream` is using the connection.",
    params: &[],
    ret: "The version. It is never empty and never changed. A suffix that a distribution adds \
          is part of it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The connection has been closed.",
    }],
};

/// `Core\Db\Connection::isOpen`'s reference card — `rule:core-api/reference-card`.
const IS_OPEN_DOC: MethodDoc = MethodDoc {
    short: "Checks whether this connection can still be used. It returns `true` until `close` \
            is called, and `false` after that. It is the only method that works on a closed \
            connection.",
    params: &[],
    ret: "`true` if statements can still run on the connection. `false` if `close` has \
          returned it to the pool.",
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
/// because § 7 says so out loud — a callable may have side effects that are not
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
/// level. `retries` is read by the helper instead, which re-runs the callable on
/// a conflict the driver says may be re-run, after the wait § 7 asks for.
/// [`retry_backoff`] draws that wait exponentially and with full jitter and
/// [`wait_between_attempts`] gives the core back for it; the declared default
/// of 0 is what every call that does not ask gets.
pub(super) const TRANSACTION_ROW: CoreMethod = CoreMethod {
    name: "transaction",
    names: &["fn"],
    // Written, as `rule:types/callable-signature` has every callback spell what
    // it receives: what this one is handed is a [`TRANSACTION`], and a callable
    // declaring no parameter at all still satisfies the row under
    // `rule:types/callable-arity`'s prefix match — which is § 7's R9 allowance,
    // now stated where it is checked rather than left to
    // `nvs_runtime::call_callable`'s trim.
    params: &[
        CoreTy::CallableSig(&[CoreTy::Instance(TRANSACTION_NAME)], &CoreTy::Var("T")),
        CoreTy::Options(TRANSACTION_OPTIONS),
    ],
    defaults: &[],
    // § 7's `: T`. The member's answer *is* the callable's, so the transaction
    // is scenery around a call that computes whatever it was going to compute
    // — the same binding `Core\Cli::live` performs, and the reason a
    // transaction can wrap an existing expression without retyping it.
    return_ty: CoreTy::Var("T"),
    symbol: "nvs_core_db_connection_transaction",
    doc: Some(&TRANSACTION_DOC),
};

/// `Core\Db\Transaction` — what § 7's callable is handed, and the
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
///
/// What `rule:core-classes/db-transactions`'s callable is handed. It carries the same four rows
/// under the same symbols — which is what the delegation above is at
/// runtime — plus `rollBack`, the one member of the pair that is a
/// transaction's alone.
// rank: 1510
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
            // `Rows<Row>` — [`ROWS`] at the one concrete argument an
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

/// `Core\Db\Driver` — `rule:core-classes/db-one-api`'s five backends, as the registry half
/// of [`nvs_db::Driver`].
///
/// **The two halves are one enum and the wire one is authoritative.** This
/// table is what a program writes into a `Db\Settings` object;
/// `nvs_db::Driver` is what a connection reports and what
/// `nvs_db::Driver::from_config_name` reads a `[db.<name>]` block's `driver`
/// as. Nothing about a backend is decided here.
///
/// **The cases are what make `Db\Settings` a discriminated union**, and they
/// are the whole of the mechanism: `rule:core-api/shape-arms-are-disjoint` selects an arm by asking which
/// one accepts the object, and `rule:types/single-value-types`'s enum-case types make
/// `Driver::Sqlite` and the other four disjoint sets. No field is declared to
/// be a discriminant, here or anywhere.
///
/// **The values are declaration ordinals and mean nothing else.** `MySql` is 0
/// and `SqlServer` is 4, and they are not a
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
    short: "Which database a connection uses. It is the first field of the settings object that \
            `Core\\Db::open` takes. The driver sets which of the two shapes the rest of that \
            object has: a server needs a `host`, and SQLite needs a `path`.",
    cases: &[
        CaseDoc {
            name: "MySql",
            desc: "MySQL, over its own client protocol.",
        },
        CaseDoc {
            name: "MariaDb",
            desc: "MariaDB. This is a separate driver, not a MySQL option. MariaDB has its own \
                   authentication methods and its own error codes.",
        },
        CaseDoc {
            name: "Postgres",
            desc: "PostgreSQL, over the extended-query protocol.",
        },
        CaseDoc {
            name: "Sqlite",
            desc: "SQLite. It uses a file named by `path`, and no host.",
        },
        CaseDoc {
            name: "SqlServer",
            desc: "Microsoft SQL Server, over TDS.",
        },
    ],
};

/// [`TLS`]'s fully-qualified name, written once for the same reason
/// [`DRIVER_NAME`] is.
pub(crate) const TLS_NAME: &str = r"Core\Db::Tls";

/// `Core\Db::Tls` — how much of the server's certificate a TCP connection
/// checks.
///
/// **`VerifyFull` is the default and it is the only mode this runtime
/// implements**, which is `rule:core-classes/db-capabilities`'s third closed hole: PHP's `pdo_pgsql`
/// defaults to `sslmode=prefer` and connects in plaintext whenever the server
/// says so, and none of § 3's three defaults is configurable to the unsafe
/// value. The other three cases are declared so that a
/// program that writes one is refused *at the call*, naming what it asked for
/// — which is an honest answer, where accepting `Disabled` and quietly
/// verifying anyway would be a second, silent one. [`settings_tls`] is where
/// that refusal is worded.
///
/// What a deployment *can* change is whose certificates are believed, and that
/// is a different field: a `tls_ca_file` on the block, or the compiled-in
/// Mozilla anchor set — [`nvs_db::PgTarget::tls_ca_file`] owns that reading.
///
/// **The values are declaration ordinals**, weakest first.
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
    short: "How much a TCP connection checks the server's identity before it sends credentials. \
            Every connection uses `VerifyFull`, and it is the default when the settings name no \
            mode. The three weaker modes throw an error. A connection that checks less than it \
            promises is a security hole.",
    cases: &[
        CaseDoc {
            name: "Disabled",
            desc: "No TLS at all. Not allowed: it throws an error.",
        },
        CaseDoc {
            name: "Required",
            desc: "TLS, but the certificate is not checked. Not allowed: it throws an error.",
        },
        CaseDoc {
            name: "VerifyCa",
            desc: "The certificate must chain to a trusted root certificate, but its name is not \
                   checked. Not allowed: it throws an error.",
        },
        CaseDoc {
            name: "VerifyFull",
            desc: "The certificate must chain to a trusted root certificate and must be issued \
                   for the host in the settings. This is the default, and the only mode a \
                   connection uses.",
        },
    ],
};

/// [`ISOLATION`]'s fully-qualified name, written once so the row, the option
/// that will take it and every message quoting it cannot drift apart.
pub(crate) const ISOLATION_NAME: &str = r"Core\Db\Isolation";

/// `Core\Db\Isolation` — `rule:core-classes/db-transactions`'s five levels, as the
/// registry half of [`nvs_db::Isolation`].
///
/// **The two halves are one enum and the wire one is authoritative.** This
/// table is what a program writes; `nvs_db::Isolation` is what a driver renders,
/// and its doc comment owns both the rule that a driver lacking a level throws
/// rather than running the callable at a weaker one, and the reason a driver may
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
    short: "How much a transaction can see of other transactions running at the same time. \
            This is the `isolation` option of `transaction`. Without it, the connection's own \
            level is used. A driver that cannot offer the requested level throws an error. It \
            does not run `$fn` at a weaker level.",
    cases: &[
        CaseDoc {
            name: "ReadUncommitted",
            desc: "A statement may read rows another transaction has written and not committed. \
                   This applies only on a database that supports this level.",
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
            desc: "The transaction reads from one snapshot taken when it began. Conflicting \
                   writes fail and do not wait. This is SQL Server's own level. Databases that \
                   keep row versions call it `REPEATABLE READ`.",
        },
        CaseDoc {
            name: "Serializable",
            desc: "Transactions that run at the same time give the same result as if they ran \
                   one after another in some order. A transaction that cannot do this is rolled \
                   back, so the caller can retry it.",
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
pub(crate) const ERROR_KIND_NAME: &str = r"Core\Db\DbError::Kind";

/// `rule:core-classes/db-error`'s `ErrorKind` — the
/// eleven conditions an application branches on, as the registry half of
/// [`nvs_db::DbErrorKind`].
///
/// **The two halves are one enum and the wire one is authoritative.** This
/// table is what a program matches on; `nvs_db::DbErrorKind` is what a driver
/// maps its own codes onto, and its doc comment owns why the set is normalised
/// at all — PDO exposes a `SQLSTATE` and a vendor integer, so real PHP matches
/// on `"Duplicate entry"` or hard-codes `1062`. `nvs_db::DbErrorKind::is_retryable`
/// owns which two of these § 7's `{ retries: n }` re-runs a callable over, and
/// nothing about that rule is decided here.
///
/// **A class per condition was rejected** — § 8's own *Alternatives*: ten more
/// types in the deliberately small closed exception set
/// `rule:errors/throwable-hierarchy` fixes, for
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
    short: "Why the server rejected a statement. The cases are the same for every driver, so a \
            program can check the condition without knowing each database's own error code. \
            `Core\\Db\\DbError::$kind` returns one of these. For anything more specific, read \
            the raw `SQLSTATE` beside it.",
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
            desc: "A `CHECK` constraint rejected the row.",
        },
        CaseDoc {
            name: "Deadlock",
            desc: "Two transactions each wait for a lock the other has, so the server stopped \
                   this one. SQLite's `SQLITE_BUSY` and `SQLITE_LOCKED` also give this kind, so \
                   a retry works on SQLite too.",
        },
        CaseDoc {
            name: "SerializationFailure",
            desc: "The transaction conflicted with another transaction running at the same time, \
                   and the server stopped it. This is normal under `REPEATABLE READ` or a \
                   stronger level.",
        },
        CaseDoc {
            name: "ConnectionLost",
            desc: "The connection is gone, or the server is going away.",
        },
        CaseDoc {
            name: "Timeout",
            desc: "A statement or an idle transaction ran past a time limit and was cancelled.",
        },
        CaseDoc {
            name: "Syntax",
            desc: "The server cannot run the statement: a syntax error, an undefined table, or a \
                   type it cannot resolve.",
        },
        CaseDoc {
            name: "Permission",
            desc: "The database role is not allowed to do this.",
        },
        CaseDoc {
            name: "Other",
            desc: "Any condition the driver does not map to a kind above, including one that only \
                   one database has. A `Core\\Db\\DbError` that a program creates itself also \
                   has this kind, because no server classified it.",
        },
    ],
};

/// [`COLUMN_TYPE`]'s fully-qualified name, written once so the row, the member
/// that answers with it and every message quoting it cannot drift apart.
pub(crate) const COLUMN_TYPE_NAME: &str = r"Core\Db\Column::Type";

/// `Core\Db\Column::Type` — the fourteen types a result column can be
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
/// **The values are declaration ordinals and mean nothing else.** They follow
/// [`nvs_db::ColumnType`]'s declaration order, so `Int` is 0 and `Other` is 13, but they are not a rank a program may compare: these are a
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
    short: "The type a result column was declared with. It describes the column, not the value \
            you get when you read it. For example, a `JSON` column and a `TEXT` column both \
            read as `tainted string`. This type tells them apart, if the database has a \
            separate type for each.",
    cases: &[
        CaseDoc {
            name: "Int",
            desc: "A signed integer column: `SMALLINT`, `INTEGER` or `BIGINT`. A `TINYINT(1)` on \
                   MySQL and MariaDB is also `Int`, not `Bool`.",
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
            desc: "An exact numeric column: `DECIMAL`, `NUMERIC` or `MONEY`.",
        },
        CaseDoc {
            name: "Text",
            desc: "A text column: `CHAR`, `VARCHAR`, `TEXT` or `ENUM`. A JSON column is also \
                   `Text` on a database where JSON is only another name for a text type.",
        },
        CaseDoc {
            name: "Bytes",
            desc: "A binary column: `BINARY`, `BLOB` or `BYTEA`. This includes the `BINARY(16)` \
                   column that MySQL schemas use to store a UUID.",
        },
        CaseDoc {
            name: "Bool",
            desc: "`BOOLEAN`, and `BIT(1)`.",
        },
        CaseDoc {
            name: "Date",
            desc: "A `DATE`. Reading it gives a `Core\\Time\\Date`.",
        },
        CaseDoc {
            name: "Time",
            desc: "A `TIME`. Reading it gives a `Core\\Time\\TimeOfDay`.",
        },
        CaseDoc {
            name: "DateTime",
            desc: "A `DATETIME` or `TIMESTAMP` without a time zone. Reading it gives a \
                   `Core\\Time\\DateTime` in the time zone set for the connection.",
        },
        CaseDoc {
            name: "Instant",
            desc: "A column that stores its own offset: `TIMESTAMPTZ`, or SQL Server's \
                   `datetimeoffset`. Reading it gives a `Core\\Time\\Instant`.",
        },
        CaseDoc {
            name: "Uuid",
            desc: "A `UUID` or a `uniqueidentifier`. Reading it gives a `Core\\Uuid`.",
        },
        CaseDoc {
            name: "Json",
            desc: "A column that the database declares as JSON. Reading it still gives a \
                   `tainted string`, because JSON is never decoded for you. A database with no \
                   JSON type of its own reports the column as `Text`.",
        },
        CaseDoc {
            name: "Other",
            desc: "Every other column: one with no matching Novis type, and every array. This \
                   case is not an error. It means a column list can describe every column a \
                   server can send.",
        },
    ],
};

/// `Core\Db\Rows` — what a buffered statement answers with.
///
/// What the slot holds is the whole of what the members read, decided here
/// rather than left to them: **every row, already decoded, each as a
/// string-keyed array of its columns** — which is `Row::toArray`'s own shape, so
/// every member below is a reader over it and never a second decoder.
///
/// **Six members, and the sixth reads a second slot.**
/// `columns()` answers what the *statement* described rather than anything a
/// row holds, so [`ROWS_COLUMNS_SLOT`] is filled beside the rows at query time:
/// [`nvs_db::PgRows`] lends its row description out of the borrow the rows are
/// read from, so it is captured there or not at all. What a program
/// matches the answer on is [`COLUMN_TYPE`], whose own doc owns why a
/// description of a column is not a summary of the value a read of it
/// produces.
///
/// **It is generic at `T`, and `Rows` and `Rows<T>` are this one class.**
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
///
/// What `query` answers with — `rule:core-classes/db-statement-members`'s buffered result set, and all
/// six of its readers over it. Five read the rows it holds and
/// `columns()` reads the description beside them, which is why it is
/// readable before a row is.
// rank: 1520
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
            // `all` answers the class's own type
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
            // `int|string $key`: a number is the column's *position* in
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
            // `array<Column>` and deliberately not `array<T>`: a
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

/// `Core\Db\Row` — one row of a [`ROWS`], and the whole of what
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
/// universal path — `->get()` plus `as` — is what a program that
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
/// which is the universal path and what that roster means by leaving it
/// out.
///
/// One row of that set: the associative reading plus the eleven typed
/// readers `rule:core-classes/db-column-types` puts in place of PHP's three fetch modes. Four of
/// the eleven answer only their refusal until § 9's structured columns land.
// rank: 1540
pub(crate) const ROW: CoreClass = CoreClass {
    name: ROW_NAME,
    doc: Some(&ROW_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "has",
            names: &["name"],
            // Neutral, which is what every other name
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
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_db_row_string",
            doc: Some(&ROW_STRING_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedBytes),
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

/// `Core\Db\Write` — what a statement that answers no rows answers
/// with, and the whole of what `rule:core-classes/db-statement-members` puts in place of `rowCount` on a
/// write, `lastInsertId` and `mysqli_info`.
///
/// **Three readers rather than three readonly properties**, because a
/// `Core`-owned instance has
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
///
/// What `execute` answers with: § 4's two counts and the id a `RETURNING`
/// clause handed back, as three readers — for the reason
/// [`CoreTy::Instance`](crate::registry::CoreTy::Instance) states, that a `Core` instance has no property a
/// program can reach.
// rank: 1550
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

/// `Core\Db\Column` — one column of what a statement described,
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
/// **Three readers rather than three properties**, for the reason
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
///
/// One column of what a statement described — three readers in place
/// of `getColumnMeta`'s per-driver array, reached through `Rows::columns`
/// and produced nowhere else. Its `type()` is what gives `crate::db`'s
/// `COLUMN_TYPE` a member that answers one.
// rank: 1560
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
            // Unqualified: a label is
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

/// `Core\Db\InList` — opaque, produced by one member and read by the bind. It
/// has no members at all, because it is accepted only as a bound parameter.
///
/// What `inList` answers with: `rule:core-classes/db-parameters`'s explicit
/// expansion marker, memberless because `Core\Db` accepts it nowhere but a
/// bound parameter. [`crate::db`] owns why the expansion itself stays in
/// `nvs-db`.
// rank: 1570
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
/// **Two members here and no builders.** `rule:core-classes/schema-is-a-value` gives a schema three
/// interchangeable spellings and no privileged one, and the array form is the
/// one the other two are defined against — so `fromArray` is the door and
/// `toArray` is the window, and a typed builder surface would be a second way
/// to say what this one already says. `rule:core-classes/schema-apply-capability`'s `planAgainst`, `applySafe` and
/// `applyIncludingRisky` are the members that do something with the value; they
/// need a connection and this class needs none.
///
/// `rule:core-classes/schema-is-a-value`'s schema value, beside the connection classes because that is
/// what it is asked about: a schema is compared against a live database and
/// applied to one. It needs no connection to exist, which is why its two
/// members here are the array form alone — § 9's three are the ones that take
/// a `Core\Db\Connection`.
// rank: 1580
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
    short: "Creates a schema from its standard array form. This is the form that `toArray` \
            returns, that a schema file contains and that `nvs schema dump` prints. Every rule \
            is checked here, so any schema value works with all five databases.",
    params: &[ParamDoc {
        name: "array",
        desc: "The schema, as `[\"tables\" => [...]]`. A table has `name`, `columns`, and \
               optionally `primary_key`, `unique` and `indexes`. A column has `name` and `type`, \
               and optionally `null`, `identity` and a one-key `default`. A missing key means \
               the empty list or `false`.",
        shape: &[],
    }],
    ret: "A `Core\\Db\\Schema` in **normalized** form: tables in name order, columns in \
          declaration order, and every optional key filled in. So `toArray` returns the same \
          array for every way to write one schema. This is what makes two schemas comparable.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "A key is missing or has the wrong type of value, a type name is not supported, or \
               an identifier is not a plain identifier. It is also thrown when the schema is not \
               consistent. Examples are a table with no columns, a second identity column, an \
               identity column outside the primary key, or an index on a missing column.",
    }],
};

/// `Core\Db\Schema::toArray`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_TO_ARRAY_DOC: MethodDoc = MethodDoc {
    short: "Returns the schema in its standard array form. You can save it to a file, pass it to \
            `Core\\Json::encode`, or compare it with another schema.",
    params: &[],
    ret: "The array that `fromArray` reads back as the same schema. Tables are in name order. \
          Columns are in the order they were declared, because `create table` keeps that order. \
          Constraints and indexes are in name order, because the order they were added in has \
          no effect.",
    errors: &[],
};

/// `Core\Db\Schema::planAgainst`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_PLAN_AGAINST_DOC: MethodDoc = MethodDoc {
    short: "Reads the database of this connection and returns every difference between it and \
            this schema. The differences are in the order they must be fixed. Making a plan \
            changes nothing. An empty plan means the database already matches the schema.",
    params: &[ParamDoc {
        name: "connection",
        desc: "The database to compare with. Planning only reads, so it needs only the \
               `db.connect` capability this connection was opened with.",
        shape: &[],
    }],
    ret: "The plan. Each of its `steps()` has a grade, `Safe`, `Locking` or `Destructive`, and \
          the complete SQL for the change. A table or column that the database has and this \
          schema does not is a **report**. It is in the plan with the SQL that would remove it, \
          but no `apply` ever runs it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The database has a type, a default or an identifier that a schema cannot \
               describe, so a plan could not cover every difference.",
    }],
};

/// `Core\Db\Schema::applySafe`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_APPLY_SAFE_DOC: MethodDoc = MethodDoc {
    short: "Makes a plan for this connection and runs it, but only if every step it would run \
            is graded `Safe`. It needs the `db.schema` capability for the connection's block. \
            Running DDL needs this extra permission, beyond access to the database.",
    params: &[ParamDoc {
        name: "connection",
        desc: "The database to change. `db.schema` is granted for the name of its \
               `[db.<name>]` block. So you cannot use a connection from `Core\\Db::open` here.",
        shape: &[],
    }],
    ret: "Nothing. Afterwards the database matches the schema. Only what the schema does not \
          declare is different: it is reported and not changed.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The plan has a step that is not `Safe`, and the message names it. It is also \
                   thrown when the connection has no block to grant `db.schema` for, and for \
                   every error that `planAgainst` throws.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The `db.schema` capability is not granted for this connection's block.",
        },
    ],
};

/// `Core\Db\Schema::applyIncludingRisky`'s reference card — `rule:core-api/reference-card`.
const SCHEMA_APPLY_RISKY_DOC: MethodDoc = MethodDoc {
    short: "Works like `applySafe`, without the grade check. It runs every step of the plan. \
            This includes steps that can hold a long lock, rewrite a table or fail on rows that \
            already exist. The long name shows a reviewer the risk at the call.",
    params: &[ParamDoc {
        name: "connection",
        desc: "The database to change. It works as in `applySafe`, and needs the same \
               `db.schema` capability.",
        shape: &[],
    }],
    ret: "Nothing. A reported drop still does not run, because a risky change is not the same \
          as data loss. The step has the SQL for the drop if a program wants to run it.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The connection has no block to grant `db.schema` for. It is also thrown for \
                   every error that `planAgainst` throws.",
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
///
/// `rule:core-classes/schema-plan`'s plan and its steps, immediately after the schema whose
/// `planAgainst` answers one: a plan is the value that carries the answer,
/// and neither class is reachable except through that member.
// rank: 1590
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
    short: "Returns every step of the plan in the order they must run, including reports.",
    params: &[],
    ret: "The steps. An empty array means the database already matches the schema. No `apply` \
          runs a step whose `isRefused()` is `true`.",
    errors: &[],
};

/// `rule:core-classes/schema-plan`'s step — one difference, its grade, the sentence explaining the
/// grade, and the SQL that makes it.
///
/// **Four readers over four slots and no `change()`.** `rule:core-classes/schema-vocabulary-is-closed`'s vocabulary is
/// closed and a [`nvs_db::Change`] is closed with it, so exposing the change
/// itself would mean a Novis class per variant — a surface that grows with the
/// vocabulary and that no caller of this member needs, since what a program
/// does with a step is read its grade, print its reason, or run its SQL.
// rank: 1600
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
    short: "Returns the worst risk of this step.",
    params: &[],
    ret: "`Safe`, `Locking` or `Destructive`. The grade is the worst case, not the likely case. \
          When there is no rule for a change, it gets the higher grade. Too high a grade costs \
          one confirmation, and too low a grade can cost an outage.",
    errors: &[],
};

/// `Core\Db\Plan\Step::reason`'s reference card — `rule:core-api/reference-card`.
const STEP_REASON_DOC: MethodDoc = MethodDoc {
    short: "Returns one sentence for a person that says why the step has its grade.",
    params: &[],
    ret: "The sentence. It is written for the same database as the SQL. So it gives the reason \
          for that database, such as a table rewrite that this server does and another does not.",
    errors: &[],
};

/// `Core\Db\Plan\Step::sql`'s reference card — `rule:core-api/reference-card`.
const STEP_SQL_DOC: MethodDoc = MethodDoc {
    short: "Returns the complete SQL of this step, with terminators, in the right dialect for the \
            database. It is never shortened, also for a step that no `apply` runs. Some \
            applications cannot run DDL and give the plan to a database administrator. A summary \
            does not help there.",
    params: &[],
    ret: "The statements, one per line. Most steps are one statement, and you can pass it \
          directly to `Core\\Db::execute`. SQLite's table rebuild is four statements. A person \
          pastes those, and a program does not run them.",
    errors: &[],
};

/// `Core\Db\Plan\Step::isRefused`'s reference card — `rule:core-api/reference-card`.
const STEP_IS_REFUSED_DOC: MethodDoc = MethodDoc {
    short: "Checks whether this step is a report. A report is a table, column or key that the \
            database has and the schema does not name.",
    params: &[],
    ret: "`true` for a step that no `apply` ever runs. Something missing from the schema is \
          never deleted. A database is often shared with a queue, a reporting view or tables \
          an administrator added. A program that wants the drop runs this step's `sql()`.",
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
    short: "The worst risk of one step of a schema plan. A deployment can use it to run only \
            the steps it is willing to run without a person watching.",
    cases: &[
        CaseDoc {
            name: "Safe",
            desc: "Cannot lose data, cannot fail on rows that already exist, and cannot hold a \
                   long lock.",
        },
        CaseDoc {
            name: "Locking",
            desc: "Cannot lose data, but can fail on existing rows or block writes for a long \
                   time. Examples are a unique key on data with duplicates, `NOT NULL` on a \
                   column with data, or a type change that rewrites the table.",
        },
        CaseDoc {
            name: "Destructive",
            desc: "Can lose data. Every drop has this grade. So does SQLite's table rebuild \
                   (create, copy, drop, rename), because it always copies the data.",
        },
    ],
};

/// `Core\Db::connect`'s reference card — `rule:core-api/reference-card`.
pub(super) const CONNECT_DOC: MethodDoc = MethodDoc {
    short: "Opens the database connection that a `[db.<name>]` block of `nvs.toml` describes. Later \
            calls in the same request return the same connection. The password is in the \
            configuration file, not in the program. It needs the `db.connect` capability for that \
            name.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The block to open. It must match exactly: `\"main\"` is `[db.main]`. Two \
                   blocks for the same server give two connections, because whoever wrote two \
                   blocks wanted two.",
            shape: &[],
        },
        ParamDoc {
            name: "shared",
            desc: "Whether this call may return the connection that an earlier call opened. \
                   `false` opens a separate connection. You need one for a write that must stay \
                   after a rollback, for a session lock, or for a second statement during a \
                   `stream`.",
            shape: &[],
        },
        ParamDoc {
            name: "timeout",
            desc: "How long opening the connection may take, including the DNS lookup and TLS. \
                   Without it, only the server and the network limit the time.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Db\\Connection`. The same call twice in one request returns the same object, \
          unless `shared` is `false`. The connection is closed when the request ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`db.connect` does not grant `$name`, or there is no `[db.<name>]` block of that \
                   name. It is also thrown when the block does not describe a valid connection. \
                   Examples are a missing `driver`, a setting of another driver, or a `time_zone` \
                   that is not an offset.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The host name does not resolve, or the connection, the TLS handshake or the \
                   login failed. It is also thrown when this CPU core already uses \
                   `[db.<name>.pool] max` connections with that name and none is free within \
                   `acquire`. When the server sends its own error message, the error includes it.",
        },
    ],
};

/// `Core\Db::open`'s reference card — `rule:core-api/reference-card`.
///
/// `shape` is filled here and empty on every other card in this module,
/// because this is the one parameter that is a written shape rather than a
/// value: [`SETTINGS`]' merged key list, in the order the ABI flattens it.
pub(super) const OPEN_DOC: MethodDoc = MethodDoc {
    short: "Opens a connection to a server that the program names itself. Use it when a \
            `[db.<name>]` block does not work. Examples are a customer whose database is stored \
            as a row in another database, or an admin tool where a person types a host. It \
            needs the `db.open` capability for that host. The address is fully checked against \
            the blocked address ranges. This check is stricter than for `connect`.",
    params: &[
        ParamDoc {
            name: "settings",
            desc: "All settings of the connection. The object has one of two shapes, and \
                   `driver` selects the shape. Four of the five databases need a host, and \
                   SQLite needs a file path.",
            shape: &[
                ShapeKeyDoc {
                    key: "driver",
                    ty: "Driver",
                    desc: "Which database this is. It selects which of the two shapes the rest \
                           of the object must have.",
                },
                ShapeKeyDoc {
                    key: "host",
                    ty: "string",
                    desc: "The server to connect to. Its certificate is checked against this \
                           name. It cannot be `tainted`, and no check can remove that, because \
                           no check on a string proves that a host is safe for a password.",
                },
                ShapeKeyDoc {
                    key: "port",
                    ty: "uint",
                    desc: "The port to connect to. Without it, the driver's default is used: \
                           5432 for PostgreSQL, and 3306 for MySQL and MariaDB.",
                },
                ShapeKeyDoc {
                    key: "database",
                    ty: "tainted string",
                    desc: "The database or schema to use. It may be `tainted`, because it is \
                           sent as a separate value and never read as SQL.",
                },
                ShapeKeyDoc {
                    key: "user",
                    ty: "tainted string",
                    desc: "The role to log in as. It may be `tainted` for the same reason.",
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
                    desc: "How much of the certificate is checked. Only `VerifyFull` works, and \
                           it is the default. The three weaker modes throw an error.",
                },
                ShapeKeyDoc {
                    key: "timeZone",
                    ty: "Core\\Time\\Zone",
                    desc: "The time zone for reading a column that has no time zone. The server \
                           is also told to use it. The default is UTC.",
                },
                ShapeKeyDoc {
                    key: "timeout",
                    ty: "Core\\Time\\Duration",
                    desc: "How long opening the connection may take, including the DNS lookup \
                           and TLS.",
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
                    desc: "The SQLite file, used in place of a host. It cannot be `tainted`, \
                           and it also needs the `fs.read` and `fs.write` capabilities.",
                },
            ],
        },
        ParamDoc {
            name: "shared",
            desc: "Whether this call may return the connection that an earlier call opened with \
                   the same settings. `false` opens a separate connection.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Db\\Connection`. It is closed when the request ends. Two calls with the same \
          value in every setting return the same object, unless `shared` is `false`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`db.open` does not grant the host, or the host's address is in a private range \
                   that `net.internal` does not allow. It is also thrown when this build cannot \
                   open a connection with these settings, or when `tls` names a mode weaker than \
                   `VerifyFull`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The host name does not resolve, or the connection, the TLS handshake or the \
                   login failed. It is also thrown when this CPU core already uses its `max` \
                   connections with these settings and none became free in time.",
        },
    ],
};

/// `Core\Db::inList`'s reference card — `rule:core-api/reference-card`.
pub(super) const IN_LIST_DOC: MethodDoc = MethodDoc {
    short: "Binds `$values` as a list of values. The placeholder it is bound to becomes a list \
            in parentheses with one placeholder per value, such as `IN (?, ?, ?)`. Nothing else \
            in a statement changes the SQL text.",
    params: &[ParamDoc {
        name: "values",
        desc: "The values to bind, one placeholder each, in the order of the array. Keys are \
               ignored: the values fill positions inside one placeholder and have no names.",
        shape: &[],
    }],
    ret: "A `Core\\Db\\InList` to bind to a single placeholder. It has no methods, and you can \
          use it only as a bound parameter. Two lists of different lengths with the same SQL \
          are two entries in the statement cache. The expanded statement has a different \
          number of placeholders.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$values` is empty. An empty list means \"match nothing\" inside `IN` and \"match \
               everything\" inside `NOT IN`. Check for an empty list in your code instead.",
    }],
};

/// `Core\Db::quoteIdentifier`'s reference card — `rule:core-api/reference-card`.
pub(super) const QUOTE_IDENTIFIER_DOC: MethodDoc = MethodDoc {
    short: "Checks that `$name` is a plain SQL identifier: a letter or `_`, then letters, digits or \
            `_`. It returns the name unchanged and no longer `tainted`, so you can write it into the \
            text of a statement. Use it for a table or column name. It checks the syntax only. A \
            visitor can still name any table or column, such as `users`. If a visitor \
            chooses the name, check it against a list of allowed names first.",
    params: &[ParamDoc {
        name: "name",
        desc: "The identifier to check. It is returned exactly as given. Nothing is escaped, \
               shortened or lower-cased, because a name that passes the check needs none of it.",
        shape: &[],
    }],
    ret: "The same text, without the `tainted` qualifier. It has no quotes around it. \
          This method has no connection, so it has no SQL dialect. The five databases use \
          different quote characters.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$name` is empty, starts with a digit, or contains a character that is not a \
               letter, a digit or `_`. This includes a name that is valid only inside quotes. \
               It is also thrown when the database reads `$name` as a value without quotes, \
               such as `current_user`, `user` or `current_timestamp`, in upper or lower case.",
    }],
};

/// `Core\Db\Connection::query`'s reference card — `rule:core-api/reference-card`.
pub(super) const QUERY_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement with its values bound, and reads every row it returns into memory. \
            Every statement is prepared, so there is no separate prepare step. The connection is \
            free again as soon as this returns. `stream` keeps the connection while you read.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, with a `?` or a `:name` for each value. Never write a value \
                   into the text. A `tainted` string does not compile here, and no escape \
                   function can make one allowed.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind. Use list keys for `?` and string keys for `:name`, and \
                   never both in one array. A `Core\\Db::inList` element expands into several \
                   placeholders at its position. Nothing else expands.",
            shape: &[],
        },
        TIMEOUT_PARAM_DOC,
    ],
    ret: "A `Core\\Db\\Rows<Core\\Db\\Row>` with every row the statement returned, in the \
          server's order. A statement that returns no rows, such as an `update` or a \
          `create table`, gives an empty result. It is not an error.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong, not the database. The placeholders and the array use \
                   different styles or different counts, or a `:name` has no element. It is also \
                   thrown for an element that cannot be bound, such as an array or an object \
                   that is not an `inList`. A statement already streaming on this connection \
                   throws it too.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server rejected the statement, for example for a syntax error, a \
                   constraint or a permission. The error has the server's `SQLSTATE` and \
                   message. It is also thrown for a column type this driver cannot read yet.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the statement was running, or `timeout` passed \
                   before it finished. The connection cannot be used for the rest of the \
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
    desc: "How long this statement may take. The limit covers the whole statement: the \
           prepare, the execution and every row of the result. When the time passes, the \
           statement stops with an `IOError`. The connection cannot be used again, because it \
           stopped in the middle of a message. Without it, the statement waits as long as the \
           server takes.",
    shape: &[],
};

/// `Core\Db\Connection::queryAs`'s reference card — `rule:core-api/reference-card`.
pub(super) const QUERY_AS_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement as `query` does and returns each row as an object of the class \
            written at the call site. `#[Db\\Derive]` generates the mapping from the class's \
            declared properties.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, bound the same way as in `query`. Use a `?` or a `:name` for \
                   each value, and never write a value into the text. A `tainted` string does \
                   not compile here.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind, with the same rule as `query`: one array, with list keys \
                   for `?` and string keys for `:name`.",
            shape: &[],
        },
        TIMEOUT_PARAM_DOC,
    ],
    ret: "A `Core\\Db\\Rows<T>` with one `T` per row, in the server's order. Each object is \
          created when you read its row with `all`, `first` or a `foreach`. If you only count \
          the rows, no object is created.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The type argument is an `array<...>`. A result is already a list of arrays, \
                   one per row. It is also thrown when `T` has no `#[Db\\Derive]`, so there is no \
                   column mapping to create it from. Both errors come from the call itself, but \
                   they are found when the program runs.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "A row did not match `T`. A column is missing, or has a different type than \
                   the property. A SQL NULL is in a property that is not `?T`, or a property's \
                   type has no column mapping. Every bad column of the row is reported at once \
                   in `issues`, and each `path` is the column's name.",
        },
    ],
};

/// `Core\Db\Connection::execute`'s reference card — `rule:core-api/reference-card`.
pub(super) const EXECUTE_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement that changes data, such as an `insert`, an `update`, a `delete` or a \
            `create table`. It returns how many rows changed and the new key, if there is one. The \
            values are bound the same way `query` binds them.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, with a `?` or a `:name` for each value. Never write a value \
                   into the text. A `tainted` string does not compile here, and no escape \
                   function can make one allowed.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind. Use list keys for `?` and string keys for `:name`, and \
                   never both in one array. This is the same rule as in `query`.",
            shape: &[],
        },
        TIMEOUT_PARAM_DOC,
    ],
    ret: "A `Core\\Db\\Write` with the number of affected rows as the server reported it, and \
          the id that a `RETURNING` clause returned. Any rows the statement returned are read \
          and discarded, so the connection is free again. Use `query` to keep rows.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong, not the database. The placeholders and the array use \
                   different styles or different counts, or a `:name` has no element. It is also \
                   thrown for an element that cannot be bound, such as an array or an object \
                   that is not an `inList`. A statement already streaming on this connection \
                   throws it too.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server rejected the statement, for example for a syntax error, a \
                   constraint or a permission. The error has the server's `SQLSTATE` and \
                   message.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the statement was running, or `timeout` passed \
                   before it finished. The connection cannot be used for the rest of the \
                   request.",
        },
    ],
};

/// `Core\Db\Connection::executeMany`'s reference card — `rule:core-api/reference-card`.
pub(super) const EXECUTE_MANY_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement once for each set of values and returns how many rows the whole \
            batch wrote. The statement is prepared once, and all sets go to the server in one \
            round trip.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, written once and bound once for each set. A `tainted` string \
                   does not compile here, as in `execute`. All sets use this one text.",
            shape: &[],
        },
        ParamDoc {
            name: "sets",
            desc: "One `$params` array for each run, each with keys as `execute` needs them. All \
                   sets must bind the same number of values. A set whose `inList` has a \
                   different length is a different statement.",
            shape: &[],
        },
        TIMEOUT_PARAM_DOC,
    ],
    ret: "The sum of the counts each run reported. A command that reports no count adds \
          nothing. An empty `$sets` writes nothing and returns `0`. Rows from a `RETURNING` \
          clause are discarded. There is no `lastId`, because no single run owns it.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong, not the database. A set uses both list keys and string \
                   keys, two sets bind different numbers of values, or an element cannot be \
                   bound. A statement already streaming on this connection throws it too.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server rejected one run, for example for a syntax error, a constraint or \
                   a permission. Each run is its own transaction, so the writes before the \
                   failed run stay. Use `transaction` to save all or nothing.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the batch was running, or `timeout` passed before \
                   it finished. The connection cannot be used for the rest of the request.",
        },
    ],
};

/// `Core\Db\Queryable::stream`'s reference card — `rule:core-api/reference-card`.
pub(super) const STREAM_DOC: MethodDoc = MethodDoc {
    short: "Runs one statement and gives its rows one at a time to a `foreach` loop. The connection \
            stays open until the loop ends. The memory used stays the same however many rows \
            there are. Use it instead of `query` for a large result.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, bound the same way as in `query`. Use a `?` or a `:name` for \
                   each value, and never write a value into the text. A `tainted` string does \
                   not compile here.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind, with the same rule as `query`. Use list keys for `?` and \
                   string keys for `:name`, and never both in one array.",
            shape: &[],
        },
        ParamDoc {
            name: "timeout",
            desc: "How long reading all the rows may take. The limit applies while the rows are \
                   being read, so it covers every row and not only the first call. When the time \
                   passes, the statement stops with an `IOError`, and the connection cannot be \
                   used again. Without it, the loop waits as long as the server takes.",
            shape: &[],
        },
    ],
    ret: "The statement's rows for a `foreach` loop, each one a `Core\\Db\\Row`, in the server's \
          order. No row has been read yet when this returns. The connection is busy until the \
          loop reaches the last row, and no other statement can run on it. A loop that writes \
          for each row needs a second connection (`{ shared: false }`), or `query` instead.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong, not the database. The placeholders and the array use \
                   different styles or different counts, a `:name` has no element, or an element \
                   cannot be bound. It is also thrown when the connection is closed, or when a \
                   statement is already streaming on it.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server rejected the statement, at the start or while the rows were read. \
                   The error has the server's `SQLSTATE` and message. It is also thrown for a \
                   column type this driver cannot read yet.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the statement or one of its rows was being sent, \
                   or `timeout` passed before the loop ended. The connection cannot be used for \
                   the rest of the request.",
        },
    ],
};

/// `Core\Db\Queryable::streamAs`'s reference card — `rule:core-api/reference-card`.
pub(super) const STREAM_AS_DOC: MethodDoc = MethodDoc {
    short: "Reads a result one row at a time as `stream` does, and builds each row into an object \
            of the class written at the call site, as `queryAs` does. The memory used stays the \
            same however many rows there are.",
    params: &[
        ParamDoc {
            name: "sql",
            desc: "The statement, bound the same way as in `query`. Use a `?` or a `:name` for \
                   each value, and never write a value into the text. A `tainted` string does \
                   not compile here.",
            shape: &[],
        },
        ParamDoc {
            name: "params",
            desc: "The values to bind, with the same rule as `query`. Use list keys for `?` and \
                   string keys for `:name`, and never both in one array.",
            shape: &[],
        },
        ParamDoc {
            name: "timeout",
            desc: "How long reading all the rows may take, with the same rule as `stream`. The \
                   limit applies while the rows are being read, so it covers every row and not \
                   only the first call. Without it, the loop waits as long as the server takes.",
            shape: &[],
        },
    ],
    ret: "The statement's rows for a `foreach` loop, each one created as a `T`, in the server's \
          order. No row has been read yet when this returns. The connection is busy, as with \
          `stream`. Only one `T` exists at a time. That is the only difference from `queryAs`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call is wrong, not the database. The placeholders and the array use \
                   different styles or different counts, a `:name` has no element, or an element \
                   cannot be bound. It is also thrown when the connection is closed, or when a \
                   statement is already streaming on it. A `T` without `#[Db\\Derive]` also \
                   throws it here, although the compiler already reports that.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "A row did not match `T`. A column is missing, or has a different type than \
                   the property. A SQL NULL is in a property that is not `?T`, or a property's \
                   type has no column mapping. Every bad column of that row is reported at once \
                   in `issues`, and each `path` is the column's name. The loop stops there, \
                   because the rest of the result is not read.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server rejected the statement, at the start or while the rows were read. \
                   The error has the server's `SQLSTATE` and message. It is also thrown for a \
                   column type this driver cannot read yet.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the statement or one of its rows was being sent, \
                   or `timeout` passed before the loop ended. The connection cannot be used for \
                   the rest of the request.",
        },
    ],
};

/// `Core\Db\Queryable::transaction`'s reference card — `rule:core-api/reference-card`.
pub(super) const TRANSACTION_DOC: MethodDoc = MethodDoc {
    short: "Runs `$fn` inside a transaction and returns what `$fn` returns. If `$fn` returns, the \
            transaction is committed. If `$fn` throws, the transaction is rolled back and the error \
            is thrown again. An early return cannot leave the transaction open.",
    params: &[
        ParamDoc {
            name: "fn",
            desc: "The work to run. It gets a `Core\\Db\\Transaction`, which has the same query \
                   methods as the connection. The function may declare that parameter or no \
                   parameter.",
            shape: &[],
        },
        ParamDoc {
            name: "isolation",
            desc: "How much this transaction can see of other transactions running at the same \
                   time. Without it, the transaction uses the level set on the server. A nested \
                   call cannot set it. The level is for the whole transaction, not for a \
                   savepoint inside it.",
            shape: &[],
        },
        ParamDoc {
            name: "readOnly",
            desc: "Writes are not allowed during the transaction, so the server can plan for \
                   reads only. The default is `false`. A nested call cannot set it, for the same \
                   reason as `isolation`.",
            shape: &[],
        },
        ParamDoc {
            name: "retries",
            desc: "How many times `$fn` runs again when the commit reports a deadlock or a \
                   serialization failure. This applies to the outermost transaction only. The \
                   default is `0`, because a function with side effects should run again only \
                   when you ask for it. Nothing else is retried, and there is no wait between \
                   attempts. A conflict from a statement inside `$fn` is thrown and not retried.",
            shape: &[],
        },
    ],
    ret: "What `$fn` returned, after the commit. A nested call on the same connection is a \
          savepoint. So you can call a function that wraps its own writes in a transaction \
          from inside another transaction.",
    errors: &[
        ErrorDoc {
            error: "Core\\Db\\RolledBack",
            desc: "`$fn` called `rollBack`. This call throws it even if code inside `$fn` caught \
                   it, because the rollback is recorded on the transaction itself.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "A statement inside `$fn` was not allowed because of the way it was written. It \
                   is also thrown when the transaction is used after its `transaction()` call \
                   returned, or when a nested call sets its own `isolation` or `readOnly`.",
        },
        ErrorDoc {
            error: "Core\\Db\\DbError",
            desc: "The server rejected the `BEGIN`, or rejected the `COMMIT` after `$fn` \
                   returned, for example for a serialization failure or a deferred constraint. \
                   In both cases nothing is committed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while the transaction was open. The connection cannot be \
                   used for the rest of the request.",
        },
    ],
};

/// `Core\Db\Transaction::rollBack`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROLL_BACK_DOC: MethodDoc = MethodDoc {
    short: "Rolls back this transaction. It records `$reason` and throws `Core\\Db\\RolledBack` \
            with it. You cannot request a rollback and then continue in the same transaction. \
            So no code has to remember to check a flag such as `setRollbackOnly`.",
    params: &[ParamDoc {
        name: "reason",
        desc: "Why the work is stopped. It becomes the `reason` property and the message of the \
               thrown `RolledBack`, so write it for the person who reads the error.",
        shape: &[],
    }],
    ret: "Nothing. This method always throws.",
    errors: &[
        ErrorDoc {
            error: "Core\\Db\\RolledBack",
            desc: "Always. The `transaction()` call throws it even if code between here and \
                   there catches it, because that call checks the recorded reason, not the \
                   thrown error.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "The transaction is used after its `transaction()` call returned, so there is \
                   nothing left to roll back.",
        },
    ],
};

/// `Core\Db\Rows::all`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_ALL_DOC: MethodDoc = MethodDoc {
    short: "Returns every row of the result, in the order the server sent them.",
    params: &[],
    ret: "An `array<T>`. It is empty for a statement that returned no rows. `T` is a \
          `Core\\Db\\Row` for `query`, and the class you named for `queryAs<T>`. The rows are \
          already read, so this creates one object for each row and decodes nothing again.",
    errors: &[],
};

/// `Core\Db\Rows::first`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_FIRST_DOC: MethodDoc = MethodDoc {
    short: "Returns the first row, or `null` if there is none. A second call returns the same row.",
    params: &[],
    ret: "A `T`, as `all` describes it, or `null` for an empty result. Everywhere in `Core`, \
          `?T` means that a value can be missing. A query that matched nothing is not an error.",
    errors: &[],
};

/// `Core\Db\Rows::value`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_VALUE_DOC: MethodDoc = MethodDoc {
    short: "Returns the first column of the first row. Use it to read the result of a `select \
            count(*)`.",
    params: &[],
    ret: "That column's value, or `null` if the result has no rows. A NULL column also gives \
          `null`, because the declared type is `mixed`. To tell the two apart, call `count()` \
          first.",
    errors: &[],
};

/// `Core\Db\Rows::column`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_COLUMN_DOC: MethodDoc = MethodDoc {
    short: "Returns the value of one column from every row, in the order the server sent them.",
    params: &[ParamDoc {
        name: "key",
        desc: "The column. An `int` is its position in the server's column list, counted from \
               zero. A `string` is its label.",
        shape: &[],
    }],
    ret: "An `array<mixed>` with one entry per row, empty for a result with no rows.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "No column has that name, or the position is negative or past the last column. An \
               empty result returns an empty array and does not throw.",
    }],
};

/// `Core\Db\Rows::count`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_COUNT_DOC: MethodDoc = MethodDoc {
    short: "Returns how many rows the statement returned. For the number of rows a write changed, \
            use `Core\\Db\\Write::affected`.",
    params: &[],
    ret: "The number of rows in the result. It is exact, because `query` reads all rows before \
          it returns.",
    errors: &[],
};

/// `Core\Db\Rows::columns`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROWS_COLUMNS_DOC: MethodDoc = MethodDoc {
    short: "Returns a description of each column of the result, as one `Core\\Db\\Column` per \
            column. The columns are in the order the server sent them.",
    params: &[],
    ret: "The columns. An empty result has them too. A `select` that matched nothing still \
          describes the columns it would return, so you can read them before any row.",
    errors: &[],
};

/// `Core\Db\Column::name`'s reference card — `rule:core-api/reference-card`.
pub(super) const COLUMN_LABEL_DOC: MethodDoc = MethodDoc {
    short: "Returns the column's label as the server described it. If the `select` list gives the column an alias, the \
            label is the alias.",
    params: &[],
    ret: "The label. It is not a unique key: `select a, a` gives two columns with one label. So \
          read a column by its position in the array that `columns()` returns.",
    errors: &[],
};

/// `Core\Db\Column::type`'s reference card — `rule:core-api/reference-card`.
pub(super) const COLUMN_DECLARED_DOC: MethodDoc = MethodDoc {
    short: "Returns the type the column was declared with, as a `Core\\Db\\Column::Type` case. \
            Each database has its own type names, but the cases are the same for every \
            database.",
    params: &[],
    ret: "The case. It describes the column, not the value you get when you read it. For \
          example, a `JSON` column and a `TEXT` column both read as `tainted string`, and this \
          case tells them apart. A type with no matching Novis type is `Other`. This includes \
          every array and, on PostgreSQL, every `ENUM`.",
    errors: &[],
};

/// `Core\Db\Column::nullable`'s reference card — `rule:core-api/reference-card`.
pub(super) const COLUMN_NULLABLE_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the column can contain NULL. On PostgreSQL this is always `true`. The \
            server does not send a NOT NULL flag with the result. Only an extra catalog query \
            for each statement could find it, and this driver does not run one.",
    params: &[],
    ret: "`true` for every column this driver describes, so treat every column as nullable. \
          The typed methods of `Core\\Db\\Row` already do this, and each returns `?T`. A \
          database that sends the flag gives its value here.",
    errors: &[],
};

/// `Core\Db\Row::has`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_HAS_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the row has a column with this name. Call it first when a method \
            would throw an error for an unknown column.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it. It may be `tainted`, because this \
               method returns a result and does not throw.",
        shape: &[],
    }],
    ret: "`true` if the row has the column, whatever its value. A NULL column is present. \
          `false` otherwise.",
    errors: &[],
};

/// `Core\Db\Row::get`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_GET_DOC: MethodDoc = MethodDoc {
    short: "Returns one column's value, as whatever Novis type the SQL type maps to. It reads \
            any column. Use `as` to narrow the value when no typed method fits.",
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
    short: "Returns the whole row as an array with string keys, in the server's column order.",
    params: &[],
    ret: "An `array<mixed>` with the column labels as keys. A NULL column is a `null` entry, \
          and the key is present.",
    errors: &[],
};

/// `Core\Db\Row::string`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_STRING_DOC: MethodDoc = MethodDoc {
    short: "Returns one column as `string`. It reads text columns only: `CHAR`, `VARCHAR`, \
            `TEXT`, `ENUM` and `JSON`. Each gives a `tainted string`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The text, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not text. A `BYTEA` is \
               `bytes`, so read it with `->bytes`. A number is not converted to text here.",
    }],
};

/// `Core\Db\Row::bytes`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns one column as `bytes`. It reads `BINARY`, `BLOB` and `BYTEA` columns. These \
            have no text form, and `bytes` is a separate type from `string`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The bytes as `tainted bytes`, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is text. Text and `bytes` \
               are separate types, and this method reads only `bytes`.",
    }],
};

/// `Core\Db\Row::int`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_INT_DOC: MethodDoc = MethodDoc {
    short: "Returns one column as `int`. It reads `SMALLINT`, `INT` and `BIGINT`, and an \
            unsigned column whose value fits.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The integer, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, the column is not an integer, or it is an \
               unsigned value larger than the largest `int`.",
    }],
};

/// `Core\Db\Row::uint`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_UINT_DOC: MethodDoc = MethodDoc {
    short: "Returns one column as `uint`. It reads an `UNSIGNED` integer on MySQL and MariaDB, \
            and a signed column that is not negative.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The integer, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, the column is not an integer, or its value \
               is negative. A negative value cannot be converted to `uint`.",
    }],
};

/// `Core\Db\Row::float`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_FLOAT_DOC: MethodDoc = MethodDoc {
    short: "Returns one column as `float`. It reads only `FLOAT`, `REAL` and `DOUBLE`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The number, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not a floating-point \
               column. A `DECIMAL` is exact, so read it with `->decimal`. Binary floating point \
               gives wrong sums for money.",
    }],
};

/// `Core\Db\Row::bool`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_BOOL_DOC: MethodDoc = MethodDoc {
    short: "Returns one column as `bool`. It reads `BOOLEAN`, `BIT(1)`, and an integer column \
            that contains `0` or `1`. Use it to read a `TINYINT(1)` on MySQL and MariaDB. That \
            column is an `int`, and this method converts it.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The truth value, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is neither boolean nor an \
               integer. It is also thrown for an integer other than `0` or `1`. A stored `7` \
               throws this error.",
    }],
};

/// `Core\Db\Row::decimal`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_DECIMAL_DOC: MethodDoc = MethodDoc {
    short: "Returns one column as an exact `decimal`. It reads `DECIMAL`, `NUMERIC` and \
            `MONEY` columns.",
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
    short: "Returns one column as a `Core\\Time\\Instant`. It reads `TIMESTAMPTZ` and \
            `datetimeoffset`, the two types that store their own time zone.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The instant, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not a timestamp with a \
               time zone. A timestamp without a time zone is a `Core\\Time\\DateTime` in the \
               time zone set for the connection.",
    }],
};

/// `Core\Db\Row::date`'s reference card — `rule:core-api/reference-card`.
pub(super) const ROW_DATE_DOC: MethodDoc = MethodDoc {
    short: "Returns one column as a `Core\\Time\\Date`. It reads a `DATE`, which is a calendar \
            day with no time.",
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
    short: "Returns one column as a `Core\\Time\\TimeOfDay`. It reads a `TIME`, which is a time \
            of day with no date.",
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
    short: "Returns one column as a `Core\\Uuid`. It reads PostgreSQL's `UUID`, SQL Server's \
            `uniqueidentifier` and the `UUID` of MariaDB 10.7 and later. MySQL stores a UUID as \
            `BINARY(16)`, and that column is `bytes`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The column label, as the server described it.",
        shape: &[],
    }],
    ret: "The identifier, or `null` for a NULL column.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The row has no column with that name, or the column is not a UUID type. A \
               `BINARY(16)` is `bytes`, and a UUID stored as text is a `string`.",
    }],
};

/// `Core\Db\Write::affected`'s reference card — `rule:core-api/reference-card`.
pub(super) const WRITE_AFFECTED_DOC: MethodDoc = MethodDoc {
    short: "Returns how many rows the statement changed.",
    params: &[],
    ret: "A `uint`. It is `0` for a statement that changed no rows. It is also `0` for a \
          statement type that reports no count, such as `create table`. Use `changed` to tell \
          the two apart.",
    errors: &[],
};

/// `Core\Db\Write::changed`'s reference card — `rule:core-api/reference-card`.
pub(super) const WRITE_CHANGED_DOC: MethodDoc = MethodDoc {
    short: "Returns the same count, exactly as the server reported it. It is `null` when this \
            type of statement has no row count. `affected` cannot show that case.",
    params: &[],
    ret: "A `?uint`. When it is not `null`, it is equal to `affected`. MySQL counts matched rows \
          and changed rows separately. PostgreSQL does not send that difference, so there is no \
          second count. A second count that always matched the first would mislead programs.",
    errors: &[],
};

/// `Core\Db\Write::lastId`'s reference card — `rule:core-api/reference-card`.
pub(super) const WRITE_LAST_ID_DOC: MethodDoc = MethodDoc {
    short: "Returns the key that this statement generated. The key belongs to this statement, so \
            another statement on the same connection cannot change it.",
    params: &[],
    ret: "A `?uint`: the key this statement generated, or `null` if it generated none. A \
          statement that inserted no row returns `null`. PostgreSQL has no last insert id, so \
          the key comes from a `RETURNING` clause. It is the first column of the last row \
          returned, if that column is declared as an integer. On MySQL it is the \
          `AUTO_INCREMENT` value the server reports for this write. On SQLite it is the rowid \
          of a row this statement inserted, which may be `0`. SQL Server always returns \
          `null`, because the server does not send a generated key.",
    errors: &[],
};
