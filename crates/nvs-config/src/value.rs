//! [ADR 0064 § 5]'s one parser: what a [`Setting`] means as a quantity, and how two of them compare.
//!
//! § 5 gives `Core\Config::set` a `string` value in both directions and then says the registry
//! parses it "with the same parser the boot path uses" — the shape
//! `rule:expressions/intrinsic-literals` already establishes, where a
//! prepared and a runtime path cannot diverge because there is one implementation. So a `512M` a
//! ceiling refuses at boot is refused identically by a `set` at request time, and neither path can
//! grow its own idea of what `M` means.
//!
//! **The parse is directed by the unit, never by the spelling.** `m` is mega under a size and
//! minutes under a duration, and a bare `600` is bytes under one and seconds under the other;
//! nothing in the text decides that, so [`unit_of`] decides it once for both paths. It is keyed on
//! the key's **last segment**, because one limit is written in several places — `[limits]`,
//! `[limits.hard]`, `[app.limits]`, `[app.limits.hard]` and the bare `memory` a
//! `Core\Config::set` names it with — and a ceiling that parsed differently from the value it
//! bounds would be comparing two different quantities. The block is still checked, so a future
//! block spelling `memory` for something that is not a heap does not inherit a unit by accident.
//!
//! **`false` is [`Quantity::Unbounded`] in both spellings** — a TOML boolean from a file, the text
//! `"false"` through `set` — because `rule:config/three-changeability-classes` gives "no ceiling" that spelling and § 5 crosses
//! every value as a string. It is above every magnitude, which is exactly what removing a ceiling
//! means, and it is a value as well as a ceiling: a `[limits] memory = false` under a `512M`
//! `[limits.hard]` is a default that exceeds its own ceiling and is refused as one.
//!
//! **An incomparable pair exceeds.** [`Quantity::within`] answers `false` for two quantities with
//! no order between them rather than letting the pair through, because a ceiling check that cannot
//! compare must refuse — priority 1 in `AGENTS.md`, and the only way to reach that case is a caller
//! that parsed the two sides under different units.
//!
//! What this module does **not** do is decide who may set a directive: that is
//! [`Class`](crate::Class), asked before the value is looked at, and a value within its ceiling set
//! by a request that may not set it at all is still refused.
//!
//! Cost: no allocation on a value that parses, one `String` per refusal. It runs at boot, on each
//! reload, and once per `Core\Config::set` — never on a read of a limit already in the snapshot.
//!
//! [ADR 0064 § 5]: ../../../docs/decisions/0064.md

use std::cmp::Ordering;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::Setting;

/// What a key's value is measured in — see the module doc on why the parse is directed by this and
/// not by the spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// A size: `512M`, `1G`, or a bare number of bytes.
    Bytes,
    /// A duration: `600s`, `250ms`, or a bare number of seconds.
    Duration,
    /// A count: `max_tasks = 64`.
    Count,
    /// A fraction of one: `[trace] sample = 0.01`.
    Ratio,
}

impl Unit {
    /// What a value in this unit is called, as a noun phrase completing "which is not …".
    #[must_use]
    pub fn noun(self) -> &'static str {
        match self {
            Unit::Bytes => "a size",
            Unit::Duration => "a duration",
            Unit::Count => "a count",
            Unit::Ratio => "a ratio",
        }
    }

    /// The whole of what this unit accepts, for the note under a refusal.
    #[must_use]
    pub fn accepts(self) -> &'static str {
        match self {
            Unit::Bytes => {
                "a size is a whole number of bytes, or one with a `K`, `M`, `G` or `T` suffix in \
                 binary multiples (`KB`, `KiB` and lower case all read the same), or `false` for no \
                 ceiling"
            }
            Unit::Duration => {
                "a duration is a whole number of seconds, or one with an `ns`, `us`, `ms`, `s`, \
                 `m`, `h` or `d` suffix, or `false` for no ceiling"
            }
            Unit::Count => "a count is a whole number with no suffix, or `false` for no ceiling",
            Unit::Ratio => "a ratio is a number between 0 and 1",
        }
    }

    /// A value this unit accepts, for the help under a refusal.
    #[must_use]
    pub fn example(self) -> &'static str {
        match self {
            Unit::Bytes => "512M",
            Unit::Duration => "600s",
            Unit::Count => "64",
            Unit::Ratio => "0.01",
        }
    }
}

