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
//! **Filled on first read, and never copied.** The cell is empty in every snapshot, however it was
//! built, and the first reader fills it. A clone of a snapshot starts empty, because cloning is how
//! a snapshot is changed before it is published, and a table read off the tree before the change
//! would be stale. The one change made in place, a publish carrying a `Boot` key back
//! (`Current::carry_boot`), empties the cell too.
//!
//! What it spends: per snapshot, two rows of [`LIMITS`] readings, four flags and a size, under a
//! kilobyte, built once and shared by every request that clones the snapshot. Nothing in it is
//! keyed by what a request supplies, so nothing crosses requests
//! (`rule:security/no-cross-request-state`).

use std::sync::{Arc, OnceLock};

use crate::errors::{self, Deprecated};
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

/// `base`'s resolved limits, resolving them on the first call.
pub(crate) fn resolved(base: &Arc<Snapshot>) -> &Resolved {
    base.limits.0.get_or_init(|| Resolved::of(base))
}
