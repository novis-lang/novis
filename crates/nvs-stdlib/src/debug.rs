//! `Core\Debug` — [ADR 0092](../../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
//! § 4's `dump` and `render`, and the walk that turns a runtime value into
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
//! [ADR 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
//! § 5's rule verbatim. Because it answers a carrier, `echo Core\Debug::render($x)`
//! is singly escaped: those bytes have already been through the record's own
//! transformations, and the carrier is what stops the next `echo` escaping
//! them again.
//!
//! # What the walk decides, and what it does not
//!
//! Every one of ADR 0092 § 5's four transformations is the *model*'s, not
//! this module's: control bytes and bidi are `nvs_render::Rendered`'s
//! constructor, elision is a `nvs_render::Elision` node, and a cycle is a
//! `nvs_render::Node::Cycle`. What this walk decides is the two things only a
//! runtime value can answer — which node kind a tag denotes, and which
//! property is `secret` — and it applies the caps it was given rather than
//! choosing them.
//!
//! # Known gaps
//!
//! 1. **A `secret` value reaches this walk without a property to be declared
//!    on.** ADR 0092 § 5's redaction row is closed at both of its own ends —
//!    a `secret` argument is refused where the call is written
//!    (`nvs_types::expr::quals::reject_secret_debug_argument`) and a
//!    `secret`-typed *property* is a [`Node::Redacted`](nvs_render::Node::Redacted),
//!    read off `nvs_runtime::ClassDesc::field_is_secret` — but neither end
//!    reaches a `secret` value held in an `array<T>` element or in an ADR 0036
//!    shape literal's field. Both are ADR 0033's unmodelled container axis
//!    rather than a hole here: the element type of an array of `secret string`
//!    is not something the qualifier composes onto today, and a shape field's
//!    type is *inferred* from its initializer rather than declared, so there
//!    is no declaration for the bit to be carried off. A `secret` property of
//!    a *nested* object is redacted, that object's own class having declared
//!    it.
//! 2. **An enum case dumps as its backing integer.** ADR 0010 § 5 gives an
//!    enum no tag of its own — it *is* an `int` at run time — so a case
//!    arriving through `mixed` is indistinguishable from one here.
//!    `nvs_render::Node::EnumCase` exists and is what a producer with a static
//!    type would build; reaching it from a dump wants the tag roster to
//!    distinguish an enum, which is a representation change ADR 0010 § 5
//!    deliberately declined.
//! 3. **The `Throwable` producer is not here.** ADR 0092 § 6 makes an uncaught
//!    `Throwable` a record at `Error` with its frames as Sequence-of-Object
//!    nodes, and that walk belongs to `nvs-runtime`'s fatal path rather than
//!    to a `Core` member — see `nvs_render`'s own § *Where this sits* for the
//!    crate edge it needs first.

use nvs_render::{Caps, Elision, Level, Node, Record, Rendered, Scalar};
use nvs_runtime::{Fault, NvsObj, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// The class's fully-qualified name.
pub(crate) const NAME: &str = r"Core\Debug";

/// Spec § 16's `Core\Debug`, as much of it as ADR 0092 § 4 declares.
///
/// The coverage, trace and profile members that section also lists are
/// [ADR 0018](../../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s
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

/// `Core\Debug::dump`'s reference card — ADR 0117.
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

/// `Core\Debug::render`'s reference card — ADR 0117.
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
    /// `"0"`, `"1"`, … — [`crate::registry::CoreTy::Variadic`] owns why — so
    /// this is an ordinary one-slot helper.
    ///
    /// A dump with no arguments at all writes nothing rather than an empty
    /// line: `Core\Debug::dump()` says nothing, and a blank line in a build
    /// log is worse than silence.
    fn nvs_core_debug_dump(ctx, args: [1]) {
        let record = record_of(&args[0])?;
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
    /// ADR 0088 § 5's rule verbatim: bytes that have been through a sink
    /// cannot be handed back as a `string` without the next `echo` escaping
    /// them a second time, so this answers what `Core\Out::capture` answers.
    fn nvs_core_debug_render(_ctx, args: [1]) {
        let node = node_of(args[0], &Caps::default(), 0, &mut Seen::default());
        // Without the trailing newline `dump` writes: this member answers a
        // *value*, and a caller composing one into a larger output decides
        // where the line ends. `dump` is the one that writes a line.
        let rendered = nvs_render::plain::render_nodes(std::slice::from_ref(&node));
        Ok(crate::cli::built(Value::str(nvs_runtime::NvsStr::new(
            rendered.trim_end_matches('\n').as_bytes(),
        ))))
    }
}

/// ADR 0092 § 6's *"a record at `Debug`, one node per argument"* — the whole
/// of what `dump` produces, and the shape M8's log record is built from too.
fn record_of(tail: &Value) -> Result<Record, Fault> {
    let mut record = Record::at(Level::Debug);
    let caps = Caps::default();
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
        record
            .nodes
            .push(node_of(value, &caps, 0, &mut Seen::default()));
        from = slot + 1;
    }
    Ok(record)
}

