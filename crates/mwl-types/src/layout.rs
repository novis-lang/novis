//! Every declared class's **field slot order** and its flattened set of
//! supertypes — the second thing this crate publishes for `mwl-ir` to read
//! back, alongside [`crate::expr_table`].
//!
//! # Why it lives here and not in `mwl-ir`
//!
//! Exactly [`crate::expr_table`]'s reasoning, and no new one: a slot order has
//! to put an ancestor's properties before a subclass's own
//! (`mwl_runtime::object`'s layout rule), which needs the resolved
//! [`ClassGraph`] — and `mwl-ir` deliberately does not depend on `mwl-hir` at
//! all. So the *resolution* happens once, here, and `mwl-ir` copies the answer
//! into `mwl_ir::ir::Class` rather than re-deriving it.
//!
//! # What it is not
//!
//! Not a byte layout. This table says *which slot index* a field occupies and
//! nothing about how wide one is, because `mwl_runtime::object` makes every
//! slot the same width — see that module's own docs for the decision and its
//! cost. `mwl-codegen` turns a slot index into an offset through
//! `mwl_runtime::field_offset`, so the arithmetic exists in exactly one place.
//!
//! # Declaration order is the slot order
//!
//! A class's own properties occupy slots in the order they are written, after
//! every slot its ancestors already claimed. That makes a recompile of an
//! unchanged file reproduce the same layout — the same stability property
//! `mwl_ir::ids` needs for a probe id, for the same reason: an artifact cached
//! under ADR 0042 must still describe the code it is paired with.
//!
//! A `static` property claims no slot: ADR 0008 makes it class storage, not
//! instance storage. A *hooked* property (ADR 0014 § 1) does claim one, even
//! when nothing ever reads it — MWL has no virtual/backed split, and
//! [`crate::signatures::PropertyHooks`] owns that decision and what it
//! spends.
//!
//! # Known gaps
//!
//! * **A promoted constructor parameter claims no slot yet.**
//!   [`crate::signatures`] does not record one as a property either (its own
//!   known-gaps list), so this table is exactly as complete as the signature
//!   table it must agree with — making the two disagree would be worse than
//!   both being narrow.
//! * **An enum has no entry.** ADR 0010 makes an enum a closed integer value
//!   type, not an instance with fields.
//! * **Only the classes declared in the files walked are present.** A `Core`
//!   class has no source declaration and therefore no layout; `mwl-stdlib`
//!   owns those, and they are native Rust rather than field-slot objects
//!   (`docs/agent/loop-goal.md`).

use mwl_diagnostics::SourceFile;
use mwl_hir::{ClassGraph, QName};
use mwl_syntax::ast::{ClassMemberKind, Modifier, NamespaceDecl, PropertyMember, Stmt, StmtKind};
use rustc_hash::FxHashMap;

use crate::span_text;

/// A namespace name's segments — the same one-liner [`crate::check`] and
/// [`crate::signatures`] each keep for themselves, kept here too rather than
/// exported: it is two lines, and hoisting it would only move the duplication.
fn qname_segments(src: &SourceFile, name: &mwl_syntax::ast::Name) -> Vec<String> {
    QName::parse(span_text(src, name.span)).segments().to_vec()
}

