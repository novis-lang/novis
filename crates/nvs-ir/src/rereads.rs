//! The same value read twice
//! (`rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`).
//!
//! A [`Row`] is one reading `Core` member whose second call does the whole
//! work of the first again. Where a body calls one twice and nothing between
//! the two calls can change the answer, the second call is a `W1035`
//! ([`warnings`]). Keeping the first result in a variable is the fix, and it
//! needs a name only the program's author can choose, so the warning carries
//! none.
//!
//! The rows are three, and the table is closed:
//!
//! - [`Row::Cache`]: `$s->get(K)` of `Core\Cache\Store`, with the same plain
//!   variable as the receiver and the same string literal as the key. Each
//!   read copies the whole stored value.
//! - [`Row::Query`]: `$u->queryParameter(...)` of `Core\Uri`, with the same
//!   plain variable as the receiver. Each call parses the whole query, and
//!   `Core\Uri::parseQuery` parses it once, so the keys may differ.
//! - [`Row::File`]: `Core\IO::isFile`, `isDir`, `size` and `modifiedAt`, with
//!   the same plain variable as the path. Each call asks the file system, and
//!   `Core\IO::stat` asks once, so any two of the four pair.
//!
//! A session read is not a row. Under `nvs serve` with a 16 KiB record, a
//! second `Core\Session::get` of one key cost the request 1.0x, so the rule
//! declines it.
//!
//! The match is conservative, and where it is unsure it says nothing. The
//! first call runs whenever its statement does: it is not on the right of
//! `&&`, `||` or `??`, not in a ternary or `match` arm, not after `?->` and
//! not in an anonymous function or class. The second is in a later statement
//! of the same block, anywhere outside an anonymous function or class. From
//! the first call's statement to the second's, nothing writes the variable,
//! and no code that can change the answer runs: no callable is called, no
//! anonymous function is written, no method outside `Core` is called, no
//! `Core` member that takes a callable is called (its callback is Novis
//! code), no class outside `Core` is built, no property is read or written,
//! nothing suspends, and no `Core` call that [`Row::may_change`] names runs:
//! for a file, that is every class but plain computation, because an open
//! handle or a spawned process changes the file too.
//! [`find`] runs once per
//! lowered program, beside `crate::forms::run`.
//!
//! **What it spends:** while compiling, one walk of each body. At run time,
//! nothing.

use nvs_diagnostics::{Diagnostic, SourceFile, Span, code};
use nvs_syntax::ast::{
    Arg, AssignOp, BinaryOp, CallArgs, ClassMember, ClassMemberKind, DestructureElement,
    DestructureTarget, Expr, ExprKind, FnBody, ForInit, MemberName, NewTarget, PropertyHookBody,
    Stmt, StmtKind, StringPart, TestOperand,
};
use nvs_types::expr_table::{ExprInfo, ExprTypeTable, ResolvedCall};

use crate::{span_text, strip_sigil};

/// Which reading member was called twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// `Core\Cache\Store::get` with one literal key.
    Cache,
    /// `Core\Uri::queryParameter`.
    Query,
    /// `Core\IO::isFile`, `isDir`, `size` and `modifiedAt` of one path.
    File,
}

impl Row {
    /// The class whose members this row reads.
    fn class(self) -> &'static str {
        match self {
            Self::Cache => r"Core\Cache\Store",
            Self::Query => r"Core\Uri",
            Self::File => r"Core\IO",
        }
    }

    /// The members of [`Self::class`] this row reads. Every other member of
    /// the class may change the answer.
    fn members(self) -> &'static [&'static str] {
        match self {
            Self::Cache => &["get"],
            Self::Query => &["queryParameter"],
            Self::File => &["isFile", "isDir", "size", "modifiedAt"],
        }
    }

    /// Whether a call of a `Core` class other than this row's reading
    /// members may change the row's answer. A query is a value, so only its
    /// own class touches it. A cache store shares its entries with every
    /// other store and with `Core\Cache`, so any class under `Core\Cache`
    /// may. A file is changed by an open handle, a spawned process, a
    /// database or an archive alike, so every class that is not plain
    /// computation may.
    fn may_change(self, class: &str) -> bool {
        match self {
            Self::Query => class == self.class(),
            Self::Cache => class == r"Core\Cache" || class.starts_with(r"Core\Cache\"),
            Self::File => !FILE_QUIET.contains(&class),
        }
    }
}

