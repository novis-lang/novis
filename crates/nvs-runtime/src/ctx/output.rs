//! Where a request's bytes go, and what it declares about them.
//!
//! [`OutputSink`] is `rule:tooling/echo-always-has-a-sink`
//! 's row as a type: the sink decides the carrier, so the `CARRIER_*`
//! constants and [`is_carrier`] are named here rather than in `nvs-stdlib`
//! where the classes themselves are declared.
//!
//! The channels share one file because they share one sink switch: the body
//! ([`Ctx::write_output`]), diagnostics ([`Ctx::write_diagnostic`]), log
//! records ([`Ctx::write_log_record`]) and the records [`LogWriter`] takes from
//! a thread holding no request at all end in [`write_to`], so a variant added
//! later cannot be handled at one channel and forgotten at another.
//! § 5's captures, and the content type, status and headers a request declares
//! back, sit beside them because each is a decision about the same response.

use std::collections::HashMap;

use super::*;

/// The `Core` class a captured terminal sink hands its bytes back as —
/// `rule:tooling/echo-always-has-a-sink`
/// 's default row, and § 5's carrier.
///
/// Named here rather than in `nvs-stdlib`, where the class itself is declared,
/// because the *sink* is what decides the carrier and the sink lives in this
/// crate. `nvs_stdlib::cli::TEXT` takes its `name` from this constant, so the
/// class a program writes and the class [`crate::value_to_string`] renders
/// cannot drift apart.
pub const CARRIER_CLI_TEXT: &str = r"Core\Cli\Text";

/// The carrier of the **HTML** sink — `rule:tooling/echo-always-has-a-sink`'s HTTP-request row.
///
/// Selected by [`OutputSink::Body`] and by nothing else, which is that row's
/// "attached by an HTTP request and by nothing else" written as a fact about
/// the sink rather than as a rule: `nvs_host::Isolate` builds that sink for an
/// isolate answering a request, so a CLI program, a scheduled script, a job
/// worker and a test keep [`CARRIER_CLI_TEXT`] without any of them saying so.
/// Declared beside [`CARRIER_CLI_TEXT`] so the pair is one fact in one file,
/// and so [`crate::value_to_string`]'s carrier row is written against the *set*
/// of carriers rather than against one of them.
pub const CARRIER_HTML_MARKUP: &str = r"Core\Html\Markup";

/// The field slot every sink carrier holds its already-escaped bytes in.
///
/// Both carriers declare exactly one slot and this is it, so
/// [`crate::value_to_string`] can render either without asking `nvs-stdlib`
/// anything — which it could not do anyway, the dependency running
/// `nvs-stdlib` → `nvs-runtime` and not back. `nvs_stdlib::cli`'s
/// `the_carrier_slot_matches_the_registered_layout` is the check that the
/// class's own registered layout agrees with this number.
pub const CARRIER_TEXT_SLOT: usize = 0;

/// Whether `name` is a sink carrier — [`CARRIER_CLI_TEXT`] or
/// [`CARRIER_HTML_MARKUP`].
#[must_use]
pub fn is_carrier(name: &str) -> bool {
    name == CARRIER_CLI_TEXT || name == CARRIER_HTML_MARKUP
}

/// The `Core` class a sink hands its bytes back as — one table, read by
/// [`Ctx::carrier`] for the channel `echo` writes to and by
/// [`Ctx::diagnostic_carrier`] for the channel a diagnostic writes to, so the
/// two cannot come to disagree about what [`OutputSink::Body`] means.
///
/// [`CARRIER_HTML_MARKUP`] for [`OutputSink::Body`], and [`CARRIER_CLI_TEXT`]
/// for every other sink, because every other one is a terminal or a stand-in
/// for one: `nvs run`'s stdout, a test's buffer, a discarded run.
fn carrier_of(sink: &OutputSink) -> &'static str {
    match sink {
        OutputSink::Body(_) => CARRIER_HTML_MARKUP,
        OutputSink::Stdout
        | OutputSink::Stderr
        | OutputSink::Buffer(_)
        | OutputSink::File(_)
        | OutputSink::Sink => CARRIER_CLI_TEXT,
    }
}

/// Where a request's `echo` output goes.
#[derive(Debug)]
#[non_exhaustive]
pub enum OutputSink {
    /// The process's standard output — `nvs run`'s destination.
    Stdout,
    /// The process's standard error — the *diagnostic* channel's destination,
    /// and never a request's `echo`.
    ///
    /// `rule:errors/debug-dump` sends a CLI `Core\Debug::dump` here rather than to stdout, so
    /// `prog | jq` and `prog > out.txt` keep working while a program is being
    /// debugged. `var_dump` writing to stdout is a small thing that makes PHP
    /// CLI tools unpipeable, and there is no reason to inherit it.
    Stderr,
    /// An in-memory buffer, read back with [`Ctx::take_buffered_output`].
    ///
    /// This is what a test uses, and the shape [`Self::Body`] reuses.
    Buffer(Vec<u8>),
    /// An HTTP **response body** — the same buffer as [`Self::Buffer`], read
    /// back the same way, and the one sink that answers
    /// [`CARRIER_HTML_MARKUP`].
    ///
    /// `rule:tooling/echo-always-has-a-sink`
    /// 's first row: inside an HTTP request `echo` writes to the response
    /// body, and what carries those bytes is `Core\Html\Markup`. A variant
    /// rather than a flag on [`Self::Buffer`], because "which sink is attached"
    /// is then one question with one answer and [`Ctx::carrier`] is one arm
    /// rather than a rule a call site states. It is selected in exactly one
    /// place — `nvs_host::Isolate`, from whether the isolate was handed a
    /// request — and a `spawn script` child inside a request takes it because
    /// § 3's third row gives that child the *parent's* carrier.
    Body(Vec<u8>),
    /// A file on disk, under `rule:http-server/the-floor-cannot-fill-the-disk`
    /// 's rotation and retention bound.
    ///
    /// What `[log] target = "file:…"` selects, built by
    /// [`Ctx::write_log_record`]'s reader and reachable directly through
    /// [`Ctx::set_diagnostic_sink`], which is how the floor's own bound is
    /// asserted without a configuration in front of it.
    File(crate::logfile::LogFile),
    /// Discarded.
    Sink,
}

/// Where a record goes when `[log] target` names no destination — which is a
/// different channel for each of `rule:errors/record-producers`'s two writers, and the same one for both as soon as it does name one.
///
/// [`Ctx::write_log_record`] is the whole of the routing and its doc comment is
/// the home of why the unconfigured default is a split rather than a single
/// channel. A writer that holds no request picks neither value: only one of the
/// two channels exists with no program underneath it, which is [`LogWriter`]'s
/// own doc comment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogChannel {
    /// The program's own output, through [`Ctx::write_output`] and so through
    /// `rule:security/capture-answers-the-carrier`'s capture stack — `Core\Log::write`'s, because a record a
    /// program chose to write is something it said.
    Output,
    /// The diagnostic channel, through [`Ctx::write_diagnostic`] — the engine
    /// floor's, because a record about a program that has already stopped is
    /// not that program's output.
    Diagnostic,
}

