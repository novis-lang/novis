//! Seeding the checker's signature table with the exception tree.
//!
//! [`nvs_hir::errors`] is the one home for the tree's shape; this is the one
//! place that turns it into the same
//! [`ClassSignature`](crate::signatures::ClassSignature) a user-declared class
//! produces — exactly the shape [`crate::core_lib`] already gives `Core`, and
//! for the same reason: after this point `resolve_property` finds
//! `$e->message` the way it finds `$animal->legs`, and `resolve_method` finds
//! `Throwable::constructor` the way it finds any inherited one.
//!
//! # Members are properties, not accessors
//!
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) § 10
//! is authoritative: `$e->message`, `$e->previous`, `$e->backtrace`,
//! `$e->location`. There is no `getMessage()`/`getTraceAsString()` to seed,
//! and nothing here declares one.
//!
//! # `ParseError` is the one class with state of its own
//!
//! `rule:core-classes/derive-reports-every-field`'s
//! `issues: array<Core\Issue>`, seeded from
//! [`nvs_hir::errors::OWN_PROPERTIES`] rather than named here, so that the
//! slot order and the signature cannot disagree. `Core\Issue` is an `rule:types/object-top`
//! **shape**, `{path: string, message: string}` — see [`issue_shape`] — so a
//! class of that name exists nowhere and a decoder builds one with nothing
//! declared.
//!
//! A class that declares properties also declares a constructor to assign
//! them, which is why the root is no longer the only entry with methods; the
//! bodies are `nvs_ir::lower::exception`'s.
//!
//! # `location` is the throw site, not the construction site
//!
//! The synthesized constructor leaves it empty and `nvs_runtime::nvs_raise`
//! fills it at the `throw`, from the site constant `nvs_ir::lower`'s
//! `Lowering::throw_source` hands it — the choice that agrees
//! with the backtrace beside it, since that holds the frames the exception
//! unwound *out of*. Two consequences, both pinned by
//! `tests/conformance/error/a-location-is-the-throw-site-and-a-rethrow-moves-it.nvst`:
//! an exception constructed and never thrown reads `""`, and rethrowing one
//! that was already caught *rewrites* its location to the rethrow site rather
//! than keeping the original. Chaining is what preserves the original — the
//! cause keeps its own location, and `{previous: $e}` is how it is kept.
//!
//! An `issues` entry is read like any other value now: `$issue->path` is a
//! property access on an `rule:types/erased-member-access` shape receiver, which
//! [`crate::expr_table::ExprInfo::ShapeProperty`] resolves to the field's slot
//! and `nvs-ir` reads by index — there is no class to name, so there is no
//! class to record.
//!
//! A user subclass that declares its own constructor and does not chain to
//! `parent::constructor(…)` is already refused, by the same check every other
//! `extends` gets — nothing exception-specific is needed for that.
//!
//! # `previous` is read like every other nullable property
//!
//! The chain is built and read back at the type it was given: `new
//! RuntimeError("…", {previous: $e})` stores it and `$e->previous` is the
//! `Throwable|null` that went in. Reaching a member of *that* is the ordinary
//! nullable-receiver question, and nothing exception-specific answers it. A
//! plain `->` straight off `$e->previous` is `E0459`; `$e->previous?->message`
//! reads through it in one expression; a binding the same `== null` test
//! narrows is the other route, and the one worth taking when more than one
//! read follows.
//!
//! What rules the middle term out — narrowing `$e->previous` in place — is
//! `rule:types/narrowing`'s subject rather than anything about exceptions:
//! every spelling narrows a *binding*, by name, which is what [`crate::locals`]
//! keys the narrowed type on. That `Throwable|null` erases to
//! `nvs_ir::Ty::Tagged` costs this nothing, since an erased receiver lowers
//! like any other.

use nvs_hir::QName;
use nvs_hir::errors::{PROPERTIES, ROOT, TREE};
use rustc_hash::FxHashMap;

use crate::defaults::ConstArg;
use crate::enums::EnumBacking;
use crate::signatures::{MethodSig, SignatureTable};
use crate::ty::{ShapeField, TypeId, TypeInterner};

