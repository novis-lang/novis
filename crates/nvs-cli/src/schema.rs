//! `nvs schema plan|apply|dump` — `rule:core-classes/schema-converges`'s
//! convergence as the operator's own command, beside `nvs queue migrate` and for
//! its reason.
//!
//! A schema value says what the tables should be; a plan is the difference
//! between that and what a connection has. There is no version number, no
//! history table and no ordering between changes, because the difference is
//! computed against the server every time — which is what makes it correct after
//! a manual change, after a half-applied earlier plan, and against a database
//! this tool has never seen.
//!
//! The three verbs are the same walk stopped at different points:
//!
//! * **`dump`** reads the database and writes the schema value out as JSON. That
//!   is the whole of reverse-engineering an existing database, and
//!   `rule:core-classes/schema-introspection` is why it is a catalog read rather
//!   than a `CREATE TABLE` parsed back: there is no DDL parser in this tree at
//!   any tier, and the catalog is structured tables where a dump is a language.
//! * **`plan`** reads a schema value from a JSON file, introspects the
//!   connection, and prints every step with its grade, its reason and its
//!   complete SQL — including the steps `apply` will refuse to run.
//!   `rule:core-classes/schema-plan` is why nothing is elided: in a serious
//!   deployment the application's own credentials cannot issue DDL at all, a DBA
//!   applies the change from a ticket, and a plan they can paste is the product.
//! * **`apply`** runs the steps of that plan that may run.
//!   `rule:core-classes/schema-absence-never-destroys` keeps a report out of
//!   that set whichever flag was written, so a table the schema does not declare
//!   is printed with the SQL that would drop it and never dropped.
//!
//! ## The capability is deliberately not asked
//!
//! `rule:core-classes/schema-apply-capability` gates `Core\Db\Schema::applySafe`
//! behind the deny-by-default `db.schema`, and that grant bounds a *program*: it
//! is keyed on the entry file this command does not have. The principal here is
//! the operator who ran it, over configuration only a trusted account may write
//! (`rule:config/ownership-is-the-trust-boundary`), which is exactly the
//! reasoning [`crate::queue`]'s module doc gives for `db.connect`. Asking a
//! program's capability of the operator's own command would be a check with
//! nothing to check.
//!
//! **`--including-risky` is the operator's spelling of the two entry points.**
//! Without it, `apply` refuses a plan holding a step that is not `Safe` and
//! names the first one — `applySafe`'s rule, in a command where the claim is
//! made on the command line rather than at a call site.
//!
//! ## Why this opens its own connection
//!
//! [`nvs_db::direct`] is the connection-generic path for a caller with no
//! request behind it, and it answers all five drivers because a catalog read is
//! the same read on each. [`crate::queue`] opens the three its statement lists
//! cover inside one macro that applies as it opens; this command has to hold the
//! connection across a read, a diff and a write, so it hands back an
//! [`nvs_db::Connection`] instead. Whether the two openers become one is a
//! question for the slice that gives `nvs queue migrate` a schema value.
//!
//! Cost: one connection for the length of the command, closed with the process,
//! and no pool — a pool exists to be reused by a second request, and this is a
//! command.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use nvs_db::{Connection, Grade, Plan, Schema};
use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};

use crate::config::{LocalFiles, named_roots, working_directory};
use crate::queue::address_of;
use crate::render_diagnostics;

/// How long the whole handshake may take, as [`crate::queue`]'s own deadline and
/// for its reason: finite because a command that hangs reports nothing at all,
/// and generous because the server is often a container the operator started
/// moments ago.
const CONNECT_DEADLINE: Duration = Duration::from_secs(10);

/// `nvs schema dump --connection <name> [<file>...]`.
///
/// The schema value the database already holds, as the JSON `plan` and `apply`
/// read back. Nothing is written to the server and nothing is compared: this is
/// one introspection printed.
pub(crate) fn dump(config: &[PathBuf], paths: &[PathBuf], connection: &str) -> ExitCode {
    let mut conn = match opened(config, paths, connection) {
        Ok(conn) => conn,
        Err(code) => return code,
    };
    let schema = match introspected(&mut conn, connection) {
        Ok(schema) => schema,
        Err(code) => return code,
    };
    println!("{}", json_of(&schema.to_array()));
    ExitCode::SUCCESS
}

