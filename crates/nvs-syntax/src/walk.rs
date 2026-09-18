//! The parse tree as a walkable shape: one entry per node, its production's
//! name and the nodes it contains — what
//! `rule:core-classes/ast-is-inert`'s `Core\Ast::parse` hands a running program.
//!
//! # Decision: the walk lives beside the grammar, not beside the `Core` class
//!
//! `Core\Ast` is `nvs_stdlib`'s, and the obvious place for this match was
//! there. It cannot be: [`crate::ast::ExprKind`] and [`crate::ast::StmtKind`]
//! are `#[non_exhaustive]`, so a `match` written in any other crate needs a
//! wildcard arm and a production added later would silently walk as a leaf —
//! the tree would go quietly wrong rather than failing to build. Written here
//! the compiler checks it, and a new variant is a build error in this file,
//! which is the file the variant's author is already in.
//!
//! What crosses the crate boundary is therefore [`Node`], a rose tree of
//! `&'static str` kinds owning nothing of the source. `nvs_stdlib` turns that
//! into `Core\Ast\Node` instances and never sees an AST type — which is also
//! what keeps `rule:core-classes/ast-is-inert`'s inertness structural rather than promised: there
//! is no [`crate::ast::Expr`] on the other side of this function to reach a
//! lowering with.
//!
//! # Decision: a node is a statement, an expression, or a member of a
//! declaration
//!
//! Those are the productions `rule:core-classes/ast-is-inert`'s own examples name
//! (`Core\Ast\ClassDecl`, `Core\Ast\MethodDecl`). Everything else the grammar
//! carries — a type, a name, a modifier, an attribute, a parameter, a match
//! arm, a catch clause, a `foreach` binding — is a *property* of the node it
//! belongs to rather than a node of its own, and its nested expressions are
//! children of that node. So a parameter's default is a child of the function
//! it belongs to, and no node stands for the parameter itself.
//!
//! The alternative was one node per struct in [`crate::ast`], and
//! `rule:tooling/reflection-and-source-parsing-are-core-features`'s typed
//! roster is not it: that roster is one class per *production*, which is
//! [`KINDS`] — the table this walk's kinds are, and the one `Core\Ast` names
//! its classes from. Drawing the line at the three productions the ADR names
//! keeps every expression in the tree, which is what a walk is for, without
//! making a parameter or a type annotation a node in order to have a class.
//!
//! # Decision: a field is what the span does not show
//!
//! A node carries its production's scalars as [`Field`]s, and they are the
//! ones a reader cannot recover from the span: the operator a production
//! chose, a flag it recorded, and which form a member name took — which is
//! `rule:ide/recovery-is-explicit`'s `Missing` made visible to a consumer that
//! never sees a [`MemberName`].
//!
//! A name is not a field either, because a field is a word out of a closed set
//! and a name is the source's. What the node carries instead is
//! [`Node::name`] — **where** the production wrote its own name, when it wrote
//! one. That span is what a consumer needs to draw a box around a name rather
//! than around the expression holding it, and recording it here is what keeps
//! `nvs_lsp` from matching [`crate::ast`] a second time to find it: those enums
//! are `#[non_exhaustive]`, so a match in another crate needs a wildcard arm and
//! a production landing later goes quietly nameless there, while this one is a
//! build error in the file its author is already in. `Core\Ast` renders it
//! nowhere: the JSON schema is frozen (`rule:ide/ast-json-schema-is-frozen`) and
//! a consumer holding no file has nothing to do with an offset into one.
//!
//! **A literal's text is deliberately not one of them.** The span names it, so
//! a consumer holding the file already has it; and a consumer that does not
//! hold the file must not be handed a `secret` literal's bytes, which this
//! walk cannot recognise because it is a parse and the qualifier is a type
//! (`rule:security/redaction-reaches-the-tools-own-renderings`,
//! `rule:ide/ast-json-schema-is-frozen`). Text arrives here the day a caller
//! arrives that has type-checked, and the placeholder arrives with it.
//!
//! **What it spends:** one [`Node`] per statement, expression and member —
//! two words for the kind, a span, an optional second span for the name, and
//! two `Vec` headers, allocated per parse and dropped when the caller is done
//! with it, plus one small allocation for each node that has a field at all.
//! [`located`] holds one thing more — the parsed file itself, its text and its
//! line table — for as long as its caller keeps asking nodes where they start.
//! Nothing here is cached: a parse is a call, not a compilation unit.
//!
//! # The second consumer
//!
//! [`of_stmts`] is the same walk over statements already parsed, and
//! [`crate::index::SyntaxIndex`] flattens it into the offset-to-node index
//! `rule:ide/the-index-answers-the-cursor` specifies. That is why a [`Node`]
//! carries its span: the walk is the one place the grammar is matched
//! production by production, and an IDE asking which node a cursor is in must
//! not be a second place it is matched.

