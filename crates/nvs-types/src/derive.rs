//! `rule:core-classes/derive-attribute`'s derive pass:
//! which classes carry `#[Json\Derive]` or `#[Db\Derive]`, which of their
//! properties are fields, and what wire key each field has.
//!
//! # One pass, two formats
//!
//! [`Format`] is the only thing anything below branches on, and it is checked
//! for each of its two values over the same class. `rule:core-classes/derive-attribute` states §§ 2, 3, 5
//! and 7 once, for "a derived codec", so they are written here once and asked
//! of both: the two formats differ in their **type map** — JSON's is § 2's
//! reachable set, a row's is `rule:core-classes/db-column-types`'s — in the attribute pair that names them, and in nothing else. Two
//! passes that agreed today would be two passes that disagree the first time
//! one of those sections is amended.
//!
//! # The nominal match, and why it lives here
//!
//! `rule:core-classes/derive-attribute` makes a **compiler-recognized** attribute the one thing matched
//! by name rather than by shape: the compiler acts on `#[Json\Derive]` only
//! when that `Name` *resolves* — through the ordinary namespace and `use` rules
//! — to one of a closed, `Core`-owned list. Resolution is exactly
//! [`nvs_hir::resolve_ref`], and the active namespace and import set are things
//! only [`crate::check`]'s walk holds, so this pass runs from that walk rather
//! than as a second traversal of its own. [`crate::Ctx`] carries both.
//!
//! [`ATTRIBUTES`] is that closed list. Nothing else is ever matched by name;
//! `Core\Attributes::get<T>`/`::all<T>` retrieval stays structural
//! (`rule:attributes/structural-retrieval`), and a userland `type Derive = {};` resolves to a different `QName` and
//! generates nothing.
//!
//! # What the pass produces
//!
//! One [`DerivedCodec`] per deriving class, recorded into
//! [`crate::ExprTypeTable`] — the table this crate already publishes for
//! `nvs-ir` to read back. It carries the *property* name rather than a slot
//! index, because the slot order is
//! [`crate::layout::ClassLayout`]'s and that table is built after checking; the
//! two are joined in `nvs_ir::lower::lower_file`, which holds both. Each field
//! also carries the declared type a decoder checks against, erased to
//! [`nvs_stdlib::CodecTy`], and the *constructor position* it fills — `rule:core-classes/derive-field-list`'s "a decode is an ordinary `new`" resolved to an index, so that nothing
//! below this line looks a parameter up by name.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, ClassDecl, ClassMemberKind, ExprKind, Modifier, ObjectLiteralField,
    Param, PropertyMember,
};

use nvs_stdlib::{CodecElement, CodecTy, EnumCases};

use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// `rule:core-classes/derive-attribute`'s closed, `Core`-owned list of compiler-recognized attribute
/// names, each already fully qualified.
///
/// A `Name` written at an attribute site is resolved with
/// [`nvs_hir::resolve_ref`] and compared against these — so `#[Core\Json\Derive]`
/// and a `use Core\Json;`d `#[Json\Derive]` are the same attribute, and no
/// spelling of a userland name is any of them. Extended, never widened, and
/// each entry owes its own argued ADR section.
///
/// A name on this roster names no shape, so [`crate::attributes`]'s `rule:attributes/attach-sites-and-forms` rule does not apply to it and what its payload may hold is the
/// recognizing pass's own question: `#[Json\Derive]`/`#[Json\Field]` are this
/// module's, `#[Test]` is [`crate::testing`]'s, `#[Command]`/`#[Option]` are
/// [`crate::commands`]', `#[Route]`/`#[Query]`/`#[Access]` are
/// [`crate::routes`]', and `#[Core\Path]` is [`crate::paths`]'.
pub const ATTRIBUTES: &[&str] = [
    DERIVE, FIELD, DB_DERIVE, DB_FIELD, TEST, FIXTURE, TEST_WITH, COMMAND, OPTION, ROUTE, QUERY,
    ACCESS, API, PATH,
]
.as_slice();

/// One compiler attribute's reference card: what the attribute is for, in one
/// sentence, the declaration it is written above, and the shape its payload
/// satisfies, spelled the way a userland attribute declares one
/// (`rule:attributes/attach-sites-and-forms`).
///
/// The registry side of [`ATTRIBUTES`]. A recognized name is matched nominally
/// and its payload is checked by the pass that owns it, so nothing in this
/// crate holds that payload as a shape a reader could look at — this table is
/// where an editor's hover, a stub file and `nvs stubs` read one.
/// `every_attribute_carries_a_card` holds it against [`ATTRIBUTES`] and, for
/// every attribute whose pass declares an option roster, against that roster,
/// so a field cannot be documented here that no pass admits or admitted and
/// left out.
///
/// `short` and `site` are read by somebody who looked the attribute up in an
/// editor and has never seen this repository, so each is written to
/// `AGENTS.md` § *Text an end user reads*.
#[derive(Clone, Copy, Debug)]
pub struct AttributeDoc {
    /// The fully-qualified name, spelled as [`ATTRIBUTES`] spells it.
    pub name: &'static str,
    /// The payload's fields, in the order the owning rule writes them.
    pub fields: &'static [AttributeField],
    /// What the attribute does, in one or two sentences.
    pub short: &'static str,
    /// The declaration it is written above — `a method`, `a class`, `a
    /// parameter` — as the object of "written above".
    pub site: &'static str,
}

/// One field of a compiler attribute's payload.
#[derive(Clone, Copy, Debug)]
pub struct AttributeField {
    /// The field's name, as a payload writes it on the left of the `:`.
    pub name: &'static str,
    /// Its type, spelled as a program writes it — `string`, `bool`,
    /// `Core\Http\Method`, `mixed`.
    pub ty: &'static str,
    /// Whether a payload must write it.
    pub required: bool,
}

impl AttributeDoc {
    /// The last segment of [`Self::name`] — `Route`, `Derive`.
    #[must_use]
    pub fn short_name(&self) -> &'static str {
        self.name.rsplit('\\').next().unwrap_or(self.name)
    }

    /// The payload as a shape type — `{path: string, method: Core\Http\Method,
    /// name?: string}` — which is what a stub declares the attribute as, and
    /// `object` for an attribute whose fields are not fixed.
    #[must_use]
    pub fn shape(&self) -> String {
        if self.fields.is_empty() {
            return "{}".to_owned();
        }
        let fields: Vec<String> = self
            .fields
            .iter()
            .map(|field| {
                let optional = if field.required { "" } else { "?" };
                format!("{}{optional}: {}", field.name, field.ty)
            })
            .collect();
        format!("{{{}}}", fields.join(", "))
    }

    /// The attribute as a program writes it, with each field at its type —
    /// `#[Core\Route(path: string, method: Core\Http\Method, name?: string)]`,
    /// and `#[Core\Query]` for one that carries nothing.
    #[must_use]
    pub fn spelled(&self) -> String {
        if self.fields.is_empty() {
            return format!("#[{}]", self.name);
        }
        let fields: Vec<String> = self
            .fields
            .iter()
            .map(|field| {
                let optional = if field.required { "" } else { "?" };
                format!("{}{optional}: {}", field.name, field.ty)
            })
            .collect();
        format!("#[{}({})]", self.name, fields.join(", "))
    }
}

/// The card of the attribute `name` names, fully qualified, or `None` for a
/// name that is not one of [`ATTRIBUTES`].
#[must_use]
pub fn attribute_doc(name: &str) -> Option<&'static AttributeDoc> {
    ATTRIBUTE_DOCS.iter().find(|doc| doc.name == name)
}

/// One [`AttributeDoc`] per entry of [`ATTRIBUTES`], in that roster's order.
pub const ATTRIBUTE_DOCS: &[AttributeDoc] = &[
    AttributeDoc {
        name: DERIVE,
        fields: &[],
        short: "Generates the JSON encoder and decoder for this class. Every public property \
                becomes a key.",
        site: "a class",
    },
    AttributeDoc {
        name: FIELD,
        fields: &[
            AttributeField {
                name: "name",
                ty: "string",
                required: false,
            },
            AttributeField {
                name: "skip",
                ty: "bool",
                required: false,
            },
        ],
        short: "Changes how one property is written in JSON: the key it is written under, or \
                whether it is left out.",
        site: "a property",
    },
    AttributeDoc {
        name: DB_DERIVE,
        fields: &[],
        short: "Generates the row decoder for this class, so a query can return instances of it.",
        site: "a class",
    },
    AttributeDoc {
        name: DB_FIELD,
        fields: &[
            AttributeField {
                name: "name",
                ty: "string",
                required: false,
            },
            AttributeField {
                name: "skip",
                ty: "bool",
                required: false,
            },
        ],
        short: "Changes how one property is read from a row: the column it is read from, or \
                whether it is left out.",
        site: "a property",
    },
    AttributeDoc {
        name: TEST,
        fields: &[
            AttributeField {
                name: "skip",
                ty: "string",
                required: false,
            },
            AttributeField {
                name: "at",
                ty: "string",
                required: false,
            },
            AttributeField {
                name: "seed",
                ty: "int",
                required: false,
            },
            AttributeField {
                name: "db",
                ty: "string",
                required: false,
            },
            AttributeField {
                name: "server",
                ty: "bool",
                required: false,
            },
            AttributeField {
                name: "retries",
                ty: "int",
                required: false,
            },
            AttributeField {
                name: "because",
                ty: "string",
                required: false,
            },
        ],
        short: "Says that this method is a test. `nvs test` runs it and reports the result.",
        site: "a method",
    },
    AttributeDoc {
        name: FIXTURE,
        fields: &[],
        short: "Says that this static method builds a value. A test receives that value by \
                declaring a parameter of the method's return type.",
        site: "a static method",
    },
    AttributeDoc {
        name: TEST_WITH,
        fields: &[],
        short: "Gives one row of arguments to the test method below it. The fields are the \
                method's own parameters, and the method runs once per row.",
        site: "a test method",
    },
    AttributeDoc {
        name: COMMAND,
        fields: &[
            AttributeField {
                name: "name",
                ty: "string",
                required: true,
            },
            AttributeField {
                name: "about",
                ty: "string",
                required: false,
            },
        ],
        short: "Says that this static method is a command line command. `name` is what a user \
                types, and `about` is the sentence `--help` prints.",
        site: "a static method",
    },
    AttributeDoc {
        name: OPTION,
        fields: &[
            AttributeField {
                name: "short",
                ty: "string",
                required: false,
            },
            AttributeField {
                name: "long",
                ty: "string",
                required: false,
            },
            AttributeField {
                name: "about",
                ty: "string",
                required: false,
            },
        ],
        short: "Says that this parameter is a command line option, written as `--name value`. \
                A parameter without it is a positional argument.",
        site: "a parameter of a command",
    },
    AttributeDoc {
        name: ROUTE,
        fields: &[
            AttributeField {
                name: "path",
                ty: "string",
                required: true,
            },
            AttributeField {
                name: "method",
                ty: r"Core\Http\Method",
                required: true,
            },
            AttributeField {
                name: "name",
                ty: "string",
                required: false,
            },
        ],
        short: "Declares the HTTP path and method this method handles. Write it twice to \
                handle two methods.",
        site: "a method",
    },
    AttributeDoc {
        name: QUERY,
        fields: &[],
        short: "Says that this parameter is read from the query string, under the parameter's \
                own name and converted to its type.",
        site: "a parameter of a route",
    },
    AttributeDoc {
        name: ACCESS,
        fields: &[
            AttributeField {
                name: "allow",
                ty: "mixed",
                required: true,
            },
            AttributeField {
                name: "csrf",
                ty: "bool",
                required: false,
            },
        ],
        short: "Says who may call this route. Every route needs one, and `csrf: false` turns \
                the CSRF check off for this route.",
        site: "a method with a route",
    },
    AttributeDoc {
        name: API,
        fields: &[
            AttributeField {
                name: "tags",
                ty: "mixed",
                required: false,
            },
            AttributeField {
                name: "errors",
                ty: "mixed",
                required: false,
            },
            AttributeField {
                name: "security",
                ty: "mixed",
                required: false,
            },
            AttributeField {
                name: "example",
                ty: "mixed",
                required: false,
            },
        ],
        short: "Adds documentation to a route for the generated OpenAPI file. It changes nothing \
                about what the route does.",
        site: "a method with a route",
    },
    AttributeDoc {
        name: PATH,
        fields: &[],
        short: "Says that this parameter is a file path. A relative path written as a string \
                literal in a call is joined to the folder of the file that contains the call.",
        site: "a `string` parameter",
    },
];