/// What `[log] target` resolved to, read once per context.
#[derive(Debug)]
pub(super) enum LogTarget {
    /// The directive has not been read yet. Every context starts here and
    /// returns here at [`Ctx::set_config`], so the read happens after the
    /// configuration is in place and never twice.
    Unread,
    /// Read, and the configuration names no destination this build can open.
    /// Each writer keeps [`LogChannel`]'s own channel.
    Unnamed,
    /// Read: both writers land here.
    Named(OutputSink),
}

/// [`Ctx::write_log_record`]'s routing, for a writer that holds no request: the
/// same three directives resolved through the same readers, and a record
/// rendered and written where they say.
///
/// The control thread is what needs this.
/// `rule:config/one-local-control-socket` has every reload written to
/// `Core\Log` with its outcome, and the thread that performs one is answering
/// an operator rather than serving a request, so there is no [`Ctx`] under it
/// to have read `[log] target`, `level` and `format` already. It resolves them
/// here rather than reading them for itself, because a directive with a second
/// reader is a directive two answers can be given for — which is the same
/// argument [`Ctx::stamp_envelope`] makes about the envelope.
///
/// **A configuration that names no target writes to [`OutputSink::Stderr`]**,
/// which is [`LogChannel::Diagnostic`]'s channel. The other value is not a
/// choice this writer can make: [`LogChannel::Output`] is the program's own
/// output through `rule:security/capture-answers-the-carrier`'s capture stack,
/// and a writer that is no request has no program, no response body and no
/// capture stack to reach — so the split a context makes between the two
/// collapses to the engine's own channel, which is where the floor already puts
/// a record about a program that is not running.
///
/// **What it spends** (`rule:programs/memory-priority`): one resolved sink per
/// writer — a path, a descriptor and two counters where the target is a file —
/// and one `String` per record written. Nothing is O(records), and a process
/// that never writes one holds a discriminant.
#[derive(Debug)]
pub struct LogWriter {
    /// What `[log] target` named, or [`OutputSink::Stderr`] where it named
    /// nothing this build opens. Held for the life of the writer, because a
    /// rotation bound counted against a handle needs the handle to outlive the
    /// record.
    sink: OutputSink,
    /// The quietest level `[log] level` writes — records below it are dropped
    /// before they are rendered.
    minimum: Level,
    /// Which of `rule:errors/renderings`'s two renderings the sink is handed.
    format: LogFormat,
}

impl LogWriter {
    /// The writer `config` describes, with the directives read once — `None`
    /// for a caller that has resolved no tree, which answers as a context
    /// nobody configured does.
    #[must_use]
    pub fn resolve(config: Option<&nvs_config::Request>) -> Self {
        Self {
            sink: match log_target(config) {
                LogTarget::Named(sink) => sink,
                LogTarget::Unread | LogTarget::Unnamed => OutputSink::Stderr,
            },
            minimum: log_minimum(config),
            format: log_format(config),
        }
    }

    /// `record` as the line `[log] format` names, or `None` where the record is
    /// below `[log] level`.
    ///
    /// The render without the write, for a caller that has somewhere else to
    /// put the line — a test reading back what a reload said, and the answer a
    /// control operation writes into its own response.
    #[must_use]
    pub fn render(&self, record: &Record) -> Option<String> {
        (record.envelope.level >= self.minimum).then(|| rendered(self.format, record))
    }

    /// Writes `record` where `[log] target` says, or nowhere when it is below
    /// `[log] level`.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns. The caller swallows it, which is
    /// `rule:errors/engine-floor`'s answer at the floor: a log write that fails
    /// is not a reason to fail the operation it was reporting.
    pub fn write(&mut self, record: &Record) -> io::Result<()> {
        let Some(line) = self.render(record) else {
            return Ok(());
        };
        write_to(&mut self.sink, line.as_bytes())
    }
}

/// Which of `rule:concurrency/two-doors-one-isolate`'s two hand-overs opened
/// the event stream a context is writing.
///
/// Not a boolean, for [`crate::Scheme`]'s reason and one of its own: the doors
/// differ in what the program behind them may *do* and not only in how it got
/// here, so a flag a call site had to remember the direction of is one that
/// could quietly give a request a connection's powers. A context that opened
/// neither door holds no value at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventStreamDoor {
    /// The request answers with the stream itself — `Core\Sse::stream`, whose
    /// events end when the response does, and which is a streaming response
    /// and not a connection
    /// (`rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`).
    Response,
    /// The stream outlives the request that opened it: `Core\Sse::upgrade`'s
    /// isolate, handed the body of a `200 text/event-stream` the connection is
    /// still writing. This is the door `Core\Topic` means by a connection,
    /// beside the socket one [`Ctx::has_peer`] answers for.
    Connection,
}

impl Ctx {
    /// Writes raw bytes to this request's output, unescaped.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns. [`OutputSink::Buffer`] and
    /// [`OutputSink::Sink`] never fail.
    pub fn write_output(&mut self, bytes: &[u8]) -> io::Result<()> {
        // `rule:security/capture-answers-the-carrier`: while a `Core\Out::capture` is in force, the innermost
        // one takes the bytes and the sink below sees nothing.
        if let Some(capture) = self.captures.last_mut() {
            capture.extend_from_slice(bytes);
            return Ok(());
        }
        // `[limits] max_output` is bytes written to the *response*, so the
        // charge is here: below the capture, above the sink. What a capture
        // swallowed is not a response yet and is already bounded by
        // `[limits] memory`, the capture buffer being heap `crate::budget`
        // counts; it is charged when the program writes the captured text back
        // out, and charging it here as well would bill the same bytes twice.
        //
        // Charged whether or not *this* context has a ceiling, because the
        // counter is the thread's and the context holding the ceiling may be a
        // parent two levels up. The compare is `Ctx::over_output_limit`'s and
        // happens at the safepoint poll.
        crate::budget::wrote(bytes.len());
        write_to(&mut self.output, bytes)
    }

    /// Writes raw bytes to this request's **diagnostic** channel — `rule:errors/debug-dump`'s destination for a CLI `Core\Debug::dump`.
    ///
    /// Deliberately **not** routed through [`Self::captures`]: a
    /// `Core\Out::capture` redirects what a program `echo`s, and a dump is not
    /// that. Capturing one would make `echo Core\Out::capture(fn () =>
    /// Core\Debug::dump($x))` swallow the dump it was meant to make visible.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns. [`OutputSink::Buffer`] and
    /// [`OutputSink::Sink`] never fail.
    pub fn write_diagnostic(&mut self, bytes: &[u8]) -> io::Result<()> {
        write_to(&mut self.diagnostic, bytes)
    }

