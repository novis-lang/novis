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
//! 2. **Only PostgreSQL opens.** A block naming another driver is refused by
//!    `nvs_db::PgTarget::resolve` with the message that names the driver it is,
//!    which is the honest answer while `nvs_db::Connection`'s other four
//!    variants have no connect path behind them.
//! 3. **A connection is never reused across requests.** ADR 0067 § 13's
//!    per-core pool is what would change that, and it may only do so behind
//!    that section's reset; [`nvs_runtime::Ctx::hold_open_connection`] is where
//!    that is written down.
//! 4. **`Db\DbError` is not in spec § 10's tree yet**, so a refusal here is a
//!    plain `RuntimeError` or an `IOError` and carries no `kind`, `sqlState`
//!    or `constraint`. Nothing about the messages changes when it lands; what
//!    changes is what a `catch` can name. `Db\RolledBack` *is* in the tree
//!    (`nvs_hir::errors::TREE`), with the `reason` its message fills, and
//!    `nvs_runtime::ThrownClass::DbRolledBack` is what a helper names to raise
//!    one — nothing in this module raises one yet, because § 7's
//!    `transaction` is what would.
//! 5. **`query`, `execute`, `executeMany` and `transaction` are what has landed
//!    of `Core\Db\Queryable`.** `queryAs`, `stream` and `streamAs` are owed, and
//!    so are `close` and § 18's three readonly properties on `Connection`. On
//!    the result side [`ROWS`] owes one member of six —
//!    `columns(): array<Column>`, which needs three things at once: a
//!    `Core\Db\Column` class, a `Core\ColumnType` enum for § 18's own fourteen
//!    cases, and a classification of a `PgColumn`'s type OID that `nvs-db` does
//!    not expose (`PgColumn::decode` maps an OID to a *value*, which is a
//!    different question from what a NULL column's declared type is).
//! 6. **§ 9's five structured rows do not read back.** A `DATE`, `TIME`,
//!    `TIMESTAMP`, `TIMESTAMPTZ` or `UUID` column is a `Core\Time` or
//!    `Core\Uuid` *instance*, which only this crate can allocate;
//!    [`structured_column`] refuses one by name in the *decoder*. `Core\Db\Row`
//!    has carried the four readers that would answer with them since its own
//!    members landed, so what is left is building the instance from
//!    [`nvs_db::PgScalar`]'s parsed components. Every other row of that table
//!    decodes now.
//! 7. **Neither `query` nor `execute` declares a `{timeout?: Duration}`.**
//!    § 4's option is in both spec signatures and is deliberately in neither
//!    registry row, for one reason on both: a deadline
//!    on a statement has to reach the socket the way
//!    [`nvs_db::PgConn::connect`]'s does, and there is no seam for one on the
//!    statement path yet. An option that parsed and did nothing would be worse
//!    than its absence, which the compiler can at least report.
//! 8. **A delimiting quoter, if one is ever wanted, belongs on `Connection`**
//!    and not here — that is the only place a dialect exists. § 18 does not ask
//!    for one, and this module's second decision above is why adding it to
//!    `Core\Db` cannot be the answer.

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

/// The one slot a [`ROWS`] holds: every row the statement answered, in the
/// server's order, each one a string-keyed array of its own columns.
const ROWS_SLOT: &str = "rows";

/// Where [`ROWS_SLOT`] sits, for the six members that read it back.
const ROWS_AT: usize = 0;

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

/// Where [`VALUES_SLOT`] sits inside an [`IN_LIST`], for the bind that expands
/// it.
const VALUES_AT: usize = 0;

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
/// `Core\Db\Queryable`'s landed members and the rest are owed**: `queryAs`,
/// `stream` and `streamAs`, plus `close` and § 18's three readonly properties.
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

