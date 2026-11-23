//! A prefix or suffix cut written by hand
//! (`rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`).
//!
//! `Core\Str::afterPrefix(S, P)` tests that `S` starts with `P` and cuts it
//! off in one call, and `Core\Str::beforeSuffix(S, P)` does the same at the
//! end. Where a body writes the test and the cut as two or three calls,
//! the cut is a `W1036` ([`warnings`]). The shapes, and only these:
//!
//! | Written | Named |
//! |---|---|
//! | `Str::slice(S, N)` reached only where `Str::startsWith(S, P)` is `true` | `Str::afterPrefix(S, P)` |
//! | `Str::slice(S, 0, -N)` or `Str::slice(S, 0, Str::length(S) - N)` reached only where `Str::endsWith(S, P)` is `true` | `Str::beforeSuffix(S, P)` |
//!
//! `N` is `Str::length(P)`, with or without `as int`, or an integer literal
//! equal to the length of a literal `P` that is ASCII with no escape and no
//! `\r`, so its byte count is its grapheme count. `S` is a plain variable and
//! `P` a string literal, a constant or a plain variable, and neither is
//! written between the test and the cut. "Reached only where" is the `then`
//! statement of an `if` whose condition is the test or has it as one operand
//! of a chain of `&&`, or a later statement of the same block after an `if`
//! with no `else` whose condition is `!test` (or has it as one operand of a
//! chain of `||`) and whose `then` ends in `return`, `throw`, `continue` or
//! `break`. The match is conservative, as `crate::rereads`' is: an anonymous
//! function anywhere between the test and the cut stops it, and so does a
//! statement or expression the walk does not know.
//!
//! **The fix is offered for one exact shape.** The `if` has one arm and no
//! `else`, its condition is the test alone, and the first statement of its
//! block declares `$v` from the cut, with `var` or `string`. The fix is an
//! alternative (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`) that
//! writes `?string $v = Str::afterPrefix(S, P);` before the `if`, the
//! condition `$v != null`, and the block without its first statement. Every
//! other match warns with no fix, because a wider fix must pick a variable
//! name or move a declaration across code the author wrote.
//!
//! [`find`] runs once per lowered program, beside `crate::rereads::find`.
//!
//! **What it spends:** while compiling, one walk of each body, and for each
//! test it finds, one walk of the statements it guards. At run time, nothing.

use nvs_diagnostics::{Diagnostic, SourceFile, Span, code};
use nvs_syntax::ast::{
    BinaryOp, CallArgs, DestructureElement, DestructureTarget, Expr, ExprKind, Stmt, StmtKind,
    UnaryOp,
};
use nvs_types::expr_table::{ExprInfo, ExprTypeTable};

use crate::rereads::{Part, blocks_of, children, plain, stmt_parts};
use crate::{span_text, strip_sigil};

/// Which end a cut removes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// A prefix, named `Core\Str::afterPrefix`.
    Prefix,
    /// A suffix, named `Core\Str::beforeSuffix`.
    Suffix,
}

/// One cut written by hand.
#[derive(Clone, Debug)]
pub struct HandCut {
    /// The `Str::slice` call, where the warning is.
    pub cut: Span,
    /// The `Str::startsWith` or `Str::endsWith` call that guards it.
    pub test: Span,
    /// Which end it removes.
    pub end: End,
    /// The one call that replaces both, as the author would write it:
    /// `Str::afterPrefix($s, 'x')`.
    pub call: String,
    /// The fix, where [the exact shape](self) holds: the `if` statement's
    /// span and its replacement.
    pub fix: Option<(Span, String)>,
}