    /// Writes one rendered record where `[log] target` says, and where
    /// `unconfigured` says when the directive names nothing.
    ///
    /// **This is the only reader of that directive**, and both of
    /// `rule:errors/record-producers`'s writers reach it: `Core\Log::write` with [`LogChannel::Output`]
    /// and [`crate::floor::report`] with [`LogChannel::Diagnostic`]. § 6's
    /// claim is about *sameness* — one serialiser, two callers — and a
    /// destination each caller resolved for itself is the second way that
    /// sameness could be lost after the record's shape.
    ///
    /// **A named target wins over `Core\Out::capture`.** The record leaves
    /// through the sink rather than through [`Self::write_output`], so `rule:security/capture-answers-the-carrier`
    /// 's capture stack does not see it and `[limits] max_output` is not
    /// charged for it. Both follow from what the directive means: an operator
    /// naming a destination is saying where the deployment's records go, and a
    /// program capturing its own output has said nothing about that. With no
    /// target configured the record is still the program's output and both
    /// rules apply to it exactly as before.
    ///
    /// **`[log] level` is the floor, and it is read here for the same reason.**
    /// `rule:errors/log-level`'s last paragraph makes the directive the minimum level
    /// written, so a record quieter than it is dropped and answers `Ok`: it was
    /// not written, and nothing failed. Asked at this one call rather than at
    /// each writer, so the two of them cannot come to disagree about which
    /// records a deployment collects — which is § 6's sameness a second time,
    /// after the record's shape and its destination.
    ///
    /// The comparison is `<` over [`Level`]'s own ordering, which is § 2's
    /// roster quietest-first, and so is that section's `<=` over the syslog
    /// severities read the other way round — those run *downward*, `Debug` at 7
    /// and `Critical` at 2. Written as the enum ordering because that is the
    /// one of the two spellings a reader cannot get backwards.
    ///
    /// **`[log] format` picks the rendering, which is why this takes a
    /// [`Record`] and not bytes.** `rule:errors/renderings` gives a log target
    /// its JSON Lines and plaintext renderings, and § 6's producers name
    /// neither, so the choice belongs at the sink and nowhere else. A
    /// caller that rendered first would be a caller that had chosen, and the
    /// two of them would have chosen separately: the same drift the record's
    /// shape, its destination and its floor are each held here to avoid. The
    /// price is one `String` per written record, which is what the caller
    /// allocated before.
    ///
    /// Those directives are read once — see [`Self::set_config`] — and the
    /// sink `target` names is held for the life of the context, because a
    /// rotation bound counted against a handle needs the handle to survive the
    /// record.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns; a file target's failure is the caller's to
    /// swallow, which is `rule:errors/engine-floor`'s answer at the floor.
    pub fn write_log_record(
        &mut self,
        record: &Record,
        unconfigured: LogChannel,
    ) -> io::Result<()> {
        if !self.log_writes(record.envelope.level) {
            return Ok(());
        }
        let rendered = rendered(self.log_format, record);
        let line = rendered.as_bytes();
        if let LogTarget::Named(sink) = &mut self.log {
            return write_to(sink, line);
        }
        match unconfigured {
            LogChannel::Output => self.write_output(line),
            LogChannel::Diagnostic => self.write_diagnostic(line),
        }
    }

    /// Whether [`Self::write_log_record`] writes a record at `level`, or drops
    /// it below `[log] level`.
    ///
    /// A producer whose record is costly to build asks this first, so a call
    /// the floor drops builds nothing: `Core\Log::write` converts no fields,
    /// reads no clock and takes no coalescing slot for a record that would
    /// never be written. The answer is the same comparison
    /// [`Self::write_log_record`] makes, over the same directives, read once.
    pub fn log_writes(&mut self, level: Level) -> bool {
        if matches!(self.log, LogTarget::Unread) {
            self.log = log_target(self.config.as_ref());
            self.log_minimum = log_minimum(self.config.as_ref());
            self.log_format = log_format(self.config.as_ref());
        }
        level >= self.log_minimum
    }

    /// Fills the envelope keys `rule:errors/log-write` asks for beyond `level` and `msg` — `ts`, `request_id`, and
    /// `trace_id`/`span_id` when a trace is active.
    ///
    /// **Called by both of § 6's writers**, `Core\Log::write` and
    /// [`crate::floor::report`], for the reason
    /// [`Self::write_log_record`] gives about the format, the floor and the
    /// destination: a rule asked once cannot come to be answered two ways. It
    /// is a separate call from that one because
    /// [`crate::floor::key`](crate::floor)'s coalescing window is keyed on
    /// everything *except* these four, so they have to be on the record after
    /// the key is taken and not before.
    ///
    /// **All four come from the request, and a context answering none stamps
    /// nothing.** That is why `ts` is here rather than read off the clock
    /// unconditionally: § 6's four keys are what a *request* contributes to a
    /// record, and a bare clock read would give a CLI run an envelope shape of
    /// its own — neither the bare envelope the floor writes with no request in
    /// front of it nor the full one a served request carries. Those two shapes
    /// and no third is § 6's sameness. An absent key is omitted rather than
    /// written empty, which is `rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`'s rule
    /// for `trace_id`/`span_id` applied to the whole envelope.
    ///
    /// `request_id` is the trace id, because `rule:observability/a-trace-id-exists-for-every-request` has that be Novis's
    /// only request identifier — there is deliberately no second one to stamp.
    /// It repeats in `trace_id` for a sampled trace on purpose: a log pipeline
    /// correlating by request and a tracing backend correlating by trace read
    /// their own key, and neither has to know the other's rule.
    ///
    /// **What it spends:** two 32-byte strings and one 16-byte one per written
    /// record inside a request, and one clock read. Nothing per context, and
    /// nothing that outlives the record.
    pub fn stamp_envelope(&self, envelope: &mut nvs_render::Envelope) {
        if self.inbound().is_none() {
            return;
        }
        let trace = self.trace_context();
        envelope.ts = Some(jiff::Timestamp::now().to_string());
        envelope.request_id = Some(trace.trace_id_hex());
        // § 6's "whenever a trace is active", where active is the sampling
        // decision § 2 makes at the door: an id exists for every request, and
        // what a sampled trace additionally has is spans a backend will be
        // asked to join this line to.
        if trace.sampled() {
            envelope.trace_id = Some(trace.trace_id_hex());
            envelope.span_id = Some(trace.span_id_hex());
        }
    }

    /// Points this context's diagnostic channel somewhere else — what a test
    /// that wants to read a dump back calls, and the one way to move it off
    /// [`OutputSink::Stderr`].
    pub fn set_diagnostic_sink(&mut self, sink: OutputSink) {
        self.diagnostic = sink;
    }

