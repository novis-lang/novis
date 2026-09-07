//! What every name in a document is, for the colour a regex cannot choose.
//!
//! `textDocument/semanticTokens/full` is the second of the two highlighting
//! layers ADR 0099 § 4 splits colour into, and the division of labour is what
//! decides everything in this file: the TextMate grammar colours what a regex
//! can see — a keyword, a string, a type *position* — and this layer colours
//! what only a parse can answer. So the walk emits a token exactly where **the
//! tree says what a name is**, and stays silent where saying would mean
//! guessing.
//!
//! The legend is [`crate::TOKEN_TYPES`] and its order is the wire encoding;
//! [`Kind`] names the subset this walk can answer and finds its own index in
//! that list rather than repeating it, so the two cannot fall out of step.
//!
//! # Decision: the grammar answers the kind, and nothing here resolves a name
//!
//! `rule:ide/one-grammar-one-tree` gives this crate one tree, and
//! [`crate::definition`]'s module doc gives it the standing rule that a server
//! resolving a written name for itself is a second implementation of the
//! checker's own resolution — one that disagrees the first time an import is
//! involved. So a token is emitted only where the production itself is the
//! answer:
//!
//! - a declaration names what it declares — `class`, `interface`, `enum`, a
//!   case, a `type` alias, a namespace, a property, a method, a parameter;
//! - a *position* names what may stand in it — `extends` on a class is a class,
//!   `implements` is an interface, `new C()` is a class, and the receiver of
//!   `C::m()` or `C::$p` is a class, because
//!   `rule:enums/no-class-machinery` leaves an enum with neither;
//! - a member access names a member — `$u->name` is a property and `$u->m()` a
//!   method, which is the whole reason this layer exists: the two are one
//!   regex away from each other and a parse tells them apart.
//!
//! What that leaves silent is deliberate, and each has one reason:
//!
//! - **A type annotation gets no token.** `User` in `function f(User $u)` is a
//!   name whose kind needs `nvs_hir::hierarchy::resolve_ref` and the site's
//!   namespace and imports. A type *position* is exactly what a regex can see,
//!   so ADR 0099 § 4 already gives it to the TextMate grammar, and the layer
//!   that would have to resolve it is the one that does not have to.
//! - **A class constant gets none either way.** `Config::MAX` and
//!   `Status::Draft` are one production, and telling a constant from an enum
//!   case is resolution again. LSP's legend has `enumMember` and no name at all
//!   for a class constant, so a guess here would be wrong in one direction and
//!   unspellable in the other.
//! - **A free `function` or `const` gets none.**
//!   `rule:classes/no-free-functions-or-constants` refuses both, and
//!   `rule:ide/rejected-syntax-gets-no-colour` is why nothing here dresses one
//!   as a method. Its parameters and body are still walked, because the
//!   variables in them are variables wherever they were written.
//! - **`typeParameter` is in the legend and is emitted nowhere.** No production
//!   declares one today; the legend is declared for the whole goal rather than
//!   stage by stage (`crate::capabilities`), so an unemitted type is expected
//!   rather than a gap.
//!
//! # Decision: the walk is this crate's, and a new production emits nothing
//!
//! `nvs_syntax::walk` is the one exhaustive match over the grammar, and it
//! cannot serve here: its nodes are statements, expressions and members, and a
//! node's span is the whole production — `$u->name`, not `name`. A token needs
//! the *name's* span, which that walk deliberately does not model, for the
//! reasons its own module doc gives.
//!
//! So this is a second traversal, in another crate, over `#[non_exhaustive]`
//! enums it must therefore match with a wildcard. The cost is named rather than
//! hidden: a production landing in `nvs_syntax::ast` after this file was
//! written emits no tokens and fails no build. What catches it is the coverage
//! matrix `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case` requires, which
//! infers a request's coverage from the nodes its cases reached.
//!
//! # What it spends
//!
//! One `(Span, Kind)` pair per name while the walk runs, then one
//! `SemanticToken` per name that survives encoding — both O(names in the entry
//! document), allocated per request and dropped with the answer. Nothing is
//! cached: a re-analysis rebuilds the tree this reads
//! (`rule:ide/a-full-reanalysis-stays-under-a-bound`).

