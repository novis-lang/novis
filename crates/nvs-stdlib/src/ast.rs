//! `Core\Ast` — `rule:core-classes/ast-is-inert`'s door onto the compiler's own parser, and the inert tree it answers
//! with.
//!
//! # Decision: one grammar, reached rather than reimplemented
//!
//! [`nvs_core_ast_parse`] calls `nvs_syntax::parse_file` — through
//! [`nvs_syntax::walk::of_source`], which is the same call plus the walk — so
//! a construct that parses when `nvs` compiles a file parses identically when
//! a running program describes the same text, and a construct the parser
//! refuses is refused identically in both places. § 3 states that as a rule
//! and this module is the whole of keeping it: there is no second grammar
//! here, no tolerant re-lexing, and no arm that repairs input the compiler
//! would reject (`rule:errors/ambiguous-input-refused`).
//!
//! That is also why source with an error is a `ParseError` rather than a tree
//! carrying an `Error` node. The compiler keeps error nodes because it has
//! more passes to run and more diagnostics to collect; a program asking what
//! this text *is* has one question and gets one answer.
//!
//! # Decision: the tree is inert because there is nothing in it to run
//!
//! § 3's "no path from an AST value back into execution" is structural here,
//! not a promise. A [`NODE`] instance holds two ordinary Novis values — a kind
//! string and an array of more nodes — and `nvs_syntax::walk::Node`, the only
//! thing that crosses out of `nvs-syntax`, holds no `Expr`, no `Span` and no
//! borrow of the source. So there is no handle for a later member to accept
//! and no descriptor for one to look up: `eval` stays absent by having nothing
//! to be spelled with, which is the same argument
//! `rule:security/closed-doors` makes for the other
//! three doors.
//!
//! The cost is that a node cannot answer its own source text, which a
//! pretty-printer wants — known gap 2. That is a slot away, and adding it is
//! the point at which `parse`'s `$source` stops being
//! [`Qual::Neutral`](crate::registry::Qual::Neutral).
//!
//! **What it spends:** one object per node in the parsed file, plus one array
//! per node with children, charged to the request that called `parse` and
//! released with the tree. A source file is bounded by the caller's own
//! memory ceiling and the parser's 96-level nesting limit bounds the depth,
//! so both this walk and [`nvs_core_ast_node_nodes`]'s recurse on a bounded
//! stack.
//!
//! # Known gaps
//!
//! 1. § 3's typed roster — `Core\Ast\ClassDecl`, `Core\Ast\MethodDecl`, one
//!    class per production — is not here. Every node is a [`NODE`] whose
//!    `kind()` names its production, which is the shape of the answer rather
//!    than the answer: the roster refines it, and `nvs_syntax::walk`'s own
//!    module doc owns which productions are nodes at all today.
//!    — owner: M8
//! 2. A node carries no position and no text, so a walk can count and classify
//!    but not quote. See the second decision above for what adding it costs.
//!    Closing this is also what makes userland architecture rules real: a
//!    `#[Test]` that walks the tree can fail today, but cannot name the
//!    `file:line` it failed about, and a structural rule that cannot point is
//!    a check rather than a report (docs/adr/tooling-parity.md, the Deptrac
//!    row).
//!    Decided: Position (line/column/offset) only — Architecture tests can point at file:line, and the
//!    input stays qualifier-neutral because no text comes back out.
//!    — owner: unowned-closures
//! 3. § 3's `Core\Ast::parseFile` is not here. It reads a path, so it is a
//!    capability-bearing member (`rule:security/capability-declaration-is-one-table`
//!    's `fs.read`) rather than a second spelling of this one, and the
//!    `Core\IO` door it goes through is where that check already lives.
//!    Decided: Strike the spec roster row; compose IO::read + parse — Class stays capability-free and
//!    the fs.read check stays where it already lives; users write two calls.
//!    — owner: m8-stdlib-depth

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// The class name, as a program writes it.
pub(crate) const NAME: &str = r"Core\Ast";

/// The node class's name, as a program writes it.
pub(crate) const NODE_NAME: &str = r"Core\Ast\Node";

/// [`NODE`]'s slot holding the production's name.
const KIND_SLOT: usize = 0;

/// [`NODE`]'s slot holding the nodes this one directly contains.
const CHILDREN_SLOT: usize = 1;

