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
//! ADR 0092's and belongs to `nvs-render`, and **this module owns none of it**.
//! [`record`] builds an [`nvs_render::Record`] and [`nvs_render::json::line`]
//! renders it; which keys a line carries, that an absent one is omitted rather
//! than written empty, and how a value JSON has no spelling for is written are
//! all decided there, once, for both callers. That module's own doc comment
//! owns why the rendering sits beside `nvs_render::plain::render` rather than in
//! `nvs-runtime` beside the ladder.
//!
//! **The second caller is `nvs_runtime::floor`**, and its module doc owns the
//! floor's half of § 6. `floor::uncaught` builds the same [`Record`] out of an
//! uncaught `Throwable` and renders it through the same
//! [`nvs_render::json::line`]; the one difference is *where* the bytes go, not
//! what they are, and § *Where the bytes go* below owns that. The claim is
//! asserted rather than asserted-about:
//! `application_code_and_the_engine_floor_produce_schema_identical_records`
//! puts one error through both callers and compares the two lines byte for
//! byte. What that caller cost is the `nvs-runtime` → `nvs-render` dependency
//! edge, which ADR 0092 § 1 sanctions and `nvs-render`'s own § *Where this
//! sits* prices.
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

use nvs_render::{Level, Node, Record, Rendered};
use nvs_runtime::{Fault, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, MethodDoc, ParamDoc, Qual,
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
                   ordinary call costs no key. Nothing a bag can hold makes a write fail: a \
                   value the format has no spelling for — `bytes`, a closure, a cycle — is \
                   rendered as what it is rather than refused.",
            shape: &[],
        },
    ],
    ret: "Nothing. A record that cannot be written is dropped rather than retried: the log is not \
          the program's storage.",
    errors: &[],
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
        let line = nvs_render::json::line(&record(level, message, args[2]));
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

