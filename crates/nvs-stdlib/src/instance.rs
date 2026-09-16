//! The value behind [`registry::CoreTy::Instance`]: what a native member
//! returns when the spec writes a `Core`-owned object, and how another member
//! reads one back.
//!
//! # Decision: a `Core` instance is an ordinary Novis object
//!
//! Not a native handle, not a boxed `dyn Any`, not a tag of its own. A
//! `Core\Regex\Match` is exactly what `new Point(1, 2)` produces — one
//! [`nvs_runtime::NvsObj`] allocation, a [`ClassDesc`] pointer, and 16-byte
//! [`Value`] slots behind it — so **nothing below this line learns that `Core`
//! owns a class**: refcounting, the release sweep, strict identity, `Tag`/
//! `Untag` for a `?T` return and `nvs-codegen`'s argument slots all meet a
//! shape they already had.
//!
//! The cost is that every slot must be a value Novis can already hold, so a
//! member whose state is genuinely native (a compiled `regex::Regex`) has to
//! keep that state somewhere else — for `Core\Regex` that is the pattern text
//! plus [`crate::regex`]'s per-core compiled-pattern cache, which is a lookup
//! rather than a second representation. The rejected alternative was a
//! `Tag::Resource` handle into a per-request table: it buys native state
//! directly and costs a second heap shape, a second release path, and a
//! liveness rule every future `Core` class would have to restate. `rule:security/closed-doors`'s
//! closed door on stream wrappers is the same instinct — an engine-owned
//! handle is a thing a program can hold and nothing can check.
//!
//! **What it spends:** one object allocation per instance, `FIELDS_OFFSET`
//! bytes plus 16 per declared slot, charged to the request that produced it and
//! released with it.
//!
//! # Decision: `new` on a `Core` class lowers to that class's own helper
//!
//! Most `Core` instances come from a member that produces one, and those need
//! nothing here. `docs/spec/01-core-library.md` § 9's collections are the
//! carve-out — the spec writes `new Core\ObjectSet<Tag>()` — and a `Core`
//! class still has no `constructor` member for `nvs_types` to resolve, because
//! its slots are this crate's layout rather than a surface a program fills in.
//!
//! So the roster is [`registry::CONSTRUCTORS`], one line per constructible
//! class, and `nvs-ir` lowers `new` on a name it holds to an ordinary
//! `InstKind::CoreCall` on that symbol instead of an `InstKind::New`
//! (`nvs_ir::lower::expr`'s `lower_new`). **Nothing below `nvs-ir` learns that
//! `Core` owns a class**, which is the promise this module's first decision
//! makes: codegen emits the same helper call it emits for `Core\Uuid::v4()`,
//! and the descriptor comes from the leaked table below rather than from the
//! program's own class list — which would not hold one. The rejected
//! alternative was a synthetic `constructor` row in [`registry::CLASSES`]: it
//! would make `Core\ObjectSet::constructor(…)` a spelling the checker
//! resolves, and every consumer that iterates a class's members would have to
//! learn to skip it.
//!
//! # Decision: the descriptors are one leaked table for the process
//!
//! A [`ClassDesc`]'s *address* is its identity, and it must outlive every
//! instance made from it. A compiled unit's own [`ClassTable`] cannot own these
//! — a `Core` class is not in any program's class list, and an instance can
//! outlive the unit that produced it in a hot-reload swap
//! (`rule:config/an-edit-reaches-the-next-request-without-a-restart`). So
//! this module builds one table, on first use, and **leaks** it.
//!
//! **One table for the whole process, not one per core**, because an identity
//! compiled code has to *write down* cannot be per-thread.
//! `rule:core-classes/html-literal` folds a hole-free `` html`…` `` into a
//! `Core\Html\Markup` constant in the compiled unit's own data section, and the
//! class word of that constant is written once, while compiling, into bytes
//! every core then reads — so a second core's descriptor would be a second
//! identity that constant could not name. [`ClassTable`] is `Send` and `Sync`
//! already, for the reason `nvs_runtime::object` gives about a unit's own
//! table, and a [`OnceLock`] past its first call is a read rather than a lock.
//!
//! **What it spends:** one descriptor per `Core` instance class —
//! O(classes), bounded by [`registry::CLASSES`] and growing with neither
//! traffic nor cores, which is the property
//! [AGENTS.md](/AGENTS.md)'s memory rule actually asks for. Leaked
//! rather than dropped because a dangling descriptor is a use-after-free and
//! the bytes are bounded by a compile-time roster.