/// `nvs schema plan --connection <name> --schema <file> [<file>...]`.
///
/// The dry run, and the document a DBA applies by hand. Every step is printed
/// whatever its grade, because `rule:core-classes/schema-plan` makes eliding one
/// the failure this command exists to avoid.
pub(crate) fn plan(
    config: &[PathBuf],
    paths: &[PathBuf],
    connection: &str,
    schema: &Path,
) -> ExitCode {
    let (mut conn, want) = match both(config, paths, connection, schema) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let plan = match planned(&mut conn, &want, connection) {
        Ok(plan) => plan,
        Err(code) => return code,
    };
    print!("{plan}");
    println!("-- {}", counted(&plan));
    ExitCode::SUCCESS
}

/// `nvs schema apply --connection <name> --schema <file> [--including-risky]`.
///
/// The same plan, run. A statement refused mid-way leaves the steps before it
/// applied and exits non-zero naming the change that failed — safe to re-run,
/// because the next plan is computed against the database as it is then.
pub(crate) fn apply(
    config: &[PathBuf],
    paths: &[PathBuf],
    connection: &str,
    schema: &Path,
    including_risky: bool,
) -> ExitCode {
    let (mut conn, want) = match both(config, paths, connection, schema) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    converge(&mut conn, &want, connection, including_risky)
}

/// The plan, run — which is `apply` above once the schema has been obtained, and
/// what `nvs queue migrate` runs too.
///
/// The schema is the caller's because the two commands answer for a different
/// one: this command's is a file an operator wrote, and the queue's is the value
/// `nvs_stdlib::queue::schema` owns. Everything after that is the same
/// convergence, so it is written once.
pub(crate) fn converge(
    conn: &mut Connection,
    want: &Schema,
    connection: &str,
    including_risky: bool,
) -> ExitCode {
    let plan = match planned(conn, want, connection) {
        Ok(plan) => plan,
        Err(code) => return code,
    };

    // `rule:core-classes/schema-apply-capability`'s two entry points, as one
    // flag: the refusal reads the grades of the steps that would **run**, since
    // a plan against any shared database carries reports it was never going to
    // touch and reading those too would refuse every plan ever computed.
    if !including_risky && let Some(step) = plan.first_refused() {
        eprintln!(
            "error: the plan holds a step that is not `Safe`, and this command runs none of it — \
             `{}` is {} because {}",
            step.change(),
            step.grade(),
            step.reason()
        );
        eprintln!(
            "note: `--including-risky` runs the same plan and says so where it is written, which \
             is `Core\\Db\\Schema::applyIncludingRisky`'s rule spelled on a command line"
        );
        return ExitCode::FAILURE;
    }

    let mut ran = 0;
    for step in plan.runnable() {
        for sql in step.sql() {
            if let Err(err) = nvs_db::direct::run(conn, sql) {
                eprintln!(
                    "error: `{}` was refused by the server: {err}",
                    step.change()
                );
                eprintln!(
                    "note: the steps before it are applied, and the next plan is computed against \
                     the database as it is now — so re-running this command once the refusal is \
                     fixed converges the rest rather than colliding with what is already there"
                );
                return ExitCode::FAILURE;
            }
        }
        println!("-- [{}] {}: applied", step.grade(), step.change());
        ran += 1;
    }
    println!("-- applied {ran} step(s), of {}", counted(&plan));
    ExitCode::SUCCESS
}

/// The connection and the declared schema, which `plan` and `apply` both need
/// and in this order: the file is read first, so a JSON refusal costs no
/// handshake.
fn both(
    config: &[PathBuf],
    paths: &[PathBuf],
    connection: &str,
    schema: &Path,
) -> Result<(Connection, Schema), ExitCode> {
    let want = declared(schema)?;
    Ok((opened(config, paths, connection)?, want))
}

/// `rule:core-classes/schema-converges`'s difference: the declared schema
/// against the database, with the header that says what the rest of the output
/// is.
///
/// The header is printed here rather than by each caller because it describes
/// the *plan*, and `apply` owes its reader the same sentence `plan` does: how
/// many steps there are, how many would run, and how many of those are `Safe`.
fn planned(conn: &mut Connection, want: &Schema, named: &str) -> Result<Plan, ExitCode> {
    let driver = conn.driver();
    let server = nvs_db::ddl::Server::of(conn);
    let have = introspected(conn, named)?;
    let plan = nvs_db::diff_on(want, &have, nvs_db::Dialect::of(driver), server);
    let runnable = plan.runnable().count();
    let safe = plan
        .runnable()
        .filter(|step| step.grade() == Grade::Safe)
        .count();
    println!(
        "-- `[db.{named}]` on {}: {} step(s), of which {runnable} would run and {safe} of those \
         are Safe.",
        driver.display_name(),
        plan.len()
    );
    Ok(plan)
}