/// The `Core` classes that compute and never touch a file, which a pair of
/// [`Row::File`] reads may have between them.
const FILE_QUIET: &[&str] = &[
    r"Core\Str",
    r"Core\Math",
    r"Core\Arr",
    r"Core\Regex",
    r"Core\Path",
];

/// One second call that does the first one's work again.
#[derive(Clone, Debug)]
pub struct Reread {
    /// The first call.
    pub first: Span,
    /// The second call, where the warning is.
    pub second: Span,
    /// The row both calls are.
    pub row: Row,
    /// What the message names: the cache key without its quotes, the `Uri`
    /// variable, or the path variable, each `$` included for a variable.
    pub subject: String,
}

/// Every reread of `files`, in the order of each file and then of each
/// second call.
#[must_use]
pub fn find(
    files: &[nvs_types::ProgramFile<'_>],
    exprs: &ExprTypeTable,
    types: &nvs_types::TypeInterner,
) -> Vec<Reread> {
    let mut out = Vec::new();
    for file in files {
        let cx = Cx {
            exprs,
            types,
            src: file.src,
        };
        let mut blocks = Vec::new();
        blocks_of(file.stmts, &mut blocks);
        let start = out.len();
        for block in blocks {
            cx.scan(block, &mut out);
        }
        out[start..].sort_by_key(|reread| reread.second.start);
    }
    out
}

/// The `W1035` warning of each reread.
#[must_use]
pub fn warnings(rows: &[Reread]) -> Vec<Diagnostic> {
    rows.iter()
        .map(|reread| {
            let (message, help) = match reread.row {
                Row::Cache => (
                    format!(
                        "this reads cache key `{}` again, and each read copies the whole value",
                        reread.subject
                    ),
                    "keep the first result in a variable and use it again".to_owned(),
                ),
                Row::Query => (
                    format!("this parses the query of `{}` again", reread.subject),
                    "call `Core\\Uri::parseQuery` once and read each parameter from its result"
                        .to_owned(),
                ),
                Row::File => (
                    format!("this asks the file system about `{}` again", reread.subject),
                    "call `Core\\IO::stat` once and read each fact from its result".to_owned(),
                ),
            };
            Diagnostic::warning(code::W_REPEATED_READ, message)
                .with_primary(reread.second, "the second read")
                .with_secondary(reread.first, "the first read")
                .with_help(help)
        })
        .collect()
}

/// One call of a row's member.
struct Read<'a> {
    span: Span,
    row: Row,
    /// The receiver or path variable, without its `$`.
    var: &'a str,
    /// The cache key's literal, quotes included, for [`Row::Cache`].
    key: Option<&'a str>,
}

impl Read<'_> {
    /// Whether `later` does this read's work again.
    fn repeated_by(&self, later: &Read<'_>) -> bool {
        self.row == later.row && self.var == later.var && self.key == later.key
    }
}

/// What every match reads: the checker's resolved calls and the file's text.
struct Cx<'a> {
    exprs: &'a ExprTypeTable,
    types: &'a nvs_types::TypeInterner,
    src: &'a SourceFile,
}

