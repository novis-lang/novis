//! `rule:testing/property-testing`'s `#[Property]` attribute: what its payload may hold, and the
//! generator each of its parameters gets from its declared type.
//!
//! # A property is a test whose arguments are generated
//!
//! A `#[Property]` method has the shape a `#[Test]` method has — `public`, an instance method,
//! `: void`, with a body — and [`crate::testing::check_class_tests`] holds it to that shape in the
//! same walk. What differs is where its arguments come from. A test's parameters are injected
//! by a fixture or a data row; a property's are **all** generated, so a `#[TestWith]` row or a
//! second marker on one is refused there, and a fixture is never asked. Its rows ride in their
//! own table ([`crate::ExprTypeTable::properties`]) rather than in the `#[Test]` table, because a
//! runner that reads [`crate::testing::Injection`] has no answer for a generated parameter.
//!
//! # The generators come from the declared types, by the codec's walk
//!
//! [`Deriver::gen_of`] walks a parameter's declared type the way [`crate::derive`]'s `codec_ty`
//! erases one for a decoder: a scalar is a leaf, an `array<T>` and an inline shape recurse, an
//! enum is its case roster, and a class is its constructor's parameters. That last arm is what a
//! codec reaches through a class's own `#[Json\Derive]`; here it needs no attribute, because a
//! generated object is built with `new` and a constructor is what every concrete class has.
//! Each class is derived once per property and kept by label in [`PropertyCase::classes`], so a
//! class that reaches itself through a `?T`, a list or a union is a finite table.
//!
//! A type with no generator is [`code::E_PROPERTY_PARAMETER`] where the method is written: a
//! function, an interface, an abstract class, a generic class, a `Core` class, and a `secret`
//! value. The last one is a choice about the report, not the generator: stage 7's report prints
//! the failing input, and a `secret` value is the one kind no report may print.
//!
//! # `gen:` narrows one parameter
//!
//! `gen: { age: "0..120" }` names a parameter and an inclusive range. For a number the range bounds
//! the value; for a `string`, `bytes` or `array<T>` it bounds the length, in grapheme clusters for
//! a string. A `?T` parameter narrows its `T`. A length range ends at [`MAX_LENGTH`], and
//! [`generate()`]'s budget bounds the input as a whole, so no range a program writes makes the
//! generator allocate more than [`generate::MAX_INPUT_UNITS`] units for one call.
//!
//! **Memory:** one [`PropertyCase`] per property, built once per compile. At run time, one call's
//! input set is at most [`generate::MAX_INPUT_UNITS`] elements and bytes, per property in flight.

pub mod generate;
pub mod shrink;

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{AnonObjectField, Attribute, ExprKind, MethodMember};

use crate::defaults::ConstArg;
use crate::testing::OptionTy;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

pub use generate::{Generated, TooLarge, generate};

/// `#[Property]`'s option roster: `#[Test]`'s, less `retries:` and `because:`, plus `runs:`
/// and `gen:`. A property that fails is shrunk and reproduced from its seed, so a retry would
/// only hide the input that broke it.
pub(crate) const OPTIONS: &[(&str, OptionTy)] = &[
    ("skip", OptionTy::Str),
    ("at", OptionTy::Str),
    ("seed", OptionTy::Int),
    ("db", OptionTy::Str),
    ("server", OptionTy::Bool),
    ("timeout", OptionTy::Str),
    ("tags", OptionTy::StrList),
    ("runs", OptionTy::Int),
    // An anonymous object of parameter names and ranges, which no roster type states; it is
    // read against the method's own parameters in [`collect`].
    ("gen", OptionTy::Mixed),
];

/// How many calls a property gets when `runs:` is not written.
pub const DEFAULT_RUNS: u32 = 100;

/// The most calls `runs:` may ask for. A property is one test, and one test is meant to finish
/// in seconds.
pub const MAX_RUNS: u32 = 100_000;

/// The longest length a `gen:` range may give a `string`, `bytes` or `array<T>`.
pub const MAX_LENGTH: u32 = 10_000;