/// One class's or interface's instance-field slot order and supertype set.
#[derive(Clone, Debug, Default)]
pub struct ClassLayout {
    /// Every field slot in index order — every ancestor's first, then this
    /// class's own in declaration order. `$`-sigil not included.
    pub fields: Vec<String>,
    /// Every *other* class and interface an instance of this one also is,
    /// rendered the same way [`ClassLayout`]'s own key is. Transitive, and
    /// deliberately excluding the class itself — `instanceof` checks identity
    /// separately (`mwl_runtime::ClassDesc::conforms_to`).
    pub conforms: Vec<String>,
    /// Every method callable on an instance of this class, as `(method name,
    /// declaring class label)` — its own first, then the nearest ancestor
    /// declaring each name it does not. Only methods with a *body*: an
    /// abstract or bodiless interface method has no code to name.
    ///
    /// This is what `mwl_runtime::ClassDesc::method` answers a
    /// `static::method(...)` dispatch from, so the precedence has to be the
    /// language's: a class's own override, then its superclass chain, then an
    /// interface default (ADR 0043 § 2). A depth-first walk that takes
    /// `extends` before `implements` produces exactly that order.
    pub methods: Vec<(String, String)>,
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
/// label's class half, so `mwl-ir` can match a `New`/`FieldGet` target against
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

/// Builds the layout of every class and interface declared in `stmts`.
///
/// `graph` is the already-resolved hierarchy `mwl_hir::resolve_file` produced
/// for the same file — the one thing this pass cannot derive from the AST,
/// since an `extends Foo` reference has to be resolved against the active
/// namespace and imports.
///
/// A class whose ancestor chain is broken (an unresolved or cyclic `extends`,
/// both of which `mwl_hir::hierarchy` has already diagnosed) simply gets a
/// layout built from as much of the chain as resolves, rather than being
/// omitted: this pass never reports a diagnostic of its own, so leaving a hole
/// would turn an already-reported error into a second, unexplained failure
/// further down the pipeline.
#[must_use]
pub fn build_class_layouts(
    stmts: &[Stmt],
    src: &SourceFile,
    graph: &ClassGraph,
) -> ClassLayoutTable {
    let mut own: FxHashMap<QName, Vec<String>> = FxHashMap::default();
    let mut own_methods: FxHashMap<QName, Vec<String>> = FxHashMap::default();
    // The exception tree first: it has no source declaration to collect from
    // (`mwl_hir::errors`), and a user class extending it needs its four slots
    // already claimed before its own are appended.
    for (name, _) in mwl_hir::errors::TREE {
        let fields = mwl_hir::errors::own_properties(name)
            .iter()
            .map(|p| (*p).to_owned())
            .collect();
        // These constructors are synthesized rather than written
        // (`mwl_ir::lower::exception`), so they are the methods with a body
        // that no source walk can find. One per class that declares
        // properties of its own — `mwl_hir::errors::declares_constructor`.
        let methods = if mwl_hir::errors::declares_constructor(name) {
            vec!["constructor".to_owned()]
        } else {
            Vec::new()
        };
        own.insert(QName::parse(name), fields);
        own_methods.insert(QName::parse(name), methods);
    }
    collect_own(stmts, src, &[], &mut own, &mut own_methods);

    let mut table = ClassLayoutTable::default();
    for qname in own.keys() {
        let mut fields = Vec::new();
        let mut seen = Vec::new();
        flatten_fields(qname, graph, &own, &mut fields, &mut seen);

        let mut conforms = Vec::new();
        let mut visited = vec![qname.clone()];
        collect_conforms(qname, graph, &mut conforms, &mut visited);

        let mut methods = Vec::new();
        let mut walked = Vec::new();
        flatten_methods(qname, graph, &own_methods, &mut methods, &mut walked);

        table.by_label.insert(
            qname.to_string(),
            ClassLayout {
                fields,
                conforms: conforms.iter().map(QName::to_string).collect(),
                methods,
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
    out: &mut FxHashMap<QName, Vec<String>>,
    methods: &mut FxHashMap<QName, Vec<String>>,
) {
    let mut current = namespace.to_vec();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let scoped = name
                    .as_ref()
                    .map_or_else(Vec::new, |n| qname_segments(src, n));
                match body {
                    Some(block) => collect_own(&block.stmts, src, &scoped, out, methods),
                    None => current = scoped,
                }
            }
            StmtKind::ClassDecl(decl) => {
                let qname = QName::join(&current, span_text(src, decl.name.span));
                out.insert(qname.clone(), own_properties(&decl.members, src));
                methods.insert(qname, own_methods(&decl.members, src));
            }
            // An interface declares no instance property (ADR 0043 § 2 gives
            // it method bodies, not state), but it still needs an entry: it is
            // a legal `instanceof` target and a legal `catch` type, so
            // `mwl-codegen` must have a descriptor to point at. Its *default*
            // method bodies are real code, though, so they are collected the
            // same way a class's are.
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current, span_text(src, decl.name.span));
                out.insert(qname.clone(), Vec::new());
                methods.insert(qname, own_methods(&decl.members, src));
            }
            _ => {}
        }
    }
}