/// Adds every [`nvs_hir::errors::TREE`] entry to `table`.
///
/// Called once, alongside [`crate::core_lib::seed`], at the head of
/// [`build_signatures`](crate::signatures::build_signatures).
pub(crate) fn seed(table: &mut SignatureTable, interner: &mut TypeInterner) {
    let root = QName::parse(ROOT);
    for (name, _) in TREE {
        let qname = QName::parse(name);
        // A class that declares no state of its own inherits both its
        // properties and its constructor through the graph
        // `nvs_hir::hierarchy` seeded — the same walk a user subclass goes
        // through. The root, `ParseError` and `Core\Db\RolledBack` are the
        // three that do declare some; `nvs_hir::errors::OWN_PROPERTIES` is
        // that roster's home.
        let properties = if qname == root {
            root_properties(interner)
        } else {
            own_properties(name, interner)
        };
        let methods = if nvs_hir::errors::declares_constructor(name) {
            constructor(interner)
        } else {
            FxHashMap::default()
        };
        table.seed_class(qname, properties, methods);
    }
}

/// `name`'s own properties beyond the root's, typed —
/// [`nvs_hir::errors::OWN_PROPERTIES`]'s non-root rows.
fn own_properties(name: &str, interner: &mut TypeInterner) -> FxHashMap<String, TypeId> {
    nvs_hir::errors::own_properties(name)
        .iter()
        .map(|property| {
            let ty = match *property {
                "issues" => {
                    let issues = issue_shape(interner);
                    interner.array(issues)
                }
                // `Core\Db\RolledBack::$reason` — spec § 18 types it a plain
                // `string`, not a `?string`: a rollback always has the reason
                // its caller passed, and the synthesized constructor writes
                // the message into it (`nvs_ir::lower::exception`).
                "reason" => interner.string(),
                // `Core\Db\DbError::$kind` — `rule:core-classes/db-error`'s normalised
                // `ErrorKind`, which is a registered enum and not a string:
                // the eleven conditions are a closed set the compiler can
                // check a `match` against, and `nvs_stdlib::db::ERROR_KIND` is
                // the row that declares them. `EnumBacking::Int` is what every
                // `Core` enum is backed by (`crate::core_lib`'s `CoreTy::Enum`
                // arm interns exactly this).
                "kind" => interner.enum_(QName::parse(ERROR_KIND), EnumBacking::Int),
                // `Core\Db\DbError::$sqlState` and `$driverCode` — `rule:core-classes/db-error`'s raw pair, and both `?T` where `kind` is not: a refusal
                // the wire produced rather than the server has neither, and a
                // driver whose only code is its `SQLSTATE` has no second
                // integer to answer with. Nothing seeds them for that case,
                // because an unwritten slot already reads `null` — which is
                // exactly what a `?T` allows and what `issues` above, being
                // `array<Issue>` and not `?array<Issue>`, does not.
                "sqlState" => {
                    let text = interner.string();
                    let null = interner.null();
                    interner.make_union([text, null])
                }
                "driverCode" => {
                    let code = interner.int();
                    let null = interner.null();
                    interner.make_union([code, null])
                }
                // `Core\Db\DbError::$constraint` — the same `?string` as
                // `sqlState`, and absent far more often: § 8's kinds are
                // conditions, and only some of them — a unique, foreign-key,
                // not-null or check violation — have a constraint to name at
                // all. A syntax error or a permission answers `null` on every
                // driver there will ever be.
                "constraint" => {
                    let text = interner.string();
                    let null = interner.null();
                    interner.make_union([text, null])
                }
                // `Core\Db\DbError::$sql` — the statement that was refused, and
                // a plain `string` rather than a `tainted` one because § 8 only
                // allows the text on the error at all on the grounds that it is
                // developer-authored; a bound value, which is where the
                // request's own bytes are, never joins it. `?string` for the
                // member that has no caller-written statement to name — § 7's
                // `BEGIN` and `COMMIT` are the runtime's own.
                "sql" => {
                    let text = interner.string();
                    let null = interner.null();
                    interner.make_union([text, null])
                }
                other => panic!("no type seeded for `{name}::{other}`"),
            };
            ((*property).to_owned(), ty)
        })
        .collect()
}

/// `Core\Db\DbError::$kind`'s enum, by name.
///
/// `nvs_stdlib::db::ERROR_KIND_NAME` is the home of this spelling and this is
/// a second copy of it, because that const is `pub(crate)` and the module is
/// private — `the_kind_property_names_a_registered_enum` is what stops the two
/// drifting.
const ERROR_KIND: &str = r"Core\Db\ErrorKind";

/// `type Core\Issue = {path: string, message: string}` —
/// `rule:core-classes/derive-reports-every-field`'s one shape.
///
/// An `rule:types/object-top` shape rather than a class, which is what that ADR writes and
/// what lets a decoder build one with no declaration anywhere: the value is an
/// anonymous methodless instance, and `nvs_stdlib::issue` is what builds it.
/// [`TypeInterner::shape`] canonicalizes the field order, so the runtime slot
/// order is the *sorted* one — `message`, then `path`.
fn issue_shape(interner: &mut TypeInterner) -> TypeId {
    let string = interner.string();
    interner.shape(vec![
        ShapeField::required("path".to_owned(), string),
        ShapeField::required("message".to_owned(), string),
    ])
}

