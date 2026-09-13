//! `rule:classes/lateinit-read-before-write` — the one intraprocedural, false-positive-free `lateinit`
//! read-before-write check: reuses `rule:classes/definite-property-initialization`'s [`crate::ctor_init`]
//! definite-assignment idea (tracking "definitely written so far" and
//! joining branches by intersection, the same conservative direction
//! [`crate::locals`]'s own "read before definite assignment" check for
//! locals already uses), but scoped differently — every method body (not
//! only constructors), and a call anywhere immediately marks *every* tracked
//! property "assumed written" rather than leaving it to per-property
//! tracking, since the analysis can never know which properties a callee
//! wrote and the ADR's own design mandate is to stay silent rather than risk
//! a false positive.
//!
//! [`check_class_lateinit_reads`] runs, for every method `decl` declares
//! (constructors included — unlike [`crate::ctor_init`], a `lateinit`
//! property's obligation has nothing to do with construction), a fresh walk
//! of [`walk_stmts`] starting with none of the class's own `lateinit`
//! properties ([`crate::signatures::own_lateinit_properties`]) in the
//! "written" set. [`scan_expr`] threads that set through the same handful of
//! composite expression forms [`crate::ctor_init::scan_expr`] does, plus a
//! `PropertyAccess` arm (a plain read of `$this->prop` — the shape
//! `ctor_init` never needed, since it only ever looks for an *assignment*
//! target) that reports [`code::E_LATEINIT_READ_BEFORE_WRITE_LOCAL`] the
//! first time it sees a tracked property read with nothing yet proven to
//! have written it on this path, then marks it written itself so a repeated
//! read on the same straight-line path isn't reported twice.
//!
//! This module deliberately does not share a walker with `ctor_init`, for
//! the same reason that module gives for not sharing one with `locals`: the
//! two passes track different per-path state (a set of "written" property
//! names here, versus `ctor_init`'s pair of "assigned" set and
//! "parent-called" flag) and run over a different scope (every method here,
//! only the constructor there).
//!
//! **Known gaps**, deliberately out of scope for this slice, same standard
//! as every other gap in this crate — reject or stay silent, never wrongly
//! accept and never wrongly flag:
//! - Only a class's *own* `lateinit` properties (see
//!   [`crate::signatures::own_lateinit_properties`]) are tracked here. A
//!   property declared `lateinit` on a parent class, read through `$this` in
//!   a *subclass*'s own method, is not checked by this pass at all — it
//!   relies entirely on `rule:classes/lateinit`'s runtime throw. This mirrors
//!   `crate::signatures::own_required_properties`'s own choice to exclude
//!   `extends`, for the same reason: the property is checked when its own
//!   declaring class's methods are checked, not re-derived here.
//!   Decided: Keep own-class only; the runtime throw covers subclasses — Simple and consistent with the
//!   sibling pass, and `lateinit` is by definition checked at run time anyway.
//!   — owner: unowned-closures
//! - [`scan_expr`] only descends into the same handful of common composite
//!   expression forms `crate::ctor_init::scan_expr` does. A read buried
//!   inside a closure body, a `match` arm, or another form this module
//!   doesn't descend into is silently not checked — safe, since a missed
//!   diagnostic is never a false positive.
//!   — owner: unowned
//! - A `set`-hooked `lateinit` property is not modeled specially here either
//!   (mirroring `crate::ctor_init`'s identical gap for a hooked required
//!   property) — see `rule:classes/lateinit`'s own *Revisiting* section, which defers this
//!   exact question to `docs/spec/`.
//!   Decided: Keep the exemption; the runtime read-before-write throw covers it — Safe (an unwritten
//!   read throws) and simple, and the error comes only at run time.
//!   — owner: unowned-closures

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    AssignOp, CallArgs, ClassDecl, ClassMemberKind, Expr, ExprKind, MemberName, Stmt, StmtKind,
};
use rustc_hash::FxHashSet;

use crate::expr::is_this_receiver;
use crate::signatures::own_lateinit_properties;
use crate::{Env, span_text, strip_sigil};