impl<'a> Cx<'a> {
    /// The resolved call at `e`, when `e` is one.
    fn resolved(&self, e: &Expr) -> Option<&'a ResolvedCall> {
        match self.exprs.lookup(e.span) {
            Some(ExprInfo::Call(call)) => Some(call),
            _ => None,
        }
    }

    /// The name of `e` without its `$`, when `e` is a plain variable other
    /// than `$this`.
    fn variable(&self, e: &Expr) -> Option<&'a str> {
        let ExprKind::Variable(span) = &e.kind else {
            return None;
        };
        let name = strip_sigil(span_text(self.src, *span));
        (!name.is_empty() && name != "this").then_some(name)
    }

    /// `e` as a call of a row's member.
    fn read_of(&self, e: &'a Expr) -> Option<Read<'a>> {
        match &e.kind {
            ExprKind::MethodCall {
                object,
                nullsafe: false,
                args: CallArgs::List(args),
                ..
            } => {
                let var = self.variable(object)?;
                let call = self.resolved(e)?;
                let class = call.class.to_string();
                if class == Row::Cache.class() && call.method == "get" {
                    let [arg] = args.as_slice() else {
                        return None;
                    };
                    if !plain(arg) {
                        return None;
                    }
                    let ExprKind::Str(literal) = &arg.value.kind else {
                        return None;
                    };
                    let key = span_text(self.src, *literal);
                    (key.len() >= 2).then_some(Read {
                        span: e.span,
                        row: Row::Cache,
                        var,
                        key: Some(key),
                    })
                } else if class == Row::Query.class() && call.method == "queryParameter" {
                    args.iter().all(plain).then_some(Read {
                        span: e.span,
                        row: Row::Query,
                        var,
                        key: None,
                    })
                } else {
                    None
                }
            }
            ExprKind::StaticCall {
                args: CallArgs::List(args),
                ..
            } => {
                let call = self.resolved(e)?;
                if call.class.to_string() != Row::File.class()
                    || !Row::File.members().contains(&call.method.as_str())
                {
                    return None;
                }
                let [arg] = args.as_slice() else {
                    return None;
                };
                if !plain(arg) {
                    return None;
                }
                Some(Read {
                    span: e.span,
                    row: Row::File,
                    var: self.variable(&arg.value)?,
                    key: None,
                })
            }
            _ => None,
        }
    }

    /// Every reread whose two calls are statements of `stmts`. Each later
    /// read pairs with the earliest first read it repeats, so a third read
    /// warns too.
    fn scan(&self, stmts: &'a [Stmt], out: &mut Vec<Reread>) {
        let mut firsts: Vec<(usize, Read<'a>)> = Vec::new();
        for (i, stmt) in stmts.iter().enumerate() {
            let mut seconds = Vec::new();
            if !firsts.is_empty() {
                self.every_read_of_stmt(stmt, &mut seconds);
            }
            for second in &seconds {
                let first = firsts.iter().find(|(at, first)| {
                    first.repeated_by(second)
                        && self.nothing_changes(&stmts[*at..=i], [first.span, second.span], first)
                });
                if let Some((_, first)) = first {
                    out.push(Reread {
                        first: first.span,
                        second: second.span,
                        row: second.row,
                        subject: match second.key {
                            Some(key) => key[1..key.len() - 1].to_owned(),
                            None => format!("${}", second.var),
                        },
                    });
                }
            }
            for head in heads(stmt) {
                self.unconditional_reads(head, &mut firsts, i);
            }
        }
    }

    /// The reads `e` makes whenever it runs, each paired with `at`.
    fn unconditional_reads(&self, e: &'a Expr, out: &mut Vec<(usize, Read<'a>)>, at: usize) {
        if let Some(read) = self.read_of(e) {
            out.push((at, read));
        }
        let mut walk = |child: &'a Expr| self.unconditional_reads(child, out, at);
        match &e.kind {
            ExprKind::Binary {
                op: BinaryOp::And | BinaryOp::Or | BinaryOp::Coalesce,
                lhs,
                ..
            } => walk(lhs),
            ExprKind::Ternary { cond, .. } => walk(cond),
            ExprKind::Match { subject, .. } => walk(subject),
            ExprKind::Catch { guarded, .. } => walk(guarded),
            ExprKind::Assign {
                op, target, value, ..
            } => {
                walk(target);
                if always_assigns(*op) {
                    walk(value);
                }
            }
            ExprKind::MethodCall { object, .. } | ExprKind::PropertyAccess { object, .. }
                if short_circuits(e) =>
            {
                walk(object);
            }
            ExprKind::Index { base, .. } if short_circuits(e) => walk(base),
            ExprKind::Fn(_)
            | ExprKind::New {
                target: NewTarget::AnonClass(_),
                ..
            } => {}
            _ => {
                let mut kids = Vec::new();
                children(e, &mut kids);
                kids.into_iter().for_each(walk);
            }
        }
    }

    /// Every read in `stmt`, nested statements included, outside an
    /// anonymous function or class.
    fn every_read_of_stmt(&self, stmt: &'a Stmt, out: &mut Vec<Read<'a>>) {
        let mut parts = Vec::new();
        stmt_parts(stmt, &mut parts);
        for part in parts {
            match part {
                Part::Expr(e) => self.every_read(e, out),
                Part::Stmt(s) => self.every_read_of_stmt(s, out),
                Part::Block(stmts) => stmts.iter().for_each(|s| self.every_read_of_stmt(s, out)),
            }
        }
    }

    /// Every read in `e`, outside an anonymous function or class.
    fn every_read(&self, e: &'a Expr, out: &mut Vec<Read<'a>>) {
        if let Some(read) = self.read_of(e) {
            out.push(read);
        }
        if matches!(
            e.kind,
            ExprKind::Fn(_)
                | ExprKind::New {
                    target: NewTarget::AnonClass(_),
                    ..
                }
        ) {
            return;
        }
        let mut kids = Vec::new();
        children(e, &mut kids);
        kids.into_iter()
            .for_each(|child| self.every_read(child, out));
    }

    /// Whether nothing in `stmts`, apart from the two calls at `reads`,
    /// writes `first`'s variable or runs code that can change its answer.
    fn nothing_changes(&self, stmts: &[Stmt], reads: [Span; 2], first: &Read<'_>) -> bool {
        let guard = Guard {
            cx: self,
            reads,
            row: first.row,
            var: first.var,
        };
        stmts.iter().all(|stmt| guard.stmt(stmt))
    }
}

