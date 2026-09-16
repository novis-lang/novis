//! `rule:errors/renderings`'s plaintext rendering — the one the terminal sink selects.
//!
//! # What it looks like, and why
//!
//! Indented, one value per line, and **typed on every line**: `int(42)` and
//! `string(1) "1"` are two different lines, which is the distinction
//! `print_r` cannot make and the reason PHP developers reach for `var_dump`
//! whenever the answer matters. The layout is `var_dump`'s in shape, because
//! that is the one PHP output a reader already parses at a glance, and Novis's
//! in vocabulary: `uint`, `decimal` and `bytes` are their own lines, an enum
//! case renders as `Enum::Case` rather than as its backing integer, and a
//! closure renders as a signature rather than as a `Closure` object with
//! invisible captures.
//!
//! # No colour yet
//!
//! `rule:errors/renderings` makes the plaintext rendering coloured *iff*
//! `Cli::colorDepth() != None`, and reads that one answer from
//! `rule:tooling/the-terminal-profile-resolves-once` rather
//! than resolving `NO_COLOR`/`CLICOLOR_FORCE`/`TERM` again here. `Core\Cli`
//! does not exist yet (`nvs_stdlib::cli`'s own gap 2), so this
//! rendering is uncoloured and there is deliberately no second resolution of
//! that question standing in for it — structure, substitution and redaction
//! never vary with a tty, so nothing about *what* is rendered waits on it.
//!
//! # The renderings agree because the cuts are in the model
//!
//! Nothing here decides what to truncate. An [`Elision`] arrived as a node,
//! and this rendering prints it — so the JSON and HTML renderings print the
//! same cut of the same value rather than each choosing its own.

use crate::{Elision, Envelope, Node, Record, Scalar};

/// One level of indentation.
const INDENT: &str = "  ";

/// A whole [`Record`] — the envelope's own line, then its fields, then its
/// nodes.
///
/// An absent envelope field is omitted rather than rendered empty, per
/// [`Envelope`]'s own docs, so a producer with nothing but a level prints one
/// short header line.
#[must_use]
pub fn render(record: &Record) -> String {
    let mut out = String::new();
    out.push_str(&header(&record.envelope));
    out.push('\n');
    for (name, node) in &record.envelope.fields {
        out.push_str(INDENT);
        out.push_str(name);
        out.push_str(" => ");
        write_node(&mut out, node, 1);
        out.push('\n');
    }
    out.push_str(&render_nodes(&record.nodes));
    out
}

/// Just the nodes, at the left margin and one per line — what
/// `Core\Debug::dump` writes to stderr.
///
/// A dump renders no envelope because `rule:errors/debug-dump` sends the CLI form straight
/// to stderr rather than through the log target: there is no `ts`, no
/// `request_id` and no level for a reader to want, and printing an empty
/// header above every dumped value would be noise on the one output a
/// developer reads most often. The record still *has* an envelope — that is
/// what the log record built from the same walk carries.
#[must_use]
pub fn render_nodes(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        write_node(&mut out, node, 0);
        out.push('\n');
    }
    out
}

/// The envelope's one line: the level, then each present field in the order
/// `rule:errors/diagnostic-record`'s table writes them.
///
/// `rule:http-server/the-floor-cannot-fill-the-disk`'s `count` closes the line as `x37` rather than as a
/// `name=value` like its neighbours: it is a multiplier on the line it trails,
/// not another identifier to read, and a reader tailing a log wants it where
/// the eye already is.
fn header(envelope: &Envelope) -> String {
    let mut parts = vec![format!("[{}]", envelope.level.name())];
    if let Some(ts) = &envelope.ts {
        parts.push(ts.clone());
    }
    if let Some(message) = &envelope.message {
        parts.push(message.as_str().to_owned());
    }
    if let Some(request_id) = &envelope.request_id {
        parts.push(format!("request={request_id}"));
    }
    if let Some(trace_id) = &envelope.trace_id {
        parts.push(format!("trace={trace_id}"));
    }
    if let Some(span_id) = &envelope.span_id {
        parts.push(format!("span={span_id}"));
    }
    if let Some(source) = &envelope.source {
        let member = source
            .member
            .as_ref()
            .map_or_else(String::new, |m| format!(" in {m}"));
        parts.push(format!("at {}:{}{member}", source.file, source.line));
    }
    if let Some(count) = envelope.count {
        parts.push(format!("x{count}"));
    }
    parts.join(" ")
}

