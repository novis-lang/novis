//! `Core\Reflect` — `rule:tooling/reflection-and-source-parsing-are-core-features`'s
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
//! (`rule:config/an-edit-reaches-the-next-request-without-a-restart`) leaves
//! valid. Holding the answers instead makes the description exactly as inert as
//! § 1 says it is: nothing it carries can be dereferenced back into the program.
//!
//! **What it spends:** per `forClass` or `forObject` call, two arrays of one
//! string per property — the public names and every name, which § *the call
//! site arrives as a constant* is why — plus one array and one [`METHOD_INFO`]
//! of three slots per declared method, charged to the request that asked and
//! released with the description. A program that describes the same class in a
//! loop pays per call; the alternative is a per-core cache keyed by descriptor
//! address, which nothing yet needs. The roster is built eagerly rather than on
//! the first `methods` call because the alternative is a slot holding the
//! descriptor, which the decision above rules out for every slot alike. An
//! acting call spends one decoded [`nvs_render::Source`] on top, which is two
//! short strings read out of the unit's own data section.
//!
//! # Decision: the method roster is complete, and each row carries its own
//! visibility
//!
//! [`CLASS_INFO`]'s property walk names only what the calling site may read,
//! and its method roster names every method with [`METHOD_INFO`]'s bit saying
//! which may be called. That is not two answers to one question: § 2 divides
//! *reading metadata*, which is always available, from *acting on a member*,
//! which faces the ordinary check — and a roster is metadata, while a property
//! walk is the list a `get` is about to be made against.
//!
//! `rule:core-classes/reflect` is what forces the complete list: a refusal has
//! to be distinguishable from a misspelling, and a roster that dropped the
//! `private` methods would make `hasMethod` answer `false` to both. Nothing
//! leaks by it — a name, a visibility bit and a parameter count are what the
//! declaration already published to the checker, and no state of any instance
//! is reachable through them. Acting is still
//! [`nvs_core_reflect_class_info_call`]'s, which is where § 2's check is made.
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
//! # Decision: the call site arrives as a constant of the call
//!
//! `rule:security/reflection-enforces-visibility` states its rule over the
//! **call site**, and a native member sees only the values it was handed — so
//! the compiler hands it one more: the class the call is written inside, as the
//! last argument, which [`crate::registry::CALL_SITE_MEMBERS`] owns the ABI of
//! and [`site_class`] reads. It is a constant rather than a parameter, so no
//! program can write it and none can forge one; a site inside no class arrives
//! as the zero word and is treated as outside, which is the answer that fails
//! closed.
//!
//! That is what makes [`nvs_core_reflect_class_info_properties`] answer two
//! ways off one description. A walk is the list a `get` is about to be made
//! against, so it names what *this* site may read: every declared property from
//! inside the class, the public ones from anywhere else — the two lists
//! [`describe`] fills. The method roster is the other half of the same rule and
//! does not move: naming a method is metadata, which § 2 makes always
//! available, and the bit each row carries is what a caller reads instead.
//!
//! # Decision: `TypeKind` is one case per representation, and no case is a
//! question about a value
//!
//! [`TYPE_KIND`] replaces fourteen `is_*` predicates plus `gettype`
//! (`docs/spec/01-core-library.md` § 13) by being *finer* than any of them and
//! overlapping none of them: `is_scalar` and `is_int` both answer `true` for a
//! `7`, so a program asking both learns nothing the second time, while ten
//! cases that partition [`nvs_runtime::Tag`]'s value-carrying half answer the
//! whole family in one call and a `match` with no `default` is exhaustive.
//! That is also why the cases stop where the *representations* do. `Callable`
//! is not one, because `rule:types/closure-literal` makes a closure an ordinary object and a case
//! for it would be a second case one value satisfies; `Numeric` is not one,
//! because `is_numeric` asks about a `string`'s **contents** and that is
//! `Core\Validate`'s question, not this member's; and `Iterable` and
//! `Countable` are not, because they ask what a value can *do* — which is
//! [`CLASS_INFO`]'s half of this class, one call away and answering for a
//! class rather than for a tag.
//!
//! # Known gaps
//!
//! 1. § 1's roster is short of five of the classes it names. [`METHOD_INFO`] is
//!    here, so `get_class_methods` and `method_exists` have their answers; the
//!    spec's roster row (`docs/spec/01-core-library.md` § 13) is the home of the
//!    full list, and each of the five is waiting on descriptor data no crate
//!    carries yet rather than on a decision. `PropertyInfo` and `ParameterInfo`
//!    are the near pair — a property's declared type and a parameter's name are
//!    neither of them in [`nvs_runtime::ClassDesc`], which holds a slot's name,
//!    its visibility bit and at most one [`nvs_runtime::Tag`]. `PropertyInfo`
//!    also re-asks the decision below for the walk that already exists:
//!    [`nvs_core_reflect_class_info_properties`] names only what the calling
//!    site may read, where the method roster names everything, and a row
//!    carrying its own bit is what would let the two answer alike.
//!    `ConstantInfo`,
//!    `AttributeInfo` and `EnumInfo` are the far three: a descriptor carries no
//!    constants, no attributes and no enum cases at all, so each is a join from
//!    `nvs_types` through `nvs-codegen` before it is a member here.
//!    — owner: M8
//! 2. A `protected` member is reached reflectively from the declaring class's
//!    own bodies and from nowhere else, where an ordinary call from a subclass
//!    reaches it too. [`nvs_runtime::ClassDesc`] carries one bit per member and
//!    not the level behind it, so the class a site is inside is compared for
//!    equality rather than walked up the graph the way
//!    `nvs_types::signatures`'s `is_visible_from` walks it. What closes it is a
//!    second bit carried down from `nvs_types::layout`, next to the one
//!    [`nvs_runtime::ClassDesc::field_is_public`] answers; until then the
//!    narrower answer is the one that refuses rather than the one that leaks.
//!    — owner: M8
//! 3. Invoking a constructor reflectively — the third acting member § 2 names,
//!    after the read and the write that are both here now — is not. What the
//!    write does *not* do is run a per-property `set` hook (`rule:classes/property-hooks`): it
//!    reaches storage through [`nvs_runtime::write_erased_property`], which is
//!    the erased store and not the hook call a known class's write lowers to,
//!    so a hooked property is written past its own hook and its observer is
//!    told what storage took. The observer step itself is § 3's and is not a
//!    gap.
//!    — owner: M8

