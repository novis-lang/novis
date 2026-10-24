//! `Core\Debug` — `rule:errors/debug-dump`'s `dump` and `render`, and the walk that turns a runtime value into
//! `rule:errors/diagnostic-record`'s record.
//!
//! The probes' start and stop pairs and the coverage read-back are here too.
//! `rule:testing/debug-surface` is their whole contract, and [`start`] and
//! [`start_sink`] are the two doors every start goes through.
//!
//! ```text
//! Core\Debug::dump(mixed ...$values): void
//! Core\Debug::render(mixed $value): Core\Cli\Text
//! ```
//!
//! **There is no format argument on either, and there is no `dumpRaw`.** The
//! sink in force is the only input to which rendering runs
//! (`rule:errors/renderings`), which is
//! what keeps the five producers from each growing a `$format` parameter and
//! the renderings from becoming twenty. [`rendered_for`] is the whole of that
//! choice, asked of the channel the bytes leave through: a dump reaching a
//! terminal leaves through `nvs_runtime::Ctx::write_diagnostic` and a block
//! appended to a response leaves through the body, so each takes that
//! channel's own carrier and neither is a table this module states twice.
//!
//! # Where a dump lands
//!
//! `rule:errors/debug-dump`'s table, and [`nvs_core_debug_dump`] is the whole
//! of the routing.
//! Outside a request — a CLI program, a scheduled script, a job worker, a test
//! — a dump goes to **stderr**, never stdout, so `prog | jq` and
//! `prog > out.txt` keep working while a program is being debugged.
//! `nvs_runtime::Ctx::write_diagnostic` is that channel and owns why it is a
//! second sink rather than a fourth `OutputSink` variant. A dump is
//! deliberately *not* captured by `Core\Out::capture`: capturing one would
//! swallow the very output it was written to make visible.
//!
//! **Inside a request a dump is a log record**, at `Log::Level::Debug`, written
//! through `nvs_runtime::Ctx::write_log_record` like `rule:errors/record-producers`'
//! other writers — so `[log] target`, `[log] level` and `[log] format` answer
//! for it too, and a forgotten call in production is a line in whatever the
//! deployment already collects rather than an engine-side channel nobody reads.
//! With no target configured the fallback channel is the diagnostic one and not
//! the program's output, which is the difference between this producer and
//! `Core\Log::write`: a dump is something the engine writes *about* a program,
//! not something the program said.
//!
//! **`[debug] inline` adds the block, and changes nothing else.** With the
//! directive in force the same record is rendered a second time, for the body's
//! own carrier, and handed to `nvs_runtime::Ctx::append_inline_debug` — which
//! holds it until the request finishes and appends it there, and owns why the
//! two questions that placement turns on are answerable only at the end. The
//! record is written in both modes; the block is the whole of what the
//! directive buys. It is `RuntimeTighten` (`nvs_config::directive`), so a
//! request may turn its own inline output off and can never turn it on, which
//! leaves the run mode's default as the only thing that enables it — and a host
//! that wrote no configuration starts in `production`, where it is off.
//!
//! The HTTP rows of that table are the security half of the rule: PHP's
//! most-exploited information disclosure is not a bug in `var_dump`, it is
//! that `var_dump` writes to *output*. Here a forgotten call writes a log line
//! wherever it was made, and the one spelling that would put it in a response
//! body is a directive whose ceiling is closed by default and which no request
//! can open. A JSON body is never modified in either mode.
//!
//! # `render`, and why it is one member rather than a second mechanism
//!
//! `render` answers a carrier rather than a `string`, so a dump can be
//! *embedded* rather than written, by
//! `rule:security/capture-answers-the-carrier`
//! 's rule verbatim. Because it answers a carrier, `echo Core\Debug::render($x)`
//! is singly escaped: those bytes have already been through the record's own
//! transformations, and the carrier is what stops the next `echo` escaping
//! them again.
//!
//! **The carrier is `Core\Cli\Text` under every sink, and that is a bound
//! rather than a hole.** A member answering the sink's own carrier would have
//! to *declare* both of `rule:errors/renderings`' two, and which arm a call
//! answers is the channel's question rather than the program's:
//! [`rendered_for`] asks the sink in force, so a caller narrowing that union
//! with `is` (`rule:types/type-test`) would be branching on the deployment's
//! configuration and never on anything its own source says. Declaring one
//! class while answering the other is the worse trade, since it hands a
//! `Core\Html\Markup` to the `as string` conversion written for the terminal
//! carrier and bypasses `rule:core-classes/html-to-source`'s demand for a
//! reason. So the plaintext rendering is what `render` answers — one carrier a
//! caller can embed under either sink — and a program that wants the block in
//! a body turns on the directive above instead.
//!
//! **`render` is never coloured, and `dump` to a terminal is.** A dump knows
//! its stream, so [`diagnostic_colour`] reads standard error's depth from the
//! one terminal profile. A rendered value has no stream yet, and the same
//! bytes are also `rule:testing/inline-snapshots`' snapshot text, which must
//! not change with the terminal a test runs in.
//!
//! # The walk is not here
//!
//! What turns a runtime value into `rule:errors/diagnostic-record`'s nodes is
//! `nvs_runtime::record`, and `dump` is one of its callers. It sits there
//! because `nvs_runtime::floor::uncaught` is a producer too and runs where no
//! `Core` class may be depended on; that module's header owns which
//! transformations the walk applies, what it still cannot see, and why one walk
//! rather than two.

use nvs_config::capability::{Cap, Scope};
use nvs_render::plain::Colour;
use nvs_render::{Level, Node, Record, Source};
use nvs_runtime::{Ctx, DebugFlags, Fault, LogChannel, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class's fully-qualified name.
pub(crate) const NAME: &str = r"Core\Debug";

/// `Core\Debug`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Shows any value in a readable form while you debug a program. It can print the \
            value to stderr or return it as a string.",
};

