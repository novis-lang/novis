//! `Core\Reflect` — [ADR 0019](../../../../docs/adr/0019-reflection-and-ast-parsing-are-core-features.md)'s
//! read-only structural introspection, as the member that describes a value and
//! the description it answers with.
//!
//! # Decision: the description is built from the instance's descriptor, and
//! from nothing else
//!
//! A reflective walk is handed a value whose class the checker never saw — that
//! is the whole point of asking at run time — so the only thing it has is the
//! `nvs_runtime::ClassDesc` the object points back at. Everything § 1 describes
//! is therefore a question that descriptor has to be able to answer, and the one
//! this member needs is *visibility*: § 2 makes a reflective read face the check
//! ordinary code at that site would face, and a `private int $n` is
//! byte-identical to a `public int $n` in every representation below the
//! checker. So the bit is carried down from `nvs_types::layout`, exactly as ADR
//! 0092 § 5's `secret` bit is, and
//! [`nvs_runtime::ClassDesc::field_is_public`] is what this module asks. The
//! rejected alternative was a compile-time table keyed by class name: it answers
//! nothing for a class reached through a `mixed`, which is the only receiver
//! shape reflection is for.
//!
//! # Decision: `ClassInfo` holds its answers, rather than the described class
//!
//! [`CLASS_INFO`]'s two slots are the class's name and its visible property
//! names, both computed at [`nvs_core_reflect_for_object`] time. A slot holding
//! the descriptor itself would be smaller and would defer the walk — but
//! [`crate::instance`]'s first decision is that a `Core` instance is an ordinary
//! Novis object, so every slot must be a value Novis already holds, and a raw
//! descriptor pointer is neither that nor something a hot-reload swap
//! ([ADR 0017](../../../../docs/adr/0017-hot-reload-without-restart.md)) leaves
//! valid. Holding the answers instead makes the description exactly as inert as
//! § 1 says it is: nothing it carries can be dereferenced back into the program.
//!
//! **What it spends:** one array of one string per visible property, per
//! `forObject` call, charged to the request that asked and released with the
//! description. A program that describes the same class in a loop pays per
//! call; the alternative is a per-core cache keyed by descriptor address, which
//! nothing yet needs.
//!
//! # Decision: a description reads its own class's instances, and says so
//!
//! [`nvs_core_reflect_class_info_get`] takes the object back as an argument
//! rather than the description holding one, because a description is of a
//! *class*: `forObject` is the only way to reach one, and a description that
//! captured its subject would be a second reference to it with a lifetime
//! nobody asked for. What that costs is a pairing the member has to check, so it
//! does: a value whose class is not the described one is a `LogicError`, not a
//! best-effort read of whatever slot happens to share the name. Exact class,
//! not `instanceof` — a subclass has its own description, one call away, and
//! answering for it here would mean answering visibility from one class's table
//! about another class's slot.
//!
//! # Decision: `TypeKind` is one case per representation, and no case is a
//! question about a value
//!
//! [`TYPE_KIND`] replaces fourteen `is_*` predicates plus `gettype`
//! (`docs/spec/01-core-library.md` § 20) by being *finer* than any of them and
//! overlapping none of them: `is_scalar` and `is_int` both answer `true` for a
//! `7`, so a program asking both learns nothing the second time, while ten
//! cases that partition [`nvs_runtime::Tag`]'s value-carrying half answer the
//! whole family in one call and a `match` with no `default` is exhaustive.
//! That is also why the cases stop where the *representations* do. `Callable`
//! is not one, because ADR 0031 makes a closure an ordinary object and a case
//! for it would be a second case one value satisfies; `Numeric` is not one,
//! because `is_numeric` asks about a `string`'s **contents** and that is
//! `Core\Validate`'s question, not this member's; and `Iterable` and
//! `Countable` are not, because they ask what a value can *do* — which is
//! [`CLASS_INFO`]'s half of this class, one call away and answering for a
//! class rather than for a tag.
//!
//! # Known gaps
//!
//! 1. § 1's remaining `*Info` classes are not here yet — a description names
//!    its properties and no methods, so `get_class_methods` and
//!    `method_exists` have no answer here yet; the spec's roster row
//!    (`docs/spec/01-core-library.md` § 20) is the home of the full list.
//! 2. A description's property walk is the same from inside the described class
//!    as from outside it. § 2's rule is stated over the *call site*, and a
//!    native member has no view of its caller's class — so this answers the
//!    narrower question, which is the one that cannot leak a member.
//! 3. Reading a property is here; *calling* a method and *writing* a property,
//!    which § 2 governs on the same terms and which additionally owe ADR 0014's
//!    hook, are not.

