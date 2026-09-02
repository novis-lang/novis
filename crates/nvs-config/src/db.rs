//! What a `[db.<name>]` block still owes once it has deserialized: the path that names a file,
//! resolved and trust-checked, and ADR 0067 § 13's pool bounds, read into numbers. Both at boot.
//!
//! The path is one field — [`Database::tls_ca_file`](crate::tree::Database::tls_ca_file), the PEM bundle
//! ADR 0067 § 3's TLS leg verifies a server's certificate against. It is here rather than in
//! [`mod@crate::secret`] because it is not a secret: nothing about a CA bundle is confidential, and
//! § 7's whole shape — one value, materialized into a sibling directive, kept out of the merged
//! table — is the wrong one for a file a connection re-reads by path. What it *shares* with § 7 is
//! the trust boundary, which is why both run at resolve time and neither at use time.
//!
//! **A CA bundle is a trust-boundary file.** Whoever can rewrite it decides which server this
//! deployment's queries and credentials go to, which is the same authority ADR 0103 § 6 refuses to
//! leave on the configuration files themselves. So it goes through
//! [`Files::trust`] exactly as a `password_file` does, and an
//! unreadable or group-writable bundle is `E0605`/`E0607` at boot. It is deliberately *not* read
//! here: the anchors are parsed by `nvs_host::tls`, which owns the one client configuration every
//! session in this process shares, and a second parse in this crate would be a second answer to
//! "whose certificates do you believe".
//!
//! **The path is rewritten in place**, absolute, exactly as [`mod@crate::app`] canonicalizes a
//! block's key and for its second reason: § 9's `nvs config dump` then prints the file the
//! handshake will actually open rather than a fragment whose meaning depends on which file in the
//! tree wrote it. A connection opened from a working directory that is not the configuration's —
//! every request, since ADR 0103 § 5 resolves against the *file* — would otherwise read a different
//! bundle or none.
//!
//! **The pool's bounds are read here and not where the pool is built**, for the reason every other
//! `validate` in this crate runs at boot: an operator who wrote `lifetime = "30 minutes"` learns it
//! from the boot that refuses, naming the file and the line, rather than from the first request
//! whose acquire fails at three in the morning. [`validate`] is that pass and [`pool_for`] is the
//! reader under it, which the pool calls again when it builds itself — four integers, once per pool,
//! against carrying a derived value through [`crate::Snapshot`] that a reload would have to keep in
//! step with the tree it came from.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, Database, Pool, Setting};
use crate::value::{Quantity, Unit};

/// Makes every `[db.<name>] tls_ca_file` absolute and proves it is inside the trust boundary.
///
/// Runs over the merged tree for [`mod@crate::secret`]'s reason: which bundle is in force is a
/// question only the merge has answered, and trust-checking a path a later file replaced would
/// refuse a boot over a file the deployment does not use.
///
/// # Errors
///
/// `E0607` for a bundle outside ADR 0103 § 6's trust boundary and `E0605` for one that cannot be
/// read at all — [`crate::resolve::untrusted`]'s split, so an operator told "cannot read" goes
/// looking for a typo and one told the other goes looking at a mode.
pub fn canonicalize(
    config: &mut Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    for (name, db) in &mut config.db {
        let Some(written) = db.tls_ca_file.as_deref() else {
            continue;
        };
        // § 5: relative to the file it was written in. A key with no origin cannot have been
        // written anywhere, so there is nothing but the path itself to resolve against.
        let base = origins
            .get(&format!("db.{name}.tls_ca_file"))
            .and_then(|origin| origin.path.parent())
            .unwrap_or(Path::new("."));
        let path = crate::resolve::absolute(base, Path::new(written));
        let trusted = files.trust(&path).map_err(|why| {
            crate::resolve::untrusted(
                &path,
                &why,
                "a `[db]` block's `tls_ca_file` names it, and whoever can write it chooses which \
                 server the connection may be talking to",
            )
        })?;
        db.tls_ca_file = Some(trusted.to_string_lossy().into_owned());
    }
    Ok(())
}

/// ADR 0067 § 13's pool, resolved: every bound a number, and nothing left to decide at acquire time.
///
/// Held by value and `Copy`, because the acquire path reads it and a pool is per core: five words
/// beside a connection costs less than the pointer chase that would save four of them.
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
    /// [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md). The ADR writes these
    /// four numbers out, so they are transcribed here rather than chosen.
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

/// Every `[db.<name>]` block's pool resolves — the boot half of [`pool_for`], which is where an
/// unwritable bound becomes a refusal naming its own file.
///
/// It runs over the merged tree for [`canonicalize`]'s reason: which `pool` is in force is a
/// question only the merge has answered.
///
/// # Errors
///
/// The first block whose bounds do not describe a pool, as [`pool_for`] refuses it.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    for (name, db) in &config.db {
        pool_for(name, db, origins)?;
    }
    Ok(())
}

/// The bounds `name`'s block asks for, over [`PoolBounds::DEFAULT`] — every unwritten key keeps the
/// default, since ADR 0103 § 3's override record is per key and a partly-written `[db.x.pool]` is
/// four independent decisions rather than one.
///
/// `origins` names the file a refusal points at, and an empty map simply leaves the note off.
///
/// # Errors
///
/// `E0601` for a bound that is not a duration at all, and for the three that parse and still cannot
/// describe a pool: a `max` of `0` (which can never hand out a connection), an `idle` above `max`
/// (which asks for more warm connections than may exist), and a `lifetime` of `0` or `false` — the
/// first retires a connection before it can be reused and the second is a bound ADR 0074 does not
/// allow to be missing. Each of the three has one correct spelling and the help names it.
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
        duration(&key("lifetime"), written.lifetime.as_ref(), origins)?,
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

    let acquire = duration(&key("acquire"), written.acquire.as_ref(), origins)?
        .unwrap_or(PoolBounds::DEFAULT.acquire);

    Ok(PoolBounds {
        enabled: true,
        max,
        idle,
        lifetime,
        acquire,
    })
}

/// One written duration bound, and `Ok(None)` for one the block left out.
///
/// [`mod@crate::value`] is the parser, so `"30m"`, `"1800s"` and a bare `1800` all read the same and
/// a suffix it does not know is refused in its own words rather than in this module's.
fn duration(
    key: &str,
    written: Option<&Setting>,
    origins: &BTreeMap<String, Origin>,
) -> Result<Option<Duration>, Diagnostic> {
    let Some(setting) = written else {
        return Ok(None);
    };
    let quantity = Quantity::parse(key, Unit::Duration, setting)
        .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
    match quantity {
        Quantity::Nanos(nanos) => Ok(Some(Duration::from_nanos(nanos))),
        // `Unit::Duration` yields nothing else, and the reachable one is `false`. Everywhere else in
        // this tree that reads "no ceiling", which is a meaning a pool bound does not have: ADR 0074
        // is finite with nothing configured, and finite with something configured too.
        _ => Err(refuse(
            key,
            &crate::value::as_written(setting),
            "a pool bound is finite — `false` removes a ceiling, and this is not one",
            "write a duration, as `30m`, or `pool = false` if the pool is not wanted at all",
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
        format!("`{key}` is `{what}`, which is not a bound a pool can hold"),
    )
    .with_note(format!("{why}{}", origin_note(origins.get(key))))
    .with_help(help.to_string())
}
