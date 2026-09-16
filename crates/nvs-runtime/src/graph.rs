//! `rule:classes/graph-copy`'s graph copy: one walk, reached by two carriers.
//!
//! A recursive, cycle-safe traversal of a value's reachable structure that
//! produces a result sharing no mutable heap state with its source. § 2 names
//! two carriers for it — live, arena-to-arena at the `spawn` boundary, and
//! externalized to bytes as `Core\Serialize::encode`/`decode` — and the whole
//! point of the item is that they are **one** operation. Two implementations
//! that agree today is the failure this module exists to prevent.
//!
//! # Decision: the carrier is a trait over the walk, not a second walk
//!
//! [`walk`] is the only traversal. It decides what is shared, what is refused
//! and what recurses; a [`Carrier`] decides only what a node *becomes*.
//! [`Live`] builds values into the destination and [`Encode`] appends bytes, and
//! neither of them contains a `match` on [`Tag`] — so a rule added to the walk
//! (a new refusal, a new shared shape) reaches both carriers or neither, and
//! there is no third place to forget.
//!
//! # Decision: identity is *object* identity, and nothing else
//!
//! § 2 asks for shared substructure to stay shared and for a cycle to
//! terminate, and it states both in terms of objects — "two properties pointing
//! at the same nested object". That is not an omission:
//!
//! * A **string** is immutable, so two references to one and two equal copies
//!   are indistinguishable to every program.
//! * An **array** is a value with copy-on-write storage
//!   ([`crate::array`]), so sharing is unobservable there too, and a cycle
//!   cannot be built at all: `$a[] = $a` appends a copy.
//! * An **object** is the one heap shape with reference semantics, so it is the
//!   one shape whose sharing a program can see and the one that can close a
//!   cycle.
//!
//! So [`walk`] keeps an identity map of objects only, and the byte format has
//! back-references for objects only. Giving strings and arrays identity too
//! would cost a hash lookup per node on the request path to preserve a
//! distinction nothing can observe.
//!
//! # Decision: a move at refcount 1 is the semantics, not an optimisation
//!
//! The walk *consumes* one reference to every value it is handed, and asks the
//! carrier whether it may [adopt](Carrier::adopt) the allocation instead of
//! building a new one. [`Live`] adopts exactly when that reference is the only
//! one, which is what makes the reuse sound rather than lucky: a node nothing
//! else reaches cannot be observed to be shared after the crossing. It is
//! decided **per node** rather than at the root, because a uniquely-owned root
//! can hold a child something else still holds — copying the root and adopting
//! that child is the bug this rule's shape avoids.
//!
//! [`Encode`] never adopts: bytes are always built.
//!
//! # Decision: a refusal is a bit on the class, never a shape it happens to have
//!
//! § 2's refusals are about what a value *is*, and the two an object can still
//! carry into the walk are both marks on its [`ClassDesc`]:
//! [`ClassDesc::is_closure()`] and [`ClassDesc::holds_host_handle()`]. Neither
//! is inferred from the instance in front of [`refusable`] — a declared
//! `invoke` is a method name a program may use, and the slot holding a host
//! handle is a `uint` like every other — so the crate that knows is the one
//! that says so: `nvs_stdlib::instance` marks every `Core` class whose slot
//! carries a key into a request's own table, and a class a program declares
//! carries neither mark. The third refusal, an `inout` binding, reaches no
//! value here at all: it is a frame alias the checker refuses at the copy site.
//!
//! # What it spends
//!
//! One [`HashMap`] entry per distinct object reached, for the length of one
//! copy, plus one recursion frame per level of nesting. Both are O(the graph
//! being copied) and are released when it ends, so nothing here grows with
//! requests served ([AGENTS.md](/AGENTS.md)'s priority 5).
//!
//! # Known gaps
//!
//! 1. **A decoded `Core` instance is a `mixed` a program cannot narrow.** The
//!    value itself is rebuilt under its own descriptor — a `Core\Time\Date`
//!    arrives back as one, slots intact, because [`crate::Ctx::class_desc`]
//!    asks the `Core` resolver after the program's table — but the checker
//!    refuses both ways of binding it to that class: `E0496`, since
//!    `instanceof` finds no descriptor to walk, and `E0711`, since
//!    `rule:types/conversion` tabulates no conversion into one. So the round
//!    trip is reachable through `Core\Debug` and through anything taking a
//!    `mixed`, and not yet by naming the class. Both refusals are
//!    `nvs-types`', and the address `instanceof` would test against is the one
//!    `nvs_stdlib::class_descriptors` already hands the backend for a folded
//!    `` html`…` `` constant.
//!    — owner: unowned

use std::collections::HashMap;

use crate::array::{NvsArray, SlotKey};
use crate::decimal::Decimal;
use crate::object::{ClassDesc, NvsObj, ObjHeader};
use crate::string::NvsStr;
use crate::value::{Tag, Value};

/// Novis's own format marker — `rule:classes/serialize-is-a-closed-format`'s "does not carry Novis's format
/// marker and version is refused outright".
const MAGIC: &[u8; 4] = b"NVS\x1b";

/// The format version. Bumped whenever the node grammar below changes; a
/// payload carrying any other value is refused whole.
const VERSION: u8 = 1;

/// How deep a graph may nest before it is refused.
///
/// [`walk`] and [`Reader::node`] both recurse, so this is what keeps a hostile
/// payload off the engine's stack — `rule:http-server/no-path-reaches-abort`
/// 's "every depth a request drives is bounded".
const MAX_DEPTH: u32 = 256;

/// The node tags of the byte carrier's grammar.
mod node {
    pub(super) const NULL: u8 = 0;
    pub(super) const BOOL: u8 = 1;
    pub(super) const INT: u8 = 2;
    pub(super) const UINT: u8 = 3;
    pub(super) const FLOAT: u8 = 4;
    pub(super) const DECIMAL: u8 = 5;
    pub(super) const STR: u8 = 6;
    pub(super) const BYTES: u8 = 7;
    pub(super) const ARRAY: u8 = 8;
    pub(super) const OBJECT: u8 = 9;
    pub(super) const BACKREF: u8 = 10;
}