/// `#[Json\Derive]` — `rule:core-classes/derive-attribute`'s opt-in, on a class.
pub const DERIVE: &str = r"Core\Json\Derive";

/// `#[Json\Field(name?: string, skip?: bool)]` — `rule:core-classes/derive-field-list`'s per-field
/// override, on a property.
pub const FIELD: &str = r"Core\Json\Field";

/// `#[Db\Derive]` — `rule:core-classes/derive-attribute`'s opt-in again, on a class, for the row half
/// of the same table. It generates `Core\Db\Codec`'s `fromRow` and nothing
/// else: § 7 makes this format one-directional, because a write is
/// `rule:core-classes/db-one-api`'s explicit statement plus
/// bound parameters and a generated `INSERT` is the ORM that ADR settled
/// against.
pub const DB_DERIVE: &str = r"Core\Db\Derive";

/// `#[Db\Field(name?: string, skip?: bool)]` — `rule:core-classes/derive-field-list`'s per-field
/// override, on a property, and a *second* attribute rather than a spelling
/// shared with [`FIELD`]: a JSON key and a column name are independently
/// chosen, so forcing them equal would need an escape hatch immediately.
pub const DB_FIELD: &str = r"Core\Db\Field";

/// `#[Test(skip?: string, …)]` — `rule:testing/test-attribute`'s marker, on a method. It is
/// the class the assertions are members of, so the `use Core\Test;` that lets
/// a test body write `Test::assertEquals(…)` is the same one that places the
/// attribute; [`crate::testing`] owns the payload.
pub const TEST: &str = r"Core\Test";

/// `#[Fixture]` — `rule:testing/fixtures`'s marker, on a `static` method whose return
/// type is what a test asks for by declaring a parameter of it. It sits in
/// the `Core\Test` namespace beside [`crate::error_lib`]'s `Core\Test\Failure`
/// rather than being a second segment of the class itself, because it names
/// no member of anything: it is a recognized name and nothing else, so a file
/// that writes it bare places it with `use Core\Test\Fixture;`.
/// [`crate::testing`] owns what it may carry, which is nothing.
pub const FIXTURE: &str = r"Core\Test\Fixture";

/// `#[TestWith(...)]` — `rule:testing/data-rows`'s data row, on a `#[Test]` method. It
/// sits in the `Core\Test` namespace beside [`FIXTURE`] and for that entry's
/// reason exactly, and it keeps the ADR's own spelling rather than the
/// shorter `With` a namespace would allow: `#[TestWith]` is what § 9 writes,
/// and a file that spells it bare places it with `use Core\Test\TestWith;`.
///
/// It is on this roster rather than being an `rule:attributes/attach-sites-and-forms` shape alias because
/// the shape it is checked against is not written anywhere: it is the
/// *parameter list* of the method it is attached to, which only
/// [`crate::testing::check_class_tests`] holds. That module owns what a row
/// may carry.
pub const TEST_WITH: &str = r"Core\Test\TestWith";

/// `#[Command(name: string, about?: string)]` — `rule:tooling/commands-are-compiled`'s marker, on a
/// `static` method. It is the same class the entry point `Core\Command::run`
/// is a member of, so the `use Core\Command;` that lets a program spell the
/// attribute bare is the one that reaches the runner too;
/// [`crate::commands`] owns the payload.
pub const COMMAND: &str = r"Core\Command";

/// `#[Option(short?: string, long?: string, about?: string)]` — `rule:tooling/commands-are-compiled`'s
/// per-parameter marker, and the one sentence that decides what a parameter is:
/// a parameter is a positional argument unless it carries this, with no
/// inference from defaults or types. [`crate::commands`] owns the payload.
pub const OPTION: &str = r"Core\Option";

/// `#[Route(path: string, method: Core\Http\Method, name?: string)]` — `rule:routing/route-attribute`
/// 's route declaration, on a method, and repeatable (`rule:attributes/repeatable`) so one
/// method serves two verbs. It names no member of anything — the table is read
/// back through the separate `Core\Router` class — so a file that spells it
/// bare places it with `use Core\Route;`, and importing the router instead
/// imports a different name. [`crate::routes`] owns the payload.
pub const ROUTE: &str = r"Core\Route";

/// `#[Query]` — `rule:routing/a-query-parameter-is-declared-like-a-capture`'s per-parameter marker, on a parameter of a
/// `#[Route]` method, and it carries nothing at all: the key it binds by is the
/// parameter's own name and the type it converts to is the parameter's own
/// type, so there is no field left for a payload to hold. It is the counterpart
/// of [`OPTION`] — the sentence that decides where a parameter's value comes
/// from, with no inference from defaults or types — and like [`ROUTE`] it names
/// no member of anything, so a file that spells it bare places it with
/// `use Core\Query;`. [`crate::routes`] owns what it means.
pub const QUERY: &str = r"Core\Query";

/// `#[Access(allow: mixed, csrf?: bool)]` — `rule:attributes/access-is-a-required-sibling`'s required sibling of
/// `#[Route]`, on the same method, and the only name here whose point is to
/// make an omission visible: § 3 gives it no implicit default, so a route that
/// is public because its author decided so and one that is public because its
/// author forgot are not the same text. Like [`ROUTE`] it names no member of
/// anything, so a file that spells it bare places it with `use Core\Access;`.
/// [`crate::routes`] owns the payload, including the two rules of § 1a that a
/// roster cannot state.
pub const ACCESS: &str = r"Core\Access";

/// `#[Api(tags?, errors?, security?, example?)]` — `rule:attributes/api-adds-and-cannot-contradict`'s annotation,
/// on a `#[Route]` method, and the only name here that is *purely* additive:
/// every field supplies something the route table and the signature cannot
/// say, and none of them changes what the program does. That is why § 2's rule
/// is stated as "it may add, and it may not contradict" rather than as a shape
/// — a field that agreed with the code would be a copy, and one that disagrees
/// is [`nvs_diagnostics::code::E_API_CONTRADICTS_THE_CODE`]. Like [`ROUTE`] it
/// names no member of anything, so a file that spells it bare places it with
/// `use Core\Api;`. [`crate::routes`] owns the payload and the four
/// contradictions.
pub const API: &str = r"Core\Api";

/// `#[Core\Path]` — `rule:programs/path-literals-resolve-from-their-file`'s
/// per-parameter marker, on a `string` parameter of a method, and it carries
/// nothing: what it says is the parameter's own
/// [`ParamText::Path`](nvs_stdlib::registry::ParamText::Path), the mark a
/// `Core` row writes as `CoreTy::Path`. It shares its name with the
/// `Core\Path` class, as [`TEST`] and [`COMMAND`] share theirs, so the `use
/// Core\Path;` that reaches `Path::join` also places the attribute.
/// [`crate::paths`] owns what it means and where it may be written.
pub const PATH: &str = r"Core\Path";

/// One of `rule:core-classes/derive-attribute`'s two derived formats — the only thing this pass
/// branches on.
///
/// Every rule of §§ 2, 3, 5 and 7 is stated once and asked of both; what a
/// format supplies is its attribute pair, the codec members a class may
/// hand-write instead, and its **type map**. A third format would be a third
/// variant and no new pass.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Format {
    /// `#[Json\Derive]` — spec § 6's `Core\Json\Codec`, both halves.
    Json,
    /// `#[Db\Derive]` — spec § 6's `Core\Db\Codec`, which declares `fromRow`
    /// alone (§ 7).
    Db,
}

impl Format {
    /// Both formats, in the order a class is asked about them.
    ///
    /// A loop rather than a choice, because a class may carry both attributes:
    /// a row read into an object that is then encoded is the ordinary case,
    /// and the two contracts are independent — § 3's own reason for two
    /// `Field` attributes rather than one.
    const ALL: [Self; 2] = [Self::Json, Self::Db];

    /// The class attribute that opts in, fully qualified as [`ATTRIBUTES`]
    /// holds it.
    const fn derive(self) -> &'static str {
        match self {
            Self::Json => DERIVE,
            Self::Db => DB_DERIVE,
        }
    }

    /// The per-property override attribute, fully qualified.
    const fn field(self) -> &'static str {
        match self {
            Self::Json => FIELD,
            Self::Db => DB_FIELD,
        }
    }

    /// How [`Self::derive`] is *written* in a diagnostic — the short form
    /// `rule:core-classes/derive-attribute`'s own example writes, since a message naming the fully
    /// qualified spelling would name the one form the reader did not use.
    const fn attribute(self) -> &'static str {
        match self {
            Self::Json => r"#[Json\Derive]",
            Self::Db => r"#[Db\Derive]",
        }
    }

    /// How [`Self::field`] is written in a diagnostic.
    const fn field_attribute(self) -> &'static str {
        match self {
            Self::Json => r"#[Json\Field]",
            Self::Db => r"#[Db\Field]",
        }
    }

    /// § 3's escape hatch, spelled for this format — the fix half of every
    /// refusal below.
    const fn skip_hint(self) -> &'static str {
        match self {
            Self::Json => r"#[Json\Field(skip: true)]",
            Self::Db => r"#[Db\Field(skip: true)]",
        }
    }

    /// § 3's signature, spelled for this format.
    const fn field_signature(self) -> &'static str {
        match self {
            Self::Json => r"#[Json\Field(name?: string, skip?: bool)]",
            Self::Db => r"#[Db\Field(name?: string, skip?: bool)]",
        }
    }

    /// The codec members a class may hand-write itself, all of which it has to
    /// write for § 7's "the attribute generates nothing" to apply.
    ///
    /// Two for JSON and one for a row, which is what makes § 7's
    /// one-directional rule a row in this table rather than a branch: a class
    /// that writes `fromRow` has left the `#[Db\Derive]` on it nothing to do.
    const fn halves(self) -> &'static [&'static str] {
        match self {
            Self::Json => &[ENCODE, DECODE],
            Self::Db => &[DB_DECODE],
        }
    }

    /// What the derived thing is called in a message — the contract a class
    /// with no field would have derived.
    const fn contract(self) -> &'static str {
        match self {
            Self::Json => "a JSON codec",
            Self::Db => "a row mapping",
        }
    }

    /// The clause naming what a field's declared type does not have, for the
    /// type-map refusal.
    const fn no_mapping(self) -> &'static str {
        match self {
            Self::Json => "which has no JSON representation",
            Self::Db => "which has no column mapping",
        }
    }

    /// That refusal's primary label, at the property's own name.
    const fn no_mapping_label(self) -> &'static str {
        match self {
            Self::Json => "this type cannot be encoded or decoded",
            Self::Db => "no column reads back as this",
        }
    }

    /// That refusal's help — the format's type map, named where it is
    /// specified.
    const fn no_mapping_help(self) -> &'static str {
        match self {
            Self::Json => {
                "`rule:core-classes/derive-field-list`: a field is a scalar, an enum, an inline shape, an `array<T>` or \
                 `?T` of one of those, or another class that itself carries a codec — write \
                 `#[Json\\Field(skip: true)]` to leave it off the contract"
            }
            Self::Db => {
                "`rule:core-classes/db-column-types`: a column reads back as a scalar, a `decimal`, `bytes`, an enum, a \
                 `Core\\Time` date or time, a `Core\\Uuid`, or an `array<T>` or `?T` of one of \
                 those — a row is flat, so a nested object is not a column type; write \
                 `#[Db\\Field(skip: true)]` to leave it off the mapping"
            }
        }
    }
}

