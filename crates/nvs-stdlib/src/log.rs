//! `Core\Log` — [ADR 0020](../../../../docs/adr/0020-error-escalation-ladder.md)
//! § 6's reporting half: one member a program writes a record with, and
//! [ADR 0092](../../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
//! § 2's `Core\Log\Level` beside it because that member is the only thing that
//! takes one.
//!
//! **The level enum's integers are the syslog severities, not ordinals.** ADR
//! 0092 § 2 fixes the mapping — `Debug` 7, `Info` 6, `Warn` 4, `Error` 3,
//! `Critical` 2 — because ADR 0020 § 4 names `syslog` as a target and a
//! severity is not optional there. Writing the severity as each case's own
//! constant makes the enum *be* the mapping rather than the first of two
//! tables that would then have to agree; [`crate::router::METHOD`] writes its
//! integers out for the same reason and spends them differently. Two
//! consequences a reader should have in hand: the roster is deliberately not
//! monotonic in ordinal order (`Warn` is 4, skipping syslog's `Notice` 5, which
//! § 2 rejects along with the rest of PSR-3's eight), and `[log] level`'s
//! minimum is a `<=` over these numbers rather than a `>=`, because a smaller
//! severity is a louder record.
//!
//! # What a record is, and what this module does *not* own
//!
//! ADR 0020 § 6's claim is about **sameness**: the tier-4 engine floor and
//! ordinary application code write the same record through the same native
//! helper, so a log pipeline never has to reconcile two shapes. The record
//! itself — its envelope, its node model and its three renderings — is
//! ADR 0092's and belongs to `nvs-render`; the JSON Lines rendering is § 3's
//! entry for a log target.
//!
//! **Known gap: this module is still the second writer that arrangement
//! exists to prevent.** [`Record`] below serialises `level`, `message` and
//! `fields` here, in `nvs-stdlib`, because the floor it is supposed to share
//! has not been written — nothing in `nvs-runtime` emits a record at all
//! today, so there is nothing yet to reach. What is already shared is the
//! *value* encoder: `fields` is written by [`crate::json::Encodable`], the one
//! `Core\Json::encode` uses, so a `float` or a nested array cannot be spelled
//! two ways depending on which member wrote it. Closing the rest means moving
//! [`Record`] down to the floor and calling it from here, which is the slice
//! after this one.
//!
//! **The envelope is `level` and `msg` and stops there.** § 6 lists `ts`,
//! `request_id`, `trace_id` and `span_id` as well; none of them has a source
//! yet — there is no request, no trace and no clock the fixture could freeze —
//! and § 6 already says `trace_id`/`span_id` are omitted rather than empty
//! when no trace is active, which is the same treatment the rest take here.
//! `fields` follows that rule too: an empty bag is an absent key, not `{}`.
//!
//! # Where the bytes go
//!
//! [`Ctx::write_output`](nvs_runtime::Ctx::write_output) — the program's own
//! output stream, not [`Ctx::write_diagnostic`](nvs_runtime::Ctx::write_diagnostic),
//! which is where [`crate::debug`] sends a dump. The split is by *whose*
//! record it is. A `Core\Log::write` is something the program chose to say, so
//! it is output and ADR 0088 § 5's sink rules apply to it — `Core\Out::capture`
//! around one captures it, which is exactly what ADR 0092 § 3 asks of every
//! rendering. A record the engine writes about a program that has already
//! stopped is not the program's output and lands on the diagnostic channel
//! instead; `[log] target`'s `stderr`/`file:`/`syslog` routing is that side's,
//! and it arrives with the floor.

use nvs_runtime::{Fault, ThrownClass, Value};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use crate::json::Encodable;
use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc,
    ParamDoc, Qual,
};

/// This class's fully-qualified name, in one place so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Log";

/// [`LEVEL`]'s fully-qualified name, written once for the same reason.
pub(crate) const LEVEL_NAME: &str = r"Core\Log\Level";

/// ADR 0092 § 2's five cases, valued by the syslog severity that section fixes
/// for each — this module's own doc comment owns why the value is the severity
/// rather than an ordinal.
pub(crate) const LEVEL: CoreEnum = CoreEnum {
    name: LEVEL_NAME,
    cases: &[
        ("Debug", 7),
        ("Info", 6),
        ("Warn", 4),
        ("Error", 3),
        ("Critical", 2),
    ],
    doc: Some(&LEVEL_DOC),
};