/// The unit `key` is written in, and `None` for a key that is not a quantity at all.
///
/// The limit keys are the whole table; adding a key is one row, and a key outside a
/// limits block gets `None` however it is spelled.
#[must_use]
pub fn unit_of(key: &str) -> Option<Unit> {
    let (block, leaf) = match key.rfind('.') {
        Some(dot) => (&key[..dot], &key[dot + 1..]),
        None => ("", key),
    };
    if !in_a_limits_block(block) {
        return None;
    }
    match leaf {
        // `rule:errors/on-limit`'s reserved slice is a quantity of the same heap `memory`
        // bounds, so it is read in the same units and by the same parser.
        // `rule:core-classes/decompression-bound`'s absolute half is a quantity of the same heap
        // `memory` bounds, so it is read in the same units and by the same parser.
        "memory" | "max_output" | "fatal_reserve_memory" | "max_decompressed" => Some(Unit::Bytes),
        // The other half of that slice is a quantity of the same CPU time `cpu_time` bounds, and
        // is read here for the same reason: a limit missing from this table has no block, so a
        // bare name never reaches `[limits]` and the reader silently answers its default instead.
        // The grace after a disconnect is a stretch of the same wall-clock time `wall_time`
        // bounds, read here so a bare number of seconds means what it means there.
        "cpu_time" | "wall_time" | "fatal_reserve_time" | "disconnect_grace" => {
            Some(Unit::Duration)
        }
        // The ratio half is a multiplier rather than a fraction — output per octet of input — so
        // it is a `Count` and not `Unit::Ratio`, whose values run between zero and one.
        // The backtracking tier's step budget is a count of steps for the reason `max_tasks` is a
        // count of tasks: a number of things, with no unit anyone would suffix it with.
        "max_tasks" | "max_script_depth" | "max_decompression_ratio" | "max_regex_steps" => {
            Some(Unit::Count)
        }
        // `rule:observability/memory-high-water-writes-a-warn`'s threshold is a share of the
        // memory ceiling, which is what a `Ratio` is.
        "memory_high_water" => Some(Unit::Ratio),
        _ => None,
    }
}

/// Whether a limit written in `block` is one of `rule:config/three-changeability-classes`'s — `""` for the bare name
/// `Core\Config::set` uses, and any block ending in `limits` or `limits.hard` for the file, which
/// covers `app.0.limits` and a `[[schedule]]`'s alike.
///
/// The tail is matched on a dot boundary, so a block called `oldlimits` is not a limits block.
fn in_a_limits_block(block: &str) -> bool {
    ["limits", "limits.hard"].iter().any(|tail| {
        block == *tail
            || (block.len() > tail.len()
                && block.ends_with(tail)
                && block.as_bytes()[block.len() - tail.len() - 1] == b'.')
    }) || block.is_empty()
}

/// One value as the quantity it spells.
///
/// Sizes and durations are held in their smallest unit — bytes and nanoseconds — so that two
/// values written with different suffixes compare directly. A duration is `u64` nanoseconds, which
/// runs out at 584 years and so cannot be reached by any ceiling an operator would write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Quantity {
    /// `false`: no ceiling at all (`rule:config/three-changeability-classes`). Above every magnitude, whatever its unit.
    Unbounded,
    /// A size, in bytes.
    Bytes(u64),
    /// A duration, in nanoseconds.
    Nanos(u64),
    /// A count.
    Count(u64),
    /// A ratio, between zero and one inclusive.
    Ratio(f64),
}

impl Quantity {
    /// The quantity `value` spells, read in `unit`. `key` names the directive for the refusal only.
    ///
    /// # Errors
    ///
    /// [`Invalid`] when the value is not a quantity in that unit: a suffix the unit does not know,
    /// a negative or fractional number where a whole one is required, a magnitude that overflows,
    /// or a shape — a list, a `true` — that is not a measurement at all.
    pub fn parse(key: &str, unit: Unit, value: &Setting) -> Result<Self, Invalid> {
        parse(unit, value).map_err(|reason| Invalid {
            key: key.to_string(),
            written: as_written(value),
            unit,
            reason,
        })
    }

    /// Whether this quantity stays within `ceiling`, which is the ceiling check itself: `true` for
    /// a value at or below it, and for anything at all under an [`Unbounded`](Quantity::Unbounded)
    /// one.
    ///
    /// Two quantities with no order between them answer `false` — see the module doc.
    #[must_use]
    pub fn within(self, ceiling: Self) -> bool {
        matches!(
            self.partial_cmp(&ceiling),
            Some(Ordering::Less | Ordering::Equal)
        )
    }
}

