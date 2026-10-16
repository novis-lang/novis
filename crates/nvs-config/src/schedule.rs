//! `rule:config/scheduled-work-is-a-config-block`'s `[[schedule]]` entries, checked at boot: everything an entry must answer before
//! the scheduler can arm it.
//!
//! **Every refusal here is a boot refusal, and that is the whole point of the module.** A schedule
//! fails silently by construction — an entry that never fires is indistinguishable from one whose
//! interval has not come round, and the operator learns about it from the work that did not happen.
//! So the questions are asked once, over the merged tree, and a tree that cannot answer them does
//! not serve: § 1 for `name` and `script`, § 2 for `cron`, § 3 for `scope`, § 6 for `overlap` —
//! which has a default and is checked anyway, because a word that is not one of its three is a mode
//! the operator asked for and will not get.
//!
//! **Over the merged tree, beside [`app::canonicalize`](crate::app::canonicalize), and for the same
//! reason:** `[[schedule]]` is an array of tables, so entries accumulate across the files
//! (`rule:config/a-value-array-replaces-and-a-table-appends`) and the roster only exists once the merge is done. A per-file check would refuse
//! a base file an include was about to complete.
//!
//! **The roots check is [`Capabilities::allows`](crate::tree::Capabilities::allows) and not a
//! second comparison.** § 1 puts a scheduled script under the same `script.spawn` roots a `spawn
//! script` target lives under, and the reason it says so — the set of files a deployment can execute
//! is one list — is exactly the reason this module must not grow its own canonicalize-then-prefix.
//! The grant side is canonicalized here first, on a clone, because at resolve time the tree still
//! holds the roots as the operator spelled them and the snapshot that would canonicalize them does
//! not exist yet.
//!
//! **The next fire is this module's question too, and that is why [`Cron`] is `pub`.** § 2 has the
//! expression read at boot with the offending line named, so the parse is already here; a scheduler
//! that read the same string a second time would be a second dialect, and the two disagreeing
//! produces exactly the failure this module exists to prevent — an entry that booted and fires at
//! the wrong minute, which nothing observes. So the one parse answers [`Cron::next_after`] as well,
//! and § 6's two DST rules are decided in the single place a civil minute becomes an instant.
//!
//! Cost: one clone of `[capabilities]` and one canonicalization per path-scoped grant at boot and at
//! reload, plus one `realpath` and one IANA zone lookup per entry. A next-fire computation walks
//! fields rather than minutes — at most a month, a day, an hour and a minute of stepping per
//! answer — and the scheduler asks for one per fire. Nothing here runs on a request path.
//!

use std::collections::BTreeMap;
use std::path::Path;

use jiff::civil::{Date, DateTime};
use jiff::tz::{AmbiguousOffset, TimeZone};
use jiff::{Span, Zoned};
use nvs_diagnostics::{Diagnostic, code};

use crate::capability::{Cap, Scope};
use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, Schedule};

/// The five named shorthands § 2 accepts beside the five-field form, and the expression each one is
/// short for.
///
/// Expanding rather than special-casing is what keeps the dialect one reader wide: `@daily` and
/// `0 0 * * *` are the same schedule, and nothing downstream of this table can tell them apart.
const SHORTHANDS: [(&str, &str); 5] = [
    ("@hourly", "0 * * * *"),
    ("@daily", "0 0 * * *"),
    ("@weekly", "0 0 * * 0"),
    ("@monthly", "0 0 1 * *"),
    ("@yearly", "0 0 1 1 *"),
];

/// How far ahead [`Cron::next_after`] will look before answering [`None`].
///
/// Nine years rather than one: `0 0 29 2 *` is a schedule, and the longest gap between two 29
/// Februaries is the eight years a century that is not a leap year opens — 2096 to 2104. A search
/// that gave up sooner would report "never fires" about an entry that fires.
const HORIZON_YEARS: i16 = 9;