use std::sync::OnceLock;

use nvs_runtime::sequence;
use nvs_runtime::{ClassDesc, ClassTable, Fault, NvsObj, ObjHeader, Tag, Value};

use crate::registry::{self, CoreClass};

/// `Core`-owned classes with instances that no program can name, and so with
/// no [`registry::CLASSES`] row: a runtime artifact rather than surface.
///
/// The singletons — the cursor § 9's collections hand a `foreach`, whose own
/// module docs own why it has no row. They are named here beside the
/// registered classes because a descriptor is a descriptor: everything below
/// [`ClassDesc`] is the same for both. A row-less *family* is a slice of its
/// own instead, chained in [`descriptors`] where this one is.
const INTERNAL_CLASSES: &[&CoreClass] = &[&crate::cursor::CLASS];

/// Every member compiled code reaches on a `Core` instance **by name** — one
/// row per class, `(member, symbol)`.
///
/// `rule:iteration/two-interfaces`'s
/// iteration trio and nothing else so far. Those three declarations are
/// bodiless (`nvs_types::iter_lib`), so a `foreach` names no helper to call and
/// dispatches on the receiver's runtime class instead — this table is what a
/// `Core` receiver answers that lookup with, and [`crate::cursor`] owns the
/// decision and the one convention difference it carries: a member reached
/// this way is handed its receiver's reference rather than borrowing it.
///
/// Deliberately not a flag on [`registry::CoreMethod`]: a row here is *not* a
/// registered member — nothing resolves `$map->iterate()` in source, no
/// `.nvst` case can call one, and `nvs_stdlib::symbols` does not list it. The
/// registry is the surface a program reaches; this is the protocol the engine
/// reaches.
///
/// **`toString` is not a row here and never becomes one.** The engine reaches
/// it by name too, but through [`ClassDesc::renderer`] rather than the method
/// table, because it keeps the ordinary `Core` convention of borrowing its
/// receiver where a row here transfers one — see [`descriptors`], which
/// derives it from the registry instead.
const DISPATCH_ROSTER: &[(&str, &[(&str, &str)])] = &[
    (
        crate::objmap::NAME,
        &[(sequence::ITERATE, crate::objmap::ITERATE_SYMBOL)],
    ),
    (
        crate::objset::NAME,
        &[(sequence::ITERATE, crate::objset::ITERATE_SYMBOL)],
    ),
    (
        crate::heap::NAME,
        &[(sequence::ITERATE, crate::heap::ITERATE_SYMBOL)],
    ),
    (
        crate::channel::NAME,
        &[
            (sequence::ITERATE, crate::channel::ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::channel::ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::channel::CURRENT_SYMBOL),
        ],
    ),
    (
        crate::io::LINES_NAME,
        &[(sequence::ITERATE, crate::io::LINES_ITERATE_SYMBOL)],
    ),
    (
        crate::io::WALK_NAME,
        &[(sequence::ITERATE, crate::io::WALK_ITERATE_SYMBOL)],
    ),
    (
        crate::db::ROWS_NAME,
        &[(sequence::ITERATE, crate::db::ROWS_ITERATE_SYMBOL)],
    ),
    // The second class here that carries all three names rather than handing a
    // [`crate::cursor`] back, and for [`crate::channel`]'s reason: the next
    // element does not exist yet when the walk is named. `crate::request`'s
    // `BODY_STREAM` docs are the argument.
    (
        crate::request::BODY_STREAM_NAME,
        &[
            (
                sequence::ITERATE,
                crate::request::BODY_STREAM_ITERATE_SYMBOL,
            ),
            (
                sequence::ADVANCE,
                crate::request::BODY_STREAM_ADVANCE_SYMBOL,
            ),
            (
                sequence::CURRENT,
                crate::request::BODY_STREAM_CURRENT_SYMBOL,
            ),
        ],
    ),
    // And the third, for the third time: `rule:http-server/an-upload-is-received-only-through-files`'s walk over a multipart
    // body's file parts, whose next part has not arrived when the walk is named.
    // `crate::request`'s `FILES` docs are the argument.
    (
        crate::request::FILES_NAME,
        &[
            (sequence::ITERATE, crate::request::FILES_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::request::FILES_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::request::FILES_CURRENT_SYMBOL),
        ],
    ),
    // And the fourth: `rule:http-server/a-part-is-consumed-in-one-of-three-ways`'s walk over one part's bytes, which is the
    // body walk above narrowed to a position in the body. `crate::request`'s
    // `PART_CONTENT` docs are the argument.
    (
        crate::request::PART_CONTENT_NAME,
        &[
            (
                sequence::ITERATE,
                crate::request::PART_CONTENT_ITERATE_SYMBOL,
            ),
            (
                sequence::ADVANCE,
                crate::request::PART_CONTENT_ADVANCE_SYMBOL,
            ),
            (
                sequence::CURRENT,
                crate::request::PART_CONTENT_CURRENT_SYMBOL,
            ),
        ],
    ),
    // And the fifth: `rule:core-classes/db-statement-members`'s walk over a statement's rows, whose next row
    // does not exist until the walk asks the server for it. `crate::db::stream`
    // is the argument, and it is the same one the three above make.
    (
        crate::db::STREAM_NAME,
        &[
            (sequence::ITERATE, crate::db::STREAM_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::db::STREAM_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::db::STREAM_CURRENT_SYMBOL),
        ],
    ),
    // And spec § 12's walk over a CSV file's records, which makes the same
    // argument over a descriptor: a record does not exist until the `advance`
    // that answers it has read that far. `crate::csv`'s `ROWS` docs are where
    // it is made, beside why `Core\IO\Lines` — which holds its lines — hands a
    // [`crate::cursor`] back instead.
    (
        crate::csv::ROWS_NAME,
        &[
            (sequence::ITERATE, crate::csv::ROWS_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::csv::ROWS_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::csv::ROWS_CURRENT_SYMBOL),
        ],
    ),
    // A streamed reply's three framings, which carry all three names for the
    // reason every walk above does: an event, a line and a chunk are framed off
    // the reply by the `advance` that answers them, so none of them exists when
    // the walk is named. `crate::http::stream`'s module doc is the argument, and
    // three classes over one implementation is [`registry::ITERABLES`]'s doing
    // rather than this table's.
    (
        crate::http::stream::EVENTS_NAME,
        &[
            (
                sequence::ITERATE,
                crate::http::stream::EVENTS_ITERATE_SYMBOL,
            ),
            (
                sequence::ADVANCE,
                crate::http::stream::EVENTS_ADVANCE_SYMBOL,
            ),
            (
                sequence::CURRENT,
                crate::http::stream::EVENTS_CURRENT_SYMBOL,
            ),
        ],
    ),
    (
        crate::http::stream::LINES_NAME,
        &[
            (sequence::ITERATE, crate::http::stream::LINES_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::http::stream::LINES_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::http::stream::LINES_CURRENT_SYMBOL),
        ],
    ),
    (
        crate::http::stream::CHUNKS_NAME,
        &[
            (
                sequence::ITERATE,
                crate::http::stream::CHUNKS_ITERATE_SYMBOL,
            ),
            (
                sequence::ADVANCE,
                crate::http::stream::CHUNKS_ADVANCE_SYMBOL,
            ),
            (
                sequence::CURRENT,
                crate::http::stream::CHUNKS_CURRENT_SYMBOL,
            ),
        ],
    ),
    (
        crate::cursor::NAME,
        &[
            (sequence::ADVANCE, crate::cursor::ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::cursor::CURRENT_SYMBOL),
        ],
    ),
];

