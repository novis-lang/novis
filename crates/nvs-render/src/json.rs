//! `rule:errors/renderings`'s JSON rendering — what a log target emits under `[log] format = "json"`.
//!
//! # Why it is here rather than in `nvs-runtime`
//!
//! `rule:errors/log-write`'s claim is
//! that ordinary application code and the engine floor write the **same record
//! through the same native helper**, so a log pipeline never has to reconcile
//! two shapes depending on which tier produced a line. [`line()`] is that
//! helper's rendering half. It sits beside [`crate::plain::render`] because
//! `rule:errors/diagnostic-record` puts the model *and every rendering* in one crate that
//! both the runtime and the compiler front end depend on — the floor reaching
//! it is exactly the edge § 1 sanctions and the crate docs' § *Where this sits*
//! prices. The alternative, a JSON writer in `nvs-runtime` for the floor and
//! this one for everything else, is two writers that agree today, which is the
//! failure `rule:errors/diagnostic-record` exists to prevent.
//!
//! # The envelope's keys
//!
//! `rule:errors/log-write`'s list, in reading order: `ts`, `level`, `msg`, `request_id`,
//! `trace_id`, `span_id`, `source`, `count`, `fields`, then the record's own
//! `nodes`. Only `level` is unconditional; every other key is **omitted rather
//! than written empty**, which is [`Envelope`]'s own rule and § 6's for
//! `trace_id`/`span_id` in particular.
//!
//! `count` is the one key § 6 does not list, because it belongs to
//! `rule:http-server/the-floor-cannot-fill-the-disk`
//! instead; it sits last of the envelope's own keys and ahead of
//! the producer's `fields` for the reason [`Envelope::count`] gives — it is the
//! sink talking about the record, not the call site talking about the failure.
//!
//! **`msg` is § 6's `message`, spelled short.** That is what
//! `examples/logging.nvs` and `Core\Log::write`'s conformance cases pin, while
//! § 6's prose writes the long form; the two are one word apart in one of the
//! two places and nothing else depends on which is chosen.
//!
//! # What a node is spelled as
//!
//! **A node is spelled as the closest JSON value, not as a tagged pair** —
//! `{"user":7}`, never `{"user":{"int":7}}`. JSON already carries the
//! distinction the plaintext rendering spends `int(42)` on, and a log field is
//! read by a pipeline that expects the field to *be* its value.
//!
//! **A node kind JSON has no value for becomes a one-key object whose key
//! begins with `$`**: `{"$bytes":"74657874"}`, `{"$decimal":"1.50"}`,
//! `{"$float":"NaN"}`, `{"$enum":"Level::Info"}`, `{"$callable":{"parameters":2}}`,
//! `{"$redacted":true}`, `{"$elided":…}`, `{"$cycle":3}`, `{"$span":…}`, and an
//! instance as `{"$class":…,"$id":…,"$properties":{…}}`. The consequences
//! worth having in hand:
//!
//! * **`bytes` is lowercase hex**, not base64 — it costs this crate no
//!   dependency it would not otherwise carry, and a short value stays
//!   decodable by eye. A `bytes` field is *rendered* rather than refused,
//!   because § 1's model carries the scalar and a floor that throws while
//!   reporting has nothing left to report with.
//! * **`decimal` is a string**, not a bare JSON number: the exactness is the
//!   whole reason that type exists, and a consumer parsing the number into a
//!   `double` would lose it silently.
//! * The one ambiguity left is a [`Node::Map`] whose single key is itself
//!   spelled `$bytes` (or another of the tags). It is accepted rather than
//!   closed, because closing it means tagging every node and paying for it on
//!   every ordinary field.
//!
//! **A [`Node::Frame`] is the one kind JSON has no value for that is spelled
//! without a tag**: `{"function":…,"file":…,"line":…}`, because
//! `rule:errors/record-producers` fixes the trace's wire shape as an array of
//! those objects and a pipeline that alerts on `nodes[0].function` should not
//! have to reach through a `$`-key to find it. A frame is never mistaken for a
//! `Map`, since the producer of one is the only writer of the other keys beside
//! it.

