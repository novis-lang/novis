//! Every declared class's **field slot order** and its flattened set of
//! supertypes — the second thing this crate publishes for `nvs-ir` to read
//! back, alongside [`crate::expr_table`].
//!
//! # Why it lives here and not in `nvs-ir`
//!
//! Exactly [`crate::expr_table`]'s reasoning, and no new one: a slot order has
//! to put an ancestor's properties before a subclass's own
//! (`nvs_runtime::object`'s layout rule), which needs the resolved
//! [`ClassGraph`] — and `nvs-ir` deliberately does not depend on `nvs-hir` at
//! all. So the *resolution* happens once, here, and `nvs-ir` copies the answer
//! into `nvs_ir::ir::Class` rather than re-deriving it.
//!
//! # What it is not
//!
//! Not a byte layout. This table says *which slot index* a field occupies and
//! nothing about how wide one is, because `nvs_runtime::object` makes every
//! slot the same width — see that module's own docs for the decision and its
//! cost. `nvs-codegen` turns a slot index into an offset through
//! `nvs_runtime::field_offset`, so the arithmetic exists in exactly one place.
//!
//! # Declaration order is the slot order
//!
//! A class's own properties occupy slots in the order they are written, after
//! every slot its ancestors already claimed. That makes a recompile of an
//! unchanged file reproduce the same layout — the same stability property
//! `nvs_ir::ids` needs for a probe id, for the same reason: an artifact cached
//! under `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` must still describe the code it is paired with.
//!
//! A `static` property claims no slot: `rule:statements/static-is-a-member-modifier` makes it class storage, not
//! instance storage. A *hooked* property (`rule:classes/property-hooks`) does claim one, even
//! when nothing ever reads it — Novis has no virtual/backed split, and
//! [`crate::signatures::PropertyHooks`] owns that decision and what it
//! spends.
//!
//! Two kinds of type have no entry at all, by decision rather than by
//! omission. An **enum** has none: `rule:enums/closed-integer-type` makes it a closed integer
//! value type, not an instance with fields. A **`Core` class** has none
//! either — it has no source declaration for this walk to read, and
//! `nvs-stdlib` owns those as native Rust rather than as field-slot objects.
//! So the table holds exactly the classes declared in the files walked, and a
//! lookup that misses is a class from one of those two families rather than a
//! class this pass failed to lay out.

use nvs_diagnostics::SourceFile;
use nvs_hir::{ClassGraph, QName};
use nvs_syntax::ast::{ClassMemberKind, Modifier, NamespaceDecl, PropertyMember, Stmt, StmtKind};
use rustc_hash::FxHashMap;

use crate::span_text;

/// A namespace name's segments — the same one-liner [`crate::check`] and
/// [`crate::signatures`] each keep for themselves, kept here too rather than
/// exported: it is two lines, and hoisting it would only move the duplication.
fn qname_segments(src: &SourceFile, name: &nvs_syntax::ast::Name) -> Vec<String> {
    QName::parse(span_text(src, name.span)).segments().to_vec()
}

