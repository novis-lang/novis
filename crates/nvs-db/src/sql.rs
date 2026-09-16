//! [ADR 0067 § 5](/docs/decisions/0067.md)'s placeholder rewriter: one
//! spelling in, the driver's own out, and the `inList` expansion that § 1's
//! statement cache keys on.
//!
//! Novis code writes `?` or `:name` whichever database it is talking to, and
//! every driver gets the spelling its own protocol wants — `$1` on PostgreSQL,
//! `@p1` on SQL Server, `?` on MySQL, MariaDB and SQLite. That is the whole of
//! this module: [`rewrite`] is a pure function of the SQL text, the shape of the
//! bound arguments and a [`Dialect`], and it never sees a value. It is a free
//! function rather than a method on a connection for the reason `rule:core-classes/db-drivers-are-an-enum`
//! gives for the rest of the shared half — there is nothing per-connection in
//! it, and a test that had to build a `PgConn` to reach it would need a socket
//! and a certificate to ask what `IN ?` expands to.
//!
//! # What it returns, and why it is two things
//!
//! [`Statement::sql`] is the text to send; [`Statement::binds`] is the order the
//! caller's arguments go on the wire in, which is *not* the order they were
//! written in. A `:name` used twice is one argument bound at two markers, and an
//! `inList` is one argument bound at several — so a driver cannot recover the
//! bind order by counting the arguments it was handed, and this is the only
//! place that knows it.
//!
//! `binds.len()` is § 1's **expansion arity**, and the statement cache keys on
//! the original SQL text plus that number rather than on the rewritten text: the
//! rewriting is a pure function of the two, so they identify the same entries
//! while being cheaper to hash. `IN` over three ids and over four are two
//! entries, which is the whole reason the arity is in the key at all.
//!
//! [`StatementCache`] is here for that reason and no other — it is the half of
//! § 1 that decides whether a batch carries a `Parse`, and it is the same
//! decision on every driver, so it is plain data with no wire in it. Which
//! messages the answer turns into is each driver's own.
//!
//! [`statement_cache_for`] and [`time_zone_for`] are here on that same test and
//! no other: they read the two `[db.<name>]` fields whose meaning is a decision
//! rather than a string — how large § 1's cache is, and what § 9's declared
//! zone is in seconds — and they answer identically for every driver.
//! A driver takes the *answer* on its target, so no connect path reaches into a
//! config tree and every one of these is testable with neither a socket nor a
//! configuration file.
//!
//! # What it skips, and what it does not diagnose
//!
//! A `?` inside a string literal, a comment or a quoted identifier is text, not
//! a placeholder, so the scan tracks those regions. It also leaves PostgreSQL's
//! `::` cast and its `?|`/`?&` jsonb operators alone. What it cannot tell apart
//! is jsonb's bare `?` operator from a placeholder — they are the same byte in
//! the same position — which is why § 5 gives `??` as the escape for a literal
//! question mark, and why that escape is the one piece of syntax this rewriter
//! adds to SQL rather than removing.
//!
//! **A region the text never leaves is this module's own refusal.** A `'…'`, a
//! `"…"`, a backtick- or bracket-quoted name, a `$tag$…$tag$` body or a `/*…*/`
//! comment with no closing delimiter makes the scan read every byte after it as
//! being inside it, so what [`rewrite`] binds is what fits inside an opening
//! delimiter rather than what was written — and it refuses rather than send a
//! statement bound against that. `rule:core-classes/db-literal-query-checking`
//! asks for the same refusal while compiling a literal query, and that one can
//! be made only because this one is made here. A `--` or `#` comment opens
//! nothing and ends at the end of the text legitimately.
//!
//! **The rest of malformed SQL is the server's diagnosis, not ours.** A text
//! whose regions all close goes out to be rejected by a parser that can say what
//! is actually wrong with it. The errors below are otherwise only the ones about
//! *placeholders and arguments*, which the server cannot see because it never
//! receives the original spelling.

use std::io;

use nvs_config::tree::Database;

use crate::conn::Driver;

/// How one driver spells a bound parameter, and how it quotes and comments.
///
/// One value per syntax rather than per driver: MariaDB and MySQL share a
/// syntax exactly, and `rule:core-classes/db-one-api`'s insistence that they
/// are two drivers is about auth plugins, error tables and capability flags,
/// none of which reaches the SQL text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// `$1`, dollar-quoted bodies, nested block comments, no backslash escape.
    PostgreSql,
    /// `?`, backtick identifiers, `#` comments, and `\` escapes inside strings.
    MySql,
    /// `?`, backtick identifiers, and no backslash escape.
    Sqlite,
    /// `@p1`, and `[bracketed]` identifiers.
    SqlServer,
}

impl Dialect {
    /// The dialect a driver writes.
    #[must_use]
    pub const fn of(driver: Driver) -> Dialect {
        match driver {
            Driver::Postgres => Dialect::PostgreSql,
            Driver::MySql | Driver::MariaDb => Dialect::MySql,
            Driver::SqlServer => Dialect::SqlServer,
            Driver::Sqlite => Dialect::Sqlite,
        }
    }

    /// The `n`th marker, one-based, as the protocol wants to read it.
    ///
    /// `pub(crate)` for SQL Server's sake alone: `sp_prepexec` is handed a
    /// *declaration* of the parameters beside the SQL — `@p1 nvarchar(4000),…` —
    /// and the names in it have to be the names this rewriter wrote. Asking
    /// here is what keeps the two spellings one spelling.
    pub(crate) fn marker(self, n: usize) -> String {
        match self {
            Dialect::PostgreSql => format!("${n}"),
            Dialect::MySql | Dialect::Sqlite => "?".to_string(),
            Dialect::SqlServer => format!("@p{n}"),
        }
    }

    /// Whether a marker carries its own number, which is what lets a repeated
    /// `:name` be sent once and read twice.
    const fn numbered(self) -> bool {
        matches!(self, Dialect::PostgreSql | Dialect::SqlServer)
    }

    /// Whether `\` escapes the next byte inside a quoted run. MySQL's default,
    /// and nobody else's: PostgreSQL has `standard_conforming_strings` on, and
    /// SQLite and SQL Server never had it.
    ///
    /// Read twice: by the rewriter below, which must not mistake an escaped
    /// quote for the end of a string, and by [`crate::ddl`], which writes a
    /// text default into DDL where there is no parameter to bind it through.
    /// One judgement rather than two, because a value ending in `\` is exactly
    /// where the two would disagree.
    pub(crate) const fn backslash_escapes(self) -> bool {
        matches!(self, Dialect::MySql)
    }

    /// Whether `$tag$…$tag$` is a string body. PostgreSQL only, and it matters
    /// because a function body is exactly where a stray `?` lives.
    const fn dollar_quotes(self) -> bool {
        matches!(self, Dialect::PostgreSql)
    }

    /// Whether `[…]` is a quoted identifier. On PostgreSQL and SQLite the same
    /// bracket is an array subscript, so this cannot be unconditional.
    const fn bracket_quotes(self) -> bool {
        matches!(self, Dialect::SqlServer)
    }

    /// Whether `` `…` `` is a quoted identifier.
    const fn backtick_quotes(self) -> bool {
        matches!(self, Dialect::MySql | Dialect::Sqlite)
    }

    /// Whether `#` starts a comment. MySQL's alone: on PostgreSQL `#` is a
    /// legal operator character.
    const fn hash_comments(self) -> bool {
        matches!(self, Dialect::MySql)
    }

    /// Whether `/* /* */ */` nests, as PostgreSQL's does and no one else's.
    const fn nested_block_comments(self) -> bool {
        matches!(self, Dialect::PostgreSql)
    }

    /// Whether `?|` and `?&` are operators rather than a placeholder followed
    /// by something. jsonb's, so PostgreSQL's; elsewhere a `?` is always a
    /// placeholder and `? | 3` is written with the space it needs anyway.
    const fn jsonb_question_operators(self) -> bool {
        matches!(self, Dialect::PostgreSql)
    }
}

