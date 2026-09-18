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

use crate::consts::ConstValue;
use crate::span_text;

/// A namespace name's segments — the same one-liner [`crate::check`] and
/// [`crate::signatures`] each keep for themselves, kept here too rather than
/// exported: it is two lines, and hoisting it would only move the duplication.
fn qname_segments(src: &SourceFile, name: &nvs_syntax::ast::Name) -> Vec<String> {
    QName::parse(span_text(src, name.span)).segments().to_vec()
}

/// One row of [`ClassLayout::methods`]: `(method name, declaring class label,
/// is `public`, is `protected`, parameter names, parameter type spellings)`.
///
/// A name rather than the tuple written out at each of the four layers that
/// carry it — this crate, `nvs_ir::ir::Class`, `nvs-codegen`'s own roster and
/// the pattern each of them destructures it with — so the shape is stated once
/// and a layer cannot come to disagree with the one above it about which
/// element is which. The field's own doc is what every element means.
pub type MethodEntry = (String, String, bool, bool, Vec<String>, Vec<String>);

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
    /// Whether each field slot is declared `protected`, in [`Self::fields`]'
    /// own order and beside [`Self::public_fields`] rather than folded into
    /// it: two bits spell the three levels, and it is the *level* rather than
    /// the readable/not pair that
    /// `rule:security/reflection-enforces-visibility`
    /// needs. A `private` slot is the one both bits are false for, and it is
    /// reached only from the declaring class's own bodies, where a `protected`
    /// one is reached from every class in the hierarchy that declares it too —
    /// which is what an ordinary member access at each of those sites does.
    ///
    /// `nvs_ir::ir::Class::protected_fields` carries it down,
    /// `nvs_runtime::ClassDesc::field_is_protected` answers it and
    /// `nvs_runtime::visibility` is what asks.
    ///
    /// **Cost:** one `bool` per field slot per class, once per compiled unit.
    pub protected_fields: Vec<bool>,
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
    /// deliberately excluding the class itself — an `is` test checks identity
    /// separately (`nvs_runtime::ClassDesc::conforms_to`).
    pub conforms: Vec<String>,
    /// Every method callable on an instance of this class, as a
    /// [`MethodEntry`] — its own first,
    /// then the nearest ancestor declaring each name it does not. Only methods
    /// with a *body*: an abstract or bodiless interface method has no code to
    /// name.
    ///
    /// This is what `nvs_runtime::ClassDesc::method` answers a
    /// `static::method(...)` dispatch from, so the precedence has to be the
    /// language's: a class's own override, then its superclass chain, then an
    /// interface default (`rule:classes/interface-default-methods`). A depth-first walk that takes
    /// `extends` before `implements` produces exactly that order.
    ///
    /// The visibility bits are carried because the keyword has no source below
    /// the front end and one dispatch needs the level: a call whose receiver
    /// names no class is checked where it lands rather than where it was
    /// written, so `nvs_runtime::MethodRow` — the far end of this table — is
    /// what `nvs_runtime::visibility` reads that level off. Two bits rather
    /// than one three-valued field, for [`Self::protected_fields`]' reason
    /// exactly, and `private` is the pair both are false for. Every visibility
    /// question a receiver whose class is written down asks is answered at the
    /// call site instead, against the signature table.
    ///
    /// The parameter names ride with the row for that bit's reason one step
    /// further, and they are [`Self::field_types`]' reason exactly: a
    /// parameter's spelling exists only where its declaration does, and
    /// `nvs_runtime::MethodRow::arity` — the one other parameter fact the
    /// runtime carries — counts them without naming one. The `$` sigil is not
    /// included and the receiver is excluded, so the list is either that
    /// arity long or **empty**, which reads as "no declaration was read for
    /// this row" rather than "the method takes nothing": a synthesized
    /// exception constructor and a `rule:classes/delegation-by-field` forward
    /// are the two such rows, and neither has written parameters to spell.
    /// `Core\Reflect\MethodInfo::parameters` is what reads them back.
    ///
    /// The type spellings ride beside the names, in the same order and in
    /// [`Self::field_types`]' currency exactly: the text the declaration wrote
    /// with its whitespace collapsed, and the **empty string** for a parameter
    /// that wrote no type, which on a method is source
    /// `nvs_syntax::check_declarations` has already refused — only a closure
    /// literal may leave one out, and a closure declares no class member. The
    /// list is the names' own length or empty, and
    /// empty reads there exactly as it reads for the names, which is why the
    /// two travel as one row rather than as a pair a reader would have to check
    /// for agreement. `Core\Command`'s help page is what asks for them: a
    /// `nvs_runtime::Tag` nibble cannot tell `int` from `uint` and cannot name
    /// an enum, so a page rendering `[--retries]` has no other source for the
    /// word `uint`.
    ///
    /// **Cost:** two `String`s per declared parameter per method per class,
    /// once per compiled unit, never per request.
    pub methods: Vec<MethodEntry>,
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
    /// Every class constant a program can name on this class — its own first,
    /// then the nearest ancestor declaring each name it does not, on
    /// [`Self::methods`]' precedence exactly.
    ///
    /// Flattened rather than own-only for the reason
    /// `rule:core-classes/reflect` gives the two rosters beside it: a roster
    /// that dropped an inherited name would answer a declared constant and a
    /// misspelling the same way, and `Foo::BAR` in source reads through the
    /// chain, so a description that did not would describe a different class
    /// than the one running.
    ///
    /// **Cost:** one [`ClassConstant`] per constant per class, once per
    /// compiled unit, never per request.
    pub constants: Vec<ClassConstant>,
    /// Every `#[...]` attached to this declaration or to one of its own
    /// members, in source order — the class first, then each member's in
    /// declaration order.
    ///
    /// **Own-only, where [`Self::constants`] beside it is flattened**, and the
    /// difference is the difference between the two things: a constant is a
    /// *name a program may write on this class*, which `Foo::BAR` resolves up
    /// the chain, while an attribute is a fact about *where it was written*.
    /// A roster that inherited one would report a base class's attach site as
    /// this class's, which is the one thing
    /// `rule:attributes/structural-retrieval`'s target spellings never do.
    ///
    /// **Cost:** one [`ClassAttribute`] per attach site per class, once per
    /// compiled unit, never per request.
    pub attributes: Vec<ClassAttribute>,
}