use nvs_diagnostics::{Diagnostics, MAX_SOURCE_LEN, SourceMap, Span};

use crate::ast::{
    AnonClassDecl, AssignOp, BinaryOp, Block, CallArgs, ClassMember, ClassMemberKind,
    DestructureElement, DestructureTarget, EnumCase, Expr, ExprKind, FnBody, FnExpr, ForInit,
    IncDecOp, MemberName, MethodMember, NewTarget, Param, PropertyHook, PropertyHookBody, Stmt,
    StmtKind, StringPart, TestOperand, UnaryOp,
};
use crate::parse_file;

/// One node of a parsed tree: which production it is, and what it contains.
///
/// The kind is the [`crate::ast`] variant's own spelling — `"Binary"`,
/// `"Echo"`, `"Method"` — because the grammar is the thing being described and
/// a second vocabulary for it would be a second thing to keep true.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// The production this node is.
    pub kind: &'static str,
    /// The source range it covers — the AST node's own span, never a second
    /// measurement of one.
    pub span: Span,
    /// Where this production wrote its own name, and [`None`] for one that
    /// wrote none.
    ///
    /// A declaration's own name, the member after `->` or `::`, the class a
    /// `new` allocates, the variable or constant a name reads. [`None`] is
    /// therefore never "not recorded yet": it is a production with no name in
    /// the source at all — `new $class()`, `$u->{$name}`, an operator, a
    /// literal — and a consumer that wants a range regardless falls back to
    /// [`span`](Self::span), which is the whole expression holding it.
    ///
    /// The name a parser *invented* is not one either
    /// (`rule:ide/recovery-is-explicit`): `$u->` with the caret after the arrow
    /// wrote no name, so this is [`None`] and the `member` field says
    /// `"missing"`.
    pub name: Option<Span>,
    /// The nodes this one contains, in source order.
    pub children: Vec<Node>,
    /// This production's own scalars, in a fixed order per production — the
    /// module doc's third decision says which scalars those are.
    pub fields: Vec<(&'static str, Field)>,
}

/// One scalar of a production: a word the grammar chose, or a flag it recorded.
///
/// Both are the parser's own vocabulary rather than the source's, on the same
/// terms as [`Node::kind`]: a spelling written here is what a consumer freezes,
/// so renaming the Rust variant behind it changes no consumer's contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Field {
    /// A fixed spelling out of a closed set — an operator, or which form a
    /// member name took.
    Word(&'static str),
    /// A flag the grammar recorded, such as `?->` or `&`.
    Flag(bool),
}

/// Every production this walk names, in [`Node::kind`]'s own spelling and in
/// sorted order.
///
/// The table `Core\Ast`'s typed roster is generated from
/// (`rule:core-classes/ast-is-inert`): one class per entry, at the same index,
/// so a consumer holding a kind finds its class by a binary search here rather
/// than by a second copy of the grammar's vocabulary.
///
/// The kinds themselves are match arms below, because the match is what the
/// compiler checks against `#[non_exhaustive]` enums — so this is the *second*
/// place a production is written, and
/// `every_kind_the_walk_answers_with_is_in_the_table` is what keeps
/// the two one home: it reads this file's own text and fails on a production
/// in either and not the other.
pub const KINDS: &[&str] = &[
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

impl Node {
    /// This node's whole subtree, itself excluded, in source order.
    ///
    /// The transitive closure of [`Self::children`], and excluding the
    /// receiver for the same reason `children` does: both answer *what this
    /// node contains*, and a node does not contain itself.
    #[must_use]
    pub fn descendants(&self) -> Vec<&Self> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect<'a>(&'a self, out: &mut Vec<&'a Self>) {
        for child in &self.children {
            out.push(child);
            child.collect(out);
        }
    }
}

/// A parsed tree and the file it was parsed from, so a consumer holding no
/// [`nvs_diagnostics::SourceFile`] can still ask where a node starts.
///
/// `Core\Ast` is that consumer: the nodes it hands a running program answer
/// their own line, column and offset, and the arithmetic behind the first two
/// is `nvs_diagnostics`' (`rule:ide/positions-have-one-home`) rather than a
/// newline count written a second time in `nvs_stdlib`.
#[derive(Debug)]
pub struct Located {
    /// The file's root node — [`of_source`]'s answer, unchanged.
    pub tree: Node,
    /// The map [`tree`](Self::tree) was parsed against, holding the line table
    /// [`position`](Self::position) reads.
    map: SourceMap,
}