/// The length range a collection gets when `gen:` does not narrow it.
const DEFAULT_LENGTH: Len = Len { min: 0, max: 100 };

/// One `#[Property]` method of one class.
#[derive(Clone, Debug)]
pub struct PropertyCase {
    /// The method's own name, exactly as declared.
    pub method: String,
    /// Where the method's name is written.
    pub span: Span,
    /// The options written on the attribute, folded, in source order. `gen:` is not among them:
    /// it is already applied to [`Self::params`].
    pub options: Vec<(String, ConstArg)>,
    /// How many times the runner calls the method: `runs:`, or [`DEFAULT_RUNS`].
    pub runs: u32,
    /// One generator per declared parameter, in parameter order.
    pub params: Vec<Gen>,
    /// Every class [`Self::params`] reaches, by label, with one generator per constructor
    /// parameter. A [`Gen::Class`] names a row here.
    pub classes: Vec<ClassGen>,
}

impl PropertyCase {
    /// The constructor arguments' generators of the class labelled `label`.
    #[must_use]
    pub fn class(&self, label: &str) -> Option<&ClassGen> {
        self.classes.iter().find(|class| class.label == label)
    }
}

/// One class a property's inputs build with `new`.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassGen {
    /// The class's label, as [`crate::ExprTypeTable::tests`] keys a class.
    pub label: String,
    /// One generator per constructor parameter, in parameter order.
    pub args: Vec<Gen>,
}

/// An inclusive length range, in elements, bytes or grapheme clusters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Len {
    /// The shortest length.
    pub min: u32,
    /// The longest length, at most [`MAX_LENGTH`] where `gen:` wrote it.
    pub max: u32,
}

/// One inline shape field's generator.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeGen {
    /// The field's key.
    pub name: String,
    /// What generates its value.
    pub generator: Gen,
    /// Whether every value carries the field. An optional one is left out of some values.
    pub required: bool,
}

/// What generates one value, read off a declared type. Every `min` and `max` is inclusive.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "each range's `min` and `max`, and each `len` and `tainted`, mean one thing \
              wherever they are written, and the enum's own doc says it"
)]
pub enum Gen {
    /// A single-value type: `true`, `false`, a single-value string or integer.
    Literal(Generated),
    /// `bool`.
    Bool,
    /// `int`, in a range.
    Int { min: i64, max: i64 },
    /// `uint`, in a range.
    Uint { min: u64, max: u64 },
    /// A finite `float`, in a range.
    Float { min: f64, max: f64 },
    /// A `decimal` with up to four digits after the point, its whole part in the range.
    Decimal { min: i64, max: i64 },
    /// A valid UTF-8 string, its length counted in grapheme clusters, and `tainted` where the
    /// declared type is.
    Str { len: Len, tainted: bool },
    /// `bytes`, and `tainted` where the declared type is.
    Bytes { len: Len, tainted: bool },
    /// The backing values of an enum's cases, ascending, and whether the enum is `uint`-backed.
    Enum { values: Vec<i128>, uint: bool },
    /// `array<T>`, its `element` generating each `T`.
    List { element: Box<Gen>, len: Len },
    /// An inline shape, its fields in the order the type interns them.
    Shape(Vec<ShapeGen>),
    /// `?T`: `null`, or a value of `T`.
    Nullable(Box<Gen>),
    /// A union of two or more types that are not `null`, and `mixed`.
    OneOf(Vec<Gen>),
    /// A class built with `new`, its arguments generated by [`PropertyCase::classes`]' row of
    /// this label.
    Class(String),
}

impl Gen {
    /// The generator every scalar `mixed` may hold.
    fn mixed() -> Self {
        Self::Nullable(Box::new(Self::OneOf(vec![
            Self::Bool,
            Self::Int {
                min: i64::MIN,
                max: i64::MAX,
            },
            Self::Float {
                min: -f64::MAX,
                max: f64::MAX,
            },
            Self::Str {
                len: DEFAULT_LENGTH,
                tainted: false,
            },
        ])))
    }
}