/// One derived class's field list, in declaration order — `rule:core-classes/derive-field-list`'s
/// "declaration order fixes encode order, so output is byte-deterministic".
#[derive(Clone, Debug, Default)]
pub struct DerivedCodec {
    /// Every field the codec reads and writes, in declaration order. A
    /// `#[Json\Field(skip: true)]` property is absent rather than marked.
    pub fields: Vec<DerivedField>,
    /// How many parameters the class's `constructor` declares — what a
    /// generated decoder has to fill before it can run one. Zero for a class
    /// that declares none, which [`check_constructor_parameter`] has already
    /// reported through [`crate::ctor_init`].
    pub ctor_arity: usize,
}

/// One field of a [`DerivedCodec`], as read off the *declaration*.
///
/// The slot-resolved half is [`nvs_stdlib::CodecField`]; the two are joined in
/// `nvs_ir::lower::lower_file`, which is the one place both this table and
/// [`crate::layout`]'s slot order are in hand.
#[derive(Clone, Debug)]
pub struct DerivedField {
    /// The declaring property's own name, `$`-sigil stripped — the key into
    /// [`crate::layout::ClassLayout::slot_of`].
    pub property: String,
    /// The JSON key or the column this field is read under: the property's own
    /// name, or the format's `Field(name: "...")` override.
    pub key: String,
    /// What a decode has to produce for this field — the declared property
    /// type, erased to the closed roster a native decoder branches on.
    pub ty: CodecTy,
    /// What each position holds where [`Self::ty`] is [`CodecTy::List`], every
    /// level of it — see [`nvs_stdlib::CodecField::element`], which this is
    /// the declaration half of.
    pub element: Option<CodecElement>,
    /// The class's label where the erasure above dropped one: the field's own
    /// class for a [`CodecTy::Class`], the *terminal element's* for a
    /// [`CodecTy::List`] at any depth. The only half a front end can state;
    /// `nvs-codegen` resolves it to a descriptor.
    pub class: Option<String>,
    /// The accepted backing values where the erasure above produced a
    /// [`CodecTy::Enum`], for the field itself or for a list's element — see
    /// [`nvs_stdlib::CodecField::cases`], which this is the declaration half
    /// of.
    pub cases: Option<EnumCases>,
    /// The inline shape's own field list where the erasure above produced a
    /// [`CodecTy::Shape`] — the declaration half of
    /// [`nvs_stdlib::CodecField::shape`], whose key `nvs_ir::lower` renders
    /// from this.
    ///
    /// The contract itself rather than that key, because the key's spelling is
    /// `nvs-ir`'s — it renders the slot and parameter indices no front-end
    /// crate has resolved yet — and this is the pass that read the type.
    pub shape: Option<Box<DerivedCodec>>,
    /// Whether the declared type admits `null` (`rule:core-api/required-optional-and-nullable`'s second column).
    pub nullable: bool,
    /// Whether a document must carry [`Self::key`] at all
    /// (`rule:core-api/required-optional-and-nullable`'s first column, which
    /// is independent of [`Self::nullable`]).
    ///
    /// Read off the constructor parameter's own default, which is where that
    /// rule puts optionality: a parameter with a default may be filled without
    /// the key, one without a default may not. `true` where the declaration
    /// named no parameter at all, so a field
    /// [`check_constructor_parameter`] has already refused does not also read
    /// as an optional one.
    pub required: bool,
    /// This field's position in the constructor's parameter list, or `None`
    /// when the class declares no matching parameter — which
    /// [`check_constructor_parameter`] has already reported.
    pub param: Option<usize>,
    /// The constant that parameter's written `= <literal>` default evaluates
    /// to — what a call site omitting the argument would emit, carried down so
    /// that a decoder, which is not a call site, can emit it instead
    /// (`nvs_stdlib::CodecField::default`).
    ///
    /// `Some` exactly where [`Self::required`] is `false`, since a parameter
    /// default is a literal of the declared type and nothing else
    /// (`crate::defaults::literal_default`), and `None` for a shape field,
    /// which has no constructor to declare one.
    pub default: Option<crate::defaults::ConstArg>,
}

/// The inline shape `declared` names, read as a codec — the door
/// `rule:core-api/required-optional-and-nullable`'s three columns arrive
/// through when no class was written to carry them, which is what a
/// `shapeAs<{...}>` type argument is.
///
/// `None` for anything that is not a `rule:types/shape-type` shape. A shape has
/// no property to hang a `#[Json\Field]` on and no constructor to check a field
/// against, so all three columns are read off the type itself: `required` is
/// the written `?` that [`crate::ty::ShapeField::required`] carries, `nullable`
/// is the field type's own `null` arm, and the wire key is the field's name,
/// since there is nowhere to write an override.
///
/// [`DerivedField::param`] is the field's index in the **sorted** order
/// [`crate::ty::TypeInterner::shape`] interns fields in, which is the order
/// `nvs_ir::lower::shape_class_label` keys a shape class's slots on — so
/// filling parameter `n` fills slot `n`, and the two sides need no second
/// table to agree. [`DerivedCodec::ctor_arity`] is the field count for the
/// same reason: a shape class declares no constructor, so what a decode has to
/// fill is every slot it has.
pub fn shape_codec(
    declared: TypeId,
    interner: &mut crate::ty::TypeInterner,
    enums: &crate::enums::EnumTable,
) -> Option<DerivedCodec> {
    let Ty::Shape(shape) = interner.get(declared) else {
        return None;
    };
    // Cloned because stripping a field's `null` arm interns, and the borrow
    // above is what the interner would have to hand back out to do it.
    let shape = shape.clone();
    let fields: Vec<DerivedField> = shape
        .iter()
        .enumerate()
        .map(|(param, field)| {
            let nullable = interner.is_nullable(field.ty);
            // The `null` arm is what nullability *is*, so the decode target is
            // the rest of the union — the same strip [`codec_field`] makes on a
            // declared property.
            let carried = if nullable {
                interner.without_null(field.ty)
            } else {
                field.ty
            };
            let erased = codec_ty(carried, interner, enums);
            DerivedField {
                property: field.name.clone(),
                key: field.name.clone(),
                ty: erased.ty,
                element: erased.element,
                class: erased.class,
                cases: erased.cases,
                shape: erased.shape,
                nullable,
                required: field.required,
                param: Some(param),
                // A shape declares no constructor and so no default: its
                // optional field is `rule:types/shape-type`'s `?`, and what
                // fills an absent one is the never-written marker.
                default: None,
            }
        })
        .collect();
    Some(DerivedCodec {
        ctor_arity: fields.len(),
        fields,
    })
}

/// The label the class synthesized for an inline shape carries — `$shape{n}`
/// for `{n: int}`, built from the field names
/// [`crate::ty::TypeInterner::shape`] has already sorted.
///
/// The format lives here, a crate before the one that synthesizes that class,
/// because both ends spell it: `nvs_ir::lower::shape_class_label` names the
/// class it registers, and [`crate::expr::args::written_class_of`] names the
/// same class when it records a call site's shape. A label, not a name — it
/// identifies a row in `nvs_ir::ir::Program`'s class table and nothing else,
/// and `$` cannot start a Novis identifier, so no declaration collides with it.
///
/// Keyed on the field names alone, so two shapes with the same fields share one
/// class whatever their field *types* are: a class carries slot names, not slot
/// types. What that costs is that the label cannot key a wire contract —
/// [`crate::expr_table::ExprTypeTable::record_shape_codec`] keys that by the
/// call site instead.
#[must_use]
pub fn shape_class_label(sorted_fields: &[String]) -> String {
    format!("$shape{{{}}}", sorted_fields.join(","))
}

/// `declared`, erased to what a native decoder branches on, with the class
/// label and the enum roster beside it where the erasure loses one.
///
/// `rule:core-classes/derive-field-list`'s codec-reachable set is wider than
/// this in one place still: the four `Core` value types of
/// [`DB_COLUMN_CLASSES`] that are not an `Instant` erase to [`CodecTy::Class`]
/// and are told from a codec-carrying class by their label alone, which is what
/// `nvs_stdlib::db::row` reads. § 2's compile-time refusal of a genuinely
/// unreachable type is this module's gap 1.
/// An inline shape keeps **both** halves of what it loses — the class a decode
/// constructs and the contract holding its field types — because a class label
/// is keyed on the field names alone and cannot carry the second
/// (`nvs_ir::lower::shape_class_label`). A shape written as the whole type
/// argument goes through [`shape_codec`], which is this same read one level up.
/// Nothing here narrows what *encodes*, which walks the value rather than the
/// declared type.
///
/// A class is [`CodecTy::Class`] whether or not it turns out to carry a
/// codec, because that is a question about the *whole program* — the class may
/// be declared further down the file — and this runs inside the walk. It is
/// [`resolve_field_types`] that refuses a class with no codec at all, and
/// `nvs_stdlib::json` that reports `rule:core-classes/derive-generates-what-is-missing`'s hand-written half, which no
/// derived decoder calls yet.
fn codec_ty(
    declared: TypeId,
    interner: &mut crate::ty::TypeInterner,
    enums: &crate::enums::EnumTable,
) -> Erased {
    // The arms that recurse need the interner handed back — a nested shape's
    // contract is read with [`shape_codec`], which interns — so what the type
    // is gets read out into an owned answer before the match ends, rather than
    // inside an arm that still holds the borrow.
    match nested_of(declared, interner) {
        Nested::Array(elem) => return list_erasure(elem, interner, enums),
        Nested::Shape(names) => {
            // § 2's inline shape reached as a field, keeping both halves the
            // erasure would otherwise drop: the class a decode constructs,
            // named the way a literal of those same fields names it, and the
            // per-field wire types that label cannot carry.
            return shape_codec(declared, interner, enums).map_or_else(
                || Erased::wire(CodecTy::Opaque),
                |codec| Erased {
                    ty: CodecTy::Shape,
                    element: None,
                    class: Some(shape_class_label(&names)),
                    cases: None,
                    shape: Some(Box::new(codec)),
                },
            );
        }
        Nested::No => {}
    }
    match interner.get(declared) {
        Ty::Bool => Erased::wire(CodecTy::Bool),
        Ty::Int => Erased::wire(CodecTy::Int),
        Ty::Uint => Erased::wire(CodecTy::Uint),
        Ty::Float => Erased::wire(CodecTy::Float),
        // `rule:types/decimal`'s own wire type, never `float`'s: the types are
        // not assignable to each other in the language, so a decoder reaching
        // one through the other would round away what the type is for.
        Ty::Decimal => Erased::wire(CodecTy::Decimal),
        // A `tainted` string is still a string on the wire; `rule:security/derived-codec-qualifiers` makes
        // the qualifier a call-site question, not a decoder one.
        Ty::String | Ty::TaintedString => Erased::wire(CodecTy::Str),
        // `rule:types/bytes`, under the same call-site reading of the
        // qualifier. It reaches a wire through a column alone — [`db_reachable`]
        // admits it and [`json_reachable`] does not — so the erasure is shared
        // and the reachable set is what keeps it out of a JSON document.
        Ty::Bytes | Ty::TaintedBytes => Erased::wire(CodecTy::Bytes),
        Ty::Mixed => Erased::wire(CodecTy::Mixed),
        // § 2's "another class that itself has a codec", and the one `Core`
        // value type that is a wire type instead. The label is what
        // `crate::layout` keys on and `nvs_ir::lower::lower_file` joins
        // through, so `nvs-codegen` can resolve it to a descriptor; an
        // `Instant` keeps it too, because a decoder that only has to ask
        // whether the column built what the field declared still needs the
        // name to say so. The rest of [`DB_COLUMN_CLASSES`] is
        // [`CodecTy::Class`], which `nvs_stdlib::db::row` reads by that same
        // label.
        Ty::Class(name, _) => {
            let label = name.to_string();
            let wire = if label == nvs_stdlib::time::INSTANT_NAME {
                CodecTy::Instant
            } else {
                CodecTy::Class
            };
            Erased {
                ty: wire,
                element: None,
                class: Some(label),
                cases: None,
                shape: None,
            }
        }
        // § 2's enum. What travels is the roster and not the name: `rule:enums/representation` reserves an enum tag that nothing writes, so by the time a case
        // is a value it is the integer behind it, and a decoder has nothing to
        // resolve a name against. The membership test is therefore the whole
        // of the decode — see `nvs_stdlib::EnumCases`.
        Ty::Enum(name, backing) => Erased {
            ty: CodecTy::Enum,
            element: None,
            class: None,
            cases: enum_cases(name, *backing, enums),
            shape: None,
        },
        _ => Erased::wire(CodecTy::Opaque),
    }
}

