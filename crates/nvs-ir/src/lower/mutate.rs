//! `rule:testing/mutation-testing`'s mutants: one changed site in the lowered
//! program each, made while the checked tree is lowered.
//!
//! # A survey, then one lowering with every mutant behind a switch
//!
//! [`mutants`] lowers the program once in *survey* mode, and every site that
//! could take a change records a [`Mutant`] instead of changing.
//! [`lower_schema`] lowers it again with many of those mutants armed at once,
//! each with a number from 1. An armed site lowers both its forms behind one
//! test, [`InstKind::MutantOn`], which compares the number with the run's
//! active mutant: `if (active == k) changed else original`. So a mutation run
//! compiles the program once, and sets the active number before each mutant's
//! tests. A `true` or `false`, a condition and a connector's first operand are
//! `bool` values, so their change is the value compared unequal to the test.
//! A changed operator lowers each operator on its own edge of the test, so a
//! form that can throw runs only when it is chosen.
//!
//! [`lower_fixed`] lowers the program with some mutants' changes made and no
//! test, and [`lower_mutant`] with one. The equivalence check fixes at most
//! one mutant per function in each lowering, and compiles each function a
//! change is in.
//!
//! Parse and check ran once before every pass; none checks anything again.
//! The plan reaches the sites through a thread-local set for the length of one
//! [`lower_program_as`] call, because a program's functions are lowered by
//! nine [`Lowering::new`] callers, anonymous functions and generators among
//! them, all on the calling thread. The guard clears it on every exit, so a
//! lowering outside these functions never sees a plan.
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
//! # Test code and `#[NoMutate]` code are never mutated
//!
//! A method `rule:testing/tests-never-reach-a-build` names as test code is
//! lowered with the survey turned off, and so is every anonymous function and
//! generator lowered inside it. A method or a class with `#[NoMutate]` is
//! lowered the same way.
//!
//! # Glue makes no mutant
//!
//! A site whose nearest enclosing call is a `Core\Log` method, or that is in
//! the arguments of the `new` a `throw` throws, changes a message and never a
//! result: no test is meant to read it. [`in_call`] wraps every call the
//! lowering makes and says whether that call is glue; the enumeration of glue
//! calls is [`Lowering::is_glue_call`] and [`Lowering::lower_throw`]'s
//! [`throwing`]. Output written with `echo` or `Core\Html` is not glue: in a
//! web program the page is the result.
//!
//! # Every mutant names its function
//!
//! [`within`] sets the label of the method being lowered, and every site in
//! it, an anonymous function's and a generator's included, records that
//! label. A user finds a mutant by the method name they wrote.
//!
//! # What it spends
//!
//! Nothing for a lowering with no plan: each site reads one thread-local and
//! returns. A survey holds one [`Mutant`] per site, its function label
//! included. A schema holds one map entry per armed mutant while it lowers,
//! and its program has, per armed site, one [`InstKind::MutantOn`] and either
//! one compare or a branch with both operators. That code is the run's
//! alone: a program with a switch is never written to the artifact cache.

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

/// One mutant: where it is, the function it is in, which operator made it,
/// and the text before and after.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Mutant {
    /// The label of the method or property hook the site is lowered in, as
    /// the lowering names it (`Shop\Cart::total`). A site in an anonymous
    /// function or a generator body has the label of the method it is written
    /// in, and a site in a file's own statements has the file's script label.
    pub function: String,
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
    let guard = Guard::set(Mode::Survey(Vec::new()));
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
        Mode::Arm { .. } => unreachable!("a survey is armed as one"),
    }
}

/// A program lowered with one mutant's change made, and the function the
/// change is in.
#[derive(Debug)]
pub struct Mutated {
    /// The whole program, test code included.
    pub program: crate::ir::Program,
    /// The name of the [`crate::ir::Function`] the change is in. For a site in
    /// an anonymous function or a generator body it is that function's own
    /// name, not [`Mutant::function`]'s method label, so a caller can find the
    /// one compiled function the change touches.
    pub changed: String,
}