    /// Takes everything written to the diagnostic channel so far, if it
    /// buffers.
    #[must_use]
    pub fn take_buffered_diagnostic(&mut self) -> Option<Vec<u8>> {
        match &mut self.diagnostic {
            OutputSink::Buffer(buffer) | OutputSink::Body(buffer) => Some(std::mem::take(buffer)),
            OutputSink::Stdout | OutputSink::Stderr | OutputSink::File(_) | OutputSink::Sink => {
                None
            }
        }
    }

    /// Flushes this request's output.
    ///
    /// `nvs run` calls this once the script's frame returns: Rust's standard
    /// output is line-buffered, and a script whose last `echo` has no trailing
    /// newline would otherwise depend on the process-exit flush.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns.
    pub fn flush_output(&mut self) -> io::Result<()> {
        match &mut self.output {
            OutputSink::Stdout => io::stdout().flush(),
            OutputSink::Stderr => io::stderr().flush(),
            // A `LogFile` writes straight through — there is no buffer of its
            // own between `write_all` and the descriptor.
            OutputSink::Buffer(_)
            | OutputSink::Body(_)
            | OutputSink::File(_)
            | OutputSink::Sink => Ok(()),
        }
    }

    /// The `Core` class this request's sink hands captured bytes back as —
    /// `rule:tooling/echo-always-has-a-sink`
    /// 's table, read as a class name.
    ///
    /// [`carrier_of`] over the sink `echo` writes to. § 3's direction is the
    /// fail-closed one — the HTML sink is attached by an HTTP request and by
    /// nothing else — and it is that variant plus that table rather than a rule
    /// any call site states: `nvs_host::Isolate` picks the sink from the
    /// request it was handed, so a scheduled script, a job worker, a `#[Test]`
    /// method and a CLI program all stay on the terminal sink by never having
    /// attached anything.
    #[must_use]
    pub fn carrier(&self) -> &'static str {
        carrier_of(&self.output)
    }

    /// The same class for the **diagnostic** channel: the sink
    /// [`Self::write_diagnostic`] writes to, rather than the one `echo` writes
    /// to.
    ///
    /// `rule:errors/renderings` picks a rendering from the sink in force, and
    /// for a `Core\Debug::dump` the sink in force is the one the dump's own
    /// bytes leave through — which is this one and not [`Self::carrier`]'s.
    /// The two channels answer differently on purpose:
    /// `rule:errors/debug-dump`'s table gives the HTML rendering to the row
    /// that appends a block to the response **body**, so a request that
    /// answers HTML while its dumps still go to the engine's stderr reads them
    /// as the plaintext a developer greps.
    #[must_use]
    pub fn diagnostic_carrier(&self) -> &'static str {
        carrier_of(&self.diagnostic)
    }

    /// Opens a capture level: from here until the matching [`Self::end_capture`],
    /// everything written to this request's output is buffered instead.
    pub fn begin_capture(&mut self) {
        self.captures.push(Vec::new());
    }

    /// Closes the innermost capture level and answers what it captured, or
    /// `None` when none was open.
    ///
    /// A caller that opened one **must** close it on every edge, the throwing
    /// one included — `nvs_stdlib::out` is the only such caller, and it does.
    pub fn end_capture(&mut self) -> Option<Vec<u8>> {
        self.captures.pop()
    }

    /// How many captures are open — a test's window onto the invariant that
    /// [`Self::begin_capture`] and [`Self::end_capture`] pair on every edge.
    #[must_use]
    pub fn capture_depth(&self) -> usize {
        self.captures.len()
    }

    /// Whether what this request writes reaches the process's own standard
    /// streams, rather than a buffer, a response body or nothing at all.
    ///
    /// `rule:tooling/a-prompt-is-a-core-member`'s prompts are the caller: a question is only a question if
    /// the person answering can see it, so `Core\Cli::ask` under `nvs serve`,
    /// inside a `Core\Out::capture` or under a test's [`OutputSink::Buffer`]
    /// is not interactive however many terminals the process has. Without
    /// this, a prompt in a request handler would write into the response body
    /// and then block the core waiting for a keystroke.
    #[must_use]
    pub fn output_reaches_the_terminal(&self) -> bool {
        self.captures.is_empty() && matches!(self.output, OutputSink::Stdout | OutputSink::Stderr)
    }

    /// Declares what this request's output *is* — `rule:security/response-body-is-one-typed-member`'s
    /// `Content-Type`, set by the body member that wrote it.
    ///
    /// Whoever declares last is what the response carries; [`Self::content_type`]
    /// owns why that is not a rule this method has to enforce.
    pub fn declare_content_type(&mut self, media_type: &str) {
        self.content_type = Some(media_type.into());
    }

    /// Takes the declaration away, leaving the context with none — what the
    /// isolate's finish path calls once, on its way to building a
    /// [`crate::host::Completion`].
    #[must_use]
    pub fn take_content_type(&mut self) -> Option<Box<str>> {
        self.content_type.take()
    }

    /// Holds one rendered `Core\Debug::dump` block for this request's body —
    /// `rule:errors/debug-dump`'s `[debug] inline` row, the only one of that
    /// table's four that puts a dump in a response at all.
    ///
    /// The rendering arrives already made, from [`Self::carrier`], so what a
    /// block looks like stays where `rule:errors/renderings` puts it: the
    /// channel the bytes leave through picks it, and there is no second table
    /// here to disagree with that one.
    pub fn append_inline_debug(&mut self, block: &str) {
        self.inline_debug.extend_from_slice(block.as_bytes());
    }

    /// Appends those blocks to this request's body, and never to a body that
    /// declared it is not HTML.
    ///
    /// The isolate's finish path calls this once, beside
    /// [`Self::take_content_type`] and before the output is taken, because two
    /// things are true only there. The declaration saying what the body *is* has
    /// been made if it is going to be; and every `Core\Out::capture` has closed,
    /// so a dump is still not something a capture can swallow even though these
    /// bytes go out through [`Self::write_output`] — which is what charges them
    /// to `[limits] max_output`, the response being what they are part of.
    ///
    /// **A JSON body is never modified**, in either mode, which
    /// `rule:errors/debug-dump` makes a property of the language rather than of
    /// a deployment: an endpoint answers the same shape in development as in
    /// production, so a client's strict validator cannot pass against one and
    /// fail against the other. `None` is the request that only echoed, which
    /// `rule:security/response-body-is-one-typed-member` reads as `text/html`.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns. [`OutputSink::Body`] never fails, and it is
    /// the sink every request this can write anything for is on.
    pub fn flush_inline_debug(&mut self) -> io::Result<()> {
        let blocks = std::mem::take(&mut self.inline_debug);
        if blocks.is_empty() || !self.body_takes_a_block() {
            return Ok(());
        }
        self.write_output(&blocks)
    }

    /// Whether this response's body is the HTML one a block may be appended to.
    ///
    /// Compared case-insensitively over the media type's own token, because
    /// `Core\Response::bytes` takes the type as a parameter and a program may
    /// spell it in any case RFC 9110 admits; the parameters after it — a
    /// charset, a boundary — say nothing about whether the body is markup.
    fn body_takes_a_block(&self) -> bool {
        self.content_type.as_deref().is_none_or(|media| {
            media
                .as_bytes()
                .get(..b"text/html".len())
                .is_some_and(|head| head.eq_ignore_ascii_case(b"text/html"))
        })
    }

    /// Records the file this response's body is — `Core\Response::sendFile`,
    /// the one body member that hands over a name instead of bytes.
    ///
    /// **Answers whether anything will read it**, which is what makes the
    /// member's fallback a branch rather than a rule to remember. `true` for a
    /// context answering an HTTP request: the finish path lifts the name onto a
    /// [`crate::host::Completion`] and the server streams the file under the
    /// static policy, so nothing is read here at all. `false` everywhere else —
    /// a CLI program, a `#[Test]` method, a `.nvst` case, a `spawn script`
    /// child, none of which is answering a request — and there the member
    /// writes the bytes into this context's own output, which is
    /// `nvs_stdlib::response` § *A body goes out verbatim, and off a request
    /// the declaration is inert* for the member whose body is a file.
    ///
    /// The request is the question rather than the sink, unlike
    /// [`Self::carrier`]: a child isolate inside a request takes its parent's
    /// carrier and answers no response of its own, so a file it sent would
    /// otherwise reach nobody.
    ///
    /// The declaration is made regardless, on [`Self::declare_content_type`]'s
    /// terms — a word left on a context nobody asks costs a name — so the two
    /// halves of the answer are one field and never a fact the caller has to
    /// keep.
    #[must_use]
    pub fn declare_file_body(&mut self, path: &std::path::Path) -> bool {
        self.file_body = Some(path.into());
        self.inbound.is_some()
    }

    /// Takes the name away, leaving the context with none — the finish path's
    /// call beside [`Self::take_content_type`], and the only reader.
    #[must_use]
    pub fn take_file_body(&mut self) -> Option<Box<std::path::Path>> {
        self.file_body.take()
    }

    /// Records the writing half of a response body being written over time —
    /// `Core\Response::stream`, once per request.
    ///
    /// The neighbours above are the model, with one difference: a declaration
    /// says what the body *is* and this says where it *goes*, so a second call
    /// would not be a later opinion but a second body. The refusal belongs to
    /// the member, which reaches `crate::stream::BodySlot::open` first and never
    /// gets here on a request that already has one.
    pub fn set_body_stream(&mut self, emit: crate::stream::Emit) {
        self.body_stream = Some(emit);
    }

    /// Takes the writing half away, which **ends the body it was written on** —
    /// the finish path's fourth call, beside [`Self::take_headers`].
    ///
    /// This is where "a request-scoped stream ends with its request" is a fact
    /// rather than a hope, and it is a take at the end of the program rather
    /// than a wait on this context's own teardown: what ends the stream is the
    /// last [`crate::stream::Emit`] going away, so a body left to the arena
    /// would hold the peer's connection open until an isolate nobody had joined
    /// yet was reclaimed — and nobody joins it until the body ends.
    #[must_use]
    pub fn take_body_stream(&mut self) -> Option<crate::stream::Emit> {
        self.body_stream.take()
    }

    /// The writing half, for the member putting a chunk on it, and `None` for a
    /// request answering with a whole body.
    ///
    /// That `None` is what makes a chunk written off a connection an ordinary
    /// [`Self::write_output`] rather than a rule to remember: a CLI program, a
    /// `#[Test]` method and a `.nvst` case each answer it, because none of them
    /// was offered a cell to open a stream into.
    pub fn body_stream(&mut self) -> Option<&mut crate::stream::Emit> {
        self.body_stream.as_mut()
    }

    /// Records that what this context writes is an event stream, and which
    /// door opened it — `Core\Sse::stream` for the request that opened one for
    /// itself, and the hand-over for the connection that outlives its request.
    ///
    /// Not beside [`Self::set_body_stream`] in what it takes, because it is not
    /// the same kind of fact: that one is handed the half the bytes go through
    /// and this one is told what they *are*, which is why a program off a
    /// connection — writing its events to its own output — still marks.
    ///
    /// **The first door holds**, and that is what makes the second call safe
    /// rather than a downgrade: a connection isolate is marked before its
    /// program's first statement, so a `Core\Sse::stream` inside one is a
    /// program opening a stream on its own output and never a connection
    /// turning into a response that ends. There is no undoing it either way.
    pub fn mark_event_stream(&mut self, door: EventStreamDoor) {
        self.event_stream.get_or_insert(door);
    }

    /// Whether an event stream was opened on this context, by either door.
    ///
    /// `false` everywhere else, and that is what makes `Core\Sse::current()` a
    /// refusal outside one rather than a rule to remember: a command-line
    /// program, a `spawn script` child and a request answering with an ordinary
    /// body each answer it — including one streaming that body, a response body
    /// written over time not being an event stream.
    #[must_use]
    pub fn has_event_stream(&self) -> bool {
        self.event_stream.is_some()
    }

    /// Whether this context is an event stream's **connection** — the isolate
    /// the stream outlives its request on, and not the request that answers
    /// with one.
    ///
    /// [`Self::has_peer`]'s question asked of the other door, and the two
    /// together are what `Core\Topic` means by a connection: what a
    /// subscription needs is not a socket but a wait that drains it, which is
    /// exactly what a program outliving its request has. A request streaming
    /// its own events answers `false` and is refused, its response being over
    /// before anything could be published to it.
    #[must_use]
    pub fn has_event_stream_connection(&self) -> bool {
        self.event_stream == Some(EventStreamDoor::Connection)
    }

    /// Declares what this request's response *means* — spec § 15's status,
    /// set by `Core\Response::setStatus`.
    ///
    /// The neighbour above is the model: one word recorded on the context, put
    /// back on the completion, and turned into what the peer sees by whoever is
    /// answering. [`Self::status`] owns why it is a second field rather than a
    /// second half of the first, and why the last caller wins.
    ///
    /// The range is the member's to enforce, not this method's: `setStatus`
    /// refuses a code no peer can classify before calling here, so what arrives
    /// is already a status, and a second check would be a second answer to a
    /// question that has one.
    pub fn declare_status(&mut self, code: u16) {
        self.status = Some(code);
    }

    /// Takes the declaration away, leaving the context with none — the finish
    /// path's other half, called once beside [`Self::take_content_type`].
    #[must_use]
    pub fn take_status(&mut self) -> Option<u16> {
        self.status.take()
    }

    /// Sets one header on this request's response, replacing any value this
    /// request had already set under that name — spec § 15's `setHeader`, and
    /// `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`
    /// 's override of a policy-owned header.
    ///
    /// **Set, not add**: the member is named for replacement, and a second
    /// value under one name is [`Self::append_header`]'s question rather than
    /// this one's. A replaced entry keeps the position it was first set at, so
    /// a program that overwrote one header did not thereby reorder the rest,
    /// and every *further* value already declared under that name is dropped —
    /// after this call the name has exactly one value, which is what "set"
    /// means and is not something a scan stopping at the first match would
    /// leave true. The comparison is ASCII-case-insensitive because RFC 9110's
    /// field name is.
    ///
    /// **The invariant whoever answers reads off this list:** a replacing row
    /// for a name always precedes every appending row for it. So the answer can
    /// be written a row at a time in this order — replacing the map's entry for
    /// one, joining it for the other — without a later replacement wiping a
    /// value that was meant to survive.
    ///
    /// What a name and a value may be is the member's to enforce, on
    /// [`Self::declare_status`]'s reasoning: `setHeader` refuses anything a
    /// header line cannot carry before it calls here, so a second check would
    /// be a second answer to a question that has one.
    pub fn declare_header(&mut self, name: &str, value: &str) {
        self.headers.set(name, value);
    }

    /// A second value under a name that may already carry one — spec § 15's
    /// `addCookie`, and the half of the header path [`Self::declare_header`]
    /// deliberately is not.
    ///
    /// **Pushes without searching**, which is the whole difference. A
    /// `Set-Cookie` is meaningful exactly as many times as it was written, so
    /// the scan that makes `setHeader` an override of one policy-owned header
    /// is exactly what would collapse two cookies into the last one — and a
    /// response that silently carries one of the two cookies a program set is
    /// a session bug rather than a formatting one.
    ///
    /// What a name and a value may be is the calling member's to enforce, for
    /// the reason [`Self::declare_header`] gives.
    pub fn append_header(&mut self, name: &str, value: &str) {
        self.headers.add(name, value);
    }

    /// Takes the declared headers away, leaving the context with none — the
    /// finish path's third call, made once beside [`Self::take_status`].
    #[must_use]
    pub fn take_headers(&mut self) -> Vec<DeclaredHeader> {
        self.headers.take()
    }

    /// Puts `body` back in front of whatever was written since it was taken
    /// with [`Self::take_buffered_output`], without charging `[limits]
    /// max_output` a second time for bytes it already counted.
    pub(crate) fn restore_body(&mut self, mut body: Vec<u8>) {
        if let OutputSink::Buffer(buffer) | OutputSink::Body(buffer) = &mut self.output {
            body.append(buffer);
            *buffer = body;
        }
    }

    /// Takes everything written so far, if this context buffers its output.
    #[must_use]
    pub fn take_buffered_output(&mut self) -> Option<Vec<u8>> {
        match &mut self.output {
            OutputSink::Buffer(buffer) | OutputSink::Body(buffer) => Some(std::mem::take(buffer)),
            OutputSink::Stdout | OutputSink::Stderr | OutputSink::File(_) | OutputSink::Sink => {
                None
            }
        }
    }
}