/// `class`'s method table, as [`ClassTable::set_methods`] takes it — empty for
/// every class not on [`DISPATCH_ROSTER`], which is most of them.
///
/// Every row here is `native`, which is the whole of what
/// `nvs_runtime::MethodRow` can say about one: these addresses are `rule:errors/propagation`
/// helpers that **borrow** argument 0, and this crate holds no signature for
/// them, so the arity, the parameter tags and the parameter names a compiled
/// method's row carries are left empty rather than guessed. That field is what a caller with no
/// class in hand refuses on; the callers that reach these rows today —
/// `foreach` over a cursor, `crate::sequence` — name the member statically and
/// know the convention because they wrote it.
fn dispatch_table(class: &str) -> Vec<nvs_runtime::MethodRow> {
    DISPATCH_ROSTER
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, members)| {
            members
                .iter()
                .map(|(member, symbol)| nvs_runtime::MethodRow {
                    name: (*member).to_owned(),
                    code: crate::address_of(symbol),
                    arity: 0,
                    param_tags: 0,
                    param_names: Vec::new(),
                    public: true,
                    native: true,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The process's descriptors, built on first use and never dropped — see this
/// module's docs. A `OnceLock<&'static _>` rather than a `RwLock<ClassTable>`:
/// the table is written once and read from every construction on every core, so
/// what a reader needs is the address it was published at and nothing else.
static DESCRIPTORS: OnceLock<&'static ClassTable> = OnceLock::new();

/// The table, building and leaking it if this is the first call.
fn descriptors() -> &'static ClassTable {
    DESCRIPTORS.get_or_init(|| {
        let mut table = ClassTable::new();
        for class in registry::CLASSES
            .iter()
            .chain(INTERNAL_CLASSES.iter().copied())
            // `rule:core-classes/ast-is-inert`'s typed roster, which is the
            // other family of row-less classes and a slice rather than lines
            // above because it is one class per production of the grammar —
            // `crate::ast`'s own second decision owns why none of them is
            // registry surface.
            .chain(crate::ast::PRODUCTIONS.iter())
        {
            // A **namespace** class is what is skipped here, and declaring no
            // slots is not on its own what makes one: `Core\Socket` has
            // instance members and no slots, because a connection's whole
            // state is its isolate's — the peer and the topic queue are
            // [`nvs_runtime::Ctx`] fields, so the receiver is a handle and
            // there is nothing for it to carry. The question is whether the
            // class has instances at all, which is either roster being
            // non-empty.
            if class.slots.is_empty() && class.instance.is_empty() {
                continue;
            }
            // No parents: a `Core` class is not part of any hierarchy, so
            // `instanceof` on one answers only for itself.
            let id = table.define(class.name, class.slots, &[]);
            table.set_methods(id, dispatch_table(class.name));
            // `rule:classes/stringable`'s one rendering member, which is *not* a
            // `DISPATCH_ROSTER` row: it has its own descriptor field because
            // it keeps the ordinary `Core` convention of borrowing its
            // receiver, and it is derived from the registry rather than
            // written down here so that it cannot name a member
            // `registry::class_renders` — the check the compiler makes at
            // `echo $uri` — did not see.
            if let Some(symbol) = registry::render_symbol(class.name) {
                table.set_render(id, crate::address_of(symbol));
            }
        }
        Box::leak(Box::new(table))
    })
}

/// `class`'s descriptor.
///
/// # Panics
///
/// Panics naming the class if it declares no slots — a namespace class has no
/// instances, and asking for one is a build-time oversight in this crate.
fn descriptor(class: &CoreClass) -> *const ClassDesc {
    let table = descriptors();
    let id = table
        .id_of(class.name)
        .unwrap_or_else(|| panic!("{} is not a `Core` class with instances", class.name));
    table.desc(id)
}

/// Every `Core` descriptor a compiled unit may write into its own data section,
/// as `(class name, descriptor address)`.
///
/// A hole-free `` html`…` `` folds to a `Core\Html\Markup` constant in the
/// unit's constant pool (`rule:core-classes/html-literal`), and the class word
/// of that constant is one of these addresses, written once while compiling and
/// read by every core afterwards — which is the half of this module's
/// § *Decision: the descriptors are one leaked table for the process* that a
/// per-core table would fail.
///
/// Separate from [`crate::symbols`] because that roster is *code*, under the
/// name a call site calls it by; a descriptor is data, under a mangled name
/// only the backend can spell (`nvs_codegen::class_desc_symbol`). The two
/// crates agree on the name there and on the address here.
#[must_use]
pub fn class_descriptors() -> Vec<(&'static str, *const ClassDesc)> {
    vec![(crate::html::MARKUP_NAME, descriptor(&crate::html::MARKUP))]
}

/// Whether `value` is an instance of `class`, asked by descriptor address.
///
/// A descriptor's address **is** its identity — [`descriptors`] leaks one table
/// for the process and hands the same pointer back for the same class every
/// time — so this is a pointer comparison and never a name comparison, which
/// would be
/// both slower and true of a program's own class that spelled its name the
/// same way.
///
/// The one caller is a `mixed` parameter that has to tell one `Core` class from
/// everything else: [`crate::db`]'s bind reads `rule:core-classes/db-parameters`'s `Core\Db\InList`
/// out of a `array<mixed>` whose other elements are ordinary values. A member
/// whose parameter is *declared* as a class needs none of this — the checker
/// has already answered it.
pub(crate) fn is_instance(value: Value, class: &CoreClass) -> bool {
    let Some(object) = value.obj_ptr() else {
        return false;
    };
    #[expect(
        unsafe_code,
        reason = "the value argument owns a reference to a live allocation, so \
                  it is live for the length of this call"
    )]
    let found = unsafe { NvsObj::class_of(object) };
    std::ptr::eq(found, descriptor(class))
}

