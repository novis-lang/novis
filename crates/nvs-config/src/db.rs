//! What a `[db.<name>]` block still owes once it has deserialized: the paths that name a file,
//! resolved — the CA bundle trust-checked as well — and the durations `rule:core-classes/db-one-api` states in prose —
//! § 13's pool bounds and § 11's `slow_query` threshold — read into numbers. All of it at boot.
//!
//! The paths are [`Database::tls_ca_file`](crate::tree::Database::tls_ca_file), the PEM bundle
//! `rule:core-classes/db-capabilities`'s TLS leg verifies a server's certificate against, and
//! [`Database::path`](crate::tree::Database::path), the file a SQLite block opens. Both are here
//! rather than in [`mod@crate::secret`] because neither is a secret: nothing about a CA bundle, or
//! about a database file's *name*, is confidential, and § 7's whole shape — one value, materialized
//! into a sibling directive, kept out of the merged table — is the wrong one for a file a connection
//! re-reads by path. What the bundle *shares* with § 7 is the trust boundary, which is why both run
//! at resolve time and neither at use time.
//!
//! **A CA bundle is a trust-boundary file.** Whoever can rewrite it decides which server this
//! deployment's queries and credentials go to, which is the same authority `rule:config/ownership-is-the-trust-boundary` refuses to
//! leave on the configuration files themselves. So it goes through
//! [`Files::trust`] exactly as a `password_file` does, and an
//! unreadable or group-writable bundle is `E0605`/`E0607` at boot. It is deliberately *not* read
//! here: the anchors are parsed by `nvs_host::tls`, which owns the one client configuration every
//! session in this process shares, and a second parse in this crate would be a second answer to
//! "whose certificates do you believe".
//!
//! **A SQLite `path` is resolved and nothing more**, and the difference is worth stating because
//! the fields sit one line apart in the block. [`Files::trust`] asks who *else* may write a
//! file, which is the right question about a set of trust anchors and the wrong one about a
//! database: that file is the deployment's own data, written by exactly the account the check would
//! be objecting to, and it usually does not exist at boot at all — SQLite creates it on first open,
//! so a boot-time `trust` would refuse every first run. What a `path` still owes is § 5's
//! resolution, for the reason below — and only when it is a relative file at all, which
//! [`is_relative_file`] is and owns, several of SQLite's spellings not being paths.
//!
//! **A path a *program* supplies is not resolved here and is deliberately not resolved like this
//! one.** `Db\Settings.path` reaches `Core\Db::open` as `rule:core-classes/db-capabilities`'s path sink, needing
//! `fs.read`/`fs.write`, and it stays relative to the process rather than to a configuration file
//! the program never named — `nvs_stdlib::db`'s `sqlite_settings` is that half. The asymmetry is the
//! same one § 3 draws about an address: what an operator wrote in root-owned configuration is
//! resolved against that configuration, and what a program computed is not.
//!
//! **The paths are rewritten in place**, absolute, exactly as [`mod@crate::app`] canonicalizes a
//! block's key and for its second reason: § 9's `nvs config dump` then prints the file the
//! handshake will actually open rather than a fragment whose meaning depends on which file in the
//! tree wrote it. A connection opened from a working directory that is not the configuration's —
//! every request, since `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` resolves against the *file* — would otherwise read a different
//! bundle, a different database, or none.
//!
//! **The pool's bounds are read here and not where the pool is built**, for the reason every other
//! `validate` in this crate runs at boot: an operator who wrote `lifetime = "30 minutes"` learns it
//! from the boot that refuses, naming the file and the line, rather than from the first request
//! whose acquire fails at three in the morning. [`validate`] is that pass and [`pool_for`] is the
//! reader under it, which the pool calls again when it builds itself — a few integers, once per
//! pool, against carrying a derived value through [`crate::Snapshot`] that a reload would have to
//! keep in step with the tree it came from.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, Database, Pool, Setting};
use crate::value::{Quantity, Unit};