use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq, Serializer};

use crate::{Elision, Envelope, Node, Record, Rendered, Scalar, Source};

/// One record as a JSON Lines line — the terminating newline included, because
/// a JSON Lines record without it is not one.
///
/// Infallible on purpose: this is what `rule:errors/log-write`'s tier-4 floor calls with its one shot, so a record always renders to a
/// line rather than to a `Result` the floor has nowhere to send.
#[must_use]
pub fn line(record: &Record) -> String {
    let mut line = render(record);
    line.push('\n');
    line
}

/// One record as a JSON object, with no terminator — the same rendering
/// [`line()`] writes, for a caller composing it into something larger.
#[must_use]
pub fn render(record: &Record) -> String {
    serde_json::to_string(&AsRecord(record)).unwrap_or_else(|_| {
        // Unreachable by construction: every map key below is a `&str`, every
        // float that reaches `serialize_f64` is finite, and none of these
        // `Serialize` impls returns an error of its own, which covers every
        // way `serde_json` fails. The fallback is a *record* rather
        // than a panic or an empty line for the reason this function is
        // infallible at all: the floor has one shot.
        r#"{"level":"critical","msg":"a record could not be rendered"}"#.to_owned()
    })
}

/// A whole [`Record`]: its envelope's keys, then its own nodes.
struct AsRecord<'a>(&'a Record);

impl Serialize for AsRecord<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let Envelope {
            ts,
            level,
            message,
            request_id,
            trace_id,
            span_id,
            source,
            count,
            fields,
        } = &self.0.envelope;
        let mut map = ser.serialize_map(None)?;
        if let Some(ts) = ts {
            map.serialize_entry("ts", ts)?;
        }
        map.serialize_entry("level", level.name())?;
        if let Some(message) = message {
            map.serialize_entry("msg", message.as_str())?;
        }
        if let Some(request_id) = request_id {
            map.serialize_entry("request_id", request_id)?;
        }
        if let Some(trace_id) = trace_id {
            map.serialize_entry("trace_id", trace_id)?;
        }
        if let Some(span_id) = span_id {
            map.serialize_entry("span_id", span_id)?;
        }
        if let Some(source) = source {
            map.serialize_entry("source", &AsSource(source))?;
        }
        if let Some(count) = count {
            map.serialize_entry("count", count)?;
        }
        if !fields.is_empty() {
            map.serialize_entry("fields", &AsFields(fields))?;
        }
        if !self.0.nodes.is_empty() {
            map.serialize_entry("nodes", &AsNodes(&self.0.nodes))?;
        }
        map.end()
    }
}

/// `rule:errors/diagnostic-record`'s `source` — file, line, and the enclosing member when there is
/// one.
struct AsSource<'a>(&'a Source);

impl Serialize for AsSource<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(None)?;
        map.serialize_entry("file", &self.0.file)?;
        map.serialize_entry("line", &self.0.line)?;
        if let Some(member) = &self.0.member {
            map.serialize_entry("member", member)?;
        }
        map.end()
    }
}

/// Named nodes under their own names — the envelope's `fields` and an
/// [`Node::Object`]'s properties, which are the same shape.
struct AsFields<'a>(&'a [(String, Node)]);

impl Serialize for AsFields<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(self.0.len()))?;
        for (name, node) in self.0 {
            map.serialize_entry(name, &AsNode(node))?;
        }
        map.end()
    }
}

/// A [`Node::Map`]'s entries, whose keys are [`Rendered`] rather than plain
/// text because a key is a value too and § 5 substitutes it like one.
struct AsEntries<'a>(&'a [(Rendered, Node)]);

impl Serialize for AsEntries<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(self.0.len()))?;
        for (key, node) in self.0 {
            map.serialize_entry(key.as_str(), &AsNode(node))?;
        }
        map.end()
    }
}