/// One declaration's own method names — only those with a body, since a
/// bodiless one has no compiled code for a descriptor to point at.
fn own_methods(members: &[mwl_syntax::ast::ClassMember], src: &SourceFile) -> Vec<String> {
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Method(m) if m.body.is_some() => {
                Some(span_text(src, m.name).to_owned())
            }
            _ => None,
        })
        .collect()
}

/// One declaration's own instance-property names, in declaration order.
fn own_properties(members: &[mwl_syntax::ast::ClassMember], src: &SourceFile) -> Vec<String> {
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Property(p) if !is_static(p) => {
                Some(crate::strip_sigil(span_text(src, p.name)).to_owned())
            }
            _ => None,
        })
        .collect()
}

fn is_static(p: &PropertyMember) -> bool {
    p.modifiers.contains(&Modifier::Static)
}

/// Appends `qname`'s slots to `fields`: its superclass chain's first, then its
/// own. `seen` guards a cyclic `extends` the hierarchy pass already diagnosed.
fn flatten_fields(
    qname: &QName,
    graph: &ClassGraph,
    own: &FxHashMap<QName, Vec<String>>,
    fields: &mut Vec<String>,
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
        for name in names {
            // A subclass redeclaring an inherited property names the same
            // slot rather than claiming a second one — PHP's own behaviour,
            // and the only one under which a `FieldGet` naming the *declaring*
            // class stays valid for every subclass.
            if !fields.contains(name) {
                fields.push(name.clone());
            }
        }
    }
}