/// Why a graph did not cross.
///
/// One shape rather than a refused/malformed pair: every caller classifies it
/// into a `Throwable` of its own choosing — `Core\Serialize::decode` answers
/// spec § 10's `ParseError` for all of them, and the live boundary answers a
/// diagnostic — so a second axis here would be one nothing reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphError(String);

impl GraphError {
    /// The refusal's message, which names the offending value or class.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The receiving side of a copy, as the one question the walk asks of it: does
/// this program declare the class this name spells, and at what address?
///
/// The same shape [`decode`] takes for the same question, named here because a
/// carrier holds one and an `Option` of it is past what a signature should
/// spell out twice.
pub type Receiving<'a> = &'a dyn Fn(&str) -> Option<*const ClassDesc>;

/// What one node of a walked graph becomes.
///
/// Implemented twice and no more — the module docs' decision that the carrier
/// is a trait over the walk rather than a second walk.
trait Carrier {
    /// What this carrier produces per node. [`Live`] produces a [`Value`];
    /// [`Encode`] produces the node's index, which is what a back-reference
    /// names.
    type Node: Clone;

    /// May this allocation be reused rather than rebuilt? See the module docs
    /// on a move at refcount 1 being the semantics rather than an optimisation.
    fn adopt(&mut self, value: Value) -> bool;

    /// A node with no reachable structure: `null`, a `bool`, an `int`, a
    /// `uint`, a `float` or a `decimal`.
    fn scalar(&mut self, value: Value) -> Self::Node;

    /// A `string` or a `bytes`, whose contents are `text`.
    fn text(&mut self, tag: Tag, text: &[u8], adopted: Option<Value>) -> Self::Node;

    /// The second and later encounter of an object already walked.
    fn backref(&mut self, seen: &Self::Node) -> Self::Node;

    /// An array of `keys.len()` entries, before any of them is walked.
    fn open_array(&mut self, keys: &[SlotKey], adopted: Option<Value>) -> Self::Node;

    /// An instance of `class`, before any of its fields is walked. Registered
    /// in the identity map immediately, which is what lets a cycle terminate.
    fn open_object(&mut self, class: &ClassDesc, adopted: Option<Value>) -> Self::Node;

    /// § 2's *unresolvable class*: may an instance of `class` exist on the
    /// receiving side at all?
    ///
    /// A carrier question rather than one of [`refusable`]'s, because the two
    /// carriers have different receiving sides — [`Encode`] writes the class
    /// name into the payload and the answer is `decode`'s to give, once it
    /// knows the table it is decoding *into*, while [`Live`] has that table in
    /// hand at the crossing. The default is therefore "yes", and only `Live`
    /// overrides it.
    ///
    /// # Errors
    ///
    /// [`GraphError`] naming the class, per § 2's "never a stub".
    fn admit(&self, _class: &ClassDesc) -> Result<(), GraphError> {
        Ok(())
    }

    /// Entry or field `index` of `holder`, now that it has been walked. `key`
    /// is the array key it goes under, and `None` for an object's field slot.
    fn close_entry(
        &mut self,
        holder: &Self::Node,
        index: usize,
        key: Option<&SlotKey>,
        child: Self::Node,
    );

    /// Give up a half-built holder, because a node beneath it was refused.
    fn discard(&mut self, node: Self::Node);
}

/// The identity map: one entry per distinct object reached.
type Seen<N> = HashMap<*mut ObjHeader, N>;

/// The one traversal. **Consumes one reference to `value`.**
///
/// Every caller arranges that reference: [`copy_graph`] is handed one, and a
/// child slot is retained before it is descended into, so an adopted holder and
/// a freshly built one take the same path through here.
fn walk<C: Carrier>(
    carrier: &mut C,
    value: Value,
    seen: &mut Seen<C::Node>,
    depth: u32,
) -> Result<C::Node, GraphError> {
    if depth > MAX_DEPTH {
        release(value);
        return Err(GraphError(format!(
            "the graph nests deeper than the limit of {MAX_DEPTH}"
        )));
    }
    let tag = value.tag().ok_or_else(|| {
        GraphError(format!(
            "a value with tag {} has no meaning on the other side",
            value.tag_byte()
        ))
    })?;
    match tag {
        Tag::Null | Tag::Bool | Tag::Int | Tag::Uint | Tag::Float | Tag::Decimal => {
            Ok(carrier.scalar(value))
        }
        Tag::Str | Tag::Bytes => {
            let adopt = carrier.adopt(value);
            // The borrow ends with this call: `as_str_bytes` reads the
            // allocation the caller's reference keeps alive.
            let text = borrow_str(value)
                .expect("a `Str` or `Bytes` value carries a string header")
                .as_bytes()
                .to_vec();
            let node = carrier.text(tag, &text, adopt.then_some(value));
            if !adopt {
                release(value);
            }
            Ok(node)
        }
        Tag::Array => {
            let adopt = carrier.adopt(value);
            let array = borrow_array(value);
            let mut keys = Vec::with_capacity(array.count());
            let mut values = Vec::with_capacity(array.count());
            let mut slot = 0;
            while let Some(live) = array.next_slot(slot) {
                keys.push(
                    array
                        .slot_key(live)
                        .expect("a live slot has a key beside its value"),
                );
                values.push(
                    array
                        .value_at(live)
                        .expect("a live slot has a value beside its key"),
                );
                slot = live + 1;
            }
            let holder = carrier.open_array(&keys, adopt.then_some(value));
            for (index, (key, entry)) in keys.iter().zip(values).enumerate() {
                retain(entry);
                let child = match walk(carrier, entry, seen, depth + 1) {
                    Ok(child) => child,
                    Err(why) => {
                        carrier.discard(holder);
                        if !adopt {
                            release(value);
                        }
                        return Err(why);
                    }
                };
                carrier.close_entry(&holder, index, Some(key), child);
            }
            if !adopt {
                release(value);
            }
            Ok(holder)
        }
        Tag::Object => {
            let ptr = value
                .obj_ptr()
                .expect("an `Object` value carries an object header");
            if let Some(node) = seen.get(&ptr) {
                let node = carrier.backref(node);
                release(value);
                return Ok(node);
            }
            let object = borrow_object(value);
            // The borrow above kept the descriptor alive: an instance's class
            // outlives every instance of it (`crate::object`).
            #[expect(
                unsafe_code,
                reason = "the descriptor came from a live instance, whose class \
                          table outlives it by `ClassTable`'s own contract"
            )]
            let class: &ClassDesc = unsafe { &*object.class() };
            refusable(class)?;
            carrier.admit(class)?;
            let adopt = carrier.adopt(value);
            let holder = carrier.open_object(class, adopt.then_some(value));
            seen.insert(ptr, holder.clone());
            for index in 0..object.field_count() {
                if class.field_is_secret(index) {
                    carrier.discard(holder);
                    if !adopt {
                        release(value);
                    }
                    return Err(GraphError(format!(
                        "{}::${} is `secret` and does not cross a copy boundary \
                         unless it is revealed",
                        class.name(),
                        class.field_name(index).unwrap_or("?")
                    )));
                }
                let field = object.field(index);
                retain(field);
                let child = match walk(carrier, field, seen, depth + 1) {
                    Ok(child) => child,
                    Err(why) => {
                        carrier.discard(holder);
                        if !adopt {
                            release(value);
                        }
                        return Err(why);
                    }
                };
                carrier.close_entry(&holder, index, None, child);
            }
            if !adopt {
                release(value);
            }
            Ok(holder)
        }
        // A closure is refused as the object it is ([`refusable`]), so the one
        // tag left here is the never-written storage state, whose own name is
        // what the message has to say rather than a type spelling.
        Tag::Unset => Err(GraphError(
            "a never-written property has no meaning on the other side of a \
             copy boundary"
                .to_owned(),
        )),
    }
}