/// The plan's grades, as the trailer under the document.
///
/// The words are `Core\Db\Plan\Grade`'s own case names, so what an operator
/// reads here is what a program written against the same plan matches on. The
/// leading comment marker is the caller's, because `apply` puts its own count in
/// front of this one.
fn counted(plan: &Plan) -> String {
    format!(
        "{} Safe, {} Locking, {} Destructive; {} reported and never applied.",
        plan.count(Grade::Safe),
        plan.count(Grade::Locking),
        plan.count(Grade::Destructive),
        plan.steps().iter().filter(|step| step.is_report()).count()
    )
}

/// The database behind `conn`, as the schema value it introspects into.
///
/// `pub(crate)` for the boot: [`crate::serve`] asks the same question of the connection a `[queue]`
/// block names, and one catalog read owes an operator one refusal however it was reached.
pub(crate) fn introspected(conn: &mut Connection, named: &str) -> Result<Schema, ExitCode> {
    nvs_db::direct::schema_of(conn).map_err(|why| {
        eprintln!("error: `[db.{named}]` could not be read: {why}");
        ExitCode::FAILURE
    })
}

/// The schema value a JSON file declares.
///
/// The file holds `rule:core-classes/schema-is-a-value`'s canonical array form,
/// which is what `dump` writes and what a program passes to
/// `Core\Db\Schema::fromArray` — one form, three spellings of it, and no second
/// grammar for a file to be wrong in.
fn declared(path: &Path) -> Result<Schema, ExitCode> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        eprintln!("error: {} could not be read: {err}", path.display());
        ExitCode::FAILURE
    })?;
    let document: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
        eprintln!("error: {} is not a JSON document: {err}", path.display());
        ExitCode::FAILURE
    })?;
    let node = node_of(&document, "the document").map_err(|why| {
        eprintln!("error: {} does not hold a schema: {why}", path.display());
        ExitCode::FAILURE
    })?;
    nvs_db::Schema::from_array(&node).map_err(|why| {
        eprintln!("error: {} does not hold a schema: {why}", path.display());
        eprintln!(
            "note: the form is the one `nvs schema dump` writes and \
             `Core\\Db\\Schema::fromArray` reads, and its vocabulary is closed on purpose"
        );
        ExitCode::FAILURE
    })
}

/// The `[db.<name>]` block `--connection` names, resolved out of the merged tree
/// and opened.
///
/// The roots are read exactly as [`crate::config::check`] reads them, for the
/// reason that function gives: an audit and a convergence are the same question
/// about the same files, so naming a root positionally disables the search for
/// `./nvs.toml` here too. The name is proven against the tree before a socket is
/// touched, because refusing a database nobody configured is worth more than a
/// connection error against a host nobody meant.
pub(crate) fn opened(
    config: &[PathBuf],
    paths: &[PathBuf],
    name: &str,
) -> Result<Connection, ExitCode> {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let named = named_roots(config, paths);
    let resolved = working_directory().and_then(|cwd| {
        let roots = crate::config::roots_in(&named, &cwd);
        nvs_config::resolve::resolve(&roots, &mut sources, &files)
    });
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(diagnostic) => return Err(refuse(diagnostic, &mut sources)),
    };

    let Some(block) = resolved.config.db.get(name) else {
        let known = if resolved.config.db.is_empty() {
            "this tree writes no `[db]` block at all".to_owned()
        } else {
            let names: Vec<String> = resolved
                .config
                .db
                .keys()
                .map(|known| format!("`{known}`"))
                .collect();
            format!("the blocks this tree writes are {}", names.join(", "))
        };
        eprintln!("error: no `[db.{name}]` block is in force, so there is nothing to converge");
        eprintln!("note: {known}");
        return Err(ExitCode::FAILURE);
    };

    let Some(written) = block.driver.as_deref() else {
        eprintln!("error: `[db.{name}]` names no `driver`, so it is not openable at all");
        return Err(ExitCode::FAILURE);
    };
    let Some(driver) = nvs_db::Driver::from_config_name(written) else {
        eprintln!(
            "error: `[db.{name}]` names the `{written}` driver, which Novis has no backend for"
        );
        eprintln!(
            "note: `rule:core-classes/db-one-api`'s five are {}",
            nvs_db::Driver::ALL
                .iter()
                .map(|one| format!("`{}`", one.matrix_name()))
                .collect::<Vec<_>>()
                .join(", ")
        );
        return Err(ExitCode::FAILURE);
    };

    open(name, block, driver)
}