impl Located {
    /// Where `node` starts: its 1-based line, its 1-based column counted in
    /// `char`s, and its byte offset into the source.
    ///
    /// The line and the column are 1-based because they are what a person is
    /// shown beside a path, which is the whole reason a node carries them;
    /// [`nvs_diagnostics::SourceFile::line_col`] counts from zero and this is
    /// the one place the two conventions meet.
    #[must_use]
    pub fn position(&self, node: &Node) -> (u32, u32, u32) {
        let (line, col) = self.map.file(node.span.file).line_col(node.span.start);
        (
            u32::try_from(line).unwrap_or(u32::MAX).saturating_add(1),
            u32::try_from(col).unwrap_or(u32::MAX).saturating_add(1),
            node.span.start,
        )
    }
}

/// Parses `source` exactly as the compiler parses a file of that name, and
/// answers the walk over it.
///
/// One grammar, `rule:core-classes/ast-is-inert`: this calls [`parse_file`], so a construct that
/// compiles parses here and a construct the parser refuses is refused here.
/// The root node is the file, whose children are its top-level statements.
///
/// # Errors
///
/// [`located`]'s, unchanged — this is that call with the map dropped.
pub fn of_source(name: &str, source: &str) -> Result<Node, String> {
    located(name, source).map(|parsed| parsed.tree)
}

/// [`of_source`], keeping the file it parsed against so a caller can ask a node
/// where it starts.
///
/// # Errors
///
/// The first error diagnostic, as `line N, column M: message` — a parse that
/// reported an error is a failure here rather than a tree with
/// [`StmtKind::Error`] in it, because a caller asking for a description of
/// source it cannot compile is asking about source it should be told about.
pub fn located(name: &str, source: &str) -> Result<Located, String> {
    if source.len() > MAX_SOURCE_LEN {
        return Err("source exceeds 4 GiB".to_owned());
    }
    let mut map = SourceMap::new();
    let id = map.add(name, source);
    let tree = {
        let file = map.file(id);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(file, &mut diags);
        if let Some(first) = diags.iter().find(|d| d.is_error()) {
            let where_ = first.primary_span().map_or_else(String::new, |span| {
                let (line, col) = file.line_col(span.start);
                format!("line {}, column {}: ", line + 1, col + 1)
            });
            return Err(format!("{where_}{}", first.message));
        }
        Node {
            kind: "File",
            span: stmts
                .iter()
                .map(|s| s.span)
                .reduce(Span::to)
                .unwrap_or_else(|| Span::at(id, 0)),
            name: None,
            children: of_stmts(&stmts),
            fields: Vec::new(),
        }
    };
    Ok(Located { tree, map })
}

/// The nodes of a file that is already parsed, in source order.
///
/// The same walk [`of_source`] does, for a caller holding the statements —
/// [`crate::index::SyntaxIndex`], which flattens them by span. There is no
/// `File` node here because a file is a list of statements rather than one of
/// them; `of_source` adds that root for `Core\Ast`, whose tree has to have one.
#[must_use]
pub fn of_stmts(stmts: &[Stmt]) -> Vec<Node> {
    stmts.iter().map(stmt).collect()
}