/// What one bound argument does to the SQL text — all this rewriter needs to
/// know about a value it never sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// One value at one marker, whatever it encodes to. § 5's "one parameter is
    /// always one value": a list bound this way is a PostgreSQL array or a JSON
    /// document, not an expansion.
    One,
    /// `Core\Db::inList($values)`, holding the number of values — the explicit
    /// marker that expands to a parenthesised list of that many markers.
    List(usize),
}

/// Which spelling the caller's arguments arrived in, per § 5.
///
/// The two are exclusive by construction here: an array is list-keyed or
/// string-keyed, and the `LogicError` for one that is both is raised where the
/// array is, not here.
#[derive(Debug, Clone, Copy)]
pub enum Params<'a> {
    /// A list-keyed array: positional `?`, bound in the order they appear.
    Positional(&'a [Binding]),
    /// A string-keyed array: `:name`, in the caller's own order, which is what
    /// [`Source::arg`] indexes into.
    Named(&'a [(&'a str, Binding)]),
}

/// Where one marker takes its value from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    /// The index into [`Params`]'s own slice, whichever form it took.
    pub arg: usize,
    /// Which element of an [`Binding::List`] argument, and always `0` for a
    /// single value.
    pub element: usize,
}

/// A rewritten statement: the text to send and the order to bind in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// The SQL with every placeholder replaced by the dialect's own marker.
    pub sql: String,
    /// One entry per marker, in the order the protocol reads them.
    pub binds: Vec<Source>,
}

impl Statement {
    /// § 1's expansion arity — the number of markers, which is what the
    /// statement cache's key carries alongside the *original* SQL text.
    #[must_use]
    pub fn arity(&self) -> usize {
        self.binds.len()
    }
}

/// What a `[db.<name>]` block that names no `statement_cache` is sized by.
///
/// A request runs a handful of distinct statements, and this holds them all
/// without asking a server to keep a hundred plans alive for a connection that
/// is idle in a pool.
pub const DEFAULT_STATEMENT_CACHE: usize = 16;

/// § 1's size for one connection, read from its `[db.<name>]` block.
///
/// The whole of the field's meaning is here, so nothing else has to decide what
/// an absent one means: an unset `statement_cache` is
/// [`DEFAULT_STATEMENT_CACHE`], and a written `0` is honoured rather than
/// treated as unset — that is § 1's cache turned off, not a block that forgot
/// to size it. `nvs-config` deliberately names no driver's constant, which is
/// why the default lives on this side of the edge.
///
/// Free rather than an associated function, and beside [`time_zone_for`]
/// because it is the same kind of thing: one `[db.<name>]` field, read the same
/// way for every driver, with no wire in it. It also has nothing to do with
/// what a driver's [`StatementCache`] holds — PostgreSQL's handle is a name it
/// mints and MySQL's is the server's own id — so hanging it off that type would
/// mean writing a handle down to ask a question about a number.
///
/// A number too large for a `usize` saturates instead of wrapping. It is not
/// reachable on any target this runs on, and the cache holds what the server
/// accepts rather than what the number claims.
#[must_use]
pub fn statement_cache_for(block: &Database) -> usize {
    block
        .statement_cache
        .map_or(DEFAULT_STATEMENT_CACHE, |size| {
            usize::try_from(size).unwrap_or(usize::MAX)
        })
}

/// [ADR 0067 § 1](/docs/decisions/0067.md)'s per-connection LRU of
/// server-side prepared statements.
///
/// There is no `prepare` step in the Novis API, so this is what makes "every
/// statement is prepared" cost what § 1 says it costs: the first execution of a
/// statement in a connection's life pays for a parse and every later one does
/// not. It is the *connection's* cache and never a process-wide one — a prepared
/// statement is a name on one session, and two connections that shared this
/// would bind against names the other's server has never heard of.
///
/// **The key is the SQL text plus [`Statement::arity`]**, not the rewritten
/// text: `IN` over three ids and over four are two server-side statements
/// because they are two different texts, and the arity is what tells them apart
/// without hashing the longer string. Text alone would hand the second one the
/// first one's plan, which is the bug this key exists to make unspellable.
///
/// # Its size, and why it is a parameter here
///
/// § 1 sizes it by `statement_cache` in the connection's config block, and
/// [`statement_cache_for`] is that field's reader. A driver takes the *answer*
/// on its target — `PgTarget` carries it beside the zone, for the reason that
/// type's doc gives — rather than reaching into a config tree from the connect
/// path, so the capacity is still a plain parameter of [`StatementCache::new`]
/// and a test can size one with no configuration at all. A capacity of `0` is
/// not a broken cache: it is the unnamed statement every time, which is what
/// PostgreSQL does with a statement that carries no name.
///
/// # `H` is the handle, and only its owner knows what one is
///
/// What a driver holds a cached statement *by* is the protocol's business, and
/// the two shapes are genuinely different rather than one shape spelled twice.
/// PostgreSQL **mints** the name and the server accepts it, so `H` is a
/// `String` this type generates ([`StatementCache::prepare`], the only place
/// [`Prepared`] is answered). MySQL **receives** a handle: the statement id
/// comes back from `COM_STMT_PREPARE` and nothing this side chooses it, so `H`
/// is that id and the driver drives the cache through [`Self::lookup`],
/// [`Self::make_room`] and [`Self::commit`] instead.
///
/// Everything above the handle is shared, which is the reason for the type
/// parameter rather than a second cache: the key, the LRU order, the capacity
/// and the eviction are one set of rules, and § 1 states them once.
#[derive(Debug)]
pub struct StatementCache<H = String> {
    /// Most recently used first. A `Vec` rather than a map because the capacity
    /// is a handful: a linear scan over that beats hashing the SQL text, and
    /// the LRU order is then the vector's own with nothing to maintain.
    entries: Vec<Entry<H>>,
    capacity: usize,
    /// Names are minted and never reused, so a `Close` still in flight can
    /// never collide with a `Parse` that follows it.
    ///
    /// Only a driver that names its own statements moves this: MySQL's handle
    /// is the server's, so its cache leaves the counter at zero.
    next: u64,
}

/// One statement the server is holding for us.
#[derive(Debug)]
struct Entry<H> {
    sql: String,
    arity: usize,
    handle: H,
}

/// What the cache says about a statement that is about to be sent.
///
/// Each variant is a shape the batch takes, which is why this is an answer and
/// not a lookup: a driver matches once and writes the messages the answer
/// names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prepared {
    /// The server already has it under this name, so the batch carries no
    /// `Parse` at all — § 1's "cached re-executions cost one round trip".
    Hit(String),
    /// It must be parsed under this name, and `evicted` is the name that had to
    /// leave to make room, to be closed in the same batch.
    Miss {
        /// The fresh name to parse under.
        name: String,
        /// The statement to deallocate alongside it, if the cache was full.
        evicted: Option<String>,
    },
    /// No caching: the unnamed statement, parsed every time. What a capacity of
    /// zero means, and it is a supported configuration rather than a failure.
    Unnamed,
}

impl Prepared {
    /// The server-side name to bind against — the empty string for the unnamed
    /// statement, which is the protocol's own spelling of it.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Prepared::Hit(name) | Prepared::Miss { name, .. } => name,
            Prepared::Unnamed => "",
        }
    }
}

impl<H: Clone> StatementCache<H> {
    /// An empty cache holding at most `capacity` statements; `0` disables it.
    #[must_use]
    pub fn new(capacity: usize) -> StatementCache<H> {
        StatementCache {
            entries: Vec::with_capacity(capacity.min(64)),
            capacity,
            next: 0,
        }
    }

    /// How many statements the server may be holding at once.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// How many it is holding now.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the server is holding none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The handle the server is already holding this statement under, promoted
    /// to most-recently-used, or `None` for one it has never seen.
    ///
    /// A miss records nothing: the statement does not exist on the server until
    /// it has been prepared, and a driver calls [`Self::commit`] once it has.
    /// Forgetting to commit costs a round trip and nothing else, which is the
    /// direction this split is biased in.
    pub fn lookup(&mut self, sql: &str, arity: usize) -> Option<H> {
        let at = self
            .entries
            .iter()
            .position(|entry| entry.arity == arity && entry.sql == sql)?;
        let entry = self.entries.remove(at);
        let handle = entry.handle.clone();
        self.entries.insert(0, entry);
        Some(handle)
    }