/// § 2's list field, erased. The element goes through [`codec_ty`] once and
/// comes back as a [`CodecElement`] chain, so a second `List` is one more link
/// rather than a type this has no room for: `array<array<Tag>>` and
/// `array<{n: int}>` describe themselves in full.
///
/// **Everything the element's own erasure dropped rides up onto the field** —
/// the class label, the case roster and the inline shape's contract alike —
/// because a chain of lists bottoms out in exactly one node that is not a
/// `List`, and only that node can name any of the three. The field is the one
/// row a decoder has in hand when it reaches position `n` at any depth, and
/// `nvs-codegen` resolves its two pointers once.
///
/// An `Opaque` element is the one case that takes the whole field with it: the
/// terminal type has no wire form, so no depth of list around it has one
/// either. `rule:core-classes/derive-field-list`'s reachable test refuses that
/// declaration outright ([`resolve_field_types`]).
fn list_erasure(
    element: TypeId,
    interner: &mut crate::ty::TypeInterner,
    enums: &crate::enums::EnumTable,
) -> Erased {
    let erased = codec_ty(element, interner, enums);
    if erased.ty == CodecTy::Opaque {
        return Erased::wire(CodecTy::Opaque);
    }
    Erased {
        ty: CodecTy::List,
        element: Some(CodecElement {
            ty: erased.ty,
            element: erased.element.map(Box::new),
        }),
        class: erased.class,
        cases: erased.cases,
        shape: erased.shape,
    }
}

/// What [`codec_ty`]'s recursive arms need, read off the declared type and
/// owned, so the interner's borrow ends with the match that produced it.
enum Nested {
    /// An `array<T>`, carrying `T`.
    Array(TypeId),
    /// An inline shape, carrying its field names in the sorted order
    /// [`crate::ty::TypeInterner::shape`] interns them in — which is what
    /// [`shape_class_label`] keys the synthesized class on.
    Shape(Vec<String>),
    /// Every other declared type, which [`codec_ty`] erases without recursing.
    No,
}

/// Which of [`Nested`]'s answers `declared` is.
fn nested_of(declared: TypeId, interner: &crate::ty::TypeInterner) -> Nested {
    match interner.get(declared) {
        Ty::Array(element) => Nested::Array(*element),
        Ty::Shape(fields) => Nested::Shape(fields.iter().map(|f| f.name.clone()).collect()),
        _ => Nested::No,
    }
}

/// What [`codec_ty`] answers: the wire type, then what the erasure drops — a
/// list's element type, a class label, an enum's case roster, an inline shape's
/// own contract — each present only for the wire type that lost it.
#[derive(Clone, Debug)]
struct Erased {
    /// The wire type a decoder branches on.
    ty: CodecTy,
    /// A [`CodecTy::List`]'s element, every level of it.
    element: Option<CodecElement>,
    /// The class label a [`CodecTy::Class`], a [`CodecTy::Shape`] or a list
    /// bottoming out in either lost.
    class: Option<String>,
    /// A [`CodecTy::Enum`]'s accepted backing values, for the field itself or
    /// for a list's terminal element.
    cases: Option<EnumCases>,
    /// A [`CodecTy::Shape`]'s own field list, read the way [`shape_codec`]
    /// reads a written one.
    shape: Option<Box<DerivedCodec>>,
}

impl Erased {
    /// A wire type the erasure dropped nothing from.
    fn wire(ty: CodecTy) -> Self {
        Self {
            ty,
            element: None,
            class: None,
            cases: None,
            shape: None,
        }
    }
}

/// `qname`'s declared cases, ascending — [`CodecTy::Enum`]'s whole decode.
///
/// `None` for a name this program declares no enum for. That is a resolution
/// failure already reported where the annotation is written, and the decoder
/// treats it as the engine fault it is rather than accepting every integer.
fn enum_cases(
    qname: &QName,
    backing: crate::enums::EnumBacking,
    enums: &crate::enums::EnumTable,
) -> Option<EnumCases> {
    let info = enums.get(qname)?;
    let mut values: Vec<i128> = info
        .cases
        .values()
        .map(|value| match value {
            crate::enums::EnumValue::Int(number) => i128::from(*number),
            crate::enums::EnumValue::Uint(number) => i128::from(*number),
        })
        .collect();
    // A `FxHashMap`'s iteration order is not the declaration order and is not
    // stable between runs, so sorting is what makes this roster — and the
    // artifact cache key over it — reproducible. A binary search wants it
    // sorted anyway.
    values.sort_unstable();
    Some(EnumCases {
        unsigned: backing == crate::enums::EnumBacking::Uint,
        values,
    })
}

/// One field `rule:core-classes/derive-field-list`'s codec-reachable test still owes an answer,
/// recorded as the walk reaches it and resolved by [`resolve_field_types`].
///
/// Carries the *null-stripped* declared type, because `?T` is reachable
/// exactly when `T` is, and the property's own span, because § 2's refusal is
/// at the declaration that wrote the field rather than at the `decodeAs<T>`
/// that would later fail on it.
#[derive(Debug)]
pub struct CodecFieldSite {
    /// The deriving class, for the message alone.
    class: String,
    /// The property's name, `$` stripped.
    property: String,
    /// Where that name is written.
    span: Span,
    /// The declared type, with `?`'s `null` arm already removed.
    declared: TypeId,
    /// Which format's type map answers for it — the two disagree, and a class
    /// carrying both attributes records the same property twice.
    format: Format,
}

/// `rule:core-classes/derive-field-list`'s "a field's type must be codec-reachable", once every
/// deriving class in the program has recorded its codec.
///
/// Run after the walk, from [`crate::check::check_program`], for
/// [`crate::links::resolve`]'s reason: a field naming another deriving class
/// must not depend on which file declared it first.
///
/// **What it refuses is the unreachable set**, and the JSON half of the
/// erasure now answers for all of it: § 2 lists a `decimal`, an `Instant`, an
/// enum, an inline shape, an `array<T>` and a nested codec-carrying class as
/// reachable, and each of those — at any depth of `array<…>`, since
/// [`list_erasure`]'s chain describes every level — erases to a wire type
/// `nvs_stdlib::json` decodes. So the test is over the declared type alone,
/// and an [`CodecTy::Opaque`] reaching a decoder is a declaration this pass
/// has already reported rather than a missing case.
///
/// The row half is narrower on purpose and refuses where it is: a column is
/// one value, so [`check_row_sites`] turns away the nested documents the JSON
/// door reads.
pub(crate) fn resolve_field_types(
    sites: &[CodecFieldSite],
    signatures: &crate::signatures::SignatureTable,
    interner: &crate::ty::TypeInterner,
    exprs: &crate::expr_table::ExprTypeTable,
    diags: &mut Diagnostics,
) {
    for site in sites {
        let format = site.format;
        if reachable(format, site.declared, interner, signatures, exprs) {
            continue;
        }
        let spelling = interner.describe(site.declared);
        let class = &site.class;
        let property = &site.property;
        let missing = format.no_mapping();
        diags.report(
            Diagnostic::error(
                code::E_DERIVE_FIELD_NOT_CODEC_REACHABLE,
                format!("`{class}::${property}` is declared `{spelling}`, {missing}"),
            )
            .with_primary(site.span, format.no_mapping_label())
            .with_help(format.no_mapping_help()),
        );
    }
}

/// One `Core\Db\…::queryAs<T>` or `::streamAs<T>` call site, held until every
/// deriving class in the program has recorded its mapping.
///
/// [`CodecFieldSite`]'s reason, one layer out: the row class a call names is
/// routinely declared in a file the walk has not reached, so answering where
/// the call is written would make the refusal depend on file order.
#[derive(Debug)]
pub struct RowSite {
    /// `Class::method` as the message spells it — both of `rule:core-classes/db-transactions`'s
    /// spellings reach here, and a reader needs to see the one they wrote.
    member: String,
    /// The class the type argument named, resolved.
    class: QName,
    /// Whether it was written `array<C>`.
    list: bool,
    /// Where the type argument is written.
    span: Span,
}

impl RowSite {
    /// Records a site, from [`crate::expr::args::written_class_of`] — the one
    /// place a written class and the member that asked for it are both in hand.
    pub(crate) fn new(member: String, class: QName, list: bool, span: Span) -> Self {
        Self {
            member,
            class,
            list,
            span,
        }
    }
}

