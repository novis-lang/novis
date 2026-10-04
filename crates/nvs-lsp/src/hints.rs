//! The two things a reader cannot see, written where the source left them out.
//!
//! `rule:ide/every-feature-is-staged-behind-its-dependency` lists inlay hints
//! among what the VS Code client eventually gets, and ADR 0099 § 3 deferred
//! them past M4B for one reason: they "want a settings story and
//! encode idioms that are still moving through M5–M9". Both halves of that have
//! since settled, and this module reverses the deferral **only** for the two
//! idioms that stopped moving:
//!
//! | Shape | Hint |
//! |---|---|
//! | `var $x = …` or a `foreach` binding written `var` (`rule:types/var-inference`) | `: T`, after the name |
//! | a call argument written as a bare literal | `name:`, before the literal |
//!
//! There is no third, and a candidate is a decision rather than an addition:
//! everything else an editor could annotate — a `foreach` binding with its
//! type written, a chained call's intermediate type, an anonymous function's return — is
//! either written in the source already or is a claim about an idiom M5–M9 is
//! still moving.
//!
//! # Decision: a hint is read out of the type phase's table, never re-derived
//!
//! Both shapes are answered from what `nvs_types::check_program` already
//! recorded and from nothing else. A local declaration's type is the binding
//! `crate::Analysed::local_ty` reads out of `ExprTypeTable::local_scopes`, and a
//! `foreach` binding's is `ExprTypeTable::declared_ty` at its `var` keyword; the
//! parameter's name is `nvs_types::ResolvedCall::param_names`, reached through
//! that same call's `arg_slots`, which is the checker's own answer to *which
//! parameter did this written argument fill*. A site the table holds nothing
//! for gets **no hint**, which is `tests/hints.rs`'
//! `no_hint_is_produced_from_anything_the_type_phase_did_not_record` and the
//! whole of this module's failure mode: a hint is text an editor draws as if it
//! were in the file, so one this server guessed at would be a second, unchecked
//! description of the program — the failure
//! `rule:ide/completion-offers-only-what-the-compiler-derived` refuses for
//! completion, and the same answer here.
//!
//! # Decision: the walk matches the AST, on [`crate::semantic`]'s terms
//!
//! [`crate::redactions`] reads `nvs_syntax::walk` because a span and a
//! production's name are all it needs. Both facts this module turns on are
//! **absences** that walk does not model: that a declaration wrote *no* type,
//! and that an argument wrote *no* name. So this matches `nvs_syntax::ast`
//! directly and pays what that crate's module doc names — those enums are
//! `#[non_exhaustive]`, so the match below carries a wildcard and a production
//! landing later reaches no hint and fails no build. The cost is bounded by
//! what a miss means: a hint that is not drawn, never one that is wrong.
//!
//! # Decision: only a literal argument, and only an anonymous one
//!
//! An argument that is a variable, a property or a call already says what it
//! is at the call site; a literal is the one that does not, which is the same
//! bound every editor that ships this feature converged on. A **named**
//! argument (`greet(name: "hi")`) writes the word itself, and a spread or an
//! `inout` marker writes one too, so all three are skipped rather than
//! annotated twice.
//!
//! The document is walked whole and never bounded by a range, which is not a
//! gap: `textDocument/inlayHint` is asked over the visible region and asked
//! again for every region a reader scrolls onto, so a walk bounded by the range
//! would repeat most of itself per frame to save a filter. `crate::server`'s
//! `inlay_hints` is where the answer is narrowed.
//!
//! # Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-lsp/src/hints.rs` lists them.

use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel};
use nvs_diagnostics::{BytePos, PositionEncoding, Span};
use nvs_syntax::ast::{
    Block, CallArgs, ClassMember, ClassMemberKind, Expr, ExprKind, FnBody, FnExpr, ForInit,
    ForeachBinding, ForeachBindingTy, Param, PropertyHook, PropertyHookBody, Stmt, StmtKind,
    TestOperand, Type,
};
use nvs_types::ExprInfo;
use nvs_types::expr_table::ArgSlot;

use crate::document::Analysed;
use crate::position::position_at;