/// Writes `node` at `depth` levels of indentation, opening its own line but
/// not closing it — a container writes its children's newlines itself.
fn write_node(out: &mut String, node: &Node, depth: usize) {
    match node {
        Node::Scalar(scalar) => out.push_str(&scalar_line(scalar)),
        Node::Sequence(items) => {
            out.push_str(&format!("array({}) [\n", items.len()));
            for (index, item) in items.iter().enumerate() {
                indent(out, depth + 1);
                out.push_str(&format!("{index} => "));
                write_node(out, item, depth + 1);
                out.push('\n');
            }
            indent(out, depth);
            out.push(']');
        }
        Node::Map(entries) => {
            out.push_str(&format!("array({}) [\n", entries.len()));
            for (key, value) in entries {
                indent(out, depth + 1);
                out.push_str(&format!("{:?} => ", key.as_str()));
                write_node(out, value, depth + 1);
                out.push('\n');
            }
            indent(out, depth);
            out.push(']');
        }
        Node::Object {
            class,
            id,
            properties,
        } => {
            let id = id.map_or_else(String::new, |id| format!("#{id}"));
            out.push_str(&format!("{class}{id} ({}) {{\n", properties.len()));
            for (name, value) in properties {
                indent(out, depth + 1);
                out.push_str(&format!("${name} => "));
                write_node(out, value, depth + 1);
                out.push('\n');
            }
            indent(out, depth);
            out.push('}');
        }
        Node::EnumCase { enum_name, case } => out.push_str(&format!("{enum_name}::{case}")),
        Node::Closure { parameters } => {
            let plural = if *parameters == 1 { "" } else { "s" };
            out.push_str(&format!("callable({parameters} parameter{plural})"));
        }
        Node::Redacted => out.push_str("[redacted]"),
        Node::Elided(elision) => out.push_str(&elision_line(elision)),
        Node::Cycle { id } => out.push_str(&format!("[cycle -> #{id}]")),
        Node::Span { file, line, label } => {
            out.push_str(&format!("{file}:{line}: {}", label.as_str()));
        }
        Node::Frame {
            depth,
            function,
            file,
            line,
        } => {
            out.push_str(&format!("#{depth} {}()", function.as_str()));
            if let Some(file) = file {
                out.push_str(&format!(" at {file}"));
                if let Some(line) = line {
                    out.push_str(&format!(":{line}"));
                }
            }
        }
    }
}

/// One scalar's whole line, tagged with the Novis type it is.
fn scalar_line(scalar: &Scalar) -> String {
    match scalar {
        Scalar::Null => "null".to_owned(),
        Scalar::Bool(value) => format!("bool({value})"),
        Scalar::Int(value) => format!("int({value})"),
        Scalar::Uint(value) => format!("uint({value})"),
        Scalar::Float(value) => format!("float({value})"),
        Scalar::Decimal(value) => format!("decimal({value})"),
        Scalar::Str { text, bytes } => format!("string({bytes}) {:?}", text.as_str()),
        Scalar::Bytes(bytes) => format!("bytes({}) 0x{}", bytes.len(), hex(bytes)),
    }
}

/// An [`Elision`]'s whole line — the cut named, and for a text cut the kept
/// prefix printed beside it so the reader sees the value *and* what is
/// missing.
fn elision_line(elision: &Elision) -> String {
    match elision {
        Elision::Depth => "[elided: below the depth cap]".to_owned(),
        Elision::Entries { total, cut } => {
            format!("[elided: {cut} of {total} entries]")
        }
        Elision::Text { kept, cut } => {
            format!(
                "string({}+) {:?} [elided: {cut} bytes]",
                kept.as_str().len(),
                kept.as_str()
            )
        }
    }
}