/// One class's or interface's instance-field slot order and supertype set.
#[derive(Clone, Debug, Default)]
pub struct ClassLayout {
    /// Every field slot in index order — every ancestor's first, then this
    /// class's own in declaration order. `$`-sigil not included.
    pub fields: Vec<String>,
    /// Whether each field slot is readable from outside its class, in
    /// [`Self::fields`]' own order — the property half of what
    /// [`Self::methods`]' third element says about a method.
    ///
    /// Carried for [`Self::methods`]' reason exactly: visibility is a keyword
    /// on a declaration and nothing below the front end can see one, while
    /// `rule:security/reflection-enforces-visibility`
    /// 's rule — a reflective read faces the check ordinary code at that
    /// site faces — has to be answered at run time, of a value whose class the
    /// checker never saw. `nvs_ir::ir::Class::public_fields` carries it down and
    /// `nvs_runtime::ClassDesc::field_is_public` is what
    /// `nvs_stdlib::reflect`'s walk asks.
    ///
    /// **Cost:** one `bool` per field slot per class, once per compiled unit.
    pub public_fields: Vec<bool>,
    /// Each field slot's declared type as the declaration spells it, in
    /// [`Self::fields`]' own order, and the **empty string** for a slot no
    /// declaration laid out — the exception tree's, whose types live as
    /// `TypeId`s in [`crate::error_lib`] and never as text.
    ///
    /// A name rather than a [`crate::ty::TypeId`], because the reader is
    /// `Core\Reflect\PropertyInfo` and a descriptor is what it reads from: the
    /// interner that resolves an id is gone by then, and `nvs_runtime::Tag` —
    /// the one type fact the runtime already carries per slot — cannot tell
    /// `?int` from `int`, `array<string>` from `array<User>`, or one class from
    /// another. Carried for [`Self::public_fields`]' reason exactly, one step
    /// further: the spelling exists only where the declaration does.
    ///
    /// **Cost:** one `String` per field slot per class, once per compiled unit,
    /// never per request.
    pub field_types: Vec<String>,
    /// Every *other* class and interface an instance of this one also is,
    /// rendered the same way [`ClassLayout`]'s own key is. Transitive, and
    /// deliberately excluding the class itself — `instanceof` checks identity
    /// separately (`nvs_runtime::ClassDesc::conforms_to`).
    pub conforms: Vec<String>,
    /// Every method callable on an instance of this class, as `(method name,
    /// declaring class label, is `public`)` — its own first, then the nearest
    /// ancestor declaring each name it does not. Only methods with a *body*:
    /// an abstract or bodiless interface method has no code to name.
    ///
    /// This is what `nvs_runtime::ClassDesc::method` answers a
    /// `static::method(...)` dispatch from, so the precedence has to be the
    /// language's: a class's own override, then its superclass chain, then an
    /// interface default (`rule:classes/interface-default-methods`). A depth-first walk that takes
    /// `extends` before `implements` produces exactly that order.
    ///
    /// The visibility bit is carried because it has no source below the front
    /// end and one dispatch needs it: a call whose receiver names no class is
    /// outside every class by construction, so `nvs_runtime::MethodRow` — the
    /// far end of this table — answers a non-`public` member with a throw
    /// rather than with the address. Every other visibility question is
    /// answered where the call is written, against the signature table.
    pub methods: Vec<(String, String, bool)>,
    /// Every property hook an instance of this class answers, as `(property
    /// name, hook label, is the `set` accessor)` — its own first, then the
    /// nearest ancestor declaring each `(property, accessor)` pair it does
    /// not, on [`Self::methods`]' precedence exactly. Only hooks with a
    /// *body*: an abstract one is a requirement, not code.
    ///
    /// Beside the method roster rather than in it, because
    /// `rule:classes/property-hooks` makes a hook an accessor of a property
    /// and not a method: a `Class::name()` call must never reach one, and
    /// `Core\Reflect\ClassInfo::methods` reads [`Self::methods`].
    ///
    /// The label is `crate::signatures::hook_label`'s, which is the name the
    /// hook's compiled function is emitted under — the join
    /// `nvs-codegen` needs to put an address on the row, and the reason the
    /// declaring class is not carried separately as it is for a method.
    /// The accessor bit is carried rather than read back off the label's
    /// tail, so that spelling stays this crate's alone.
    pub hooks: Vec<(String, String, bool)>,
}

impl ClassLayout {
    /// The slot `field` occupies, if this class has one for it.
    #[must_use]
    pub fn slot_of(&self, field: &str) -> Option<usize> {
        self.fields.iter().position(|name| name == field)
    }
}

/// Every declared class's [`ClassLayout`], keyed by its rendered `Class`/
/// `Ns\Class` label — the same rendering
/// [`crate::expr_table::ExprTypeTable::method_label`] uses for a method
/// label's class half, so `nvs-ir` can match a `New`/`FieldGet` target against
/// it without re-spelling a namespace.
#[derive(Debug, Default)]
pub struct ClassLayoutTable {
    by_label: FxHashMap<String, ClassLayout>,
}

impl ClassLayoutTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One class's layout, if it was declared in the files this table was
    /// built from.
    #[must_use]
    pub fn get(&self, label: &str) -> Option<&ClassLayout> {
        self.by_label.get(label)
    }

    /// Every class, label and layout, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ClassLayout)> {
        self.by_label
            .iter()
            .map(|(label, layout)| (label.as_str(), layout))
    }

    /// How many classes have a layout.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_label.len()
    }

    /// Whether nothing was collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_label.is_empty()
    }
}

