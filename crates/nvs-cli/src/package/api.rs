//! `nvs pkg api <dir>` — a package's public API as JSON
//! (`rule:packaging/only-a-major-may-break-the-api`).
//!
//! The package's `src` directory is compiled through an entry that is never on disk: one `require`
//! per `.nvs` file, in path order ([`crate::directory_entry`]). The entry is checked and nothing is
//! run, so a top-level statement in a package file has no effect here
//! (`rule:packaging/nothing-runs-before-the-program`). A package that does not check prints nothing
//! and exits non-zero, as `nvs check` does. A file outside `src`, such as a test, is not the API. A
//! package whose source names another package's declarations does not check, because nothing here
//! fetches its dependencies. An extension package's one `.nvsx` is loaded and checked as `nvs pkg
//! pack` checks it, and the compile is typed against its manifest and carried source, so a class
//! under `src` that calls the component checks; nothing of the component runs.
//!
//! ## What is written
//!
//! Every class, interface, enum and `type` alias the package declares, an enum or alias a class
//! body owns included under its `Owner::Name`. For each, every public and protected method,
//! property and constant: a private member is no caller's business. A promoted constructor
//! parameter is a property. A method lists its parameters in order, each with its name, its type,
//! and whether a caller may leave it out; and its return type. A class lists the parent and the
//! interfaces it names itself; a comparison closes over the parents the document lists.
//!
//! **A type is written resolved and normalised**, so a change of spelling is not a change: every
//! class name is fully qualified by [`nvs_hir::resolve_ref`], the one function that decides what a
//! name means; an alias is replaced by what it stands for; `self` and `parent` are the class they
//! name; `?T` is `T|null`; and a union's and an intersection's members are sorted. What is not the
//! API's shape is left out: a doc comment, a default's value, a constant's value and an enum case's
//! value. Behaviour may change in any release, and only shape is promised.
//!
//! ## Deterministic
//!
//! Every list is sorted by name, except a method's parameters, whose order is the API. The keys are
//! sorted by `serde_json`'s map, and the pretty printer writes `\n` on every platform, so two runs
//! over one package are byte-identical anywhere. `schema` is [`SCHEMA`], and goes up when a reader
//! of an older document could misread a newer one.
//!
//! ## Doc comments
//!
//! [`extract_counted`] also returns the [`Docs`] of the declarations the document lists, for
//! `nvs registry build` (`rule:packaging/search-sorts-by-the-readers-choice`): how many have a doc
//! comment, of how many, and their text for the registry's search. They are kept out of the
//! document, because a doc comment is not the API's shape.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use nvs_diagnostics::{SourceFile, SourceMap, Span};
use nvs_hir::QName;
use nvs_syntax::ast::{
    ClassDecl, ClassMember, ClassMemberKind, ConstMember, DocComment, EnumDecl, InterfaceDecl,
    MethodMember, Modifier, Name, Param, PropertyMember, Stmt, StmtKind, Type, TypeAtom, TypeKind,
    Visibility,
};
use rustc_hash::FxHashMap;
use serde_json::{Map, Value};

/// The document's shape version.
pub(crate) const SCHEMA: u64 = 1;

/// The file name the package is compiled through. Nothing is written there: the source map serves
/// the entry from memory.
const ENTRY: &str = ".nvs-pkg-api.nvs";

/// How deep a type may nest, aliases expanded, before it is written as its source text. The
/// checker refuses an alias cycle before this runs, so only a pathological type reaches it.
const MAX_DEPTH: u32 = 64;

