//! [ADR 0073]'s `[[schedule]]` entries, checked at boot: everything an entry must answer before
//! the scheduler can arm it.
//!
//! **Every refusal here is a boot refusal, and that is the whole point of the module.** A schedule
//! fails silently by construction — an entry that never fires is indistinguishable from one whose
//! interval has not come round, and the operator learns about it from the work that did not happen.
//! So the questions are asked once, over the merged tree, and a tree that cannot answer them does
//! not serve: § 1 for `name` and `script`, § 2 for `cron`, § 3 for `scope`.
//!
//! **Over the merged tree, beside [`app::canonicalize`](crate::app::canonicalize), and for the same
//! reason:** `[[schedule]]` is an array of tables, so entries accumulate across the files
//! ([ADR 0103] § 4) and the roster only exists once the merge is done. A per-file check would refuse
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
//! Cost: one clone of `[capabilities]` and one canonicalization per path-scoped grant at boot and at
//! reload, plus one `realpath` per entry with a `script`. Nothing here runs on a request path.
//!
//! [ADR 0073]: ../../../docs/adr/0073-scheduled-work-is-config.md
//! [ADR 0103]: ../../../docs/adr/0103-configuration-is-a-tree-of-files.md

use std::collections::BTreeMap;
use std::path::Path;

use nvs_diagnostics::{Diagnostic, code};

use crate::capability::{Cap, Scope};
use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, Schedule};

/// The five named shorthands § 2 accepts beside the five-field form, and no others.
const SHORTHANDS: [&str; 5] = ["@hourly", "@daily", "@weekly", "@monthly", "@yearly"];

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

/// § 1–§ 3's boot questions, asked of every `[[schedule]]` entry in the merged tree.
///
/// # Errors
///
/// One [`Diagnostic`], `E0611`, for the first entry that cannot answer: no `name` or a duplicate
/// one, no `cron` or one outside § 2's dialect, no `script` or one outside the `script.spawn` roots,
/// no `scope` or one that is neither `fleet` nor `host`, or a `fleet` entry with no shared store to
/// hold § 3's lease.
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
        if let Some(fault) = cron_fault(cron) {
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

        let script = required(entry, index, "script", entry.script.as_deref(), origins)?;
        let written_in = origins.get(&format!("schedule.{index}.script"));
        // § 5 of ADR 0103: relative to the file that wrote it, the same rule an `[[include]]`, a
        // `password_file` and an `[[app]]` key follow.
        let base = written_in
            .and_then(|origin| origin.path.parent())
            .unwrap_or(Path::new("."));
        let named = crate::resolve::absolute(base, Path::new(script));
        if !granted.allows(Cap::ScriptSpawn, Scope::Path(&named), files) {
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
/// **There is no spelling for one yet.** ADR 0073's *Scope* excludes the store's own configuration
/// and no block in [`tree`](crate::tree) holds it, so the honest answer for every tree today is
/// `false` and every `fleet` entry refuses. That is § 3's rule applied rather than a stand-in for
/// it — the alternative it rejects is degrading to one run per host, not accepting the entry — and
/// when the store gains its block this function reads it and nothing else in the module moves.
fn configures_a_shared_store(_config: &Config) -> bool {
    false
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

/// Why `expression` is not a schedule, or [`None`] when it is one.
///
/// A *validator*, not a parser: it answers whether the scheduler will be able to read the
/// expression, and the firing times are the scheduler's own question. Splitting them keeps the
/// boot refusal in the crate that owns the configuration.
fn cron_fault(expression: &str) -> Option<String> {
    let expression = expression.trim();
    if expression.starts_with('@') {
        return if SHORTHANDS.contains(&expression.to_ascii_lowercase().as_str()) {
            None
        } else {
            Some(format!(
                "`{expression}` is not one of {}",
                SHORTHANDS.join(", ")
            ))
        };
    }
    let fields: Vec<&str> = expression.split_whitespace().collect();
    if fields.len() != 5 {
        return Some(format!(
            "it has {} field(s), and the dialect is exactly five",
            fields.len()
        ));
    }
    for (text, (name, low, high)) in fields.iter().zip(FIELDS) {
        if let Some(fault) = field_fault(text, name, low, high) {
            return Some(fault);
        }
    }
    None
}

/// One field: a comma-separated list of terms, each optionally stepped.
fn field_fault(text: &str, name: &str, low: u32, high: u32) -> Option<String> {
    for term in text.split(',') {
        let (range, step) = match term.split_once('/') {
            Some((range, step)) => (range, Some(step)),
            None => (term, None),
        };
        if let Some(step) = step {
            match step.parse::<u32>() {
                Ok(0) | Err(_) => {
                    return Some(format!(
                        "the {name} field steps by `{step}`, which is not a count"
                    ));
                }
                Ok(_) if range == "*" || range.contains('-') => {}
                // `5/15` is Quartz's "every 15 from 5"; § 2 takes the range it is short for.
                Ok(_) => {
                    return Some(format!(
                        "the {name} field steps a single value (`{term}`); a step applies to `*` \
                         or to a range, as `*/{step}`"
                    ));
                }
            }
        }
        if range == "*" {
            continue;
        }
        let (first, last) = match range.split_once('-') {
            Some((first, last)) => (first, last),
            None => (range, range),
        };
        let (Some(first), Some(last)) = (value_of(first, name), value_of(last, name)) else {
            return Some(format!("the {name} field does not accept `{range}`"));
        };
        if first < low || last > high || first > last {
            return Some(format!(
                "the {name} field accepts {low}-{high}, and `{range}` is outside it"
            ));
        }
    }
    None
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