/// A fresh instance of `class`, its slots filled from `slots` in declaration
/// order, as the [`Value`] a helper returns.
///
/// Takes over each slot value's reference, exactly as
/// [`NvsObj::set_field`](nvs_runtime::NvsObj::set_field) does — so a caller
/// builds the values it means and hands them straight over.
///
/// # Panics
///
/// Panics if `slots` is not exactly as long as `class.slots`: the layout is
/// this crate's on both sides, so a mismatch is a paste error rather than
/// anything a program can cause.
pub(crate) fn build<const N: usize>(class: &CoreClass, slots: [Value; N]) -> Value {
    assert_eq!(
        N,
        class.slots.len(),
        "{} has {} slots, filled with {N} values",
        class.name,
        class.slots.len()
    );
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by this crate's leaked table, so it \
                  outlives every instance made from it — which is `NvsObj::new`'s \
                  whole safety obligation"
    )]
    let object = unsafe { NvsObj::new(descriptor(class)) };
    for (index, value) in slots.into_iter().enumerate() {
        object.set_field(index, value);
    }
    Value::object(object)
}

/// The process's *shape* descriptors — see [`shape`]. A second table beside
/// [`DESCRIPTORS`] rather than more rows in it, because a shape is not a
/// [`CoreClass`]: it has no members, no name a program resolves, and no
/// registry row. Published once for the process for [`DESCRIPTORS`]'s reason,
/// a shape descriptor's address being an identity in exactly the same way.
static SHAPES: OnceLock<&'static ClassTable> = OnceLock::new();

