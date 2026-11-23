//! The attribute declarations an editor offers and describes: a program's
//! `attribute` declarations and `Core`'s compiler attributes, read as one value
//! (`rule:attributes/an-attribute-is-declared-with-its-sites`).
//!
//! The checker holds a program's declaration as `nvs_hir::AttributeDef` and a
//! `Core` one as a row of `nvs_types::derive::ATTRIBUTE_DOCS`. Both carry the
//! same facts: a name, the sites its `on` list names, `repeatable`, an owner
//! for a declaration in a class or interface body, and fields with their docs.
//! [`Declared`] is that pair under one type, so completion at `#[`, the keys of
//! a payload, hover and a completion item's card read one answer and never
//! two.
//!
//! A field's name is written in two places: in the declaration's body
//! ([`declared_fields`]) and as a key of a payload ([`written_keys`]). The
//! index keeps both under one symbol, `Name::field` ([`FieldName::symbol`]),
//! and a cursor on either is read by [`field_at`].
//!
//! The `///` runs are read off the declaration's own syntax tree, which the
//! analysis already holds for every loaded file, so nothing is parsed again.
//! What this module costs is one walk over a declaring file's statements per
//! declaration asked about, in the editor only.

use nvs_diagnostics::{BytePos, SourceFile, SourceId, Span};
use nvs_hir::{AttributeDef, QName};
use nvs_syntax::ast::{AttributeDecl, AttributeSite, ClassMemberKind, DocComment, Stmt, StmtKind};
use nvs_syntax::walk;
use nvs_types::derive::{self, AttributeDoc};

use crate::document::Analysed;

/// One attribute declaration, a program's or `Core`'s.
pub(crate) enum Declared<'a> {
    /// A program's `attribute` declaration, with its syntax node and the
    /// `///` run above it when the declaring file is loaded.
    Program {
        /// What the checker holds.
        def: &'a AttributeDef,
        /// The declaration as written, for its field docs.
        decl: Option<&'a AttributeDecl>,
        /// The `///` run above the declaration.
        doc: Option<&'a DocComment>,
        /// The declaring file.
        file: &'a SourceFile,
    },
    /// One of `Core`'s compiler attributes.
    Core(&'static AttributeDoc),
}

/// One field of a declaration: a key a payload writes.
pub(crate) struct Key {
    /// The key's name.
    pub name: String,
    /// Its type, as a program writes it.
    pub ty: String,
    /// Whether a payload must write it.
    pub required: bool,
    /// The key's doc as Markdown, when it has one.
    pub doc: Option<String>,
}

impl<'a> Declared<'a> {
    /// The declaration's full name: `Shop\Storable::Index`, `Core\Route`.
    pub(crate) fn name(&self) -> String {
        match self {
            Self::Program { def, .. } => def.name.to_string(),
            Self::Core(doc) => doc.name.to_owned(),
        }
    }

    /// The declared name alone: `Index`, `Route`.
    pub(crate) fn short_name(&self) -> &str {
        match self {
            Self::Program { def, .. } => def.short_name(),
            Self::Core(doc) => doc.short_name(),
        }
    }

    /// The sites its `on` list names, in the order they are written.
    pub(crate) fn sites(&self) -> Vec<AttributeSite> {
        match self {
            Self::Program { def, .. } => def.sites.iter().map(|word| word.site).collect(),
            Self::Core(doc) => doc.sites.to_vec(),
        }
    }

    /// Whether one declaration may carry it more than once.
    pub(crate) fn repeatable(&self) -> bool {
        match self {
            Self::Program { def, .. } => def.repeatable,
            Self::Core(doc) => doc.repeatable,
        }
    }

    /// The class or interface whose body declares it.
    pub(crate) fn owner(&self) -> Option<&QName> {
        match self {
            Self::Program { def, .. } => def.owner.as_ref(),
            Self::Core(_) => None,
        }
    }

    /// Whether it may be used on `site`. A promoted constructor parameter is a
    /// property and a parameter, so the caller asks for both there.
    pub(crate) fn fits(&self, site: AttributeSite) -> bool {
        self.sites().contains(&site)
    }

    /// The declaration's doc as Markdown, when it has one.
    pub(crate) fn doc(&self) -> Option<String> {
        match self {
            Self::Program { doc, file, .. } => {
                let text = crate::hover::markdown(file.text(), doc.as_ref()?);
                (!text.trim().is_empty()).then_some(text)
            }
            Self::Core(doc) => Some(doc.short.to_owned()),
        }
    }

