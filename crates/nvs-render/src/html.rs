//! `rule:errors/renderings`' HTML rendering of a [`Record`], and the HTML
//! sink's own transform, which that rendering and every other write through
//! that sink go out through.
//!
//! # The rendering
//!
//! A record becomes one element — a `<div class="nvs-record">` holding the
//! envelope's header, the producer's fields and the record's own nodes — and
//! it is what `rule:errors/renderings`' table asks for on the three counts it
//! names. **Collapsible**: every container is a `<details>`/`<summary>` pair,
//! so an entity graph is one summary line until a reader opens it. **Typed**:
//! a scalar carries the Novis type the model tagged it with, in its text as
//! [`crate::plain`] writes it and in its class for a stylesheet to reach.
//! **Class-aware**: an object names its class, anchors its identity, and a
//! [`Node::Cycle`] is a link to that anchor rather than a `*RECURSION*`
//! string — which is what `rule:errors/record-transformations` keeps a cycle
//! an identity rather than a marker for.
//!
//! Every `<details>` is written `open`. Collapsing is an affordance the reader
//! reaches for, not a cut: content hidden by default is content the other two
//! renderings show, and three views that disagree about what is *in* a record
//! are the failure the model exists to prevent. What was genuinely cut arrived
//! as a [`Node::Elided`] and is rendered as the cut it is, here as everywhere.
//!
//! **Class names and nothing else** — no inline style, no script, no colour.
//! The text reads without a stylesheet, and the caller that has a response
//! supplies one: `[debug] inline`'s block carries its style under the CSP
//! nonce (`docs/decisions/0092.md:423`), which is a property of that request
//! rather than of the markup. An identity is numbered within its own record,
//! so a page that embeds two records is what scopes their anchors.
//!
//! The carrier is `Core\Html\Markup`, which this crate cannot name: it is a
//! leaf and the class is `nvs-stdlib`'s. [`render`] answers a `String` and the
//! caller holding the sink wraps it, exactly as [`crate::plain::render`]
//! answers the bytes a `Cli\Text` is built from.
//!
//! # The sink transform
//!
//! [`escape`] is `rule:core-classes/html-auto-escape`'s escape, written as a
//! table beside [`crate::text::substitute`]'s. The two are peers: each is the
//! whole of what one sink does to text on the way in, each answers a borrow
//! for the input that needs nothing done to it, and each is called rather than
//! restated by everything that writes through that sink. `nvs_runtime`'s
//! `echo` picks between them from the sink in force, and `Core\Html::escape`
//! is a caller of this one rather than a second copy of it — which is what
//! makes the launderer and the sink incapable of disagreeing about what an `&`
//! becomes.
//!
//! The rendering above is a caller too: every piece of model text it writes
//! goes through [`escape`], so a dumped `<script>` reaches the page as
//! `&lt;script&gt;` without the rendering owning a second answer.
//!
//! [`Record`]: crate::Record
//! [`Node::Cycle`]: crate::Node::Cycle
//! [`Node::Elided`]: crate::Node::Elided

use std::borrow::Cow;

use crate::{Elision, Envelope, Node, Record, Scalar};

/// The prefix an object's anchor and a cycle's link share, so the two are
/// written once and cannot come to disagree.
const ANCHOR: &str = "nvs-id-";

/// What each of the five characters is written as.
///
/// A `match` rather than a table because it is the whole of the transformation
/// and the compiler turns it into one: five arms over an ASCII byte.
fn escaped(c: char) -> Option<&'static str> {
    match c {
        // First, and the only one whose escape is not about a delimiter: an
        // unescaped `&` makes every other reference in the output ambiguous.
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '"' => Some("&quot;"),
        // Not `&apos;` — that name is XML's and HTML 4 never defined it.
        '\'' => Some("&#39;"),
        _ => None,
    }
}

/// What an unterminated directional control becomes.
///
/// The same replacement [`crate::text`] uses for it, and deliberately not a
/// character reference: the control is being *removed*, not shown, and
/// `&#8235;` in the output would be an escaped payload rather than a neutral
/// one.
const REPLACEMENT: char = '\u{FFFD}';

