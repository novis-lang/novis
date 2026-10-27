//! Per-class property and method signatures — what property, method-call and
//! `new` expression typing read: `nvs_hir::members` checks only whether a
//! member *exists*, never what type it has, since that needs this crate's type
//! table.
//!
//! [`build_signatures`] walks a file once, ahead of any body-checking, the
//! same shape [`crate::check::check_program`] itself walks (mirroring
//! `nvs_hir::members`'s own two-pass split): a property's declared type and
//! a method's parameter/return types are lowered the same way
//! `check::check_method` lowers a method body's own parameters, into a
//! [`SignatureTable`] every method body is then checked against.
//! [`resolve_property`]/[`resolve_method`] look a name up on a class and,
//! failing that, walk its `extends`/`implements` ancestors via
//! [`nvs_hir::ClassGraph`] — the same ancestor walk
//! `nvs_hir::members::member_declared` already does for existence-only
//! checking.
//!
//! **A class constant declared with no annotation takes its type from the value
//! [`crate::consts`] folded it to**, and an ineligible value with no annotation
//! is `mixed` — [`ConstSig`] owns that rule.
//!
//! **A promoted constructor parameter is an ordinary property here**
//! (`rule:classes/promotion-is-constructor-only`): `record_promoted_properties`
//! records the type the parameter declares and the level its keyword names, so
//! [`property_visibility`] answers a `constructor(private int $x)` exactly as
//! it answers a written declaration and [`is_visible_from`] is reached for
//! both, while `nvs_hir::members`'s own table records the name for the
//! existence-only half. A keyword on any other method's parameter promotes
//! nothing and is refused (`reject_promotion_outside_constructor`).
//!
//! **A variadic parameter's slot holds the type of *each* trailing argument,
//! and the arity rule is [`MethodSig::variadic`] beside it.** A tail is not a
//! type of its own on this side: [`MethodSig::param_at`] matches every argument
//! from that position onward against the element, which is the check
//! [`crate::expr`] performs, so nothing here spells the tail's own `array<T>`.
//! The body is handed the one array the call site packed, and `crate::check` is
//! where that binding wraps the element type once.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_hir::{ClassGraph, QName, SymbolKind};
use nvs_stdlib::registry::{ParamText, Qual};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, Modifier, NamespaceDecl, PropertyMember, Stmt, StmtKind, Type,
    TypeAtom, TypeKind, Visibility,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lower::{lower_optional_type, lower_type};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// One method's own declared shape: its parameters' types, in declaration
/// order, and its return type. Never includes an inherited override —
/// walking through `extends`/`implements` to find one is [`resolve_method`]'s
/// job, not this type's.
#[derive(Clone, Debug)]
pub struct MethodSig {
    /// Each parameter's declared type (`mixed` for one written with none —
    /// already diagnosed elsewhere).
    pub params: Vec<TypeId>,
    /// Each parameter's own name without the `$`, positionally — one entry per
    /// [`Self::params`] entry, for **every** signature this crate builds.
    ///
    /// There is no "nameless signature" case left to spell, and `rule:core-api/shape-rules` R2 is
    /// why: a `Core` row's names come from
    /// `nvs_stdlib::registry::CoreMethod::names` through [`crate::core_lib`],
    /// the synthesized `Throwable` constructor's from [`crate::error_lib`],
    /// and a reserved interface's from the rule that declares it
    /// ([`crate::iter_lib`]). So a `name:` that reaches no parameter is one
    /// refusal — `E_UNKNOWN_ARG_NAME` — wherever it is written, and a member
    /// taking no parameters names an empty list rather than nothing at all.
    ///
    /// Read it through [`Self::param_index`] rather than indexed directly, so
    /// the "a variadic tail cannot be filled by name" rule stays in one place.
    pub param_names: Vec<String>,
    /// Whether each parameter is declared `inout $x`, positionally — one entry per
    /// [`Self::params`] entry, read through [`Self::is_inout`] rather than
    /// indexed directly so the variadic rule stays in one place.
    ///
    /// A parallel `Vec` rather than a field on a per-parameter struct because
    /// every existing consumer reads [`Self::params`] positionally already
    /// (see [`Self::param_at`]), and an `inout` parameter is rare enough that
    /// turning one `Vec<TypeId>` into a `Vec<Param>` would rewrite every one
    /// of those call sites to buy nothing.
    pub inout: Vec<bool>,
    /// Whether each parameter is declared `covariant`, positionally, and
    /// **empty** for a signature that declares none, which is every signature
    /// but a user method's. Read through [`Self::is_covariant`]
    /// (`rule:classes/a-covariant-type-is-checked-where-a-value-enters`).
    pub covariant: Vec<bool>,
    /// Whether the last parameter is `...$x` — every argument from that
    /// position onward is checked against its type instead of requiring an
    /// exact count.
    pub variadic: bool,
    /// Each parameter's already-evaluated default, positionally — one entry
    /// per [`Self::params`] entry, `None` for a parameter a call must supply.
    /// [`crate::defaults`] owns what a default may be and why it is evaluated
    /// here rather than in the callee.
    ///
    /// One entry is present without making its parameter optional, and it is
    /// the only exception: a
    /// [`ConstArg::RequiredShape`](crate::defaults::ConstArg::RequiredShape)
    /// records `rule:core-api/shape-flattens-at-the-abi`'s per-field fills for a shape parameter a call
    /// must still write. That is why [`Self::required`] is the one reader
    /// allowed to answer "is this parameter optional" off this vector.
    ///
    /// A parallel `Vec` for [`Self::inout`]'s reason, and read through
    /// [`Self::required`] rather than scanned at each call site.
    pub defaults: Vec<Option<crate::defaults::ConstArg>>,
    /// What the member does with each parameter's qualifier — `rule:security/unclassified-parameter-refuses-tainted`'s
    /// classification, positionally, and **empty** for a signature that is not
    /// a `Core` row.
    ///
    /// Empty rather than a vector of some default, because "unclassified" and
    /// "`Contagious`" are different answers: the judgement belongs to the
    /// registry row that wrote it ([`nvs_stdlib::registry::Qual`] holds the
    /// rule a class is classified by), and a user-declared method's parameters
    /// have never been classified at all. A `Core` row fills one entry per
    /// [`Self::params`] entry from `nvs_stdlib::registry::CoreTy::classification`,
    /// `None` where the parameter's type carries no mark.
    ///
    /// A parallel `Vec` for [`Self::inout`]'s reason, and read through
    /// [`Self::qual_at`] so the variadic rule stays in one place. The checker
    /// asks this question of a whole argument, so a mark nested in an options
    /// bag is not carried, and one on an `array<…>` element is carried only
    /// when it is `Contagious` — the registry's `classification` owns why.
    pub param_quals: Vec<Option<Qual>>,
    /// What each parameter's text names, positionally — `rule:programs/relative-paths-resolve-from-their-file`'s
    /// mark, read through [`Self::text_at`]. **Empty** means every parameter is
    /// [`ParamText::Plain`], which is the answer for every signature that
    /// marks nothing.
    ///
    /// A `Core` row fills it from `nvs_stdlib::registry::CoreMethod::param_text`;
    /// a user method marks a parameter [`ParamText::Path`] with `#[Core\Path]`.
    /// `crate::paths` is the one reader: it resolves a relative string literal
    /// written at a [`ParamText::Path`] parameter.
    pub param_text: Vec<ParamText>,
    /// The type parameters a **call site** must write, in the order its
    /// `<...>` list binds them — `["T"]` for `Core\Json::decodeAs<T>`, and
    /// empty for everything else.
    ///
    /// Only a `Core` member registered through [`crate::core_lib`] can have
    /// any: `rule:types/declaration` parks user-declared generics, so
    /// [`build_signatures`] always writes an empty list here. It is *not* the
    /// list of every variable the signature mentions — an inferred one
    /// ([`nvs_stdlib::registry::CoreTy::Var`]) is bound from an argument's
    /// type and is deliberately not writable, so the two kinds are separated
    /// at the registry and stay separated here.
    pub type_params: Vec<String>,
    /// Each bounded inferred variable and the type it must fit — `("T",
    /// int|float|decimal)` for `Core\Math::abs`, and empty for everything
    /// else. `nvs_stdlib::registry::CoreTy::Bounded` owns the rule;
    /// `crate::expr::args`' `check_generic_args` applies it after binding.
    pub type_bounds: Vec<(String, TypeId)>,
    /// The declared return type (`mixed` if omitted).
    pub return_ty: TypeId,
    /// What a call answers when an argument at a [`Qual::Contagious`]
    /// parameter carried `tainted`, when the signature says so itself; `None`
    /// answers `crate::expr::quals::tainted_result` of [`Self::return_ty`].
    ///
    /// A `Core` member that decodes a document into its written class writes
    /// that class (`crate::core_lib`'s `decodes_a_document`), and the decode
    /// site carries the mark into the class's text fields.
    ///
    /// An extension method writes one too, a built-in component's engine
    /// included (`rule:security/extension-contagion`): its declared return with
    /// `tainted` on every `string` and `bytes`, the fields of a shape included,
    /// which `tainted_result` does not reach into.
    /// `crate::ext_lib` owns why a shape is reached there and nowhere else, and
    /// leaves it `None` for a return with no `string` or `bytes` in it, which
    /// refuses a `tainted` argument as every such signature does. A `Some`
    /// admits one (`crate::expr::quals::admits_tainted_argument_at`).
    pub contagious_return: Option<TypeId>,
    /// Whether the declaration writes `static` as its **return type** — late
    /// static binding (`rule:statements/static-is-a-member-modifier`), seen as
    /// a type rather than as a dispatch.
    ///
    /// `static` means the class the *call* named, which
    /// [`crate::lower::lower_type`] cannot intern: inside the body it has no
    /// choice but the declaring class, and that is sound there (every called
    /// class is one). At a call site the called class *is* known, so
    /// `crate::expr::calls` answers `Leaf` for `Leaf::make()` on a
    /// `Base::make(): static`, through [`Self::bound_to`] and
    /// [`Self::return_binds_static`]. This bit is the exact spelling alone,
    /// which an interface's implementor must keep (`E0404`).
    ///
    /// # Why the body is refused rather than the hole pinned
    ///
    /// Substituting the called class is only sound if the body actually
    /// produces one. `Base::make(): static { return new self(); }` type-checks
    /// against the declaring class and would then hand a `Leaf`-typed binding
    /// a `Base`; PHP raises a `TypeError` there at run time and Novis has no
    /// such check below the type system, so `crate::check` refuses the body
    /// instead (`E0741`). The three alternatives were weighed and each is
    /// worse: interning `static` as a *distinct* type is the principled fix
    /// and rewrites every consumer of a `TypeId`; setting this bit only for a
    /// body that provably forwards would weaken a call site silently, which
    /// `rule:types/declaration` exists to prevent; and accepting the hole would
    /// break that rule to keep the compiler simple, which no ordering allows.
    ///
    /// [`crate::expr_table::ResolvedCall`] deliberately keeps the *declared*
    /// return type rather than the substituted one: `nvs-ir` reads that record
    /// for a representation, and a class and its subclass erase to the same
    /// `Ty::Object`.
    pub returns_static: bool,
    /// Which parameters bind `static` to the class a call names, positionally,
    /// and **empty** when none does. [`static_use`] decides each one, and
    /// [`Self::bound_to`] applies them at a call.
    pub params_bind_static: Vec<bool>,
    /// Whether the return type binds `static` to the class a call names. True
    /// for every return [`Self::returns_static`] is true for, and also for a
    /// `?static`, a `static[]` or any other type [`static_use`] admits.
    /// `crate::check` holds the body to the same promise (`E0741`).
    pub return_binds_static: bool,
    /// Whether the declaration carries the `static` modifier — `rule:statements/static-is-a-member-modifier`'s
    /// one surviving meaning of the keyword.
    ///
    /// Recorded because a call's *shape* does not settle it: `parent::method()`
    /// and `self::method()` are written like a static call but invoke an
    /// instance method with the enclosing `$this` whenever the target is not
    /// static, which is how a subclass constructor reaches its parent's. A
    /// consumer that got this wrong would pass `null` where the callee expects
    /// a receiver.
    pub is_static: bool,
    /// Whether this is a `private` interface method (`rule:classes/interface-private-methods`) —
    /// declared with the `private` modifier inside an `interface`, not a
    /// `class`. Kept alongside [`Self::visibility`], which records the same
    /// keyword, because `rule:classes/interface-private-methods` is a *different* rule than `rule:core-api/written-visibility`'s
    /// level with the same name: a private interface method is not part of
    /// that interface's contract, so it is never reachable outside that
    /// interface's own method bodies, not even from an implementing class —
    /// and its diagnostic says so, which is why
    /// `crate::expr::members::check_method_visibility` reports it and stops
    /// rather than reporting both.
    pub interface_private: bool,
    /// `rule:core-api/written-visibility`'s level, as this declaration wrote it — `public` where
    /// nothing did, which is every synthesized and `Core`-installed method
    /// (only user source can write a keyword at all) and every user
    /// declaration `nvs_syntax::check_declarations` is already refusing with
    /// `E_MISSING_VISIBILITY`.
    ///
    /// Stored on the signature rather than in a side map the way
    /// [`ClassSignature::property_visibility`] is, because a method's
    /// signature is already the thing every call site holds: `resolve_method`
    /// hands back the declaring [`QName`] with it, and those two together are
    /// exactly what [`is_visible_from`] asks for.
    pub visibility: Visibility,
    /// Whether the declaration carries a *body* — false for an `abstract`
    /// method and for an interface method declared without a default
    /// (`rule:classes/interface-default-methods`).
    ///
    /// Recorded for the same reason [`Self::is_static`] is: the call site
    /// cannot see it. A call resolving to a bodiless declaration has no
    /// compiled function to name, so it must dispatch on the receiver's
    /// runtime class instead — which is exactly what an interface default
    /// method calling back into the contract it declares
    /// (`$this->name()` inside `Greets::greet`) does.
    pub has_body: bool,
    /// Whether the declaration's body holds a `yield`, so a call returns a
    /// generator's state object (`rule:iteration/generators`). `false` for every
    /// signature that is not a user-declared method with a body.
    ///
    /// Recorded because a call site's type cannot say it: a generator returns an
    /// `Iterator<T>`, which a program's own classes implement too. The one reader
    /// is `crate::expr::members::reject_generator_clone`.
    pub generator: bool,
}

