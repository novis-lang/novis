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
//! [`CLASS_INFO`]'s slots are the class's name and its three rosters, all
//! computed at [`nvs_core_reflect_for_object`] time. A slot holding
//! the descriptor itself would be smaller and would defer the walk — but
//! [`crate::instance`]'s first decision is that a `Core` instance is an ordinary
//! Novis object, so every slot must be a value Novis already holds, and a raw
//! descriptor pointer is neither that nor something a hot-reload swap
//! (`rule:config/an-edit-reaches-the-next-request-without-a-restart`) leaves
//! valid. Holding the answers instead makes the description exactly as inert as
//! § 1 says it is: nothing it carries can be dereferenced back into the program.
//!
//! **What it spends:** per `forClass` or `forObject` call, one array and one
//! [`PROPERTY_INFO`] of three slots per declared property, one array and one
//! [`METHOD_INFO`] of three slots per declared method, one array and one
//! [`CONSTANT_INFO`] of three slots per class constant, and the attach-site
//! roster the attribute decision below prices, charged to the
//! request that asked and released with the description. A program that
//! describes the same class in a loop pays per call; the alternative is a
//! per-core cache keyed by descriptor address, which nothing yet needs. Both
//! rosters are built eagerly rather than on the first `properties` or `methods`
//! call because the alternative is a slot holding the descriptor, which the
//! decision above rules out for every slot alike.
//! [`nvs_core_reflect_class_info_readable_properties`] spends one array of one
//! string per name it hands back, per call, because what it answers is a
//! property of the asking site rather than of the description. An acting call
//! spends one decoded [`nvs_render::Source`] on top, which is two short strings
//! read out of the unit's own data section.
//!
//! # Decision: every roster is complete, and the readable walk is a member of
//! its own
//!
//! [`CLASS_INFO`]'s `properties`, its `methods` and its `constants` answer the
//! same shape: every member the class declares, each row carrying the bit
//! saying whether this call site may act on it. `readableProperties` is the other question — the
//! list a `get` is about to be made against — and it has a spelling of its own
//! because it has an answer of its own, which depends on where the call is
//! written. § 2 is what divides them: *reading metadata* is always available,
//! *acting on a member* faces the ordinary check, and a member named for one of
//! those must not quietly answer the other. That division is `docs/spec/01-core-library.md`
//! § 13's own: `property_exists` and `get_class_methods` are complete in PHP
//! and `get_object_vars` is scope-sensitive, and this class replaces all three.
//!
//! `rule:core-classes/reflect` is what forces both rosters to be complete: a
//! refusal has to be distinguishable from a misspelling, and a roster that
//! dropped the `private` members would make [`nvs_core_reflect_class_info_has_property`]
//! and its `hasMethod` twin answer `false` to a declared name and to a typo
//! alike. Nothing leaks by it — a name, a visibility bit, a declared type and a
//! parameter count are what the declaration already published to the checker,
//! and no state of any instance is reachable through them. A constant's roster
//! row carries no value for the same reading one step further: the value *is*
//! the declaration, so it is the one thing on a row that would be state, and it
//! is [`nvs_core_reflect_class_info_constant`]'s to hand back. Acting is still
//! that member's and [`nvs_core_reflect_class_info_get`]'s and
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
//! not what `is` answers — a subclass has its own description, one call away, and
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
//! That is what makes [`nvs_core_reflect_class_info_readable_properties`]
//! answer two ways off one description. A readable walk is the list a `get` is
//! about to be made against, so it names what *this* site may read: every
//! declared property from inside the class, the `public` ones from anywhere
//! else, filtered out of the roster [`describe`] built. The rosters are the
//! other half of the same rule and do not move: naming a member is metadata,
//! which § 2 makes always available, and the bit each row carries is what a
//! caller reads instead.
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
//! # Decision: an enum is described by name, because there is nothing else to
//! describe it from
//!
//! [`ENUM_INFO`] is the one description with no `forObject` twin and no
//! descriptor behind it. `rule:enums/representation` makes a case *be* the
//! integer behind it at run time, so no value carries which enum it came from
//! and [`nvs_core_reflect_type_of`] answers `Int` for one; what the runtime
//! carries instead is the shape itself, as a [`nvs_runtime::EnumDesc`] per
//! declared enum, filled by `nvs-codegen` off `nvs_ir::ir::Program::enums`.
//! `Core` enums are in that roster beside the program's own, because the
//! checker's table is seeded with them and a second door for them would be a
//! second thing to keep in step.
//!
//! **What it spends:** one `String` per case per declared enum, once per
//! process and never per instance — an enum has none. Per `of` call, two
//! arrays and one string per case, charged to the request that asked and
//! released with the description, on [`describe`]'s terms exactly.
//!
//! `valueOf` answers a union, `int|uint`, and that is the one place this class
//! costs a caller something: the backing is a property of the enum the *name*
//! named, so a description reached by name cannot promise one of the two, and a
//! caller that knows which it asked for writes `as int`. The alternative —
//! answering `int` always — reports a `uint` case above `i64::MAX` as a wrapped
//! negative, which is `rule:programs/memory-priority`'s second priority spent
//! to save a conversion.
//!
//! # Decision: an attach site is described where it was written, and the
//! payload is the constant set
//!
//! [`ATTRIBUTE_INFO`] is the roster [`describe`] builds from
//! [`nvs_runtime::ClassDesc::attributes`], and it is **own-only** where the
//! constants beside it are flattened over the ancestors: a constant is a name a
//! program may write on this class, which `Foo::BAR` resolves up the chain,
//! while an attribute is a fact about the declaration it is written on.
//! `nvs_types::layout::ClassAttribute` owns that reading, and the row keeps the
//! declaration it names — the empty string for the class itself, a member's
//! name, and a parameter's beside its method's — rather than folding the four
//! of `rule:attributes/structural-retrieval`'s target spellings into one
//! invented one.
//!
//! **This is not `Core\Attributes`, and the two never meet.** That retrieval is
//! *structural*: it matches an attached literal against a shape type and is
//! replaced by its answer at compile time, so a running program holds no table
//! for it at all (`nvs_types::retrieval`). This is the reflective question
//! instead — *what is attached to this class, which the checker never saw* —
//! and it is answered the only way a description can be, off the descriptor.
//! A program that knows the shape it wants should ask the first: it costs
//! nothing at run time and is type-checked where it is written.
//!
//! A payload field's value is [`nvs_runtime::ConstantValue`], the same currency
//! a class constant travels in, because
//! `rule:attributes/payload-is-a-compile-time-constant` admits the literals
//! `nvs_types::consts` already folds. The three spellings it admits that need
//! the attach site's own namespace to resolve — a class constant, an enum case
//! and `Foo::class` — read as *no folded value* here, which is
//! `Core\Reflect\ConstantInfo::hasValue`'s stated bound one declaration over,
//! and [`nvs_core_reflect_attribute_info_field`] reports it as the bound it is.
//! No `secret` bit rides with the payload, where a constant needs one: a
//! `secret` value is refused where the payload is written (`E0727`), so none is
//! ever in one to hand back.
//!
//! **What it spends:** one [`nvs_runtime::AttributeDesc`] per attach site per
//! class and one `ConstantValue` per payload field, once per process; per
//! `forClass` or `forObject` call, one array and one [`ATTRIBUTE_INFO`] of five
//! slots per attach site, plus two arrays and one string per payload field,
//! charged to the request that asked and released with the description.

use nvs_runtime::{ClassDesc, Fault, NvsArray, NvsObj, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, ClassDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc,
    ParamDoc, Qual,
};

/// The class name, as a program writes it.
pub(crate) const NAME: &str = "Core\\Reflect";

/// The described class's own name, as a program writes it.
pub(crate) const CLASS_INFO_NAME: &str = "Core\\Reflect\\ClassInfo";

/// One described method's own name, as a program writes it.
pub(crate) const METHOD_INFO_NAME: &str = "Core\\Reflect\\MethodInfo";

/// One described property's own name, as a program writes it.
pub(crate) const PROPERTY_INFO_NAME: &str = "Core\\Reflect\\PropertyInfo";

/// One described parameter's own name, as a program writes it.
pub(crate) const PARAMETER_INFO_NAME: &str = "Core\\Reflect\\ParameterInfo";

/// One described class constant's own name, as a program writes it.
pub(crate) const CONSTANT_INFO_NAME: &str = "Core\\Reflect\\ConstantInfo";

/// The described enum's own name, as a program writes it.
pub(crate) const ENUM_INFO_NAME: &str = "Core\\Reflect\\EnumInfo";

/// One described attach site's own name, as a program writes it.
pub(crate) const ATTRIBUTE_INFO_NAME: &str = "Core\\Reflect\\AttributeInfo";

/// [`CLASS_INFO`]'s slot holding the described class's name.
const NAME_SLOT: usize = 0;

/// [`CLASS_INFO`]'s slot holding one [`PROPERTY_INFO`] per declared property.
const PROPERTIES_SLOT: usize = 1;

/// [`CLASS_INFO`]'s slot holding one [`METHOD_INFO`] per declared method.
const METHODS_SLOT: usize = 2;

/// [`CLASS_INFO`]'s slot holding one [`CONSTANT_INFO`] per class constant.
const CONSTANTS_SLOT: usize = 3;

/// [`CLASS_INFO`]'s slot holding one [`ATTRIBUTE_INFO`] per attach site the
/// class's own declaration carries.
const ATTRIBUTES_SLOT: usize = 4;

/// [`ATTRIBUTE_INFO`]'s slot holding the name the named form gave the
/// attribute, or the empty string for the bare form.
const ATTRIBUTE_NAME_SLOT: usize = 0;

/// [`ATTRIBUTE_INFO`]'s slot holding the member the attribute is written on, or
/// the empty string for the class declaration itself.
const ATTRIBUTE_TARGET_SLOT: usize = 1;

/// [`ATTRIBUTE_INFO`]'s slot holding the parameter the attribute is written on,
/// or the empty string for every other attach site.
const ATTRIBUTE_PARAMETER_SLOT: usize = 2;

/// [`ATTRIBUTE_INFO`]'s slot holding the payload's field names, in source
/// order.
const ATTRIBUTE_FIELDS_SLOT: usize = 3;

/// [`ATTRIBUTE_INFO`]'s slot holding one folded value per name in
/// [`ATTRIBUTE_FIELDS_SLOT`], in that slot's own order — `null` where the
/// declaration's value is one Novis does not fold here.
const ATTRIBUTE_VALUES_SLOT: usize = 4;

/// [`PROPERTY_INFO`]'s slot holding the property's name.
const PROPERTY_NAME_SLOT: usize = 0;

/// [`PROPERTY_INFO`]'s slot holding whether the property is `public`.
const PROPERTY_PUBLIC_SLOT: usize = 1;

/// [`PROPERTY_INFO`]'s slot holding the type the declaration spells, or `null`
/// where it named none.
const PROPERTY_TYPE_SLOT: usize = 2;

/// [`CONSTANT_INFO`]'s slot holding the constant's name.
const CONSTANT_NAME_SLOT: usize = 0;

/// [`CONSTANT_INFO`]'s slot holding whether the constant is `public`.
const CONSTANT_PUBLIC_SLOT: usize = 1;

/// [`CONSTANT_INFO`]'s slot holding whether the constant folded to a value at
/// all — see [`CONSTANT_HAS_VALUE_DOC`].
const CONSTANT_HAS_VALUE_SLOT: usize = 2;

/// [`METHOD_INFO`]'s slot holding the method's name.
const METHOD_NAME_SLOT: usize = 0;

/// [`METHOD_INFO`]'s slot holding whether the method is `public`.
const METHOD_PUBLIC_SLOT: usize = 1;

/// [`METHOD_INFO`]'s slot holding how many parameters the method declares.
const METHOD_PARAMETER_COUNT_SLOT: usize = 2;

/// [`METHOD_INFO`]'s slot holding one [`PARAMETER_INFO`] per parameter a
/// declaration spelled — empty where none did, which is not the same answer as
/// [`METHOD_PARAMETER_COUNT_SLOT`]'s zero.
const METHOD_PARAMETERS_SLOT: usize = 3;

/// [`PARAMETER_INFO`]'s slot holding the parameter's name.
const PARAMETER_NAME_SLOT: usize = 0;

/// [`PARAMETER_INFO`]'s slot holding the type the declaration spells, or `null`
/// where it named none — [`PROPERTY_TYPE_SLOT`]'s answer for the other half of
/// a declaration.
const PARAMETER_TYPE_SLOT: usize = 1;

/// [`ENUM_INFO`]'s slot holding the described enum's name.
const ENUM_NAME_SLOT: usize = 0;

/// [`ENUM_INFO`]'s slot holding one string per declared case, ascending by the
/// case's constant.
const ENUM_CASES_SLOT: usize = 1;

/// [`ENUM_INFO`]'s slot holding each case's constant, in
/// [`ENUM_CASES_SLOT`]'s order and one entry for one.
///
/// A second roster rather than a name-keyed array, because the two answers have
/// different types: the case list is `array<string>` and a constant is `int` or
/// `uint` by [`ENUM_UNSIGNED_SLOT`]'s bit, and a single array would have to
/// admit both in one element type to carry either.
const ENUM_VALUES_SLOT: usize = 2;

/// [`ENUM_INFO`]'s slot holding whether the backing type is `uint`.
const ENUM_UNSIGNED_SLOT: usize = 3;

/// `Core\Reflect` — the door onto a description, and nothing that acts.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "forClass",
            names: &["name"],
            params: &[CoreTy::ClassName(Qual::Neutral)],
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

/// `Core\Reflect`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Describes a value or a class while the program runs. `typeOf` returns what kind of \
            value you have. `forClass` and `forObject` return a `Core\\Reflect\\ClassInfo` with \
            the class's properties, methods, constants and attributes.",
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
    doc: Some(&CLASS_INFO_CARD),
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
            return_ty: CoreTy::Array(&CoreTy::Instance(PROPERTY_INFO_NAME)),
            symbol: "nvs_core_reflect_class_info_properties",
            doc: Some(&PROPERTIES_DOC),
        },
        CoreMethod {
            name: "readableProperties",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_reflect_class_info_readable_properties",
            doc: Some(&READABLE_PROPERTIES_DOC),
        },
        CoreMethod {
            name: "hasProperty",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_reflect_class_info_has_property",
            doc: Some(&HAS_PROPERTY_DOC),
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
            name: "constants",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(CONSTANT_INFO_NAME)),
            symbol: "nvs_core_reflect_class_info_constants",
            doc: Some(&CONSTANTS_DOC),
        },
        CoreMethod {
            name: "constant",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            // `mixed`, on `get`'s terms one member down: which of the four
            // literal types a constant folded to is not known where the call
            // is written, the class being named at run time.
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_reflect_class_info_constant",
            doc: Some(&CONSTANT_DOC),
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
            name: "attributes",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(ATTRIBUTE_INFO_NAME)),
            symbol: "nvs_core_reflect_class_info_attributes",
            doc: Some(&ATTRIBUTES_DOC),
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
        CoreMethod {
            name: "construct",
            names: &["arguments"],
            params: &[CoreTy::Array(&CoreTy::Mixed)],
            // Required rather than defaulted to `[]` for the row above's
            // reason: `registry::Const` has no array variant.
            defaults: &[],
            // `mixed`, and not `CoreTy::Instance` of anything: the class being
            // built is named by the description at run time, so what comes back
            // has no type at the site the call is written.
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_reflect_class_info_construct",
            doc: Some(&CONSTRUCT_DOC),
        },
    ],
    slots: &["name", "properties", "methods", "constants", "attributes"],
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

/// `Core\Reflect\ClassInfo::constants`'s reference card — `rule:core-api/reference-card`.
const CONSTANTS_DOC: MethodDoc = MethodDoc {
    short: "The described class's constants — its own and every inherited one — each with its name \
            and its visibility. Replaces `ReflectionClass::getReflectionConstants`.",
    params: &[],
    ret: "One `Core\\Reflect\\ConstantInfo` per constant the class answers, its own declarations \
          first. A constant a subclass redeclares appears once, with the declaration that wins. \
          The *values* are not here: naming a constant is metadata, reading one is acting, and \
          `constant` is where that check is made.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::constant`'s reference card — `rule:core-api/reference-card`.
const CONSTANT_DOC: MethodDoc = MethodDoc {
    short: "The value of the class constant `$name`, under exactly the visibility ordinary code at \
            this call site would face. Replaces `ReflectionClassConstant::getValue`, and there is \
            no `setAccessible` to lift the check with.",
    params: &[ParamDoc {
        name: "name",
        desc: "The constant's name, as the declaration writes it — no class qualifier and no \
               `::`.",
        shape: &[],
    }],
    ret: "The folded value: a `string`, `int`, `bool` or `float`, whichever the declaration's \
          right-hand side is.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$name` names a constant this call site may not reach — a reflective read has \
                   the visibility ordinary code has; or one whose declared type carries `secret`, \
                   which is refused at every site, because the `mixed` this answers with carries \
                   no qualifier and the value would reach the next sink unmarked.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "The class declares and inherits no constant of that name; or the declaration's \
                   value is not one of the four literals Novis folds, which `hasValue` reports \
                   ahead of the call. Both are mistakes in the program rather than privilege \
                   questions, which is what separates them from the refusals above.",
        },
    ],
};

/// `Core\Reflect\ClassInfo::properties`'s reference card — `rule:core-api/reference-card`.
const PROPERTIES_DOC: MethodDoc = MethodDoc {
    short: "The described class's properties — its own and every inherited one — each with its \
            name, its visibility and the type its declaration spells. Replaces \
            `ReflectionClass::getProperties`.",
    params: &[],
    ret: "One `Core\\Reflect\\PropertyInfo` per declared property, in slot order: every \
          ancestor's first, then the class's own. A `private` or `protected` property is among \
          them, carrying the bit that says so — naming a member is introspection of the \
          program's shape, and reading one is `get`'s question, checked there.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::readableProperties`'s reference card — `rule:core-api/reference-card`.
const READABLE_PROPERTIES_DOC: MethodDoc = MethodDoc {
    short: "The property names a read written at this call site may make, in slot order. Replaces \
            `get_object_vars`, whose answer is scope-sensitive in the same way.",
    params: &[],
    ret: "One name per property this site may read, `$`-sigil excluded: every declared property \
          where the call is written inside the described class, and the `public` ones anywhere \
          else. The complete list is `properties`, and the difference between the two answers is \
          exactly what `get` would refuse here.",
    errors: &[],
};

/// `Core\Reflect\ClassInfo::hasProperty`'s reference card — `rule:core-api/reference-card`.
const HAS_PROPERTY_DOC: MethodDoc = MethodDoc {
    short: "Whether the described class declares a property called `$name`. Replaces \
            `property_exists`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The property's name, `$`-sigil excluded, as the declaration writes it.",
        shape: &[],
    }],
    ret: "`true` for a `private` or `protected` property as well as a `public` one, which is what \
          tells a refused read from a misspelled name. Whether this site may read it is \
          `properties`' own bit.",
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
///
/// The class's name is compared in place and copied out only by
/// [`class_name_of`], on a refusal's path: a read that succeeds allocates
/// nothing for a sentence it never prints.
fn subject_of(
    receiver: *mut nvs_runtime::ObjHeader,
    subject: Value,
    member: &str,
) -> Result<*mut nvs_runtime::ObjHeader, Fault> {
    let ptr = subject.obj_ptr().ok_or_else(|| {
        Fault::thrown(format!(
            "{CLASS_INFO_NAME}::{member}() expected an object, got tag {}",
            subject.tag_byte()
        ))
    })?;
    // `as_text` rather than the bytes: the slot was written by `forObject`
    // from a `Tag::Str`, and `rule:types/conversion` makes that tag the UTF-8 guarantee —
    // see `crate::str`'s `text` on why re-deriving it costs an O(n) pass for
    // nothing.
    let described = crate::instance::slot(receiver, NAME_SLOT);
    let described = described.as_text().unwrap_or_default();
    #[expect(
        unsafe_code,
        reason = "the argument owns a reference to a live allocation, so it is \
                  live for this borrow; the handle is never dropped, so that \
                  reference is not released twice, and the descriptor is owned \
                  by the unit's class table, which outlives every instance of \
                  the class it describes"
    )]
    let same = unsafe {
        let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(ptr));
        (*object.class()).name() == described
    };
    if !same {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{CLASS_INFO_NAME}::{member}(): this describes `{described}`, and the value is a \
                 `{}`",
                class_name_of(ptr)
            ),
        ));
    }
    Ok(ptr)
}

/// The name of the class of the object at `ptr`, copied out for a refusal's
/// sentence — [`subject_of`] has already checked that `ptr` is a live object.
fn class_name_of(ptr: *mut nvs_runtime::ObjHeader) -> String {
    #[expect(
        unsafe_code,
        reason = "the caller's argument owns a reference to this live allocation; \
                  the handle is never dropped, so that reference is not released \
                  twice, and the descriptor outlives every instance it describes"
    )]
    unsafe {
        let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(ptr));
        (*object.class()).name().to_owned()
    }
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
/// the direction `docs/agent/goals/m8-stdlib-depth.md` § *Standing decisions* fixes.
///
/// The name is borrowed from the carrier rather than copied, so asking costs no
/// allocation: every reflective read, write and call asks once.
fn site_class(operand: Value) -> Option<&'static str> {
    #[expect(
        unsafe_code,
        reason = "the carrier came out of a `SourceConst` the compiled unit baked into its own data section, which outlives every request served from it"
    )]
    let member = unsafe { nvs_runtime::source::member_of_operand(operand) }?;
    let (class, _) = member.split_once("::")?;
    Some(class)
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
            desc: "One entry per declared parameter, in order, keys ignored. Entries after the \
                   last parameter are ignored. Required even where the method takes none, which \
                   is then `[]`.",
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