use lsp_types::{SemanticToken, SemanticTokenType};
use nvs_diagnostics::{PositionEncoding, SourceFile, Span};
use nvs_syntax::ast::{
    Block, CallArgs, ClassMember, ClassMemberKind, DestructureElement, DestructureTarget, Expr,
    ExprKind, FnBody, FnExpr, ForInit, MemberName, Name, NewTarget, Param, PropertyHook,
    PropertyHookBody, Stmt, StmtKind, StringPart,
};

use crate::TOKEN_TYPES;
use crate::document::Analysed;
use crate::position::range_at;

/// The token types this walk can answer, out of the legend's eleven.
///
/// A variant carries no index of its own: [`Kind::index`] finds its spelling in
/// [`crate::TOKEN_TYPES`], so the wire order is written once, where the legend
/// the client registers is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// A `namespace` declaration's own name.
    Namespace,
    /// A class, and every position only a class may stand in.
    Class,
    /// An interface, and every position only an interface may stand in.
    Interface,
    /// An `enum` declaration's own name.
    Enum,
    /// A `case` of one.
    EnumMember,
    /// A `type` alias's own name.
    Type,
    /// A method, declared or called.
    Method,
    /// A property, declared or accessed.
    Property,
    /// A parameter, declared or named at a call site.
    Parameter,
    /// Any other `$name` — a local, a `foreach` binding, a caught exception.
    Variable,
}

impl Kind {
    /// LSP's own name for this kind.
    fn spelling(self) -> SemanticTokenType {
        match self {
            Self::Namespace => SemanticTokenType::NAMESPACE,
            Self::Class => SemanticTokenType::CLASS,
            Self::Interface => SemanticTokenType::INTERFACE,
            Self::Enum => SemanticTokenType::ENUM,
            Self::EnumMember => SemanticTokenType::ENUM_MEMBER,
            Self::Type => SemanticTokenType::TYPE,
            Self::Method => SemanticTokenType::METHOD,
            Self::Property => SemanticTokenType::PROPERTY,
            Self::Parameter => SemanticTokenType::PARAMETER,
            Self::Variable => SemanticTokenType::VARIABLE,
        }
    }

    /// Where the legend holds this kind, which is what the wire carries.
    ///
    /// `None` for a kind the legend does not name, which a token is dropped
    /// over rather than sent as some other colour — the test at the foot of
    /// this file is what says that cannot happen.
    fn index(self) -> Option<u32> {
        let spelling = self.spelling();
        let at = TOKEN_TYPES.iter().position(|named| *named == spelling)?;
        u32::try_from(at).ok()
    }
}

/// Every name in one analysed document, in the wire's delta encoding.
///
/// `keep` narrows the answer to the token types it names, and an empty one is
/// every type — that is `semanticTokens types=`, which a `.lspt` case writes so
/// a case about one kind is not rewritten by every unrelated token added later
/// (`crate::RequestArgs`). A client is offered no such filter and passes none.
/// It is applied here rather than to the returned list because the wire's
/// deltas are relative to the token before, so dropping one afterwards would
/// move every token after it.
///
/// Empty for a document whose analysis reached no statements at all, which is
/// what an unreadable entry file leaves behind — the same answer
/// [`crate::symbols::for_document`] gives, for the same reason.
#[must_use]
pub fn for_document(
    analysed: &Analysed,
    encoding: PositionEncoding,
    keep: &[String],
) -> Vec<SemanticToken> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    of_stmts(
        &loaded.stmts,
        analysed.map.file(analysed.entry),
        encoding,
        keep,
    )
}

/// The tokens `stmts` names, encoded against `file`.
fn of_stmts(
    stmts: &[Stmt],
    file: &SourceFile,
    encoding: PositionEncoding,
    keep: &[String],
) -> Vec<SemanticToken> {
    let mut named = Named::default();
    named.stmts(stmts);
    encode(named.tokens, file, encoding, keep)
}