/// The five fields, in order: what each is called, and the closed range it accepts.
///
/// `day-of-week` is `0`–`6` with Sunday at `0`, which is POSIX's own range. `7` for Sunday is a
/// Vixie extension and is refused by name rather than accepted quietly, because a dialect that
/// takes both spellings has to answer what `0-7` means.
const FIELDS: [(&str, u32, u32); 5] = [
    ("minute", 0, 59),
    ("hour", 0, 23),
    ("day-of-month", 1, 31),
    ("month", 1, 12),
    ("day-of-week", 0, 6),
];

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// Makes every `[[schedule]] script` absolute against the file that wrote it —
/// `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`, as
/// [`crate::db::canonicalize`] does for a `[db]` path.
///
/// By index, because an appended entry's script resolves against the file that wrote that entry.
/// The joined path replaces the written one in the typed tree and in the table, so every fire runs
/// the file [`validate`] checked against the `script.spawn` roots, whatever the working directory.
pub fn anchor(
    config: &mut Config,
    table: &mut toml::value::Table,
    origins: &BTreeMap<String, Origin>,
) {
    for (index, entry) in config.schedule.iter_mut().enumerate() {
        let Some(written) = entry
            .script
            .as_deref()
            .filter(|written| !written.is_empty() && !Path::new(written).has_root())
        else {
            continue;
        };
        let base = crate::db::written_in(origins, &format!("schedule.{index}.script"));
        let script = crate::resolve::absolute(base, Path::new(written))
            .to_string_lossy()
            .into_owned();
        if let Some(block) = table
            .get_mut("schedule")
            .and_then(toml::Value::as_array_mut)
            .and_then(|entries| entries.get_mut(index))
            .and_then(toml::Value::as_table_mut)
        {
            block.insert("script".to_owned(), toml::Value::String(script.clone()));
        }
        entry.script = Some(script);
    }
}

