//! What a `Core\Db` call's SQL has to satisfy at *compile* time.
//!
//! [ADR 0067 § 4](/docs/decisions/0067.md)'s one-statement rule, the
//! closing delimiter every region of a statement owes, and § 10's placeholder
//! count are properties of a literal string alone, so a call that breaks any of
//! them is refused where it is written and never reaches a connection. What is
//! below is the whole of that surface and the only part of this module
//! `nvs-types` calls — which is the edge that puts the front end above this
//! crate.
//!
//! It imports nothing from the rest of `db`: what it reads is the caller's own
//! string and [`nvs_db::sql`], which is why the dependency it creates is on
//! this module and not on a database connection.

/// Whether every region a literal query opens is one it also closes —
/// `rule:core-classes/db-compile-time-query-checking`'s "an unterminated string
/// literal", over [`nvs_db::sql::holds_an_unterminated_region`].
///
/// A fact about the text alone, like [`check_single_statement`]'s, so it is
/// asked of every literal query and not only of one whose params array this
/// pass could read whole. It is an earlier answer and not a different one in
/// the way `rule:expressions/preparation-preserves-behaviour` asks for:
/// [`nvs_db::sql::rewrite`] refuses these same texts when a request runs them,
/// because a region the text never leaves makes what the statement binds what
/// fits inside an opening delimiter rather than what was written.
///
/// All four dialects, for [`check_written_query`]'s reason: a backtick opens a
/// quoted name on MySQL and is ordinary text on PostgreSQL, so a region only
/// one of them enters is a region this pass says nothing about.
///
/// # Errors
///
/// One message, since there is only one thing this can find.
pub fn check_closed_regions(sql: &str) -> Result<(), String> {
    let unterminated = [
        nvs_db::sql::Dialect::PostgreSql,
        nvs_db::sql::Dialect::MySql,
        nvs_db::sql::Dialect::Sqlite,
        nvs_db::sql::Dialect::SqlServer,
    ]
    .into_iter()
    .all(|dialect| nvs_db::sql::holds_an_unterminated_region(sql, dialect));
    if unterminated {
        return Err(
            "a string literal, a quoted name or a comment this statement opens is never \
             closed — every byte after it is read as being inside it, so what the statement \
             binds is what fits inside an opening delimiter rather than what was written"
                .to_owned(),
        );
    }
    Ok(())
}

/// Whether a literal query holds the one statement [ADR 0067
/// § 1](/docs/decisions/0067.md) prepares — § 10's "a refused second
/// statement", over [`nvs_db::sql::holds_a_second_statement`].
///
/// All four dialects, for [`check_written_query`]'s reason and with the same
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
/// § 10](/docs/decisions/0067.md)'s "a literal params array", which is
/// the only shape it promises anything about.
///
/// § 5's two spellings are exclusive, so this is two cases and not a pair: a
/// list-keyed array is positional and a string-keyed one is named. An array the
/// checker cannot read whole — a spread, a variable, a mixture of keyed and
/// unkeyed elements — is not one of these at all, and the call is left to run
/// time.
#[derive(Debug, Clone, Copy)]
pub enum WrittenQueryParams<'a> {
    /// A list-keyed array literal, and how many elements it has.
    Positional(usize),
    /// A string-keyed array literal, its keys in written order.
    Named(&'a [&'a str]),
}

/// Whether a literal query agrees with the literal params array written beside
/// it — [ADR 0067 § 10](/docs/decisions/0067.md)'s "placeholder count
/// against a literal params array, positional-vs-named consistency".
///
/// It is [`nvs_db::sql::rewrite`] itself and deliberately not a second reader of
/// the same grammar: `rule:expressions/preparation-preserves-behaviour` asks that this pass produce an earlier answer
/// and never a different one, which is a property of *asking the runtime's own
/// question* rather than of two scanners being kept in step. The message handed
/// back is the one the first call would have thrown.
///
/// **A refusal must hold on every dialect.** The scan is dialect-shaped — a `?`
/// inside backticks is a placeholder on PostgreSQL and text on MySQL — and a
/// call site does not name the driver its `[db.<name>]` block will resolve to.
/// So all four are asked and the first acceptance ends it, which is the
/// soundness direction `rule:expressions/preparation-preserves-behaviour` fixes: a refusal that would be a guess is
/// not made. Where they all refuse, PostgreSQL's wording is the one reported,
/// since two dialects can refuse the same query over different placeholder
/// counts.
///
/// # Errors
///
/// The rewriter's own message, where every dialect refused.
pub fn check_written_query(sql: &str, params: WrittenQueryParams<'_>) -> Result<(), String> {
    let positional;
    let named;
    let params = match params {
        WrittenQueryParams::Positional(count) => {
            positional = vec![nvs_db::sql::Binding::One; count];
            nvs_db::sql::Params::Positional(&positional)
        }
        WrittenQueryParams::Named(keys) => {
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
