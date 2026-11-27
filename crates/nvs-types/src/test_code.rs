//! `rule:testing/tests-never-reach-a-build`'s test code: which declarations it is, and the
//! compile error a call into it from other code draws.
//!
//! **Test code is a method that carries one of the seven testing markers**: `#[Test]`,
//! `#[Bench]`, `#[Property]`, `#[Fixture]`, `#[BeforeClass]`, `#[AfterClass]` or
//! `#[AfterEachTest]`. Everything else the rule names rides on one of those methods and needs no
//! mark of its own: a data row is an attribute on a `#[Test]` method, and an assertion message and
//! a double are expressions in its body. The class that declares them is not test code. Its
//! constructor is the test's setup (`rule:testing/constructor-is-setup`), but a class with a
//! `#[Test]` method may be the class under test as well, and a mark it did not ask for would leave
//! its production methods out of a build.
//!
//! **The eighth way in is `#[TestOnly]`** (`rule:testing/a-test-only-declaration-is-test-code`).
//! On a method it makes that method test code. On a class, interface or enum it makes every method
//! of the type test code, the constructor included, and puts the type in a second set: the uses a
//! method set cannot see, such as a `new` or a type written in a signature, are judged against it.
//! A class is test code as a whole only when its author writes the attribute on it.
//!
//! [`check_hierarchy`] raises `E1019` where production code would still need a test-only
//! declaration: a production type that extends or implements a test-only one, a test-only method
//! of a production type that overrides or implements a production method, and a production method
//! that overrides a test-only one. A test-only type may implement a production interface, because
//! no object of it exists outside tests.
//!
//! [`TestCode`] is built whole before any body is checked, for
//! [`crate::deprecated::Deprecations`]' reason: a call may be written above the declaration it
//! names. It is O(test methods and test-only declarations in the program), once per compile, and
//! nothing per request.
//!
//! **Where `E0889` is raised.** [`crate::deprecated::window`] checks each body and then maps every
//! expression table entry the body recorded to the declaration it names. [`refuse_calls`] reads
//! the same entries. A call, a method reference or a class-reference call whose method resolves to
//! test code is refused, unless the body is test code itself. So is a `new`, a static property, a
//! class constant, an enum case and a `Foo::class` reference whose type is test-only. A body is
//! the whole method, so an anonymous function written inside a test is test code too.
//!
//! **A type that names a test-only type** is refused in two places, and both give the same
//! diagnostic, so [`Diagnostics`] keeps one where both see it. [`refuse_type`] runs where a type
//! is lowered inside a production body: a parameter, a return type, `is`, `catch`, `class<T>` and
//! every type argument there. [`refuse_signatures`] reads the declarations a body never lowers: a
//! property's type, a constant's type, a method with no body, and the type arguments of an
//! `implements` clause. A production signature may not name a test-only type at all, because a
//! build has no class for it.
//!
//! **Why a compile error and not a missing method at run time.** The same program compiles under
//! `nvs run` and `nvs test`. Without the error, a call into test code would type-check under both
//! and then name a function the build left out.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::{ClassGraph, QName};
use nvs_syntax::ast::{
    AttributeGroup, ClassMember, ClassMemberKind, Name, Stmt, StmtKind, Type, TypeAtom, TypeKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::deprecated::Member;
use crate::expr_table::ExprInfo;
use crate::{Env, ProgramFile, SignatureTable, span_text};

/// The seven markers that make a method test code.
const MARKERS: [&str; 7] = [
    crate::derive::TEST,
    crate::derive::BENCH,
    crate::derive::PROPERTY,
    crate::derive::FIXTURE,
    crate::derive::BEFORE_CLASS,
    crate::derive::AFTER_CLASS,
    crate::derive::AFTER_EACH_TEST,
];

/// Every method in the program that is test code, keyed by the type that declares it and the
/// method's name, and the subset of them and the types that `#[TestOnly]` marks.
#[derive(Default)]
pub(crate) struct TestCode {
    /// Every method that is test code, for any of the eight reasons.
    methods: FxHashSet<(QName, String)>,
    /// The methods `#[TestOnly]` makes test code, on the method or on its type.
    only_methods: FxHashSet<(QName, String)>,
    /// The classes, interfaces and enums that carry `#[TestOnly]`.
    only_types: FxHashSet<QName>,
}

impl TestCode {
    /// Whether method `name` of `owner`, the class that declares it, is test code.
    pub(crate) fn contains(&self, owner: &QName, name: &str) -> bool {
        !self.methods.is_empty() && self.methods.contains(&(owner.clone(), name.to_owned()))
    }

    /// Whether method `name` of `owner` is test code because of `#[TestOnly]`.
    pub(crate) fn is_test_only_method(&self, owner: &QName, name: &str) -> bool {
        !self.only_methods.is_empty()
            && self
                .only_methods
                .contains(&(owner.clone(), name.to_owned()))
    }

    /// Whether `qname` is a class, interface or enum that carries `#[TestOnly]`.
    pub(crate) fn is_test_only_type(&self, qname: &QName) -> bool {
        !self.only_types.is_empty() && self.only_types.contains(qname)
    }

    /// Whether a body that is `member` of `owner` is test code: a test-code method, or any
    /// member of a test-only type.
    pub(crate) fn is_test_body(&self, owner: Option<&QName>, member: Option<&Member>) -> bool {
        let Some(owner) = owner else {
            return false;
        };
        self.is_test_only_type(owner)
            || matches!(member, Some(Member::Method(name)) if self.contains(owner, name))
    }

    /// Whether the program has no test code at all, so no use can be refused.
    fn is_empty(&self) -> bool {
        self.methods.is_empty() && self.only_types.is_empty()
    }
}

/// `E0889` for a use of test-only type `qname` at `span`. [`refuse_type`] and
/// [`refuse_signatures`] both build it here, so the same use is one diagnostic.
fn type_use(span: Span, qname: &QName) -> Diagnostic {
    Diagnostic::error(
        code::E_CALL_INTO_TEST_CODE,
        format!("`{qname}` is test code, so only test code can use it"),
    )
    .with_primary(span, "this code is not a test")
    .with_help(
        "It is marked `#[TestOnly]`. Use it from a method with a testing attribute or from other \
         `#[TestOnly]` code.",
    )
}

/// `E0889` at a type written at `span` that names `qname`, when the body being checked is
/// production code and `qname` is a test-only type.
pub(crate) fn refuse_type(span: Span, qname: &QName, env: &mut Env<'_>) {
    if env.in_production_body && env.test_code.is_test_only_type(qname) {
        env.diags.report(type_use(span, qname));
    }
}

/// Every class name written in `ty`, at any depth: a union's members, a type argument, a
/// `class<T>`, a shape's fields and a callable's signature.
fn named_types<'t>(ty: &'t Type, out: &mut Vec<&'t Name>) {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => named_types(inner, out),
        TypeKind::Union(parts) | TypeKind::Intersection(parts) => {
            for part in parts {
                named_types(part, out);
            }
        }
        TypeKind::Atom(atom) => match atom {
            TypeAtom::Name(name, args) => {
                out.push(name);
                for arg in args {
                    named_types(arg, out);
                }
            }
            TypeAtom::Member(name, _) | TypeAtom::MemberCase(name, _, _) => out.push(name),
            TypeAtom::Array(key, value) => {
                for part in [key, value].into_iter().flatten() {
                    named_types(part, out);
                }
            }
            TypeAtom::ClassRef(inner)
            | TypeAtom::PropertyKey(inner)
            | TypeAtom::PropertyPath(inner)
            | TypeAtom::StringHoles { key: inner, .. } => named_types(inner, out),
            TypeAtom::Shape(fields) => {
                for field in fields {
                    named_types(&field.ty, out);
                }
            }
            TypeAtom::CallableSig { params, ret, .. } => {
                for param in params {
                    named_types(param, out);
                }
                named_types(ret, out);
            }
            _ => {}
        },
        _ => {}
    }
}