/// `rule:core-classes/db-column-types`'s map, asked of the class a hydrating
/// member wrote, once every deriving class in the program has recorded its
/// mapping.
///
/// Run after the walk, from [`crate::check::check_program`], beside
/// [`resolve_field_types`] and for the same reason.
///
/// **Every condition, one code.** They are the ways one question — can a row be
/// hydrated into this `T`? — is answered no, and a reader at the call site is
/// fixing the same thing in each: the type argument. The two over the class's
/// *fields* are the ones that cannot move to the declaration, and they are why
/// this pass exists at all: `#[Db\Field(skip: true)]` is
/// `rule:core-classes/derive-field-list`'s sanctioned way to take a property off
/// the mapping, so a class carrying one is well formed and stays well formed,
/// and a field the erasure gave no wire type is
/// [`resolve_field_types`]'s undecoded set rather than its unreachable one. It
/// is only a hydrating call over such a class that has a constructor parameter
/// nothing can fill.
pub(crate) fn check_row_sites(
    sites: &[RowSite],
    signatures: &crate::signatures::SignatureTable,
    exprs: &crate::expr_table::ExprTypeTable,
    diags: &mut Diagnostics,
) {
    for site in sites {
        let member = &site.member;
        let class = &site.class;
        if site.list {
            report_row_site(
                site,
                format!("`{member}` builds one class per row, and `array<{class}>` is a list"),
                "`rule:core-classes/db-statement-members`: the member already answers one row per row — a \
                 `Core\\Db\\Rows` to read or a walk to step — so a list form asks for the plural \
                 twice. Write the row class alone",
                diags,
            );
            continue;
        }
        let Some(codec) = exprs.db_codec(&class.to_string()) else {
            // The hand-written door, which is the *whole* of what
            // `rule:core-classes/derive-generates-what-is-missing` leaves a row
            // class to write: `Core\Db\Codec` declares [`DB_DECODE`] alone, so
            // a class that declares it has opted in as squarely as the
            // attribute does and records no mapping precisely because there is
            // nothing left to generate. Every condition below is about a
            // mapping, so there is none of them to ask — what the member does
            // with the row is the member's own. `nvs_stdlib::db::row`'s
            // `hydrate` dispatches to it.
            if signatures
                .get(class)
                .is_some_and(|sig| sig.methods.contains_key(DB_DECODE))
            {
                continue;
            }
            report_row_site(
                site,
                format!(
                    "`{class}` carries no `#[Db\\Derive]` and declares no `{DB_DECODE}`, so \
                     `{member}` has no mapping"
                ),
                "`rule:core-classes/derive-attribute`: hydrating a row is opt-in — write `#[Db\\Derive]` on the class, \
                 which is what generates the `Core\\Db\\Codec` this call needs, or declare that \
                 interface's `fromRow` yourself and build the instance from the \
                 `Core\\Db\\Row`. A `#[Json\\Derive]` \
                 is the document half and answers for nothing here: `rule:core-classes/db-column-types`'s map is over \
                 columns",
                diags,
            );
            continue;
        };
        // The condition that was said once per row until now. A field the
        // erasure could not give a wire type is a property of the *class*, but
        // the class is not wrong for it — [`resolve_field_types`]'s doc says
        // why the unreachable set and the undecoded one are different
        // questions — so the call that asks for a whole row out of it is where
        // it is answered. `nvs_stdlib::db::row`'s `hydrate` keeps the same
        // refusal as the backstop for a class built by hand.
        if let Some(field) = codec.fields.iter().find(|field| {
            matches!(field.ty, CodecTy::Opaque | CodecTy::Shape)
                // A column is one value, so a list of them is as deep as a row
                // goes: an element that is itself a list or an inline shape is
                // a nested document, which `rule:core-classes/db-column-types`
                // maps no column to however well the JSON door reads one.
                || field.element.as_ref().is_some_and(|element| {
                    element.any(|ty| matches!(ty, CodecTy::Opaque | CodecTy::Shape | CodecTy::List))
                })
        }) {
            let property = &field.property;
            report_row_site(
                site,
                format!(
                    "`{class}::${property}` is declared a type no column reads back, so \
                     `{member}` has no value to give it"
                ),
                "`rule:core-classes/db-column-types`: a row is a flat list of columns, and an \
                 inline shape is a nested document rather than one. Declare the property as \
                 the column's own type, or leave it off the mapping with \
                 `#[Db\\Field(skip: true)]` and give the constructor a value for it yourself",
                diags,
            );
            continue;
        }
        let filled: std::collections::BTreeSet<usize> = codec
            .fields
            .iter()
            .filter_map(|field| field.param)
            .collect();
        if filled.len() != codec.ctor_arity {
            let arity = codec.ctor_arity;
            let mapped = filled.len();
            report_row_site(
                site,
                format!(
                    "`{class}`'s mapping fills {mapped} of its constructor's {arity} \
                     parameter(s), so `{member}` cannot build one"
                ),
                "a property left off the mapping — `#[Db\\Field(skip: true)]`, `rule:core-classes/derive-field-list` — is \
                 still a constructor parameter, and a row has no column to fill it from. Give it \
                 a column and drop the `skip`, or build the class yourself from a \
                 `Core\\Db\\Row`",
                diags,
            );
        }
    }
}

/// `E_QUERY_AS_NOT_A_ROW_CLASS`, from every one of [`check_row_sites`]'
/// conditions.
fn report_row_site(site: &RowSite, message: String, help: &str, diags: &mut Diagnostics) {
    diags.report(
        Diagnostic::error(code::E_QUERY_AS_NOT_A_ROW_CLASS, message)
            .with_primary(site.span, "written here")
            .with_help(help),
    );
}

/// Whether a written-class member of `owner` hydrates a **row** rather than a
/// document — `rule:core-classes/db-column-types`'s question, which is
/// [`check_row_sites`]'.
///
/// The owner rather than the member's name, because the name is not the
/// program's: `queryAs` is `Core\Db\Connection`'s statement member and
/// `Core\Request`'s query-string reader alike, so a classification keyed on the
/// spelling answers a request's class against a column map that has nothing to
/// say about a query string. Every other written class is a document's, which
/// is [`check_json_sites`].
pub(crate) fn hydrates_a_row(owner: &str) -> bool {
    owner == r"Core\Db\Connection" || owner == r"Core\Db\Transaction"
}

/// Whether the octets a written-class member of `owner` decodes are a
/// **peer's** — `rule:security/tainted-sources`, which is what
/// [`check_decode_sites`] and [`check_shape_decode_site`] ask the fields
/// receiving them to declare.
///
/// Keyed on the owner for [`hydrates_a_row`]'s reason and one of its own: every
/// member of these owners answers alike. Every `Core\Request` member on the
/// roster reads the request, and that rule makes the body and the query alike a
/// peer's octets; `Core\Http\Response::jsonAs` reads the reply another host sent
/// back, which is input in the same sense — a pinned address settles which host
/// wrote the octets and nothing about what is in them; and
/// `Core\Jwt::verifyIssued` reads a payload another party wrote, which
/// `rule:security/verification-does-not-launder` keeps `tainted` because a
/// signature proves origin and not safety.
///
/// The rest are not: `Core\Json::decodeAs` takes its document through a plain
/// `string` parameter, so a tainted one is refused where it is passed;
/// `Core\Arr::shapeAs` converts an array the program already holds, whose taint
/// it carries in already; and a `Core\Db` row is not a taint source at all.
pub(crate) fn reads_a_peers_octets(owner: &str) -> bool {
    owner == r"Core\Request" || owner == r"Core\Http\Response" || owner == r"Core\Jwt"
}

/// Whether a written-class member of `owner` **stands in for** the class it
/// names rather than building one out of anything — `rule:testing/doubles`'s
/// `double` and `partial`, and nothing else.
///
/// Keyed on the owner for [`hydrates_a_row`]'s reason, and what it separates is
/// a door with no octets behind it at all: a double's fields are closures the
/// call site wrote, so nothing arrives from outside, no field receives a
/// document and there is nothing to decode. Every deferred question the two
/// rosters above raise would therefore be asked of a class nobody is
/// hydrating — and the class a double names is normally an `interface`, which
/// carries no deriving attribute and never will, so [`check_json_sites`] would
/// refuse every double in the program.
pub(crate) fn stands_in_for_a_class(owner: &str) -> bool {
    owner == r"Core\Test"
}

/// One call site of a member that builds a written class out of a **document**,
/// held until every deriving class in the program has recorded its fields.
///
/// [`RowSite`]'s reason at the other door: the class a call names is routinely
/// declared in a file the walk has not reached, so answering where the call is
/// written would make the refusal depend on file order.
#[derive(Debug)]
pub struct JsonSite {
    /// The member that asked, as `Class::member` — every door that runs
    /// `nvs_stdlib::json`'s decoder reaches here, and a reader needs to see the
    /// one they wrote.
    member: String,
    /// The class the type argument named, resolved. `array<C>` records `C`: a
    /// list document is the same decode run once per element, and a JSON array
    /// of objects is a document in its own right, which is where this door
    /// parts company with [`RowSite`]'s.
    class: QName,
    /// Where the type argument is written.
    span: Span,
}

impl JsonSite {
    /// Records a site, from [`crate::expr::args::written_class_of`] — the one
    /// place a written class and the member that asked for it are both in hand.
    pub(crate) fn new(member: String, class: QName, span: Span) -> Self {
        Self {
            member,
            class,
            span,
        }
    }
}

/// `rule:core-classes/derive-field-list`'s skipped field, asked of the class a
/// document decoder wrote, once every deriving class in the program has
/// recorded its fields.
///
/// Run after the walk, from [`crate::check::check_program`], beside
/// [`check_row_sites`] and for the same reason.
///
/// **One condition, where the row door has four.** A row is a flat list of
/// columns and a document is a tree, so the three about a column map are the
/// other door's alone. What the two share is a constructor the contract cannot
/// fill, and `#[Json\Field(skip: true)]` on a property that stayed a
/// constructor parameter is the one way to write one: the class is well formed
/// and stays well formed — `rule:core-classes/derive-field-list` sanctions the
/// skip — so it is the call asking for a whole instance out of a document that
/// is where there is nothing to fill that position from.
///
/// **The reachable set, not the written class's own fields**, which is
/// [`check_decode_sites`]' walk for the same reason: a field typed as another
/// deriving class is built by the same decode, so its constructor has to be
/// fillable too. `seen` is what stops the walk on a class holding a field of
/// its own type, which `rule:core-classes/derive-field-list` admits.
///
/// **Two conditions, two codes**, which is where this parts company with
/// [`check_row_sites`]' one. The written class participating at all is asked
/// first, of the root alone, and is `E0821`; the constructor the contract fills
/// less than all of is asked of the reachable set and is `E0820`. A reader
/// fixing the first writes an attribute or a decoder on a class that has
/// neither, and a reader fixing the second edits a contract that is already
/// there — different edits on different declarations, where the row door's
/// conditions are all one edit at the call.
pub(crate) fn check_json_sites(
    sites: &[JsonSite],
    signatures: &crate::signatures::SignatureTable,
    exprs: &crate::expr_table::ExprTypeTable,
    diags: &mut Diagnostics,
) {
    for site in sites {
        if exprs.codec(&site.class.to_string()).is_none() {
            check_json_participation(site, signatures, diags);
            // Nothing recorded a field list, so there is no contract to walk
            // and no reachable set under it.
            continue;
        }
        let mut seen = std::collections::BTreeSet::<String>::new();
        let mut pending = vec![site.class.to_string()];
        while let Some(class) = pending.pop() {
            if !seen.insert(class.clone()) {
                continue;
            }
            // A field naming a class that carries no codec, which
            // [`resolve_field_types`] has already refused at the declaration
            // that wrote it. The root took the branch above.
            let Some(codec) = exprs.codec(&class) else {
                continue;
            };
            for nested in codec.fields.iter().filter_map(|field| field.class.as_ref()) {
                pending.push(nested.clone());
            }
            let filled: std::collections::BTreeSet<usize> = codec
                .fields
                .iter()
                .filter_map(|field| field.param)
                .collect();
            if filled.len() == codec.ctor_arity {
                continue;
            }
            let arity = codec.ctor_arity;
            let mapped = filled.len();
            let member = &site.member;
            diags.report(
                Diagnostic::error(
                    code::E_DECODED_CLASS_NOT_CONSTRUCTIBLE,
                    format!(
                        "`{class}`'s codec fills {mapped} of its constructor's {arity} \
                         parameter(s), so `{member}` cannot build one"
                    ),
                )
                .with_primary(site.span, "written here")
                .with_help(
                    "a property left off the contract — `#[Json\\Field(skip: true)]`, \
                     `rule:core-classes/derive-field-list` — is still a constructor parameter, \
                     and a document has no key to fill it from. Drop the `skip` and let the \
                     field carry it, or assign the property in the constructor body rather than \
                     taking it as a parameter",
                ),
            );
        }
    }
}