/// Every hint the entry document carries, in source order.
///
/// Source order because a client draws them where they are and a case freezes
/// them as a list: the walk reaches a declaration's name before the call on the
/// line under it, but a `foreach` header's subject after the binding it
/// precedes, so the order a walk produces is not the order a reader sees.
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<InlayHint> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    let mut hinting = Hinting {
        analysed,
        encoding,
        hints: Vec::new(),
    };
    hinting.stmts(&loaded.stmts);
    hinting
        .hints
        .sort_by_key(|hint| (hint.position.line, hint.position.character));
    hinting.hints
}

/// One walk of the entry document, collecting what it finds.
struct Hinting<'a> {
    /// The analysis every hint is read out of.
    analysed: &'a Analysed,
    /// The encoding the client negotiated, which is what a position is in.
    encoding: PositionEncoding,
    /// What the walk has found, in the order it found it.
    hints: Vec<InlayHint>,
}

impl Hinting<'_> {
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

    /// Walks one statement, hinting the declaration it may be.
    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.expr(expr),
            StmtKind::Return(value) => self.opt_expr(value.as_ref()),
            StmtKind::Block(block) => self.block(block),
            StmtKind::If { arms, else_ } => {
                for arm in arms {
                    self.expr(&arm.cond);
                    self.stmt(&arm.then);
                }
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
                if let Some(key) = key {
                    self.foreach_binding(key);
                }
                self.foreach_binding(value);
                self.expr(subject);
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
                    self.block(&catch.body);
                }
                if let Some(finally) = finally {
                    self.block(finally);
                }
            }
            StmtKind::Echo(exprs) | StmtKind::Unset(exprs) => self.exprs(exprs),
            StmtKind::LocalDecl { ty, name, value } => {
                self.declared(ty.as_ref(), *name);
                self.opt_expr(value.as_ref());
            }
            StmtKind::Destructure { value, .. } => self.expr(value),
            StmtKind::StaticLocal { vars, .. } => {
                for var in vars {
                    self.opt_expr(var.default.as_ref());
                }
            }
            StmtKind::ClassDecl(decl) => self.members(&decl.members),
            StmtKind::InterfaceDecl(decl) => self.members(&decl.members),
            StmtKind::EnumDecl(decl) => {
                for case in &decl.cases {
                    self.opt_expr(case.value.as_ref());
                }
                self.members(&decl.members);
            }
            StmtKind::NamespaceDecl(decl) => {
                if let Some(body) = &decl.body {
                    self.block(body);
                }
            }
            // Refused by `rule:classes/no-free-functions-or-constants`, and the
            // body is still walked for [`crate::semantic`]'s reason: a document
            // that does not compile is the one an editor is looking at most.
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
                    self.opt_expr(property.default.as_ref());
                    for hook in property.hooks.iter().flatten() {
                        self.hook(hook);
                    }
                }
                ClassMemberKind::Const(declared) => self.expr(&declared.value),
                ClassMemberKind::Method(method) => {
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

    /// Walks one parameter's default, which is where a call can hide.
    fn param(&mut self, param: &Param) {
        self.opt_expr(param.default.as_ref());
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

    /// Walks one expression, hinting the call it may be.
    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Interpolated(_) | ExprKind::Variable(_) | ExprKind::ConstFetch(_) => {}
            ExprKind::ArrayLiteral(items) => {
                for item in items {
                    self.opt_expr(item.key.as_ref());
                    self.expr(&item.value);
                }
            }
            ExprKind::AnonObject(fields) => {
                for field in fields {
                    self.expr(&field.value);
                }
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::PreIncDec { expr, .. }
            | ExprKind::PostIncDec { expr, .. }
            | ExprKind::Conversion { expr, .. } => self.expr(expr),
            ExprKind::TypeTest { expr, against } => {
                self.expr(expr);
                if let TestOperand::Value(operand) = against {
                    self.expr(operand);
                }
            }
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
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                self.args(expr.span, args);
            }
            ExprKind::MethodCall { object, args, .. } => {
                self.expr(object);
                self.args(expr.span, args);
            }
            ExprKind::StaticCall { args, .. } => self.args(expr.span, args),
            ExprKind::PropertyAccess { object, .. } => self.expr(object),
            ExprKind::Index { base, index } => {
                self.expr(base);
                self.opt_expr(index.as_deref());
            }
            ExprKind::New { args, .. } => self.args(expr.span, args),
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
            ExprKind::Fn(anon_fn) => self.anon_fn(anon_fn),
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

    /// An anonymous function's parameters and body.
    fn anon_fn(&mut self, anon_fn: &FnExpr) {
        self.params(&anon_fn.params);
        match &anon_fn.body {
            FnBody::Expr(expr) => self.expr(expr),
            FnBody::Block(block) => self.block(block),
        }
    }

    /// The inferred type of a `var` declaration, and nothing for one that wrote
    /// its type out.
    ///
    /// The binding rather than the initializer's own expression: `var`'s type
    /// is what the checker *fixed onto the name*, which is the same value a
    /// hand-written annotation would have had to spell, and it is the one a
    /// reader is asking for. `crate::Analysed::local_ty` is that lookup's one
    /// home and answers `None` for a body the type phase never reached.
    fn declared(&mut self, ty: Option<&Type>, name: Span) {
        if ty.is_some() {
            return;
        }
        let Some(inferred) = self.analysed.local_ty(name, name.start) else {
            return;
        };
        let label = format!(": {}", self.analysed.interner.describe(inferred));
        self.push(name.end, label, InlayHintKind::TYPE, false);
    }

    /// The type a `foreach` binding written `var` was given, and nothing for
    /// one that wrote its type out.
    ///
    /// Read from where the checker recorded it, under the keyword's span —
    /// the lookup `nvs-ir`'s lowering makes — rather than by the binding's
    /// name, which a later binding of the same name in another loop would
    /// answer for as well.
    fn foreach_binding(&mut self, binding: &ForeachBinding) {
        let ForeachBindingTy::Var(var) = binding.ty else {
            return;
        };
        let Some(inferred) = self.analysed.exprs.declared_ty(var) else {
            return;
        };
        let label = format!(": {}", self.analysed.interner.describe(inferred));
        self.push(binding.name.end, label, InlayHintKind::TYPE, false);
    }

    /// The parameter each bare-literal argument of this call fills.
    ///
    /// `span` is the call expression's own, which is the key the checker
    /// recorded its `nvs_types::ResolvedCall` under. `arg_slots` is what joins
    /// the two sides: it carries one entry per *written* argument, so a call
    /// that filled a later parameter by name or left a defaulted one out is
    /// still read correctly, and an argument the checker could not place
    /// carries a slot this skips rather than a position it would have guessed.
    fn args(&mut self, span: Span, args: &CallArgs) {
        let CallArgs::List(list) = args else {
            return;
        };
        let analysed = self.analysed;
        let Some(ExprInfo::Call(call)) = analysed.exprs.lookup(span) else {
            return;
        };
        for (written, arg) in list.iter().enumerate() {
            if arg.name.is_some() || arg.spread || arg.inout || !is_literal(&arg.value) {
                continue;
            }
            let Some(ArgSlot::Param(slot)) = call.arg_slots.get(written).copied() else {
                continue;
            };
            let Some(name) = call.param_names.get(slot).filter(|name| !name.is_empty()) else {
                continue;
            };
            let label = format!("{name}:");
            self.push(arg.value.span.start, label, InlayHintKind::PARAMETER, true);
        }
    }

    /// Records one hint at `at`, padded on the side the text it abuts is on.
    ///
    /// A type hint follows the name it describes and needs no space before it —
    /// `$x: int` is how the annotation would have been written. A parameter
    /// hint precedes the value and takes one after it, so `greet(name: "hi")`
    /// reads as the named argument it stands in for.
    fn push(&mut self, at: BytePos, label: String, kind: InlayHintKind, padded: bool) {
        let file = self.analysed.map.file(self.analysed.entry);
        self.hints.push(InlayHint {
            position: position_at(file, at, self.encoding),
            label: InlayHintLabel::String(label),
            kind: Some(kind),
            text_edits: None,
            tooltip: None,
            padding_left: None,
            padding_right: padded.then_some(true),
            data: None,
        });
    }
}

/// Whether `expr` is a literal written out at the call site.
///
/// The bound the module doc's third decision names. An interpolated string and
/// an array literal are not one: each holds expressions, and what a reader
/// cannot see there is what those are rather than which parameter they fill.
fn is_literal(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Null
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Duration(_)
            | ExprKind::Str(_)
    )
}