/// § 2's "refuses what has no meaning on the other side", for the shapes that
/// arrive wearing [`Tag::Object`].
///
/// Both refusals are a bit on the class and nothing structural. A closure is
/// [`ClassDesc::is_closure()`] and not a declared `invoke`, which is a method
/// name a program may use and refusing on it would make a user class
/// uncopyable for spelling it. A host handle is
/// [`ClassDesc::holds_host_handle()`] and not the shape of the slot, which is a
/// `uint` like every other: the key indexes the table of the [`crate::Ctx`]
/// that opened it ([`crate::Ctx::hold_open_socket`]), so a copy that crossed
/// would address the *receiving* side's table at that index and read whatever
/// that side has open there.
fn refusable(class: &ClassDesc) -> Result<(), GraphError> {
    if class.is_closure() {
        return Err(GraphError(
            "a closure captures a heap and a scope, so it has no meaning on the \
             other side of a copy boundary"
                .to_owned(),
        ));
    }
    if class.holds_host_handle() {
        return Err(GraphError(format!(
            "a {} holds a handle onto something this side of the boundary \
             opened, so it has no meaning on the other side of a copy boundary \
             — pass what identifies the resource and open it there",
            class.name()
        )));
    }
    Ok(())
}

// ============================================================================
// The live carrier
// ============================================================================

/// § 2's first carrier: arena-to-arena, with no intervening byte
/// representation.
///
/// `receiving` is the class table the copy lands in, when the crossing has one
/// — the isolate boundary's answer, on its way back to a parent that compiled
/// its own unit. `None` is a copy that does not leave the program that made it
/// (`clone`, and the argument going *in*, whose destination table does not
/// exist yet), where every class is by construction the receiving side's own.
struct Live<'a> {
    receiving: Option<Receiving<'a>>,
}