/// Makes every `[db.<name>]` path absolute — `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` — and proves the `tls_ca_file` among them
/// is inside the trust boundary.
///
/// Runs over the merged tree for [`mod@crate::secret`]'s reason: which bundle is in force is a
/// question only the merge has answered, and trust-checking a path a later file replaced would
/// refuse a boot over a file the deployment does not use.
///
/// # Errors
///
/// `E0607` for a bundle outside `rule:config/ownership-is-the-trust-boundary`'s trust boundary and `E0605` for one that cannot be
/// read at all — [`crate::resolve::untrusted`]'s split, so an operator told "cannot read" goes
/// looking for a typo and one told the other goes looking at a mode. A `path` resolves and cannot
/// fail: it is arithmetic on a string, and this module's doc says why the file behind it is not
/// asked about here.
pub fn canonicalize(
    config: &mut Config,
    table: &mut toml::value::Table,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    for (name, db) in &mut config.db.blocks {
        if let Some(written) = db
            .path
            .as_deref()
            .filter(|written| is_relative_file(written))
        {
            let base = written_in(origins, &format!("db.{name}.path"));
            let path = crate::resolve::absolute(base, Path::new(written))
                .to_string_lossy()
                .into_owned();
            rewrite(table, name, "path", &path);
            db.path = Some(path);
        }
        let Some(written) = db.tls_ca_file.as_deref() else {
            continue;
        };
        let base = written_in(origins, &format!("db.{name}.tls_ca_file"));
        let path = crate::resolve::absolute(base, Path::new(written));
        let trusted = files.trust(&path).map_err(|why| {
            crate::resolve::untrusted(
                &path,
                &why,
                "a `[db]` block's `tls_ca_file` names it, and whoever can write it chooses which \
                 server the connection may be talking to",
            )
        })?;
        let trusted = trusted.to_string_lossy().into_owned();
        rewrite(table, name, "tls_ca_file", &trusted);
        db.tls_ca_file = Some(trusted);
    }
    Ok(())
}

/// Makes every `[storage.<name>] root` absolute against the file that wrote it —
/// `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`, as [`canonicalize`]
/// does for a `[db]` path.
///
/// A disk's objects are opened through the same doors as any other path, and those doors refuse a
/// relative one (`rule:programs/relative-paths-resolve-from-their-file`), so a root left relative
/// would make every object on the disk unreachable. Like a `path`, it is arithmetic on a string and
/// cannot fail: whether the directory exists is the disk's own question, asked when it is used.
pub fn canonicalize_storage(
    config: &mut Config,
    table: &mut toml::value::Table,
    origins: &BTreeMap<String, Origin>,
) {
    for (name, disk) in &mut config.storage {
        let Some(written) = disk.root.as_deref().filter(|written| {
            let path = Path::new(written);
            !written.is_empty() && !path.has_root() && !path.is_absolute()
        }) else {
            continue;
        };
        let base = written_in(origins, &format!("storage.{name}.root"));
        let root = crate::resolve::absolute(base, Path::new(written))
            .to_string_lossy()
            .into_owned();
        if let Some(block) = table
            .get_mut("storage")
            .and_then(toml::Value::as_table_mut)
            .and_then(|disks| disks.get_mut(name))
            .and_then(toml::Value::as_table_mut)
        {
            block.insert("root".to_owned(), toml::Value::String(root.clone()));
        }
        disk.root = Some(root);
    }
}

/// The directory `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` resolves a relative key against: the one the key was written in.
///
/// A key with no origin cannot have been written in a file anywhere, so there is nothing but the
/// path itself to resolve against and `.` is the honest base — the same answer the process's own
/// working directory would give.
///
/// Reached by [`crate::http`] for `[http.client.tls] roots`, which resolves against the file that
/// wrote it by the same rule: a second answer to "which directory is this relative to" is a second
/// chance for the two passes to disagree about which file was trust-checked.
pub(crate) fn written_in<'a>(origins: &'a BTreeMap<String, Origin>, key: &str) -> &'a Path {
    origins
        .get(key)
        .and_then(|origin| origin.path.parent())
        .unwrap_or(Path::new("."))
}

/// Whether a written `path` is a relative file, and so something `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` has anything to say
/// about.
///
/// **Some of SQLite's spellings are not paths at all, and resolving one destroys it.** `:memory:`
/// is the private in-memory database, the empty string is a private temporary file the engine names
/// itself, and a `file:` scheme is a URI whose query carries `mode=memory` and `cache=shared` — the
/// spelling two handles onto one in-memory database need, which `crates/nvs-db/src/sqlite.rs`'s
/// deadlock case is written on. Prefixing any of them with a directory produces a file name
/// nothing can open, which is exactly the failure this predicate exists to have already prevented.
///
/// **A rooted path is skipped for the other half of the same reason**: § 5 resolves what is
/// *relative*, and rootedness is the honest test where [`Path::is_absolute`] is not — on Windows a
/// path written `/etc/novis.db` is not absolute (it names no drive) and is plainly not relative to
/// anything either, so joining it onto a base directory would move it to the base's drive rather
/// than resolve it.
fn is_relative_file(written: &str) -> bool {
    let path = Path::new(written);
    !written.is_empty()
        && written != ":memory:"
        && !written.starts_with("file:")
        && !path.has_root()
        && !path.is_absolute()
}