impl MethodSig {
    /// How many arguments a call must supply — the number of leading
    /// parameters with no [`Self::defaults`] entry.
    ///
    /// Derived rather than stored so it cannot disagree with `defaults`.
    /// `E_PARAM_DEFAULT_ORDER` makes "leading" exact: a required parameter
    /// after an optional one is refused at the declaration, so the two are
    /// never interleaved and this count is also the index of the first
    /// optional parameter.
    ///
    /// A **variadic** tail never counts, whatever its `defaults` entry says:
    /// `...$rest` already accepts zero arguments, so it carries no default to
    /// be optional *by*, and counting it would make every call to
    /// `Core\Str::format` look one argument short.
    ///
    /// A [`ConstArg::RequiredShape`](crate::defaults::ConstArg::RequiredShape)
    /// entry does not end the leading run either: `rule:core-api/shape-flattens-at-the-abi`'s fill list is
    /// a property of the parameter's *type* — what a written literal's missing
    /// keys pass — and a parameter that carries one is written at every call
    /// site. `Self::defaults`' own docs name this as its one exception.
    #[must_use]
    pub fn required(&self) -> usize {
        let leading = self
            .defaults
            .iter()
            .take_while(|default| {
                matches!(
                    default,
                    None | Some(crate::defaults::ConstArg::RequiredShape(_))
                )
            })
            .count();
        match self.variadic {
            true => leading.min(self.params.len().saturating_sub(1)),
            false => leading,
        }
    }

    /// The parameter type at `index`, following the variadic rule: every
    /// argument from the last parameter's position onward is checked against
    /// that parameter's own type. `None` for an index past a non-variadic
    /// signature's parameters, which the arity check has already reported.
    #[must_use]
    pub fn param_at(&self, index: usize) -> Option<TypeId> {
        if self.variadic && index >= self.params.len().saturating_sub(1) {
            return self.params.last().copied();
        }
        self.params.get(index).copied()
    }

    /// The index a `name:` argument fills, or `None` where the name reaches
    /// no parameter a call may fill by name.
    ///
    /// The variadic tail is deliberately excluded: it is one `array<T>` built
    /// at the call site out of the arguments written into it, so a name has
    /// nowhere to be recorded there — which is the one reason a name that *is*
    /// in [`Self::param_names`] still answers `None`.
    #[must_use]
    pub fn param_index(&self, name: &str) -> Option<usize> {
        let index = self.param_names.iter().position(|p| p == name)?;
        let fillable = match self.variadic {
            true => self.params.len().saturating_sub(1),
            false => self.params.len(),
        };
        (index < fillable).then_some(index)
    }

    /// Whether the parameter at `index` is declared `inout $x`, following the same
    /// variadic rule [`Self::param_at`] does — every argument from a variadic
    /// parameter's position onward binds the way that parameter declares.
    /// `false` for an index past a non-variadic signature's parameters, which
    /// the arity check has already reported.
    #[must_use]
    pub fn is_inout(&self, index: usize) -> bool {
        if self.variadic && index >= self.inout.len().saturating_sub(1) {
            return self.inout.last().copied().unwrap_or(false);
        }
        self.inout.get(index).copied().unwrap_or(false)
    }

    /// Whether the parameter at `index` is declared `covariant`, the same way
    /// [`Self::is_inout`] reads `inout`. `false` for every parameter of a
    /// signature whose [`Self::covariant`] is empty.
    #[must_use]
    pub fn is_covariant(&self, index: usize) -> bool {
        if self.variadic && index >= self.covariant.len().saturating_sub(1) {
            return self.covariant.last().copied().unwrap_or(false);
        }
        self.covariant.get(index).copied().unwrap_or(false)
    }

    /// `rule:security/unclassified-parameter-refuses-tainted`'s classification of the parameter an argument at `index`
    /// fills — `None` where this signature is not a `Core` row, or where that
    /// parameter's type carries no classification.
    ///
    /// The variadic rule is [`Self::is_inout`]'s: every argument from the
    /// tail's position onward fills the tail, so it answers the tail's own
    /// classification rather than nothing at all.
    #[must_use]
    pub fn qual_at(&self, index: usize) -> Option<Qual> {
        if self.variadic && index >= self.param_quals.len().saturating_sub(1) {
            return self.param_quals.last().copied().flatten();
        }
        self.param_quals.get(index).copied().flatten()
    }

    /// What the text of the parameter filled by argument position `index`
    /// names — [`Self::param_text`]'s entry, with a variadic tail answering for
    /// every position from its own onward, and [`ParamText::Plain`] wherever
    /// the vector says nothing.
    #[must_use]
    pub fn text_at(&self, index: usize) -> ParamText {
        if self.variadic
            && !self.param_text.is_empty()
            && index >= self.param_text.len().saturating_sub(1)
        {
            return self.param_text.last().copied().unwrap_or_default();
        }
        self.param_text.get(index).copied().unwrap_or_default()
    }

    /// Whether any parameter is declared `inout $x` — the cheap test a call site
    /// runs before doing any by-reference work at all.
    #[must_use]
    pub fn has_inout(&self) -> bool {
        self.inout.iter().any(|&r| r)
    }

    /// Whether this signature mentions a type variable anywhere — the test
    /// that decides whether a call site needs [`crate::generics`] at all.
    /// Always false for a user-declared signature: `rule:types/declaration` parks
    /// user-declared generics, so only a `Core` member registered through
    /// [`crate::core_lib`] can answer true.
    #[must_use]
    pub fn is_generic(&self, interner: &crate::ty::TypeInterner) -> bool {
        self.params
            .iter()
            .chain(std::iter::once(&self.return_ty))
            .any(|id| crate::generics::mentions_type_var(*id, interner))
    }

    /// This signature with `bindings` applied to every parameter and to the
    /// return type — see [`crate::generics`] for the binding rule and for why
    /// an unbound variable becomes `mixed`.
    #[must_use]
    pub(crate) fn substituted(
        self,
        bindings: &crate::generics::Bindings,
        interner: &mut crate::ty::TypeInterner,
    ) -> Self {
        Self {
            params: self
                .params
                .iter()
                .map(|id| crate::generics::substitute(*id, bindings, interner))
                .collect(),
            return_ty: crate::generics::substitute(self.return_ty, bindings, interner),
            contagious_return: self
                .contagious_return
                .map(|id| crate::generics::substitute(id, bindings, interner)),
            ..self
        }
    }

    /// This signature as a call on `called` sees it, when `declaring` declares
    /// it: each position that binds `static` has the declaring class replaced
    /// by `called` ([`bind_static`]). A variadic tail's one entry covers every
    /// trailing argument, because [`Self::params`] holds the element type.
    #[must_use]
    pub(crate) fn bound_to(
        &self,
        declaring: &QName,
        called: TypeId,
        interner: &mut crate::ty::TypeInterner,
    ) -> Self {
        let mut out = self.clone();
        for (ty, binds) in out.params.iter_mut().zip(&self.params_bind_static) {
            if *binds {
                *ty = bind_static(*ty, declaring, called, interner);
            }
        }
        if self.return_binds_static {
            out.return_ty = bind_static(self.return_ty, declaring, called, interner);
        }
        out
    }

    /// Whether any position of this signature binds `static` at a call.
    #[must_use]
    pub fn binds_static(&self) -> bool {
        self.return_binds_static || self.params_bind_static.iter().any(|b| *b)
    }
}

/// Which of a property's two `rule:classes/property-hooks` hooks a declaration actually
/// writes with a body. A property with neither has no entry in
/// [`ClassSignature::hooked_properties`] at all.
///
/// # Every hooked property is still *backed*
///
/// PHP 8.4 splits hooked properties into "backed" (some hook body mentions
/// `$this->thatSameProperty`, so the slot is kept) and "virtual" (no hook
/// mentions it, so the slot is dropped). Novis keeps the slot either way —
/// [`crate::layout`] gives every declared property a slot, hooked or not —
/// which is why nothing here records backedness. The trade is one machine
/// word per instance for a property whose hooks never touch storage, bought
/// against an AST walk over every hook body, a second layout rule, and a
/// second legality rule for what a hook body may say. AGENTS.md's priority
/// ordering puts simplicity above footprint and names exactly this shape of
/// trade; the word is per *instance* of a class that declares a virtual
/// hooked property, so it is O(in-flight objects), not O(traffic).
///
/// The observable consequence is that a hooked property always has somewhere
/// to store: `$this->p = v` inside the declaring class stores into that slot
/// and a `get` hook reading `$this->p` sees it, whether or not PHP would
/// have called the property virtual. From *outside* the declaring class a
/// `get`-only property is read-only — `expr::assign`'s
/// `reject_get_only_hook_write` owns that rule and why it is scope-shaped
/// rather than backedness-shaped, which is the one place `rule:classes/property-hooks`'s
/// "same as PHP 8.4" is narrower than PHP.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PropertyHooks {
    /// A `get` hook with a body is declared.
    pub get: bool,
    /// A `set` hook with a body is declared.
    pub set: bool,
}

/// The label the compiled function for `class`'s `$name` property hook is
/// emitted under — `Ns\Class::$prop::get`.
///
/// Spelled here rather than at either end so a hook's *definition* (recorded
/// by [`crate::check`] through `ExprTypeTable::record_method`) and its *call*
/// site (`ExprInfo::HookedProperty`) agree by construction, exactly the
/// reason `ExprTypeTable::method_label` exists for an ordinary method. The
/// `$` is what keeps a hook label out of any method's namespace: a class name
/// never contains `::`, so `A::$b::get` can only ever be a hook.
#[must_use]
pub fn hook_label(class: &QName, name: &str, kind: nvs_syntax::ast::PropertyHookKind) -> String {
    let accessor = match kind {
        nvs_syntax::ast::PropertyHookKind::Get => "get",
        nvs_syntax::ast::PropertyHookKind::Set => "set",
    };
    format!("{class}::${name}::{accessor}")
}

/// The declaring class's label back out of a [`hook_label`] — `Ns\Class` for
/// `Ns\Class::$prop::get`. `None` for any string that is not a hook label.
///
/// The inverse lives beside the spelling for the same reason the spelling is
/// centralised at all: `nvs-ir` has to name the field a `set => expr;` hook
/// stores into, and it holds the hook's label but has no `QName` machinery of
/// its own to rebuild one from. Splitting here keeps both halves of the
/// format in one file, so a change to it cannot leave the two disagreeing.
#[must_use]
pub fn hook_label_class(label: &str) -> Option<&str> {
    let (class, rest) = label.split_once("::$")?;
    (rest.ends_with("::get") || rest.ends_with("::set")).then_some(class)
}