impl Carrier for Live<'_> {
    type Node = Value;

    /// § 2's third bullet, at the live carrier: **the same class, not a class
    /// of the same name.**
    ///
    /// Identity is the descriptor's address, which is stricter than resolving
    /// the name and is deliberately so. A copy keeps the descriptor it was
    /// built with — that is what makes an adopted allocation a pointer handoff
    /// rather than a rebuild (`rule:security/isolate-values-cross-by-copy`) — so admitting a same-named class
    /// from another compiled unit would hand the receiving side an object
    /// whose field indices are the *sender's* layout and whose methods are the
    /// sender's compiled code. That is a hole in the isolation the boundary
    /// exists for, and the priority ordering does not trade rule 1 for the
    /// convenience of rule 4.
    ///
    /// What it costs is worth stating plainly: `nvs-cli`'s resolver compiles
    /// one unit per written path, so today a class declared in both files is
    /// two descriptors and an instance of it does not cross. Making it cross
    /// is a question about *sharing a class table between units*, not about
    /// this walk.
    fn admit(&self, class: &ClassDesc) -> Result<(), GraphError> {
        let Some(receiving) = self.receiving else {
            return Ok(());
        };
        let name = class.name();
        match receiving(name) {
            Some(desc) if std::ptr::eq(desc, std::ptr::from_ref(class)) => Ok(()),
            Some(_) => Err(GraphError(format!(
                "`{name}` on the receiving side is a different class, so an \
                 instance of this one has no meaning there"
            ))),
            None => Err(GraphError(format!(
                "`{name}` is not a class the receiving side declares"
            ))),
        }
    }

    /// § 2's move, and **the one place in the runtime an allocation changes
    /// owners** — so an object's live-list membership changes here too, inside
    /// this implementation rather than at any call site.
    ///
    /// `crate::object`'s *Decision: every object is on its context's live list*
    /// owns the reasoning; the short of it is that an adopted object left on
    /// the source context's list is one that context's teardown sweep may
    /// dismantle while the destination is still holding it. Only an object
    /// needs this: a string and an array are on no list, because neither can
    /// close a cycle (this module's decision that identity is object identity).
    fn adopt(&mut self, value: Value) -> bool {
        // The walk holds one reference; if it is the only one, nothing else
        // can observe that this allocation was reused.
        match value.tag() {
            Some(Tag::Str | Tag::Bytes) => {
                borrow_str(value).is_some_and(|held| held.refcount() == 1)
            }
            Some(Tag::Array) => borrow_array(value).refcount() == 1,
            Some(Tag::Object) => {
                if borrow_object(value).refcount() != 1 {
                    return false;
                }
                let ptr = value
                    .obj_ptr()
                    .expect("an `Object` value carries an object header");
                #[expect(
                    unsafe_code,
                    reason = "the walk holds the allocation's only reference, \
                              which is what this arm just established"
                )]
                unsafe {
                    crate::object::relink_to_current(ptr);
                }
                true
            }
            _ => false,
        }
    }

    fn scalar(&mut self, value: Value) -> Value {
        value
    }

    fn text(&mut self, tag: Tag, text: &[u8], adopted: Option<Value>) -> Value {
        if let Some(value) = adopted {
            return value;
        }
        let held = NvsStr::new(text);
        if tag == Tag::Bytes {
            Value::bytes(held)
        } else {
            Value::str(held)
        }
    }

    fn backref(&mut self, seen: &Value) -> Value {
        retain(*seen);
        *seen
    }

    fn open_array(&mut self, _keys: &[SlotKey], adopted: Option<Value>) -> Value {
        adopted.unwrap_or_else(|| Value::array(NvsArray::new()))
    }

    fn open_object(&mut self, class: &ClassDesc, adopted: Option<Value>) -> Value {
        adopted.unwrap_or_else(|| {
            #[expect(
                unsafe_code,
                reason = "the descriptor belongs to a live instance's class \
                          table, which outlives every instance made from it"
            )]
            let object = unsafe { NvsObj::new(std::ptr::from_ref(class)) };
            Value::object(object)
        })
    }

    fn close_entry(&mut self, holder: &Value, index: usize, key: Option<&SlotKey>, child: Value) {
        match key {
            // A `Live` holder is always uniquely owned here — freshly built, or
            // adopted at refcount 1 — so an array write never separates the
            // handle from the value registered beside it.
            Some(SlotKey::Index(at)) => borrow_array(*holder).set_index(*at, child),
            Some(SlotKey::Str(name)) => borrow_array(*holder).set(name.clone(), child),
            None => borrow_object(*holder).set_field(index, child),
        }
    }

    fn discard(&mut self, node: Value) {
        release(node);
    }
}

/// § 2's live carrier, as one call. **Consumes one reference to `value`** and
/// returns one, which is the same allocation wherever a move at refcount 1 let
/// it be adopted.
///
/// # Errors
///
/// [`GraphError`] naming the value that has no meaning on the other side, or
/// the `secret` property that may not cross unrevealed.
pub fn copy_graph(value: Value) -> Result<Value, GraphError> {
    copy_graph_into(value, None)
}

/// The same copy, made *into* a program whose class table `receiving` answers
/// for — [`Live::admit`] is the whole of the difference.
///
/// One function with an `Option` rather than two, because a caller that has a
/// receiving table and a caller that does not are the same crossing otherwise,
/// and the walk may not learn which one it is running under.
///
/// # Errors
///
/// Everything [`copy_graph`] refuses, plus an object whose class the receiving
/// side does not have — `rule:classes/graph-copy`'s third bullet, which [`Live::admit`]
/// owns the reading of.
pub fn copy_graph_into(
    value: Value,
    receiving: Option<Receiving<'_>>,
) -> Result<Value, GraphError> {
    let mut seen = Seen::new();
    walk(&mut Live { receiving }, value, &mut seen, 0)
}

// ============================================================================
// The byte carrier
// ============================================================================

/// § 2's second carrier: the same walk, appending `rule:classes/serialize-is-a-closed-format`'s closed format.
struct Encode {
    /// The payload so far, magic and version already written.
    out: Vec<u8>,
    /// How many objects have been opened — the index a back-reference names.
    objects: usize,
}

impl Encode {
    /// A length or an index, LEB128.
    fn varint(&mut self, mut value: u64) {
        loop {
            let byte = u8::try_from(value & 0x7f).expect("seven bits fit in a byte");
            value >>= 7;
            if value == 0 {
                self.out.push(byte);
                return;
            }
            self.out.push(byte | 0x80);
        }
    }

    /// A length-prefixed byte run.
    fn blob(&mut self, bytes: &[u8]) {
        self.varint(length(bytes.len()));
        self.out.extend_from_slice(bytes);
    }

    /// One array key, tagged by which of [`SlotKey`]'s shapes it is so a
    /// list's integer keys survive the round trip as integers.
    fn key(&mut self, key: &SlotKey) {
        match key {
            SlotKey::Index(at) => {
                self.out.push(0);
                self.out.extend_from_slice(&at.to_le_bytes());
            }
            SlotKey::Str(name) => {
                self.out.push(1);
                let name = name.as_bytes().to_vec();
                self.blob(&name);
            }
        }
    }
}

impl Carrier for Encode {
    /// The node's own index, which is only ever read for an object.
    type Node = usize;

    fn adopt(&mut self, _value: Value) -> bool {
        false
    }

    fn scalar(&mut self, value: Value) -> usize {
        match value.tag() {
            Some(Tag::Bool) => {
                self.out.push(node::BOOL);
                self.out
                    .push(u8::from(value.as_bool().expect("a `Bool` carries one")));
            }
            Some(Tag::Int) => {
                self.out.push(node::INT);
                let held = value.as_int().expect("an `Int` carries one");
                self.out.extend_from_slice(&held.to_le_bytes());
            }
            Some(Tag::Uint) => {
                self.out.push(node::UINT);
                let held = value.as_uint().expect("a `Uint` carries one");
                self.out.extend_from_slice(&held.to_le_bytes());
            }
            Some(Tag::Float) => {
                self.out.push(node::FLOAT);
                let held = value.as_float().expect("a `Float` carries one");
                self.out.extend_from_slice(&held.to_bits().to_le_bytes());
            }
            Some(Tag::Decimal) => {
                self.out.push(node::DECIMAL);
                let held = value.as_decimal().expect("a `Decimal` carries one");
                self.out.extend_from_slice(&held.to_bits().to_le_bytes());
            }
            _ => self.out.push(node::NULL),
        }
        0
    }