/// `Core\Debug`: `rule:errors/debug-dump`'s `dump` and `render`, then
/// `rule:testing/debug-surface`'s start and stop pair per probe and the coverage read-back.
// rank: 450
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "dump",
            names: &["values"],
            params: &[CoreTy::Variadic(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_debug_dump",
            doc: Some(&DUMP_DOC),
        },
        CoreMethod {
            name: "render",
            names: &["value"],
            params: &[CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Instance(crate::cli::NAME),
            symbol: "nvs_core_debug_render",
            doc: Some(&RENDER_DOC),
        },
        CoreMethod {
            name: "startCoverage",
            names: &["branches"],
            params: &[CoreTy::Bool],
            defaults: &[Const::Bool(false)],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_debug_start_coverage",
            doc: Some(&START_COVERAGE_DOC),
        },
        CoreMethod {
            name: "stopCoverage",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_debug_stop_coverage",
            doc: Some(&STOP_COVERAGE_DOC),
        },
        CoreMethod {
            name: "coverage",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Array(&CoreTy::Int)),
            symbol: "nvs_core_debug_coverage",
            doc: Some(&COVERAGE_DOC),
        },
        CoreMethod {
            name: "startTrace",
            names: &["path"],
            params: &[CoreTy::Path(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_debug_start_trace",
            doc: Some(&START_TRACE_DOC),
        },
        CoreMethod {
            name: "stopTrace",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_debug_stop_trace",
            doc: Some(&STOP_TRACE_DOC),
        },
        CoreMethod {
            name: "startProfile",
            names: &["path"],
            params: &[CoreTy::Path(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_debug_start_profile",
            doc: Some(&START_PROFILE_DOC),
        },
        CoreMethod {
            name: "stopProfile",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_debug_stop_profile",
            doc: Some(&STOP_PROFILE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Debug::dump`'s reference card — `rule:core-api/reference-card`.
const DUMP_DOC: MethodDoc = MethodDoc {
    short: "Writes each value in a readable form, for debugging. It never writes to standard \
            output. Outside a web request, it writes to standard error, in colour when standard \
            error is a terminal that shows colour. \
            Inside a web request, it writes a log record at the `Debug` level. In production, \
            that record is not kept unless `[log] level` is `\"Debug\"`. With `[debug] inline`, \
            the dump is also added to an HTML response.",
    params: &[ParamDoc {
        name: "values",
        desc: "Any number of values. Control bytes and bidirectional text characters are shown \
               as visible text. A deep or long structure is shortened, and a cycle is shown as \
               a cycle. A `secret` property is hidden.",
        shape: &[],
    }],
    ret: "Nothing. A call with no arguments writes nothing. `Core\\Out::capture` never \
          captures a dump.",
    errors: &[],
};

/// `Core\Debug::render`'s reference card — `rule:core-api/reference-card`.
const RENDER_DOC: MethodDoc = MethodDoc {
    short: "Formats `$value` exactly as `dump` does, and returns it as a `Core\\Cli\\Text`. It \
            writes nothing. You can put the result into your own output, and it is escaped \
            only once.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to format. It is formatted the same way as a value given to `dump`.",
        shape: &[],
    }],
    ret: "The plain text as a `Core\\Cli\\Text`, wherever the output goes. It has no colour \
          and no newline at the end.",
    errors: &[],
};

/// The error a start throws when `[debug] mode` does not include its probe.
const NOT_IN_MODE: ErrorDoc = ErrorDoc {
    error: "RuntimeError",
    desc: "`[debug] mode` in `nvs.toml` does not include this probe. A server never runs a probe.",
};

/// `Core\Debug::startCoverage`'s reference card — `rule:core-api/reference-card`.
const START_COVERAGE_DOC: MethodDoc = MethodDoc {
    short: "Starts counting how many times each line runs. Read the counts with `coverage`. \
            `[debug] mode` must include `\"coverage\"`.",
    params: &[ParamDoc {
        name: "branches",
        desc: "When `true`, each side of every `if` and loop condition is counted too. \
               `[debug] mode` must then also include `\"branch\"`.",
        shape: &[],
    }],
    ret: "Nothing. Counting goes on until `stopCoverage`.",
    errors: &[NOT_IN_MODE],
};

/// `Core\Debug::stopCoverage`'s reference card — `rule:core-api/reference-card`.
const STOP_COVERAGE_DOC: MethodDoc = MethodDoc {
    short: "Stops counting lines and branches. The counts so far are kept.",
    params: &[],
    ret: "Nothing. It does nothing when coverage is not running.",
    errors: &[],
};

/// `Core\Debug::coverage`'s reference card — `rule:core-api/reference-card`.
const COVERAGE_DOC: MethodDoc = MethodDoc {
    short: "Returns how many times each line ran. The keys are file paths. Each value is an \
            array from a line number to its count.",
    params: &[],
    ret: "Only lines that ran are listed. Lines are counted when `nvs test` runs with a \
          coverage option. Otherwise the array is empty.",
    errors: &[],
};

/// `Core\Debug::startTrace`'s reference card — `rule:core-api/reference-card`.
const START_TRACE_DOC: MethodDoc = MethodDoc {
    short: "Starts writing a trace of every call to the file at `$path`. Each line of the file is \
            one JSON object. `[debug] mode` must include `\"trace\"`.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file to write. The capability `debug.trace` must grant this path. Permission \
               to write files is not enough.",
        shape: &[],
    }],
    ret: "Nothing. The trace goes on until `stopTrace`.",
    errors: &[
        NOT_IN_MODE,
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `debug.trace` for this path.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "A trace started by `startTrace` is already running.",
        },
    ],
};

/// `Core\Debug::stopTrace`'s reference card — `rule:core-api/reference-card`.
const STOP_TRACE_DOC: MethodDoc = MethodDoc {
    short: "Stops the trace and closes its file.",
    params: &[],
    ret: "Nothing. It does nothing when no trace is running.",
    errors: &[],
};

/// `Core\Debug::startProfile`'s reference card — `rule:core-api/reference-card`.
const START_PROFILE_DOC: MethodDoc = MethodDoc {
    short: "Starts timing every call, and writes the result to the file at `$path` in the \
            speedscope format. `[debug] mode` must include `\"profile\"`. Timing every call \
            adds a small cost to each one. So code with many small calls looks slower in the \
            profile than it is.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file to write. The capability `debug.profile` must grant this path. \
               Permission to write files is not enough.",
        shape: &[],
    }],
    ret: "Nothing. The profile goes on until `stopProfile`.",
    errors: &[
        NOT_IN_MODE,
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `debug.profile` for this path.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "A profile started by `startProfile` is already running.",
        },
    ],
};