/// One class/interface/enum's own directly-declared property types and
/// method signatures — never anything pulled in via `extends`/`implements`;
/// walking those is [`resolve_property`]/[`resolve_method`]'s job, done at
/// check time.
#[derive(Clone, Debug, Default)]
pub struct ClassSignature {
    /// Instance property types, keyed by name with the `$` sigil stripped.
    pub properties: FxHashMap<String, TypeId>,
    /// This declaration's own properties' **read** visibility (`rule:core-api/asymmetric-visibility-is-a-pair`:
    /// the plain keyword of an asymmetric pair is the read half, and
    /// `private(set)`'s write half is a separate rule this map does not
    /// model). Read through [`property_visibility`], never directly — a
    /// property with no entry is `public`, which is what a `Core` class
    /// installed through [`SignatureTable::seed_class`] and every synthesized
    /// declaration is, since only user source can write a level at all.
    pub property_visibility: FxHashMap<String, Visibility>,
    /// This declaration's own properties that write a `(set)` half, such as
    /// `public private(set)`, by name. Read through
    /// [`property_set_visibility`]. A write through a union of classes
    /// compares it across the classes
    /// (`rule:types/a-member-every-class-of-a-union-declares`).
    pub property_set_visibility: FxHashMap<String, Visibility>,
    /// This declaration's own properties that carry an `rule:classes/property-hooks` hook
    /// block, by name — see [`PropertyHooks`]. A property with no hooks, or
    /// whose hooks are all bodiless (an abstract hook in an interface), is
    /// absent.
    pub hooked_properties: FxHashMap<String, PropertyHooks>,
    /// This declaration's own properties that carry a written `= expr`
    /// default, already evaluated into the constant every fresh instance's
    /// slot is written with — name and value, in declaration order.
    ///
    /// Own properties only, exactly like every other map here: an inherited
    /// property's default belongs to the class that declared it, and the two
    /// are joined against the flattened slot order in `nvs_ir::lower`, which
    /// is the one place both this table and `crate::layout`'s slots are in
    /// hand. `crate::defaults` owns what a default may be and where it ends up
    /// at run time; a `static` property is excluded, since it occupies no
    /// instance slot for anything to be written into — it takes
    /// [`Self::static_properties`] instead.
    pub property_defaults: Vec<(String, crate::defaults::ConstArg)>,
    /// This declaration's own `static` properties — **every one of them**, in
    /// declaration order, each with its evaluated initializer.
    ///
    /// Separate from [`Self::property_defaults`] because the two are joined
    /// against different things: an instance default is written into a slot of
    /// the flattened *layout*, and a static's is materialized once per request
    /// into `nvs_runtime::Ctx`'s own slot vector (that module's docs own the
    /// lifetime). A static property is never inherited into a second slot —
    /// `Sub::$count` and `Base::$count` are the one storage PHP makes them —
    /// so this is read per declaring class and never flattened.
    ///
    /// Every static appears, initializer or not: this is what `nvs_ir::lower`
    /// enumerates the program's slots from, so a static missing here has no
    /// storage at all. A `None` initializer is one `rule:classes/definite-property-initialization` required no
    /// default of — a nullable or `lateinit` static — and its slot starts each
    /// request at `null`, the same "an absent default is the zeroed slot"
    /// convention an instance property already has.
    pub static_properties: Vec<(String, Option<crate::defaults::ConstArg>)>,
    /// Method signatures, keyed by method name.
    pub methods: FxHashMap<String, MethodSig>,
    /// This declaration's own properties that `rule:classes/definite-property-initialization` requires a
    /// constructor to definitely assign: non-nullable (`TypeInterner::is_nullable`
    /// is false), no inline default, and a backing slot something reads. A
    /// hooked property's slot is read when it has no `get` hook, or when a hook
    /// names `$this->prop` other than as an `=` target; a property whose hooks
    /// compute its value reads none. A `lateinit` property is
    /// also excluded (`rule:classes/lateinit-restrictions`: it is exempt from this obligation by
    /// design, not merely by accident of shape). Name, declaration span, in
    /// declaration order.
    pub required_properties: Vec<(String, Span)>,
    /// The hooked properties in [`Self::required_properties`] only because
    /// this declaration writes a `set` hook and no `get` hook, so a plain read
    /// reads the slot. [`own_required_properties`] drops one whose ancestor
    /// declares a `get` hook for it, since that hook is what a read runs.
    pub read_without_get: FxHashSet<String>,
    /// This declaration's own properties declared `lateinit` (`rule:classes/lateinit-restrictions`),
    /// by name. Never includes one pulled in from an `extends`/`implements`
    /// ancestor — `crate::lateinit` walks those in.
    pub lateinit_properties: FxHashSet<String>,
    /// This declaration's own properties declared `readonly` (`rule:classes/lateinit-restrictions`),
    /// by name, a promoted constructor parameter's included. Own properties
    /// only, like every other map here: the modifier is written where the
    /// property is declared, so [`property_is_readonly`] asks this of the
    /// declaring class [`resolve_property_owned`] found and never of the class
    /// a write happened to name.
    pub readonly_properties: FxHashSet<String>,
    /// Whether the declaration was written `final` — no class may name it as
    /// its superclass ([`class_is_final`]).
    pub is_final: bool,
    /// This declaration's own methods written `final`, by name — no subclass
    /// may redeclare one ([`method_is_final`]).
    ///
    /// A set beside [`Self::methods`] rather than a flag on [`MethodSig`] for
    /// the reason [`Self::readonly_properties`] is one: the modifier is a fact
    /// about the *declaration*, asked only where a subclass's own member is
    /// being weighed against its ancestors', while a `MethodSig` is what every
    /// call site matches arguments against and is built by four seeding
    /// modules that have no modifier to report.
    pub final_methods: FxHashSet<String>,
    /// This declaration's own `implements` entries, in source order, each
    /// with the concrete type arguments it fixed (`rule:iteration/concrete-generic-implements`). Empty
    /// arguments for every interface but `Iterable`/`Iterator`, which is
    /// every interface in the language today except those two.
    ///
    /// [`nvs_hir::ClassGraph`] already records *which* interfaces a class
    /// implements, and is the right table for a reachability question. This
    /// one exists because the arguments need [`TypeId`]s, which `nvs-hir` has
    /// no interner for — so a question like "what does a `foreach` over a
    /// `Counter` yield" is answered here and the plain "does `Counter` reach
    /// `Iterable` at all" stays there.
    pub implements: Vec<(QName, Vec<TypeId>)>,
    /// This declaration's own method names that some *subtype* redeclares —
    /// the set that decides whether `$obj->m()` can be bound to a compiled
    /// label at all, or has to dispatch on the receiver's runtime class.
    ///
    /// Filled by [`mark_overridden_methods`] once the whole table is
    /// collected, because the question is about declarations this one has
    /// never heard of: a class cannot know who extends it. Empty for the
    /// overwhelming majority of classes, which is the point — an
    /// instance call only pays for a name lookup where the language
    /// actually admits two answers (`nvs_ir::ir::InstKind::CallVirtual`).
    pub overridden_methods: FxHashSet<String>,
    /// This declaration's own class constants (`rule:classes/no-free-functions-or-constants`),
    /// by name — the type a *read* of one answers with, and the value that
    /// read is emitted as. Own declarations only, exactly like every other map
    /// here; [`resolve_const`] walks the ancestors.
    pub constants: FxHashMap<String, ConstSig>,
    /// Whether this class is a loaded extension's, seeded by [`crate::ext_lib`] from its manifest.
    /// A call to one of its methods is a call of the export's trampoline, not of a compiled
    /// Novis function ([`crate::expr_table::ResolvedCall::extension`]).
    pub extension: bool,
    /// On a built-in component's manifest class, the component's name and the class's card,
    /// which names the public class a program calls instead. A call to it from outside the
    /// component's own Novis source does not compile
    /// (`rule:packaging/a-components-engine-class-is-internal`).
    pub engine: Option<(String, String)>,
}

/// One class constant, as a **read** of it sees it: the type
/// `Class::CONST` answers with, and the value it is emitted as.
///
/// Collected here rather than in [`crate::consts`] because both halves need
/// the interner, and [`crate::consts::build_const_table`] deliberately runs
/// before the first annotation is interned — `rule:types/constant-in-type-position`'s `Foo::TYPE_A` in
/// *type* position is folded there, out of the written literal alone, and that
/// pass has to be complete before this one begins. So the two tables answer
/// two different questions about the same declaration and neither is a copy of
/// the other: that one holds what the written value folds to, this one holds
/// what the declaration's own annotation says a use of it *is*.
#[derive(Clone, Debug)]
pub struct ConstSig {
    /// The constant's declared type.
    ///
    /// The written annotation where there is one — a class constant's type is
    /// optional per PHP 8.3 and [`nvs_syntax`] accepts it absent, so the other
    /// two rows are not hypothetical. With none written, the type of the value
    /// [`crate::consts`] folded it to, which is the closest thing to a
    /// declaration the source contains; and `mixed` where that fold produced
    /// [`crate::consts::ConstValue::Ineligible`], since an `array` or object
    /// constant has no type this table could name.
    pub ty: TypeId,
    /// The constant's value, already placed in [`Self::ty`] by
    /// [`crate::defaults::eval_const_value`] — which is
    /// [`crate::defaults::literal_default`], the same routine that places a
    /// property default in its own declared type, plus the array literal a
    /// constant may be and a default may not. So `const uint MAX = 3;` yields a
    /// [`crate::defaults::ConstArg::Uint`] rather than an `Int` for `nvs-ir` to
    /// emit at the wrong representation, and `const array<int> ROWS = [1, 2];`
    /// yields the [`crate::defaults::ConstArg::Array`] `nvs-ir` lowers to one
    /// `nvs_ir::ir::InstKind::ArrayNew`.
    ///
    /// `None` for a value with no constant form at all — a *nested* reference
    /// to another class's constant, an enum case, or `Foo::class`, none of
    /// which has a constant emitter. A read of one is
    /// `code::E_CLASS_CONST_NO_CONSTANT_FORM`, reported by
    /// [`crate::expr::members`] so that `nvs-ir`'s `ClassConstAccess` arm is
    /// never reached with nothing to lower.
    pub value: Option<crate::defaults::ConstArg>,
    /// Whether the declaration already reported that its value does not have
    /// its type (`reject_const_value_of_another_type`). A read of such a
    /// constant is then not a second diagnostic about the same mistake.
    pub refused: bool,
    /// The written visibility, `public` where none is written.
    /// [`crate::expr::members`] checks a read against it with
    /// [`is_visible_from`], as it checks a property.
    pub visibility: Visibility,
}

/// Every declaration's own [`ClassSignature`], keyed by its [`QName`].
///
/// The native declarations — `Core`, the error classes and the iteration
/// interfaces — are a frozen `base` that [`crate::core_lib::base`] builds once
/// per process and every check reads. A program's own declarations go in
/// `by_class`. A base entry that a check has to change, such as an overridden
/// method mark, is copied into `by_class` first, and from then on that copy
/// is the one the check reads.
#[derive(Debug, Default)]
pub struct SignatureTable {
    base: Option<&'static FxHashMap<QName, ClassSignature>>,
    by_class: FxHashMap<QName, ClassSignature>,
    property_default_types: Vec<(Span, TypeId)>,
    /// The name of every instance property some class declares `covariant`.
    /// A write to a property whose name is not here is never tested, which
    /// keeps the test's class walk off every other write. See
    /// [`Self::is_narrowed`].
    narrowed_properties: FxHashSet<String>,
}

impl SignatureTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One declaration's own signatures, if any were collected under this
    /// name.
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&ClassSignature> {
        // A `Core` class's signatures are its registry row, seeded from the whole registry, so a
        // lookup here is the program's use of that row, and of every class its members' types
        // name.
        if nvs_footprint::enabled() && qname.is_core() {
            nvs_stdlib::registry::record_signature(&qname.to_string());
        }
        self.by_class
            .get(qname)
            .or_else(|| self.base.and_then(|base| base.get(qname)))
    }

    /// Every declaration in the table, name and signatures, in no particular
    /// order — the whole-table view [`mark_overridden_methods`] needs, since
    /// "does anything extend me" is not a question the extended class's own
    /// entry can answer.
    pub fn iter(&self) -> impl Iterator<Item = (&QName, &ClassSignature)> {
        let base = self
            .base
            .into_iter()
            .flatten()
            .filter(|(qname, _)| !self.by_class.contains_key(*qname));
        self.by_class.iter().chain(base)
    }

    /// A table whose native declarations are `base`, with nothing of a
    /// program's own in it yet.
    pub(crate) fn over(base: &'static FxHashMap<QName, ClassSignature>) -> Self {
        Self {
            base: Some(base),
            ..Self::default()
        }
    }

    /// Every entry this table holds itself, for [`crate::core_lib::base`] to
    /// freeze.
    pub(crate) fn into_classes(self) -> FxHashMap<QName, ClassSignature> {
        self.by_class
    }

    /// Every property initializer written in the program, paired with the
    /// declared type it is written into.
    ///
    /// Keyed by the *initializer's* span rather than the annotation's, because
    /// the one consumer asks at the expression: a `secret` property's default
    /// is bytes an editor conceals
    /// (`rule:ide/redaction-covers-bytes-only`), and the walk that
    /// answers that reaches the default as a node and never sees the
    /// annotation at all.
    ///
    /// **Kept here rather than recorded straight into the expression table,
    /// and that is the whole point of it.** Signature collection lowers its
    /// annotations against a throwaway [`crate::expr_table::ExprTypeTable`]
    /// (see [`build_signatures`]) so that nothing it interns reaches
    /// `nvs-ir`. Handing it the real table instead would be the one-line
    /// version of this, and it would also put every property annotation's span
    /// under [`crate::expr_table::ExprTypeTable::declared_ty`] — which
    /// `nvs_ir::lower::lower_decl_type` consults *first*, so a property
    /// annotation would silently start taking a different lowering path than
    /// it takes today. This vector crosses the seam carrying only what the
    /// editor asked for.
    pub fn property_default_types(&self) -> &[(Span, TypeId)] {
        &self.property_default_types
    }

    /// Whether some class declares an instance property named `property`
    /// `covariant` (`rule:classes/a-covariant-type-is-checked-where-a-value-enters`).
    #[must_use]
    pub fn is_narrowed(&self, property: &str) -> bool {
        self.narrowed_properties.contains(property)
    }

    /// Records that some class declares the instance property `property`
    /// `covariant`.
    pub(crate) fn record_narrowed(&mut self, property: String) {
        self.narrowed_properties.insert(property);
    }

    /// Whether `owner`'s own declaration of `method` can be reached by a
    /// receiver whose runtime class answers `method` with different code —
    /// see [`ClassSignature::overridden_methods`].
    #[must_use]
    pub fn is_overridden(&self, owner: &QName, method: &str) -> bool {
        self.get(owner)
            .is_some_and(|sig| sig.overridden_methods.contains(method))
    }

    fn entry(&mut self, qname: QName) -> &mut ClassSignature {
        let base = self.base;
        self.by_class.entry(qname).or_insert_with_key(|qname| {
            base.and_then(|base| base.get(qname))
                .cloned()
                .unwrap_or_default()
        })
    }

    /// Installs a whole class's properties and method signatures at once,
    /// with no constructor obligations — the one shape a *native* declaration
    /// has, since [`crate::core_lib`] and [`crate::error_lib`] are the only
    /// callers and neither has source text to collect from. Deliberately not
    /// a general insertion point: everything else goes through
    /// [`build_signatures`]'s own walk.
    ///
    /// `required_properties` stays empty on purpose. `rule:classes/definite-property-initialization`'s obligation is
    /// a check on a *written* constructor, and neither caller has one — a
    /// `Core` class has no state at all, and `Throwable`'s constructor is
    /// synthesized by `nvs_ir::lower`.
    pub(crate) fn seed_class(
        &mut self,
        qname: QName,
        properties: FxHashMap<String, TypeId>,
        methods: FxHashMap<String, MethodSig>,
    ) {
        let entry = self.entry(qname);
        entry.properties = properties;
        entry.methods = methods;
    }

    /// Records that every property [`seed_class`](Self::seed_class) gave `qname` is `readonly`.
    /// Only [`crate::error_lib`] calls it: `Throwable`'s properties are written by the synthesized
    /// constructor and by nothing after it (`rule:classes/readonly-is-written-once`).
    pub(crate) fn seed_readonly(&mut self, qname: QName) {
        let entry = self.entry(qname);
        entry.readonly_properties = entry.properties.keys().cloned().collect();
    }

    /// Records that a seeded class implements `interface` at `args` — the
    /// [`ClassSignature::implements`] half of [`seed_class`](Self::seed_class),
    /// which [`resolve_iteration_element`] and [`resolve_interface_args`] read.
    ///
    /// Separate from `seed_class` because only `Core`'s § 9 collections have
    /// anything to say here (`nvs_stdlib::registry::ITERABLES`), and threading
    /// an empty vector through every other seeded class would say nothing four
    /// hundred times.
    pub(crate) fn seed_implements(&mut self, qname: QName, interface: QName, args: Vec<TypeId>) {
        self.entry(qname).implements.push((interface, args));
    }

    /// Installs a seeded class's constants. Only [`crate::ext_lib`] calls it:
    /// a `Core` constant is read from its registry row where it is written
    /// ([`crate::core_lib::constant`]), and an extension's has no row to read.
    pub(crate) fn seed_constants(&mut self, qname: QName, constants: FxHashMap<String, ConstSig>) {
        self.entry(qname).constants = constants;
    }

    /// Records that `qname` is a loaded extension's class. Only [`crate::ext_lib`] calls it.
    pub(crate) fn seed_extension(&mut self, qname: QName) {
        self.entry(qname).extension = true;
    }

    /// Records that `qname` is the manifest class of the built-in component `component`, whose
    /// card is `help`. Only [`crate::ext_lib`] calls it.
    pub(crate) fn seed_engine(&mut self, qname: QName, component: String, help: String) {
        self.entry(qname).engine = Some((component, help));
    }

    /// The component and the card of `qname` when it is a built-in component's manifest class —
    /// [`ClassSignature::engine`].
    #[must_use]
    pub fn engine(&self, qname: &QName) -> Option<(&str, &str)> {
        self.get(qname)
            .and_then(|sig| sig.engine.as_ref())
            .map(|(component, help)| (component.as_str(), help.as_str()))
    }

    /// Whether `qname` is a loaded extension's class — [`ClassSignature::extension`].
    #[must_use]
    pub fn is_extension(&self, qname: &QName) -> bool {
        self.get(qname).is_some_and(|sig| sig.extension)
    }
}