/// `Core\Reflect\ClassInfo::construct`'s reference card — `rule:core-api/reference-card`.
const CONSTRUCT_DOC: MethodDoc = MethodDoc {
    short: "Builds an instance of the described class, running its constructor with `$arguments` \
            under exactly the visibility a `new` written at this call site would face. Replaces \
            `ReflectionClass::newInstanceArgs`, and there is no `setAccessible` to lift the check \
            with.",
    params: &[ParamDoc {
        name: "arguments",
        desc: "One entry per declared constructor parameter, in order, keys ignored. Entries \
               after the last parameter are ignored. Required even where the class declares no \
               constructor, which is then `[]`.",
        shape: &[],
    }],
    ret: "The new instance, with its own class erased to `mixed`. A class declaring no \
          constructor answers with the allocation its declared defaults armed.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The constructor is not `public` and this call site is outside the class; or \
               `$arguments` has fewer entries than it declares, or an entry whose type a \
               parameter does not accept; or the described class is not one this program \
               declares. Each is the refusal the ordinary door meets, raised by that same check.",
    }],
};

/// `Core\Reflect\MethodInfo` — one row of [`CLASS_INFO`]'s roster: a method's
/// name, its visibility and how many parameters it declares.
///
/// Every member is a reader over a slot the description was built with, for
/// [`CLASS_INFO`]'s own reason: what it carries is answers, not a way back into
/// the class it came from. Nothing here acts — the doors onto an invocation are
/// [`nvs_core_reflect_class_info_call`] and
/// [`nvs_core_reflect_class_info_construct`], and each is where § 2's check is
/// made.
pub(crate) const METHOD_INFO: CoreClass = CoreClass {
    name: METHOD_INFO_NAME,
    doc: Some(&METHOD_INFO_CARD),
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
        CoreMethod {
            name: "parameters",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(PARAMETER_INFO_NAME)),
            symbol: "nvs_core_reflect_method_info_parameters",
            doc: Some(&METHOD_PARAMETERS_DOC),
        },
    ],
    slots: &["name", "public", "parameterCount", "parameters"],
    constants: &[],
};

/// `Core\Reflect\MethodInfo`'s class card — `rule:core-api/reference-card`.
const METHOD_INFO_CARD: ClassDoc = ClassDoc {
    short: "One method of a class, as `Core\\Reflect\\ClassInfo::methods` returns it. `name` and \
            `isPublic` describe the declaration. `parameterCount` and `parameters` describe what \
            the method takes.",
};

/// `Core\Reflect\MethodInfo::name`'s reference card — `rule:core-api/reference-card`.
const METHOD_NAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the method.",
    params: &[],
    ret: "The name as the class declares it, with no class name and no parentheses. A \
          constructor is named `constructor`. You can pass the name to \
          `Core\\Reflect\\ClassInfo::hasMethod` and `Core\\Reflect\\ClassInfo::call`.",
    errors: &[],
};

/// `Core\Reflect\MethodInfo::isPublic`'s reference card — `rule:core-api/reference-card`.
const IS_PUBLIC_DOC: MethodDoc = MethodDoc {
    short: "Tells you whether code outside the class may call the method.",
    params: &[],
    ret: "`true` for a `public` method, and `false` for a `protected` or `private` method. Those \
          methods are still in the list that `Core\\Reflect\\ClassInfo::methods` returns. If \
          `isPublic` returns `false`, `Core\\Reflect\\ClassInfo::call` from outside the class \
          throws a `LogicError`.",
    errors: &[],
};

/// `Core\Reflect\MethodInfo::parameterCount`'s reference card — `rule:core-api/reference-card`.
const PARAMETER_COUNT_DOC: MethodDoc = MethodDoc {
    short: "Returns how many parameters the method declares.",
    params: &[],
    ret: "The number of parameters, as a `uint`. `$this` is not counted. The number is also \
          correct for a method the compiler adds, where `parameters` returns an empty array.",
    errors: &[],
};

/// `Core\Reflect\MethodInfo::parameters`'s reference card — `rule:core-api/reference-card`.
const METHOD_PARAMETERS_DOC: MethodDoc = MethodDoc {
    short: "Returns the parameters of the method, in the order they are written. It replaces \
            PHP's `ReflectionMethod::getParameters`.",
    params: &[],
    ret: "One `Core\\Reflect\\ParameterInfo` for each parameter. `$this` is not included. For a \
          method the compiler adds, such as the constructor of `LogicError`, the array is empty. \
          `parameterCount` still returns how many arguments that method takes.",
    errors: &[],
};

/// `Core\Reflect\ParameterInfo` — one row of [`METHOD_INFO`]'s roster: the name
/// a parameter is declared under, and the type it is declared at.
///
/// A class rather than the bare `array<string>` of names, on [`METHOD_INFO`]'s
/// own terms: the roster ADR 0019 § 1 names is a family of descriptions, and a
/// description is what the next question hangs off. The type is that next
/// question, and it travelled the road this class was shaped around — down
/// `nvs_types::layout::ClassLayout::methods` to
/// [`nvs_runtime::MethodRow::param_types`] — so it is a member here rather than
/// a second roster beside this one, which is the same pair
/// [`PROPERTY_INFO`] answers for a declared property.
pub(crate) const PARAMETER_INFO: CoreClass = CoreClass {
    name: PARAMETER_INFO_NAME,
    doc: Some(&PARAMETER_INFO_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_parameter_info_name",
            doc: Some(&PARAMETER_NAME_DOC),
        },
        CoreMethod {
            name: "type",
            names: &[],
            params: &[],
            defaults: &[],
            // `?string` on [`PROPERTY_INFO`]'s terms: a parameter the front end
            // admitted wrote a type, so the `null` here is the row whose
            // *declaration* was never read rather than a parameter that
            // declined to name one.
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_reflect_parameter_info_type",
            doc: Some(&PARAMETER_TYPE_DOC),
        },
    ],
    slots: &["name", "type"],
    constants: &[],
};

/// `Core\Reflect\ParameterInfo`'s class card — `rule:core-api/reference-card`.
const PARAMETER_INFO_CARD: ClassDoc = ClassDoc {
    short: "One parameter of a method, as `Core\\Reflect\\MethodInfo::parameters` returns it. \
            `name` returns the name of the parameter, and `type` returns its declared type.",
};

/// `Core\Reflect\ParameterInfo::name`'s reference card — `rule:core-api/reference-card`.
const PARAMETER_NAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the parameter.",
    params: &[],
    ret: "The name as the method declares it, without the `$`. This is the name you write for a \
          named argument. A promoted constructor parameter has the same name as its property.",
    errors: &[],
};

/// `Core\Reflect\ParameterInfo::type`'s reference card — `rule:core-api/reference-card`.
const PARAMETER_TYPE_DOC: MethodDoc = MethodDoc {
    short: "Returns the declared type of the parameter, written the way the method writes it.",
    params: &[],
    ret: "The type as text, such as `int`, `?int`, `array<string>` or `App\\User`. A parameter \
          declared in a source file always has a type, so the result is text. The result is \
          `null` only if no type is known for the parameter. To check the type of a value, use \
          `Core\\Reflect::typeOf`.",
    errors: &[],
};

/// `Core\Reflect\EnumInfo` — `rule:enums/reflection`'s description of one
/// declared `enum`: its name, its closed case list and which of
/// `rule:enums/one-backing-type`'s two integer types the cases are constants
/// of.
///
/// **The only description reached by name alone, and the only one with no
/// `forObject` twin.** `rule:enums/representation` makes a case *be* the
/// integer behind it at run time, so there is no value to ask and no descriptor
/// to ask it of: what the runtime carries is the shape itself
/// ([`nvs_runtime::EnumDesc`]), and `of` is the one door onto it.
///
/// It grants an enum nothing `rule:enums/no-class-machinery` withholds. Every
/// member here is a reader over a slot the description was built with, there is
/// no `get`, no `call` and no `construct` for [`CLASS_INFO`]'s acting half to
/// be the twin of, and nothing it carries reaches back into the program.
pub(crate) const ENUM_INFO: CoreClass = CoreClass {
    name: ENUM_INFO_NAME,
    doc: Some(&ENUM_INFO_CARD),
    methods: &[CoreMethod {
        name: "of",
        names: &["name"],
        params: &[CoreTy::Text(Qual::Neutral)],
        defaults: &[],
        // `?EnumInfo`, on `Core\Reflect::forClass`'s terms exactly: a name the
        // program declares no enum for is an absence (`rule:core-api/shape-rules` R6), not a failure.
        return_ty: CoreTy::Nullable(&CoreTy::Instance(ENUM_INFO_NAME)),
        symbol: "nvs_core_reflect_enum_info_of",
        doc: Some(&ENUM_OF_DOC),
    }],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_enum_info_name",
            doc: Some(&ENUM_NAME_DOC),
        },
        CoreMethod {
            name: "cases",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_reflect_enum_info_cases",
            doc: Some(&ENUM_CASES_DOC),
        },
        CoreMethod {
            name: "valueOf",
            names: &["case"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            // The union is `rule:enums/one-backing-type`'s two options and no
            // third: which one this enum answers with is `isUnsigned`, asked
            // once rather than per case.
            return_ty: CoreTy::Union(&[CoreTy::Int, CoreTy::Uint]),
            symbol: "nvs_core_reflect_enum_info_value_of",
            doc: Some(&ENUM_VALUE_OF_DOC),
        },
        CoreMethod {
            name: "isUnsigned",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_reflect_enum_info_is_unsigned",
            doc: Some(&ENUM_IS_UNSIGNED_DOC),
        },
    ],
    slots: &["name", "cases", "values", "unsigned"],
    constants: &[],
};

/// `Core\Reflect\EnumInfo`'s class card — `rule:core-api/reference-card`.
const ENUM_INFO_CARD: ClassDoc = ClassDoc {
    short: "One enum, as `Core\\Reflect\\EnumInfo::of` returns it for the enum's name. `cases` \
            lists the names of its cases, `valueOf` returns the value of one case, and \
            `isUnsigned` says whether the values are `uint` or `int`.",
};

/// `Core\Reflect\EnumInfo::of`'s reference card — `rule:core-api/reference-card`.
const ENUM_OF_DOC: MethodDoc = MethodDoc {
    short: "Returns a description of the enum with the name `$name`. An enum has no methods of its \
            own, so this is the way to read the list of its cases.",
    params: &[ParamDoc {
        name: "name",
        desc: "The full name of the enum, with its namespace and with no `\\` at the start. \
               `Status::class` gives this name.",
        shape: &[],
    }],
    ret: "A `Core\\Reflect\\EnumInfo` for that enum. The result is `null` if the program has no \
          enum with that name, and also for the name of a class.",
    errors: &[],
};

/// `Core\Reflect\EnumInfo::name`'s reference card — `rule:core-api/reference-card`.
const ENUM_NAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the full name of the enum.",
    params: &[],
    ret: "The name that was passed to `of`, with its namespace. You can pass it to `of` again to \
          get a description of the same enum.",
    errors: &[],
};

/// `Core\Reflect\EnumInfo::cases`'s reference card — `rule:core-api/reference-card`.
const ENUM_CASES_DOC: MethodDoc = MethodDoc {
    short: "Every case the enum declares, by name.",
    params: &[],
    ret: "One string per case, sorted by the value of the case from the smallest to the largest. \
          Two cases with the same value are sorted by name. The order is not the order of the \
          declaration. An enum with no case gives an empty list.",
    errors: &[],
};

/// `Core\Reflect\EnumInfo::valueOf`'s reference card — `rule:core-api/reference-card`.
const ENUM_VALUE_OF_DOC: MethodDoc = MethodDoc {
    short: "Returns the value of one case of the enum.",
    params: &[ParamDoc {
        name: "case",
        desc: "The name of the case, as `cases` gives it. The name is case-sensitive.",
        shape: &[],
    }],
    ret: "The value of the case. It is a `uint` if `isUnsigned` returns `true`, and an `int` \
          otherwise. Use `as uint` or `as int` to store it in a variable of that type.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The enum has no case with that name. Use `cases` to get the names that exist.",
    }],
};

/// `Core\Reflect\EnumInfo::isUnsigned`'s reference card — `rule:core-api/reference-card`.
const ENUM_IS_UNSIGNED_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the values of the enum's cases are `uint` or `int`.",
    params: &[],
    ret: "`true` for an enum declared with `: uint`. `false` for an enum declared with `: int` \
          or with no type, because an enum with no type is an `int` enum.",
    errors: &[],
};

/// `Core\Reflect\PropertyInfo` — one row of [`CLASS_INFO`]'s property roster: a
/// property's name, its visibility and the type its declaration spells.
///
/// [`METHOD_INFO`]'s shape, one member over, and for the same reasons: every
/// member is a reader over a slot the description was built with, nothing here
/// acts, and the row exists for a `private` property as much as for a `public`
/// one. Reading the property is [`nvs_core_reflect_class_info_get`]'s, which is
/// where § 2's check is made.
pub(crate) const PROPERTY_INFO: CoreClass = CoreClass {
    name: PROPERTY_INFO_NAME,
    doc: Some(&PROPERTY_INFO_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_property_info_name",
            doc: Some(&PROPERTY_NAME_DOC),
        },
        CoreMethod {
            name: "isPublic",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_reflect_property_info_is_public",
            doc: Some(&PROPERTY_IS_PUBLIC_DOC),
        },
        CoreMethod {
            name: "type",
            names: &[],
            params: &[],
            defaults: &[],
            // `?string`, and the `null` is an absence rather than an unknown:
            // [`nvs_runtime::ClassDesc::field_type`]'s own doc comment owns the
            // two slots no declaration named a type for.
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_reflect_property_info_type",
            doc: Some(&PROPERTY_TYPE_DOC),
        },
    ],
    slots: &["name", "public", "type"],
    constants: &[],
};

/// `Core\Reflect\PropertyInfo`'s class card — `rule:core-api/reference-card`.
const PROPERTY_INFO_CARD: ClassDoc = ClassDoc {
    short: "One property of a class, as `Core\\Reflect\\ClassInfo::properties` returns it. `name` \
            and `isPublic` describe the declaration. `type` returns the type the property is \
            declared with.",
};

/// `Core\Reflect\PropertyInfo::name`'s reference card — `rule:core-api/reference-card`.
const PROPERTY_NAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the property.",
    params: &[],
    ret: "The name as the class declares it, with no `$` and no class name. You can pass the name \
          to `Core\\Reflect\\ClassInfo::hasProperty`, `Core\\Reflect\\ClassInfo::get` and \
          `Core\\Reflect\\ClassInfo::set`.",
    errors: &[],
};

/// `Core\Reflect\PropertyInfo::isPublic`'s reference card — `rule:core-api/reference-card`.
const PROPERTY_IS_PUBLIC_DOC: MethodDoc = MethodDoc {
    short: "Tells you whether code outside the class may read and write the property.",
    params: &[],
    ret: "`true` for a `public` property, and `false` for a `protected` or `private` property. \
          Those properties are still in the list that `Core\\Reflect\\ClassInfo::properties` \
          returns. If `isPublic` returns `false`, `Core\\Reflect\\ClassInfo::get` from outside \
          the class throws a `RuntimeError`.",
    errors: &[],
};

/// `Core\Reflect\PropertyInfo::type`'s reference card — `rule:core-api/reference-card`.
const PROPERTY_TYPE_DOC: MethodDoc = MethodDoc {
    short: "Returns the type the property is declared with, written the way the class writes it.",
    params: &[],
    ret: "The type as a string, such as `int`, `?int`, `array<string>` or `Customer`. The \
          properties of a built-in error class, such as `message` on `LogicError`, have no \
          written type, so the result is `null`. To find the type of a value, use \
          `Core\\Reflect::typeOf`.",
    errors: &[],
};

/// `Core\Reflect\ConstantInfo`'s class card — `rule:core-api/reference-card`.
const CONSTANT_INFO_CARD: ClassDoc = ClassDoc {
    short: "One class constant, as `Core\\Reflect\\ClassInfo::constants` returns it. `name` and \
            `isPublic` describe the declaration. `hasValue` says whether \
            `Core\\Reflect\\ClassInfo::constant` can return its value.",
};

/// One class constant of a description's roster — `Core\Reflect\ConstantInfo`.
///
/// [`PROPERTY_INFO`]'s shape for the member that is a *value*, and the third
/// slot is where the two part company. A property's row names the type its
/// declaration spells; a constant's declaration *is* its value, so what the row
/// carries instead is whether there is one to read — a constant folds when its
/// right-hand side is one of the four literals `nvs_types::consts` resolves,
/// and an `array` or object constant keeps its row with nothing to hand back.
///
/// **No slot holds the value**, and that is `rule:core-classes/reflect` § 2's
/// division applied to a constant: naming one is metadata and is always
/// available, reading one is acting and faces the check ordinary code at the
/// call site faces. A row built by [`describe`] has no site to be judged
/// against, so the value is [`nvs_core_reflect_class_info_constant`]'s to hand
/// back — the same split `properties` and `get` already make.
pub(crate) const CONSTANT_INFO: CoreClass = CoreClass {
    name: CONSTANT_INFO_NAME,
    doc: Some(&CONSTANT_INFO_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_constant_info_name",
            doc: Some(&CONSTANT_NAME_DOC),
        },
        CoreMethod {
            name: "isPublic",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_reflect_constant_info_is_public",
            doc: Some(&CONSTANT_IS_PUBLIC_DOC),
        },
        CoreMethod {
            name: "hasValue",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_reflect_constant_info_has_value",
            doc: Some(&CONSTANT_HAS_VALUE_DOC),
        },
    ],
    slots: &["name", "public", "hasValue"],
    constants: &[],
};

/// `Core\Reflect\ConstantInfo::name`'s reference card — `rule:core-api/reference-card`.
const CONSTANT_NAME_DOC: MethodDoc = MethodDoc {
    short: "The constant's name, as the declaring class writes it.",
    params: &[],
    ret: "The name with no class qualifier and no `::` — what \
          `Core\\Reflect\\ClassInfo::constant` takes.",
    errors: &[],
};

/// `Core\Reflect\ConstantInfo::isPublic`'s reference card — `rule:core-api/reference-card`.
const CONSTANT_IS_PUBLIC_DOC: MethodDoc = MethodDoc {
    short: "Whether code outside the declaring class may name the constant.",
    params: &[],
    ret: "`false` for a `private` or `protected` constant, which is still listed, on \
          `Core\\Reflect\\PropertyInfo::isPublic`'s terms exactly: knowing that a constant exists \
          and may not be read from here is what tells a refusal from a misspelling.",
    errors: &[],
};

/// `Core\Reflect\ConstantInfo::hasValue`'s reference card — `rule:core-api/reference-card`.
const CONSTANT_HAS_VALUE_DOC: MethodDoc = MethodDoc {
    short: "Whether the constant's declared value is one Novis folds at compile time, and so one \
            `Core\\Reflect\\ClassInfo::constant` can hand back.",
    params: &[],
    ret: "`true` for a `string`, `int`, `bool` or `float` literal — `false` for an `array` or \
          object constant, and for an integer no `int` holds. Novis folds a constant that *is* a \
          literal and runs no second constant-expression evaluator, so this reports a stated \
          bound rather than an unknown, and it is `false` for exactly the constants the checker \
          also refuses in type position.",
    errors: &[],
};

/// One attach site of a description's roster — `Core\Reflect\AttributeInfo`.
///
/// [`CONSTANT_INFO`]'s shape for the thing that is not a member at all. A
/// constant's row is a name and a visibility; an attribute has neither, being
/// written above a declaration rather than declared, so what its row carries
/// instead is *where* it was written — the member and, for a parameter's site,
/// the parameter — and the payload it attaches.
///
/// The payload is two parallel slots rather than one array keyed by name, on
/// [`ENUM_INFO`]'s terms exactly: a `Core` instance's slot holds a value Novis
/// already holds, and a map is not one of those. [`ATTRIBUTE_FIELDS_SLOT`] is
/// the roster, in source order, and [`nvs_core_reflect_attribute_info_field`]
/// is the acting door onto the value — the same split `constants` and
/// `constant` already make, minus the visibility check, which a payload has
/// nothing to be judged by.
pub(crate) const ATTRIBUTE_INFO: CoreClass = CoreClass {
    name: ATTRIBUTE_INFO_NAME,
    doc: Some(&ATTRIBUTE_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_attribute_info_name",
            doc: Some(&ATTRIBUTE_NAME_DOC),
        },
        CoreMethod {
            name: "target",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_attribute_info_target",
            doc: Some(&ATTRIBUTE_TARGET_DOC),
        },
        CoreMethod {
            name: "parameter",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_reflect_attribute_info_parameter",
            doc: Some(&ATTRIBUTE_PARAMETER_DOC),
        },
        CoreMethod {
            name: "fields",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_reflect_attribute_info_fields",
            doc: Some(&ATTRIBUTE_FIELDS_DOC),
        },
        CoreMethod {
            name: "field",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            // `mixed`, on `ClassInfo::constant`'s terms: which of the four
            // literal types a payload field folded to is not known where the
            // call is written, the class being named at run time.
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_reflect_attribute_info_field",
            doc: Some(&ATTRIBUTE_FIELD_DOC),
        },
    ],
    slots: &["name", "target", "parameter", "fields", "values"],
    constants: &[],
};