/// `Core\Debug::stopProfile`'s reference card — `rule:core-api/reference-card`.
const STOP_PROFILE_DOC: MethodDoc = MethodDoc {
    short: "Stops timing calls and closes the profile's file.",
    params: &[],
    ret: "Nothing. It does nothing when no profile is running.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_debug_dump" => (nvs_core_debug_dump as *const ()).cast(),
        "nvs_core_debug_render" => (nvs_core_debug_render as *const ()).cast(),
        "nvs_core_debug_start_coverage" => (nvs_core_debug_start_coverage as *const ()).cast(),
        "nvs_core_debug_stop_coverage" => (nvs_core_debug_stop_coverage as *const ()).cast(),
        "nvs_core_debug_coverage" => (nvs_core_debug_coverage as *const ()).cast(),
        "nvs_core_debug_start_trace" => (nvs_core_debug_start_trace as *const ()).cast(),
        "nvs_core_debug_stop_trace" => (nvs_core_debug_stop_trace as *const ()).cast(),
        "nvs_core_debug_start_profile" => (nvs_core_debug_start_profile as *const ()).cast(),
        "nvs_core_debug_stop_profile" => (nvs_core_debug_stop_profile as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::dump(mixed ...$values): void` — one node per argument,
    /// written to the diagnostic channel.
    ///
    /// The tail arrives as one `array` argument holding the arguments under
    /// `"0"`, `"1"`, … — [`crate::registry::CoreTy::Variadic`] owns why — and
    /// ahead of it is where this call was written, which
    /// [`crate::registry::SOURCE_MEMBERS`] puts in argument 0 and owns the
    /// position of.
    ///
    /// A dump with no arguments at all writes nothing rather than an empty
    /// line: `Core\Debug::dump()` says nothing, and a blank line in a build
    /// log is worse than silence.
    ///
    /// The two destinations below are `rule:errors/debug-dump`'s table and the
    /// module doc is where
    /// each row's reasoning is. What decides between them is whether this
    /// context is answering a request, asked the same way
    /// `Ctx::stamp_envelope` asks it — the request and not the sink, so that a
    /// `spawn script` child inside one, which takes its parent's carrier and
    /// answers no response of its own, is still a program whose dumps belong
    /// in that request's log.
    fn nvs_core_debug_dump(ctx, args: [2]) {
        #[expect(
            unsafe_code,
            reason = "the carrier came out of a `SourceConst` the compiled unit baked into its own data section, which outlives every request served from it"
        )]
        let source = unsafe { nvs_runtime::source::of_operand(args[0]) };
        let mut record = record_of(source, &args[1])?;
        if record.nodes.is_empty() {
            return Ok(Value::null());
        }
        if ctx.inbound().is_none() {
            let rendered = rendered_for(
                ctx.diagnostic_carrier(),
                &record.nodes,
                diagnostic_colour(ctx),
            );
            // Unreachable from source: the only diagnostic sink that can fail
            // is `OutputSink::Stderr` — `Buffer` and `Sink` never do, which
            // `Ctx::write_diagnostic`'s `# Errors` states — and nothing in the
            // language moves the channel or closes the descriptor. Only the
            // host can, by breaking the stderr it handed the process.
            ctx.write_diagnostic(rendered.as_bytes())
                .map_err(|e| Fault::fatal(format!("Core\\Debug::dump could not write: {e}")))?;
            return Ok(Value::null());
        }
        // Rendered for `Ctx::carrier` and not for the diagnostic channel's:
        // these bytes leave through the response body, so that is the sink in
        // force for them. The nodes and never the envelope, which is what makes
        // this the same rendering the terminal row gets rather than a third one
        // — `rule:errors/renderings` gives the envelope's keys to the log
        // target, and the record below is where they go.
        if inline(ctx) {
            let block = rendered_for(ctx.carrier(), &record.nodes, Colour::None);
            ctx.append_inline_debug(&block);
        }
        ctx.stamp_envelope(&mut record.envelope);
        // Swallowed rather than raised, which is `rule:errors/engine-floor`'s
        // answer for every record write: a log target that could not be written
        // is not a reason to fail the program that was being debugged, and
        // `Core\Log::write` beside it makes the same call.
        drop(ctx.write_log_record(&record, LogChannel::Diagnostic));
        Ok(Value::null())
    }
}

/// Whether `[debug] inline` is in force — `rule:errors/debug-dump`'s two HTML
/// rows, and the only input to this module that is not a value being dumped.
///
/// Read **in force** rather than off the snapshot, unlike `[debug]
/// keep_temporary` beside it in `nvs_runtime::sweep`: that key is `System` and
/// has no in-language setter, while this one is `RuntimeTighten`, so a request
/// that turned its own inline output off has written an overlay entry that only
/// `nvs_config::Request::get` sees. A context carrying no configuration at all
/// — every `.nvst` case, every unit test — answers `false`, which is the
/// fail-closed direction and the same one an unconfigured host gets.
fn inline(ctx: &Ctx) -> bool {
    ctx.config()
        .and_then(|config| config.get("debug.inline"))
        .is_some_and(|value| value == "true")
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::render(mixed $value): Core\Cli\Text` — the same record,
    /// answered as the sink's carrier instead of written.
    ///
    /// `rule:security/capture-answers-the-carrier`'s rule verbatim: bytes that have been through a sink
    /// cannot be handed back as a `string` without the next `echo` escaping
    /// them a second time, so this answers what `Core\Out::capture` answers.
    ///
    /// The terminal carrier under every sink, which the module doc states as a
    /// bound: the answer's class is a *declared* type, and the two carriers
    /// have no spelling a program could tell apart.
    fn nvs_core_debug_render(_ctx, args: [1]) {
        Ok(crate::cli::built(Value::text(&rendered(args[0]))))
    }
}

/// Turns `bits` on under the ceiling `[debug] mode` set, or throws the `RuntimeError` naming the
/// first probe it lacks — `rule:testing/debug-mode-directive`'s *narrow and never widen*.
///
/// A `RuntimeError` and not a `LogicError`, for the reason a capability denial is one
/// (`rule:security/denial-is-a-runtime-error`): the deployment said no, and a program that runs on
/// without the probe is a reasonable program.
fn start(ctx: &mut Ctx, bits: DebugFlags, member: &str) -> Result<(), Fault> {
    ctx.start_probes(bits).map_err(|missing| {
        let name = PROBES
            .iter()
            .find(|(bit, _)| missing.contains(*bit))
            .map_or("coverage", |(_, name)| name);
        Fault::thrown(format!(
            "Core\\Debug::{member} cannot start the `{name}` probe, because `[debug] mode` does \
             not include it.\nhelp: add `\"{name}\"` to `mode` under `[debug]` in nvs.toml. A \
             server never runs a probe."
        ))
    })
}