/// Runs `rule:classes/lateinit-read-before-write`'s check over every method `decl` declares with a body,
/// for each of `qname`'s own `lateinit` properties. A class with none of its
/// own does nothing — there is nothing to track.
pub(crate) fn check_class_lateinit_reads(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let tracked = own_lateinit_properties(qname, env.signatures);
    if tracked.is_empty() {
        return;
    }
    for member in &decl.members {
        if let ClassMemberKind::Method(m) = &member.kind
            && let Some(body) = &m.body
        {
            let mut written: FxHashSet<String> = FxHashSet::default();
            walk_stmts(&body.stmts, &mut written, &tracked, env);
        }
    }
}

/// The state true of a path only when it was true on *both* of two branches
/// that join back together — the same intersection
/// [`crate::ctor_init::InitState::merge`] uses for its own "assigned" set,
/// for the identical reason: a property counts as "definitely written" after
/// a join only if every path reaching the join wrote it.
fn merge(a: FxHashSet<String>, b: FxHashSet<String>) -> FxHashSet<String> {
    a.intersection(&b).cloned().collect()
}

fn walk_stmts(
    stmts: &[Stmt],
    written: &mut FxHashSet<String>,
    tracked: &FxHashSet<String>,
    env: &mut Env<'_>,
) -> bool {
    for stmt in stmts {
        if walk_stmt(stmt, written, tracked, env) {
            return true;
        }
    }
    false
}

/// Walks one statement, updating `written` in place. Returns `true` when
/// every path through `stmt` definitely ends in a `return` or `throw` — used
/// the same way [`crate::ctor_init::walk_stmt`]'s own return value is, to
/// decide whether a branch contributes to a later join.
fn walk_stmt(
    stmt: &Stmt,
    written: &mut FxHashSet<String>,
    tracked: &FxHashSet<String>,
    env: &mut Env<'_>,
) -> bool {
    match &stmt.kind {
        StmtKind::Return(Some(e)) => {
            scan_expr(e, written, tracked, env);
            true
        }
        StmtKind::Return(None) => true,
        StmtKind::Expr(e) => {
            if let ExprKind::Throw(inner) = &e.kind {
                scan_expr(inner, written, tracked, env);
                return true;
            }
            scan_expr(e, written, tracked, env);
            false
        }
        StmtKind::Block(b) => walk_stmts(&b.stmts, written, tracked, env),
        StmtKind::If { cond, then, else_ } => {
            scan_expr(cond, written, tracked, env);
            let mut then_written = written.clone();
            let then_terminates = walk_stmt(then, &mut then_written, tracked, env);
            if let Some(else_stmt) = else_ {
                let mut else_written = written.clone();
                let else_terminates = walk_stmt(else_stmt, &mut else_written, tracked, env);
                match (then_terminates, else_terminates) {
                    (true, true) => return true,
                    (true, false) => *written = else_written,
                    (false, true) => *written = then_written,
                    (false, false) => *written = merge(then_written, else_written),
                }
            }
            // No `else`: only the pre-existing `written` carries forward,
            // exactly like `crate::ctor_init::walk_stmt`'s own `If` arm.
            false
        }
        StmtKind::While { cond, body } => {
            scan_expr(cond, written, tracked, env);
            let mut body_written = written.clone();
            walk_stmt(body, &mut body_written, tracked, env);
            // The body may run zero times, so nothing it writes carries
            // forward — same conservative treatment `ctor_init` uses.
            false
        }
        StmtKind::DoWhile { body, cond } => {
            // The body runs at least once, so its writes do carry forward.
            if walk_stmt(body, written, tracked, env) {
                return true;
            }
            scan_expr(cond, written, tracked, env);
            false
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            // `rule:iteration/for-init-clause`: a declaration in the init clause runs once,
            // before the loop, exactly where the line above it used to.
            if let Some(decl) = init.decl() {
                walk_stmt(decl, written, tracked, env);
            }
            for e in init.exprs() {
                scan_expr(e, written, tracked, env);
            }
            for e in cond {
                scan_expr(e, written, tracked, env);
            }
            let mut body_written = written.clone();
            walk_stmt(body, &mut body_written, tracked, env);
            for e in step {
                scan_expr(e, &mut body_written, tracked, env);
            }
            false
        }
        StmtKind::Foreach { subject, body, .. } => {
            scan_expr(subject, written, tracked, env);
            let mut body_written = written.clone();
            walk_stmt(body, &mut body_written, tracked, env);
            false
        }
        StmtKind::Switch { subject, cases } => {
            scan_expr(subject, written, tracked, env);
            // Same join as `crate::ctor_init::walk_stmt`'s own `Switch` arm.
            let last_index = cases.len().saturating_sub(1);
            let mut candidates: Vec<FxHashSet<String>> = Vec::new();
            for (i, case) in cases.iter().enumerate() {
                let mut case_written = written.clone();
                if let Some(c) = &case.cond {
                    scan_expr(c, &mut case_written, tracked, env);
                }
                let terminates = walk_stmts(&case.body, &mut case_written, tracked, env);
                if terminates {
                    continue;
                }
                let exits = case.body.last();
                if exits.is_some_and(crate::locals::ends_in_break_or_continue) || i == last_index {
                    candidates.push(case_written);
                }
            }
            if cases.iter().all(|c| c.cond.is_some()) {
                candidates.push(written.clone());
            }
            if let Some(merged) = candidates.into_iter().reduce(merge) {
                *written = merged;
            }
            false
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            // Same join as `crate::ctor_init::walk_stmt`'s own `Try` arm.
            let mut candidates: Vec<FxHashSet<String>> = Vec::new();
            let mut body_written = written.clone();
            if !walk_stmts(&body.stmts, &mut body_written, tracked, env) {
                candidates.push(body_written);
            }
            for catch in catches {
                let mut catch_written = written.clone();
                if !walk_stmts(&catch.body.stmts, &mut catch_written, tracked, env) {
                    candidates.push(catch_written);
                }
            }
            let merged = candidates.into_iter().reduce(merge);
            if let Some(finally) = finally {
                let mut finally_written = written.clone();
                walk_stmts(&finally.stmts, &mut finally_written, tracked, env);
                *written = match merged {
                    Some(m) => m.union(&finally_written).cloned().collect(),
                    None => finally_written,
                };
            } else if let Some(m) = merged {
                *written = m;
            }
            false
        }
        StmtKind::Echo(xs) | StmtKind::Unset(xs) => {
            for x in xs {
                scan_expr(x, written, tracked, env);
            }
            false
        }
        StmtKind::LocalDecl {
            value: Some(value), ..
        } => {
            scan_expr(value, written, tracked, env);
            false
        }
        StmtKind::Destructure { value, .. } => {
            scan_expr(value, written, tracked, env);
            false
        }
        _ => false,
    }
}