/// [`Unbounded`](Quantity::Unbounded) is the maximum and two magnitudes compare numerically;
/// magnitudes in different units have no order, which is the case [`Quantity::within`] refuses.
impl PartialOrd for Quantity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Quantity::Unbounded, Quantity::Unbounded) => Some(Ordering::Equal),
            (Quantity::Unbounded, _) => Some(Ordering::Greater),
            (_, Quantity::Unbounded) => Some(Ordering::Less),
            (Quantity::Bytes(ours), Quantity::Bytes(theirs))
            | (Quantity::Nanos(ours), Quantity::Nanos(theirs))
            | (Quantity::Count(ours), Quantity::Count(theirs)) => Some(ours.cmp(theirs)),
            (Quantity::Ratio(ours), Quantity::Ratio(theirs)) => ours.partial_cmp(theirs),
            _ => None,
        }
    }
}

/// Whether `value` stays within `ceiling` for `key` — the ceiling check `rule:config/three-changeability-classes` states, in the one
/// place both the boot path and `Core\Config::set` reach it, so the two cannot disagree about which
/// values are allowed.
///
/// `Ok(None)` for a key that is not a quantity: it has no ceiling to compare against and its caller
/// has some other rule for it (`[mode] ceiling` orders an enumeration, a capability is a grant).
///
/// # Errors
///
/// [`Invalid`] for whichever side does not parse, the value first: a ceiling nobody can satisfy and
/// a value nobody can write are both refusals, and reporting the value's own line first is what the
/// operator can act on.
pub fn within_ceiling(
    key: &str,
    value: &Setting,
    ceiling: &Setting,
) -> Result<Option<bool>, Invalid> {
    let Some(unit) = unit_of(key) else {
        return Ok(None);
    };
    let value = Quantity::parse(key, unit, value)?;
    let ceiling = Quantity::parse(key, unit, ceiling)?;
    Ok(Some(value.within(ceiling)))
}

/// A value that does not spell the quantity its key takes.
///
/// It carries what was written rather than a rendered message, so that a caller holding the
/// [`Origin`] the merge recorded can name the file in the same breath — which is the difference
/// between this refusal and `serde`'s, and the reason [`crate::tree`] leaves value checking to this
/// module in the first place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invalid {
    /// The directive key, as the caller named it.
    pub key: String,
    /// The value as it was written, TOML-side quoting removed.
    pub written: String,
    /// The unit the key takes.
    pub unit: Unit,
    /// Why this is not one, as a clause completing "…, and ".
    pub reason: &'static str,
}

impl Invalid {
    /// `E0601`, which is the code for a directive with an invalid value however it arrived.
    ///
    /// `written_in` is the origin the merge recorded for this key, when the caller has one; a
    /// `Core\Config::set` has none, and the note then simply stops after the reason.
    #[must_use]
    pub fn diagnostic(&self, written_in: Option<&Origin>) -> Diagnostic {
        Diagnostic::error(
            code::E_BAD_DIRECTIVE,
            format!(
                "`{}` is `{}`, which is not {}",
                self.key,
                self.written,
                self.unit.noun()
            ),
        )
        .with_note(format!(
            "{}, and {}{}",
            self.unit.accepts(),
            self.reason,
            origin_note(written_in)
        ))
        .with_help(format!(
            "write it as `{}`, or as `false` where the ceiling is meant to be removed",
            self.unit.example()
        ))
    }
}

/// The parse itself, over the shapes a [`Setting`] can be.
fn parse(unit: Unit, value: &Setting) -> Result<Quantity, &'static str> {
    match value {
        Setting::Bool(false) => Ok(Quantity::Unbounded),
        Setting::Bool(true) => Err("`true` grants where a limit measures"),
        Setting::Integer(number) => {
            let magnitude =
                u64::try_from(*number).map_err(|_| "a measurement is never negative")?;
            scale(unit, magnitude, "")
        }
        Setting::Float(fraction) => match unit {
            Unit::Ratio => ratio(*fraction),
            _ => Err("this one is fractional, so write it in a smaller unit instead"),
        },
        Setting::Text(text) => parse_text(unit, text),
        Setting::List(_) => Err("a list of values is not one measurement"),
    }
}