use nvs_runtime::{ClassDesc, Fault, NvsArray, NvsObj, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, as a program writes it.
pub(crate) const NAME: &str = "Core\\Reflect";

/// The described class's own name, as a program writes it.
pub(crate) const CLASS_INFO_NAME: &str = "Core\\Reflect\\ClassInfo";

/// One described method's own name, as a program writes it.
pub(crate) const METHOD_INFO_NAME: &str = "Core\\Reflect\\MethodInfo";

/// [`CLASS_INFO`]'s slot holding the described class's name.
const NAME_SLOT: usize = 0;

/// [`CLASS_INFO`]'s slot holding the property names a call site outside the
/// described class may read.
const PROPERTIES_SLOT: usize = 1;

/// [`CLASS_INFO`]'s slot holding one [`METHOD_INFO`] per declared method.
const METHODS_SLOT: usize = 2;

/// [`CLASS_INFO`]'s slot holding every declared property name — the answer a
/// call site *inside* the described class gets, where its own bodies reach
/// every slot the declaration wrote.
const ALL_PROPERTIES_SLOT: usize = 3;

/// [`METHOD_INFO`]'s slot holding the method's name.
const METHOD_NAME_SLOT: usize = 0;

/// [`METHOD_INFO`]'s slot holding whether the method is `public`.
const METHOD_PUBLIC_SLOT: usize = 1;

/// [`METHOD_INFO`]'s slot holding how many parameters the method declares.
const METHOD_PARAMETERS_SLOT: usize = 2;

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
            // (`rule:core-api/shape-rules` R6), not a failure — see the card.
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

/// `Core\Reflect::forClass`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Reflect::forObject`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Reflect::typeOf`'s reference card — `rule:core-api/reference-card`.
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
/// are ordinals in declaration order, per `rule:enums/closed-integer-type` and the roster's siblings —
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

/// [`TYPE_KIND`]'s reference card — `rule:core-api/reference-card`.
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
            name: "methods",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(METHOD_INFO_NAME)),
            symbol: "nvs_core_reflect_class_info_methods",
            doc: Some(&METHODS_DOC),
        },
        CoreMethod {
            name: "hasMethod",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_reflect_class_info_has_method",
            doc: Some(&HAS_METHOD_DOC),
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
        CoreMethod {
            name: "set",
            names: &["object", "name", "value"],
            // `mixed` for `$value` for the mirror of `get`'s reason: the
            // property's declared type is not known where the call is written,
            // so what the write is checked against is the *class's* answer, at
            // run time — `nvs_runtime::write_erased_property`'s tag check.
            params: &[CoreTy::Mixed, CoreTy::Text(Qual::Neutral), CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_reflect_class_info_set",
            doc: Some(&SET_DOC),
        },
        CoreMethod {
            name: "call",
            names: &["object", "name", "arguments"],
            params: &[
                CoreTy::Mixed,
                CoreTy::Text(Qual::Neutral),
                CoreTy::Array(&CoreTy::Mixed),
            ],
            // `$arguments` is required rather than defaulted to `[]`:
            // `registry::Const` has no array variant, and a member whose
            // spec signature the registry cannot express is one this crate
            // declines to register at all rather than one it approximates.
            defaults: &[],
            // `mixed`, for `get`'s reason one line up: the method's declared
            // return type is not known where the call is written.
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_reflect_class_info_call",
            doc: Some(&CALL_DOC),
        },
    ],
    slots: &["name", "properties", "methods", "allProperties"],
    constants: &[],
};

/// `Core\Reflect\ClassInfo::name`'s reference card — `rule:core-api/reference-card`.
const NAME_DOC: MethodDoc = MethodDoc {
    short: "The described class's name, namespace included, spelled as the declaration writes it.",
    params: &[],
    ret: "The class name — `App\\Model\\User` for a namespaced declaration, and never an alias \
          the naming site happened to use.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::properties`'s reference card — `rule:core-api/reference-card`.
const PROPERTIES_DOC: MethodDoc = MethodDoc {
    short: "The described class's property names, in slot order — every ancestor's first, then \
            its own.",
    params: &[],
    ret: "One name per property code outside the class may read, `$`-sigil excluded. A `private` \
          or `protected` property is not among them: reflection has the visibility ordinary code \
          has, and no way to widen it.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::methods`'s reference card — `rule:core-api/reference-card`.
const METHODS_DOC: MethodDoc = MethodDoc {
    short: "The described class's methods — its own and every inherited one — each with its name, \
            its visibility and how many parameters it declares. Replaces `get_class_methods`.",
    params: &[],
    ret: "One `Core\\Reflect\\MethodInfo` per declared method, in name order, and a method the \
          calling site could not call is among them carrying `isPublic() === false`. Naming a \
          method is not calling it, which is why the roster is complete and \
          `Core\\Reflect\\ClassInfo::call` is where the check is made.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::hasMethod`'s reference card — `rule:core-api/reference-card`.
const HAS_METHOD_DOC: MethodDoc = MethodDoc {
    short: "Reports whether the described class declares or inherits a method named `$name`. \
            Replaces `method_exists`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The method's name, as the declaration writes it and as `methods` spells it.",
        shape: &[],
    }],
    ret: "`true` for a method the class answers for at any visibility, `false` for a name it \
          declares none of — so a refusal to call tells a `private` method from a misspelling.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::get`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Reflect\ClassInfo::set`'s reference card — `rule:core-api/reference-card`.
const SET_DOC: MethodDoc = MethodDoc {
    short: "Writes `$object`'s `$name` property, under exactly the visibility ordinary code at \
            this call site would face, and then runs the `PropertyObserver` an ordinary write \
            runs. Replaces `ReflectionProperty::setValue`, again with no `setAccessible`.",
    params: &[
        ParamDoc {
            name: "object",
            desc: "An instance of the described class — `get`'s own argument, for `get`'s reason.",
            shape: &[],
        },
        ParamDoc {
            name: "name",
            desc: "The property's name, `$`-sigil excluded, as `properties` spells it.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "What to store. It is checked against what the concrete class declares the \
                   property to hold, since the declared type is not knowable here.",
            shape: &[],
        },
    ],
    ret: "Nothing. What was stored is what a following `get` answers, and what the observer was \
          told about.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$object` is not an object, `$name` names a property that is not `public`, or \
                   `$value` is not of the type that property declares — the first two being the \
                   refusals an ordinary out-of-class write would meet, and the third the one a \
                   write through an erased view meets.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$object` is not an instance of the described class, or `$name` names no \
                   property of it at all — `get`'s pair, told apart from the refusals above for \
                   `get`'s reason.",
        },
        ErrorDoc {
            error: "Throwable",
            desc: "Whatever the class's own `onPropertySet` observer throws. It is told of the \
                   write once the value is stored, and a throw out of it still fails the write — \
                   a reflective write is not the place that changes.",
        },
    ],
};