/// One `#[Property]` payload, checked against [`OPTIONS`]. `gen:` is checked against the
/// method's parameters in [`collect`].
pub(crate) fn check_payload(fields: &[AnonObjectField], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    crate::attributes::check_roster("Property", OPTIONS, fields, ctx, env);
    crate::testing::check_option_values(fields, env);
    let int = env.interner.int();
    for field in fields {
        if span_text(env.src, field.name) != "runs" {
            continue;
        }
        let fault = match crate::defaults::literal_default(&field.value, int, env) {
            Some(ConstArg::Int(count)) if count < 1 => "is below 1, so the property never runs",
            Some(ConstArg::Int(count)) if count > i64::from(MAX_RUNS) => {
                "is above 100000, so one test would take too long"
            }
            _ => continue,
        };
        env.diags.report(
            Diagnostic::error(code::E_TEST_OPTION_VALUE, format!("`runs:` {fault}"))
                .with_primary(field.value.span, "this value")
                .with_help(
                    "write a number from 1 to 100000. Without `runs:`, a property runs 100 times",
                ),
        );
    }
}

/// The row of one `#[Property]` method, or `None` where a generator could not be derived or a
/// `gen:` entry does not fit — each already reported, so the table only holds a property the
/// runner can call.
pub(crate) fn collect(
    m: &MethodMember,
    method: &str,
    attr: &Attribute,
    class: &QName,
    env: &mut Env<'_>,
) -> Option<PropertyCase> {
    let sig = env.signatures.get(class)?.methods.get(method)?;
    let params = sig.params.clone();
    let names = sig.param_names.clone();
    let mut deriver = Deriver::default();
    let mut gens = Vec::with_capacity(params.len());
    let mut ok = true;
    for (position, &ty) in params.iter().enumerate() {
        // An unresolved type has reported already (`rule:types/one-mistake-reports-once`).
        if env.interner.is_error(ty) {
            return None;
        }
        match deriver.gen_of(ty, env) {
            Ok(generator) => gens.push(generator),
            Err(none) => {
                let parameter = names.get(position).map_or_else(
                    || format!("parameter {}", position + 1),
                    |n| format!("${n}"),
                );
                let declared = env.interner.describe(ty);
                env.diags.report(
                    Diagnostic::error(
                        code::E_PROPERTY_PARAMETER,
                        format!(
                            "`{parameter}` of the `#[Property]` method `{method}` is declared \
                             as `{declared}`, and {none}"
                        ),
                    )
                    .with_primary(m.name, format!("`{parameter}` has no random values"))
                    .with_help(
                        "a property test gets random values for scalars, strings, enums, \
                         arrays, shapes, and classes that can be created with `new`. Change \
                         the parameter's type, or write a `#[Test]` with `#[TestWith]` rows",
                    ),
                );
                ok = false;
            }
        }
    }
    if !ok {
        return None;
    }
    if let Some(written) = attr
        .fields
        .iter()
        .find(|field| span_text(env.src, field.name) == "gen")
        && !apply_gen(written, &names, &mut gens, env)
    {
        return None;
    }
    let options: Vec<(String, ConstArg)> = crate::testing::fold_options(&attr.fields, OPTIONS, env);
    let runs = options
        .iter()
        .find_map(|(name, value)| match value {
            ConstArg::Int(count) if name == "runs" => u32::try_from(*count).ok(),
            _ => None,
        })
        .unwrap_or(DEFAULT_RUNS)
        .clamp(1, MAX_RUNS);
    Some(PropertyCase {
        method: method.to_owned(),
        span: m.name,
        options,
        runs,
        params: gens,
        classes: deriver.classes,
    })
}

