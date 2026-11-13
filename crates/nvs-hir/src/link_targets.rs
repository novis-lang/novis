//! The classes a program links to under an `autoload links` root
//! (`rule:programs/autoload`), and what a link needs to know about each.
//!
//! A links root is read for routes and never compiled. When the walk in
//! [`crate::requires`] finds a name the ordinary map cannot place and the
//! links half of [`crate::AutoloadMap`] can, it loads and parses that one file
//! and keeps what [`read`] takes out of it: the class's methods, and each
//! `#[Route]` with its path and the parameters it binds. Nothing in the file is
//! collected into the symbol table, so its classes never join the route table,
//! `implementing<T>()` or any lowering. The file is in the program's
//! [`nvs_diagnostics::SourceMap`], so its content joins the unit's revalidation
//! set exactly as a compiled file's does.
//!
//! **The one place such a name may be written is the route argument of
//! `Core\Router::url` and `Core\Router::urlAbsolute`.** [`crate::members`]
//! leaves that argument to `nvs_types::links`, which checks it against these
//! routes. Every other use of the name is an `E0303` from whichever pass met
//! it, and [`LinkTargets::mark_links_only`] turns each such report into
//! `E_LINKS_ONLY_CLASS`, which says why the class is not there.
//!
//! - **Memory:** one [`LinkTarget`] per linked class, holding its method names
//!   and its routes' paths and parameters, for the length of one compile.

use std::path::PathBuf;

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceId, code};
use nvs_syntax::ast::{
    AttributeGroup, ClassMemberKind, ExprKind, MethodMember, Param, Stmt, StmtKind, Type,
    TypeAtom, TypeKind,
};
use rustc_hash::FxHashMap;

use crate::QName;
use crate::hierarchy::resolve_ref;
use crate::resolve::{name_text, qname_segments};

/// `#[Route]`'s attribute, as `nvs_types::derive::ROUTE` names it.
const ROUTE: &str = r"Core\Route";
/// `#[Query]`'s attribute, as `nvs_types::derive::QUERY` names it.
const QUERY: &str = r"Core\Query";

/// Every class this program links to under a links root, by its full name.
#[derive(Clone, Debug, Default)]
pub struct LinkTargets {
    targets: FxHashMap<QName, LinkTarget>,
}

/// One class under a links root, as a link reads it.
#[derive(Clone, Debug)]
pub struct LinkTarget {
    /// The file that declares the class.
    pub file: SourceId,
    /// That file's canonical path.
    pub path: PathBuf,
    /// Whether the file declares the class at all. A file that declares
    /// something else is still the file the name resolves to, and a link to
    /// it reports the missing method.
    pub declared: bool,
    /// Every method the class declares, by name.
    pub methods: Vec<String>,
    /// Every `#[Route]` the class's methods carry, in declaration order.
    pub routes: Vec<LinkRoute>,
}

/// One `#[Route]` of a links-root class.
#[derive(Clone, Debug)]
pub struct LinkRoute {
    /// The method the attribute is on.
    pub method: String,
    /// Whether that method is `static`.
    pub is_static: bool,
    /// The `path` as written. `None` where it is not a plain string literal,
    /// which the module that owns the file reports when it compiles.
    pub path: Option<String>,
    /// The method's parameters, in declaration order.
    pub params: Vec<LinkParam>,
}

/// One parameter of a routed method.
#[derive(Clone, Debug)]
pub struct LinkParam {
    /// The name without its `$`.
    pub name: String,
    /// The declared type, where it is one a link value is checked against:
    /// `int`, `uint`, `decimal`, `string` or `Core\Uuid`, with a `?` dropped.
    /// `None` for every other type, such as a class or an enum of the links
    /// root, whose value is not checked.
    pub ty: Option<String>,
    /// Whether the parameter carries `#[Query]`.
    pub query: bool,
    /// Whether the parameter declares a default.
    pub has_default: bool,
}

impl LinkTargets {
    /// Records `name`'s target.
    pub(crate) fn insert(&mut self, name: QName, target: LinkTarget) {
        self.targets.insert(name, target);
    }

    /// The target `name` resolves to, or `None` where `name` is not under a
    /// links root.
    #[must_use]
    pub fn get(&self, name: &QName) -> Option<&LinkTarget> {
        self.targets.get(name)
    }

    /// Whether no name resolved through a links root.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Every target, in no fixed order.
    pub fn iter(&self) -> impl Iterator<Item = (&QName, &LinkTarget)> {
        self.targets.iter()
    }

    /// Turns each `E0303` for a name under a links root into
    /// `E_LINKS_ONLY_CLASS`. The class is not missing: it is declared under a
    /// root the program reads for links only, and the report says so.
    ///
    /// Run once every pass that may report such a name has run. A second run
    /// changes nothing, because a rewritten report no longer has `E0303`'s
    /// code.
    pub fn mark_links_only(&self, diags: &mut Diagnostics) {
        if self.targets.is_empty() {
            return;
        }
        for diagnostic in diags.iter_mut() {
            let Some(name) = crate::hierarchy::undeclared_subject(diagnostic) else {
                continue;
            };
            if !self.targets.contains_key(&name) {
                continue;
            }
            let span = diagnostic.primary_span();
            let mut marked = links_only(&name);
            if let Some(span) = span {
                marked = marked.with_primary(span, "declared for links only");
            }
            *diagnostic = marked;
        }
    }
}

