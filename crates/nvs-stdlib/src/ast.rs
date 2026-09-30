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
//! **There is one member, and parsing a *file* is it composed with
//! `Core\IO::read`.** A path is the filesystem's question, and a second member
//! asking it would put a `fs.read` check somewhere other than the door that
//! already owns one — leaving this class holding a capability for the sake of
//! one spelling, when its whole safety argument
//! (`rule:security/reflection-needs-no-capability`) is that it needs none. The
//! rule is the home of that trade;
//! `tests/conformance/core/ast-parse-file-reads-through-the-io-door-under-fs-read.nvst`
//! is the composition written out.
//!
//! That is also why source with an error is a `ParseError` rather than a tree
//! carrying an `Error` node. The compiler keeps error nodes because it has
//! more passes to run and more diagnostics to collect; a program asking what
//! this text *is* has one question and gets one answer.
//!
//! # Decision: a production is a class, and the class is the node's identity
//!
//! `rule:core-classes/ast-is-inert` asks for one type per production, and
//! [`PRODUCTIONS`] is it: one [`CoreClass`] per entry of
//! [`nvs_syntax::walk::KINDS`], at the same index, so [`class_of`] finds a
//! node's class by a binary search rather than by a second copy of the
//! grammar's vocabulary. A parsed node is an instance of *its production's*
//! class — a `Core\Ast\Binary`, a `Core\Ast\ClassDecl` — and
//! `Core\Reflect::forObject($node)->name()` is where a program reads that
//! back.
//!
//! **They carry no registry row, and that is the decision rather than an
//! omission.** The registry is the whole roster of names a program may write,
//! so a production stays off it and no source can name `Core\Ast\Binary` in a
//! type position — a row would freeze every production's name as language
//! surface, which costs the simplicity AGENTS.md's ordering puts fourth and
//! buys nothing at any level above it. The descriptor exists all the same,
//! chained into `crate::instance`'s table beside the registered classes, which
//! is what makes a parsed node an instance of its own production. What a
//! program branches on is [`NODE`]'s `kind()`, which is the same production
//! spelled short; what the class adds is the identity behind it, which is the
//! half `kind()` cannot be wrong about.
//!
//! So [`NODE`] is the *written* type — what `parse`, `children` and `nodes`
//! declare — and a production class is the *runtime* one. Every one of them
//! declares [`NODE`]'s own slots, so a member body reads slot by index and
//! never asks which class the receiver is.
//!
//! # Decision: the tree is inert because there is nothing in it to run
//!
//! § 3's "no path from an AST value back into execution" is structural here,
//! not a promise. A [`NODE`] instance holds ordinary Novis values — a kind
//! string, an array of more nodes, and the three integers saying where in the
//! source it starts — and `nvs_syntax::walk::Node`, the only thing that
//! crosses out of `nvs-syntax`, holds no `Expr` and no borrow of the source:
//! its span is a pair of offsets into text this crate never keeps. So there is
//! no handle for a later member to accept and no descriptor for one to look
//! up: `eval` stays absent by having nothing to be spelled with, which is the
//! same argument `rule:security/closed-doors` makes for the other three doors.
//!
//! **A node answers where it is and never what it says.** `line()`, `column()`
//! and `offset()` are what a `#[Test]` walking the tree names a `file:line`
//! with — a structural rule that cannot point is a check rather than a report
//! — and they carry none of the source's own bytes, which is why `parse`'s
//! `$source` stays [`Qual::Neutral`](crate::registry::Qual::Neutral). A member
//! answering a node's own text is what would end that, and there is none.
//!
//! **What it spends:** one object per node in the parsed file — five slots, of
//! which the kind string and the children array are the two that allocate —
//! plus one array per node with children, charged to the request that called
//! `parse` and released with the tree. The line and column are read at parse
//! time from the parsed file's own line table, which `parse` holds for the
//! length of the call and drops with it (`nvs_syntax::walk::Located`). A
//! source file is bounded by the caller's own memory ceiling and the parser's
//! 96-level nesting limit bounds the depth, so both this walk and
//! [`nvs_core_ast_node_nodes`]'s recurse on a bounded stack.
//!
//! # Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-stdlib/src/ast.rs` lists them.

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, as a program writes it.
pub(crate) const NAME: &str = r"Core\Ast";

/// The node class's name, as a program writes it.
pub(crate) const NODE_NAME: &str = r"Core\Ast\Node";

/// [`NODE`]'s slot holding the production's name.
const KIND_SLOT: usize = 0;

/// [`NODE`]'s slot holding the nodes this one directly contains.
const CHILDREN_SLOT: usize = 1;

/// [`NODE`]'s slot holding the 1-based line the production starts on.
const LINE_SLOT: usize = 2;

/// [`NODE`]'s slot holding the 1-based column the production starts at,
/// counted in characters.
const COLUMN_SLOT: usize = 3;

/// [`NODE`]'s slot holding the byte offset the production starts at.
const OFFSET_SLOT: usize = 4;

/// `Core\Ast`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads Novis source code and gives its structure as a tree of nodes. It uses the same \
            parser as the compiler. Nothing in the tree runs.",
};