/// One statement, and the nodes under it.
fn stmt(s: &Stmt) -> Node {
    let mut kids = Vec::new();
    let mut name = None;
    let kind = match &s.kind {
        StmtKind::Expr(e) => {
            kids.push(expr(e));
            "Expr"
        }
        StmtKind::Return(value) => {
            push_opt(&mut kids, value.as_ref());
            "Return"
        }
        StmtKind::Block(b) => {
            push_block(&mut kids, b);
            "Block"
        }
        StmtKind::Empty => "Empty",
        StmtKind::InlineHtml(_) => "InlineHtml",
        StmtKind::If { cond, then, else_ } => {
            kids.push(expr(cond));
            kids.push(stmt(then));
            if let Some(otherwise) = else_ {
                kids.push(stmt(otherwise));
            }
            "If"
        }
        StmtKind::While { cond, body } => {
            kids.push(expr(cond));
            kids.push(stmt(body));
            "While"
        }
        StmtKind::DoWhile { body, cond } => {
            kids.push(stmt(body));
            kids.push(expr(cond));
            "DoWhile"
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            match init {
                ForInit::Decl(decl) => kids.push(stmt(decl)),
                ForInit::Exprs(exprs) => push_exprs(&mut kids, exprs),
            }
            push_exprs(&mut kids, cond);
            push_exprs(&mut kids, step);
            kids.push(stmt(body));
            "For"
        }
        StmtKind::Foreach { subject, body, .. } => {
            // The bindings are not nodes — the module doc's second decision.
            kids.push(expr(subject));
            kids.push(stmt(body));
            "Foreach"
        }
        StmtKind::Switch { subject, cases } => {
            kids.push(expr(subject));
            for case in cases {
                push_opt(&mut kids, case.cond.as_ref());
                kids.extend(case.body.iter().map(stmt));
            }
            "Switch"
        }
        StmtKind::Break(depth) => {
            push_opt(&mut kids, depth.as_ref());
            "Break"
        }
        StmtKind::Continue(depth) => {
            push_opt(&mut kids, depth.as_ref());
            "Continue"
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            push_block(&mut kids, body);
            for catch in catches {
                push_block(&mut kids, &catch.body);
            }
            if let Some(finally) = finally {
                push_block(&mut kids, finally);
            }
            "Try"
        }
        StmtKind::Echo(values) => {
            push_exprs(&mut kids, values);
            "Echo"
        }
        StmtKind::Unset(targets) => {
            push_exprs(&mut kids, targets);
            "Unset"
        }
        StmtKind::LocalDecl {
            name: declared,
            value,
            ..
        } => {
            name = Some(*declared);
            push_opt(&mut kids, value.as_ref());
            "LocalDecl"
        }
        StmtKind::Destructure { target, value } => {
            push_destructure(&mut kids, target);
            kids.push(expr(value));
            "Destructure"
        }
        StmtKind::Global(_) => "Global",
        StmtKind::Goto(label) => {
            name = Some(*label);
            "Goto"
        }
        StmtKind::StaticLocal { vars, .. } => {
            for var in vars {
                push_opt(&mut kids, var.default.as_ref());
            }
            "StaticLocal"
        }
        StmtKind::ClassDecl(decl) => {
            name = Some(decl.name.span);
            kids.extend(decl.members.iter().map(member));
            "ClassDecl"
        }
        StmtKind::InterfaceDecl(decl) => {
            name = Some(decl.name.span);
            kids.extend(decl.members.iter().map(member));
            "InterfaceDecl"
        }
        StmtKind::EnumDecl(decl) => {
            name = Some(decl.name.span);
            kids.extend(decl.cases.iter().map(enum_case));
            kids.extend(decl.members.iter().map(member));
            "EnumDecl"
        }
        StmtKind::NamespaceDecl(decl) => {
            name = decl.name.as_ref().map(|written| written.span);
            if let Some(body) = &decl.body {
                push_block(&mut kids, body);
            }
            "NamespaceDecl"
        }
        StmtKind::UseDecl(_) => "UseDecl",
        StmtKind::AutoloadDecl(_) => "AutoloadDecl",
        StmtKind::TypeAliasDecl(decl) => {
            name = Some(decl.name.span);
            "TypeAliasDecl"
        }
        StmtKind::TopLevelFunction(f) => {
            name = Some(f.name);
            push_method(&mut kids, f);
            "Function"
        }
        StmtKind::TopLevelConst(consts) => {
            for c in consts {
                kids.push(expr(&c.value));
            }
            "Const"
        }
        StmtKind::Error => "Error",
    };
    Node {
        kind,
        span: s.span,
        name,
        children: kids,
        fields: Vec::new(),
    }
}