/// `bytes` as lower-case hex, which is the one rendering of binary that is
/// exact, fixed-width and needs no encoding decision.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Pushes `depth` levels of indentation.
fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str(INDENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Caps, Level, Rendered, Source};

    fn line(node: Node) -> String {
        render_nodes(&[node]).trim_end().to_owned()
    }

    /// Every scalar names its own Novis type, which is the whole reason the
    /// model tags them: `int(1)` and `string(1) "1"` are two lines, and
    /// `print_r` writes one.
    #[test]
    fn a_scalar_names_its_type() {
        assert_eq!(line(Node::Scalar(Scalar::Null)), "null");
        assert_eq!(line(Node::Scalar(Scalar::Bool(true))), "bool(true)");
        assert_eq!(line(Node::Scalar(Scalar::Int(-1))), "int(-1)");
        assert_eq!(line(Node::Scalar(Scalar::Uint(18))), "uint(18)");
        assert_eq!(line(Node::Scalar(Scalar::Float(1.5))), "float(1.5)");
        assert_eq!(
            line(Node::Scalar(Scalar::Decimal("1.50".to_owned()))),
            "decimal(1.50)"
        );
        assert_eq!(
            line(Node::Scalar(Scalar::Str {
                text: Rendered::new("1"),
                bytes: 1
            })),
            "string(1) \"1\""
        );
        assert_eq!(
            line(Node::Scalar(Scalar::Bytes(vec![0x61, 0xFF]))),
            "bytes(2) 0x61ff"
        );
    }

    /// A container indents its entries and closes at its own level, so nesting
    /// reads without counting brackets.
    #[test]
    fn a_container_indents_its_entries() {
        let node = Node::Map(vec![(
            Rendered::new("a"),
            Node::Sequence(vec![Node::Scalar(Scalar::Int(1))]),
        )]);
        assert_eq!(
            line(node),
            "array(1) [\n  \"a\" => array(1) [\n    0 => int(1)\n  ]\n]"
        );
    }

    /// An object names its class and its identity, and its properties carry
    /// the `$` sigil the declaration writes.
    #[test]
    fn an_object_names_its_class_and_identity() {
        let node = Node::Object {
            class: "Point".to_owned(),
            id: Some(1),
            properties: vec![("x".to_owned(), Node::Scalar(Scalar::Int(1)))],
        };
        assert_eq!(line(node), "Point#1 (1) {\n  $x => int(1)\n}");
    }

    /// The node kinds a rendering must not invent an answer for: what was
    /// cut, what was hidden and what repeated all arrived as nodes.
    #[test]
    fn a_cut_a_redaction_and_a_cycle_are_rendered_as_themselves() {
        assert_eq!(line(Node::Redacted), "[redacted]");
        assert_eq!(line(Node::Cycle { id: 2 }), "[cycle -> #2]");
        assert_eq!(
            line(Node::Elided(Elision::Depth)),
            "[elided: below the depth cap]"
        );
        assert_eq!(
            line(Node::Elided(Elision::Entries {
                total: 140,
                cut: 40
            })),
            "[elided: 40 of 140 entries]"
        );
        assert_eq!(
            line(Node::Elided(Elision::Text {
                kept: Rendered::new("ab"),
                cut: 30
            })),
            "string(2+) \"ab\" [elided: 30 bytes]"
        );
    }

    /// An enum case renders as its name, never as the integer `rule:types/literal-types`
    /// spends no representation on hiding, and a closure as its signature
    /// rather than as an object with invisible captures.
    #[test]
    fn an_enum_case_and_a_closure_render_as_what_they_are() {
        assert_eq!(
            line(Node::EnumCase {
                enum_name: "Mode".to_owned(),
                case: "Read".to_owned()
            }),
            "Mode::Read"
        );
        assert_eq!(
            line(Node::Closure { parameters: 2 }),
            "callable(2 parameters)"
        );
        assert_eq!(
            line(Node::Closure { parameters: 1 }),
            "callable(1 parameter)"
        );
    }

    /// An absent envelope field is omitted rather than rendered empty, so a
    /// producer that has no request to name prints one short header.
    #[test]
    fn an_absent_envelope_field_is_omitted() {
        let mut record = Record::at(Level::Warn);
        assert_eq!(render(&record).trim_end(), "[warn]");
        record.envelope.message = Some(Rendered::new("bound publicly"));
        record.envelope.source = Some(Source {
            file: "app.nvs".to_owned(),
            line: 12,
            member: Some("Main::main".to_owned()),
        });
        assert_eq!(
            render(&record).trim_end(),
            "[warn] bound publicly at app.nvs:12 in Main::main"
        );
    }

    /// The caps are the model's, not a rendering's — this rendering reads none
    /// of them, which is what makes every rendering agree on a cut.
    #[test]
    fn the_caps_are_not_a_rendering_input() {
        let caps = Caps::default();
        assert_eq!((caps.depth, caps.entries, caps.text), (8, 100, 1024));
    }
}
