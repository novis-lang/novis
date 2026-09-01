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
//! 4. **`Db\DbError` and `Db\RolledBack` are not in spec § 10's tree yet**, so
//!    a refusal here is a plain `RuntimeError` or an `IOError` and carries no
//!    `kind`, `sqlState` or `constraint`. Nothing about the messages changes
//!    when they land; what changes is what a `catch` can name.
//! 5. **`query` is the whole of `Core\Db\Queryable` so far.** `queryAs`,
//!    `execute`, `executeMany`, `stream`, `streamAs` and `transaction` are
//!    owed, and so are `close` and § 18's three readonly properties on
//!    `Connection`. What [`ROWS`] answers with has no members yet either.
//! 6. **§ 9's five structured rows do not read back.** A `DATE`, `TIME`,
//!    `TIMESTAMP`, `TIMESTAMPTZ` or `UUID` column is a `Core\Time` or
//!    `Core\Uuid` *instance*, which only this crate can allocate;
//!    [`structured_column`] refuses one by name until `Core\Db\Row`'s typed
//!    readers build them. Every other row of that table decodes now.
//! 7. **`query` declares no `{timeout?: Duration}`.** § 4's option is in the
//!    spec's signature and is deliberately not in the registry row: a deadline
//!    on a statement has to reach the socket the way
//!    [`nvs_db::PgConn::connect`]'s does, and there is no seam for one on the
//!    statement path yet. An option that parsed and did nothing would be worse
//!    than its absence, which the compiler can at least report.
//! 8. **A delimiting quoter, if one is ever wanted, belongs on `Connection`**
//!    and not here — that is the only place a dialect exists. § 18 does not ask
//!    for one, and this module's second decision above is why adding it to
//!    `Core\Db` cannot be the answer.

use std::net::{SocketAddr, ToSocketAddrs as _};

use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
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

/// `Core\Db\Rows`'s fully-qualified name, as [`CoreTy::Instance`] spells it.
///
/// `pub(crate)` for `registry`'s handle roster, for [`IN_LIST_NAME`]'s reason —
/// and only until § 18's `Rows` members land, which is what takes it off.
pub(crate) const ROWS_NAME: &str = r"Core\Db\Rows";

/// The one slot a [`ROWS`] holds: every row the statement answered, in the
/// server's order, each one a string-keyed array of its own columns.
const ROWS_SLOT: &str = "rows";

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
/// **`query` is `Core\Db\Queryable`'s first member and the rest are owed**:
/// `queryAs`, `execute`, `executeMany`, `stream`, `streamAs` and `transaction`,
/// plus `close` and § 18's three readonly properties. ADR 0043 makes
/// `Transaction` delegate the interface to its connection, so every one of them
/// is declared once — here — and this class is where they land.
pub(crate) const CONNECTION: CoreClass = CoreClass {
    name: CONNECTION_NAME,
    methods: &[],
    instance: &[CoreMethod {
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
        return_ty: CoreTy::Instance(ROWS_NAME),
        symbol: "nvs_core_db_connection_query",
        doc: Some(&QUERY_DOC),
    }],
    slots: &[HANDLE_SLOT, CONNECTION_NAME_SLOT],
    constants: &[],
};

/// Spec § 18's `Core\Db\Rows` — what a buffered statement answers with.
///
/// One slot and no members **yet**, so it is a handle in `registry`'s
/// `a_class_with_slots_has_instance_members_and_the_reverse` sense and is on
/// that test's roster until § 18's `all`, `first`, `value`, `column`, `count`
/// and `columns` land beside `Core\Db\Row`. What the slot holds is the whole of
/// what those members read, decided here rather than left to them: **every row,
/// already decoded, each as a string-keyed array of its columns** — which is
/// `Row::toArray`'s own shape, so the members are readers over it and never a
/// second decoder.
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
    instance: &[],
    slots: &[ROWS_SLOT],
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
    ret: "A `Core\\Db\\Rows` holding every row the statement answered, in the server's order. A \
          statement that answers none — an `update`, a `create table` — is an empty one rather \
          than a refusal.",
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

/// `Core\Db::connect`, as its own refusals spell it.
const CONNECT: &str = r"Core\Db::connect";

/// `Core\Db\Connection::query`, as its own refusals spell it.
const QUERY: &str = r"Core\Db\Connection::query";

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
fn statement_failure(block: &Value, refused: &std::io::Error) -> Fault {
    let name = block.as_text().unwrap_or("?");
    match refused.kind() {
        std::io::ErrorKind::InvalidInput => {
            Fault::thrown_as(ThrownClass::Logic, format!("{QUERY}: {refused}"))
        }
        std::io::ErrorKind::Other => Fault::thrown(format!(
            "{QUERY}: `[db.{name}]` refused the statement: {refused}"
        )),
        _ => Fault::thrown_as(
            ThrownClass::Io,
            format!("{QUERY}: `[db.{name}]` failed while the statement was running: {refused}"),
        ),
    }
}