/// [`LEVEL`]'s reference card — ADR 0117.
const LEVEL_DOC: EnumDoc = EnumDoc {
    short: "How loud a log record is — five cases, each valued by its own syslog severity so that \
            a `syslog` target needs no second table. A smaller number is a louder record.",
    cases: &[
        CaseDoc {
            name: "Debug",
            desc: "Detail kept for whoever is looking at this run; `Core\\Debug::dump`'s own \
                   destination.",
        },
        CaseDoc {
            name: "Info",
            desc: "Something the program did that an operator would want in the record.",
        },
        CaseDoc {
            name: "Warn",
            desc: "Something recoverable that nobody chose — a retry, a fallback, a deprecated \
                   path still in use.",
        },
        CaseDoc {
            name: "Error",
            desc: "An operation failed. An uncaught `Throwable` is reported at this level.",
        },
        CaseDoc {
            name: "Critical",
            desc: "The escalation ladder's own level: a resource limit stopped the request, or \
                   the engine floor is reporting for a program that can no longer report for \
                   itself.",
        },
    ],
};

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
        name: "write",
        names: &["level", "message", "fields"],
        params: &[
            CoreTy::Enum(LEVEL_NAME),
            CoreTy::Text(Qual::Neutral),
            CoreTy::Array(&CoreTy::Mixed),
        ],
        defaults: &[Const::EmptyArray],
        return_ty: CoreTy::Void,
        symbol: "nvs_core_log_write",
        doc: Some(&WRITE_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Log::write`'s reference card — ADR 0117.
const WRITE_DOC: MethodDoc = MethodDoc {
    short: "Writes one log record — the same record, through the same writer, the engine itself \
            uses when it reports for a program that has stopped, so a log pipeline never sees two \
            shapes for one event. The rendering at a log target is JSON Lines: one object per \
            line.",
    params: &[
        ParamDoc {
            name: "level",
            desc: "How loud the record is. `Core\\Log\\Level`'s five cases carry their own syslog \
                   severities.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "What happened, as one plain sentence. A `tainted` value is accepted here — a \
                   log record is data and recording untrusted input is the point — while a \
                   `secret` one is refused, which is the rule for every message a human reads.",
            shape: &[],
        },
        ParamDoc {
            name: "fields",
            desc: "Structured context, written as a `fields` object beside the message rather \
                   than pasted into it. Omitted from the record entirely when it is empty, so an \
                   ordinary call costs no key.",
            shape: &[],
        },
    ],
    ret: "Nothing. A record that cannot be written is dropped rather than retried: the log is not \
          the program's storage.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "A `fields` value has no JSON encoding — a closure, or a value nested past the \
               depth `Core\\Json::encode` accepts. The bag was built by the program, so an \
               unwritable one is a bug in it.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_log_write" => (nvs_core_log_write as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Log::write(Core\Log\Level $level, string $message, array<string, mixed> $fields = []): void`
    /// — ADR 0020 § 6.
    ///
    /// One `write_output` call per record rather than one per field: JSON
    /// Lines' whole contract is that a record is a line, and a partial write
    /// interleaved with another core's is the one way to break it.
    fn nvs_core_log_write(ctx, args: [3]) {
        let level = level_of(&args[0])?;
        let message = message_of(&args[1])?;
        let line = render(level, message, args[2])?;
        // Unreachable from source, for the reason `Core\Debug::dump`'s own
        // write is: the only output sink that can fail is the process's
        // stdout, and nothing in the language moves the channel or closes the
        // descriptor. A program that logs cannot make this happen; only the
        // host can, by handing the process an output stream it then breaks.
        ctx.write_output(line.as_bytes())
            .map_err(|why| Fault::fatal(format!("Core\\Log::write could not write: {why}")))?;
        Ok(Value::null())
    }
}

/// The five cases, as the thing this module dispatches on.
///
/// A Rust enum rather than the raw severity so that [`Self::tag`] and
/// [`LEVEL`] are held together by `the_levels_and_their_tags_agree` below: a
/// case added to one alone is either surface with no rendering or a rendering
/// nothing can reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

impl Level {
    /// The case whose syslog severity is `severity`, or `None` for a number
    /// [`LEVEL`] does not declare.
    fn from_severity(severity: i64) -> Option<Self> {
        Some(match severity {
            7 => Self::Debug,
            6 => Self::Info,
            4 => Self::Warn,
            3 => Self::Error,
            2 => Self::Critical,
            _ => return None,
        })
    }

