//! `Core\Log` — `rule:errors/log-write`'s reporting half: one member a program writes a record with, and
//! `rule:errors/log-level`'s `Core\Log\Level` beside it because that member is the only thing that
//! takes one.
//!
//! **The level enum's integers are the syslog severities, not ordinals.** ADR
//! 0092 § 2 fixes the mapping — `Debug` 7, `Info` 6, `Warn` 4, `Error` 3,
//! `Critical` 2 — because `rule:errors/engine-floor` names `syslog` as a target and a
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
//! `rule:errors/log-write`'s claim is about **sameness**: the tier-4 engine floor and
//! ordinary application code write the same record through the same native
//! helper, so a log pipeline never has to reconcile two shapes. The record
//! itself — its envelope, its node model and its three renderings — is
//! `rule:errors/diagnostic-record`'s and belongs to `nvs-render`, and **this module owns none of it**.
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
//! edge, which `rule:errors/diagnostic-record` sanctions and `nvs-render`'s own § *Where this
//! sits* prices.
//!
//! **The envelope is `level` and `msg` for a CLI run, and six keys inside a
//! request.** § 6's other four — `ts`, `request_id`, `trace_id` and `span_id` —
//! are stamped by
//! [`Ctx::stamp_envelope`](nvs_runtime::Ctx::stamp_envelope), which is the
//! method `nvs_runtime::floor` calls too and is the one home for which key
//! comes from what: `request_id` is `rule:observability/a-trace-id-exists-for-every-request`'s trace id, that section
//! having made it Novis's only request identifier, and `trace_id`/`span_id`
//! arrive on top of it for a *sampled* trace, which is § 6's "whenever a trace
//! is active". Without them a log line could not be jumped to from a trace,
//! which is the whole of what § 6 asks for. An absent key is omitted rather
//! than written empty; `fields` follows that rule too, so an empty bag is an
//! absent key and not `{}`.
//!
//! # Where the bytes go
//!
//! [`Ctx::write_log_record`](nvs_runtime::Ctx::write_log_record), which is the
//! one reader of `[log] target`, `level` and `format`, and takes the channel to
//! use when the first of those names nothing. **Both of `rule:errors/record-producers`'s writers call it**, so a
//! deployment naming a destination gets one destination and not two —
//! § 6's sameness is about the record, and a per-caller destination is the
//! other half of the same claim. That method's doc comment owns the routing,
//! including why a named target is not captured by `Core\Out::capture`.
//!
//! With no target configured the two callers keep the split they have always
//! had, and it is by *whose* record it is. A `Core\Log::write` is something the
//! program chose to say, so it is output —
//! [`Ctx::write_output`](nvs_runtime::Ctx::write_output), where `rule:security/capture-answers-the-carrier`'s
//! sink rules apply and `Core\Out::capture` around one captures it, which is
//! exactly what `rule:errors/renderings` asks of every rendering. A record the engine
//! writes about a program that has already stopped is not the program's output
//! and lands on [`Ctx::write_diagnostic`](nvs_runtime::Ctx::write_diagnostic)
//! instead, beside where [`crate::debug`] sends a dump.

use nvs_render::{Level, Node, Record, Rendered, Source};
use nvs_runtime::{Ctx, Fault, LogChannel, Value};

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, MethodDoc,
    ParamDoc, Qual,
};

/// This class's fully-qualified name, in one place so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Log";

/// [`LEVEL`]'s fully-qualified name, written once for the same reason.
pub(crate) const LEVEL_NAME: &str = r"Core\Log\Level";

/// `rule:errors/log-level`'s five cases, valued by the syslog severity that section fixes
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

/// [`LEVEL`]'s reference card — `rule:core-api/reference-card`.
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
    doc: Some(&CARD),
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

/// `Core\Log`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Writes log records. `write` writes one record as one line of JSON, with a level, a \
            message and any extra fields. The server's `[log]` settings decide where records go \
            and which levels are kept.",
};