/// The objects this walk has already entered, so a graph that points back at
/// one becomes a [`Node::Cycle`] rather than an infinite traversal.
///
/// Keyed by the allocation's address, which is `nvs_runtime::identity`'s own
/// answer for an object (ADR 0090 § 3 lowers object equality to it) — so the
/// id a `Cycle` names is an identity rather than a position, which is what
/// lets the HTML rendering link the repeat.
#[derive(Default)]
struct Seen(Vec<usize>);

impl Seen {
    /// The record-local id of `address` if this walk is already inside it, or
    /// `None` having recorded it as entered.
    fn enter(&mut self, address: usize) -> Option<usize> {
        if let Some(index) = self.0.iter().position(|seen| *seen == address) {
            return Some(index + 1);
        }
        self.0.push(address);
        None
    }

    /// Leaves the object entered last — a *sibling* that repeats an object is
    /// not a cycle, only an ancestor is.
    fn leave(&mut self) {
        self.0.pop();
    }

    /// The id the object entered last was given.
    fn depth(&self) -> usize {
        self.0.len()
    }
}

/// One value as a node, at `depth` levels of container below the record's root.
fn node_of(value: Value, caps: &Caps, depth: usize, seen: &mut Seen) -> Node {
    match value.tag() {
        None | Some(Tag::Null) | Some(Tag::Unset) => Node::Scalar(Scalar::Null),
        Some(Tag::Bool) => Node::Scalar(Scalar::Bool(value.as_bool() == Some(true))),
        Some(Tag::Int) => Node::Scalar(Scalar::Int(value.as_int().unwrap_or(0))),
        Some(Tag::Uint) => Node::Scalar(Scalar::Uint(value.as_uint().unwrap_or(0))),
        Some(Tag::Float) => Node::Scalar(Scalar::Float(value.as_float().unwrap_or(f64::NAN))),
        Some(Tag::Decimal) => Node::Scalar(Scalar::Decimal(
            value
                .as_decimal()
                .map_or_else(|| "0".to_owned(), |d| d.to_string()),
        )),
        Some(Tag::Str) => text_node(value.as_str_bytes().unwrap_or_default(), caps),
        Some(Tag::Bytes) => bytes_node(value.as_bytes().unwrap_or_default(), caps),
        Some(Tag::Array) => array_node(value, caps, depth, seen),
        Some(Tag::Object) => object_node(value, caps, depth, seen),
        // Neither tag has a representation behind it — `nvs_runtime::Tag`'s own
        // known gap 1 — so nothing can hold one and this is unreachable rather
        // than unhandled. It renders as a redaction rather than as a wrong
        // value, which is the direction a dump should fail in.
        Some(Tag::Closure | Tag::Resource) => Node::Redacted,
    }
}

/// A `string`, cut at [`Caps::text`].
///
/// The bytes are decoded lossily rather than refused: a `string` is UTF-8 by
/// ADR 0009's promise, so a run that is not is a value that came from outside
/// the type system, and a dump is exactly the tool a developer reaches for to
/// see one.
fn text_node(bytes: &[u8], caps: &Caps) -> Node {
    if bytes.len() <= caps.text {
        return Node::Scalar(Scalar::Str {
            text: Rendered::new(&String::from_utf8_lossy(bytes)),
            bytes: bytes.len(),
        });
    }
    // Cut on a character boundary, so the kept prefix is still text.
    let mut end = caps.text;
    while end > 0 && !is_char_boundary(bytes, end) {
        end -= 1;
    }
    Node::Elided(Elision::Text {
        kept: Rendered::new(&String::from_utf8_lossy(&bytes[..end])),
        cut: bytes.len() - end,
    })
}