/// ADR 0067 § 7's `transaction`, written once because it is declared once: the
/// row is `Core\Db\Queryable`'s and both [`CONNECTION`] and [`TRANSACTION`]
/// carry it, a nested call on the second being the savepoint § 7 asks for.
///
/// **The options bag is owed and its absence is a subset, not a divergence.**
/// § 7's `{isolation?, readOnly?, retries?}` needs a `Core\Db\Isolation` enum
/// this registry has no row for and a retry that suspends the coroutine; what
/// is here is the shape with all three at their § 7 defaults — the driver's own
/// isolation, read-write, and no retries, which is the default § 7 argues for
/// because re-running a closure that sends mail is worse than surfacing the
/// conflict. This module's known gaps carry it.
const TRANSACTION_ROW: CoreMethod = CoreMethod {
    name: "transaction",
    names: &["fn"],
    // Opaque, as ADR 0031 § 4 keeps every `callable`: what this one is handed
    // is a [`TRANSACTION`] and what it may declare is zero parameters or one,
    // and neither is sayable here — `nvs_runtime::call_closure` trims to the
    // arity the closure recorded, which is § 7's R9 allowance.
    params: &[CoreTy::CallableTo("T")],
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

/// Spec § 18's `Core\Db\Rows` — what a buffered statement answers with.
///
/// What the slot holds is the whole of what the members read, decided here
/// rather than left to them: **every row, already decoded, each as a
/// string-keyed array of its columns** — which is `Row::toArray`'s own shape, so
/// every member below is a reader over it and never a second decoder.
///
/// **Five of § 18's six members, and `columns()` is the one owed.** It answers
/// `array<Column>`, which needs three things this slot has not got: a
/// `Core\Db\Column`, a `Core\ColumnType` enum for spec § 18's own fourteen
/// cases, and a classification of a `PgColumn`'s type OID that `nvs-db` does not
/// yet expose. This module's known gap 5 is that list.
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
    ],
    slots: &[ROWS_SLOT],
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
/// **Four of the eleven cannot yet answer anything but their refusal**, because
/// `instant`, `date`, `time` and `uuid` read back a `Core\Time`/`Core\Uuid`
/// instance and [`nvs_core_db_connection_query`] throws on the five columns that
/// would carry one ([`structured_column`]). They are written as the lookups they
/// will always be rather than left out, so that landing § 9's structured columns
/// changes the decoder and not this class.
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
            error: "RuntimeError",
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
            error: "RuntimeError",
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
            error: "RuntimeError",
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
    params: &[ParamDoc {
        name: "fn",
        desc: "The work. It is handed a `Core\\Db\\Transaction`, which has the same query surface \
               the connection has, and may declare that parameter or no parameter at all. Called \
               once — retries are not on by default, because a closure with side effects should \
               not be re-run without being asked for.",
        shape: &[],
    }],
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
            desc: "A statement inside the closure was refused for the way it was written, or the \
                   transaction was reached after the call that owned it returned.",
        },
        ErrorDoc {
            error: "RuntimeError",
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
/// the certificate is checked against stays the written host, which is
/// `PgTarget::host` and not this.
///
/// # Errors
///
/// A thrown `IOError` for a host that resolves to nothing.
fn address_of(host: &str, port: Option<u16>, name: &str) -> Result<SocketAddr, Fault> {
    let port = port.unwrap_or(nvs_db::pg::DEFAULT_PORT);
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

        let held = if shared {
            ctx.memoized_connection(&name)
        } else {
            None
        };
        if let Some(key) = held {
            return Ok(crate::instance::build(
                &CONNECTION,
                [Value::uint(key), Value::str(NvsStr::new(name.as_bytes()))],
            ));
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
                    "{CONNECT}: this program is running with no configuration at all, so there is \
                     no `[db.{name}]` block to open"
                ))
            })?;
        let block = snapshot.config.db.get(&name).ok_or_else(|| {
            Fault::thrown(format!(
                "{CONNECT}: `db.connect` grants `{name}`, and no `[db.{name}]` block sets the \
                 connection up — the grant names a block an operator has not written yet"
            ))
        })?;
        let target = nvs_db::PgTarget::resolve(block)
            .map_err(|refused| Fault::thrown(format!("{CONNECT}: {}", refused.refusal(&name))))?;
        let address = address_of(target.host, block.port, &name)?;
        let opened = nvs_db::PgConn::connect(address, &target, deadline).map_err(|err| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{CONNECT}: `[db.{name}]` at {address} did not open: {err}"),
            )
        })?;
        let key = ctx.hold_open_connection(
            shared.then(|| name.clone()),
            Box::new(nvs_db::Connection::Postgres(opened)),
        );
        Ok(crate::instance::build(
            &CONNECTION,
            [Value::uint(key), Value::str(NvsStr::new(name.as_bytes()))],
        ))
    }
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
/// This module's known gap 4 owns the ceiling on the middle one: `Db\DbError`
/// is not in spec § 10's tree yet, so a server refusal arrives as a plain
/// `RuntimeError` and a program cannot yet catch it by kind or read its
/// `sqlState` off the object. Nothing about the message changes when it lands.
fn statement_failure(named: &str, block: &Value, refused: &std::io::Error) -> Fault {
    let name = block.as_text().unwrap_or("?");
    match refused.kind() {
        std::io::ErrorKind::InvalidInput => {
            Fault::thrown_as(ThrownClass::Logic, format!("{named}: {refused}"))
        }
        std::io::ErrorKind::Other => Fault::thrown(format!(
            "{named}: `[db.{name}]` refused the statement: {refused}"
        )),
        _ => Fault::thrown_as(
            ThrownClass::Io,
            format!("{named}: `[db.{name}]` failed while the statement was running: {refused}"),
        ),
    }
}