use nvs_runtime::{ClassDesc, Fault, NvsArray, NvsObj, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, as a program writes it.
pub(crate) const NAME: &str = "Core\\Reflect";

/// The described class's own name, as a program writes it.
pub(crate) const CLASS_INFO_NAME: &str = "Core\\Reflect\\ClassInfo";

/// [`CLASS_INFO`]'s slot holding the described class's name.
const NAME_SLOT: usize = 0;

/// [`CLASS_INFO`]'s slot holding the described class's visible property names.
const PROPERTIES_SLOT: usize = 1;

/// `Core\Reflect` — the door onto a description, and nothing that acts.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "forClass",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            // `?ClassInfo`, and the `null` is what replaces `class_exists`:
            // a name the running program declares no class for is an absence
            // (ADR 0063 R6), not a failure — see the card.
            return_ty: CoreTy::Nullable(&CoreTy::Instance(CLASS_INFO_NAME)),
            symbol: "nvs_core_reflect_for_class",
            doc: Some(&FOR_CLASS_DOC),
        },
        CoreMethod {
            name: "forObject",
            names: &["object"],
            // `mixed` rather than a class type: reflection exists for the receiver
            // whose class the checker does not know, and there is no spelling for
            // "any object" that a `mixed` does not already cover.
            params: &[CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Instance(CLASS_INFO_NAME),
            symbol: "nvs_core_reflect_for_object",
            doc: Some(&FOR_OBJECT_DOC),
        },
        CoreMethod {
            name: "typeOf",
            names: &["value"],
            // `mixed` for the same reason `forObject`'s is, and here it is the
            // *only* meaningful argument type: on anything narrower the checker
            // already knows the answer and the call would be a constant.
            params: &[CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Enum(TYPE_KIND_NAME),
            symbol: "nvs_core_reflect_type_of",
            doc: Some(&TYPE_OF_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Reflect::forClass`'s reference card — ADR 0117.
const FOR_CLASS_DOC: MethodDoc = MethodDoc {
    short: "Describes the class `$name` names, reaching it by name rather than through a value. \
            Replaces `ReflectionClass`'s constructor and `class_exists`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The class's name as its declaration writes it, namespace included and with no \
               leading separator — what `Core\\Reflect\\ClassInfo::name` answers.",
        shape: &[],
    }],
    ret: "A description of that class, or `null` where the running program declares no class of \
          that name — the `null` is `class_exists`'s answer, which is why asking is not a \
          failure.",
    errors: &[],
};

/// `Core\Reflect::forObject`'s reference card — ADR 0117.
const FOR_OBJECT_DOC: MethodDoc = MethodDoc {
    short: "Describes `$object`'s class — its name, and the properties code outside the class can \
            see. Replaces `get_class` and `get_object_vars`.",
    params: &[ParamDoc {
        name: "object",
        desc: "The value to describe. Reflection is for the receiver whose class is not known \
               while compiling, so the parameter is `mixed`.",
        shape: &[],
    }],
    ret: "A `Core\\Reflect\\ClassInfo` for the object's own class, carrying answers rather than a \
          way back to the object.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$object` is not an object — a `mixed` carries no promise that it is one, so the \
               check is made here rather than by the caller.",
    }],
};

/// `Core\Reflect::typeOf`'s reference card — ADR 0117.
const TYPE_OF_DOC: MethodDoc = MethodDoc {
    short: "Which of the language's representations `$value` currently holds. The single \
            replacement for PHP's fourteen `is_*` predicates and `gettype`, which are only \
            meaningful on a `mixed` at all.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to ask about. On anything but a `mixed` the checker already knows the \
               answer, so the interesting receiver is the one whose type was erased.",
        shape: &[],
    }],
    ret: "One `Core\\Reflect\\TypeKind` case — exactly one, since the cases partition the \
          representations rather than overlapping the way `is_scalar` and `is_int` do.",
    errors: &[],
};

/// [`TYPE_KIND`]'s name, as a program writes it.
pub(crate) const TYPE_KIND_NAME: &str = r"Core\Reflect\TypeKind";