/// The program, with its test code, lowered with `mutant`'s one change made
/// and no switch.
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
) -> Mutated {
    let (program, mut changed) = lower_fixed(
        script,
        files,
        exprs,
        checked_types,
        enums,
        layouts,
        &[mutant],
    );
    Mutated {
        program,
        changed: changed.pop().expect("one mutant was armed"),
    }
}

/// The program, with its test code, lowered with every one of `fixed`'s
/// changes made and no switch, and the name of the [`crate::ir::Function`]
/// each change is in, in `fixed`'s order. A caller that fixes at most one
/// mutant per function reads each changed function as [`lower_mutant`] would
/// have lowered it alone, from one lowering for all of them.
///
/// # Panics
///
/// If a mutant matches no site, as [`lower_mutant`] does.
#[must_use]
pub fn lower_fixed(
    script: &str,
    files: &[nvs_types::ProgramFile<'_>],
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
    layouts: &ClassLayoutTable,
    fixed: &[&Mutant],
) -> (crate::ir::Program, Vec<String>) {
    lower_armed(
        (script, files),
        exprs,
        checked_types,
        enums,
        layouts,
        fixed,
        false,
    )
}

/// The program, with its test code, lowered with every one of `armed` behind
/// the mutant switch: the change at `armed[i]`'s site is made when the run's
/// active mutant is `i + 1` ([`InstKind::MutantOn`]). With the active number
/// at 0 the program runs unchanged.
///
/// # Panics
///
/// If a mutant matches no site, as [`lower_mutant`] does.
#[must_use]
pub fn lower_schema(
    script: &str,
    files: &[nvs_types::ProgramFile<'_>],
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
    layouts: &ClassLayoutTable,
    armed: &[&Mutant],
) -> crate::ir::Program {
    lower_armed(
        (script, files),
        exprs,
        checked_types,
        enums,
        layouts,
        armed,
        true,
    )
    .0
}

/// The lowering both [`lower_mutant`] and [`lower_schema`] run, and the name
/// of the function each armed mutant's site is in.
fn lower_armed(
    (script, files): (&str, &[nvs_types::ProgramFile<'_>]),
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
    layouts: &ClassLayoutTable,
    armed: &[&Mutant],
    switched: bool,
) -> (crate::ir::Program, Vec<String>) {
    let sites = armed
        .iter()
        .enumerate()
        .map(|(at, mutant)| ((mutant.span, mutant.operator), at))
        .collect();
    let guard = Guard::set(Mode::Arm {
        sites,
        switched,
        applied: vec![None; armed.len()],
    });
    let mut program = lower_program_as(
        script,
        files,
        exprs,
        checked_types,
        enums,
        layouts,
        Tests::Lower,
    );
    let Mode::Arm { applied, .. } = guard.take() else {
        unreachable!("an armed lowering is armed as one")
    };
    program.mutant_switch = switched;
    let changed = applied
        .into_iter()
        .zip(armed)
        .map(|(applied, mutant)| {
            applied.unwrap_or_else(|| {
                panic!(
                    "nvs-ir: no site matches the mutant at {:?} ({:?}); it was not listed for this program",
                    mutant.span, mutant.operator
                )
            })
        })
        .collect();
    (program, changed)
}

enum Mode {
    Survey(Vec<Mutant>),
    Arm {
        /// Each armed site, by its span and operator, at its index in the
        /// armed list.
        sites: std::collections::HashMap<(Span, Operator), usize>,
        /// Whether each change is made behind the switch, or always.
        switched: bool,
        /// The name of the function each change was made in, once it is.
        applied: Vec<Option<String>>,
    },
}

/// What [`Lowering::mutation_site`] says a site lowers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Site {
    /// The original form.
    Keep,
    /// The changed form, with no test.
    Change,
    /// Both forms, the changed one when mutant `k` is the active one.
    When(u32),
}

struct Plan {
    mode: Mode,
    /// Set while a test-code or `#[NoMutate]` method is lowered: no site there
    /// is listed or changed.
    in_test_code: bool,
    /// Set while the arguments of a glue call are lowered ([`in_call`]): no
    /// site there is listed or changed.
    in_glue: bool,
    /// The `new` expression a `throw` is about to lower, whose arguments are
    /// glue ([`throwing`]).
    thrown: Option<Span>,
    /// The label of the function being lowered, which each listed mutant names.
    function: String,
    /// The `true` and `false` literals [`Lowering::shield_literal`] named.
    shielded: Vec<Span>,
}

thread_local! {
    static PLAN: RefCell<Option<Plan>> = const { RefCell::new(None) };
}

/// The plan for one lowering, cleared when it drops.
struct Guard;

impl Guard {
    fn set(mode: Mode) -> Self {
        PLAN.with(|plan| {
            *plan.borrow_mut() = Some(Plan {
                mode,
                in_test_code: false,
                in_glue: false,
                thrown: None,
                function: String::new(),
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

impl Drop for Guard {
    fn drop(&mut self) {
        PLAN.with(|plan| plan.borrow_mut().take());
    }
}

/// Runs `lower` as the lowering of the function labelled `function`, with no
/// mutant made inside it when `unmutated` is set: test code, and a method or
/// class with `#[NoMutate]`. A lowering with no plan runs `lower` and nothing
/// else, so it pays no allocation for the label.
pub(crate) fn within<T>(function: &str, unmutated: bool, lower: impl FnOnce() -> T) -> T {
    let before = PLAN.with(|plan| {
        plan.borrow_mut().as_mut().map(|plan| {
            (
                std::mem::replace(&mut plan.in_test_code, unmutated),
                std::mem::replace(&mut plan.in_glue, false),
                std::mem::replace(&mut plan.function, function.to_owned()),
            )
        })
    });
    let out = lower();
    if let Some((test_code, glue, function)) = before {
        PLAN.with(|plan| {
            if let Some(plan) = plan.borrow_mut().as_mut() {
                plan.in_test_code = test_code;
                plan.in_glue = glue;
                plan.function = function;
            }
        });
    }
    out
}

/// Runs `lower`, the lowering of one call, as glue when `glue` says the call
/// is glue: a `Core\Log` method, or the `new` a `throw` throws. A call that is
/// not glue turns glue off for its own arguments, so the nearest enclosing
/// call decides. With no plan, `glue` is never called.
pub(crate) fn in_call<'a, T>(
    lowering: &mut Lowering<'a>,
    glue: impl FnOnce(&Lowering<'a>) -> bool,
    lower: impl FnOnce(&mut Lowering<'a>) -> T,
) -> T {
    if PLAN.with(|plan| plan.borrow().is_none()) {
        return lower(lowering);
    }
    let glue = glue(lowering);
    let before = PLAN.with(|plan| {
        plan.borrow_mut()
            .as_mut()
            .map(|plan| std::mem::replace(&mut plan.in_glue, glue))
    });
    let out = lower(lowering);
    if let Some(before) = before {
        PLAN.with(|plan| {
            if let Some(plan) = plan.borrow_mut().as_mut() {
                plan.in_glue = before;
            }
        });
    }
    out
}

/// Records that `thrown`, the operand of a `throw`, is about to be lowered.
/// When it is a `new`, [`is_thrown`] then names it as glue.
pub(crate) fn throwing(thrown: &Expr) {
    let thrown = thrown.unparenthesized();
    if !matches!(thrown.kind, ExprKind::New { .. }) {
        return;
    }
    PLAN.with(|plan| {
        if let Some(plan) = plan.borrow_mut().as_mut() {
            plan.thrown = Some(thrown.span);
        }
    });
}

/// Whether `span` is the `new` a `throw` throws ([`throwing`]).
pub(crate) fn is_thrown(span: Span) -> bool {
    PLAN.with(|plan| {
        plan.borrow()
            .as_ref()
            .is_some_and(|plan| plan.thrown == Some(span))
    })
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
    /// Whether the call at `call` is glue by what it calls: a method of
    /// `Core\Log`. The `new` a `throw` throws is the other glue call
    /// ([`is_thrown`]).
    pub(crate) fn is_glue_call(&self, call: Span) -> bool {
        matches!(
            self.exprs.lookup(call),
            Some(ExprInfo::Call(resolved)) if resolved.class.to_string() == r"Core\Log"
        )
    }

    /// Offers the site at `span` to the plan. In a survey it records the
    /// mutant and returns [`Site::Keep`]; armed, it says whether this site
    /// changes, and whether behind the switch. With no plan, inside test code,
    /// or in a glue call's arguments, it returns [`Site::Keep`].
    pub(crate) fn mutation_site(
        &self,
        span: Span,
        operator: Operator,
        from: &'static str,
        to: &'static str,
    ) -> Site {
        PLAN.with(|plan| {
            let mut plan = plan.borrow_mut();
            let Some(plan) = plan.as_mut() else {
                return Site::Keep;
            };
            if plan.in_test_code || plan.in_glue {
                return Site::Keep;
            }
            match &mut plan.mode {
                Mode::Survey(found) => {
                    found.push(Mutant {
                        function: plan.function.clone(),
                        span,
                        file: span.file,
                        line: self.src.line_col(span.start).0 + 1,
                        operator,
                        from,
                        to,
                    });
                    Site::Keep
                }
                Mode::Arm {
                    sites,
                    switched,
                    applied,
                } => {
                    let Some(&at) = sites.get(&(span, operator)) else {
                        return Site::Keep;
                    };
                    applied[at] = Some(self.fn_label.clone());
                    if *switched {
                        Site::When(
                            u32::try_from(at + 1).expect("a run arms fewer than 2^32 mutants"),
                        )
                    } else {
                        Site::Change
                    }
                }
            }
        })
    }

    /// `v`, a `bool`, compared unequal to the test of mutant `k`: `v` while
    /// another mutant is active, and `!v` while `k` is.
    fn flip_when(&mut self, k: u32, v: ValueId, cur: BlockId) -> ValueId {
        let on = self.emit(cur, Ty::Bool, InstKind::MutantOn(k)).0;
        self.emit(
            cur,
            Ty::Bool,
            InstKind::BinOp {
                op: BinOp::NotEq,
                lhs: v,
                rhs: on,
            },
        )
        .0
    }

    /// The condition and short-circuit value of a `&&` or `||` whose first
    /// operand is `lhs`, a `bool` in `lhs_end`. With no switch they are `lhs`
    /// and `short`, the connector's own result when it stops early. Under
    /// mutant `k` the condition is `lhs` compared unequal to the test, and
    /// the short-circuit value is `lhs` itself: `&&` and `||` both stop early
    /// with the value of their first operand, so swapping one for the other
    /// only inverts which edge is short.
    pub(crate) fn connector_branch(
        &mut self,
        lhs: ValueId,
        short: bool,
        switch: Option<u32>,
        lhs_end: BlockId,
    ) -> (ValueId, ValueId) {
        match switch {
            None => (
                lhs,
                self.emit(lhs_end, Ty::Bool, InstKind::ConstBool(short)).0,
            ),
            Some(k) => (self.flip_when(k, lhs, lhs_end), lhs),
        }
    }

    /// Lowers both forms of a changed operator, each on its own edge of
    /// mutant `k`'s test, and joins their results. `span` names the two edges.
    pub(crate) fn lower_switched(
        &mut self,
        (k, span): (u32, Span),
        env: &mut Env,
        cur: &mut BlockId,
        changed: impl FnOnce(&mut Self, &mut Env, &mut BlockId) -> (ValueId, Ty),
        original: impl FnOnce(&mut Self, &mut Env, &mut BlockId) -> (ValueId, Ty),
    ) -> (ValueId, Ty) {
        let on = self.emit(*cur, Ty::Bool, InstKind::MutantOn(k)).0;
        let changed_block = self.new_block();
        let original_block = self.new_block();
        let merge = self.new_block();
        let then_edge = self.ids.next_edge(span);
        let else_edge = self.ids.next_edge(span);
        self.seal(
            *cur,
            Terminator::Branch {
                cond: on,
                then_block: changed_block,
                then_edge,
                else_block: original_block,
                else_edge,
            },
        );
        let mut changed_end = changed_block;
        let (cv, cty) = changed(self, env, &mut changed_end);
        self.seal(changed_end, Terminator::Jump(merge));
        let mut original_end = original_block;
        let (ov, oty) = original(self, env, &mut original_end);
        self.seal(original_end, Terminator::Jump(merge));
        assert_eq!(
            cty, oty,
            "nvs-ir: a mutant's two forms have one result type"
        );
        let (v, _) = self.emit(
            merge,
            oty,
            InstKind::Phi {
                incoming: vec![(changed_end, cv), (original_end, ov)],
            },
        );
        *cur = merge;
        (v, oty)
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

    /// The literal `b` at `span`, lowered in `cur`: flipped when this is the
    /// armed site, and compared with the switch under a schema.
    pub(crate) fn lower_bool_literal(
        &mut self,
        span: Span,
        b: bool,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        let shielded = PLAN.with(|plan| {
            plan.borrow()
                .as_ref()
                .is_some_and(|plan| plan.shielded.contains(&span))
        });
        let (from, to) = if b {
            ("true", "false")
        } else {
            ("false", "true")
        };
        let site = if shielded {
            Site::Keep
        } else {
            self.mutation_site(span, Operator::BooleanLiteral, from, to)
        };
        let (v, _) = self.emit(
            cur,
            Ty::Bool,
            InstKind::ConstBool(b != (site == Site::Change)),
        );
        match site {
            Site::When(k) => (self.flip_when(k, v, cur), Ty::Bool),
            Site::Keep | Site::Change => (v, Ty::Bool),
        }
    }

    /// `&&` or `||` at `whole`: swapped when this is the armed site and the
    /// expression narrows nothing ([`Self::narrows`]), and with the mutant's
    /// number beside the unswapped one under a schema
    /// ([`Self::connector_branch`]).
    pub(crate) fn mutated_connector(
        &self,
        whole: &Expr,
        op: BinaryOp,
        env: &Env,
    ) -> (BinaryOp, Option<u32>) {
        if self.narrows(whole, env) {
            return (op, None);
        }
        let (swapped, from, to) = match op {
            BinaryOp::And => (BinaryOp::Or, "&&", "||"),
            BinaryOp::Or => (BinaryOp::And, "||", "&&"),
            _ => return (op, None),
        };
        match self.mutation_site(whole.span, Operator::LogicalConnector, from, to) {
            Site::Keep => (op, None),
            Site::Change => (swapped, None),
            Site::When(k) => (op, Some(k)),
        }
    }

    /// The operator `lower_binary` lowers at `whole`, once both operands are
    /// lowered at `lty` and `rty`: `op`, or its swap when this is the armed
    /// site, the swap keeps the result type, and an equality narrows nothing
    /// ([`Self::narrows`]). Under a schema it is `op`, with the swap and the
    /// mutant's number beside it for [`Self::lower_switched`].
    pub(crate) fn mutated_binary(
        &self,
        whole: &Expr,
        op: BinaryOp,
        lty: Ty,
        rty: Ty,
        env: &Env,
    ) -> (BinaryOp, Option<(BinaryOp, u32)>) {
        if !matches!(whole.kind, ExprKind::Binary { .. }) {
            return (op, None);
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
            BinaryOp::Eq | BinaryOp::NotEq if self.narrows(whole, env) => return (op, None),
            BinaryOp::Eq => (Operator::EqualityNegated, BinaryOp::NotEq, "==", "!="),
            BinaryOp::NotEq => (Operator::EqualityNegated, BinaryOp::Eq, "!=", "=="),
            BinaryOp::Add | BinaryOp::Sub if !(numeric(lty) && numeric(rty)) => return (op, None),
            BinaryOp::Add => (Operator::ArithmeticSwap, BinaryOp::Sub, "+", "-"),
            BinaryOp::Sub => (Operator::ArithmeticSwap, BinaryOp::Add, "-", "+"),
            BinaryOp::Mul if fractional => (Operator::ArithmeticSwap, BinaryOp::Div, "*", "/"),
            BinaryOp::Div if fractional => (Operator::ArithmeticSwap, BinaryOp::Mul, "/", "*"),
            _ => return (op, None),
        };
        match self.mutation_site(whole.span, operator, from, to) {
            Site::Keep => (op, None),
            Site::Change => (swapped, None),
            Site::When(k) => (op, Some((swapped, k))),
        }
    }

    /// An `if`, `elseif` or loop condition, lowered to the `bool` its branch
    /// tests: inverted when this is the armed site, and compared with the
    /// switch under a schema. A condition that narrows a tagged value
    /// ([`Self::narrows`]), or is a bare `true` or `false`, is never inverted.
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
        if self.narrows(cond, env) || matches!(bare.kind, ExprKind::Bool(_)) {
            return v;
        }
        match self.mutation_site(
            cond.span,
            Operator::BranchNegated,
            "condition",
            "!condition",
        ) {
            Site::Keep => v,
            Site::Change => {
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
            Site::When(k) => self.flip_when(k, v, *cur),
        }
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
                )
                .program;
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

    /// Every mutant names the method it is written in, a site in an anonymous
    /// function or a generator body included, and a method or class with
    /// `#[NoMutate]` makes none.
    #[test]
    fn a_survey_names_the_function_each_mutant_is_in() {
        let (found, _, _) = lowered(
            "<?nvs
use Core\\Test\\NoMutate;

final class Cart {
    public function total(int $a, int $b): int {
        var $add = fn(int $x): int => $x + $b;
        return $add($a) - 1;
    }

    public function items(int $n): Iterator<int> {
        while ($n > 0) {
            yield $n;
            $n = $n - 1;
        }
    }

    #[NoMutate(because: \"only formats a line\")]
    public function label(int $n): int {
        return $n + 1;
    }
}

#[NoMutate(because: \"generated code\")]
final class Codec {
    public function size(int $n): int {
        return $n * 2 + 1;
    }
}
",
        );
        let mut rows: Vec<(String, usize, &'static str)> = found
            .iter()
            .map(|m| (m.function.clone(), m.line, m.from))
            .collect();
        rows.sort();
        assert_eq!(
            rows,
            vec![
                ("Cart::items".to_owned(), 11, ">"),
                ("Cart::items".to_owned(), 11, "condition"),
                ("Cart::items".to_owned(), 13, "-"),
                ("Cart::total".to_owned(), 6, "+"),
                ("Cart::total".to_owned(), 7, "-"),
            ],
        );
    }

    /// A site in a `Core\Log` call's arguments or a thrown exception's makes no
    /// mutant. A call nested inside one decides for its own arguments.
    #[test]
    fn a_log_call_and_an_exception_message_make_no_mutant() {
        let (found, _, _) = lowered(
            "<?nvs
use Core\\Log;

final class Order {
    public function place(int $n): int {
        Log::write(Log::Level::Info, 'placing', ['n' => $n + 1]);
        if ($n < 0) {
            throw new LogicError('bad count ' . ($n - 1));
        }
        Log::write(Log::Level::Info, 'size', ['n' => $this->size($n + 3)]);
        return $n - 2;
    }

    public function size(int $n): int {
        return $n;
    }
}
",
        );
        use Operator::*;
        assert_eq!(
            rows(&found),
            vec![
                (7, BranchNegated, "condition", "!condition"),
                (7, RelationalBoundary, "<", "<="),
                (10, ArithmeticSwap, "+", "-"),
                (11, ArithmeticSwap, "-", "+"),
            ],
        );
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
