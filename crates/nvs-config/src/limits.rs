//! `rule:config/the-config-is-an-immutable-snapshot`: the limits a snapshot resolves once, so a
//! request that sets nothing reads its ceilings with no text parsing and no allocation.
//!
//! Every request reads its ceilings when it starts, and they are the same for every request under
//! one snapshot until one of them calls `Core\Config::set`. So [`Request::limit`] reads this
//! table for every key the request's overlay does not hold, and a request that set the key, or
//! flipped its mode, reads it the way [`Request::get`] does, because its overlay decides.
//!
//! **Resolved by the path `get` takes.** Each reading is [`Request::get`] of a fresh request over
//! the snapshot, parsed by [`Quantity::parse`] in the key's unit, which is what every reader ran
//! per request before. So the table cannot disagree with `get`: it is `get`'s answer, kept.
//!
//! **Two halves, because one default depends on the run.** An unwritten `cpu_time` is a ceiling
//! for server work and nothing for a CLI program ([`crate::defaults::Value::Serving`]), so both are
//! resolved, and a request reads the half its [`Request::serves`] names.
//!
//! **Filled when it is published, and never copied.** A [`Published`](crate::Published) set fills
//! the cell of the host's snapshot when it is made, and of each entry's snapshot when that entry is
//! folded. So a reload pays the parsing before the swap, and no request pays it. A snapshot that
//! is never published, such as one a test builds, fills it on its first read. A clone of a
//! snapshot starts empty, because cloning is how a snapshot is changed before it is published,
//! and a table read off the tree before the change would be stale. The one change made in place,
//! a publish carrying a `Boot` key back (`Current::carry_boot`), runs before the set is made, and
//! empties the cell too.
//!
//! **The door's settings are here too.** The cell holds the decoded `[http] csrf_key`,
//! `[limits] upload_total`, the `cancel_on_disconnect` and `disconnect_grace` pair, and `[log]`'s
//! target with its bounds, level and format. The door and the log writer read them per request,
//! so each is decoded or parsed once per snapshot. The key's type prints nothing of itself,
//! because [`Resolved`] derives `Debug`. `log.level` and `log.format` are `Runtime` keys, so a
//! request whose overlay holds one, or flips the mode, reads it through `get`. The other values
//! are `System` keys, so no overlay holds them.
//!
//! What it spends: per snapshot, two rows of [`LIMITS`] readings, four flags, two sizes, a
//! 32-byte key, the disconnect list and the log target's path, under a kilobyte, built once and
//! shared by every request that clones the snapshot. Nothing in it is keyed by what a request
//! supplies, so nothing crosses requests (`rule:security/no-cross-request-state`).

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use nvs_render::Level;

use crate::app::Disconnect;
use crate::errors::{self, Deprecated};
use crate::http::CSRF_KEY_BYTES;
use crate::log::{Format, Target};
use crate::request::Request;
use crate::snapshot::Snapshot;
use crate::tree::Setting;
use crate::value::{LIMITS, Quantity, Unit};

/// `[debug] store`, which [`Request::debug_store`] reads in force.
pub(crate) const STORE: &str = "debug.store";
/// `[debug] spool_max`, a `System` key, so read off the snapshot alone.
const SPOOL_MAX: &str = "debug.spool_max";

/// One limit as a run reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reading {
    /// Nothing is in force: no written value and no default for this run.
    Unset,
    /// The value in force, in the key's unit.
    Read(Quantity),
    /// A value is in force and does not spell the key's unit. Only a snapshot built by hand can
    /// hold one, because the boot and `Core\Config::set` both refuse it.
    Malformed,
}

impl Reading {
    /// `written`, the text `get` gave for `key`, read in `unit`.
    #[must_use]
    pub fn of(key: &str, unit: Unit, written: Option<String>) -> Self {
        let Some(written) = written else {
            return Self::Unset;
        };
        match Quantity::parse(key, unit, &Setting::Text(written)) {
            Ok(quantity) => Self::Read(quantity),
            Err(_) => Self::Malformed,
        }
    }
}