/// `gen:`'s entries applied to `gens`, each to the parameter it names. `false` once any entry
/// has been reported.
fn apply_gen(
    written: &AnonObjectField,
    names: &[String],
    gens: &mut [Gen],
    env: &mut Env<'_>,
) -> bool {
    let ExprKind::AnonObject(entries) = &written.value.kind else {
        report_gen(
            written.value.span,
            "`gen:` is not a list of parameters and ranges".to_owned(),
            env,
        );
        return false;
    };
    let string = env.interner.string();
    let mut ok = true;
    let mut seen: Vec<usize> = Vec::new();
    for entry in entries {
        let name = span_text(env.src, entry.name).to_owned();
        let Some(position) = names.iter().position(|declared| *declared == name) else {
            report_gen(
                entry.span,
                format!("`gen:` names `{name}`, and the method declares no parameter `${name}`"),
                env,
            );
            ok = false;
            continue;
        };
        if seen.contains(&position) {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("`gen:` gives `{name}` twice"),
                )
                .with_primary(entry.span, "already given above"),
            );
            ok = false;
            continue;
        }
        seen.push(position);
        let Some(ConstArg::Str(text)) = crate::defaults::literal_default(&entry.value, string, env)
        else {
            report_gen(
                entry.value.span,
                format!("the range for `{name}` is not a string such as `\"0..100\"`"),
                env,
            );
            ok = false;
            continue;
        };
        match narrow(&gens[position], &text) {
            Ok(narrowed) => gens[position] = narrowed,
            Err(why) => {
                report_gen(
                    entry.value.span,
                    format!("the range `{text}` for `{name}` {why}"),
                    env,
                );
                ok = false;
            }
        }
    }
    ok
}

/// One [`code::E_PROPERTY_PARAMETER`] about a `gen:` entry.
fn report_gen(at: Span, message: String, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_PROPERTY_PARAMETER, message)
            .with_primary(at, "this `gen:` entry does not fit the method")
            .with_help(
                "each `gen:` entry names a parameter and gives a range with both ends, such as \
                 `gen: { age: \"0..120\" }`. For a number the range limits the value. For a string, \
                 `bytes` or an array it limits the length",
            ),
    );
}

/// `gen` with the range `text` applied, or why it does not fit.
pub(crate) fn narrow(generator: &Gen, text: &str) -> Result<Gen, String> {
    let (low, high) = text
        .split_once("..")
        .map(|(low, high)| (low.trim(), high.trim()))
        .filter(|(low, high)| !low.is_empty() && !high.is_empty())
        .ok_or_else(|| "is not two numbers with `..` between them".to_owned())?;
    let not_a_number = || "has an end that is not a number".to_owned();
    let backwards = || "starts above its end".to_owned();
    Ok(match generator {
        Gen::Nullable(inner) => Gen::Nullable(Box::new(narrow(inner, text)?)),
        Gen::Int { .. } => {
            let (min, max) = (
                low.parse::<i64>().map_err(|_| not_a_number())?,
                high.parse::<i64>().map_err(|_| not_a_number())?,
            );
            if min > max {
                return Err(backwards());
            }
            Gen::Int { min, max }
        }
        Gen::Uint { .. } => {
            let (min, max) = (
                low.parse::<u64>().map_err(|_| not_a_number())?,
                high.parse::<u64>().map_err(|_| not_a_number())?,
            );
            if min > max {
                return Err(backwards());
            }
            Gen::Uint { min, max }
        }
        Gen::Decimal { .. } => {
            let (min, max) = (
                low.parse::<i64>().map_err(|_| not_a_number())?,
                high.parse::<i64>().map_err(|_| not_a_number())?,
            );
            if min > max {
                return Err(backwards());
            }
            Gen::Decimal { min, max }
        }
        Gen::Float { .. } => {
            let (min, max) = (
                low.parse::<f64>().map_err(|_| not_a_number())?,
                high.parse::<f64>().map_err(|_| not_a_number())?,
            );
            if !min.is_finite() || !max.is_finite() {
                return Err(not_a_number());
            }
            if min > max {
                return Err(backwards());
            }
            Gen::Float { min, max }
        }
        Gen::Str { tainted, .. } => Gen::Str {
            len: length(low, high)?,
            tainted: *tainted,
        },
        Gen::Bytes { tainted, .. } => Gen::Bytes {
            len: length(low, high)?,
            tainted: *tainted,
        },
        Gen::List { element, .. } => Gen::List {
            element: element.clone(),
            len: length(low, high)?,
        },
        _ => return Err("is for a number, a string, `bytes` or an array".to_owned()),
    })
}