/// One expression, and the nodes under it.
fn expr(e: &Expr) -> Node {
    let mut kids = Vec::new();
    let mut fields = Vec::new();
    let mut name = None;
    let kind = match &e.kind {
        ExprKind::Null => "Null",
        ExprKind::Bool(value) => {
            fields.push(("value", Field::Flag(*value)));
            "Bool"
        }
        ExprKind::Int(_) => "Int",
        ExprKind::Float(_) => "Float",
        ExprKind::Duration(_) => "Duration",
        ExprKind::Str(_) => "Str",
        ExprKind::Interpolated(parts) => {
            for part in parts {
                match part {
                    StringPart::Text(_) => {}
                    StringPart::Expr(e) => kids.push(expr(e)),
                }
            }
            "Interpolated"
        }
        ExprKind::Markup(parts) => {
            for part in parts {
                match part {
                    StringPart::Text(_) => {}
                    StringPart::Expr(e) => kids.push(expr(e)),
                }
            }
            "Markup"
        }
        ExprKind::Variable(written) => {
            name = Some(*written);
            "Variable"
        }
        ExprKind::ConstFetch(written) => {
            name = Some(written.span);
            "ConstFetch"
        }
        ExprKind::SelfExpr => "SelfExpr",
        ExprKind::StaticExpr => "StaticExpr",
        ExprKind::ParentExpr => "ParentExpr",
        ExprKind::ArrayLiteral(items) => {
            for item in items {
                push_opt(&mut kids, item.key.as_ref());
                kids.push(expr(&item.value));
            }
            "ArrayLiteral"
        }
        ExprKind::Unary { op, expr: operand } => {
            fields.push(("op", Field::Word(unary_op(*op))));
            kids.push(expr(operand));
            "Unary"
        }
        ExprKind::PreIncDec { op, expr: operand } => {
            fields.push(("op", Field::Word(inc_dec_op(*op))));
            kids.push(expr(operand));
            "PreIncDec"
        }
        ExprKind::PostIncDec { op, expr: operand } => {
            fields.push(("op", Field::Word(inc_dec_op(*op))));
            kids.push(expr(operand));
            "PostIncDec"
        }
        ExprKind::Binary { op, lhs, rhs } => {
            fields.push(("op", Field::Word(binary_op(*op))));
            kids.push(expr(lhs));
            kids.push(expr(rhs));
            "Binary"
        }
        ExprKind::Assign {
            op,
            target,
            value,
            by_ref,
        } => {
            fields.push(("op", Field::Word(assign_op(*op))));
            fields.push(("byRef", Field::Flag(*by_ref)));
            kids.push(expr(target));
            kids.push(expr(value));
            "Assign"
        }
        ExprKind::Ternary { cond, then, else_ } => {
            kids.push(expr(cond));
            if let Some(then) = then {
                kids.push(expr(then));
            }
            kids.push(expr(else_));
            "Ternary"
        }
        ExprKind::Conversion { expr: operand, .. } => {
            kids.push(expr(operand));
            "Conversion"
        }
        ExprKind::TypeTest {
            expr: operand,
            against,
        } => {
            kids.push(expr(operand));
            if let TestOperand::Value(value) = against {
                kids.push(expr(value));
            }
            "TypeTest"
        }
        ExprKind::Call { callee, args } => {
            kids.push(expr(callee));
            push_args(&mut kids, args);
            "Call"
        }
        ExprKind::MethodCall {
            object,
            nullsafe,
            method,
            args,
            ..
        } => {
            fields.push(("nullsafe", Field::Flag(*nullsafe)));
            fields.push(("member", Field::Word(member_form(method))));
            name = member_span(method);
            kids.push(expr(object));
            push_member_name(&mut kids, method);
            push_args(&mut kids, args);
            "MethodCall"
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
            ..
        } => {
            fields.push(("member", Field::Word(member_form(method))));
            name = member_span(method);
            kids.push(expr(class));
            push_member_name(&mut kids, method);
            push_args(&mut kids, args);
            "StaticCall"
        }
        ExprKind::PropertyAccess {
            object,
            nullsafe,
            property,
        } => {
            fields.push(("nullsafe", Field::Flag(*nullsafe)));
            fields.push(("member", Field::Word(member_form(property))));
            name = member_span(property);
            kids.push(expr(object));
            push_member_name(&mut kids, property);
            "PropertyAccess"
        }
        ExprKind::StaticPropertyAccess { class, name: read } => {
            name = Some(*read);
            kids.push(expr(class));
            "StaticPropertyAccess"
        }
        ExprKind::ClassConstAccess { class, name: read } => {
            name = Some(*read);
            kids.push(expr(class));
            "ClassConstAccess"
        }
        ExprKind::ClassNameConst { class } => {
            kids.push(expr(class));
            "ClassNameConst"
        }
        ExprKind::Index { base, index } => {
            kids.push(expr(base));
            push_opt(&mut kids, index.as_deref());
            "Index"
        }
        ExprKind::New { target, args, .. } => {
            fields.push(("target", Field::Word(new_target(target))));
            match target {
                NewTarget::Name(written) => name = Some(written.span),
                NewTarget::SelfTy | NewTarget::StaticTy | NewTarget::ParentTy => {}
                NewTarget::Expr(e) => kids.push(expr(e)),
                NewTarget::AnonClass(decl) => push_anon_class(&mut kids, decl),
            }
            push_args(&mut kids, args);
            "New"
        }
        ExprKind::Clone(operand) => {
            kids.push(expr(operand));
            "Clone"
        }
        ExprKind::Fn(f) => {
            push_fn(&mut kids, f);
            "Fn"
        }
        ExprKind::Match { subject, arms } => {
            kids.push(expr(subject));
            for arm in arms {
                if let Some(conditions) = &arm.conditions {
                    push_exprs(&mut kids, conditions);
                }
                kids.push(expr(&arm.body));
            }
            "Match"
        }
        ExprKind::Catch { guarded, arms } => {
            kids.push(expr(guarded));
            for arm in arms {
                kids.push(expr(&arm.body));
            }
            "Catch"
        }
        ExprKind::Yield { key, value } => {
            push_opt(&mut kids, key.as_deref());
            push_opt(&mut kids, value.as_deref());
            "Yield"
        }
        ExprKind::YieldFrom(operand) => {
            kids.push(expr(operand));
            "YieldFrom"
        }
        ExprKind::Print(operand) => {
            kids.push(expr(operand));
            "Print"
        }
        ExprKind::Throw(operand) => {
            kids.push(expr(operand));
            "Throw"
        }
        ExprKind::Isset(targets) => {
            push_exprs(&mut kids, targets);
            "Isset"
        }
        ExprKind::Empty(operand) => {
            kids.push(expr(operand));
            "Empty"
        }
        ExprKind::Exit(status) => {
            push_opt(&mut kids, status.as_deref());
            "Exit"
        }
        ExprKind::SpawnScript { path, options } => {
            kids.push(expr(path));
            for option in options {
                kids.push(expr(&option.value));
            }
            "SpawnScript"
        }
        ExprKind::Await(operand) => {
            kids.push(expr(operand));
            "Await"
        }
        ExprKind::Require { path } => {
            kids.push(expr(path));
            "Require"
        }
        ExprKind::Paren(inner) => {
            kids.push(expr(inner));
            "Paren"
        }
        ExprKind::ObjectLiteral(fields) => {
            for field in fields {
                kids.push(expr(&field.value));
            }
            "ObjectLiteral"
        }
        ExprKind::Error(_) => "Error",
    };
    Node {
        kind,
        span: e.span,
        name,
        children: kids,
        fields,
    }
}