    /// The keys a payload writes, in declaration order.
    pub(crate) fn keys(&self) -> Vec<Key> {
        match self {
            Self::Program { decl, file, .. } => decl.map_or_else(Vec::new, |decl| {
                decl.fields
                    .iter()
                    .map(|field| Key {
                        name: slice(file, field.name).to_owned(),
                        ty: slice(file, field.ty.span).to_owned(),
                        required: field.required,
                        doc: field
                            .doc
                            .as_ref()
                            .map(|doc| crate::hover::markdown(file.text(), doc))
                            .filter(|text| !text.trim().is_empty()),
                    })
                    .collect()
            }),
            Self::Core(doc) => doc
                .fields
                .iter()
                .map(|field| Key {
                    name: field.name.to_owned(),
                    ty: field.ty.to_owned(),
                    required: field.required,
                    doc: (!field.doc.is_empty()).then(|| field.doc.to_owned()),
                })
                .collect(),
        }
    }

    /// The `on` list as a program writes it, with `repeatable` in front where
    /// it is written: `repeatable, on method`.
    pub(crate) fn header(&self) -> String {
        let sites: Vec<&str> = self.sites().iter().map(|site| site.word()).collect();
        let on = format!("on {}", sites.join(", "));
        if self.repeatable() {
            format!("repeatable, {on}")
        } else {
            on
        }
    }

    /// The hover and completion card: the declaration as a code block, its
    /// doc, and for an owned declaration the classes it is used on.
    pub(crate) fn card(&self) -> String {
        let repeatable = if self.repeatable() { "repeatable " } else { "" };
        let sites: Vec<&str> = self.sites().iter().map(|site| site.word()).collect();
        let keys = self.keys();
        let body = if keys.is_empty() {
            "{}".to_owned()
        } else {
            let lines: Vec<String> = keys
                .iter()
                .map(|key| {
                    let optional = if key.required { "" } else { "?" };
                    format!("    {}{optional}: {},", key.name, key.ty)
                })
                .collect();
            format!("{{\n{}\n}}", lines.join("\n"))
        };
        let mut out = format!(
            "```nvs\n{repeatable}attribute {} on {} {body}\n```",
            self.name(),
            sites.join(", ")
        );
        if let Some(doc) = self.doc() {
            out.push_str("\n\n");
            out.push_str(doc.trim_end());
        }
        if let Some(owner) = self.owner() {
            out.push_str(&format!(
                "\n\nDeclared in `{owner}`. You can use it only on classes that extend or \
                 implement `{owner}`, and on their members."
            ));
        }
        out
    }
}

/// Every attribute declaration in reach: the program's, then `Core`'s.
pub(crate) fn every(analysed: &Analysed) -> Vec<Declared<'_>> {
    let program = analysed
        .module
        .aliases
        .attributes()
        .map(|def| program(analysed, def));
    let core = derive::ATTRIBUTE_DOCS.iter().map(Declared::Core);
    program.chain(core).collect()
}

/// The declaration `name` names, a program's or `Core`'s.
pub(crate) fn named<'a>(analysed: &'a Analysed, name: &QName) -> Option<Declared<'a>> {
    if let Some(def) = analysed.module.aliases.attribute(name) {
        return Some(program(analysed, def));
    }
    derive::attribute_doc(&name.to_string()).map(Declared::Core)
}

/// The declaration the attribute whose node is at `attribute` names, read off
/// the name the checker resolved inside it.
pub(crate) fn of_attribute(analysed: &Analysed, attribute: Span) -> Option<Declared<'_>> {
    let (_, name) = analysed.exprs.attribute_names().find(|(span, _)| {
        span.file == attribute.file && attribute.start <= span.start && span.end <= attribute.end
    })?;
    named(analysed, name)
}

/// A program's declaration with its syntax node and its `///` run.
fn program<'a>(analysed: &'a Analysed, def: &'a AttributeDef) -> Declared<'a> {
    let file = analysed.map.file(def.name_span.file);
    let found = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == def.name_span.file)
        .and_then(|loaded| find(&loaded.stmts, def.name_span));
    let (decl, doc) = match found {
        Some((decl, doc)) => (Some(decl), doc),
        None => (None, None),
    };
    Declared::Program {
        def,
        decl,
        doc,
        file,
    }
}

/// The declaration whose name is written at `name` among `stmts`, with the
/// `///` run above it, which a body member keeps on its member.
fn find(stmts: &[Stmt], name: Span) -> Option<(&AttributeDecl, Option<&DocComment>)> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::AttributeDecl(decl) if decl.name.span == name => Some((decl, decl.doc.as_ref())),
        StmtKind::NamespaceDecl(namespace) => find(&namespace.body.as_ref()?.stmts, name),
        StmtKind::ClassDecl(class) => member(&class.members, name),
        StmtKind::InterfaceDecl(interface) => member(&interface.members, name),
        _ => None,
    })
}

/// The attribute declaration among a body's `members` whose name is at `name`.
fn member(
    members: &[nvs_syntax::ast::ClassMember],
    name: Span,
) -> Option<(&AttributeDecl, Option<&DocComment>)> {
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Attribute(decl) if decl.name.span == name => {
            Some((decl, member.doc.as_ref().or(decl.doc.as_ref())))
        }
        _ => None,
    })
}