/// One header a program declared for its response, and how it joins the ones
/// whoever answers wrote for itself.
///
/// A row rather than a bare name-and-value pair, because two members declare
/// headers and they mean opposite things about a name that is already present.
/// [`Ctx::declare_header`] is
/// `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`'s
/// override of *one* policy-owned header, so it replaces; [`Ctx::append_header`]
/// is the `Set-Cookie` path, where a second value under one name is the entire
/// point. Which of the two a row is cannot be recovered from the pair — a
/// repeated name looks identical either way — so the row carries it, and no
/// layer below has to guess.
///
/// **What it spends:** two short allocations and one byte, per declared header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredHeader {
    /// The field name, in the spelling the program wrote it in: the peer sees
    /// that spelling, and the case-insensitive comparison is
    /// [`Ctx::declare_header`]'s alone.
    pub name: Box<str>,
    /// The field value, checked by the member that declared it rather than
    /// here.
    pub value: Box<str>,
    /// Whether this value joins whatever the answer already carries under
    /// [`Self::name`] instead of replacing it.
    pub append: bool,
}

impl DeclaredHeader {
    /// A row that replaces what the answer carries under `name`.
    #[must_use]
    pub fn set(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            append: false,
        }
    }

    /// A row that joins it instead.
    #[must_use]
    pub fn add(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            append: true,
        }
    }
}