/// Every `rule:types/object-top` shape a `Core` member builds a value of, as
/// `(descriptor name, fields in slot order)`.
///
/// **Slot order is the field name order, sorted** — `nvs_types::ty::Ty::Shape`
/// canonicalizes `{y: …, x: …}` and `{x: …, y: …}` to one interned type by
/// sorting, so the runtime layout has to be the same order or a written shape
/// type and a built value would disagree about which slot is which.
const SHAPE_ROSTER: &[(&str, &[&str])] = &[
    (crate::issue::SHAPE, crate::issue::FIELDS),
    (crate::script::RESULT_SHAPE, crate::script::RESULT_FIELDS),
    (crate::script::FAILURE_SHAPE, crate::script::FAILURE_FIELDS),
];

/// `name`'s shape descriptor, built and leaked on first use.
///
/// # Panics
///
/// Panics naming the shape if it is not in [`SHAPE_ROSTER`].
fn shape_descriptor(name: &str) -> *const ClassDesc {
    let table = *SHAPES.get_or_init(|| {
        let mut table = ClassTable::new();
        for (shape, fields) in SHAPE_ROSTER {
            // No parents, and no methods: `rule:types/object-literal` makes a shape value an
            // anonymous *methodless* instance, so there is nothing to inherit
            // and nothing to dispatch.
            table.define(*shape, fields, &[]);
        }
        Box::leak(Box::new(table))
    });
    let id = table
        .id_of(name)
        .unwrap_or_else(|| panic!("{name} is not a `Core`-built shape"));
    table.desc(id)
}