/// Builds a [`SignatureTable`] for every class/interface/enum declared in
/// any file of the program, lowering every property/parameter/return type
/// through the same [`nvs_hir::AliasTable`]/[`nvs_hir::SymbolTable`] a method body's own types
/// go through.
///
/// The whole [`crate::ProgramFile`] slice is collected before
/// [`mark_overridden_methods`] runs, because an override is a fact about the
/// *program*: a subclass in a `require`d file overriding a method declared in
/// the entry file is exactly the case a per-file table would answer wrong,
/// and it decides `Call` against `CallVirtual` in `nvs-ir`.
///
/// This writes into its own `table` return value rather than `env.signatures`
/// — the [`Env`] this function builds internally points `signatures` at an
/// unrelated, empty placeholder (never read during collection, only during
/// later body-checking), which is what lets this run *before* the table it
/// produces exists: [`crate::Env`] borrows `interner`/`diags` mutably, and a
/// `SignatureTable` under active construction can't also be borrowed
/// immutably through the same `Env` at once.
pub fn build_signatures(
    files: &[crate::ProgramFile<'_>],
    module: &nvs_hir::Module,
    enums: &crate::enums::EnumTable,
    consts: &crate::consts::ConstTable,
    extensions: &[nvs_ext::manifest::Manifest],
    interner: &mut crate::ty::TypeInterner,
    diags: &mut Diagnostics,
) -> SignatureTable {
    let (symbols, aliases, graph) = (&module.symbols, &module.aliases, &module.graph);
    // `Core` first, so a user declaration can never be collected under a name
    // the stdlib already owns without the later insertion being visible. A
    // fresh interner reads the process's frozen copy. One that already has
    // types in it would give the native types other ids, so it seeds its own.
    let mut table = if interner.is_empty() {
        let base = crate::core_lib::base();
        interner.layer_over(&base.interner);
        SignatureTable::over(&base.classes)
    } else {
        let mut table = SignatureTable::default();
        crate::core_lib::seed_natives(&mut table, interner);
        table
    };
    // The loaded extension set next, still ahead of every user declaration. It
    // is the configuration's and not the binary's, so it is a layer over the
    // frozen base rather than part of it — see `crate::ext_lib`.
    crate::ext_lib::seed(&mut table, interner, extensions);
    let placeholder = SignatureTable::default();
    // Same placeholder idea as `signatures` above: signature collection only
    // ever lowers property/parameter/return *type annotations*, never a call
    // expression, so nothing during this pass ever records into `exprs`.
    let mut placeholder_exprs = crate::expr_table::ExprTypeTable::default();
    // And the same for `rule:routing/table-is-opt-in`'s rows: they are collected by
    // `crate::check`'s per-class walk, which is a later pass than this one.
    let mut placeholder_routes = crate::routes::RouteTable::default();
    let mut placeholder_commands = crate::commands::CommandTable::default();
    let mut placeholder_links = Vec::new();
    // And again: this pass reaches no `#[Json\Derive]` class body, so the
    // sites it would collect are none and the vector is never read.
    let mut placeholder_codec_sites = Vec::new();
    // And once more: a `queryAs<T>` and a `jsonAs<T>` are expressions too.
    let mut placeholder_row_sites = Vec::new();
    let mut placeholder_json_sites = Vec::new();
    let mut placeholder_decode_sites = Vec::new();
    let mut placeholder_encode_sites = Vec::new();
    // Same again: `rule:attributes/structural-retrieval`'s retrieval is an expression, and this pass
    // checks none, so the table it reads is empty here rather than built twice.
    let empty_attributes = crate::retrieval::AttributeTable::default();
    let empty_deprecations = crate::deprecated::Deprecations::default();
    let empty_test_code = crate::test_code::TestCode::default();
    for file in files {
        let mut env = Env {
            symbols,
            aliases,
            graph,
            signatures: &placeholder,
            enums,
            consts,
            attributes: &empty_attributes,
            deprecations: &empty_deprecations,
            test_code: &empty_test_code,
            // This pass checks no expression, so nothing in it can ask a
            // capability question — the placeholder tables beside it are here
            // for the same reason.
            grants: None,
            namespace_grants: &nvs_config::grants::EMPTY,
            packages: None,
            src: file.src,
            stmts: file.stmts,
            files,
            interner: &mut *interner,
            exprs: &mut placeholder_exprs,
            routes: &mut placeholder_routes,
            commands: &mut placeholder_commands,
            links: &mut placeholder_links,
            codec_sites: &mut placeholder_codec_sites,
            row_sites: &mut placeholder_row_sites,
            json_sites: &mut placeholder_json_sites,
            decode_sites: &mut placeholder_decode_sites,
            encode_sites: &mut placeholder_encode_sites,
            diags: &mut *diags,
            anon_fn_seq: 0,
            refused_exprs: 0,
            fn_self: None,
            exit_targets: Vec::new(),
            write_target_levels: FxHashMap::default(),
            unset_target: false,
            coalesce_guarded: FxHashSet::default(),
            method_ref_args: FxHashSet::default(),
            body_writers: crate::response::BodyWriters::default(),
            in_call_argument: false,
            deprecated_uses: None,
        };
        collect_stmts(file.stmts, &[], &FxHashMap::default(), &mut table, &mut env);
    }
    mark_overridden_methods(&mut table, graph);
    table
}

/// Fills every [`ClassSignature::overridden_methods`] set, once the whole
/// table is collected.
///
/// The question — "can a call that statically resolves to `Owner::m` land on
/// a different body at run time" — is one no single declaration can answer,
/// because it is about subtypes it has never heard of. So it is asked here,
/// once per program, rather than per call site: `nvs_types::expr` records the
/// answer on every [`crate::expr_table::ResolvedCall`] it builds, and
/// `nvs-ir` reads it back to pick
/// [`InstKind::Call`](../../nvs_ir/ir/enum.InstKind.html) or `CallVirtual`.
///
/// A redeclaration counts whether or not it has a body of its own: an
/// `abstract` override still means a *further* subclass supplies one, and
/// over-marking only costs a name lookup, while under-marking calls the
/// wrong code. The ancestor's own declaration must have a body, though —
/// without one there is no static label to devirtualize *to*, and the call
/// already dispatches on the receiver for that separate reason.
fn mark_overridden_methods(table: &mut SignatureTable, graph: &ClassGraph) {
    let mut marks: Vec<(QName, String)> = Vec::new();
    for (qname, sig) in table.iter() {
        // Most entries are native classes with no supertype in the program's
        // graph, and none of their methods can override anything.
        let direct = supertypes(qname, graph);
        if direct.is_empty() {
            continue;
        }
        for name in sig.methods.keys() {
            let mut seen = FxHashSet::default();
            seen.insert(qname.clone());
            let mut queue: Vec<QName> = direct.clone();
            while let Some(ancestor) = queue.pop() {
                if !seen.insert(ancestor.clone()) {
                    continue;
                }
                if table
                    .get(&ancestor)
                    .and_then(|found| found.methods.get(name))
                    .is_some_and(|found| found.has_body)
                {
                    marks.push((ancestor.clone(), name.clone()));
                }
                queue.extend(supertypes(&ancestor, graph));
            }
        }
    }
    for (qname, method) in marks {
        table.entry(qname).overridden_methods.insert(method);
    }
}

/// One declaration's direct `extends` and `implements` targets, together —
/// the step [`mark_overridden_methods`] walks upwards, matching
/// [`resolve_method`]'s own chain exactly so the two agree on what "inherits
/// from" means.
fn supertypes(qname: &QName, graph: &ClassGraph) -> Vec<QName> {
    graph.get(qname).map_or_else(Vec::new, |links| {
        links
            .extends
            .iter()
            .chain(links.implements.iter())
            .cloned()
            .collect()
    })
}

fn qname_segments(src: &SourceFile, name: &nvs_syntax::ast::Name) -> Vec<String> {
    QName::parse(span_text(src, name.span)).segments().to_vec()
}

fn collect_stmts(
    stmts: &[Stmt],
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    table: &mut SignatureTable,
    env: &mut Env<'_>,
) {
    let mut current_ns: Vec<String> = namespace.to_vec();
    let mut current_imports: FxHashMap<String, QName> = imports.clone();

    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name
                    .as_ref()
                    .map_or_else(Vec::new, |n| qname_segments(env.src, n));
                match body {
                    Some(block) => {
                        collect_stmts(&block.stmts, &new_ns, &FxHashMap::default(), table, env);
                    }
                    None => {
                        current_ns = new_ns;
                        current_imports.clear();
                    }
                }
            }
            StmtKind::UseDecl(use_decl) => {
                let target = QName::parse(span_text(env.src, use_decl.path.span));
                current_imports.insert(target.short_name().to_owned(), target);
            }
            StmtKind::ClassDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                    generator_elem: None,
                    in_constructor: false,
                    in_anon_fn: false,
                    in_test_code: false,
                };
                // Before the members: `rule:iteration/concrete-generic-implements`'s type arguments are part
                // of the declaration's own shape, not of any one member's.
                let implements: Vec<(QName, Vec<TypeId>)> = decl
                    .implements
                    .iter()
                    .map(|clause| crate::lower::lower_implemented_interface(clause, &ctx, env))
                    .collect();
                table.entry(qname.clone()).implements = implements;
                table.entry(qname.clone()).is_final = decl.modifiers.contains(&Modifier::Final);
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                    generator_elem: None,
                    in_constructor: false,
                    in_anon_fn: false,
                    in_test_code: false,
                };
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                    generator_elem: None,
                    in_constructor: false,
                    in_anon_fn: false,
                    in_test_code: false,
                };
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            _ => {}
        }
    }
}

/// The one plain `public`/`protected`/`private` keyword a member declaration
/// carries, if it wrote one. [`Modifier::SetVisibility`] is deliberately not
/// read here: `private(set)` is the *write* half of `rule:core-api/asymmetric-visibility-is-a-pair`'s pair, and
/// the read half is always the plain keyword written alongside it.
///
/// `None` where nothing was written — which `nvs_syntax::check_declarations`
/// already reports as `E_MISSING_VISIBILITY`, so this only has to not invent
/// a level for source that is already being refused.
fn declared_set_visibility(modifiers: &[Modifier]) -> Option<Visibility> {
    modifiers.iter().find_map(|modifier| match modifier {
        Modifier::SetVisibility(level) => Some(*level),
        _ => None,
    })
}

/// The `(set)` half `owner::$name` was declared with, or `None` where the
/// declaration wrote none. `owner` is the declaring class
/// [`resolve_property_owned`] returned, as for [`property_visibility`].
#[must_use]
pub fn property_set_visibility(
    owner: &QName,
    name: &str,
    table: &SignatureTable,
) -> Option<Visibility> {
    table
        .get(owner)
        .and_then(|sig| sig.property_set_visibility.get(name).copied())
}

fn declared_visibility(modifiers: &[Modifier]) -> Option<Visibility> {
    modifiers.iter().find_map(|modifier| match modifier {
        Modifier::Public => Some(Visibility::Public),
        Modifier::Protected => Some(Visibility::Protected),
        Modifier::Private => Some(Visibility::Private),
        _ => None,
    })
}