/// § 1–§ 3's boot questions, asked of every `[[schedule]]` entry in the merged tree.
///
/// # Errors
///
/// One [`Diagnostic`], `E0611`, for the first entry that cannot answer: no `name` or a duplicate
/// one, no `cron` or one outside § 2's dialect, a `timezone` no IANA database knows, no `script` or
/// one outside the `script.spawn` roots, no `scope` or one that is neither `fleet` nor `host`, an
/// `overlap` that is none of § 6's three, or a `fleet` entry with no shared store to hold § 3's
/// lease.
pub fn validate(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    if config.schedule.is_empty() {
        return Ok(());
    }
    // The grant side, canonical, once for the whole roster rather than once per entry.
    let mut granted = config.capabilities.clone().unwrap_or_default();
    granted.canonicalize(files);

    let mut claimed: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, entry) in config.schedule.iter().enumerate() {
        let name = required(entry, index, "name", entry.name.as_deref(), origins)?;
        if let Some(first) = claimed.insert(name, index) {
            return Err(refusal(
                index,
                entry,
                format!(
                    "`{name}` is also the name of {}",
                    label(first, &config.schedule[first])
                ),
                "§ 1 makes `name` the label a run is logged and measured under, so two entries \
                 sharing one cannot be told apart in either",
                "give one of them a name of its own",
                origins,
                "name",
            ));
        }

        let cron = required(entry, index, "cron", entry.cron.as_deref(), origins)?;
        if let Err(fault) = Cron::parse(cron) {
            return Err(refusal(
                index,
                entry,
                format!("`cron = \"{cron}\"` is not a schedule: {fault}"),
                "§ 2 accepts five-field POSIX cron — minute, hour, day-of-month, month, \
                 day-of-week — plus `@hourly`, `@daily`, `@weekly`, `@monthly` and `@yearly`; a \
                 seconds field and Quartz's `L`, `W`, `#` and `?` are not part of the dialect",
                "write the five fields, or one of the five shorthands",
                origins,
                "cron",
            ));
        }

        if let Err(unknown) = zone_of(entry.timezone.as_deref()) {
            return Err(refusal(
                index,
                entry,
                format!("`timezone = \"{unknown}\"` is not a zone this build knows"),
                "§ 6 pins the zone because implementations differ about DST and the difference is a \
                 production incident; the name is the operator's only way to say which rules apply, \
                 and a name the IANA database does not carry has no fires to compute at all",
                "name an IANA zone, as `Europe/Vienna`, or drop the key for the `UTC` default",
                origins,
                "timezone",
            ));
        }

        let scope = required(entry, index, "scope", entry.scope.as_deref(), origins)?;
        if scope != "fleet" && scope != "host" {
            return Err(refusal(
                index,
                entry,
                format!("`scope = \"{scope}\"` is neither `fleet` nor `host`"),
                "§ 3 has an entry fire either once across the deployment, over a lease in the \
                 shared store, or once on every host that runs it; there is no third answer and no \
                 default, because which one is right is a property of the job",
                "set `scope = \"host\"` for work whose effect is local, `scope = \"fleet\"` for \
                 work that must happen once",
                origins,
                "scope",
            ));
        }
        if scope == "fleet" && !configures_a_shared_store(config) {
            return Err(refusal(
                index,
                entry,
                "`scope = \"fleet\"` needs a shared store, and this configuration has none"
                    .to_string(),
                "§ 3 fires a fleet entry under a lease held in the shared store; with no store the \
                 only thing left is one run per host, which is the failure `scope` exists to \
                 prevent, so the boot refuses rather than degrading to it",
                "configure the shared store, or set `scope = \"host\"` if one run per host is what \
                 this job means",
                origins,
                "scope",
            ));
        }

        // § 6's `overlap` has a default, so what is refused here is a *word* rather than a missing
        // key. It earns a refusal because the three differ in whether a fire is dropped, held or
        // allowed to replace the run before it: a typo falling through to `skip` is a job that
        // quietly does nothing where the operator asked for the previous run to be cancelled, and
        // the only place that is observable is the run that did not happen.
        if let Some(overlap) = entry.overlap.as_deref()
            && !matches!(overlap, "skip" | "queue" | "kill")
        {
            return Err(refusal(
                index,
                entry,
                format!("`overlap = \"{overlap}\"` is none of `skip`, `queue` or `kill`"),
                "§ 6 gives an entry that comes due while its last run is still going exactly three \
                 answers — drop this fire, hold one until that run ends, or cancel that run and \
                 wait for its teardown — and `skip` is what an entry that says nothing gets",
                "drop the key for `skip`, or name one of the three",
                origins,
                "overlap",
            ));
        }

        let script = required(entry, index, "script", entry.script.as_deref(), origins)?;
        // [`anchor`] has already joined a relative script to the folder of the file that wrote it,
        // so this checks the path every fire will run.
        let named = Path::new(script);
        if !granted.allows(Cap::ScriptSpawn, Scope::Path(named), files) {
            return Err(refusal(
                index,
                entry,
                format!("`{}` is not under any `script.spawn` root", named.display()),
                "§ 1 resolves a scheduled script against the same `[capabilities] script.spawn` \
                 roots a `spawn script` target is checked against: the set of files a deployment \
                 can execute is one list, and a schedule is not a way around it",
                "move the script under a granted root, or add its directory to `script.spawn`",
                origins,
                "script",
            ));
        }
    }
    Ok(())
}

/// Whether the tree configures the shared store § 3's `fleet` lease lives in.
///
/// **`[cache.shared] url` is that store, and it is the only one.**
/// `rule:core-api/two-cache-tiers`'s coherent tier is what
/// every core and every host sees, `Core\Cache::shared()` opens it from this same key
/// (`nvs_stdlib::cache`), and a lease no other host can read is not a lease — so what is asked here
/// is whether that URL is written, not whether the tree configures a store of some kind. A block
/// present with an empty `url` has said nothing, which is how [`required`] reads an empty required
/// key and is the same rule for the same reason.
///
/// **This does not connect, on purpose.** A store that is configured and unreachable is an interval
/// that does not run and is logged, which is § 3's stated at-most-once; a boot that dialled the
/// store to decide whether to start would turn an unreachable Redis into a refusal to serve requests
/// that have nothing to do with the schedule.
fn configures_a_shared_store(config: &Config) -> bool {
    config
        .cache
        .as_ref()
        .and_then(|cache| cache.shared.as_ref())
        .and_then(|shared| shared.url.as_deref())
        .is_some_and(|url| !url.trim().is_empty())
}