/// A unary operator's own spelling.
///
/// Written out rather than derived from `Debug`, because a consumer freezes
/// what this returns: renaming the variant is then a rename and not a schema
/// change, and adding one is a build error in this file.
fn unary_op(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "Neg",
        UnaryOp::Plus => "Plus",
        UnaryOp::Not => "Not",
        UnaryOp::BitNot => "BitNot",
        UnaryOp::Suppress => "Suppress",
    }
}

/// An increment or decrement operator's own spelling.
fn inc_dec_op(op: IncDecOp) -> &'static str {
    match op {
        IncDecOp::Inc => "Inc",
        IncDecOp::Dec => "Dec",
    }
}

/// A binary operator's own spelling.
fn binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "Add",
        BinaryOp::Sub => "Sub",
        BinaryOp::Mul => "Mul",
        BinaryOp::Div => "Div",
        BinaryOp::Mod => "Mod",
        BinaryOp::Pow => "Pow",
        BinaryOp::Concat => "Concat",
        BinaryOp::BitAnd => "BitAnd",
        BinaryOp::BitOr => "BitOr",
        BinaryOp::BitXor => "BitXor",
        BinaryOp::Shl => "Shl",
        BinaryOp::Shr => "Shr",
        BinaryOp::And => "And",
        BinaryOp::Or => "Or",
        BinaryOp::Eq => "Eq",
        BinaryOp::NotEq => "NotEq",
        BinaryOp::Lt => "Lt",
        BinaryOp::LtEq => "LtEq",
        BinaryOp::Gt => "Gt",
        BinaryOp::GtEq => "GtEq",
        BinaryOp::Cmp => "Cmp",
        BinaryOp::Coalesce => "Coalesce",
    }
}

/// An assignment operator's own spelling.
fn assign_op(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Assign => "Assign",
        AssignOp::AddAssign => "AddAssign",
        AssignOp::SubAssign => "SubAssign",
        AssignOp::MulAssign => "MulAssign",
        AssignOp::DivAssign => "DivAssign",
        AssignOp::ModAssign => "ModAssign",
        AssignOp::PowAssign => "PowAssign",
        AssignOp::ConcatAssign => "ConcatAssign",
        AssignOp::BitAndAssign => "BitAndAssign",
        AssignOp::BitOrAssign => "BitOrAssign",
        AssignOp::BitXorAssign => "BitXorAssign",
        AssignOp::ShlAssign => "ShlAssign",
        AssignOp::ShrAssign => "ShrAssign",
        AssignOp::CoalesceAssign => "CoalesceAssign",
    }
}