/// Delta-encodes the collected names, in source order.
///
/// LSP carries each token relative to the one before it, so the list is sorted
/// by where it starts first — the walk emits a declaration's name before the
/// body it precedes but a `foreach` binding after the subject it follows, and
/// the encoding is only defined on a sorted list.
///
/// **A token that is not one line of text is dropped.** LSP's encoding has no
/// way to spell one, and a name never spans a line break, so this is a
/// structural impossibility being refused rather than a case being handled.
fn encode(
    mut tokens: Vec<(Span, Kind)>,
    file: &SourceFile,
    encoding: PositionEncoding,
    keep: &[String],
) -> Vec<SemanticToken> {
    tokens.sort_by_key(|(span, kind)| (span.start, span.end, *kind));
    let mut out = Vec::with_capacity(tokens.len());
    let (mut line, mut character) = (0_u32, 0_u32);
    for (span, kind) in tokens {
        if !keep.is_empty() && !keep.iter().any(|named| named == kind.spelling().as_str()) {
            continue;
        }
        let range = range_at(file, span, encoding);
        let (Some(token_type), true) = (kind.index(), range.start.line == range.end.line) else {
            continue;
        };
        let length = range.end.character.saturating_sub(range.start.character);
        if length == 0 {
            continue;
        }
        let delta_line = range.start.line.saturating_sub(line);
        out.push(SemanticToken {
            delta_line,
            delta_start: if delta_line == 0 {
                range.start.character.saturating_sub(character)
            } else {
                range.start.character
            },
            length,
            token_type,
            token_modifiers_bitset: 0,
        });
        line = range.start.line;
        character = range.start.character;
    }
    out
}

/// The names one walk has reached, with what the tree said each one is.
#[derive(Default)]
struct Named {
    /// In the order the walk found them, which is not source order.
    tokens: Vec<(Span, Kind)>,
}

impl Named {
    /// Records one name, unless it covers no source bytes.
    ///
    /// An empty span is `rule:ide/recovery-is-explicit`'s insertion point — a
    /// `MemberName::Missing` after a trailing `->`, a declaration whose name
    /// has not been typed yet — and there is nothing on screen to colour.
    fn push(&mut self, span: Span, kind: Kind) {
        if span.start < span.end {
            self.tokens.push((span, kind));
        }
    }

    /// Records a qualified name's whole span.
    fn name(&mut self, name: &Name, kind: Kind) {
        self.push(name.span, kind);
    }

    /// Records a name a production may have omitted.
    fn opt_name(&mut self, name: Option<&Name>, kind: Kind) {
        if let Some(name) = name {
            self.name(name, kind);
        }
    }

    /// Walks a run of statements.
    fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    /// Walks a block's statements.
    fn block(&mut self, block: &Block) {
        self.stmts(&block.stmts);
    }