/// What [`Cx::nothing_changes`] checks one statement range against.
struct Guard<'g, 'a> {
    cx: &'g Cx<'a>,
    reads: [Span; 2],
    row: Row,
    var: &'g str,
}

impl Guard<'_, '_> {
    /// Whether `e` names the variable, through any index.
    fn writes_through(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Index { base, .. } => self.writes_through(base),
            ExprKind::Paren(inner) => self.writes_through(inner),
            ExprKind::Variable(_) => self.cx.variable(e) == Some(self.var),
            _ => false,
        }
    }

    /// Whether `name` is the variable's.
    fn is_var(&self, name: Span) -> bool {
        strip_sigil(span_text(self.cx.src, name)) == self.var
    }

    /// Whether `target` binds the variable.
    fn destructures(&self, target: &DestructureTarget) -> bool {
        target.elements.iter().any(|element| match element {
            DestructureElement::Leaf { name, .. } => self.is_var(*name),
            DestructureElement::Nested { target, .. } => self.destructures(target),
            _ => false,
        })
    }

    /// Whether a call's arguments leave the answer as it is.
    fn args(&self, args: &CallArgs) -> bool {
        match args {
            CallArgs::List(args) => !args
                .iter()
                .any(|arg| arg.inout && self.writes_through(&arg.value)),
            CallArgs::MethodRef => false,
        }
    }

    /// Whether `stmt` writes nothing the answer depends on.
    fn stmt(&self, stmt: &Stmt) -> bool {
        let binds = match &stmt.kind {
            StmtKind::Unset(targets) => targets.iter().any(|t| self.writes_through(t)),
            StmtKind::LocalDecl { name, .. } => self.is_var(*name),
            StmtKind::Destructure { target, .. } => self.destructures(target),
            StmtKind::Foreach { key, value, .. } => key
                .iter()
                .chain(std::iter::once(value))
                .any(|binding| self.is_var(binding.name)),
            StmtKind::Try { catches, .. } => catches
                .iter()
                .any(|catch| catch.var.is_some_and(|var| self.is_var(var))),
            StmtKind::Global(_) | StmtKind::StaticLocal { .. } | StmtKind::Goto(_) => true,
            _ => false,
        };
        if binds {
            return false;
        }
        let mut parts = Vec::new();
        if !stmt_parts(stmt, &mut parts) {
            return false;
        }
        parts.into_iter().all(|part| match part {
            Part::Expr(e) => self.expr(e),
            Part::Stmt(s) => self.stmt(s),
            Part::Block(stmts) => stmts.iter().all(|s| self.stmt(s)),
        })
    }

    /// Whether `e` writes nothing the answer depends on.
    fn expr(&self, e: &Expr) -> bool {
        let own = self.reads.contains(&e.span)
            || match &e.kind {
                ExprKind::Call { .. }
                | ExprKind::Fn(_)
                | ExprKind::Clone(_)
                | ExprKind::Yield { .. }
                | ExprKind::YieldFrom(_)
                | ExprKind::Await(_)
                | ExprKind::SpawnScript { .. }
                | ExprKind::Require { .. }
                | ExprKind::PropertyAccess { .. }
                | ExprKind::StaticPropertyAccess { .. } => false,
                ExprKind::MethodCall { args, .. } | ExprKind::StaticCall { args, .. } => {
                    self.core_call(e) && self.args(args)
                }
                ExprKind::New { target, .. } => {
                    matches!(target, NewTarget::Name(_))
                        && matches!(
                            self.cx.exprs.lookup(e.span),
                            Some(ExprInfo::New { class, .. }) if in_core(&class.to_string())
                        )
                }
                ExprKind::Assign { target, by_ref, .. } => !by_ref && !self.writes_through(target),
                ExprKind::PreIncDec { expr, .. } | ExprKind::PostIncDec { expr, .. } => {
                    !self.writes_through(expr)
                }
                ExprKind::ArrayLiteral(items) => !items.iter().any(|item| item.by_ref),
                ExprKind::Catch { arms, .. } => !arms
                    .iter()
                    .any(|arm| arm.var.is_some_and(|var| self.is_var(var))),
                _ => true,
            };
        if !own {
            return false;
        }
        let mut kids = Vec::new();
        children(e, &mut kids) && kids.into_iter().all(|child| self.expr(child))
    }

    /// Whether the call at `e` runs `Core` code that leaves the row's answer
    /// as it is.
    fn core_call(&self, e: &Expr) -> bool {
        let Some(call) = self.cx.resolved(e) else {
            return false;
        };
        let class = call.class.to_string();
        in_core(&class)
            && (self.row.members().contains(&call.method.as_str()) && class == self.row.class()
                || !self.row.may_change(&class))
            && !call
                .param_tys
                .iter()
                .any(|&ty| takes_code(self.cx.types, ty))
    }
}