/// `Core\Log::write`'s reference card — `rule:core-api/reference-card`.
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
                   ordinary call costs no key. Nothing in `fields` makes a write fail. A \
                   value the format cannot write, such as `bytes`, a callable or a cycle, is \
                   written as text that says what it is.",
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
    /// — `rule:errors/log-write`.
    ///
    /// One write call per record rather than one per field: JSON Lines' whole
    /// contract is that a record is a line, and a partial write interleaved
    /// with another core's is the one way to break it.
    ///
    /// Where that one call lands is
    /// [`Ctx::write_log_record`](nvs_runtime::Ctx::write_log_record)'s, not
    /// this member's — the module doc's § *Where the bytes go* owns the split.
    fn nvs_core_log_write(ctx, args: [4]) {
        #[expect(
            unsafe_code,
            reason = "the carrier came out of a `SourceConst` the compiled unit baked into its own data section, which outlives every request served from it"
        )]
        let source = unsafe { nvs_runtime::source::of_operand(args[0]) };
        let level = level_of(&args[1])?;
        let message = message_of(&args[2])?;
        // A record below `[log] level` is never written, so it is never built
        // either. Building it first would cost the fields' conversion, the
        // clock and a slot in `nvs_runtime::floor`'s coalescing table, and
        // that slot could push out a record that is written.
        if !ctx.log_writes(level) {
            return Ok(Value::null());
        }
        let mut record = record(ctx, source, level, message, args[3]);
        // `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`: this
        // target's bound is a finite disk, and the cheapest place to protect
        // one is before the bytes exist. A run of identical records is one line
        // carrying the count `nvs_runtime::floor` stamps on, and the writes it
        // stands for do not happen — over that module's table rather than a
        // window of this member's own, because what makes two records the same
        // record has to be one answer for both of the writers § 6 keeps
        // identical.
        if !nvs_runtime::floor::admit_log_record(&mut record) {
            return Ok(Value::null());
        }
        // Unreachable from source. Absent a `[log] target` the destination is
        // the process's own output stream, which nothing in the language moves
        // or closes — the reason `Core\Debug::dump`'s own write gives. With one
        // configured it is the operator's file, which a full disk can fail:
        // still nothing the program said, and still a `FATAL` rather than a
        // throw, because a `catch` around a log write is not where a
        // deployment's unwritable log gets handled.
        ctx.write_log_record(&record, LogChannel::Output)
            .map_err(|why| Fault::fatal(format!("Core\\Log::write could not write: {why}")))?;
        Ok(Value::null())
    }
}

/// The `Core\Log\Level` case in slot 0, as the record model's own level.
///
/// [`LEVEL`]'s cases are valued by their syslog severities, so what arrives
/// here is one of `rule:errors/log-level`'s five integers and
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

/// This call as `rule:errors/diagnostic-record`'s record — the envelope fields this crate has a
/// source for, and the bag as named nodes.
///
/// `source` is the one field neither this crate nor the context can answer for:
/// `rule:errors/a-record-names-where-it-was-produced` makes it a property of the
/// *call site*, so it arrives as the constant
/// [`crate::registry::SOURCE_MEMBERS`] puts in argument 0 and is set here
/// rather than by [`Ctx::stamp_envelope`], which answers for the request.
///
/// Everything past building it belongs elsewhere: which keys a rendering
/// writes and that an absent one is omitted rather than empty are
/// `nvs-render`'s, and *which* rendering is
/// [`Ctx::write_log_record`](nvs_runtime::Ctx::write_log_record)'s, under
/// `[log] format`. That is `rule:errors/log-write`'s *one implementation, two callers* —
/// the engine floor builds the same `Record` and hands it to the same method,
/// so neither this member nor the floor has a rendering to choose.
fn record(ctx: &Ctx, source: Option<Source>, level: Level, message: &str, fields: Value) -> Record {
    let mut record = Record::at(level);
    record.envelope.message = Some(Rendered::new(message));
    record.envelope.source = source;
    record.envelope.fields = named(fields);
    // § 6's other four envelope keys, from the one place that has them and for
    // both of that section's writers — [`Ctx::stamp_envelope`]'s own doc owns
    // which key comes from what and why a CLI run gets none of them.
    ctx.stamp_envelope(&mut record.envelope);
    record
}