/// The `$object` argument of an *acting* member, checked against the class the
/// receiving description is of — its pointer, and that class's name.
///
/// Every acting member asks exactly this before anything else, and the two
/// refusals are written once because a description answering the same question
/// two ways would be describing two rules rather than one class. The order is
/// [`nvs_core_reflect_class_info_get`]'s, and its doc comment owns why: a value
/// that is not an object at all is a `RuntimeError`, and one that is an object
/// of the wrong class is a `LogicError`, so a misspelling and a mis-typed
/// argument never arrive as the same refusal.
fn subject_of(
    receiver: *mut nvs_runtime::ObjHeader,
    subject: Value,
    member: &str,
) -> Result<(*mut nvs_runtime::ObjHeader, String), Fault> {
    let ptr = subject.obj_ptr().ok_or_else(|| {
        Fault::thrown(format!(
            "{CLASS_INFO_NAME}::{member}() expected an object, got tag {}",
            subject.tag_byte()
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
    let class = unsafe {
        let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(ptr));
        (*object.class()).name().to_owned()
    };
    // `as_text` rather than the bytes: the slot was written by `forObject`
    // from a `Tag::Str`, and `rule:types/conversion` makes that tag the UTF-8 guarantee —
    // see `crate::str`'s `text` on why re-deriving it costs an O(n) pass for
    // nothing.
    let described = crate::instance::slot(receiver, NAME_SLOT);
    let described = described.as_text().unwrap_or_default();
    if described != class {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{CLASS_INFO_NAME}::{member}(): this describes `{described}`, and the value is a \
                 `{class}`"
            ),
        ));
    }
    Ok((ptr, class))
}

/// The class an acting member's call site is written inside, or `None` for a
/// site inside no class at all.
///
/// The last argument of every member on [`crate::registry::CALL_SITE_MEMBERS`]
/// is where this reads from, and that roster owns the ABI: the constant carries
/// the whole `Class::member` label the enclosing frame has, of which the class
/// half is the only part a visibility question is asked against. A label is
/// split on its first `::` because a class name cannot hold one and a member's
/// own label can — a property hook's is `Class::$prop::get`.
///
/// `None` covers three sites that are all outside: a script frame, a callable
/// reference's thunk, and a carrier that is the zero word for any other reason.
/// Every one of them is refused what a `private` member would refuse, which is
/// the direction `docs/agent/loop-goal.md` § *Standing decisions* fixes.
fn site_class(operand: Value) -> Option<String> {
    #[expect(
        unsafe_code,
        reason = "the carrier came out of a `SourceConst` the compiled unit baked into its own data section, which outlives every request served from it"
    )]
    let source = unsafe { nvs_runtime::source::of_operand(operand) }?;
    let member = source.member?;
    let (class, _) = member.split_once("::")?;
    Some(class.to_owned())
}

/// `Core\Reflect\ClassInfo::call`'s reference card — `rule:core-api/reference-card`.
const CALL_DOC: MethodDoc = MethodDoc {
    short: "Calls `$object`'s `$name` method with `$arguments`, under exactly the visibility \
            ordinary code at this call site would face. Replaces `ReflectionMethod::invoke`, and \
            there is no `setAccessible` to lift the check with.",
    params: &[
        ParamDoc {
            name: "object",
            desc: "An instance of the described class — the description is of a class, so the \
                   value to call on is named here rather than held.",
            shape: &[],
        },
        ParamDoc {
            name: "name",
            desc: "The method's name, `()` excluded, as the declaration writes it.",
            shape: &[],
        },
        ParamDoc {
            name: "arguments",
            desc: "One entry per declared parameter, in order, keys ignored. Required even where \
                   the method takes none, which is then `[]`.",
            shape: &[],
        },
    ],
    ret: "Whatever the method returned, with its own declared type erased to `mixed`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$name` is not `public`, names no method of the class, or names a `Core` \
                   member; or `$arguments` has fewer entries than the method declares, or an \
                   entry whose type the parameter does not accept. Every one of these is the \
                   refusal an ordinary call through an erased receiver meets, raised by that same \
                   check rather than by a second one written here.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$object` is not an object at all. A `$object` that is an object but not an \
                   instance of the described class is the `LogicError` above.",
        },
    ],
};

/// `Core\Reflect\MethodInfo` — one row of [`CLASS_INFO`]'s roster: a method's
/// name, its visibility and how many parameters it declares.
///
/// Every member is a reader over a slot the description was built with, for
/// [`CLASS_INFO`]'s own reason: what it carries is answers, not a way back into
/// the class it came from. Nothing here acts —
/// [`nvs_core_reflect_class_info_call`] is the one door onto an invocation, and
/// it is where § 2's check is made.
pub(crate) const METHOD_INFO: CoreClass = CoreClass {
    name: METHOD_INFO_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_method_info_name",
            doc: Some(&METHOD_NAME_DOC),
        },
        CoreMethod {
            name: "isPublic",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_reflect_method_info_is_public",
            doc: Some(&IS_PUBLIC_DOC),
        },
        CoreMethod {
            name: "parameterCount",
            names: &[],
            params: &[],
            defaults: &[],
            // `uint`, and the receiver is not among them: a declared parameter
            // list is what a call site is judged against, which is
            // [`nvs_runtime::MethodRow::arity`]'s own reading.
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_reflect_method_info_parameter_count",
            doc: Some(&PARAMETER_COUNT_DOC),
        },
    ],
    slots: &["name", "public", "parameterCount"],
    constants: &[],
};