/// `Core\Reflect\ClassInfo`'s class card — `rule:core-api/reference-card`.
const CLASS_INFO_CARD: ClassDoc = ClassDoc {
    short: "One class, as `Core\\Reflect::forClass` and `Core\\Reflect::forObject` return it. \
            `properties`, `methods`, `constants` and `attributes` list what the class declares. \
            `get`, `set`, `call` and `construct` use a class whose name is only known at run time.",
};

/// `Core\Reflect\ClassInfo::attributes`'s reference card — `rule:core-api/reference-card`.
const ATTRIBUTES_DOC: MethodDoc = MethodDoc {
    short: "Every `#[...]` written on the class's own declaration or on one of its own members.",
    params: &[],
    ret: "One row per attach site, in source order — the declaration's own first, then each \
          member's, and a method's parameters' after that method's. **Own-only**, where \
          `constants` is flattened over the ancestors: an attribute is a fact about the \
          declaration it is written on, so a base class's site is never reported as this \
          class's. Asking for a payload by its *shape* is `Core\\Attributes::get`, which is \
          checked where it is written and costs nothing at run time; this is the reflective \
          question, for a class the caller does not name in source.",
    errors: &[],
};

/// `Core\Reflect\AttributeInfo`'s class card — `rule:core-api/reference-card`.
const ATTRIBUTE_CARD: ClassDoc = ClassDoc {
    short: "One `#[...]` attribute written on a class or on one of its members, as \
            `Core\\Reflect\\ClassInfo::attributes` returns it. `target` and `parameter` say where \
            it is written. `fields` lists its payload's field names, and `field` returns one value.",
};

/// `Core\Reflect\AttributeInfo::name`'s reference card — `rule:core-api/reference-card`.
const ATTRIBUTE_NAME_DOC: MethodDoc = MethodDoc {
    short: "The `type` alias the named form gave the attribute, as it is written.",
    params: &[],
    ret: "The written name, or the **empty string** for the bare `#[{...}]` form. Unresolved on \
          purpose: the name checks the literal where it is written and is never how a caller \
          asks for one, so a resolved name here would report the checker's answer to a question \
          nobody asked.",
    errors: &[],
};

/// `Core\Reflect\AttributeInfo::target`'s reference card — `rule:core-api/reference-card`.
const ATTRIBUTE_TARGET_DOC: MethodDoc = MethodDoc {
    short: "The member the attribute is written on.",
    params: &[],
    ret: "A property's name with no `$` sigil, or a method's name — or the **empty string** for \
          the class or interface declaration itself. A parameter's site names its method here \
          and the parameter in `parameter`.",
    errors: &[],
};

/// `Core\Reflect\AttributeInfo::parameter`'s reference card — `rule:core-api/reference-card`.
const ATTRIBUTE_PARAMETER_DOC: MethodDoc = MethodDoc {
    short: "The parameter of `target` the attribute is written on.",
    params: &[],
    ret: "The parameter's name with no `$` sigil, or the **empty string** for every attach site \
          that is not a parameter's. The pair is what tells `#[X] public function f(...)` from \
          `public function f(#[X] int $n)`, which name one declaration each and not the same one.",
    errors: &[],
};

/// `Core\Reflect\AttributeInfo::fields`'s reference card — `rule:core-api/reference-card`.
const ATTRIBUTE_FIELDS_DOC: MethodDoc = MethodDoc {
    short: "The payload's field names, in the order the attribute writes them.",
    params: &[],
    ret: "One string per field, including a field whose value Novis does not fold here — the \
          name is as much a fact about the attach site as the value is, and `field` is what \
          reports the difference.",
    errors: &[],
};

/// `Core\Reflect\AttributeInfo::field`'s reference card — `rule:core-api/reference-card`.
const ATTRIBUTE_FIELD_DOC: MethodDoc = MethodDoc {
    short: "One payload field's value, folded at compile time.",
    params: &[ParamDoc {
        name: "name",
        desc: "The field's own name, as `fields` answers it.",
        shape: &[],
    }],
    ret: "The value as a `string`, `int`, `bool` or `float` — the four a payload's literal folds \
          to.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The attribute's payload has no field of that name, so the ask is a mistake rather \
               than an absence to report — `fields` is the list that cannot be wrong. Or the \
               field's value is one Novis does not fold into a description: a class constant, an \
               enum case or `Foo::class`, each of which resolves through the namespace the \
               attribute was *written* in, plus the literals `Core\\Reflect\\ConstantInfo` \
               reports the same way. `Core\\Attributes::get` reads all of those, at compile time \
               and by shape.",
    }],
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
/// Both walks name every member the class declares, each row carrying its own
/// visibility bit, and the module doc's § *both rosters are complete* owns why.
/// *Where a property is readable* is a question about the asking site rather
/// than about the class, so it is not a slot at all:
/// [`nvs_core_reflect_class_info_readable_properties`] filters this roster per
/// call, which is the only arrangement that can answer two sites two ways off
/// one description.
///
/// The one name skipped is a method whose name holds a `#`, which no source can
/// spell — `nvs_ir::lower`'s generator transform mints those, and a roster
/// naming one would be naming a rewriting rather than a declaration. A property
/// slot the descriptor cannot name is skipped for the reason above; a slot it
/// can name but whose type no declaration spelled keeps its row, with
/// [`PROPERTY_TYPE_SLOT`] holding `null`. A method row carries one
/// [`PARAMETER_INFO`] per name [`nvs_runtime::MethodRow::param_names`] holds,
/// each naming what [`nvs_runtime::MethodRow::param_types`] spells at the same
/// position, so a member nothing declared in source keeps its row with that
/// roster empty while [`METHOD_PARAMETER_COUNT_SLOT`] still answers what the
/// compiled code takes — that field's own doc comment owns which rows those
/// are.
///
/// The attach-site roster is the one that is not a walk over the class's
/// members at all: each [`nvs_runtime::AttributeDesc`] names the declaration it
/// was written on, so the rows are copied in the order the descriptor holds
/// them, which is source order. Its payload becomes two parallel slots per row,
/// and [`constant_value`] is what fills the second.
fn describe(desc: &ClassDesc) -> Value {
    let mut properties = NvsArray::new();
    for slot in 0..desc.field_count() {
        let Some(name) = desc.field_name(slot) else {
            continue;
        };
        let ty = match desc.field_type(slot) {
            Some(ty) => Value::str(NvsStr::new(ty.as_bytes())),
            None => Value::null(),
        };
        properties.append(crate::instance::build(
            &PROPERTY_INFO,
            [
                Value::str(NvsStr::new(name.as_bytes())),
                Value::bool(desc.field_is_public(slot)),
                ty,
            ],
        ));
    }
    let mut methods = NvsArray::new();
    for index in 0..desc.method_count() {
        let Some(row) = desc.method_at(index) else {
            continue;
        };
        if row.name.contains('#') {
            continue;
        }
        let mut parameters = NvsArray::new();
        for (slot, name) in row.param_names.iter().enumerate() {
            // The two rosters are one declaration read twice, so the type is
            // taken by the name's own position rather than by a second search.
            // A position the table spelled nothing at answers `null`, which is
            // the absence [`PARAMETER_TYPE_SLOT`] carries.
            let ty = match row.param_types.get(slot) {
                Some(ty) if !ty.is_empty() => Value::str(NvsStr::new(ty.as_bytes())),
                _ => Value::null(),
            };
            parameters.append(crate::instance::build(
                &PARAMETER_INFO,
                [Value::str(NvsStr::new(name.as_bytes())), ty],
            ));
        }
        methods.append(crate::instance::build(
            &METHOD_INFO,
            [
                Value::str(NvsStr::new(row.name.as_bytes())),
                Value::bool(row.public),
                Value::uint(u64::from(row.arity)),
                Value::array(parameters),
            ],
        ));
    }
    let mut constants = NvsArray::new();
    for constant in desc.constants() {
        constants.append(crate::instance::build(
            &CONSTANT_INFO,
            [
                Value::str(NvsStr::new(constant.name.as_bytes())),
                Value::bool(constant.public),
                Value::bool(constant.value != nvs_runtime::ConstantValue::Opaque),
            ],
        ));
    }
    let mut attributes = NvsArray::new();
    for attribute in desc.attributes() {
        let mut names = NvsArray::new();
        let mut values = NvsArray::new();
        for (name, value) in &attribute.fields {
            names.append(Value::str(NvsStr::new(name.as_bytes())));
            values.append(constant_value(value));
        }
        attributes.append(crate::instance::build(
            &ATTRIBUTE_INFO,
            [
                Value::str(NvsStr::new(attribute.name.as_bytes())),
                Value::str(NvsStr::new(attribute.member.as_bytes())),
                Value::str(NvsStr::new(attribute.parameter.as_bytes())),
                Value::array(names),
                Value::array(values),
            ],
        ));
    }
    crate::instance::build(
        &CLASS_INFO,
        [
            Value::str(NvsStr::new(desc.name().as_bytes())),
            Value::array(properties),
            Value::array(methods),
            Value::array(constants),
            Value::array(attributes),
        ],
    )
}

/// One folded constant as the value a program reads it as, with
/// [`nvs_runtime::ConstantValue::Opaque`] answering `null`.
///
/// The `null` is a slot's filler and never an answer: it stands in a payload's
/// value roster so the two slots stay aligned name for name, and
/// [`nvs_core_reflect_attribute_info_field`] turns reading one into the throw
/// the bound deserves. No payload value folds to `null` itself — a written
/// `null` is not one of the four literals — so the two cannot be confused.
fn constant_value(value: &nvs_runtime::ConstantValue) -> Value {
    match value {
        nvs_runtime::ConstantValue::Str(text) => Value::str(NvsStr::new(text.as_bytes())),
        nvs_runtime::ConstantValue::Int(value) => Value::int(*value),
        nvs_runtime::ConstantValue::Bool(value) => Value::bool(*value),
        nvs_runtime::ConstantValue::Float(value) => Value::float(*value),
        nvs_runtime::ConstantValue::Opaque => Value::null(),
    }
}

/// One [`nvs_runtime::EnumDesc`] as the [`ENUM_INFO`] a program reads it
/// through — [`describe`]'s twin for the type that has no descriptor.
///
/// Both rosters are built here rather than on the first `cases` or `valueOf`
/// call, for [`describe`]'s reason exactly: a slot holding the shape itself
/// would be a raw pointer, and this module's second decision makes every slot a
/// value Novis already holds.
///
/// **What it spends:** per `of` call, two arrays and one string per declared
/// case, charged to the request that asked and released with the description.
fn describe_enum(desc: &nvs_runtime::EnumDesc) -> Value {
    let mut cases = NvsArray::new();
    let mut values = NvsArray::new();
    for (case, value) in desc.cases() {
        cases.append(Value::str(NvsStr::new(case.as_bytes())));
        // Both narrowings are exact rather than checked: the widening to `i128`
        // is what `nvs_ir::lower` did to an `i64` or a `u64` on the way down,
        // and `rule:enums/one-backing-type` is what says which one it was.
        values.append(if desc.unsigned() {
            Value::uint(u64::try_from(*value).expect("a `uint`-backed case widened from a `u64`"))
        } else {
            Value::int(i64::try_from(*value).expect("an `int`-backed case widened from an `i64`"))
        });
    }
    crate::instance::build(
        &ENUM_INFO,
        [
            Value::str(NvsStr::new(desc.name().as_bytes())),
            Value::array(cases),
            Value::array(values),
            Value::bool(desc.unsigned()),
        ],
    )
}

