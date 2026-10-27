//! `rule:testing/mutation-testing`'s mutants: one changed site in the lowered
//! program each, made while the checked tree is lowered.
//!
//! # Two passes over one lowering
//!
//! [`mutants`] lowers the program once in *survey* mode, and every site that
//! could take a change records a [`Mutant`] instead of changing. [`lower_mutant`]
//! lowers it again with one of those mutants armed, and the one site whose span
//! and [`Operator`] match makes the change. Parse and check ran once before
//! either; neither pass checks anything again.
//!
//! The plan reaches the sites through a thread-local set for the length of one
//! [`lower_program_as`] call, because a program's functions are lowered by
//! nine [`Lowering::new`] callers, anonymous functions and generators among
//! them, all on the calling thread. The guard clears it on every exit, so a
//! lowering outside these two functions never sees a plan.
//!
//! # Every mutant compiles and keeps what the checker proved
//!
//! A swap is made only where the operands' lowered representations give the
//! new operator the same result type: `*` and `/` swap over `float` and
//! `decimal` alone, since `int / int` is `int|float`. A swap that changes
//! which way a condition goes is also held to the checker's narrowing
//! (`nvs_types::locals`): a type the checker narrowed on one edge of a
//! condition is read at that type on that edge, so a mutant that took the
//! edge while the test was false would read a value at a representation it
//! does not have. [`Lowering::narrows`] over-approximates the conditions
//! whose narrowing changes a representation: an `is` test, and an equality
//! with a literal, `null` or a class constant over a value stored as
//! [`Ty::Tagged`]. A condition it names keeps its connectors, its negation
//! and its narrowing tests. An equality over any other value is reversed and
//! negated like any condition. A relational or arithmetic swap narrows
//! nothing and is made anywhere.
//!
//! A `true` or `false` is not flipped where it is an `if` or loop condition,
//! a `match` subject or an `==` operand: the checker reads a loop on `true` as
//! one that does not end, and a `match (true)` arm as a condition.
//!
//! # Test code is never mutated
//!
//! A method `rule:testing/tests-never-reach-a-build` names as test code is
//! lowered with the survey turned off, and so is every anonymous function and
//! generator lowered inside it.
//!
//! # What it spends
//!
//! Nothing for a lowering with no plan: each site reads one thread-local and
//! returns. A survey holds one [`Mutant`] per site. An armed lowering clones
//! the one expression it changes.

use std::cell::RefCell;

use nvs_diagnostics::{SourceId, Span};

use super::*;

/// One of the six changes, each keeping its operands' and its result's types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operator {
    /// `<` ↔ `<=`, `>` ↔ `>=`.
    RelationalBoundary,
    /// `==` ↔ `!=`.
    EqualityNegated,
    /// `+` ↔ `-`, `*` ↔ `/`.
    ArithmeticSwap,
    /// `&&` ↔ `||`.
    LogicalConnector,
    /// `true` ↔ `false`.
    BooleanLiteral,
    /// An `if`'s, an `elseif`'s or a loop's condition is inverted.
    BranchNegated,
}

/// One mutant: where it is, which operator made it, and the text before and
/// after.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Mutant {
    /// The changed expression's span. With [`Self::operator`] it is the key.
    pub span: Span,
    /// The file the site is written in.
    pub file: SourceId,
    /// The 1-based line the site starts on.
    pub line: usize,
    /// Which of the six changes this is.
    pub operator: Operator,
    /// What was written, such as `<`, `true` or `condition`.
    pub from: &'static str,
    /// What the mutant has instead, such as `<=`, `false` or `!condition`.
    pub to: &'static str,
}

impl Mutant {
    /// The change as a report line names it: ``<` to `<=``.
    #[must_use]
    pub fn change(&self) -> String {
        format!("`{}` to `{}`", self.from, self.to)
    }
}