fn collect_members(
    members: &[ClassMember],
    qname: &QName,
    ctx: &Ctx<'_>,
    table: &mut SignatureTable,
    env: &mut Env<'_>,
) {
    // Needed only to decide `MethodSig::interface_private` below — a
    // `private` method modifier means something (`rule:classes/interface-private-methods`) exactly when
    // the enclosing declaration is an `interface`, not a `class`/`enum`.
    let is_interface = env
        .symbols
        .get(qname)
        .is_some_and(|sym| sym.kind == SymbolKind::Interface);
    reject_members_declared_twice(members, qname, env);
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                let ty = lower_type(&p.ty, ctx, env);
                let text = span_text(env.src, p.name);
                let name = strip_sigil(text).to_owned();
                let void_or_never = reject_void_or_never_property(p, ty, env);
                if p.covariant && p.modifiers.contains(&Modifier::Static) {
                    report_misplaced_covariant(p.ty.span, "a static property", env);
                } else if p.covariant && p.modifiers.contains(&Modifier::Readonly) {
                    report_misplaced_covariant(p.ty.span, "a `readonly` property", env);
                } else if p.covariant {
                    table.record_narrowed(name.clone());
                }
                let is_lateinit = p.modifiers.contains(&Modifier::Lateinit);
                if is_lateinit {
                    check_lateinit_property(p, env);
                }
                let hooks = declared_hooks(p);
                // A hooked property needs a value in its slot when something
                // reads the slot: a plain read where there is no `get` hook,
                // or a hook that reads `$this->name`. A property whose hooks
                // compute the value from other properties reads no slot.
                let reads_own_slot = p.hooks.is_none() || hooks_read_own_slot(p, &name, env.src);
                // A `set`-only redeclaration may inherit its parent's `get`
                // hook, which [`own_required_properties`] asks once every
                // class is known.
                let read_without_get = !reads_own_slot && hooks.set && !hooks.get;
                let slot_read = reads_own_slot || read_without_get;
                // A `void` or `never` property is already refused above, and
                // no value could initialize it or be its default.
                let required = p.default.is_none()
                    && !void_or_never
                    && slot_read
                    && !is_lateinit
                    && !env.interner.is_nullable(ty);
                let visibility = declared_visibility(&p.modifiers);
                // Evaluated here rather than at check time because this is
                // the one pass that holds the declared type and the written
                // expression together, and because it must happen exactly
                // once: `crate::defaults` reports a bad default, and a second
                // walk would report it twice.
                let default = p
                    .default
                    .as_ref()
                    .filter(|_| !void_or_never)
                    .and_then(|expr| crate::defaults::eval_property_default(expr, ty, ctx, env));
                // A `static` property occupies no instance slot, so its
                // constant goes to the other vector — the one
                // `nvs_runtime::Ctx` arms once per request rather than once
                // per `new`. See `ClassSignature::static_property_defaults`.
                let is_static_property = p.modifiers.contains(&Modifier::Static);
                // The initializer paired with the type it is written into —
                // see `SignatureTable::property_default_types` for why it is
                // collected here and not recorded into `env.exprs`, which this
                // pass deliberately points at a table that is thrown away.
                if let Some(default) = p.default.as_ref().filter(|_| !void_or_never) {
                    table.property_default_types.push((default.span, ty));
                }
                let sig = table.entry(qname.clone());
                sig.properties.insert(name.clone(), ty);
                if let Some(level) = visibility {
                    sig.property_visibility.insert(name.clone(), level);
                }
                if let Some(level) = declared_set_visibility(&p.modifiers) {
                    sig.property_set_visibility.insert(name.clone(), level);
                }
                if hooks != PropertyHooks::default() {
                    sig.hooked_properties.insert(name.clone(), hooks);
                }
                if is_static_property {
                    sig.static_properties.push((name.clone(), default));
                } else if let Some(default) = default {
                    sig.property_defaults.push((name.clone(), default));
                }
                // A static property is deliberately **not** an `rule:classes/definite-property-initialization`
                // obligation: its storage is the request's rather than any
                // instance's, so no constructor can discharge one and the
                // declaration is the only place it can be initialized. Same
                // test, its own diagnostic, and reported here because this is
                // the pass that holds the type and the written default
                // together.
                if required && is_static_property {
                    env.diags.report(
                        Diagnostic::error(
                            code::E_UNINITIALIZED_PROPERTY,
                            format!(
                                "the static property `${name}` has no initializer, and no \
                                 constructor can give it one"
                            ),
                        )
                        .with_primary(p.name, "never initialized")
                        .with_help(
                            "give it a value here (`= 0`), or declare it nullable so it starts \
                             each request at `null`",
                        ),
                    );
                } else if required {
                    sig.required_properties.push((name.clone(), p.name));
                    if read_without_get {
                        sig.read_without_get.insert(name.clone());
                    }
                }
                if p.modifiers.contains(&Modifier::Readonly) {
                    sig.readonly_properties.insert(name.clone());
                }
                if is_lateinit {
                    sig.lateinit_properties.insert(name);
                }
            }
            ClassMemberKind::Method(m) => {
                for p in &m.params {
                    if p.modifiers.contains(&Modifier::Lateinit) {
                        env.diags.report(
                            Diagnostic::error(
                                code::E_LATEINIT_PROMOTED_PARAM,
                                "`lateinit` cannot be used on a constructor parameter",
                            )
                            .with_primary(p.name, "declared `lateinit` here")
                            .with_help(
                                "binding a parameter already assigns it — `lateinit` has \
                                 nothing left to defer",
                            ),
                        );
                    }
                }
                let params: Vec<TypeId> = m
                    .params
                    .iter()
                    .map(|p| lower_optional_type(p.ty.as_ref(), ctx, env))
                    .collect();
                reject_void_or_never_params(&m.params, &params, env);
                let param_names: Vec<String> = m
                    .params
                    .iter()
                    .map(|p| strip_sigil(span_text(env.src, p.name)).to_owned())
                    .collect();
                let inout: Vec<bool> = m.params.iter().map(|p| p.inout).collect();
                let covariant: Vec<bool> = if m.params.iter().any(|p| p.covariant) {
                    m.params.iter().map(|p| p.covariant).collect()
                } else {
                    Vec::new()
                };
                let param_text = crate::paths::declared_text(&m.params, ctx, env);
                let variadic = m.params.last().is_some_and(|p| p.variadic);
                let mut defaults = collect_defaults(&m.params, &params, ctx, env);
                crate::paths::resolve_defaults(&m.params, &param_text, &mut defaults, env);
                let return_ty = lower_optional_type(m.return_type.as_ref(), ctx, env);
                let returns_static = writes_static_return(m.return_type.as_ref());
                let mut params_bind_static: Vec<bool> = m
                    .params
                    .iter()
                    .map(|p| static_use(p.ty.as_ref(), qname, env.src).binds)
                    .collect();
                if !params_bind_static.contains(&true) {
                    params_bind_static.clear();
                }
                let return_binds_static = static_use(m.return_type.as_ref(), qname, env.src).binds;
                let interface_private = is_interface && m.modifiers.contains(&Modifier::Private);
                let visibility = declared_visibility(&m.modifiers).unwrap_or(Visibility::Public);
                let is_static = m.modifiers.contains(&Modifier::Static);
                let name = span_text(env.src, m.name).to_owned();
                if m.modifiers.contains(&Modifier::Final) {
                    table
                        .entry(qname.clone())
                        .final_methods
                        .insert(name.clone());
                }
                if name == "constructor" {
                    for p in m.params.iter().filter(|p| p.covariant) {
                        if !p.is_promoted() {
                            report_misplaced_covariant(
                                p.span,
                                "a constructor parameter that is not a property",
                                env,
                            );
                        } else if p.modifiers.contains(&Modifier::Readonly) {
                            report_misplaced_covariant(p.span, "a `readonly` property", env);
                        } else {
                            table.record_narrowed(
                                strip_sigil(span_text(env.src, p.name)).to_owned(),
                            );
                        }
                    }
                    record_promoted_properties(&m.params, &params, qname, table, env);
                } else {
                    reject_promotion_outside_constructor(&m.params, &name, env);
                }
                table.entry(qname.clone()).methods.insert(
                    name,
                    MethodSig {
                        params,
                        param_names,
                        inout,
                        covariant,
                        variadic,
                        defaults,
                        // `rule:security/unclassified-parameter-refuses-tainted` classifies a registry row's parameters,
                        // and nothing classifies a user-declared one — see
                        // `MethodSig::param_quals`.
                        param_quals: Vec::new(),
                        param_text,
                        // `rule:types/declaration`: a user-declared method
                        // has no type parameters to write.
                        type_params: Vec::new(),
                        type_bounds: Vec::new(),
                        return_ty,
                        contagious_return: None,
                        returns_static,
                        params_bind_static,
                        return_binds_static,
                        is_static,
                        interface_private,
                        visibility,
                        has_body: m.body.is_some(),
                        generator: m
                            .body
                            .as_ref()
                            .is_some_and(nvs_syntax::ast::is_generator_body),
                    },
                );
            }
            // `rule:classes/no-free-functions-or-constants`'s class constant. Evaluated here for the reason a
            // property default is evaluated here — this is the one pass that
            // holds the declared type and the written value together — and
            // typed here because the alternative, `crate::consts`, runs before
            // there is an interner to intern an annotation with. See
            // [`ConstSig`].
            ClassMemberKind::Const(c) => {
                let name = span_text(env.src, c.name).to_owned();
                let ty = match &c.ty {
                    Some(written) => lower_type(written, ctx, env),
                    None => folded_const_ty(qname, &name, env),
                };
                let value = crate::defaults::eval_const_value(&c.value, ty, env);
                // An unannotated constant is already `E0246`, and its type is
                // read off the value, so only a written type is checked.
                let refused = value.is_none()
                    && c.ty.is_some()
                    && reject_const_value_of_another_type(c, &name, ty, env);
                let visibility = declared_visibility(&c.modifiers).unwrap_or(Visibility::Public);
                table.entry(qname.clone()).constants.insert(
                    name,
                    ConstSig {
                        ty,
                        value,
                        refused,
                        visibility,
                    },
                );
            }
            ClassMemberKind::Error => {}
            _ => {}
        }
    }
}

/// Records each promoted constructor parameter (`constructor(public int $n)`,
/// [`nvs_syntax::ast::Param::is_promoted`]) as a property of `qname`, with the
/// type the parameter declares and the visibility its keyword names.
///
/// This is the whole of what "a promoted parameter *is* a property" means on
/// this side: [`resolve_property`] answers it, so a `$this->n` read, an
/// `$obj->n` access, an `rule:core-api/written-visibility` visibility check and `rule:classes/delegation-by-property`'s
/// `check_delegate_field` all see it with no case of their own.
/// [`crate::layout`] gives it the slot, and `nvs_ir::lower` emits the store.
///
/// Three of the maps beside `properties` are deliberately left alone.
/// **`required_properties`** is `rule:classes/definite-property-initialization`'s obligation, and a promoted
/// parameter discharges it by construction — the store is emitted from the
/// binding rather than written in the body, so there is nothing for a
/// constructor to be checked against. **`property_defaults`** holds what a
/// `new` arms a slot with before the constructor runs, and a promoted
/// parameter's `= expr` is the *parameter's* default, applied at the call
/// site by [`collect_defaults`] and stored by the same assignment every other
/// argument is. **`hooked_properties`** has no spelling to record: a
/// parameter list has nowhere to write a hook body.
fn record_promoted_properties(
    params: &[nvs_syntax::ast::Param],
    types: &[TypeId],
    qname: &QName,
    table: &mut SignatureTable,
    env: &mut Env<'_>,
) {
    for (p, &ty) in params.iter().zip(types) {
        if !p.is_promoted() {
            continue;
        }
        let name = strip_sigil(span_text(env.src, p.name)).to_owned();
        let sig = table.entry(qname.clone());
        sig.properties.insert(name.clone(), ty);
        if p.modifiers.contains(&Modifier::Readonly) {
            sig.readonly_properties.insert(name.clone());
        }
        if let Some(level) = declared_set_visibility(&p.modifiers) {
            sig.property_set_visibility.insert(name.clone(), level);
        }
        if let Some(level) = declared_visibility(&p.modifiers) {
            sig.property_visibility.insert(name, level);
        }
    }
}

/// `E0304` at the second declaration of a property, method or constant name
/// in one member list, in source order. Each name has one slot or one body,
/// so the second would replace the first or reach code generation as a
/// duplicate. A promoted parameter of the
/// `constructor` is a property declaration at its own position, so it
/// collides with a property written before or after the constructor.
///
/// Properties, methods and constants are three name spaces: `$x`, `x()` and
/// `X` never collide with each other. Names compare exactly, because names
/// are case-sensitive (`rule:classes/names-resolve-case-sensitively`).
fn reject_members_declared_twice(members: &[ClassMember], qname: &QName, env: &mut Env<'_>) {
    let mut properties: FxHashMap<&str, Span> = FxHashMap::default();
    let mut methods: FxHashMap<&str, Span> = FxHashMap::default();
    let mut constants: FxHashMap<&str, Span> = FxHashMap::default();
    let src = env.src;
    let mut twice: Vec<(&'static str, String, Span, Span)> = Vec::new();
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                let name = strip_sigil(span_text(src, p.name));
                if let Some(&first) = properties.get(name) {
                    twice.push(("property", format!("${name}"), p.name, first));
                } else {
                    properties.insert(name, p.name);
                }
            }
            ClassMemberKind::Method(m) => {
                let name = span_text(src, m.name);
                if let Some(&first) = methods.get(name) {
                    twice.push(("method", format!("{name}()"), m.name, first));
                } else {
                    methods.insert(name, m.name);
                }
                if name != "constructor" {
                    continue;
                }
                for p in m.params.iter().filter(|p| p.is_promoted()) {
                    let name = strip_sigil(span_text(src, p.name));
                    if let Some(&first) = properties.get(name) {
                        twice.push(("property", format!("${name}"), p.name, first));
                    } else {
                        properties.insert(name, p.name);
                    }
                }
            }
            ClassMemberKind::Const(c) => {
                let name = span_text(src, c.name);
                if let Some(&first) = constants.get(name) {
                    twice.push(("constant", name.to_owned(), c.name, first));
                } else {
                    constants.insert(name, c.name);
                }
            }
            _ => {}
        }
    }
    for (kind, name, at, first) in twice {
        env.diags.report(
            Diagnostic::error(
                code::E_DUPLICATE_DECLARATION,
                format!("`{qname}` declares the {kind} `{name}` twice"),
            )
            .with_primary(at, "declared again here")
            .with_secondary(first, "first declared here")
            .with_help(format!(
                "a class has one {kind} of each name. Delete one of the two declarations, or \
                 give one of them another name"
            )),
        );
    }
}

/// `E0742` for a property declared `void` or `never`, and whether it reported
/// one. `rule:types/grammar` makes both return-only: no value has either type,
/// so the property could never be given one. The caller then skips the
/// default and the initialization obligation, which would report the same
/// mistake a second time under another code.
fn reject_void_or_never_property(p: &PropertyMember, ty: TypeId, env: &mut Env<'_>) -> bool {
    let atom = match env.interner.get(ty) {
        Ty::Void => "void",
        Ty::Never => "never",
        _ => return false,
    };
    env.diags.report(
        Diagnostic::error(
            code::E_VOID_OR_NEVER_OUTSIDE_RETURN,
            format!("`{atom}` is a return type only, and this is a property"),
        )
        .with_primary(p.ty.span, format!("declared `{atom}` here"))
        .with_help(
            "no value has this type, so the property could never have a value. Write the type \
             of the value the property contains",
        ),
    );
    true
}

/// `E0401` at a class constant whose value is a constant of another type than
/// the one it declares: `const uint W = -1;` or `const int S = "a";`, and
/// whether it reported one. The value is folded with no declared type, and a
/// value that folds there but not at the declared type does not fit it.
///
/// A value that folds at neither is left to the read, which reports `E0792`
/// for a value with no constant form ([`crate::expr::members`]). So is a
/// declared type the literal grid does not place a value in, such as `mixed`,
/// `bytes`, a shape or a class: [`crate::defaults::eval_const_value`] gives
/// `None` there for a value that fits too.
fn reject_const_value_of_another_type(
    c: &nvs_syntax::ast::ConstMember,
    name: &str,
    declared: TypeId,
    env: &mut Env<'_>,
) -> bool {
    if !placed_by_the_literal_grid(declared, env)
        || crate::defaults::fold_constant_value(&c.value, None, env).is_none()
    {
        return false;
    }
    let want = env.interner.describe(declared);
    let Some(diag) = crate::expr::assign::type_mismatch(
        &[declared],
        format!("the value of the constant `{name}` does not have its declared type `{want}`"),
        env,
    ) else {
        return false;
    };
    env.diags.report(
        diag.with_primary(c.value.span, format!("this value is not of type `{want}`"))
            .with_help("write a value of the declared type, or change the type of the constant"),
    );
    true
}