/// Every hand-written cut of `files`, in the order of each file.
#[must_use]
pub fn find(files: &[nvs_types::ProgramFile<'_>], exprs: &ExprTypeTable) -> Vec<HandCut> {
    let mut out = Vec::new();
    for file in files {
        let cx = Cx {
            exprs,
            src: file.src,
        };
        let mut blocks = Vec::new();
        blocks_of(file.stmts, &mut blocks);
        let start = out.len();
        for block in blocks {
            cx.scan(block, &mut out);
        }
        let found = &mut out[start..];
        found.sort_by_key(|cut| cut.cut.start);
        let mut seen = rustc_hash::FxHashSet::default();
        let kept: Vec<HandCut> = found
            .iter()
            .filter(|cut| seen.insert(cut.cut))
            .cloned()
            .collect();
        out.truncate(start);
        out.extend(kept);
    }
    out
}

/// The `W1036` warning of each cut.
#[must_use]
pub fn warnings(cuts: &[HandCut]) -> Vec<Diagnostic> {
    cuts.iter()
        .map(|cut| {
            let (label, help) = match cut.end {
                End::Prefix => (
                    "the test of the prefix",
                    "it returns the rest of the string, or `null` when the string does not \
                     start with the prefix",
                ),
                End::Suffix => (
                    "the test of the suffix",
                    "it returns the rest of the string, or `null` when the string does not \
                     end with the suffix",
                ),
            };
            let warning = Diagnostic::warning(
                code::W_HAND_CUT,
                format!("`{}` does this test and this cut in one call", cut.call),
            )
            .with_primary(cut.cut, "the cut")
            .with_secondary(cut.test, label)
            .with_help(help);
            match &cut.fix {
                Some((span, text)) => {
                    warning.with_alternative_fix(*span, text.clone(), "test and cut in one call")
                }
                None => warning,
            }
        })
        .collect()
}

/// A `startsWith` or `endsWith` call whose arguments the match can follow.
struct Test<'a> {
    /// The whole call.
    span: Span,
    end: End,
    /// The class as written: `Str`, `Core\Str` or `\Core\Str`.
    class: &'a str,
    /// The subject variable, without its `$`.
    subject: &'a str,
    /// The prefix or suffix as written.
    affix: &'a Expr,
}

/// What every match reads: the checker's resolved calls and the file's text.
struct Cx<'a> {
    exprs: &'a ExprTypeTable,
    src: &'a SourceFile,
}

