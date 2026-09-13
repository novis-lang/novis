//! `rule:classes/definite-property-initialization` — definite property initialization: every constructor a
//! class declares must assign, on every path out of it, every property the
//! class declares itself ([`crate::signatures::own_required_properties`]),
//! and — when the class `extends` another *that declares a constructor*
//! ([`parent_declares_constructor`]) — call `parent::constructor(...)` on
//! every path too,
//! discharging the inherited properties without re-deriving what the
//! parent's own constructor already assigns (the parent was checked against
//! this same rule when it was compiled, exactly as an ordinary call's callee
//! is trusted rather than re-verified at every call site — `rule:classes/definite-property-initialization`,
//! second bullet). A class with no constructor of its own and an
//! unassigned required property is refused at the property's own
//! declaration instead, since there is no constructor body to attach the
//! diagnostic to (`rule:classes/definite-property-initialization`, third bullet).
//!
//! This is a second, narrower flow-analysis pass over a constructor's body,
//! separate from [`crate::locals`]'s: it never checks an expression's
//! *type* (`crate::check::check_method` already walks every method body,
//! constructors included, for that), only whether `$this->prop = ...`
//! (plain `=`, never a compound operator — mirroring
//! `crate::expr::assign::check_assign`'s own "only `=` counts" rule for locals)
//! and `parent::constructor(...)` are reached on every path. [`walk_stmt`]
//! threads one [`InitState`] through the same control-flow shape
//! [`crate::locals::check_block`] does — `if`/`else`, `switch` and `try`/
//! `catch` all join by intersecting every branch that can finish normally
//! (a `do`-`while` body, which always runs, needs no join at all) — see that
//! module's own docs for the reasoning, which carries over unchanged with
//! [`InitState`] standing in for `live`.
//! Rather than computing a whole-body "definitely assigned" set once and
//! diagnosing at the end the way [`crate::locals`] does for a local read,
//! [`finish`] runs at every point a path can leave the constructor (an
//! explicit `return`, and — if some path never returns — the implicit one
//! at the end of the body), since `rule:classes/definite-property-initialization` is stated per-return, not
//! per-body.
//!
//! **A promoted constructor parameter carries no obligation here**, although it
//! is a property everywhere else: [`crate::signatures`] records its type and
//! its visibility and `crate::layout` gives it a slot. Its store is emitted
//! from the binding by `nvs_ir::lower::promoted_stores` rather than written in
//! the body, so `rule:classes/definite-property-initialization` is discharged
//! by construction and a constructor body has nothing to be checked against.
//!
//! **Known gaps**, deliberately out of scope for this slice:
//! - A property backed by a `set` hook is exempted from
//!   [`crate::signatures::ClassSignature::required_properties`] entirely,
//!   rather than checked against whether the hook actually commits a value —
//!   this module has no model of a hook's body at all.
//!   Decided: Keep the exemption; the runtime read-before-write throw covers it — Safe (an unwritten
//!   read throws) and simple, and the error comes only at run time.
//!   — owner: unowned-closures
//! - [`scan_expr`] only descends into a handful of common composite
//!   expression forms (assignment, calls, binary/unary/cast/ternary,
//!   `instanceof`, array literals). A `$this->prop = ...` or
//!   `parent::constructor(...)` buried inside a closure body, a `match` arm,
//!   or another form this module doesn't descend into produces a spurious
//!   diagnostic rather than being missed silently — safe by the same
//!   "reject, never wrongly accept" standard as every other known gap here.
//!   — owner: unowned
//! - A class with no explicit `constructor` is not itself checked against
//!   the `parent::constructor(...)` obligation — it has no constructor body
//!   of its own for such a call to go in; PHP inherits the parent
//!   constructor unchanged in that case, and this slice does not model that
//!   inheritance.
//!   Decided: Model the inherited constructor: refuse own required properties without a default — The
//!   error comes where the class is written, at the cost of a small extra rule in this pass.
//!   — owner: unowned-closures

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    AssignOp, CallArgs, ClassDecl, ClassMemberKind, Expr, ExprKind, MemberName, Stmt, StmtKind,
};
use rustc_hash::FxHashSet;

use crate::expr::is_this_receiver;
use crate::signatures::{own_required_properties, resolve_method};
use crate::{Env, span_text, strip_sigil};