/// A fresh `rule:types/object-top` shape value — `{path: "…", message: "…"}` — its slots
/// filled from `slots` in [`SHAPE_ROSTER`]'s order.
///
/// The same anonymous methodless instance an Novis `{…}` literal builds, so
/// nothing downstream learns that `Core` produced this one. Takes over each
/// slot value's reference, exactly as [`build`] does.
///
/// # Panics
///
/// Panics if `slots` is not exactly as long as the shape's field list, or if
/// `name` is not in [`SHAPE_ROSTER`].
pub(crate) fn shape<const N: usize>(name: &str, slots: [Value; N]) -> Value {
    let fields = SHAPE_ROSTER
        .iter()
        .find(|(shape, _)| *shape == name)
        .map_or(&[][..], |(_, fields)| fields);
    assert_eq!(
        N,
        fields.len(),
        "{name} has {} slots, filled with {N} values",
        fields.len()
    );
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by this crate's leaked table, so it \
                  outlives every instance made from it — which is `NvsObj::new`'s \
                  whole safety obligation"
    )]
    let object = unsafe { NvsObj::new(shape_descriptor(name)) };
    for (index, value) in slots.into_iter().enumerate() {
        object.set_field(index, value);
    }
    Value::object(object)
}

/// The receiver of an instance member: argument slot 0, as a raw object
/// pointer live for the length of the call.
///
/// # Errors
///
/// A `Fault::fatal` if the slot does not hold an object — the same treatment
/// every other mistyped argument slot gets, since compiled code wrote the tag
/// and `nvs_types` already checked the receiver's declared type.
pub(crate) fn receiver(
    value: Value,
    class: &CoreClass,
    member: &str,
) -> Result<*mut ObjHeader, Fault> {
    value.obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{}::{member} expected {:?} for its receiver, got tag {}",
            class.name,
            Tag::Object,
            value.tag_byte()
        ))
    })
}

/// Overwrites slot `index` of `receiver`, **releasing** what it held and
/// taking over `value`'s reference — the write half of [`slot`].
///
/// The one thing that mutates a built instance, and it exists because
/// `docs/spec/01-core-library.md` § 9's collections are the spec's only
/// mutable `Core` types: every other class here is built once and read.
pub(crate) fn set_slot(receiver: *mut ObjHeader, index: usize, value: Value) {
    #[expect(
        unsafe_code,
        reason = "the receiver argument owns a reference to a live allocation, so \
                  it is live for the length of the call, the index is one of this \
                  crate's own slot constants, and the slot held a well-formed \
                  `Value` this object owned"
    )]
    unsafe {
        nvs_runtime::nvs_object_field_set(receiver, index, value);
    }
}

