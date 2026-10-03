//! The walk that turns a runtime value into `rule:errors/diagnostic-record`'s
//! record nodes — one walk, reached by every producer that has a value to show.
//!
//! # Why it is here rather than in `Core\Debug`
//!
//! `rule:errors/record-producers` names five producers and none of them
//! implements a format, which only holds while they share the *walk* as well as
//! the renderings. Two of them live on opposite sides of a crate edge:
//! `Core\Debug::dump` is a `Core` member in `nvs-stdlib`, and
//! [`crate::floor::uncaught`] is the engine floor, which runs where there is no
//! script left and so may depend on no `Core` class at all. A walk in
//! `nvs-stdlib` therefore cannot be the floor's, and a second walk beside it is
//! exactly the failure `rule:errors/record-transformations` exists to prevent:
//! redaction, substitution, elision and cycle detection decided twice are
//! decided differently the first time one of them changes.
//!
//! So the walk sits in the crate both producers already depend on, beside the
//! [`crate::graph`] copy it is shaped like — one traversal, several callers,
//! and no `match` on [`Tag`] anywhere else.
//!
//! # What it decides, and what it does not
//!
//! Every one of `rule:errors/record-transformations`'s four transformations is
//! the *model*'s, not this module's: control bytes and bidi are
//! [`Rendered`]'s constructor, elision is an [`Elision`] node, and a cycle is a
//! [`Node::Cycle`]. What this walk decides is the two things only a runtime
//! value can answer — which node kind a tag denotes, and which property is
//! `secret` — and it applies the caps it was given rather than choosing them.
//!
//! It reads that node kind off a [`Tag`] and off nothing else, which fixes one
//! bound: **an enum case walks to its backing integer.** `rule:types/conversion`
//! gives an enum no tag of its own — a case *is* an `int` by the time it is a
//! [`Value`] — so one arriving through `mixed` is a `Scalar` int and shows the
//! number it is rather than the case it was written as. [`Node::EnumCase`] is
//! the model's kind for a producer that holds the static type and therefore
//! knows which enum a number came from; recovering it from a walk would want
//! the tag roster to tell an enum apart, which is the representation
//! `rule:types/conversion` declines to spend.
//!
//! A declared property is the only place this walk can *see* the qualifier:
//! [`ClassDesc::field_is_secret`] is a fact the class carries at run time, and
//! `rule:errors/record-transformations`'s redaction row turns it into a
//! [`Node::Redacted`] wherever the walk reaches it, a nested object's property
//! included. Everywhere else the bit is a **static** one, erased before a
//! [`Value`] exists — an `array<secret string>`'s element arrives here as a
//! plain string — so confidentiality outside a property is answered in front
//! of this walk and not inside it. Two rules do that, and the second is what
//! keeps the first honest: `rule:security/secret-sinks-refuse` refuses an
//! argument carrying a `secret` *anywhere* in it at every producer that takes
//! a value, and `rule:security/secret-qualifier`'s container axis refuses the
//! write that would put one into an array element or a shape field whose own
//! type does not carry the qualifier
//! (`nvs_types::expr::quals::reject_secret_into_container`), so a credential
//! cannot be laundered into a container one statement before the call.
//!
//! # What it spends
//!
//! One node per value reached, bounded by [`Caps`], plus one recursion frame
//! and one [`Seen`] entry per level of nesting. All of it is released with the
//! record it built, so nothing here grows with records written.

use nvs_render::{Caps, Elision, Node, Rendered, Scalar};

use crate::array::NvsArray;
use crate::object::{ClassDesc, NvsObj, ObjHeader};
use crate::value::{Tag, Value};

/// One value as a node, under the default [`Caps`].
///
/// The entry point every producer uses: `Core\Debug::dump`'s one node per
/// argument, `Core\Log::write`'s `fields` bag, and the throw's own declared
/// properties in [`crate::floor::uncaught`] — so a value written as a log field
/// and the same value dumped are the same node, with the record's
/// transformations applied once and in one place.
#[must_use]
pub fn node(value: Value) -> Node {
    node_of(value, &Caps::default(), 0, &mut Seen::default())
}

/// The objects this walk has already entered, so a graph that points back at
/// one becomes a [`Node::Cycle`] rather than an infinite traversal.
///
/// Keyed by the allocation's address, which is [`crate::identity`]'s own answer
/// for an object (`rule:expressions/equality-semantics` lowers object equality
/// to it) — so the id a `Cycle` names is an identity rather than a position,
/// which is what lets the HTML rendering link the repeat.
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
        Some(Tag::Int | Tag::EnumInt) => Node::Scalar(Scalar::Int(value.as_int().unwrap_or(0))),
        Some(Tag::Uint | Tag::EnumUint) => Node::Scalar(Scalar::Uint(value.as_uint().unwrap_or(0))),
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
    }
}

/// A `string`, cut at [`Caps::text`].
///
/// The bytes are decoded lossily rather than refused: a `string` is UTF-8 by
/// `rule:types/bytes`'s promise, so a run that is not is a value that came from
/// outside the type system, and a dump is exactly the tool a developer reaches
/// for to see one.
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

/// A `bytes` value, cut at [`Caps::text`]. The record's substitution does not
/// apply: it is a transformation of *text*, and these are not.
fn bytes_node(bytes: &[u8], caps: &Caps) -> Node {
    if bytes.len() <= caps.text {
        return Node::Scalar(Scalar::Bytes(bytes.to_vec()));
    }
    Node::Elided(Elision::Entries {
        total: bytes.len(),
        cut: bytes.len() - caps.text,
    })
}