/// Builds the layout of every class and interface declared in any file of
/// `files`, plus the two rosters no source declares — `nvs_hir::errors`'
/// exception tree and `nvs_hir::interfaces`' global interfaces.
///
/// `graph` is the already-resolved hierarchy `nvs_hir::resolve_file` or
/// `nvs_hir::resolve_program` produced for the same set of files — the one
/// thing this pass cannot derive from the AST, since an `extends Foo`
/// reference has to be resolved against the active namespace and imports.
/// The set is walked whole before any layout is assembled: a subclass in one
/// file inherits the slots of a base declared in another, and a per-file
/// table would place its own properties at slot zero.
///
/// A class whose ancestor chain is broken (an unresolved or cyclic `extends`,
/// both of which `nvs_hir::hierarchy` has already diagnosed) simply gets a
/// layout built from as much of the chain as resolves, rather than being
/// omitted: this pass never reports a diagnostic of its own, so leaving a hole
/// would turn an already-reported error into a second, unexplained failure
/// further down the pipeline.
#[must_use]
pub fn build_class_layouts(
    files: &[crate::ProgramFile<'_>],
    graph: &ClassGraph,
) -> ClassLayoutTable {
    let mut own: FxHashMap<QName, Vec<(String, bool, String)>> = FxHashMap::default();
    let mut own_methods: FxHashMap<QName, Vec<(String, bool)>> = FxHashMap::default();
    let mut own_hooks: FxHashMap<QName, Vec<(String, String, bool)>> = FxHashMap::default();
    // The exception tree first: it has no source declaration to collect from
    // (`nvs_hir::errors`), and a user class extending it needs its four slots
    // already claimed before its own are appended.
    for (name, _) in nvs_hir::errors::TREE {
        // Public on `own_methods`' terms, and for its reason: spec § 10's four
        // properties are read from inside every `catch` block, and a
        // synthesized declaration writes no modifier to read. The declared
        // type is `ClassLayout::field_types`' empty case for the reason that
        // field's doc gives: this roster is names, and `crate::error_lib` types
        // it in a currency no text of these slots exists in.
        let fields = nvs_hir::errors::own_properties(name)
            .iter()
            .map(|p| ((*p).to_owned(), true, String::new()))
            .collect();
        // These constructors are synthesized rather than written
        // (`nvs_ir::lower::exception`), so they are the methods with a body
        // that no source walk can find. One per class that declares
        // properties of its own — `nvs_hir::errors::declares_constructor`.
        let methods = if nvs_hir::errors::declares_constructor(name) {
            // Public: spec § 10's tree is constructed by every program that
            // throws, and a synthesized member writes no modifier — see
            // `own_methods` on why absent reads as `public`.
            vec![("constructor".to_owned(), true)]
        } else {
            Vec::new()
        };
        own.insert(QName::parse(name), fields);
        own_methods.insert(QName::parse(name), methods);
    }
    // The compiler-declared global interfaces, for the same reason and on the
    // same terms (`nvs_hir::interfaces`): nothing declares `Stringable` in
    // source, but `$x instanceof Stringable` needs a descriptor to point at
    // and `class S implements Stringable` needs the edge to it in `conforms`,
    // which `collect_conforms` only keeps for a label the table has an entry
    // for. Both lists are empty: an interface declares no property, and no
    // member on the roster has code for a descriptor's method table to name —
    // every one but `Parses::tryParse` is bodiless (`crate::iter_lib`), and
    // that default is a signature the checker resolves against rather than a
    // compiled function, so a class inheriting it inherits nothing to enter
    // here.
    for (name, _) in nvs_hir::interfaces::RESERVED {
        own.insert(QName::parse(name), Vec::new());
        own_methods.insert(QName::parse(name), Vec::new());
    }
    for file in files {
        collect_own(
            file.stmts,
            file.src,
            &[],
            &mut own,
            &mut own_methods,
            &mut own_hooks,
        );
    }

    let mut table = ClassLayoutTable::default();
    for qname in own.keys() {
        let mut slots = Vec::new();
        let mut seen = Vec::new();
        flatten_fields(qname, graph, &own, &mut slots, &mut seen);
        let mut fields = Vec::with_capacity(slots.len());
        let mut public_fields = Vec::with_capacity(slots.len());
        let mut field_types = Vec::with_capacity(slots.len());
        for (name, public, ty) in slots {
            fields.push(name);
            public_fields.push(public);
            field_types.push(ty);
        }

        let mut conforms = Vec::new();
        let mut visited = vec![qname.clone()];
        collect_conforms(qname, graph, &mut conforms, &mut visited);

        let mut methods = Vec::new();
        let mut walked = Vec::new();
        flatten_methods(qname, graph, &own_methods, &mut methods, &mut walked);

        let mut hooks = Vec::new();
        let mut hook_walked = Vec::new();
        flatten_hooks(qname, graph, &own_hooks, &mut hooks, &mut hook_walked);

        table.by_label.insert(
            qname.to_string(),
            ClassLayout {
                fields,
                public_fields,
                field_types,
                conforms: conforms.iter().map(QName::to_string).collect(),
                methods,
                hooks,
            },
        );
    }
    table
}

/// Records each declared class's/interface's *own* instance properties, in
/// declaration order.
fn collect_own(
    stmts: &[Stmt],
    src: &SourceFile,
    namespace: &[String],
    out: &mut FxHashMap<QName, Vec<(String, bool, String)>>,
    methods: &mut FxHashMap<QName, Vec<(String, bool)>>,
    hooks: &mut FxHashMap<QName, Vec<(String, String, bool)>>,
) {
    let mut current = namespace.to_vec();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let scoped = name
                    .as_ref()
                    .map_or_else(Vec::new, |n| qname_segments(src, n));
                match body {
                    Some(block) => collect_own(&block.stmts, src, &scoped, out, methods, hooks),
                    None => current = scoped,
                }
            }
            StmtKind::ClassDecl(decl) => {
                let qname = QName::join(&current, span_text(src, decl.name.span));
                out.insert(qname.clone(), own_properties(&decl.members, src));
                methods.insert(qname.clone(), own_methods(&decl.members, src));
                hooks.insert(qname.clone(), own_hooks(&qname, &decl.members, src));
            }
            // An interface declares no instance property (`rule:classes/interface-default-methods` gives
            // it method bodies, not state), but it still needs an entry: it is
            // a legal `instanceof` target and a legal `catch` type, so
            // `nvs-codegen` must have a descriptor to point at. Its *default*
            // method bodies are real code, though, so they are collected the
            // same way a class's are.
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current, span_text(src, decl.name.span));
                out.insert(qname.clone(), Vec::new());
                methods.insert(qname.clone(), own_methods(&decl.members, src));
                // An interface declares no *hooked* property either, its
                // properties being none at all; the entry is still made, on
                // the terms above.
                hooks.insert(qname.clone(), own_hooks(&qname, &decl.members, src));
            }
            _ => {}
        }
    }
}