/// The `fields` bag as the envelope's named nodes.
///
/// Walked by [`crate::debug::node`] — the *one* walk — rather than by a second
/// traversal written here, so a value carried as a log field and the same value
/// dumped are the same node, and `rule:errors/record-transformations`'s substitution, redaction and
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
    use nvs_render::{Level, Source};
    use nvs_runtime::logfile::LogFile;
    use nvs_runtime::{
        ClassTable, Ctx, ErrorClass, Inbound, NvsArray, NvsStr, OutputSink, TraceContext, Value,
        call, floor,
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
            .expect("`rule:errors/log-level`'s roster has an `Error`")
    }

    /// `rule:errors/log-write`'s headline claim, asked as an **agreement** rather than as
    /// a sentence: the tier-4 engine floor and ordinary application code
    /// produce the *same* record for the same error, so a log pipeline never
    /// has to reconcile two shapes depending on which tier happened to write a
    /// line.
    ///
    /// Both halves are driven for real — the floor through
    /// [`nvs_runtime::floor::uncaught`], the application through
    /// [`super::nvs_core_log_write`] itself — and the comparison is of the two
    /// rendered lines, byte for byte up to the frames. That is what makes this
    /// a check on the *serialiser* rather than on two struct literals: a second
    /// writer growing on either side would still print plausibly on its own
    /// line, and would differ here in the first key it spelled its own way.
    ///
    /// The one thing the floor's line carries that the application's does not
    /// is the **frames**, which `rule:errors/record-producers` makes the
    /// record's own nodes and which this caller has no source for — the same
    /// asymmetry `the-floor-and-core-log-write-record-the-same-error-the-same-way.nvst`
    /// states from the other end, where the application's line is the one
    /// carrying a `source`. So the agreement asserted is that the floor's line
    /// *is* the application's with the nodes appended: one envelope, one bag,
    /// one order, one serialiser.
    ///
    /// The application's side is written the way a program writes it: the class
    /// is an ordinary `fields` bag, since § 6 makes it a field and not an
    /// envelope key, and the level crosses as the integer a
    /// lowered case is. `ts`, `request_id`, `trace_id` and `span_id` are absent
    /// from both, because this context answers no request and § 6's four
    /// request keys are stamped from one — the same agreement one step further
    /// out, and a floor that filled an envelope key its ordinary-code twin does
    /// not would be the divergence this test exists to catch.
    /// [`a_cli_runs_record_is_still_level_and_msg_alone`] asks the same
    /// absence of the application half on its own.
    // covers: Core\Log::write
    #[test]
    fn application_code_and_the_engine_floor_produce_schema_identical_records() {
        // `rule:errors/log-write`'s error, thrown the way a helper's failure is and
        // unwound through one compiled frame so that it carries a backtrace.
        // The class table is spec § 10's root shape and arrives the one way a
        // context takes one (`Ctx::set_runtime_error_class`).
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = ClassTable::new();
        let root = classes.define("RuntimeError", &SLOTS, &[]);
        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(classes), root));
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
        call(
            nvs_core_log_write,
            &mut ctx,
            &[
                no_source(),
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

        let (envelope, nodes) = floored
            .split_once(",\"nodes\":")
            .expect("the floor's record carries the frames this caller has none of");
        assert_eq!(
            format!("{envelope}}}\n"),
            written,
            "`rule:errors/log-write`: one record, two callers — every key, in one order, \
             from one serialiser"
        );
        assert_eq!(
            nodes, "[{\"function\":\"Main::main\"}]}\n",
            "and what the floor adds is the frames, one node each, from the same \
             trace `{trace}` a program reads off the object"
        );
        assert_eq!(
            written,
            "{\"level\":\"error\",\"msg\":\"the store said no\",\
             \"fields\":{\"class\":\"RuntimeError\"}}\n",
            "and the shape both wrote is § 6's — the envelope keys they have a \
             source for, then the bag, and nothing empty"
        );
    }

    /// The zero word [`crate::registry::SOURCE_MEMBERS`] hands a producer
    /// with no call site — what a test driving the helper by hand holds, there
    /// being no compiled unit under it to have baked a carrier.
    fn no_source() -> Value {
        Value::source_const(std::ptr::null())
    }

    /// One `Core\Log::write` of `"a message"` from `source`, as the line it
    /// wrote — the whole helper, so what is asserted is what a target receives.
    fn from_source(source: Value) -> String {
        let mut ctx = Ctx::buffered();
        call(
            nvs_core_log_write,
            &mut ctx,
            &[
                source,
                Value::int(error_severity()),
                Value::str(NvsStr::new(b"a message")),
                Value::array(NvsArray::new()),
            ],
        )
        .expect("a buffered sink is the one output that cannot fail");
        String::from_utf8(
            ctx.take_buffered_output()
                .expect("a buffered context hands its bytes back"),
        )
        .expect("a JSON Lines line is text")
    }

    /// `rule:errors/a-record-names-where-it-was-produced`'s member half: a
    /// record produced inside a method names it, and one produced at file scope
    /// names the file and the line alone rather than an empty member.
    ///
    /// Read off the rendered line rather than off the envelope, because the
    /// omission is only a *fact* once a rendering has had the chance to write
    /// the key — the same reason
    /// [`a_field_with_no_value_is_omitted_rather_than_empty`] asks its question
    /// of the serialiser.
    #[test]
    fn a_log_record_reports_its_enclosing_member_and_none_at_file_scope() {
        let inside = nvs_runtime::source::encode(&Source {
            file: "app/Handler.nvs".to_owned(),
            line: 118,
            member: Some("Handler::respond".to_owned()),
        });
        let at_scope = nvs_runtime::source::encode(&Source {
            file: "script.nvs".to_owned(),
            line: 3,
            member: None,
        });
        let member = from_source(Value::source_const(inside.as_ptr()));
        let scope = from_source(Value::source_const(at_scope.as_ptr()));
        assert!(
            member.contains(
                "\"source\":{\"file\":\"app/Handler.nvs\",\"line\":118,\
                 \"member\":\"Handler::respond\"}"
            ),
            "a record produced in a member names it: {member}"
        );
        assert!(
            scope.contains("\"source\":{\"file\":\"script.nvs\",\"line\":3}"),
            "and a script's own statement names the file and the line: {scope}"
        );
        assert!(
            !scope.contains("member"),
            "with no member key at all, rather than an empty one: {scope}"
        );
    }

    /// `rule:errors/a-record-names-where-it-was-produced`'s other end: a
    /// producer the compiler had no call site for is handed the zero word, and
    /// the record it writes has no `source` key rather than an empty one.
    ///
    /// The whole line, not a `contains`: an empty envelope key would print
    /// plausibly against any assertion made about the keys around it.
    #[test]
    fn a_producer_with_no_source_omits_the_field_rather_than_rendering_it_empty() {
        assert_eq!(
            from_source(no_source()),
            "{\"level\":\"error\",\"msg\":\"a message\"}\n",
            "the envelope's existing rule, applied to one more field"
        );
    }

    /// A trace id and parent span that are somebody else's, so that a
    /// continuation can be told from a fresh draw.
    ///
    /// W3C's own worked example, and both are what a `traceparent` carries:
    /// thirty-two and sixteen characters of lower-case hex, neither all-zero.
    const TRACE: &str = "4bf92f3577b34da6a3ce929d0e0e4736";
    /// [`TRACE`]'s parent span, on the same terms.
    const SPAN: &str = "00f067aa0ba902b7";

    /// A buffered context **answering a request**, which is the source
    /// [`Ctx::stamp_envelope`] reads `rule:errors/log-write`'s four request keys from.
    ///
    /// `traceparent` is the only way a trace becomes *active* today: `rule:observability/a-trace-id-exists-for-every-request`
    /// 's head-based `[trace] sample` is unbuilt, so a root's flag is always
    /// `false` and an inbound sampled header is the one thing that sets it —
    /// `nvs_runtime::trace_context`'s own *What is not here yet* owns that.
    /// `None` is therefore the ordinary served request, which has an id and no
    /// trace being recorded.
    fn serving(traceparent: Option<&str>) -> Ctx {
        let mut ctx = Ctx::buffered();
        let mut inbound = Inbound::new("GET", "/orders", "");
        if let Some(header) = traceparent {
            inbound.set_trace_context(TraceContext::continuing(Some(header), 0.0));
        }
        ctx.set_inbound(inbound);
        ctx
    }

    /// One `Core\Log::write` of `message` on `ctx`, as the line it wrote.
    fn written(ctx: &mut Ctx, message: &str) -> String {
        call(
            nvs_core_log_write,
            ctx,
            &[
                no_source(),
                Value::int(error_severity()),
                Value::str(NvsStr::new(message.as_bytes())),
                Value::array(NvsArray::new()),
            ],
        )
        .expect("a buffered sink is the one output that cannot fail");
        String::from_utf8(
            ctx.take_buffered_output()
                .expect("a buffered context hands its bytes back"),
        )
        .expect("a JSON Lines line is text")
    }

    /// That line as the object it is, parsed rather than matched on, so an
    /// assertion is about a key and not about where a comma fell.
    fn parsed(line: &str) -> serde_json::Value {
        serde_json::from_str(line.trim_end())
            .expect("`rule:errors/renderings` renders one JSON object per line")
    }

    /// The envelope keys that object carries, sorted.
    fn keys(line: &str) -> Vec<String> {
        let mut found: Vec<String> = parsed(line)
            .as_object()
            .expect("one record is one object")
            .keys()
            .cloned()
            .collect();
        found.sort();
        found
    }

    /// `rule:errors/log-write`'s `ts` and `request_id`, which a record written inside a
    /// request carries.
    ///
    /// Neither is *frozen*: a fixture pinning a timestamp or an id is a fixture
    /// that has to be rewritten every run. What is asserted is that the keys
    /// are there, that `ts` leads the line — `rule:errors/renderings`'s reading order — and
    /// that `request_id` is the context's **own** trace id rather than a second
    /// identifier this member drew for itself, which `rule:observability/a-trace-id-exists-for-every-request` forbids in as
    /// many words.
    #[test]
    fn a_record_written_inside_a_request_carries_ts_and_request_id() {
        let mut ctx = serving(None);
        let expected = ctx.trace_context().trace_id_hex();
        let line = written(&mut ctx, "the store said no");
        let doc = parsed(&line);

        assert!(
            line.starts_with("{\"ts\":\""),
            "§ 3 renders the envelope in reading order and `ts` leads it: {line}"
        );
        assert_eq!(expected.len(), 32, "a trace id is sixteen bytes as hex");
        assert_eq!(
            doc["request_id"].as_str(),
            Some(expected.as_str()),
            "`rule:observability/a-trace-id-exists-for-every-request` makes the trace id the only request identifier, so \
             the record names that one rather than minting its own"
        );
        assert!(
            doc["ts"].as_str().is_some_and(|ts| ts.ends_with('Z')),
            "§ 6 fixes RFC 3339 and the stamp renders in UTC: {line}"
        );
    }

    /// § 6's `trace_id`/`span_id`, asked on **both sides of the bound**: a
    /// request whose trace is being recorded carries them, and the ordinary
    /// request — which has an id and is not sampled — does not.
    ///
    /// A stamp that wrote them unconditionally passes the first half on its own
    /// and makes every line claim a span no backend was ever sent, which is
    /// exactly the thing § 6's "omitted rather than empty" is protecting.
    ///
    /// The trace id is compared against the *header* and not against the
    /// context, so this pins the continuation too: § 2 adopts an inbound
    /// trace's id, and a line naming a freshly drawn one could not be joined to
    /// the caller's trace at all. The span goes the other way — a record names
    /// the span it was **written in**, which is this request's own root and
    /// never the caller's, the two being kept apart by
    /// `nvs_runtime::TraceContext` — so what is asserted of it is both halves:
    /// this request's, and not the one that arrived.
    #[test]
    fn a_record_written_while_a_trace_is_active_carries_trace_id_and_span_id() {
        let header = format!("00-{TRACE}-{SPAN}-01");
        let mut sampled = serving(Some(&header));
        let doc = parsed(&written(&mut sampled, "the store said no"));
        assert_eq!(
            doc["trace_id"].as_str(),
            Some(TRACE),
            "§ 2 continues the trace the request arrived carrying"
        );
        let own = sampled.trace_context().span_id_hex();
        assert_eq!(
            doc["span_id"].as_str(),
            Some(own.as_str()),
            "and the span the record was written in"
        );
        assert_ne!(
            doc["span_id"].as_str(),
            Some(SPAN),
            "the record claimed the caller's span as its own"
        );
        assert_eq!(
            doc["request_id"].as_str(),
            Some(TRACE),
            "§ 2 has one identifier, so the request key repeats the trace id \
             rather than disagreeing with it"
        );

        let mut unsampled = serving(None);
        // Another message, so this is a record of its own rather than a second
        // occurrence of the one above: the request keys are exactly what
        // `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers` does not
        // let a window distinguish by, so two writes differing only in them are
        // one line.
        let quiet = keys(&written(&mut unsampled, "the queue is full"));
        assert!(
            !quiet
                .iter()
                .any(|key| key == "trace_id" || key == "span_id"),
            "§ 6 omits both while no trace is active: {quiet:?}"
        );
    }

    /// § 6's omission rule, asserted by **counting the keys three shapes
    /// produce** rather than by reading one of them: a record carries exactly
    /// the envelope keys its context has a source for, and an absent one is
    /// gone rather than written empty.
    ///
    /// Three contexts, because the rule only bites where the sources differ — a
    /// CLI run, a served request with no active trace, and a served request
    /// with one. A stamp that wrote `""` for what it did not have would print
    /// plausibly on any single line here and would fail every count; so would
    /// one that wrote `"fields":{}` for the empty bag each of the three passes.
    #[test]
    fn a_field_with_no_value_is_omitted_rather_than_empty() {
        let header = format!("00-{TRACE}-{SPAN}-01");
        let shapes: [(Ctx, &[&str]); 3] = [
            (Ctx::buffered(), &["level", "msg"]),
            (serving(None), &["level", "msg", "request_id", "ts"]),
            (
                serving(Some(&header)),
                &["level", "msg", "request_id", "span_id", "trace_id", "ts"],
            ),
        ];
        for (index, (mut ctx, expected)) in shapes.into_iter().enumerate() {
            // A message per shape, because what is being counted is the keys of
            // three separate records: three identical ones are one line and a
            // count, under `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`.
            let line = written(&mut ctx, &format!("the store said no {index}"));
            let found = keys(&line);
            let mut wanted: Vec<String> = expected.iter().map(|key| (*key).to_owned()).collect();
            wanted.sort();
            assert_eq!(
                found, wanted,
                "§ 6: the keys this context has a source for, and no others: {line}"
            );
            assert!(
                !line.contains("\"\""),
                "and nothing it lacks is written as an empty value: {line}"
            );
        }
    }

    /// The two-key envelope a CLI run still produces, which is what makes
    /// § 6's four request keys **additive**: no record that had a shape before
    /// them has a different one now.
    ///
    /// Byte for byte rather than by key, because this is also the shape
    /// [`application_code_and_the_engine_floor_produce_schema_identical_records`]
    /// compares the engine floor against. A `ts` read off the clock regardless
    /// of context would pass a key count here and would put the two writers of
    /// § 6 one key apart, which is the divergence that section forbids.
    #[test]
    fn a_cli_runs_record_is_still_level_and_msg_alone() {
        let mut ctx = Ctx::buffered();
        assert_eq!(
            written(&mut ctx, "the store said no"),
            "{\"level\":\"error\",\"msg\":\"the store said no\"}\n",
            "§ 6's four request keys have no source outside a request, and a \
             record that had none of them still has none"
        );
    }

    /// `rule:http-server/the-floor-cannot-fill-the-disk`'s two bounds on the floor, asked of the **sink** rather
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

        // `rule:http-server/the-floor-cannot-fill-the-disk`'s first bullet. Two records fit in a 256-byte file and
        // 64 of them do not, so the rotation is reached many times over and the
        // retention bound is what stops the target growing with the loop.
        const MAX_BYTES: u64 = 256;
        const KEEP: usize = 2;
        let (_dir, path) = scratch("floor.log");
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

    /// `rule:errors/engine-floor`'s directive, asked of both of `rule:errors/record-producers`'s writers at
    /// once: a deployment that names a destination gets **one** destination,
    /// and neither caller keeps a channel of its own beside it.
    ///
    /// Asserted as a destination the two of them *share* rather than as two
    /// readings of the directive, which is § 6's sameness one step past the
    /// record's shape. The two channels a target displaces are checked as well
    /// as the file that replaced them, because a writer that wrote to both
    /// would satisfy every assertion about the file alone while doubling every
    /// record a deployment collects.
    #[test]
    fn both_writers_land_in_the_target_the_deployment_named() {
        let (_dir, path) = scratch("named-target.log");
        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(OutputSink::Buffer(Vec::new()));
        ctx.set_config(crate::tests::granting(&format!(
            "[log]\ntarget = \"file:{}\"\n",
            path.display().to_string().replace('\\', "\\\\")
        )));

        call(
            nvs_core_log_write,
            &mut ctx,
            &[
                no_source(),
                Value::int(error_severity()),
                Value::str(NvsStr::new(b"the application said so")),
                Value::array(NvsArray::new()),
            ],
        )
        .expect("a configured file target is not a failure a program can cause");
        floor::report(&mut ctx, &floor::note(Level::Error, "the floor said so"));

        assert!(
            ctx.take_buffered_output()
                .expect("a buffered context hands its bytes back")
                .is_empty(),
            "a named target is where the record goes, not a second copy beside \
             the program's own output"
        );
        assert!(
            diagnostic(&mut ctx).is_empty(),
            "and the floor's own channel is displaced by it in the same way"
        );

        // Dropping the context closes the handle, so the assertions below read
        // what the sink actually committed rather than what it may still hold.
        drop(ctx);
        let written = std::fs::read_to_string(&path)
            .expect("the target the configuration named is the file on disk");
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "one line per record, from both writers, in the order they wrote: \
             {written}"
        );
        assert!(
            lines[0].contains("\"msg\":\"the application said so\"")
                && lines[1].contains("\"msg\":\"the floor said so\""),
            "and each is the record its own caller built: {written}"
        );
        assert!(
            lines
                .iter()
                .all(|line| line.starts_with("{\"level\":\"error\",")),
            "rendered by one serialiser at one destination: {written}"
        );
    }

    /// `rule:errors/log-level`'s last paragraph, asked of both writers at once: `[log]
    /// level` is the minimum level **written**, so a record quieter than it is
    /// written by neither caller and one at or above it by both.
    ///
    /// A sweep over the whole roster, asserted by *counting* rather than by
    /// reading one pair off: the failure this is written around is the floor
    /// implemented with `rule:errors/log-level`'s syslog severities the wrong way round,
    /// which inverts the entire table while still answering plausibly for the
    /// configured level itself — `Warn` under a `Warn` minimum is written
    /// either way. One row of this table cannot tell those apart and the table
    /// can.
    #[test]
    fn the_configured_minimum_is_the_floor_both_writers_write_over() {
        let (_dir, path) = scratch("minimum-level.log");
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(&format!(
            "[log]\ntarget = \"file:{}\"\nlevel = \"Warn\"\n",
            path.display().to_string().replace('\\', "\\\\")
        )));

        for level in Level::ALL {
            call(
                nvs_core_log_write,
                &mut ctx,
                &[
                    no_source(),
                    Value::int(level.syslog_severity().into()),
                    Value::str(NvsStr::new(level.name().as_bytes())),
                    Value::array(NvsArray::new()),
                ],
            )
            .expect("a record the floor drops is not a failure either");
            floor::report(&mut ctx, &floor::note(level, "the floor said so"));
        }

        // Dropping the context closes the handle, for the reason the test
        // above gives.
        drop(ctx);
        let written = std::fs::read_to_string(&path)
            .expect("the target the configuration named is the file on disk");
        for level in Level::ALL {
            let wanted = usize::from(level >= Level::Warn) * 2;
            let landed = written
                .lines()
                .filter(|line| line.starts_with(&format!("{{\"level\":\"{}\",", level.name())))
                .count();
            assert_eq!(
                landed,
                wanted,
                "under `level = \"Warn\"`, a `{}` record is written by {}: {written}",
                level.name(),
                if wanted == 0 {
                    "neither writer"
                } else {
                    "both writers"
                }
            );
        }
        assert_eq!(
            written.lines().count(),
            6,
            "three levels at or above the minimum, from two writers: {written}"
        );
    }

    /// A record below `[log] level` takes no slot in the coalescing table, so a
    /// run of them cannot push out the window of a record that is written.
    ///
    /// One `Error` opens a window, then more distinct `Debug` records than the
    /// table has slots arrive under `level = "Warn"`. The repeat of the `Error`
    /// is still inside its window and is swallowed, so the output is one line.
    /// A filtered record that took a slot would have evicted that window, and
    /// the repeat would be a second line.
    // covers: Core\Log::write
    #[test]
    fn a_record_below_the_minimum_takes_no_coalescing_slot() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting("[log]\nlevel = \"Warn\"\n"));
        let write = |ctx: &mut Ctx, level: Level, message: &str| {
            call(
                nvs_core_log_write,
                ctx,
                &[
                    no_source(),
                    Value::int(level.syslog_severity().into()),
                    Value::str(NvsStr::new(message.as_bytes())),
                    Value::array(NvsArray::new()),
                ],
            )
            .expect("a buffered sink is the one output that cannot fail");
        };

        write(&mut ctx, Level::Error, "the one that is written");
        for n in 0..floor::LOG_WINDOW_SLOTS * 2 {
            write(&mut ctx, Level::Debug, &format!("filtered {n}"));
        }
        write(&mut ctx, Level::Error, "the one that is written");

        let output = String::from_utf8(
            ctx.take_buffered_output()
                .expect("a buffered context hands its bytes back"),
        )
        .expect("a JSON Lines line is text");
        assert_eq!(
            output.lines().count(),
            1,
            "the repeat is inside its window, which no filtered record evicted: {output}"
        );
    }

    /// `rule:errors/renderings`'s second rendering, asked of both writers at once: under
    /// `[log] format = "text"` the target carries plaintext records and no JSON
    /// Lines at all, from the application's writer and from the floor alike.
    ///
    /// The *agreement* is the claim, not the layout — `nvs_render::plain`'s own
    /// tests own what a plaintext record looks like. What could break here is
    /// one writer rendering for itself: a producer that still called
    /// `nvs_render::json::line` would leave a file with one of each and pass
    /// every assertion made about the other one alone.
    #[test]
    fn both_writers_emit_the_rendering_the_deployment_configured() {
        let (_dir, path) = scratch("text-format.log");
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(&format!(
            "[log]\ntarget = \"file:{}\"\nformat = \"text\"\n",
            path.display().to_string().replace('\\', "\\\\")
        )));

        call(
            nvs_core_log_write,
            &mut ctx,
            &[
                no_source(),
                Value::int(Level::Error.syslog_severity().into()),
                Value::str(NvsStr::new(b"the application said so")),
                Value::array(NvsArray::new()),
            ],
        )
        .expect("a configured target is where the record goes");
        floor::report(&mut ctx, &floor::note(Level::Error, "the floor said so"));

        drop(ctx);
        let written = std::fs::read_to_string(&path)
            .expect("the target the configuration named is the file on disk");
        assert!(
            !written.contains("{\"level\""),
            "`format = \"text\"` selects one rendering for every writer: {written}"
        );
        let headers: Vec<&str> = written
            .lines()
            .filter(|line| line.starts_with('['))
            .collect();
        assert_eq!(
            headers,
            vec![
                "[error] the application said so",
                "[error] the floor said so"
            ],
            "both writers render through `nvs_render::plain`: {written}"
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

    /// A path in a scratch directory this case owns, and the guard that
    /// deletes the directory, with every rotation in it, when the case ends.
    fn scratch(name: &str) -> (nvs_repo::Scratch, std::path::PathBuf) {
        let dir = nvs_repo::scratch("log-rotation");
        let path = dir.join(name);
        (dir, path)
    }

    /// [`LEVEL`]'s rows and the record model's roster name the same five cases,
    /// and each row's integer is the severity the model reads it back by. These
    /// are the pair that would otherwise drift: the enum is what a program
    /// writes and the roster is what the record is rendered from, so a case in
    /// one alone is either surface with no rendering or a rendering nothing can
    /// reach.
    #[test]
    fn the_levels_and_their_tags_agree() {
        assert_eq!(
            LEVEL.cases.len(),
            5,
            "`rule:errors/log-level`'s roster is five cases"
        );
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
    /// syslog's `Notice` (5) and `Alert` (1) are the two `rule:errors/log-level` names as
    /// rejected, so they are the ones asked for here.
    #[test]
    fn an_undeclared_severity_is_not_a_case() {
        for absent in [0, 1, 5, 8, 255] {
            assert!(
                Level::from_syslog_severity(absent).is_none(),
                "{absent} is not one of `rule:errors/log-level`'s five severities"
            );
        }
    }
}