/// A run of nodes as a JSON array.
struct AsNodes<'a>(&'a [Node]);

impl Serialize for AsNodes<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut seq = ser.serialize_seq(Some(self.0.len()))?;
        for node in self.0 {
            seq.serialize_element(&AsNode(node))?;
        }
        seq.end()
    }
}

/// One node, under the module doc's rules: the closest JSON value, or a
/// one-key `$`-tagged object where JSON has no value for the kind.
struct AsNode<'a>(&'a Node);

impl Serialize for AsNode<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Node::Scalar(scalar) => AsScalar(scalar).serialize(ser),
            Node::Sequence(items) => AsNodes(items).serialize(ser),
            Node::Map(entries) => AsEntries(entries).serialize(ser),
            Node::Object {
                class,
                id,
                properties,
            } => {
                let mut map = ser.serialize_map(None)?;
                map.serialize_entry("$class", class)?;
                if let Some(id) = id {
                    map.serialize_entry("$id", id)?;
                }
                map.serialize_entry("$properties", &AsFields(properties))?;
                map.end()
            }
            Node::EnumCase { enum_name, case } => {
                tagged(ser, "$enum", &format!("{enum_name}::{case}"))
            }
            Node::Callable { parameters } => {
                let mut map = ser.serialize_map(Some(1))?;
                map.serialize_entry("$callable", &Parameters(*parameters))?;
                map.end()
            }
            Node::Redacted => tagged(ser, "$redacted", &true),
            Node::Elided(elision) => tagged(ser, "$elided", &AsElision(elision)),
            Node::Cycle { id } => tagged(ser, "$cycle", id),
            Node::Span { file, line, label } => {
                let mut map = ser.serialize_map(Some(1))?;
                map.serialize_entry("$span", &AsSpan(file, *line, label))?;
                map.end()
            }
            Node::Frame {
                function,
                file,
                line,
                ..
            } => {
                let mut map = ser.serialize_map(None)?;
                map.serialize_entry("function", function.as_str())?;
                if let Some(file) = file {
                    map.serialize_entry("file", file)?;
                }
                if let Some(line) = line {
                    map.serialize_entry("line", line)?;
                }
                map.end()
            }
        }
    }
}

/// A callable's declared arity, which is all a node carries of one.
struct Parameters(usize);

impl Serialize for Parameters {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(1))?;
        map.serialize_entry("parameters", &self.0)?;
        map.end()
    }
}

/// A [`Node::Span`]'s three parts.
struct AsSpan<'a>(&'a str, u32, &'a Rendered);

impl Serialize for AsSpan<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(3))?;
        map.serialize_entry("file", self.0)?;
        map.serialize_entry("line", &self.1)?;
        map.serialize_entry("label", self.2.as_str())?;
        map.end()
    }
}

/// What was cut and how much of it — `rule:errors/record-transformations`'s shapes, each naming
/// its own kind so a reader never has to infer it from which keys are present.
struct AsElision<'a>(&'a Elision);

impl Serialize for AsElision<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(None)?;
        match self.0 {
            Elision::Depth => map.serialize_entry("cut", "depth")?,
            Elision::Entries { total, cut } => {
                map.serialize_entry("cut", "entries")?;
                map.serialize_entry("total", total)?;
                map.serialize_entry("entries", cut)?;
            }
            Elision::Text { kept, cut } => {
                map.serialize_entry("cut", "text")?;
                map.serialize_entry("kept", kept.as_str())?;
                map.serialize_entry("bytes", cut)?;
            }
        }
        map.end()
    }
}

/// One scalar as the JSON value closest to the Novis type it is tagged with.
struct AsScalar<'a>(&'a Scalar);