/// `Core\Ast` — one member, because parsing is one question.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
        name: "parse",
        names: &["source"],
        // Neutral, and not a sink: the answer names productions rather than
        // carrying the argument's content, and nothing executes what it
        // describes (`rule:core-classes/ast-is-inert`). The day known gap 2 lets a node answer its
        // own text is the day this becomes `Qual::Contagious`.
        params: &[CoreTy::Text(Qual::Neutral)],
        defaults: &[],
        return_ty: CoreTy::Instance(NODE_NAME),
        symbol: "nvs_core_ast_parse",
        doc: Some(&PARSE_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Ast::parse`'s reference card — `rule:core-api/reference-card`.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Parses `$source` with the compiler's own parser and answers the file's node tree. \
            Replaces `token_get_all` and every userland parser over it.",
    params: &[ParamDoc {
        name: "source",
        desc: "Novis source, starting outside `<?nvs` the way a file does.",
        shape: &[],
    }],
    ret: "The file's root node — `kind()` is `File`, and its children are the top-level \
          statements. The tree is data: nothing on it runs what it describes.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "`$source` does not parse. The message carries the first error and its line and \
               column, and a source the compiler refuses is refused here identically.",
    }],
};

/// `Core\Ast\Node` — what [`CLASS`]'s member answers with, and what its own
/// two collections are made of.
pub(crate) const NODE: CoreClass = CoreClass {
    name: NODE_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "kind",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_ast_node_kind",
            doc: Some(&KIND_DOC),
        },
        CoreMethod {
            name: "children",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(NODE_NAME)),
            symbol: "nvs_core_ast_node_children",
            doc: Some(&CHILDREN_DOC),
        },
        CoreMethod {
            name: "nodes",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(NODE_NAME)),
            symbol: "nvs_core_ast_node_nodes",
            doc: Some(&NODES_DOC),
        },
    ],
    slots: &["kind", "children"],
    constants: &[],
};

/// `Core\Ast\Node::kind`'s reference card — `rule:core-api/reference-card`.
const KIND_DOC: MethodDoc = MethodDoc {
    short: "Which production this node is, spelled as the grammar spells it — `Binary`, `Echo`, \
            `Method`, `File`.",
    params: &[],
    ret: "The production's name. One vocabulary, the grammar's own, so a walk that recognises a \
          construct recognises it by the name the language documents.",
    errors: &[],
};

/// `Core\Ast\Node::children`'s reference card — `rule:core-api/reference-card`.
const CHILDREN_DOC: MethodDoc = MethodDoc {
    short: "The nodes this one directly contains, in source order.",
    params: &[],
    ret: "The direct children, empty for a leaf such as an `Int`.",
    errors: &[],
};

/// `Core\Ast\Node::nodes`'s reference card — `rule:core-api/reference-card`.
const NODES_DOC: MethodDoc = MethodDoc {
    short: "Every node this one contains, however deeply — `children` closed transitively, which \
            is the whole walk when the receiver is the file.",
    params: &[],
    ret: "The subtree in source order, the receiver excluded: both this and `children` answer \
          what the node *contains*, and a node does not contain itself.",
    errors: &[],
};

/// One [`NODE`] instance per node of the walk, built bottom-up.
///
/// Recursion rather than an explicit stack because the parser's own nesting
/// limit already bounds the depth at 96 levels — see `nvs_syntax::parser`'s
/// depth guard, which is the reason this cannot be handed a tree deep enough
/// to matter.
fn instance_of(node: &nvs_syntax::walk::Node) -> Value {
    let mut children = NvsArray::new();
    for child in &node.children {
        children.append(instance_of(child));
    }
    crate::instance::build(
        &NODE,
        [
            Value::str(NvsStr::new(node.kind.as_bytes())),
            Value::array(children),
        ],
    )
}

/// A slot of the receiving node, retained because it is being answered.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not an object — unreachable from
/// source, since an instance member's receiver is typed and `E0401` refuses a
/// call on anything else.
fn slot_of(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &NODE, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot's reference belongs to the receiver, which is live for \
                  the length of the call, and this value is being handed to the \
                  caller — which is exactly `Value::retain`'s obligation"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

/// Appends every node of `children` and of their own children to `out`, each
/// with a reference of its own.
fn flatten(children: &Value, out: &mut NvsArray) {
    let Some(array) = children.array_ptr() else {
        return;
    };
    let array = crate::arr::borrowed(array);
    let mut from = 0usize;
    while let Some(slot) = array.next_slot(from) {
        let child = array
            .value_at(slot)
            .expect("next_slot only names live entries");
        #[expect(
            unsafe_code,
            reason = "the entry is owned by the receiver's own children array, \
                      which outlives this call, so the copy stored in the answer \
                      needs a reference of its own"
        )]
        unsafe {
            child.retain();
        }
        out.append(child);
        if let Some(receiver) = child.obj_ptr() {
            let grandchildren = crate::instance::slot(receiver, CHILDREN_SLOT);
            flatten(&grandchildren, out);
        }
        from = slot + 1;
    }
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_ast_parse" => (nvs_core_ast_parse as *const ()).cast(),
        "nvs_core_ast_node_kind" => (nvs_core_ast_node_kind as *const ()).cast(),
        "nvs_core_ast_node_children" => (nvs_core_ast_node_children as *const ()).cast(),
        "nvs_core_ast_node_nodes" => (nvs_core_ast_node_nodes as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Ast::parse(string $source): Core\Ast\Node` — `rule:core-classes/ast-is-inert`'s one
    /// grammar, reached at run time.
    ///
    /// The whole body is the call and the shaping: `nvs_syntax` does the
    /// parsing and the walk, and this turns the walk into ordinary Novis
    /// objects. Everything interesting about the member is in those two
    /// modules' doc comments rather than here, which is the point of the
    /// split.
    fn nvs_core_ast_parse(_ctx, args: [1]) {
        // Unreachable from source: `$source` is a `string` parameter, so
        // `E0401` refuses anything else before any of this runs. The guard is
        // what makes the tag read below it sound, the way a `debug_assert!`
        // is.
        let source = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Ast::parse expected {:?}, got tag {}",
                Tag::Str,
                args[0].tag_byte()
            ))
        })?;
        // The name is what a diagnostic would print for this text, and there
        // is no file behind it — the member is the whole of its provenance.
        let tree = nvs_syntax::walk::of_source("Core\\Ast::parse", source)
            .map_err(|message| {
                Fault::thrown_as(
                    ThrownClass::Parse,
                    format!("Core\\Ast::parse(): {message}"),
                )
            })?;
        Ok(instance_of(&tree))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Ast\Node::kind(): string` — the production's name.
    fn nvs_core_ast_node_kind(_ctx, args: [1]) {
        slot_of(args, KIND_SLOT, "kind")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Ast\Node::children(): array<Core\Ast\Node>` — the slot, as it was
    /// built.
    fn nvs_core_ast_node_children(_ctx, args: [1]) {
        slot_of(args, CHILDREN_SLOT, "children")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Ast\Node::nodes(): array<Core\Ast\Node>` — the subtree, the
    /// receiver excluded.
    ///
    /// Built by walking the instances rather than kept as a third slot: a slot
    /// would hold one reference per descendant on every node, which is
    /// O(nodes²) references for a tree whose whole point is that it is already
    /// linked. The walk is O(nodes) per call and allocates one array.
    fn nvs_core_ast_node_nodes(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &NODE, "nodes")?;
        let children = crate::instance::slot(receiver, CHILDREN_SLOT);
        let mut out = NvsArray::new();
        flatten(&children, &mut out);
        Ok(Value::array(out))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsStr, OutputSink, Tag, Value, call};

    use super::{CHILDREN_SLOT, KIND_SLOT, NAME, NODE, NODE_NAME};
    use crate::registry::{CLASSES, CoreTy};

    /// Whether a signature's type mentions `class` anywhere inside it.
    ///
    /// Exhaustive on purpose, with the leaves grouped rather than swept up by
    /// a `_`: this backs an assertion about an *absence*, and a composite
    /// variant added later must be a build error here rather than a hole the
    /// sweep walks straight past.
    fn mentions(ty: &CoreTy, class: &str) -> bool {
        match ty {
            CoreTy::Instance(name)
            | CoreTy::ShapeOfCallables(name)
            | CoreTy::Written(name)
            | CoreTy::Enum(name)
            | CoreTy::EnumCase(name, _)
            | CoreTy::Var(name) => *name == class,
            CoreTy::Array(inner)
            | CoreTy::Nullable(inner)
            | CoreTy::Iterated(inner)
            | CoreTy::Variadic(inner) => mentions(inner, class),
            CoreTy::InstanceAt(name, args) => {
                *name == class || args.iter().any(|arg| mentions(arg, class))
            }
            CoreTy::Union(members) => members.iter().any(|member| mentions(member, class)),
            CoreTy::CallableSig(params, ret) => {
                params.iter().any(|param| mentions(param, class)) || mentions(ret, class)
            }
            CoreTy::Options(options) => options.iter().any(|option| mentions(&option.ty, class)),
            CoreTy::Shape(arms) => arms
                .iter()
                .any(|arm| arm.iter().any(|field| mentions(&field.ty, class))),
            CoreTy::Bool
            | CoreTy::Int
            | CoreTy::Uint
            | CoreTy::Float
            | CoreTy::Decimal
            | CoreTy::Str
            | CoreTy::Bytes
            | CoreTy::Text(_)
            | CoreTy::Blob(_)
            | CoreTy::SecretBytes
            | CoreTy::SecretBlob(_)
            | CoreTy::SecretStr
            | CoreTy::SecretText(_)
            | CoreTy::TaintedStr
            | CoreTy::TaintedBytes
            | CoreTy::SecretTaintedStr
            | CoreTy::SecretTaintedBytes
            | CoreTy::Void
            | CoreTy::Mixed
            | CoreTy::Object
            | CoreTy::Callable
            | CoreTy::Entry
            | CoreTy::IntLiteral(_) => false,
        }
    }

    /// `rule:core-classes/ast-is-inert`'s second rule, which is the one that has to be *shown*
    /// rather than stated: a program can walk a parsed tree, and there is
    /// nothing on it or around it that turns the tree back into behaviour.
    ///
    /// Three properties, none of which is an assertion about one answer:
    ///
    /// 1. **No member anywhere in `Core` accepts a node.** The sweep is over
    ///    the whole registry rather than over this class, because a door that
    ///    ran a tree would not be declared here — it would be declared next
    ///    to whatever ran it.
    /// 2. **A walk of the tree reaches nothing but the tree.** Every member a
    ///    node has answers a string or more nodes, so no amount of walking
    ///    produces a value of another class, and there is no `Core\Ast`
    ///    handle a later member could take.
    /// 3. **The values are ordinary data at run time too**, over a real
    ///    parse: every slot is a `Tag::Str` or a `Tag::Array` of objects, and
    ///    no closure, callable or resource is anywhere in it.
    #[test]
    fn a_parsed_ast_is_inert_data_with_no_path_back_into_execution() {
        for class in CLASSES {
            for member in class.members() {
                for param in member.params {
                    assert!(
                        !mentions(param, NODE_NAME) && !mentions(param, NAME),
                        "{}::{} takes a parsed tree, which would be the path back \
                         into execution `rule:core-classes/ast-is-inert` closes",
                        class.name,
                        member.name
                    );
                }
            }
        }

        for member in NODE.members() {
            let answers_itself = mentions(&member.return_ty, NODE_NAME);
            let answers_text = matches!(member.return_ty, CoreTy::Str);
            assert!(
                answers_itself || answers_text,
                "{NODE_NAME}::{} answers something that is neither the tree nor \
                 text, so walking the tree reaches outside it",
                member.name
            );
        }

        let source = Value::str(NvsStr::new(
            b"<?nvs class C { public function m(): void {} }",
        ));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_ast_parse, &mut ctx, &[source]).expect("that source parses");
        let mut seen = 0usize;
        assert_data(tree, &mut seen);
        // File, ClassDecl, Method — the walk reached every one of them, so the
        // property above was checked over a tree and not over its root.
        assert_eq!(seen, 3, "every node of the parsed file was inspected");

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference `parse` answered with, and \
                      the tree's own references are the nodes' own"
        )]
        unsafe {
            tree.release();
            source.release();
        }
    }

    /// Asserts that `node` is a [`NODE`] holding nothing but data, and
    /// recurses into its children, counting what it inspected.
    fn assert_data(node: Value, seen: &mut usize) {
        *seen += 1;
        let receiver = node.obj_ptr().expect("a node is an object");
        let kind = crate::instance::slot(receiver, KIND_SLOT);
        assert_eq!(
            kind.tag(),
            Some(Tag::Str),
            "a node's kind is text — the grammar's name for the production"
        );
        let children = crate::instance::slot(receiver, CHILDREN_SLOT);
        assert_eq!(
            children.tag(),
            Some(Tag::Array),
            "a node's children are an array"
        );
        let array = children.array_ptr().expect("the slot is an array");
        let array = crate::arr::borrowed(array);
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let child = array
                .value_at(slot)
                .expect("next_slot only names live entries");
            assert_data(child, seen);
            from = slot + 1;
        }
    }
}