/// Which form a member name took — and `"missing"` is the one that matters,
/// since it is the access the parser recovered at rather than one anybody
/// wrote (`rule:ide/recovery-is-explicit`).
fn member_form(name: &MemberName) -> &'static str {
    match name {
        MemberName::Ident(_) => "written",
        MemberName::Missing(_) => "missing",
        MemberName::Variable(_) => "variable",
        MemberName::Expr(_) => "expression",
    }
}

/// Which form a `new` target took. The named cases contribute no child, so
/// without this a consumer cannot tell `new self` from `new C`.
fn new_target(target: &NewTarget) -> &'static str {
    match target {
        NewTarget::Name(_) => "name",
        NewTarget::SelfTy => "self",
        NewTarget::StaticTy => "static",
        NewTarget::ParentTy => "parent",
        NewTarget::Expr(_) => "expression",
        NewTarget::AnonClass(_) => "anonymous",
    }
}

/// One member of a class, interface or enum body.
fn member(m: &ClassMember) -> Node {
    let mut kids = Vec::new();
    let mut name = None;
    let kind = match &m.kind {
        ClassMemberKind::Property(p) => {
            name = Some(p.name);
            push_opt(&mut kids, p.default.as_ref());
            for hook in p.hooks.iter().flatten() {
                push_hook(&mut kids, hook);
            }
            "Property"
        }
        ClassMemberKind::Const(c) => {
            name = Some(c.name);
            kids.push(expr(&c.value));
            "Const"
        }
        ClassMemberKind::Method(f) => {
            name = Some(f.name);
            push_method(&mut kids, f);
            "Method"
        }
        ClassMemberKind::TypeAlias(a) => {
            name = Some(a.name.span);
            "TypeAliasDecl"
        }
        ClassMemberKind::Error => "Error",
    };
    Node {
        kind,
        span: m.span,
        name,
        children: kids,
        fields: Vec::new(),
    }
}

/// One `case` of an enum declaration.
fn enum_case(c: &EnumCase) -> Node {
    let mut kids = Vec::new();
    push_opt(&mut kids, c.value.as_ref());
    Node {
        kind: "EnumCase",
        span: c.span,
        name: Some(c.name.span),
        children: kids,
        fields: Vec::new(),
    }
}

fn push_opt(kids: &mut Vec<Node>, value: Option<&Expr>) {
    if let Some(value) = value {
        kids.push(expr(value));
    }
}

fn push_exprs(kids: &mut Vec<Node>, values: &[Expr]) {
    kids.extend(values.iter().map(expr));
}

/// A block that is not itself a statement contributes its statements — a
/// `try` body's braces are the `try`'s, not a node between them.
fn push_block(kids: &mut Vec<Node>, block: &Block) {
    kids.extend(block.stmts.iter().map(stmt));
}

fn push_args(kids: &mut Vec<Node>, args: &CallArgs) {
    match args {
        CallArgs::List(list) => kids.extend(list.iter().map(|arg| expr(&arg.value))),
        CallArgs::FirstClassCallable => {}
    }
}

/// A member name computed from an expression carries that expression; a
/// written one is a name, and the module doc's second decision says a name is
/// not a node. A missing one is a position rather than a name, so it is not a
/// node either: the access that carries it is what an offset lands in, which
/// is [`crate::index`]'s own first consequence.
/// Where a written member name is, and [`None`] for a computed or missing one.
///
/// [`Node::name`]'s own rule at one production: a name the source wrote is a
/// span, a name an expression computes is a child node, and a name the parser
/// invented is neither.
fn member_span(name: &MemberName) -> Option<Span> {
    match name {
        MemberName::Ident(span) => Some(*span),
        MemberName::Missing(_) | MemberName::Variable(_) | MemberName::Expr(_) => None,
    }
}

fn push_member_name(kids: &mut Vec<Node>, name: &MemberName) {
    match name {
        MemberName::Ident(_) | MemberName::Missing(_) => {}
        MemberName::Variable(e) | MemberName::Expr(e) => kids.push(expr(e)),
    }
}

fn push_params(kids: &mut Vec<Node>, params: &[Param]) {
    for param in params {
        push_opt(kids, param.default.as_ref());
    }
}

fn push_method(kids: &mut Vec<Node>, f: &MethodMember) {
    push_params(kids, &f.params);
    if let Some(body) = &f.body {
        push_block(kids, body);
    }
}

