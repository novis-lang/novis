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
//! the renderings from becoming twenty. Today one sink exists — the terminal —
//! so both members render `nvs_render::plain`.
//!
//! # Where a dump lands
//!
//! § 4's table: in a CLI program a dump goes to **stderr**, never stdout, so
//! `prog | jq` and `prog > out.txt` keep working while a program is being
//! debugged. `nvs_runtime::Ctx::write_diagnostic` is that channel and owns why
//! it is a second sink rather than a fourth `OutputSink` variant. A dump is
//! deliberately *not* captured by `Core\Out::capture`: capturing one would
//! swallow the very output it was written to make visible.
//!
//! The HTTP rows of that table — a dump reaching an HTML body under
//! `[debug] inline`, and never reaching a JSON one — are M7's, and they are
//! the security half of the ADR: PHP's most-exploited information disclosure
//! is not a bug in `var_dump`, it is that `var_dump` writes to *output*. Here
//! the forgotten call writes to stderr and, at M8, a log record; the spelling
//! that would put it in a response body does not exist outside a development
//! mode whose ceiling is closed by default.
//!
//! # `render`, and why it is one member rather than a second mechanism
//!
//! `render` answers the carrier of the sink in force — `Core\Cli\Text` today —
//! so a dump can be *embedded* rather than written, by
//! `rule:security/capture-answers-the-carrier`
//! 's rule verbatim. Because it answers a carrier, `echo Core\Debug::render($x)`
//! is singly escaped: those bytes have already been through the record's own
//! transformations, and the carrier is what stops the next `echo` escaping
//! them again.
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
use nvs_runtime::{Fault, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// The class's fully-qualified name.
pub(crate) const NAME: &str = r"Core\Debug";

/// Spec § 16's `Core\Debug`, as much of it as `rule:errors/debug-dump` declares.
///
/// The coverage, trace and profile members that section also lists are
/// `rule:testing/debug-probes`'s
/// and land at M10 — that ADR's own scope, which the spec row used to
/// attribute `dump` to as well.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
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
    short: "Writes one rendered node per argument to the diagnostic channel — stderr in a CLI \
            program, never stdout — which is what `var_dump` is for, minus its writing to \
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
    short: "Renders `$value` exactly as `dump` would and answers it as the carrier of the sink \
            in force instead of writing it, so a dump can be embedded in output and stays \
            singly escaped.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to render, walked as `dump` walks one.",
        shape: &[],
    }],
    ret: "The rendering as a `Core\\Cli\\Text`, without the trailing newline `dump` writes.",
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
    /// [`crate::registry::RECORD_PRODUCERS`] puts in argument 0 and owns the
    /// position of.
    ///
    /// A dump with no arguments at all writes nothing rather than an empty
    /// line: `Core\Debug::dump()` says nothing, and a blank line in a build
    /// log is worse than silence.
    fn nvs_core_debug_dump(ctx, args: [2]) {
        #[expect(
            unsafe_code,
            reason = "the carrier came out of a `SourceConst` the compiled unit baked into its own data section, which outlives every request served from it"
        )]
        let source = unsafe { nvs_runtime::source::of_operand(args[0]) };
        let record = record_of(source, &args[1])?;
        if record.nodes.is_empty() {
            return Ok(Value::null());
        }
        let rendered = nvs_render::plain::render_nodes(&record.nodes);
        // Unreachable from source, and by a different route than the checker
        // refusals elsewhere in this file: the only diagnostic sink that can
        // fail is `OutputSink::Stderr` — `Buffer` and `Sink` never do, which
        // `Ctx::write_diagnostic`'s own `# Errors` states — and nothing in the
        // language moves the channel or closes the descriptor. A program that
        // dumps cannot make this happen; only the host can, by handing the
        // process a stderr it then breaks, and there is no case that spells it.
        ctx.write_diagnostic(rendered.as_bytes())
            .map_err(|e| Fault::fatal(format!("Core\\Debug::dump could not write: {e}")))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Debug::render(mixed $value): Core\Cli\Text` — the same record,
    /// answered as the sink's carrier instead of written.
    ///
    /// `rule:security/capture-answers-the-carrier`'s rule verbatim: bytes that have been through a sink
    /// cannot be handed back as a `string` without the next `echo` escaping
    /// them a second time, so this answers what `Core\Out::capture` answers.
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

/// One value as `Core\Debug::render` answers it: the canonical, ordered,
/// `secret`-redacting text of the whole value, with no trailing newline.
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
    #[test]
    fn dump_is_variadic_and_render_answers_the_carrier() {
        let dump = CLASS.methods[0];
        assert_eq!(dump.name, "dump");
        assert!(matches!(dump.params[0], CoreTy::Variadic(&CoreTy::Mixed)));
        assert!(matches!(dump.return_ty, CoreTy::Void));
        let render = CLASS.methods[1];
        assert_eq!(render.name, "render");
        assert!(matches!(
            render.return_ty,
            CoreTy::Instance(name) if name == crate::cli::NAME
        ));
    }

    /// **There is no format argument on either member** — § 7. The sink in
    /// force is the only input to which rendering runs, which is what keeps
    /// the renderings from becoming twenty.
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

    /// `rule:errors/a-record-names-where-it-was-produced`: the record a dump
    /// produces names the file, line and member its own call was written at,
    /// read off the constant [`crate::registry::RECORD_PRODUCERS`] puts in
    /// argument 0.
    ///
    /// Asserted on the record rather than on what the channel shows, because
    /// `rule:errors/debug-dump` sends a dump's *nodes* to the terminal and
    /// renders no envelope there. The datum is on the record for the renderings
    /// that do carry one — `nvs_render::json` writes every envelope key — and
    /// putting it anywhere else would be the second construction § 1 refuses.
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
}