/// Each probe bit a program starts, under the name `[debug] mode` writes it with
/// (`nvs_config::debug::NAMES`).
const PROBES: [(DebugFlags, &str); 4] = [
    (DebugFlags::COVERAGE, "coverage"),
    (DebugFlags::BRANCH, "branch"),
    (DebugFlags::TRACE, "trace"),
    (DebugFlags::PROFILE, "profile"),
];

/// `startTrace` and `startProfile`'s shared door: the probe under the ceiling, the path under its
/// own grant, and no second start while a sink is named. The grant is asked first, so a path
/// outside it throws whatever the mode says.
///
/// **What it spends:** one path per running sink, until its stop.
fn start_sink(
    ctx: &mut Ctx,
    path: &Value,
    probe: (DebugFlags, Cap),
    member: &str,
) -> Result<std::path::PathBuf, Fault> {
    let (bit, cap) = probe;
    // Unreachable from source: the parameter is `CoreTy::Path`, a `string`.
    let text = path.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Debug::{member} expected {:?} for its path, got tag {}",
            Tag::Str,
            path.tag_byte()
        ))
    })?;
    let path = std::path::Path::new(text);
    nvs_runtime::capability::require(
        ctx,
        cap,
        Scope::Path(path),
        &format!("Core\\Debug::{member}"),
    )?;
    let running = if bit == DebugFlags::TRACE {
        ctx.trace_sink().is_some()
    } else {
        ctx.profile_sink().is_some()
    };
    if running {
        let (what, stop) = if bit == DebugFlags::TRACE {
            ("trace", "stopTrace")
        } else {
            ("profile", "stopProfile")
        };
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Debug::{member} cannot start a {what}, because one is already running. \
                 Call `Core\\Debug::{stop}` first."
            ),
        ));
    }
    start(ctx, bit, member)?;
    Ok(path.to_path_buf())
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::startCoverage(bool $branches = false): void` — the line probe, and the edge
    /// probe beside it when `$branches` is `true`, under the ceiling.
    fn nvs_core_debug_start_coverage(ctx, args: [1]) {
        let bits = if args[0].as_bool() == Some(true) {
            DebugFlags::COVERAGE | DebugFlags::BRANCH
        } else {
            DebugFlags::COVERAGE
        };
        start(ctx, bits, "startCoverage")?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::stopCoverage(): void` — both coverage bits off, the table untouched.
    fn nvs_core_debug_stop_coverage(ctx, args: [0]) {
        ctx.stop_probes(DebugFlags::COVERAGE | DebugFlags::BRANCH);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::coverage(): array<array<int>>` — `nvs_runtime::StmtHits::line_counts` as a
    /// value, path to line to count.
    ///
    /// The table is the host's, so this reads it whether or not a probe is on now. Lines a
    /// statement ran on and none other, so the answer grows with the code a request touched and
    /// never with the program's size.
    fn nvs_core_debug_coverage(ctx, args: [0]) {
        let mut files = NvsArray::new();
        if let Some(hits) = ctx.stmt_hits() {
            for (path, lines) in hits.line_counts() {
                let mut counts = NvsArray::new();
                for (line, count) in lines {
                    counts.set_index(i64::from(line), Value::int(i64::try_from(count).unwrap_or(i64::MAX)));
                }
                files.set(NvsStr::new(path.as_bytes()), Value::array(counts));
            }
        }
        Ok(Value::array(files))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::startTrace(string $path): void` — the call probe on, and the file its calls
    /// stream to created. A file that cannot be created turns the probe back off and throws.
    fn nvs_core_debug_start_trace(ctx, args: [1]) {
        let path = start_sink(ctx, &args[0], (DebugFlags::TRACE, Cap::DebugTrace), "startTrace")?;
        match nvs_runtime::TraceSink::create(&path) {
            Ok(sink) => {
                ctx.set_trace_sink(Some(sink));
                Ok(Value::null())
            }
            Err(err) => {
                ctx.stop_probes(DebugFlags::TRACE);
                Err(Fault::thrown(format!(
                    "Core\\Debug::startTrace cannot create `{}`: {err}.",
                    path.display()
                )))
            }
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::stopTrace(): void` — the call probe off and its file closed. The probe is off
    /// even when the close fails, and the failure throws, because the file is missing calls.
    fn nvs_core_debug_stop_trace(ctx, args: [0]) {
        ctx.stop_probes(DebugFlags::TRACE);
        let Some(sink) = ctx.set_trace_sink(None) else {
            return Ok(Value::null());
        };
        let path = sink.path().to_path_buf();
        sink.finish().map_err(|err| {
            Fault::thrown(format!(
                "Core\\Debug::stopTrace could not write every call to `{}`: {err}.",
                path.display()
            ))
        })?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::startProfile(string $path): void` — the timing probe on, its sink named.
    fn nvs_core_debug_start_profile(ctx, args: [1]) {
        let path = start_sink(ctx, &args[0], (DebugFlags::PROFILE, Cap::DebugProfile), "startProfile")?;
        ctx.set_profile_sink(Some(path));
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::stopProfile(): void` — the timing probe off and its sink closed.
    fn nvs_core_debug_stop_profile(ctx, args: [0]) {
        ctx.stop_probes(DebugFlags::PROFILE);
        ctx.set_profile_sink(None);
        Ok(Value::null())
    }
}

/// One value as a node at a record's root, under the default caps.
///
/// The entry point for a producer outside this module — [`crate::log`]'s
/// `fields` bag is walked through here — so that a value written as a log field
/// and the same value dumped are the same node, with
/// `rule:errors/record-transformations`' transformations
/// applied once and in one place. Everything else about the walk, including
/// what it cannot see, is `nvs_runtime::record`'s.
pub(crate) fn node(value: Value) -> Node {
    nvs_runtime::record::node(value)
}

/// `rule:errors/record-producers`'s *"a record at `Debug`, one node per argument"* — the whole
/// of what `dump` produces.
///
/// `source` is where the call was written, in the argument order the ABI hands
/// it over in: on the envelope rather than among the nodes, because
/// `rule:errors/a-record-names-where-it-was-produced` makes it a property of the
/// record and not one of the values dumped. `None` is the producer with no call
/// site, whose field every rendering omits.
fn record_of(source: Option<Source>, tail: &Value) -> Result<Record, Fault> {
    let mut record = Record::at(Level::Debug);
    record.envelope.source = source;
    // Unreachable from source: `dump`'s one parameter is `CoreTy::Variadic`,
    // so `nvs_ir::lower::lower_call_args` builds this `array<mixed>` rather
    // than any source expression supplying it — the same judgement as
    // `crate::path`'s `join`, which states it in full.
    let array = tail.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Debug::dump expected {:?} for its arguments, got tag {}",
            Tag::Array,
            tail.tag_byte()
        ))
    })?;
    let values = crate::arr::borrowed(array);
    let mut from = 0usize;
    while let Some(slot) = values.next_slot(from) {
        // Unreachable from source: this is `next_slot`/`value_at`'s shared
        // post-condition rather than a boundary. Both answer off the same
        // predicate in both shapes — `slot < values.len()` packed, `hashed.at`
        // hashed — and nothing runs between the two calls that could shorten
        // the array, so a slot the first answered is one the second has. The
        // same pair, and the same judgement, in `crate::path`'s `join`.
        let value = values.value_at(slot).ok_or_else(|| {
            Fault::fatal("Core\\Debug::dump read an empty slot the array reported as live")
        })?;
        record.nodes.push(node(value));
        from = slot + 1;
    }
    Ok(record)
}

/// `nodes` in the rendering `carrier`'s sink takes — `rule:errors/renderings`'
/// table as a function of the carrier alone.
///
/// The carrier rather than the sink, because that is the one form in which
/// every channel already answers the question — `nvs_runtime::Ctx::carrier`
/// for what `echo` writes to, `Ctx::diagnostic_carrier` for what a dump writes
/// to — so a second caller needs no second table. Every carrier that is not the
/// HTML one is a terminal or a stand-in for one, so the plaintext arm is the
/// fail-closed direction and a sink added later keeps it by saying nothing.
///
/// `colour` reaches the plaintext arm alone: the HTML rendering is styled by
/// classes, never by escape sequences.
fn rendered_for(carrier: &str, nodes: &[Node], colour: Colour) -> String {
    if carrier == nvs_runtime::CARRIER_HTML_MARKUP {
        nvs_render::html::render_nodes(nodes)
    } else {
        nvs_render::plain::render_nodes_in(nodes, colour)
    }
}

/// Whether a dump written to the diagnostic channel is coloured —
/// `rule:errors/renderings`' *"coloured iff the terminal has colour"*.
///
/// The answer is standard error's depth in the one profile
/// `rule:tooling/the-terminal-profile-resolves-once` resolves, read through
/// [`crate::cli::depth_of`] so `Core\Cli` and a dump cannot disagree about it.
/// A channel that is not standard error shows no colour.
fn diagnostic_colour(ctx: &Ctx) -> Colour {
    let depth = crate::cli::depth_of(nvs_runtime::terminal::Stream::Err);
    if ctx.diagnostic_is_stderr() && depth != nvs_runtime::terminal::ColorDepth::None {
        Colour::Ansi
    } else {
        Colour::None
    }
}

/// One value as `Core\Debug::render` answers it at a terminal: the canonical,
/// ordered, `secret`-redacting text of the whole value, with no trailing
/// newline.
///
/// Public to the crate because
/// `rule:testing/inline-snapshots`'s inline snapshot is *this* rendering held in a string literal in the source —
/// `Core\Test::assertMatchesInline` calls it rather than growing one of its
/// own, so a snapshot and a dump of the same value cannot disagree about what
/// that value looks like, and `rule:errors/record-transformations`' redaction
/// reaches a snapshot for free.
///
/// Without the trailing newline `dump` writes: this answers a *value*, and a
/// caller composing one into a larger output decides where the line ends.
/// `dump` is the one that writes a line.
pub(crate) fn rendered(value: Value) -> String {
    let node = node(value);
    nvs_render::plain::render_nodes(std::slice::from_ref(&node))
        .trim_end_matches('\n')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two rows `rule:errors/debug-dump` declares, as the registry states them — what
    /// `nvs-types` seeds and what `nvs_ir::lower_call_args` flattens against.
    // covers: Core\Debug::render
    #[test]
    fn dump_is_variadic_and_render_answers_the_carrier() {
        let dump = CLASS.methods[0];
        assert_eq!(dump.name, "dump");
        assert!(matches!(dump.params[0], CoreTy::Variadic(&CoreTy::Mixed)));
        assert!(matches!(dump.return_ty, CoreTy::Void));
        let render = CLASS.methods[1];
        assert_eq!(render.name, "render");
        assert!(
            matches!(
                render.return_ty,
                CoreTy::Instance(name) if name == crate::cli::NAME
            ),
            "`render` answers one class under every sink, which the module doc \
             states as a bound: the arm of the union the other reading needs is \
             the channel's question rather than the program's"
        );
    }

    /// **There is no format argument on either member** —
    /// `rule:errors/renderings`. The sink in
    /// force is the only input to which rendering runs, which is what keeps
    /// the renderings from becoming twenty.
    // covers: Core\Debug::dump
    // covers: Core\Debug::render
    #[test]
    fn neither_member_takes_a_format_argument() {
        for method in CLASS.methods {
            assert!(
                !method
                    .params
                    .iter()
                    .any(|param| matches!(param, CoreTy::Options(_))),
                "`{}` grew an options bag, and `rule:errors/renderings` has no room for one",
                method.name
            );
        }
    }

    /// `rule:errors/diagnostic-record`: `Core\Debug::dump` writes to
    /// **stderr** in a CLI program, and standard output stays byte-empty.
    ///
    /// The assertion is made here rather than in a `.nvst` case because
    /// `--EXPECT-ERROR--` is also what tells the runner a case is expected to
    /// *fail*, so the suite has no way to read standard error off a successful
    /// run. What the conformance case asserts instead is the other half — that
    /// a dump puts nothing on standard output — and the two together are the
    /// bullet.
    // covers: Core\Debug::dump
    #[test]
    fn a_dump_writes_to_the_diagnostic_channel_and_not_to_the_output() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_diagnostic_sink(nvs_runtime::OutputSink::Buffer(Vec::new()));
        let mut tail = nvs_runtime::NvsArray::new();
        tail.append(Value::int(7));
        let tail = Value::array(tail);
        nvs_runtime::call(
            nvs_core_debug_dump,
            &mut ctx,
            &[Value::source_const(std::ptr::null()), tail],
        )
        .expect("a dump cannot fail");
        assert_eq!(
            ctx.take_buffered_diagnostic().as_deref(),
            Some(b"int(7)\n".as_slice())
        );
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(b"".as_slice()));
        #[expect(
            unsafe_code,
            reason = "this frame built the array and still owns the only \
                      reference to it; the helper borrowed it"
        )]
        unsafe {
            tail.release();
        }
    }

    /// `rule:errors/renderings`: a dump renders for the channel its own bytes
    /// leave through, so the same call writes the collapsible block once the
    /// diagnostic channel is the HTML sink and the plaintext line everywhere
    /// else.
    ///
    /// Asserted on the block's frame rather than on the whole document,
    /// because what the nodes inside it look like is `nvs_render::html`'s own
    /// test and repeating it here would make one rendering change two files.
    // covers: Core\Debug::dump
    #[test]
    fn a_dump_renders_for_the_channel_it_writes_to() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_diagnostic_sink(nvs_runtime::OutputSink::Body(Vec::new()));
        let mut tail = nvs_runtime::NvsArray::new();
        tail.append(Value::int(7));
        let tail = Value::array(tail);
        nvs_runtime::call(
            nvs_core_debug_dump,
            &mut ctx,
            &[Value::source_const(std::ptr::null()), tail],
        )
        .expect("a dump cannot fail");
        let written = ctx
            .take_buffered_diagnostic()
            .expect("the HTML sink buffers what is written to it");
        let written = String::from_utf8(written).expect("the HTML rendering is text");
        assert!(
            written.starts_with("<div class=\"nvs-nodes\">") && written.ends_with("</div>"),
            "the HTML sink takes the collapsible block, not the plaintext line: {written}"
        );
        assert!(
            written.contains('7'),
            "and it is still the dumped value inside it: {written}"
        );
        #[expect(
            unsafe_code,
            reason = "this frame built the array and still owns the only \
                      reference to it; the helper borrowed it"
        )]
        unsafe {
            tail.release();
        }
    }

    /// A context answering an HTTP request and writing a response body, under
    /// the configuration `written` states.
    ///
    /// The three HTTP rows of `rule:errors/debug-dump`'s table are read against
    /// this and the row above them against [`nvs_runtime::Ctx::buffered`], which
    /// is the whole difference the routing turns on. The diagnostic channel
    /// buffers so that the record a request writes with no `[log] target`
    /// configured is readable at all.
    fn requesting(written: &str) -> nvs_runtime::Ctx {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Body(Vec::new()));
        ctx.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
        ctx.set_config(crate::tests::granting(written));
        ctx.set_diagnostic_sink(nvs_runtime::OutputSink::Buffer(Vec::new()));
        ctx
    }

    /// One `Core\Debug::dump(7)` made in `ctx`, tail array and release included.
    fn dump_in(ctx: &mut nvs_runtime::Ctx) {
        let mut tail = nvs_runtime::NvsArray::new();
        tail.append(Value::int(7));
        let tail = Value::array(tail);
        nvs_runtime::call(
            nvs_core_debug_dump,
            ctx,
            &[Value::source_const(std::ptr::null()), tail],
        )
        .expect("a dump cannot fail");
        #[expect(
            unsafe_code,
            reason = "this frame built the array and still owns the only \
                      reference to it; the helper borrowed it"
        )]
        unsafe {
            tail.release();
        }
    }

    /// `rule:errors/debug-dump`'s HTTP rows: inside a request a dump is a
    /// **record** at `Debug`, not the plaintext line the row above it writes,
    /// and with `[debug] inline` unset nothing reaches the body at all. The
    /// case writes `[log] level = "Debug"`, because the production default
    /// `Info` drops a `Debug` record.
    ///
    /// Asserted on the rendering's frame rather than on the whole line, because
    /// which envelope keys a record carries is `nvs_render::json`'s own test and
    /// `Ctx::write_log_record`'s; what is this module's is that the dump went
    /// through them.
    // covers: Core\Debug::dump
    #[test]
    fn a_dump_inside_a_request_is_a_record_and_not_a_line() {
        let mut ctx = requesting("[log]\nlevel = \"Debug\"\n");
        dump_in(&mut ctx);
        let written = ctx
            .take_buffered_diagnostic()
            .expect("the diagnostic channel buffers");
        let written = String::from_utf8(written).expect("a JSON Lines record is text");
        assert!(
            written.starts_with('{') && written.contains(r#""level":"debug""#),
            "a dump in a request is one record at `Debug`: {written}"
        );
        assert!(
            written.contains('7'),
            "and the dumped value is still its node: {written}"
        );
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(b"".as_slice()),
            "with the directive unset the record is all there is"
        );
    }

    /// `[debug] inline`'s two rows, both halves: the block reaches an HTML body
    /// and never a JSON one, and it reaches it when the request *finishes*
    /// rather than where the dump was written.
    ///
    /// The JSON leg is the security half — an endpoint answers the same shape
    /// in development as in production, so a client's strict validator cannot
    /// pass against one and fail against the other — and it is asserted through
    /// the same flush as the HTML leg, a declaration being the only thing
    /// between them.
    #[test]
    fn debug_inline_appends_a_block_to_an_html_body_and_never_to_a_json_one() {
        let mut html = requesting("[debug]\ninline = true\n");
        dump_in(&mut html);
        assert_eq!(
            html.take_buffered_output().as_deref(),
            Some(b"".as_slice()),
            "nothing is in the body while there is still markup to write"
        );
        html.flush_inline_debug().expect("a body sink never fails");
        let body =
            String::from_utf8(html.take_buffered_output().unwrap_or_default()).expect("markup");
        let nonce = html
            .take_style_nonce()
            .expect("a block brings its style's nonce");
        let block = body
            .strip_prefix(&nvs_render::html::style_element(&nonce))
            .expect("the style element comes first");
        assert!(
            block.starts_with("<div class=\"nvs-nodes\">") && block.contains('7'),
            "the body takes the collapsible block, rendered for its own carrier: {body}"
        );

        let mut json = requesting("[debug]\ninline = true\n");
        dump_in(&mut json);
        json.declare_content_type("application/json");
        json.flush_inline_debug().expect("a body sink never fails");
        assert_eq!(
            json.take_buffered_output().as_deref(),
            Some(b"".as_slice()),
            "a JSON body is never modified, in either mode"
        );
    }

    /// `rule:errors/a-record-names-where-it-was-produced`: the record a dump
    /// produces names the file, line and member its own call was written at,
    /// read off the constant [`crate::registry::SOURCE_MEMBERS`] puts in
    /// argument 0.
    ///
    /// Asserted on the record rather than on what the channel shows, because
    /// `rule:errors/debug-dump` sends a dump's *nodes* to the terminal and
    /// renders no envelope there. The datum is on the record for the renderings
    /// that do carry one — `nvs_render::json` writes every envelope key — and
    /// putting it anywhere else would be the second construction
    /// `rule:errors/diagnostic-record` refuses.
    // covers: Core\Debug::dump
    #[test]
    fn a_dump_reports_the_line_it_was_written_on() {
        let written_at = Source {
            file: "app/Main.nvs".to_owned(),
            line: 42,
            member: Some("Main::main".to_owned()),
        };
        let blob = nvs_runtime::source::encode(&written_at);
        #[expect(
            unsafe_code,
            reason = "this frame baked the blob and it outlives the read"
        )]
        let source = unsafe { nvs_runtime::source::of_operand(Value::source_const(blob.as_ptr())) };
        let mut tail = nvs_runtime::NvsArray::new();
        tail.append(Value::int(7));
        let tail = Value::array(tail);
        let record = record_of(source, &tail).expect("a variadic tail is an array");
        assert_eq!(
            record.envelope.source,
            Some(written_at),
            "one datum, carried from the call site to the envelope unchanged"
        );
        assert_eq!(
            record.nodes.len(),
            1,
            "and the value dumped is still the record's one node"
        );
        #[expect(
            unsafe_code,
            reason = "this frame built the array and still owns the only \
                      reference to it; the record borrowed it"
        )]
        unsafe {
            tail.release();
        }
    }

    /// A command-line context under `written`, whose `[debug] mode` is the ceiling.
    fn probing(written: &str) -> nvs_runtime::Ctx {
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_config(crate::tests::granting(written));
        ctx
    }

    /// `startTrace($path)` in `ctx`: the message it threw, or `None` when it started.
    fn start_trace_at(ctx: &mut nvs_runtime::Ctx, path: &std::path::Path) -> Option<String> {
        let args = [Value::text(&path.to_string_lossy())];
        let answered = nvs_runtime::call(nvs_core_debug_start_trace, ctx, &args);
        #[expect(unsafe_code, reason = "the list holds the one reference it built")]
        unsafe {
            args[0].release();
        }
        answered
            .err()
            .map(|_| ctx.take_pending().unwrap_or_default().into_owned())
    }

    /// `rule:testing/debug-surface`: a start and its stop bracket the calls between them. The
    /// probe is on and the sink named from `startTrace` to `stopTrace`, and both are gone after.
    // covers: Core\Debug::startTrace
    // covers: Core\Debug::stopTrace
    #[cfg_attr(
        miri,
        ignore = "canonicalises a scratch directory, which Miri does not emulate"
    )]
    #[test]
    fn a_trace_starts_and_stops_around_the_calls_between() {
        let dir = nvs_repo::scratch("debug-trace");
        let mut ctx = probing(&format!(
            "[debug]\nmode = []\n\n[capabilities.debug]\ntrace = ['{}']\n",
            dir.display()
        ));
        assert!(
            start_trace_at(&mut ctx, &dir.join("calls.jsonl")).is_some(),
            "an empty mode has no `trace` to start"
        );

        let mut ctx = probing(&format!(
            "[debug]\nmode = [\"trace\"]\n\n[capabilities.debug]\ntrace = ['{}']\n",
            dir.display()
        ));
        nvs_runtime::call(nvs_core_debug_stop_trace, &mut ctx, &[]).expect("a stop never throws");
        assert!(
            !ctx.debug_flags().contains(DebugFlags::TRACE),
            "a stop narrows the mode's default"
        );
        let path = dir.join("calls.jsonl");
        assert_eq!(
            start_trace_at(&mut ctx, &path),
            None,
            "the mode allows `trace`"
        );
        assert!(ctx.debug_flags().contains(DebugFlags::TRACE));
        assert_eq!(ctx.trace_sink(), Some(path.as_path()));
        let again = start_trace_at(&mut ctx, &path).expect("one trace at a time");
        assert!(again.contains("stopTrace"), "{again}");
        nvs_runtime::call(nvs_core_debug_stop_trace, &mut ctx, &[]).expect("a stop never throws");
        assert!(!ctx.debug_flags().contains(DebugFlags::TRACE));
        assert_eq!(ctx.trace_sink(), None);
    }

    /// `rule:testing/debug-probes`: writing a trace is its own grant, so a path outside
    /// `debug.trace` throws even with `fs.write` granted everywhere, and the probe stays off.
    // covers: Core\Debug::startTrace
    #[cfg_attr(
        miri,
        ignore = "canonicalises a scratch directory, which Miri does not emulate"
    )]
    #[test]
    fn a_trace_path_outside_its_grant_throws() {
        let dir = nvs_repo::scratch("debug-ungranted");
        let mut ctx = probing("[debug]\nmode = [\"trace\"]\n\n[capabilities.fs]\nwrite = true\n");
        nvs_runtime::call(nvs_core_debug_stop_trace, &mut ctx, &[]).expect("a stop never throws");
        let refused = start_trace_at(&mut ctx, &dir.join("calls.jsonl"))
            .expect("`fs.write` is not `debug.trace`");
        assert!(refused.contains("debug.trace"), "{refused}");
        assert!(!ctx.debug_flags().contains(DebugFlags::TRACE));
        assert_eq!(ctx.trace_sink(), None);
    }

    /// `rule:testing/debug-mode-directive`: a program narrows `[debug] mode` and never widens it.
    /// `coverage` alone in the mode lets coverage start, and neither its branch half nor a profile.
    // covers: Core\Debug::startCoverage
    // covers: Core\Debug::stopCoverage
    // covers: Core\Debug::startProfile
    #[test]
    fn a_program_cannot_widen_debug_mode() {
        let mut ctx = probing("[debug]\nmode = [\"coverage\"]\n");
        nvs_runtime::call(nvs_core_debug_stop_coverage, &mut ctx, &[])
            .expect("a stop never throws");
        assert!(ctx.debug_flags().is_empty());
        nvs_runtime::call(
            nvs_core_debug_start_coverage,
            &mut ctx,
            &[Value::bool(false)],
        )
        .expect("the mode allows `coverage`");
        assert_eq!(ctx.debug_flags(), DebugFlags::COVERAGE);

        let branches = nvs_runtime::call(
            nvs_core_debug_start_coverage,
            &mut ctx,
            &[Value::bool(true)],
        );
        assert!(branches.is_err(), "`branch` is not in the mode");
        let refused = ctx.take_pending().unwrap_or_default();
        assert!(
            refused.contains("`branch`") && refused.contains("[debug]"),
            "{refused}"
        );
        assert_eq!(
            ctx.debug_flags(),
            DebugFlags::COVERAGE,
            "a refused start turns nothing on"
        );

        let path = Value::text("/profile.json");
        let profile = nvs_runtime::call(nvs_core_debug_start_profile, &mut ctx, &[path]);
        #[expect(
            unsafe_code,
            reason = "the call borrowed the one reference this frame built"
        )]
        unsafe {
            path.release();
        }
        assert!(
            profile.is_err(),
            "`profile` is not in the mode, and nothing grants the path"
        );
        drop(ctx.take_pending());
        assert!(!ctx.debug_flags().contains(DebugFlags::PROFILE));
    }

    /// `rule:testing/debug-surface`: `coverage()` reads the host's table back as path, line and
    /// count. A line is its largest statement's count, and a line that did not run is not listed.
    // covers: Core\Debug::coverage
    #[test]
    fn coverage_reads_the_table_back_by_path_and_line() {
        let mut lines = nvs_runtime::StmtLines::new();
        lines.push("app/Main.nvs", 3);
        lines.push("app/Main.nvs", 3);
        lines.push("app/Main.nvs", 4);
        lines.push("app/Lib.nvs", 9);
        let hits = std::sync::Arc::new(nvs_runtime::StmtHits::new(4).located(lines));
        hits.record(0);
        hits.record(1);
        hits.record(1);
        hits.record(3);
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_stmt_hits(Some(hits));

        let answered = nvs_runtime::call(nvs_core_debug_coverage, &mut ctx, &[])
            .expect("a read-back never throws");
        let files = crate::arr::borrowed(answered.array_ptr().expect("an array"));
        let main = files
            .get(b"app/Main.nvs")
            .expect("a file that ran is listed");
        let main = crate::arr::borrowed(main.array_ptr().expect("an array of lines"));
        assert_eq!(main.get_index(3).and_then(Value::as_int), Some(2));
        assert!(
            main.get_index(4).is_none(),
            "a line that did not run is not listed"
        );
        let lib = files.get(b"app/Lib.nvs").expect("the second file");
        let lib = crate::arr::borrowed(lib.array_ptr().expect("an array of lines"));
        assert_eq!(lib.get_index(9).and_then(Value::as_int), Some(1));
        #[expect(
            unsafe_code,
            reason = "the call returned the one reference to the array"
        )]
        unsafe {
            answered.release();
        }
    }

    /// `rule:errors/renderings`: `render` answers exactly what a `dump` of the
    /// same value writes, minus the line `dump` ends with — [`rendered`]'s own
    /// bound, and what lets an inline snapshot share the walk rather than grow
    /// a second one.
    ///
    /// Asserted as the agreement between the two rather than against a text
    /// literal, so a change to the walk cannot leave one of them right and the
    /// other wrong.
    // covers: Core\Debug::render
    #[test]
    fn render_answers_what_a_dump_writes_without_its_trailing_line() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_diagnostic_sink(nvs_runtime::OutputSink::Buffer(Vec::new()));
        dump_in(&mut ctx);
        let written = ctx
            .take_buffered_diagnostic()
            .expect("the diagnostic channel buffers");
        let answered = rendered(Value::int(7));
        assert!(
            !answered.ends_with('\n'),
            "`render` answers a value and the caller decides where the line ends: {answered:?}"
        );
        assert_eq!(
            written,
            format!("{answered}\n").into_bytes(),
            "one walk and two producers: a dump is the rendering plus the line it ends with"
        );
    }
}