/// The answer [`nvs_core_reflect_type_of`] gives — one case per representation
/// a value can be in.
///
/// The roster is `nvs_runtime::Tag`'s value-carrying half and nothing else, and
/// this module's own doc comment owns why: a case that no value can produce is
/// surface with nothing behind it, and a *pair* of cases one value could
/// satisfy would put the caller back to asking a second question. The values
/// are ordinals in declaration order, per ADR 0010 and the roster's siblings —
/// deliberately not the tag byte, which is a representation this enum must be
/// able to outlive.
pub(crate) const TYPE_KIND: CoreEnum = CoreEnum {
    name: TYPE_KIND_NAME,
    cases: &[
        ("Null", 0),
        ("Bool", 1),
        ("Int", 2),
        ("Uint", 3),
        ("Float", 4),
        ("Decimal", 5),
        ("Text", 6),
        ("Bytes", 7),
        ("Array", 8),
        ("Object", 9),
    ],
    doc: Some(&TYPE_KIND_DOC),
};

/// [`TYPE_KIND`]'s reference card — ADR 0117.
const TYPE_KIND_DOC: EnumDoc = EnumDoc {
    short: "What a value is, once its static type is gone — ten cases, one per representation the \
            runtime has, and every value is in exactly one of them.",
    cases: &[
        CaseDoc {
            name: "Null",
            desc: "The `null` value; what `is_null` asked.",
        },
        CaseDoc {
            name: "Bool",
            desc: "A `bool`, `true` or `false` alike.",
        },
        CaseDoc {
            name: "Int",
            desc: "A signed `int`.",
        },
        CaseDoc {
            name: "Uint",
            desc: "An unsigned `uint`, which is a type of its own here and so a case of its own — \
                   the one PHP had no predicate to ask with.",
        },
        CaseDoc {
            name: "Float",
            desc: "A `float`; what `is_float` and its `is_double` alias asked.",
        },
        CaseDoc {
            name: "Decimal",
            desc: "A `decimal` — the exact scalar, and never a `float` that happens to be \
                   round.",
        },
        CaseDoc {
            name: "Text",
            desc: "A `string`, which is UTF-8 by the language's own guarantee; what `is_string` \
                   asked.",
        },
        CaseDoc {
            name: "Bytes",
            desc: "A `bytes` value — the same heap shape as `Text` without the UTF-8 promise, and \
                   the distinction PHP's one string type could not make.",
        },
        CaseDoc {
            name: "Array",
            desc: "An `array<T>`; what `is_array`, `is_iterable` and `is_countable` between them \
                   asked.",
        },
        CaseDoc {
            name: "Object",
            desc: "A class instance, a closure included — a closure is an ordinary object here, \
                   so there is no `Callable` case to disagree with it.",
        },
    ],
};

/// `Core\Reflect\ClassInfo` — what [`CLASS`]'s member answers with.
pub(crate) const CLASS_INFO: CoreClass = CoreClass {
    name: CLASS_INFO_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_class_info_name",
            doc: Some(&NAME_DOC),
        },
        CoreMethod {
            name: "properties",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_reflect_class_info_properties",
            doc: Some(&PROPERTIES_DOC),
        },
        CoreMethod {
            name: "get",
            names: &["object", "name"],
            params: &[CoreTy::Mixed, CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            // `mixed`: the whole point of the read is that the property's
            // declared type is not known where the call is written.
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_reflect_class_info_get",
            doc: Some(&GET_DOC),
        },
    ],
    slots: &["name", "properties"],
    constants: &[],
};

/// `Core\Reflect\ClassInfo::name`'s reference card — ADR 0117.
const NAME_DOC: MethodDoc = MethodDoc {
    short: "The described class's name, namespace included, spelled as the declaration writes it.",
    params: &[],
    ret: "The class name — `App\\Model\\User` for a namespaced declaration, and never an alias \
          the naming site happened to use.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::properties`'s reference card — ADR 0117.
const PROPERTIES_DOC: MethodDoc = MethodDoc {
    short: "The described class's property names, in slot order — every ancestor's first, then \
            its own.",
    params: &[],
    ret: "One name per property code outside the class may read, `$`-sigil excluded. A `private` \
          or `protected` property is not among them: reflection has the visibility ordinary code \
          has, and no way to widen it.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::get`'s reference card — ADR 0117.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Reads `$object`'s `$name` property, under exactly the visibility ordinary code at \
            this call site would face. Replaces `ReflectionProperty::getValue`, and there is no \
            `setAccessible` to lift the check with.",
    params: &[
        ParamDoc {
            name: "object",
            desc: "An instance of the described class — the description is of a class, so the \
                   value to read is named here rather than held.",
            shape: &[],
        },
        ParamDoc {
            name: "name",
            desc: "The property's name, `$`-sigil excluded, as `properties` spells it.",
            shape: &[],
        },
    ],
    ret: "The property's value, with its own declared type erased to `mixed`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$object` is not an object, or `$name` names a property that is not `public` \
                   — a reflective read has the visibility ordinary code has, so the refusal is \
                   the one an ordinary out-of-class read would meet.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$object` is not an instance of the described class, or `$name` names no \
                   property of it at all. Both are mistakes in the program rather than facts \
                   about the value, which is what separates them from the refusal above.",
        },
    ],
};