/// One driver's half of [`opened`]: the block resolved as that driver's target,
/// and the socket opened to it.
///
/// **A macro because the arms differ in names and not in shape**, which is
/// [`crate::queue`]'s reasoning verbatim: `rule:core-classes/db-drivers-are-an-enum`
/// makes the drivers an enum rather than a trait, so there is no type parameter
/// to write this as a generic function over, and the refusals are already one
/// vocabulary in `BlockError`.
macro_rules! open_over_tcp {
    ($target:ty, $conn:ty, $port:path, $held:path, $name:expr, $block:expr) => {{
        let target = match <$target>::resolve($block) {
            Ok(target) => target,
            Err(refused) => {
                eprintln!("error: {}", refused.refusal($name));
                return Err(ExitCode::FAILURE);
            }
        };
        let Some(address) = address_of(target.host, $block.port, $port) else {
            eprintln!(
                "error: `[db.{}]` names the host `{}`, which resolves to no address",
                $name, target.host
            );
            return Err(ExitCode::FAILURE);
        };
        match <$conn>::connect(address, &target, Some(Instant::now() + CONNECT_DEADLINE)) {
            Ok(conn) => Ok($held(conn)),
            Err(err) => {
                eprintln!("error: `[db.{}]` at {address} did not open: {err}", $name);
                Err(ExitCode::FAILURE)
            }
        }
    }};
}

/// The block opened as whichever of the five drivers it named.
///
/// SQLite is the arm that is not a socket at all — a path, opened where the
/// process stands — which is why it is worth having here: a schema is most often
/// converged first against a developer's own file, and refusing to do that would
/// make the one backend that needs no container the one this command could not
/// reach.
/// `pub(crate)` for the caller that already holds a block: [`opened`] resolves one out of the files
/// on disk, and the boot in [`crate::serve`] has the merged tree in hand and no roots to read.
pub(crate) fn open(
    name: &str,
    block: &nvs_config::tree::Database,
    driver: nvs_db::Driver,
) -> Result<Connection, ExitCode> {
    match driver {
        nvs_db::Driver::Postgres => open_over_tcp!(
            nvs_db::PgTarget<'_>,
            nvs_db::PgConn,
            nvs_db::pg::DEFAULT_PORT,
            Connection::Postgres,
            name,
            block
        ),
        nvs_db::Driver::MySql => open_over_tcp!(
            nvs_db::MySqlTarget<'_>,
            nvs_db::MySqlConn,
            nvs_db::mysql::DEFAULT_PORT,
            Connection::MySql,
            name,
            block
        ),
        nvs_db::Driver::MariaDb => open_over_tcp!(
            nvs_db::MariaTarget<'_>,
            nvs_db::MariaConn,
            nvs_db::maria::DEFAULT_PORT,
            Connection::MariaDb,
            name,
            block
        ),
        nvs_db::Driver::SqlServer => open_over_tcp!(
            nvs_db::TdsTarget<'_>,
            nvs_db::TdsConn,
            nvs_db::tds::DEFAULT_PORT,
            Connection::SqlServer,
            name,
            block
        ),
        nvs_db::Driver::Sqlite => {
            let target = match nvs_db::SqliteTarget::resolve(block) {
                Ok(target) => target,
                Err(refused) => {
                    eprintln!("error: {}", refused.refusal(name));
                    return Err(ExitCode::FAILURE);
                }
            };
            match nvs_db::sqlite::open(&target) {
                Ok(conn) => Ok(Connection::Sqlite(conn)),
                Err(err) => {
                    eprintln!("error: `[db.{name}]` did not open: {err}");
                    Err(ExitCode::FAILURE)
                }
            }
        }
    }
}

/// A configuration refusal, rendered with the line of the file it came from —
/// [`crate::queue`]'s own, for the reason its doc gives.
fn refuse(diagnostic: Diagnostic, sources: &mut SourceMap) -> ExitCode {
    let mut diags = Diagnostics::new();
    diags.report(diagnostic);
    render_diagnostics(&mut diags, sources);
    ExitCode::FAILURE
}