/// One attached attribute, as much of it as a reflective description reads
/// back: which declaration it is written on, the name the named form gave it,
/// and its payload folded field by field.
///
/// A second reading of the same `#[...]` the checker's own
/// `crate::retrieval` walk indexes, and deliberately a coarser one.
/// That walk answers `Core\Attributes::get<T>` — a *structural* match against a
/// shape type, folded away at compile time so the running program holds no
/// table at all — and it borrows the AST to do it. This roster is the other
/// question: `Core\Reflect\ClassInfo::attributes` names the attach sites of a
/// class the checker never saw, so what it needs is owned data on the
/// descriptor. The payload is where the two part company, and
/// [`Self::fields`] says how.
#[derive(Clone, Debug)]
pub struct ClassAttribute {
    /// The member the attribute is written on — the **empty string** for the
    /// class or interface declaration itself.
    ///
    /// A property's name with no `$` sigil, or a method's name; a parameter's
    /// attach site names the method here and the parameter in
    /// [`Self::parameter`], which is
    /// `rule:attributes/structural-retrieval`'s own pair of target spellings
    /// kept as a pair rather than folded into one invented one.
    pub member: String,
    /// The parameter of [`Self::member`] the attribute is written on, or the
    /// empty string for every other attach site.
    pub parameter: String,
    /// The `type` alias the named form names, or the **empty string** for the
    /// bare form.
    ///
    /// Carried as what was written rather than as a resolved name: the name
    /// checks the literal where it is *written* and is never part of how a
    /// caller asks for it (`rule:attributes/structural-retrieval`), so a
    /// description that resolved it would be reporting the checker's answer to
    /// a question the reader did not ask.
    pub name: String,
    /// The payload's fields in source order, each folded by
    /// [`crate::consts::fold_expr`].
    ///
    /// [`ConstValue::Ineligible`] is a value's absence and not a field's, on
    /// [`ClassConstant::value`]'s terms exactly — the field keeps its row, and
    /// `Core\Reflect\AttributeInfo::field` reports the bound rather than a name
    /// it does not know. Two shapes reach it here that a constant's right-hand
    /// side never does: a payload value that is a class constant, an enum case
    /// or `Foo::class`, each of which resolves through the *attach site's*
    /// namespace and `use` table, which this pass does not build.
    /// `crate::retrieval` is where that scope lives, and it is a
    /// checker pass over borrowed AST rather than a roster to copy.
    ///
    /// No `secret` bit rides here, where [`ClassConstant::secret`] needs one: a
    /// `secret` class constant reaching a payload is refused where it is
    /// written (`E0727`, `rule:attributes/payload-is-a-compile-time-constant`),
    /// so no qualified value is ever in one to hand back.
    pub fields: Vec<(String, ConstValue)>,
}