/// A `string` argument of `member`, as text.
///
/// A [`Fault::fatal`] for the wrong tag, on `crate::json`'s own `text_of`
/// terms: the parameter is a declared `string`, so compiled code wrote the tag
/// and a mismatch is the ABI's problem rather than the program's.
fn text_of<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected {:?}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// The description of one class — [`CLASS_INFO`]'s two slots, filled.
///
/// Both doors onto a description share this, which is the point: `forObject`
/// reaches a descriptor through a value and `forClass` reaches one through the
/// unit's class table, and *what a description is* must not depend on which
/// door was used. A slot the descriptor cannot name is skipped rather than
/// numbered — `field_name` answers `None` only past the last slot, so the loop
/// bound already excludes it, and a synthesized class reads as having nothing
/// visible at all.
fn describe(desc: &ClassDesc) -> Value {
    let mut visible = NvsArray::new();
    for slot in 0..desc.field_count() {
        if !desc.field_is_public(slot) {
            continue;
        }
        if let Some(name) = desc.field_name(slot) {
            visible.append(Value::str(NvsStr::new(name.as_bytes())));
        }
    }
    crate::instance::build(
        &CLASS_INFO,
        [
            Value::str(NvsStr::new(desc.name().as_bytes())),
            Value::array(visible),
        ],
    )
}

/// The [`TYPE_KIND`] case a tag is, or `None` for a tag no value carries.
///
/// The three `None`s are the whole of what [`TYPE_KIND`] leaves out, and each
/// is unreachable for its own reason rather than by omission:
/// [`Tag::Closure`] is reserved and unused, since ADR 0031's closure carries
/// [`Tag::Object`]; [`Tag::Resource`] has no representation behind it yet; and
/// [`Tag::Unset`] is a storage state that every read turns into a throw before
/// a member can see one. A fourth tag arriving here would be a new
/// representation, and answering it *some* case would be worse than the fatal
/// [`nvs_core_reflect_type_of`] gives it.
fn kind_of(tag: Tag) -> Option<i64> {
    Some(match tag {
        Tag::Null => 0,
        Tag::Bool => 1,
        Tag::Int => 2,
        Tag::Uint => 3,
        Tag::Float => 4,
        Tag::Decimal => 5,
        Tag::Str => 6,
        Tag::Bytes => 7,
        Tag::Array => 8,
        Tag::Object => 9,
        Tag::Closure | Tag::Resource | Tag::Unset => return None,
    })
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_reflect_for_class" => (nvs_core_reflect_for_class as *const ()).cast(),
        "nvs_core_reflect_for_object" => (nvs_core_reflect_for_object as *const ()).cast(),
        "nvs_core_reflect_type_of" => (nvs_core_reflect_type_of as *const ()).cast(),
        "nvs_core_reflect_class_info_name" => {
            (nvs_core_reflect_class_info_name as *const ()).cast()
        }
        "nvs_core_reflect_class_info_properties" => {
            (nvs_core_reflect_class_info_properties as *const ()).cast()
        }
        "nvs_core_reflect_class_info_get" => (nvs_core_reflect_class_info_get as *const ()).cast(),
        _ => return None,
    })
}