/// A snapshot's resolved limits, filled on first read. See the module doc.
#[derive(Debug, Default)]
pub struct Cell(OnceLock<Resolved>);

impl Clone for Cell {
    /// An empty cell: the module doc's *Filled on first read, and never copied*.
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// What one snapshot resolved.
#[derive(Debug)]
pub(crate) struct Resolved {
    /// Each [`LIMITS`] row, by its index there, for a run that is not server work.
    cli: [Reading; LIMITS.len()],
    /// The same rows for server work.
    serving: [Reading; LIMITS.len()],
    /// `[errors] deprecated`.
    pub(crate) deprecated: Deprecated,
    /// Whether `[log] access` is `true`.
    pub(crate) access: bool,
    /// Whether `[log] client_ip` is anything but `false`.
    pub(crate) client_ip: bool,
    /// Whether `[trace] response_header` is anything but `false`.
    pub(crate) trace_response: bool,
    /// Whether `[debug] store` is `true`.
    pub(crate) debug_store: bool,
    /// `[debug] spool_max` in bytes, [`u64::MAX`] for `false`.
    pub(crate) spool_max: u64,
    /// `[http] csrf_key`, decoded, or `None` where the tree names none.
    pub(crate) csrf_key: Option<CsrfKey>,
    /// The global `[limits] upload_total` in bytes ([`crate::app::upload_total`]).
    pub(crate) upload_total: u64,
    /// `[limits] cancel_on_disconnect` and `disconnect_grace`, or `None` for a value the boot
    /// refuses ([`crate::app::disconnect_for`]).
    pub(crate) disconnect: Option<Disconnect>,
    /// `[log] target` with its two bounds, or `None` where it names no target.
    pub(crate) log_sink: Option<LogSink>,
    /// `[log] level`, or `None` where it names none.
    pub(crate) log_level: Option<Level>,
    /// `[log] format`, or `None` where it names none.
    pub(crate) log_format: Option<Format>,
}

/// The decoded `[http] csrf_key`. Its `Debug` prints no byte of the key, because [`Resolved`]
/// derives `Debug`.
pub(crate) struct CsrfKey([u8; CSRF_KEY_BYTES]);

impl CsrfKey {
    /// The key's bytes.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for CsrfKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CsrfKey(..)")
    }
}

/// `[log] target` as a sink reads it, with the path owned so the cell can keep it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogSink {
    /// `stderr`.
    Stderr,
    /// `file:<path>`, with `[log] max_size` and `[log] keep`.
    File {
        /// The path as written.
        path: PathBuf,
        /// The size at which the file rotates.
        max_size: u64,
        /// How many rotations are kept beside it.
        keep: usize,
    },
    /// `syslog`.
    Syslog,
}

impl Resolved {
    /// The table `base` resolves to, read through two fresh requests over it.
    fn of(base: &Arc<Snapshot>) -> Self {
        let cli = Request::new(Arc::clone(base));
        let mut server = cli.clone();
        server.serve();
        let row = |request: &Request| {
            std::array::from_fn(|index| {
                let limit = &LIMITS[index];
                Reading::of(limit.key, limit.unit, request.get(limit.key))
            })
        };
        Self {
            cli: row(&cli),
            serving: row(&server),
            deprecated: cli
                .get(errors::KEY)
                .and_then(|written| Deprecated::of(&written))
                .unwrap_or_default(),
            access: cli
                .get("log.access")
                .is_some_and(|written| written == "true"),
            client_ip: cli
                .get("log.client_ip")
                .is_none_or(|written| written != "false"),
            trace_response: cli
                .get("trace.response_header")
                .is_none_or(|written| written != "false"),
            debug_store: cli.get(STORE).is_some_and(|written| written == "true"),
            spool_max: match Reading::of(SPOOL_MAX, Unit::Bytes, cli.get(SPOOL_MAX)) {
                Reading::Read(Quantity::Bytes(bytes)) => bytes,
                Reading::Read(Quantity::Unbounded) => u64::MAX,
                _ => crate::defaults::DEBUG_SPOOL_MAX,
            },
            csrf_key: crate::http::csrf_key(&base.config)
                .and_then(|key| <[u8; CSRF_KEY_BYTES]>::try_from(key).ok())
                .map(CsrfKey),
            upload_total: crate::app::upload_total(&base.config),
            disconnect: crate::app::disconnect_for(&base.config, &base.origins).ok(),
            log_sink: log_sink(&cli),
            log_level: cli.get("log.level").and_then(|written| Level::of(&written)),
            log_format: cli
                .get("log.format")
                .and_then(|written| Format::of(&written)),
        }
    }