/// The rows a request declared, in declaration order, with an index from each
/// name to the rows that carry it.
///
/// The index is what keeps one declaration constant-time. Without it,
/// [`Ctx::declare_header`] compares against every row already declared, so a
/// program declaring many distinct names spends time quadratic in their count
/// while holding only a few bytes per row — the memory ceiling never trips, and
/// the request runs for minutes. A row [`Ctx::declare_header`] drops becomes a
/// `None` rather than being removed, so the positions the index holds stay
/// valid, and [`Self::take`] is the one place the holes are skipped.
///
/// **What it spends:** per request, one lower-cased copy of each distinct
/// name and one position per row, beside the rows themselves; one `None` per
/// row dropped. All of it is charged to that request's budget.
#[derive(Debug, Default)]
pub(crate) struct DeclaredHeaders {
    /// Every row, in the order it was first declared.
    rows: Vec<Option<DeclaredHeader>>,
    /// Each name, ASCII-lower-cased because RFC 9110's field name is
    /// case-insensitive, to the positions in [`Self::rows`] that carry it —
    /// first position first.
    by_name: HashMap<Box<str>, Vec<usize>>,
}

impl DeclaredHeaders {
    /// [`Ctx::declare_header`]'s whole effect: the first row under `name`
    /// takes `value` and becomes a replacing row, and every further row under
    /// it is dropped.
    fn set(&mut self, name: &str, value: &str) {
        let key: Box<str> = name.to_ascii_lowercase().into();
        if let Some(positions) = self.by_name.get_mut(&key) {
            for &further in &positions[1..] {
                self.rows[further] = None;
            }
            positions.truncate(1);
            if let Some(first) = self.rows[positions[0]].as_mut() {
                first.value = value.into();
                first.append = false;
            }
            return;
        }
        self.by_name.insert(key, vec![self.rows.len()]);
        self.rows.push(Some(DeclaredHeader::set(name, value)));
    }

    /// [`Ctx::append_header`]'s: one more row, whatever the name carries.
    fn add(&mut self, name: &str, value: &str) {
        let key: Box<str> = name.to_ascii_lowercase().into();
        self.by_name.entry(key).or_default().push(self.rows.len());
        self.rows.push(Some(DeclaredHeader::add(name, value)));
    }