/// The refusal for a column whose Novis type is one of ADR 0067 § 9's five
/// class instances — the gap `Core\Db\Row`'s typed readers close.
fn structured_column(column: &str) -> Fault {
    Fault::thrown(format!(
        "{QUERY}: the column `{column}` is a `DATE`, `TIME`, `TIMESTAMP`, `TIMESTAMPTZ` or \
         `UUID`, and ADR 0067 § 9 reads those back as `Core\\Time` and `Core\\Uuid` instances \
         rather than as text — which this decoder does not build yet, though \
         `Core\\Db\\Row`'s `date`, `time`, `instant` and `uuid` are already waiting for one. \
         Every other row of § 9's table reads back now, and a `::text` cast in the statement is \
         the way to have one of these until then"
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
    /// § 5's rewritten text, in the driver's own placeholder spelling.
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
        .map_err(|refused| statement_failure(named, &block, &refused))?;

    let bounds: Vec<&Bound> = if keys.is_empty() {
        positional.iter().collect()
    } else {
        keys.iter().map(|(_, bound)| bound).collect()
    };
    let mut rendered: Vec<Option<Vec<u8>>> = Vec::with_capacity(rewritten.binds.len());
    for source in &rewritten.binds {
        rendered.push(
            nvs_db::encode(bounds[source.arg].values[source.element])
                .map_err(|refused| statement_failure(named, &block, &refused))?,
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
        let statement = statement_of(args, "query", QUERY)?;
        let sending: Vec<Option<&[u8]>> =
            statement.binds.iter().map(|one| one.as_deref()).collect();
        let postgres = postgres_of(ctx, statement.key, &statement.block, QUERY)?;
        let mut answered = postgres
            .query(&statement.sql, &sending)
            .map_err(|refused| statement_failure(QUERY, &statement.block, &refused))?;
        // Taken before the first row: a `PgRows` lends its columns and its rows
        // out of one borrow, and the rows are read with it held mutably.
        let columns: Vec<nvs_db::PgColumn> = answered.columns().to_vec();

        let mut rows = nvs_runtime::NvsArray::new();
        loop {
            let Some(row) = answered
                .next_row()
                .map_err(|refused| statement_failure(QUERY, &statement.block, &refused))?
            else {
                break;
            };
            // Built whole before it joins the result, so that a column this
            // driver cannot read back releases the row it was half way through
            // rather than leaving it in one — `NvsArray`'s own `Drop`.
            let mut one = nvs_runtime::NvsArray::new();
            for (index, column) in columns.iter().enumerate() {
                let body = row
                    .column(index)
                    .map_err(|refused| statement_failure(QUERY, &statement.block, &refused))?;
                let value = column
                    .decode(body)
                    .map_err(|refused| statement_failure(QUERY, &statement.block, &refused))?
                    .ok_or_else(|| structured_column(&column.name))?;
                one.set(NvsStr::new(column.name.as_bytes()), value);
            }
            rows.append(Value::array(one));
        }

        Ok(crate::instance::build(&ROWS, [Value::array(rows)]))
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
    /// Nothing is decoded, which is why an `insert … returning` of a `UUID`
    /// column answers here while the same column refuses in `query`
    /// ([`structured_column`]).
    ///
    /// **Both counts come off `CommandComplete`**, so neither exists until that
    /// stream has ended, and the pair is § 4's own: `affected` folds a command
    /// whose tag carries no count at all — a `create table` — to `0`, and
    /// `changed` keeps the absence, which is the only thing the two say
    /// differently on this driver.
    fn nvs_core_db_connection_execute(ctx, args: [3]) {
        let statement = statement_of(args, "execute", EXECUTE)?;
        let sending: Vec<Option<&[u8]>> =
            statement.binds.iter().map(|one| one.as_deref()).collect();
        let postgres = postgres_of(ctx, statement.key, &statement.block, EXECUTE)?;
        let mut answered = postgres
            .query(&statement.sql, &sending)
            .map_err(|refused| statement_failure(EXECUTE, &statement.block, &refused))?;
        while answered
            .next_row()
            .map_err(|refused| statement_failure(EXECUTE, &statement.block, &refused))?
            .is_some()
        {}

        let changed = answered.affected();
        let last_id = answered.last_id();
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

        let postgres = postgres_of(ctx, batch.key, &batch.block, EXECUTE_MANY)?;
        let written = postgres
            .execute_many(&batch.sql, &sets)
            .map_err(|refused| statement_failure(EXECUTE_MANY, &batch.block, &refused))?;
        Ok(Value::uint(written))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Queryable::transaction(callable $fn): T` — ADR 0067 § 7's whole
    /// shape, and the only way to open a transaction on this surface.
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
    /// **Rolling back after a throw discards its own failure.** The exception
    /// the closure raised is what the request is about, and a connection whose
    /// `ROLLBACK` was refused is one § 13's reset destroys rather than pools —
    /// so replacing the program's exception with the driver's would lose the
    /// only half a caller can act on.
    fn nvs_core_db_connection_transaction(ctx, args: [2]) {
        let (key, block) = handle_of(args[0], "transaction")?;
        postgres_of(ctx, key, &block, TRANSACTION_MEMBER)?
            .begin(None, false)
            .map_err(|refused| statement_failure(TRANSACTION_MEMBER, &block, &refused))?;

        // The block name is handed on rather than looked up again: a
        // transaction refuses under the same `[db.<name>]` its connection does,
        // and the slot is the only place that name lives.
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
        // stored somewhere is already refusing by the time this call returns —
        // and closed on every path, which is why it is not inside a branch.
        let receiver = crate::instance::receiver(scope, &TRANSACTION, "transaction")?;
        crate::instance::set_slot(receiver, SCOPE_AT, Value::bool(false));
        let held = crate::instance::slot(receiver, REASON_AT);
        let abandoned = held.as_text().map(str::to_owned);
        discard(scope);

        let answered = match outcome {
            Ok(value) => value,
            Err(fault) => {
                if let Ok(postgres) = postgres_of(ctx, key, &block, TRANSACTION_MEMBER) {
                    let _ = postgres.roll_back();
                }
                return Err(fault);
            }
        };

        let closed = postgres_of(ctx, key, &block, TRANSACTION_MEMBER).and_then(|postgres| {
            let ended = if abandoned.is_some() {
                postgres.roll_back()
            } else {
                postgres.commit()
            };
            ended.map_err(|refused| statement_failure(TRANSACTION_MEMBER, &block, &refused))
        });

        // On two of the three paths the closure's answer is not this call's, and
        // this frame owns the only reference to it.
        match (abandoned, closed) {
            (_, Err(fault)) => {
                discard(answered);
                Err(fault)
            }
            (Some(reason), Ok(())) => {
                discard(answered);
                Err(Fault::thrown_as(ThrownClass::DbRolledBack, reason))
            }
            (None, Ok(())) => Ok(answered),
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

/// A value read out of an array, with a reference of the caller's own.
///
/// Every read below borrows — [`NvsArray::get`] and `value_at` both hand back a
/// reference the array still owns — so this is the one place the second one is
/// taken, rather than an `unsafe` block at each of the fourteen members that
/// hands a slot's value out.
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
/// written by [`nvs_core_db_connection_query`] and by nothing else, so that is a
/// paste error in this crate rather than anything a program can cause.
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
    fn nvs_core_db_rows_all(_ctx, args: [1]) {
        let rows = result_rows(args, "all")?;
        let mut all = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = rows.next_slot(from) {
            let row = rows
                .value_at(slot)
                .expect("next_slot only names live entries");
            all.append(crate::instance::build(&ROW, [owned(row)]));
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
    fn nvs_core_db_rows_iterate(_ctx, args: [1]) {
        let cursor = (|| {
            let rows = result_rows(args, nvs_runtime::sequence::ITERATE)?;
            let mut all = NvsArray::new();
            let mut from = 0usize;
            while let Some(slot) = rows.next_slot(from) {
                let row = rows
                    .value_at(slot)
                    .expect("next_slot only names live entries");
                all.append(crate::instance::build(&ROW, [owned(row)]));
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
    fn nvs_core_db_rows_first(_ctx, args: [1]) {
        let rows = result_rows(args, "first")?;
        let Some(slot) = rows.next_slot(0) else {
            return Ok(Value::null());
        };
        let row = rows
            .value_at(slot)
            .expect("next_slot only names live entries");
        Ok(crate::instance::build(&ROW, [owned(row)]))
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
    /// One of the four [`ROW`]'s docs name as refusing everything until § 9's
    /// structured columns land: [`structured_column`] is where such a column
    /// stops today, so nothing reaches this slot yet.
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
