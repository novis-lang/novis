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
//! # Known gaps
//!
//! 1. `forClass`, `typeOf` and § 1's remaining `*Info` classes are not here
//!    yet; the spec's roster row (`docs/spec/01-core-library.md` § 20) is the
//!    home of the full list.
//! 2. A description's property walk is the same from inside the described class
//!    as from outside it. § 2's rule is stated over the *call site*, and a
//!    native member has no view of its caller's class — so this answers the
//!    narrower question, which is the one that cannot leak a member.
//! 3. Reading a property is here; *calling* a method and *writing* a property,
//!    which § 2 governs on the same terms and which additionally owe ADR 0014's
//!    hook, are not.

use nvs_runtime::{Fault, NvsArray, NvsObj, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

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
    methods: &[CoreMethod {
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
    }],
    instance: &[],
    slots: &[],
    constants: &[],
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

/// The `$name` argument, as text.
///
/// A [`Fault::fatal`] for the wrong tag, on `crate::json`'s own `text_of`
/// terms: the parameter is a declared `string`, so compiled code wrote the tag
/// and a mismatch is the ABI's problem rather than the program's.
fn text_of(value: &Value) -> Result<&str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{CLASS_INFO_NAME}::get expected {:?}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_reflect_for_object" => (nvs_core_reflect_for_object as *const ()).cast(),
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
    /// `Core\Reflect::forObject(mixed $object): Core\Reflect\ClassInfo` — ADR
    /// 0019 § 1's description, reached from a value.
    ///
    /// The walk is the module doc's first decision in five lines: the object's
    /// descriptor names every slot, and § 2's visibility bit says which of them
    /// this description is allowed to name. A slot the descriptor cannot name
    /// is skipped rather than numbered — `field_name` answers `None` only past
    /// the last slot, so the loop bound already excludes it, and a synthesized
    /// class reads as having nothing visible at all.
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
        let (class, visible) = unsafe {
            let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(ptr));
            let desc = &*object.class();
            let mut visible = NvsArray::new();
            for slot in 0..desc.field_count() {
                if !desc.field_is_public(slot) {
                    continue;
                }
                if let Some(name) = desc.field_name(slot) {
                    visible.append(Value::str(NvsStr::new(name.as_bytes())));
                }
            }
            (desc.name().to_owned(), visible)
        };
        Ok(crate::instance::build(
            &CLASS_INFO,
            [
                Value::str(NvsStr::new(class.as_bytes())),
                Value::array(visible),
            ],
        ))
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
        let name = text_of(&args[2])?;
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