/// Whether `site`'s class participates in the JSON format at all, reported as
/// `E0821` where it does not.
///
/// **Two doors into participation, and the class picks one.**
/// `rule:core-classes/derive-attribute` generates the codec where the attribute
/// is written, and `rule:core-classes/derive-generates-what-is-missing` lets a
/// class write a half itself — so a class declaring [`DECODE`] has opted in as
/// squarely as the attribute does and records no field list precisely because
/// there is nothing left to generate. The encoding half answers for nothing
/// here: it is the decoder this call needs, and a class writing only that one
/// takes the same door `rule:core-classes/derive-generates-what-is-missing`'s
/// "the generated decoder and keeps its encoder" class takes from the other
/// side. Where that half is *dispatched* is the encoder's own class arm in
/// `nvs_stdlib::json`, which is why a class writing only it encodes and is
/// refused here all the same: what this call needs is the door in.
///
/// The root only. A field naming a class with no codec is refused at the
/// declaration that wrote it, which is [`resolve_field_types`], so asking again
/// here would report one edit twice.
fn check_json_participation(
    site: &JsonSite,
    signatures: &crate::signatures::SignatureTable,
    diags: &mut Diagnostics,
) {
    if signatures
        .get(&site.class)
        .is_some_and(|sig| sig.methods.contains_key(DECODE))
    {
        return;
    }
    let class = &site.class;
    let member = &site.member;
    diags.report(
        Diagnostic::error(
            code::E_DECODED_CLASS_HAS_NO_CODEC,
            format!(
                "`{class}` carries no `#[Json\\Derive]` and declares no `{DECODE}`, so \
                 `{member}` has no codec to read one with"
            ),
        )
        .with_primary(site.span, "written here")
        .with_help(
            "`rule:core-classes/derive-attribute`: reading a class out of a document is opt-in — \
             write `#[Json\\Derive]` on the class, which is what generates the `Core\\Json\\Codec` \
             this call needs, or declare that interface's `fromJson` yourself and build the \
             instance from the `mixed` value. A `#[Db\\Derive]` is the row half and answers for \
             nothing here: `rule:core-classes/db-column-types`'s map is over columns",
        ),
    );
}

/// One call site of a member that decodes a peer's octets into a written class,
/// held until every deriving class in the program has recorded its fields.
///
/// [`RowSite`]'s reason for the other members on
/// [`crate::expr::args::written_class_of`]'s roster: the class a call names is
/// routinely declared in a file the walk has not reached, so answering where
/// the call is written would make the refusal depend on file order.
#[derive(Debug)]
pub struct DecodeSite {
    /// The member that asked, as `Class::member`, so the report says whose
    /// octets they are — a request body under `Core\Request`, an issuer's
    /// payload under `Core\Jwt::verifyIssued`.
    member: String,
    /// The class the type argument named, resolved. `array<C>` records `C`: a
    /// list decode is the same decode run once per element, so the fields
    /// receiving the body are the same fields.
    class: QName,
    /// Where the type argument is written — the only part of the *call* the
    /// report keeps, and it keeps it as the secondary label.
    span: Span,
}

impl DecodeSite {
    /// Records a site, from [`crate::expr::args::written_class_of`] — the one
    /// place a written class and the member that asked for it are both in hand.
    pub(crate) fn new(member: String, class: QName, span: Span) -> Self {
        Self {
            member,
            class,
            span,
        }
    }
}

/// `rule:security/derived-codec-qualifiers`'s qualifier, asked of the class a
/// decoding member wrote, once every deriving class in the program has recorded
/// its fields.
///
/// Run after the walk, from [`crate::check::check_program`], beside
/// [`check_row_sites`] and for the same reason.
///
/// **The call site, not the declaration.** A derived codec says nothing about
/// where its documents come from, and the same class is legitimate over one the
/// program built itself; it is the *document* that is a peer's octets — a
/// request body, or the payload of a token another party signed. So the site
/// that decodes one is where the question has an answer, and the message still
/// points at the property, which is where the fix is written.
///
/// **The reachable set, not the written class's own fields.** A field typed as
/// another deriving class is filled from the same document, so its text fields
/// receive the same octets; `seen` is what stops the walk on a class holding a
/// field of its own type, which § 2 admits.
///
/// One report per property however many sites decode into it: the fix is one
/// `tainted` on one declaration, and repeating it once per call would report
/// the same edit as several.
pub(crate) fn check_decode_sites(
    sites: &[DecodeSite],
    fields: &[CodecFieldSite],
    interner: &crate::ty::TypeInterner,
    diags: &mut Diagnostics,
) {
    let mut reported = std::collections::BTreeSet::<(String, String)>::new();
    for site in sites {
        let mut seen = std::collections::BTreeSet::<String>::new();
        let mut pending = vec![site.class.to_string()];
        while let Some(class) = pending.pop() {
            if !seen.insert(class.clone()) {
                continue;
            }
            for field in fields
                .iter()
                .filter(|field| field.format == Format::Json && field.class == class)
            {
                if let Some(nested) = codec_class_label(field.declared, interner) {
                    pending.push(nested);
                }
                if !unqualified_text(field.declared, interner)
                    || !reported.insert((class.clone(), field.property.clone()))
                {
                    continue;
                }
                let property = &field.property;
                let spelling = interner.describe(field.declared);
                let member = &site.member;
                diags.report(
                    Diagnostic::error(
                        code::E_DECODED_FIELD_NOT_TAINTED,
                        format!(
                            "`{class}::${property}` is declared `{spelling}`, and `{member}` \
                             answers a peer's octets"
                        ),
                    )
                    .with_primary(field.span, "declared without a qualifier")
                    .with_secondary(site.span, format!("hydrated by `{member}` here"))
                    .with_help(
                        "`rule:security/derived-codec-qualifiers`: a decoder assigns into the \
                         declared property types, so a field that receives a peer's octets is \
                         the one that has to declare them — write `tainted` on it",
                    ),
                );
            }
        }
    }
}

/// Whether a peer's octets can land in `declared` without the qualifier: a
/// `string` or a `bytes`, or anything built out of one — an array of either,
/// which decodes element by element into the same declared text; an arm of a
/// union, since the value may arrive as that arm; or a field of a nested shape,
/// which receives the same document one level down.
///
/// `secret` never reaches here — [`codec_field`] refuses a `secret` property at
/// its declaration — and the tainted forms are atoms of their own
/// (`rule:security/tainted-qualifier`), so an `int`, a `decimal`, an enum or a
/// nested class answers `false` and is walked instead.
fn unqualified_text(declared: TypeId, interner: &crate::ty::TypeInterner) -> bool {
    match interner.get(declared) {
        Ty::String | Ty::Bytes => true,
        Ty::Array(element) => unqualified_text(*element, interner),
        Ty::Union(arms) => arms.iter().any(|arm| unqualified_text(*arm, interner)),
        Ty::Shape(fields) => fields
            .iter()
            .any(|field| unqualified_text(field.ty, interner)),
        _ => false,
    }
}

/// `rule:security/tainted-qualifier`'s qualifier asked of an **inline shape**
/// written at a decode site — the shape half of the question
/// [`check_decode_sites`] asks of a deriving class.
///
/// **Answered where it is written rather than recorded.** A shape declares its
/// own fields at the call site, so there is no declaration further down the
/// file to find a qualifier on and nothing about the rest of the program to
/// wait for; the whole question is in hand at
/// [`crate::expr::args::written_class_of`], which is the one caller and which
/// decides there which members read a peer's octets.
///
/// The report names the field and the shape, because a shape has no class name
/// to give, and the fix is one `tainted` in front of the braces: the qualifier
/// distributes to every text-carrying field, so a shape of six strings takes
/// one word and not six.
pub(crate) fn check_shape_decode_site(
    element: TypeId,
    member: &str,
    span: Span,
    interner: &crate::ty::TypeInterner,
    diags: &mut Diagnostics,
) {
    let Ty::Shape(fields) = interner.get(element) else {
        return;
    };
    let shape = interner.describe(element);
    for field in fields.iter().filter(|f| unqualified_text(f.ty, interner)) {
        let name = &field.name;
        let declared = interner.describe(field.ty);
        diags.report(
            Diagnostic::error(
                code::E_DECODED_FIELD_NOT_TAINTED,
                format!(
                    "`{name}` is declared `{declared}` in `{shape}`, and `{member}` \
                     answers a peer's octets"
                ),
            )
            .with_primary(span, "written without a qualifier")
            .with_help(
                "`rule:security/tainted-qualifier`: write the qualifier in front of the \
                 shape — `tainted {…}` distributes to every text-carrying field, so it is \
                 one word however many keys receive text",
            ),
        );
    }
}

/// The class label [`check_decode_sites`] walks into next, for a field typed as
/// a class or as an array of one — [`DerivedField::class`]'s question asked of
/// the declared type rather than of the erasure, which drops the element's.
fn codec_class_label(declared: TypeId, interner: &crate::ty::TypeInterner) -> Option<String> {
    match interner.get(declared) {
        Ty::Class(name, _) => Some(name.to_string()),
        Ty::Array(element) => codec_class_label(*element, interner),
        _ => None,
    }
}

/// The format's type map, over the interned type rather than over [`CodecTy`]'s
/// erasure — see [`resolve_field_types`] for why the two are not the same
/// question.
fn reachable(
    format: Format,
    ty: TypeId,
    interner: &crate::ty::TypeInterner,
    signatures: &crate::signatures::SignatureTable,
    exprs: &crate::expr_table::ExprTypeTable,
) -> bool {
    match format {
        Format::Json => json_reachable(ty, interner, signatures, exprs),
        Format::Db => db_reachable(ty, interner),
    }
}

/// `rule:core-classes/db-column-types`'s type map, read as a predicate over the declared type.
///
/// Wider than [`json_reachable`] in one place and narrower in another, which
/// is the whole reason the two formats are not one map. `bytes` is a column
/// type — `BLOB`/`BYTEA` — where `rule:types/bytes` leaves it no JSON spelling at all;
/// and a **nested class is not**, because a row is a flat list of columns and
/// § 9 maps none of them to an object. The classes it does map are that
/// section's own value types, [`DB_COLUMN_CLASSES`].
///
/// Takes no signature or codec table for that second reason: nothing here is
/// a question about the rest of the program, so the answer cannot depend on
/// which file declared what first.
fn db_reachable(ty: TypeId, interner: &crate::ty::TypeInterner) -> bool {
    match interner.get(ty) {
        // § 9's scalar rows, plus `rule:types/literal-types`'s three singleton refinements of
        // them. `tainted` is not a distinction a column makes — § 6 makes
        // every text column tainted on the way out.
        Ty::Null
        | Ty::Bool
        | Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::String
        | Ty::TaintedString
        | Ty::Bytes
        | Ty::TaintedBytes
        | Ty::Mixed
        | Ty::True
        | Ty::False
        | Ty::StringLiteral(_)
        | Ty::IntLiteral(_) => true,
        // `rule:core-classes/derive-field-list`'s enum, unchanged by the format: what travels is the
        // backing value, range-checked on the way back in.
        Ty::Enum(..) | Ty::EnumCase(..) => true,
        // § 9's `array<T>` rows — a PostgreSQL array and a MySQL `SET`. One
        // dimension only: `array<array<T>>` is a column type no driver of the
        // five reads back, and `nvs_stdlib::CodecField::element` has no room
        // to describe it either.
        Ty::Array(elem) => {
            !matches!(interner.get(*elem), Ty::Array(_)) && db_reachable(*elem, interner)
        }
        Ty::Class(class, _) => {
            let label = class.to_string();
            DB_COLUMN_CLASSES.contains(&label.as_str())
        }
        // Everything else: an inline shape and a nested class are objects a
        // row has no column for, `object`, `callable` and `iterable` name no
        // contract, and a union of two non-`null` arms gives a reader nothing
        // to pick between.
        _ => false,
    }
}