/// `Core\Ast` — one member, because parsing is one question.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[CoreMethod {
        name: "parse",
        names: &["source"],
        // Neutral, and not a sink: the answer names productions and says where
        // each one starts, never carrying the argument's content, and nothing
        // executes what it describes (`rule:core-classes/ast-is-inert`). A
        // member answering a node's own text is what would make this
        // `Qual::Contagious`, and the module doc says why there is none.
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

/// The slots a node holds, shared by [`NODE`] and by every [`PRODUCTIONS`]
/// entry so a member body reads by index and never asks which class its
/// receiver is.
const NODE_SLOTS: &[&str] = &["kind", "children", "line", "column", "offset"];

/// `Core\Ast\Node`'s class card — `rule:core-api/reference-card`.
const NODE_CARD: ClassDoc = ClassDoc {
    short: "One part of a parsed source file, such as a class, a statement or an expression. A \
            node has its kind, the nodes inside it, and the place where it starts in the source.",
};

/// `Core\Ast\Node` — the type [`CLASS`]'s member and this class's own two
/// collections are declared as, and the members every parsed node answers.
///
/// No instance carries *this* class: a node's runtime class is its
/// production's ([`PRODUCTIONS`]), which the module doc's second decision
/// owns.
pub(crate) const NODE: CoreClass = CoreClass {
    name: NODE_NAME,
    doc: Some(&NODE_CARD),
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
        CoreMethod {
            name: "line",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_ast_node_line",
            doc: Some(&LINE_DOC),
        },
        CoreMethod {
            name: "column",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_ast_node_column",
            doc: Some(&COLUMN_DOC),
        },
        CoreMethod {
            name: "offset",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_ast_node_offset",
            doc: Some(&OFFSET_DOC),
        },
    ],
    slots: NODE_SLOTS,
    constants: &[],
};

/// One class per production, in [`nvs_syntax::walk::KINDS`]'s order.
///
/// `concat!` builds each name at compile time, so a production is written here
/// once and as the production — and
/// `every_production_the_walk_names_has_a_typed_ast_class` is what holds this
/// list to that table, index by index.
macro_rules! productions {
    ($($kind:literal),* $(,)?) => {
        /// The typed roster `rule:core-classes/ast-is-inert` asks for: one
        /// class per production of the grammar, parallel to
        /// [`nvs_syntax::walk::KINDS`].
        ///
        /// Each declares [`NODE`]'s slots and no members of its own — the
        /// module doc's second decision says why a production is an identity
        /// rather than a second surface.
        pub(crate) const PRODUCTIONS: &[CoreClass] = &[
            $(CoreClass {
                name: concat!(r"Core\Ast\", $kind),
                doc: None,
                methods: &[],
                instance: &[],
                slots: NODE_SLOTS,
                constants: &[],
            }),*
        ];
    };
}

productions![
    "ArrayLiteral",
    "Assign",
    "AutoloadDecl",
    "Await",
    "Binary",
    "Block",
    "Bool",
    "Break",
    "Call",
    "Catch",
    "ClassConstAccess",
    "ClassDecl",
    "ClassNameConst",
    "Clone",
    "Const",
    "ConstFetch",
    "Continue",
    "Conversion",
    "Destructure",
    "DoWhile",
    "Duration",
    "Echo",
    "Empty",
    "EnumCase",
    "EnumDecl",
    "Error",
    "Exit",
    "Expr",
    "File",
    "Float",
    "Fn",
    "For",
    "Foreach",
    "Function",
    "Global",
    "Goto",
    "If",
    "Index",
    "InlineHtml",
    "Int",
    "InterfaceDecl",
    "Interpolated",
    "Isset",
    "LocalDecl",
    "Markup",
    "Match",
    "Method",
    "MethodCall",
    "NamespaceDecl",
    "New",
    "Null",
    "ObjectLiteral",
    "Paren",
    "ParentExpr",
    "PostIncDec",
    "PreIncDec",
    "Print",
    "Property",
    "PropertyAccess",
    "Require",
    "Return",
    "SelfExpr",
    "SpawnScript",
    "StaticCall",
    "StaticExpr",
    "StaticLocal",
    "StaticPropertyAccess",
    "Str",
    "Switch",
    "Ternary",
    "Throw",
    "Try",
    "TypeAliasDecl",
    "TypeTest",
    "Unary",
    "Unset",
    "UseDecl",
    "Variable",
    "While",
    "Yield",
    "YieldFrom",
];

/// The class a node of production `kind` is an instance of.
///
/// A binary search over [`nvs_syntax::walk::KINDS`], which is sorted and
/// index-parallel to [`PRODUCTIONS`] — so this is the grammar's own table
/// answering, and nothing here is a second list of production names.
///
/// # Panics
///
/// Panics naming the production if the walk answered with one the roster has
/// no class for. Both halves are checked against each other by
/// `every_production_the_walk_names_has_a_typed_ast_class`, so reaching this
/// is a build-time oversight rather than anything a program can cause.
fn class_of(kind: &str) -> &'static CoreClass {
    let index = nvs_syntax::walk::KINDS
        .binary_search(&kind)
        .unwrap_or_else(|_| panic!("the walk answered with the unrostered production `{kind}`"));
    &PRODUCTIONS[index]
}

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