/// An `array`, as `rule:errors/diagnostic-record`'s Sequence or Map.
///
/// The two shapes are one runtime type, so which one this is is decided from
/// the keys: an array whose keys are `"0"`, `"1"`, … in order is the list shape
/// and renders by position, and anything else renders by key. That is the same
/// reading `Core\Json::encode` already makes of the same value, so a dump and
/// an encode do not disagree about what an array *is*.
fn array_node(value: Value, caps: &Caps, depth: usize, seen: &mut Seen) -> Node {
    let Some(ptr) = value.array_ptr() else {
        return Node::Scalar(Scalar::Null);
    };
    if depth >= caps.depth {
        return Node::Elided(Elision::Depth);
    }
    let array = borrowed(ptr);
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

/// A borrowed handle on a live array: the value owns the reference, and
/// `ManuallyDrop` keeps this read free of refcount traffic.
fn borrowed(array: *mut crate::array::ArrayHeader) -> std::mem::ManuallyDrop<NvsArray> {
    #[expect(
        unsafe_code,
        reason = "a Tag::Array value owns a reference to a live allocation, so \
                  it is live for the length of this call, and the handle is \
                  never dropped"
    )]
    std::mem::ManuallyDrop::new(unsafe { NvsArray::from_raw(array) })
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

/// A class instance, as `rule:errors/diagnostic-record`'s Object node — the
/// class name and its **declared** properties, per `rule:classes/no-debug-hook`.
///
/// Never a `toString` result and never a customization hook: a dump shows a
/// class's real declared properties and their real current values, with the
/// record's transformations and nothing else.
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
    ptr: *mut ObjHeader,
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

    // `rule:types/implicit-capture` gives a closure no user-visible state at
    // all — the fields are its captures, and a dump that showed them would be
    // showing an implementation. A synthesized closure class is named
    // `{owner}$fn{n}` by `nvs_types::expr::calls`, and `$` cannot appear in a
    // declared name (`rule:core-api/identifier-casing`), so the marker is
    // unambiguous.
    if class.contains("$fn") {
        let parameters = crate::closure_arity(value).unwrap_or(0);
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
    let desc: &ClassDesc = unsafe { &*object.class() };
    Node::Object {
        class,
        id,
        properties: properties_of(&object, desc, caps, depth, seen, 0),
    }
}

/// Every declared property from slot `first` on, as a named node — the shape an
/// [`Node::Object`] carries, and the one [`crate::floor::uncaught`] reads a
/// throw's own properties into.
///
/// `rule:errors/record-transformations`'s redaction row: the *declared* type
/// decides, so a `secret` value is never walked at all rather than walked and
/// then discarded — a `secret` object's own properties are not read, and a
/// `secret` string contributes no elision node saying how long it was.
fn properties_of(
    object: &NvsObj,
    desc: &ClassDesc,
    caps: &Caps,
    depth: usize,
    seen: &mut Seen,
    first: usize,
) -> Vec<(String, Node)> {
    let mut properties = Vec::new();
    for slot in first..object.field_count() {
        let name = desc
            .field_name(slot)
            .map_or_else(|| slot.to_string(), ToOwned::to_owned);
        let node = if desc.field_is_secret(slot) {
            Node::Redacted
        } else {
            node_of(object.field(slot), caps, depth + 1, seen)
        };
        properties.push((name, node));
    }
    properties
}

/// The properties an object declares beyond its first `first` slots, walked
/// under the default caps — [`crate::floor::uncaught`]'s reader, for which the
/// first four slots are `Throwable`'s own and are the record's other parts.
#[must_use]
pub fn properties_past(value: Value, first: usize) -> Vec<(String, Node)> {
    let Some(ptr) = value.obj_ptr() else {
        return Vec::new();
    };
    #[expect(
        unsafe_code,
        reason = "the caller holds a live reference to the object for the length \
                  of this call, and the handle is never dropped"
    )]
    let object = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by the unit's class table, which \
                  outlives every instance of the class it describes"
    )]
    let desc: &ClassDesc = unsafe { &*object.class() };
    properties_of(
        &object,
        desc,
        &Caps::default(),
        0,
        &mut Seen::default(),
        first,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every scalar tag reaches the node kind that names its own Novis type,
    /// which is the whole reason the model tags them.
    #[test]
    fn a_scalar_becomes_its_own_node() {
        assert_eq!(node(Value::null()), Node::Scalar(Scalar::Null));
        assert_eq!(node(Value::bool(true)), Node::Scalar(Scalar::Bool(true)));
        assert_eq!(node(Value::int(-3)), Node::Scalar(Scalar::Int(-3)));
        assert_eq!(node(Value::uint(3)), Node::Scalar(Scalar::Uint(3)));
        assert_eq!(node(Value::float(0.5)), Node::Scalar(Scalar::Float(0.5)));
    }

    /// The record's substitution is the *model*'s, so a control byte in a
    /// walked string is already neutralized by the time any rendering sees it —
    /// the CWE-117 property, asserted at the walk.
    #[test]
    fn a_control_byte_is_substituted_on_the_way_into_the_record() {
        let node = text_node(b"a\rb", &Caps::default());
        let Node::Scalar(Scalar::Str { text, bytes }) = node else {
            panic!("a short string is a scalar node");
        };
        assert_eq!(text.as_str(), "a\u{240D}b");
        assert_eq!(bytes, 3);
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