impl<'a> Cx<'a> {
    /// The text `span` covers.
    fn text(&self, span: Span) -> &'a str {
        span_text(self.src, span)
    }

    /// `e`'s `Core\Str` method name and arguments, when `e` is a call of one
    /// with every argument written positionally, once and by value.
    fn str_call(&self, e: &'a Expr) -> Option<(&'a str, &'a [nvs_syntax::ast::Arg], &'a Expr)> {
        let ExprKind::StaticCall {
            class,
            args: CallArgs::List(args),
            ..
        } = &e.kind
        else {
            return None;
        };
        let Some(ExprInfo::Call(call)) = self.exprs.lookup(e.span) else {
            return None;
        };
        (call.class.to_string() == r"Core\Str" && args.iter().all(plain)).then_some((
            call.method.as_str(),
            args.as_slice(),
            &**class,
        ))
    }

    /// The name of `e` without its `$`, when `e` is a plain variable other
    /// than `$this`.
    fn variable(&self, e: &Expr) -> Option<&'a str> {
        let ExprKind::Variable(span) = &e.kind else {
            return None;
        };
        let name = strip_sigil(self.text(*span));
        (!name.is_empty() && name != "this").then_some(name)
    }

    /// `e` as a test the match can follow.
    fn test_of(&self, e: &'a Expr) -> Option<Test<'a>> {
        let e = unparen(e);
        let (method, args, class) = self.str_call(e)?;
        let end = match method {
            "startsWith" => End::Prefix,
            "endsWith" => End::Suffix,
            _ => return None,
        };
        let [subject, affix] = args else {
            return None;
        };
        let affix = unparen(&affix.value);
        let fits = match &affix.kind {
            ExprKind::Str(_) | ExprKind::ConstFetch(_) | ExprKind::ClassConstAccess { .. } => true,
            ExprKind::Variable(_) => self.variable(affix).is_some(),
            _ => false,
        };
        fits.then_some(())?;
        Some(Test {
            span: e.span,
            end,
            class: self.text(class.span),
            subject: self.variable(unparen(&subject.value))?,
            affix,
        })
    }

    /// Whether `a` is written as the same prefix or suffix as `affix`.
    fn same(&self, a: &Expr, affix: &Expr) -> bool {
        let a = unparen(a);
        std::mem::discriminant(&a.kind) == std::mem::discriminant(&affix.kind)
            && self.text(a.span) == self.text(affix.span)
    }

    /// Whether `e` is the length of `test`'s prefix or suffix.
    fn is_length(&self, e: &Expr, test: &Test<'_>) -> bool {
        let e = strip(e, self);
        if let ExprKind::Int(span) = &e.kind {
            return int_value(self.text(*span)).is_some()
                && int_value(self.text(*span)) == self.literal_length(test.affix);
        }
        self.length_of(e)
            .is_some_and(|arg| self.same(arg, test.affix))
    }

    /// The argument of a `Core\Str::length` call `e`.
    fn length_of(&self, e: &'a Expr) -> Option<&'a Expr> {
        match self.str_call(e)? {
            ("length", [arg], _) => Some(&arg.value),
            _ => None,
        }
    }

    /// The grapheme length of a string literal that is ASCII, with no escape
    /// and no `\r`, which is its byte length.
    fn literal_length(&self, affix: &Expr) -> Option<u64> {
        let ExprKind::Str(span) = &affix.kind else {
            return None;
        };
        let text = self.text(*span);
        let body = text.get(1..text.len().checked_sub(1)?)?;
        (text.len() >= 2 && body.is_ascii() && !body.contains(['\\', '\r']))
            .then(|| u64::try_from(body.len()).ok())
            .flatten()
    }

    /// Whether `e` is a cut of `test`'s subject at `test`'s prefix or suffix.
    fn is_cut(&self, e: &'a Expr, test: &Test<'_>) -> bool {
        let Some(("slice", args, _)) = self.str_call(e) else {
            return false;
        };
        let subject_is = |arg: &Expr| self.variable(unparen(arg)) == Some(test.subject);
        match (test.end, args) {
            (End::Prefix, [s, n]) => subject_is(&s.value) && self.is_length(&n.value, test),
            (End::Suffix, [s, zero, n]) => {
                let zero = strip(&zero.value, self);
                let zero_is = matches!(&zero.kind, ExprKind::Int(span) if int_value(self.text(*span)) == Some(0));
                subject_is(&s.value)
                    && zero_is
                    && match &strip(&n.value, self).kind {
                        ExprKind::Unary {
                            op: UnaryOp::Neg,
                            expr,
                        } => self.is_length(expr, test),
                        ExprKind::Binary {
                            op: BinaryOp::Sub,
                            lhs,
                            rhs,
                        } => {
                            self.length_of(strip(lhs, self)).is_some_and(subject_is)
                                && self.is_length(rhs, test)
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }

    /// The names a test reads: the subject, and the prefix or suffix when it
    /// is a variable.
    fn names(&self, test: &Test<'a>) -> Vec<&'a str> {
        let mut names = vec![test.subject];
        names.extend(self.variable(test.affix));
        names
    }

    /// Every hand-written cut guarded by an `if` of `stmts`.
    fn scan(&self, stmts: &'a [Stmt], out: &mut Vec<HandCut>) {
        for (i, stmt) in stmts.iter().enumerate() {
            let StmtKind::If { arms, else_ } = &stmt.kind else {
                continue;
            };
            let Some(first) = arms.first() else {
                continue;
            };
            let mut tests = Vec::new();
            operands(&first.cond, BinaryOp::And, &mut tests);
            for operand in tests {
                let Some(test) = self.test_of(operand) else {
                    continue;
                };
                let guarded: &[Stmt] = match &first.then.kind {
                    StmtKind::Block(block) => &block.stmts,
                    _ => std::slice::from_ref(&first.then),
                };
                let start = out.len();
                self.cuts_in(guarded, &test, out);
                let exact = arms.len() == 1
                    && else_.is_none()
                    && unparen(&first.cond).span == test.span
                    && matches!(first.then.kind, StmtKind::Block(_));
                if exact && let Some(cut) = out.get_mut(start) {
                    cut.fix = self.fix(stmt, &first.then, cut.cut, &test);
                }
            }
            if arms.len() != 1 || else_.is_some() || !exits(&first.then) {
                continue;
            }
            let mut negated = Vec::new();
            operands(&first.cond, BinaryOp::Or, &mut negated);
            for operand in negated {
                let ExprKind::Unary {
                    op: UnaryOp::Not,
                    expr,
                } = &unparen(operand).kind
                else {
                    continue;
                };
                if let Some(test) = self.test_of(expr) {
                    self.cuts_in(&stmts[i + 1..], &test, out);
                }
            }
        }
    }

    /// Pushes every cut of `test` in `stmts`, up to the first statement that
    /// writes one of the names the test reads.
    fn cuts_in(&self, stmts: &'a [Stmt], test: &Test<'a>, out: &mut Vec<HandCut>) {
        let names = self.names(test);
        for stmt in stmts {
            if self.stmt_writes(stmt, &names) {
                return;
            }
            let mut found = Vec::new();
            self.cuts_of_stmt(stmt, test, &mut found);
            out.extend(found.into_iter().map(|cut| HandCut {
                cut,
                test: test.span,
                end: test.end,
                call: format!(
                    "{}::{}(${}, {})",
                    test.class,
                    match test.end {
                        End::Prefix => "afterPrefix",
                        End::Suffix => "beforeSuffix",
                    },
                    test.subject,
                    self.text(test.affix.span)
                ),
                fix: None,
            }));
        }
    }

    /// The span of every cut of `test` in `stmt`.
    fn cuts_of_stmt(&self, stmt: &'a Stmt, test: &Test<'_>, out: &mut Vec<Span>) {
        let mut parts = Vec::new();
        stmt_parts(stmt, &mut parts);
        for part in parts {
            match part {
                Part::Expr(e) => self.cuts_of_expr(e, test, out),
                Part::Stmt(s) => self.cuts_of_stmt(s, test, out),
                Part::Block(stmts) => stmts.iter().for_each(|s| self.cuts_of_stmt(s, test, out)),
            }
        }
    }

    /// The span of every cut of `test` in `e`, outside an anonymous function.
    fn cuts_of_expr(&self, e: &'a Expr, test: &Test<'_>, out: &mut Vec<Span>) {
        if self.is_cut(e, test) {
            out.push(e.span);
            return;
        }
        let mut kids = Vec::new();
        children(e, &mut kids);
        kids.into_iter()
            .for_each(|child| self.cuts_of_expr(child, test, out));
    }

    /// Whether `stmt` may write one of `names`. A statement the walk does not
    /// know may.
    fn stmt_writes(&self, stmt: &Stmt, names: &[&str]) -> bool {
        let is = |span: Span| names.contains(&strip_sigil(self.text(span)));
        let binds = match &stmt.kind {
            StmtKind::LocalDecl { name, .. } => is(*name),
            StmtKind::Destructure { target, .. } => self.destructures(target, names),
            StmtKind::Foreach { key, value, .. } => key
                .iter()
                .chain(std::iter::once(value))
                .any(|binding| is(binding.name)),
            StmtKind::Try { catches, .. } => catches.iter().any(|catch| catch.var.is_some_and(is)),
            StmtKind::Global(_) | StmtKind::StaticLocal { .. } | StmtKind::Goto(_) => true,
            _ => false,
        };
        let mut parts = Vec::new();
        binds
            || !stmt_parts(stmt, &mut parts)
            || parts.into_iter().any(|part| match part {
                Part::Expr(e) => self.expr_writes(e, names),
                Part::Stmt(s) => self.stmt_writes(s, names),
                Part::Block(stmts) => stmts.iter().any(|s| self.stmt_writes(s, names)),
            })
    }

    /// Whether `target` binds one of `names`.
    fn destructures(&self, target: &DestructureTarget, names: &[&str]) -> bool {
        target.elements.iter().any(|element| match element {
            DestructureElement::Leaf { name, .. } => names.contains(&strip_sigil(self.text(*name))),
            DestructureElement::Nested { target, .. } => self.destructures(target, names),
            _ => false,
        })
    }

    /// Whether `e` may write one of `names`. An anonymous function and an
    /// expression the walk does not know may.
    fn expr_writes(&self, e: &Expr, names: &[&str]) -> bool {
        let named = |target: &Expr| {
            let mut target = target;
            loop {
                match &target.kind {
                    ExprKind::Index { base, .. } => target = base,
                    ExprKind::Paren(inner) => target = inner,
                    _ => break,
                }
            }
            self.variable(target)
                .is_some_and(|name| names.contains(&name))
        };
        let own = match &e.kind {
            ExprKind::Fn(_) => true,
            ExprKind::Assign { target, by_ref, .. } => *by_ref || named(target),
            ExprKind::PreIncDec { expr, .. } | ExprKind::PostIncDec { expr, .. } => named(expr),
            ExprKind::Call { args, .. }
            | ExprKind::MethodCall { args, .. }
            | ExprKind::StaticCall { args, .. } => match args {
                CallArgs::List(args) => args.iter().any(|arg| arg.inout && named(&arg.value)),
                CallArgs::MethodRef => false,
            },
            ExprKind::ArrayLiteral(items) => items.iter().any(|item| item.by_ref),
            ExprKind::Catch { arms, .. } => arms.iter().any(|arm| {
                arm.var
                    .is_some_and(|var| names.contains(&strip_sigil(self.text(var))))
            }),
            _ => false,
        };
        let mut kids = Vec::new();
        own || !children(e, &mut kids) || kids.into_iter().any(|kid| self.expr_writes(kid, names))
    }

    /// The fix for the exact shape: `if_stmt` has one arm, no `else`, and the
    /// test alone as its condition, and `then` is its block. Its first
    /// statement must declare a variable from `cut` with `var` or `string`.
    fn fix(
        &self,
        if_stmt: &Stmt,
        then: &Stmt,
        cut: Span,
        test: &Test<'_>,
    ) -> Option<(Span, String)> {
        let StmtKind::Block(block) = &then.kind else {
            return None;
        };
        let first = block.stmts.first()?;
        let StmtKind::LocalDecl {
            ty,
            name,
            value: Some(value),
        } = &first.kind
        else {
            return None;
        };
        if unparen(value).span != cut
            || ty.as_ref().is_some_and(|ty| self.text(ty.span) != "string")
        {
            return None;
        }
        let whole = self.src.text();
        let start = if_stmt.span.start as usize;
        let line_start = whole[..start].rfind('\n').map_or(0, |at| at + 1);
        let indent = &whole[line_start..start];
        if !indent.chars().all(|c| c == ' ' || c == '\t') {
            return None;
        }
        let variable = self.text(*name);
        let call = format!(
            "{}::{}(${}, {})",
            test.class,
            match test.end {
                End::Prefix => "afterPrefix",
                End::Suffix => "beforeSuffix",
            },
            test.subject,
            self.text(test.affix.span)
        );
        let rest = whole.get(first.span.end as usize..then.span.end as usize)?;
        Some((
            if_stmt.span,
            format!("?string {variable} = {call};\n{indent}if ({variable} != null) {{{rest}"),
        ))
    }
}

/// `e` without its parentheses.
fn unparen(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Paren(inner) => unparen(inner),
        _ => e,
    }
}

/// `e` without its parentheses and its conversions to `int`.
fn strip<'e>(e: &'e Expr, cx: &Cx<'_>) -> &'e Expr {
    match &e.kind {
        ExprKind::Paren(inner) => strip(inner, cx),
        ExprKind::Conversion { expr, ty, .. } if cx.text(ty.span) == "int" => strip(expr, cx),
        _ => e,
    }
}

/// The value of a decimal integer literal.
fn int_value(text: &str) -> Option<u64> {
    text.replace('_', "").parse().ok()
}

/// Pushes each operand of a chain of `op` in `e`: `a && b && c` gives `a`,
/// `b` and `c`, and any other expression gives itself.
fn operands<'a>(e: &'a Expr, op: BinaryOp, out: &mut Vec<&'a Expr>) {
    match &unparen(e).kind {
        ExprKind::Binary {
            op: found,
            lhs,
            rhs,
        } if *found == op => {
            operands(lhs, op, out);
            operands(rhs, op, out);
        }
        _ => out.push(e),
    }
}

/// Whether `then` always leaves its block: a `return`, `throw`, `continue`
/// or `break`, alone or as a block's last statement.
fn exits(then: &Stmt) -> bool {
    match &then.kind {
        StmtKind::Return(_) | StmtKind::Continue(_) | StmtKind::Break(_) => true,
        StmtKind::Expr(e) => matches!(unparen(e).kind, ExprKind::Throw(_)),
        StmtKind::Block(block) => block.stmts.last().is_some_and(exits),
        _ => false,
    }
}