    /// How the record spells this level — the case's own name, lowercased,
    /// which is what every log pipeline in this ecosystem already reads.
    fn tag(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

/// The `Core\Log\Level` case in slot 0.
fn level_of(value: &Value) -> Result<Level, Fault> {
    if let Some(level) = value.as_int().and_then(Level::from_severity) {
        return Ok(level);
    }
    // Unreachable from source: the row's parameter is `CoreTy::Enum`, so
    // `E0401` refuses anything that is not a case of it at the call, and a
    // case of it is one of the five integers above. What this catches is a
    // lowering bug, and it is fatal rather than thrown for the reason
    // `crate::hash`'s `digest_kind` gives: a runtime-contract violation is not
    // something a program can catch its way out of.
    Err(Fault::fatal(format!(
        "Core\\Log::write expected a `Core\\Log\\Level` case, got tag {} value {:?}",
        value.tag_byte(),
        value.as_int()
    )))
}

/// The message in slot 1.
fn message_of(value: &Value) -> Result<&str, Fault> {
    // Unreachable from source: the row's parameter is `CoreTy::Text`, so
    // `E0401` refuses anything that is not a `string` at the call — `bytes`
    // included, which is its own tag and not a string this could be handed by
    // accident.
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Log::write expected a `string` for its message, got tag {}",
            value.tag_byte()
        ))
    })
}

/// One record, as the line that carries it — the newline included, because a
/// JSON Lines record without its terminator is not one.
fn render(level: Level, message: &str, fields: Value) -> Result<String, Fault> {
    let record = Record {
        level: level.tag(),
        msg: message,
        fields: encodable(fields),
    };
    let mut line = serde_json::to_string(&record).map_err(|why| {
        Fault::thrown_as(ThrownClass::Logic, format!("Core\\Log::write(): {why}"))
    })?;
    line.push('\n');
    Ok(line)
}

/// The `fields` bag as something to write, or `None` for one carrying nothing.
///
/// An empty bag is an absent key rather than `{}`: § 6's own `trace_id` rule
/// is that a field with no content is omitted, and a record is read by people
/// far more often than it is parsed.
fn encodable(fields: Value) -> Option<Encodable> {
    let ptr = fields.array_ptr()?;
    if crate::arr::borrowed(ptr).is_empty() {
        return None;
    }
    Some(Encodable::document(fields))
}

/// ADR 0020 § 6's record, as far as this crate can fill it — the module doc
/// owns which of § 6's envelope fields are absent and why.
///
/// Serialised by hand rather than by `derive` for the one thing a derive
/// cannot express: `fields` is omitted when there is nothing in it, and the
/// key order is the reading order rather than a struct's declaration order
/// being trusted to stay one.
struct Record<'a> {
    level: &'static str,
    msg: &'a str,
    fields: Option<Encodable>,
}

impl Serialize for Record<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(None)?;
        map.serialize_entry("level", self.level)?;
        map.serialize_entry("msg", self.msg)?;
        if let Some(fields) = self.fields.as_ref() {
            map.serialize_entry("fields", fields)?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::{LEVEL, Level};

    /// [`LEVEL`]'s rows and [`Level`]'s dispatch name the same five cases, and
    /// each row's integer is the severity the dispatch reads it back by. These
    /// are the pair that would otherwise drift: the enum is what a program
    /// writes and the `match` is what the record is rendered from, so a case
    /// in one alone is either surface with no rendering or a rendering nothing
    /// can reach.
    #[test]
    fn the_levels_and_their_tags_agree() {
        assert_eq!(LEVEL.cases.len(), 5, "ADR 0092 § 2's roster is five cases");
        for (case, severity) in LEVEL.cases {
            let level = Level::from_severity(*severity)
                .unwrap_or_else(|| panic!("`Core\\Log\\Level::{case}` has no dispatch"));
            assert_eq!(
                level.tag(),
                case.to_lowercase(),
                "`Core\\Log\\Level::{case}` renders as its own name, lowercased"
            );
        }
    }

    /// A severity no row declares has no case, which is what makes
    /// [`super::level_of`]'s refusal a real check rather than a formality —
    /// syslog's `Notice` (5) and `Alert` (1) are the two ADR 0092 § 2 names as
    /// rejected, so they are the ones asked for here.
    #[test]
    fn an_undeclared_severity_is_not_a_case() {
        for absent in [0, 1, 5, 8, -1] {
            assert!(
                Level::from_severity(absent).is_none(),
                "{absent} is not one of ADR 0092 § 2's five severities"
            );
        }
    }
}