/// `Core\Ast\Node::line`'s reference card — `rule:core-api/reference-card`.
const LINE_DOC: MethodDoc = MethodDoc {
    short: "The 1-based line this production starts on, so a rule that walks the tree can report a \
            `file:line` rather than only a verdict.",
    params: &[],
    ret: "The line of the node's first character, counting the file's first line as 1.",
    errors: &[],
};

/// `Core\Ast\Node::column`'s reference card — `rule:core-api/reference-card`.
const COLUMN_DOC: MethodDoc = MethodDoc {
    short: "The 1-based column this production starts at, counted in characters rather than bytes.",
    params: &[],
    ret: "The column of the node's first character, counting the line's first character as 1. \
          Characters, so a line holding a `ß` before the node still points at it.",
    errors: &[],
};

/// `Core\Ast\Node::offset`'s reference card — `rule:core-api/reference-card`.
const OFFSET_DOC: MethodDoc = MethodDoc {
    short: "The byte offset this production starts at, from the beginning of the parsed source.",
    params: &[],
    ret: "The 0-based offset into the string `parse` was given, which is what a caller holding \
          that string slices with. The node never answers the slice itself.",
    errors: &[],
};

/// One instance per node of the walk, built bottom-up, each of its own
/// production's class.
///
/// Recursion rather than an explicit stack because the parser's own nesting
/// limit already bounds the depth at 96 levels — see `nvs_syntax::parser`'s
/// depth guard, which is the reason this cannot be handed a tree deep enough
/// to matter.
fn instance_of(node: &nvs_syntax::walk::Node, parsed: &nvs_syntax::walk::Located) -> Value {
    let mut children = NvsArray::new();
    for child in &node.children {
        children.append(instance_of(child, parsed));
    }
    let (line, column, offset) = parsed.position(node);
    crate::instance::build(
        class_of(node.kind),
        [
            Value::str(NvsStr::new(node.kind.as_bytes())),
            Value::array(children),
            Value::int(i64::from(line)),
            Value::int(i64::from(column)),
            Value::int(i64::from(offset)),
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
        "nvs_core_ast_node_line" => (nvs_core_ast_node_line as *const ()).cast(),
        "nvs_core_ast_node_column" => (nvs_core_ast_node_column as *const ()).cast(),
        "nvs_core_ast_node_offset" => (nvs_core_ast_node_offset as *const ()).cast(),
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
        let parsed = nvs_syntax::walk::located("Core\\Ast::parse", source)
            .map_err(|message| {
                Fault::thrown_as(
                    ThrownClass::Parse,
                    format!("Core\\Ast::parse(): {message}"),
                )
            })?;
        Ok(instance_of(&parsed.tree, &parsed))
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

nvs_runtime::nvs_helper! {
    /// `Core\Ast\Node::line(): int` — the slot, resolved when the tree was
    /// built.
    ///
    /// The three position members read slots rather than computing anything:
    /// the line table belongs to the parse, which is over by the time a program
    /// holds the node.
    fn nvs_core_ast_node_line(_ctx, args: [1]) {
        slot_of(args, LINE_SLOT, "line")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Ast\Node::column(): int` — the slot, in characters.
    fn nvs_core_ast_node_column(_ctx, args: [1]) {
        slot_of(args, COLUMN_SLOT, "column")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Ast\Node::offset(): int` — the slot, in bytes.
    fn nvs_core_ast_node_offset(_ctx, args: [1]) {
        slot_of(args, OFFSET_SLOT, "offset")
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostic, Diagnostics, PositionEncoding, SourceFile, SourceMap};
    use nvs_runtime::{Ctx, NvsArray, NvsObj, NvsStr, OutputSink, THROWN, Tag, Value, call};

    use super::{
        CHILDREN_SLOT, COLUMN_SLOT, KIND_SLOT, LINE_SLOT, NAME, NODE, NODE_NAME, OFFSET_SLOT,
        PRODUCTIONS, class_of,
    };
    use crate::registry::{CLASSES, CoreTy};

    /// The namespace every production's class name carries, which is the one
    /// [`NODE`] itself sits in.
    const PRODUCTION_PREFIX: &str = r"Core\Ast\";

    /// `rule:core-classes/ast-is-inert`'s typed roster is the grammar's, whole:
    /// every production `nvs_syntax::walk` names has a class of its own, at the
    /// index the walk's own table gives it.
    ///
    /// The index is what [`class_of`] searches, so this is not a spelling check
    /// — a roster one entry short or one out of order builds fine and answers
    /// the wrong class for every production after the hole.
    #[test]
    fn every_production_the_walk_names_has_a_typed_ast_class() {
        let kinds = nvs_syntax::walk::KINDS;
        assert_eq!(
            PRODUCTIONS.len(),
            kinds.len(),
            "one class per production, and no class for a production the walk \
             never answers with"
        );
        for (kind, class) in kinds.iter().zip(PRODUCTIONS) {
            assert_eq!(
                class.name,
                format!("{PRODUCTION_PREFIX}{kind}"),
                "the roster and `nvs_syntax::walk::KINDS` have gone apart at `{kind}`"
            );
            assert_eq!(
                class_of(kind).name,
                class.name,
                "`class_of` found `{kind}` at another index"
            );
            assert_eq!(
                class.slots, NODE.slots,
                "{} holds a node's slots, which is what every member body reads \
                 by index",
                class.name
            );
            assert!(
                !CLASSES.iter().any(|row| row.name == class.name),
                "{} is registry surface, which the module doc's second decision \
                 refuses: the registry is the roster of names a program may \
                 write, and a production is identity rather than a name source \
                 spells",
                class.name
            );
        }
    }

    /// `rule:php-migration/one-type-test` read off the roster: one production
    /// answers for a type test, and there is no second node beside it for the
    /// class case.
    ///
    /// `$x is T` and `$x is $cls` are the same production — the class
    /// reference is the type test's second child in `nvs_syntax::walk` — so a
    /// class-test node would be one no parse could ever answer with, and a
    /// program walking the tree would branch on two kinds where the grammar
    /// has one.
    #[test]
    fn the_ast_roster_names_a_type_test_and_no_second_node_for_a_class_test() {
        assert_eq!(
            class_of("TypeTest").name,
            r"Core\Ast\TypeTest",
            "the type test's own production is what `is` parses to"
        );
        let tested: Vec<&str> = PRODUCTIONS
            .iter()
            .map(|class| class.name)
            .filter(|name| {
                let production = name.trim_start_matches(PRODUCTION_PREFIX);
                production.contains("TypeTest") || production.contains("ClassTest")
            })
            .collect();
        assert_eq!(
            tested,
            [r"Core\Ast\TypeTest"],
            "the roster names one node for both spellings of the one type test"
        );
    }

    /// A parsed node is an instance of its production's class, and the walk
    /// below the root is too.
    #[test]
    fn a_parsed_node_carries_its_productions_own_class() {
        let source = Value::str(NvsStr::new(b"<?nvs echo 1 + 2;"));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_ast_parse, &mut ctx, &[source]).expect("that source parses");
        let mut seen: Vec<String> = Vec::new();
        collect_classes(tree, &mut seen);
        assert_eq!(
            seen,
            [
                r"Core\Ast\File",
                r"Core\Ast\Echo",
                r"Core\Ast\Binary",
                r"Core\Ast\Int",
                r"Core\Ast\Int",
            ],
            "every node of the tree names its own production"
        );

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

    /// Appends `node`'s class name and its children's, in source order.
    fn collect_classes(node: Value, out: &mut Vec<String>) {
        let receiver = node.obj_ptr().expect("a node is an object");
        #[expect(
            unsafe_code,
            reason = "the tree under test owns this node's reference for the \
                      length of the call, and the descriptor is owned by this \
                      crate's leaked table, which outlives every instance"
        )]
        let name = unsafe {
            let object = std::mem::ManuallyDrop::new(NvsObj::from_raw(receiver));
            (*object.class()).name().to_owned()
        };
        out.push(name);
        let children = crate::instance::slot(receiver, CHILDREN_SLOT);
        let Some(array) = children.array_ptr() else {
            return;
        };
        let array = crate::arr::borrowed(array);
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let child = array
                .value_at(slot)
                .expect("next_slot only names live entries");
            collect_classes(child, out);
            from = slot + 1;
        }
    }

    /// `children` answers the node's own array with a reference of its own,
    /// rather than the bare slot.
    ///
    /// Only reachable from Rust: a program cannot count references, so a member
    /// answering the slot without retaining it reads correctly in every
    /// conformance case and frees the array under whoever asked next. The
    /// answer is also the node's own array rather than a copy, which is what
    /// makes the walk O(nodes) instead of O(nodes²).
    // covers: Core\Ast\Node::children
    #[test]
    fn children_answers_the_nodes_own_array_with_a_reference_of_its_own() {
        let source = Value::str(NvsStr::new(b"<?nvs echo 1 + 2;"));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_ast_parse, &mut ctx, &[source]).expect("that source parses");
        let receiver = tree.obj_ptr().expect("a node is an object");
        let array = crate::instance::slot(receiver, CHILDREN_SLOT)
            .array_ptr()
            .expect("the children slot is an array");

        #[expect(
            unsafe_code,
            reason = "the array is the live tree's own children slot, which this \
                      test holds the only reference to"
        )]
        let before = unsafe { NvsArray::refcount_of(array) };
        let answer = call(super::nvs_core_ast_node_children, &mut ctx, &[tree])
            .expect("a node answers its children");
        assert_eq!(
            answer.array_ptr(),
            Some(array),
            "the answer is the node's own array and not a copy of it"
        );
        #[expect(
            unsafe_code,
            reason = "the array is still the live tree's, and the answer is a \
                      second reference to it"
        )]
        let held = unsafe { NvsArray::refcount_of(array) };
        assert_eq!(
            held,
            before + 1,
            "the caller owns the answer, so `children` retained before it \
             answered"
        );

        #[expect(
            unsafe_code,
            reason = "this test owns the answer's reference and the one `parse` \
                      gave it, and releases each exactly once"
        )]
        unsafe {
            answer.release();
            assert_eq!(
                NvsArray::refcount_of(array),
                before,
                "releasing the answer gives back exactly the one reference it \
                 took, so a program walking a tree neither leaks nor frees it \
                 early"
            );
            tree.release();
            source.release();
        }
    }

    /// A node's column is `nvs_diagnostics`' own column for that node's offset,
    /// counted in `char`s and made 1-based.
    ///
    /// `rule:ide/positions-have-one-home` from the side a program cannot reach:
    /// the member answers a slot, and the arithmetic that filled the slot lives
    /// in another crate. The source puts a two-byte `ä` before the nodes asked
    /// about, so a column counted in bytes and one counted in characters
    /// disagree from there on and this cannot pass by counting the wrong thing.
    // covers: Core\Ast\Node::column
    #[test]
    fn a_nodes_column_is_the_one_homes_column_for_that_nodes_own_offset() {
        let text = "<?nvs\necho \"ä\", 7 + 1;\n";
        let source = Value::str(NvsStr::new(text.as_bytes()));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_ast_parse, &mut ctx, &[source]).expect("that source parses");

        let mut map = SourceMap::new();
        let id = map.add("a-node's-column", text);

        // Down the first child of each node in turn, so the deepest node asked
        // about sits past the `ä` rather than only at column 1 with it.
        let mut node = tree;
        let mut seen = 0usize;
        let mut widest = 0i64;
        loop {
            widest = widest.max(column_agrees(node, map.file(id), &mut ctx));
            seen += 1;
            let children =
                crate::instance::slot(node.obj_ptr().expect("a node is an object"), CHILDREN_SLOT);
            let Some(array) = children.array_ptr() else {
                break;
            };
            let array = crate::arr::borrowed(array);
            let Some(slot) = array.next_slot(0) else {
                break;
            };
            node = array.value_at(slot).expect("next_slot names a live entry");
        }
        assert!(
            seen >= 3 && widest > 1,
            "the descent asked {seen} node(s) and reached column {widest}, so it \
             stopped before it left the start of the line and asserted nothing \
             about a multi-byte character"
        );

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference `parse` answered with, and \
                      the tree's own references are the nodes'"
        )]
        unsafe {
            tree.release();
            source.release();
        }
    }

    /// Asserts that `node`'s `column` member agrees with `file`'s own column for
    /// `node`'s offset, and answers the column it checked.
    fn column_agrees(node: Value, file: &SourceFile, ctx: &mut Ctx) -> i64 {
        let answered = call(super::nvs_core_ast_node_column, ctx, &[node])
            .expect("a node answers its column")
            .as_int()
            .expect("a column is a whole number");
        let offset =
            crate::instance::slot(node.obj_ptr().expect("a node is an object"), OFFSET_SLOT)
                .as_int()
                .expect("an offset is a whole number");
        let (_, col) = file.line_col(u32::try_from(offset).expect("an offset is a byte position"));
        assert_eq!(
            answered,
            i64::try_from(col).expect("a column fits") + 1,
            "the column a node answers is the one home's column for its own \
             offset, made 1-based"
        );
        answered
    }

    /// Every kind a parsed tree answers is a name `nvs_syntax::walk::KINDS`
    /// holds, and one walk reaches several of them.
    ///
    /// Only reachable from Rust: that table is the grammar's own vocabulary and
    /// no program can name it, so a conformance case can compare `kind()` only
    /// against names it spells out itself. This asserts the closed set instead,
    /// which is the half the module doc's second decision rests on — a member
    /// answering a spelling the walk does not have, or a production reaching
    /// the tree without an entry there, fails on the node carrying it. The two
    /// counts are what stop a descent that never left the root from passing.
    // covers: Core\Ast\Node::kind
    #[test]
    fn every_kind_a_node_answers_is_one_of_the_walks_own_names() {
        let source = Value::str(NvsStr::new(
            b"<?nvs\nint $count = 2;\nif ($count > 1) { foreach ([1, 2] as int $n) { echo $n; } }\n",
        ));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_ast_parse, &mut ctx, &[source]).expect("that source parses");

        let mut seen = std::collections::BTreeSet::new();
        let mut asked = 0usize;
        kinds_are_the_walks_own(tree, &mut ctx, &mut seen, &mut asked);
        assert!(
            asked >= 10 && seen.len() >= 5,
            "the descent asked {asked} node(s) and saw {} distinct kind(s), so it \
             stopped near the root and asserted nothing about the roster",
            seen.len()
        );

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference `parse` answered with, and \
                      the tree's own references are the nodes'"
        )]
        unsafe {
            tree.release();
            source.release();
        }
    }

    /// Asserts that `node`'s `kind` member answers a name the walk itself names,
    /// records it and counts the node, then recurses into its children.
    fn kinds_are_the_walks_own(
        node: Value,
        ctx: &mut Ctx,
        seen: &mut std::collections::BTreeSet<String>,
        asked: &mut usize,
    ) {
        let answered =
            call(super::nvs_core_ast_node_kind, ctx, &[node]).expect("a node answers its kind");
        let name = answered
            .as_text()
            .expect("a kind is text — the grammar's name for the production")
            .to_owned();
        assert!(
            nvs_syntax::walk::KINDS.contains(&name.as_str()),
            "`{name}` is not one of the walk's own production names, so a program \
             branching on the grammar's vocabulary has no arm that could match it"
        );
        assert_eq!(
            class_of(&name).name,
            format!("{PRODUCTION_PREFIX}{name}"),
            "the name `{name}` answers does not find its own production's class"
        );
        *asked += 1;
        seen.insert(name);

        #[expect(
            unsafe_code,
            reason = "`kind` retains the slot before it answers, so this reference \
                      is the caller's to give back"
        )]
        unsafe {
            answered.release();
        }

        let children =
            crate::instance::slot(node.obj_ptr().expect("a node is an object"), CHILDREN_SLOT);
        let Some(array) = children.array_ptr() else {
            return;
        };
        let array = crate::arr::borrowed(array);
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let child = array
                .value_at(slot)
                .expect("next_slot only names live entries");
            kinds_are_the_walks_own(child, ctx, seen, asked);
            from = slot + 1;
        }
    }

    /// `nodes` answers a fresh array carrying a reference of its own to every
    /// node in it, so releasing a walk leaves the tree exactly as it was.
    ///
    /// Only reachable from Rust: a program cannot count references, so a walk
    /// that collected the nodes without retaining them reads correctly in every
    /// conformance case and then frees a subtree under whoever asked next. The
    /// array is a new one rather than the receiver's own children slot, which
    /// is what the member's doc prices as one array per call instead of one
    /// reference per descendant on every node.
    // covers: Core\Ast\Node::nodes
    #[test]
    fn nodes_answers_a_fresh_array_holding_a_reference_to_every_node_in_it() {
        let source = Value::str(NvsStr::new(b"<?nvs if (1) { echo 2; }"));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_ast_parse, &mut ctx, &[source]).expect("that source parses");
        let receiver = tree.obj_ptr().expect("a node is an object");
        let own = crate::instance::slot(receiver, CHILDREN_SLOT)
            .array_ptr()
            .expect("the children slot is an array");
        let branch = {
            let array = crate::arr::borrowed(own);
            let slot = array.next_slot(0).expect("the file holds one statement");
            array
                .value_at(slot)
                .expect("next_slot names a live entry")
                .obj_ptr()
                .expect("a node is an object")
        };
        #[expect(
            unsafe_code,
            reason = "the node is the live tree's own, which this test holds the \
                      only reference to"
        )]
        let before = unsafe { NvsObj::refcount_of(branch) };

        let walk = call(super::nvs_core_ast_node_nodes, &mut ctx, &[tree])
            .expect("a node answers its walk");
        let answered = walk.array_ptr().expect("the walk is an array");
        assert_ne!(
            answered, own,
            "the walk is an array of its own and not the receiver's children \
             slot, so a caller releasing it cannot free the tree's own link"
        );
        assert_eq!(
            entries(answered),
            5,
            "the walk is the subtree in source order — the `if`, its condition, \
             its block, the `echo` and the number — and not one level of it"
        );
        #[expect(
            unsafe_code,
            reason = "the node is still the live tree's, and the walk is a second \
                      reference to it"
        )]
        let held = unsafe { NvsObj::refcount_of(branch) };
        assert_eq!(
            held,
            before + 1,
            "the walk retained every node it collected, so a program holding \
             the array after the tree goes still holds live nodes"
        );

        let again = call(super::nvs_core_ast_node_nodes, &mut ctx, &[tree])
            .expect("a node answers its walk a second time");
        assert_ne!(
            again.array_ptr(),
            Some(answered),
            "each call builds its own array, so walking one node twice hands \
             out two arrays rather than one shared with the caller"
        );

        #[expect(
            unsafe_code,
            reason = "this test owns both walks, the reference `parse` answered \
                      with and the source, and releases each exactly once"
        )]
        unsafe {
            again.release();
            walk.release();
            assert_eq!(
                NvsObj::refcount_of(branch),
                before,
                "releasing the walks gives back exactly the references they \
                 took, so a program walking a tree neither leaks it nor frees \
                 it early"
            );
            tree.release();
            source.release();
        }
    }

    /// How many live entries `array` holds.
    fn entries(array: *mut nvs_runtime::ArrayHeader) -> usize {
        let array = crate::arr::borrowed(array);
        let mut from = 0usize;
        let mut seen = 0usize;
        while let Some(slot) = array.next_slot(from) {
            seen += 1;
            from = slot + 1;
        }
        seen
    }

    /// A node's line is `nvs_diagnostics`' own line for that node's offset,
    /// made 1-based — a newline inside a text value included.
    ///
    /// `rule:ide/positions-have-one-home` from the side a program cannot reach:
    /// the member answers a slot, and the arithmetic that filled the slot lives
    /// in another crate. The source writes one value over three lines, so a
    /// line counted from the statement boundaries rather than from the text
    /// puts everything after that value three lines too high, and the set of
    /// lines reached is what makes the walk say so.
    // covers: Core\Ast\Node::line
    #[test]
    fn a_nodes_line_is_the_one_homes_line_for_that_nodes_own_offset() {
        let text = "<?nvs\necho 1;\necho \"a\nb\nc\";\necho 2 + 3;\n";
        let source = Value::str(NvsStr::new(text.as_bytes()));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_ast_parse, &mut ctx, &[source]).expect("that source parses");

        let mut map = SourceMap::new();
        let id = map.add("a-node's-line", text);

        let mut seen = std::collections::BTreeSet::new();
        line_agrees(tree, map.file(id), &mut ctx, &mut seen);
        assert!(
            seen.len() >= 3 && seen.contains(&6),
            "the walk reached lines {seen:?}, so it never got past the value \
             written over three lines — line 6 is the statement after it, and \
             reaching it is what says the newlines inside a value were counted"
        );

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference `parse` answered with, and \
                      the tree's own references are the nodes'"
        )]
        unsafe {
            tree.release();
            source.release();
        }
    }

    /// Asserts that `node`'s `line` member agrees with `file`'s own line for
    /// `node`'s offset, records the line, and recurses into its children.
    fn line_agrees(
        node: Value,
        file: &SourceFile,
        ctx: &mut Ctx,
        seen: &mut std::collections::BTreeSet<i64>,
    ) {
        let answered = call(super::nvs_core_ast_node_line, ctx, &[node])
            .expect("a node answers its line")
            .as_int()
            .expect("a line is a whole number");
        let receiver = node.obj_ptr().expect("a node is an object");
        let offset = crate::instance::slot(receiver, OFFSET_SLOT)
            .as_int()
            .expect("an offset is a whole number");
        let (line, _) = file.line_col(u32::try_from(offset).expect("an offset is a byte position"));
        assert_eq!(
            answered,
            i64::try_from(line).expect("a line fits") + 1,
            "the line a node answers is the one home's line for its own offset, \
             made 1-based"
        );
        seen.insert(answered);

        let children = crate::instance::slot(receiver, CHILDREN_SLOT);
        let Some(array) = children.array_ptr() else {
            return;
        };
        let array = crate::arr::borrowed(array);
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let child = array
                .value_at(slot)
                .expect("next_slot only names live entries");
            line_agrees(child, file, ctx, seen);
            from = slot + 1;
        }
    }

    /// A node's offset is the byte position the one home converts that node's
    /// own line and column back to, and it points at a byte the construct
    /// starts with.
    ///
    /// `rule:ide/positions-have-one-home` closed the other way round: the
    /// column test reads a column off an offset, and this reads the offset back
    /// off a line and a column. The source puts a four-byte emoji before the
    /// nodes asked about, so a member answering a character position where a
    /// byte one is documented disagrees from there on, and the largest offset
    /// reached is what says the walk went past it.
    // covers: Core\Ast\Node::offset
    #[test]
    fn a_nodes_offset_is_the_byte_position_of_its_own_line_and_column() {
        let text = "<?nvs\necho \"\u{1F600}\", 7 + 1;\n";
        let source = Value::str(NvsStr::new(text.as_bytes()));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_ast_parse, &mut ctx, &[source]).expect("that source parses");

        let mut map = SourceMap::new();
        let id = map.add("a-node's-offset", text);

        let mut seen = std::collections::BTreeSet::new();
        offset_agrees(tree, map.file(id), text.as_bytes(), &mut ctx, &mut seen);
        assert!(
            seen.len() >= 4 && seen.iter().any(|&at| at > 16),
            "the walk reached offsets {seen:?}, so it stopped before the \
             four-byte character and asserted nothing about a position counted \
             in bytes"
        );

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference `parse` answered with, and \
                      the tree's own references are the nodes'"
        )]
        unsafe {
            tree.release();
            source.release();
        }
    }

    /// Asserts that `node`'s `offset` member is what `file` converts that
    /// node's own line and column back to, records it, and recurses.
    fn offset_agrees(
        node: Value,
        file: &SourceFile,
        text: &[u8],
        ctx: &mut Ctx,
        seen: &mut std::collections::BTreeSet<i64>,
    ) {
        let answered = member_int(super::nvs_core_ast_node_offset, node, ctx, "offset");
        let line = member_int(super::nvs_core_ast_node_line, node, ctx, "line");
        let column = member_int(super::nvs_core_ast_node_column, node, ctx, "column");
        let at = file.offset_of(
            usize::try_from(line - 1).expect("a line is 1-based"),
            usize::try_from(column - 1).expect("a column is 1-based"),
            PositionEncoding::Utf32,
        );
        assert_eq!(
            answered,
            i64::from(at),
            "the offset a node answers is not the byte position of the line and \
             column it answers, so the three name three places"
        );
        let byte = text[usize::try_from(answered).expect("an offset is a position")];
        assert!(
            !byte.is_ascii_whitespace(),
            "offset {answered} points at whitespace, so it names the gap before \
             the construct rather than the construct"
        );
        seen.insert(answered);

        let receiver = node.obj_ptr().expect("a node is an object");
        let children = crate::instance::slot(receiver, CHILDREN_SLOT);
        let Some(array) = children.array_ptr() else {
            return;
        };
        let array = crate::arr::borrowed(array);
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let child = array
                .value_at(slot)
                .expect("next_slot only names live entries");
            offset_agrees(child, file, text, ctx, seen);
            from = slot + 1;
        }
    }

    /// The whole number `member` answers for `node`.
    fn member_int(member: nvs_runtime::NvsFn, node: Value, ctx: &mut Ctx, what: &str) -> i64 {
        call(member, ctx, &[node])
            .unwrap_or_else(|_| panic!("a node answers its {what}"))
            .as_int()
            .unwrap_or_else(|| panic!("a {what} is a whole number"))
    }

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
            | CoreTy::MethodRef
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
    ///    node has answers a string, an integer or more nodes, so no amount of
    ///    walking produces a value of another class, and there is no
    ///    `Core\Ast` handle a later member could take.
    /// 3. **The values are ordinary data at run time too**, over a real
    ///    parse: every slot is a `Tag::Str`, a `Tag::Int` or a `Tag::Array` of
    ///    objects, and no closure, callable or resource is anywhere in it.
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
            let answers_scalar = matches!(member.return_ty, CoreTy::Str | CoreTy::Int);
            assert!(
                answers_itself || answers_scalar,
                "{NODE_NAME}::{} answers something that is neither the tree nor \
                 a scalar, so walking the tree reaches outside it",
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
        for (slot, what) in [
            (LINE_SLOT, "line"),
            (COLUMN_SLOT, "column"),
            (OFFSET_SLOT, "offset"),
        ] {
            assert_eq!(
                crate::instance::slot(receiver, slot).tag(),
                Some(Tag::Int),
                "a node's {what} is a number — where the production starts, and \
                 never a handle to the source it starts in"
            );
        }
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

    /// `rule:core-classes/ast-is-inert`'s one grammar, over a corpus: every seed
    /// the fuzz targets read gets the compiler's own verdict from
    /// `Core\Ast::parse` — source `nvs_syntax::parse_file` reports an error
    /// for throws `ParseError`, and source it accepts answers a tree whose
    /// every node is one of the roster's production classes.
    ///
    /// The corpus is `fuzz/seeds/parse/`, which the `parse` and `ast` targets
    /// both take, and this is the leg that runs everywhere: libFuzzer needs
    /// nightly and does not build on Windows, so the unbounded half runs where
    /// it can and the seeds are replayed here.
    ///
    /// The verdict compared against is `parse_file`'s own diagnostics rather
    /// than `nvs_syntax::walk::of_source`'s error, which is the door the
    /// member itself calls: asking one function twice would assert nothing
    /// about two answers agreeing. Both verdicts are then required of the
    /// corpus by counting, so a seed directory that drifted into holding only
    /// programs that parse stops testing the refusal silently.
    // covers: Core\Ast::parse
    #[test]
    fn core_ast_parse_gives_the_compilers_verdict_on_every_parse_seed() {
        let dir = nvs_repo::path("fuzz/seeds/parse");
        let mut seeds: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|why| {
                panic!("the seed corpus at {} is committed: {why}", dir.display())
            })
            .map(|entry| entry.expect("a readable seed").path())
            .collect();
        seeds.sort();
        let mut parsed = 0usize;
        let mut refused = 0usize;

        for seed in &seeds {
            let name = seed.display().to_string();
            let source = std::fs::read_to_string(seed)
                .unwrap_or_else(|why| panic!("{name} is UTF-8 source: {why}"));
            let mut map = SourceMap::new();
            let id = map.add(name.clone(), source.clone());
            let mut diags = Diagnostics::new();
            let _ = nvs_syntax::parse_file(map.file(id), &mut diags);
            let compiler_refuses = diags.iter().any(Diagnostic::is_error);

            let value = Value::str(NvsStr::new(source.as_bytes()));
            let mut ctx = Ctx::new(OutputSink::Sink);
            match call(super::nvs_core_ast_parse, &mut ctx, &[value]) {
                Ok(tree) => {
                    parsed += 1;
                    assert!(
                        !compiler_refuses,
                        "{name}: the compiler refuses this source and \
                         `Core\\Ast::parse` answered a tree"
                    );
                    let mut seen = Vec::new();
                    collect_classes(tree, &mut seen);
                    assert_eq!(
                        seen.first().map(String::as_str),
                        Some(r"Core\Ast\File"),
                        "{name}: a parsed file is a `File` node"
                    );
                    for class in &seen {
                        assert!(
                            class.starts_with(PRODUCTION_PREFIX),
                            "{name}: {class} is not one of the roster's production classes"
                        );
                    }
                    #[expect(
                        unsafe_code,
                        reason = "this test owns the one reference `parse` answered \
                                  with, and the tree's own references are the nodes'"
                    )]
                    unsafe {
                        tree.release();
                    }
                }
                Err(status) => {
                    refused += 1;
                    assert!(
                        compiler_refuses,
                        "{name}: the compiler accepts this source and \
                         `Core\\Ast::parse` refused it"
                    );
                    assert_eq!(
                        status, THROWN,
                        "{name}: a refused parse is a throw a program can catch, \
                         never a fault it cannot"
                    );
                    assert_eq!(
                        ctx.pending_class().as_deref(),
                        Some("ParseError"),
                        "{name}: the class a `catch` clause names"
                    );
                }
            }

            #[expect(
                unsafe_code,
                reason = "the argument is this test's own string, alive across the \
                          call and released once after it"
            )]
            unsafe {
                value.release();
            }
        }

        assert!(
            parsed > 0 && refused > 0,
            "the corpus asks both questions: {parsed} seed(s) parse and \
             {refused} are refused, of {}",
            seeds.len()
        );
    }
}
