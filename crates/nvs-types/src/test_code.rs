//! `rule:testing/tests-never-reach-a-build`'s test code: which declarations it is, and the
//! compile error a call into it from other code draws.
//!
//! **Test code is a method that carries one of the seven testing markers**: `#[Test]`,
//! `#[Bench]`, `#[Property]`, `#[Fixture]`, `#[BeforeClass]`, `#[AfterClass]` or `#[AfterEachTest]`. Everything else the rule
//! names rides on one of those methods and needs no mark of its own: a data row is an attribute
//! on a `#[Test]` method, and an assertion message and a double are expressions in its body. The
//! class that declares them is not test code. Its constructor is the test's setup
//! (`rule:testing/constructor-is-setup`), but a class with a `#[Test]` method may be the class
//! under test as well, and a mark on the class would leave its production methods out of a build.
//!
//! [`TestCode`] is built whole before any body is checked, for
//! [`crate::deprecated::Deprecations`]' reason: a call may be written above the declaration it
//! names. It is O(test methods in the program), once per compile.
//!
//! **Where `E0889` is raised.** [`crate::deprecated::window`] checks each body and then maps every
//! expression table entry the body recorded to the declaration it names. [`refuse_calls`] reads
//! the same entries. A call, a method reference or a class-reference call whose method resolves to
//! test code is refused, unless the body is test code itself. A body is the whole method, so an
//! anonymous function written inside a test is test code too.
//!
//! **Why a compile error and not a missing method at run time.** The same program compiles under
//! `nvs run` and `nvs test`. Without the error, a call into test code would type-check under both
//! and then name a function the build left out.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{AttributeGroup, ClassMemberKind, NamespaceDecl, Stmt, StmtKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::deprecated::Member;
use crate::expr_table::ExprInfo;
use crate::{Env, ProgramFile, span_text};

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

/// Every method in the program that is test code, keyed by the class or interface that declares
/// it and the method's name.
#[derive(Default)]
pub(crate) struct TestCode(FxHashSet<(QName, String)>);

impl TestCode {
    /// Whether method `name` of `owner`, the class that declares it, is test code.
    pub(crate) fn contains(&self, owner: &QName, name: &str) -> bool {
        !self.0.is_empty() && self.0.contains(&(owner.clone(), name.to_owned()))
    }
}

/// Collects [`TestCode`] over every file. The namespace and `use` walk is
/// [`crate::deprecated::build_table`]'s, so a marker's name means here what it means to the
/// `#[Test]` table.
pub(crate) fn build_table(files: &[ProgramFile<'_>]) -> TestCode {
    let mut table = TestCode::default();
    for file in files {
        collect(file.stmts, file.src, &[], &FxHashMap::default(), &mut table);
    }
    table
}

fn collect(
    stmts: &[Stmt],
    src: &nvs_diagnostics::SourceFile,
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    table: &mut TestCode,
) {
    let mut current_ns = namespace.to_vec();
    let mut current_imports = imports.clone();
    for stmt in stmts {
        let (name, members) = match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name.as_ref().map_or_else(Vec::new, |n| {
                    QName::parse(span_text(src, n.span)).segments().to_vec()
                });
                match body {
                    Some(block) => {
                        collect(&block.stmts, src, &new_ns, &FxHashMap::default(), table);
                    }
                    None => {
                        current_ns = new_ns;
                        current_imports.clear();
                    }
                }
                continue;
            }
            StmtKind::UseDecl(use_decl) => {
                let target = QName::parse(span_text(src, use_decl.path.span));
                current_imports.insert(target.short_name().to_owned(), target);
                continue;
            }
            StmtKind::ClassDecl(d) => (&d.name, &d.members),
            StmtKind::InterfaceDecl(d) => (&d.name, &d.members),
            _ => continue,
        };
        let owner = QName::join(&current_ns, span_text(src, name.span));
        let marked = |groups: &[AttributeGroup]| {
            groups.iter().flat_map(|g| &g.attributes).any(|attr| {
                attr.member.is_none()
                    && attr.name.as_ref().is_some_and(|n| {
                        let written = nvs_hir::resolve_ref(
                            span_text(src, n.span),
                            &current_ns,
                            &current_imports,
                        );
                        MARKERS.iter().any(|marker| written == QName::parse(marker))
                    })
            })
        };
        for member in members {
            if let ClassMemberKind::Method(m) = &member.kind
                && marked(&m.attributes)
            {
                table
                    .0
                    .insert((owner.clone(), span_text(src, m.name).to_owned()));
            }
        }
    }
}

/// `E0889` at every call the entries recorded after `mark` make into test code, unless the body
/// they were recorded for is method `member` of `owner` and that method is test code.
pub(crate) fn refuse_calls(
    owner: Option<&QName>,
    member: Option<&Member>,
    mark: usize,
    env: &mut Env<'_>,
) {
    if env.test_code.0.is_empty() {
        return;
    }
    if let (Some(owner), Some(Member::Method(name))) = (owner, member)
        && env.test_code.contains(owner, name)
    {
        return;
    }
    let mut found: Vec<(Span, QName, String)> = Vec::new();
    for (span, info) in env.exprs.since_at(mark) {
        let (ExprInfo::Call(call) | ExprInfo::CallableRef(call) | ExprInfo::ClassRefCall(call)) =
            info
        else {
            continue;
        };
        let Some((declaring, _)) =
            crate::signatures::resolve_method(&call.class, &call.method, env.signatures, env.graph)
        else {
            continue;
        };
        if env.test_code.contains(&declaring, &call.method) {
            found.push((span, declaring, call.method.clone()));
        }
    }
    let mut seen = FxHashSet::default();
    for (span, declaring, method) in found {
        if !seen.insert(span) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_CALL_INTO_TEST_CODE,
                format!("`{declaring}::{method}` is test code, so only a test can call it"),
            )
            .with_primary(span, "this code is not a test")
            .with_help(
                "`nvs run` and `nvs build` do not compile test code. Move the shared steps into \
                 a method without a testing attribute, and call that method from both places",
            ),
        );
    }
}