/// `E_LINKS_ONLY_CLASS` for `name`, with no span.
fn links_only(name: &QName) -> Diagnostic {
    Diagnostic::error(
        code::E_LINKS_ONLY_CLASS,
        format!(
            "`{name}` is under an `autoload links` root, and a class there can only be the \
             route of a link"
        ),
    )
    .with_help(
        "write the class only as the first argument of `Core\\Router::url` or \
         `Core\\Router::urlAbsolute`. To use it in another way, declare its root with \
         `autoload` and no `links`",
    )
}

/// What a link needs of `class`, read out of `stmts`, the file the links map
/// placed it in. No method body is read.
pub(crate) fn read(
    stmts: &[Stmt],
    src: &SourceFile,
    class: &QName,
    file: SourceId,
    path: PathBuf,
) -> LinkTarget {
    let mut target = LinkTarget {
        file,
        path,
        declared: false,
        methods: Vec::new(),
        routes: Vec::new(),
    };
    let mut scope = Scope::default();
    read_in(stmts, src, class, &mut scope, &mut target);
    target
}

/// The namespace and imports in force where a declaration is written.
#[derive(Default, Clone)]
struct Scope {
    namespace: Vec<String>,
    imports: FxHashMap<String, QName>,
}

fn read_in(
    stmts: &[Stmt],
    src: &SourceFile,
    class: &QName,
    scope: &mut Scope,
    target: &mut LinkTarget,
) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(decl) => {
                let namespace = decl
                    .name
                    .as_ref()
                    .map_or_else(Vec::new, |name| qname_segments(src, name));
                match &decl.body {
                    Some(block) => {
                        let mut inner = Scope {
                            namespace,
                            imports: FxHashMap::default(),
                        };
                        read_in(&block.stmts, src, class, &mut inner, target);
                    }
                    None => {
                        scope.namespace = namespace;
                        scope.imports.clear();
                    }
                }
            }
            StmtKind::UseDecl(decl) => {
                let path = QName::parse(name_text(src, &decl.path));
                if let Some(last) = path.segments().last() {
                    scope.imports.insert(last.clone(), path.clone());
                }
            }
            StmtKind::ClassDecl(decl) => {
                let name = QName::join(&scope.namespace, name_text(src, &decl.name));
                if name != *class {
                    continue;
                }
                target.declared = true;
                for member in &decl.members {
                    let ClassMemberKind::Method(method) = &member.kind else {
                        continue;
                    };
                    read_method(method, src, scope, target);
                }
            }
            _ => {}
        }
    }
}

fn read_method(method: &MethodMember, src: &SourceFile, scope: &Scope, target: &mut LinkTarget) {
    let name = src.span_text(method.name).unwrap_or_default().to_owned();
    target.methods.push(name.clone());
    let is_static = method
        .modifiers
        .contains(&nvs_syntax::ast::Modifier::Static);
    let params: Vec<LinkParam> = method
        .params
        .iter()
        .map(|param| read_param(param, src, scope))
        .collect();
    for attr in method.attributes.iter().flat_map(|group| &group.attributes) {
        if attr.member.is_some() {
            continue;
        }
        let Some(written) = &attr.name else {
            continue;
        };
        if resolve_ref(name_text(src, written), &scope.namespace, &scope.imports).to_string()
            != ROUTE
        {
            continue;
        }
        let path = attr
            .fields
            .iter()
            .find(|field| src.span_text(field.name) == Some("path"))
            .and_then(|field| match field.value.unparenthesized().kind {
                ExprKind::Str(span) => crate::requires::cook_quoted(src, span),
                _ => None,
            });
        target.routes.push(LinkRoute {
            method: name.clone(),
            is_static,
            path,
            params: params.clone(),
        });
    }
}

fn read_param(param: &Param, src: &SourceFile, scope: &Scope) -> LinkParam {
    let name = src
        .span_text(param.name)
        .unwrap_or_default()
        .trim_start_matches('$')
        .to_owned();
    LinkParam {
        name,
        ty: param.ty.as_ref().and_then(|ty| checked_type(ty, src, scope)),
        query: has_attribute(&param.attributes, QUERY, src, scope),
        has_default: param.default.is_some(),
    }
}

/// The type a link value is checked against, as `nvs_types` renders it.
fn checked_type(ty: &Type, src: &SourceFile, scope: &Scope) -> Option<String> {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => checked_type(inner, src, scope),
        TypeKind::Atom(TypeAtom::Int) => Some("int".to_owned()),
        TypeKind::Atom(TypeAtom::Uint) => Some("uint".to_owned()),
        TypeKind::Atom(TypeAtom::Decimal) => Some("decimal".to_owned()),
        TypeKind::Atom(TypeAtom::String | TypeAtom::TaintedString) => Some("string".to_owned()),
        TypeKind::Atom(TypeAtom::Name(name, _)) => {
            let resolved = resolve_ref(name_text(src, name), &scope.namespace, &scope.imports);
            (resolved.to_string() == r"Core\Uuid").then(|| resolved.to_string())
        }
        _ => None,
    }
}

fn has_attribute(groups: &[AttributeGroup], want: &str, src: &SourceFile, scope: &Scope) -> bool {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .filter(|attr| attr.member.is_none())
        .filter_map(|attr| attr.name.as_ref())
        .any(|name| {
            resolve_ref(name_text(src, name), &scope.namespace, &scope.imports).to_string() == want
        })
}