/// Looks for a `$this->prop` read or a `$this->prop = ...` write anywhere
/// `e` directly nests one of a handful of common composite forms — see the
/// module docs' known gaps for what this does not descend into. Any call
/// (`Call`/`MethodCall`/`StaticCall`) conservatively marks *every* tracked
/// property written, per `rule:classes/lateinit-read-before-write`.
fn scan_expr(
    e: &Expr,
    written: &mut FxHashSet<String>,
    tracked: &FxHashSet<String>,
    env: &mut Env<'_>,
) {
    match &e.kind {
        ExprKind::Assign {
            op, target, value, ..
        } => {
            scan_expr(value, written, tracked, env);
            if *op == AssignOp::Assign
                && let ExprKind::PropertyAccess {
                    object,
                    property: MemberName::Ident(name_span),
                    ..
                } = &target.kind
                && is_this_receiver(object, env.src)
            {
                // A pure `=` write: nothing is read here, so `target` is
                // deliberately not scanned generically (mirrors
                // `crate::ctor_init::scan_expr`'s identical `Assign` arm).
                let name = strip_sigil(span_text(env.src, *name_span)).to_owned();
                if tracked.contains(&name) {
                    written.insert(name);
                }
            } else {
                // A compound assignment (or any other target shape) reads
                // before it writes — scan `target` generically so the
                // `PropertyAccess` arm below can flag an unwritten read, then
                // record the write unconditionally afterward so a later
                // statement is never falsely flagged for this one.
                scan_expr(target, written, tracked, env);
                if let ExprKind::PropertyAccess {
                    object,
                    property: MemberName::Ident(name_span),
                    ..
                } = &target.kind
                    && is_this_receiver(object, env.src)
                {
                    written.insert(strip_sigil(span_text(env.src, *name_span)).to_owned());
                }
            }
        }
        ExprKind::PropertyAccess {
            object,
            property: MemberName::Ident(name_span),
            ..
        } => {
            scan_expr(object, written, tracked, env);
            if is_this_receiver(object, env.src) {
                let name = strip_sigil(span_text(env.src, *name_span)).to_owned();
                if tracked.contains(&name) && !written.contains(&name) {
                    env.diags.report(
                        Diagnostic::error(
                            code::E_LATEINIT_READ_BEFORE_WRITE_LOCAL,
                            format!(
                                "`$this->{name}` is read here, but nothing on this path has \
                                 written to it yet"
                            ),
                        )
                        .with_primary(*name_span, "possibly still unwritten")
                        .with_help(
                            "assign it before this read, or move the read after whatever call \
                             populates it",
                        ),
                    );
                    // Don't cry wolf again for a repeated read on the same
                    // straight-line path.
                    written.insert(name);
                }
            }
        }
        ExprKind::StaticCall { class, args, .. } => {
            scan_expr(class, written, tracked, env);
            scan_call_args(args, written, tracked, env);
            written.extend(tracked.iter().cloned());
        }
        ExprKind::MethodCall { object, args, .. } => {
            scan_expr(object, written, tracked, env);
            scan_call_args(args, written, tracked, env);
            written.extend(tracked.iter().cloned());
        }
        ExprKind::Call { callee, args } => {
            scan_expr(callee, written, tracked, env);
            scan_call_args(args, written, tracked, env);
            written.extend(tracked.iter().cloned());
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            scan_expr(lhs, written, tracked, env);
            scan_expr(rhs, written, tracked, env);
        }
        ExprKind::Catch { guarded, arms } => {
            scan_expr(guarded, written, tracked, env);
            for arm in arms {
                scan_expr(&arm.body, written, tracked, env);
            }
        }
        ExprKind::Ternary { cond, then, else_ } => {
            scan_expr(cond, written, tracked, env);
            if let Some(then) = then {
                scan_expr(then, written, tracked, env);
            }
            scan_expr(else_, written, tracked, env);
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::PreIncDec { expr: inner, .. }
        | ExprKind::PostIncDec { expr: inner, .. }
        | ExprKind::Conversion { expr: inner, .. }
        | ExprKind::TypeTest { expr: inner, .. } => {
            scan_expr(inner, written, tracked, env);
        }
        ExprKind::InstanceOf { expr: inner, class } => {
            scan_expr(inner, written, tracked, env);
            scan_expr(class, written, tracked, env);
        }
        ExprKind::ArrayLiteral(items) => {
            for item in items {
                if let Some(key) = &item.key {
                    scan_expr(key, written, tracked, env);
                }
                scan_expr(&item.value, written, tracked, env);
            }
        }
        _ => {}
    }
}

fn scan_call_args(
    args: &CallArgs,
    written: &mut FxHashSet<String>,
    tracked: &FxHashSet<String>,
    env: &mut Env<'_>,
) {
    if let CallArgs::List(list) = args {
        for arg in list {
            scan_expr(&arg.value, written, tracked, env);
        }
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap, code};
    use nvs_hir::resolve_file;
    use nvs_syntax::parse_file;

    use crate::expr_table::ExprTypeTable;
    use crate::ty::TypeInterner;

    fn check_src(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        crate::check_program(
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        diags
    }

    #[test]
    fn reading_a_lateinit_property_before_any_write_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function boom(): void {\n    $this->logger;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_READ_BEFORE_WRITE_LOCAL)),
            "{diags:?}"
        );
    }

    #[test]
    fn reading_a_lateinit_property_after_writing_it_first_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function setup(Logger $l): void {\n    $this->logger = $l;\n    $this->logger;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_call_before_the_read_silences_the_check() {
        // `rule:classes/lateinit-read-before-write`: a call is opaque and immediately assumed to have
        // written every tracked property, so no diagnostic fires here even
        // though `init()` might not actually assign `$logger`.
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function init(): void {}\n  function boom(): void {\n    $this->init();\n    $this->logger;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_read_on_only_one_branch_of_an_if_is_still_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function boom(bool $flag): void {\n    if ($flag) {\n      $this->logger = new Logger();\n    }\n    $this->logger;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_READ_BEFORE_WRITE_LOCAL)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_write_on_both_branches_of_an_if_satisfies_a_later_read() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function boom(bool $flag): void {\n    if ($flag) {\n      $this->logger = new Logger();\n    } else {\n      $this->logger = new Logger();\n    }\n    $this->logger;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }
}