/// Every mutant of the program, in the order its sites are lowered.
///
/// The program is lowered with its test code, as `nvs test` runs it, and no
/// site inside test code is listed.
///
/// # Panics
///
/// The same way [`lower_program_with_tests`] does.
#[must_use]
pub fn mutants(
    script: &str,
    files: &[nvs_types::ProgramFile<'_>],
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
    layouts: &ClassLayoutTable,
) -> Vec<Mutant> {
    let guard = Armed::set(Mode::Survey(Vec::new()));
    let _ = lower_program_as(
        script,
        files,
        exprs,
        checked_types,
        enums,
        layouts,
        Tests::Lower,
    );
    match guard.take() {
        Mode::Survey(found) => found,
        Mode::Apply { .. } => unreachable!("a survey is armed as one"),
    }
}

/// The program, with its test code, lowered with `mutant`'s one change made.
///
/// # Panics
///
/// If no site matches `mutant`: it was not listed by [`mutants`] for this
/// checked program. Otherwise the same way [`lower_program_with_tests`] does.
#[must_use]
pub fn lower_mutant(
    script: &str,
    files: &[nvs_types::ProgramFile<'_>],
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
    layouts: &ClassLayoutTable,
    mutant: &Mutant,
) -> crate::ir::Program {
    let guard = Armed::set(Mode::Apply {
        span: mutant.span,
        operator: mutant.operator,
        applied: false,
    });
    let program = lower_program_as(
        script,
        files,
        exprs,
        checked_types,
        enums,
        layouts,
        Tests::Lower,
    );
    let Mode::Apply { applied, .. } = guard.take() else {
        unreachable!("an armed mutant is armed as one")
    };
    assert!(
        applied,
        "nvs-ir: no site matches the mutant at {:?} ({:?}); it was not listed for this program",
        mutant.span, mutant.operator
    );
    program
}

enum Mode {
    Survey(Vec<Mutant>),
    Apply {
        span: Span,
        operator: Operator,
        applied: bool,
    },
}

struct Plan {
    mode: Mode,
    /// Set while a test-code method is lowered: no site there is listed or
    /// changed.
    in_test_code: bool,
    /// The `true` and `false` literals [`Lowering::shield_literal`] named.
    shielded: Vec<Span>,
}

thread_local! {
    static PLAN: RefCell<Option<Plan>> = const { RefCell::new(None) };
}

/// The plan for one lowering, cleared when it drops.
struct Armed;

impl Armed {
    fn set(mode: Mode) -> Self {
        PLAN.with(|plan| {
            *plan.borrow_mut() = Some(Plan {
                mode,
                in_test_code: false,
                shielded: Vec::new(),
            });
        });
        Self
    }

    fn take(self) -> Mode {
        PLAN.with(|plan| plan.borrow_mut().take())
            .expect("the plan stays set until its guard takes it")
            .mode
    }
}

impl Drop for Armed {
    fn drop(&mut self) {
        PLAN.with(|plan| plan.borrow_mut().take());
    }
}

/// Runs `lower` with sites inside it treated as test code when `test_code`
/// is set. A lowering with no plan runs `lower` and nothing else.
pub(crate) fn within<T>(test_code: bool, lower: impl FnOnce() -> T) -> T {
    let before = PLAN.with(|plan| {
        plan.borrow_mut()
            .as_mut()
            .map(|plan| std::mem::replace(&mut plan.in_test_code, test_code))
    });
    let out = lower();
    if let Some(before) = before {
        PLAN.with(|plan| {
            if let Some(plan) = plan.borrow_mut().as_mut() {
                plan.in_test_code = before;
            }
        });
    }
    out
}

impl Lowering<'_> {
    /// Whether `cond` is a condition whose narrowing may change how a value is
    /// read on one of its edges: an `is` test, or an `==`/`!=` that compares a
    /// [`Ty::Tagged`] value with a literal, `null` or a class constant, alone or
    /// under `!`, `&&`, `||` and parentheses. It names every such shape
    /// `nvs_types::locals::narrow` reads, and some it does not.
    ///
    /// An equality narrows its tested side to a single value, or `?T` to `T`,
    /// and `nvs_types::locals::subject` takes only a variable or a property
    /// access as that side. A value stored at any representation but
    /// [`Ty::Tagged`] narrows to the same one (a single value is stored as its
    /// base, an enum case as its enum), so [`Lowering::untag_narrowed`] has
    /// nothing to change and taking the edge with the test false reads it at the
    /// representation it has. `int $n == 10` therefore narrows nothing here, and
    /// `?int $n == null` does. A variable is judged by its slot, which stays
    /// tagged after an earlier test narrowed it, so this over-approximates.
    pub(crate) fn narrows(&self, cond: &Expr, env: &Env) -> bool {
        match &cond.kind {
            ExprKind::Paren(inner)
            | ExprKind::Unary {
                op: AstUnaryOp::Not,
                expr: inner,
            } => self.narrows(inner, env),
            ExprKind::Binary {
                op: BinaryOp::And | BinaryOp::Or,
                lhs,
                rhs,
            } => self.narrows(lhs, env) || self.narrows(rhs, env),
            ExprKind::Binary {
                op: BinaryOp::Eq | BinaryOp::NotEq,
                lhs,
                rhs,
            } => {
                (is_narrowing_operand(rhs) && self.stored_tagged(lhs, env))
                    || (is_narrowing_operand(lhs) && self.stored_tagged(rhs, env))
            }
            ExprKind::TypeTest { .. } => true,
            _ => false,
        }
    }

    /// Whether `e` is a narrowing subject stored as [`Ty::Tagged`]: a variable
    /// whose slot (or `inout` pointee) is, or a property access whose checked
    /// type erases to it. A property access the checker recorded nothing for is
    /// counted as tagged, and nothing else is a subject.
    fn stored_tagged(&self, e: &Expr, env: &Env) -> bool {
        match &e.kind {
            ExprKind::Paren(inner) => self.stored_tagged(inner, env),
            ExprKind::Variable(span) => {
                let name = strip_sigil(span_text(self.src, *span));
                match env.get(name) {
                    Some(&(_, Ty::Ref)) => self.pointee_of(name) == Ty::Tagged,
                    Some(&(_, ty)) => ty == Ty::Tagged,
                    None => true,
                }
            }
            ExprKind::PropertyAccess { .. } => match self.exprs.lookup(e.span) {
                Some(ExprInfo::Property { ty, .. }) => {
                    erase_checked_ty(*ty, self.checked_types) == Ty::Tagged
                }
                _ => true,
            },
            _ => false,
        }
    }
}

/// An operand an `==` narrows the other side to: a literal, `null`, or a
/// class constant, an enum case among them.
fn is_narrowing_operand(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Paren(inner) => is_narrowing_operand(inner),
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Duration(_)
        | ExprKind::ClassConstAccess { .. } => true,
        ExprKind::Unary { expr, .. } => is_narrowing_operand(expr),
        _ => false,
    }
}