/// The `$this->prop = ...`/`parent::constructor(...)` obligations one class
/// declaration's constructor is checked against, bundled so [`walk_stmt`]
/// and [`scan_expr`] thread one argument instead of several.
struct CtorObligations<'a> {
    qname: &'a QName,
    required: &'a [(String, Span)],
    ctor_name: Span,
    needs_parent_call: bool,
}

/// Per-path state while walking a constructor body: which required
/// properties have definitely been assigned so far, and whether
/// `parent::constructor(...)` has definitely been called so far.
#[derive(Clone, Default)]
pub(crate) struct InitState {
    assigned: FxHashSet<String>,
    parent_called: bool,
}

impl InitState {
    /// The state true of a path only when it was true on *both* of two
    /// branches that join back together — the same intersection
    /// [`crate::locals::check_block`]'s `if`/`else` handling uses for `live`.
    fn merge(a: Self, b: Self) -> Self {
        Self {
            assigned: a.assigned.intersection(&b.assigned).cloned().collect(),
            parent_called: a.parent_called && b.parent_called,
        }
    }
}

/// Checks one class declaration against `rule:classes/definite-property-initialization`. Interfaces and enums
/// are never called here — only [`crate::check::check_stmts`]'s `ClassDecl`
/// arm calls this, since only a class is ever instantiated through a
/// constructor.
pub(crate) fn check_class_init(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let required = own_required_properties(qname, env.signatures);

    let ctor = decl.members.iter().find_map(|m| match &m.kind {
        ClassMemberKind::Method(method) if span_text(env.src, method.name) == "constructor" => {
            Some(method)
        }
        _ => None,
    });

    let Some(ctor) = ctor else {
        // No constructor at all: a required property has no path to get
        // assigned along, so it is refused right at its own declaration
        // (`rule:classes/definite-property-initialization`, third bullet).
        for (name, span) in &required {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNINITIALIZED_PROPERTY,
                    format!(
                        "`${name}` has no default value, and `{qname}` declares no constructor \
                         to assign it"
                    ),
                )
                .with_primary(*span, "never assigned")
                .with_help("give it a default value, or add a `constructor` that assigns it"),
            );
        }
        return;
    };

    let Some(body) = &ctor.body else {
        return; // an abstract constructor has no body to walk
    };

    let obligations = CtorObligations {
        qname,
        required: &required,
        ctor_name: ctor.name,
        needs_parent_call: decl.extends.is_some() && parent_declares_constructor(qname, env),
    };
    let mut state = InitState::default();
    let terminates = walk_stmts(&body.stmts, &mut state, &obligations, env);
    if !terminates {
        // Fell off the end of the body without an explicit `return` — the
        // implicit return `rule:classes/definite-property-initialization` also covers.
        finish(&state, &obligations, env);
    }
}

/// Whether any ancestor of `qname` declares a `constructor` at all.
///
/// The `parent::constructor(...)` obligation is owed only when there is one
/// to call: an `abstract class Shape` that declares no constructor gives its
/// subclass nothing to reach, and demanding the call anyway made such a class
/// impossible to compile — `parent::constructor()` is itself an `E0309`
/// against a parent with no such method. Nothing is lost by the narrowing:
/// a parent with no constructor has no inherited property to discharge,
/// because `rule:classes/definite-property-initialization`'s third bullet already refuses one at its own
/// declaration.
fn parent_declares_constructor(qname: &QName, env: &Env<'_>) -> bool {
    let Some(links) = env.graph.get(qname) else {
        return false;
    };
    links
        .extends
        .iter()
        .any(|parent| resolve_method(parent, "constructor", env.signatures, env.graph).is_some())
}

/// Reports whatever `state` still leaves unsatisfied at one point a
/// constructor can return from.
fn finish(state: &InitState, obligations: &CtorObligations<'_>, env: &mut Env<'_>) {
    for (name, span) in obligations.required {
        if !state.assigned.contains(name) {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNINITIALIZED_PROPERTY,
                    format!("`${name}` is not assigned on every path out of the constructor"),
                )
                .with_primary(*span, "declared here")
                .with_secondary(
                    obligations.ctor_name,
                    "not assigned along every path out of this constructor",
                ),
            );
        }
    }
    if obligations.needs_parent_call && !state.parent_called {
        env.diags.report(
            Diagnostic::error(
                code::E_MISSING_PARENT_CONSTRUCTOR_CALL,
                format!(
                    "`{}` extends another class, but a path through its constructor never \
                     calls `parent::constructor(...)`",
                    obligations.qname
                ),
            )
            .with_primary(obligations.ctor_name, "missing call on some path"),
        );
    }
}