/// The canonical array form as a JSON document, indented two spaces.
///
/// Written here rather than through `serde_json` for one reason: a
/// [`nvs_db::Node`] map is an **ordered** list of pairs, because
/// `rule:expressions/object-identity-equality` makes key order part of an
/// array's value — and a serializer holding its keys in a sorted map would
/// reorder them on the way out. Reading is the other direction and does not care
/// (`rule:core-classes/schema-is-a-value`'s readers take a key by name), so that
/// half is `serde_json`'s.
fn json_of(node: &nvs_db::Node) -> String {
    let mut out = String::new();
    write_node(node, 0, &mut out);
    out
}

/// One node, at `depth` levels of indentation.
fn write_node(node: &nvs_db::Node, depth: usize, out: &mut String) {
    match node {
        nvs_db::Node::Text(text) => {
            out.push_str(&serde_json::Value::String(text.clone()).to_string());
        }
        nvs_db::Node::Int(number) => out.push_str(&number.to_string()),
        nvs_db::Node::Uint(number) => out.push_str(&number.to_string()),
        // `{:?}` rather than `{}`, so a whole number keeps the `.0` that tells a
        // reader — and this file's own reader — that it is a float.
        nvs_db::Node::Float(number) => out.push_str(&format!("{number:?}")),
        nvs_db::Node::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        nvs_db::Node::List(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                indent(depth + 1, out);
                write_node(item, depth + 1, out);
                out.push_str(if index + 1 == items.len() {
                    "\n"
                } else {
                    ",\n"
                });
            }
            indent(depth, out);
            out.push(']');
        }
        nvs_db::Node::Map(pairs) => {
            if pairs.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (index, (key, item)) in pairs.iter().enumerate() {
                indent(depth + 1, out);
                out.push_str(&serde_json::Value::String(key.clone()).to_string());
                out.push_str(": ");
                write_node(item, depth + 1, out);
                out.push_str(if index + 1 == pairs.len() {
                    "\n"
                } else {
                    ",\n"
                });
            }
            indent(depth, out);
            out.push('}');
        }
    }
}

/// Two spaces per level.
fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// A JSON document as the array form's node tree.
///
/// **JSON has one number type and the form has two integers**, and the whole of
/// the ambiguity is one key: a `uint` default is the only place a
/// [`nvs_db::Node::Uint`] appears at all, and its own key says so. Everywhere
/// else an integer that fits an `i64` is one, which is what every length,
/// ordinal and `int` default in the form is.
///
/// A `null` is refused rather than dropped. The form spells absence by leaving a
/// key out — `null` in it is a file that means to say something and has not.
fn node_of(value: &serde_json::Value, at: &str) -> Result<nvs_db::Node, String> {
    Ok(match value {
        serde_json::Value::String(text) => nvs_db::Node::Text(text.clone()),
        serde_json::Value::Bool(flag) => nvs_db::Node::Bool(*flag),
        serde_json::Value::Number(number) => {
            if let Some(signed) = number.as_i64() {
                nvs_db::Node::Int(signed)
            } else if let Some(unsigned) = number.as_u64() {
                nvs_db::Node::Uint(unsigned)
            } else if let Some(float) = number.as_f64() {
                nvs_db::Node::Float(float)
            } else {
                return Err(format!("{at} holds a number nothing can read"));
            }
        }
        serde_json::Value::Array(items) => nvs_db::Node::List(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| node_of(item, &format!("{at}[{index}]")))
                .collect::<Result<Vec<nvs_db::Node>, String>>()?,
        ),
        serde_json::Value::Object(pairs) => nvs_db::Node::Map(
            pairs
                .iter()
                .map(|(key, item)| {
                    let at = format!("{at}.{key}");
                    let node = match (key.as_str(), item.as_u64()) {
                        ("uint", Some(unsigned)) => nvs_db::Node::Uint(unsigned),
                        _ => node_of(item, &at)?,
                    };
                    Ok((key.clone(), node))
                })
                .collect::<Result<Vec<(String, nvs_db::Node)>, String>>()?,
        ),
        serde_json::Value::Null => {
            return Err(format!(
                "{at} is null, and this form spells absence by leaving the key out"
            ));
        }
    })
}