/// `Core\Reflect\MethodInfo::name`'s reference card — `rule:core-api/reference-card`.
const METHOD_NAME_DOC: MethodDoc = MethodDoc {
    short: "The method's name, as the declaring class writes it.",
    params: &[],
    ret: "The name with no class qualifier and no parentheses — what `hasMethod` and `call` take.",
    errors: &[],
};

/// `Core\Reflect\MethodInfo::isPublic`'s reference card — `rule:core-api/reference-card`.
const IS_PUBLIC_DOC: MethodDoc = MethodDoc {
    short: "Whether code outside the declaring class may call the method.",
    params: &[],
    ret: "`false` for a `private` or `protected` method, which is still listed: knowing that a \
          method exists and may not be called is what tells a refusal from a misspelling, and \
          neither answer reaches any state the declaration did not expose.",
    errors: &[],
};

/// `Core\Reflect\MethodInfo::parameterCount`'s reference card — `rule:core-api/reference-card`.
const PARAMETER_COUNT_DOC: MethodDoc = MethodDoc {
    short: "How many parameters the method declares, excluding the implicit receiver.",
    params: &[],
    ret: "The count an argument list is judged against — the same number a call through \
          `Core\\Reflect\\ClassInfo::call` must supply.",
    errors: &[],
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

/// The description of one class — [`CLASS_INFO`]'s slots, filled.
///
/// Both doors onto a description share this, which is the point: `forObject`
/// reaches a descriptor through a value and `forClass` reaches one through the
/// unit's class table, and *what a description is* must not depend on which
/// door was used. A slot the descriptor cannot name is skipped rather than
/// numbered — `field_name` answers `None` only past the last slot, so the loop
/// bound already excludes it, and a synthesized class reads as having nothing
/// visible at all.
///
/// The property walk and the method walk answer the visibility bit differently,
/// and the module doc's § *the roster is complete* owns why: a property is
/// named only where it is readable, and a method is named whatever it is, with
/// [`METHOD_PUBLIC_SLOT`] carrying the answer. The one name skipped is a method
/// whose name holds a `#`, which no source can spell — `nvs_ir::lower`'s
/// generator transform mints those, and a roster naming one would be naming a
/// rewriting rather than a declaration.
///
/// *Where it is readable* is two answers, so the walk fills two slots: the
/// public names and every name, of which
/// [`nvs_core_reflect_class_info_properties`] picks the one its call site is
/// owed. Both are built here rather than filtered per call because the
/// descriptor is not reachable from a description — the module doc's § *holds
/// its answers* is that decision — and building the wider list costs the walk
/// it is already doing.
fn describe(desc: &ClassDesc) -> Value {
    let mut visible = NvsArray::new();
    let mut declared = NvsArray::new();
    for slot in 0..desc.field_count() {
        let Some(name) = desc.field_name(slot) else {
            continue;
        };
        declared.append(Value::str(NvsStr::new(name.as_bytes())));
        if desc.field_is_public(slot) {
            visible.append(Value::str(NvsStr::new(name.as_bytes())));
        }
    }
    let mut methods = NvsArray::new();
    for index in 0..desc.method_count() {
        let Some(row) = desc.method_at(index) else {
            continue;
        };
        if row.name.contains('#') {
            continue;
        }
        methods.append(crate::instance::build(
            &METHOD_INFO,
            [
                Value::str(NvsStr::new(row.name.as_bytes())),
                Value::bool(row.public),
                Value::uint(u64::from(row.arity)),
            ],
        ));
    }
    crate::instance::build(
        &CLASS_INFO,
        [
            Value::str(NvsStr::new(desc.name().as_bytes())),
            Value::array(visible),
            Value::array(methods),
            Value::array(declared),
        ],
    )
}

/// The [`TYPE_KIND`] case a tag is, or `None` for a tag no value carries.
///
/// The three `None`s are the whole of what [`TYPE_KIND`] leaves out, and each
/// is unreachable for its own reason rather than by omission:
/// [`Tag::Closure`] is reserved and unused, since `rule:types/closure-literal`'s closure carries
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
        "nvs_core_reflect_class_info_methods" => {
            (nvs_core_reflect_class_info_methods as *const ()).cast()
        }
        "nvs_core_reflect_class_info_has_method" => {
            (nvs_core_reflect_class_info_has_method as *const ()).cast()
        }
        "nvs_core_reflect_method_info_name" => {
            (nvs_core_reflect_method_info_name as *const ()).cast()
        }
        "nvs_core_reflect_method_info_is_public" => {
            (nvs_core_reflect_method_info_is_public as *const ()).cast()
        }
        "nvs_core_reflect_method_info_parameter_count" => {
            (nvs_core_reflect_method_info_parameter_count as *const ()).cast()
        }
        "nvs_core_reflect_class_info_get" => (nvs_core_reflect_class_info_get as *const ()).cast(),
        "nvs_core_reflect_class_info_set" => (nvs_core_reflect_class_info_set as *const ()).cast(),
        "nvs_core_reflect_class_info_call" => {
            (nvs_core_reflect_class_info_call as *const ()).cast()
        }
        _ => return None,
    })
}

