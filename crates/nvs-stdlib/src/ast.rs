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

/// [`NODE`]'s slot holding the 1-based line the production starts on.
const LINE_SLOT: usize = 2;

/// [`NODE`]'s slot holding the 1-based column the production starts at,
/// counted in characters.
const COLUMN_SLOT: usize = 3;

/// [`NODE`]'s slot holding the byte offset the production starts at.
const OFFSET_SLOT: usize = 4;

/// `Core\Ast` — one member, because parsing is one question.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
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

/// `Core\Ast\Node` — the type [`CLASS`]'s member and this class's own two
/// collections are declared as, and the members every parsed node answers.
///
/// No instance carries *this* class: a node's runtime class is its
/// production's ([`PRODUCTIONS`]), which the module doc's second decision
/// owns.
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
    use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};
    use nvs_runtime::{Ctx, NvsObj, NvsStr, OutputSink, THROWN, Tag, Value, call};

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