/// Whether a parameter of type `ty` can be given Novis code to call: a
/// callable, or a shape, union or array that has one.
fn takes_code(types: &nvs_types::TypeInterner, ty: nvs_types::ty::TypeId) -> bool {
    use nvs_types::ty::Ty;
    match types.get(ty) {
        Ty::Callable | Ty::CallableSig { .. } | Ty::ShapeOfCallables(_) => true,
        Ty::Shape(fields) => fields.iter().any(|field| takes_code(types, field.ty)),
        Ty::Union(tys) => tys.iter().any(|&ty| takes_code(types, ty)),
        Ty::Array(element) => takes_code(types, *element),
        _ => false,
    }
}

/// Whether `class` is one of `Core`'s.
fn in_core(class: &str) -> bool {
    class.starts_with(r"Core\")
}

/// Whether `arg` is written positionally, once, and by value.
fn plain(arg: &Arg) -> bool {
    arg.name.is_none() && !arg.spread && !arg.inout
}

/// Whether an assignment with `op` always runs its value: every operator but
/// the `??` ones.
fn always_assigns(op: AssignOp) -> bool {
    matches!(
        op,
        AssignOp::Assign
            | AssignOp::AddAssign
            | AssignOp::SubAssign
            | AssignOp::MulAssign
            | AssignOp::DivAssign
            | AssignOp::ModAssign
            | AssignOp::PowAssign
            | AssignOp::ConcatAssign
            | AssignOp::BitAndAssign
            | AssignOp::BitOrAssign
            | AssignOp::BitXorAssign
            | AssignOp::ShlAssign
            | AssignOp::ShrAssign
    )
}

/// Whether a `?->` in the chain of `e` can skip the rest of `e`.
fn short_circuits(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::MethodCall {
            object, nullsafe, ..
        }
        | ExprKind::PropertyAccess {
            object, nullsafe, ..
        } => *nullsafe || short_circuits(object),
        ExprKind::Index { base, .. } => short_circuits(base),
        ExprKind::Paren(inner) => short_circuits(inner),
        _ => false,
    }
}