/// One class constant, as much of it as a reflective description reads back:
/// the name, the two bits that spell its visibility, the one
/// `rule:security/secret-qualifier` bit, and the value itself.
///
/// A struct rather than [`ClassLayout::methods`]' tuple because the last field
/// is a value and not a spelling, and that is the decision this roster turns
/// on. A constant *is* its value — a reflective read that answered the
/// declaration's source text would hand back `"[1, 2]"` and `"1 + 1"` and make
/// every caller write a parser — so the fold
/// [`crate::consts::fold_const`] already performs for
/// `rule:types/constant-in-type-position` is what travels, and
/// `nvs_runtime::ConstantDesc` is where it lands.
///
/// **[`ConstValue::Ineligible`] is a value's absence, not a constant's.** An
/// `array` or object constant keeps its row — the name and the visibility are
/// as much a fact about the class as any other constant's — and only the value
/// is missing, which `Core\Reflect\ClassInfo::constant` reports as the bound it
/// is rather than as a name it does not know.
#[derive(Clone, Debug)]
pub struct ClassConstant {
    /// The constant's name, as its declaration writes it — no class qualifier
    /// and no `::`.
    pub name: String,
    /// Whether code outside the declaring class may name it, on
    /// [`ClassLayout::public_fields`]' terms.
    pub public: bool,
    /// Whether it is declared `protected` — the second bit, read for
    /// [`ClassLayout::protected_fields`]' reason exactly, so a subclass's site
    /// reaches what an ordinary `static::CONST` there reaches.
    pub protected: bool,
    /// Whether the declaration's own annotation carries
    /// `rule:security/secret-qualifier`'s `secret`.
    ///
    /// It travels because the reflective read's return type is `mixed`, which
    /// carries no qualifier: a `secret string` handed back through one would be
    /// an ordinary string at the next sink, and
    /// `rule:security/secret-sinks-refuse`'s refusal would never be made. So
    /// the bit rides with the value and the reading member refuses rather than
    /// laundering — priority 1 of `rule:programs/memory-priority`, bought with
    /// one `bool`.
    pub secret: bool,
    /// What `rule:types/constant-in-type-position`'s fold makes of the
    /// declaration's right-hand side.
    pub value: ConstValue,
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

/// What one declaration says about one of its own methods: `(method name, is
/// `public`, is `protected`, parameter names, parameter type spellings)` — a
/// [`MethodEntry`] without the declaring class label, which is the one element
/// the declaration itself does not spell and [`flatten_methods`] joins on.
type OwnMethod = (String, bool, bool, Vec<String>, Vec<String>);

/// What every declaration says about its own methods, keyed by the class that
/// wrote them — [`own_methods`]' answer before [`flatten_methods`] walks a
/// chain of them into [`ClassLayout::methods`].
type OwnMethods = FxHashMap<QName, Vec<OwnMethod>>;

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
    let mut own: FxHashMap<QName, Vec<(String, bool, bool, String)>> = FxHashMap::default();
    let mut own_methods: OwnMethods = FxHashMap::default();
    let mut own_hooks: FxHashMap<QName, Vec<(String, String, bool)>> = FxHashMap::default();
    let mut own_constants: FxHashMap<QName, Vec<ClassConstant>> = FxHashMap::default();
    let mut own_attributes: FxHashMap<QName, Vec<ClassAttribute>> = FxHashMap::default();
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
            .map(|p| ((*p).to_owned(), true, false, String::new()))
            .collect();
        // These constructors are synthesized rather than written
        // (`nvs_ir::lower::exception`), so they are the methods with a body
        // that no source walk can find. One per class that declares
        // properties of its own — `nvs_hir::errors::declares_constructor`.
        let methods = if nvs_hir::errors::declares_constructor(name) {
            // Public: spec § 10's tree is constructed by every program that
            // throws, and a synthesized member writes no modifier — see
            // `own_methods` on why absent reads as `public`. Neither parameter
            // roster is filled, which is `ClassLayout::methods`' empty case:
            // `crate::error_lib` seeds this constructor in the call site's
            // currency — a message and an options bag — while the lowered body
            // takes the flattened values, so no one declaration spells its
            // parameters or types them.
            vec![(
                "constructor".to_owned(),
                true,
                false,
                Vec::new(),
                Vec::new(),
            )]
        } else {
            Vec::new()
        };
        own.insert(QName::parse(name), fields);
        own_methods.insert(QName::parse(name), methods);
    }
    // The compiler-declared global interfaces, for the same reason and on the
    // same terms (`nvs_hir::interfaces`): nothing declares `Stringable` in
    // source, but `$x is Stringable` needs a descriptor to point at
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
            &mut Own {
                properties: &mut own,
                methods: &mut own_methods,
                hooks: &mut own_hooks,
                constants: &mut own_constants,
                attributes: &mut own_attributes,
            },
        );
    }

    let mut table = ClassLayoutTable::default();
    for qname in own.keys() {
        let mut slots = Vec::new();
        let mut seen = Vec::new();
        flatten_fields(qname, graph, &own, &mut slots, &mut seen);
        let mut fields = Vec::with_capacity(slots.len());
        let mut public_fields = Vec::with_capacity(slots.len());
        let mut protected_fields = Vec::with_capacity(slots.len());
        let mut field_types = Vec::with_capacity(slots.len());
        for (name, public, protected, ty) in slots {
            fields.push(name);
            public_fields.push(public);
            protected_fields.push(protected);
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

        let mut constants = Vec::new();
        let mut const_walked = Vec::new();
        flatten_constants(
            qname,
            graph,
            &own_constants,
            &mut constants,
            &mut const_walked,
        );

        table.by_label.insert(
            qname.to_string(),
            ClassLayout {
                fields,
                public_fields,
                protected_fields,
                field_types,
                conforms: conforms.iter().map(QName::to_string).collect(),
                methods,
                hooks,
                constants,
                // Own-only, so there is no flattening walk beside the three
                // above — `ClassLayout::attributes` owns why.
                attributes: own_attributes.get(qname).cloned().unwrap_or_default(),
            },
        );
    }
    table
}