/// Prints the API of the package in `dir`, and returns success; or returns failure with the
/// reason on standard error.
pub(crate) fn run(dir: &Path) -> ExitCode {
    match extract(dir) {
        Ok(document) => {
            let text =
                serde_json::to_string_pretty(&document).expect("a tree of strings serialises");
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

/// The most bytes of doc-comment text [`Docs::text`] keeps. The registry indexes it for search,
/// once per package, so this bounds what one package adds to the index.
pub(crate) const MAX_DOCS_TEXT: usize = 32 * 1024;

/// The `///` doc comments of a package's public declarations. A declaration is a class,
/// interface, enum or `type` alias, an enum case, and each constant, property and method the
/// document lists. A promoted constructor parameter has no doc comment of its own, so it is not
/// counted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Docs {
    /// The declarations with a doc comment.
    pub(crate) documented: usize,
    /// Every declaration counted.
    pub(crate) public: usize,
    /// The comments' lines without their `///`, one per line, in declaration order. A line that
    /// would pass [`MAX_DOCS_TEXT`] is left out, and so is every line after it.
    pub(crate) text: String,
    /// Whether a line was left out, so no later line is added.
    full: bool,
}

impl Docs {
    fn add(&mut self, doc: Option<&DocComment>, src: &SourceFile) {
        self.public += 1;
        let Some(doc) = doc else { return };
        self.documented += 1;
        for &line in &doc.lines {
            let line = text(src, line).trim_start_matches('/').trim();
            if self.full || line.is_empty() {
                continue;
            }
            if self.text.len() + line.len() + 1 > MAX_DOCS_TEXT {
                self.full = true;
                continue;
            }
            self.text.push_str(line);
            self.text.push('\n');
        }
    }
}

/// The API document of the package in `dir`, or the exit code to return. Each way to fail has
/// already reported on standard error.
pub(crate) fn extract(dir: &Path) -> Result<Value, ExitCode> {
    extract_counted(dir, component(dir)?).map(|(document, _)| document)
}

/// The manifest of the package's one `.nvsx`, checked by [`super::pack::check_component`] over
/// the package's [`super::pack::file_set`], or `None` for a folder with no `package.toml`, an
/// application's, or a package whose `files` lists no `.nvsx`.
fn component(dir: &Path) -> Result<Option<nvs_ext::manifest::Manifest>, ExitCode> {
    let path = dir.join(nvs_config::grants::FILE);
    if !path.is_file() {
        return Ok(None);
    }
    let mut sources = SourceMap::new();
    let manifest = super::manifest::read(&path, &mut sources).map_err(|diagnostic| {
        let mut err = std::io::stderr().lock();
        // Standard error that cannot be written to has nobody to tell.
        let _ = nvs_diagnostics::Renderer::new().render(&diagnostic, &sources, &mut err);
        ExitCode::FAILURE
    })?;
    let Some(name) = &manifest.name else {
        return Ok(None);
    };
    // A package with no `.nvsx` compiles as before, so this command never makes the checks that
    // only a pack needs, such as the license file.
    if !manifest
        .files
        .iter()
        .any(|file| super::pack::is_component(file))
    {
        return Ok(None);
    }
    super::pack::file_set(dir, &manifest)
        .and_then(|entries| super::pack::check_component(&entries, name))
        .map_err(|refusal| {
            eprintln!("error: {refusal}");
            ExitCode::FAILURE
        })
}

/// [`extract`]'s document, and its declarations' [`Docs`]. `component` is the package's
/// extension, which the compile is typed against, so a class under `src` that calls it checks
/// (`rule:packaging/a-package-is-its-digest`).
pub(crate) fn extract_counted(
    dir: &Path,
    component: Option<nvs_ext::manifest::Manifest>,
) -> Result<(Value, Docs), ExitCode> {
    let src = dir.join("src");
    let mut files = Vec::new();
    let mut cases = false;
    if let Err(err) = crate::programs_under(&src, &mut files, &mut cases) {
        eprintln!("cannot read {}: {err}", src.display());
        return Err(ExitCode::FAILURE);
    }
    if files.is_empty() {
        eprintln!("{} contains no `.nvs` file", src.display());
        return Err(ExitCode::FAILURE);
    }
    let entry = src.join(ENTRY);
    let mut map = SourceMap::new();
    map.overlay(&entry, crate::directory_entry(&src, &files));
    let checked = crate::front_end_in(
        map,
        &entry,
        None,
        component.into_iter().collect(),
        // A package's API is the same under every deployment's `[regex]` policy.
        nvs_types::Backtracking::Allow,
        false,
        crate::Sink::Text,
        crate::config::Init::Never,
        None,
        None,
    )?;
    Ok(document(&checked))
}

/// The document over a checked program: every declaration but the entry's, which has none.
fn document(checked: &crate::Checked) -> (Value, Docs) {
    let mut decls = Vec::new();
    for file in checked.files.iter().filter(|file| file.id != checked.id) {
        collect(&file.stmts, checked.map.file(file.id), "", &mut decls);
    }
    decls.sort_by(|a, b| a.name.cmp(&b.name));
    let index = decls
        .iter()
        .enumerate()
        .map(|(at, decl)| (decl.name.as_str(), at))
        .collect();
    let api = Api {
        decls: &decls,
        index,
    };
    let mut out = Map::new();
    out.insert("schema".into(), Value::from(SCHEMA));
    out.insert(
        "declarations".into(),
        Value::Array(decls.iter().map(|decl| api.decl_json(decl)).collect()),
    );
    (Value::Object(out), docs(&decls))
}

/// The [`Docs`] of `decls` and of the members the document lists for them. An enum or alias a
/// body owns is one of `decls`, so the member walk skips it.
fn docs(decls: &[Decl<'_>]) -> Docs {
    let mut count = Docs::default();
    for decl in decls {
        let src = decl.site.src;
        count.add(decl.doc, src);
        let members = match decl.item {
            Item::Class(class) => class.members.as_slice(),
            Item::Interface(interface) => interface.members.as_slice(),
            Item::Enum(decl) => {
                for case in &decl.cases {
                    count.add(case.doc.as_ref(), src);
                }
                decl.members.as_slice()
            }
            Item::Alias(_) => &[],
        };
        for member in members {
            let listed = match &member.kind {
                ClassMemberKind::Const(constant) => visible(&constant.modifiers),
                ClassMemberKind::Property(property) => visible(&property.modifiers),
                ClassMemberKind::Method(method) => visible(&method.modifiers),
                _ => false,
            };
            if listed {
                count.add(member.doc.as_ref(), src);
            }
        }
    }
    count
}

/// One declaration, and where its names are resolved.
struct Decl<'a> {
    /// Fully qualified; `Owner::Name` for an enum or alias a body owns.
    name: String,
    item: Item<'a>,
    site: Site<'a>,
    /// The `///` run above it, for [`docs`].
    doc: Option<&'a DocComment>,
}

enum Item<'a> {
    Class(&'a ClassDecl),
    Interface(&'a InterfaceDecl),
    Enum(&'a EnumDecl),
    Alias(&'a Type),
}

/// What a name written at one place needs to resolve as the compiler resolved it.
#[derive(Clone)]
struct Site<'a> {
    src: &'a SourceFile,
    namespace: Vec<String>,
    /// Each `use` import, by the short name it brings into scope.
    imports: FxHashMap<String, QName>,
    /// The class, interface or enum whose body this is: what `self` names, and where a short
    /// owned enum or alias name is looked up first.
    owner: Option<String>,
    /// The owner's parent class: what `parent` names.
    parent: Option<String>,
}

impl Site<'_> {
    /// `name` as written, fully qualified, before the owner's own enums and aliases are tried.
    fn resolve(&self, name: Name) -> String {
        nvs_hir::resolve_ref(text(self.src, name.span), &self.namespace, &self.imports).to_string()
    }
}

/// Walks one statement list, as `crate::meta`'s program half walks it: a bracketed namespace is
/// walked with its own scope, and the statement form starts a fresh one for the rest of the list.
fn collect<'a>(stmts: &'a [Stmt], src: &'a SourceFile, enclosing: &str, out: &mut Vec<Decl<'a>>) {
    let mut site = Site {
        src,
        namespace: segments(enclosing),
        imports: FxHashMap::default(),
        owner: None,
        parent: None,
    };
    let mut namespace = enclosing.to_owned();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(decl) => {
                let name = decl
                    .name
                    .map(|name| text(src, name.span).to_owned())
                    .unwrap_or_default();
                match &decl.body {
                    Some(block) => collect(&block.stmts, src, &name, out),
                    None => {
                        site.namespace = segments(&name);
                        site.imports.clear();
                        namespace = name;
                    }
                }
            }
            // An import is never renamed: the parser refuses `as`.
            StmtKind::UseDecl(decl) => {
                let target = QName::parse(text(src, decl.path.span));
                site.imports.insert(target.short_name().to_owned(), target);
            }
            StmtKind::ClassDecl(decl) => {
                let name = qualify(&namespace, text(src, decl.name.span));
                let parent = decl.extends.map(|parent| site.resolve(parent));
                let inner = Site {
                    owner: Some(name.clone()),
                    parent,
                    ..site.clone()
                };
                owned(&decl.members, &name, &inner, out);
                out.push(Decl {
                    name,
                    item: Item::Class(decl),
                    site: inner,
                    doc: decl.doc.as_ref(),
                });
            }
            StmtKind::InterfaceDecl(decl) => {
                let name = qualify(&namespace, text(src, decl.name.span));
                let inner = Site {
                    owner: Some(name.clone()),
                    ..site.clone()
                };
                owned(&decl.members, &name, &inner, out);
                out.push(Decl {
                    name,
                    item: Item::Interface(decl),
                    site: inner,
                    doc: decl.doc.as_ref(),
                });
            }
            StmtKind::EnumDecl(decl) => {
                let name = qualify(&namespace, text(src, decl.name.span));
                let inner = Site {
                    owner: Some(name.clone()),
                    ..site.clone()
                };
                out.push(Decl {
                    name,
                    item: Item::Enum(decl),
                    site: inner,
                    doc: decl.doc.as_ref(),
                });
            }
            StmtKind::TypeAliasDecl(decl) => out.push(Decl {
                name: qualify(&namespace, text(src, decl.name.span)),
                item: Item::Alias(&decl.ty),
                site: site.clone(),
                doc: decl.doc.as_ref(),
            }),
            _ => {}
        }
    }
}