/// A field § 1 requires, or the refusal naming it. An empty string is absent: a key set to `""` has
/// said nothing, and reporting it as present would send the operator looking for a different bug.
fn required<'a>(
    entry: &Schedule,
    index: usize,
    field: &str,
    value: Option<&'a str>,
    origins: &BTreeMap<String, Origin>,
) -> Result<&'a str, Diagnostic> {
    match value {
        Some(text) if !text.trim().is_empty() => Ok(text),
        _ => Err(refusal(
            index,
            entry,
            format!("has no `{field}`"),
            "§ 1 gives a `[[schedule]]` entry no defaults for `name`, `cron`, `script` or `scope`: \
             each is a decision about the job that only the operator can make, and a default would \
             be this file guessing at one",
            "add the key to the entry",
            origins,
            field,
        )),
    }
}

/// One `E0611`, with the entry named and the origin of the key at fault.
#[allow(clippy::too_many_arguments)]
fn refusal(
    index: usize,
    entry: &Schedule,
    what: String,
    note: &str,
    help: &str,
    origins: &BTreeMap<String, Origin>,
    field: &str,
) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_SCHEDULE,
        format!("{} {what}", label(index, entry)),
    )
    .with_note(format!(
        "{note}{}",
        origin_note(origins.get(&format!("schedule.{index}.{field}")))
    ))
    .with_help(help.to_string())
}

/// How a refusal names an entry: by the label § 1 requires, and by position until it has one.
fn label(index: usize, entry: &Schedule) -> String {
    match entry.name.as_deref() {
        Some(name) if !name.trim().is_empty() => format!("`[[schedule]]` `{name}`"),
        _ => format!("`[[schedule]]` entry {}", index + 1),
    }
}

/// § 2's dialect, read: which minutes, hours, days and months an entry names, as one bit per
/// accepted value.
///
/// **The boot refusal and the scheduler are the same read**, which is the module doc's own reason
/// for this type being `pub`: [`validate`] refuses whatever [`Cron::parse`] cannot answer, and
/// [`Cron::next_after`] answers it for the entries that survived. A `Cron` in hand is therefore an
/// expression a boot already accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cron {
    /// One bit per accepted value, `0` at the least significant end. Five fields, `FIELDS` order.
    accepted: [u64; 5],
    /// Whether `day-of-month` and `day-of-week` each narrow the day, in that order — POSIX's own
    /// star rule, and the reason the two are read together in [`Cron::day_accepts`].
    narrows: [bool; 2],
}