    /// The rows still standing, in order, leaving none behind.
    fn take(&mut self) -> Vec<DeclaredHeader> {
        self.by_name = HashMap::new();
        std::mem::take(&mut self.rows)
            .into_iter()
            .flatten()
            .collect()
    }
}

/// `[log] target` as the sink it names, through `rule:errors/engine-floor`'s grammar and
/// not through a second reading of it.
///
/// Free of [`Ctx`] because both of its callers are: a request's context resolves
/// it once at its first record, and [`LogWriter`] resolves it for a thread that
/// has no context at all. The directive therefore keeps one reader however the
/// record was produced, which is what [`LogWriter`]'s doc comment means by two
/// readers being two answers.
///
/// [`nvs_config::log::Target`] is that grammar and it has two readers:
/// this one, and the boot check that refuses a tree naming a target § 4
/// does not spell. So a value that reached here is one the grammar spells,
/// and [`LogTarget::Unnamed`] covers two facts rather than one:
///
/// - **`syslog` is spelled and not yet transported.** A syslog sink is a
///   datagram to a platform endpoint carrying `rule:errors/log-level`'s severity in a
///   priority field — a transport, a framing and an argument the
///   byte-oriented sinks here do not take. Routing it to `stderr` instead
///   would be this module claiming a destination it does not reach, so it
///   routes nowhere new and each writer's own channel still carries the
///   record.
/// - **A target nobody spelled** never boots, so reaching it here means a
///   caller was configured by something other than a resolved tree — a
///   test, in practice. It is not a diagnostic at the one moment the
///   engine has a failure to report; it is the unconfigured routing.
fn log_target(config: Option<&nvs_config::Request>) -> LogTarget {
    let Some(written) = config.and_then(|config| config.get("log.target")) else {
        return LogTarget::Unnamed;
    };
    match nvs_config::log::Target::of(&written) {
        Some(nvs_config::log::Target::Stderr) => LogTarget::Named(OutputSink::Stderr),
        Some(nvs_config::log::Target::File(path)) => LogTarget::Named(OutputSink::File(
            crate::logfile::LogFile::new(std::path::PathBuf::from(path)),
        )),
        Some(nvs_config::log::Target::Syslog) | None => LogTarget::Unnamed,
    }
}

/// What `[log] level` names, or [`Level::Debug`] where it names nothing —
/// [`Ctx::write_log_record`]'s floor, resolved with the target above.
///
/// `Debug` for an unset directive rather than `rule:config/a-mode-is-five-defaults`'s per-mode
/// `Info`: that default belongs to the *tree* — not yet applied at boot,
/// which `nvs_config::mode`'s module doc carries as its open gap — so a
/// resolved configuration is where it will arrive, and a caller configured by
/// something other than a resolved tree has said nothing about which
/// records it wants. The safe answer to that is all of them. A word the
/// grammar does not carry reads the same way and never boots — `E0614`
/// refuses it at the file, for the reason `nvs_config::log`'s module doc
/// gives about doing this at boot rather than at the first record.
fn log_minimum(config: Option<&nvs_config::Request>) -> Level {
    config
        .and_then(|config| config.get("log.level"))
        .and_then(|written| Level::of(&written))
        .unwrap_or(Level::Debug)
}

/// What `[log] format` names, or [`LogFormat::Json`] where it names
/// nothing — [`Ctx::write_log_record`]'s rendering, resolved with the two
/// directives above.
///
/// One record per line for an unset directive, which is both `rule:config/a-mode-is-five-defaults`'s
/// per-mode default and [`LogFormat`]'s own: a pipeline reading a target
/// nobody configured can find the record boundaries without being told, and
/// the plaintext rendering's are a blank-line-free block. A word the grammar
/// does not carry reads the same way and never boots — `E0615` refuses it at
/// the file.
fn log_format(config: Option<&nvs_config::Request>) -> LogFormat {
    config
        .and_then(|config| config.get("log.format"))
        .and_then(|written| LogFormat::of(&written))
        .unwrap_or(LogFormat::Json)
}

/// One record as `format` renders it — `rule:errors/renderings`'s two
/// renderings behind one call, so a writer with a context and one without
/// cannot come to spell the same record two ways.
fn rendered(format: LogFormat, record: &Record) -> String {
    match format {
        LogFormat::Json => nvs_render::json::line(record),
        LogFormat::Text => nvs_render::plain::render(record),
    }
}