    /// The handle that has to leave before one more statement can be recorded,
    /// dropped from the cache here and deallocated on the server by its caller.
    ///
    /// `None` when there is already room, which is the common answer. It is the
    /// *caller's* job to close what this hands back, and calling it without
    /// closing leaks a server-side statement for the life of the connection —
    /// which is why it is spelled as a question about room rather than as a
    /// side effect of [`Self::commit`].
    pub fn make_room(&mut self) -> Option<H> {
        (self.entries.len() >= self.capacity)
            .then(|| self.entries.pop().map(|entry| entry.handle))
            .flatten()
    }

    /// Drops the statement recorded under this key, handing back the handle its
    /// caller must now deallocate, or `None` for a key the cache does not hold.
    ///
    /// The eviction [`Self::make_room`] does not cover: a driver reaches for
    /// this when a *hit* turns out to be unusable rather than when there is no
    /// room. SQL Server compiles a plan against the `@params` declaration the
    /// first execution sent, so the same key run later with a value that
    /// declaration does not fit names a plan that would truncate it —
    /// `crate::tds::TdsPlan`'s own doc owns that reading. Removing the entry
    /// rather than committing over it is what keeps the cache from holding two
    /// under one key, which [`Self::lookup`]'s linear scan could not tell apart.
    pub fn forget(&mut self, sql: &str, arity: usize) -> Option<H> {
        let at = self
            .entries
            .iter()
            .position(|entry| entry.arity == arity && entry.sql == sql)?;
        Some(self.entries.remove(at).handle)
    }

    /// Records a statement the server has now prepared, as most-recently-used.
    ///
    /// Only ever called for a statement whose prepare has landed, and once per
    /// miss: `rule:core-classes/db-statement-members` allows one statement at a time, so nothing can have
    /// touched the cache in between.
    pub fn commit(&mut self, sql: &str, arity: usize, handle: H) {
        if self.capacity == 0 {
            return;
        }
        self.entries.insert(
            0,
            Entry {
                sql: sql.to_string(),
                arity,
                handle,
            },
        );
    }

    /// Forgets every statement, for a reset that deallocated them.
    ///
    /// § 13's asymmetry, and it is the protocol's rather than a choice:
    /// `COM_RESET_CONNECTION` and `sp_reset_connection` drop prepared statements
    /// along with everything else, so their drivers call this and PostgreSQL's —
    /// whose reset is deliberately not `DISCARD ALL` — never does.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl StatementCache<String> {
    /// Answers what the batch for this statement has to carry, promoting a hit
    /// to most-recently-used and minting a name for a miss.
    ///
    /// The name is this side's to choose, which is what makes a whole batch
    /// answerable in one call — a driver that is *handed* its handle has to
    /// wait for the prepare and drives [`Self::lookup`] and [`Self::commit`]
    /// itself instead.
    pub fn prepare(&mut self, sql: &str, arity: usize) -> Prepared {
        if self.capacity == 0 {
            return Prepared::Unnamed;
        }
        if let Some(name) = self.lookup(sql, arity) {
            return Prepared::Hit(name);
        }
        let evicted = self.make_room();
        let name = format!("s{}", self.next);
        self.next += 1;
        Prepared::Miss { name, evicted }
    }
}

/// Rewrites `sql`'s `?` or `:name` placeholders into `dialect`'s markers.
///
/// # Errors
///
/// Every one of these is [`io::ErrorKind::InvalidInput`] and becomes a
/// `LogicError` at the `Core\Db` boundary — they are all mistakes in the call
/// rather than answers from a server, and this crate builds no fault of its own
/// (`Cargo.toml` § 1 is the rule):
///
/// - a string literal, a quoted name, a `$tag$` body or a `/*` comment the text
///   opens and never closes, which leaves every byte after it read as being
///   inside it;
/// - the SQL's spelling disagrees with the arguments' — a `:name` against a
///   list-keyed array, or a `?` against a string-keyed one;
/// - a positional statement has more or fewer `?` than arguments;
/// - a `:name` names no argument, or an argument is never named;
/// - an `inList` is empty, which § 5 refuses because the rewriter cannot tell
///   `IN` from `NOT IN` and the two want opposite answers.
pub fn rewrite(sql: &str, params: Params<'_>, dialect: Dialect) -> io::Result<Statement> {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len() + 16);
    let mut binds: Vec<Source> = Vec::new();
    // A numbered dialect renders a repeated `:name` back to the same marker, so
    // the value goes out once; keeping the rendered text is simpler than
    // recovering it from the bind indices, and there is one entry per name.
    let mut rendered: Vec<(&str, String)> = Vec::new();
    let mut used = match params {
        Params::Named(list) => vec![false; list.len()],
        Params::Positional(_) => Vec::new(),
    };
    let mut positional = 0usize;
    // Everything from here to `i` is text we have not yet had a reason to cut.
    let mut copied = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                let quoted = skip_quoted(bytes, i, b'\'', dialect.backslash_escapes());
                i = closed(quoted, "a `'` string literal", i)?;
            }
            b'"' => {
                let quoted = skip_quoted(bytes, i, b'"', dialect.backslash_escapes());
                i = closed(quoted, "a `\"` quoted region", i)?;
            }
            b'`' if dialect.backtick_quotes() => {
                i = closed(
                    skip_quoted(bytes, i, b'`', false),
                    "a backtick-quoted name",
                    i,
                )?;
            }
            b'[' if dialect.bracket_quotes() => {
                i = closed(skip_bracket(bytes, i), "a `[` quoted name", i)?;
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => i = skip_line(bytes, i),
            b'#' if dialect.hash_comments() => i = skip_line(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let comment = skip_block(bytes, i, dialect.nested_block_comments());
                i = closed(comment, "a `/*` comment", i)?;
            }
            b'$' if dialect.dollar_quotes() => {
                i = match skip_dollar(bytes, i) {
                    Some(body) => closed(body, "a `$…$` body", i)?,
                    None => i + 1,
                };
            }
            // A cast, not a name. `::` is the reason a `:` alone is not enough
            // to start one.
            b':' if bytes.get(i + 1) == Some(&b':') => i += 2,
            b':' if bytes.get(i + 1).is_some_and(|c| is_name_start(*c)) => {
                let end = name_end(bytes, i + 1);
                let name = &sql[i + 1..end];
                let list = match params {
                    Params::Positional(_) => return Err(named_against_a_list(name)),
                    Params::Named(list) => list,
                };
                let arg = list
                    .iter()
                    .position(|(bound, _)| *bound == name)
                    .ok_or_else(|| unbound_name(name))?;
                used[arg] = true;
                out.push_str(&sql[copied..i]);
                match rendered.iter().find(|(seen, _)| *seen == name) {
                    Some((_, text)) if dialect.numbered() => out.push_str(text),
                    _ => {
                        let text = expand(dialect, &mut binds, arg, list[arg].1)?;
                        out.push_str(&text);
                        rendered.push((name, text));
                    }
                }
                i = end;
                copied = end;
            }
            b'?' => {
                // `??` is § 5's escape, and on PostgreSQL it is also the only
                // way to write jsonb's own one-byte `?` operator.
                if bytes.get(i + 1) == Some(&b'?') {
                    out.push_str(&sql[copied..i]);
                    out.push('?');
                    i += 2;
                    copied = i;
                } else if dialect.jsonb_question_operators()
                    && matches!(bytes.get(i + 1), Some(b'|' | b'&'))
                {
                    i += 2;
                } else {
                    let slots = match params {
                        Params::Named(_) => return Err(positional_against_names()),
                        Params::Positional(slots) => slots,
                    };
                    let binding = *slots
                        .get(positional)
                        .ok_or_else(|| positional_mismatch(positional + 1, slots.len()))?;
                    out.push_str(&sql[copied..i]);
                    let text = expand(dialect, &mut binds, positional, binding)?;
                    out.push_str(&text);
                    positional += 1;
                    i += 1;
                    copied = i;
                }
            }
            _ => i += 1,
        }
    }
    out.push_str(&sql[copied..]);

    match params {
        Params::Positional(slots) if positional != slots.len() => {
            Err(positional_mismatch(positional, slots.len()))
        }
        Params::Named(list) => match used.iter().position(|seen| !seen) {
            Some(unused) => Err(unused_name(list[unused].0)),
            None => Ok(Statement { sql: out, binds }),
        },
        Params::Positional(_) => Ok(Statement { sql: out, binds }),
    }
}