/// Puts a resolved path back at `db.<name>.<key>` of the merged table as well as onto the typed tree.
///
/// **The table is the half that reaches a driver**, and rewriting only the typed tree is a resolution
/// that silently does not happen: [`Snapshot`](crate::Snapshot) deserializes itself out of the table
/// (`Snapshot::retype`, which every path producing a `Config` goes through), so a `Config` this pass
/// rewrote is discarded and the written fragment is what a connection opens. The typed tree is
/// rewritten too because the passes between here and the snapshot — [`validate`],
/// [`crate::queue::validate`], [`crate::app::canonicalize`] — read that one.
///
/// A block the table does not hold is not an error to find: the typed tree is what says a `[db]`
/// block exists, and a key written nowhere has nothing to resolve.
fn rewrite(table: &mut toml::value::Table, name: &str, key: &str, value: &str) {
    if let Some(block) = table
        .get_mut("db")
        .and_then(toml::Value::as_table_mut)
        .and_then(|blocks| blocks.get_mut(name))
        .and_then(toml::Value::as_table_mut)
    {
        block.insert(key.to_string(), toml::Value::String(value.to_string()));
    }
}

/// `rule:security/db-pool-reset-is-a-boundary`'s pool, resolved: every bound a number, and nothing left to decide at acquire time.
///
/// Held by value and `Copy`, because the acquire path reads it and a pool is per core: a few words
/// beside a connection cost less than the pointer chase that would save most of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoolBounds {
    /// Whether a released connection rejoins a pool at all. `false` is § 13's `pool = false`, which
    /// restores connect-per-request exactly; the bounds below are then the defaults and nothing
    /// reads them.
    pub enabled: bool,
    /// Connections one core may hold open, so the deployment's ceiling on the server is
    /// `cores × max`. Never `0` — that is a refusal, not a second spelling of `pool = false`.
    pub max: u32,
    /// How many of [`max`](Self::max) stay open with nothing to do.
    pub idle: u32,
    /// How long a connection may live before it is retired regardless of health.
    pub lifetime: Duration,
    /// How long an acquire waits for a free connection before it throws. Zero is legal and means
    /// exactly what it says: a request arriving at [`max`](Self::max) is refused rather than queued.
    pub acquire: Duration,
}

impl PoolBounds {
    /// § 13's own example, which is this crate's default set — finite with nothing configured, per
    /// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`. The ADR writes these
    /// numbers out, so they are transcribed here rather than chosen.
    pub const DEFAULT: PoolBounds = PoolBounds {
        enabled: true,
        max: 16,
        idle: 2,
        lifetime: Duration::from_secs(30 * 60),
        acquire: Duration::from_secs(5),
    };

    /// § 13's `pool = false`: the default set with the pool switched off, so a caller that reads a
    /// bound anyway reads a finite one rather than a zero it would divide by.
    pub const OFF: PoolBounds = PoolBounds {
        enabled: false,
        ..PoolBounds::DEFAULT
    };
}

impl Default for PoolBounds {
    fn default() -> Self {
        PoolBounds::DEFAULT
    }
}

/// Every `[db.<name>]` block's pool and § 11 threshold resolve — the boot half of [`pool_for`] and
/// [`slow_query_for`], which is where an unwritable setting becomes a refusal naming its own file.
///
/// It runs over the merged tree for [`canonicalize`]'s reason: which `pool` is in force is a
/// question only the merge has answered.
///
/// # Errors
///
/// An unscoped `[db.pool]` table of bounds, which § 13 gives no meaning to and which
/// [`bounds_for`]'s doc argues could not be sized against anything; then the first block whose
/// bounds do not describe a pool or whose `slow_query` is not a duration, as [`pool_for`] and
/// [`slow_query_for`] refuse it.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    if let Some(Pool::Bounds(_)) = config.db.pool {
        return Err(refuse(
            "db.pool",
            "a table of bounds",
            "bounds written with no block in front of them would size a pool whose key is a \
             credential hash, and no one server's `max_connections` is what that is sized against",
            "write the bounds under the block they are for, as `[db.<name>.pool]`; `pool = false` \
             is the one directive § 13 gives an unscoped meaning",
            origins,
        ));
    }
    for (name, db) in &config.db.blocks {
        pool_for(name, db, origins)?;
        slow_query_for(name, db, origins)?;
    }
    Ok(())
}