/// Writes `bytes` to one sink — the body [`Ctx::write_output`] and
/// [`Ctx::write_diagnostic`] share, so a sink variant added later cannot be
/// handled at one channel and forgotten at the other.
fn write_to(sink: &mut OutputSink, bytes: &[u8]) -> io::Result<()> {
    match sink {
        OutputSink::Stdout => io::stdout().write_all(bytes),
        OutputSink::Stderr => io::stderr().write_all(bytes),
        OutputSink::Buffer(buffer) | OutputSink::Body(buffer) => {
            buffer.extend_from_slice(bytes);
            Ok(())
        }
        OutputSink::File(file) => file.write(bytes),
        OutputSink::Sink => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The invariant `nvs-server` writes this list back out under: a set is
    /// total, so a name that already carried appended values has exactly one
    /// afterwards, and the row that survives is the replacing one at the
    /// position the name was first declared.
    ///
    /// Asserted here rather than at the layer that sends it, because the
    /// collapse is [`Ctx::declare_header`]'s: a server applying every row
    /// faithfully would still send the value this dropped.
    #[test]
    fn a_set_is_total_over_the_values_already_declared_under_one_name() {
        let mut ctx = Ctx::buffered();
        ctx.append_header("Set-Cookie", "sid=1");
        ctx.declare_header("X-Trace", "a");
        ctx.append_header("set-cookie", "theme=dark");
        ctx.declare_header("Set-Cookie", "sid=2");
        assert_eq!(
            ctx.take_headers(),
            vec![
                DeclaredHeader::set("Set-Cookie", "sid=2"),
                DeclaredHeader::set("X-Trace", "a"),
            ],
            "a set left a second value under its own name, or moved a name it did not set"
        );
    }

    /// Many distinct names cost time linear in their count, and a set after a
    /// run of appends still collapses them: the index [`DeclaredHeaders`] keeps
    /// is what both rest on.
    ///
    /// The count is the pin. A set that compared against every row declared
    /// before it takes minutes here in a debug build, and the hostile case
    /// `tests/hostile/core/Response/setHeader/` is the same attack from a
    /// program.
    #[test]
    fn many_distinct_names_are_declared_without_comparing_each_to_every_other() {
        const NAMES: usize = 200_000;
        let mut ctx = Ctx::buffered();
        for n in 0..NAMES {
            ctx.declare_header(&format!("X-Header-{n}"), "v");
        }
        for _ in 0..1_000 {
            ctx.append_header("Set-Cookie", "a=1");
            ctx.declare_header("set-cookie", "b=2");
        }
        ctx.declare_header("x-header-7", "last");
        let headers = ctx.take_headers();
        assert_eq!(headers.len(), NAMES + 1);
        assert_eq!(headers[7], DeclaredHeader::set("X-Header-7", "last"));
        assert_eq!(headers[NAMES], DeclaredHeader::set("Set-Cookie", "b=2"));
        assert!(ctx.take_headers().is_empty(), "a take left rows behind");
    }

    /// `[limits] max_output` bounds the *response*: what a capture swallowed is
    /// not one yet, and is charged when the program writes it back out rather
    /// than at both points.
    #[test]
    fn captured_bytes_are_charged_when_they_reach_the_sink_and_not_before() {
        let mut ctx = Ctx::buffered();
        ctx.begin_capture();
        ctx.write_output(b"inside").expect("a buffer");
        let taken = ctx.end_capture().expect("a capture was open");
        assert_eq!(ctx.output_used(), 0);

        ctx.write_output(&taken).expect("a buffer");
        assert_eq!(ctx.output_used(), 6);
    }

    /// `rule:security/capture-answers-the-carrier`'s "always swallows": while a capture is open the sink below
    /// it sees nothing at all, and it sees everything again once it closes.
    #[test]
    fn a_capture_takes_the_output_and_the_sink_below_sees_none_of_it() {
        let mut ctx = Ctx::buffered();
        ctx.write_output(b"before").unwrap();
        ctx.begin_capture();
        ctx.write_output(b"inside").unwrap();
        assert_eq!(ctx.end_capture().as_deref(), Some(&b"inside"[..]));
        ctx.write_output(b"after").unwrap();
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"beforeafter"[..])
        );
    }

    /// A capture is scoped to the callable it runs, so captures nest by call nesting: the
    /// innermost one takes the bytes, and what it re-emits afterwards lands in
    /// the one outside it.
    #[test]
    fn captures_nest_innermost_first() {
        let mut ctx = Ctx::buffered();
        ctx.begin_capture();
        ctx.write_output(b"outer<").unwrap();
        ctx.begin_capture();
        ctx.write_output(b"inner").unwrap();
        let inner = ctx.end_capture().expect("the inner capture was open");
        assert_eq!(inner, b"inner");
        assert_eq!(ctx.capture_depth(), 1);
        ctx.write_output(&inner).unwrap();
        ctx.write_output(b">").unwrap();
        assert_eq!(ctx.end_capture().as_deref(), Some(&b"outer<inner>"[..]));
        assert_eq!(ctx.capture_depth(), 0);
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
    }

    /// Closing a capture nobody opened answers `None` rather than corrupting
    /// the stack — the shape a helper's error edge relies on.
    #[test]
    fn ending_a_capture_that_was_never_begun_answers_nothing() {
        let mut ctx = Ctx::buffered();
        assert!(ctx.end_capture().is_none());
        assert_eq!(ctx.capture_depth(), 0);
    }

    /// Every sink but the response body is a terminal or a stand-in for one, so
    /// each names the same carrier — `rule:tooling/echo-always-has-a-sink`'s default row.
    #[test]
    fn every_sink_today_carries_cli_text() {
        assert_eq!(Ctx::stdout().carrier(), CARRIER_CLI_TEXT);
        assert_eq!(Ctx::buffered().carrier(), CARRIER_CLI_TEXT);
        assert_eq!(Ctx::new(OutputSink::Sink).carrier(), CARRIER_CLI_TEXT);
        assert_eq!(
            Ctx::new(OutputSink::Body(Vec::new())).carrier(),
            CARRIER_HTML_MARKUP
        );
        assert!(is_carrier(CARRIER_CLI_TEXT) && is_carrier(CARRIER_HTML_MARKUP));
        assert!(!is_carrier(r"Core\Str"));
    }

    #[test]
    fn buffered_output_accumulates_and_is_taken_once() {
        let mut ctx = Ctx::buffered();
        ctx.write_output(b"Hello, ").unwrap();
        ctx.write_output(b"World!").unwrap();
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"Hello, World!"[..])
        );
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
    }

    #[test]
    fn a_discarding_sink_reports_nothing_buffered() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.write_output(b"gone").unwrap();
        assert!(ctx.take_buffered_output().is_none());
    }

    /// A configuration from the text an operator would have written rather than
    /// from the typed tree — the boot path deserializes, so a case that built
    /// the struct could pin a value no configuration file can express.
    fn configured(written: &str) -> nvs_config::Request {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        nvs_config::Request::new(std::sync::Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        }))
    }

    /// [`LogWriter`]'s own decision: with no request underneath it, a record
    /// the configuration named no target for goes to the engine's channel.
    /// [`LogChannel::Output`] is not a second answer this writer could give —
    /// there is no program whose output it would be — so a tree that named
    /// nothing and one that named a target this build does not transport both
    /// land on stderr.
    #[test]
    fn a_writer_holding_no_request_sends_an_unconfigured_record_to_the_engines_channel() {
        assert!(
            matches!(LogWriter::resolve(None).sink, OutputSink::Stderr),
            "a writer with no configuration under it routed somewhere other than the engine's own \
             channel"
        );
        let syslog = configured("[log]\ntarget = \"syslog\"\n");
        assert!(
            matches!(LogWriter::resolve(Some(&syslog)).sink, OutputSink::Stderr),
            "a target this build does not transport reached a sink instead of the engine's channel"
        );
    }

    /// The directives are read once and answer the same way they do for a
    /// context: the target names the sink, the level is a floor the render is
    /// dropped below, and the format selects the rendering. The file target is
    /// opened on its first write, so resolving one here touches no disk.
    #[test]
    fn a_writer_reads_the_same_three_directives_a_context_does() {
        let config =
            configured("[log]\ntarget = \"file:nvs.log\"\nlevel = \"warn\"\nformat = \"text\"\n");
        let writer = LogWriter::resolve(Some(&config));
        assert!(
            matches!(writer.sink, OutputSink::File(_)),
            "`file:` named a rotating file and the writer resolved something else"
        );
        assert!(
            writer.render(&Record::at(Level::Info)).is_none(),
            "a record below `[log] level` was rendered anyway"
        );

        let record = Record::at(Level::Warn);
        let text = writer.render(&record).expect("`warn` is at the floor");
        let json = LogWriter::resolve(None)
            .render(&record)
            .expect("nothing configured writes every level");
        assert!(json.starts_with('{'), "the unset default is JSON Lines");
        assert_ne!(
            json, text,
            "`format = \"text\"` did not reach the rendering"
        );
    }
}