/// Whether every part of `ty` is a type [`crate::defaults::literal_default`]
/// places a written value in: a scalar, a `string`, a single value, `null`,
/// and an array or a union of those.
fn placed_by_the_literal_grid(ty: TypeId, env: &Env<'_>) -> bool {
    match env.interner.get(ty) {
        Ty::Null
        | Ty::Bool
        | Ty::True
        | Ty::False
        | Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::String
        | Ty::TaintedString
        | Ty::SecretString
        | Ty::SecretTaintedString
        | Ty::SingleValueString(_)
        | Ty::SingleValueInt(_) => true,
        Ty::Array(element) => placed_by_the_literal_grid(*element, env),
        Ty::Union(members) => members.iter().all(|&m| placed_by_the_literal_grid(m, env)),
        _ => false,
    }
}

/// `rule:types/grammar`: `void` and `never` are return-only, so neither is a parameter.
///
/// Read off the *lowered* type rather than the written spelling, so a `type`
/// alias resolving to one is refused with the same code as the keyword — and
/// so a qualifier or a `?` wrapper, neither of which can produce `Ty::Void` or
/// `Ty::Never` at the top level, is left to the rules that already own it.
///
/// Here rather than in [`crate::check`] because that pass returns early for an
/// abstract method and an interface signature, and the declaration is just as
/// uncallable without a body.
fn reject_void_or_never_params(
    params: &[nvs_syntax::ast::Param],
    lowered: &[TypeId],
    env: &mut Env<'_>,
) {
    for (p, &ty) in params.iter().zip(lowered) {
        let atom = match env.interner.get(ty) {
            Ty::Void => "void",
            Ty::Never => "never",
            _ => continue,
        };
        let span = p.ty.as_ref().map_or(p.name, |t| t.span);
        let why = match atom {
            "void" => "`void` is the absence of a value, so no argument satisfies it",
            _ => "`never` is the empty type, so no argument satisfies it",
        };
        env.diags.report(
            Diagnostic::error(
                code::E_VOID_OR_NEVER_OUTSIDE_RETURN,
                format!("`{atom}` is a return type only, and this is a parameter"),
            )
            .with_primary(span, format!("declared `{atom}` here"))
            .with_help(format!(
                "{why} — a method declaring one can never be called. Write the type the \
                 argument actually has, or drop the parameter"
            )),
        );
    }
}

/// `E0882` for a `covariant` written on `what`, where no value of a wider type
/// can enter: a constructor parameter, a static or `readonly` property and an
/// anonymous function's parameter
/// (`rule:classes/a-covariant-type-is-checked-where-a-value-enters`).
pub(crate) fn report_misplaced_covariant(at: Span, what: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_COVARIANT_MISPLACED,
            format!("`covariant` cannot be used on {what}"),
        )
        .with_primary(at, "`covariant` is written here")
        .with_help(
            "`covariant` is for a parameter of a method that replaces a parent's method, and for \
             a property that a parent class declares. Remove it here",
        ),
    );
}

/// Refuses a visibility keyword on the parameter of a method that is not the
/// `constructor` (`E0722`), which is `rule:classes/delegation-by-property`'s own backlog line.
///
/// [`record_promoted_properties`] above is only ever reached from a
/// constructor, so the keyword written anywhere else declared nothing, took no
/// slot and was silently ignored — PHP refuses it, and so does this. The
/// judgement is [`nvs_syntax::ast::Param::is_promoted`]'s, unchanged: that
/// predicate deliberately does not ask which method encloses it, because that
/// is this caller's question and no other's.
///
/// Widening promotion to every method was never the alternative. A property is
/// a slot on an instance, armed once where the instance is made, and an
/// ordinary method has no such moment — it may be called any number of times,
/// or none.
fn reject_promotion_outside_constructor(
    params: &[nvs_syntax::ast::Param],
    method: &str,
    env: &mut Env<'_>,
) {
    for p in params {
        if !p.is_promoted() {
            continue;
        }
        let keyword = match declared_visibility(&p.modifiers) {
            Some(Visibility::Public) => "public",
            Some(Visibility::Protected) => "protected",
            Some(Visibility::Private) => "private",
            None => continue,
        };
        env.diags.report(
            Diagnostic::error(
                code::E_PROMOTED_PARAM_OUTSIDE_CONSTRUCTOR,
                format!(
                    "`{keyword}` on a parameter promotes it to a property, and only a \
                     `constructor` declares one"
                ),
            )
            .with_primary(p.span, format!("declared `{keyword}` on `{method}`"))
            .with_help(
                "drop the keyword — a parameter is a binding here. To declare a property, \
                 write it on the class or promote it in the `constructor`",
            ),
        );
    }
}

/// Evaluates every written `= expr` parameter default in `params` against its
/// own already-lowered type from `types`, and enforces the one ordering rule
/// they carry: no required parameter may follow an optional one.
///
/// The ordering check is here rather than in [`crate::defaults`] because it is
/// a property of the *list*, not of any one default — and it is what makes
/// [`MethodSig::required`] a count of leading entries rather than a scan.
/// A parameter whose default failed to evaluate is left required: the
/// diagnostic already fired, and treating it as optional would report a second
/// one at every call that omitted it.
///
/// A variadic parameter never carries a default — `...$rest` already accepts
/// zero arguments — so it is skipped rather than diagnosed for having none.
fn collect_defaults(
    params: &[nvs_syntax::ast::Param],
    types: &[TypeId],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Vec<Option<crate::defaults::ConstArg>> {
    let mut out: Vec<Option<crate::defaults::ConstArg>> = Vec::with_capacity(params.len());
    let mut seen_optional = false;
    for (p, &ty) in params.iter().zip(types) {
        let Some(expr) = p.default.as_ref() else {
            if seen_optional && !p.variadic {
                env.diags.report(
                    Diagnostic::error(
                        code::E_PARAM_DEFAULT_ORDER,
                        "a parameter with no default cannot follow one that has a default",
                    )
                    .with_primary(p.name, "this parameter has no default")
                    .with_help(
                        "arguments are matched positionally, so nothing could ever reach this \
                         parameter without also supplying the optional one before it — move it \
                         ahead of them, or give it a default too",
                    ),
                );
            }
            out.push(None);
            continue;
        };
        seen_optional = true;
        out.push(crate::defaults::eval_param_default(expr, ty, ctx, env));
    }
    out
}

/// Which hooks `p` declares *with a body* — a bodiless `get;` in an
/// interface or abstract class declares a requirement, not code to call, so
/// it contributes nothing here.
/// Whether one of `p`'s hooks reads `$this->name`, the property's own backing
/// slot. Such a slot needs a value before the first read, as a plain property
/// does. A hook that computes the value from other properties reads no slot,
/// and neither does the target of a plain `$this->name = v`.
fn hooks_read_own_slot(p: &PropertyMember, name: &str, src: &SourceFile) -> bool {
    use nvs_syntax::ast::{AssignOp, Expr, ExprKind, MemberName, PropertyHookBody};
    use nvs_syntax::visit::{StmtChild, each_child_expr, each_child_of_stmt};
    fn is_slot(e: &Expr, name: &str, src: &SourceFile) -> bool {
        matches!(
            &e.kind,
            ExprKind::PropertyAccess {
                object,
                property: MemberName::Ident(span),
                ..
            } if crate::expr::is_this_receiver(object, src)
                && strip_sigil(span_text(src, *span)) == name
        )
    }
    fn in_expr(e: &Expr, name: &str, src: &SourceFile) -> bool {
        if is_slot(e, name, src) {
            return true;
        }
        if let ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            value,
            ..
        } = &e.kind
            && is_slot(target, name, src)
        {
            return in_expr(value, name, src);
        }
        let mut found = false;
        each_child_expr(e, &mut |child| found = found || in_expr(child, name, src));
        found
    }
    fn in_stmt(s: &nvs_syntax::ast::Stmt, name: &str, src: &SourceFile) -> bool {
        let mut found = false;
        each_child_of_stmt(s, &mut |child| {
            found = found
                || match child {
                    StmtChild::Stmt(s) => in_stmt(s, name, src),
                    StmtChild::Expr(e) => in_expr(e, name, src),
                };
        });
        found
    }
    p.hooks.iter().flatten().any(|hook| match &hook.body {
        Some(PropertyHookBody::Expr(e)) => in_expr(e, name, src),
        Some(PropertyHookBody::Block(block)) => block.stmts.iter().any(|s| in_stmt(s, name, src)),
        None => false,
    })
}

fn declared_hooks(p: &PropertyMember) -> PropertyHooks {
    let mut out = PropertyHooks::default();
    for hook in p.hooks.iter().flatten() {
        if hook.body.is_none() {
            continue;
        }
        match hook.kind {
            nvs_syntax::ast::PropertyHookKind::Get => out.get = true,
            nvs_syntax::ast::PropertyHookKind::Set => out.set = true,
        }
    }
    out
}

/// Validates a `lateinit` property against `rule:classes/lateinit-restrictions`: every type is
/// admitted, and `readonly` beside it is refused. A promoted parameter is refused where promotion
/// is recorded.
fn check_lateinit_property(p: &PropertyMember, env: &mut Env<'_>) {
    if p.modifiers.contains(&Modifier::Readonly) {
        env.diags.report(
            Diagnostic::error(
                code::E_LATEINIT_READONLY_CONFLICT,
                "`lateinit` cannot be combined with `readonly`",
            )
            .with_primary(p.name, "both modifiers declared on this property")
            .with_help(
                "`readonly` requires assignment during construction; `lateinit` requires \
                 assignment after it — pick one",
            ),
        );
    }
}

/// Looks `name` up as a property on `qname`, falling back to walking its
/// `extends`/`implements` ancestors — the same shape
/// `nvs_hir::members::member_declared` already walks for existence-only
/// checking, generalised to return the type found rather than a bool.
#[must_use]
pub fn resolve_property(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<TypeId> {
    resolve_property_owned(qname, name, table, graph).map(|(_, ty)| ty)
}

/// [`resolve_property`], plus the [`QName`] that actually *declares* the
/// property — the receiver's own class, or whichever ancestor it inherited
/// the declaration from.
///
/// Separate from [`resolve_property`] because only one caller needs the
/// owner: a hooked property's compiled hook is labelled with its declaring
/// class ([`hook_label`]), the same way a method call names the class that
/// declares the method rather than the one the call was written on. A field
/// *slot* needs no such thing — [`crate::layout`] flattens an ancestor's
/// slots into every subclass, so a `FieldGet` names the receiver's own class.
#[must_use]
pub fn resolve_property_owned(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<(QName, TypeId)> {
    let mut seen = FxHashSet::default();
    resolve_property_rec(qname, name, table, graph, &mut seen)
}

fn resolve_property_rec(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<(QName, TypeId)> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname)
        && let Some(&ty) = sig.properties.get(name)
    {
        return Some((qname.clone(), ty));
    }
    let links = graph.get(qname)?;
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .find_map(|parent| resolve_property_rec(parent, name, table, graph, seen))
}

/// The type of a class constant whose declaration writes no annotation: the
/// type of the value [`crate::consts`] folded it to, and `mixed` where that
/// fold produced [`crate::consts::ConstValue::Ineligible`] or nothing at all.
///
/// [`ConstSig::ty`] owns why this is a fallback rather than a diagnostic — an
/// unannotated `const` parses, so a read of one still has to answer something,
/// and the value's own type is the closest thing to a declaration the source
/// contains. A `string` constant answers `string` rather than `rule:types/single-value-types`'s
/// single-value type: § 2's single-value-type fold is what a use in *type* position
/// gets, and a read is an ordinary expression.
fn folded_const_ty(qname: &QName, name: &str, env: &mut Env<'_>) -> TypeId {
    let (consts, graph) = (env.consts, env.graph);
    match consts.get(qname, name, graph) {
        Some(crate::consts::ConstValue::Str(_)) => env.interner.string(),
        Some(crate::consts::ConstValue::Int(_)) => env.interner.int(),
        Some(crate::consts::ConstValue::Bool(_)) => env.interner.bool_ty(),
        Some(crate::consts::ConstValue::Float(_)) => env.interner.float(),
        Some(crate::consts::ConstValue::Decimal { .. }) => env.interner.decimal(),
        Some(crate::consts::ConstValue::Ineligible) | None => env.interner.mixed(),
    }
}

/// Looks `name` up as a class constant on `qname`, falling back to walking its
/// `extends`/`implements` ancestors — [`resolve_property`]'s walk, for the
/// same reason [`crate::consts::ConstTable`]'s own lookup walks both edges: a
/// constant may be declared on an interface and reached through either link.
#[must_use]
pub fn resolve_const<'t>(
    qname: &QName,
    name: &str,
    table: &'t SignatureTable,
    graph: &ClassGraph,
) -> Option<&'t ConstSig> {
    resolve_const_owned(qname, name, table, graph).map(|(_, sig)| sig)
}

/// [`resolve_const`]'s walk with the class it stopped at, which is the class
/// that **declares** the constant and not the one the read was written on.
///
/// The distinction is [`resolve_property_owned`]'s: a name is what an
/// occurrence of a member is keyed by, and `Child::LIMIT` and `Parent::LIMIT`
/// are one declaration read two ways, so recording the written class would key
/// the read under a symbol nothing declares.
#[must_use]
pub fn resolve_const_owned<'t>(
    qname: &QName,
    name: &str,
    table: &'t SignatureTable,
    graph: &ClassGraph,
) -> Option<(QName, &'t ConstSig)> {
    let mut seen = FxHashSet::default();
    resolve_const_rec(qname, name, table, graph, &mut seen)
}

fn resolve_const_rec<'t>(
    qname: &QName,
    name: &str,
    table: &'t SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<(QName, &'t ConstSig)> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname)
        && let Some(found) = sig.constants.get(name)
    {
        return Some((qname.clone(), found));
    }
    let links = graph.get(qname)?;
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .find_map(|parent| resolve_const_rec(parent, name, table, graph, seen))
}

/// The declared **read** visibility of `owner::$name` — `owner` being the
/// declaring class [`resolve_property_owned`] returned, not the class the
/// access was written on, since that is where the keyword is.
///
/// `public` where no entry exists, and that is not a default in `rule:core-api/written-visibility`'s
/// sense: a property with no entry is one no user declaration wrote, which
/// today means a `Core` class installed through [`SignatureTable::seed_class`].
#[must_use]
pub fn property_visibility(owner: &QName, name: &str, table: &SignatureTable) -> Visibility {
    table
        .get(owner)
        .and_then(|sig| sig.property_visibility.get(name).copied())
        .unwrap_or(Visibility::Public)
}