/// Appends every method `qname` answers to `methods`, as `(name, declaring
/// class label)` — its own first, then its superclass chain's, then any
/// interface default it inherits. The first entry for a name wins, which is
/// what makes an override beat the declaration it overrides.
///
/// `walked` guards the cyclic `extends` the hierarchy pass has already
/// diagnosed, exactly like [`flatten_fields`]' own `seen`.
fn flatten_methods(
    qname: &QName,
    graph: &ClassGraph,
    own: &FxHashMap<QName, Vec<String>>,
    methods: &mut Vec<(String, String)>,
    walked: &mut Vec<QName>,
) {
    if walked.contains(qname) {
        return;
    }
    walked.push(qname.clone());
    let label = qname.to_string();
    if let Some(names) = own.get(qname) {
        for name in names {
            if !methods.iter().any(|(have, _)| have == name) {
                methods.push((name.clone(), label.clone()));
            }
        }
    }
    let Some(links) = graph.get(qname) else {
        return;
    };
    // `extends` before `implements`: a superclass's concrete method beats an
    // interface default of the same name (ADR 0043 § 2's conflict rule).
    for parent in links.extends.iter().chain(links.implements.iter()) {
        flatten_methods(parent, graph, own, methods, walked);
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
    use mwl_diagnostics::{Diagnostics, SourceMap};
    use mwl_hir::resolve_file;
    use mwl_syntax::parse_file;

    use super::*;

    /// Parses and resolves `src`, then builds its layout table.
    fn layouts(src: &str) -> ClassLayoutTable {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        build_class_layouts(&stmts, map.file(file), &module.graph)
    }

    #[test]
    fn a_plain_class_gets_its_properties_in_declaration_order() {
        let table = layouts(
            "<?mwl\nclass Point {\n  public int $x;\n  public int $y;\n  \
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
            "<?mwl\nclass Animal {\n  public int $legs;\n  \
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
            "<?mwl\nclass A { public int $a; function constructor() { $this->a = 1; } }\n\
             class B extends A { public int $b; function constructor() { $this->b = 2; } }\n\
             class C extends B { public int $c; function constructor() { $this->c = 3; } }\n",
        );
        assert_eq!(table.get("C").expect("C").fields, ["a", "b", "c"]);
        assert_eq!(table.get("C").expect("C").conforms, ["B", "A"]);
    }

    #[test]
    fn an_interface_has_an_entry_with_no_slots() {
        let table = layouts(
            "<?mwl\ninterface Greets { public function greet(): string; }\n\
             class Dog implements Greets {\n  public string $name;\n  \
             function constructor(string $name) { $this->name = $name; }\n  \
             public function greet(): string { return $this->name; }\n}\n",
        );
        let greets = table.get("Greets").expect("Greets has a layout");
        assert!(greets.fields.is_empty());
        assert_eq!(table.get("Dog").expect("Dog").conforms, ["Greets"]);
    }

    #[test]
    fn an_interfaces_own_parents_are_flattened_in() {
        let table = layouts(
            "<?mwl\ninterface Named { public function name(): string; }\n\
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
            "<?mwl\nclass Counter {\n  public static int $total;\n  public int $count;\n  \
             function constructor() { $this->count = 0; }\n}\n",
        );
        assert_eq!(table.get("Counter").expect("Counter").fields, ["count"]);
    }

    #[test]
    fn a_namespaced_class_is_keyed_by_its_full_label() {
        let table = layouts(
            "<?mwl\nnamespace App\\Model;\nclass User {\n  public string $email;\n  \
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
            "<?mwl\nclass A { public int $v; function constructor() { $this->v = 1; } }\n\
             class B extends A { public int $v; public int $w; \
             function constructor() { $this->v = 2; $this->w = 3; } }\n",
        );
        assert_eq!(table.get("B").expect("B").fields, ["v", "w"]);
    }

    /// A file declaring nothing still gets the exception tree, and nothing
    /// else — `mwl_hir::errors`' classes exist in every program, with the
    /// root's four slots inherited at the same indices by every one of them.
    #[test]
    fn a_file_declaring_nothing_yields_exactly_the_exception_tree() {
        let table = layouts("<?mwl\necho \"hi\";\n");
        assert_eq!(table.len(), mwl_hir::errors::TREE.len());
        for (name, _) in mwl_hir::errors::TREE {
            let layout = table.get(name).unwrap_or_else(|| panic!("{name}"));
            // The root's own row *is* `PROPERTIES`; every other row adds to it.
            let mut want: Vec<&str> = mwl_hir::errors::PROPERTIES.to_vec();
            if *name != mwl_hir::errors::ROOT {
                want.extend(mwl_hir::errors::own_properties(name));
            }
            assert_eq!(layout.fields, want, "{name}");
            assert_eq!(layout.slot_of("backtrace"), Some(2), "{name}");
        }
        assert_eq!(
            table
                .get("ParseError")
                .expect("ParseError")
                .slot_of("issues"),
            Some(mwl_hir::errors::ISSUES_SLOT)
        );
    }

    /// A user class extending the tree gets its own slots *after* the root's,
    /// which is what makes `mwl_runtime::throwable`'s fixed slot indices hold.
    #[test]
    fn a_user_exception_class_appends_its_slots_after_the_roots() {
        let table = layouts("<?mwl\nclass MyError extends IOError { public int $code; }\n");
        let layout = table.get("MyError").expect("MyError");
        assert_eq!(
            layout.fields,
            ["message", "previous", "backtrace", "location", "code"]
        );
        assert_eq!(layout.conforms, ["IOError", "RuntimeError", "Throwable"]);
    }
}