/// The rosters [`collect_own`] fills as it descends, gathered into one
/// parameter because one walk is what fills all of them: a map apiece would be
/// five walks over the same statements, each re-deriving the namespace the
/// others already resolved.
struct Own<'r> {
    properties: &'r mut FxHashMap<QName, Vec<(String, bool, bool, String)>>,
    methods: &'r mut OwnMethods,
    hooks: &'r mut FxHashMap<QName, Vec<(String, String, bool)>>,
    constants: &'r mut FxHashMap<QName, Vec<ClassConstant>>,
    attributes: &'r mut FxHashMap<QName, Vec<ClassAttribute>>,
}

/// Records each declared class's/interface's *own* instance properties, in
/// declaration order.
fn collect_own(stmts: &[Stmt], src: &SourceFile, namespace: &[String], out: &mut Own<'_>) {
    let mut current = namespace.to_vec();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let scoped = name
                    .as_ref()
                    .map_or_else(Vec::new, |n| qname_segments(src, n));
                match body {
                    Some(block) => {
                        collect_own(&block.stmts, src, &scoped, out);
                    }
                    None => current = scoped,
                }
            }
            StmtKind::ClassDecl(decl) => {
                let qname = QName::join(&current, span_text(src, decl.name.span));
                out.properties
                    .insert(qname.clone(), own_properties(&decl.members, src));
                out.methods
                    .insert(qname.clone(), own_methods(&decl.members, src));
                out.hooks
                    .insert(qname.clone(), own_hooks(&qname, &decl.members, src));
                out.constants
                    .insert(qname.clone(), own_constants(&decl.members, src));
                out.attributes.insert(
                    qname.clone(),
                    own_attributes(&decl.attributes, &decl.members, src),
                );
            }
            // An interface declares no instance property (`rule:classes/interface-default-methods` gives
            // it method bodies, not state), but it still needs an entry: it is
            // a legal `is` target and a legal `catch` type, so
            // `nvs-codegen` must have a descriptor to point at. Its *default*
            // method bodies are real code, though, so they are collected the
            // same way a class's are.
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current, span_text(src, decl.name.span));
                out.properties.insert(qname.clone(), Vec::new());
                out.methods
                    .insert(qname.clone(), own_methods(&decl.members, src));
                // An interface declares no *hooked* property either, its
                // properties being none at all; the entry is still made, on
                // the terms above.
                out.hooks
                    .insert(qname.clone(), own_hooks(&qname, &decl.members, src));
                // Constants are the one roster an interface fills as a class
                // does: `interface Limits { const int MAX = 10; }` is a name
                // every implementor answers, so the walk reads the same
                // members here that it reads there.
                out.constants
                    .insert(qname.clone(), own_constants(&decl.members, src));
                // Attributes likewise: `rule:attributes/attach-sites-and-forms`
                // names an interface and its members among the sites, and an
                // interface is a declaration a reflective description reaches
                // through `forClass` like any other.
                out.attributes.insert(
                    qname.clone(),
                    own_attributes(&decl.attributes, &decl.members, src),
                );
            }
            _ => {}
        }
    }
}

