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
//! "written" set. [`scan_expr`] threads that set through every expression an
//! expression evaluates ([`nvs_syntax::visit::each_child_expr`], the one match
//! over the productions [`nvs_syntax::ast::ExprKind`] holds), plus a
//! `PropertyAccess` arm (a plain read of `$this->prop` — the shape
//! `ctor_init` never needed, since it only ever looks for an *assignment*
//! target) that reports [`code::E_LATEINIT_READ_BEFORE_WRITE_LOCAL`] the
//! first time it sees a tracked property read with nothing yet proven to
//! have written it on this path, then marks it written itself so a repeated
//! read on the same straight-line path isn't reported twice.
//!
//! What this module shares with `ctor_init` is the descent and nothing else.
//! The two passes track different per-path state (a set of "written" property
//! names here, versus `ctor_init`'s pair of "assigned" set and
//! "parent-called" flag), run over a different scope (every method here, only
//! the constructor there), and — the difference that shows in the walk itself —
//! want opposite things from a branch. `ctor_init` joins a ternary, a `match`
//! and an expression `catch` by intersection, because an assignment made on
//! one branch is not made on every path; this pass threads one set straight
//! through them, because a write seen on *any* branch suppressing a later
//! read is exactly the silence `rule:classes/lateinit-read-before-write`
//! asks for. One walk over the grammar, two answers to what an unrun operand
//! is worth.
//!
//! **An anonymous function's body is not walked into** — `each_child_expr` stops at one,
//! and this pass never reaches one as a body of its own either, since it walks
//! the methods a class declares. A read written inside an anonymous function is checked by
//! nothing here and falls through to `rule:classes/lateinit`'s runtime throw,
//! which is the sound direction: the anonymous function runs when it is called, so no
//! walk of the method holding it can say what has been written by then.
//!
//! **Only a class's own `lateinit` properties are tracked**, the set
//! [`crate::signatures::own_lateinit_properties`] returns. A property
//! declared `lateinit` on a parent and read through `$this` in a *subclass*'s
//! own method is not checked by this pass: it is checked when its own
//! declaring class's methods are, which is the same reason
//! [`crate::signatures::own_required_properties`] excludes `extends` rather
//! than re-deriving an inherited obligation at every subclass. A read this
//! pass does not reach falls through to `rule:classes/lateinit`'s runtime
//! throw, which is what `lateinit` is defined against in any case.
//!
//! **A `set`-hooked `lateinit` property is not modeled specially either**,
//! mirroring [`crate::ctor_init`]'s identical bound for a hooked required
//! property and holding for the same reason: neither pass has a model of a
//! hook's body, so a read of one is left to that same runtime throw.
//! `rule:classes/lateinit`'s own *Revisiting* section defers the question to
//! `docs/spec/`.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    AssignOp, ClassDecl, ClassMemberKind, Expr, ExprKind, MemberName, Stmt, StmtKind,
};
use nvs_syntax::visit::each_child_expr;
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
        // Each arm's `else` is the rest of the chain: the arms are walked in
        // order with `written` as the rest's set, then joined from the last
        // arm back to the first. A set that ends in a terminating path is
        // never read, so `written` may carry it.
        StmtKind::If { arms, else_ } => {
            let mut joins = Vec::with_capacity(arms.len());
            for (i, arm) in arms.iter().enumerate() {
                scan_expr(&arm.cond, written, tracked, env);
                let mut then_written = written.clone();
                let then_terminates = walk_stmt(&arm.then, &mut then_written, tracked, env);
                // No `else`: only the pre-existing `written` carries forward,
                // exactly like `crate::ctor_init::walk_stmt`'s own `If` arm.
                if i + 1 < arms.len() || else_.is_some() {
                    joins.push((then_written, then_terminates));
                }
            }
            let mut rest_terminates = match else_ {
                Some(else_stmt) => walk_stmt(else_stmt, written, tracked, env),
                None => false,
            };
            while let Some((then_written, then_terminates)) = joins.pop() {
                match (then_terminates, rest_terminates) {
                    (true, _) => {}
                    (false, true) => *written = then_written,
                    (false, false) => *written = merge(then_written, std::mem::take(written)),
                }
                rest_terminates = then_terminates && rest_terminates;
            }
            rest_terminates
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
/// inside `e`, descending through [`each_child_expr`] so no composite form
/// hides one. A call — `new` included — conservatively marks *every* tracked
/// property written once its own operands are scanned, per
/// `rule:classes/lateinit-read-before-write`.
///
/// A write and a plain read are the two shapes read off the node itself, so
/// they are matched here and take their own descent; every other form is
/// carried by the generic one.
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
            return;
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
            return;
        }
        _ => {}
    }

    each_child_expr(e, &mut |child| scan_expr(child, written, tracked, env));

    if matches!(
        &e.kind,
        ExprKind::Call { .. }
            | ExprKind::MethodCall { .. }
            | ExprKind::StaticCall { .. }
            | ExprKind::New { .. }
    ) {
        // `rule:classes/lateinit-read-before-write`: a call is opaque, so every
        // tracked property becomes "assumed written" the moment one is made —
        // after its own operands are scanned, which are read before it runs. A
        // `new` is a call like any other: the constructor it reaches is handed
        // whatever the arguments name.
        written.extend(tracked.iter().cloned());
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

    /// The descent is total over the grammar, so a read inside a `match` arm
    /// is a read like any other.
    #[test]
    fn a_read_inside_a_match_arm_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function boom(int $x): void {\n    Logger $l = match ($x) { default => $this->logger };\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_READ_BEFORE_WRITE_LOCAL)),
            "{diags:?}"
        );
    }

    /// An anonymous function's body is not this method's path: it runs when it is
    /// called, so nothing here can say what has been written by then and the
    /// read falls through to `rule:classes/lateinit`'s runtime throw.
    #[test]
    fn a_read_inside_an_anon_fn_body_is_not_checked() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function boom(): void {\n    var $later = fn (): Logger => $this->logger;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The asymmetry with `crate::ctor_init`: a write on one branch of a
    /// ternary is threaded straight through rather than joined, because
    /// `rule:classes/lateinit-read-before-write` would rather say nothing than
    /// flag a read some path has already written.
    #[test]
    fn a_write_on_one_ternary_branch_silences_a_later_read() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function boom(bool $flag): void {\n    Logger $l = $flag ? ($this->logger = new Logger()) : new Logger();\n    $this->logger;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_write_on_both_branches_of_an_if_satisfies_a_later_read() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function boom(bool $flag): void {\n    if ($flag) {\n      $this->logger = new Logger();\n    } else {\n      $this->logger = new Logger();\n    }\n    $this->logger;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }
}