/// The expressions of `stmt` that run whenever it does: an `if`'s first
/// condition, a loop's subject or condition, and the whole of a simple
/// statement.
fn heads(stmt: &Stmt) -> Vec<&Expr> {
    match &stmt.kind {
        StmtKind::Expr(e) | StmtKind::Return(Some(e)) => vec![e],
        StmtKind::Echo(values) => values.iter().collect(),
        StmtKind::LocalDecl { value: Some(e), .. }
        | StmtKind::Destructure { value: e, .. }
        | StmtKind::While { cond: e, .. }
        | StmtKind::Foreach { subject: e, .. }
        | StmtKind::Switch { subject: e, .. } => vec![e],
        StmtKind::If { arms, .. } => arms.first().map(|arm| &arm.cond).into_iter().collect(),
        StmtKind::For {
            init: ForInit::Exprs(init),
            ..
        } => init.iter().collect(),
        StmtKind::For {
            init: ForInit::Decl(decl),
            ..
        } => heads(decl),
        _ => Vec::new(),
    }
}

/// One thing a statement contains.
enum Part<'a> {
    /// An expression of the statement itself.
    Expr(&'a Expr),
    /// A nested statement.
    Stmt(&'a Stmt),
    /// A nested statement list: a block, a `switch` case, a `try` part.
    Block(&'a [Stmt]),
}

/// Pushes what `stmt` runs onto `out`. A declaration runs nothing, and its
/// bodies are reached by [`blocks_of`]. False for a statement this walk does
/// not know.
fn stmt_parts<'a>(stmt: &'a Stmt, out: &mut Vec<Part<'a>>) -> bool {
    match &stmt.kind {
        StmtKind::Expr(e) => out.push(Part::Expr(e)),
        StmtKind::Return(value) | StmtKind::Break(value) | StmtKind::Continue(value) => {
            out.extend(value.iter().map(Part::Expr));
        }
        StmtKind::Block(block) => out.push(Part::Block(&block.stmts)),
        StmtKind::If { arms, else_ } => {
            for arm in arms {
                out.push(Part::Expr(&arm.cond));
                out.push(Part::Stmt(&arm.then));
            }
            out.extend(else_.iter().map(|s| Part::Stmt(s)));
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            out.push(Part::Expr(cond));
            out.push(Part::Stmt(body));
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            match init {
                ForInit::Decl(decl) => out.push(Part::Stmt(decl)),
                ForInit::Exprs(exprs) => out.extend(exprs.iter().map(Part::Expr)),
            }
            out.extend(cond.iter().chain(step).map(Part::Expr));
            out.push(Part::Stmt(body));
        }
        StmtKind::Foreach { subject, body, .. } => {
            out.push(Part::Expr(subject));
            out.push(Part::Stmt(body));
        }
        StmtKind::Switch { subject, cases } => {
            out.push(Part::Expr(subject));
            for case in cases {
                out.extend(case.cond.iter().map(Part::Expr));
                out.push(Part::Block(&case.body));
            }
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            out.push(Part::Block(&body.stmts));
            for catch in catches {
                out.push(Part::Block(&catch.body.stmts));
            }
            out.extend(finally.iter().map(|block| Part::Block(&block.stmts)));
        }
        StmtKind::Echo(values) | StmtKind::Unset(values) => {
            out.extend(values.iter().map(Part::Expr));
        }
        StmtKind::LocalDecl { value, .. } => out.extend(value.iter().map(Part::Expr)),
        StmtKind::Destructure { target, value } => {
            destructure_keys(target, out);
            out.push(Part::Expr(value));
        }
        StmtKind::NamespaceDecl(decl) => {
            out.extend(decl.body.iter().map(|block| Part::Block(&block.stmts)));
        }
        StmtKind::Empty
        | StmtKind::InlineHtml(_)
        | StmtKind::ClassDecl(_)
        | StmtKind::InterfaceDecl(_)
        | StmtKind::EnumDecl(_)
        | StmtKind::UseDecl(_)
        | StmtKind::AutoloadDecl(_)
        | StmtKind::TypeAliasDecl(_)
        | StmtKind::AttributeDecl(_)
        | StmtKind::TopLevelFunction(_)
        | StmtKind::TopLevelConst(_) => {}
        _ => return false,
    }
    true
}

/// Pushes the key expressions of a destructuring pattern.
fn destructure_keys<'a>(target: &'a DestructureTarget, out: &mut Vec<Part<'a>>) {
    for element in &target.elements {
        match element {
            DestructureElement::Leaf { key, .. } => out.extend(key.iter().map(Part::Expr)),
            DestructureElement::Nested { key, target, .. } => {
                out.extend(key.iter().map(Part::Expr));
                destructure_keys(target, out);
            }
            _ => {}
        }
    }
}