/// Whether `sql` holds a second statement — [ADR 0067
/// § 1](/docs/decisions/0067.md)'s "every statement is prepared", as
/// the one question about statement *count* that can be asked without a
/// vendor's grammar.
///
/// A prepared statement is one statement on every backend this crate speaks to,
/// so a text holding two is refused by the server rather than run — and § 10
/// moves that refusal to `nvs check` for a literal, which is the only reason
/// this is here rather than left to the wire. It shares [`rewrite`]'s regions
/// exactly: a `;` inside a string, an identifier, a comment or a dollar-quoted
/// body is text, and a **trailing** `;` terminates the one statement rather
/// than starting another, comments after it included.
///
/// It is not a parser and does not try to be. Whether the second statement is
/// well formed is the server's diagnosis, per this module's own rule.
#[must_use]
pub fn holds_a_second_statement(sql: &str, dialect: Dialect) -> bool {
    let bytes = sql.as_bytes();
    let mut ended = false;
    let mut i = 0usize;

    while i < bytes.len() {
        // What neither ends a statement nor begins one, taken first so that a
        // comment or a run of spaces after the `;` still leaves the text one
        // statement long.
        match bytes[i] {
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                i = skip_line(bytes, i);
                continue;
            }
            b'#' if dialect.hash_comments() => {
                i = skip_line(bytes, i);
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = skip_block(bytes, i, dialect.nested_block_comments()).or_end(bytes.len());
                continue;
            }
            b';' => {
                ended = true;
                i += 1;
                continue;
            }
            c if c.is_ascii_whitespace() => {
                i += 1;
                continue;
            }
            _ => {}
        }
        // Anything else is a statement's own text, which after a `;` is the
        // second one however short it is.
        if ended {
            return true;
        }
        i = match bytes[i] {
            b'\'' => skip_quoted(bytes, i, b'\'', dialect.backslash_escapes()).or_end(bytes.len()),
            b'"' => skip_quoted(bytes, i, b'"', dialect.backslash_escapes()).or_end(bytes.len()),
            b'`' if dialect.backtick_quotes() => {
                skip_quoted(bytes, i, b'`', false).or_end(bytes.len())
            }
            b'[' if dialect.bracket_quotes() => skip_bracket(bytes, i).or_end(bytes.len()),
            b'$' if dialect.dollar_quotes() => match skip_dollar(bytes, i) {
                Some(body) => body.or_end(bytes.len()),
                None => i + 1,
            },
            _ => i + 1,
        };
    }
    false
}

/// Whether `sql` opens a region it never closes —
/// `rule:core-classes/db-literal-query-checking`'s "an unterminated string
/// literal", as the question [`rewrite`] answers by refusing, asked of a text
/// with no arguments written beside it.
///
/// That is the shape the compile-time half needs: a literal query whose params
/// array the checker could not read whole has nothing to hand [`rewrite`] as
/// [`Params`], and the quoting is a fact about the text either way. It enters
/// the regions that function enters, through the same helpers, so the two agree
/// by construction rather than by being kept in step.
#[must_use]
pub fn holds_an_unterminated_region(sql: &str, dialect: Dialect) -> bool {
    let bytes = sql.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        let region = match bytes[i] {
            b'\'' => skip_quoted(bytes, i, b'\'', dialect.backslash_escapes()),
            b'"' => skip_quoted(bytes, i, b'"', dialect.backslash_escapes()),
            b'`' if dialect.backtick_quotes() => skip_quoted(bytes, i, b'`', false),
            b'[' if dialect.bracket_quotes() => skip_bracket(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                skip_block(bytes, i, dialect.nested_block_comments())
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                i = skip_line(bytes, i);
                continue;
            }
            b'#' if dialect.hash_comments() => {
                i = skip_line(bytes, i);
                continue;
            }
            b'$' if dialect.dollar_quotes() => match skip_dollar(bytes, i) {
                Some(body) => body,
                None => {
                    i += 1;
                    continue;
                }
            },
            _ => {
                i += 1;
                continue;
            }
        };
        match region {
            Region::Ends(end) => i = end,
            Region::Unterminated => return true,
        }
    }
    false
}

/// Renders the markers one argument expands to, recording what each binds.
fn expand(
    dialect: Dialect,
    binds: &mut Vec<Source>,
    arg: usize,
    binding: Binding,
) -> io::Result<String> {
    match binding {
        Binding::One => {
            let text = dialect.marker(binds.len() + 1);
            binds.push(Source { arg, element: 0 });
            Ok(text)
        }
        Binding::List(0) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "argument {arg} is an empty `inList`, and `rule:core-classes/db-parameters` refuses one: an empty list \
                 matches nothing inside `IN` and everything inside `NOT IN`, the rewriter cannot \
                 tell which it is in, and the caller has to branch"
            ),
        )),
        Binding::List(len) => {
            let mut text = String::with_capacity(len * 5 + 2);
            text.push('(');
            for element in 0..len {
                if element > 0 {
                    text.push_str(", ");
                }
                text.push_str(&dialect.marker(binds.len() + 1));
                binds.push(Source { arg, element });
            }
            text.push(')');
            Ok(text)
        }
    }
}

/// A `:name` in the SQL, and a list-keyed array at the call.
fn named_against_a_list(name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "the statement binds `:{name}` but its arguments are a list, and `rule:core-classes/db-parameters` reads \
             the array's keys as the choice: give the array string keys, or write `?` in the SQL"
        ),
    )
}

/// A `?` in the SQL, and a string-keyed array at the call.
fn positional_against_names() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "the statement binds `?` but its arguments are string-keyed, and `rule:core-classes/db-parameters` reads the \
         array's keys as the choice: write `:name` in the SQL, or give the array list keys",
    )
}

/// More or fewer `?` than there were arguments.
fn positional_mismatch(placeholders: usize, arguments: usize) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "the statement has {placeholders} `?` placeholder(s) and was given {arguments} \
             argument(s), and `rule:core-classes/db-parameters` binds them one for one — an `inList` counts as the one \
             argument it is, however many values it holds"
        ),
    )
}

/// A `:name` that no argument answers to.
fn unbound_name(name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("the statement binds `:{name}` and no argument is keyed `{name}`"),
    )
}

/// An argument no `:name` ever asked for — a misspelling on one side or the
/// other, and never something the server could report.
fn unused_name(name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("the argument keyed `{name}` is never bound: the statement has no `:{name}`"),
    )
}

/// One past a region the scan entered, or the refusal for a text that never
/// leaves it.
fn closed(region: Region, opened: &str, at: usize) -> io::Result<usize> {
    match region {
        Region::Ends(end) => Ok(end),
        Region::Unterminated => Err(unterminated(opened, at)),
    }
}

/// A region opened at `at` and closed nowhere.
fn unterminated(opened: &str, at: usize) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "the statement opens {opened} at byte {at} and never closes it: every byte after \
             that is read as being inside it, so what the statement binds is what fits inside \
             an opening delimiter rather than what was written"
        ),
    )
}

/// Whether a byte may open a `:name`.
const fn is_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

/// One past the end of the name starting at `start`.
fn name_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    i
}

/// Where a region the scan entered ends: the two answers a reader that refuses
/// an unclosed one has to tell apart.
#[derive(Debug, Clone, Copy)]
enum Region {
    /// One past its closing delimiter.
    Ends(usize),
    /// There is none, and the region runs to the end of the text.
    Unterminated,
}