/// One declaration's own method names, each with the two bits that spell its
/// visibility and the name and declared type of every parameter it writes —
/// only those with a body, since a bodiless one has no compiled code for a
/// descriptor to point at.
///
/// The parameters are read here for [`own_properties`]' reason: both spellings
/// are a keyword's neighbour and live exactly as long, and [`declared_type`] is
/// the one that renders the type, so a parameter and a property report their
/// declarations in one currency. A promoted one is a
/// parameter like any other and keeps its place in both lists, where
/// [`own_properties`] also claims a slot for it, and the receiver is not among
/// them — `nvs_runtime::MethodRow::arity`, which this list is read against,
/// counts what the source wrote.
///
/// A declaration that wrote no visibility keyword at all counts as `public`,
/// which is `nvs_syntax::check_declarations`' `E_MISSING_VISIBILITY` already
/// being reported for it: this pass only has to not invent a level for source
/// that is being refused anyway, and the same reading is what keeps a
/// synthesized method — an exception constructor, `rule:iteration/generators`'s state machine
/// — callable, none of them writing a modifier.
fn own_methods(members: &[nvs_syntax::ast::ClassMember], src: &SourceFile) -> Vec<OwnMethod> {
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Method(m) if m.body.is_some() => Some((
                span_text(src, m.name).to_owned(),
                is_public(&m.modifiers),
                is_protected(&m.modifiers),
                m.params
                    .iter()
                    .map(|p| crate::strip_sigil(span_text(src, p.name)).to_owned())
                    .collect(),
                m.params
                    .iter()
                    .map(|p| declared_type(src, p.ty.as_ref()))
                    .collect(),
            )),
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

/// One declaration's own class constants, each folded to the value
/// [`ClassConstant`] carries.
///
/// Every one of them, where [`own_methods`] keeps only what has a body: a
/// constant is a declaration and nothing else, so there is no bodiless half to
/// exclude. The fold is [`crate::consts::fold_const`]'s and the `secret` bit is
/// [`crate::consts::type_carries_secret`]'s, both reused rather than rewritten
/// — a second reading of `const int X = -1;` in this crate is a second answer
/// waiting to differ from the one `rule:types/constant-in-type-position`
/// already gives.
fn own_constants(members: &[nvs_syntax::ast::ClassMember], src: &SourceFile) -> Vec<ClassConstant> {
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Const(c) => Some(ClassConstant {
                name: span_text(src, c.name).to_owned(),
                public: is_public(&c.modifiers),
                protected: is_protected(&c.modifiers),
                secret: c
                    .ty
                    .as_ref()
                    .is_some_and(crate::consts::type_carries_secret),
                value: crate::consts::fold_const(c, src),
            }),
            _ => None,
        })
        .collect()
}