/// A slot of the receiving `ClassInfo`, retained because it is being answered.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not an object — unreachable from
/// source, since an instance member's receiver is typed and `E0401` refuses a
/// call on anything else.
fn slot_of(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot's reference belongs to the receiver, which is live for \
                  the length of the call, and this value is being handed to the \
                  caller — which is exactly `Value::retain`'s obligation"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect::forClass(string $name): ?Core\Reflect\ClassInfo` — ADR
    /// 0019 § 1's description, reached by name.
    ///
    /// The name is resolved against the *running program's* class table
    /// (`nvs_runtime::Ctx::class_desc`), which is the only table a native
    /// member can reach and the only one the question is about: a class the
    /// compiled unit does not carry is a class no value in this program can be
    /// an instance of. So `Core` classes are not among the answers — they
    /// declare no properties a walk could name, and their surface is the
    /// reference cards rather than a description.
    fn nvs_core_reflect_for_class(ctx, args: [1]) {
        let name = text_of(&args[0], "Core\\Reflect::forClass")?;
        let Some(desc) = ctx.class_desc(name) else {
            return Ok(Value::null());
        };
        #[expect(
            unsafe_code,
            reason = "`class_desc` answers with a pointer into the compiled unit's \
                      class table, which outlives this context and is never \
                      rewritten while a member of it is running"
        )]
        let description = unsafe { describe(&*desc) };
        Ok(description)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect::forObject(mixed $object): Core\Reflect\ClassInfo` — ADR
    /// 0019 § 1's description, reached from a value.
    ///
    /// The walk is the module doc's first decision, and [`describe`] is where
    /// it is written: the object's descriptor names every slot, and § 2's
    /// visibility bit says which of them a description is allowed to name.
    fn nvs_core_reflect_for_object(_ctx, args: [1]) {
        let ptr = args[0].obj_ptr().ok_or_else(|| {
            Fault::thrown(format!(
                "{NAME}::forObject() expected an object, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "the argument owns a reference to a live allocation, so it is \
                      live for this borrow; the handle is never dropped, so that \
                      reference is not released twice, and the descriptor is owned \
                      by the unit's class table, which outlives every instance of \
                      the class it describes"
        )]
        let description = unsafe {
            let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(ptr));
            describe(&*object.class())
        };
        Ok(description)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect::typeOf(mixed $value): Core\Reflect\TypeKind` — the one
    /// question that survives type erasure, and so the one replacement for
    /// fourteen predicates that each asked a piece of it.
    ///
    /// An enum answers as its ordinal, exactly as a user-declared enum does
    /// (ADR 0010) and as `Core\Cli::colorDepth` already does. The tag is read
    /// rather than the value: nothing here dereferences a payload, so this is
    /// the one `Core` member that is total over every argument shape without
    /// looking at one.
    fn nvs_core_reflect_type_of(_ctx, args: [1]) {
        let kind = args[0].tag().and_then(kind_of).ok_or_else(|| {
            // Not a throw: no source can write a value with one of these tags,
            // so a program reaching here is the ABI's problem rather than its
            // own — `text_of`'s fatal is the same reading.
            Fault::fatal(format!(
                "{NAME}::typeOf got tag {}, which is no value's",
                args[0].tag_byte()
            ))
        })?;
        Ok(Value::int(kind))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::name(): string` — the described class's name.
    ///
    /// A reader rather than a `->name` property, because a `Core` instance has
    /// none at all: [`crate::registry::CoreTy::Instance`] is the home of why,
    /// and `Core\RateLimit\Decision`'s four readers are the precedent.
    fn nvs_core_reflect_class_info_name(_ctx, args: [1]) {
        slot_of(args, NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::properties(): array<string>` — ADR 0019 § 2's
    /// visibility-respecting walk, answered off the slot the description was
    /// built with.
    fn nvs_core_reflect_class_info_properties(_ctx, args: [1]) {
        slot_of(args, PROPERTIES_SLOT, "properties")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::get(mixed $object, string $name): mixed` — ADR
    /// 0019 § 2's rule that *acting* on a member faces the ordinary check.
    ///
    /// Four refusals in one order, and the order is the point: the value has to
    /// be an object, it has to be an instance of the class this description is
    /// of, the name has to be one of that class's slots, and only then is
    /// visibility asked. A member that asked visibility first would answer a
    /// misspelling with the same refusal as a `private` read, which is
    /// precisely the confusion `examples/reflect.nvs` catches on `RuntimeError`
    /// rather than on `Throwable` to avoid.
    fn nvs_core_reflect_class_info_get(_ctx, args: [3]) {
        let member = "get";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let described = crate::instance::slot(receiver, NAME_SLOT);
        let subject = args[1].obj_ptr().ok_or_else(|| {
            Fault::thrown(format!(
                "{CLASS_INFO_NAME}::get() expected an object, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        let name = text_of(&args[2], "Core\\Reflect\\ClassInfo::get")?;
        #[expect(
            unsafe_code,
            reason = "the argument owns a reference to a live allocation, so it is \
                      live for this borrow; the handle is never dropped, so that \
                      reference is not released twice, and the descriptor is owned \
                      by the unit's class table, which outlives every instance of \
                      the class it describes"
        )]
        let (class, slot, visible) = unsafe {
            let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(subject));
            let desc = &*object.class();
            let slot = desc.field_slot(name, 0);
            (
                desc.name().to_owned(),
                slot,
                slot.is_some_and(|at| desc.field_is_public(at)),
            )
        };
        // `as_text` rather than the bytes: the slot was written by `forObject`
        // from a `Tag::Str`, and ADR 0009 § 3 makes that tag the UTF-8
        // guarantee — see `crate::str`'s `text` on why re-deriving it costs an
        // O(n) pass for nothing.
        let described = described.as_text().unwrap_or_default();
        if described != class {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{CLASS_INFO_NAME}::get(): this describes `{described}`, and the value is a \
                     `{class}`"
                ),
            ));
        }
        let Some(slot) = slot else {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{CLASS_INFO_NAME}::get(): `{class}` has no property named `{name}`"),
            ));
        };
        if !visible {
            return Err(Fault::thrown(format!(
                "{CLASS_INFO_NAME}::get(): `{class}::{name}` is not readable from outside the \
                 class, and reflection does not lift that"
            )));
        }
        let held = crate::instance::slot(subject, slot);
        #[expect(
            unsafe_code,
            reason = "the slot's reference belongs to the subject, which its argument \
                      keeps live for the length of the call, and this value is being \
                      handed to the caller — which is exactly `Value::retain`'s \
                      obligation"
        )]
        unsafe {
            held.retain();
        }
        Ok(held)
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsArray, NvsStr, OutputSink, Tag, Value, call};

    use super::{TYPE_KIND, kind_of};

    /// The case a name is declared with, so the assertions below read in the
    /// spellings a program writes rather than in ordinals.
    fn case(name: &str) -> i64 {
        TYPE_KIND
            .cases
            .iter()
            .find(|(declared, _)| *declared == name)
            .unwrap_or_else(|| panic!("`{name}` is a declared TypeKind case"))
            .1
    }

    /// The member is one call where PHP had fifteen, and that rests on two
    /// properties of the roster rather than on any one answer: every
    /// representation a value can be in has a case, and no two share one. The
    /// sweep is over the runtime's own tag roster rather than over a list
    /// written here, so a thirteenth tag fails this rather than silently
    /// answering `Object`.
    #[test]
    fn type_of_is_the_single_replacement_for_the_is_predicates() {
        let mut answered = Vec::new();
        let mut unrepresented = Vec::new();
        for byte in 0..=u8::MAX {
            let Some(tag) = Tag::from_byte(byte) else {
                continue;
            };
            match kind_of(tag) {
                Some(kind) => answered.push(kind),
                None => unrepresented.push(tag),
            }
        }

        // The three the enum leaves out, named rather than counted: each is a
        // tag no value carries, and `kind_of`'s own doc says why per tag.
        assert_eq!(
            unrepresented,
            [Tag::Closure, Tag::Resource, Tag::Unset],
            "every other tag is a value's, so every other tag owes a case"
        );

        answered.sort_unstable();
        let declared: Vec<i64> = TYPE_KIND.cases.iter().map(|(_, value)| *value).collect();
        assert_eq!(
            answered, declared,
            "the cases and the representations are the same roster: a case with \
             no tag is surface nothing can produce, and a tag with no case is a \
             value the member cannot answer for"
        );

        let asked = |value: Value| {
            let mut ctx = Ctx::new(OutputSink::Sink);
            call(super::nvs_core_reflect_type_of, &mut ctx, &[value])
                .expect("typeOf reads a tag and never fails")
                .as_int()
                .expect("an enum answers as its ordinal")
        };
        let text = Value::str(NvsStr::new(b"x"));
        let bytes = Value::bytes(NvsStr::new(b"x"));
        let array = Value::array(NvsArray::new());
        assert_eq!(asked(Value::null()), case("Null"));
        assert_eq!(asked(Value::bool(true)), case("Bool"));
        assert_eq!(asked(Value::int(7)), case("Int"));
        assert_eq!(asked(Value::uint(7)), case("Uint"));
        assert_eq!(asked(Value::float(7.5)), case("Float"));
        // The pair PHP's one string type could not tell apart, and the pair
        // `is_int`/`is_float` answered as one `is_numeric` besides.
        assert_eq!(asked(text), case("Text"));
        assert_eq!(asked(bytes), case("Bytes"));
        assert_eq!(asked(array), case("Array"));

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built for each of the \
                      three, and the helper borrowed rather than consumed them"
        )]
        unsafe {
            text.release();
            bytes.release();
            array.release();
        }
    }
}
