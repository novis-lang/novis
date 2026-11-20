//! `rule:routing/a-match-reads-an-attribute-as-data`: `Core\Router\Match::attribute<T>()`,
//! folded while checking.
//!
//! The question a call asks — *which payload on the matched route's method is
//! written under the declaration `T`* — has an answer for every routed method before the program
//! starts, because a method's attributes are fixed by its source. Which method
//! matched is the one thing the call site cannot know. So this pass folds `T`
//! once per method carrying a `#[Route]`, exactly as
//! `Core\Attributes::get<T>(Class::method(...))` would fold it
//! ([`crate::retrieval`]), and records the answers on the call's
//! [`ResolvedCall::route_attributes`], keyed by the handler label
//! `crate::routes::check_class_routes` writes into the route table. At run time
//! the helper reads the matched row's label on its own side and returns that
//! entry, so the label never reaches the program.
//!
//! The attribute table is built before any body is checked, so the walk here
//! sees every routed method in the program, including one declared in a file
//! below the call. The route table itself is not complete yet at this point,
//! which is why the routed methods are found in the attribute table rather
//! than in [`crate::routes::RouteTable`].
//!
//! The refusals are [`crate::program::expand_with`]'s, for the same mistakes:
//! `T` that is not an attribute declaration is `E1008`, reported where the type
//! argument is lowered ([`crate::attributes::lower_retrieval_arg`]), and two
//! payloads of `T` on one routed method are `E0728` naming it. A method with no
//! payload of `T` has no entry, and the helper returns `null` for it.
//!
//! [`ResolvedCall::route_attributes`]: crate::expr_table::ResolvedCall::route_attributes

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::Type;

use crate::defaults::ConstArg;
use crate::retrieval::{fold_payload, matching, routed_methods};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env};