impl Cron {
    /// § 2's dialect, parsed — or why `expression` is not a schedule.
    ///
    /// # Errors
    ///
    /// The fault, as the clause [`validate`]'s refusal reads into `is not a schedule: {fault}`: a
    /// field count that is not five, a shorthand outside the five, a term the field's range does not
    /// hold, or a step that is not a count or does not apply to a range.
    pub fn parse(expression: &str) -> Result<Self, String> {
        let expression = expression.trim();
        let expanded = if expression.starts_with('@') {
            let named = expression.to_ascii_lowercase();
            *SHORTHANDS
                .iter()
                .find_map(|(name, expansion)| (*name == named).then_some(expansion))
                .ok_or_else(|| {
                    format!(
                        "`{expression}` is not one of {}",
                        SHORTHANDS.map(|(name, _)| name).join(", ")
                    )
                })?
        } else {
            expression
        };

        let fields: Vec<&str> = expanded.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(format!(
                "it has {} field(s), and the dialect is exactly five",
                fields.len()
            ));
        }
        let mut accepted = [0_u64; 5];
        for (slot, (text, (name, low, high))) in fields.iter().zip(FIELDS).enumerate() {
            accepted[slot] = field_bits(text, name, low, high)?;
        }
        // Vixie's rule, and POSIX's: a field that opens with `*` does not narrow the day, so `*/2`
        // counts as a star here exactly as a bare `*` does.
        Ok(Self {
            accepted,
            narrows: [!fields[2].starts_with('*'), !fields[4].starts_with('*')],
        })
    }

    /// The first instant strictly after `after` that this expression names, in `after`'s own zone.
    ///
    /// [`None`] when nothing inside [`HORIZON_YEARS`] matches — `30 2 30 2 *`, and the end of the
    /// representable range. **A missed fire is never caught up** (§ 6), so the scheduler asks this
    /// from the clock rather than from the last fire, and an interval that passed while the host was
    /// down is skipped rather than replayed.
    ///
    /// § 6's two DST rules are here, in [`at`], because this is the one place a civil minute becomes
    /// an instant: a minute in a spring-forward gap fires once, at the first instant after the gap;
    /// one in a fall-back repeat fires once, on the first occurrence.
    #[must_use]
    pub fn next_after(&self, after: &Zoned) -> Option<Zoned> {
        let zone = after.time_zone();
        let from = after.datetime();
        // The fire at `after` has happened, so the search opens at the next whole minute. Seconds
        // are not part of the dialect (§ 2), which is what makes truncation the same as flooring.
        let mut when = DateTime::new(
            from.year(),
            from.month(),
            from.day(),
            from.hour(),
            from.minute(),
            0,
            0,
        )
        .ok()?
        .checked_add(Span::new().minutes(1))
        .ok()?;

        // Field by field rather than minute by minute: a wrong month skips a month, not 44,640
        // reads of the same bitset. `30 2 29 2 *` is the case that decides this.
        let horizon = when.year().saturating_add(HORIZON_YEARS);
        while when.year() <= horizon {
            if !accepts(self.accepted[3], when.month()) {
                when = midnight_of(
                    when.date()
                        .first_of_month()
                        .checked_add(Span::new().months(1))
                        .ok()?,
                )?;
                continue;
            }
            if !self.day_accepts(when.date()) {
                when = midnight_of(when.date().tomorrow().ok()?)?;
                continue;
            }
            if !accepts(self.accepted[1], when.hour()) {
                when = DateTime::new(when.year(), when.month(), when.day(), when.hour(), 0, 0, 0)
                    .ok()?
                    .checked_add(Span::new().hours(1))
                    .ok()?;
                continue;
            }
            if accepts(self.accepted[0], when.minute()) {
                let fire = at(when, zone)?;
                // A fall-back repeat runs one civil minute twice. Asked from *inside* the second
                // pass, § 6's first-occurrence rule would answer an instant already behind the
                // clock — the fire that has just happened — so the search carries on instead.
                if fire.timestamp() > after.timestamp() {
                    return Some(fire);
                }
            }
            when = when.checked_add(Span::new().minutes(1)).ok()?;
        }
        None
    }

    /// Whether a date is one this expression fires on, under POSIX's own two-field rule.
    ///
    /// **When `day-of-month` and `day-of-week` both narrow, a date matching *either* fires.** That
    /// is what POSIX says and what every cron an operator has used does, and it is the one place in
    /// the dialect where two fields are not an intersection: `0 0 1 * MON` is the first of the month
    /// *and* every Monday. When only one narrows, the other is a star and the intersection is the
    /// same answer.
    fn day_accepts(&self, date: Date) -> bool {
        let by_month_day = accepts(self.accepted[2], date.day());
        let by_week_day = accepts(self.accepted[4], date.weekday().to_sunday_zero_offset());
        if self.narrows[0] && self.narrows[1] {
            by_month_day || by_week_day
        } else {
            by_month_day && by_week_day
        }
    }
}

/// § 6's zone: what `timezone` names, or `UTC` when the entry does not say.
///
/// There is no ambient timezone anywhere in Novis (`rule:core-api/no-ambient-state`), so an absent key is the documented
/// default rather than the host's setting, and an empty one has said nothing — the same reading
/// [`required`] gives every other key.
///
/// # Errors
///
/// The name as written, when the IANA database this build carries does not know it.
pub fn zone_of(timezone: Option<&str>) -> Result<TimeZone, String> {
    let named = match timezone {
        Some(text) if !text.trim().is_empty() => text.trim(),
        _ => return Ok(TimeZone::UTC),
    };
    TimeZone::get(named).map_err(|_| named.to_string())
}