    /// The [`LIMITS`] row at `index`, for a run that is server work where `serving` says so.
    pub(crate) fn limit(&self, index: usize, serving: bool) -> Reading {
        if serving {
            self.serving[index]
        } else {
            self.cli[index]
        }
    }
}

/// `[log] target` and its bounds as `request` reads them, or `None` where the target is not one
/// [`Target::of`] spells.
fn log_sink(request: &Request) -> Option<LogSink> {
    let written = request.get("log.target")?;
    Some(match Target::of(&written)? {
        Target::Stderr => LogSink::Stderr,
        Target::Syslog => LogSink::Syslog,
        Target::File(path) => {
            let (max_size, keep) = crate::log::bounds(
                request.get("log.max_size").as_deref(),
                request.get("log.keep").as_deref(),
            );
            LogSink::File {
                path: PathBuf::from(path),
                max_size,
                keep,
            }
        }
    })
}

/// `base`'s resolved limits, resolving them on the first call.
pub(crate) fn resolved(base: &Arc<Snapshot>) -> &Resolved {
    base.limits.0.get_or_init(|| Resolved::of(base))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::snapshot::Snapshot;

    /// The snapshot `written` configures.
    fn snapshot(written: &str) -> Arc<Snapshot> {
        let table: toml::Table = written.parse().expect("the fixture is valid TOML");
        let config = toml::Value::Table(table.clone())
            .try_into()
            .expect("the fixture deserializes into the tree it is written for");
        Arc::new(Snapshot {
            config,
            table,
            ..Snapshot::default()
        })
    }

    /// The door's settings are read off one cell: two reads under one snapshot return the same
    /// addresses, and a clone of the snapshot resolves its own.
    #[test]
    fn the_door_settings_are_resolved_once_per_snapshot() {
        let key = "A".repeat(43);
        let base = snapshot(&format!("[http]\ncsrf_key = \"{key}\"\n"));
        let first = base.csrf_key().expect("the fixture names a key");
        let second = base.csrf_key().expect("the fixture names a key");
        assert_eq!(first.len(), crate::http::CSRF_KEY_BYTES);
        assert!(std::ptr::eq(first, second), "one decode per snapshot");
        let disconnect = base.disconnect().expect("the defaults are readable");
        assert!(std::ptr::eq(
            disconnect,
            base.disconnect().expect("the defaults are readable")
        ));
        assert!(std::ptr::eq(super::resolved(&base), super::resolved(&base)));
        assert_eq!(base.upload_total(), crate::app::upload_total(&base.config));

        let clone = Arc::new(Snapshot::clone(&base));
        let cloned = clone.csrf_key().expect("a clone names the same key");
        assert_eq!(cloned, first);
        assert!(!std::ptr::eq(cloned, first), "a clone resolves its own");
        assert!(!std::ptr::eq(
            clone.disconnect().expect("the defaults are readable"),
            disconnect
        ));
        assert!(format!("{:?}", super::resolved(&base)).contains("CsrfKey(..)"));
    }
}