/// One declaration's own method names, each with whether it is `public` — only
/// those with a body, since a bodiless one has no compiled code for a
/// descriptor to point at.
///
/// A declaration that wrote no visibility keyword at all counts as `public`,
/// which is `nvs_syntax::check_declarations`' `E_MISSING_VISIBILITY` already
/// being reported for it: this pass only has to not invent a level for source
/// that is being refused anyway, and the same reading is what keeps a
/// synthesized method — an exception constructor, `rule:iteration/generators`'s state machine
/// — callable, none of them writing a modifier.
fn own_methods(members: &[nvs_syntax::ast::ClassMember], src: &SourceFile) -> Vec<(String, bool)> {
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Method(m) if m.body.is_some() => {
                Some((span_text(src, m.name).to_owned(), is_public(&m.modifiers)))
            }
            _ => None,
        })
        .collect()
}

/// One declaration's own property hooks, as `(property name, hook label, is
/// the `set` accessor)` — only those with a body, on [`own_methods`]' terms
/// exactly, since an abstract hook states a requirement and compiles to
/// nothing.
///
/// The label is spelled here with [`crate::signatures::hook_label`] rather
/// than assembled downstream, which is that function's whole purpose: the
/// hook's *definition* and every name for it agree by construction. `class` is
/// the declaration's own qualified name, so an inherited hook still carries
/// the class whose body it is — the join `nvs-codegen` makes to find its
/// compiled address.
fn own_hooks(
    class: &QName,
    members: &[nvs_syntax::ast::ClassMember],
    src: &SourceFile,
) -> Vec<(String, String, bool)> {
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Property(p) => p.hooks.as_ref().map(|hooks| (p, hooks)),
            _ => None,
        })
        .flat_map(|(p, hooks)| {
            let name = crate::strip_sigil(span_text(src, p.name)).to_owned();
            hooks
                .iter()
                .filter(|hook| hook.body.is_some())
                .map(|hook| {
                    (
                        name.clone(),
                        crate::signatures::hook_label(class, &name, hook.kind),
                        hook.kind == nvs_syntax::ast::PropertyHookKind::Set,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether `modifiers` leave the member they decorate readable from outside its
/// class — one reading, shared by [`own_methods`] and [`own_properties`],
/// because a class's two rosters answering visibility differently is a
/// difference no caller could justify.
///
/// The absent case is `public`, on [`own_methods`]' terms above.
fn is_public(modifiers: &[Modifier]) -> bool {
    !modifiers.contains(&Modifier::Private) && !modifiers.contains(&Modifier::Protected)
}

/// One declaration's own instance-property names, in declaration order, each
/// with whether it is `public` and the type it declares — a written `public
/// int $n;` and a promoted constructor parameter alike, each where it stands
/// among the members.
///
/// The visibility bit is [`own_methods`]' bit, read the same way by
/// [`is_public`]: `rule:security/reflection-enforces-visibility`'s reflective read has to face the check ordinary
/// code faces, and nothing below this crate can see a keyword. The type rides
/// with it because the spelling is a keyword's neighbour and lives exactly as
/// long — see [`declared_type`] and [`ClassLayout::field_types`].
///
/// A promoted parameter occupies an ordinary slot, because it is an ordinary
/// property: `nvs_types::signatures` records its type and visibility and
/// `nvs_ir::lower` stores the argument into this slot at constructor entry.
/// Its place in the order is the `constructor` member's own, which is all
/// that "declaration order" can mean for it — nothing reads a slot by number
/// across two declarations, `flatten_fields` keying every field by name.
fn own_properties(
    members: &[nvs_syntax::ast::ClassMember],
    src: &SourceFile,
) -> Vec<(String, bool, String)> {
    members
        .iter()
        .flat_map(|member| match &member.kind {
            ClassMemberKind::Property(p) if !is_static(p) => {
                vec![(
                    crate::strip_sigil(span_text(src, p.name)).to_owned(),
                    is_public(&p.modifiers),
                    declared_type(src, Some(&p.ty)),
                )]
            }
            ClassMemberKind::Method(m) if span_text(src, m.name) == "constructor" => m
                .params
                .iter()
                .filter(|p| p.is_promoted())
                .map(|p| {
                    (
                        crate::strip_sigil(span_text(src, p.name)).to_owned(),
                        is_public(&p.modifiers),
                        declared_type(src, p.ty.as_ref()),
                    )
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// One declaration's type as it is written, with every run of whitespace
/// flattened to a single space so a union broken across lines still reads as
/// one name — and the empty string where the declaration wrote none, which
/// `nvs_syntax::check_declarations` has already reported.
///
/// The written spelling rather than [`crate::ty::TypeInterner::describe`]'s
/// canonical one, because this pass resolves no types: it walks an AST and a
/// hierarchy, and taking the interned rendering would mean threading the
/// signature table through every caller to canonicalise a name a program's
/// author already wrote. What a reflective read then reports is the
/// declaration, which is the thing it is describing.
fn declared_type(src: &SourceFile, ty: Option<&nvs_syntax::ast::Type>) -> String {
    ty.map_or_else(String::new, |ty| {
        span_text(src, ty.span)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn is_static(p: &PropertyMember) -> bool {
    p.modifiers.contains(&Modifier::Static)
}

/// Appends `qname`'s slots to `fields`: its superclass chain's first, then its
/// own. `seen` guards a cyclic `extends` the hierarchy pass already diagnosed.
fn flatten_fields(
    qname: &QName,
    graph: &ClassGraph,
    own: &FxHashMap<QName, Vec<(String, bool, String)>>,
    fields: &mut Vec<(String, bool, String)>,
    seen: &mut Vec<QName>,
) {
    if seen.contains(qname) {
        return;
    }
    seen.push(qname.clone());
    // A class has at most one `extends`; an interface may list several, and
    // none of them contributes a slot, so taking the whole list is correct for
    // both without a kind check.
    if let Some(links) = graph.get(qname) {
        for parent in &links.extends {
            flatten_fields(parent, graph, own, fields, seen);
        }
    }
    if let Some(names) = own.get(qname) {
        for slot in names {
            // A subclass redeclaring an inherited property names the same
            // slot rather than claiming a second one — PHP's own behaviour,
            // and the only one under which a `FieldGet` naming the *declaring*
            // class stays valid for every subclass. The ancestor's visibility
            // is the slot's too, for the same reason it is one slot: there is
            // one field, so there is one answer to who may read it, and the
            // narrower one is the safe direction for a question `rule:security/reflection-enforces-visibility`
            // makes a privilege check, and its declared type is the slot's for
            // the same reason: one field, one type.
            if !fields.iter().any(|(held, _, _)| *held == slot.0) {
                fields.push(slot.clone());
            }
        }
    }
}

/// Appends every method `qname` answers to `methods`, as `(name, declaring
/// class label, is `public`)` — its own first, then its superclass chain's,
/// then any interface default it inherits. The first entry for a name wins,
/// which is what makes an override beat the declaration it overrides, and it
/// carries that declaration's own visibility with it.
///
/// `walked` guards the cyclic `extends` the hierarchy pass has already
/// diagnosed, exactly like [`flatten_fields`]' own `seen`.
fn flatten_methods(
    qname: &QName,
    graph: &ClassGraph,
    own: &FxHashMap<QName, Vec<(String, bool)>>,
    methods: &mut Vec<(String, String, bool)>,
    walked: &mut Vec<QName>,
) {
    if walked.contains(qname) {
        return;
    }
    walked.push(qname.clone());
    let label = qname.to_string();
    if let Some(names) = own.get(qname) {
        for (name, public) in names {
            if !methods.iter().any(|(have, _, _)| have == name) {
                methods.push((name.clone(), label.clone(), *public));
            }
        }
    }
    let Some(links) = graph.get(qname) else {
        return;
    };
    // `extends` before `implements`: a superclass's concrete method beats an
    // interface default of the same name (`rule:classes/interface-default-methods`'s conflict rule).
    for parent in links.extends.iter().chain(links.implements.iter()) {
        flatten_methods(parent, graph, own, methods, walked);
    }
}

/// Appends every property hook `qname` answers to `hooks`, as `(property
/// name, hook label, is the `set` accessor)` — its own first, then its
/// superclass chain's, then any interface's.
///
/// [`flatten_methods`]' walk, keyed on the *pair* rather than on a name: a
/// class that hooks only the `set` of a property its superclass hooks both
/// accessors of answers its own `set` and the inherited `get`, which is what
/// overriding one accessor means.
fn flatten_hooks(
    qname: &QName,
    graph: &ClassGraph,
    own: &FxHashMap<QName, Vec<(String, String, bool)>>,
    hooks: &mut Vec<(String, String, bool)>,
    walked: &mut Vec<QName>,
) {
    if walked.contains(qname) {
        return;
    }
    walked.push(qname.clone());
    if let Some(declared) = own.get(qname) {
        for (property, label, set) in declared {
            if !hooks
                .iter()
                .any(|(have, _, kind)| have == property && kind == set)
            {
                hooks.push((property.clone(), label.clone(), *set));
            }
        }
    }
    let Some(links) = graph.get(qname) else {
        return;
    };
    for parent in links.extends.iter().chain(links.implements.iter()) {
        flatten_hooks(parent, graph, own, hooks, walked);
    }
}

/// Appends every class and interface `qname` is a subtype of — transitively,
/// excluding `qname` itself — to `conforms`.
fn collect_conforms(
    qname: &QName,
    graph: &ClassGraph,
    conforms: &mut Vec<QName>,
    visited: &mut Vec<QName>,
) {
    let Some(links) = graph.get(qname) else {
        return;
    };
    for parent in links.extends.iter().chain(links.implements.iter()) {
        if visited.contains(parent) {
            continue;
        }
        visited.push(parent.clone());
        conforms.push(parent.clone());
        collect_conforms(parent, graph, conforms, visited);
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};
    use nvs_hir::resolve_file;
    use nvs_syntax::parse_file;

    use super::*;

    /// Parses and resolves `src`, then builds its layout table.
    fn layouts(src: &str) -> ClassLayoutTable {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        build_class_layouts(
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
            &module.graph,
        )
    }

    #[test]
    fn a_plain_class_gets_its_properties_in_declaration_order() {
        let table = layouts(
            "<?nvs\nclass Point {\n  public int $x;\n  public int $y;\n  \
             function constructor(int $x, int $y) { $this->x = $x; $this->y = $y; }\n}\n",
        );
        let layout = table.get("Point").expect("Point has a layout");
        assert_eq!(layout.fields, ["x", "y"]);
        assert_eq!(layout.slot_of("x"), Some(0));
        assert_eq!(layout.slot_of("y"), Some(1));
        assert_eq!(layout.slot_of("z"), None);
        assert!(layout.conforms.is_empty());
    }

    #[test]
    fn a_subclass_slots_follow_its_parents() {
        let table = layouts(
            "<?nvs\nclass Animal {\n  public int $legs;\n  \
             function constructor(int $legs) { $this->legs = $legs; }\n}\n\
             class Dog extends Animal {\n  public string $name;\n  \
             function constructor(string $name) { parent::constructor(4); $this->name = $name; }\n}\n",
        );
        assert_eq!(table.get("Animal").expect("Animal").fields, ["legs"]);
        let dog = table.get("Dog").expect("Dog");
        assert_eq!(dog.fields, ["legs", "name"]);
        assert_eq!(dog.slot_of("legs"), Some(0));
        assert_eq!(dog.slot_of("name"), Some(1));
        assert_eq!(dog.conforms, ["Animal"]);
    }

    #[test]
    fn three_levels_stack_in_declaration_order() {
        let table = layouts(
            "<?nvs\nclass A { public int $a; function constructor() { $this->a = 1; } }\n\
             class B extends A { public int $b; function constructor() { $this->b = 2; } }\n\
             class C extends B { public int $c; function constructor() { $this->c = 3; } }\n",
        );
        assert_eq!(table.get("C").expect("C").fields, ["a", "b", "c"]);
        assert_eq!(table.get("C").expect("C").conforms, ["B", "A"]);
    }

    #[test]
    fn an_interface_has_an_entry_with_no_slots() {
        let table = layouts(
            "<?nvs\ninterface Greets { public function greet(): string; }\n\
             class Dog implements Greets {\n  public string $name;\n  \
             function constructor(string $name) { $this->name = $name; }\n  \
             public function greet(): string { return $this->name; }\n}\n",
        );
        let greets = table.get("Greets").expect("Greets has a layout");
        assert!(greets.fields.is_empty());
        assert_eq!(table.get("Dog").expect("Dog").conforms, ["Greets"]);
    }

    /// The four `nvs_hir::interfaces` names have no source declaration at
    /// all, so without the seeding in [`build_class_layouts`] an implementor's
    /// `conforms` would name a label the table has no entry for — and
    /// `nvs-codegen` drops exactly those edges, leaving `$m instanceof
    /// Stringable` with nothing to test against.
    #[test]
    fn a_reserved_global_interface_has_an_entry_and_an_implementor_keeps_the_edge() {
        let table = layouts(
            "<?nvs\nclass Money implements Stringable {\n  \
             public function toString(): string { return \"m\"; }\n}\n\
             class Coin extends Money {\n}\n",
        );
        for (name, _) in nvs_hir::interfaces::RESERVED {
            let layout = table.get(name).expect("a reserved interface has a layout");
            assert!(layout.fields.is_empty(), "{name} claims a slot");
        }
        assert_eq!(table.get("Money").expect("Money").conforms, ["Stringable"]);
        assert_eq!(
            table.get("Coin").expect("Coin").conforms,
            ["Money", "Stringable"]
        );
    }

    #[test]
    fn an_interfaces_own_parents_are_flattened_in() {
        let table = layouts(
            "<?nvs\ninterface Named { public function name(): string; }\n\
             interface Greets extends Named { public function greet(): string; }\n\
             class Dog implements Greets {\n  \
             public function name(): string { return \"rex\"; }\n  \
             public function greet(): string { return \"woof\"; }\n}\n",
        );
        let dog = table.get("Dog").expect("Dog");
        assert_eq!(dog.conforms, ["Greets", "Named"]);
    }

    #[test]
    fn a_static_property_claims_no_slot() {
        let table = layouts(
            "<?nvs\nclass Counter {\n  public static int $total;\n  public int $count;\n  \
             function constructor() { $this->count = 0; }\n}\n",
        );
        assert_eq!(table.get("Counter").expect("Counter").fields, ["count"]);
    }

    #[test]
    fn a_namespaced_class_is_keyed_by_its_full_label() {
        let table = layouts(
            "<?nvs\nnamespace App\\Model;\nclass User {\n  public string $email;\n  \
             function constructor(string $email) { $this->email = $email; }\n}\n",
        );
        assert!(table.get("User").is_none());
        assert_eq!(
            table.get("App\\Model\\User").expect("User").fields,
            ["email"]
        );
    }

    #[test]
    fn a_redeclared_property_reuses_its_inherited_slot() {
        let table = layouts(
            "<?nvs\nclass A { public int $v; function constructor() { $this->v = 1; } }\n\
             class B extends A { public int $v; public int $w; \
             function constructor() { $this->v = 2; $this->w = 3; } }\n",
        );
        assert_eq!(table.get("B").expect("B").fields, ["v", "w"]);
    }

    /// A host classifies a finished ending by the marker's *name*, because the
    /// crate that answers the question depends on `nvs-runtime` and on no part
    /// of the compiler — `nvs_runtime::FINISH_MARKER_NAME` says so at the
    /// spelling itself, and `nvs_stdlib::script` re-exports it.
    /// This crate sees both sides, so this is where the two are held together:
    /// without it, renaming the class the compiler declares would leave every
    /// finish reported as an uncaught throw and no build would say so.
    #[test]
    fn the_marker_the_runtime_classifies_by_is_the_one_the_compiler_declares() {
        assert_eq!(
            nvs_stdlib::script::FINISH_MARKER_NAME,
            nvs_hir::errors::FINISH_MARKER
        );
    }

    /// A file declaring nothing still gets both rosters no source declares,
    /// and nothing else — `nvs_hir::errors`' classes and
    /// `nvs_hir::interfaces`' interfaces exist in every program, with the
    /// exception root's four slots inherited at the same indices by every one
    /// of its subclasses.
    #[test]
    fn a_file_declaring_nothing_yields_exactly_the_two_compiler_owned_rosters() {
        let table = layouts("<?nvs\necho \"hi\";\n");
        assert_eq!(
            table.len(),
            nvs_hir::errors::TREE.len() + nvs_hir::interfaces::RESERVED.len()
        );
        for (name, _) in nvs_hir::errors::TREE {
            let layout = table.get(name).unwrap_or_else(|| panic!("{name}"));
            // The exception root's own row *is* `PROPERTIES` and every row
            // under it adds to those. The finish marker is the row under
            // nothing, so it carries no slot at all — not even `backtrace`,
            // which is the property a `catch` would have read off it.
            let marker = *name == nvs_hir::errors::FINISH_MARKER;
            let mut want: Vec<&str> = if marker {
                Vec::new()
            } else {
                nvs_hir::errors::PROPERTIES.to_vec()
            };
            if *name != nvs_hir::errors::ROOT {
                want.extend(nvs_hir::errors::own_properties(name));
            }
            assert_eq!(layout.fields, want, "{name}");
            assert_eq!(
                layout.slot_of("backtrace"),
                if marker { None } else { Some(2) },
                "{name}"
            );
        }
        assert_eq!(
            table
                .get("ParseError")
                .expect("ParseError")
                .slot_of("issues"),
            Some(nvs_hir::errors::ISSUES_SLOT)
        );
    }

    /// A user class extending the tree gets its own slots *after* the root's,
    /// which is what makes `nvs_runtime::throwable`'s fixed slot indices hold.
    #[test]
    fn a_user_exception_class_appends_its_slots_after_the_roots() {
        let table = layouts("<?nvs\nclass MyError extends IOError { public int $code; }\n");
        let layout = table.get("MyError").expect("MyError");
        assert_eq!(
            layout.fields,
            ["message", "previous", "backtrace", "location", "code"]
        );
        assert_eq!(layout.conforms, ["IOError", "RuntimeError", "Throwable"]);
    }
}