/// The class half of `rule:core-classes/db-column-types`'s type map: the `Core` value types a column
/// reads back as.
///
/// A closed list rather than a `Core\` prefix test, because the point of the
/// map is that a type absent from it is absent *on purpose* — § 9 says so of
/// `interval`, which is deliberately not a `Duration` since it carries months.
/// A `Core` class that gains a column form gains a row here and an entry in
/// the driver's reader in the same change.
/// Each name is taken from the module that declares the class rather than
/// respelled here, so the map cannot drift from the registry.
const DB_COLUMN_CLASSES: &[&str] = &[
    nvs_stdlib::time::DATE_NAME,
    nvs_stdlib::time::TIME_OF_DAY_NAME,
    nvs_stdlib::time::INSTANT_NAME,
    nvs_stdlib::time::DATETIME_NAME,
    nvs_stdlib::uuid::NAME,
];

/// `rule:core-classes/derive-field-list`'s reachable set, over the interned type — JSON's half of
/// [`reachable`].
fn json_reachable(
    ty: TypeId,
    interner: &crate::ty::TypeInterner,
    signatures: &crate::signatures::SignatureTable,
    exprs: &crate::expr_table::ExprTypeTable,
) -> bool {
    match interner.get(ty) {
        // The scalars, plus `rule:types/literal-types`'s three singleton refinements of them:
        // each erases to a scalar and is exactly as spellable on the wire.
        Ty::Null
        | Ty::Bool
        | Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::String
        | Ty::TaintedString
        | Ty::Mixed
        | Ty::True
        | Ty::False
        | Ty::StringLiteral(_)
        | Ty::IntLiteral(_) => true,
        // ADR 0010: an enum travels as its backing value, range-checked on
        // the way back in.
        Ty::Enum(..) | Ty::EnumCase(..) => true,
        // `rule:types/object-top`'s inline shape, and `array<T>`, both
        // reachable exactly when what they hold is.
        Ty::Shape(fields) => fields
            .iter()
            .all(|field| json_reachable(field.ty, interner, signatures, exprs)),
        Ty::Array(elem) => json_reachable(*elem, interner, signatures, exprs),
        Ty::Class(class, _) => class_has_codec(class, signatures, exprs),
        // Everything else: `bytes` has no JSON spelling (`rule:types/bytes` makes it a
        // separate type for that reason), `object`, `callable` and `iterable`
        // name no contract, and a union of two non-`null` arms gives a
        // decoder nothing to pick between — § 2 lists none of them.
        _ => false,
    }
}

/// Whether `class` participates in the JSON wire format at all — `rule:core-classes/derive-field-list`'s "another class that itself has a codec — derived or hand-written".
fn class_has_codec(
    class: &QName,
    signatures: &crate::signatures::SignatureTable,
    exprs: &crate::expr_table::ExprTypeTable,
) -> bool {
    let label = class.to_string();
    if exprs.codec(&label).is_some() {
        return true;
    }
    match signatures.get(class) {
        // § 7's hand-written half, either one of them: a class writing only
        // `toJson` gets the decoder generated, so one declared half is a
        // codec as much as two are.
        Some(sig) => {
            sig.methods.contains_key(ENCODE)
                || sig.methods.contains_key(DECODE)
                // A `Core` value type — § 2's `Instant`, `Duration`, `Uuid`
                // and their siblings — declares no member for this to find:
                // which of them has a wire form is `nvs_stdlib::json`'s
                // roster, not this pass's, and that crate's § *A value type
                // crosses as text* is where the two that have one are named.
                || label.starts_with(r"Core\")
        }
        // A name with no signature is a name that did not resolve, and that
        // has already been reported where it was written; a second
        // diagnostic here would name the same property for the same reason.
        None => true,
    }
}

/// Records `decl`'s [`DerivedCodec`] for each of [`Format`]'s two values it
/// opts into, reporting every `rule:core-classes/derive-field-list`/§ 3/§ 6 rule it breaks.
///
/// A no-op — not even a walk of the members — for a class with no recognized
/// attribute, which `rule:core-classes/derive-generates-what-is-missing` requires: "a program with no derive attribute
/// pays nothing at all, including no pass". The two formats cost two scans of
/// the attribute groups written on the class and nothing else.
pub(crate) fn check_class_derive(
    decl: &ClassDecl,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    for format in Format::ALL {
        check_class_format(decl, class, format, ctx, env);
    }
}

/// [`check_class_derive`] for one format. Every rule below is `rule:core-classes/derive-attribute`'s,
/// stated for "a derived codec" and therefore asked of both.
fn check_class_format(
    decl: &ClassDecl,
    class: &QName,
    format: Format,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let Some(attribute) = attribute_span(&decl.attributes, format.derive(), ctx, env) else {
        return;
    };
    let derive = format.attribute();
    // `rule:core-classes/derive-generates-what-is-missing`: the derive generates only what the class does not write
    // itself, so a class writing every half gets nothing from it. Reported
    // before anything is collected and returning without a codec, because
    // "generates nothing" is the rule rather than a description of the error.
    // For a row that is one member, § 7's one-directional rule reaching the
    // same conclusion one step earlier.
    if format
        .halves()
        .iter()
        .all(|half| declares_method(decl, half, env))
    {
        let (declared, help) = match format {
            Format::Json => (
                format!("both `{ENCODE}` and `{DECODE}`"),
                "`rule:core-classes/derive-generates-what-is-missing`: the derive fills in the half a class does not write — keep one \
                 of the two and the attribute generates the other, or delete the attribute",
            ),
            Format::Db => (
                format!("`{DB_DECODE}`"),
                "`rule:core-classes/derive-generates-what-is-missing`: `Core\\Db\\Codec` declares `fromRow` and nothing else, so a \
                 class that writes it has left the attribute nothing to generate — delete one \
                 of the two",
            ),
        };
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_BOTH_HALVES,
                format!("`{class}` declares {declared}, so `{derive}` generates nothing"),
            )
            .with_primary(attribute, "this attribute has no effect")
            .with_help(help),
        );
        return;
    }
    let params = constructor_params(decl, env);
    let mut codec = DerivedCodec {
        ctor_arity: params.map_or(0, <[Param]>::len),
        ..DerivedCodec::default()
    };
    // Whether any property was *refused* rather than skipped — see the empty
    // contract check below, which a refusal must not also report.
    let mut refused = false;
    // § 2's declaration order, over both spellings of a declaration: the
    // members in the order they are written, and the constructor's promoted
    // parameters at the point the constructor itself appears. Walking the
    // members rather than concatenating two lists is what keeps the encode
    // order the one a reader sees in the file.
    for member in &decl.members {
        let fields: Vec<FieldDecl<'_>> = match &member.kind {
            ClassMemberKind::Property(p) if p.modifiers.contains(&Modifier::Static) => {
                continue; // ADR 0008: class storage, not instance storage
            }
            ClassMemberKind::Property(p) => vec![FieldDecl::property(p)],
            ClassMemberKind::Method(m) if span_text(env.src, m.name) == CONSTRUCTOR => m
                .params
                .iter()
                .filter(|p| p.is_promoted())
                .map(FieldDecl::promoted)
                .collect(),
            _ => continue,
        };
        for field in fields {
            match codec_field(&field, class, format, params, ctx, env) {
                FieldOutcome::Kept(field) => codec.fields.push(*field),
                FieldOutcome::Skipped => {}
                FieldOutcome::Refused => refused = true,
            }
        }
    }
    // § 2's field list is the declared property list, so no property is an
    // empty wire contract — § 7's rule about an attribute with no effect,
    // reaching the other way a derive can generate nothing. The codec is
    // still recorded: the program does not run with an error reported, and
    // recording it keeps every reader of the table off a special case.
    //
    // A class whose properties were all *refused* is excluded: it has already
    // been told what is wrong with each of them, and "the contract came out
    // empty" is the same mistake counted a second time. A class that skipped
    // them all in writing is not excluded — that is a contract it chose.
    if codec.fields.is_empty() && !refused {
        let contract = format.contract();
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_NO_FIELDS,
                format!("`{class}` derives {contract} with no fields in it"),
            )
            .with_primary(attribute, "no declared property reaches the wire contract")
            .with_help(
                "`rule:core-classes/derive-field-list`: the field list is the class's own declared instance properties, \
                 written as properties or promoted in the `constructor` — declare one, or \
                 delete the attribute",
            ),
        );
    }
    env.exprs.record_codec(format, class.to_string(), codec);
}

/// The two members [`docs/spec/01-core-library.md`] § 6's `Core\Json\Codec`
/// declares, which are also the two halves `rule:core-classes/derive-generates-what-is-missing` talks about.
const ENCODE: &str = "toJson";
/// The decoding half of [`ENCODE`] — `static fromJson(mixed $value): static`.
const DECODE: &str = "fromJson";
/// The one member § 6's `Core\Db\Codec` declares —
/// `static fromRow(Db\Row $row): static`. There is no encoding half at all:
/// `rule:core-classes/derive-generates-what-is-missing` makes the row format one-directional, so this is the whole of
/// what a `#[Db\Derive]` generates and the whole of what a class can write
/// instead.
const DB_DECODE: &str = "fromRow";

/// Whether `decl` writes a method named `want` **itself**.
///
/// Own members only: § 7 is about a class that hand-writes a half, and an
/// inherited `toJson` is the base class's declaration, whose own derive
/// question was already asked where it was written.
fn declares_method(decl: &ClassDecl, want: &str, env: &Env<'_>) -> bool {
    decl.members.iter().any(|member| match &member.kind {
        ClassMemberKind::Method(m) => span_text(env.src, m.name) == want,
        _ => false,
    })
}

/// The span of the first `want`-named attribute in `groups`, or `None` when
/// none of them is it.
///
/// Both a "does this class carry it" test and the site it matched at, because
/// the two refusals § 7 reaches put their primary span on the attribute
/// rather than on any member — a class with no property has no member to
/// point at, and the attribute is the thing to delete either way.
fn attribute_span(
    groups: &[AttributeGroup],
    want: &str,
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> Option<Span> {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .find(|attr| attribute_is(attr, want, ctx, env))
        .map(|attr| attr.span)
}

/// What one declared property contributed to the codec.
///
/// Three outcomes rather than an [`Option`] because the caller has to tell a
/// property left off the contract *in writing* from one that was refused: an
/// empty contract is an error, and a class whose every property was already
/// refused must not be told so twice.
enum FieldOutcome {
    /// A field, in declaration order, in the wire contract.
    Kept(Box<DerivedField>),
    /// `rule:core-classes/derive-field-list`'s `#[Json\Field(skip: true)]`.
    Skipped,
    /// Refused by § 2 or § 6, with the diagnostic already reported.
    Refused,
}

/// One declaration, as `rule:core-classes/derive-field-list`'s field list reads it.
///
/// A view rather than an enum over the two AST nodes, because § 2 asks a
/// property and a promoted constructor parameter exactly the same four
/// questions — what it is called, what it is declared, what modifiers it
/// carries and what attributes are on it — and every difference between
/// `public int $n;` and `constructor(public int $n)` is a difference in where
/// those four were written, not in what they mean. That is `rule:core-classes/derive-field-list`'s own
/// "for a class written with promoted parameters the two lists are literally
/// the same declaration", made true of this pass rather than assumed by it.
struct FieldDecl<'a> {
    /// `#[Json\Field(...)]` and whatever else was written on it.
    attributes: &'a [AttributeGroup],
    /// `readonly`, `lateinit`, a visibility — the same roster either way.
    modifiers: &'a [Modifier],
    /// The declared type, `None` only where the parser already reported its
    /// absence.
    ty: Option<&'a nvs_syntax::ast::Type>,
    /// The name, `$`-sigil included — and the span every § 2 refusal points
    /// at.
    name: Span,
}

impl<'a> FieldDecl<'a> {
    /// A property written in the class body.
    fn property(p: &'a PropertyMember) -> Self {
        Self {
            attributes: &p.attributes,
            modifiers: &p.modifiers,
            ty: Some(&p.ty),
            name: p.name,
        }
    }