/// `Throwable`'s four readonly properties, spec § 10's list.
fn root_properties(interner: &mut TypeInterner) -> FxHashMap<String, TypeId> {
    let string = interner.string();
    let throwable = interner.class(QName::parse(ROOT));
    let null = interner.null();
    let previous = interner.make_union([throwable, null]);
    let backtrace = interner.array(string);
    let by_name: FxHashMap<&str, TypeId> = [
        ("message", string),
        ("previous", previous),
        ("backtrace", backtrace),
        ("location", string),
    ]
    .into_iter()
    .collect();
    // Driven off `PROPERTIES` rather than the literal above, so a name added
    // there without a type here fails loudly instead of silently vanishing
    // from the checker while still occupying a slot.
    PROPERTIES
        .iter()
        .map(|name| {
            let ty = by_name
                .get(name)
                .copied()
                .unwrap_or_else(|| panic!("no type seeded for `Throwable::{name}`"));
            ((*name).to_owned(), ty)
        })
        .collect()
}

/// `constructor(string $message, {previous?: Throwable|null})` — spec § 10's
/// one required message and one options shape, which is
/// `rule:core-api/shape-rules` R2 applied to a
/// constructor like any other member.
///
/// The bag is optional by construction rather than by a second rule: its
/// [`ConstArg::Options`] default gives every option its own `null`, so
/// `new LogicError("…")` supplies one argument and
/// [`MethodSig::required`](crate::signatures::MethodSig::required) is 1. The
/// flattening at the call site is generic over any signature carrying a
/// [`Ty::CoreShape`](crate::ty::Ty::CoreShape) parameter — `nvs_ir::lower::call`'s
/// `lower_options_arg` — so a seeded constructor reaches it on exactly the
/// terms a `Core` member does.
///
/// The same signature for every class that declares one: a subclass's
/// synthesized body differs only in which slots it fills, never in what a
/// `new` writes at the call site.
fn constructor(interner: &mut TypeInterner) -> FxHashMap<String, MethodSig> {
    let string = interner.string();
    let void = interner.void();
    let throwable = interner.class(QName::parse(ROOT));
    let null = interner.null();
    let previous = interner.make_union([throwable, null]);
    let options = interner.options(vec![(
        "previous".to_owned(),
        previous,
        None,
        nvs_stdlib::registry::ParamText::Plain,
    )]);
    [(
        "constructor".to_owned(),
        MethodSig {
            params: vec![string, options],
            // A constructor declares no return type at all, so there is
            // no `static` to bind — see `MethodSig::returns_static`.
            returns_static: false,
            // `rule:core-api/shape-rules` R2 reaches a synthesized member too: spec § 10 writes
            // `$message`, and the bag is every other trailing bag's
            // `options` — read from `nvs_stdlib::registry` rather than
            // spelled again here, so `new LogicError(message: "…")` and a
            // `Core` member's `options:` are one rule and not two.
            param_names: vec![
                "message".to_owned(),
                nvs_stdlib::registry::OPTIONS_NAME.to_owned(),
            ],
            inout: vec![false, false],
            // Synthesized here rather than registered, so there is no row to
            // carry a classification — see `MethodSig::param_quals`.
            param_quals: Vec::new(),
            param_text: Vec::new(),
            variadic: false,
            defaults: vec![
                None,
                Some(ConstArg::Options(vec![(
                    "previous".to_owned(),
                    ConstArg::Null,
                )])),
            ],
            type_params: Vec::new(),
            type_bounds: Vec::new(),
            return_ty: void,
            is_static: false,
            interface_private: false,
            visibility: nvs_syntax::ast::Visibility::Public,
            // Synthesized by `nvs_ir::lower`, which is still a body.
            has_body: true,
        },
    )]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::{resolve_method, resolve_property};
    use nvs_hir::ClassGraph;

    /// The very graph `nvs_hir::hierarchy` seeds into every real compilation,
    /// so these tests exercise the shipped links rather than a copy of them.
    fn tree_graph() -> ClassGraph {
        let mut graph = ClassGraph::default();
        nvs_hir::seed_exception_tree(&mut graph);
        graph
    }

    #[test]
    fn the_root_declares_every_property_the_spec_lists() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let root = table.get(&QName::parse(ROOT)).expect("Throwable is seeded");
        for name in PROPERTIES {
            assert!(
                root.properties.contains_key(*name),
                "Throwable should declare `{name}`"
            );
        }
        assert_eq!(root.properties.len(), PROPERTIES.len());
    }

    #[test]
    fn a_leaf_inherits_the_root_s_properties_and_constructor() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let graph = tree_graph();

        let ty = resolve_property(&QName::parse("TimeoutError"), "message", &table, &graph)
            .expect("`message` resolves through the seeded links");
        assert_eq!(interner.describe(ty), "string");
        let backtrace = resolve_property(&QName::parse("IOError"), "backtrace", &table, &graph)
            .expect("`backtrace` resolves through the seeded links");
        assert_eq!(interner.describe(backtrace), "array<string>");

        let (owner, sig) = resolve_method(&QName::parse("IOError"), "constructor", &table, &graph)
            .expect("the constructor resolves through the seeded links");
        assert_eq!(owner.to_string(), ROOT);
        assert!(!sig.is_static);
        // The message, then spec § 10's options bag — which carries its own
        // default, so a `new IOError("…")` still supplies one argument.
        assert_eq!(sig.params.len(), 2);
        let throwable = interner.class(QName::parse(ROOT));
        let null = interner.null();
        let previous = interner.make_union([throwable, null]);
        // Rebuilt rather than compared against a `describe` string: a union
        // orders its members by type id, so the rendering is not stable.
        assert_eq!(
            sig.params[1],
            interner.options(vec![(
                "previous".to_owned(),
                previous,
                None,
                nvs_stdlib::registry::ParamText::Plain,
            )])
        );
        assert_eq!(sig.required(), 1);
        assert_eq!(
            sig.defaults[1],
            Some(ConstArg::Options(vec![(
                "previous".to_owned(),
                ConstArg::Null
            )]))
        );
    }

    #[test]
    fn parse_error_declares_the_issue_list_and_a_constructor_of_its_own() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let graph = tree_graph();

        let ty = resolve_property(&QName::parse("ParseError"), "issues", &table, &graph)
            .expect("`issues` is ParseError's own property");
        let issues = issue_shape(&mut interner);
        assert_eq!(ty, interner.array(issues));

        let (owner, _) = resolve_method(&QName::parse("ParseError"), "constructor", &table, &graph)
            .expect("ParseError declares its own constructor");
        assert_eq!(owner.to_string(), "ParseError");
        // Every sibling still reaches the root's.
        let (owner, _) = resolve_method(&QName::parse("IOError"), "constructor", &table, &graph)
            .expect("IOError inherits the root's");
        assert_eq!(owner.to_string(), ROOT);
        assert!(resolve_property(&QName::parse("IOError"), "issues", &table, &graph).is_none());
    }

    #[test]
    fn the_kind_property_names_a_registered_enum() {
        // Three spellings of one enum meet here and nothing but this test
        // holds them together: `ERROR_KIND` above, `nvs_stdlib::db`'s
        // `ERROR_KIND_NAME` (which declares the cases), and
        // `nvs_ir::lower::exception`'s `ERROR_KIND_OTHER` ordinal, which the
        // synthesized constructor writes into the slot. The third crate
        // depends on neither of the other two, so the ordinal is pinned here
        // rather than compared there.
        let registered = nvs_stdlib::registry::ENUMS
            .iter()
            .find(|entry| entry.name == ERROR_KIND)
            .expect("`Core\\Db\\ErrorKind` is a registered enum");
        assert_eq!(
            registered.cases.iter().find(|(name, _)| *name == "Other"),
            Some(&("Other", 10)),
            "`nvs_ir::lower::exception::ERROR_KIND_OTHER` restates this ordinal"
        );

        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let graph = tree_graph();
        let ty = resolve_property(&QName::parse("Core\\Db\\DbError"), "kind", &table, &graph)
            .expect("`kind` is DbError's own property");
        assert_eq!(
            ty,
            interner.enum_(QName::parse(ERROR_KIND), EnumBacking::Int)
        );
        // A sibling under `RuntimeError` gains nothing from it.
        assert!(
            resolve_property(
                &QName::parse("Core\\Db\\RolledBack"),
                "kind",
                &table,
                &graph
            )
            .is_none()
        );
    }

    #[test]
    fn the_root_owes_its_constructor_no_definite_initialization() {
        // The constructor is synthesized by `nvs_ir::lower`, not written in
        // Novis, so `rule:classes/definite-property-initialization`'s obligation has nothing to check it against.
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let root = table.get(&QName::parse(ROOT)).expect("Throwable is seeded");
        assert!(root.required_properties.is_empty());
    }
}