    fn text(&mut self, tag: Tag, text: &[u8], _adopted: Option<Value>) -> usize {
        self.out.push(if tag == Tag::Bytes {
            node::BYTES
        } else {
            node::STR
        });
        self.blob(text);
        0
    }

    fn backref(&mut self, seen: &usize) -> usize {
        self.out.push(node::BACKREF);
        self.varint(length(*seen));
        *seen
    }

    fn open_array(&mut self, keys: &[SlotKey], _adopted: Option<Value>) -> usize {
        self.out.push(node::ARRAY);
        self.varint(length(keys.len()));
        for key in keys {
            self.key(key);
        }
        0
    }

    fn open_object(&mut self, class: &ClassDesc, _adopted: Option<Value>) -> usize {
        self.out.push(node::OBJECT);
        let name = class.name().as_bytes().to_vec();
        self.blob(&name);
        self.varint(length(class.field_count()));
        // Every declared property's name, *before* any of their values — which
        // is what lets a decode check § 3's "recorded property set does not
        // exactly match the target class's current declared properties" before
        // it builds anything at all.
        for index in 0..class.field_count() {
            let field = class
                .field_name(index)
                .unwrap_or_default()
                .as_bytes()
                .to_vec();
            self.blob(&field);
        }
        let id = self.objects;
        self.objects += 1;
        id
    }

    fn close_entry(
        &mut self,
        _holder: &usize,
        _index: usize,
        _key: Option<&SlotKey>,
        _child: usize,
    ) {
        // The child appended itself in walk order, so there is nothing to join.
    }

    fn discard(&mut self, _node: usize) {
        // A refusal throws the whole buffer away, so a half-written node in it
        // is never read.
    }
}

/// § 2's externalizing carrier — `Core\Serialize::encode`'s whole body.
/// **Consumes one reference to `value`.**
///
/// # Errors
///
/// [`GraphError`], exactly as [`copy_graph`] does: the two carriers refuse the
/// same graphs because they share the walk that decides.
pub fn encode(value: Value) -> Result<Vec<u8>, GraphError> {
    let mut carrier = Encode {
        out: MAGIC.to_vec(),
        objects: 0,
    };
    carrier.out.push(VERSION);
    let mut seen = Seen::new();
    walk(&mut carrier, value, &mut seen, 0)?;
    Ok(carrier.out)
}

// ============================================================================
// Reading the bytes back
// ============================================================================

/// The decoder: the byte carrier run backwards.
///
/// Not a [`Carrier`] — there is nothing to walk on the way in, since the
/// structure is the payload's rather than a live value's. What it *shares* with
/// the walk is the grammar above and the identity rule: objects have indices,
/// and an object is registered before its fields are read, which is what makes
/// a cycle decode rather than recurse.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    /// Every object built so far, by the index [`Encode::open_object`] gave it.
    objects: Vec<Value>,
    /// The program's class table, asked by name.
    resolve: &'a dyn Fn(&str) -> Option<*const ClassDesc>,
}

