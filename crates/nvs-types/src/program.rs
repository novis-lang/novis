//! `rule:programs/implementing`: `Core\Program::implementing<T>()`, answered here rather than at run
//! time.
//!
//! The sibling of [`crate::retrieval`], and the same pass shape for the same
//! reason: the question has an answer before the program starts, so the call
//! is **replaced** with it. What differs is that the answer is not a constant.
//! § 3 expands the call "to an array literal of `new` expressions — one per
//! non-abstract class in the program implementing `T`, sorted by
//! fully-qualified name", and a `new` allocates. So this records
//! [`ExprInfo::ProgramInstances`] — the list — where retrieval records
//! [`crate::expr_table::ExprInfo::CoreConst`], and `nvs-ir` emits the `new`s
//! and the array. The instances are therefore per-request like any other
//! object and nothing crosses an isolate boundary
//! ([ADR 0006](/docs/adr/0006-isolated-script-execution.md)).
//!
//! # The scan already happened
//!
//! Nothing here walks a directory. `nvs_hir::AutoloadMap::enumerate` names
//! every file the autoload roots declare a name in, `nvs_hir::requires` loads
//! them — and *only* for a program that writes this call, which is § 3's
//! "a program containing no `implementing<T>()` call never performs the scan"
//! — and `nvs_hir::implementors` filters the resulting class graph. By the
//! time this pass runs, `Env::graph` holds every declaration the answer is
//! drawn from, so the whole of this module is a lookup and two refusals.
//!
//! The two refusals are § 3's own sentences, and each is reported at the call:
//! `T` must be an interface (`E0743`), and every class the enumeration would
//! instantiate needs a no-argument constructor (`E0744`). Their own `Code`
//! doc comments own why each is refused rather than worked around.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::{QName, SymbolKind};
use nvs_syntax::ast::Expr;

use crate::Env;
use crate::expr::members::class_qname_of;
use crate::expr_table::ExprInfo;
use crate::signatures::resolve_method;
use crate::ty::TypeId;

/// The one class this pass answers for.
const OWNER: &str = r"Core\Program";

/// Whether `owner::member` is the enumeration — the same nominal test
/// [`crate::retrieval::is_retrieval`] makes, against a resolved [`QName`]
/// rather than against what the call site spelled.
pub(crate) fn is_enumeration(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && member == "implementing"
}

