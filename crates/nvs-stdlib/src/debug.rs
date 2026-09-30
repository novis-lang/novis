//! `Core\Debug` — `rule:errors/debug-dump`'s `dump` and `render`, and the walk that turns a runtime value into
//! § 1's record.
//!
//! ```text
//! Core\Debug::dump(mixed ...$values): void
//! Core\Debug::render(mixed $value): Core\Cli\Text
//! ```
//!
//! **There is no format argument on either, and there is no `dumpRaw`.** The
//! sink in force is the only input to which rendering runs (§ 3), which is
//! what keeps the five producers from each growing a `$format` parameter and
//! the renderings from becoming twenty. [`rendered_for`] is the whole of that
//! choice, asked of the channel the bytes leave through: a dump reaching a
//! terminal leaves through `nvs_runtime::Ctx::write_diagnostic` and a block
//! appended to a response leaves through the body, so each takes that
//! channel's own carrier and neither is a table this module states twice.
//!
//! # Where a dump lands
//!
//! § 4's table, and [`nvs_core_debug_dump`] is the whole of the routing.
//! Outside a request — a CLI program, a scheduled script, a job worker, a test
//! — a dump goes to **stderr**, never stdout, so `prog | jq` and
//! `prog > out.txt` keep working while a program is being debugged.
//! `nvs_runtime::Ctx::write_diagnostic` is that channel and owns why it is a
//! second sink rather than a fourth `OutputSink` variant. A dump is
//! deliberately *not* captured by `Core\Out::capture`: capturing one would
//! swallow the very output it was written to make visible.
//!
//! **Inside a request a dump is a log record**, at `Log\Level::Debug`, written
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
//! # The walk is not here
//!
//! What turns a runtime value into `rule:errors/diagnostic-record`'s nodes is
//! `nvs_runtime::record`, and `dump` is one of its callers. It sits there
//! because `nvs_runtime::floor::uncaught` is a producer too and runs where no
//! `Core` class may be depended on; that module's header owns which
//! transformations the walk applies, what it still cannot see, and why one walk
//! rather than two.

use nvs_render::{Level, Node, Record, Source};
use nvs_runtime::{Ctx, Fault, LogChannel, Tag, Value};

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// The class's fully-qualified name.
pub(crate) const NAME: &str = r"Core\Debug";

/// `Core\Debug`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Shows any value in a readable form while you debug a program. It can print the \
            value to stderr or return it as a string.",
};

/// Spec § 16's `Core\Debug`, as much of it as `rule:errors/debug-dump` declares.
///
/// The coverage, trace and profile members that section also lists are
/// `rule:testing/debug-probes`'s
/// and land at M10 — that ADR's own scope, which the spec row used to
/// attribute `dump` to as well.
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
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Debug::dump`'s reference card — `rule:core-api/reference-card`.
const DUMP_DOC: MethodDoc = MethodDoc {
    short: "Writes one rendered node per argument where the sink in force sends it: the \
            diagnostic channel outside a request — stderr in a CLI program, never stdout — and \
            a log record at `Debug` inside one, which `[debug] inline` additionally appends to \
            an HTML response body. This is what `var_dump` is for, minus its writing to \
            output.",
    params: &[ParamDoc {
        name: "values",
        desc: "Any number of values, each rendered as a debug record: control bytes and bidi \
               made visible, a deep or long structure elided, a cycle marked, and a `secret` \
               property redacted.",
        shape: &[],
    }],
    ret: "Nothing; a call with no arguments writes nothing at all, and a dump is never \
          captured by `Core\\Out::capture`.",
    errors: &[],
};

/// `Core\Debug::render`'s reference card — `rule:core-api/reference-card`.
const RENDER_DOC: MethodDoc = MethodDoc {
    short: "Renders `$value` exactly as `dump` would and answers it as a `Core\\Cli\\Text` \
            instead of writing it, so a dump can be embedded in output and stays singly \
            escaped.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to render, walked as `dump` walks one.",
        shape: &[],
    }],
    ret: "The plaintext rendering as a `Core\\Cli\\Text` under every sink, without the trailing \
          newline `dump` writes.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_debug_dump" => (nvs_core_debug_dump as *const ()).cast(),
        "nvs_core_debug_render" => (nvs_core_debug_render as *const ()).cast(),
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
    /// The two destinations below are § 4's table and the module doc is where
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
            let rendered = rendered_for(ctx.diagnostic_carrier(), &record.nodes);
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
            let block = rendered_for(ctx.carrier(), &record.nodes);
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
        Ok(crate::cli::built(Value::str(nvs_runtime::NvsStr::new(
            rendered(args[0]).as_bytes(),
        ))))
    }
}

/// One value as a node at a record's root, under the default caps.
///
/// The entry point for a producer outside this module — [`crate::log`]'s
/// `fields` bag is walked through here — so that a value written as a log field
/// and the same value dumped are the same node, with § 5's transformations
/// applied once and in one place. Everything else about the walk, including
/// what it cannot see, is `nvs_runtime::record`'s.
pub(crate) fn node(value: Value) -> Node {
    nvs_runtime::record::node(value)
}

/// `rule:errors/record-producers`'s *"a record at `Debug`, one node per argument"* — the whole
/// of what `dump` produces, and the shape M8's log record is built from too.
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
fn rendered_for(carrier: &str, nodes: &[Node]) -> String {
    if carrier == nvs_runtime::CARRIER_HTML_MARKUP {
        nvs_render::html::render_nodes(nodes)
    } else {
        nvs_render::plain::render_nodes(nodes)
    }
}

/// One value as `Core\Debug::render` answers it at a terminal: the canonical,
/// ordered, `secret`-redacting text of the whole value, with no trailing
/// newline.
///
/// Public to the crate because
/// `rule:testing/inline-snapshots`'s inline snapshot is *this* rendering held in a source literal —
/// `Core\Test::assertMatchesInline` calls it rather than growing one of its
/// own, so a snapshot and a dump of the same value cannot disagree about what
/// that value looks like, and § 5's redaction reaches a snapshot for free.
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

    /// **There is no format argument on either member** — § 7. The sink in
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
                "`{}` grew an options bag, and § 7 has no room for one",
                method.name
            );
        }
    }

    /// `rule:errors/diagnostic-record`'s own M4 verification bullet: `Core\Debug::dump` writes to
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
    /// and with `[debug] inline` unset nothing reaches the body at all.
    ///
    /// Asserted on the rendering's frame rather than on the whole line, because
    /// which envelope keys a record carries is `nvs_render::json`'s own test and
    /// `Ctx::write_log_record`'s; what is this module's is that the dump went
    /// through them.
    // covers: Core\Debug::dump
    #[test]
    fn a_dump_inside_a_request_is_a_record_and_not_a_line() {
        let mut ctx = requesting("");
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
        assert!(
            body.starts_with("<div class=\"nvs-nodes\">") && body.contains('7'),
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
    /// putting it anywhere else would be the second construction § 1 refuses.
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