/// The `Core\Log\Level` case in slot 0, as the record model's own level.
///
/// [`LEVEL`]'s cases are valued by their syslog severities, so what arrives
/// here is one of ADR 0092 § 2's five integers and
/// [`Level::from_syslog_severity`] reads it back — this module holds no second
/// enum and no second table, which is what keeps a case added to one from being
/// either surface with no rendering or a rendering nothing can reach.
fn level_of(value: &Value) -> Result<Level, Fault> {
    if let Some(level) = value
        .as_int()
        .and_then(|severity| u8::try_from(severity).ok())
        .and_then(Level::from_syslog_severity)
    {
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

/// This call as ADR 0092 § 1's record — the envelope fields this crate has a
/// source for, and the bag as named nodes.
///
/// Everything past building it belongs to `nvs-render`: which keys a rendering
/// writes, that an absent one is omitted rather than empty, and the JSON Lines
/// line itself. That is ADR 0020 § 6's *one implementation, two callers* — the
/// engine floor builds the same `Record` and calls the same [`nvs_render::json::line`].
fn record(level: Level, message: &str, fields: Value) -> Record {
    let mut record = Record::at(level);
    record.envelope.message = Some(Rendered::new(message));
    record.envelope.fields = named(fields);
    record
}

/// The `fields` bag as the envelope's named nodes.
///
/// Walked by [`crate::debug::node`] — the *one* walk — rather than by a second
/// traversal written here, so a value carried as a log field and the same value
/// dumped are the same node, and ADR 0092 § 5's substitution, redaction and
/// elision reach a log record without this module applying any of them itself.
///
/// A bag walks to whichever of the two array shapes its keys make it: a map
/// keeps its own keys, and a list — which `["a", "b"]` is — takes its indices
/// back as names, because the envelope's fields are named and an array's list
/// shape is exactly the one whose names are `"0"`, `"1"`, ….
fn named(fields: Value) -> Vec<(String, Node)> {
    match crate::debug::node(fields) {
        Node::Map(entries) => entries
            .into_iter()
            .map(|(key, node)| (key.as_str().to_owned(), node))
            .collect(),
        Node::Sequence(items) => items
            .into_iter()
            .enumerate()
            .map(|(index, node)| (index.to_string(), node))
            .collect(),
        // Unreachable from source: the row's parameter is a `CoreTy::Array`, so
        // `E0401` refuses anything that is not one at the call, and the walk
        // answers one of the two shapes above for every array — the third
        // answer it has, `Node::Elided(Elision::Depth)`, needs a depth this is
        // the root of.
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use nvs_render::Level;
    use nvs_runtime::logfile::LogFile;
    use nvs_runtime::{
        ClassTable, Ctx, ErrorClass, NvsArray, NvsStr, OutputSink, Value, call, floor,
    };

    use super::{LEVEL, nvs_core_log_write};

    /// The integer `Core\Log\Level::Error` arrives as, read off [`LEVEL`]'s own
    /// row rather than written out here: a lowered enum case *is* that integer,
    /// and a second copy would be exactly the drift the row exists to prevent.
    fn error_severity() -> i64 {
        LEVEL
            .cases
            .iter()
            .find(|(case, _)| *case == "Error")
            .map(|(_, severity)| *severity)
            .expect("ADR 0092 § 2's roster has an `Error`")
    }

    /// ADR 0020 § 6's headline claim, asked as an **agreement** rather than as
    /// a sentence: the tier-4 engine floor and ordinary application code
    /// produce the *same* record for the same error, so a log pipeline never
    /// has to reconcile two shapes depending on which tier happened to write a
    /// line.
    ///
    /// Both halves are driven for real — the floor through
    /// [`nvs_runtime::floor::uncaught`], the application through
    /// [`super::nvs_core_log_write`] itself — and the comparison is of the two
    /// rendered lines, byte for byte. That is what makes this a check on the
    /// *serialiser* rather than on two struct literals: a second writer growing
    /// on either side would still print plausibly on its own line, and would
    /// differ here in the first key it spelled its own way.
    ///
    /// The application's side is written the way a program writes it: the class
    /// and the backtrace are an ordinary `fields` bag, since § 6 makes them
    /// fields and not envelope keys, and the level crosses as the integer a
    /// lowered case is. `ts`, `request_id`, `trace_id` and `span_id` are absent
    /// from both, which is the same agreement one step further out — a floor
    /// that filled an envelope key its ordinary-code twin does not would be the
    /// divergence this test exists to catch.
    #[test]
    fn application_code_and_the_engine_floor_produce_schema_identical_records() {
        // ADR 0020 § 6's error, thrown the way a helper's failure is and
        // unwound through one compiled frame so that it carries a backtrace.
        // The class table is spec § 10's root shape and arrives the one way a
        // context takes one (`Ctx::set_runtime_error_class`).
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = ClassTable::new();
        let root = classes.define("RuntimeError", &SLOTS, &[]);
        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::rc::Rc::new(classes), root));
        ctx.set_pending("the store said no");
        ctx.push_frame("Main::main");
        let thrown = ctx.take_thrown();
        assert!(
            !thrown.is_none(),
            "an installed class is what promotes a bare failure to an object"
        );
        let trace = thrown.trace_as_string();
        assert!(!trace.is_empty(), "one unwound frame is one backtrace");

        let floored = nvs_render::json::line(&floor::uncaught(&thrown));

        let mut fields = NvsArray::new();
        fields.set(
            NvsStr::new(b"class"),
            Value::str(NvsStr::new(thrown.class_name().as_bytes())),
        );
        fields.set(
            NvsStr::new(b"backtrace"),
            Value::str(NvsStr::new(trace.as_bytes())),
        );
        call(
            nvs_core_log_write,
            &mut ctx,
            &[
                Value::int(error_severity()),
                Value::str(NvsStr::new(thrown.message().as_bytes())),
                Value::array(fields),
            ],
        )
        .expect("a buffered sink is the one output that cannot fail");
        let written = String::from_utf8(
            ctx.take_buffered_output()
                .expect("a buffered context hands its bytes back"),
        )
        .expect("a JSON Lines line is text");

        assert_eq!(
            written, floored,
            "ADR 0020 § 6: one record, two callers — every key, in one order, \
             from one serialiser"
        );
        assert!(
            written.starts_with(
                "{\"level\":\"error\",\"msg\":\"the store said no\",\
                 \"fields\":{\"class\":\"RuntimeError\",\"backtrace\":\""
            ) && written.ends_with("\"}}\n"),
            "and the shape both wrote is § 6's — the envelope keys they have a \
             source for, then the bag, and nothing empty: {written}"
        );
    }

    /// ADR 0106 § 10's two bounds on the floor, asked of the **sink** rather
    /// than of either caller — which is the section's own shape, so that no
    /// caller has to be trusted to be rare.
    ///
    /// The rate limit is asked in three steps, because two of them pass on
    /// their own over a sink that simply dropped repeats: a burst becomes one
    /// line, the *next* occurrence after the window carries how many it stands
    /// for, and a record that differs is never held back at all. A limiter
    /// missing the second would lose the multiplicity silently, and one missing
    /// the third would delay the one line a developer is waiting for behind an
    /// unrelated loop.
    ///
    /// Rotation is asked as the **product** that is the actual bound —
    /// everything on disk, live file and retained rotations together, against
    /// `(keep + 1) * max_bytes` — rather than as a count of files, since a
    /// target that rotated diligently and retained everything would pass a file
    /// count while filling the disk anyway. The records are made distinct so
    /// the rate limit above does not answer for the rotation below, which is
    /// also the one arrangement in which both bounds are in force at once.
    #[test]
    fn the_engine_floor_rotates_and_rate_limits_itself() {
        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(OutputSink::Buffer(Vec::new()));
        let repeated = floor::note(Level::Error, "the store said no");

        for _ in 0..5 {
            floor::report(&mut ctx, &repeated);
        }
        let burst = diagnostic(&mut ctx);
        assert_eq!(
            burst.len(),
            1,
            "five identical records inside one window are one line: {burst:?}"
        );
        assert!(
            !burst[0].contains("\"count\""),
            "and the first of them stands only for itself: {}",
            burst[0]
        );

        floor::expire_coalescing_window();
        floor::report(&mut ctx, &repeated);
        let carried = diagnostic(&mut ctx);
        assert_eq!(carried.len(), 1, "the window reopens on one line");
        assert!(
            carried[0].contains("\"count\":5"),
            "carrying the four it swallowed and itself: {}",
            carried[0]
        );

        floor::report(&mut ctx, &floor::note(Level::Error, "a different failure"));
        let other = diagnostic(&mut ctx);
        assert_eq!(other.len(), 1, "a record that differs is written at once");
        assert!(
            !other[0].contains("\"count\""),
            "and stands only for itself: {}",
            other[0]
        );

        // ADR 0106 § 10's first bullet. Two records fit in a 256-byte file and
        // 64 of them do not, so the rotation is reached many times over and the
        // retention bound is what stops the target growing with the loop.
        const MAX_BYTES: u64 = 256;
        const KEEP: usize = 2;
        let path = scratch("floor.log", KEEP);
        ctx.set_diagnostic_sink(OutputSink::File(LogFile::with_bounds(
            path.clone(),
            MAX_BYTES,
            KEEP,
        )));
        let mut produced = 0;
        for n in 0..64 {
            let record = floor::note(Level::Error, &format!("failure {n}"));
            produced += nvs_render::json::line(&record).len() as u64;
            floor::report(&mut ctx, &record);
        }
        // Drop the sink, so the handle is closed before the assertions read
        // what it wrote.
        ctx.set_diagnostic_sink(OutputSink::Sink);

        let mut held = 0;
        for n in 0..=KEEP {
            let rotation = rotation(&path, n);
            let bytes = std::fs::read(&rotation)
                .unwrap_or_else(|why| panic!("rotation {n} is on disk: {why}"));
            assert!(
                bytes.len() as u64 <= MAX_BYTES,
                "no file passes its own bound: rotation {n} holds {} bytes",
                bytes.len()
            );
            for line in String::from_utf8(bytes.clone())
                .expect("JSON Lines is text")
                .lines()
            {
                assert!(
                    line.starts_with('{') && line.ends_with('}'),
                    "a record is a line, and a rotation never cuts one: {line}"
                );
            }
            held += bytes.len() as u64;
        }
        assert!(
            !rotation(&path, KEEP + 1).exists(),
            "the retention bound is {KEEP} rotations beside the live file"
        );
        assert!(
            held <= (KEEP as u64 + 1) * MAX_BYTES && held < produced,
            "the target holds at most `(keep + 1) * max_bytes` however much is \
             written through it: {held} of {produced} produced"
        );
    }

    /// Everything on the diagnostic channel since it was last read, one line
    /// each.
    fn diagnostic(ctx: &mut Ctx) -> Vec<String> {
        String::from_utf8(
            ctx.take_buffered_diagnostic()
                .expect("the case pointed the channel at a buffer"),
        )
        .expect("a JSON Lines line is text")
        .lines()
        .map(ToOwned::to_owned)
        .collect()
    }

    /// The path rotation `n` of `path` takes, where 0 is the live file — the
    /// spelling `nvs_runtime::logfile` writes, restated here because a case
    /// asserting a retention bound has to name the file the bound is about.
    fn rotation(path: &std::path::Path, n: usize) -> std::path::PathBuf {
        if n == 0 {
            return path.to_owned();
        }
        let mut name = path.to_owned().into_os_string();
        name.push(format!(".{n}"));
        std::path::PathBuf::from(name)
    }

    /// A path under the host's temporary directory that this case owns, with
    /// its live file and every rotation a previous run left behind removed.
    fn scratch(name: &str, keep: usize) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("nvs-floor-rotation");
        std::fs::create_dir_all(&dir).expect("a temporary directory the tests own");
        let path = dir.join(name);
        for n in 0..=keep + 1 {
            let _ = std::fs::remove_file(rotation(&path, n));
        }
        path
    }

    /// [`LEVEL`]'s rows and the record model's roster name the same five cases,
    /// and each row's integer is the severity the model reads it back by. These
    /// are the pair that would otherwise drift: the enum is what a program
    /// writes and the roster is what the record is rendered from, so a case in
    /// one alone is either surface with no rendering or a rendering nothing can
    /// reach.
    #[test]
    fn the_levels_and_their_tags_agree() {
        assert_eq!(LEVEL.cases.len(), 5, "ADR 0092 § 2's roster is five cases");
        for (case, severity) in LEVEL.cases {
            let severity = u8::try_from(*severity)
                .unwrap_or_else(|_| panic!("`Core\\Log\\Level::{case}` is not a syslog severity"));
            let level = Level::from_syslog_severity(severity)
                .unwrap_or_else(|| panic!("`Core\\Log\\Level::{case}` has no level"));
            assert_eq!(
                level.name(),
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
        for absent in [0, 1, 5, 8, 255] {
            assert!(
                Level::from_syslog_severity(absent).is_none(),
                "{absent} is not one of ADR 0092 § 2's five severities"
            );
        }
    }
}