/// § 6's two DST rules, in the one place a civil minute becomes an instant.
fn at(when: DateTime, zone: &TimeZone) -> Option<Zoned> {
    let ambiguous = zone.to_ambiguous_zoned(when);
    match ambiguous.offset() {
        AmbiguousOffset::Unambiguous { .. } => ambiguous.unambiguous().ok(),
        // A fall-back repeat: the first occurrence, so "runs once a day" stays true.
        AmbiguousOffset::Fold { .. } => ambiguous.earlier().ok(),
        // A spring-forward gap: the minute does not exist, and the fire is the first instant that
        // does — the transition itself. Reading the missing minute with the offset that *follows*
        // the transition lands before it, so the next transition after that is the one wanted.
        AmbiguousOffset::Gap { after, .. } => {
            let inside = after.to_timestamp(when).ok()?;
            let transition = zone.following(inside).next()?;
            Some(transition.timestamp().to_zoned(zone.clone()))
        }
    }
}

/// Whether a field's bitset holds `value`. Every value in the dialect is under 64, which is what
/// makes one `u64` per field the whole representation.
fn accepts(bits: u64, value: i8) -> bool {
    u32::try_from(value).is_ok_and(|value| value < 64 && bits & (1 << value) != 0)
}

/// The first minute of `date`.
fn midnight_of(date: Date) -> Option<DateTime> {
    DateTime::new(date.year(), date.month(), date.day(), 0, 0, 0, 0).ok()
}

/// One field: a comma-separated list of terms, each optionally stepped, as the values it accepts —
/// or why it is not a field.
fn field_bits(text: &str, name: &str, low: u32, high: u32) -> Result<u64, String> {
    let mut bits = 0;
    for term in text.split(',') {
        let (range, written) = match term.split_once('/') {
            Some((range, step)) => (range, Some(step)),
            None => (term, None),
        };
        let mut step = 1;
        if let Some(written) = written {
            match written.parse::<u32>() {
                Ok(0) | Err(_) => {
                    return Err(format!(
                        "the {name} field steps by `{written}`, which is not a count"
                    ));
                }
                Ok(parsed) if range == "*" || range.contains('-') => step = parsed,
                // `5/15` is Quartz's "every 15 from 5"; § 2 takes the range it is short for.
                Ok(_) => {
                    return Err(format!(
                        "the {name} field steps a single value (`{term}`); a step applies to `*` \
                         or to a range, as `*/{written}`"
                    ));
                }
            }
        }
        let (first, last) = if range == "*" {
            (low, high)
        } else {
            let (first, last) = match range.split_once('-') {
                Some((first, last)) => (first, last),
                None => (range, range),
            };
            let (Some(first), Some(last)) = (value_of(first, name), value_of(last, name)) else {
                return Err(format!("the {name} field does not accept `{range}`"));
            };
            if first < low || last > high || first > last {
                return Err(format!(
                    "the {name} field accepts {low}-{high}, and `{range}` is outside it"
                ));
            }
            (first, last)
        };
        let mut value = first;
        while value <= last {
            bits |= 1_u64 << value;
            value += step;
        }
    }
    Ok(bits)
}

/// One term as a number: POSIX's own three-letter names for `month` and `day-of-week`, and digits
/// everywhere. The names are part of the dialect § 2 names, not an extension of it.
fn value_of(term: &str, field: &str) -> Option<u32> {
    if let Ok(number) = term.parse::<u32>() {
        return Some(number);
    }
    let lower = term.to_ascii_lowercase();
    let names: &[&str] = match field {
        "month" => &MONTHS,
        "day-of-week" => &DAYS,
        _ => return None,
    };
    let offset = u32::from(field == "month");
    names
        .iter()
        .position(|known| *known == lower)
        .map(|found| u32::try_from(found).unwrap_or(0) + offset)
}