/// `text` with `&`, `<`, `>`, `"` and `'` written as character references and
/// every unterminated bidirectional control replaced.
///
/// # Why a [`Cow`] rather than a `String`
///
/// [`crate::text::substitute`]'s reason one sink over, and a stronger one:
/// `rule:core-classes/html-auto-escape` makes this the HTML sink's *only*
/// behaviour, so every non-carrier value written into a response passes
/// through here whether or not it is tainted. Text with nothing to escape is
/// therefore not an edge case but most of a page, and answering the borrow
/// keeps it at one scan and no allocation — which is what makes a rule that
/// cannot be switched off affordable on the request path, where latency is
/// what it would cost.
#[must_use]
pub fn escape(text: &str) -> Cow<'_, str> {
    // Both halves are a scan and neither fires on ordinary text, so they are
    // asked before anything is allocated.
    let mut unterminated = Vec::new();
    crate::bidi::for_each_unterminated(text, |offset, _| unterminated.push(offset));
    if unterminated.is_empty() && !text.chars().any(|c| escaped(c).is_some()) {
        return Cow::Borrowed(text);
    }

    // Every escape is longer than what it replaces, so the input's length is a
    // floor and never a wasted reservation.
    let mut out = String::with_capacity(text.len());
    let mut cuts = unterminated.into_iter().peekable();
    for (offset, c) in text.char_indices() {
        if cuts.peek() == Some(&offset) {
            cuts.next();
            out.push(REPLACEMENT);
            continue;
        }
        match escaped(c) {
            Some(reference) => out.push_str(reference),
            None => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// A whole [`Record`] as one element: the envelope's header, then the
/// producer's fields, then the record's own nodes.
///
/// An absent envelope field is omitted rather than rendered empty, per
/// [`Envelope`]'s own docs, and a record with no fields and no nodes is its
/// header alone.
#[must_use]
pub fn render(record: &Record) -> String {
    let mut out = format!(
        "<div class=\"nvs-record nvs-level-{}\">",
        record.envelope.level.name()
    );
    write_header(&mut out, &record.envelope);
    if !record.envelope.fields.is_empty() {
        out.push_str("<ul class=\"nvs-fields\">");
        for (name, node) in &record.envelope.fields {
            write_entry(&mut out, name, node);
        }
        out.push_str("</ul>");
    }
    if !record.nodes.is_empty() {
        out.push_str(&render_nodes(&record.nodes));
    }
    out.push_str("</div>");
    out
}

/// Just the nodes, as one element — what `Core\Debug::dump` hands the HTML
/// sink when that is the sink in force.
///
/// One element rather than a run of them because the carrier holds a value, and
/// a caller composing it into a page should not have to know how many nodes the
/// producer wrote. A dump renders no envelope for [`crate::plain::render_nodes`]'s
/// reason: there is no `ts`, no `request_id` and no level a reader of a dumped
/// value wants.
#[must_use]
pub fn render_nodes(nodes: &[Node]) -> String {
    let mut out = String::from("<div class=\"nvs-nodes\">");
    for node in nodes {
        write_node(&mut out, node);
    }
    out.push_str("</div>");
    out
}

/// The envelope's own row: the level, then each present field in the order
/// `rule:errors/diagnostic-record`'s table writes them.
///
/// Each part keeps the label [`crate::plain`] prints — `request=…`, `x37` —
/// rather than moving it into the class alone, so the header still says which
/// identifier is which when the markup is read with no stylesheet.
fn write_header(out: &mut String, envelope: &Envelope) {
    out.push_str("<div class=\"nvs-envelope\">");
    span(out, "nvs-level", envelope.level.name());
    if let Some(ts) = &envelope.ts {
        span(out, "nvs-ts", ts);
    }
    if let Some(message) = &envelope.message {
        span(out, "nvs-message", message.as_str());
    }
    if let Some(request_id) = &envelope.request_id {
        span(out, "nvs-request-id", &format!("request={request_id}"));
    }
    if let Some(trace_id) = &envelope.trace_id {
        span(out, "nvs-trace-id", &format!("trace={trace_id}"));
    }
    if let Some(span_id) = &envelope.span_id {
        span(out, "nvs-span-id", &format!("span={span_id}"));
    }
    if let Some(source) = &envelope.source {
        let member = source
            .member
            .as_ref()
            .map_or_else(String::new, |m| format!(" in {m}"));
        span(
            out,
            "nvs-source",
            &format!("at {}:{}{member}", source.file, source.line),
        );
    }
    if let Some(count) = envelope.count {
        span(out, "nvs-count", &format!("x{count}"));
    }
    out.push_str("</div>");
}

/// One `<li>` of a container: the key it is filed under, then the node itself.
///
/// The key is escaped like any other model text — a map's key is a value too,
/// and an object's property name reaches here with the `$` sigil the
/// declaration writes already on it.
fn write_entry(out: &mut String, key: &str, node: &Node) {
    out.push_str("<li>");
    span(out, "nvs-key", key);
    write_node(out, node);
    out.push_str("</li>");
}

/// Writes `node` as one element, and its children inside it.
fn write_node(out: &mut String, node: &Node) {
    match node {
        Node::Scalar(scalar) => write_scalar(out, scalar),
        Node::Sequence(items) => {
            open_container(
                out,
                "nvs-sequence",
                None,
                &format!("array({})", items.len()),
            );
            for (index, item) in items.iter().enumerate() {
                write_entry(out, &index.to_string(), item);
            }
            out.push_str("</ul></details>");
        }
        Node::Map(entries) => {
            open_container(out, "nvs-map", None, &format!("array({})", entries.len()));
            for (key, value) in entries {
                write_entry(out, &format!("{:?}", key.as_str()), value);
            }
            out.push_str("</ul></details>");
        }
        Node::Object {
            class,
            id,
            properties,
        } => {
            let identity = id.map_or_else(String::new, |id| format!("#{id}"));
            open_container(
                out,
                "nvs-object",
                *id,
                &format!("{class}{identity} ({})", properties.len()),
            );
            for (name, value) in properties {
                write_entry(out, &format!("${name}"), value);
            }
            out.push_str("</ul></details>");
        }
        Node::EnumCase { enum_name, case } => {
            span(out, "nvs-enum", &format!("{enum_name}::{case}"));
        }
        Node::Callable { parameters } => {
            let plural = if *parameters == 1 { "" } else { "s" };
            span(
                out,
                "nvs-callable",
                &format!("callable({parameters} parameter{plural})"),
            );
        }
        Node::Redacted => span(out, "nvs-redacted", "[redacted]"),
        Node::Elided(elision) => write_elision(out, elision),
        Node::Cycle { id } => {
            out.push_str(&format!(
                "<a class=\"nvs-cycle\" href=\"#{ANCHOR}{id}\">[cycle -&gt; #{id}]</a>"
            ));
        }
        Node::Span { file, line, label } => {
            span(
                out,
                "nvs-span",
                &format!("{file}:{line}: {}", label.as_str()),
            );
        }
        Node::Frame {
            depth,
            function,
            file,
            line,
        } => {
            out.push_str("<span class=\"nvs-frame\">");
            span(out, "nvs-frame-depth", &format!("#{depth}"));
            span(out, "nvs-frame-function", &format!("{function}()"));
            if let Some(file) = file {
                let at = line.map_or_else(|| file.clone(), |line| format!("{file}:{line}"));
                span(out, "nvs-frame-site", &at);
            }
            out.push_str("</span>");
        }
    }
}

/// Opens a container: its `<details>`, its `<summary>` and the `<ul>` its
/// entries go in. The caller closes all three once it has written them.
///
/// `anchor` is an object's identity, which becomes the `id` a
/// [`Node::Cycle`]'s link resolves against.
fn open_container(out: &mut String, class: &str, anchor: Option<usize>, summary: &str) {
    let id = anchor.map_or_else(String::new, |id| format!(" id=\"{ANCHOR}{id}\""));
    out.push_str(&format!("<details class=\"{class}\"{id} open><summary>"));
    out.push_str(&escape(summary));
    out.push_str("</summary><ul>");
}

/// One scalar as a span carrying both the Novis type it is tagged with and the
/// text [`crate::plain`] writes for it, so the two renderings name a value the
/// same way and only the markup around it differs.
fn write_scalar(out: &mut String, scalar: &Scalar) {
    match scalar {
        Scalar::Null => span(out, "nvs-null", "null"),
        Scalar::Bool(value) => span(out, "nvs-bool", &format!("bool({value})")),
        Scalar::Int(value) => span(out, "nvs-int", &format!("int({value})")),
        Scalar::Uint(value) => span(out, "nvs-uint", &format!("uint({value})")),
        Scalar::Float(value) => span(out, "nvs-float", &format!("float({value})")),
        Scalar::Decimal(value) => span(out, "nvs-decimal", &format!("decimal({value})")),
        Scalar::Str { text, bytes } => span(
            out,
            "nvs-string",
            &format!("string({bytes}) {:?}", text.as_str()),
        ),
        Scalar::Bytes(bytes) => span(
            out,
            "nvs-bytes",
            &format!("bytes({}) 0x{}", bytes.len(), crate::hex(bytes)),
        ),
    }
}

/// An [`Elision`] as the cut it is, naming the same amounts the other two
/// renderings name — the model decided what was cut and every rendering says
/// so.
fn write_elision(out: &mut String, elision: &Elision) {
    match elision {
        Elision::Depth => span(out, "nvs-elided", "[elided: below the depth cap]"),
        Elision::Entries { total, cut } => span(
            out,
            "nvs-elided",
            &format!("[elided: {cut} of {total} entries]"),
        ),
        Elision::Text { kept, cut } => span(
            out,
            "nvs-string nvs-elided",
            &format!(
                "string({}+) {:?} [elided: {cut} bytes]",
                kept.as_str().len(),
                kept.as_str()
            ),
        ),
    }
}

/// One `<span>` of `class` around `text`, escaped.
///
/// Every piece of model text the rendering writes goes through here, which is
/// what makes "escaped on the way in" a property of the rendering rather than
/// of each arm remembering to ask.
fn span(out: &mut String, class: &str, text: &str) {
    out.push_str(&format!("<span class=\"{class}\">"));
    out.push_str(&escape(text));
    out.push_str("</span>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Level, Rendered};

    /// The five characters, in one string rather than one assertion each, so a
    /// table that lost a row fails here rather than passing five lines that
    /// each ask about something else.
    #[test]
    fn every_delimiter_becomes_a_character_reference() {
        assert_eq!(escape("&<>\"'"), "&amp;&lt;&gt;&quot;&#39;");
    }

    /// The transformation, over the boundary each of the five characters sits
    /// on: a character reference is produced for every one of them and for
    /// nothing else in ASCII.
    ///
    /// Counted rather than read off five lines, so a table that escaped a
    /// sixth character — `/`, which several PHP escapers add — fails here.
    /// Asked through [`escape`] rather than of [`escaped`], so it is the
    /// transform callers reach that is being counted.
    #[test]
    fn exactly_five_ascii_characters_are_escaped() {
        let escaped_set: Vec<char> = (0u8..128)
            .map(char::from)
            .filter(|c| escape(&c.to_string()) != c.to_string())
            .collect();
        assert_eq!(escaped_set, vec!['"', '&', '\'', '<', '>']);
    }

    /// The borrow is the point of the signature, so it is asserted rather than
    /// left to the type: a copy taken on every ordinary write would still pass
    /// every other test in this file.
    #[test]
    fn ordinary_text_is_answered_as_the_borrow_it_arrived_as() {
        assert!(matches!(escape("an ordinary paragraph"), Cow::Borrowed(_)));
        assert!(matches!(escape("a & b"), Cow::Owned(_)));
    }

    /// The bidi half, which is the reason this is not five `replace` calls: an
    /// override with no terminator is written out rather than escaped, because
    /// what it does is reorder everything after it and a character reference
    /// would carry the payload through.
    #[test]
    fn an_unterminated_bidi_control_is_replaced_and_not_referenced() {
        assert_eq!(escape("a\u{202E}b"), "a\u{FFFD}b");
    }

    /// Escaping is not idempotent, and that is the property the carrier exists
    /// to protect: text that already reads as a reference is escaped again,
    /// because the author's data said `&amp;`.
    #[test]
    fn a_reference_in_the_input_is_escaped_again() {
        assert_eq!(escape("&amp;"), "&amp;amp;");
    }

    /// A container is a `<details>` a reader can close, its entries are keyed,
    /// and a scalar names the Novis type the model tagged it with — the
    /// collapsible and typed halves of `rule:errors/renderings`' row.
    #[test]
    fn a_container_is_collapsible_and_its_scalars_are_typed() {
        let node = Node::Map(vec![(
            Rendered::new("a"),
            Node::Sequence(vec![Node::Scalar(Scalar::Int(1))]),
        )]);
        assert_eq!(
            render_nodes(&[node]),
            "<div class=\"nvs-nodes\">\
             <details class=\"nvs-map\" open><summary>array(1)</summary><ul>\
             <li><span class=\"nvs-key\">&quot;a&quot;</span>\
             <details class=\"nvs-sequence\" open><summary>array(1)</summary><ul>\
             <li><span class=\"nvs-key\">0</span><span class=\"nvs-int\">int(1)</span></li>\
             </ul></details></li></ul></details></div>"
        );
    }

    /// The class-aware half: an object names its class and anchors its
    /// identity, and a repeat links to that anchor rather than saying
    /// `*RECURSION*` — which is what `rule:errors/record-transformations`
    /// keeps a cycle an identity for.
    #[test]
    fn an_object_anchors_its_identity_and_a_cycle_links_to_it() {
        let markup = render_nodes(&[
            Node::Object {
                class: "Point".to_owned(),
                id: Some(1),
                properties: vec![("x".to_owned(), Node::Scalar(Scalar::Int(1)))],
            },
            Node::Cycle { id: 1 },
        ]);
        assert!(
            markup.contains(
                "<details class=\"nvs-object\" id=\"nvs-id-1\" open><summary>Point#1 (1)</summary>"
            ),
            "{markup}"
        );
        assert!(
            markup.contains("<span class=\"nvs-key\">$x</span>"),
            "{markup}"
        );
        assert!(
            markup.contains("<a class=\"nvs-cycle\" href=\"#nvs-id-1\">[cycle -&gt; #1]</a>"),
            "{markup}"
        );
    }

    /// Every piece of model text goes out through [`escape`], so a value that
    /// reads as markup reaches the page as text — the rendering owns no second
    /// answer about what a `<` becomes.
    #[test]
    fn model_text_is_escaped_on_the_way_into_the_markup() {
        let markup = render_nodes(&[Node::Object {
            class: "A<b>".to_owned(),
            id: None,
            properties: vec![(
                "x".to_owned(),
                Node::Scalar(Scalar::Str {
                    text: Rendered::new("<script>alert('x')</script>"),
                    bytes: 27,
                }),
            )],
        }]);
        assert!(!markup.contains("<script>"), "{markup}");
        assert!(
            markup.contains("&lt;script&gt;alert(&#39;x&#39;)"),
            "{markup}"
        );
        assert!(markup.contains("A&lt;b&gt;"), "{markup}");
    }

    /// An envelope's present fields are its header and its absent ones are
    /// omitted, which is the model's rule rather than this rendering's.
    #[test]
    fn an_absent_envelope_field_is_omitted() {
        let mut record = Record::at(Level::Warn);
        assert_eq!(
            render(&record),
            "<div class=\"nvs-record nvs-level-warn\">\
             <div class=\"nvs-envelope\"><span class=\"nvs-level\">warn</span></div></div>"
        );
        record.envelope.message = Some(Rendered::new("bound publicly"));
        record.envelope.count = Some(37);
        let markup = render(&record);
        assert!(
            markup.contains("<span class=\"nvs-message\">bound publicly</span>"),
            "{markup}"
        );
        assert!(
            markup.contains("<span class=\"nvs-count\">x37</span>"),
            "{markup}"
        );
    }

    /// Nothing here decides what to truncate: an [`Elision`] arrived as a node
    /// and all three renderings name the same cut of the same value, which is
    /// the property `rule:errors/record-transformations` puts elision in the
    /// model for.
    #[test]
    fn the_html_rendering_elides_where_plain_and_json_elide() {
        let mut record = Record::at(Level::Debug);
        record.nodes.push(Node::Sequence(vec![
            Node::Scalar(Scalar::Int(1)),
            Node::Elided(Elision::Entries {
                total: 140,
                cut: 40,
            }),
        ]));
        record.nodes.push(Node::Elided(Elision::Text {
            kept: Rendered::new("ab"),
            cut: 30,
        }));
        record.nodes.push(Node::Elided(Elision::Depth));

        let html = render(&record);
        let plain = crate::plain::render(&record);
        let json = crate::json::render(&record);

        assert!(html.contains("[elided: 40 of 140 entries]"), "{html}");
        assert!(plain.contains("[elided: 40 of 140 entries]"), "{plain}");
        assert!(
            json.contains(r#"{"$elided":{"cut":"entries","total":140,"entries":40}}"#),
            "{json}"
        );

        assert!(
            html.contains("string(2+) &quot;ab&quot; [elided: 30 bytes]"),
            "{html}"
        );
        assert!(
            plain.contains("string(2+) \"ab\" [elided: 30 bytes]"),
            "{plain}"
        );
        assert!(
            json.contains(r#"{"$elided":{"cut":"text","kept":"ab","bytes":30}}"#),
            "{json}"
        );

        assert!(html.contains("[elided: below the depth cap]"), "{html}");
        assert!(plain.contains("[elided: below the depth cap]"), "{plain}");
        assert!(json.contains(r#"{"$elided":{"cut":"depth"}}"#), "{json}");

        // The one entry the model kept is in all three, so what agrees is the
        // cut rather than the whole container having been dropped.
        assert!(html.contains("int(1)") && plain.contains("int(1)") && json.contains('1'));
    }

    /// Redaction is the record's, applied by the producer before any rendering
    /// sees the value: the node that arrives is [`Node::Redacted`], so each
    /// rendering writes its placeholder and none of them has the secret to
    /// leak.
    #[test]
    fn a_secret_renders_as_the_placeholder_in_all_three_renderings() {
        let mut record = Record::at(Level::Error);
        record
            .envelope
            .fields
            .push(("token".to_owned(), Node::Redacted));
        record.nodes.push(Node::Object {
            class: "Session".to_owned(),
            id: Some(1),
            properties: vec![
                ("token".to_owned(), Node::Redacted),
                (
                    "user".to_owned(),
                    Node::Scalar(Scalar::Str {
                        text: Rendered::new("ann"),
                        bytes: 3,
                    }),
                ),
            ],
        });

        let html = render(&record);
        let plain = crate::plain::render(&record);
        let json = crate::json::render(&record);

        assert_eq!(
            html.matches("<span class=\"nvs-redacted\">[redacted]</span>")
                .count(),
            2,
            "{html}"
        );
        assert_eq!(plain.matches("[redacted]").count(), 2, "{plain}");
        assert_eq!(json.matches(r#"{"$redacted":true}"#).count(), 2, "{json}");

        // The value beside it is rendered, so the placeholder is standing for
        // what was declared `secret` rather than for the whole object.
        assert!(html.contains("ann") && plain.contains("ann") && json.contains("ann"));
    }

    /// A frame is its own kind, and this rendering spells it as the parts the
    /// model carries rather than as the one line the plaintext rendering
    /// greps — `rule:errors/record-producers` is why it is not an object.
    #[test]
    fn a_frame_renders_as_its_three_parts() {
        let markup = render_nodes(&[Node::Frame {
            depth: 0,
            function: Rendered::new("Main::main"),
            file: Some("app.nvs".to_owned()),
            line: Some(12),
        }]);
        assert!(
            markup.contains(
                "<span class=\"nvs-frame\">\
                 <span class=\"nvs-frame-depth\">#0</span>\
                 <span class=\"nvs-frame-function\">Main::main()</span>\
                 <span class=\"nvs-frame-site\">app.nvs:12</span></span>"
            ),
            "{markup}"
        );
    }
}