/// One declaration's own attach sites, in source order: the declaration's own
/// `#[...]` groups first, then each member's, and each method's parameters'
/// after that method's own.
///
/// The site walk `crate::retrieval` makes over the same members,
/// made a second time and for the other reader: that one indexes an attach site
/// by the target a `Core\Attributes::get` call names and borrows the AST to do
/// it, where this one owns what it reads because it is headed for a class
/// descriptor that outlives every source file. The two agree on *which* sites
/// exist because they read the same member kinds — a class's or interface's
/// own groups, a property's, a method's, and a method parameter's — and that
/// list is `rule:attributes/attach-sites-and-forms`'.
///
/// A class constant's own groups are not among them, and that is the rule's
/// roster rather than an omission here.
fn own_attributes(
    groups: &[nvs_syntax::ast::AttributeGroup],
    members: &[nvs_syntax::ast::ClassMember],
    src: &SourceFile,
) -> Vec<ClassAttribute> {
    let mut out = Vec::new();
    push_attributes(&mut out, groups, "", "", src);
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                let name = crate::strip_sigil(span_text(src, p.name));
                push_attributes(&mut out, &p.attributes, name, "", src);
            }
            ClassMemberKind::Method(m) => {
                let method = span_text(src, m.name);
                push_attributes(&mut out, &m.attributes, method, "", src);
                for param in &m.params {
                    let name = crate::strip_sigil(span_text(src, param.name));
                    push_attributes(&mut out, &param.attributes, method, name, src);
                }
            }
            _ => {}
        }
    }
    out
}

/// Appends one attach site's attributes to `out`, flattening the groups: two
/// `#[A] #[B]` lines and one `#[A, B]` attach the same two attributes to the
/// same declaration, so a roster that kept the grouping would be reporting
/// the source's line breaks.
fn push_attributes(
    out: &mut Vec<ClassAttribute>,
    groups: &[nvs_syntax::ast::AttributeGroup],
    member: &str,
    parameter: &str,
    src: &SourceFile,
) {
    for group in groups {
        for attr in &group.attributes {
            out.push(ClassAttribute {
                member: member.to_owned(),
                parameter: parameter.to_owned(),
                name: attr
                    .name
                    .as_ref()
                    .map_or_else(String::new, |n| span_text(src, n.span).to_owned()),
                fields: attr
                    .fields
                    .iter()
                    .map(|field| {
                        (
                            span_text(src, field.name).to_owned(),
                            crate::consts::fold_expr(&field.value, src),
                        )
                    })
                    .collect(),
            });
        }
    }
}