/// The payloads a `Core\Router\Match::attribute<T>` call folds to, keyed by
/// each routed method's `Class::method` label and sorted by it, or `None` for
/// a member off `nvs_stdlib::registry::ROUTE_ATTRIBUTE_MEMBERS` and for a call
/// this reports a refusal for.
///
/// `written` is the call's type arguments, already lowered, and `type_args`
/// what the call wrote. A missing or wrong one is reported where it is
/// lowered, so this adds nothing.
pub(crate) fn fold(
    owner: &QName,
    member: &str,
    (written, type_args): (&[TypeId], &[Type]),
    call: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<Vec<(String, ConstArg)>> {
    if !nvs_stdlib::registry::takes_route_attributes(&owner.to_string(), member) {
        return None;
    }
    let want = *written.first()?;
    // `E1008` or the name's own error has been reported already
    // (`rule:types/one-mistake-reports-once`).
    if env.interner.is_error(want) || !matches!(env.interner.get(want), Ty::Shape(_)) {
        return None;
    }
    let declared = crate::attributes::declaration_of(type_args.first()?, ctx, env)?;
    let mut entries = Vec::new();
    let mut refused = false;
    for (class, method, sites) in routed_methods(env) {
        let matched = matching(&sites, &declared.name, env);
        match matched.len() {
            0 => {}
            1 => match fold_payload(matched[0], env) {
                Some(value) => entries.push((format!("{class}::{method}"), value)),
                // `E0731` has been reported at the payload.
                None => refused = true,
            },
            count => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_ATTRIBUTE_RETRIEVAL_AMBIGUOUS,
                        format!("`{class}::{method}` has {count} `{}` attributes", declared.name),
                    )
                    .with_primary(call, "`attribute` returns at most one")
                    .with_help(format!(
                        "read `{class}::{method}` with `Core\\Attributes::all<T>(…)`, which \
                         returns every one"
                    )),
                );
                refused = true;
            }
        }
    }
    (!refused).then_some(entries)
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap, code};
    use nvs_syntax::ast::StmtKind;

    use crate::defaults::ConstArg;
    use crate::expr_table::{ExprInfo, ExprTypeTable};
    use crate::ty::TypeInterner;

    /// Checks `src` and returns the folded payloads of the call written as its
    /// **last expression statement**, or `None` where none were recorded,
    /// beside the diagnostics.
    fn check(src: &str) -> (Option<Vec<(String, ConstArg)>>, Diagnostics) {
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
            .rev()
            .find_map(|stmt| match &stmt.kind {
                StmtKind::Expr(expr) => Some(expr.span),
                _ => None,
            })
            .expect("fixture must write the call as an expression statement");
        let folded = match exprs.lookup(span) {
            Some(ExprInfo::Call(call)) => call.route_attributes.clone(),
            _ => None,
        };
        (folded, diags)
    }

    /// Two routed methods, one plain method and the call reading an attribute.
    fn program(shape: &str, show_attributes: &str) -> String {
        format!(
            "<?nvs\n\
             repeatable attribute Timeout on method {{ seconds: int, label?: string }}\n\
             class Shop {{\n\
                 #[Core\\Route(path: \"/shop\", method: Core\\Http\\Method::Get)]\n\
                 #[Core\\Access(allow: Core\\Audience::Public)]\n\
                 {show_attributes}\n\
                 public function show(): void {{}}\n\
                 #[Core\\Route(path: \"/list\", method: Core\\Http\\Method::Get)]\n\
                 #[Core\\Access(allow: Core\\Audience::Public)]\n\
                 public function index(): void {{}}\n\
                 #[Timeout(seconds: 5)]\n\
                 public function helper(): void {{}}\n\
             }}\n\
             ?Core\\Router\\Match $route = Core\\Router::match(Core\\Http\\Method::Get, \"/shop\");\n\
             $route?->attribute<{shape}>();\n"
        )
    }

    /// The call folds one entry per routed method holding a payload of the
    /// shape, keyed by `Class::method`. A routed method with no such payload
    /// and a method with no `#[Route]` have no entry.
    // covers: Core\Router\Match::attribute
    #[test]
    fn attribute_folds_one_payload_per_routed_method_that_has_one() {
        let (folded, diags) = check(&program("Timeout", "#[Timeout(seconds: 60, label: \"a\")]"));
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let folded = folded.expect("the call recorded its payloads");
        let labels: Vec<&str> = folded.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(labels, ["Shop::show"]);
        let ConstArg::Shape(fields) = &folded[0].1 else {
            panic!("a payload folds to a shape: {:?}", folded[0].1);
        };
        assert!(
            fields
                .iter()
                .any(|(name, value)| name == "seconds" && *value == ConstArg::Int(60)),
            "{fields:?}"
        );
    }

    /// A type argument that is not an attribute declaration is `E1008`, and
    /// the call records nothing. An inline shape is not one either.
    #[test]
    fn attribute_of_a_type_that_is_not_a_declaration_is_e1008() {
        for written in ["int", "{ seconds: int }"] {
            let (folded, diags) = check(&program(written, "#[Timeout(seconds: 60)]"));
            assert!(
                diags
                    .iter()
                    .any(|d| d.code == Some(code::E_ATTRIBUTE_TYPE_ARG_NOT_DECLARED)),
                "{written}: {diags:?}"
            );
            assert!(folded.is_none(), "{written}");
        }
    }

    /// A payload of another declaration with the same fields is not a match.
    #[test]
    fn attribute_finds_only_payloads_of_the_declaration_it_names() {
        let src = program("Timeout", "#[Delay(seconds: 60)]").replace(
            "class Shop",
            "attribute Delay on method { seconds: int }\nclass Shop",
        );
        let (folded, diags) = check(&src);
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        assert_eq!(folded.expect("the call recorded its payloads"), []);
    }

    /// Two payloads of the declaration on one routed method are `E0728`,
    /// naming the method.
    #[test]
    fn two_matching_payloads_on_one_routed_method_are_e0728() {
        let (folded, diags) = check(&program(
            "Timeout",
            "#[Timeout(seconds: 60)]\n#[Timeout(seconds: 30)]",
        ));
        let ambiguous: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_ATTRIBUTE_RETRIEVAL_AMBIGUOUS))
            .collect();
        assert_eq!(ambiguous.len(), 1, "{diags:?}");
        assert!(ambiguous[0].message.contains("`Shop::show`"), "{diags:?}");
        assert!(folded.is_none());
    }
}