/// The refusal every short read makes, one place rather than at each `?`.
fn truncated() -> GraphError {
    GraphError("the payload ends in the middle of a value".to_owned())
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, GraphError> {
        let byte = *self.bytes.get(self.at).ok_or_else(truncated)?;
        self.at += 1;
        Ok(byte)
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], GraphError> {
        let end = self.at.checked_add(N).ok_or_else(truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or_else(truncated)?;
        self.at = end;
        Ok(slice.try_into().expect("the slice is exactly N bytes long"))
    }

    fn varint(&mut self) -> Result<u64, GraphError> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.byte()?;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(GraphError("a length runs past 64 bits".to_owned()))
    }

    fn blob(&mut self) -> Result<Vec<u8>, GraphError> {
        let len = usize::try_from(self.varint()?).map_err(|_| truncated())?;
        let end = self.at.checked_add(len).ok_or_else(truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or_else(truncated)?;
        self.at = end;
        Ok(slice.to_vec())
    }

    fn key(&mut self) -> Result<SlotKey, GraphError> {
        match self.byte()? {
            0 => Ok(SlotKey::Index(i64::from_le_bytes(self.fixed::<8>()?))),
            1 => Ok(SlotKey::Str(NvsStr::new(&self.blob()?))),
            other => Err(GraphError(format!("{other} is not an array key shape"))),
        }
    }

    /// One node, and everything beneath it.
    fn node(&mut self, depth: u32) -> Result<Value, GraphError> {
        if depth > MAX_DEPTH {
            return Err(GraphError(format!(
                "the payload nests deeper than the limit of {MAX_DEPTH}"
            )));
        }
        match self.byte()? {
            node::NULL => Ok(Value::null()),
            node::BOOL => Ok(Value::bool(self.byte()? != 0)),
            node::INT => Ok(Value::int(i64::from_le_bytes(self.fixed::<8>()?))),
            node::UINT => Ok(Value::uint(u64::from_le_bytes(self.fixed::<8>()?))),
            node::FLOAT => Ok(Value::float(f64::from_le_bytes(self.fixed::<8>()?))),
            node::DECIMAL => {
                let bits = u128::from_le_bytes(self.fixed::<16>()?);
                let held = Decimal::from_bits(bits)
                    .ok_or_else(|| GraphError("a `decimal` node is malformed".to_owned()))?;
                Ok(Value::decimal(held))
            }
            node::STR => Ok(Value::str(NvsStr::new(&self.blob()?))),
            node::BYTES => Ok(Value::bytes(NvsStr::new(&self.blob()?))),
            node::ARRAY => {
                let count = usize::try_from(self.varint()?).map_err(|_| truncated())?;
                let mut keys = Vec::with_capacity(count.min(self.bytes.len()));
                for _ in 0..count {
                    keys.push(self.key()?);
                }
                let mut array = NvsArray::new();
                for key in keys {
                    let child = self.node(depth + 1)?;
                    match key {
                        SlotKey::Index(at) => array.set_index(at, child),
                        SlotKey::Str(name) => array.set(name, child),
                    }
                }
                Ok(Value::array(array))
            }
            node::OBJECT => self.object(depth),
            node::BACKREF => {
                let id = usize::try_from(self.varint()?).map_err(|_| truncated())?;
                let held = *self
                    .objects
                    .get(id)
                    .ok_or_else(|| GraphError("a back-reference names no object".to_owned()))?;
                retain(held);
                Ok(held)
            }
            other => Err(GraphError(format!("{other} is not a node shape"))),
        }
    }

    /// One object node — § 3's refusals, every one of them made *before* a
    /// single field is read.
    fn object(&mut self, depth: u32) -> Result<Value, GraphError> {
        let name = String::from_utf8(self.blob()?)
            .map_err(|_| GraphError("a class name is not UTF-8".to_owned()))?;
        let count = usize::try_from(self.varint()?).map_err(|_| truncated())?;
        let mut recorded = Vec::with_capacity(count.min(self.bytes.len()));
        for _ in 0..count {
            recorded.push(
                String::from_utf8(self.blob()?)
                    .map_err(|_| GraphError("a property name is not UTF-8".to_owned()))?,
            );
        }
        let desc = (self.resolve)(&name).ok_or_else(|| {
            GraphError(format!(
                "`{name}` is not a class this program declares or `Core` owns"
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "the resolver answers with a descriptor from the compiled \
                      unit's class table or from the process's leaked `Core` \
                      one, and both outlive this decode"
        )]
        let class: &ClassDesc = unsafe { &*desc };
        if class.field_count() != recorded.len() {
            return Err(GraphError(format!(
                "`{name}` declares {} properties and the payload records {}",
                class.field_count(),
                recorded.len()
            )));
        }
        for (index, held) in recorded.iter().enumerate() {
            let declared = class.field_name(index).unwrap_or_default();
            if declared != held {
                return Err(GraphError(format!(
                    "`{name}` declares `${declared}` where the payload records `${held}`"
                )));
            }
        }
        // Registered before its fields are read: that is what a cycle's
        // back-reference finds, and § 2's "a cycle terminates instead of
        // recursing forever" on this side.
        #[expect(
            unsafe_code,
            reason = "the descriptor came from a class table that outlives every \
                      instance made from it — the unit's, or the leaked `Core` one"
        )]
        let object = unsafe { NvsObj::new(desc) };
        let value = Value::object(object);
        self.objects.push(value);
        for index in 0..class.field_count() {
            let child = self.node(depth + 1)?;
            borrow_object(value).set_field(index, child);
        }
        retain(value);
        Ok(value)
    }
}

/// § 3's `Core\Serialize::decode`: the closed format read back, or a refusal.
///
/// `resolve` answers with the descriptor for a class name, from whichever table
/// holds it: [`crate::Ctx::class_desc`] is the resolver every call site passes,
/// and it asks the compiled unit's own classes first and the `Core` library's
/// second.
///
/// # Errors
///
/// [`GraphError`] for a payload that is not this format, that nests past
/// [`MAX_DEPTH`], that names a class this program cannot resolve, or whose
/// recorded property set is not the class's current one. Every one of them is
/// made before the offending object is filled, so nothing partially built is
/// ever handed back.
pub fn decode(
    bytes: &[u8],
    resolve: &dyn Fn(&str) -> Option<*const ClassDesc>,
) -> Result<Value, GraphError> {
    if bytes.len() < MAGIC.len() + 1 || &bytes[..MAGIC.len()] != MAGIC {
        return Err(GraphError(
            "the payload does not carry Novis's serialization marker".to_owned(),
        ));
    }
    let version = bytes[MAGIC.len()];
    if version != VERSION {
        return Err(GraphError(format!(
            "the payload is format version {version}, and this build reads {VERSION}"
        )));
    }
    let mut reader = Reader {
        bytes,
        at: MAGIC.len() + 1,
        objects: Vec::new(),
        resolve,
    };
    let value = reader.node(0)?;
    // Every object the reader registered still holds the reference it was built
    // with; the graph itself holds whatever it needs, so those are surplus.
    for held in std::mem::take(&mut reader.objects) {
        release(held);
    }
    if reader.at != bytes.len() {
        release(value);
        return Err(GraphError(
            "the payload carries bytes past the end of its value".to_owned(),
        ));
    }
    Ok(value)
}

// ============================================================================
// Borrowing, without taking the caller's reference
// ============================================================================

/// One more owner of `value`'s allocation, for the walk's own use.
fn retain(value: Value) {
    #[expect(
        unsafe_code,
        reason = "the caller holds a live reference for the length of the call, \
                  which is exactly `Value::retain`'s obligation"
    )]
    unsafe {
        value.retain();
    }
}

/// One fewer owner.
fn release(value: Value) {
    #[expect(
        unsafe_code,
        reason = "the walk consumed one reference to every value it is handed, \
                  and this is where that reference is given up"
    )]
    unsafe {
        value.release();
    }
}

/// `value`'s object, borrowed rather than adopted — the handle is forgotten on
/// the way out, so the caller's reference is untouched.
fn borrow_object(value: Value) -> std::mem::ManuallyDrop<NvsObj> {
    let ptr = value
        .obj_ptr()
        .expect("an `Object` value carries an object header");
    #[expect(
        unsafe_code,
        reason = "the caller holds a live reference to this allocation, and the \
                  handle is never dropped"
    )]
    let object = unsafe { NvsObj::from_raw(ptr) };
    std::mem::ManuallyDrop::new(object)
}