/// Whether `index` starts a UTF-8 sequence in `bytes` — `str::is_char_boundary`
/// over a slice that is not yet known to be one.
fn is_char_boundary(bytes: &[u8], index: usize) -> bool {
    bytes
        .get(index)
        .is_none_or(|byte| byte & 0b1100_0000 != 0b1000_0000)
}

/// A `bytes` value, cut at [`Caps::text`]. § 5's substitution does not apply:
/// it is a transformation of *text*, and these are not.
fn bytes_node(bytes: &[u8], caps: &Caps) -> Node {
    if bytes.len() <= caps.text {
        return Node::Scalar(Scalar::Bytes(bytes.to_vec()));
    }
    Node::Elided(Elision::Entries {
        total: bytes.len(),
        cut: bytes.len() - caps.text,
    })
}

/// An `array`, as ADR 0092 § 1's Sequence or Map.
///
/// The two shapes are one runtime type, so which one this is is decided from
/// the keys: an array whose keys are `"0"`, `"1"`, … in order is the list
/// shape and renders by position, and anything else renders by key. That is
/// the same reading `Core\Json::encode` already makes of the same value, so a
/// dump and an encode do not disagree about what an array *is*.
fn array_node(value: Value, caps: &Caps, depth: usize, seen: &mut Seen) -> Node {
    let Some(ptr) = value.array_ptr() else {
        return Node::Scalar(Scalar::Null);
    };
    if depth >= caps.depth {
        return Node::Elided(Elision::Depth);
    }
    let array = crate::arr::borrowed(ptr);
    let mut entries = Vec::new();
    let mut is_list = true;
    let mut total = 0usize;
    let mut from = 0usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        let index = total;
        total += 1;
        let Some(key) = array.key_at(slot) else {
            continue;
        };
        let key = String::from_utf8_lossy(key.as_bytes()).into_owned();
        if key != index.to_string() {
            is_list = false;
        }
        if index >= caps.entries {
            continue;
        }
        let element = array
            .value_at(slot)
            .map_or(Node::Scalar(Scalar::Null), |element| {
                node_of(element, caps, depth + 1, seen)
            });
        entries.push((key, element));
    }
    let cut = total.saturating_sub(entries.len());
    let mut node = if is_list {
        Node::Sequence(entries.into_iter().map(|(_, value)| value).collect())
    } else {
        Node::Map(
            entries
                .into_iter()
                .map(|(key, value)| (Rendered::new(&key), value))
                .collect(),
        )
    };
    if cut > 0 {
        node = append_cut(node, total, cut);
    }
    node
}

/// Puts the [`Elision::Entries`] node after the entries that were kept, which
/// is where a reader expects to meet it.
fn append_cut(node: Node, total: usize, cut: usize) -> Node {
    let elided = Node::Elided(Elision::Entries { total, cut });
    match node {
        Node::Sequence(mut items) => {
            items.push(elided);
            Node::Sequence(items)
        }
        Node::Map(mut entries) => {
            entries.push((Rendered::new("…"), elided));
            Node::Map(entries)
        }
        other => other,
    }
}

/// A class instance, as ADR 0092 § 1's Object node — the class name and its
/// **declared** properties, per ADR 0028 § 4.
///
/// Never a `toString` result and never a customization hook (§ 7): a dump
/// shows a class's real declared properties and their real current values,
/// with § 5's transformations and nothing else.
fn object_node(value: Value, caps: &Caps, depth: usize, seen: &mut Seen) -> Node {
    let Some(ptr) = value.obj_ptr() else {
        return Node::Scalar(Scalar::Null);
    };
    if let Some(id) = seen.enter(ptr as usize) {
        return Node::Cycle { id };
    }
    let node = object_body(value, ptr, caps, depth, seen);
    seen.leave();
    node
}