impl Region {
    /// Where a scan that refuses nothing continues: one past the closing
    /// delimiter, or `end` for a region the text never leaves.
    fn or_end(self, end: usize) -> usize {
        match self {
            Region::Ends(at) => at,
            Region::Unterminated => end,
        }
    }
}

/// One past the closing `quote`, or [`Region::Unterminated`] where the text
/// holds none.
///
/// Doubling is the escape everywhere; a backslash is one only where the dialect
/// says so.
fn skip_quoted(bytes: &[u8], start: usize, quote: u8, backslash: bool) -> Region {
    let mut i = start + 1;
    while i < bytes.len() {
        if backslash && bytes[i] == b'\\' {
            i += 2;
        } else if bytes[i] == quote {
            if bytes.get(i + 1) == Some(&quote) {
                i += 2;
            } else {
                return Region::Ends(i + 1);
            }
        } else {
            i += 1;
        }
    }
    Region::Unterminated
}

/// One past the closing `]`, where `]]` is the escape.
fn skip_bracket(bytes: &[u8], start: usize) -> Region {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b']' {
            if bytes.get(i + 1) == Some(&b']') {
                i += 2;
            } else {
                return Region::Ends(i + 1);
            }
        } else {
            i += 1;
        }
    }
    Region::Unterminated
}

/// One past the newline that ends a `--` or `#` comment.
fn skip_line(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    // The newline itself is ordinary text, and copying it with the comment is
    // what keeps the rewritten SQL line-for-line with what was written.
    (i + 1).min(bytes.len())
}

/// One past the `*/` that closes a block comment, counting depth where the
/// dialect nests.
fn skip_block(bytes: &[u8], start: usize, nested: bool) -> Region {
    let mut depth = 1usize;
    let mut i = start + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth -= 1;
            if depth == 0 {
                return Region::Ends(i + 2);
            }
            i += 2;
        } else if nested && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    Region::Unterminated
}

/// The `$tag$` body this `$` opens, or `None` if it opens none at all — which
/// is what `$1` and a bare `$` are.
fn skip_dollar(bytes: &[u8], start: usize) -> Option<Region> {
    let mut j = start + 1;
    if bytes.get(j).is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
        j += 1;
    }
    if bytes.get(j) != Some(&b'$') {
        return None;
    }
    let tag = &bytes[start..=j];
    let mut i = j + 1;
    while i + tag.len() <= bytes.len() {
        if &bytes[i..i + tag.len()] == tag {
            return Some(Region::Ends(i + tag.len()));
        }
        i += 1;
    }
    Some(Region::Unterminated)
}

/// [ADR 0067 § 9](/docs/decisions/0067.md)'s declared zone for one
/// connection, as whole seconds east of UTC.
///
/// The zone a zone-less `DATETIME`/`TIMESTAMP` column is read in, and the one
/// sent to the server so `CURRENT_TIMESTAMP` agrees with it. It is here rather
/// than in a driver because it is the same field on every driver and has the
/// same answer on every one: § 9 sends an offset and never a zone name, so
/// nothing downstream needs a zone database to act on this number.
///
/// **`Some(0)` for a block that names none** — § 9's default is UTC — and
/// `Some` of the offset for one that writes `UTC`, `Z`, `±HH`, `±HH:MM` or
/// `±HH:MM:SS`, each field exactly two digits. **`None` is a value that is not
/// an offset at all**: a zone name like `Europe/Vienna`, the compact `+0200`,
/// a minutes or seconds field past 59, or a magnitude past 18 hours. That is a
/// distinct answer from the default on purpose — a reader folding it into `0`
/// would run a deployment two hours out on a typo, and an operator writes this
/// field precisely because UTC is not what the columns mean. Turning the
/// `None` into a refusal naming the block is the resolver's job:
/// [`PgTarget::resolve`](crate::pg::PgTarget::resolve) answers
/// [`BlockError::TimeZone`](crate::conn::BlockError::TimeZone) for it, and every
/// driver added later owes the same refusal rather than a default.
#[must_use]
pub fn time_zone_for(block: &Database) -> Option<i32> {
    match block.time_zone.as_deref() {
        None => Some(0),
        Some(written) => offset_seconds(written.trim()),
    }
}

/// One written zone as seconds east of UTC, or `None` for a spelling
/// [`time_zone_for`] does not accept.
fn offset_seconds(written: &str) -> Option<i32> {
    if written.eq_ignore_ascii_case("utc") || written.eq_ignore_ascii_case("z") {
        return Some(0);
    }
    let (sign, rest) = match written.as_bytes().first().copied()? {
        b'+' => (1, &written[1..]),
        b'-' => (-1, &written[1..]),
        _ => return None,
    };
    let mut fields = rest.split(':');
    let hours = two_digits(fields.next()?)?;
    let minutes = fields.next().map_or(Some(0), two_digits)?;
    let seconds = fields.next().map_or(Some(0), two_digits)?;
    if fields.next().is_some() || minutes > 59 || seconds > 59 {
        return None;
    }
    // 18 hours is the widest zone anyone keeps, and the bound is what makes a
    // fat-fingered `+90:00` a refusal rather than a plausible number.
    let offset = sign * (hours * 3600 + minutes * 60 + seconds);
    (offset.abs() <= 18 * 3600).then_some(offset)
}