/// Walks `stmts` in sequence, updating `state` in place. Returns `true` when
/// every path through `stmts` definitely ends in a `return` or `throw` —
/// [`check_class_init`] uses that to know whether the body's fall-through
/// end still needs its own [`finish`] call.
fn walk_stmts(
    stmts: &[Stmt],
    state: &mut InitState,
    obligations: &CtorObligations<'_>,
    env: &mut Env<'_>,
) -> bool {
    for stmt in stmts {
        if walk_stmt(stmt, state, obligations, env) {
            return true;
        }
    }
    false
}

fn walk_stmt(
    stmt: &Stmt,
    state: &mut InitState,
    obligations: &CtorObligations<'_>,
    env: &mut Env<'_>,
) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) => {
            finish(state, obligations, env);
            true
        }
        StmtKind::Expr(e) => {
            if matches!(&e.kind, ExprKind::Throw(_)) {
                // A throwing path never returns the constructed object, so
                // `rule:classes/definite-property-initialization` has nothing to check on it.
                return true;
            }
            scan_expr(e, state, env);
            false
        }
        StmtKind::Block(b) => walk_stmts(&b.stmts, state, obligations, env),
        StmtKind::If { cond, then, else_ } => {
            scan_expr(cond, state, env);
            let mut then_state = state.clone();
            let then_terminates = walk_stmt(then, &mut then_state, obligations, env);
            if let Some(else_stmt) = else_ {
                let mut else_state = state.clone();
                let else_terminates = walk_stmt(else_stmt, &mut else_state, obligations, env);
                match (then_terminates, else_terminates) {
                    (true, true) => return true,
                    (true, false) => *state = else_state,
                    (false, true) => *state = then_state,
                    (false, false) => *state = InitState::merge(then_state, else_state),
                }
            }
            // No `else`: only the pre-existing `state` carries forward,
            // exactly like `crate::locals::check_block`'s own `If` arm.
            false
        }
        StmtKind::While { cond, body } => {
            scan_expr(cond, state, env);
            let mut body_state = state.clone();
            walk_stmt(body, &mut body_state, obligations, env);
            // The body may run zero times, so nothing it assigns carries
            // forward — same conservative treatment as every loop here.
            false
        }
        StmtKind::DoWhile { body, cond } => {
            // The body runs at least once, so its assignments do carry
            // forward — mirrors `crate::locals`'s own `DoWhile` arm.
            if walk_stmt(body, state, obligations, env) {
                return true;
            }
            scan_expr(cond, state, env);
            false
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            // `rule:iteration/for-init-clause`: the init clause runs once, before the loop, in
            // this same state — a declaration there is no different from one
            // written on the line above.
            if let Some(decl) = init.decl() {
                walk_stmt(decl, state, obligations, env);
            }
            for e in init.exprs() {
                scan_expr(e, state, env);
            }
            for e in cond {
                scan_expr(e, state, env);
            }
            let mut body_state = state.clone();
            walk_stmt(body, &mut body_state, obligations, env);
            for e in step {
                scan_expr(e, &mut body_state, env);
            }
            false
        }
        StmtKind::Foreach { subject, body, .. } => {
            scan_expr(subject, state, env);
            let mut body_state = state.clone();
            walk_stmt(body, &mut body_state, obligations, env);
            false
        }
        StmtKind::Switch { subject, cases } => {
            scan_expr(subject, state, env);
            // Same join as `crate::locals`'s own `Switch` arm: a case
            // contributes only when it definitely exits after the switch —
            // ending in a bare `break`/`continue`, or being the last case and
            // falling off the end — excluding one that always
            // returns/throws instead, and joining in the pre-switch `state`
            // too when there's no `default` (see that module's docs for why).
            let last_index = cases.len().saturating_sub(1);
            let mut candidates: Vec<InitState> = Vec::new();
            for (i, case) in cases.iter().enumerate() {
                let mut case_state = state.clone();
                if let Some(c) = &case.cond {
                    scan_expr(c, &mut case_state, env);
                }
                let terminates = walk_stmts(&case.body, &mut case_state, obligations, env);
                if terminates {
                    continue;
                }
                let exits = case.body.last();
                if exits.is_some_and(crate::locals::ends_in_break_or_continue) || i == last_index {
                    candidates.push(case_state);
                }
            }
            if cases.iter().all(|c| c.cond.is_some()) {
                candidates.push(state.clone());
            }
            if let Some(merged) = candidates.into_iter().reduce(InitState::merge) {
                *state = merged;
            }
            false
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            // Same join as `crate::locals`'s own `Try` arm: `body`/each
            // `catch` start fresh from the pre-`try` `state` (an exception
            // can interrupt `body` before any of its own assignments run),
            // and each contributes to the post-`try` state only when it
            // finishes normally rather than always returning/throwing.
            let mut candidates: Vec<InitState> = Vec::new();
            let mut body_state = state.clone();
            if !walk_stmts(&body.stmts, &mut body_state, obligations, env) {
                candidates.push(body_state);
            }
            for catch in catches {
                let mut catch_state = state.clone();
                if !walk_stmts(&catch.body.stmts, &mut catch_state, obligations, env) {
                    candidates.push(catch_state);
                }
            }
            let merged = candidates.into_iter().reduce(InitState::merge);
            // `finally` always runs, so it applies on top of whichever
            // candidate above actually happened — union it in rather than
            // discarding the candidates' join.
            if let Some(finally) = finally {
                let mut finally_state = state.clone();
                walk_stmts(&finally.stmts, &mut finally_state, obligations, env);
                *state = match merged {
                    Some(m) => InitState {
                        assigned: m.assigned.union(&finally_state.assigned).cloned().collect(),
                        parent_called: m.parent_called || finally_state.parent_called,
                    },
                    None => finally_state,
                };
            } else if let Some(m) = merged {
                *state = m;
            }
            false
        }
        StmtKind::Echo(xs) | StmtKind::Unset(xs) => {
            for x in xs {
                scan_expr(x, state, env);
            }
            false
        }
        StmtKind::LocalDecl {
            value: Some(value), ..
        } => {
            scan_expr(value, state, env);
            false
        }
        StmtKind::Destructure { value, .. } => {
            scan_expr(value, state, env);
            false
        }
        _ => false,
    }
}