impl Lowering<'_> {
    /// Offers the site at `span` to the plan. In a survey it records the
    /// mutant and returns `false`; armed, it returns whether this is the
    /// site to change. With no plan, or inside test code, it returns `false`.
    pub(crate) fn mutation_site(
        &self,
        span: Span,
        operator: Operator,
        from: &'static str,
        to: &'static str,
    ) -> bool {
        PLAN.with(|plan| {
            let mut plan = plan.borrow_mut();
            let Some(plan) = plan.as_mut() else {
                return false;
            };
            if plan.in_test_code {
                return false;
            }
            match &mut plan.mode {
                Mode::Survey(found) => {
                    found.push(Mutant {
                        span,
                        file: span.file,
                        line: self.src.line_col(span.start).0 + 1,
                        operator,
                        from,
                        to,
                    });
                    false
                }
                Mode::Apply {
                    span: at,
                    operator: which,
                    applied,
                } => {
                    let hit = *at == span && *which == operator;
                    *applied |= hit;
                    hit
                }
            }
        })
    }

    /// Keeps the `true` or `false` literal `e` from being flipped, when `e`
    /// is one. Called where the literal's value is read by the checker as
    /// more than a value: a condition, a `match` subject, an `==` operand.
    pub(crate) fn shield_literal(&self, e: &Expr) {
        let mut e = e;
        while let ExprKind::Paren(inner) = &e.kind {
            e = inner;
        }
        if !matches!(e.kind, ExprKind::Bool(_)) {
            return;
        }
        PLAN.with(|plan| {
            if let Some(plan) = plan.borrow_mut().as_mut() {
                plan.shielded.push(e.span);
            }
        });
    }

    /// The literal `b` at `span`, flipped when this is the armed site.
    pub(crate) fn mutated_bool(&self, span: Span, b: bool) -> bool {
        let shielded = PLAN.with(|plan| {
            plan.borrow()
                .as_ref()
                .is_some_and(|plan| plan.shielded.contains(&span))
        });
        if shielded {
            return b;
        }
        let (from, to) = if b {
            ("true", "false")
        } else {
            ("false", "true")
        };
        b != self.mutation_site(span, Operator::BooleanLiteral, from, to)
    }

    /// `&&` or `||` at `whole`, swapped when this is the armed site and the
    /// expression narrows nothing ([`Self::narrows`]).
    pub(crate) fn mutated_connector(&self, whole: &Expr, op: BinaryOp, env: &Env) -> BinaryOp {
        if self.narrows(whole, env) {
            return op;
        }
        match op {
            BinaryOp::And
                if self.mutation_site(whole.span, Operator::LogicalConnector, "&&", "||") =>
            {
                BinaryOp::Or
            }
            BinaryOp::Or
                if self.mutation_site(whole.span, Operator::LogicalConnector, "||", "&&") =>
            {
                BinaryOp::And
            }
            _ => op,
        }
    }

    /// The operator `lower_binary` lowers at `whole`, once both operands are
    /// lowered at `lty` and `rty`: `op`, or its swap when this is the armed
    /// site, the swap keeps the result type, and an equality narrows nothing
    /// ([`Self::narrows`]).
    pub(crate) fn mutated_binary(
        &self,
        whole: &Expr,
        op: BinaryOp,
        lty: Ty,
        rty: Ty,
        env: &Env,
    ) -> BinaryOp {
        if !matches!(whole.kind, ExprKind::Binary { .. }) {
            return op;
        }
        let numeric = |ty: Ty| {
            matches!(
                ty,
                Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal | Ty::Tagged
            )
        };
        let fractional = lty == rty && matches!(lty, Ty::Float | Ty::Decimal);
        let (operator, swapped, from, to) = match op {
            BinaryOp::Lt => (Operator::RelationalBoundary, BinaryOp::LtEq, "<", "<="),
            BinaryOp::LtEq => (Operator::RelationalBoundary, BinaryOp::Lt, "<=", "<"),
            BinaryOp::Gt => (Operator::RelationalBoundary, BinaryOp::GtEq, ">", ">="),
            BinaryOp::GtEq => (Operator::RelationalBoundary, BinaryOp::Gt, ">=", ">"),
            BinaryOp::Eq | BinaryOp::NotEq if self.narrows(whole, env) => return op,
            BinaryOp::Eq => (Operator::EqualityNegated, BinaryOp::NotEq, "==", "!="),
            BinaryOp::NotEq => (Operator::EqualityNegated, BinaryOp::Eq, "!=", "=="),
            BinaryOp::Add | BinaryOp::Sub if !(numeric(lty) && numeric(rty)) => return op,
            BinaryOp::Add => (Operator::ArithmeticSwap, BinaryOp::Sub, "+", "-"),
            BinaryOp::Sub => (Operator::ArithmeticSwap, BinaryOp::Add, "-", "+"),
            BinaryOp::Mul if fractional => (Operator::ArithmeticSwap, BinaryOp::Div, "*", "/"),
            BinaryOp::Div if fractional => (Operator::ArithmeticSwap, BinaryOp::Mul, "/", "*"),
            _ => return op,
        };
        if self.mutation_site(whole.span, operator, from, to) {
            swapped
        } else {
            op
        }
    }

    /// An `if`, `elseif` or loop condition, lowered to the `bool` its branch
    /// tests, and inverted when this is the armed site. A condition that
    /// narrows a tagged value ([`Self::narrows`]), or is a bare `true` or
    /// `false`, is never inverted.
    pub(crate) fn lower_branch_cond(
        &mut self,
        cond: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        self.shield_literal(cond);
        let v = self.lower_truthy_cond(cond, env, cur);
        let mut bare = cond;
        while let ExprKind::Paren(inner) = &bare.kind {
            bare = inner;
        }
        if self.narrows(cond, env)
            || matches!(bare.kind, ExprKind::Bool(_))
            || !self.mutation_site(
                cond.span,
                Operator::BranchNegated,
                "condition",
                "!condition",
            )
        {
            return v;
        }
        self.emit(
            *cur,
            Ty::Bool,
            InstKind::UnOp {
                op: UnOp::Not,
                operand: v,
            },
        )
        .0
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};
    use nvs_syntax::parse_file;

    use super::*;
    use crate::print::print_program;

    /// Checks `src`, then gives its survey and the printed lowering of each
    /// mutant beside the printed lowering of the program with no change.
    fn lowered(src: &str) -> (Vec<Mutant>, String, Vec<String>) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        let module = nvs_hir::resolve_file(&stmts, map.file(file), &mut diags);
        let mut checked_types = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        let files = [nvs_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }];
        let enums =
            nvs_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let layouts = nvs_types::layout::build_class_layouts(&files, &module.graph);
        let found = mutants("<script>", &files, &exprs, &checked_types, &enums, &layouts);
        let plain =
            lower_program_with_tests("<script>", &files, &exprs, &checked_types, &enums, &layouts);
        let each = found
            .iter()
            .map(|mutant| {
                let program = lower_mutant(
                    "<script>",
                    &files,
                    &exprs,
                    &checked_types,
                    &enums,
                    &layouts,
                    mutant,
                );
                print_program(&program, map.file(file))
            })
            .collect();
        (found, print_program(&plain, map.file(file)), each)
    }

    fn rows(found: &[Mutant]) -> Vec<(usize, Operator, &'static str, &'static str)> {
        let mut rows: Vec<_> = found
            .iter()
            .map(|m| (m.line, m.operator, m.from, m.to))
            .collect();
        rows.sort_by_key(|row| (row.0, format!("{row:?}")));
        rows
    }

    const PRICE: &str = "<?nvs