/// Whether `owner::$name` was declared `readonly` — `owner` being the
/// declaring class [`resolve_property_owned`] returned, for the same reason
/// [`property_visibility`] wants it: that is where the keyword is written, and
/// a subclass neither adds the modifier to an inherited property nor takes it
/// away.
///
/// `false` where no entry exists, which is every `Core` class and every
/// synthesized declaration — only user source can write the modifier at all.
#[must_use]
pub fn property_is_readonly(owner: &QName, name: &str, table: &SignatureTable) -> bool {
    table
        .get(owner)
        .is_some_and(|sig| sig.readonly_properties.contains(name))
}

/// Whether `qname` was declared `final` — the question a subclass's `extends`
/// clause asks, and the only one the modifier answers on a class.
#[must_use]
pub fn class_is_final(qname: &QName, table: &SignatureTable) -> bool {
    table.get(qname).is_some_and(|sig| sig.is_final)
}

/// Whether `owner::name` was declared `final` — `owner` being the class
/// [`resolve_method`] returned, since that is where the keyword is written.
///
/// Asked of an *ancestor* only: a class's own `final` method is a promise to
/// its subclasses and never a constraint on itself.
#[must_use]
pub fn method_is_final(owner: &QName, name: &str, table: &SignatureTable) -> bool {
    table
        .get(owner)
        .is_some_and(|sig| sig.final_methods.contains(name))
}

/// Whether a member declared at `level` on `owner` is reachable from code
/// written inside `accessing` — `None` for file scope, a plain function, or a
/// anonymous function's body that is not inside a class.
///
/// The question is asked of the **accessing** class and never of the
/// receiver's static type: `$other->secret` inside `Secret`'s own method is
/// legal precisely because visibility is a property of where the code is
/// written. `protected` reaches down an `extends`/`implements` chain via
/// [`nvs_hir::implements_interface`] — the same ancestor walk
/// [`resolve_property_owned`] used to find the declaration in the first
/// place — and never up one: a superclass does not see a subclass's members.
#[must_use]
pub fn is_visible_from(
    level: Visibility,
    owner: &QName,
    accessing: Option<&QName>,
    graph: &ClassGraph,
) -> bool {
    match level {
        Visibility::Public => true,
        Visibility::Private => accessing == Some(owner),
        Visibility::Protected => accessing
            .is_some_and(|from| from == owner || nvs_hir::implements_interface(from, owner, graph)),
    }
}

/// [`is_visible_from`] for the body `ctx` is checking, which also admits test code reaching a
/// `private` or `protected` member of a class declared in the file being checked
/// (`rule:testing/private-in-the-same-file`).
///
/// The file is the one that declares `owner`, read off its symbol, so a test in another file is
/// held to the public contract even where it extends the class. A non-test method in the same
/// file is held to [`is_visible_from`] alone. Test code never reaches a build
/// (`rule:testing/tests-never-reach-a-build`), so what it is admitted to here is never reachable
/// from a request.
pub(crate) fn is_visible_here(
    level: Visibility,
    owner: &QName,
    ctx: &crate::Ctx<'_>,
    env: &Env<'_>,
) -> bool {
    is_visible_from(level, owner, ctx.current_class, env.graph)
        || (ctx.in_test_code
            && env
                .symbols
                .get(owner)
                .is_some_and(|symbol| symbol.decl_span.file == env.src.id()))
}

/// Which `rule:classes/property-hooks` hooks the property `owner::$name` declares — `owner`
/// being the *declaring* class [`resolve_property_owned`] returned, not the
/// class the access was written on. [`PropertyHooks::default`] (neither hook)
/// for an ordinary stored property.
#[must_use]
pub fn hooks_of(owner: &QName, name: &str, table: &SignatureTable) -> PropertyHooks {
    table
        .get(owner)
        .and_then(|sig| sig.hooked_properties.get(name).copied())
        .unwrap_or_default()
}

/// Whether a declaration's written return type is exactly the `static` atom —
/// what [`MethodSig::returns_static`] records, and the one spelling `rule:statements/static-is-a-member-modifier`'s late static binding takes.
///
/// Read off the *written* type rather than the lowered one, because lowering
/// is precisely what loses the distinction: [`crate::lower::lower_type`]
/// interns `static` and `self` to the same class. A `?static`, an
/// `array<static>` or a union is not this: [`static_use`] reads those, and a
/// call binds them through [`MethodSig::return_binds_static`].
#[must_use]
pub(crate) fn writes_static_return(ty: Option<&Type>) -> bool {
    matches!(ty, Some(t) if matches!(t.kind, TypeKind::Atom(TypeAtom::StaticTy)))
}

/// Where a written parameter or return type uses `static`, as
/// [`static_use`] reads it off the declaration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StaticUse {
    /// A call binds `static` here to the class it names
    /// (`rule:statements/static-is-a-member-modifier`), through [`bind_static`].
    pub binds: bool,
    /// `static` is written inside a `property<…>`, so inside the body the value
    /// may name a property only the called subclass declares.
    pub in_key: bool,
}

/// Reads where `ty` writes `static`, for the class `declaring` that declares it.
///
/// [`crate::lower::lower_type`] interns `static` as the declaring class, and
/// [`bind_static`] rewrites every occurrence of that class. So a type that
/// also writes `self` or the declaring class's own name does not bind: the
/// rewrite could not tell the two apart, and the declaring class is the sound
/// answer for both. A `static` among a written callable's parameters does not
/// bind either, because the body could call that callable with an instance of
/// the declaring class.
pub(crate) fn static_use(ty: Option<&Type>, declaring: &QName, src: &SourceFile) -> StaticUse {
    #[derive(Default)]
    struct Walk {
        writes: bool,
        blocked: bool,
        in_key: bool,
    }
    fn visit(
        ty: &Type,
        in_param: bool,
        in_key: bool,
        short: &str,
        src: &SourceFile,
        out: &mut Walk,
    ) {
        match &ty.kind {
            TypeKind::Nullable(t) | TypeKind::Paren(t) => {
                visit(t, in_param, in_key, short, src, out)
            }
            TypeKind::Union(items) | TypeKind::Intersection(items) => {
                for t in items {
                    visit(t, in_param, in_key, short, src, out);
                }
            }
            TypeKind::Atom(atom) => match atom {
                TypeAtom::StaticTy => {
                    out.writes = true;
                    out.blocked |= in_param;
                    out.in_key |= in_key;
                }
                TypeAtom::SelfTy => out.blocked = true,
                TypeAtom::Name(name, args) => {
                    let text = span_text(src, name.span);
                    out.blocked |= text.rsplit('\\').next() == Some(short);
                    for t in args {
                        visit(t, in_param, in_key, short, src, out);
                    }
                }
                TypeAtom::Array(Some(t)) | TypeAtom::ClassRef(t) => {
                    visit(t, in_param, in_key, short, src, out);
                }
                TypeAtom::PropertyKey(t) | TypeAtom::PropertyPath(t) => {
                    visit(t, in_param, true, short, src, out);
                }
                TypeAtom::StringHoles { key, .. } => {
                    visit(key, in_param, in_key, short, src, out);
                }
                TypeAtom::Shape(fields) => {
                    for f in fields {
                        visit(&f.ty, in_param, in_key, short, src, out);
                    }
                }
                TypeAtom::CallableSig { params, ret } => {
                    for t in params {
                        visit(t, true, in_key, short, src, out);
                    }
                    visit(ret, in_param, in_key, short, src, out);
                }
                _ => {}
            },
            _ => {}
        }
    }
    let Some(ty) = ty else {
        return StaticUse::default();
    };
    let mut walk = Walk::default();
    visit(ty, false, false, declaring.short_name(), src, &mut walk);
    let binds = walk.writes && !walk.blocked;
    StaticUse {
        binds,
        in_key: binds && walk.in_key,
    }
}

/// `id` with every occurrence of the class `declaring` replaced by `to` — the
/// class a call names, for a position [`static_use`] says binds `static`.
///
/// Only the bare class is replaced. Generic arguments never carry `static`,
/// because [`static_use`] blocks a type that writes its class's own name.
pub(crate) fn bind_static(
    id: TypeId,
    declaring: &QName,
    to: TypeId,
    interner: &mut crate::ty::TypeInterner,
) -> TypeId {
    match interner.get(id).clone() {
        Ty::Class(q, args) if &q == declaring && args.is_empty() => to,
        Ty::Array(elem) => {
            let elem = bind_static(elem, declaring, to, interner);
            interner.array(elem)
        }
        Ty::ClassRef(inner) => {
            let inner = bind_static(inner, declaring, to, interner);
            interner.class_ref(inner)
        }
        Ty::PropertyKey(inner) => {
            let inner = bind_static(inner, declaring, to, interner);
            interner.property_key(inner)
        }
        Ty::PropertyPath(inner) => {
            let inner = bind_static(inner, declaring, to, interner);
            interner.property_path(inner)
        }
        Ty::StringHoles { key, tainted } => {
            let key = bind_static(key, declaring, to, interner);
            interner.string_holes(key, tainted)
        }
        Ty::Union(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|m| bind_static(*m, declaring, to, interner))
                .collect();
            interner.make_union(members)
        }
        Ty::Intersection(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|m| bind_static(*m, declaring, to, interner))
                .collect();
            interner.make_intersection(members)
        }
        Ty::Shape(fields) => {
            let fields: Vec<crate::ty::ShapeField> = fields
                .iter()
                .map(|f| crate::ty::ShapeField {
                    name: f.name.clone(),
                    ty: bind_static(f.ty, declaring, to, interner),
                    required: f.required,
                })
                .collect();
            interner.shape(fields)
        }
        Ty::CallableSig { params, ret } => {
            let ret = bind_static(ret, declaring, to, interner);
            interner.callable_sig(params, ret)
        }
        _ => id,
    }
}

/// `id` as a method body sees a parameter whose `static` is written inside a
/// `property<…>` or a `propertyPath<…>`: each `property<C>` and
/// `propertyPath<C>` of the declaring class is a `string`.
///
/// The caller may pass a key or a path of a subclass, and the body does not
/// know which one, so the value is the name or the dotted path as text.
/// Converting it back with `as property<…>` or `as propertyPath<…>` is checked
/// at run time.
pub(crate) fn static_key_as_text(
    id: TypeId,
    declaring: &QName,
    interner: &mut crate::ty::TypeInterner,
) -> TypeId {
    match interner.get(id).clone() {
        Ty::PropertyKey(inner) | Ty::PropertyPath(inner) if matches!(interner.get(inner), Ty::Class(q, args) if q == declaring && args.is_empty()) => {
            interner.string()
        }
        // A `string<K>` over such a key is its text, for the same reason.
        Ty::StringHoles { key, tainted } if static_key_as_text(key, declaring, interner) != key => {
            crate::expr::quals::qualified_scalar(false, tainted, false, interner)
        }
        Ty::Array(elem) => {
            let elem = static_key_as_text(elem, declaring, interner);
            interner.array(elem)
        }
        Ty::Union(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|m| static_key_as_text(*m, declaring, interner))
                .collect();
            interner.make_union(members)
        }
        Ty::Shape(fields) => {
            let fields: Vec<crate::ty::ShapeField> = fields
                .iter()
                .map(|f| crate::ty::ShapeField {
                    name: f.name.clone(),
                    ty: static_key_as_text(f.ty, declaring, interner),
                    required: f.required,
                })
                .collect();
            interner.shape(fields)
        }
        _ => id,
    }
}

/// Looks `name` up as a method on `qname`, falling back to walking ancestors
/// the same way [`resolve_property`] does. Returns the [`QName`] that actually
/// declares it alongside a clone of its signature — the owner is needed by
/// [`crate::expr`] to enforce `rule:classes/interface-private-methods`'s private-interface-method
/// visibility rule (private is only visible from inside its own declaring
/// interface, never through whatever class or subinterface the lookup
/// started from), not just to type-check the call.
#[must_use]
pub fn resolve_method(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<(QName, MethodSig)> {
    let mut seen = FxHashSet::default();
    resolve_method_rec(qname, name, table, graph, &mut seen)
}

fn resolve_method_rec(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<(QName, MethodSig)> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname)
        && let Some(found) = sig.methods.get(name)
    {
        return Some((qname.clone(), found.clone()));
    }
    let links = graph.get(qname)?;
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .find_map(|parent| resolve_method_rec(parent, name, table, graph, seen))
}

/// What a `foreach` over an instance of `qname` yields, and which of
/// `rule:iteration/two-interfaces`'s two interfaces says so — `None` when
/// `qname` reaches neither.
///
/// Walks `extends`/`implements` exactly as [`resolve_method`] does, looking
/// at each declaration's own [`ClassSignature::implements`] for an entry
/// naming `Iterable` or `Iterator`. `Iterable` wins when a class somehow
/// reaches both, since driving a fresh cursor is the safer of the two: an
/// `Iterator` is single-pass, so a second `foreach` over the same value would
/// silently see nothing.
///
/// The type argument is what makes this a signature-table question rather
/// than a [`nvs_hir::ClassGraph`] one — that graph records *which* interfaces
/// a class reaches and has no interner to record the argument in.
#[must_use]
pub fn resolve_iteration_element(
    qname: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<(QName, TypeId)> {
    let mut seen = FxHashSet::default();
    let mut cursor = None;
    resolve_iteration_rec(qname, table, graph, &mut seen, &mut cursor).or(cursor)
}

/// The concrete type arguments `qname` fixed for `target` — `Some(vec![])`
/// when it reaches `target` at no arguments at all, `None` when it does not
/// reach it.
///
/// [`nvs_hir::hierarchy::implements_interface`] answers the *reachability*
/// half of the same question and is the right table for it; this one exists
/// for [`ClassSignature::implements`]'s reason — the arguments need
/// [`TypeId`]s `nvs-hir` has no interner for. So `Nums implements
/// Iterator<int>` is reachable-from-`Iterator` there and `[int]` here, which
/// is what lets [`crate::expr::is_assignable`] accept a `Nums` where an
/// `Iterator<int>` is declared and refuse it where an `Iterator<string>` is.
#[must_use]
pub fn resolve_interface_args(
    qname: &QName,
    target: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<Vec<TypeId>> {
    let mut seen = FxHashSet::default();
    resolve_interface_args_rec(qname, target, table, graph, &mut seen)
}

fn resolve_interface_args_rec(
    qname: &QName,
    target: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<Vec<TypeId>> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname)
        && let Some((_, args)) = sig.implements.iter().find(|(name, _)| name == target)
    {
        return Some(args.clone());
    }
    let links = graph.get(qname)?;
    let parents: Vec<QName> = links
        .extends
        .iter()
        .chain(links.implements.iter())
        .cloned()
        .collect();
    parents.iter().find_map(|parent| {
        if parent == target {
            return Some(Vec::new());
        }
        resolve_interface_args_rec(parent, target, table, graph, seen)
    })
}

/// The `Iterable` half of [`resolve_iteration_element`]; any `Iterator` found
/// on the way is left in `cursor` as the fallback.
fn resolve_iteration_rec(
    qname: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
    cursor: &mut Option<(QName, TypeId)>,
) -> Option<(QName, TypeId)> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname) {
        for (interface, args) in &sig.implements {
            let Some(&elem) = args.first() else { continue };
            if !interface.is_reserved_global_interface() {
                continue;
            }
            if interface.short_name() == nvs_hir::interfaces::ITERABLE {
                return Some((interface.clone(), elem));
            }
            if interface.short_name() == nvs_hir::interfaces::ITERATOR {
                cursor.get_or_insert((interface.clone(), elem));
            }
        }
    }
    let links = graph.get(qname)?;
    let parents: Vec<QName> = links
        .extends
        .iter()
        .chain(links.implements.iter())
        .cloned()
        .collect();
    parents
        .iter()
        .find_map(|parent| resolve_iteration_rec(parent, table, graph, seen, cursor))
}