/// Resolves one enumeration and records its answer against the call's own
/// span, as the [`ExprInfo::ProgramInstances`] `nvs-ir` materializes the
/// array literal from.
///
/// Records nothing where the call is refused: a diagnostic has been reported,
/// and `nvs-ir` never reaches a unit that failed to check.
pub(crate) fn expand(call: &Expr, written: &[TypeId], env: &mut Env<'_>) {
    let Some(want) = written.first().copied() else {
        // The type-argument count is `check_written_type_args`' refusal and
        // has already been made; a second one names the same mistake twice.
        return;
    };
    let Some(interface) = class_qname_of(want, env.interner).filter(|qname| {
        env.symbols
            .get(qname)
            .is_some_and(|sym| sym.kind == SymbolKind::Interface)
    }) else {
        let found = env.interner.describe(want);
        env.diags.report(
            Diagnostic::error(
                code::E_PROGRAM_TYPE_ARG_NOT_AN_INTERFACE,
                format!("`Core\\Program::implementing` enumerates an interface, and `{found}` is not one"),
            )
            .with_primary(call.span, format!("`{found}` written here"))
            .with_help(
                "`rule:programs/implementing`: the interface is what gives the enumerated instances a static \
                 type — `object` is opaque and a shape describes data rather than methods, so \
                 an array of anything else is one nothing can be called on",
            ),
        );
        return;
    };

    let classes = nvs_hir::implementors(&interface, env.graph);
    let mut ctors = Vec::with_capacity(classes.len());
    for class in &classes {
        // A class declaring no `constructor` at all is the common case and the
        // one § 3 is written for; `ExprInfo::New`'s own `ctor` is `None` there
        // too, and `nvs-ir` allocates without calling anything.
        let Some((owner, sig)) = resolve_method(class, "constructor", env.signatures, env.graph)
        else {
            ctors.push(None);
            continue;
        };
        if sig.required() > 0 {
            env.diags.report(
                Diagnostic::error(
                    code::E_PROGRAM_IMPLEMENTOR_NEEDS_NO_ARGUMENT_CONSTRUCTOR,
                    format!(
                        "`{class}` implements `{interface}` and its constructor takes {} \
                         required argument(s)",
                        sig.required()
                    ),
                )
                .with_primary(
                    call.span,
                    format!("this enumeration would write `new {class}()`"),
                )
                .with_help(format!(
                    "`rule:programs/implementing`: every class the enumeration instantiates needs a \
                     no-argument constructor, and dependencies arrive through \
                     `{interface}`'s own methods instead — give `{class}` a \
                     zero-argument `constructor`, or drop its `implements` clause",
                )),
            );
            return;
        }
        ctors.push(Some(format!("{owner}::constructor")));
    }

    env.exprs
        .record(call.span, ExprInfo::ProgramInstances { classes, ctors });
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};
    use nvs_syntax::ast::StmtKind;

    use crate::expr_table::{ExprInfo, ExprTypeTable};
    use crate::ty::TypeInterner;

    /// Checks `src` and hands back the table plus the span of its **first
    /// expression statement** — which every fixture below writes as the
    /// enumeration and nothing else, so there is no ambiguity about which
    /// entry is being read.
    fn check(src: &str) -> (ExprTypeTable, nvs_diagnostics::Span, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = nvs_syntax::parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = nvs_hir::resolve_file(&stmts, map.file(file), &mut diags);
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        crate::check_program(
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        let span = stmts
            .iter()
            .find_map(|stmt| match &stmt.kind {
                StmtKind::Expr(expr) => Some(expr.span),
                _ => None,
            })
            .expect("fixture must write the call as an expression statement");
        (exprs, span, diags)
    }

    /// `rule:programs/implementing`, all four clauses of it at once: the call records the
    /// expansion rather than a call, the entries are the **non-abstract**
    /// classes reaching the interface, they are **sorted by fully-qualified
    /// name**, and each carries the constructor `nvs-ir` is to name — the
    /// declaring one, or `None` where the class declares none.
    #[test]
    fn program_implementing_expands_to_new_expressions() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Zulu implements Module { public function tag(): string { return \"z\"; } }\n\
             class Alpha implements Module {\n\
                 public function constructor() {}\n\
                 public function tag(): string { return \"a\"; }\n\
             }\n\
             abstract class Base implements Module { public function tag(): string { return \"b\"; } }\n\
             Core\\Program::implementing<Module>();\n",
        );
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let Some(ExprInfo::ProgramInstances { classes, ctors }) = exprs.lookup(span) else {
            panic!("the call recorded no expansion: {:?}", exprs.lookup(span));
        };
        let names: Vec<String> = classes.iter().map(ToString::to_string).collect();
        // Source order is `Zulu`, `Alpha`; the answer is not.
        assert_eq!(names, ["Alpha", "Zulu"]);
        assert_eq!(
            ctors.as_slice(),
            [Some("Alpha::constructor".to_owned()), None]
        );
    }

    /// § 3's "`T` must be an interface type", reported where the type argument
    /// is written. A class satisfies every *other* thing the signature asks
    /// for — it is a named type, and one `new` could be written for it — so
    /// this is the refusal that keeps the enumeration's static-type promise.
    #[test]
    fn program_implementing_refuses_a_non_interface_type_argument() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             class Alpha {}\n\
             Core\\Program::implementing<Alpha>();\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code
                    == Some(nvs_diagnostics::code::E_PROGRAM_TYPE_ARG_NOT_AN_INTERFACE)),
            "expected E0743, got: {diags:?}"
        );
        assert!(
            !matches!(exprs.lookup(span), Some(ExprInfo::ProgramInstances { .. })),
            "a refused enumeration must record nothing for `nvs-ir` to lower"
        );
    }

    /// § 3's second refusal, and the bound it rests on asserted from both
    /// sides: what the expansion cannot write is `new Needy(8080)`, so the
    /// test is `required() > 0` rather than "declares a constructor" — the
    /// same class with the same parameter *defaulted* is enumerated and
    /// carries its constructor. A member that read the parameter count
    /// instead would refuse both and still look right on the first half
    /// alone. The class is named in the message because the fix is in that
    /// class rather than at the call the diagnostic points at.
    #[test]
    fn an_implementor_without_a_no_argument_constructor_is_named() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Needy implements Module {\n\
                 public function constructor(int $port) {}\n\
                 public function tag(): string { return \"n\"; }\n\
             }\n\
             Core\\Program::implementing<Module>();\n",
        );
        let named = diags.iter().find(|d| {
            d.code
                == Some(nvs_diagnostics::code::E_PROGRAM_IMPLEMENTOR_NEEDS_NO_ARGUMENT_CONSTRUCTOR)
        });
        let named = named.unwrap_or_else(|| panic!("expected E0744, got: {diags:?}"));
        assert!(
            named.message.contains("Needy"),
            "the diagnostic must name the class to fix: {}",
            named.message
        );
        assert!(
            !matches!(exprs.lookup(span), Some(ExprInfo::ProgramInstances { .. })),
            "a refused enumeration must record nothing for `nvs-ir` to lower"
        );

        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Needy implements Module {\n\
                 public function constructor(int $port = 8080) {}\n\
                 public function tag(): string { return \"n\"; }\n\
             }\n\
             Core\\Program::implementing<Module>();\n",
        );
        assert!(
            !diags.has_errors(),
            "an optional argument is not one: {diags:?}"
        );
        let Some(ExprInfo::ProgramInstances { classes, ctors }) = exprs.lookup(span) else {
            panic!("the call recorded no expansion: {:?}", exprs.lookup(span));
        };
        assert_eq!(
            classes.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["Needy"]
        );
        assert_eq!(ctors.as_slice(), [Some("Needy::constructor".to_owned())]);
    }
}