/// The enums and aliases a class or interface body owns, each under `Owner::Name`
/// (`rule:enums/class-scoped-enum`, `rule:types/type-alias`). An alias resolves in its owner's
/// body; an owned enum's body is its own.
fn owned<'a>(members: &'a [ClassMember], owner: &str, site: &Site<'a>, out: &mut Vec<Decl<'a>>) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Enum(decl) => {
                let name = format!("{owner}::{}", text(site.src, decl.name.span));
                out.push(Decl {
                    item: Item::Enum(decl),
                    site: Site {
                        owner: Some(name.clone()),
                        parent: None,
                        ..site.clone()
                    },
                    name,
                    doc: member.doc.as_ref().or(decl.doc.as_ref()),
                });
            }
            ClassMemberKind::TypeAlias(decl) => out.push(Decl {
                name: format!("{owner}::{}", text(site.src, decl.name.span)),
                item: Item::Alias(&decl.ty),
                site: site.clone(),
                doc: member.doc.as_ref().or(decl.doc.as_ref()),
            }),
            _ => {}
        }
    }
}

/// Every declaration, and an index by name, for resolving a short owned name and expanding an
/// alias.
struct Api<'d, 'a> {
    decls: &'d [Decl<'a>],
    index: FxHashMap<&'d str, usize>,
}