/// Pushes the expressions `e` contains directly. An anonymous function's
/// body and an anonymous class's members are bodies of their own and are not
/// among them. False for an expression this walk does not know.
fn children<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) -> bool {
    match &e.kind {
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Duration(_)
        | ExprKind::Str(_)
        | ExprKind::Variable(_)
        | ExprKind::ConstFetch(_)
        | ExprKind::SelfExpr
        | ExprKind::StaticExpr
        | ExprKind::ParentExpr
        | ExprKind::Fn(_)
        | ExprKind::Error(_) => {}
        ExprKind::Interpolated(parts) | ExprKind::HtmlTemplate(parts) => {
            for part in parts {
                if let StringPart::Expr(inner) = part {
                    out.push(inner);
                }
            }
        }
        ExprKind::ArrayLiteral(items) => {
            for item in items {
                out.extend(item.key.iter());
                out.push(&item.value);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::PreIncDec { expr, .. }
        | ExprKind::PostIncDec { expr, .. }
        | ExprKind::Conversion { expr, .. }
        | ExprKind::Clone(expr)
        | ExprKind::YieldFrom(expr)
        | ExprKind::Print(expr)
        | ExprKind::Throw(expr)
        | ExprKind::Empty(expr)
        | ExprKind::IsInit(expr)
        | ExprKind::Await(expr)
        | ExprKind::Paren(expr)
        | ExprKind::Require { path: expr }
        | ExprKind::ClassNameConst { class: expr }
        | ExprKind::StaticPropertyAccess { class: expr, .. }
        | ExprKind::ClassConstAccess { class: expr, .. } => out.push(expr),
        ExprKind::Binary { lhs, rhs, .. } => {
            out.push(lhs);
            out.push(rhs);
        }
        ExprKind::Assign { target, value, .. } => {
            out.push(target);
            out.push(value);
        }
        ExprKind::Ternary { cond, then, else_ } => {
            out.push(cond);
            out.extend(then.iter().map(|then| &**then));
            out.push(else_);
        }
        ExprKind::TypeTest { expr, against } => {
            out.push(expr);
            if let TestOperand::Value(value) = against {
                out.push(value);
            }
        }
        ExprKind::Call { callee, args } => {
            out.push(callee);
            call_args(args, out);
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            out.push(object);
            member_name(method, out);
            call_args(args, out);
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
            ..
        } => {
            out.push(class);
            member_name(method, out);
            call_args(args, out);
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => {
            out.push(object);
            member_name(property, out);
        }
        ExprKind::Index { base, index } => {
            out.push(base);
            out.extend(index.iter().map(|index| &**index));
        }
        ExprKind::New { target, args, .. } => {
            if let NewTarget::Expr(target) = target {
                out.push(target);
            }
            call_args(args, out);
        }
        ExprKind::Match { subject, arms } => {
            out.push(subject);
            for arm in arms {
                out.extend(arm.conditions.iter().flatten());
                out.push(&arm.body);
            }
        }
        ExprKind::Catch { guarded, arms } => {
            out.push(guarded);
            out.extend(arms.iter().map(|arm| &arm.body));
        }
        ExprKind::Yield { key, value } => {
            out.extend(key.iter().map(|key| &**key));
            out.extend(value.iter().map(|value| &**value));
        }
        ExprKind::Isset(targets) => out.extend(targets.iter()),
        ExprKind::Exit(status) => out.extend(status.iter().map(|status| &**status)),
        ExprKind::SpawnScript { path, options } => {
            out.push(path);
            out.extend(options.iter().map(|option| &option.value));
        }
        ExprKind::AnonObject(fields) => out.extend(fields.iter().map(|field| &field.value)),
        _ => return false,
    }
    true
}

/// Pushes the argument values of a call.
fn call_args<'a>(args: &'a CallArgs, out: &mut Vec<&'a Expr>) {
    if let CallArgs::List(args) = args {
        out.extend(args.iter().map(|arg| &arg.value));
    }
}

/// Pushes the expression a computed member name is written with.
fn member_name<'a>(name: &'a MemberName, out: &mut Vec<&'a Expr>) {
    if let MemberName::Variable(e) | MemberName::Expr(e) = name {
        out.push(e);
    }
}

/// Pushes every statement list under `stmts`, `stmts` included: each nested
/// block, each method, hook and anonymous function body.
fn blocks_of<'a>(stmts: &'a [Stmt], out: &mut Vec<&'a [Stmt]>) {
    out.push(stmts);
    for stmt in stmts {
        blocks_in_stmt(stmt, out);
    }
}