impl Serialize for AsScalar<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Scalar::Null => ser.serialize_unit(),
            Scalar::Bool(value) => ser.serialize_bool(*value),
            Scalar::Int(value) => ser.serialize_i64(*value),
            Scalar::Uint(value) => ser.serialize_u64(*value),
            // JSON has no spelling for a non-finite float and `serde_json`
            // refuses one rather than inventing it, so this is the one place a
            // scalar's own value decides its shape.
            Scalar::Float(value) if value.is_finite() => ser.serialize_f64(*value),
            Scalar::Float(value) => tagged(ser, "$float", non_finite(*value)),
            Scalar::Decimal(text) => tagged(ser, "$decimal", text),
            Scalar::Str { text, .. } => ser.serialize_str(text.as_str()),
            Scalar::Bytes(bytes) => tagged(ser, "$bytes", &crate::hex(bytes)),
        }
    }
}

/// A one-key object under a `$`-prefixed tag — the module doc's tagging rule,
/// in one place so every kind that needs it is spelled the same way.
fn tagged<S: Serializer, T: Serialize + ?Sized>(
    ser: S,
    tag: &'static str,
    value: &T,
) -> Result<S::Ok, S::Error> {
    let mut map = ser.serialize_map(Some(1))?;
    map.serialize_entry(tag, value)?;
    map.end()
}

/// How a float JSON cannot carry is named. `inf` and `NaN` are the spellings
/// every JSON-adjacent format that admits them at all already uses.
fn non_finite(value: f64) -> &'static str {
    if value.is_nan() {
        "NaN"
    } else if value.is_sign_positive() {
        "inf"
    } else {
        "-inf"
    }
}

#[cfg(test)]
mod tests {
    use super::{line, render};
    use crate::{Elision, Level, Node, Record, Rendered, Scalar};

    /// The envelope's unconditional key is `level` and nothing else, and the
    /// line ends where JSON Lines needs it to.
    #[test]
    fn an_empty_record_is_its_level_and_a_newline() {
        let record = Record::at(Level::Critical);
        assert_eq!(line(&record), "{\"level\":\"critical\"}\n");
    }

    /// Each kind JSON has no value for takes its own `$` tag, and a scalar it
    /// does have one for takes none.
    #[test]
    fn a_kind_json_cannot_carry_is_tagged_and_one_it_can_is_not() {
        for (node, want) in [
            (Node::Scalar(Scalar::Int(7)), "7"),
            (Node::Scalar(Scalar::Float(1.5)), "1.5"),
            (Node::Scalar(Scalar::Float(f64::NAN)), r#"{"$float":"NaN"}"#),
            (
                Node::Scalar(Scalar::Decimal("1.50".to_owned())),
                r#"{"$decimal":"1.50"}"#,
            ),
            (
                Node::Scalar(Scalar::Bytes(b"text".to_vec())),
                r#"{"$bytes":"74657874"}"#,
            ),
            (Node::Redacted, r#"{"$redacted":true}"#),
            (Node::Cycle { id: 3 }, r#"{"$cycle":3}"#),
            (
                Node::Elided(Elision::Depth),
                r#"{"$elided":{"cut":"depth"}}"#,
            ),
            (
                Node::EnumCase {
                    enum_name: "Level".to_owned(),
                    case: "Info".to_owned(),
                },
                r#"{"$enum":"Level::Info"}"#,
            ),
        ] {
            let mut record = Record::at(Level::Debug);
            record.envelope.fields.push(("f".to_owned(), node));
            assert_eq!(
                render(&record),
                format!("{{\"level\":\"debug\",\"fields\":{{\"f\":{want}}}}}")
            );
        }
    }

    /// § 5's substitution is in the model, so the rendering inherits it: a
    /// message's control bytes are already Control Pictures by the time this
    /// sees them, while `LF` — which `rule:tooling/terminal-output-is-a-sink` passes through — is JSON's
    /// own escape and cannot forge a second line.
    #[test]
    fn a_messages_control_bytes_arrive_substituted() {
        let mut record = Record::at(Level::Info);
        record.envelope.message = Some(Rendered::new("a\nb\u{1}c"));
        assert_eq!(
            render(&record),
            "{\"level\":\"info\",\"msg\":\"a\\nb\u{2401}c\"}"
        );
    }
}