/// `E0889` at every type written in a production declaration's signature that names a test-only
/// type, where no body lowers it: see the module doc. With no `#[TestOnly]` type in the program
/// it returns at once.
pub(crate) fn refuse_signatures(
    files: &[ProgramFile<'_>],
    table: &TestCode,
    diags: &mut Diagnostics,
) {
    if table.only_types.is_empty() {
        return;
    }
    crate::deprecated::each_declaration(files, |stmt, src, current_ns, current_imports| {
        let Some((name, _, members)) = type_parts(stmt) else {
            return;
        };
        let owner = QName::join(current_ns, span_text(src, name.span));
        if table.is_test_only_type(&owner) {
            return;
        }
        let mut written: Vec<&Type> = Vec::new();
        if let StmtKind::ClassDecl(d) = &stmt.kind {
            written.extend(d.implements.iter().flat_map(|clause| &clause.type_args));
        }
        for member in members {
            match &member.kind {
                ClassMemberKind::Method(m) => {
                    if table.contains(&owner, span_text(src, m.name)) {
                        continue;
                    }
                    written.extend(m.params.iter().filter_map(|p| p.ty.as_ref()));
                    written.extend(m.return_type.as_ref());
                }
                ClassMemberKind::Property(p) => written.push(&p.ty),
                ClassMemberKind::Const(c) => written.extend(c.ty.as_ref()),
                _ => {}
            }
        }
        let mut names = Vec::new();
        for ty in written {
            named_types(ty, &mut names);
        }
        for name in names {
            let resolved =
                nvs_hir::resolve_ref(span_text(src, name.span), current_ns, current_imports);
            if table.is_test_only_type(&resolved) {
                diags.report(type_use(name.span, &resolved));
            }
        }
    });
}

/// Whether one of `groups` names one of `wanted`, resolved in the scope the declaration is in.
fn names_one_of(
    groups: &[AttributeGroup],
    wanted: &[&str],
    src: &nvs_diagnostics::SourceFile,
    current_ns: &[String],
    current_imports: &FxHashMap<String, QName>,
) -> bool {
    groups.iter().flat_map(|g| &g.attributes).any(|attr| {
        attr.member.is_none()
            && attr.name.as_ref().is_some_and(|n| {
                let written =
                    nvs_hir::resolve_ref(span_text(src, n.span), current_ns, current_imports);
                wanted.iter().any(|want| written == QName::parse(want))
            })
    })
}

/// The name, attributes and members of a class, interface or enum, or `None` for any other
/// statement.
fn type_parts(stmt: &Stmt) -> Option<(&Name, &[AttributeGroup], &[ClassMember])> {
    match &stmt.kind {
        StmtKind::ClassDecl(d) => Some((&d.name, &d.attributes, &d.members)),
        StmtKind::InterfaceDecl(d) => Some((&d.name, &d.attributes, &d.members)),
        StmtKind::EnumDecl(d) => Some((&d.name, &d.attributes, &d.members)),
        _ => None,
    }
}

/// Collects [`TestCode`] over every file, through [`crate::deprecated::each_declaration`], so a
/// marker's name means here what it means to the `#[Test]` table.
pub(crate) fn build_table(files: &[ProgramFile<'_>]) -> TestCode {
    let mut table = TestCode::default();
    crate::deprecated::each_declaration(files, |stmt, src, current_ns, current_imports| {
        let Some((name, attributes, members)) = type_parts(stmt) else {
            return;
        };
        let owner = QName::join(current_ns, span_text(src, name.span));
        let only = [crate::derive::TEST_ONLY];
        let type_only = names_one_of(attributes, &only, src, current_ns, current_imports);
        if type_only {
            table.only_types.insert(owner.clone());
        }
        for member in members {
            let ClassMemberKind::Method(m) = &member.kind else {
                continue;
            };
            let key = (owner.clone(), span_text(src, m.name).to_owned());
            let method_only =
                type_only || names_one_of(&m.attributes, &only, src, current_ns, current_imports);
            if method_only {
                table.only_methods.insert(key.clone());
                table.methods.insert(key);
            } else if names_one_of(&m.attributes, &MARKERS, src, current_ns, current_imports) {
                table.methods.insert(key);
            }
        }
    });
    table
}

/// `E1019` at every place production code would still need a test-only declaration, as the
/// module doc lists them. With no `#[TestOnly]` in the program it returns at once.
pub(crate) fn check_hierarchy(
    files: &[ProgramFile<'_>],
    table: &TestCode,
    signatures: &SignatureTable,
    graph: &ClassGraph,
    diags: &mut Diagnostics,
) {
    if table.only_methods.is_empty() && table.only_types.is_empty() {
        return;
    }
    crate::deprecated::each_declaration(files, |stmt, src, current_ns, current_imports| {
        let Some((name, _, members)) = type_parts(stmt) else {
            return;
        };
        let owner = QName::join(current_ns, span_text(src, name.span));
        if table.is_test_only_type(&owner) {
            return;
        }
        let parents: Vec<&Name> = match &stmt.kind {
            StmtKind::ClassDecl(d) => d
                .extends
                .iter()
                .chain(d.implements.iter().map(|clause| &clause.name))
                .collect(),
            StmtKind::InterfaceDecl(d) => d.extends.iter().collect(),
            _ => Vec::new(),
        };
        for parent in parents {
            let written = span_text(src, parent.span);
            let resolved = nvs_hir::resolve_ref(written, current_ns, current_imports);
            if table.is_test_only_type(&resolved) {
                diags.report(
                    Diagnostic::error(
                        code::E_TEST_ONLY_IN_PRODUCTION_HIERARCHY,
                        format!("`{owner}` is not test code, so it cannot extend or implement `{resolved}`"),
                    )
                    .with_primary(parent.span, "this type is only for tests")
                    .with_help(format!(
                        "`{resolved}` is marked `#[TestOnly]`. Mark `{owner}` `#[TestOnly]` too, \
                         or remove the attribute from `{resolved}`."
                    )),
                );
            }
        }
        let Some(links) = graph.get(&owner) else {
            return;
        };
        for member in members {
            let ClassMemberKind::Method(m) = &member.kind else {
                continue;
            };
            let method = span_text(src, m.name);
            if method == "constructor" {
                continue;
            }
            let overridden = links
                .extends
                .iter()
                .chain(&links.implements)
                .find_map(|parent| {
                    crate::signatures::resolve_method_ref(parent, method, signatures, graph)
                        .map(|(declaring, _)| declaring.clone())
                });
            let Some(overridden) = overridden else {
                continue;
            };
            if table.is_test_only_method(&owner, method) && !table.contains(&overridden, method) {
                diags.report(
                    Diagnostic::error(
                        code::E_TEST_ONLY_IN_PRODUCTION_HIERARCHY,
                        format!(
                            "`{owner}::{method}` is `#[TestOnly]`, but it overrides \
                             `{overridden}::{method}`, which is not test code"
                        ),
                    )
                    .with_primary(
                        m.name,
                        format!("production code can call this method through `{overridden}`"),
                    )
                    .with_help(
                        "Remove `#[TestOnly]` from this method, or give it a name that the parent \
                         type does not have.",
                    ),
                );
            } else if !table.contains(&owner, method)
                && table.is_test_only_method(&overridden, method)
            {
                diags.report(
                    Diagnostic::error(
                        code::E_TEST_ONLY_IN_PRODUCTION_HIERARCHY,
                        format!(
                            "`{owner}::{method}` overrides `{overridden}::{method}`, which is \
                             `#[TestOnly]`"
                        ),
                    )
                    .with_primary(m.name, "this method is not test code")
                    .with_help(format!("Mark `{owner}::{method}` `#[TestOnly]` too.")),
                );
            }
        }
    });
}

/// `E0889` at every call the entries recorded after `mark` make into test code, unless the body
/// they were recorded for is method `member` of `owner` and that method is test code.
pub(crate) fn refuse_calls(
    owner: Option<&QName>,
    member: Option<&Member>,
    mark: usize,
    env: &mut Env<'_>,
) {
    if env.test_code.is_empty() || env.test_code.is_test_body(owner, member) {
        return;
    }
    let mut found: Vec<(Span, QName, Option<String>)> = Vec::new();
    for (span, info) in env.exprs.since_at(mark) {
        match info {
            ExprInfo::Call(call) | ExprInfo::CallableRef(call) | ExprInfo::ClassRefCall(call) => {
                let Some((declaring, _)) = crate::signatures::resolve_method(
                    &call.class,
                    &call.method,
                    env.signatures,
                    env.graph,
                ) else {
                    continue;
                };
                if env.test_code.contains(&declaring, &call.method) {
                    found.push((span, declaring, Some(call.method.clone())));
                }
            }
            ExprInfo::New { class, .. }
            | ExprInfo::StaticProperty { class, .. }
            | ExprInfo::ClassConst { class, .. }
            | ExprInfo::ClassConstLate { class, .. }
            | ExprInfo::ClassRefConst { class }
            | ExprInfo::EnumCase { enum_: class, .. }
                if env.test_code.is_test_only_type(class) =>
            {
                found.push((span, class.clone(), None));
            }
            _ => {}
        }
    }
    let mut seen = FxHashSet::default();
    for (span, declaring, method) in found {
        if !seen.insert(span) {
            continue;
        }
        let diag = match method {
            None => type_use(span, &declaring),
            Some(method) => {
                let only = env.test_code.is_test_only_method(&declaring, &method);
                Diagnostic::error(
                    code::E_CALL_INTO_TEST_CODE,
                    format!("`{declaring}::{method}` is test code, so only test code can use it"),
                )
                .with_primary(span, "this code is not a test")
                .with_help(if only {
                    "It is marked `#[TestOnly]`. Call it from a method with a testing attribute \
                     or from other `#[TestOnly]` code."
                } else {
                    "`nvs run` and `nvs build` do not compile test code. Move the shared steps \
                     into a method without a testing attribute, and call that method from both \
                     places."
                })
            }
        };
        env.diags.report(diag);
    }
}