impl<'a> Api<'_, 'a> {
    fn lookup(&self, name: &str) -> Option<&Decl<'a>> {
        self.index.get(name).map(|&at| &self.decls[at])
    }

    fn decl_json(&self, decl: &Decl<'_>) -> Value {
        let site = &decl.site;
        let mut out = Map::new();
        out.insert("name".into(), Value::from(decl.name.as_str()));
        match decl.item {
            Item::Class(class) => {
                out.insert("kind".into(), Value::from("class"));
                for (key, modifier) in [
                    ("abstract", Modifier::Abstract),
                    ("final", Modifier::Final),
                    ("readonly", Modifier::Readonly),
                ] {
                    out.insert(key.into(), Value::from(class.modifiers.contains(&modifier)));
                }
                if let Some(parent) = &site.parent {
                    out.insert("extends".into(), Value::from(parent.as_str()));
                }
                let implements = class
                    .implements
                    .iter()
                    .map(|clause| site.resolve(clause.name));
                out.insert("implements".into(), sorted(implements));
                self.body(&class.members, site, false, &mut out);
            }
            Item::Interface(interface) => {
                out.insert("kind".into(), Value::from("interface"));
                let extends = interface.extends.iter().map(|parent| site.resolve(*parent));
                out.insert("extends".into(), sorted(extends));
                self.body(&interface.members, site, true, &mut out);
            }
            Item::Enum(decl) => {
                out.insert("kind".into(), Value::from("enum"));
                if let Some(backing) = &decl.backing {
                    out.insert("backing".into(), Value::from(self.ty(backing, site, 0)));
                }
                let implements = decl.implements.iter().map(|name| site.resolve(*name));
                out.insert("implements".into(), sorted(implements));
                let cases = decl
                    .cases
                    .iter()
                    .map(|case| text(site.src, case.name.span).to_owned());
                out.insert("cases".into(), sorted(cases));
                self.body(&decl.members, site, false, &mut out);
            }
            Item::Alias(ty) => {
                out.insert("kind".into(), Value::from("type"));
                out.insert("type".into(), Value::from(self.ty(ty, site, 0)));
            }
        }
        Value::Object(out)
    }

    /// `constants`, `properties` and `methods`, each sorted by name and each present when empty,
    /// so two documents always have the same keys to compare. A method in an interface body with
    /// no body is abstract, as one marked `abstract` in a class is.
    fn body(
        &self,
        members: &[ClassMember],
        site: &Site<'_>,
        interface: bool,
        out: &mut Map<String, Value>,
    ) {
        let mut constants = Vec::new();
        let mut properties = Vec::new();
        let mut methods = Vec::new();
        for member in members {
            match &member.kind {
                ClassMemberKind::Const(constant) if visible(&constant.modifiers) => {
                    constants.push(self.constant(constant, site));
                }
                ClassMemberKind::Property(property) if visible(&property.modifiers) => {
                    properties.push(self.property(property, site));
                }
                ClassMemberKind::Method(method) => {
                    if text(site.src, method.name) == "constructor" {
                        properties.extend(
                            method
                                .params
                                .iter()
                                .filter(|param| promoted(param) && visible(&param.modifiers))
                                .map(|param| self.promoted(param, site)),
                        );
                    }
                    if visible(&method.modifiers) {
                        methods.push(self.method(method, site, interface));
                    }
                }
                _ => {}
            }
        }
        for (key, mut list) in [
            ("constants", constants),
            ("properties", properties),
            ("methods", methods),
        ] {
            list.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
            out.insert(key.into(), Value::Array(list));
        }
    }

    fn constant(&self, constant: &ConstMember, site: &Site<'_>) -> Value {
        let mut out = Map::new();
        out.insert("name".into(), Value::from(text(site.src, constant.name)));
        out.insert(
            "visibility".into(),
            Value::from(visibility(&constant.modifiers)),
        );
        out.insert(
            "final".into(),
            Value::from(constant.modifiers.contains(&Modifier::Final)),
        );
        if let Some(ty) = &constant.ty {
            out.insert("type".into(), Value::from(self.ty(ty, site, 0)));
        }
        Value::Object(out)
    }

    fn property(&self, property: &PropertyMember, site: &Site<'_>) -> Value {
        self.property_json(
            text(site.src, property.name),
            &property.modifiers,
            Some(&property.ty),
            site,
        )
    }

    fn promoted(&self, param: &Param, site: &Site<'_>) -> Value {
        self.property_json(
            text(site.src, param.name),
            &param.modifiers,
            param.ty.as_ref(),
            site,
        )
    }

    /// A property: its `$name`, who may read it (`visibility`) and who may write it (`set`), and
    /// its type.
    fn property_json(
        &self,
        name: &str,
        modifiers: &[Modifier],
        ty: Option<&Type>,
        site: &Site<'_>,
    ) -> Value {
        let read = visibility(modifiers);
        let write = modifiers
            .iter()
            .find_map(|modifier| match modifier {
                Modifier::SetVisibility(Visibility::Public) => Some("public"),
                Modifier::SetVisibility(Visibility::Protected) => Some("protected"),
                Modifier::SetVisibility(Visibility::Private) => Some("private"),
                _ => None,
            })
            .unwrap_or(read);
        let mut out = Map::new();
        out.insert("name".into(), Value::from(name));
        out.insert("visibility".into(), Value::from(read));
        out.insert("set".into(), Value::from(write));
        out.insert(
            "static".into(),
            Value::from(modifiers.contains(&Modifier::Static)),
        );
        out.insert(
            "readonly".into(),
            Value::from(modifiers.contains(&Modifier::Readonly)),
        );
        if let Some(ty) = ty {
            out.insert("type".into(), Value::from(self.ty(ty, site, 0)));
        }
        Value::Object(out)
    }

    fn method(&self, method: &MethodMember, site: &Site<'_>, interface: bool) -> Value {
        let abstract_ =
            method.modifiers.contains(&Modifier::Abstract) || (interface && method.body.is_none());
        let params: Vec<Value> = method
            .params
            .iter()
            .map(|param| self.param(param, site))
            .collect();
        let mut out = Map::new();
        out.insert("name".into(), Value::from(text(site.src, method.name)));
        out.insert(
            "visibility".into(),
            Value::from(visibility(&method.modifiers)),
        );
        out.insert(
            "static".into(),
            Value::from(method.modifiers.contains(&Modifier::Static)),
        );
        out.insert("abstract".into(), Value::from(abstract_));
        out.insert(
            "final".into(),
            Value::from(method.modifiers.contains(&Modifier::Final)),
        );
        out.insert("params".into(), Value::Array(params));
        if let Some(ty) = &method.return_type {
            out.insert("returns".into(), Value::from(self.ty(ty, site, 0)));
        }
        Value::Object(out)
    }

    /// One parameter. `optional` is whether a caller may leave it out: it has a default, or it is
    /// variadic. Its name is part of the API, because a caller may pass it by name.
    fn param(&self, param: &Param, site: &Site<'_>) -> Value {
        let mut out = Map::new();
        out.insert("name".into(), Value::from(text(site.src, param.name)));
        if let Some(ty) = &param.ty {
            out.insert("type".into(), Value::from(self.ty(ty, site, 0)));
        }
        out.insert(
            "optional".into(),
            Value::from(param.default.is_some() || param.variadic),
        );
        out.insert("variadic".into(), Value::from(param.variadic));
        out.insert("inout".into(), Value::from(param.inout));
        Value::Object(out)
    }

    /// `ty`, resolved and normalised: its union members sorted and joined by `|`.
    fn ty(&self, ty: &Type, site: &Site<'_>, depth: u32) -> String {
        let mut members = BTreeSet::new();
        self.union(ty, site, depth, &mut members);
        members.into_iter().collect::<Vec<_>>().join("|")
    }

    /// Adds `ty`'s union members to `out`: `?T` is `T` and `null`, a nested union is flattened,
    /// and an alias is replaced by its members.
    fn union(&self, ty: &Type, site: &Site<'_>, depth: u32, out: &mut BTreeSet<String>) {
        if depth > MAX_DEPTH {
            out.insert(text(site.src, ty.span).to_owned());
            return;
        }
        match &ty.kind {
            TypeKind::Nullable(inner) => {
                self.union(inner, site, depth + 1, out);
                out.insert("null".to_owned());
            }
            TypeKind::Union(parts) => {
                for part in parts {
                    self.union(part, site, depth + 1, out);
                }
            }
            TypeKind::Paren(inner) => self.union(inner, site, depth + 1, out),
            TypeKind::Intersection(parts) => {
                let parts: BTreeSet<String> = parts
                    .iter()
                    .map(|part| {
                        let mut members = BTreeSet::new();
                        self.union(part, site, depth + 1, &mut members);
                        let joined = members.iter().cloned().collect::<Vec<_>>().join("|");
                        if members.len() > 1 {
                            format!("({joined})")
                        } else {
                            joined
                        }
                    })
                    .collect();
                out.insert(parts.into_iter().collect::<Vec<_>>().join("&"));
            }
            TypeKind::Atom(atom) => self.atom(atom, ty.span, site, depth, out),
            _ => {
                out.insert(text(site.src, ty.span).to_owned());
            }
        }
    }

    fn atom(
        &self,
        atom: &TypeAtom,
        span: Span,
        site: &Site<'_>,
        depth: u32,
        out: &mut BTreeSet<String>,
    ) {
        let inner = |ty: &Type| self.ty(ty, site, depth + 1);
        let written = match atom {
            TypeAtom::Null => "null".to_owned(),
            TypeAtom::Bool => "bool".to_owned(),
            TypeAtom::Int => "int".to_owned(),
            TypeAtom::Uint => "uint".to_owned(),
            TypeAtom::Float => "float".to_owned(),
            TypeAtom::Decimal => "decimal".to_owned(),
            TypeAtom::String => "string".to_owned(),
            TypeAtom::Bytes => "bytes".to_owned(),
            TypeAtom::TaintedString => "tainted string".to_owned(),
            TypeAtom::TaintedBytes => "tainted bytes".to_owned(),
            TypeAtom::SecretString => "secret string".to_owned(),
            TypeAtom::SecretBytes => "secret bytes".to_owned(),
            TypeAtom::SecretTaintedString => "secret tainted string".to_owned(),
            TypeAtom::SecretTaintedBytes => "secret tainted bytes".to_owned(),
            TypeAtom::Array(None) => "array".to_owned(),
            TypeAtom::Array(Some(arg)) => format!("array<{}>", inner(arg)),
            TypeAtom::ClassRef(arg) => format!("class<{}>", inner(arg)),
            TypeAtom::PropertyKey(arg) => format!("property<{}>", inner(arg)),
            TypeAtom::PropertyPath(arg) => format!("propertyPath<{}>", inner(arg)),
            TypeAtom::StringHoles { key, tainted } => {
                let prefix = if *tainted { "tainted " } else { "" };
                format!("{prefix}string<{}>", inner(key))
            }
            TypeAtom::Object => "object".to_owned(),
            TypeAtom::Shape(fields) => {
                let fields: BTreeSet<String> = fields
                    .iter()
                    .map(|field| {
                        let mark = if field.required { "" } else { "?" };
                        format!("{}{mark}: {}", text(site.src, field.name), inner(&field.ty))
                    })
                    .collect();
                format!("{{{}}}", fields.into_iter().collect::<Vec<_>>().join(", "))
            }
            TypeAtom::Mixed => "mixed".to_owned(),
            TypeAtom::Void => "void".to_owned(),
            TypeAtom::Never => "never".to_owned(),
            TypeAtom::True => "true".to_owned(),
            TypeAtom::False => "false".to_owned(),
            TypeAtom::SingleValueString(literal) | TypeAtom::SingleValueInt(literal) => {
                text(site.src, *literal).to_owned()
            }
            TypeAtom::Member(name, member) => {
                format!(
                    "{}::{}",
                    self.class_name(*name, site),
                    text(site.src, *member)
                )
            }
            TypeAtom::MemberCase(name, owned, case) => format!(
                "{}::{}::{}",
                self.class_name(*name, site),
                text(site.src, *owned),
                text(site.src, *case)
            ),
            TypeAtom::Iterable => "iterable".to_owned(),
            TypeAtom::Callable => "callable".to_owned(),
            TypeAtom::CallableSig { params, ret } => {
                let params: Vec<String> = params.iter().map(inner).collect();
                format!("callable({}): {}", params.join(", "), inner(ret))
            }
            TypeAtom::SelfTy => site.owner.clone().unwrap_or_else(|| "self".to_owned()),
            TypeAtom::StaticTy => "static".to_owned(),
            TypeAtom::Parent => site.parent.clone().unwrap_or_else(|| "parent".to_owned()),
            TypeAtom::Name(name, args) => {
                if args.is_empty()
                    && let Some(alias) = self.alias(*name, site)
                    && let Item::Alias(ty) = alias.item
                {
                    self.union(ty, &alias.site, depth + 1, out);
                    return;
                }
                let resolved = self.class_name(*name, site);
                if args.is_empty() {
                    resolved
                } else {
                    let args: Vec<String> = args.iter().map(inner).collect();
                    format!("{resolved}<{}>", args.join(", "))
                }
            }
            _ => text(site.src, span).to_owned(),
        };
        out.insert(written);
    }

    /// The alias `name` means at `site`, if it means one: the owner's own first, then the one its
    /// resolved name declares.
    fn alias(&self, name: Name, site: &Site<'_>) -> Option<&Decl<'a>> {
        let is_alias = |decl: &&Decl<'a>| matches!(decl.item, Item::Alias(_));
        let written = text(site.src, name.span);
        if !written.contains('\\')
            && let Some(owner) = &site.owner
            && let Some(decl) = self.lookup(&format!("{owner}::{written}")).filter(is_alias)
        {
            return Some(decl);
        }
        self.lookup(&site.resolve(name)).filter(is_alias)
    }

    /// The class, interface or enum `name` means at `site`: inside a body, a short name is the
    /// body's own enum before it is the namespace's (`rule:enums/class-scoped-enum`).
    fn class_name(&self, name: Name, site: &Site<'_>) -> String {
        let written = text(site.src, name.span);
        if !written.contains('\\')
            && let Some(owner) = &site.owner
        {
            let owned = format!("{owner}::{written}");
            if self
                .lookup(&owned)
                .is_some_and(|decl| matches!(decl.item, Item::Enum(_)))
            {
                return owned;
            }
        }
        site.resolve(name)
    }
}

/// Whether a member with these modifiers is part of the API: public or protected.
fn visible(modifiers: &[Modifier]) -> bool {
    !modifiers.contains(&Modifier::Private)
}

/// Whether a constructor parameter declares a property.
fn promoted(param: &Param) -> bool {
    param.modifiers.iter().any(|modifier| {
        matches!(
            modifier,
            Modifier::Public | Modifier::Protected | Modifier::Private | Modifier::Readonly
        )
    })
}

/// A member's visibility, `public` where none was written.
fn visibility(modifiers: &[Modifier]) -> &'static str {
    if modifiers.contains(&Modifier::Private) {
        "private"
    } else if modifiers.contains(&Modifier::Protected) {
        "protected"
    } else {
        "public"
    }
}

/// `names`, sorted, as a JSON array.
fn sorted(names: impl Iterator<Item = String>) -> Value {
    names
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(Value::from)
        .collect()
}

fn segments(namespace: &str) -> Vec<String> {
    if namespace.is_empty() {
        Vec::new()
    } else {
        QName::parse(namespace).segments().to_vec()
    }
}

fn qualify(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{namespace}\\{name}")
    }
}

fn text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}