fn push_fn(kids: &mut Vec<Node>, f: &FnExpr) {
    push_params(kids, &f.params);
    match &f.body {
        FnBody::Expr(e) => kids.push(expr(e)),
        FnBody::Block(b) => push_block(kids, b),
    }
}

fn push_hook(kids: &mut Vec<Node>, hook: &PropertyHook) {
    if let Some(param) = &hook.param {
        push_opt(kids, param.default.as_ref());
    }
    match &hook.body {
        None => {}
        Some(PropertyHookBody::Expr(e)) => kids.push(expr(e)),
        Some(PropertyHookBody::Block(b)) => push_block(kids, b),
    }
}

fn push_anon_class(kids: &mut Vec<Node>, decl: &AnonClassDecl) {
    kids.extend(decl.members.iter().map(member));
}

/// A destructuring target's *keys* are expressions; its leaves are names.
fn push_destructure(kids: &mut Vec<Node>, target: &DestructureTarget) {
    for element in &target.elements {
        match element {
            DestructureElement::Skip => {}
            DestructureElement::Leaf { key, .. } => push_opt(kids, key.as_ref()),
            DestructureElement::Nested { key, target, .. } => {
                push_opt(kids, key.as_ref());
                push_destructure(kids, target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{KINDS, of_source};

    /// The three spans of this file a kind literal can appear in, as the
    /// `fn` line that opens each and the one that closes it.
    ///
    /// Everything between `unary_op` and `member` is the parser's own
    /// vocabulary — a [`Field::Word`] rather than a production — which is why
    /// the productions are two spans and not one.
    ///
    /// [`Field::Word`]: super::Field::Word
    const SPANS: &[(&str, &str)] = &[
        ("pub fn of_source(", "pub fn of_stmts("),
        ("fn stmt(", "fn unary_op("),
        ("fn member(", "fn push_opt("),
    ];

    /// [`KINDS`] is the productions the match arms answer with, exactly.
    ///
    /// Read off this file's own text because a production is a match arm and
    /// there is nothing else to read: the arms are what the compiler checks
    /// against `#[non_exhaustive]`, and the table is what a consumer indexes.
    /// A new production fails here until it is in both, which is the whole
    /// obligation the table adds.
    #[test]
    fn every_kind_the_walk_answers_with_is_in_the_table() {
        let text = include_str!("walk.rs");
        let mut answered: BTreeSet<&str> = BTreeSet::new();
        for (open, close) in SPANS {
            let start = text
                .find(open)
                .unwrap_or_else(|| panic!("`{open}` is in this file"));
            let end = text[start..]
                .find(close)
                .unwrap_or_else(|| panic!("`{close}` follows `{open}`"))
                + start;
            for line in text[start..end].lines() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for (index, piece) in line.split('"').enumerate() {
                    if index % 2 == 1
                        && piece.starts_with(|c: char| c.is_ascii_uppercase())
                        && piece.chars().all(|c| c.is_ascii_alphabetic())
                    {
                        answered.insert(piece);
                    }
                }
            }
        }
        let table: BTreeSet<&str> = KINDS.iter().copied().collect();
        assert_eq!(
            answered, table,
            "the productions the walk answers with and `KINDS` have gone apart"
        );
        assert!(
            KINDS.windows(2).all(|pair| pair[0] < pair[1]),
            "`KINDS` is sorted, because a consumer finds a production's index by \
             a binary search over it"
        );
    }

    #[test]
    fn a_files_walk_is_every_statement_and_expression_under_it() {
        let tree = of_source("<test>", "<?nvs echo 1 + 2;").expect("parses");
        assert_eq!(tree.kind, "File");
        let kinds: Vec<&str> = tree.descendants().iter().map(|n| n.kind).collect();
        assert_eq!(kinds, ["Echo", "Binary", "Int", "Int"]);
    }

    #[test]
    fn a_method_body_is_reached_through_its_class_and_its_member() {
        let tree = of_source(
            "<test>",
            "<?nvs class C { public function m(): void { echo 1; } }",
        )
        .expect("parses");
        let kinds: Vec<&str> = tree.descendants().iter().map(|n| n.kind).collect();
        assert_eq!(kinds, ["ClassDecl", "Method", "Echo", "Int"]);
    }

    #[test]
    fn source_the_compiler_refuses_is_refused_here_with_its_position() {
        let error = of_source("<test>", "<?nvs echo ;").expect_err("does not parse");
        assert!(error.starts_with("line 1, column "), "{error}");
    }
}