/// Every property `qname`'s own constructor must definitely assign per
/// `rule:classes/definite-property-initialization`: exactly `qname`'s own [`ClassSignature::required_properties`].
/// Deliberately excludes `extends`/`implements`: an inherited property is
/// discharged by calling `parent::constructor(...)`, not by assigning it a
/// second time — see `crate::ctor_init`.
///
/// Before `rule:classes/no-traits`,
/// this also flattened in every used trait's own required properties
/// (recursively, through nested trait-use); that ancestor walk is gone along
/// with traits themselves — a `by`-target field used for delegation is an
/// ordinary declared property of `qname` itself, already covered by
/// `required_properties` with no special case needed (`rule:classes/no-traits`'s own
/// amendment to `rule:classes/definite-property-initialization`).
///
/// A `set`-only redeclaration whose ancestor declares a `get` hook for the
/// property is left out: a read runs that inherited hook, not the slot.
#[must_use]
pub fn own_required_properties(
    qname: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Vec<(String, Span)> {
    let Some(sig) = table.get(qname) else {
        return Vec::new();
    };
    sig.required_properties
        .iter()
        .filter(|(name, _)| {
            !sig.read_without_get.contains(name)
                || !ancestor_has_get_hook(qname, name, table, graph)
        })
        .cloned()
        .collect()
}

/// Whether a class `qname` extends, at any depth, declares a `get` hook for
/// the property `name`.
fn ancestor_has_get_hook(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> bool {
    let mut seen = FxHashSet::default();
    let mut queue: Vec<&QName> = graph
        .get(qname)
        .map(|links| links.extends.iter().collect())
        .unwrap_or_default();
    while let Some(ancestor) = queue.pop() {
        if !seen.insert(ancestor.clone()) {
            continue;
        }
        if table
            .get(ancestor)
            .and_then(|sig| sig.hooked_properties.get(name))
            .is_some_and(|hooks| hooks.get)
        {
            return true;
        }
        if let Some(links) = graph.get(ancestor) {
            queue.extend(links.extends.iter());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::SourceMap;
    use nvs_hir::resolve_file;
    use nvs_syntax::parse_file;

    use super::*;
    use crate::ty::TypeInterner;

    fn build(src: &str) -> (SignatureTable, nvs_hir::Module, TypeInterner, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let files = [crate::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }];
        let enums = crate::enums::build_enum_table(&files, &[], &mut diags);
        let consts = crate::consts::build_const_table(&files);
        let table = build_signatures(
            &files,
            &module,
            &enums,
            &consts,
            &[],
            &mut interner,
            &mut diags,
        );
        (table, module, interner, diags)
    }

    #[test]
    fn a_property_type_is_recorded() {
        let (table, _module, interner, _diags) = build("<?nvs\nclass Foo { public int $count; }\n");
        let ty = table
            .get(&QName::parse("Foo"))
            .and_then(|sig| sig.properties.get("count"))
            .copied()
            .expect("property recorded");
        assert_eq!(interner.describe(ty), "int");
    }

    #[test]
    fn a_method_signature_is_recorded() {
        let (table, _module, interner, _diags) =
            build("<?nvs\nclass Foo { function greet(string $name): bool { return true; } }\n");
        let sig = table
            .get(&QName::parse("Foo"))
            .and_then(|sig| sig.methods.get("greet"))
            .expect("method recorded");
        assert!(!sig.variadic);
        assert_eq!(interner.describe(sig.params[0]), "string");
        assert_eq!(interner.describe(sig.return_ty), "bool");
    }

    /// The whole point of [`mark_overridden_methods`]: a declaration nothing
    /// redeclares stays bindable to its label, and one something does is
    /// marked no matter how many levels down the redeclaration is, or whether
    /// the two are related by `extends` or by `implements`.
    #[test]
    fn a_method_is_marked_overridden_only_where_something_overrides_it() {
        let (table, _module, _interner, _diags) = build(
            "<?nvs
             interface Shape { function area(): int; function label(): string { return \"s\"; } }
             class Root { function kind(): string { return \"root\"; }              function alone(): int { return 1; } }
             class Middle extends Root {}
             class Leaf extends Middle { function kind(): string { return \"leaf\"; } }
             class Plain implements Shape { function area(): int { return 1; } }
             class Loud implements Shape { function area(): int { return 2; }              function label(): string { return \"l\"; } }
",
        );
        assert!(table.is_overridden(&QName::parse("Root"), "kind"));
        assert!(!table.is_overridden(&QName::parse("Root"), "alone"));
        assert!(!table.is_overridden(&QName::parse("Leaf"), "kind"));
        // `Middle` declares nothing at all, so it owns no entry to mark.
        assert!(!table.is_overridden(&QName::parse("Middle"), "kind"));
        assert!(table.is_overridden(&QName::parse("Shape"), "label"));
        // `area` is bodiless, so it already dispatches; marking it would say
        // nothing a caller does not already know.
        assert!(!table.is_overridden(&QName::parse("Shape"), "area"));
    }

    #[test]
    fn a_property_is_resolved_through_an_ancestor() {
        let (table, module, interner, _diags) =
            build("<?nvs\nclass Base { public int $count; }\nclass Sub extends Base {}\n");
        let ty = resolve_property(&QName::parse("Sub"), "count", &table, &module.graph)
            .expect("inherited property resolves");
        assert_eq!(interner.describe(ty), "int");
    }

    #[test]
    fn a_method_is_resolved_through_an_ancestor() {
        let (table, module, interner, _diags) = build(
            "<?nvs\nclass Base { function hello(): int { return 1; } }\nclass Sub extends Base {}\n",
        );
        let (owner, sig) = resolve_method(&QName::parse("Sub"), "hello", &table, &module.graph)
            .expect("inherited method resolves");
        assert_eq!(owner, QName::parse("Base"));
        assert_eq!(interner.describe(sig.return_ty), "int");
    }

    #[test]
    fn an_undeclared_member_does_not_resolve() {
        let (table, module, _interner, _diags) = build("<?nvs\nclass Foo {}\n");
        assert!(resolve_property(&QName::parse("Foo"), "missing", &table, &module.graph).is_none());
        assert!(resolve_method(&QName::parse("Foo"), "missing", &table, &module.graph).is_none());
    }

    // ------------------------------------------------------------------
    // Parameter defaults -- see `crate::defaults` for the accepted set.
    // ------------------------------------------------------------------

    fn sig_of(table: &SignatureTable, class: &str, method: &str) -> MethodSig {
        table
            .get(&QName::parse(class))
            .and_then(|sig| sig.methods.get(method))
            .expect("method recorded")
            .clone()
    }

    #[test]
    fn a_parameter_default_is_evaluated_once_into_the_declared_type() {
        let (table, _module, _interner, diags) = build(
            "<?nvs\nclass Box {\n  function scale(int $n, uint $by = 3, float $bias = -1.5, \
             string $tag = \"x\\ty\", bool $on = true): void {}\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let sig = sig_of(&table, "Box", "scale");
        assert_eq!(sig.required(), 1);
        assert_eq!(
            sig.defaults,
            vec![
                None,
                Some(crate::defaults::ConstArg::Uint(3)),
                Some(crate::defaults::ConstArg::Float(-1.5)),
                // Cooked through the one escape grammar, not a second one.
                Some(crate::defaults::ConstArg::Str("x\ty".to_owned())),
                Some(crate::defaults::ConstArg::Bool(true)),
            ]
        );
    }

    #[test]
    fn a_method_with_no_defaults_requires_every_parameter() {
        let (table, _module, _interner, _diags) =
            build("<?nvs\nclass Box { function pair(int $a, int $b): void {} }\n");
        let sig = sig_of(&table, "Box", "pair");
        assert_eq!(sig.required(), 2);
        assert_eq!(sig.defaults, vec![None, None]);
    }

    /// A named constant folds at a parameter exactly as it does at a property,
    /// and the declared type still decides: the case is accepted where the
    /// parameter declares its enum, and a class constant reaches a `uint`
    /// parameter as `crate::defaults`'s grid places it.
    #[test]
    fn a_parameter_default_takes_an_enum_case_and_a_class_constant() {
        let (table, _module, _interner, diags) = build(
            "<?nvs\nenum Mode { Off = 0, Fast = 3 }\nclass Limits { public const int COUNT = 12; }\n\
             class Box {\n  function run(Mode $m = Mode::Fast, int $n = Limits::COUNT, \
             uint $span = Limits::COUNT): void {}\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let sig = sig_of(&table, "Box", "run");
        assert_eq!(sig.required(), 0);
        assert_eq!(
            sig.defaults,
            vec![
                // The case's backing value, folded as a case
                // (`rule:enums/representation`), not as its ordinal.
                Some(crate::defaults::ConstArg::EnumInt(3)),
                Some(crate::defaults::ConstArg::Int(12)),
                Some(crate::defaults::ConstArg::Uint(12)),
            ]
        );
    }

    /// The declared type refuses a case of another enum, which is the half a
    /// fixture asserting only the accepted rows would miss.
    #[test]
    fn a_parameter_default_refuses_a_case_of_another_enum() {
        let (_table, _module, _interner, diags) = build(
            "<?nvs\nenum Mode { Fast = 3 }\nenum Speed { Slow = 1 }\n\
             class Box { function run(Mode $m = Speed::Slow): void {} }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_PARAM_DEFAULT_NOT_LITERAL)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_non_literal_parameter_default_is_refused() {
        let (_table, _module, _interner, diags) =
            build("<?nvs\nclass Box { function scale(int $n = 1 + 1): void {} }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_PARAM_DEFAULT_NOT_LITERAL)),
            "{diags:?}"
        );
    }

    /// `= null` is a constant of any type that admits a `null` and of no other
    /// — `crate::defaults`'s own doc owns the rule. Both sides are asserted
    /// here because the arm was added to a grid: a declared type that admits no
    /// `null` still refuses it along with every other constant of the wrong
    /// type.
    #[test]
    fn a_null_parameter_default_needs_a_type_that_admits_null() {
        let (_table, _module, _interner, diags) =
            build("<?nvs\nclass Box { function scale(?int $n = null): void {} }\n");
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_PARAM_DEFAULT_NOT_LITERAL)),
            "{diags:?}"
        );

        let (_table, _module, _interner, diags) =
            build("<?nvs\nclass Box { function scale(int $n = null): void {} }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_PARAM_DEFAULT_NOT_LITERAL)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_default_of_the_wrong_type_is_refused() {
        let (_table, _module, _interner, diags) =
            build("<?nvs\nclass Box { function scale(int $n = \"three\"): void {} }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_PARAM_DEFAULT_NOT_LITERAL)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_required_parameter_after_an_optional_one_is_refused() {
        let (_table, _module, _interner, diags) =
            build("<?nvs\nclass Box { function scale(int $a = 1, int $b): void {} }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_PARAM_DEFAULT_ORDER)),
            "{diags:?}"
        );
    }

    /// A variadic tail takes zero arguments already, so it is not "a required
    /// parameter after an optional one" — the one shape the ordering rule has
    /// to let through.
    #[test]
    fn a_variadic_tail_after_an_optional_parameter_is_accepted() {
        let (_table, _module, _interner, diags) =
            build("<?nvs\nclass Box { function scale(int $a = 1, int ...$rest): void {} }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // ------------------------------------------------------------------
    // `rule:classes/lateinit-restrictions` -- `lateinit`'s four rejected shapes.
    // ------------------------------------------------------------------

    #[test]
    fn a_lateinit_class_typed_property_is_excluded_from_required_properties() {
        let (table, _module, _interner, diags) =
            build("<?nvs\nclass Logger {}\nclass Widget { public lateinit Logger $logger; }\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let sig = table
            .get(&QName::parse("Widget"))
            .expect("signature recorded");
        assert!(sig.lateinit_properties.contains("logger"));
        assert!(sig.required_properties.is_empty());
    }

    #[test]
    fn lateinit_takes_a_scalar_an_array_and_a_nullable_type() {
        let (table, _module, _interner, diags) = build(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit int $count;\n  \
             public lateinit array<string> $tags;\n  public lateinit ?Logger $logger;\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let sig = table
            .get(&QName::parse("Widget"))
            .expect("signature recorded");
        for name in ["count", "tags", "logger"] {
            assert!(sig.lateinit_properties.contains(name), "{name}");
        }
        assert!(sig.required_properties.is_empty());
    }

    #[test]
    fn lateinit_on_a_promoted_parameter_is_diagnosed() {
        let (_table, _module, _interner, diags) = build(
            "<?nvs\nclass Logger {}\nclass Widget {\n  function constructor(public lateinit Logger $logger) {}\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_PROMOTED_PARAM)),
            "{diags:?}"
        );
    }

    #[test]
    fn lateinit_combined_with_readonly_is_diagnosed() {
        let (_table, _module, _interner, diags) = build(
            "<?nvs\nclass Logger {}\nclass Widget { public lateinit readonly Logger $logger; }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_READONLY_CONFLICT)),
            "{diags:?}"
        );
    }
}