/// A slot of the receiving description, retained because it is being answered.
///
/// Shared by [`CLASS_INFO`]'s readers and [`METHOD_INFO`]'s, which is why the
/// class is an argument: both hold their answers rather than a way back to what
/// they describe, so every reader of either is this one call.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not an object — unreachable from
/// source, since an instance member's receiver is typed and `E0401` refuses a
/// call on anything else.
fn slot_of(args: &[Value], class: &CoreClass, index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], class, member)?;
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
    /// (`rule:enums/closed-integer-type`) and as `Core\Cli::colorDepth` already does. The tag is read
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
        slot_of(args, &CLASS_INFO, NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::properties(): array<string>` — `rule:security/reflection-enforces-visibility`'s
    /// visibility-respecting walk, answered off the slot its **call site** is
    /// owed.
    ///
    /// The rule is stated over the site and not over the description, so the
    /// same description answers two ways: inside the described class every
    /// declared name, which is what an ordinary body there reads, and anywhere
    /// else the public ones alone. [`describe`] filled both slots, and the last
    /// argument — [`crate::registry::CALL_SITE_MEMBERS`]' constant, which no
    /// program can write — is what picks between them.
    fn nvs_core_reflect_class_info_properties(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, "properties")?;
        let described = crate::instance::slot(receiver, NAME_SLOT);
        let described = described.as_text();
        let inside = site_class(args[1]).is_some_and(|site| described == Some(site.as_str()));
        let slot = if inside { ALL_PROPERTIES_SLOT } else { PROPERTIES_SLOT };
        slot_of(args, &CLASS_INFO, slot, "properties")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::methods(): array<Core\Reflect\MethodInfo>` —
    /// ADR 0019 § 1's roster, replacing `get_class_methods`.
    ///
    /// The whole roster, and § 2 is why: naming a method reads the program's
    /// *shape*, which is always available, while calling one is acting and goes
    /// through [`nvs_core_reflect_class_info_call`]'s check. Answered off the
    /// slot [`describe`] filled, so two calls on one description walk nothing
    /// twice.
    fn nvs_core_reflect_class_info_methods(_ctx, args: [1]) {
        slot_of(args, &CLASS_INFO, METHODS_SLOT, "methods")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::hasMethod(string $name): bool` — replacing
    /// `method_exists`.
    ///
    /// A linear scan of the roster rather than a fourth slot keyed by name: a
    /// class's method list is short, and the alternative is a second copy of
    /// every name that would have to be kept saying the same thing as the
    /// first. It answers `true` for a `private` method, which is the whole
    /// point — `rule:core-classes/reflect` asks that a refusal be
    /// distinguishable from a misspelling, and this is the member that
    /// distinguishes them.
    fn nvs_core_reflect_class_info_has_method(_ctx, args: [2]) {
        let member = "hasMethod";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let name = text_of(&args[1], "Core\\Reflect\\ClassInfo::hasMethod")?;
        let held = crate::instance::slot(receiver, METHODS_SLOT);
        let Some(ptr) = held.array_ptr() else {
            return Err(Fault::fatal(format!(
                "{CLASS_INFO_NAME}::{member} expected {:?} in its roster slot, got tag {}",
                Tag::Array,
                held.tag_byte()
            )));
        };
        let roster = crate::arr::borrowed(ptr);
        // By slot rather than by key: [`describe`] builds this array with
        // `append` alone and hands it to nobody who can unset an entry, so its
        // slots are exactly `0..count` and a scan of them names every row once.
        let found = (0..roster.count()).any(|index| {
            let Some(row) = roster.value_at(index) else {
                return false;
            };
            let Some(row) = row.obj_ptr() else {
                return false;
            };
            crate::instance::slot(row, METHOD_NAME_SLOT).as_text() == Some(name)
        });
        Ok(Value::bool(found))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\MethodInfo::name(): string` — the method's own name.
    fn nvs_core_reflect_method_info_name(_ctx, args: [1]) {
        slot_of(args, &METHOD_INFO, METHOD_NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\MethodInfo::isPublic(): bool` — the visibility bit
    /// `nvs_types::layout` fixed at the declaration and
    /// [`nvs_runtime::MethodRow::public`] carried down.
    fn nvs_core_reflect_method_info_is_public(_ctx, args: [1]) {
        slot_of(args, &METHOD_INFO, METHOD_PUBLIC_SLOT, "isPublic")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\MethodInfo::parameterCount(): uint` — the declared
    /// parameter count, receiver excluded.
    fn nvs_core_reflect_method_info_parameter_count(_ctx, args: [1]) {
        slot_of(args, &METHOD_INFO, METHOD_PARAMETERS_SLOT, "parameterCount")
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
    fn nvs_core_reflect_class_info_get(_ctx, args: [4]) {
        let member = "get";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let name = text_of(&args[2], "Core\\Reflect\\ClassInfo::get")?;
        let (subject, class) = subject_of(receiver, args[1], member)?;
        let inside = site_class(args[3]).is_some_and(|site| site == class);
        #[expect(
            unsafe_code,
            reason = "the argument owns a reference to a live allocation, so it is \
                      live for this borrow; the handle is never dropped, so that \
                      reference is not released twice, and the descriptor is owned \
                      by the unit's class table, which outlives every instance of \
                      the class it describes"
        )]
        let (slot, visible) = unsafe {
            let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(subject));
            let desc = &*object.class();
            let slot = desc.field_slot(name, 0);
            (slot, slot.is_some_and(|at| desc.field_is_public(at)))
        };
        let Some(slot) = slot else {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{CLASS_INFO_NAME}::get(): `{class}` has no property named `{name}`"),
            ));
        };
        if !visible && !inside {
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

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::set(mixed $object, string $name, mixed $value): void`
    /// — [`nvs_core_reflect_class_info_get`]'s write half, and `rule:security/reflection-enforces-visibility`'s
    /// rule for the direction that changes something.
    ///
    /// The four refusals above it are `get`'s four, in `get`'s order and for
    /// `get`'s reasons — the doc comment there owns why visibility is asked
    /// last. What is *not* written here is everything past them: the store
    /// itself, the check of the incoming value against what the class declares
    /// the property to hold, and `rule:classes/property-observer-pipeline`'s observer step all belong to
    /// [`nvs_runtime::write_erased_property`], which is the same function an
    /// ordinary write through an erased receiver reaches.
    ///
    /// That is `nvs_core_reflect_class_info_call`'s decision applied to the
    /// other direction, and it is what makes the hook question answerable at
    /// all: § 2 says a reflective write runs "the `PropertyObserver` hook that
    /// ordinary code at that call site would face", and the strongest reading
    /// of *would face* is the same code facing it. A copy here would be a
    /// second pipeline to keep in step with `rule:classes/property-observer-pipeline`, and the copy is the
    /// one nothing else dispatches through.
    ///
    /// Ownership is that function's too: it retains what it stores and leaves
    /// this frame's own borrowed slots alone.
    fn nvs_core_reflect_class_info_set(ctx, args: [5]) {
        let member = "set";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let name = text_of(&args[2], "Core\\Reflect\\ClassInfo::set")?;
        let (subject, class) = subject_of(receiver, args[1], member)?;
        let inside = site_class(args[4]).is_some_and(|site| site == class);
        #[expect(
            unsafe_code,
            reason = "the argument owns a reference to a live allocation, so it is \
                      live for this borrow; the handle is never dropped, so that \
                      reference is not released twice, and the descriptor is owned \
                      by the unit's class table, which outlives every instance of \
                      the class it describes"
        )]
        let (slot, visible) = unsafe {
            let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(subject));
            let desc = &*object.class();
            let slot = desc.field_slot(name, 0);
            (slot, slot.is_some_and(|at| desc.field_is_public(at)))
        };
        if slot.is_none() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{CLASS_INFO_NAME}::set(): `{class}` has no property named `{name}`"),
            ));
        }
        if !visible && !inside {
            return Err(Fault::thrown(format!(
                "{CLASS_INFO_NAME}::set(): `{class}::{name}` is not writable from outside the \
                 class, and reflection does not lift that"
            )));
        }
        nvs_runtime::write_erased_property(ctx, args[1], name, 0, args[3])
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::call(mixed $object, string $name, array<mixed> $arguments): mixed`
    /// — `rule:security/reflection-enforces-visibility`'s rule that *acting* on a member faces the ordinary
    /// check, for the member that acts hardest.
    ///
    /// **No visibility rule is written here.** § 2 says a reflective call
    /// "fails the same way an ordinary out-of-class call would", and the
    /// strongest reading of *the same way* is the same code: past the two
    /// questions a description owes about its own subject, this hands the call
    /// to [`nvs_runtime::call_erased_method_from`], which is what an ordinary
    /// `$value->name(...)` on a `mixed` receiver reaches. That path already
    /// asks every question this one owes — is the member `public`, does the
    /// class declare it at all, is it a native `Core` member that borrows its
    /// receiver, are there enough arguments, does each argument carry the tag
    /// its parameter requires. What it cannot derive is *where the call is*,
    /// and that is the one datum handed to it: the last argument is the class
    /// this call site is inside ([`site_class`]), so a `private` method is
    /// reached from its own class's bodies and refused everywhere else, which
    /// is what an ordinary call at each of those sites does. A check
    /// re-implemented here would be a second visibility rule to keep in step
    /// with the first, and the pair would diverge in the direction that
    /// matters: the copy is the one nothing dispatches through, so a program
    /// would keep passing while the rule it states quietly stopped being the
    /// rule.
    ///
    /// So there is no `setAccessible` and nowhere to put one — the check is not
    /// this member's to relax, and the site it is made against is a constant of
    /// the call rather than anything a program hands over. Ownership is that
    /// path's too: the arguments are
    /// this frame's borrowed slots, `call_at` retains each one and the callee's
    /// own exit sweep releases them, and the [`Value`] handed back is already a
    /// fresh reference.
    fn nvs_core_reflect_class_info_call(ctx, args: [5]) {
        let member = "call";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let name = text_of(&args[2], "Core\\Reflect\\ClassInfo::call")?;
        subject_of(receiver, args[1], member)?;
        let site = site_class(args[4]);
        // Unreachable from source on `crate::arr`'s own terms: parameter 2 is
        // `array<mixed>` in `CLASS_INFO` above, so a non-container argument is
        // `E0401` at the checker. It stays because it is what makes
        // `array_ptr`'s answer safe to unwrap.
        let list = args[3].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "{CLASS_INFO_NAME}::call expected {:?}, got tag {}",
                Tag::Array,
                args[3].tag_byte()
            ))
        })?;
        // In slot order, keys ignored: a parameter list is positional, so an
        // `array` with keys is one whose keys say nothing about the call. The
        // arity the entries are then judged against is the callee's own, which
        // is `call_erased_method`'s question and not asked twice here.
        let list = crate::arr::borrowed(list);
        let mut passed = Vec::with_capacity(list.count());
        let mut slot = 0;
        while let Some(live) = list.next_slot(slot) {
            slot = live + 1;
            passed.push(list.value_at(live).expect("a live slot has a value"));
        }
        nvs_runtime::call_erased_method_from(ctx, args[1], name, &passed, site.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{
        ClassTable, Ctx, ErrorClass, MethodRow, NvsArray, NvsFn, NvsObj, NvsStr, OK, OutputSink,
        Tag, Value, call,
    };

    use super::{TYPE_KIND, kind_of};

    /// What [`vault_open`] answers with, so the assertion that the public half
    /// still runs reads as a value and not as an absence of a throw.
    const OPENED: u64 = 11;

    /// `Vault::open`'s body, as `nvs-codegen` would have compiled it: slot 0 is
    /// the receiver and there are no parameters, so the exit sweep is one
    /// release — `call_at` retained it on the way in.
    #[expect(
        unsafe_code,
        reason = "compiled code's own signature, which `call_at` calls through: \
                  one live value and the address of a live `Value` for the \
                  result, neither expressible in the type"
    )]
    unsafe extern "C" fn vault_open(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe {
            (*args).release();
            *out = Value::uint(OPENED);
        }
        OK
    }

    /// The last argument every acting member takes, for a call site inside no
    /// class — `crate::registry::CALL_SITE_MEMBERS`' zero word, which is what
    /// `nvs_ir::lower` emits for a script frame.
    const OUTSIDE: Value = Value::null();

    /// That argument for a site inside `member`'s class, as the compiler bakes
    /// it: the blob and the slot naming it, returned together because the slot
    /// is only readable while the blob it points at is alive.
    fn inside(member: &str) -> (Vec<u8>, Value) {
        let blob = nvs_runtime::source::encode(&nvs_render::Source {
            file: "app/Main.nvs".to_owned(),
            line: 1,
            member: Some(member.to_owned()),
        });
        let slot = Value::source_const(blob.as_ptr());
        (blob, slot)
    }

    /// A `Vault` declaring one `public` method and one `private` one, an
    /// instance of it, and a context anchored into the table that holds both.
    ///
    /// The table arrives through `Ctx::set_runtime_error_class` because that
    /// handle *is* this context's anchor into the compiled unit's classes —
    /// `crate::command`'s `dispatching` is the same shape and its doc comment
    /// owns why there is no second registration to make. Both rows carry the
    /// same address, which is what lets one assertion read the private method's
    /// answer: a call from inside `Vault` reaches it, and [`OPENED`] coming back
    /// is what says the check passed rather than that nothing ran.
    fn vault() -> (Ctx, Value) {
        let mut classes = ClassTable::new();
        let id = classes.define("Vault", &[] as &[&str], &[]);
        let row = |name: &str, public: bool| MethodRow {
            name: name.to_owned(),
            code: (vault_open as NvsFn) as *const u8,
            arity: 0,
            param_tags: 0,
            public,
            native: false,
        };
        classes.set_methods(id, vec![row("open", true), row("sealed", false)]);
        let classes = std::sync::Arc::new(classes);
        let desc = classes.desc(id);
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, id));
        #[expect(
            unsafe_code,
            reason = "the descriptor belongs to the table the context above now \
                      holds for its whole life, and `Vault` declares no fields, \
                      so a fresh allocation is a fully initialized instance"
        )]
        let subject = Value::object(unsafe { NvsObj::new(desc) });
        (ctx, subject)
    }

    /// `rule:security/reflection-enforces-visibility`'s headline rule, asked as an **agreement** rather than as a
    /// sentence: a reflective call to a `private` method and an ordinary
    /// out-of-class call to the same method have to fail *the same way*, so
    /// this asks both and compares the two refusals to each other. A `call`
    /// that grew a visibility check of its own would still refuse, and would
    /// still read correctly on its own line — and would fail here, which is the
    /// whole reason the question is put this way round.
    ///
    /// The ordinary call is `call_erased_method`, because an erased receiver is
    /// the one ordinary call site that is outside every class by construction,
    /// which is exactly the premise a reflective call site has. The public half
    /// is asserted in the same test so that "they agree" cannot be satisfied by
    /// a member that refuses everything.
    #[test]
    fn a_reflective_call_to_a_private_method_from_outside_fails_like_the_ordinary_call() {
        let (mut ctx, subject) = vault();
        let info = call(super::nvs_core_reflect_for_object, &mut ctx, &[subject])
            .expect("every object has a description");

        let ordinary = nvs_runtime::call_erased_method(&mut ctx, subject, "sealed", &[])
            .expect_err("`sealed` is not public, and this site is outside every class");
        let nvs_runtime::Fault::Thrown(ordinary_class, ordinary_said) = ordinary else {
            panic!("an out-of-class call to a `private` method is a catchable throw");
        };

        let sealed = Value::str(NvsStr::new(b"sealed"));
        let none = Value::array(NvsArray::new());
        assert_eq!(
            call(
                super::nvs_core_reflect_class_info_call,
                &mut ctx,
                &[info, subject, sealed, none, OUTSIDE],
            )
            .err(),
            Some(nvs_runtime::THROWN),
            "reflection does not lift the check, so the reflective call throws too"
        );
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some(ordinary_said.as_ref()),
            "the same sentence, because it is the same check: `call` dispatches \
             through the ordinary path rather than restating its rule"
        );
        assert_eq!(
            ordinary_class,
            nvs_runtime::ThrownClass::Logic,
            "and the same class, which is what a `catch` in a program sees"
        );

        let open = Value::str(NvsStr::new(b"open"));
        let none = Value::array(NvsArray::new());
        assert_eq!(
            call(
                super::nvs_core_reflect_class_info_call,
                &mut ctx,
                &[info, subject, open, none, OUTSIDE],
            )
            .expect("`open` is public")
            .as_uint(),
            Some(OPENED),
            "the public half runs and answers, so the agreement above is not \
             two members refusing everything"
        );

        // And the other side of the same rule: the identical call from inside
        // `Vault` reaches `sealed`, because that is what an ordinary call
        // written there does. Nothing about the call changes but the constant
        // the compiler supplies, which is the whole of what a call site is.
        let none = Value::array(NvsArray::new());
        let site = inside("Vault::open");
        assert_eq!(
            call(
                super::nvs_core_reflect_class_info_call,
                &mut ctx,
                &[info, subject, sealed, none, site.1],
            )
            .expect("`sealed` is reachable from `Vault`'s own bodies")
            .as_uint(),
            Some(OPENED),
            "a private method is refused for the site, not for the door"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each — `forObject` handed \
                      back a fresh one and `vault` built the other — and every \
                      call above borrowed rather than consumed them"
        )]
        unsafe {
            info.release();
            subject.release();
        }
    }

    // Every `onPropertySet` call `ledger_observed` has been handed, in arrival
    // order, as the two things `rule:classes/property-observer-pipeline` says it is told: the property's
    // name and the value that was committed.
    thread_local! {
        static OBSERVED: std::cell::RefCell<Vec<(String, u64)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }

    /// `Ledger::onPropertySet`'s body, as `nvs-codegen` would have compiled it:
    /// slot 0 is the receiver and slots 1 and 2 are `(string $name, mixed
    /// $value)`, so the exit sweep is three releases — `call_at` retained every
    /// one of them on the way in.
    #[expect(
        unsafe_code,
        reason = "compiled code's own signature, which `call_at` calls through: \
                  three live values and the address of a live `Value` for the \
                  result, neither expressible in the type"
    )]
    unsafe extern "C" fn ledger_observed(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        unsafe {
            let name = (*args.add(1))
                .as_text()
                .expect("`onPropertySet`'s first parameter is a `string`")
                .to_owned();
            let value = (*args.add(2))
                .as_uint()
                .expect("`Ledger::n` is declared `uint`, so that is what was committed");
            OBSERVED.with_borrow_mut(|seen| seen.push((name, value)));
            for slot in 0..3 {
                (*args.add(slot)).release();
            }
            *out = Value::null();
        }
        OK
    }

    /// A `Ledger` with one `public uint $n`, an instance of it, and a context
    /// anchored into the table — [`vault`]'s shape, plus the two things ADR
    /// 0014 § 4 answers from a declaration: whether the class conforms to
    /// `PropertyObserver` at all, and the body it answers `onPropertySet` with.
    ///
    /// `observes` is a parameter rather than two fixtures because § 4's "not
    /// one line from a class that implements nothing" is asserted over the same
    /// writes as the observed half, and a second fixture would let the two
    /// drift into being different writes.
    fn ledger(observes: bool) -> (Ctx, Value) {
        let mut classes = ClassTable::new();
        let interface = classes.define("PropertyObserver", &[] as &[&str], &[]);
        let conforms: &[_] = if observes { &[interface] } else { &[] };
        let id = classes.define("Ledger", &["n"], conforms);
        classes.set_public_fields(id, vec![true]);
        classes.set_field_tags(id, vec![Some(Tag::Uint)]);
        if observes {
            classes.set_methods(
                id,
                vec![MethodRow {
                    name: "onPropertySet".to_owned(),
                    code: (ledger_observed as NvsFn) as *const u8,
                    arity: 2,
                    // Both nibbles are `CLOSURE_PARAM_TAG_ANY`: the callee
                    // above reads its own two arguments and says what it
                    // expected, so a tag rule written here would be a second
                    // opinion about a signature this test declares.
                    param_tags: 0xff,
                    public: true,
                    native: false,
                }],
            );
        }
        let classes = std::sync::Arc::new(classes);
        let desc = classes.desc(id);
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, id));
        #[expect(
            unsafe_code,
            reason = "the descriptor belongs to the table the context above now \
                      holds for its whole life, and `NvsObj::new` writes every \
                      slot before it hands the allocation back"
        )]
        let subject = Value::object(unsafe { NvsObj::new(desc) });
        (ctx, subject)
    }

    /// `$ledger->n = $value;` through an erased receiver — the door compiled
    /// code reaches, called exactly as `nvs-codegen` emits it.
    #[expect(
        unsafe_code,
        reason = "`nvs_object_slot_set` is compiled code's own entry point: a \
                  context, three values by address and a static byte range, \
                  none of which its signature can bound"
    )]
    fn ordinary_write(ctx: &mut Ctx, subject: Value, name: &str, value: Value) -> i32 {
        let mut out = Value::null();
        unsafe {
            nvs_runtime::nvs_object_slot_set(
                std::ptr::from_mut(ctx),
                std::ptr::from_ref(&subject),
                name.as_ptr(),
                name.len(),
                0,
                std::ptr::from_ref(&value),
                std::ptr::from_mut(&mut out),
            )
        }
    }

    /// `rule:security/reflection-enforces-visibility`'s rule for the direction that changes something, asked as
    /// an **agreement** for the reason its sibling above is: a reflective write
    /// and an ordinary one have to reach the same `PropertyObserver`, so this
    /// puts the same value through both doors and compares what the observer
    /// was told. A `set` that stored the slot itself would still store the
    /// right value and would still read correctly on its own line — and would
    /// record nothing here, which is the whole reason the question is put this
    /// way round.
    ///
    /// The ordinary write is [`ordinary_write`], because `rule:types/erased-member-access`'s erased
    /// store is the one ordinary write whose class is unknown until it runs,
    /// which is exactly the premise a reflective write site has. `rule:classes/property-observer-costs-nothing-when-unused`'s
    /// other half is asserted in the same test, so that "they agree" cannot be
    /// satisfied by two doors that observe nothing.
    #[test]
    fn a_reflective_property_write_runs_the_hook_an_ordinary_write_runs() {
        OBSERVED.with_borrow_mut(Vec::clear);
        let (mut ctx, subject) = ledger(true);
        let info = call(super::nvs_core_reflect_for_object, &mut ctx, &[subject])
            .expect("every object has a description");

        assert_eq!(ordinary_write(&mut ctx, subject, "n", Value::uint(7)), OK);

        let name = Value::str(NvsStr::new(b"n"));
        call(
            super::nvs_core_reflect_class_info_set,
            &mut ctx,
            &[info, subject, name, Value::uint(7), OUTSIDE],
        )
        .expect("`n` is public, and 7 is the `uint` it declares");

        let seen = OBSERVED.with_borrow(Clone::clone);
        assert_eq!(
            seen.len(),
            2,
            "one observation per write, from the two doors a write can arrive through"
        );
        assert_eq!(
            seen[0], seen[1],
            "the same name and the same committed value, because it is the same \
             pipeline: `set` dispatches through the erased write rather than \
             restating `rule:classes/property-observer-pipeline`"
        );
        assert_eq!(
            seen[0],
            ("n".to_owned(), 7),
            "and it is the write that actually happened, so the agreement above \
             is not two doors observing nothing"
        );

        // The value is what a following read answers, which is what makes the
        // observation above a report of a store rather than a report instead
        // of one.
        let read = call(
            super::nvs_core_reflect_class_info_get,
            &mut ctx,
            &[info, subject, name, OUTSIDE],
        )
        .expect("`n` is public");
        assert_eq!(read.as_uint(), Some(7));

        // `rule:classes/property-observer-costs-nothing-when-unused`: a class that implements nothing pays nothing, through
        // either door.
        let (mut plain_ctx, plain) = ledger(false);
        let plain_info = call(super::nvs_core_reflect_for_object, &mut plain_ctx, &[plain])
            .expect("every object has a description");
        assert_eq!(
            ordinary_write(&mut plain_ctx, plain, "n", Value::uint(9)),
            OK
        );
        call(
            super::nvs_core_reflect_class_info_set,
            &mut plain_ctx,
            &[plain_info, plain, name, Value::uint(9), OUTSIDE],
        )
        .expect("`n` is public on this one too");
        assert_eq!(
            OBSERVED.with_borrow(Vec::len),
            2,
            "not one line from a class that implements nothing, and no door is \
             an exception to that"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each — `forObject` handed \
                      two back, `ledger` built the two subjects and this frame \
                      made the name — and every call above borrowed rather than \
                      consumed them"
        )]
        unsafe {
            read.release();
            name.release();
            info.release();
            subject.release();
            plain_info.release();
            plain.release();
        }
    }

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