/// Looks for a `$this->prop = ...` assignment or a `parent::constructor(...)`
/// call anywhere `e` directly nests one of a handful of common composite
/// forms — see the module docs' known gaps for what this does not descend
/// into.
pub(crate) fn scan_expr(e: &Expr, state: &mut InitState, env: &Env<'_>) {
    match &e.kind {
        ExprKind::Assign {
            op, target, value, ..
        } => {
            scan_expr(value, state, env);
            scan_expr(target, state, env);
            if *op == AssignOp::Assign
                && let ExprKind::PropertyAccess {
                    object,
                    property: MemberName::Ident(name_span),
                    ..
                } = &target.kind
                && is_this_receiver(object, env.src)
            {
                state
                    .assigned
                    .insert(strip_sigil(span_text(env.src, *name_span)).to_owned());
            }
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
            ..
        } => {
            scan_call_args(args, state, env);
            scan_expr(class, state, env);
            if matches!(class.kind, ExprKind::ParentExpr)
                && let MemberName::Ident(name_span) = method
                && span_text(env.src, *name_span) == "constructor"
            {
                state.parent_called = true;
            }
        }
        ExprKind::MethodCall { object, args, .. } => {
            scan_expr(object, state, env);
            scan_call_args(args, state, env);
        }
        ExprKind::Call { callee, args } => {
            scan_expr(callee, state, env);
            scan_call_args(args, state, env);
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            scan_expr(lhs, state, env);
            scan_expr(rhs, state, env);
        }
        ExprKind::Catch { guarded, arms } => {
            scan_expr(guarded, state, env);
            for arm in arms {
                scan_expr(&arm.body, state, env);
            }
        }
        ExprKind::Ternary { cond, then, else_ } => {
            scan_expr(cond, state, env);
            if let Some(then) = then {
                scan_expr(then, state, env);
            }
            scan_expr(else_, state, env);
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::PreIncDec { expr: inner, .. }
        | ExprKind::PostIncDec { expr: inner, .. }
        | ExprKind::Conversion { expr: inner, .. }
        | ExprKind::TypeTest { expr: inner, .. } => {
            scan_expr(inner, state, env);
        }
        ExprKind::InstanceOf { expr: inner, class } => {
            scan_expr(inner, state, env);
            scan_expr(class, state, env);
        }
        ExprKind::ArrayLiteral(items) => {
            for item in items {
                if let Some(key) = &item.key {
                    scan_expr(key, state, env);
                }
                scan_expr(&item.value, state, env);
            }
        }
        _ => {}
    }
}

fn scan_call_args(args: &CallArgs, state: &mut InitState, env: &Env<'_>) {
    if let CallArgs::List(list) = args {
        for arg in list {
            scan_expr(&arg.value, state, env);
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
    fn a_class_with_no_constructor_and_no_default_is_diagnosed() {
        let diags = check_src("<?nvs\nclass Foo {\n  public int $count;\n}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_class_with_no_constructor_but_an_inline_default_is_fine() {
        let diags = check_src("<?nvs\nclass Foo {\n  public int $count = 0;\n}\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_class_with_no_constructor_but_a_nullable_property_is_fine() {
        let diags = check_src("<?nvs\nclass Foo {\n  public ?int $count;\n}\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_constructor_assigning_every_property_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(int $c) {\n    $this->count = $c;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_constructor_leaving_a_property_unassigned_on_one_branch_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(bool $flag) {\n    if ($flag) {\n      $this->count = 1;\n    }\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_constructor_assigning_on_both_if_branches_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(bool $flag) {\n    if ($flag) {\n      $this->count = 1;\n    } else {\n      $this->count = 2;\n    }\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_subclass_constructor_skipping_parent_constructor_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Base {\n  public int $id;\n  function constructor(int $id) {\n    $this->id = $id;\n  }\n}\nclass Sub extends Base {\n  function constructor() {\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_MISSING_PARENT_CONSTRUCTOR_CALL)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_subclass_constructor_calling_parent_constructor_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Base {\n  public int $id;\n  function constructor(int $id) {\n    $this->id = $id;\n  }\n}\nclass Sub extends Base {\n  function constructor(int $id) {\n    parent::constructor($id);\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_hooked_property_is_exempt_from_the_check() {
        let diags = check_src("<?nvs\nclass Foo {\n  public int $count { get => 1; }\n}\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `rule:classes/lateinit-restrictions`: a `lateinit` property is exempt from `rule:classes/definite-property-initialization`'s
    /// constructor-must-assign obligation entirely, whether or not the class
    /// even has a constructor.
    #[test]
    fn a_lateinit_property_is_exempt_from_the_constructor_check() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function constructor() {\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `walk_stmt`'s `Switch` arm: every case ends in a `break`, and a
    /// `default` covers "no case matched" — so `$count` is assigned on every
    /// path out of the constructor.
    #[test]
    fn a_switch_with_default_and_a_break_in_every_case_satisfies_the_property() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(int $x) {\n    switch ($x) {\n      case 1:\n        $this->count = 1;\n        break;\n      default:\n        $this->count = 2;\n        break;\n    }\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// Same shape but with no `default` arm: "no case matched" leaves
    /// `$count` unassigned on that path, so it's still diagnosed.
    #[test]
    fn a_switch_with_no_default_does_not_satisfy_the_property() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(int $x) {\n    switch ($x) {\n      case 1:\n        $this->count = 1;\n        break;\n    }\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    /// `walk_stmt`'s `Try` arm: `body` completing and `catch` completing
    /// both assign `$count`, so it's satisfied on every path.
    #[test]
    fn a_try_and_its_catch_both_assigning_satisfies_the_property() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor() {\n    try {\n      $this->count = 1;\n    } catch (LogicError $e) {\n      $this->count = 2;\n    }\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The `catch` doesn't assign `$count`, so it's not satisfied on every
    /// path — the join now depends on what's actually inside `catch`.
    #[test]
    fn a_try_whose_catch_does_not_assign_does_not_satisfy_the_property() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor() {\n    try {\n      $this->count = 1;\n    } catch (LogicError $e) {\n    }\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }
}