/// The refusal for a column whose Novis type is one of ADR 0067 § 9's five
/// class instances — the gap `Core\Db\Row`'s typed readers close.
fn structured_column(column: &str) -> Fault {
    Fault::thrown(format!(
        "{QUERY}: the column `{column}` is a `DATE`, `TIME`, `TIMESTAMP`, `TIMESTAMPTZ` or \
         `UUID`, and ADR 0067 § 9 reads those back as `Core\\Time` and `Core\\Uuid` instances \
         rather than as text — which is `Core\\Db\\Row`'s typed readers and is the slice after \
         this one. Every other row of § 9's table reads back now, and a `::text` cast in the \
         statement is the way to have one of these until then"
    ))
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
        let (key, block) = connection_of(args[0], "query")?;
        // Unreachable from source: parameter 0 is a `string` in `CONNECTION`
        // above, so a non-text argument is refused at `E0401` first — the same
        // judgement `Core\Db::quoteIdentifier`'s guard states.
        let sql = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{QUERY} expected a `string` statement, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        // Unreachable for that reason too: the row declares `array<mixed>`.
        let params = args[2].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "{QUERY} expected {:?} for its parameters, got tag {}",
                Tag::Array,
                args[2].tag_byte()
            ))
        })?;

        // § 18: "list-keyed for `?`, string-keyed for `:name`, mixing throws".
        // The refusal is here rather than in the rewriter because an array is
        // the only thing that can be both, and the rewriter is handed one form.
        let held = crate::arr::borrowed(params);
        let mut positional: Vec<Bound> = Vec::new();
        let mut named: Vec<(String, Bound)> = Vec::new();
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
                nvs_runtime::SlotKey::Index(_) if named.is_empty() => positional.push(bound),
                nvs_runtime::SlotKey::Str(name) if positional.is_empty() => {
                    // A key is a Novis `string` and so is UTF-8 by ADR 0009;
                    // the lossy read is the spelling that needs no unreachable
                    // arm to say so.
                    named.push((String::from_utf8_lossy(name.as_bytes()).into_owned(), bound));
                }
                _ => {
                    return Err(Fault::thrown_as(
                        ThrownClass::Logic,
                        format!(
                            "{QUERY}: `$params` is keyed both ways at once, and a statement is \
                             written one way or the other — a list-keyed array binds `?` in \
                             order, a string-keyed one binds `:name`"
                        ),
                    ));
                }
            }
        }

        let arities: Vec<nvs_db::Binding> = positional.iter().map(|bound| bound.binding).collect();
        let keyed: Vec<(&str, nvs_db::Binding)> = named
            .iter()
            .map(|(name, bound)| (name.as_str(), bound.binding))
            .collect();
        let spelling = if named.is_empty() {
            nvs_db::Params::Positional(&arities)
        } else {
            nvs_db::Params::Named(&keyed)
        };
        let statement = nvs_db::rewrite(sql, spelling, nvs_db::Dialect::PostgreSql)
            .map_err(|refused| statement_failure(&block, &refused))?;

        let bounds: Vec<&Bound> = if named.is_empty() {
            positional.iter().collect()
        } else {
            named.iter().map(|(_, bound)| bound).collect()
        };
        let mut rendered: Vec<Option<Vec<u8>>> = Vec::with_capacity(statement.binds.len());
        for source in &statement.binds {
            rendered.push(
                nvs_db::encode(bounds[source.arg].values[source.element])
                    .map_err(|refused| statement_failure(&block, &refused))?,
            );
        }
        let sending: Vec<Option<&[u8]>> = rendered.iter().map(|one| one.as_deref()).collect();

        let filed = ctx.open_connection_mut(key).ok_or_else(|| {
            Fault::fatal(format!("{QUERY}: no connection is filed under the key {key}"))
        })?;
        let connection = filed
            .as_any_mut()
            .downcast_mut::<nvs_db::Connection>()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{QUERY}: the connection filed under the key {key} is not `nvs-db`'s"
                ))
            })?;
        let driver = connection.driver();
        let nvs_db::Connection::Postgres(postgres) = connection else {
            return Err(Fault::thrown(format!(
                "{QUERY}: `[db.{}]` is a {driver:?} connection, and only PostgreSQL runs a \
                 statement so far — this module's known gap 2 is the list",
                block.as_text().unwrap_or("?")
            )));
        };
        let mut answered = postgres
            .query(&statement.sql, &sending)
            .map_err(|refused| statement_failure(&block, &refused))?;
        // Taken before the first row: a `PgRows` lends its columns and its rows
        // out of one borrow, and the rows are read with it held mutably.
        let columns: Vec<nvs_db::PgColumn> = answered.columns().to_vec();

        let mut rows = nvs_runtime::NvsArray::new();
        loop {
            let Some(row) = answered
                .next_row()
                .map_err(|refused| statement_failure(&block, &refused))?
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
                    .map_err(|refused| statement_failure(&block, &refused))?;
                let value = column
                    .decode(body)
                    .map_err(|refused| statement_failure(&block, &refused))?
                    .ok_or_else(|| structured_column(&column.name))?;
                one.set(NvsStr::new(column.name.as_bytes()), value);
            }
            rows.append(Value::array(one));
        }

        Ok(crate::instance::build(&ROWS, [Value::array(rows)]))
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
        _ => return None,
    })
}