/// The bounds `name`'s block asks for, over [`PoolBounds::DEFAULT`] — every unwritten key keeps the
/// default, since `rule:config/later-wins-and-every-override-is-recorded`'s override record is per key and a partly-written `[db.x.pool]` is
/// an independent decision per key rather than one.
///
/// `origins` names the file a refusal points at, and an empty map simply leaves the note off.
///
/// # Errors
///
/// `E0601` for a bound that is not a duration at all, and for the ones that parse and still cannot
/// describe a pool: a `max` of `0` (which can never hand out a connection), an `idle` above `max`
/// (which asks for more warm connections than may exist), and a `lifetime` of `0` or `false` — the
/// first retires a connection before it can be reused and the second is a bound `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` does not
/// allow to be missing. Each has one correct spelling and the help names it.
pub fn pool_for(
    name: &str,
    db: &Database,
    origins: &BTreeMap<String, Origin>,
) -> Result<PoolBounds, Diagnostic> {
    let written = match db.pool.as_ref() {
        None | Some(Pool::Switch(true)) => return Ok(PoolBounds::DEFAULT),
        Some(Pool::Switch(false)) => return Ok(PoolBounds::OFF),
        Some(Pool::Bounds(written)) => written,
    };
    let key = |leaf: &str| format!("db.{name}.pool.{leaf}");

    let max = written.max.unwrap_or(PoolBounds::DEFAULT.max);
    if max == 0 {
        return Err(refuse(
            &key("max"),
            "0",
            "a pool that may hold no connection can never hand one out, so every acquire would \
             throw",
            "write `pool = false`, which is § 13's one spelling of turning the pool off",
            origins,
        ));
    }

    let idle = written.idle.unwrap_or(PoolBounds::DEFAULT.idle);
    if idle > max {
        return Err(refuse(
            &key("idle"),
            &idle.to_string(),
            &format!(
                "`idle` is how many of `max` stay open with nothing to do, and this block's `max` \
                 is {max}"
            ),
            "lower `idle` to at most `max`, or raise `max` to the number of connections one core \
             may hold",
            origins,
        ));
    }

    let lifetime = match (
        duration(
            &key("lifetime"),
            written.lifetime.as_ref(),
            origins,
            POOL_HELP,
        )?,
        written.lifetime.as_ref(),
    ) {
        // Zero is only reachable when the block wrote it, so the refusal always has the spelling to
        // quote back — which is the whole reason this reads the two together.
        (Some(zero), Some(setting)) if zero.is_zero() => {
            return Err(refuse(
                &key("lifetime"),
                &crate::value::as_written(setting),
                "a connection retired the instant it is opened is never reused, which is \
                 connect-per-request with a reset on top of it",
                "write the time a connection may live, as `30m`, or `pool = false` if the pool is \
                 not wanted at all",
                origins,
            ));
        }
        (Some(lifetime), _) => lifetime,
        (None, _) => PoolBounds::DEFAULT.lifetime,
    };

    let acquire = duration(
        &key("acquire"),
        written.acquire.as_ref(),
        origins,
        POOL_HELP,
    )?
    .unwrap_or(PoolBounds::DEFAULT.acquire);

    Ok(PoolBounds {
        enabled: true,
        max,
        idle,
        lifetime,
        acquire,
    })
}

/// The bounds in force for a connection — [`pool_for`] with `rule:security/db-pool-reset-is-a-boundary`'s **unscoped**
/// `pool = false` in front of it, which is where that directive is read and the only place it is.
///
/// `block` is the name of the `[db.<name>]` block whose bounds are asked for, and `None` for a
/// connection that matched no block at all: a `Core\Db::open` whose settings object names an
/// endpoint no operator wrote. That case takes [`PoolBounds::DEFAULT`], which is the only finite
/// answer available — there is no table to read — and it is still reached by the unscoped switch,
/// which is the whole reason § 13 gives that switch an unscoped spelling. An audited deployment
/// needs every connection the process opens to map to one request, and a per-block key cannot reach
/// a connection that has no block.
///
/// A name that is not in the tree is the same case as `None` rather than an error: the caller that
/// asks by name has already established the block exists, and a name that vanished between the two
/// is a reload, where the superseded generation's own key retires with it (§ 13).
///
/// # Errors
///
/// Exactly [`pool_for`]'s refusals, for the named block. The unscoped switch cannot fail: it is a
/// boolean, and the table shape written in its place is [`validate`]'s refusal at boot.
pub fn bounds_for(
    config: &Config,
    block: Option<&str>,
    origins: &BTreeMap<String, Origin>,
) -> Result<PoolBounds, Diagnostic> {
    if config.db.pool == Some(Pool::Switch(false)) {
        return Ok(PoolBounds::OFF);
    }
    match block.and_then(|name| config.db.blocks.get(name).map(|db| (name, db))) {
        Some((name, db)) => pool_for(name, db, origins),
        None => Ok(PoolBounds::DEFAULT),
    }
}