/// [`blocks_of`] for what one statement contains.
fn blocks_in_stmt<'a>(stmt: &'a Stmt, out: &mut Vec<&'a [Stmt]>) {
    match &stmt.kind {
        StmtKind::ClassDecl(decl) => members(&decl.members, out),
        StmtKind::InterfaceDecl(decl) => members(&decl.members, out),
        StmtKind::EnumDecl(decl) => members(&decl.members, out),
        StmtKind::TopLevelFunction(f) => {
            if let Some(body) = &f.body {
                blocks_of(&body.stmts, out);
            }
        }
        _ => {
            let mut parts = Vec::new();
            stmt_parts(stmt, &mut parts);
            for part in parts {
                match part {
                    Part::Expr(e) => blocks_in_expr(e, out),
                    Part::Stmt(s) => blocks_in_stmt(s, out),
                    Part::Block(stmts) => blocks_of(stmts, out),
                }
            }
        }
    }
}

/// [`blocks_of`] for the anonymous functions and classes of one expression.
fn blocks_in_expr<'a>(e: &'a Expr, out: &mut Vec<&'a [Stmt]>) {
    match &e.kind {
        ExprKind::Fn(f) => match &f.body {
            FnBody::Block(body) => blocks_of(&body.stmts, out),
            FnBody::Expr(body) => blocks_in_expr(body, out),
        },
        ExprKind::New {
            target: NewTarget::AnonClass(decl),
            ..
        } => members(&decl.members, out),
        _ => {}
    }
    let mut kids = Vec::new();
    children(e, &mut kids);
    kids.into_iter()
        .for_each(|child| blocks_in_expr(child, out));
}

/// [`blocks_of`] for a class body's methods and property hooks.
fn members<'a>(members: &'a [ClassMember], out: &mut Vec<&'a [Stmt]>) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(m) => {
                if let Some(body) = &m.body {
                    blocks_of(&body.stmts, out);
                }
            }
            ClassMemberKind::Property(p) => {
                for hook in p.hooks.iter().flatten() {
                    if let Some(PropertyHookBody::Block(body)) = &hook.body {
                        blocks_of(&body.stmts, out);
                    }
                }
            }
            ClassMemberKind::Enum(decl) => self::members(&decl.members, out),
            _ => {}
        }
    }
}