/// Appends every class constant `qname` answers to `constants` — its own
/// first, then its superclass chain's, then any interface's.
///
/// [`flatten_methods`]' walk keyed on the name alone, which is what a
/// constant's own override rule is: a class redeclaring an inherited name
/// answers with its own value, and `Foo::BAR` in source resolves the same way
/// (`crate::consts::ConstTable::get`'s ancestor walk, which this mirrors so
/// the two never disagree about which declaration won).
fn flatten_constants(
    qname: &QName,
    graph: &ClassGraph,
    own: &FxHashMap<QName, Vec<ClassConstant>>,
    constants: &mut Vec<ClassConstant>,
    walked: &mut Vec<QName>,
) {
    if walked.contains(qname) {
        return;
    }
    walked.push(qname.clone());
    if let Some(declared) = own.get(qname) {
        for constant in declared {
            if !constants.iter().any(|have| have.name == constant.name) {
                constants.push(constant.clone());
            }
        }
    }
    let Some(links) = graph.get(qname) else {
        return;
    };
    for parent in links.extends.iter().chain(links.implements.iter()) {
        flatten_constants(parent, graph, own, constants, walked);
    }
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

/// Whether `modifiers` make the member they decorate `protected` — the second
/// of the two bits [`ClassLayout::protected_fields`] and [`ClassLayout::methods`]
/// carry, read from the keyword rather than derived from [`is_public`]'s
/// answer: `private` and `protected` are both "not public", and only one of
/// them is reached from a subclass's own bodies.
fn is_protected(modifiers: &[Modifier]) -> bool {
    modifiers.contains(&Modifier::Protected)
}

/// One declaration's own instance-property names, in declaration order, each
/// with the two bits that spell its visibility and the type it declares — a
/// written `public int $n;` and a promoted constructor parameter alike, each
/// where it stands among the members.
///
/// The visibility bits are [`own_methods`]' bits, read the same way by
/// [`is_public`] and [`is_protected`]: `rule:security/reflection-enforces-visibility`'s reflective read has to face the check ordinary
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
) -> Vec<(String, bool, bool, String)> {
    members
        .iter()
        .flat_map(|member| match &member.kind {
            ClassMemberKind::Property(p) if !is_static(p) => {
                vec![(
                    crate::strip_sigil(span_text(src, p.name)).to_owned(),
                    is_public(&p.modifiers),
                    is_protected(&p.modifiers),
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
                        is_protected(&p.modifiers),
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
    own: &FxHashMap<QName, Vec<(String, bool, bool, String)>>,
    fields: &mut Vec<(String, bool, bool, String)>,
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
            if !fields.iter().any(|(held, _, _, _)| *held == slot.0) {
                fields.push(slot.clone());
            }
        }
    }
}

/// Appends every method `qname` answers to `methods`, as `(name, declaring
/// class label, is `public`, is `protected`, parameter names)` — its own
/// first, then its
/// superclass chain's, then any interface default it inherits. The first entry
/// for a name wins, which is what makes an override beat the declaration it
/// overrides, and it carries that declaration's own visibility and parameter
/// spellings with it.
///
/// `walked` guards the cyclic `extends` the hierarchy pass has already
/// diagnosed, exactly like [`flatten_fields`]' own `seen`.
fn flatten_methods(
    qname: &QName,
    graph: &ClassGraph,
    own: &OwnMethods,
    methods: &mut Vec<MethodEntry>,
    walked: &mut Vec<QName>,
) {
    if walked.contains(qname) {
        return;
    }
    walked.push(qname.clone());
    let label = qname.to_string();
    if let Some(names) = own.get(qname) {
        for (name, public, protected, params, param_types) in names {
            if !methods.iter().any(|(have, _, _, _, _, _)| have == name) {
                methods.push((
                    name.clone(),
                    label.clone(),
                    *public,
                    *protected,
                    params.clone(),
                    param_types.clone(),
                ));
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
    /// `nvs-codegen` drops exactly those edges, leaving `$m is Stringable`
    /// with nothing to test against.
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