/// The string spelling, which is what a file writes for anything with a suffix and what every
/// `Core\Config::set` writes for everything (`rule:config/ini-set-is-core-config-set`).
fn parse_text(unit: Unit, text: &str) -> Result<Quantity, &'static str> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("false") {
        return Ok(Quantity::Unbounded);
    }
    if text.eq_ignore_ascii_case("true") {
        return Err("`true` grants where a limit measures");
    }
    if unit == Unit::Ratio {
        return text
            .parse::<f64>()
            .map_err(|_| "this one is not a number")
            .and_then(ratio);
    }
    let digits = text
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(text.len());
    let (magnitude, suffix) = text.split_at(digits);
    if magnitude.is_empty() {
        return Err("this one does not begin with a whole number");
    }
    let magnitude: u64 = magnitude
        .parse()
        .map_err(|_| "this one is larger than the runtime can hold")?;
    scale(unit, magnitude, suffix.trim())
}

/// `magnitude` in `suffix`'s multiple of the unit's smallest one — bytes, or nanoseconds.
fn scale(unit: Unit, magnitude: u64, suffix: &str) -> Result<Quantity, &'static str> {
    let suffix = suffix.to_ascii_lowercase();
    match unit {
        Unit::Bytes => bytes_per(&suffix)
            .ok_or("this one's suffix is not a size")?
            .checked_mul(magnitude)
            .map(Quantity::Bytes)
            .ok_or("this one is larger than the runtime can hold"),
        Unit::Duration => nanos_per(&suffix)
            .ok_or("this one's suffix is not a duration")?
            .checked_mul(magnitude)
            .map(Quantity::Nanos)
            .ok_or("this one is longer than the runtime can hold"),
        Unit::Count => match suffix.is_empty() {
            true => Ok(Quantity::Count(magnitude)),
            false => Err("a count takes no suffix"),
        },
        // Only ever reached for the whole numbers a file can write, since [`parse_text`] answers a
        // ratio before it looks for a suffix at all.
        Unit::Ratio => match magnitude {
            0 => Ok(Quantity::Ratio(0.0)),
            1 => Ok(Quantity::Ratio(1.0)),
            _ => Err("this one is outside 0 to 1"),
        },
    }
}

/// The binary multiple each size suffix names; an empty suffix is bytes.
fn bytes_per(suffix: &str) -> Option<u64> {
    Some(match suffix {
        "" | "b" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        "t" | "tb" | "tib" => 1024 * 1024 * 1024 * 1024,
        _ => return None,
    })
}

/// The nanoseconds each duration suffix names; an empty suffix is seconds, and `m` is minutes here
/// where it is mega in [`bytes_per`] — which is the whole reason the parse is unit-directed.
fn nanos_per(suffix: &str) -> Option<u64> {
    Some(match suffix {
        "ns" => 1,
        "us" | "µs" => 1_000,
        "ms" => 1_000_000,
        "" | "s" | "sec" => 1_000_000_000,
        "m" | "min" => 60 * 1_000_000_000,
        "h" => 3_600 * 1_000_000_000,
        "d" => 86_400 * 1_000_000_000,
        _ => return None,
    })
}

/// A ratio, refusing the values that are numbers but not fractions of one.
fn ratio(fraction: f64) -> Result<Quantity, &'static str> {
    match fraction.is_finite() && (0.0..=1.0).contains(&fraction) {
        true => Ok(Quantity::Ratio(fraction)),
        false => Err("this one is outside 0 to 1"),
    }
}

/// The value as the operator wrote it: what a refusal quotes back — this module's, and
/// [`crate::app`]'s ceiling refusal, which names both sides the same way — and what a sub-cap is
/// carried as.
///
/// Public for the second of those. A `[[schedule]]` entry's `limits` crosses to a run as the text
/// beside the key (`nvs_runtime::host::Narrowing`), so the child compares a `512M` an entry asked
/// for against a `512M` a file wrote through one parser rather than two — the reason that narrowing
/// carries text at all, and the reason the ticker renders one here instead of growing a second
/// renderer of its own.
pub fn as_written(value: &Setting) -> String {
    match value {
        Setting::Bool(flag) => flag.to_string(),
        Setting::Integer(number) => number.to_string(),
        Setting::Float(fraction) => fraction.to_string(),
        Setting::Text(text) => text.clone(),
        Setting::List(items) => format!("[{}]", items.join(", ")),
    }
}