/// What a refused pool bound is told to write instead. Held once because the bounds share it, and
/// named because [`slow_query_for`] deliberately does not: `pool = false` is this family's way out
/// and a threshold has its own.
const POOL_HELP: &str = "write a duration, as `30m`, or `pool = false` if the pool is not wanted at \
                         all";

/// `rule:observability/a-slow-query-is-logged-past-a-threshold`'s `slow_query` threshold for `name`'s block,
/// or `Ok(None)` for the block that writes none — which is off, and the ADR's own default rather
/// than a number this crate picks.
///
/// Not part of [`PoolBounds`] and not read by [`pool_for`], because it is not a bound on the pool:
/// it is a property of the statements that run on the connection, and § 13's bounds are resolved
/// once per acquire where this is asked once per statement.
///
/// **The unwritten case costs a lookup and nothing else**, which is why the setting is tested before
/// the key is built: `Core\Db` asks this per statement, so the deployment that never opted in must
/// not pay a `String` for the question, and the one that did pays a parse it asked for.
///
/// # Errors
///
/// A `slow_query` that is not a duration, in [`mod@crate::value`]'s own words. [`validate`] asks at
/// boot, so the per-statement reader downstream is reading a value already proven to parse.
pub fn slow_query_for(
    name: &str,
    db: &Database,
    origins: &BTreeMap<String, Origin>,
) -> Result<Option<Duration>, Diagnostic> {
    let Some(setting) = db.slow_query.as_ref() else {
        return Ok(None);
    };
    duration(
        &format!("db.{name}.slow_query"),
        Some(setting),
        origins,
        "write how long a statement may take before it is logged, as `200ms`, or leave the key out \
         and none is",
    )
}

/// One written duration bound, and `Ok(None)` for one the block left out.
///
/// [`mod@crate::value`] is the parser, so `"30m"`, `"1800s"` and a bare `1800` all read the same and
/// a suffix it does not know is refused in its own words rather than in this module's.
///
/// `help` is the caller's because the settings this reads are not one family: a pool bound is
/// answered by `pool = false` and § 11's `slow_query` by leaving the key out, and a shared line
/// would name the wrong escape for one of them.
fn duration(
    key: &str,
    written: Option<&Setting>,
    origins: &BTreeMap<String, Origin>,
    help: &str,
) -> Result<Option<Duration>, Diagnostic> {
    let Some(setting) = written else {
        return Ok(None);
    };
    let quantity = Quantity::parse(key, Unit::Duration, setting)
        .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
    match quantity {
        Quantity::Nanos(nanos) => Ok(Some(Duration::from_nanos(nanos))),
        // `Unit::Duration` yields nothing else, and the reachable one is `false`. Everywhere else in
        // this tree that reads "no ceiling", which is a meaning a pool bound does not have: `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`
        // is finite with nothing configured, and finite with something configured too.
        _ => Err(refuse(
            key,
            &crate::value::as_written(setting),
            "a duration setting is finite — `false` removes a ceiling, and this is not one",
            help,
            origins,
        )),
    }
}

/// A bound that parses and still cannot describe a pool, under `E0601` — the code a directive with
/// an invalid value gets however it arrived, which is why this needs no code of its own.
///
/// It is built here rather than through [`crate::value::Invalid`] because that type says *which
/// unit* a value failed to be, and every value refused here is already the right unit.
fn refuse(
    key: &str,
    what: &str,
    why: &str,
    help: &str,
    origins: &BTreeMap<String, Origin>,
) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        format!("`{key}` is `{what}`, which is not a value that key can hold"),
    )
    .with_note(format!("{why}{}", origin_note(origins.get(key))))
    .with_help(help.to_string())
}