/// Slot `index` of `receiver`, **borrowed** — the caller takes no reference,
/// exactly as `nvs_ir::InstKind::FieldGet` does not.
///
/// A member handing the result back to Novis code owes it a
/// [`Value::retain`](nvs_runtime::Value::retain) first; one reading it in
/// passing owes nothing.
pub(crate) fn slot(receiver: *mut ObjHeader, index: usize) -> Value {
    #[expect(
        unsafe_code,
        reason = "the receiver argument owns a reference to a live allocation, so \
                  it is live for the length of the call, and the index is one of \
                  this crate's own slot constants"
    )]
    unsafe {
        nvs_runtime::nvs_object_field_get(receiver, index)
    }
}

/// Slot `index` of the receiver in `args[0]`, **retained** for the caller —
/// the whole of a reader on a `Core` class that is built once and never
/// written to.
///
/// One function rather than one per class, because a reader is the same three
/// steps wherever it is written: check the receiver's tag, borrow the slot,
/// and take the reference the value is handed back with.
/// `Core\Socket\Message`'s four readers and `Core\Sse\Message`'s two are the
/// callers, and neither module spells the retain itself.
///
/// # Errors
///
/// The [`receiver`] fault a wrongly-tagged receiver is, which compiled code
/// cannot produce.
pub(crate) fn read_slot(
    args: &[Value],
    class: &CoreClass,
    index: usize,
    member: &str,
) -> Result<Value, Fault> {
    let held = slot(receiver(args[0], class, member)?, index);
    #[expect(
        unsafe_code,
        reason = "the slot is owned by the receiver, which the argument slot holds a \
                  reference to for the length of the call, so the copy handed back to \
                  Novis code needs a reference of its own"
    )]
    // SAFETY: the receiver is live for the length of this call, so its slot is.
    unsafe {
        held.retain();
    }
    Ok(held)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registered class with instances gets a descriptor of the right
    /// width, and asking twice answers with the same address — which is what
    /// makes a descriptor's address an identity rather than a per-call
    /// accident.
    ///
    /// The skip is [`descriptors`]'s own, so a slotless class with instance
    /// members — `Core\Socket` — is covered here rather than silently passed
    /// over.
    #[test]
    fn one_descriptor_per_instance_class() {
        for class in registry::CLASSES {
            if class.slots.is_empty() && class.instance.is_empty() {
                continue;
            }
            let first = descriptor(class);
            assert!(std::ptr::eq(first, descriptor(class)));
            #[expect(
                unsafe_code,
                reason = "the leaked table owns the descriptor for the whole \
                          process, so this borrow is sound for any lifetime"
            )]
            let desc = unsafe { &*first };
            assert_eq!(desc.name(), class.name);
            assert_eq!(desc.field_count(), class.slots.len());
        }
    }

    /// A second core answers with the *same* descriptor address, for a class
    /// and for a shape alike — this module's § *Decision: the descriptors are
    /// one leaked table for the process*.
    ///
    /// This is the half a per-thread table would fail while still passing the
    /// test above: a compiled unit writes `Core\Html\Markup`'s address into a
    /// data section once, while compiling, and every core reads those same
    /// bytes (`rule:core-classes/html-literal`). An address is carried across
    /// as a `usize` because a raw pointer is not `Send`, which is the whole
    /// property under test stated in the type system.
    #[test]
    fn every_core_sees_one_descriptor_address() {
        let here = (
            descriptor(&crate::html::MARKUP).addr(),
            shape_descriptor(crate::issue::SHAPE).addr(),
        );
        let there = std::thread::spawn(|| {
            (
                descriptor(&crate::html::MARKUP).addr(),
                shape_descriptor(crate::issue::SHAPE).addr(),
            )
        })
        .join()
        .expect("the probing thread does not panic");
        assert_eq!(here, there);
    }

    /// The same pairing for `rule:classes/stringable`'s rendering: a class
    /// [`registry::class_renders`] answers `true` for is one the checker lets
    /// `echo $x` compile against, so a `mixed` holding one has to render at
    /// run time rather than throw. The two ways it can are the two rows of
    /// that check, and this asserts each class takes exactly one of them —
    /// a carrier through `rule:security/capture-answers-the-carrier`'s slot, everything else through the
    /// descriptor's own renderer.
    #[test]
    fn every_rendering_class_carries_a_renderer_or_is_a_carrier() {
        for class in registry::CLASSES {
            if !registry::class_renders(class.name) {
                assert!(
                    class.slots.is_empty() || renderer_of(class).is_none(),
                    "{} renders at run time but not where it is written",
                    class.name
                );
                continue;
            }
            if nvs_runtime::is_carrier(class.name) {
                continue;
            }
            assert!(
                !class.slots.is_empty(),
                "{} declares a `toString` but has no instances to render",
                class.name
            );
            assert!(
                renderer_of(class).is_some(),
                "{} renders where it is written but not at run time",
                class.name
            );
        }
    }

    /// `class`'s descriptor's native renderer — the test-side spelling of the
    /// read `nvs_runtime::stringify`'s dispatch makes.
    fn renderer_of(class: &CoreClass) -> Option<*const u8> {
        #[expect(
            unsafe_code,
            reason = "the leaked table owns the descriptor for the whole \
                      process, so this borrow is sound for any lifetime"
        )]
        unsafe { &*descriptor(class) }.renderer()
    }

    /// The two rosters this module's docs pair up: a class the checker will
    /// let a `foreach` compile over has to answer the protocol at run time, or
    /// the program type-checks and faults.
    #[test]
    fn every_iterable_class_answers_the_iteration_protocol() {
        for (class, _) in registry::ITERABLES {
            assert!(
                dispatch_table(class)
                    .iter()
                    .any(|row| row.name == sequence::ITERATE),
                "{class} is `Iterable` to the checker and answers no `{}` at run time",
                sequence::ITERATE
            );
        }
        let cursor = dispatch_table(crate::cursor::NAME);
        for member in [sequence::ADVANCE, sequence::CURRENT] {
            assert!(
                cursor.iter().any(|row| row.name == member),
                "the cursor every `iterate()` hands back owes `{member}`"
            );
        }
    }

    /// A dispatch roster row reaches the descriptor, which is what a
    /// `CallVirtual` reads — the half `dispatch_table` alone does not prove.
    #[test]
    fn a_dispatch_row_lands_on_the_descriptor() {
        let desc = descriptor(&crate::cursor::CLASS);
        #[expect(
            unsafe_code,
            reason = "the leaked table owns the descriptor for the whole \
                      process, so this borrow is sound for any lifetime"
        )]
        let desc = unsafe { &*desc };
        assert!(desc.method(sequence::ADVANCE).is_some());
        assert!(desc.method(sequence::CURRENT).is_some());
        assert!(desc.method("nothing").is_none());
    }

    /// A built instance holds exactly what it was given, and one reference —
    /// the property every member returning one depends on, since the compiled
    /// caller releases that one reference and nothing else does.
    #[test]
    fn a_built_instance_owns_one_reference_and_its_slots() {
        let value = build(&crate::regex::MATCH, [Value::null(), Value::int(7)]);
        let ptr = value.obj_ptr().expect("a built instance is an object");
        assert_eq!(slot(ptr, 1).as_int(), Some(7));
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference `build` produced, and \
                      releasing it is what the compiled caller would do"
        )]
        unsafe {
            value.release();
        }
    }
}