/// A length range, held to `0..=MAX_LENGTH`.
fn length(low: &str, high: &str) -> Result<Len, String> {
    let (min, max) = match (low.parse::<u32>(), high.parse::<u32>()) {
        (Ok(min), Ok(max)) => (min, max),
        _ => return Err("is a length, and a length is a whole number of 0 or more".to_owned()),
    };
    if min > max {
        return Err("starts above its end".to_owned());
    }
    if max > MAX_LENGTH {
        return Err(format!(
            "ends above {MAX_LENGTH}, the longest length a property may ask for"
        ));
    }
    Ok(Len { min, max })
}

/// The walk from a declared type to its [`Gen`], with the classes it has reached.
#[derive(Default)]
struct Deriver {
    classes: Vec<ClassGen>,
    /// The classes being derived, outermost first.
    path: Vec<String>,
    /// For each entry of [`Self::path`], whether a `null`, a list or a union has been passed
    /// since it was entered. A class reached again with none between is a value that contains
    /// itself, which no `new` can build.
    escaped: Vec<bool>,
}

impl Deriver {
    /// `declared`'s generator, or the clause saying why it has none.
    fn gen_of(&mut self, declared: TypeId, env: &mut Env<'_>) -> Result<Gen, String> {
        if env.interner.is_nullable(declared) && !matches!(env.interner.get(declared), Ty::Null) {
            let inner = env.interner.without_null(declared);
            return self
                .escaping(|this| this.gen_of(inner, env))
                .map(|inner| Gen::Nullable(Box::new(inner)));
        }
        let none = |what: &str| Err(format!("{what} has no random values"));
        let ty = env.interner.get(declared).clone();
        Ok(match ty {
            Ty::Null => Gen::Literal(Generated::Null),
            Ty::Bool => Gen::Bool,
            Ty::True => Gen::Literal(Generated::Bool(true)),
            Ty::False => Gen::Literal(Generated::Bool(false)),
            Ty::Int => Gen::Int {
                min: i64::MIN,
                max: i64::MAX,
            },
            Ty::SingleValueInt(value) => Gen::Literal(Generated::Int(value)),
            Ty::Uint => Gen::Uint {
                min: 0,
                max: u64::MAX,
            },
            Ty::Float => Gen::Float {
                min: -f64::MAX,
                max: f64::MAX,
            },
            Ty::Decimal => Gen::Decimal {
                min: -1_000_000_000,
                max: 1_000_000_000,
            },
            Ty::String | Ty::TaintedString => Gen::Str {
                len: DEFAULT_LENGTH,
                tainted: matches!(ty, Ty::TaintedString),
            },
            Ty::SingleValueString(text) => Gen::Literal(Generated::Str(text)),
            Ty::Bytes | Ty::TaintedBytes => Gen::Bytes {
                len: DEFAULT_LENGTH,
                tainted: matches!(ty, Ty::TaintedBytes),
            },
            Ty::SecretString
            | Ty::SecretBytes
            | Ty::SecretTaintedString
            | Ty::SecretTaintedBytes => {
                return Err(
                    "a `secret` value has no random values, because a failing value is printed"
                        .to_owned(),
                );
            }
            Ty::Mixed => Gen::mixed(),
            Ty::Array(element) => {
                let element = self.escaping(|this| this.gen_of(element, env))?;
                Gen::List {
                    element: Box::new(element),
                    len: DEFAULT_LENGTH,
                }
            }
            Ty::Shape(fields) => {
                let mut gens = Vec::with_capacity(fields.len());
                for field in fields {
                    let generator = if field.required {
                        self.gen_of(field.ty, env)?
                    } else {
                        self.escaping(|this| this.gen_of(field.ty, env))?
                    };
                    gens.push(ShapeGen {
                        name: field.name,
                        generator,
                        required: field.required,
                    });
                }
                Gen::Shape(gens)
            }
            Ty::Enum(ref name, backing) | Ty::EnumCase(ref name, backing, _) => {
                let case = match &ty {
                    Ty::EnumCase(_, _, case) => Some(case.as_str()),
                    _ => None,
                };
                let values = enum_values(name, case, env);
                if values.is_empty() {
                    return none(&format!("`{name}` has no cases, so it"));
                }
                Gen::Enum {
                    values,
                    uint: backing == crate::enums::EnumBacking::Uint,
                }
            }
            Ty::Union(members) => {
                let mut arms = Vec::with_capacity(members.len());
                for member in members {
                    arms.push(self.escaping(|this| this.gen_of(member, env))?);
                }
                Gen::OneOf(arms)
            }
            Ty::Class(name, args) => {
                if !args.is_empty() {
                    return none(&format!("the generic class `{name}`"));
                }
                return self.class(&name, env);
            }
            _ => return none(&format!("`{}`", env.interner.describe(declared))),
        })
    }