/// The [`TYPE_KIND`] case a tag is, or `None` for a tag no value carries.
///
/// The one `None` is the whole of what [`TYPE_KIND`] leaves out, and it is
/// unreachable rather than omitted: [`Tag::Unset`] is a storage state that
/// every read turns into a throw before a member can see one. A second tag
/// arriving here would be a new representation, and answering it *some* case
/// would be worse than the fatal [`nvs_core_reflect_type_of`] gives it.
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
        Tag::Unset => return None,
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
        "nvs_core_reflect_class_info_readable_properties" => {
            (nvs_core_reflect_class_info_readable_properties as *const ()).cast()
        }
        "nvs_core_reflect_class_info_has_property" => {
            (nvs_core_reflect_class_info_has_property as *const ()).cast()
        }
        "nvs_core_reflect_class_info_methods" => {
            (nvs_core_reflect_class_info_methods as *const ()).cast()
        }
        "nvs_core_reflect_class_info_has_method" => {
            (nvs_core_reflect_class_info_has_method as *const ()).cast()
        }
        "nvs_core_reflect_class_info_constants" => {
            (nvs_core_reflect_class_info_constants as *const ()).cast()
        }
        "nvs_core_reflect_class_info_constant" => {
            (nvs_core_reflect_class_info_constant as *const ()).cast()
        }
        "nvs_core_reflect_class_info_attributes" => {
            (nvs_core_reflect_class_info_attributes as *const ()).cast()
        }
        "nvs_core_reflect_attribute_info_name" => {
            (nvs_core_reflect_attribute_info_name as *const ()).cast()
        }
        "nvs_core_reflect_attribute_info_target" => {
            (nvs_core_reflect_attribute_info_target as *const ()).cast()
        }
        "nvs_core_reflect_attribute_info_parameter" => {
            (nvs_core_reflect_attribute_info_parameter as *const ()).cast()
        }
        "nvs_core_reflect_attribute_info_fields" => {
            (nvs_core_reflect_attribute_info_fields as *const ()).cast()
        }
        "nvs_core_reflect_attribute_info_field" => {
            (nvs_core_reflect_attribute_info_field as *const ()).cast()
        }
        "nvs_core_reflect_constant_info_name" => {
            (nvs_core_reflect_constant_info_name as *const ()).cast()
        }
        "nvs_core_reflect_constant_info_is_public" => {
            (nvs_core_reflect_constant_info_is_public as *const ()).cast()
        }
        "nvs_core_reflect_constant_info_has_value" => {
            (nvs_core_reflect_constant_info_has_value as *const ()).cast()
        }
        "nvs_core_reflect_property_info_name" => {
            (nvs_core_reflect_property_info_name as *const ()).cast()
        }
        "nvs_core_reflect_property_info_is_public" => {
            (nvs_core_reflect_property_info_is_public as *const ()).cast()
        }
        "nvs_core_reflect_property_info_type" => {
            (nvs_core_reflect_property_info_type as *const ()).cast()
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
        "nvs_core_reflect_method_info_parameters" => {
            (nvs_core_reflect_method_info_parameters as *const ()).cast()
        }
        "nvs_core_reflect_parameter_info_name" => {
            (nvs_core_reflect_parameter_info_name as *const ()).cast()
        }
        "nvs_core_reflect_parameter_info_type" => {
            (nvs_core_reflect_parameter_info_type as *const ()).cast()
        }
        "nvs_core_reflect_enum_info_of" => (nvs_core_reflect_enum_info_of as *const ()).cast(),
        "nvs_core_reflect_enum_info_name" => (nvs_core_reflect_enum_info_name as *const ()).cast(),
        "nvs_core_reflect_enum_info_cases" => {
            (nvs_core_reflect_enum_info_cases as *const ()).cast()
        }
        "nvs_core_reflect_enum_info_value_of" => {
            (nvs_core_reflect_enum_info_value_of as *const ()).cast()
        }
        "nvs_core_reflect_enum_info_is_unsigned" => {
            (nvs_core_reflect_enum_info_is_unsigned as *const ()).cast()
        }
        "nvs_core_reflect_class_info_get" => (nvs_core_reflect_class_info_get as *const ()).cast(),
        "nvs_core_reflect_class_info_set" => (nvs_core_reflect_class_info_set as *const ()).cast(),
        "nvs_core_reflect_class_info_call" => {
            (nvs_core_reflect_class_info_call as *const ()).cast()
        }
        "nvs_core_reflect_class_info_construct" => {
            (nvs_core_reflect_class_info_construct as *const ()).cast()
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

/// One of the receiving description's two rosters, borrowed for a scan.
///
/// The three members that walk a roster rather than handing it back share this,
/// because the refusal is the same one each time: a slot [`describe`] fills
/// with an array holds an array, so a tag that is not one is this crate's own
/// mistake and never a program's.
fn roster_of(
    receiver: *mut nvs_runtime::ObjHeader,
    class: &CoreClass,
    index: usize,
    member: &str,
) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let held = crate::instance::slot(receiver, index);
    let Some(ptr) = held.array_ptr() else {
        return Err(Fault::fatal(format!(
            "{}::{member} expected {:?} in its roster slot, got tag {}",
            class.name,
            Tag::Array,
            held.tag_byte()
        )));
    };
    Ok(crate::arr::borrowed(ptr))
}

/// Row `index` of a borrowed roster, as the object its slots are read off.
///
/// By slot rather than by key: [`describe`] builds both rosters with `append`
/// alone and hands them to nobody who can unset an entry, so their slots are
/// exactly `0..count` and a scan of them names every row once.
fn row_at(roster: &NvsArray, index: usize) -> Option<*mut nvs_runtime::ObjHeader> {
    roster.value_at(index)?.obj_ptr()
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
    /// `Core\Reflect\ClassInfo::properties(): array<Core\Reflect\PropertyInfo>`
    /// — ADR 0019 § 1's property roster, replacing
    /// `ReflectionClass::getProperties`.
    ///
    /// Every declared property, [`METHOD_INFO`]'s roster one member over, and §
    /// 2 is why the `private` ones are among them: naming a member and reading
    /// its declared type is the program's *shape*, which is always available,
    /// while reading the property is acting and goes through
    /// [`nvs_core_reflect_class_info_get`]'s check. What this call site may read
    /// is [`nvs_core_reflect_class_info_readable_properties`], which is a
    /// question about the site rather than about the class.
    fn nvs_core_reflect_class_info_properties(_ctx, args: [1]) {
        slot_of(args, &CLASS_INFO, PROPERTIES_SLOT, "properties")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::readableProperties(): array<string>` —
    /// `rule:security/reflection-enforces-visibility`'s visibility-respecting
    /// walk, replacing `get_object_vars`.
    ///
    /// The rule is stated over the **call site** and not over the description,
    /// so one description answers as many ways as there are places to ask
    /// from: inside the described class every declared name, from another
    /// class in its hierarchy the `protected` ones beside the `public`, and
    /// anywhere else the `public` ones alone — each being what an ordinary body
    /// at that site reads. The last argument —
    /// [`crate::registry::CALL_SITE_MEMBERS`]' constant, which no program can
    /// write — is what picks between them, and a site inside no class at all
    /// arrives as the zero word and is answered as outside.
    ///
    /// The level comes off the described class's own descriptor
    /// ([`nvs_runtime::Ctx::field_is_visible_from`]) and not off the roster row
    /// beside the name, which carries the `public` bit and not the level behind
    /// it. A description of a class this program does not declare has no
    /// descriptor to ask and falls back to that bit, which is the narrower
    /// answer for the family that has no `protected` property to report.
    ///
    /// Filtered out of the roster rather than held in a slot, because two sites
    /// asking one description are owed two lists and a slot can hold one. What
    /// that spends is the module doc's § *what it spends*: one array of one
    /// string per name, per call.
    fn nvs_core_reflect_class_info_readable_properties(ctx, args: [2]) {
        let member = "readableProperties";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let described = crate::instance::slot(receiver, NAME_SLOT);
        let described = described.as_text();
        let site = site_class(args[1]);
        let inside = site.is_some_and(|site| described == Some(site));
        #[expect(
            unsafe_code,
            reason = "`class_desc` answers with a pointer into the compiled unit's class table, \
                      which outlives this context and is never rewritten while a member of it is \
                      running"
        )]
        let desc = described
            .and_then(|name| ctx.class_desc(name))
            .map(|desc| unsafe { &*desc });
        let roster = roster_of(receiver, &CLASS_INFO, PROPERTIES_SLOT, member)?;
        let mut names = NvsArray::new();
        for index in 0..roster.count() {
            let Some(row) = row_at(&roster, index) else {
                continue;
            };
            let held = crate::instance::slot(row, PROPERTY_NAME_SLOT);
            let Some(name) = held.as_text() else {
                continue;
            };
            let visible = if let Some(desc) = desc {
                ctx.field_is_visible_from(desc, name, site)
            } else {
                inside || crate::instance::slot(row, PROPERTY_PUBLIC_SLOT).as_bool() == Some(true)
            };
            if !visible {
                continue;
            }
            names.append(Value::str(NvsStr::new(name.as_bytes())));
        }
        Ok(Value::array(names))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::hasProperty(string $name): bool` — replacing
    /// `property_exists`.
    ///
    /// [`nvs_core_reflect_class_info_has_method`]'s scan over the other roster,
    /// and its doc comment owns the shape. It answers `true` for a `private`
    /// property, which is the whole point: `rule:core-classes/reflect` asks that
    /// a refusal be distinguishable from a misspelling, and a site that may not
    /// read the property learns here that it exists.
    fn nvs_core_reflect_class_info_has_property(_ctx, args: [2]) {
        let member = "hasProperty";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let name = text_of(&args[1], "Core\\Reflect\\ClassInfo::hasProperty")?;
        let roster = roster_of(receiver, &CLASS_INFO, PROPERTIES_SLOT, member)?;
        let found = (0..roster.count()).any(|index| {
            row_at(&roster, index).is_some_and(|row| {
                crate::instance::slot(row, PROPERTY_NAME_SLOT).as_text() == Some(name)
            })
        });
        Ok(Value::bool(found))
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
    /// `Core\Reflect\ClassInfo::constants(): array<Core\Reflect\ConstantInfo>`
    /// — ADR 0019 § 1's third roster, replacing
    /// `ReflectionClass::getReflectionConstants`.
    ///
    /// The whole roster, on the two above it terms exactly: naming a constant
    /// reads the program's shape, and reading its value is acting and goes
    /// through [`nvs_core_reflect_class_info_constant`]'s check. Answered off
    /// the slot [`describe`] filled.
    fn nvs_core_reflect_class_info_constants(_ctx, args: [1]) {
        slot_of(args, &CLASS_INFO, CONSTANTS_SLOT, "constants")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::constant(string $name): mixed` — ADR 0019 § 2's
    /// rule that *acting* on a member faces the ordinary check, for the member
    /// whose declaration is its value.
    ///
    /// Four refusals in one order, and the order is
    /// [`nvs_core_reflect_class_info_get`]'s for its reason: the name has to be
    /// one this class answers, then the site has to reach it, then the
    /// declaration has to carry no `secret`, and only then is the fold asked
    /// for. A member that asked visibility first would answer a misspelling
    /// with a `private` read's refusal.
    ///
    /// The `secret` refusal is the one that is not `get`'s, and it is made at
    /// **every** site including the declaring class's own. This member answers
    /// `mixed`, which carries no qualifier, so a `secret string` handed through
    /// it would be an ordinary string at the next sink and
    /// `rule:security/secret-sinks-refuse` would never fire. Refusing is the
    /// direction that fails closed; the value is still reachable by naming the
    /// constant in source, where the qualifier survives.
    ///
    /// The subject is reached by name rather than through an object, unlike
    /// `get`: a constant belongs to the class, so there is no instance for a
    /// caller to hand over and none for this to pair against.
    fn nvs_core_reflect_class_info_constant(ctx, args: [3]) {
        let member = "constant";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let name = text_of(&args[1], "Core\\Reflect\\ClassInfo::constant")?;
        let site = site_class(args[2]);
        let described = crate::instance::slot(receiver, NAME_SLOT);
        let described = described.as_text().unwrap_or_default();
        #[expect(
            unsafe_code,
            reason = "`class_desc` answers with a pointer into the compiled unit's class table, \
                      which outlives this context and is never rewritten while a member of it is \
                      running"
        )]
        let desc = ctx.class_desc(described).map(|desc| unsafe { &*desc });
        let Some(constant) = desc.and_then(|desc| desc.constant(name)) else {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{CLASS_INFO_NAME}::constant(): `{described}` has no constant named `{name}`"
                ),
            ));
        };
        if !desc.is_some_and(|desc| ctx.constant_is_visible_from(desc, name, site)) {
            return Err(Fault::thrown(format!(
                "{CLASS_INFO_NAME}::constant(): `{described}::{name}` is not readable from \
                 outside the class, and reflection does not lift that"
            )));
        }
        if constant.secret {
            return Err(Fault::thrown(format!(
                "{CLASS_INFO_NAME}::constant(): `{described}::{name}` is declared `secret`, and \
                 this member answers `mixed`, which carries no qualifier — name the constant in \
                 source, where it does"
            )));
        }
        Ok(match &constant.value {
            nvs_runtime::ConstantValue::Str(text) => Value::str(NvsStr::new(text.as_bytes())),
            nvs_runtime::ConstantValue::Int(value) => Value::int(*value),
            nvs_runtime::ConstantValue::Bool(value) => Value::bool(*value),
            nvs_runtime::ConstantValue::Float(value) => Value::float(*value),
            nvs_runtime::ConstantValue::Opaque => {
                return Err(Fault::thrown_as(
                    ThrownClass::Logic,
                    format!(
                        "{CLASS_INFO_NAME}::constant(): `{described}::{name}` is declared with a \
                         value Novis does not fold, so there is none to read — `hasValue` reports \
                         that ahead of the call"
                    ),
                ));
            }
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::attributes(): array<Core\Reflect\AttributeInfo>`
    /// — ADR 0019 § 1's fourth roster, and the one no PHP `Reflection*` member
    /// it replaces had an equivalent for.
    ///
    /// Answered off the slot [`describe`] filled, on `constants`' terms: naming
    /// an attach site reads the program's shape, and reading a payload value is
    /// [`nvs_core_reflect_attribute_info_field`]'s.
    fn nvs_core_reflect_class_info_attributes(_ctx, args: [1]) {
        slot_of(args, &CLASS_INFO, ATTRIBUTES_SLOT, "attributes")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\AttributeInfo::name(): string` — the name the named form
    /// gave the attribute, unresolved.
    fn nvs_core_reflect_attribute_info_name(_ctx, args: [1]) {
        slot_of(args, &ATTRIBUTE_INFO, ATTRIBUTE_NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\AttributeInfo::target(): string` — the member the
    /// attribute is written on, empty for the class declaration itself.
    fn nvs_core_reflect_attribute_info_target(_ctx, args: [1]) {
        slot_of(args, &ATTRIBUTE_INFO, ATTRIBUTE_TARGET_SLOT, "target")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\AttributeInfo::parameter(): string` — the parameter of
    /// `target` it is written on, empty for every other attach site.
    fn nvs_core_reflect_attribute_info_parameter(_ctx, args: [1]) {
        slot_of(args, &ATTRIBUTE_INFO, ATTRIBUTE_PARAMETER_SLOT, "parameter")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\AttributeInfo::fields(): array<string>` — the payload's
    /// field names in source order.
    fn nvs_core_reflect_attribute_info_fields(_ctx, args: [1]) {
        slot_of(args, &ATTRIBUTE_INFO, ATTRIBUTE_FIELDS_SLOT, "fields")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\AttributeInfo::field(string $name): mixed` — one payload
    /// field's folded value.
    ///
    /// Two refusals in one order, and it is
    /// [`nvs_core_reflect_class_info_constant`]'s: the name has to be one this
    /// payload carries, and only then is the value asked for. A member that
    /// tested the value first would answer a misspelling with the unfolded
    /// field's refusal, which is a different fact about a different field.
    ///
    /// No visibility check, where that member makes one: an attribute is
    /// written above a declaration and carries no modifier of its own, and
    /// `rule:attributes/payload-is-a-compile-time-constant` refuses a `secret`
    /// value where the payload is written — so there is nothing here to judge
    /// and nothing qualified to launder.
    fn nvs_core_reflect_attribute_info_field(_ctx, args: [2]) {
        let member = "field";
        let receiver = crate::instance::receiver(args[0], &ATTRIBUTE_INFO, member)?;
        let name = text_of(&args[1], "Core\\Reflect\\AttributeInfo::field")?;
        let names = roster_of(receiver, &ATTRIBUTE_INFO, ATTRIBUTE_FIELDS_SLOT, member)?;
        let values = roster_of(receiver, &ATTRIBUTE_INFO, ATTRIBUTE_VALUES_SLOT, member)?;
        let found = (0..names.count()).find(|index| {
            names
                .value_at(*index)
                .is_some_and(|held| held.as_text() == Some(name))
        });
        let Some(index) = found else {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{ATTRIBUTE_INFO_NAME}::field(): this attribute's payload has no field named \
                     `{name}` — `fields` is the list it does carry"
                ),
            ));
        };
        let held = values.value_at(index).unwrap_or_else(Value::null);
        if held.tag() == Some(Tag::Null) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{ATTRIBUTE_INFO_NAME}::field(): `{name}` is written with a value Novis does \
                     not fold into a description, so there is none to read — \
                     `Core\\Attributes::get` reads a payload by shape, at compile time, and \
                     resolves the names this cannot"
                ),
            ));
        }
        #[expect(
            unsafe_code,
            reason = "the value's reference belongs to the receiver's roster, which is live for \
                      the length of the call, and this value is being handed to the caller — \
                      which is exactly `Value::retain`'s obligation"
        )]
        unsafe {
            held.retain();
        }
        Ok(held)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ConstantInfo::name(): string` — the constant's own name.
    fn nvs_core_reflect_constant_info_name(_ctx, args: [1]) {
        slot_of(args, &CONSTANT_INFO, CONSTANT_NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ConstantInfo::isPublic(): bool` — the visibility bit
    /// `nvs_types::layout` fixed at the declaration and
    /// [`nvs_runtime::ConstantDesc`] carried down.
    fn nvs_core_reflect_constant_info_is_public(_ctx, args: [1]) {
        slot_of(args, &CONSTANT_INFO, CONSTANT_PUBLIC_SLOT, "isPublic")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ConstantInfo::hasValue(): bool` — whether the declaration
    /// folded, which is whether
    /// [`nvs_core_reflect_class_info_constant`] has a value to hand back.
    ///
    /// A bound reported as one: `nvs_runtime::ConstantValue::Opaque`'s own doc
    /// comment owns which declarations reach it and why Novis stops there.
    fn nvs_core_reflect_constant_info_has_value(_ctx, args: [1]) {
        slot_of(args, &CONSTANT_INFO, CONSTANT_HAS_VALUE_SLOT, "hasValue")
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
        let roster = roster_of(receiver, &CLASS_INFO, METHODS_SLOT, member)?;
        let found = (0..roster.count()).any(|index| {
            row_at(&roster, index).is_some_and(|row| {
                crate::instance::slot(row, METHOD_NAME_SLOT).as_text() == Some(name)
            })
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
        slot_of(args, &METHOD_INFO, METHOD_PARAMETER_COUNT_SLOT, "parameterCount")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\MethodInfo::parameters(): array<Core\Reflect\ParameterInfo>`
    /// — ADR 0019 § 1's parameter roster, off the names
    /// [`nvs_runtime::MethodRow::param_names`] carried down from the
    /// declaration.
    ///
    /// Answered off the slot [`describe`] filled, on
    /// [`nvs_core_reflect_class_info_methods`]' terms exactly. The roster is
    /// empty rather than invented for a method no source wrote, and
    /// [`nvs_core_reflect_method_info_parameter_count`] is the member that
    /// still answers for one.
    fn nvs_core_reflect_method_info_parameters(_ctx, args: [1]) {
        slot_of(args, &METHOD_INFO, METHOD_PARAMETERS_SLOT, "parameters")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ParameterInfo::name(): string` — the parameter's own name,
    /// `$`-sigil excluded.
    fn nvs_core_reflect_parameter_info_name(_ctx, args: [1]) {
        slot_of(args, &PARAMETER_INFO, PARAMETER_NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ParameterInfo::type(): ?string` — the declared type as its
    /// declaration spells it, from
    /// [`nvs_runtime::MethodRow::param_types`].
    ///
    /// `null` where the row carries no spelling for that parameter, which is
    /// the same absence `Core\Reflect\PropertyInfo::type` reports and reported
    /// the same way: what the descriptor holds, never a guess at what the
    /// compiled code takes.
    fn nvs_core_reflect_parameter_info_type(_ctx, args: [1]) {
        slot_of(args, &PARAMETER_INFO, PARAMETER_TYPE_SLOT, "type")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\EnumInfo::of(string $name): ?Core\Reflect\EnumInfo` —
    /// `rule:enums/reflection`'s description, reached by name.
    ///
    /// The name is resolved against the *running program's* enum roster
    /// (`nvs_runtime::Ctx::enum_desc`), which carries every enum the checker
    /// resolved for the unit — the program's own declarations and the `Core`
    /// ones alike, since both are seeded into the one table the compiler filled.
    /// That is the whole difference from
    /// [`nvs_core_reflect_for_class`]: a `Core` class is a descriptor this
    /// crate does not own, while a `Core` enum is a shape like any other.
    fn nvs_core_reflect_enum_info_of(ctx, args: [1]) {
        let name = text_of(&args[0], "Core\\Reflect\\EnumInfo::of")?;
        let Some(desc) = ctx.enum_desc(name) else {
            return Ok(Value::null());
        };
        Ok(describe_enum(desc))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\EnumInfo::name(): string` — the described enum's own name.
    fn nvs_core_reflect_enum_info_name(_ctx, args: [1]) {
        slot_of(args, &ENUM_INFO, ENUM_NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\EnumInfo::cases(): array<string>` — every declared case's
    /// name, in the order [`describe_enum`] built the roster in.
    fn nvs_core_reflect_enum_info_cases(_ctx, args: [1]) {
        slot_of(args, &ENUM_INFO, ENUM_CASES_SLOT, "cases")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\EnumInfo::isUnsigned(): bool` — whether the cases are
    /// `uint` constants.
    fn nvs_core_reflect_enum_info_is_unsigned(_ctx, args: [1]) {
        slot_of(args, &ENUM_INFO, ENUM_UNSIGNED_SLOT, "isUnsigned")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\EnumInfo::valueOf(string $case): int|uint` — the constant
    /// behind one case.
    ///
    /// A scan of the case roster rather than a keyed lookup, because the two
    /// rosters are parallel and the position found in one is the position read
    /// in the other — which is also what makes an unknown name answerable at
    /// all. It is a throw and not a `null`, unlike `of`'s: `cases` is the list
    /// that cannot be wrong, so a name that is not on it is a mistake in the
    /// asking rather than a fact about the enum.
    fn nvs_core_reflect_enum_info_value_of(_ctx, args: [2]) {
        let member = "valueOf";
        let receiver = crate::instance::receiver(args[0], &ENUM_INFO, member)?;
        let wanted = text_of(&args[1], "Core\\Reflect\\EnumInfo::valueOf")?;
        let cases = roster_of(receiver, &ENUM_INFO, ENUM_CASES_SLOT, member)?;
        let at = (0..cases.count()).find(|index| {
            cases
                .value_at(*index)
                .is_some_and(|value| value.as_text() == Some(wanted))
        });
        let Some(at) = at else {
            let name = crate::instance::slot(receiver, ENUM_NAME_SLOT);
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "enum `{}` declares no case `{wanted}`",
                    name.as_text().unwrap_or("?")
                ),
            ));
        };
        let values = roster_of(receiver, &ENUM_INFO, ENUM_VALUES_SLOT, member)?;
        let held = values.value_at(at).ok_or_else(|| {
            Fault::fatal(format!(
                "{ENUM_INFO_NAME}::{member} found case {at} with no constant beside it"
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "the constant's reference belongs to the receiver, which is live \
                      for the length of the call, and this value is being handed to the \
                      caller — which is exactly `Value::retain`'s obligation"
        )]
        unsafe {
            held.retain();
        }
        Ok(held)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\PropertyInfo::name(): string` — the property's own name,
    /// `$`-sigil excluded.
    fn nvs_core_reflect_property_info_name(_ctx, args: [1]) {
        slot_of(args, &PROPERTY_INFO, PROPERTY_NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\PropertyInfo::isPublic(): bool` — the visibility bit
    /// `nvs_types::layout` fixed at the declaration and
    /// [`nvs_runtime::ClassDesc::field_is_public`] carried down.
    fn nvs_core_reflect_property_info_is_public(_ctx, args: [1]) {
        slot_of(args, &PROPERTY_INFO, PROPERTY_PUBLIC_SLOT, "isPublic")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\PropertyInfo::type(): ?string` — the declared type as its
    /// declaration spells it, from
    /// [`nvs_runtime::ClassDesc::field_type`].
    ///
    /// `null` is the answer for a slot no declaration named a type for, which
    /// that method's own doc comment lists: an absence reported as one, rather
    /// than a guess at what the compiler laid the slot out as.
    fn nvs_core_reflect_property_info_type(_ctx, args: [1]) {
        slot_of(args, &PROPERTY_INFO, PROPERTY_TYPE_SLOT, "type")
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
    ///
    /// The visibility question is [`nvs_runtime::Ctx::field_is_visible_from`]'s
    /// whole answer and not a comparison written here: the site reaches a
    /// `private` property from the declaring class's own bodies and a
    /// `protected` one from every class in that hierarchy declaring it, which
    /// is what an ordinary read at each of those sites does.
    ///
    /// Past the four refusals the read itself is
    /// [`nvs_runtime::read_erased_property_owned`], the ordinary erased read,
    /// for [`nvs_core_reflect_class_info_set`]'s reason in the other direction:
    /// a `get` hook runs and a `lateinit` slot never written throws, exactly as
    /// `$value->name` on a `mixed` does. Reading the slot here would hand back a
    /// hooked property's backing store and an unwritten slot's storage state.
    fn nvs_core_reflect_class_info_get(ctx, args: [4]) {
        let member = "get";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let name = text_of(&args[2], "Core\\Reflect\\ClassInfo::get")?;
        let subject = subject_of(receiver, args[1], member)?;
        let site = site_class(args[3]);
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
            (
                desc.field_slot(name, 0),
                ctx.field_is_visible_from(desc, name, site),
            )
        };
        if slot.is_none() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{CLASS_INFO_NAME}::get(): `{}` has no property named `{name}`",
                    class_name_of(subject)
                ),
            ));
        }
        if !visible {
            return Err(Fault::thrown(format!(
                "{CLASS_INFO_NAME}::get(): `{}::{name}` is not readable from outside the \
                 class, and reflection does not lift that",
                class_name_of(subject)
            )));
        }
        nvs_runtime::read_erased_property_owned(ctx, args[1], name)
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
        let subject = subject_of(receiver, args[1], member)?;
        let site = site_class(args[4]);
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
            (
                desc.field_slot(name, 0),
                ctx.field_is_visible_from(desc, name, site),
            )
        };
        if slot.is_none() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{CLASS_INFO_NAME}::set(): `{}` has no property named `{name}`",
                    class_name_of(subject)
                ),
            ));
        }
        if !visible {
            return Err(Fault::thrown(format!(
                "{CLASS_INFO_NAME}::set(): `{}::{name}` is not writable from outside the \
                 class, and reflection does not lift that",
                class_name_of(subject)
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
        nvs_runtime::call_erased_method_from(ctx, args[1], name, &passed, site)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Reflect\ClassInfo::construct(array<mixed> $arguments): mixed` —
    /// ADR 0019 § 2's third acting member, invoking a reflected constructor.
    ///
    /// **No visibility rule is written here either**, for
    /// [`nvs_core_reflect_class_info_call`]'s reason and by the same route: past
    /// resolving the described name to a class this program declares, the work
    /// is [`nvs_runtime::construct_erased_from`]'s, which allocates and then
    /// hands the constructor to the erased call's own check with this site as
    /// the class the call is written inside. So a `private` constructor is
    /// reached from its own class's bodies — the singleton's `load()` keeps
    /// working through this door as through the ordinary one — and refused
    /// everywhere else, and `$arguments` is judged against the constructor's
    /// declared parameters rather than against its own length.
    ///
    /// The class is reached by **name**, because that is what a description
    /// holds: the module doc's first decision keeps a descriptor out of every
    /// slot, so the name goes back through [`nvs_runtime::Ctx::class_desc`] the
    /// way `forClass` first found it. A description of a `Core`-owned class has
    /// no entry there and is refused, which is the same answer `forClass` gives
    /// such a name.
    ///
    /// Ownership is the constructing path's: the entries are this frame's
    /// borrowed slots and the callee retains what it keeps, and the instance
    /// handed back is one fresh reference.
    fn nvs_core_reflect_class_info_construct(ctx, args: [3]) {
        let member = "construct";
        let receiver = crate::instance::receiver(args[0], &CLASS_INFO, member)?;
        let held = crate::instance::slot(receiver, NAME_SLOT);
        let Some(described) = held.as_text() else {
            return Err(Fault::fatal(format!(
                "{CLASS_INFO_NAME}::{member} expected {:?} in its name slot, got tag {}",
                Tag::Str,
                held.tag_byte()
            )));
        };
        let Some(desc) = ctx.class_desc(described) else {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{CLASS_INFO_NAME}::{member}(): `{described}` is not a class this program \
                     declares, so there is nothing to construct"
                ),
            ));
        };
        let site = site_class(args[2]);
        // `crate::arr`'s terms are `call`'s: parameter 1 is `array<mixed>` in
        // `CLASS_INFO` above, so a non-container argument is `E0401` at the
        // checker and this is what makes `array_ptr`'s answer safe to unwrap.
        let list = args[1].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "{CLASS_INFO_NAME}::construct expected {:?}, got tag {}",
                Tag::Array,
                args[1].tag_byte()
            ))
        })?;
        // In slot order, keys ignored, exactly as `call` reads its own list: a
        // parameter list is positional, and the count it is judged against is
        // the constructor's own.
        let list = crate::arr::borrowed(list);
        let mut passed = Vec::with_capacity(list.count());
        let mut slot = 0;
        while let Some(live) = list.next_slot(slot) {
            slot = live + 1;
            passed.push(list.value_at(live).expect("a live slot has a value"));
        }
        #[expect(
            unsafe_code,
            reason = "`class_desc` answers with a pointer into the compiled unit's class table, which outlives this context and is never rewritten while a member of it is running"
        )]
        let built = unsafe {
            nvs_runtime::construct_erased_from(ctx, desc, &passed, site)
        };
        built
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{
        AttributeDesc, ClassTable, ConstantValue, Ctx, ErrorClass, MethodRow, NvsArray, NvsFn,
        NvsObj, NvsStr, OK, OutputSink, Tag, Value, call,
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
            param_names: Vec::new(),
            param_types: Vec::new(),
            public,
            // `private`, which is the pair both bits are false for: this
            // fixture's point is the member a site outside the class is
            // refused, and `protected` would hand it to one.
            protected: false,
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
    // covers: Core\Reflect::forObject
    // covers: Core\Reflect\ClassInfo::call
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

    thread_local! {
        /// How many times [`singleton_constructor`] has run on this thread, so
        /// a refusal is read as a constructor that never ran and not only as a
        /// throw.
        static BUILT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }

    /// `Singleton::constructor`'s body, as `nvs-codegen` would have compiled a
    /// constructor declaring no parameters: slot 0 is the receiver `call_at`
    /// retained, and the answer is the `void` constructor's `null`.
    #[expect(
        unsafe_code,
        reason = "compiled code's own signature, which `call_at` calls through: \
                  one live value and the address of a live `Value` for the \
                  result, neither expressible in the type"
    )]
    unsafe extern "C" fn singleton_constructor(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        BUILT.with(|built| built.set(built.get() + 1));
        unsafe {
            (*args).release();
            *out = Value::null();
        }
        OK
    }

    /// `rule:security/reflection-enforces-visibility` for the constructing
    /// door, asked the way the `call` test above asks it: a reflective
    /// `construct` of a class whose constructor is `private` and an ordinary
    /// erased construction from outside every class have to fail with the same
    /// class and the same sentence, and neither may run the constructor. The
    /// same call from a site inside `Singleton` then builds one instance and
    /// runs the constructor once — the singleton's own `load()` keeps working
    /// through this door, so "they agree" cannot be two members refusing
    /// everything.
    // covers: Core\Reflect\ClassInfo::construct
    #[test]
    fn a_reflective_construct_of_a_private_constructor_fails_like_the_ordinary_construct() {
        let mut classes = ClassTable::new();
        let id = classes.define("Singleton", &[] as &[&str], &[]);
        classes.set_methods(
            id,
            vec![MethodRow {
                name: nvs_runtime::object::CONSTRUCTOR.to_owned(),
                code: (singleton_constructor as NvsFn) as *const u8,
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: false,
                protected: false,
                native: false,
            }],
        );
        let classes = std::sync::Arc::new(classes);
        let desc = classes.desc(id);
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, id));
        #[expect(
            unsafe_code,
            reason = "the descriptor belongs to the table the context above now \
                      holds for its whole life, and `Singleton` declares no \
                      fields, so a fresh allocation is a fully initialized instance"
        )]
        let subject = Value::object(unsafe { NvsObj::new(desc) });
        let info = call(super::nvs_core_reflect_for_object, &mut ctx, &[subject])
            .expect("every object has a description");

        #[expect(
            unsafe_code,
            reason = "the descriptor belongs to the table the context holds"
        )]
        let ordinary = unsafe { nvs_runtime::construct_erased_from(&mut ctx, desc, &[], None) }
            .expect_err("the constructor is `private`, and this site is outside every class");
        let nvs_runtime::Fault::Thrown(ordinary_class, ordinary_said) = ordinary else {
            panic!(
                "an out-of-class construction through a `private` constructor is a catchable throw"
            );
        };

        let none = Value::array(NvsArray::new());
        assert_eq!(
            call(
                super::nvs_core_reflect_class_info_construct,
                &mut ctx,
                &[info, none, OUTSIDE],
            )
            .err(),
            Some(nvs_runtime::THROWN),
            "reflection does not lift the check, so the reflective construction throws too"
        );
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some(ordinary_said.as_ref()),
            "the same sentence, because it is the same check"
        );
        assert_eq!(
            ordinary_class,
            nvs_runtime::ThrownClass::Logic,
            "and the same class, which is what a `catch` in a program sees"
        );
        assert_eq!(
            BUILT.with(std::cell::Cell::get),
            0,
            "neither door ran the constructor it refused"
        );

        let site = inside("Singleton::load");
        let built = call(
            super::nvs_core_reflect_class_info_construct,
            &mut ctx,
            &[info, none, site.1],
        )
        .expect("a `private` constructor is reachable from `Singleton`'s own bodies");
        assert!(built.obj_ptr().is_some(), "what comes back is an instance");
        assert_eq!(
            BUILT.with(std::cell::Cell::get),
            1,
            "and its constructor ran exactly once"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each — `forObject` and \
                      `construct` each handed back a fresh one, and this frame \
                      built the other two — and every call above borrowed them"
        )]
        unsafe {
            built.release();
            none.release();
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
                    param_names: Vec::new(),
                    param_types: Vec::new(),
                    public: true,
                    protected: false,
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
    // covers: Core\Reflect\ClassInfo::set
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

    // How many times `gauge_reading` has run, so a read that answered from the
    // backing slot instead reads as a count that did not move.
    thread_local! {
        static READINGS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }

    /// `Gauge::$reading`'s `get` hook, as `nvs-codegen` would have compiled it:
    /// slot 0 is the receiver, which `call_at` retained, and the answer is a
    /// fresh array — one reference, owned by whoever the read hands it to.
    #[expect(
        unsafe_code,
        reason = "compiled code's own signature, which `call_at` calls through: \
                  one live value and the address of a live `Value` for the \
                  result, neither expressible in the type"
    )]
    unsafe extern "C" fn gauge_reading(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        READINGS.set(READINGS.get() + 1);
        unsafe {
            (*args).release();
            *out = Value::array(NvsArray::new());
        }
        OK
    }

    /// A `Gauge` with three `public` properties, one per route a read can
    /// take: `tags` is a plain slot holding an array, `reading` has a `get`
    /// hook ([`gauge_reading`]) over a slot nothing writes, and `late` is a
    /// `lateinit` slot never written. The array in `tags` is returned too, so
    /// the test can count its owners.
    fn gauge() -> (Ctx, Value, Value) {
        let mut classes = ClassTable::new();
        let id = classes.define("Gauge", &["tags", "reading", "late"], &[]);
        classes.set_public_fields(id, vec![true, true, true]);
        classes.set_defaults(id, vec![(2, nvs_runtime::FieldDefault::Unset)]);
        classes.set_hooks(
            id,
            vec![nvs_runtime::HookRow {
                property: "reading".to_owned(),
                set: false,
                code: (gauge_reading as NvsFn) as *const u8,
                param_tags: 0,
            }],
        );
        let classes = std::sync::Arc::new(classes);
        let desc = classes.desc(id);
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, id));
        let tags = Value::array(NvsArray::new());
        #[expect(
            unsafe_code,
            reason = "the descriptor belongs to the table the context above now \
                      holds for its whole life, and `NvsObj::new` writes every \
                      slot before it hands the allocation back; the slot takes \
                      over the one reference `tags` was built with"
        )]
        let subject = unsafe {
            let object = NvsObj::new(desc);
            object.set_field(0, tags);
            Value::object(object)
        };
        (ctx, subject, tags)
    }

    /// `$gauge->name` through an erased receiver — the read compiled code
    /// reaches, called exactly as `nvs-codegen` emits it. The status and the
    /// borrowed answer come back together.
    #[expect(
        unsafe_code,
        reason = "`nvs_object_slot_get` is compiled code's own entry point: a \
                  context, a value by address and a static byte range, none of \
                  which its signature can bound"
    )]
    fn ordinary_read(ctx: &mut Ctx, subject: Value, name: &str) -> (i32, Value) {
        let mut out = Value::null();
        let status = unsafe {
            nvs_runtime::nvs_object_slot_get(
                std::ptr::from_mut(ctx),
                std::ptr::from_ref(&subject),
                name.as_ptr(),
                name.len(),
                0,
                std::ptr::from_mut(&mut out),
            )
        };
        (status, out)
    }

    /// How many owners the array `value` points at has.
    #[expect(
        unsafe_code,
        reason = "the caller holds a reference to the array, and the handle is \
                  never dropped, so nothing is released"
    )]
    fn owners(value: Value) -> usize {
        let array = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(value.array_ptr().expect("an array"))
        });
        array.refcount()
    }

    /// `rule:security/reflection-enforces-visibility`'s read, asked as an
    /// **agreement** with the ordinary read through a `mixed`, over each route
    /// a read can take. A `get` that read the slot itself would still answer
    /// `tags` correctly — and would answer `reading` with its empty backing
    /// slot and `late` with its storage state, which is what this test is for.
    ///
    /// Ownership is asserted beside each answer, because the two routes hand
    /// back different ones: a slot is borrowed and a hook's return is fresh.
    /// The caller of `get` owns exactly one new reference either way.
    // covers: Core\Reflect\ClassInfo::get
    #[test]
    fn a_reflective_property_read_takes_the_route_an_ordinary_read_takes() {
        READINGS.set(0);
        let (mut ctx, subject, tags) = gauge();
        let info = call(super::nvs_core_reflect_for_object, &mut ctx, &[subject])
            .expect("every object has a description");
        let name = |text: &str| Value::str(NvsStr::new(text.as_bytes()));

        // A plain slot: the same array, with one more owner — the caller.
        let tags_name = name("tags");
        let read = call(
            super::nvs_core_reflect_class_info_get,
            &mut ctx,
            &[info, subject, tags_name, OUTSIDE],
        )
        .expect("`tags` is public");
        assert_eq!(read.array_ptr(), tags.array_ptr(), "the value in the slot");
        assert_eq!(owners(tags), 2, "the object's reference and the caller's");
        #[expect(unsafe_code, reason = "`get` handed back a reference this frame owns")]
        unsafe {
            read.release();
        }
        assert_eq!(
            owners(tags),
            1,
            "and releasing it leaves the object's alone"
        );

        // A hooked property: both reads run the hook, once each.
        let (status, ordinary) = ordinary_read(&mut ctx, subject, "reading");
        assert_eq!(status, OK);
        assert_eq!(READINGS.get(), 1, "the ordinary read runs the `get` hook");
        let reading_name = name("reading");
        let read = call(
            super::nvs_core_reflect_class_info_get,
            &mut ctx,
            &[info, subject, reading_name, OUTSIDE],
        )
        .expect("`reading` is public");
        assert_eq!(READINGS.get(), 2, "and so does the reflective one");
        assert_eq!(
            owners(read),
            1,
            "the hook's fresh array, handed on without a second retain"
        );
        #[expect(
            unsafe_code,
            reason = "each read handed back the hook's fresh reference, and this \
                      frame is the one owner of both"
        )]
        unsafe {
            read.release();
            ordinary.release();
        }

        // A `lateinit` slot never written: both reads throw the same sentence.
        let (status, _) = ordinary_read(&mut ctx, subject, "late");
        assert_eq!(status, nvs_runtime::THROWN);
        let ordinary_said = ctx.take_pending();
        let late_name = name("late");
        assert_eq!(
            call(
                super::nvs_core_reflect_class_info_get,
                &mut ctx,
                &[info, subject, late_name, OUTSIDE],
            )
            .err(),
            Some(nvs_runtime::THROWN),
            "an unwritten slot is a throw, never its storage state"
        );
        assert_eq!(
            ctx.take_pending(),
            ordinary_said,
            "the same sentence, because it is the same read"
        );
        assert!(ordinary_said.is_some_and(|said| said.contains("read before it is written")));

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each — `forObject` handed \
                      back `info`, `gauge` built the subject and this frame made \
                      the names — and every call above borrowed rather than \
                      consumed them"
        )]
        unsafe {
            tags_name.release();
            reading_name.release();
            late_name.release();
            info.release();
            subject.release();
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

    /// The two doors onto a description **agree**: reaching `Vault` by its name
    /// and reaching it through an instance answer the same class, so a
    /// `forClass` that grew a lookup of its own would fail here while still
    /// answering a plausible `ClassInfo`. The other half is the absence: a name
    /// the unit declares no class for is `null`, not a throw.
    // covers: Core\Reflect::forClass
    #[test]
    fn for_class_and_for_object_describe_the_same_class_and_an_unknown_name_is_null() {
        let (mut ctx, subject) = vault();
        let named = |ctx: &mut Ctx, info: Value| {
            let name = call(super::nvs_core_reflect_class_info_name, ctx, &[info])
                .expect("every description has a name");
            name.as_text().expect("a class name is text").to_owned()
        };

        let by_value = call(super::nvs_core_reflect_for_object, &mut ctx, &[subject])
            .expect("every object has a description");
        let vault_name = Value::str(NvsStr::new(b"Vault"));
        let by_name = call(super::nvs_core_reflect_for_class, &mut ctx, &[vault_name])
            .expect("asking for a class by name never throws");
        assert_eq!(
            by_name.tag(),
            Some(Tag::Object),
            "`Vault` is declared, so it is described"
        );
        assert_eq!(named(&mut ctx, by_name), named(&mut ctx, by_value));
        assert_eq!(named(&mut ctx, by_name), "Vault");

        let unknown = Value::str(NvsStr::new(b"Vaults"));
        let absent = call(super::nvs_core_reflect_for_class, &mut ctx, &[unknown])
            .expect("a name no class has is an absence, not a failure");
        assert_eq!(absent.tag(), Some(Tag::Null));
    }

    /// One text-returning `AttributeInfo` member's answer, as an owned string,
    /// with the reference it handed back released.
    fn attribute_text(ctx: &mut Ctx, member: NvsFn, info: Value) -> String {
        let answer = call(member, ctx, &[info]).expect("a text slot always answers");
        let text = answer.as_text().expect("the slot holds text").to_owned();
        #[expect(
            unsafe_code,
            reason = "the member handed back a fresh reference, and this frame is its only owner"
        )]
        unsafe {
            answer.release();
        }
        text
    }

    /// The five `AttributeInfo` members **agree** on one row, built the way
    /// `describe` builds it for `#[Route(path: "/show", limit: 3, cap:
    /// Limits::MAX)]` before a parameter `$id` of `show`: the three site slots
    /// read back as written, and every name `fields` lists is one `field`
    /// recognises. That last half is what tells the two refusals apart — `cap`
    /// is listed and has no folded value, so it must fail with the fold's
    /// sentence, and a name the roster never listed must fail with the
    /// misspelling's. A `field` that searched the values first would pass the
    /// happy rows and swap the two sentences.
    // covers: Core\Reflect\AttributeInfo::name
    // covers: Core\Reflect\AttributeInfo::target
    // covers: Core\Reflect\AttributeInfo::parameter
    // covers: Core\Reflect\AttributeInfo::fields
    // covers: Core\Reflect\AttributeInfo::field
    #[test]
    fn attribute_info_reads_its_site_and_every_listed_field_is_one_field_recognises() {
        let mut ctx = Ctx::buffered();
        let mut names = NvsArray::new();
        let mut values = NvsArray::new();
        for (name, value) in [
            ("path", Value::str(NvsStr::new(b"/show"))),
            ("limit", Value::int(3)),
            ("cap", Value::null()),
        ] {
            names.append(Value::str(NvsStr::new(name.as_bytes())));
            values.append(value);
        }
        let info = crate::instance::build(
            &super::ATTRIBUTE_INFO,
            [
                Value::str(NvsStr::new(b"Route")),
                Value::str(NvsStr::new(b"show")),
                Value::str(NvsStr::new(b"id")),
                Value::array(names),
                Value::array(values),
            ],
        );

        let name = attribute_text(&mut ctx, super::nvs_core_reflect_attribute_info_name, info);
        let target = attribute_text(
            &mut ctx,
            super::nvs_core_reflect_attribute_info_target,
            info,
        );
        let parameter = attribute_text(
            &mut ctx,
            super::nvs_core_reflect_attribute_info_parameter,
            info,
        );
        assert_eq!(
            (name.as_str(), target.as_str(), parameter.as_str()),
            ("Route", "show", "id")
        );

        let listed = call(
            super::nvs_core_reflect_attribute_info_fields,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        #[expect(
            unsafe_code,
            reason = "`fields` answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`fields` answers an array"))
        });
        let fields: Vec<String> = (0..roster.count())
            .map(|at| {
                roster
                    .value_at(at)
                    .and_then(|held| held.as_text().map(str::to_owned))
                    .expect("every field name is text")
            })
            .collect();
        assert_eq!(
            fields,
            ["path", "limit", "cap"],
            "source order, the unfolded one kept"
        );

        let mut read = Vec::new();
        for field in &fields {
            let asked = Value::str(NvsStr::new(field.as_bytes()));
            match call(
                super::nvs_core_reflect_attribute_info_field,
                &mut ctx,
                &[info, asked],
            ) {
                Ok(value) => read.push((field.clone(), value)),
                Err(_) => {
                    let said = ctx.take_pending().unwrap_or_default();
                    assert!(
                        said.contains("does not fold"),
                        "a listed field is never refused as a misspelling: `{field}` said {said:?}"
                    );
                    read.push((field.clone(), Value::null()));
                }
            }
            #[expect(unsafe_code, reason = "the member borrowed the name this frame built")]
            unsafe {
                asked.release();
            }
        }
        assert_eq!(read[0].1.as_text(), Some("/show"));
        assert_eq!(
            read[1].1.as_int(),
            Some(3),
            "an `int` reads back as an `int`"
        );
        assert_eq!(
            read[2].1.tag(),
            Some(Tag::Null),
            "`cap` has no folded value"
        );

        let misspelt = Value::str(NvsStr::new(b"paths"));
        assert_eq!(
            call(
                super::nvs_core_reflect_attribute_info_field,
                &mut ctx,
                &[info, misspelt]
            )
            .err(),
            Some(nvs_runtime::THROWN)
        );
        let said = ctx.take_pending().unwrap_or_default();
        assert!(
            said.contains("no field named `paths`"),
            "a name the roster never listed is the misspelling's refusal, not the fold's: {said:?}"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each: the row it built, the name it \
                      built, the roster `fields` handed back and every value `field` handed back"
        )]
        unsafe {
            misspelt.release();
            listed.release();
            for (_, value) in read {
                value.release();
            }
            info.release();
        }
    }

    /// `attributes` answers one `AttributeInfo` per row the descriptor carries,
    /// in the descriptor's order, and each row reads back the site it was built
    /// from — a roster that sorted, deduplicated or dropped the parameter's row
    /// would still answer a plausible array and fail here on the pairing. A
    /// class that declares no attribute answers an **empty** array, never
    /// `null`, so a caller's `foreach` needs no guard. Which sites a descriptor
    /// carries (own-only) is `nvs_types::layout`'s, and the `.nvst` case that
    /// extends a class pins it end to end.
    // covers: Core\Reflect\ClassInfo::attributes
    #[test]
    fn class_info_attributes_is_one_row_per_attach_site_in_the_descriptors_order() {
        let mut classes = ClassTable::new();
        let page = classes.define("Page", &[] as &[&str], &[]);
        let plain = classes.define("Plain", &[] as &[&str], &[]);
        let sites = [
            ("", "", "Route"),
            ("", "", ""),
            ("title", "", ""),
            ("show", "", "Route"),
            ("show", "id", ""),
        ];
        classes.set_class_attributes(
            page,
            sites
                .iter()
                .map(|&(member, parameter, name)| AttributeDesc {
                    member: member.to_owned(),
                    parameter: parameter.to_owned(),
                    name: name.to_owned(),
                    fields: vec![("path".to_owned(), ConstantValue::Str("/".to_owned()))],
                })
                .collect(),
        );
        let mut ctx = Ctx::buffered();

        let roster_of = |ctx: &mut Ctx, id| {
            #[expect(
                unsafe_code,
                reason = "`classes` outlives this closure's every call, and the table never \
                          moves a descriptor it handed out"
            )]
            let info = super::describe(unsafe { &*classes.desc(id) });
            let roster = call(super::nvs_core_reflect_class_info_attributes, ctx, &[info])
                .expect("the roster always answers");
            #[expect(
                unsafe_code,
                reason = "the description is this frame's alone, and the roster `attributes` \
                          handed back holds its own reference to the array"
            )]
            unsafe {
                info.release();
            }
            roster
        };

        let listed = roster_of(&mut ctx, page);
        #[expect(
            unsafe_code,
            reason = "`attributes` answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let rows = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`attributes` answers an array"))
        });
        let mut read = Vec::new();
        for at in 0..rows.count() {
            let row = rows.value_at(at).expect("every position holds a row");
            read.push((
                attribute_text(&mut ctx, super::nvs_core_reflect_attribute_info_target, row),
                attribute_text(
                    &mut ctx,
                    super::nvs_core_reflect_attribute_info_parameter,
                    row,
                ),
                attribute_text(&mut ctx, super::nvs_core_reflect_attribute_info_name, row),
            ));
        }
        let expected: Vec<(String, String, String)> = sites
            .iter()
            .map(|&(m, p, n)| (m.to_owned(), p.to_owned(), n.to_owned()))
            .collect();
        assert_eq!(
            read, expected,
            "one row per site, in the descriptor's order"
        );

        let empty = roster_of(&mut ctx, plain);
        assert_eq!(
            empty.tag(),
            Some(Tag::Array),
            "no attribute is an empty roster"
        );
        #[expect(
            unsafe_code,
            reason = "`attributes` answered an array this frame owns one reference to"
        )]
        let none = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(empty.array_ptr().expect("an array"))
        });
        assert_eq!(none.count(), 0);

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each roster `attributes` handed back"
        )]
        unsafe {
            listed.release();
            empty.release();
        }
    }

    /// `constant`'s four refusals come in one order, and each pair of adjacent
    /// ones is asked here with a name that could fail both: a name the class
    /// has nothing of is the misspelling's refusal and never visibility's, a
    /// `private secret` constant read from outside is visibility's and never
    /// the qualifier's, and only a site that reaches it hears about `secret`.
    /// The happy half is asserted from both sites so "refuses everything"
    /// cannot pass, and the unfolded value is the last refusal in the chain.
    // covers: Core\Reflect\ClassInfo::constant
    #[test]
    fn class_info_constant_refuses_in_one_order_and_reads_what_the_site_reaches() {
        let mut classes = ClassTable::new();
        let id = classes.define("Vault", &[] as &[&str], &[]);
        let constant = |name: &str, public: bool, secret: bool, value| nvs_runtime::ConstantDesc {
            name: name.to_owned(),
            public,
            protected: false,
            secret,
            value,
        };
        classes.set_class_constants(
            id,
            vec![
                constant("LIMIT", true, false, ConstantValue::Int(3)),
                constant("HIDDEN", false, false, ConstantValue::Int(7)),
                constant("KEY", false, true, ConstantValue::Str("k".to_owned())),
                constant("BLOB", true, false, ConstantValue::Opaque),
            ],
        );
        let classes = std::sync::Arc::new(classes);
        #[expect(
            unsafe_code,
            reason = "the table is held by the context below for the rest of the test, and never \
                      moves a descriptor it handed out"
        )]
        let info = super::describe(unsafe { &*classes.desc(id) });
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, id));
        let (_blob, within) = inside("Vault::open");

        let mut ask = |name: &str, site: Value| -> Result<Value, String> {
            let asked = Value::str(NvsStr::new(name.as_bytes()));
            let answer = call(
                super::nvs_core_reflect_class_info_constant,
                &mut ctx,
                &[info, asked, site],
            );
            #[expect(unsafe_code, reason = "the member borrowed the name this frame built")]
            unsafe {
                asked.release();
            }
            answer.map_err(|_| ctx.take_pending().unwrap_or_default().into_owned())
        };

        assert_eq!(ask("LIMIT", OUTSIDE).ok().and_then(|v| v.as_int()), Some(3));
        assert_eq!(ask("HIDDEN", within).ok().and_then(|v| v.as_int()), Some(7));

        let said = ask("NOPE", OUTSIDE).expect_err("no such constant");
        assert!(said.contains("has no constant named `NOPE`"), "{said:?}");
        let said = ask("HIDDEN", OUTSIDE).expect_err("private, read from outside");
        assert!(said.contains("is not readable from outside"), "{said:?}");
        let said = ask("KEY", OUTSIDE).expect_err("private and secret, read from outside");
        assert!(
            said.contains("is not readable from outside"),
            "visibility is judged before the qualifier: {said:?}"
        );
        let said = ask("KEY", within).expect_err("secret, read from its own class");
        assert!(said.contains("is declared `secret`"), "{said:?}");
        let said = ask("BLOB", OUTSIDE).expect_err("no folded value");
        assert!(said.contains("does not fold"), "{said:?}");

        #[expect(unsafe_code, reason = "this frame owns the description it built")]
        unsafe {
            info.release();
        }
    }

    /// `constants` answers one `ConstantInfo` per constant the descriptor
    /// carries, in the descriptor's order, and each row reads back its own name,
    /// visibility and fold. The four rows differ in exactly one of the two bits
    /// each, so a row answered under its neighbour's bits fails on the pairing.
    /// Both closed kinds read `isPublic` as `false`. Asking twice hands back the
    /// same array, and a class with no constant answers an empty one, never
    /// `null`. Which constants a descriptor carries (own first, then inherited)
    /// is `nvs_types`'s, and the `.nvst` case that extends a class pins it.
    // covers: Core\Reflect\ClassInfo::constants
    #[test]
    fn class_info_constants_is_one_row_per_constant_with_its_own_visibility_and_fold() {
        let mut classes = ClassTable::new();
        let limits = classes.define("Limits", &[] as &[&str], &[]);
        let plain = classes.define("Plain", &[] as &[&str], &[]);
        let rows = [
            ("MAX", true, false, ConstantValue::Int(10)),
            ("TAG", false, true, ConstantValue::Str("t".to_owned())),
            ("KEY", false, false, ConstantValue::Str("k".to_owned())),
            ("ROWS", true, false, ConstantValue::Opaque),
        ];
        classes.set_class_constants(
            limits,
            rows.iter()
                .map(
                    |(name, public, protected, value)| nvs_runtime::ConstantDesc {
                        name: (*name).to_owned(),
                        public: *public,
                        protected: *protected,
                        secret: false,
                        value: value.clone(),
                    },
                )
                .collect(),
        );
        let mut ctx = Ctx::buffered();
        let bit = |ctx: &mut Ctx, member: NvsFn, row: Value| {
            call(member, ctx, &[row])
                .expect("a bit slot always answers")
                .as_bool()
                .expect("the slot holds a bool")
        };

        #[expect(
            unsafe_code,
            reason = "`classes` outlives every use below, and the table never moves a descriptor \
                      it handed out"
        )]
        let info = super::describe(unsafe { &*classes.desc(limits) });
        let listed = call(
            super::nvs_core_reflect_class_info_constants,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        let again = call(
            super::nvs_core_reflect_class_info_constants,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        assert_eq!(
            listed.array_ptr(),
            again.array_ptr(),
            "the roster is the description's own array, handed back rather than rebuilt"
        );
        #[expect(
            unsafe_code,
            reason = "`constants` answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`constants` answers an array"))
        });
        let mut read = Vec::new();
        for at in 0..roster.count() {
            let row = roster.value_at(at).expect("every position holds a row");
            read.push((
                attribute_text(&mut ctx, super::nvs_core_reflect_constant_info_name, row),
                bit(
                    &mut ctx,
                    super::nvs_core_reflect_constant_info_is_public,
                    row,
                ),
                bit(
                    &mut ctx,
                    super::nvs_core_reflect_constant_info_has_value,
                    row,
                ),
            ));
        }
        let expected: Vec<(String, bool, bool)> = rows
            .iter()
            .map(|(name, public, _, value)| {
                ((*name).to_owned(), *public, *value != ConstantValue::Opaque)
            })
            .collect();
        assert_eq!(
            read, expected,
            "one row per constant, in the descriptor's order"
        );

        #[expect(
            unsafe_code,
            reason = "`classes` outlives every use below, and the table never moves a descriptor \
                      it handed out"
        )]
        let bare = super::describe(unsafe { &*classes.desc(plain) });
        let empty = call(
            super::nvs_core_reflect_class_info_constants,
            &mut ctx,
            &[bare],
        )
        .expect("the roster always answers");
        assert_eq!(
            empty.tag(),
            Some(Tag::Array),
            "no constant is an empty roster"
        );
        #[expect(
            unsafe_code,
            reason = "`constants` answered an array this frame owns one reference to"
        )]
        let none = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(empty.array_ptr().expect("an array"))
        });
        assert_eq!(none.count(), 0);

        #[expect(
            unsafe_code,
            reason = "this frame owns the two descriptions and one reference to each of the \
                      three rosters `constants` handed back"
        )]
        unsafe {
            listed.release();
            again.release();
            empty.release();
            info.release();
            bare.release();
        }
    }

    /// `methods` answers one `MethodInfo` per row the descriptor carries, in
    /// name order whatever order the table was handed, and each row reads back
    /// its own name, visibility and arity. The constructor is listed like any
    /// other method, a `private` one is listed with `isPublic` false, and a
    /// synthesized row (a `#` in its name) is not listed at all. Asking twice
    /// hands back the same array, and a class with no method answers an empty
    /// one, never `null`.
    // covers: Core\Reflect\ClassInfo::methods
    #[test]
    fn class_info_methods_is_one_row_per_declared_method_in_name_order() {
        let mut classes = ClassTable::new();
        let router = classes.define("Router", &[] as &[&str], &[]);
        let plain = classes.define("Plain", &[] as &[&str], &[]);
        let row = |name: &str, public: bool, arity: u32| MethodRow {
            name: name.to_owned(),
            code: (vault_open as NvsFn) as *const u8,
            arity,
            param_tags: 0,
            param_names: Vec::new(),
            param_types: Vec::new(),
            public,
            protected: false,
            native: false,
        };
        classes.set_methods(
            router,
            vec![
                row("route", true, 2),
                row("lambda#0", true, 1),
                row(nvs_runtime::object::CONSTRUCTOR, true, 0),
                row("audit", false, 1),
            ],
        );
        let mut ctx = Ctx::buffered();

        #[expect(
            unsafe_code,
            reason = "`classes` outlives every use below, and the table never moves a descriptor \
                      it handed out"
        )]
        let info = super::describe(unsafe { &*classes.desc(router) });
        let listed = call(
            super::nvs_core_reflect_class_info_methods,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        let again = call(
            super::nvs_core_reflect_class_info_methods,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        assert_eq!(
            listed.array_ptr(),
            again.array_ptr(),
            "the roster is the description's own array, handed back rather than rebuilt"
        );
        #[expect(
            unsafe_code,
            reason = "`methods` answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`methods` answers an array"))
        });
        let mut read = Vec::new();
        for at in 0..roster.count() {
            let row = roster.value_at(at).expect("every position holds a row");
            read.push((
                attribute_text(&mut ctx, super::nvs_core_reflect_method_info_name, row),
                call(
                    super::nvs_core_reflect_method_info_is_public,
                    &mut ctx,
                    &[row],
                )
                .expect("a bit slot always answers")
                .as_bool()
                .expect("the slot holds a bool"),
                call(
                    super::nvs_core_reflect_method_info_parameter_count,
                    &mut ctx,
                    &[row],
                )
                .expect("a count slot always answers")
                .as_uint()
                .expect("the slot holds a uint"),
            ));
        }
        assert_eq!(
            read,
            vec![
                ("audit".to_owned(), false, 1),
                ("constructor".to_owned(), true, 0),
                ("route".to_owned(), true, 2),
            ],
            "one row per declared method, in name order, and no synthesized row"
        );

        #[expect(
            unsafe_code,
            reason = "`classes` outlives every use below, and the table never moves a descriptor \
                      it handed out"
        )]
        let bare = super::describe(unsafe { &*classes.desc(plain) });
        let empty = call(
            super::nvs_core_reflect_class_info_methods,
            &mut ctx,
            &[bare],
        )
        .expect("the roster always answers");
        assert_eq!(
            empty.tag(),
            Some(Tag::Array),
            "no method is an empty roster"
        );
        #[expect(
            unsafe_code,
            reason = "`methods` answered an array this frame owns one reference to"
        )]
        let none = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(empty.array_ptr().expect("an array"))
        });
        assert_eq!(none.count(), 0);

        #[expect(
            unsafe_code,
            reason = "this frame owns the two descriptions and one reference to each of the \
                      three rosters `methods` handed back"
        )]
        unsafe {
            listed.release();
            again.release();
            empty.release();
            info.release();
            bare.release();
        }
    }

    /// `hasMethod` asked as an **agreement** with `call`: for every probe name,
    /// it answers `true` exactly when a call from inside the class reaches a
    /// method by that name. The `private` method is the point. It answers
    /// `true`, a call from outside is refused, and a call from inside runs it,
    /// so a refusal can be told from a misspelling. A name differing only in
    /// case, a prefix, a longer name and the empty name all answer `false`.
    // covers: Core\Reflect\ClassInfo::hasMethod
    #[test]
    fn class_info_has_method_agrees_with_what_a_call_from_inside_the_class_reaches() {
        let (mut ctx, subject) = vault();
        let info = call(super::nvs_core_reflect_for_object, &mut ctx, &[subject])
            .expect("every object has a description");
        let none = Value::array(NvsArray::new());
        let site = inside("Vault::check");
        for probe in ["open", "sealed", "OPEN", "ope", "opened", ""] {
            let name = Value::str(NvsStr::new(probe.as_bytes()));
            let declared = call(
                super::nvs_core_reflect_class_info_has_method,
                &mut ctx,
                &[info, name],
            )
            .expect("`hasMethod` answers for any text")
            .as_bool()
            .expect("`hasMethod` answers a bool");
            let reached = call(
                super::nvs_core_reflect_class_info_call,
                &mut ctx,
                &[info, subject, name, none, site.1],
            );
            if reached.is_err() {
                let _refused = ctx.take_pending();
            }
            assert_eq!(
                declared,
                reached.is_ok(),
                "`hasMethod(\"{probe}\")` and a call from inside `Vault` agree"
            );
            #[expect(unsafe_code, reason = "this frame built the name and owns it")]
            unsafe {
                name.release();
            }
        }

        let sealed = Value::str(NvsStr::new(b"sealed"));
        assert_eq!(
            call(
                super::nvs_core_reflect_class_info_call,
                &mut ctx,
                &[info, subject, sealed, none, OUTSIDE],
            )
            .err(),
            Some(nvs_runtime::THROWN),
            "and the same `private` method is refused from outside, while `hasMethod` said `true`"
        );
        assert!(
            ctx.take_pending().is_some(),
            "the refusal left its sentence"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each: `forObject` handed back the \
                      description, and this frame built the other three"
        )]
        unsafe {
            sealed.release();
            none.release();
            info.release();
            subject.release();
        }
    }

    /// `name` reads back exactly the name the declaration wrote: a name that is
    /// another's prefix stays its own, a one-letter name and a long one arrive
    /// whole, and a `private` constant's name is as readable as a `public`
    /// one's. Each call hands back the row's own string with a reference of its
    /// own, so releasing an answer leaves the row readable, and a receiver that
    /// is not an object faults with the member's name rather than reading a slot.
    // covers: Core\Reflect\ConstantInfo::name
    #[test]
    fn constant_info_name_is_the_declared_name_and_survives_its_answer_being_released() {
        let long = "L".repeat(200);
        let names = ["MAX", "MAX_ITEMS", "A", long.as_str()];
        let mut classes = ClassTable::new();
        let limits = classes.define("Limits", &[] as &[&str], &[]);
        classes.set_class_constants(
            limits,
            names
                .iter()
                .enumerate()
                .map(|(at, name)| nvs_runtime::ConstantDesc {
                    name: (*name).to_owned(),
                    public: at % 2 == 0,
                    protected: false,
                    secret: false,
                    value: ConstantValue::Int(1),
                })
                .collect(),
        );
        let mut ctx = Ctx::buffered();

        #[expect(
            unsafe_code,
            reason = "`classes` outlives every use below, and the table never moves a descriptor \
                      it handed out"
        )]
        let info = super::describe(unsafe { &*classes.desc(limits) });
        let listed = call(
            super::nvs_core_reflect_class_info_constants,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        #[expect(
            unsafe_code,
            reason = "`constants` answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`constants` answers an array"))
        });
        assert_eq!(roster.count(), names.len());
        for (at, name) in names.iter().enumerate() {
            let row = roster.value_at(at).expect("every position holds a row");
            let first = call(super::nvs_core_reflect_constant_info_name, &mut ctx, &[row])
                .expect("a text slot always answers");
            let second = call(super::nvs_core_reflect_constant_info_name, &mut ctx, &[row])
                .expect("a text slot always answers");
            assert_eq!(first.as_text(), Some(*name), "row {at} reads its own name");
            assert_eq!(
                first.as_text().map(str::as_ptr),
                second.as_text().map(str::as_ptr),
                "both answers are the row's own string, not two copies"
            );
            #[expect(
                unsafe_code,
                reason = "each call handed back a fresh reference, and this frame is its only owner"
            )]
            unsafe {
                first.release();
                second.release();
            }
            assert_eq!(
                attribute_text(&mut ctx, super::nvs_core_reflect_constant_info_name, row),
                *name,
                "releasing both answers left the row's name in place"
            );
        }

        let refused = call(
            super::nvs_core_reflect_constant_info_name,
            &mut ctx,
            &[Value::int(7)],
        );
        assert!(refused.is_err(), "an integer is no receiver");
        assert!(
            ctx.take_pending()
                .unwrap_or_default()
                .contains(r"Core\Reflect\ConstantInfo::name"),
            "the fault names the member it was raised by"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the description and one reference to the roster"
        )]
        unsafe {
            listed.release();
            info.release();
        }
    }

    /// A `Vault` declaring `rows` as its constants: the context anchored into
    /// its table, its description, the roster `constants` answered, and each
    /// row of that roster beside the name it was declared under. The rows are
    /// borrowed from the roster, so the caller releases the roster and the
    /// description once it is done with them.
    fn vault_of_constants(
        rows: Vec<nvs_runtime::ConstantDesc>,
    ) -> (Ctx, Value, Value, Vec<(String, Value)>) {
        let mut classes = ClassTable::new();
        let id = classes.define("Vault", &[] as &[&str], &[]);
        let names: Vec<String> = rows.iter().map(|row| row.name.clone()).collect();
        classes.set_class_constants(id, rows);
        let classes = std::sync::Arc::new(classes);
        #[expect(
            unsafe_code,
            reason = "the table is held by the context below for the rest of the test, and never \
                      moves a descriptor it handed out"
        )]
        let info = super::describe(unsafe { &*classes.desc(id) });
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, id));
        let listed = call(
            super::nvs_core_reflect_class_info_constants,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        #[expect(
            unsafe_code,
            reason = "`constants` answered an array the caller owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`constants` answers an array"))
        });
        let paired = names
            .into_iter()
            .enumerate()
            .map(|(at, name)| (name, roster.value_at(at).expect("one row per constant")))
            .collect();
        (ctx, info, listed, paired)
    }

    /// `ClassInfo::constant`'s answer for `name` at `site`, as the sentence it
    /// threw when it refused.
    fn read_constant(ctx: &mut Ctx, info: Value, name: &str, site: Value) -> Result<Value, String> {
        let asked = Value::str(NvsStr::new(name.as_bytes()));
        let answer = call(
            super::nvs_core_reflect_class_info_constant,
            ctx,
            &[info, asked, site],
        );
        #[expect(unsafe_code, reason = "the member borrowed the name this frame built")]
        unsafe {
            asked.release();
        }
        answer.map_err(|_| ctx.take_pending().unwrap_or_default().into_owned())
    }

    /// `isPublic` and `ClassInfo::constant` answer one question two ways, and
    /// they agree on every row: `isPublic` is `true` exactly for the constants a
    /// read from outside the class is not refused for visibility.
    /// `rule:security/reflection-enforces-visibility` is what makes that the
    /// promise — a row that said `true` for a constant the read then refused
    /// would send a caller into a refusal the listing said was not there. A
    /// `protected` constant and a `private` one both read `false`, a `public`
    /// one whose value does not fold still reads `true`, and the bit does not
    /// change with the site: a read from inside the class reaches every row.
    // covers: Core\Reflect\ConstantInfo::isPublic
    #[test]
    fn constant_info_is_public_agrees_with_what_a_read_from_outside_reaches() {
        let constant =
            |name: &str, public: bool, protected: bool, value| nvs_runtime::ConstantDesc {
                name: name.to_owned(),
                public,
                protected,
                secret: false,
                value,
            };
        let (mut ctx, info, listed, rows) = vault_of_constants(vec![
            constant("OPEN", true, false, ConstantValue::Int(1)),
            constant("SHARED", false, true, ConstantValue::Int(2)),
            constant("HIDDEN", false, false, ConstantValue::Int(3)),
            constant("BLOB", true, false, ConstantValue::Opaque),
        ]);
        let (_blob, within) = inside("Vault::open");
        let mut bits = Vec::new();
        for (name, row) in &rows {
            let public = call(
                super::nvs_core_reflect_constant_info_is_public,
                &mut ctx,
                &[*row],
            )
            .expect("a bit slot always answers")
            .as_bool()
            .expect("the slot holds a bool");
            let outside = read_constant(&mut ctx, info, name, OUTSIDE);
            let refused_for_visibility = outside
                .as_ref()
                .is_err_and(|said| said.contains("is not readable from outside"));
            assert_eq!(
                public, !refused_for_visibility,
                "`{name}`: `isPublic` said {public}, and the read from outside answered {outside:?}"
            );
            let inner = read_constant(&mut ctx, info, name, within);
            assert!(
                !inner
                    .as_ref()
                    .is_err_and(|said| said.contains("is not readable")),
                "`{name}` is reachable from its own class: {inner:?}"
            );
            bits.push(public);
        }
        assert_eq!(
            bits,
            [true, false, false, true],
            "both closed kinds are `false`, and a value that does not fold is no visibility"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the description and one reference to its roster"
        )]
        unsafe {
            listed.release();
            info.release();
        }
    }

    /// `hasValue` and `ClassInfo::constant` agree on every row a site reaches:
    /// `hasValue` is `true` exactly for the constants the read returns a value
    /// for, and `false` exactly for those it refuses with the fold's sentence.
    /// Each of the four literal kinds folds, and an unfolded value is `false`
    /// whatever its visibility. `hasValue` reports the fold and nothing else, so
    /// a `secret` constant whose value folded reads `true` while the read
    /// refuses it at every site — the qualifier is `constant`'s refusal, which
    /// `rule:security/reflection-enforces-visibility` places at the read rather
    /// than at the listing.
    // covers: Core\Reflect\ConstantInfo::hasValue
    #[test]
    fn constant_info_has_value_agrees_with_what_a_read_returns() {
        let constant = |name: &str, public: bool, secret: bool, value| nvs_runtime::ConstantDesc {
            name: name.to_owned(),
            public,
            protected: false,
            secret,
            value,
        };
        let (mut ctx, info, listed, rows) = vault_of_constants(vec![
            constant("TEXT", true, false, ConstantValue::Str("t".to_owned())),
            constant("COUNT", true, false, ConstantValue::Int(-1)),
            constant("ON", false, false, ConstantValue::Bool(false)),
            constant("RATE", true, false, ConstantValue::Float(0.5)),
            constant("ROWS", true, false, ConstantValue::Opaque),
            constant("MAP", false, false, ConstantValue::Opaque),
            constant("TOKEN", false, true, ConstantValue::Str("s".to_owned())),
        ]);
        let (_blob, within) = inside("Vault::open");
        let mut bits = Vec::new();
        for (name, row) in &rows {
            let has = call(
                super::nvs_core_reflect_constant_info_has_value,
                &mut ctx,
                &[*row],
            )
            .expect("a bit slot always answers")
            .as_bool()
            .expect("the slot holds a bool");
            let read = read_constant(&mut ctx, info, name, within);
            if name == "TOKEN" {
                let said = read.expect_err("a `secret` constant is refused at every site");
                assert!(said.contains("is declared `secret`"), "{said:?}");
            } else {
                assert_eq!(
                    has,
                    read.is_ok(),
                    "`{name}`: `hasValue` said {has}, and the read answered {read:?}"
                );
                match read {
                    Err(said) => assert!(said.contains("does not fold"), "`{name}`: {said:?}"),
                    #[expect(
                        unsafe_code,
                        reason = "`constant` handed back a fresh value, and this frame is its only \
                                  owner"
                    )]
                    Ok(value) => unsafe { value.release() },
                }
            }
            bits.push(has);
        }
        assert_eq!(
            bits,
            [true, true, true, true, false, false, true],
            "the four literal kinds fold, an unfolded value does not at any visibility, and \
             `secret` is not the fold's question"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the description and one reference to its roster"
        )]
        unsafe {
            listed.release();
            info.release();
        }
    }

    /// The member is one call where PHP had fifteen, and that rests on two
    /// properties of the roster rather than on any one answer: every
    /// representation a value can be in has a case, and no two share one. The
    /// sweep is over the runtime's own tag roster rather than over a list
    /// written here, so a new tag fails this rather than silently
    /// answering `Object`.
    // covers: Core\Reflect::typeOf
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

        // The one the enum leaves out, named rather than counted: it is the
        // tag no value carries, and `kind_of`'s own doc says why.
        assert_eq!(
            unrepresented,
            [Tag::Unset],
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

    /// The `*Info` classes of ADR 0019 § 1's roster that this module has not
    /// built, and the whole of what the module doc's gap 1 owns in prose.
    ///
    /// **This list may only shrink**, and the test below is two-sided so that
    /// it cannot go stale in either direction: a name here that *is* registered
    /// fails as loudly as one that is registered nowhere and not here, so
    /// landing a class deletes its line in the same slice. Nothing is added
    /// without deleting this sentence — a roster the record names and this file
    /// silently omits is exactly the drift the gate exists for.
    const NOT_YET_BUILT: &[&str] = &[];

    /// Every `*Info` class ADR 0019 § 1 names is a registered class, or is one
    /// of [`NOT_YET_BUILT`].
    ///
    /// The roster is **read out of the record** rather than copied here,
    /// because § 1 is its one home and a copy is a second one that no run
    /// updates: a class added to the record and to nothing else fails this the
    /// day it is written. What the section spells is a family of names ending
    /// `Info`, qualified or not — `Core\Reflect\ClassInfo` and `MethodInfo`
    /// alike — so the last path segment is what is asked about.
    #[test]
    fn every_reflect_info_class_the_record_names_is_registered() {
        let record = nvs_repo::path("docs/decisions/0019.md");
        let text = std::fs::read_to_string(&record)
            .unwrap_or_else(|err| panic!("{}: {err}", record.display()));
        let section = text
            .split("### 1. ")
            .nth(1)
            .and_then(|rest| rest.split("\n### ").next())
            .expect("`docs/decisions/0019.md` § 1, which is the roster's home");

        let mut named: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for (at, _) in section.match_indices("Info") {
            let start = section[..at]
                .rfind(|c: char| !c.is_alphanumeric())
                .map_or(0, |boundary| boundary + 1);
            let name = &section[start..at + "Info".len()];
            if name.len() > "Info".len() {
                named.insert(name);
            }
        }
        assert!(
            named.len() > 4,
            "§ 1 read as {named:?}, which is not a roster — the section moved or was rewritten, \
             and this test is reading the wrong text rather than finding a gap"
        );

        let mut missing = Vec::new();
        let mut landed = Vec::new();
        for name in named {
            let spelling = format!("Core\\Reflect\\{name}");
            let registered = crate::registry::class(&spelling).is_some();
            let outstanding = NOT_YET_BUILT.contains(&name);
            if !registered && !outstanding {
                missing.push(spelling);
            } else if registered && outstanding {
                landed.push(spelling);
            }
        }
        assert!(
            missing.is_empty(),
            "ADR 0019 § 1 names {} class(es) that `registry::CLASSES` does not hold and \
             `NOT_YET_BUILT` does not own: {}. Register the class, or add it there with the \
             descriptor data it waits on written into this module's gap 1.",
            missing.len(),
            missing.join(", ")
        );
        assert!(
            landed.is_empty(),
            "{} `NOT_YET_BUILT` entr(ies) are registered: {}. Delete these lines — the list is \
             the worklist, and a stale entry is a class nobody will look at again.",
            landed.len(),
            landed.join(", ")
        );
    }

    /// `name` reads back the declared name whole, namespace included, and
    /// `forObject` names the object's own class rather than the parent it
    /// extends. Every name round-trips: `forClass(name())` describes the same
    /// class again. Two calls hand back the description's one string, and a
    /// receiver that is not a description faults naming the member.
    // covers: Core\Reflect\ClassInfo::name
    #[test]
    fn class_info_name_is_the_declared_name_and_round_trips_through_for_class() {
        let mut classes = ClassTable::new();
        let entry = classes.define("Blog\\Entry", &[] as &[&str], &[]);
        let post = classes.define("Blog\\Post", &[] as &[&str], &[entry]);
        let classes = std::sync::Arc::new(classes);
        let post_desc = classes.desc(post);
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, entry));
        #[expect(
            unsafe_code,
            reason = "the descriptor belongs to the table the context above now holds for its \
                      whole life, and `Blog\\Post` declares no fields, so a fresh allocation is a \
                      fully initialized instance"
        )]
        let subject = Value::object(unsafe { NvsObj::new(post_desc) });

        let by_value = call(super::nvs_core_reflect_for_object, &mut ctx, &[subject])
            .expect("every object has a description");
        assert_eq!(
            attribute_text(&mut ctx, super::nvs_core_reflect_class_info_name, by_value),
            "Blog\\Post",
            "the object's own class, not the parent it extends"
        );

        for declared in ["Blog\\Entry", "Blog\\Post"] {
            let asked = Value::str(NvsStr::new(declared.as_bytes()));
            let info = call(super::nvs_core_reflect_for_class, &mut ctx, &[asked])
                .expect("asking for a class by name never throws");
            let first = call(super::nvs_core_reflect_class_info_name, &mut ctx, &[info])
                .expect("every description has a name");
            let second = call(super::nvs_core_reflect_class_info_name, &mut ctx, &[info])
                .expect("every description has a name");
            assert_eq!(first.as_text(), Some(declared), "the name as declared");
            assert_eq!(
                first.as_text().map(str::as_ptr),
                second.as_text().map(str::as_ptr),
                "both answers are the description's own string, not two copies"
            );
            let again = call(super::nvs_core_reflect_for_class, &mut ctx, &[first])
                .expect("asking for a class by name never throws");
            assert_eq!(
                attribute_text(&mut ctx, super::nvs_core_reflect_class_info_name, again),
                declared,
                "`forClass(name())` describes the same class"
            );
            #[expect(
                unsafe_code,
                reason = "this frame built the name, and every call above handed back a fresh \
                          reference this frame is the only owner of"
            )]
            unsafe {
                asked.release();
                first.release();
                second.release();
                info.release();
                again.release();
            }
        }

        let refused = call(
            super::nvs_core_reflect_class_info_name,
            &mut ctx,
            &[Value::int(7)],
        );
        assert!(refused.is_err(), "an integer is no receiver");
        assert!(
            ctx.take_pending()
                .unwrap_or_default()
                .contains(r"Core\Reflect\ClassInfo::name"),
            "the fault names the member it was raised by"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each: `forObject` handed back the \
                      description, and this frame built the subject"
        )]
        unsafe {
            by_value.release();
            subject.release();
        }
    }

    /// A `Ledger` with one property at each level — `code` public, `balance`
    /// private, `notes` protected — an `Audit` extending it with a public
    /// `reviewer`, an unrelated `Other`, and a `Plain` with no property. The
    /// four descriptions come back in that order, beside a context anchored
    /// into the table the way [`vault`] anchors it, because
    /// `readableProperties` asks the context for the described class.
    fn ledger_family() -> (Ctx, [Value; 4]) {
        let mut classes = ClassTable::new();
        let ledger = classes.define("Ledger", &["code", "balance", "notes"], &[]);
        classes.set_public_fields(ledger, vec![true, false, false]);
        classes.set_protected_fields(ledger, vec![false, false, true]);
        let audit = classes.define(
            "Audit",
            &["code", "balance", "notes", "reviewer"],
            &[ledger],
        );
        classes.set_public_fields(audit, vec![true, false, false, true]);
        classes.set_protected_fields(audit, vec![false, false, true, false]);
        let other = classes.define("Other", &["code"], &[]);
        classes.set_public_fields(other, vec![true]);
        let plain = classes.define("Plain", &[] as &[&str], &[]);
        let classes = std::sync::Arc::new(classes);
        #[expect(
            unsafe_code,
            reason = "each descriptor belongs to `classes`, which is alive for the whole call, \
                      and `describe` copies what it reads rather than keeping the descriptor"
        )]
        let described =
            [ledger, audit, other, plain].map(|id| super::describe(unsafe { &*classes.desc(id) }));
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, ledger));
        (ctx, described)
    }

    /// The strings in an array a member handed back, with the reference it
    /// handed back released.
    fn texts(answer: Value) -> Vec<String> {
        #[expect(
            unsafe_code,
            reason = "the member answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let list = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(answer.array_ptr().expect("the member answers an array"))
        });
        let mut out = Vec::new();
        for at in 0..list.count() {
            let item = list.value_at(at).expect("every position holds a value");
            out.push(item.as_text().expect("every entry is text").to_owned());
        }
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference the member handed back"
        )]
        unsafe {
            answer.release();
        }
        out
    }

    /// Every row of a description's property roster, as its name and its
    /// `isPublic` bit.
    fn property_rows(ctx: &mut Ctx, info: Value) -> Vec<(String, bool)> {
        let listed = call(super::nvs_core_reflect_class_info_properties, ctx, &[info])
            .expect("the roster always answers");
        #[expect(
            unsafe_code,
            reason = "`properties` answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`properties` answers an array"))
        });
        let mut read = Vec::new();
        for at in 0..roster.count() {
            let row = roster.value_at(at).expect("every position holds a row");
            read.push((
                attribute_text(ctx, super::nvs_core_reflect_property_info_name, row),
                call(super::nvs_core_reflect_property_info_is_public, ctx, &[row])
                    .expect("a bit slot always answers")
                    .as_bool()
                    .expect("the slot holds a bool"),
            ));
        }
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference `properties` handed back"
        )]
        unsafe {
            listed.release();
        }
        read
    }

    /// `properties` is one row per slot in slot order, inherited slots first.
    /// The `private` and the `protected` property are listed, and both carry
    /// `isPublic` false. Asking twice hands back the same array, and a class
    /// with no property answers an empty one, never `null`.
    // covers: Core\Reflect\ClassInfo::properties
    #[test]
    fn class_info_properties_is_one_row_per_slot_and_lists_the_private_ones() {
        let (mut ctx, [ledger, audit, other, plain]) = ledger_family();
        let owned = |name: &str, public: bool| (name.to_owned(), public);
        assert_eq!(
            property_rows(&mut ctx, ledger),
            vec![
                owned("code", true),
                owned("balance", false),
                owned("notes", false)
            ],
            "every declared property, whatever its visibility"
        );
        assert_eq!(
            property_rows(&mut ctx, audit),
            vec![
                owned("code", true),
                owned("balance", false),
                owned("notes", false),
                owned("reviewer", true)
            ],
            "the inherited slots first, the inherited `private` one included"
        );
        assert_eq!(
            property_rows(&mut ctx, plain),
            Vec::new(),
            "no property is an empty roster"
        );

        let first = call(
            super::nvs_core_reflect_class_info_properties,
            &mut ctx,
            &[ledger],
        )
        .expect("the roster always answers");
        let second = call(
            super::nvs_core_reflect_class_info_properties,
            &mut ctx,
            &[ledger],
        )
        .expect("the roster always answers");
        assert_eq!(
            first.array_ptr(),
            second.array_ptr(),
            "the roster is the description's own array, handed back rather than rebuilt"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the four descriptions and one reference to each of the two \
                      rosters `properties` handed back"
        )]
        unsafe {
            first.release();
            second.release();
            for info in [ledger, audit, other, plain] {
                info.release();
            }
        }
    }

    /// `readableProperties` on one description answers per call site: inside
    /// `Ledger` every property, inside the subclass `Audit` the `protected`
    /// one beside the `public` one, and inside an unrelated class or no class
    /// at all the `public` one alone. Each list is the roster filtered, in
    /// roster order, so the walk never names a property the roster does not.
    // covers: Core\Reflect\ClassInfo::readableProperties
    #[test]
    fn class_info_readable_properties_answers_for_the_site_that_asks() {
        let (mut ctx, [ledger, audit, other, plain]) = ledger_family();
        let own = inside("Ledger::check");
        let sub = inside("Audit::check");
        let unrelated = inside("Other::check");
        let walk = |ctx: &mut Ctx, info: Value, site: Value| {
            texts(
                call(
                    super::nvs_core_reflect_class_info_readable_properties,
                    ctx,
                    &[info, site],
                )
                .expect("the walk always answers"),
            )
        };
        assert_eq!(walk(&mut ctx, ledger, own.1), ["code", "balance", "notes"]);
        assert_eq!(walk(&mut ctx, ledger, sub.1), ["code", "notes"]);
        assert_eq!(walk(&mut ctx, ledger, unrelated.1), ["code"]);
        assert_eq!(walk(&mut ctx, ledger, OUTSIDE), ["code"]);
        assert_eq!(
            walk(&mut ctx, audit, OUTSIDE),
            ["code", "reviewer"],
            "a subclass's description from outside names the public slots alone"
        );
        assert!(
            walk(&mut ctx, plain, own.1).is_empty(),
            "no property, nothing to walk"
        );

        let roster: Vec<String> = property_rows(&mut ctx, ledger)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        for site in [own.1, sub.1, unrelated.1, OUTSIDE] {
            let names = walk(&mut ctx, ledger, site);
            let kept: Vec<&String> = roster.iter().filter(|name| names.contains(name)).collect();
            assert_eq!(
                kept,
                names.iter().collect::<Vec<_>>(),
                "the walk is the roster filtered, in roster order"
            );
        }

        #[expect(unsafe_code, reason = "this frame owns the four descriptions")]
        unsafe {
            for info in [ledger, audit, other, plain] {
                info.release();
            }
        }
    }

    /// `hasProperty` asked as an **agreement** with the walk from inside the
    /// class: for every probe, it answers `true` exactly when that walk names
    /// the property. The `private` property is the point. It answers `true`
    /// while the walk from outside leaves it out, so a hidden property can be
    /// told from a misspelling. A subclass has the properties it inherits.
    // covers: Core\Reflect\ClassInfo::hasProperty
    #[test]
    fn class_info_has_property_agrees_with_the_walk_from_inside_the_class() {
        let (mut ctx, [ledger, audit, other, plain]) = ledger_family();
        let own = inside("Ledger::check");
        let has = |ctx: &mut Ctx, info: Value, probe: &str| {
            let name = Value::str(NvsStr::new(probe.as_bytes()));
            let found = call(
                super::nvs_core_reflect_class_info_has_property,
                ctx,
                &[info, name],
            )
            .expect("`hasProperty` answers for any text")
            .as_bool()
            .expect("`hasProperty` answers a bool");
            #[expect(unsafe_code, reason = "this frame built the name and owns it")]
            unsafe {
                name.release();
            }
            found
        };
        let inside_walk = texts(
            call(
                super::nvs_core_reflect_class_info_readable_properties,
                &mut ctx,
                &[ledger, own.1],
            )
            .expect("the walk always answers"),
        );
        for probe in [
            "code", "balance", "notes", "reviewer", "CODE", "cod", "codes", "",
        ] {
            assert_eq!(
                has(&mut ctx, ledger, probe),
                inside_walk.iter().any(|name| name == probe),
                "`hasProperty(\"{probe}\")` and the walk from inside `Ledger` agree"
            );
        }

        let outside_walk = texts(
            call(
                super::nvs_core_reflect_class_info_readable_properties,
                &mut ctx,
                &[ledger, OUTSIDE],
            )
            .expect("the walk always answers"),
        );
        assert!(
            has(&mut ctx, ledger, "balance") && !outside_walk.iter().any(|name| name == "balance"),
            "the `private` property exists while the walk from outside leaves it out"
        );
        for probe in ["code", "balance", "notes", "reviewer"] {
            assert!(has(&mut ctx, audit, probe), "`Audit` has `{probe}`");
        }
        assert!(
            !has(&mut ctx, other, "balance"),
            "`Other` declares only `code`"
        );
        assert!(!has(&mut ctx, plain, "code"), "`Plain` declares nothing");

        #[expect(unsafe_code, reason = "this frame owns the four descriptions")]
        unsafe {
            for info in [ledger, audit, other, plain] {
                info.release();
            }
        }
    }

    /// A context whose table declares three enums the way `nvs-codegen` records
    /// them — `Shop\Status` (`int`, written out of order, with `Default` and
    /// `Normal` sharing a constant), `Shop\Level` (`uint`) and `Shop\Nothing`
    /// with no case — and their three descriptions in that order. The anchor
    /// class is there because `EnumInfo::of` reads the table through
    /// `Ctx::set_runtime_error_class`'s handle and nothing else.
    fn shop_enums() -> (Ctx, [Value; 3]) {
        let mut classes = ClassTable::new();
        let anchor = classes.define("Shop\\Anchor", &[] as &[&str], &[]);
        let case = |name: &str, value: i128| (name.to_owned(), value);
        classes.define_enum(
            "Shop\\Status",
            false,
            vec![
                case("Shipped", 3),
                case("Normal", 1),
                case("Returned", -1),
                case("Default", 1),
            ],
        );
        classes.define_enum(
            "Shop\\Level",
            true,
            vec![case("Gold", 30), case("Bronze", 10)],
        );
        classes.define_enum("Shop\\Nothing", false, Vec::new());
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(classes), anchor));
        let infos = ["Shop\\Status", "Shop\\Level", "Shop\\Nothing"].map(|name| {
            let asked = Value::str(NvsStr::new(name.as_bytes()));
            let info = call(super::nvs_core_reflect_enum_info_of, &mut ctx, &[asked])
                .expect("asking for an enum by name never throws");
            #[expect(
                unsafe_code,
                reason = "this frame built the name and is its only owner"
            )]
            unsafe {
                asked.release();
            }
            assert_eq!(info.tag(), Some(Tag::Object), "`{name}` is declared");
            info
        });
        (ctx, infos)
    }

    /// `cases` lists every declared case once, ascending by constant and a
    /// shared constant by name, whatever order the declaration wrote them in.
    /// A `uint` enum is listed on the same terms, an enum with no case is an
    /// empty list, two calls hand back the description's one array, and a
    /// receiver that is not a description faults naming the member.
    // covers: Core\Reflect\EnumInfo::cases
    #[test]
    fn enum_info_cases_is_every_case_ascending_by_constant_then_by_name() {
        let (mut ctx, [status, level, nothing]) = shop_enums();
        let listed = |ctx: &mut Ctx, info: Value| {
            texts(
                call(super::nvs_core_reflect_enum_info_cases, ctx, &[info])
                    .expect("the case list always answers"),
            )
        };
        assert_eq!(
            listed(&mut ctx, status),
            ["Returned", "Default", "Normal", "Shipped"],
            "by constant, and `Default` before `Normal` because they share `1`"
        );
        assert_eq!(listed(&mut ctx, level), ["Bronze", "Gold"]);
        assert_eq!(listed(&mut ctx, nothing), Vec::<String>::new());

        let first = call(super::nvs_core_reflect_enum_info_cases, &mut ctx, &[status])
            .expect("the case list always answers");
        let second = call(super::nvs_core_reflect_enum_info_cases, &mut ctx, &[status])
            .expect("the case list always answers");
        assert_eq!(
            first.array_ptr(),
            second.array_ptr(),
            "the list is the description's own array, handed back rather than rebuilt"
        );

        let refused = call(
            super::nvs_core_reflect_enum_info_cases,
            &mut ctx,
            &[Value::int(7)],
        );
        assert!(refused.is_err(), "an integer is no receiver");
        assert!(
            ctx.take_pending()
                .unwrap_or_default()
                .contains(r"Core\Reflect\EnumInfo::cases"),
            "the fault names the member it was raised by"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the three descriptions and one reference to each of the two \
                      lists `cases` handed back"
        )]
        unsafe {
            first.release();
            second.release();
            for info in [status, level, nothing] {
                info.release();
            }
        }
    }

    /// `isUnsigned` is `true` for the `uint` enum alone — the one with no case
    /// included, since the backing is the declaration's and not the cases' —
    /// and a receiver that is not a description faults naming the member.
    // covers: Core\Reflect\EnumInfo::isUnsigned
    #[test]
    fn enum_info_is_unsigned_is_the_declared_backing_and_nothing_else() {
        let (mut ctx, [status, level, nothing]) = shop_enums();
        let unsigned = |ctx: &mut Ctx, info: Value| {
            call(super::nvs_core_reflect_enum_info_is_unsigned, ctx, &[info])
                .expect("a bit slot always answers")
                .as_bool()
                .expect("the slot holds a bool")
        };
        assert!(!unsigned(&mut ctx, status), "`Shop\\Status` is `int`");
        assert!(unsigned(&mut ctx, level), "`Shop\\Level` is `uint`");
        assert!(!unsigned(&mut ctx, nothing), "no case is still `int`");

        let refused = call(
            super::nvs_core_reflect_enum_info_is_unsigned,
            &mut ctx,
            &[Value::bool(true)],
        );
        assert!(refused.is_err(), "a bool is no receiver");
        assert!(
            ctx.take_pending()
                .unwrap_or_default()
                .contains(r"Core\Reflect\EnumInfo::isUnsigned"),
            "the fault names the member it was raised by"
        );

        #[expect(unsafe_code, reason = "this frame owns the three descriptions")]
        unsafe {
            for info in [status, level, nothing] {
                info.release();
            }
        }
    }

    /// `of` answers a description for each declared enum and `null` for a
    /// name the table has no enum under — a class of that name, a name nothing
    /// declares, and any name at all on a context with no table installed —
    /// while an argument that is not text faults naming the member.
    // covers: Core\Reflect\EnumInfo::of
    #[test]
    fn enum_info_of_describes_a_declared_enum_and_is_null_for_anything_else() {
        let (mut ctx, infos) = shop_enums();
        let asked = |ctx: &mut Ctx, name: &str| {
            let text = Value::str(NvsStr::new(name.as_bytes()));
            let answer = call(super::nvs_core_reflect_enum_info_of, ctx, &[text])
                .expect("asking for an enum by name never throws");
            #[expect(
                unsafe_code,
                reason = "this frame built the name and is its only owner"
            )]
            unsafe {
                text.release();
            }
            answer
        };
        for name in ["Shop\\Anchor", "Shop\\Missing", ""] {
            assert_eq!(
                asked(&mut ctx, name).tag(),
                Some(Tag::Null),
                "`{name}` declares no enum"
            );
        }
        let mut bare = Ctx::new(OutputSink::Sink);
        assert_eq!(
            asked(&mut bare, "Shop\\Status").tag(),
            Some(Tag::Null),
            "a context with no table has no enum to describe"
        );

        let refused = call(
            super::nvs_core_reflect_enum_info_of,
            &mut ctx,
            &[Value::int(7)],
        );
        assert!(refused.is_err(), "an integer is no name");
        assert!(
            ctx.take_pending()
                .unwrap_or_default()
                .contains(r"Core\Reflect\EnumInfo::of"),
            "the fault names the member it was raised by"
        );

        #[expect(unsafe_code, reason = "this frame owns the three descriptions")]
        unsafe {
            for info in infos {
                info.release();
            }
        }
    }

    /// `name` is the name the description was asked for, so handing it back
    /// to `of` reaches the same enum, and a receiver that is not a description
    /// faults naming the member.
    // covers: Core\Reflect\EnumInfo::name
    #[test]
    fn enum_info_name_is_the_declared_name_and_reaches_the_same_enum_again() {
        let (mut ctx, infos) = shop_enums();
        for (info, want) in infos
            .iter()
            .zip(["Shop\\Status", "Shop\\Level", "Shop\\Nothing"])
        {
            let name = call(super::nvs_core_reflect_enum_info_name, &mut ctx, &[*info])
                .expect("a text slot always answers");
            assert_eq!(name.as_text(), Some(want));
            let again = call(super::nvs_core_reflect_enum_info_of, &mut ctx, &[name])
                .expect("asking for an enum by name never throws");
            let round = call(super::nvs_core_reflect_enum_info_name, &mut ctx, &[again])
                .expect("a text slot always answers");
            assert_eq!(round.as_text(), Some(want), "the name reaches its own enum");
            #[expect(
                unsafe_code,
                reason = "this frame owns one reference to each of the two names and the \
                          second description"
            )]
            unsafe {
                name.release();
                round.release();
                again.release();
            }
        }

        let refused = call(
            super::nvs_core_reflect_enum_info_name,
            &mut ctx,
            &[Value::null()],
        );
        assert!(refused.is_err(), "null is no receiver");
        assert!(
            ctx.take_pending()
                .unwrap_or_default()
                .contains(r"Core\Reflect\EnumInfo::name"),
            "the fault names the member it was raised by"
        );

        #[expect(unsafe_code, reason = "this frame owns the three descriptions")]
        unsafe {
            for info in infos {
                info.release();
            }
        }
    }

    /// `valueOf` answers each case's constant as an `int` on an `int` enum and
    /// a `uint` on a `uint` one, two cases sharing a constant each answer it,
    /// and a case the enum does not declare is refused with a sentence naming
    /// both the enum and the case.
    // covers: Core\Reflect\EnumInfo::valueOf
    #[test]
    fn enum_info_value_of_is_the_case_constant_at_the_enums_own_backing() {
        let (mut ctx, [status, level, nothing]) = shop_enums();
        let value = |ctx: &mut Ctx, info: Value, case: &str| {
            let text = Value::str(NvsStr::new(case.as_bytes()));
            let answer = call(
                super::nvs_core_reflect_enum_info_value_of,
                ctx,
                &[info, text],
            );
            #[expect(
                unsafe_code,
                reason = "this frame built the case name and is its only owner"
            )]
            unsafe {
                text.release();
            }
            answer
        };
        for (case, want) in [
            ("Shipped", 3),
            ("Normal", 1),
            ("Default", 1),
            ("Returned", -1),
        ] {
            let answer = value(&mut ctx, status, case).expect("a declared case answers");
            assert_eq!(
                answer.as_int(),
                Some(want),
                "`Shop\\Status::{case}` is an `int`"
            );
        }
        for (case, want) in [("Gold", 30), ("Bronze", 10)] {
            let answer = value(&mut ctx, level, case).expect("a declared case answers");
            assert_eq!(
                answer.as_uint(),
                Some(want),
                "`Shop\\Level::{case}` is a `uint`"
            );
        }

        for (info, case) in [(status, "shipped"), (level, "Shipped"), (nothing, "")] {
            assert!(
                value(&mut ctx, info, case).is_err(),
                "`{case}` is not declared"
            );
            let sentence = ctx.take_pending().unwrap_or_default();
            assert!(
                sentence.contains("declares no case") && sentence.contains(case),
                "the refusal names the case: {sentence}"
            );
        }

        #[expect(unsafe_code, reason = "this frame owns the three descriptions")]
        unsafe {
            for info in [status, level, nothing] {
                info.release();
            }
        }
    }

    /// A context whose table declares `Mailer` the way `nvs-codegen` records
    /// it, the description of that class, its method roster, and the roster's
    /// rows in name order: `audit` (`private`, one parameter typed `?string`),
    /// `constructor` (two parameters and no names, as a member the compiler
    /// adds has), `notify` (one named parameter with no type recorded), `send`
    /// (three parameters) and `share` (`protected`, none). The context holds
    /// the table, so the description outlives nothing it was built from.
    fn mailer_methods() -> (Ctx, Value, Value, Vec<Value>) {
        let mut classes = ClassTable::new();
        let mailer = classes.define("Mailer", &[] as &[&str], &[]);
        let row = |name: &str,
                   (public, protected): (bool, bool),
                   arity: u32,
                   names: &[&str],
                   types: &[&str]| MethodRow {
            name: name.to_owned(),
            code: (vault_open as NvsFn) as *const u8,
            arity,
            param_tags: 0,
            param_names: names.iter().map(|name| (*name).to_owned()).collect(),
            param_types: types.iter().map(|ty| (*ty).to_owned()).collect(),
            public,
            protected,
            native: false,
        };
        classes.set_methods(
            mailer,
            vec![
                row(
                    "send",
                    (true, false),
                    3,
                    &["to", "subject", "retries"],
                    &["string", "string", "int"],
                ),
                row("share", (false, true), 0, &[], &[]),
                row(nvs_runtime::object::CONSTRUCTOR, (true, false), 2, &[], &[]),
                row("notify", (true, false), 1, &["channel"], &[]),
                row("audit", (false, false), 1, &["who"], &["?string"]),
            ],
        );
        let classes = std::sync::Arc::new(classes);
        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::clone(&classes), mailer));
        #[expect(
            unsafe_code,
            reason = "the context holds the table for as long as it lives, and the table never \
                      moves a descriptor it handed out"
        )]
        let info = super::describe(unsafe { &*classes.desc(mailer) });
        let listed = call(
            super::nvs_core_reflect_class_info_methods,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        #[expect(
            unsafe_code,
            reason = "`methods` answered an array the caller owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`methods` answers an array"))
        });
        let rows = (0..roster.count())
            .map(|at| roster.value_at(at).expect("every position holds a row"))
            .collect();
        (ctx, info, listed, rows)
    }

    /// Releases what [`mailer_methods`] handed its caller.
    fn release_mailer(info: Value, listed: Value) {
        #[expect(
            unsafe_code,
            reason = "the caller owns the description and one reference to its roster"
        )]
        unsafe {
            listed.release();
            info.release();
        }
    }

    /// The parameters of one `MethodInfo` row, each as its name and its type,
    /// with every reference the members handed back released.
    fn parameter_rows(ctx: &mut Ctx, method: Value) -> Vec<(String, Option<String>)> {
        let listed = call(
            super::nvs_core_reflect_method_info_parameters,
            ctx,
            &[method],
        )
        .expect("a roster slot always answers");
        #[expect(
            unsafe_code,
            reason = "`parameters` answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`parameters` answers an array"))
        });
        let read = (0..roster.count())
            .map(|at| {
                let parameter = roster.value_at(at).expect("every position holds a row");
                let name =
                    attribute_text(ctx, super::nvs_core_reflect_parameter_info_name, parameter);
                let ty = call(
                    super::nvs_core_reflect_parameter_info_type,
                    ctx,
                    &[parameter],
                )
                .expect("a type slot always answers");
                let written = ty.as_text().map(str::to_owned);
                #[expect(
                    unsafe_code,
                    reason = "the member handed back a fresh reference, and this frame is its \
                              only owner"
                )]
                unsafe {
                    ty.release();
                }
                (name, written)
            })
            .collect();
        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to the roster `parameters` handed back"
        )]
        unsafe {
            listed.release();
        }
        read
    }

    /// `name` reads back each declared name, the constructor's as
    /// `constructor`, and hands back a reference of its own each time, so
    /// releasing every answer leaves every row readable. A receiver that is not
    /// an object faults naming the member rather than reading a slot.
    // covers: Core\Reflect\MethodInfo::name
    #[test]
    fn method_info_name_is_the_declared_name_and_survives_its_answer_being_released() {
        let (mut ctx, info, listed, rows) = mailer_methods();
        let read = |ctx: &mut Ctx| -> Vec<String> {
            rows.iter()
                .map(|row| attribute_text(ctx, super::nvs_core_reflect_method_info_name, *row))
                .collect()
        };
        let first = read(&mut ctx);
        assert_eq!(first, ["audit", "constructor", "notify", "send", "share"]);
        assert_eq!(
            read(&mut ctx),
            first,
            "a released answer left its row whole"
        );

        let refused = call(
            super::nvs_core_reflect_method_info_name,
            &mut ctx,
            &[Value::int(7)],
        );
        assert!(refused.is_err(), "an integer is no receiver");
        assert!(
            ctx.take_pending()
                .unwrap_or_default()
                .contains(r"Core\Reflect\MethodInfo::name"),
            "the fault names the member it was raised by"
        );
        release_mailer(info, listed);
    }

    /// `isPublic` is `true` for exactly the `public` rows: a `protected`
    /// method and a `private` one both read `false` and are still listed, and
    /// the constructor the compiler adds reads `true`. Asking again reads the
    /// same bits.
    // covers: Core\Reflect\MethodInfo::isPublic
    #[test]
    fn method_info_is_public_is_true_for_the_public_rows_alone() {
        let (mut ctx, info, listed, rows) = mailer_methods();
        let bits = |ctx: &mut Ctx| -> Vec<bool> {
            rows.iter()
                .map(|row| {
                    call(super::nvs_core_reflect_method_info_is_public, ctx, &[*row])
                        .expect("a bit slot always answers")
                        .as_bool()
                        .expect("the slot holds a bool")
                })
                .collect()
        };
        let first = bits(&mut ctx);
        assert_eq!(
            first,
            [false, true, true, true, false],
            "`audit` is private and `share` protected, and both are listed"
        );
        assert_eq!(bits(&mut ctx), first);
        release_mailer(info, listed);
    }

    /// `parameterCount` is the declared arity, as a `uint`, for every row — the
    /// constructor included, whose parameter list is empty because nothing
    /// wrote one. The count and the list agree on every row that has names.
    // covers: Core\Reflect\MethodInfo::parameterCount
    #[test]
    fn method_info_parameter_count_is_the_arity_even_where_no_names_were_written() {
        let (mut ctx, info, listed, rows) = mailer_methods();
        let mut counts = Vec::new();
        for row in &rows {
            let count = call(
                super::nvs_core_reflect_method_info_parameter_count,
                &mut ctx,
                &[*row],
            )
            .expect("a count slot always answers");
            assert_eq!(count.tag(), Some(Tag::Uint), "the count is a `uint`");
            counts.push(count.as_uint().expect("the slot holds a uint"));
        }
        assert_eq!(counts, [1, 2, 1, 3, 0]);
        let listed_lengths: Vec<u64> = rows
            .iter()
            .map(|row| parameter_rows(&mut ctx, *row).len() as u64)
            .collect();
        assert_eq!(
            listed_lengths,
            [1, 0, 1, 3, 0],
            "only the constructor's list is empty while its count is not"
        );
        release_mailer(info, listed);
    }

    /// `parameters` is one `ParameterInfo` per declared parameter in declaration
    /// order, an empty array for a method the compiler adds, and the row's own
    /// array each time it is asked rather than a new one.
    // covers: Core\Reflect\MethodInfo::parameters
    #[test]
    fn method_info_parameters_are_in_declaration_order_and_empty_where_nothing_was_written() {
        let (mut ctx, info, listed, rows) = mailer_methods();
        let names: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                parameter_rows(&mut ctx, *row)
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect()
            })
            .collect();
        assert_eq!(
            names,
            [
                vec!["who"],
                vec![],
                vec!["channel"],
                vec!["to", "subject", "retries"],
                vec![],
            ]
        );

        let first = call(
            super::nvs_core_reflect_method_info_parameters,
            &mut ctx,
            &[rows[3]],
        )
        .expect("a roster slot always answers");
        let second = call(
            super::nvs_core_reflect_method_info_parameters,
            &mut ctx,
            &[rows[3]],
        )
        .expect("a roster slot always answers");
        assert_eq!(
            first.array_ptr(),
            second.array_ptr(),
            "the list is the row's own array, handed back rather than rebuilt"
        );
        #[expect(unsafe_code, reason = "this frame owns one reference to each answer")]
        unsafe {
            first.release();
            second.release();
        }
        release_mailer(info, listed);
    }

    /// `ParameterInfo::name` is the declared name without its `$`, and every
    /// parameter of a method answers its own.
    // covers: Core\Reflect\ParameterInfo::name
    #[test]
    fn parameter_info_name_is_the_declared_name_in_order() {
        let (mut ctx, info, listed, rows) = mailer_methods();
        let send: Vec<String> = parameter_rows(&mut ctx, rows[3])
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(send, ["to", "subject", "retries"]);
        assert!(send.iter().all(|name| !name.starts_with('$')));
        release_mailer(info, listed);
    }

    /// `ParameterInfo::type` is the type as the declaration writes it — the
    /// `?` of a nullable type included — and `null` for a parameter whose row
    /// recorded no type.
    // covers: Core\Reflect\ParameterInfo::type
    #[test]
    fn parameter_info_type_is_the_written_type_or_null_where_none_was_recorded() {
        let (mut ctx, info, listed, rows) = mailer_methods();
        let typed = |ctx: &mut Ctx, row: Value| -> Vec<Option<String>> {
            parameter_rows(ctx, row)
                .into_iter()
                .map(|(_, ty)| ty)
                .collect()
        };
        assert_eq!(typed(&mut ctx, rows[0]), [Some("?string".to_owned())]);
        assert_eq!(
            typed(&mut ctx, rows[2]),
            [None],
            "`notify` recorded no type"
        );
        assert_eq!(
            typed(&mut ctx, rows[3]),
            ["string", "string", "int"].map(|ty| Some(ty.to_owned()))
        );
        release_mailer(info, listed);
    }

    /// `PropertyInfo::name` is the declared name without its `$`, every
    /// inherited row included, and hands back a reference of its own each
    /// time, so releasing every answer leaves every row readable. A receiver
    /// that is not an object faults naming the member.
    // covers: Core\Reflect\PropertyInfo::name
    #[test]
    fn property_info_name_is_the_declared_name_and_survives_its_answer_being_released() {
        let (mut ctx, described) = ledger_family();
        let names = |ctx: &mut Ctx| -> Vec<String> {
            property_rows(ctx, described[1])
                .into_iter()
                .map(|(name, _)| name)
                .collect()
        };
        let first = names(&mut ctx);
        assert_eq!(first, ["code", "balance", "notes", "reviewer"]);
        assert!(first.iter().all(|name| !name.starts_with('$')));
        assert_eq!(
            names(&mut ctx),
            first,
            "a released answer left its row whole"
        );

        let refused = call(
            super::nvs_core_reflect_property_info_name,
            &mut ctx,
            &[Value::int(7)],
        );
        assert!(refused.is_err(), "an integer is no receiver");
        assert!(
            ctx.take_pending()
                .unwrap_or_default()
                .contains(r"Core\Reflect\PropertyInfo::name"),
            "the fault names the member it was raised by"
        );
        #[expect(unsafe_code, reason = "this frame owns the four descriptions")]
        unsafe {
            for info in described {
                info.release();
            }
        }
    }

    /// `PropertyInfo::isPublic` is `true` for exactly the `public` rows: the
    /// `private` and the `protected` property read `false` and are still
    /// listed, in the subclass as in the class declaring them.
    // covers: Core\Reflect\PropertyInfo::isPublic
    #[test]
    fn property_info_is_public_is_true_for_the_public_rows_alone() {
        let (mut ctx, described) = ledger_family();
        let bits = |ctx: &mut Ctx, info: Value| -> Vec<bool> {
            property_rows(ctx, info)
                .into_iter()
                .map(|(_, public)| public)
                .collect()
        };
        assert_eq!(bits(&mut ctx, described[0]), [true, false, false]);
        assert_eq!(
            bits(&mut ctx, described[1]),
            [true, false, false, true],
            "the inherited `private` and `protected` rows stay closed in the subclass"
        );
        assert_eq!(bits(&mut ctx, described[2]), [true]);
        #[expect(unsafe_code, reason = "this frame owns the four descriptions")]
        unsafe {
            for info in described {
                info.release();
            }
        }
    }

    /// `PropertyInfo::type` is the type as the declaration writes it — the `?`
    /// of a nullable type and an `array`'s element type included — and `null`
    /// for a slot whose class recorded no type, as the built-in error classes'
    /// slots are.
    // covers: Core\Reflect\PropertyInfo::type
    #[test]
    fn property_info_type_is_the_written_type_or_null_where_none_was_recorded() {
        let mut classes = ClassTable::new();
        let order = classes.define("Order", &["id", "total", "tags", "note"], &[]);
        classes.set_public_fields(order, vec![true, true, false, true]);
        classes.set_field_types(
            order,
            ["int", "?float", "array<string>", ""]
                .map(str::to_owned)
                .to_vec(),
        );
        let classes = std::sync::Arc::new(classes);
        #[expect(
            unsafe_code,
            reason = "the descriptor belongs to `classes`, which is alive for the whole call, and \
                      `describe` copies what it reads rather than keeping the descriptor"
        )]
        let info = super::describe(unsafe { &*classes.desc(order) });
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(classes, order));

        let listed = call(
            super::nvs_core_reflect_class_info_properties,
            &mut ctx,
            &[info],
        )
        .expect("the roster always answers");
        #[expect(
            unsafe_code,
            reason = "`properties` answered an array this frame owns one reference to, and the \
                      `ManuallyDrop` borrows it without taking that reference over"
        )]
        let roster = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(listed.array_ptr().expect("`properties` answers an array"))
        });
        let types: Vec<Option<String>> = (0..roster.count())
            .map(|at| {
                let row = roster.value_at(at).expect("every position holds a row");
                let ty = call(super::nvs_core_reflect_property_info_type, &mut ctx, &[row])
                    .expect("a type slot always answers");
                let written = ty.as_text().map(str::to_owned);
                #[expect(
                    unsafe_code,
                    reason = "the member handed back a fresh reference, and this frame is its \
                              only owner"
                )]
                unsafe {
                    ty.release();
                }
                written
            })
            .collect();
        assert_eq!(
            types,
            [Some("int"), Some("?float"), Some("array<string>"), None]
                .map(|ty| ty.map(str::to_owned)),
            "the private `tags` row carries its type, and `note` recorded none"
        );
        #[expect(
            unsafe_code,
            reason = "this frame owns the description and one reference to the roster"
        )]
        unsafe {
            listed.release();
            info.release();
        }
    }
}