    /// A constructor parameter promoted to a property — `rule:core-classes/derive-attribute`'s own
    /// example, and the normal way a codec class is written.
    ///
    /// The position is not carried: [`check_constructor_parameter`] finds the
    /// parameter by name in the same list this came out of, which is this
    /// parameter itself, so a promoted field is the one case that check cannot
    /// fail.
    fn promoted(p: &'a Param) -> Self {
        Self {
            attributes: &p.attributes,
            modifiers: &p.modifiers,
            ty: p.ty.as_ref(),
            name: p.name,
        }
    }
}

/// One declaration's [`DerivedField`], or why it is not one.
fn codec_field(
    p: &FieldDecl<'_>,
    class: &QName,
    format: Format,
    params: Option<&[Param]>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> FieldOutcome {
    let name = strip_sigil(span_text(env.src, p.name)).to_owned();
    let overrides = field_overrides(p.attributes, format, ctx, env);
    if overrides.skip {
        return FieldOutcome::Skipped;
    }
    let derive = format.attribute();
    let skip = format.skip_hint();
    // `rule:core-classes/derive-field-list`: `lateinit` is by definition not constructor-assigned, so
    // it can never be a field — reported before the parameter check, which
    // would otherwise report the same declaration twice.
    if p.modifiers.contains(&Modifier::Lateinit) {
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_LATEINIT_FIELD,
                format!("`${name}` is `lateinit`, so it cannot be a `{derive}` field"),
            )
            .with_primary(p.name, "assigned after the constructor, not by it")
            .with_help(format!(
                "`rule:core-classes/derive-field-list`: a decode is an ordinary `new`, and `rule:classes/lateinit` makes a `lateinit` \
                 property one the constructor does not assign — write `{skip}` on it"
            )),
        );
        return FieldOutcome::Refused;
    }
    let declared = crate::lower::lower_optional_type(p.ty, ctx, env);
    // `rule:security/derived-codec-qualifiers`: `rule:security/secret-qualifier`'s refusal, moved from wherever the value reached
    // the encoder to the declaration that put it on the wire contract.
    if is_secret(declared, env) {
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_SECRET_FIELD,
                format!("`${name}` is `secret`, so it cannot be a `{derive}` field"),
            )
            .with_primary(p.name, "a `secret` value has no wire form")
            .with_help(format!(
                "`rule:security/derived-codec-qualifiers`: encoding was already an `rule:security/secret-qualifier` sink — write `{skip}` to leave \
                 it off the contract in writing"
            )),
        );
        return FieldOutcome::Refused;
    }
    let resolved = check_constructor_parameter(p.name, &name, declared, format, params, ctx, env);
    let param = resolved.as_ref().map(|bound| bound.index);
    // A refused declaration reads as required rather than as optional: the
    // parameter that would have said otherwise is the one that is missing.
    let required = resolved.as_ref().is_none_or(|bound| bound.required);
    let default = resolved.and_then(|bound| bound.default);
    let nullable = env.interner.is_nullable(declared);
    // The `null` arm is what nullability *is*, so the decode target is the
    // rest of the union — `?int` decodes an `int` or a JSON null, never a
    // third thing.
    let carried = if nullable {
        env.interner.without_null(declared)
    } else {
        declared
    };
    // § 2's codec-reachable test, asked of the null-stripped type and asked
    // after the walk — `?T` is reachable exactly when `T` is, and "another
    // class that itself has a codec" is a question about the whole program.
    env.codec_sites.push(CodecFieldSite {
        class: class.to_string(),
        property: name.clone(),
        span: p.name,
        declared: carried,
        format,
    });
    let erased = codec_ty(carried, env.interner, env.enums);
    FieldOutcome::Kept(Box::new(DerivedField {
        key: overrides.name.unwrap_or_else(|| name.clone()),
        property: name,
        ty: erased.ty,
        element: erased.element,
        class: erased.class,
        cases: erased.cases,
        shape: erased.shape,
        nullable,
        required,
        param,
        default,
    }))
}

/// `rule:core-classes/derive-field-list`'s "every non-skipped field must also be a constructor
/// parameter of the same name and the same type".
///
/// Reports and returns a [`FieldParam`]: the parameter's *position*, which is
/// what a generated decoder fills, and the default it declares, which says both
/// whether the field is **required** and what fills it when it is not —
/// `rule:core-api/required-optional-and-nullable` puts optionality on the
/// default, and the parameter list is the one declaration that carries one.
/// The field is recorded either way, so one bad property does not silently
/// drop the rest of the wire contract.
///
/// A promoted parameter matches itself here, at its own position and its own
/// type, so neither refusal below can fire for one — which is § 2's "for a
/// class written with promoted parameters the two lists are literally the same
/// declaration" costing nothing, rather than being a case this pass skips.
///
/// `at` is the declaration's own name span, which both refusals point at.
fn check_constructor_parameter(
    at: Span,
    name: &str,
    declared: TypeId,
    format: Format,
    params: Option<&[Param]>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<FieldParam> {
    // A class with no written constructor has no parameter list to disagree
    // with, and `nvs_types::ctor_init` has already reported that its properties
    // are not definitely assigned (`rule:classes/definite-property-initialization`) — a second diagnostic here would
    // only bury that one.
    let params = params?;
    let Some((index, param)) = params
        .iter()
        .enumerate()
        .find(|(_, param)| strip_sigil(span_text(env.src, param.name)) == name)
    else {
        let derive = format.attribute();
        let skip = format.skip_hint();
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_FIELD_NOT_A_PARAMETER,
                format!("`${name}` is a `{derive}` field with no constructor parameter"),
            )
            .with_primary(at, "nothing decodes into this")
            .with_help(format!(
                "`rule:core-classes/derive-field-list`: a decode is an ordinary `new`, so every field needs a \
                 same-named constructor parameter — add one, or write `{skip}`"
            )),
        );
        return None;
    };
    // `rule:core-api/required-optional-and-nullable`: the default is where
    // optionality is written, so it is read here — beside the parameter — and
    // not from the property, which has no default of its own to carry. The
    // constant comes back with it, through the evaluator that reports nothing:
    // a default that is not a literal of its declared type is
    // `crate::signatures`' diagnostic to make, over the same expression.
    let param_ty = crate::lower::lower_optional_type(param.ty.as_ref(), ctx, env);
    let bound = FieldParam {
        index,
        required: param.default.is_none(),
        default: param
            .default
            .as_ref()
            .and_then(|expr| crate::defaults::literal_default(expr, param_ty, env)),
    };
    if param_ty == declared {
        return Some(bound);
    }
    let want = env.interner.describe(declared);
    let got = env.interner.describe(param_ty);
    env.diags.report(
        Diagnostic::error(
            code::E_DERIVE_FIELD_TYPE_MISMATCH,
            format!("`${name}` is declared `{want}` but its constructor parameter is `{got}`"),
        )
        .with_primary(param.name, format!("this is `{got}`"))
        .with_secondary(at, format!("the property is `{want}`"))
        .with_help(
            "`rule:core-classes/derive-field-list`: the decoder decodes into the property's declared type and passes \
             it to the constructor, so the two have to agree",
        ),
    );
    Some(bound)
}

/// The constructor parameter a derived field decodes into, as
/// [`check_constructor_parameter`] resolved it: one struct because the three
/// answers are one reading of one declaration.
struct FieldParam {
    /// Its position — [`DerivedField::param`].
    index: usize,
    /// Whether it declares no default — [`DerivedField::required`].
    required: bool,
    /// The constant that default evaluates to — [`DerivedField::default`].
    default: Option<crate::defaults::ConstArg>,
}

/// Whether `ty` carries `rule:security/secret-qualifier`'s `secret` qualifier.
fn is_secret(ty: TypeId, env: &Env<'_>) -> bool {
    matches!(
        env.interner.get(ty),
        Ty::SecretString | Ty::SecretBytes | Ty::SecretTaintedString | Ty::SecretTaintedBytes
    )
}

/// The written `constructor`'s parameter list, or `None` when the class
/// declares none. Found by name — `rule:classes/no-leading-underscore-identifiers` fixes the spelling.
fn constructor_params<'a>(decl: &'a ClassDecl, env: &Env<'_>) -> Option<&'a [Param]> {
    decl.members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(m) if span_text(env.src, m.name) == CONSTRUCTOR => {
            Some(m.params.as_slice())
        }
        _ => None,
    })
}

/// `rule:classes/no-leading-underscore-identifiers`'s one spelling of a constructor.
const CONSTRUCTOR: &str = "constructor";

/// `rule:core-classes/derive-field-list`'s two per-field options, as written on one property.
#[derive(Debug, Default)]
struct Overrides {
    /// `name: "..."`, if written.
    name: Option<String>,
    /// `skip: true`.
    skip: bool,
}

/// Reads `#[Json\Field(...)]` off one property, reporting anything that is not
/// `rule:core-classes/derive-field-list`'s two options with a literal of the right type.
fn field_overrides(
    groups: &[AttributeGroup],
    format: Format,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Overrides {
    let mut out = Overrides::default();
    let attribute = format.field_attribute();
    for field in attribute_fields(groups, format.field(), ctx, env) {
        match (span_text(env.src, field.name), &field.value.kind) {
            ("name", ExprKind::Str(span)) => {
                out.name = Some(crate::string_lit::cook_string_literal(env.src, *span));
            }
            ("skip", ExprKind::Bool(value)) => out.skip = *value,
            ("name" | "skip", _) => report_field_arg(
                field,
                "`name` takes a `string` literal and `skip` a `bool` literal".to_owned(),
                format,
                env,
            ),
            _ => report_field_arg(
                field,
                format!("`{attribute}` has exactly two options, `name` and `skip`"),
                format,
                env,
            ),
        }
    }
    out
}

/// One `E_DERIVE_FIELD_ATTRIBUTE`, at the offending field.
fn report_field_arg(field: &ObjectLiteralField, why: String, format: Format, env: &mut Env<'_>) {
    let signature = format.field_signature();
    env.diags.report(
        Diagnostic::error(code::E_DERIVE_FIELD_ATTRIBUTE, why)
            .with_primary(field.span, "not an option this attribute declares")
            .with_help(format!(
                "`rule:core-classes/derive-field-list`: `{signature}` — there is no whole-class naming policy and no \
                 third option"
            )),
    );
}

/// Whether one attribute is the named form spelling `want`. A bare
/// `#[{...}]` names nothing at all (`rule:attributes/attach-sites-and-forms`), so it is never one of
/// `rule:core-classes/derive-attribute`'s nominal attributes, and neither is
/// the member form `Owner::Name`, which always names a `type` alias.
pub(crate) fn attribute_is(attr: &Attribute, want: &str, ctx: &Ctx<'_>, env: &Env<'_>) -> bool {
    attr.member.is_none()
        && attr
            .name
            .as_ref()
            .is_some_and(|name| resolves_to(span_text(env.src, name.span), want, ctx))
}

/// Every field written on the *first* `want`-named attribute in `groups`.
///
/// The first: a second `#[Json\Field]` on one property is a mistake this pass
/// does not yet report, and reading both would silently merge two overrides.
fn attribute_fields<'a>(
    groups: &'a [AttributeGroup],
    want: &str,
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> &'a [ObjectLiteralField] {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .find(|attr| attribute_is(attr, want, ctx, env))
        .map(|attr| attr.fields.as_slice())
        .unwrap_or_default()
}

/// `rule:core-classes/derive-attribute`'s nominal match: `text`, resolved against the active namespace
/// and imports, is exactly `want`.
fn resolves_to(text: &str, want: &str, ctx: &Ctx<'_>) -> bool {
    nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports) == QName::parse(want)
}