    /// `walk`, run with every class on the path marked as passed through a `null`, a list or a
    /// union, and the marks put back after.
    fn escaping<T>(&mut self, walk: impl FnOnce(&mut Self) -> T) -> T {
        let saved = std::mem::replace(&mut self.escaped, vec![true; self.path.len()]);
        let result = walk(self);
        self.escaped = saved;
        result
    }

    /// The class `name`'s generator: its label, once its constructor's parameters are derived.
    fn class(&mut self, name: &QName, env: &mut Env<'_>) -> Result<Gen, String> {
        let label = name.to_string();
        if let Some(on) = self.path.iter().position(|entered| *entered == label) {
            if self.escaped[on] {
                return Ok(Gen::Class(label));
            }
            return Err(format!(
                "`{label}` contains itself with no `null`, array or union between, so no \
                 value of it can be created"
            ));
        }
        if self.classes.iter().any(|class| class.label == label) {
            return Ok(Gen::Class(label));
        }
        let is_class =
            env.symbols.get(name).map(|symbol| symbol.kind) == Some(nvs_hir::SymbolKind::Class);
        if !is_class {
            return Err(format!(
                "`{label}` is not a class of this program, so it has no random values"
            ));
        }
        if env.graph.get(name).map(|links| links.concrete) == Some(false) {
            return Err(format!("`{label}` is `abstract`, so it has no instances"));
        }
        let constructor = env
            .signatures
            .get(name)
            .and_then(|class| class.methods.get("constructor"));
        if let Some(sig) = constructor
            && sig.visibility != nvs_syntax::ast::Visibility::Public
        {
            return Err(format!("`{label}`'s constructor is not `public`"));
        }
        let params = constructor
            .map(|sig| sig.params.clone())
            .unwrap_or_default();
        self.path.push(label.clone());
        self.escaped.push(false);
        let mut args = Vec::with_capacity(params.len());
        let mut failed = None;
        for ty in params {
            match self.gen_of(ty, env) {
                Ok(generator) => args.push(generator),
                Err(why) => {
                    failed = Some(why);
                    break;
                }
            }
        }
        self.path.pop();
        self.escaped.pop();
        if let Some(why) = failed {
            return Err(format!(
                "`{label}` cannot be created, because a constructor parameter's type {why}"
            ));
        }
        self.classes.push(ClassGen {
            label: label.clone(),
            args,
        });
        Ok(Gen::Class(label))
    }
}

/// The backing values of `name`'s cases, ascending, or the one value of `case`.
fn enum_values(name: &QName, case: Option<&str>, env: &Env<'_>) -> Vec<i128> {
    let Some(info) = env.enums.get(name) else {
        return Vec::new();
    };
    let value = |value: &crate::enums::EnumValue| match value {
        crate::enums::EnumValue::Int(number) => i128::from(*number),
        crate::enums::EnumValue::Uint(number) => i128::from(*number),
    };
    let mut values: Vec<i128> = match case {
        Some(case) => info.cases.get(case).map(value).into_iter().collect(),
        None => info.cases.values().map(value).collect(),
    };
    values.sort_unstable();
    values
}