    /// Walks one statement, recording what it declares.
    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.expr(expr),
            StmtKind::Return(value) => self.opt_expr(value.as_ref()),
            StmtKind::Block(block) => self.block(block),
            StmtKind::If { cond, then, else_ } => {
                self.expr(cond);
                self.stmt(then);
                if let Some(otherwise) = else_ {
                    self.stmt(otherwise);
                }
            }
            StmtKind::While { cond, body } => {
                self.expr(cond);
                self.stmt(body);
            }
            StmtKind::DoWhile { body, cond } => {
                self.stmt(body);
                self.expr(cond);
            }
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => {
                match init {
                    ForInit::Decl(decl) => self.stmt(decl),
                    ForInit::Exprs(exprs) => self.exprs(exprs),
                }
                self.exprs(cond);
                self.exprs(step);
                self.stmt(body);
            }
            StmtKind::Foreach {
                subject,
                key,
                value,
                body,
                ..
            } => {
                self.expr(subject);
                if let Some(key) = key {
                    self.push(key.name, Kind::Variable);
                }
                self.push(value.name, Kind::Variable);
                self.stmt(body);
            }
            StmtKind::Switch { subject, cases } => {
                self.expr(subject);
                for case in cases {
                    self.opt_expr(case.cond.as_ref());
                    self.stmts(&case.body);
                }
            }
            StmtKind::Break(level) | StmtKind::Continue(level) => self.opt_expr(level.as_ref()),
            StmtKind::Try {
                body,
                catches,
                finally,
            } => {
                self.block(body);
                for catch in catches {
                    if let Some(var) = catch.var {
                        self.push(var, Kind::Variable);
                    }
                    self.block(&catch.body);
                }
                if let Some(finally) = finally {
                    self.block(finally);
                }
            }
            StmtKind::Echo(exprs) | StmtKind::Unset(exprs) => self.exprs(exprs),
            StmtKind::LocalDecl { name, value, .. } => {
                self.push(*name, Kind::Variable);
                self.opt_expr(value.as_ref());
            }
            StmtKind::Destructure { target, value } => {
                self.destructure(target);
                self.expr(value);
            }
            StmtKind::Global(vars) => {
                for var in vars {
                    self.push(*var, Kind::Variable);
                }
            }
            StmtKind::StaticLocal { vars, .. } => {
                for var in vars {
                    self.push(var.name, Kind::Variable);
                    self.opt_expr(var.default.as_ref());
                }
            }
            StmtKind::ClassDecl(decl) => {
                self.name(&decl.name, Kind::Class);
                self.opt_name(decl.extends.as_ref(), Kind::Class);
                for clause in &decl.implements {
                    self.name(&clause.name, Kind::Interface);
                    // `by $field` names a property of this class, which is the
                    // one thing `rule:classes/delegation-by-field` writes that
                    // is not the interface it delegates.
                    if let Some(field) = clause.by_field {
                        self.push(field, Kind::Property);
                    }
                }
                self.members(&decl.members);
            }
            StmtKind::InterfaceDecl(decl) => {
                self.name(&decl.name, Kind::Interface);
                for extended in &decl.extends {
                    self.name(extended, Kind::Interface);
                }
                self.members(&decl.members);
            }
            StmtKind::EnumDecl(decl) => {
                self.name(&decl.name, Kind::Enum);
                for implemented in &decl.implements {
                    self.name(implemented, Kind::Interface);
                }
                for case in &decl.cases {
                    self.name(&case.name, Kind::EnumMember);
                    self.opt_expr(case.value.as_ref());
                }
                self.members(&decl.members);
            }
            StmtKind::TypeAliasDecl(decl) => self.name(&decl.name, Kind::Type),
            StmtKind::NamespaceDecl(decl) => {
                self.opt_name(decl.name.as_ref(), Kind::Namespace);
                if let Some(body) = &decl.body {
                    self.block(body);
                }
            }
            // Refused by `rule:classes/no-free-functions-or-constants`, so the
            // name it declares is coloured as nothing — the module doc above
            // says why the body is still walked.
            StmtKind::TopLevelFunction(function) => {
                self.params(&function.params);
                if let Some(body) = &function.body {
                    self.block(body);
                }
            }
            StmtKind::TopLevelConst(declared) => {
                for constant in declared {
                    self.expr(&constant.value);
                }
            }
            _ => {}
        }
    }

    /// Walks a class, interface or enum body.
    fn members(&mut self, members: &[ClassMember]) {
        for member in members {
            match &member.kind {
                ClassMemberKind::Property(property) => {
                    self.push(property.name, Kind::Property);
                    self.opt_expr(property.default.as_ref());
                    for hook in property.hooks.iter().flatten() {
                        self.hook(hook);
                    }
                }
                // The legend names no kind for a class constant, so only its
                // value is walked.
                ClassMemberKind::Const(declared) => self.expr(&declared.value),
                ClassMemberKind::Method(method) => {
                    self.push(method.name, Kind::Method);
                    self.params(&method.params);
                    if let Some(body) = &method.body {
                        self.block(body);
                    }
                }
                _ => {}
            }
        }
    }

    /// Walks one property hook's parameter and body.
    fn hook(&mut self, hook: &PropertyHook) {
        if let Some(param) = &hook.param {
            self.param(param);
        }
        match &hook.body {
            Some(PropertyHookBody::Expr(expr)) => self.expr(expr),
            Some(PropertyHookBody::Block(block)) => self.block(block),
            _ => {}
        }
    }

    /// Walks a parameter list.
    fn params(&mut self, params: &[Param]) {
        for param in params {
            self.param(param);
        }
    }

    /// Records a parameter's own name and walks its default.
    ///
    /// A promoted constructor parameter declares a property as well, and is
    /// still recorded as the parameter it is written as: the token says what
    /// the reader is looking at, and what they are looking at is a parameter
    /// list.
    fn param(&mut self, param: &Param) {
        self.push(param.name, Kind::Parameter);
        self.opt_expr(param.default.as_ref());
    }

    /// Records every name a destructuring target binds.
    fn destructure(&mut self, target: &DestructureTarget) {
        for element in &target.elements {
            match element {
                DestructureElement::Leaf { key, name, .. } => {
                    self.opt_expr(key.as_ref());
                    self.push(*name, Kind::Variable);
                }
                DestructureElement::Nested { key, target, .. } => {
                    self.opt_expr(key.as_ref());
                    self.destructure(target);
                }
                _ => {}
            }
        }
    }

    /// Walks a run of expressions.
    fn exprs(&mut self, exprs: &[Expr]) {
        for expr in exprs {
            self.expr(expr);
        }
    }

    /// Walks an expression a production may have omitted.
    fn opt_expr(&mut self, expr: Option<&Expr>) {
        if let Some(expr) = expr {
            self.expr(expr);
        }
    }

    /// Walks one expression, recording every name it writes.
    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Variable(span) => self.push(*span, Kind::Variable),
            ExprKind::Interpolated(parts) => {
                for part in parts {
                    if let StringPart::Expr(expr) = part {
                        self.expr(expr);
                    }
                }
            }
            ExprKind::ArrayLiteral(items) => {
                for item in items {
                    self.opt_expr(item.key.as_ref());
                    self.expr(&item.value);
                }
            }
            ExprKind::ObjectLiteral(fields) => {
                for field in fields {
                    self.push(field.name, Kind::Property);
                    self.expr(&field.value);
                }
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::PreIncDec { expr, .. }
            | ExprKind::PostIncDec { expr, .. }
            | ExprKind::Conversion { expr, .. } => self.expr(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            ExprKind::Ternary { cond, then, else_ } => {
                self.expr(cond);
                self.opt_expr(then.as_deref());
                self.expr(else_);
            }
            ExprKind::InstanceOf { expr, class } => {
                self.expr(expr);
                // Not a class position: `rule:classes/no-traits` leaves an
                // interface and an enum both legal on the right of
                // `instanceof`, so the tree does not say which this is.
                self.expr(class);
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                self.args(args);
            }
            ExprKind::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                self.expr(object);
                self.member(method, Kind::Method);
                self.args(args);
            }
            ExprKind::StaticCall {
                class,
                method,
                args,
                ..
            } => {
                self.receiver(class);
                self.member(method, Kind::Method);
                self.args(args);
            }
            ExprKind::PropertyAccess {
                object, property, ..
            } => {
                self.expr(object);
                self.member(property, Kind::Property);
            }
            ExprKind::StaticPropertyAccess { class, name } => {
                self.receiver(class);
                self.push(*name, Kind::Property);
            }
            // Both halves silent: the module doc above says why a class
            // constant and an enum case cannot be told apart here.
            ExprKind::ClassConstAccess { class, .. } | ExprKind::ClassNameConst { class } => {
                self.expr(class);
            }
            ExprKind::Index { base, index } => {
                self.expr(base);
                self.opt_expr(index.as_deref());
            }
            ExprKind::New { target, args, .. } => {
                self.new_target(target);
                self.args(args);
            }
            ExprKind::Match { subject, arms } => {
                self.expr(subject);
                for arm in arms {
                    if let Some(conditions) = &arm.conditions {
                        self.exprs(conditions);
                    }
                    self.expr(&arm.body);
                }
            }
            ExprKind::Catch { guarded, arms } => {
                self.expr(guarded);
                for arm in arms {
                    if let Some(var) = arm.var {
                        self.push(var, Kind::Variable);
                    }
                    self.expr(&arm.body);
                }
            }
            ExprKind::Yield { key, value } => {
                self.opt_expr(key.as_deref());
                self.opt_expr(value.as_deref());
            }
            ExprKind::Clone(inner)
            | ExprKind::YieldFrom(inner)
            | ExprKind::Print(inner)
            | ExprKind::Throw(inner)
            | ExprKind::Empty(inner)
            | ExprKind::Await(inner)
            | ExprKind::Paren(inner) => self.expr(inner),
            ExprKind::Isset(exprs) => self.exprs(exprs),
            ExprKind::Exit(status) => self.opt_expr(status.as_deref()),
            ExprKind::Fn(closure) => self.closure(closure),
            ExprKind::SpawnScript { path, options } => {
                self.expr(path);
                for option in options {
                    self.expr(&option.value);
                }
            }
            ExprKind::Require { path, .. } => self.expr(path),
            _ => {}
        }
    }

    /// The receiver of a static call or a static property access.
    ///
    /// A bare name here is a class and nothing else: `rule:enums/no-class-machinery`
    /// gives an enum neither static methods nor properties, and an interface
    /// declares no body to reach through one. Anything else — `self`, a
    /// variable holding a class reference — is walked as the expression it is.
    fn receiver(&mut self, class: &Expr) {
        match &class.kind {
            ExprKind::ConstFetch(name) => self.name(name, Kind::Class),
            _ => self.expr(class),
        }
    }

    /// What `new` was written against.
    fn new_target(&mut self, target: &NewTarget) {
        match target {
            NewTarget::Name(name) => self.name(name, Kind::Class),
            NewTarget::Expr(expr) => self.expr(expr),
            NewTarget::AnonClass(decl) => {
                self.opt_name(decl.extends.as_ref(), Kind::Class);
                for implemented in &decl.implements {
                    self.name(implemented, Kind::Interface);
                }
                self.members(&decl.members);
            }
            _ => {}
        }
    }

    /// A member name written as an identifier, or the expression that computes
    /// one.
    fn member(&mut self, name: &MemberName, kind: Kind) {
        match name {
            MemberName::Ident(span) => self.push(*span, kind),
            MemberName::Variable(expr) | MemberName::Expr(expr) => self.expr(expr),
            _ => {}
        }
    }

    /// A call's arguments, with a named argument's label recorded as the
    /// parameter it names.
    fn args(&mut self, args: &CallArgs) {
        let CallArgs::List(list) = args else {
            return;
        };
        for arg in list {
            if let Some(name) = arg.name {
                self.push(name, Kind::Parameter);
            }
            self.expr(&arg.value);
        }
    }

    /// A closure's parameters and body.
    fn closure(&mut self, closure: &FnExpr) {
        self.params(&closure.params);
        match &closure.body {
            FnBody::Expr(expr) => self.expr(expr),
            FnBody::Block(block) => self.block(block),
        }
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};
    use nvs_syntax::parse_file;

    use super::*;
    use crate::render::Response;

    /// Every kind in the walk's vocabulary, so the guard below reads the
    /// whole of it rather than the ones a test happened to reach.
    const EVERY_KIND: [Kind; 10] = [
        Kind::Namespace,
        Kind::Class,
        Kind::Interface,
        Kind::Enum,
        Kind::EnumMember,
        Kind::Type,
        Kind::Method,
        Kind::Property,
        Kind::Parameter,
        Kind::Variable,
    ];

    /// The tokens of `source`, rendered as a `.lspt` case freezes them.
    fn coloured(source: &str) -> String {
        narrowed(source, &[])
    }

    /// The same, narrowed to the token types `keep` names.
    fn narrowed(source: &str, keep: &[String]) -> String {
        let mut map = SourceMap::new();
        let id = map.add("case.nvs", source);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        let tokens = of_stmts(&stmts, map.file(id), PositionEncoding::Utf8, keep);
        Response::SemanticTokens(tokens).render()
    }

    /// `rule:ide/semantic-tokens-carry-the-qualifiers`: the legend the client
    /// registers is the one the server declares, so a kind this walk emits has
    /// to be in it — a kind that is not would be dropped silently at encoding
    /// time.
    #[test]
    fn every_kind_this_walk_emits_is_in_the_declared_legend() {
        for kind in EVERY_KIND {
            assert!(
                kind.index().is_some(),
                "`{:?}` is not in the declared legend",
                kind
            );
        }
    }

    /// A declaration says what it declares, and a position says what may stand
    /// in it — the class its own name, the interface it implements, and each
    /// member the kind it was written as.
    #[test]
    fn a_declaration_and_the_positions_around_it_are_each_named() {
        assert_eq!(
            coloured(
                "<?nvs\ninterface Greets {}\nclass User implements Greets {\n  \
                 public string $name;\n  public function greet(): string { return $this->name; }\n}\n"
            ),
            "2:11+6 interface\n3:7+4 class\n3:23+6 interface\n4:17+5 property\n\
             5:19+5 method\n5:44+5 variable\n5:51+4 property\n"
        );
    }

    /// An enum names itself and each case; a `type` alias names itself.
    #[test]
    fn an_enum_names_its_cases_and_an_alias_names_itself() {
        assert_eq!(
            coloured("<?nvs\ntype Id = uint;\nenum Status: int {\n  case Draft = 1;\n}\n"),
            "2:6+2 type\n3:6+6 enum\n4:8+5 enumMember\n"
        );
    }

    /// The document a `.lspt` case is written about: a trailing `->` leaves a
    /// member name covering no bytes, and everything already typed around it is
    /// still coloured.
    #[test]
    fn an_unfinished_member_access_colours_what_is_written() {
        assert_eq!(
            coloured("<?nvs\nvar $u = new User();\n$u->\n"),
            "2:5+2 variable\n2:14+4 class\n3:1+2 variable\n"
        );
    }

    /// `rule:ide/rejected-syntax-gets-no-colour`: a free function is refused by
    /// `rule:classes/no-free-functions-or-constants`, so its name is coloured
    /// as nothing — and the parameter and the variable inside it still are.
    #[test]
    fn a_free_function_is_not_dressed_as_a_method() {
        assert_eq!(
            coloured("<?nvs\nfunction helper(int $n): int { return $n; }\n"),
            "2:21+2 parameter\n2:39+2 variable\n"
        );
    }

    /// `semanticTokens types=` narrows the answer *before* it is encoded, so a
    /// narrowed case's columns are the ones the whole answer would have put
    /// there — the deltas are relative and a filter applied afterwards would
    /// move every token following a dropped one.
    #[test]
    fn a_narrowed_answer_keeps_the_columns_the_whole_one_had() {
        let source = "<?nvs\nclass User {\n  public string $name;\n}\n";
        assert_eq!(coloured(source), "2:7+4 class\n3:17+5 property\n");
        assert_eq!(
            narrowed(source, &["property".to_owned()]),
            "3:17+5 property\n"
        );
    }

    /// A namespace names itself, and a document that writes no name at all
    /// answers `none` rather than an empty rendering.
    #[test]
    fn a_namespace_is_named_and_a_document_with_no_names_answers_none() {
        assert_eq!(
            coloured("<?nvs\nnamespace App {\n  class User {}\n}\n"),
            "2:11+3 namespace\n3:9+4 class\n"
        );
        assert_eq!(coloured("<?nvs\necho 1;\n"), "none\n");
    }
}