/// `value`'s array, borrowed the same way.
fn borrow_array(value: Value) -> std::mem::ManuallyDrop<NvsArray> {
    let ptr = value
        .array_ptr()
        .expect("an `Array` value carries an array header");
    #[expect(
        unsafe_code,
        reason = "the caller holds a live reference to this allocation, and the \
                  handle is forgotten rather than dropped"
    )]
    let array = unsafe { NvsArray::from_raw(ptr) };
    std::mem::ManuallyDrop::new(array)
}

/// A `usize` as the format's own width. Every one is a length or an index into
/// a payload already held in memory, so none can exceed `u64`.
fn length(value: usize) -> u64 {
    u64::try_from(value).expect("a length of a payload held in memory fits in `u64`")
}

/// `value`'s string, borrowed the same way.
fn borrow_str(value: Value) -> Option<std::mem::ManuallyDrop<NvsStr>> {
    let ptr = value.buffer_ptr()?;
    #[expect(
        unsafe_code,
        reason = "the caller holds a live reference to this allocation, and the \
                  handle is never dropped"
    )]
    let held = unsafe { NvsStr::from_raw(ptr) };
    Some(std::mem::ManuallyDrop::new(held))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{ClassTable, MethodRow};

    /// A table with one two-field class, `Node { value, next }` — enough to
    /// build sharing and a cycle, which are the two properties § 2 states.
    fn table() -> ClassTable {
        let mut table = ClassTable::new();
        table.define("Node", &["value", "next"], &[]);
        table
    }

    /// A fresh `Node` with `value` set and `next` left null.
    fn node(table: &ClassTable, value: Value) -> Value {
        let id = table.id_of("Node").expect("the table defines `Node`");
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(id)) };
        object.set_field(0, value);
        Value::object(object)
    }

    /// The resolver a decode is handed in these tests.
    fn resolver(table: &ClassTable) -> impl Fn(&str) -> Option<*const ClassDesc> + '_ {
        move |name: &str| Some(table.desc(table.id_of(name)?))
    }

    /// `rule:classes/graph-copy`'s "one operation, two carriers", asserted where a second
    /// implementation would show: both carriers are asked the same questions
    /// over the same graphs and must **agree**, rather than each being right on
    /// its own line.
    #[test]
    fn the_boundary_copy_and_serialize_share_one_walk() {
        // A closure, spelled the way `refusable` recognizes one: the bit, not
        // the `invoke`.
        let mut closures = ClassTable::new();
        let id = closures.define("Closure", &["arity"], &[]);
        closures.set_methods(
            id,
            vec![MethodRow {
                name: crate::closure::CLOSURE_INVOKE.to_owned(),
                code: std::ptr::dangling(),
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                public: true,
                native: false,
            }],
        );
        closures.set_closure(id);
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let closure = Value::object(unsafe { NvsObj::new(closures.desc(id)) });

        // A `secret` property, which § 2's walk refuses on the same pass.
        let mut wallets = ClassTable::new();
        let id = wallets.define("Wallet", &["key"], &[]);
        wallets.set_secret_fields(id, vec![true]);
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let wallet = Value::object(unsafe { NvsObj::new(wallets.desc(id)) });

        // A host handle, spelled the way `refusable` recognizes one: the bit,
        // not the `uint` slot the key sits in.
        let mut sockets = ClassTable::new();
        let id = sockets.define(r"Core\Http\Socket", &["held"], &[]);
        sockets.set_host_handle(id);
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let socket = Value::object(unsafe { NvsObj::new(sockets.desc(id)) });
        borrow_object(socket).set_field(0, Value::uint(3));

        for (what, subject, refused) in [
            ("a closure", closure, true),
            ("a secret property", wallet, true),
            ("a host handle", socket, true),
            ("an int", Value::int(7), false),
        ] {
            retain(subject);
            let live = copy_graph(subject).err().map(|why| why.0);
            retain(subject);
            let bytes = encode(subject).err().map(|why| why.0);
            assert_eq!(live.is_some(), refused, "{what} surprised the live carrier");
            assert_eq!(
                live, bytes,
                "{what} was refused differently by the two carriers, so they are \
                 not one walk"
            );
            release(subject);
        }
    }

    /// § 2's refusal is about what a value *is*, so a class a program declared
    /// crosses both carriers however it spelled its method names — the
    /// [`ClassDesc::is_closure()`] bit is the whole test, and `invoke` is a
    /// name a program may use.
    #[test]
    fn a_class_declaring_invoke_is_not_a_closure() {
        let mut table = ClassTable::new();
        let id = table.define("Command", &["code"], &[]);
        table.set_methods(
            id,
            vec![MethodRow {
                name: crate::closure::CLOSURE_INVOKE.to_owned(),
                code: std::ptr::dangling(),
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                public: true,
                native: false,
            }],
        );
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let command = Value::object(unsafe { NvsObj::new(table.desc(id)) });
        borrow_object(command).set_field(0, Value::int(7));

        retain(command);
        let copied = copy_graph(command).expect("a declared `invoke` is not a closure");
        assert_eq!(borrow_object(copied).field(0).as_int(), Some(7));
        release(copied);

        retain(command);
        let bytes = encode(command).expect("both carriers answer the same question");
        let back = decode(&bytes, &resolver(&table)).expect("and it decodes back");
        assert_eq!(borrow_object(back).field(0).as_int(), Some(7));
        release(back);
        release(command);
    }

    /// § 2's third refusal, `rule:security/isolate-values-cross-by-copy`'s "an
    /// object holding a host handle is owned by this process": the mark is the
    /// class's bit, so the refusal names the class, and a `uint` slot on a
    /// class carrying no bit is an ordinary number that crosses.
    #[test]
    fn an_object_holding_a_host_handle_is_refused_by_name() {
        let mut table = ClassTable::new();
        let opened = table.define(r"Core\Db\Connection", &["handle", "name"], &[]);
        table.set_host_handle(opened);
        let counter = table.define("Counter", &["ticks"], &[]);

        #[expect(unsafe_code, reason = "the table outlives the object")]
        let connection = Value::object(unsafe { NvsObj::new(table.desc(opened)) });
        borrow_object(connection).set_field(0, Value::uint(1));
        retain(connection);
        let why = copy_graph(connection).expect_err("a held connection does not cross");
        assert!(
            why.0.contains(r"Core\Db\Connection"),
            "the refusal names the offending value's class: {}",
            why.0
        );
        release(connection);

        // The bit and nothing else: the same slot on a class nothing marked is
        // a number, and the walk has no reason to look at it twice.
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let ticks = Value::object(unsafe { NvsObj::new(table.desc(counter)) });
        borrow_object(ticks).set_field(0, Value::uint(1));
        retain(ticks);
        let copied = copy_graph(ticks).expect("an unmarked class crosses with its `uint` intact");
        assert_eq!(borrow_object(copied).field(0).as_uint(), Some(1));
        release(copied);
        release(ticks);
    }

    /// § 2's move: at refcount 1 the allocation is *reused*, which is the
    /// semantics rather than an optimisation — asserted by pointer, because
    /// nothing about the copy's contents could tell the two apart.
    #[test]
    fn a_refcount_one_value_moves_rather_than_copying() {
        let table = table();
        let unique = node(&table, Value::int(1));
        let before = unique.obj_ptr();
        let moved = copy_graph(unique).expect("an `int` field crosses");
        assert_eq!(moved.obj_ptr(), before, "a uniquely-owned node was rebuilt");

        // The other half of the bound: one more owner, and it is copied.
        retain(moved);
        let copied = copy_graph(moved).expect("an `int` field crosses");
        assert_ne!(copied.obj_ptr(), before, "a shared node was reused");
        assert_eq!(borrow_object(copied).field(0).as_int(), Some(1));
        release(copied);
        release(moved);
    }

    /// § 2's cycle rule, from both carriers: the walk terminates, and what
    /// comes back is cyclic in the same place rather than unrolled or cut.
    #[test]
    fn a_cyclic_value_crosses_without_hanging() {
        let table = table();
        let ring = node(&table, Value::int(1));
        // `$ring->next = $ring`, which is where a naive walk never returns.
        retain(ring);
        borrow_object(ring).set_field(1, ring);

        retain(ring);
        let copy = copy_graph(ring).expect("a cycle crosses");
        assert_ne!(copy.obj_ptr(), ring.obj_ptr(), "refcount 2 is not a move");
        assert_eq!(
            borrow_object(copy).field(1).obj_ptr(),
            copy.obj_ptr(),
            "the copy's cycle closes somewhere else"
        );

        retain(ring);
        let bytes = encode(ring).expect("a cycle encodes");
        let back = decode(&bytes, &resolver(&table)).expect("a cycle decodes");
        assert_eq!(
            borrow_object(back).field(1).obj_ptr(),
            back.obj_ptr(),
            "the decoded cycle closes somewhere else"
        );
        assert_eq!(borrow_object(back).field(0).as_int(), Some(1));

        // Each ring holds itself, which is the leak a refcount cannot see; the
        // field is cleared so these fixtures do not become one.
        for held in [back, copy, ring] {
            borrow_object(held).set_field(1, Value::null());
            release(held);
        }
    }

    /// § 3's first bullet: a payload that does not carry Novis's marker and
    /// version is refused outright — never best-effort parsed, never partially
    /// accepted.
    #[test]
    fn bytes_that_are_not_our_format_are_refused_whole() {
        let table = table();
        let resolve = resolver(&table);
        let good = encode(node(&table, Value::int(3))).expect("an `int` field crosses");
        let accepted = decode(&good, &resolve).expect("the fixture is our format");
        release(accepted);

        for (what, payload) in [
            ("an empty", Vec::new()),
            ("a PHP-format", b"a:1:{i:0;i:1;}".to_vec()),
            ("a truncated-magic", good[..3].to_vec()),
            ("a wrong-version", {
                let mut bent = good.clone();
                bent[MAGIC.len()] = VERSION + 1;
                bent
            }),
            ("a truncated-body", good[..good.len() - 1].to_vec()),
            ("a trailing-bytes", {
                let mut extra = good.clone();
                extra.push(node::NULL);
                extra
            }),
        ] {
            assert!(
                decode(&payload, &resolve).is_err(),
                "the {what} payload was accepted"
            );
        }
    }

    /// § 3's third bullet: a recorded property set that is not the class's
    /// **current** one is refused naming the mismatch — never coerced, never
    /// filled with a type default, and refused before the object is built.
    #[test]
    fn a_class_whose_declared_properties_no_longer_match_is_refused_whole() {
        let written = {
            let table = table();
            encode(node(&table, Value::int(5))).expect("an `int` field crosses")
        };

        // The same class name, one property fewer — a build where `next` was
        // removed since the payload was written.
        let mut narrowed = ClassTable::new();
        narrowed.define("Node", &["value"], &[]);
        let why = decode(&written, &resolver(&narrowed)).expect_err("the shape changed");
        assert!(
            why.message().contains("declares 1 properties"),
            "the refusal did not name the mismatch: {why}"
        );

        // The same count, one property renamed — the case a length check alone
        // would accept.
        let mut renamed = ClassTable::new();
        renamed.define("Node", &["value", "tail"], &[]);
        let why = decode(&written, &resolver(&renamed)).expect_err("the shape changed");
        assert!(
            why.message().contains("`$tail`"),
            "the refusal did not name the mismatch: {why}"
        );

        // And § 3's second bullet, on the same payload: a class the receiving
        // side cannot resolve at all.
        let empty = ClassTable::new();
        let why = decode(&written, &resolver(&empty)).expect_err("the class is gone");
        assert!(
            why.message().contains("`Node`"),
            "the refusal did not name the class: {why}"
        );
    }
}