/// [`object_node`]'s body, split out so the [`Seen::leave`] above pairs with
/// its [`Seen::enter`] on every path this takes.
fn object_body(
    value: Value,
    ptr: *mut nvs_runtime::ObjHeader,
    caps: &Caps,
    depth: usize,
    seen: &mut Seen,
) -> Node {
    #[expect(
        unsafe_code,
        reason = "the value owns a reference to a live allocation, so it is live \
                  for this borrow; the handle is never dropped, so the reference \
                  is not released twice"
    )]
    let object = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
    let class = object.class_name().to_owned();
    let id = Some(seen.depth());

    // ADR 0031 § 2 gives a closure no user-visible state at all — the fields
    // are its captures, and a dump that showed them would be showing an
    // implementation. A synthesized closure class is named `{owner}$fn{n}` by
    // `nvs_types::expr::calls`, and `$` cannot appear in a declared name
    // (ADR 0029), so the marker is unambiguous.
    if class.contains("$fn") {
        let parameters = nvs_runtime::closure_arity(value).unwrap_or(0);
        return Node::Closure { parameters };
    }
    if depth >= caps.depth {
        return Node::Elided(Elision::Depth);
    }

    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by the unit's class table, which \
                  outlives every instance of the class it describes"
    )]
    let desc = unsafe { &*object.class() };
    let mut properties = Vec::new();
    for slot in 0..object.field_count() {
        let name = desc
            .field_name(slot)
            .map_or_else(|| slot.to_string(), ToOwned::to_owned);
        // ADR 0092 § 5's redaction row: the *declared* type decides, so the
        // value is never walked at all rather than walked and then discarded
        // — a `secret` object's own properties are not read, and a `secret`
        // string contributes no elision node saying how long it was.
        let node = if desc.field_is_secret(slot) {
            Node::Redacted
        } else {
            node_of(object.field(slot), caps, depth + 1, seen)
        };
        properties.push((name, node));
    }
    Node::Object {
        class,
        id,
        properties,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two rows ADR 0092 § 4 declares, as the registry states them — what
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

    /// Every scalar tag reaches the node kind that names its own Novis type,
    /// which is the whole reason § 1's model tags them.
    #[test]
    fn a_scalar_becomes_its_own_node() {
        let caps = Caps::default();
        let node = |value| node_of(value, &caps, 0, &mut Seen::default());
        assert_eq!(node(Value::null()), Node::Scalar(Scalar::Null));
        assert_eq!(node(Value::bool(true)), Node::Scalar(Scalar::Bool(true)));
        assert_eq!(node(Value::int(-3)), Node::Scalar(Scalar::Int(-3)));
        assert_eq!(node(Value::uint(3)), Node::Scalar(Scalar::Uint(3)));
        assert_eq!(node(Value::float(0.5)), Node::Scalar(Scalar::Float(0.5)));
    }

    /// § 5's substitution is the *model*'s, so a control byte in a dumped
    /// string is already neutralized by the time any rendering sees it — the
    /// CWE-117 property, asserted at the producer.
    #[test]
    fn a_control_byte_is_substituted_on_the_way_into_the_record() {
        let node = text_node(b"a\rb", &Caps::default());
        let Node::Scalar(Scalar::Str { text, bytes }) = node else {
            panic!("a short string is a scalar node");
        };
        assert_eq!(text.as_str(), "a\u{240D}b");
        assert_eq!(bytes, 3);
    }

    /// ADR 0092's own M4 verification bullet: `Core\Debug::dump` writes to
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
        nvs_runtime::call(nvs_core_debug_dump, &mut ctx, &[tail]).expect("a dump cannot fail");
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

    /// A cut is a node, so every rendering shows the same cut — and the kept
    /// prefix stays valid text.
    #[test]
    fn an_over_long_string_is_cut_into_an_elision() {
        let caps = Caps {
            text: 4,
            ..Caps::default()
        };
        assert_eq!(
            text_node(b"abcdefg", &caps),
            Node::Elided(Elision::Text {
                kept: Rendered::new("abcd"),
                cut: 3
            })
        );
        // A multi-byte character straddling the cap is dropped whole rather
        // than halved.
        assert_eq!(
            text_node("abcé".as_bytes(), &caps),
            Node::Elided(Elision::Text {
                kept: Rendered::new("abc"),
                cut: 2
            })
        );
    }
}