/// Exactly two ASCII digits as a number, which is what makes the compact
/// `+0200` a refusal rather than two hundred hours.
fn two_digits(field: &str) -> Option<i32> {
    if field.len() != 2 || !field.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    field.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{
        Binding, DEFAULT_STATEMENT_CACHE, Database, Dialect, Params, Prepared, Source, Statement,
        StatementCache, holds_an_unterminated_region, rewrite, statement_cache_for, time_zone_for,
    };
    use crate::conn::Driver;

    /// The rewritten text alone, for the cases that are about the scan.
    fn pg(sql: &str, slots: &[Binding]) -> String {
        rewrite(sql, Params::Positional(slots), Dialect::PostgreSql)
            .expect("the rewrite should succeed")
            .sql
    }

    fn pg_named(sql: &str, list: &[(&str, Binding)]) -> Statement {
        rewrite(sql, Params::Named(list), Dialect::PostgreSql).expect("the rewrite should succeed")
    }

    fn refused(sql: &str, params: Params<'_>) -> String {
        rewrite(sql, params, Dialect::PostgreSql)
            .expect_err("the rewrite should be refused")
            .to_string()
    }

    #[test]
    fn positional_placeholders_become_numbered_markers() {
        let one = Binding::One;
        let statement = rewrite(
            "select * from t where a = ? and b = ?",
            Params::Positional(&[one, one]),
            Dialect::PostgreSql,
        )
        .expect("two placeholders, two arguments");
        assert_eq!(statement.sql, "select * from t where a = $1 and b = $2");
        assert_eq!(
            statement.binds,
            [Source { arg: 0, element: 0 }, Source { arg: 1, element: 0 },]
        );
        assert_eq!(statement.arity(), 2);
    }

    #[test]
    fn every_dialect_spells_the_same_statement_its_own_way() {
        let sql = "select ? , ?";
        let slots = [Binding::One, Binding::One];
        let spellings = [
            (Dialect::PostgreSql, "select $1 , $2"),
            (Dialect::MySql, "select ? , ?"),
            (Dialect::Sqlite, "select ? , ?"),
            (Dialect::SqlServer, "select @p1 , @p2"),
        ];
        for (dialect, expected) in spellings {
            let statement = rewrite(sql, Params::Positional(&slots), dialect)
                .expect("two placeholders, two arguments");
            assert_eq!(statement.sql, expected, "{dialect:?}");
            // Whatever the spelling, the bind order is the same.
            assert_eq!(statement.arity(), 2, "{dialect:?}");
        }
    }

    #[test]
    fn mariadb_and_mysql_are_one_dialect_and_the_five_drivers_map_onto_four() {
        assert_eq!(Dialect::of(Driver::MariaDb), Dialect::of(Driver::MySql));
        assert_eq!(Dialect::of(Driver::Postgres), Dialect::PostgreSql);
        assert_eq!(Dialect::of(Driver::SqlServer), Dialect::SqlServer);
        assert_eq!(Dialect::of(Driver::Sqlite), Dialect::Sqlite);
    }

    #[test]
    fn a_repeated_name_binds_one_value_once_where_the_marker_is_numbered() {
        let statement = pg_named(
            "select * from t where a = :id or b = :id",
            &[("id", Binding::One)],
        );
        assert_eq!(statement.sql, "select * from t where a = $1 or b = $1");
        assert_eq!(statement.binds, [Source { arg: 0, element: 0 }]);
        assert_eq!(statement.arity(), 1);
    }

    #[test]
    fn a_repeated_name_is_sent_twice_where_the_marker_is_not() {
        // The asymmetry § 5 names: positional form cannot express the reuse,
        // and neither can a protocol whose marker carries no number.
        let statement = rewrite(
            "select * from t where a = :id or b = :id",
            Params::Named(&[("id", Binding::One)]),
            Dialect::MySql,
        )
        .expect("one argument, bound twice");
        assert_eq!(statement.sql, "select * from t where a = ? or b = ?");
        assert_eq!(
            statement.binds,
            [Source { arg: 0, element: 0 }, Source { arg: 0, element: 0 },]
        );
    }

    /// § 5's reuse asserted as an agreement across the dialects rather than
    /// one line each: whatever the marker, the one argument is consumed
    /// exactly once — never refused as unused, never bound to a second
    /// argument — and the arity § 1's cache keys on is the number of markers
    /// that dialect actually wrote, not the argument count.
    #[test]
    fn a_named_parameter_used_twice_binds_one_value_once() {
        let sql = "select * from t where a = :id or b = :id";
        for dialect in [
            Dialect::PostgreSql,
            Dialect::MySql,
            Dialect::Sqlite,
            Dialect::SqlServer,
        ] {
            let statement = rewrite(sql, Params::Named(&[("id", Binding::One)]), dialect)
                .expect("one argument, named twice");
            assert!(
                statement
                    .binds
                    .iter()
                    .all(|bind| *bind == Source { arg: 0, element: 0 }),
                "{dialect:?}: {:?}",
                statement.binds
            );
            assert_eq!(
                statement.arity(),
                if dialect.numbered() { 1 } else { 2 },
                "{dialect:?}"
            );
            assert!(statement.sql.contains(&dialect.marker(1)), "{dialect:?}");
            // A numbered dialect never reaches a second marker at all, which is
            // the whole of why its arity is one.
            assert_eq!(
                statement.sql.contains(&dialect.marker(2)),
                !dialect.numbered(),
                "{dialect:?}"
            );
        }
        // And the reuse is what makes the cache key differ: the same shape
        // spelled with two names binds two values and is a second entry.
        let two = pg_named(
            "select * from t where a = :id or b = :other",
            &[("id", Binding::One), ("other", Binding::One)],
        );
        assert_ne!(two.arity(), pg_named(sql, &[("id", Binding::One)]).arity());
    }

    #[test]
    fn in_list_expands_to_a_parenthesised_run_and_moves_the_arity() {
        let statement = rewrite(
            "select * from t where id in ? and k = ?",
            Params::Positional(&[Binding::List(3), Binding::One]),
            Dialect::PostgreSql,
        )
        .expect("two arguments, four markers");
        assert_eq!(
            statement.sql,
            "select * from t where id in ($1, $2, $3) and k = $4"
        );
        assert_eq!(
            statement.binds,
            [
                Source { arg: 0, element: 0 },
                Source { arg: 0, element: 1 },
                Source { arg: 0, element: 2 },
                Source { arg: 1, element: 0 },
            ]
        );
        assert_eq!(statement.arity(), 4);
    }

    #[test]
    fn one_text_over_three_ids_and_over_four_are_two_cache_keys() {
        // § 1's key is the original text plus the arity, so the arity is the
        // only thing that may distinguish these two.
        let sql = "select * from t where id in ?";
        let three = rewrite(
            sql,
            Params::Positional(&[Binding::List(3)]),
            Dialect::PostgreSql,
        )
        .expect("three values");
        let four = rewrite(
            sql,
            Params::Positional(&[Binding::List(4)]),
            Dialect::PostgreSql,
        )
        .expect("four values");
        assert_ne!(three.arity(), four.arity());
        assert_ne!(three.sql, four.sql);
    }

    #[test]
    fn an_empty_in_list_is_refused_rather_than_guessed_at() {
        let message = refused(
            "select * from t where id in ?",
            Params::Positional(&[Binding::List(0)]),
        );
        assert!(message.contains("empty `inList`"), "{message}");
        assert!(message.contains("NOT IN"), "{message}");
    }

    /// § 5's expansion at the bound it stops accepting, both sides named
    /// together: a list of one is the shortest length that still expands —
    /// parentheses and all — and zero is the first one refused, over the same
    /// statement text. A rewriter that special-cased the single element prints
    /// plausibly against either half alone.
    #[test]
    fn in_list_expands_and_an_empty_list_throws() {
        let sql = "select * from t where id in ?";
        let one = rewrite(
            sql,
            Params::Positional(&[Binding::List(1)]),
            Dialect::PostgreSql,
        )
        .expect("a list of one expands");
        assert_eq!(one.sql, "select * from t where id in ($1)");
        assert_eq!(one.binds, [Source { arg: 0, element: 0 }]);
        assert_eq!(one.arity(), 1);

        let message = refused(sql, Params::Positional(&[Binding::List(0)]));
        assert!(message.contains("empty `inList`"), "{message}");
        // The classification, not just the refusal: an empty list is a mistake
        // in the call, so it is `InvalidInput` and becomes a `LogicError` at
        // the `Core\Db` boundary rather than a fault of this crate's own.
        assert_eq!(
            rewrite(
                sql,
                Params::Positional(&[Binding::List(0)]),
                Dialect::PostgreSql
            )
            .expect_err("an empty list is refused")
            .kind(),
            std::io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn a_question_mark_inside_a_literal_or_a_comment_is_text() {
        assert_eq!(
            pg("select 'a ? b', ?", &[Binding::One]),
            "select 'a ? b', $1"
        );
        assert_eq!(
            pg("select 'it''s ?', ?", &[Binding::One]),
            "select 'it''s ?', $1"
        );
        assert_eq!(
            pg("select \"a ? b\", ?", &[Binding::One]),
            "select \"a ? b\", $1"
        );
        assert_eq!(pg("-- ?\nselect ?", &[Binding::One]), "-- ?\nselect $1");
        assert_eq!(pg("/* ? */ select ?", &[Binding::One]), "/* ? */ select $1");
        assert_eq!(
            pg("/* /* ? */ ? */ select ?", &[Binding::One]),
            "/* /* ? */ ? */ select $1"
        );
        assert_eq!(
            pg("select $fn$ a ? b $fn$, ?", &[Binding::One]),
            "select $fn$ a ? b $fn$, $1"
        );
        assert_eq!(
            pg("select $$ ? $$, ?", &[Binding::One]),
            "select $$ ? $$, $1"
        );
    }

    #[test]
    fn a_name_inside_a_literal_or_a_comment_is_text() {
        let statement = pg_named("select ':id', -- :id\n :id", &[("id", Binding::One)]);
        assert_eq!(statement.sql, "select ':id', -- :id\n $1");
        assert_eq!(statement.arity(), 1);
    }

    /// § 5's scanner as one sweep rather than one line per construct, and
    /// asserted by counting: what a naive rewriter gets wrong is never the
    /// hider it knows about but the one it has not heard of, so the claim is
    /// that *every* construct hides *both* spellings of a placeholder and that
    /// the whole sweep binds one value per row and no more.
    #[test]
    fn the_rewriter_skips_string_literals_and_comments() {
        let hidden = [
            "'a ? b :id'",
            "'it''s ? :id'",
            "\"a ? b :id\"",
            "$$ ? :id $$",
            "$fn$ ? :id $fn$",
            "-- ? :id\n",
            "/* ? :id */",
            "/* /* ? :id */ ? :id */",
        ];
        let mut bound = 0usize;
        for text in hidden {
            let positional = rewrite(
                &format!("select {text}, ?"),
                Params::Positional(&[Binding::One]),
                Dialect::PostgreSql,
            )
            .expect("one placeholder, outside the hidden text");
            assert_eq!(positional.sql, format!("select {text}, $1"), "{text}");
            // The same bytes read the other way: a `:name` inside a hider is
            // not a name either, so the one argument is neither left unbound
            // nor bound a second time.
            let named = rewrite(
                &format!("select {text}, :id"),
                Params::Named(&[("id", Binding::One)]),
                Dialect::PostgreSql,
            )
            .expect("one name, outside the hidden text");
            assert_eq!(named.sql, format!("select {text}, $1"), "{text}");
            bound += positional.arity() + named.arity();
        }
        assert_eq!(
            bound,
            hidden.len() * 2,
            "every construct hides both spellings, and nothing else binds"
        );
    }

    #[test]
    fn a_backslash_ends_a_mysql_literal_and_does_not_end_a_postgresql_one() {
        // The one place the dialects disagree about where a literal stops, and
        // getting it wrong rewrites a `?` that was never a placeholder.
        let sql = r"select 'a\', ?";
        assert_eq!(
            rewrite(
                sql,
                Params::Positional(&[Binding::One]),
                Dialect::PostgreSql
            )
            .expect("the literal runs to the second quote")
            .sql,
            r"select 'a\', $1"
        );
        // On MySQL `\'` is an escape, so the literal is never closed — and an
        // unterminated one is refused for being one rather than for the
        // placeholder count it happens to leave behind.
        let message = rewrite(sql, Params::Positional(&[Binding::One]), Dialect::MySql)
            .expect_err("the literal is never closed")
            .to_string();
        assert!(message.contains("never closes it"), "{message}");
    }

    #[test]
    fn a_region_the_text_never_leaves_is_refused_rather_than_bound() {
        // Every opener, each followed by text that would bind differently read
        // as quoted than read as SQL. The claim is that the rewriter refuses
        // all of them rather than sending a statement bound against what fits
        // inside an opening delimiter.
        for sql in [
            "select 'a, ?",
            "select \"a, ?",
            "select $tag$ a, ?",
            "select /* a, ?",
            "select ?, 'a''b",
        ] {
            let message = rewrite(
                sql,
                Params::Positional(&[Binding::One]),
                Dialect::PostgreSql,
            )
            .expect_err(sql)
            .to_string();
            assert!(message.contains("never closes it"), "{sql}: {message}");
            assert!(
                holds_an_unterminated_region(sql, Dialect::PostgreSql),
                "{sql}"
            );
        }

        // What closes is left exactly where it was, the doubled delimiter that
        // is not a closing one included.
        for sql in [
            "select 'a''b', ?",
            "select $tag$ a $tag$, ?",
            "select /* a */ ?",
            "select ? -- and a comment to the end",
        ] {
            assert!(
                !holds_an_unterminated_region(sql, Dialect::PostgreSql),
                "{sql}"
            );
            rewrite(
                sql,
                Params::Positional(&[Binding::One]),
                Dialect::PostgreSql,
            )
            .expect(sql);
        }

        // A region only one dialect enters is one the others are silent about,
        // which is why the compile-time half asks all four and refuses only
        // where every one of them does.
        assert!(holds_an_unterminated_region("select `a, ?", Dialect::MySql));
        assert!(!holds_an_unterminated_region(
            "select `a, ?",
            Dialect::PostgreSql
        ));
    }

    #[test]
    fn the_operators_a_question_mark_is_part_of_are_left_alone() {
        assert_eq!(
            pg("select a ?| array['x'], ?", &[Binding::One]),
            "select a ?| array['x'], $1"
        );
        assert_eq!(pg("select a ?& b, ?", &[Binding::One]), "select a ?& b, $1");
        // MySQL has no such operator, so there the same bytes are a
        // placeholder followed by an operator.
        assert_eq!(
            rewrite(
                "select a ?| b",
                Params::Positional(&[Binding::One]),
                Dialect::MySql
            )
            .expect("one placeholder")
            .sql,
            "select a ?| b"
        );
    }

    #[test]
    fn a_doubled_question_mark_is_one_literal_question_mark() {
        assert_eq!(
            pg("select a ?? 'k', ?", &[Binding::One]),
            "select a ? 'k', $1"
        );
        // And the escape is what makes the jsonb `?` operator writable at all.
        assert_eq!(pg("select a ?? 'k'", &[]), "select a ? 'k'");
    }

    #[test]
    fn a_cast_is_not_a_name() {
        let statement = pg_named("select :id::text", &[("id", Binding::One)]);
        assert_eq!(statement.sql, "select $1::text");
    }

    /// § 5's two PostgreSQL-only exceptions in one statement, because they are
    /// one scan and not two passes: `::` must not open a name, `?|` must not
    /// be a placeholder, `??` must render as jsonb's own one-byte operator,
    /// and the real markers between them must still be numbered in the order
    /// they were written.
    #[test]
    fn a_postgres_cast_and_a_jsonb_question_mark_survive_the_rewrite() {
        let statement = pg_named(
            "select :id::text from t where doc ?| array['a'] and doc ?? :key and n = :id::int",
            &[("id", Binding::One), ("key", Binding::One)],
        );
        assert_eq!(
            statement.sql,
            "select $1::text from t where doc ?| array['a'] and doc ? $2 and n = $1::int"
        );
        assert_eq!(
            statement.binds,
            [Source { arg: 0, element: 0 }, Source { arg: 1, element: 0 },]
        );
        assert_eq!(statement.arity(), 2);
    }

    #[test]
    fn a_quoted_identifier_hides_a_placeholder_in_the_dialect_that_has_it() {
        assert_eq!(
            rewrite(
                "select `a?b`, ?",
                Params::Positional(&[Binding::One]),
                Dialect::MySql
            )
            .expect("one placeholder")
            .sql,
            "select `a?b`, ?"
        );
        assert_eq!(
            rewrite(
                "select [a?b], ?",
                Params::Positional(&[Binding::One]),
                Dialect::SqlServer
            )
            .expect("one placeholder")
            .sql,
            "select [a?b], @p1"
        );
        // The same bracket is a subscript on PostgreSQL, so the `?` inside it
        // is a placeholder there.
        assert_eq!(pg("select a[?]", &[Binding::One]), "select a[$1]");
    }

    #[test]
    fn a_hash_comment_is_mysqls_alone() {
        assert_eq!(
            rewrite(
                "select ? # ?\n",
                Params::Positional(&[Binding::One]),
                Dialect::MySql
            )
            .expect("one placeholder")
            .sql,
            "select ? # ?\n"
        );
        assert_eq!(
            pg("select ? # ?", &[Binding::One, Binding::One]),
            "select $1 # $2"
        );
    }

    #[test]
    fn a_spelling_that_disagrees_with_the_arguments_is_refused_both_ways() {
        let named = refused("select :id", Params::Positional(&[Binding::One]));
        assert!(
            named.contains("binds `:id` but its arguments are a list"),
            "{named}"
        );
        let positional = refused("select ?", Params::Named(&[("id", Binding::One)]));
        assert!(
            positional.contains("binds `?` but its arguments are string-keyed"),
            "{positional}"
        );
    }

    #[test]
    fn a_positional_count_is_refused_on_both_sides_of_the_match() {
        let too_few = refused("select ?, ?", Params::Positional(&[Binding::One]));
        assert!(
            too_few.contains("2 `?` placeholder(s) and was given 1"),
            "{too_few}"
        );
        let too_many = refused(
            "select ?",
            Params::Positional(&[Binding::One, Binding::One]),
        );
        assert!(
            too_many.contains("1 `?` placeholder(s) and was given 2"),
            "{too_many}"
        );
    }

    #[test]
    fn a_name_with_no_argument_and_an_argument_with_no_name_are_both_refused() {
        let unbound = refused("select :nope", Params::Named(&[("id", Binding::One)]));
        assert!(unbound.contains("no argument is keyed `nope`"), "{unbound}");
        let unused = refused(
            "select :id",
            Params::Named(&[("id", Binding::One), ("spare", Binding::One)]),
        );
        assert!(unused.contains("keyed `spare` is never bound"), "{unused}");
    }

    #[test]
    fn a_statement_with_no_placeholders_and_no_arguments_passes_through_whole() {
        let sql = "select now()";
        assert_eq!(pg(sql, &[]), sql);
        assert_eq!(pg_named(sql, &[]).arity(), 0);
    }

    /// Runs one statement all the way through a cache, as a driver does.
    fn cached(cache: &mut StatementCache, sql: &str, arity: usize) -> Prepared {
        let answer = cache.prepare(sql, arity);
        if let Prepared::Miss { name, .. } = &answer {
            cache.commit(sql, arity, name.clone());
        }
        answer
    }

    #[test]
    fn the_first_execution_parses_and_every_later_one_does_not() {
        let mut cache = StatementCache::new(4);
        let first = cached(&mut cache, "select 1", 0);
        assert!(matches!(first, Prepared::Miss { evicted: None, .. }));
        assert_eq!(cache.len(), 1);
        let second = cached(&mut cache, "select 1", 0);
        assert_eq!(second, Prepared::Hit(first.name().to_string()));
        // A hit records nothing new: it is the same statement on the server.
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn one_text_at_two_arities_is_two_server_side_statements() {
        // § 1's key, and the reason it is not the SQL text alone: `IN` over
        // three ids and over four are two different texts on the wire.
        let mut cache = StatementCache::new(4);
        let three = cached(&mut cache, "select * from t where id in ?", 3);
        let four = cached(&mut cache, "select * from t where id in ?", 4);
        assert_ne!(three.name(), four.name());
        assert_eq!(cache.len(), 2);
        assert_eq!(
            cached(&mut cache, "select * from t where id in ?", 3),
            Prepared::Hit(three.name().to_string())
        );
    }

    #[test]
    fn a_full_cache_evicts_the_least_recently_used_and_names_it_to_be_closed() {
        let mut cache = StatementCache::new(2);
        let a = cached(&mut cache, "select 'a'", 0);
        let b = cached(&mut cache, "select 'b'", 0);
        // Touching `a` makes `b` the least recently used one.
        assert!(matches!(cache.prepare("select 'a'", 0), Prepared::Hit(_)));
        let evicted = match cache.prepare("select 'c'", 0) {
            Prepared::Miss { evicted, .. } => evicted,
            other => panic!("a third statement should not fit: {other:?}"),
        };
        assert_eq!(evicted.as_deref(), Some(b.name()));
        assert_eq!(
            cache.len(),
            1,
            "the victim left before the newcomer arrived"
        );
        assert!(matches!(cache.prepare("select 'a'", 0), Prepared::Hit(_)));
        drop(a);
    }

    #[test]
    fn a_name_is_never_reused_even_after_its_statement_is_closed() {
        // A `Close` and the `Parse` that follows it are in flight together, so a
        // recycled name would bind against whichever the server saw last.
        let mut cache = StatementCache::new(1);
        let first = cached(&mut cache, "select 'a'", 0);
        let second = cached(&mut cache, "select 'b'", 0);
        let third = cached(&mut cache, "select 'c'", 0);
        assert_ne!(first.name(), second.name());
        assert_ne!(second.name(), third.name());
        assert_ne!(first.name(), third.name());
    }

    #[test]
    fn a_statement_that_never_parsed_is_not_remembered() {
        // The split that costs a round trip when it goes wrong and never a
        // wrong answer: an uncommitted miss is simply missed again.
        let mut cache = StatementCache::new(4);
        let first = cache.prepare("select 1", 0);
        assert!(matches!(first, Prepared::Miss { .. }));
        assert!(cache.is_empty());
        assert!(matches!(
            cache.prepare("select 1", 0),
            Prepared::Miss { .. }
        ));
    }

    #[test]
    fn a_capacity_of_zero_is_the_unnamed_statement_every_time() {
        let mut cache = StatementCache::new(0);
        assert_eq!(cache.capacity(), 0);
        assert_eq!(cached(&mut cache, "select 1", 0), Prepared::Unnamed);
        assert_eq!(cached(&mut cache, "select 1", 0), Prepared::Unnamed);
        assert_eq!(Prepared::Unnamed.name(), "");
        assert!(cache.is_empty());
    }

    /// § 1's three answers for one field, asserted together: a block that says
    /// nothing is the default, a block that says a number is that number, and
    /// a block that says `0` is the cache turned off rather than a block that
    /// said nothing. The last is the one a `unwrap_or_default`-shaped reader
    /// gets wrong while still looking right on the first two.
    #[test]
    fn an_unset_statement_cache_is_the_default_and_a_written_zero_is_not() {
        let mut block = Database::default();
        assert_eq!(statement_cache_for(&block), DEFAULT_STATEMENT_CACHE);

        block.statement_cache = Some(4);
        assert_eq!(statement_cache_for(&block), 4);

        block.statement_cache = Some(0);
        assert_eq!(statement_cache_for(&block), 0);
        assert_ne!(statement_cache_for(&block), DEFAULT_STATEMENT_CACHE);
        assert_eq!(
            StatementCache::new(statement_cache_for(&block)).prepare("select 1", 0),
            Prepared::Unnamed
        );
    }

    /// § 9's zone in every spelling the field accepts, including both sides of
    /// the 18-hour bound. The sweep is asserted as offsets rather than one
    /// line per spelling because a parser that dropped a `:MM` field still
    /// answers plausibly on `+02:00`.
    #[test]
    fn an_unwritten_zone_is_utc_and_every_accepted_spelling_is_its_offset() {
        let mut block = Database::default();
        assert_eq!(time_zone_for(&block), Some(0));

        for (written, seconds) in [
            ("UTC", 0),
            ("utc", 0),
            ("Z", 0),
            ("+00:00", 0),
            ("-00:00", 0),
            ("+02", 2 * 3600),
            ("+02:00", 2 * 3600),
            ("-05:30", -(5 * 3600 + 30 * 60)),
            ("+05:45", 5 * 3600 + 45 * 60),
            ("  +02:00  ", 2 * 3600),
            ("-03:30:15", -(3 * 3600 + 30 * 60 + 15)),
            ("+18:00", 18 * 3600),
        ] {
            block.time_zone = Some(written.to_owned());
            assert_eq!(time_zone_for(&block), Some(seconds), "{written}");
        }
    }

    /// The other half of that bound, and the spellings that would otherwise be
    /// read as UTC. `None` and not `Some(0)` is the whole assertion: folding
    /// an unparseable zone into the default is the failure this reader exists
    /// to make impossible.
    #[test]
    fn a_zone_that_is_not_an_offset_is_no_offset_rather_than_utc() {
        let mut block = Database::default();
        for written in [
            "Europe/Vienna",
            "CET",
            "",
            "+0200",
            "+2:00",
            "02:00",
            "+02:60",
            "+02:00:60",
            "+18:00:01",
            "-19:00",
            "+02:00:00:00",
            "+02:0a",
        ] {
            block.time_zone = Some(written.to_owned());
            assert_eq!(time_zone_for(&block), None, "{written}");
        }
    }

    #[test]
    fn a_reset_that_deallocated_everything_leaves_nothing_claimed() {
        // MySQL's and SQL Server's resets, not PostgreSQL's — § 13's asymmetry.
        let mut cache = StatementCache::new(4);
        cached(&mut cache, "select 1", 0);
        cache.clear();
        assert!(cache.is_empty());
        assert!(matches!(
            cache.prepare("select 1", 0),
            Prepared::Miss { .. }
        ));
    }

    #[test]
    fn multibyte_text_around_a_placeholder_survives_the_cut() {
        // Every cut this scan makes is at an ASCII byte; the case exists
        // because a cut inside a character would panic rather than misbehave.
        assert_eq!(pg("select 'é☃', ?", &[Binding::One]), "select 'é☃', $1");
    }
}