/// One field name written in a file: a field of a program's declaration, or a
/// key of a payload that names a declared field.
pub(crate) struct FieldName<'a> {
    /// Where the field's name is written.
    pub span: Span,
    /// The declaration the field belongs to.
    pub attribute: &'a QName,
    /// The field's name.
    pub name: &'a str,
}

impl FieldName<'_> {
    /// The symbol the index keeps the field under: `Shop\Job::queue`, and
    /// `Shop\Storable::Index::columns` for a declaration a body owns.
    pub(crate) fn symbol(&self) -> String {
        format!("{}::{}", self.attribute, self.name)
    }
}

/// Every field of every program declaration `file` writes, in source order.
pub(crate) fn declared_fields(analysed: &Analysed, file: SourceId) -> Vec<FieldName<'_>> {
    let mut found: Vec<FieldName<'_>> = analysed
        .module
        .aliases
        .attributes()
        .filter(|def| def.name_span.file == file)
        .filter_map(|def| match program(analysed, def) {
            Declared::Program {
                decl: Some(decl),
                file: source,
                ..
            } => Some(decl.fields.iter().map(move |field| FieldName {
                span: field.name,
                attribute: &def.name,
                name: slice(source, field.name),
            })),
            _ => None,
        })
        .flatten()
        .filter(|field| !field.name.is_empty())
        .collect();
    found.sort_by_key(|field| field.span.start);
    found
}

/// Every key the payloads in `stmts`, parsed from `file`, write for a field
/// their declaration has, in source order. A `Core` attribute's keys are among
/// them.
///
/// A key is no node, so its name is read off the text in front of its value:
/// the name directly before the `:` the value follows.
pub(crate) fn written_keys<'a>(
    analysed: &'a Analysed,
    file: SourceId,
    stmts: &[Stmt],
) -> Vec<FieldName<'a>> {
    let source = analysed.map.file(file);
    let mut found = Vec::new();
    for root in walk::of_stmts(stmts) {
        for node in std::iter::once(&root).chain(root.descendants()) {
            if node.kind != "Attribute" || node.children.is_empty() {
                continue;
            }
            let Some((_, attribute)) = analysed.exprs.attribute_names().find(|(span, _)| {
                span.file == file && node.span.start <= span.start && span.end <= node.span.end
            }) else {
                continue;
            };
            let Some(declared) = named(analysed, attribute) else {
                continue;
            };
            let keys = declared.keys();
            for value in &node.children {
                let Some(span) = key_before(source, node.span.start, value.span.start) else {
                    continue;
                };
                let name = slice(source, span);
                if keys.iter().any(|key| key.name == name) {
                    found.push(FieldName {
                        span,
                        attribute,
                        name,
                    });
                }
            }
        }
    }
    found
}

/// The name written directly before the `:` in front of a value that starts
/// at `value`, no earlier than `from`.
fn key_before(source: &SourceFile, from: BytePos, value: BytePos) -> Option<Span> {
    let text = source.text().get(from as usize..value as usize)?;
    let before = text.trim_end().strip_suffix(':')?;
    if before.ends_with(':') {
        return None;
    }
    let before = before.trim_end();
    let name = before.trim_end_matches(|ch: char| ch.is_ascii_alphanumeric() || ch == '_');
    if name.len() == before.len() {
        return None;
    }
    let start = from + BytePos::try_from(name.len()).ok()?;
    let end = from + BytePos::try_from(before.len()).ok()?;
    Some(Span::new(source.id(), start, end))
}

/// The field of a program's declaration whose name is written at `offset` in
/// `file`, or the key of a payload written there.
pub(crate) fn field_at(analysed: &Analysed, offset: BytePos) -> Option<FieldName<'_>> {
    let file = analysed.entry;
    let covers = |span: Span| span.start <= offset && offset <= span.end;
    if let Some(field) = declared_fields(analysed, file)
        .into_iter()
        .find(|field| covers(field.span))
    {
        return Some(field);
    }
    let stmts = &analysed.loaded.iter().find(|loaded| loaded.id == file)?.stmts;
    written_keys(analysed, file, stmts)
        .into_iter()
        .find(|key| covers(key.span))
}

impl Declared<'_> {
    /// Where the declaration writes the field `name`, for a program's
    /// declaration whose file is loaded.
    pub(crate) fn field_span(&self, name: &str) -> Option<Span> {
        let Self::Program {
            decl: Some(decl),
            file,
            ..
        } = self
        else {
            return None;
        };
        decl.fields
            .iter()
            .find(|field| slice(file, field.name) == name)
            .map(|field| field.name)
    }
}

/// The text `span` covers in `file`.
fn slice(file: &SourceFile, span: Span) -> &str {
    file.text().get(span.range()).unwrap_or_default()
}