use Core\\Test;

final class Price {
    public function check(int $a, int $b, float $x, float $y, bool $f): bool {
        int $sum = $a + $b;
        float $scaled = $x * $y;
        if ($a < $b) {
            return $f;
        }
        while ($a >= 10) {
            $a = $a - 1;
        }
        bool $same = $a == $b;
        bool $flag = true;
        return $same && $f;
    }
}

final class PriceTest {
    #[Test]
    public function aSmallerPriceIsCheaper(): void {
        int $n = 1 + 2;
        if ($n < 4) {
            Test::assertSame(true, $n == 3 && $n > 0);
        }
    }
}
";

    /// Each of the six operators gives the mutants it should, at the line it
    /// is written on, and the test class gives none.
    #[test]
    fn each_operator_yields_its_mutants_and_test_code_yields_none() {
        let (found, _, _) = lowered(PRICE);
        use Operator::*;
        assert_eq!(
            rows(&found),
            vec![
                (6, ArithmeticSwap, "+", "-"),
                (7, ArithmeticSwap, "*", "/"),
                (8, BranchNegated, "condition", "!condition"),
                (8, RelationalBoundary, "<", "<="),
                (11, BranchNegated, "condition", "!condition"),
                (11, RelationalBoundary, ">=", ">"),
                (12, ArithmeticSwap, "-", "+"),
                (14, EqualityNegated, "==", "!="),
                (15, BooleanLiteral, "true", "false"),
                (16, LogicalConnector, "&&", "||"),
            ],
        );
    }

    /// Every mutant lowers, and each changes the lowered program.
    #[test]
    fn every_mutant_lowers_to_a_changed_program() {
        let (found, plain, each) = lowered(PRICE);
        assert!(!found.is_empty());
        for (mutant, program) in found.iter().zip(&each) {
            assert_ne!(*program, plain, "{mutant:?} changed nothing");
        }
    }

    /// A condition the checker narrows a type on keeps its direction, and a
    /// literal the checker reads as more than a value keeps its value.
    #[test]
    fn a_narrowing_condition_and_a_read_literal_are_not_mutated() {
        let (found, _, _) = lowered(
            "<?nvs
final class Stock {
    public function count(?int $n, int $m): int {
        if ($n != null && $m > 0) {
            return $n;
        }
        while (true) {
            return $m;
        }
    }
}
",
        );
        assert_eq!(
            rows(&found),
            vec![(4, Operator::RelationalBoundary, ">", ">=")],
        );
    }
}
