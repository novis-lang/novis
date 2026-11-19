//! The table of cheaper forms
//! (`rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`).
//!
//! Each [`Row`] is one shape a program may be written in, the form that gives
//! the same result for less work, and what must be proven before the compiler
//! may compile one as the other. The table is closed: a goal adds a row,
//! together with its measurement, and nothing else decides a form. The
//! lowering asks it at the four places a shape is still a tree — a call, a
//! `foreach` subject, a `try` and an array literal — so there is one place to
//! read, test and switch off ([`crate::lower::without_cheaper_forms`]).
//!
//! A row is one of two kinds, by its [`Proof`]:
//!
//! - **[`Proof::Static`]**: the lowering matches it on resolved calls with
//!   static types and emits the fast form directly. Each [`Condition`] is one
//!   clause of the row's "holds when" cell; where one fails, the form is
//!   compiled as written.
//! - **[`Proof::Callback`]**: the lowering emits the form as written and records
//!   a candidate, and the pass that runs after `crate::effects` reads the
//!   callback's summary. A callback that writes nothing and cannot fail gives
//!   the fast form; one that writes nothing but can fail gives a warning that
//!   carries the fast form; one with an effect, or one the compiler cannot
//!   name, gives nothing.
//!
//! Every row owes conformance cases under [`CASES_DIR`], which
//! `every_form_row_has_its_cases` counts ([`Row::cases`]). Each case prints what
//! the form returns, so `bun nv suite tests/conformance/ --no-cheaper-forms-too`
//! compares every row both ways.
//!
//! The matchers below the table are what a row's site asks: each takes the
//! written tree and the checker's resolved calls, and gives the fast form or
//! `None`. The site applies one only while `crate::lower::cheaper_forms_on` holds.
//!
//! **What it spends:** while compiling, one match of each `Core` call, null
//! test and `try` lowered against the rows whose stage applies it. At run time, nothing.

use nvs_diagnostics::{Diagnostic, SourceFile, Span, code};
use nvs_syntax::ast::BinaryOp;
use nvs_syntax::ast::{
    Arg, ArrayItem, AssignOp, Block, CallArgs, CatchClause, Expr, ExprKind, FnBody, FnExpr, Stmt,
    StmtKind, Type, UnaryOp,
};
use nvs_types::expr_table::{ExprInfo, ExprTypeTable};
use nvs_types::ty::TypeInterner;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::effects::{Summaries, Touches};
use crate::ids::ValueId;
use crate::ir::{FormCandidate, Function, Helper, InstKind, Terminator, Touch, UserCode};
use crate::ty::Ty;
use crate::{span_text, strip_sigil};

/// Where a row's conformance cases live, under the repository root.
pub const CASES_DIR: &str = "tests/conformance/forms";

/// What the compiler does with one written form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The fast form is compiled, silently.
    Fast,
    /// The form stays as written and is a warning that carries the fast form
    /// as an alternative a person picks.
    Warning,
    /// The form stays as written, with no warning.
    AsWritten,
}

/// One clause of a [`Proof::Static`] row's "holds when" cell.
#[derive(Clone, Copy, Debug)]
pub struct Condition {
    /// The case name's suffix for the program where this clause fails.
    pub case: &'static str,
    /// What the program does where this clause fails.
    pub fails_when: &'static str,
}

/// What must be proven before a row's fast form replaces the written one.
#[derive(Clone, Copy, Debug)]
pub enum Proof {
    /// Read from the resolved call and the static types while lowering: every
    /// condition holds, or the form is compiled as written.
    Static(&'static [Condition]),
    /// The callback's effect summary and its error edges decide the outcome.
    Callback,
}

/// The folder under [`CASES_DIR`] a row's cases live in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Folder {
    /// A call form: `match`, `count`, `first`, `filter`, a caught conversion,
    /// and a method call the compiler can name.
    Calls,
    /// A loop, a literal or a static call.
    Loops,
    /// A form with a callback.
    Callbacks,
}

impl Folder {
    /// The folder's name under [`CASES_DIR`].
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::Loops => "loops",
            Self::Callbacks => "callbacks",
        }
    }
}

/// One row of the table.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    /// The row's id, which starts the name of each of its cases.
    pub id: &'static str,
    /// The form as a program writes it.
    pub written: &'static str,
    /// The form the compiler runs where the row holds.
    pub fast: &'static str,
    /// What must be proven first.
    pub proof: Proof,
    /// Where its cases live.
    pub folder: Folder,
    /// The goal stage of `cheaper-forms` that applies the row.
    pub stage: u8,
}

/// One case a row owes: its file name under the row's folder, and what the
/// compiler does with the program in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    /// `<row id>-<outcome>.nvst`.
    pub file: String,
    /// What the compiler does with the case's form.
    pub outcome: Outcome,
}

impl Row {
    /// Every case the row owes. A [`Proof::Static`] row owes one where it holds
    /// and one for each condition that fails; a [`Proof::Callback`] row owes
    /// one where the callback is proven, one where it can fail and one where
    /// it has an effect.
    #[must_use]
    pub fn cases(&self) -> Vec<Case> {
        let case = |suffix: &str, outcome| Case {
            file: format!("{}-{suffix}.nvst", self.id),
            outcome,
        };
        match self.proof {
            Proof::Static(conditions) => std::iter::once(case("holds", Outcome::Fast))
                .chain(
                    conditions
                        .iter()
                        .map(|condition| case(condition.case, Outcome::AsWritten)),
                )
                .collect(),
            Proof::Callback => vec![
                case("proven", Outcome::Fast),
                case("can-fail", Outcome::Warning),
                case("has-effect", Outcome::AsWritten),
            ],
        }
    }
}

/// The clauses both caught-conversion rows share: the `try` is exactly one
/// conversion assigned, caught by one arm that assigns `null`.
const CAUGHT: &[Condition] = &[
    Condition {
        case: "wider-body",
        fails_when: "the `try` body has more than the one assignment",
    },
    Condition {
        case: "operand-can-fail",
        fails_when: "the operand is a call or a property with a hook, so the conversion is not the body's only failure",
    },
    Condition {
        case: "two-catch-arms",
        fails_when: "there is more than one `catch` arm",
    },
    Condition {
        case: "narrower-catch",
        fails_when: "the `catch` class neither is nor contains the class the conversion throws",
    },
    Condition {
        case: "catch-variable-read",
        fails_when: "the `catch` arm reads its variable",
    },
    Condition {
        case: "catch-body-not-null",
        fails_when: "the `catch` body is anything but `$v = null;`",
    },
    Condition {
        case: "has-finally",
        fails_when: "the `try` has a `finally`",
    },
];

/// The clauses of the caught `parse` row: [`CAUGHT`], and `K` declares
/// `tryParse`.
const CAUGHT_PARSE: &[Condition] = &[
    CAUGHT[0],
    CAUGHT[1],
    CAUGHT[2],
    CAUGHT[3],
    CAUGHT[4],
    CAUGHT[5],
    CAUGHT[6],
    Condition {
        case: "no-try-parse",
        fails_when: "the class declares no `tryParse`",
    },
];

/// Every row, in the order the rule's two tables list them.
pub const ROWS: &[Row] = &[
    Row {
        id: "match-null-test",
        written: "`Core\\Regex::match(s, p)` used only in `== null`, `!= null` or `is null`",
        fast: "`Core\\Regex::matches(s, p)`",
        proof: Proof::Static(&[Condition {
            case: "has-from",
            fails_when: "the call has a `from` option",
        }]),
        folder: Folder::Calls,
        stage: 2,
    },
    Row {
        id: "count-of-keys-or-values",
        written: "`Core\\Arr::count` of `keys(a)` or `values(a)`",
        fast: "`Core\\Arr::count(a)`",
        proof: Proof::Static(&[]),
        folder: Folder::Calls,
        stage: 2,
    },
    Row {
        id: "first-of-keys-or-values",
        written: "`Core\\Arr::first` of `keys(a)`, `first` of `values(a)`",
        fast: "`firstKey(a)`, `first(a)`",
        proof: Proof::Static(&[]),
        folder: Folder::Calls,
        stage: 2,
    },
    Row {
        id: "filter-contains",
        written: "`Core\\Arr::filter(a, fn(x) => !Core\\Arr::contains(b, x))`, and the same without `!`",
        fast: "`diff(a, b)`, `intersect(a, b)`",
        proof: Proof::Static(&[
            Condition {
                case: "b-assigned",
                fails_when: "`b` is assigned in the function",
            },
            Condition {
                case: "array-elements",
                fails_when: "the element type is an array",
            },
        ]),
        folder: Folder::Calls,
        stage: 2,
    },
    Row {
        id: "caught-conversion",
        written: "`try { $v = X as T; } catch (C) { $v = null; }`",
        fast: "`$v = X as ?T;`",
        proof: Proof::Static(CAUGHT),
        folder: Folder::Calls,
        stage: 2,
    },
    Row {
        id: "caught-parse",
        written: "`try { $v = K::parse(X); } catch (C) { $v = null; }`",
        fast: "`$v = K::tryParse(X);`",
        proof: Proof::Static(CAUGHT_PARSE),
        folder: Folder::Calls,
        stage: 2,
    },
    Row {
        id: "foreach-keys-or-values",
        written: "`foreach (Core\\Arr::keys(m) as $k)`, `foreach (Core\\Arr::values(m) as $v)`",
        fast: "the loop over `m` itself, binding the key or the value",
        proof: Proof::Static(&[]),
        folder: Folder::Loops,
        stage: 3,
    },
    Row {
        id: "foreach-range",
        written: "`foreach (Core\\Arr::range(a, b, step) as $i)`",
        fast: "a counted loop whose direction is decided once, before the first step",
        proof: Proof::Static(&[Condition {
            case: "bad-arguments",
            fails_when: "`range` throws for its arguments, which happens before the first step",
        }]),
        folder: Folder::Loops,
        stage: 3,
    },
    Row {
        id: "unread-foreach-key",
        written: "a `foreach` key binding the body never reads",
        fast: "no key is made",
        proof: Proof::Static(&[]),
        folder: Folder::Loops,
        stage: 3,
    },
    Row {
        id: "constant-key-literal",
        written: "an array literal whose keys are all constants, no two equal, with no spread",
        fast: "one `ArrayNew` with its entries",
        proof: Proof::Static(&[
            Condition {
                case: "computed-key",
                fails_when: "a key is not a constant",
            },
            Condition {
                case: "equal-keys",
                fails_when: "two keys are equal",
            },
            Condition {
                case: "has-spread",
                fails_when: "the literal has a spread",
            },
        ]),
        folder: Folder::Loops,
        stage: 3,
    },
    Row {
        id: "static-member",
        written: "`static::m()` and `static::NAME`",
        fast: "the direct call and the constant",
        proof: Proof::Static(&[
            Condition {
                case: "method-overridden",
                fails_when: "a class in the program overrides `m`",
            },
            Condition {
                case: "constant-redeclared",
                fails_when: "a class in the program declares `NAME` again",
            },
        ]),
        folder: Folder::Loops,
        stage: 3,
    },
    Row {
        id: "named-method-call",
        written: "`$x->m()` on a receiver of static class `C`",
        fast: "the direct call",
        proof: Proof::Static(&[Condition {
            case: "overridden-below",
            fails_when: "a class below `C` declares `m` again, so `m` has more than one body at or below `C`",
        }]),
        folder: Folder::Calls,
        stage: 9,
    },
    Row {
        id: "property-getter",
        written: "`$x->m()` where the one body of `m` is `return $this->p;`",
        fast: "the read of `p`",
        proof: Proof::Static(&[
            Condition {
                case: "not-direct",
                fails_when: "the call is not direct",
            },
            Condition {
                case: "hooked-property",
                fails_when: "`p` has a hook",
            },
        ]),
        folder: Folder::Calls,
        stage: 9,
    },
    Row {
        id: "first-of-filter",
        written: "`Core\\Arr::first(Core\\Arr::filter(a, p))`",
        fast: "`Core\\Arr::find(a, p)`",
        proof: Proof::Callback,
        folder: Folder::Callbacks,
        stage: 4,
    },
    Row {
        id: "first-key-of-filter",
        written: "`Core\\Arr::firstKey(Core\\Arr::filter(a, p))`",
        fast: "`Core\\Arr::findKey(a, p)`",
        proof: Proof::Callback,
        folder: Folder::Callbacks,
        stage: 4,
    },
    Row {
        id: "any-of-filter",
        written: "`Core\\Arr::count(Core\\Arr::filter(a, p))` compared with `0`, and `Core\\Arr::isEmpty` of it",
        fast: "`Core\\Arr::any(a, p)`, negated where the test is for none",
        proof: Proof::Callback,
        folder: Folder::Callbacks,
        stage: 4,
    },
    Row {
        id: "comparator-by",
        written: "`sort`, `diff` or `intersect` with `comparator: fn(x, y) => E(x) <=> E(y)`",
        fast: "the same member with `by: fn(x) => E(x)`",
        proof: Proof::Callback,
        folder: Folder::Callbacks,
        stage: 4,
    },
    Row {
        id: "comparator-by-desc",
        written: "`sort` with `comparator: fn(x, y) => E(y) <=> E(x)` and no `order` option",
        fast: "`by: fn(x) => E(x)` with `order: Core\\Arr::Order::Desc`",
        proof: Proof::Callback,
        folder: Folder::Callbacks,
        stage: 4,
    },
];

/// A call the lowering compiles as another `Core` member of the same class,
/// over arguments the program wrote: the fast form of a stage-2 row.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CoreForm<'a> {
    /// The fast member's name, on the written call's class.
    pub method: &'static str,
    /// The written arguments the fast member is called with.
    pub args: &'a CallArgs,
    /// How many lowered arguments the fast member takes, where it takes fewer
    /// than the written member: the rest are the written member's defaults.
    pub arity: Option<usize>,
    /// The fast member's result type, where it is not the written member's.
    pub returns: Option<Ty>,
}

/// The written arguments of `expr` when it is a static call of `class`'s
/// `method`, resolved by the checker.
fn core_call<'a>(
    expr: &'a Expr,
    exprs: &ExprTypeTable,
    class: &str,
    method: &str,
) -> Option<&'a CallArgs> {
    let ExprKind::StaticCall { args, .. } = &expr.kind else {
        return None;
    };
    let Some(ExprInfo::Call(call)) = exprs.lookup(expr.span) else {
        return None;
    };
    (call.method == method && call.class.to_string() == class).then_some(args)
}

/// The written arguments when there are `n` of them and every one is
/// positional and plain.
fn positional(args: &CallArgs, n: usize) -> Option<&[Arg]> {
    let CallArgs::List(list) = args else {
        return None;
    };
    (list.len() == n
        && list
            .iter()
            .all(|arg| arg.name.is_none() && !arg.spread && !arg.inout))
    .then_some(list.as_slice())
}

/// Rows `count-of-keys-or-values` and `first-of-keys-or-values`: `expr` is
/// `Core\Arr::count` or `Core\Arr::first` of `keys(a)` or `values(a)`, and the
/// fast form is `count(a)`, `firstKey(a)` or `first(a)`.
pub(crate) fn over_keys_or_values<'a>(
    expr: &'a Expr,
    exprs: &ExprTypeTable,
) -> Option<CoreForm<'a>> {
    const ARR: &str = r"Core\Arr";
    let (outer, args) = ["count", "first"]
        .into_iter()
        .find_map(|method| core_call(expr, exprs, ARR, method).map(|args| (method, args)))?;
    let [inner] = positional(args, 1)? else {
        return None;
    };
    let (inner_method, inner_args) = ["keys", "values"].into_iter().find_map(|method| {
        core_call(&inner.value, exprs, ARR, method).map(|args| (method, args))
    })?;
    positional(inner_args, 1)?;
    let method = match (outer, inner_method) {
        ("first", "keys") => "firstKey",
        ("first", _) => "first",
        _ => "count",
    };
    Some(CoreForm {
        method,
        args: inner_args,
        arity: None,
        returns: None,
    })
}

/// Row `filter-contains`, matched on the written tree: `expr` is
/// `Core\Arr::filter(a, fn(x) => Core\Arr::contains(b, x))`, negated or not,
/// and the fast form is `diff(a, b)` or `intersect(a, b)`.
///
/// Both members compare with the same strict identity, keep `a`'s keys and
/// order, and keep each copy of a kept entry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SetForm<'a> {
    /// `diff` where the test is negated, and `intersect` where it is not.
    pub method: &'static str,
    /// The written first argument.
    pub a: &'a Arg,
    /// The variable the callback captured and tests against.
    pub b: &'a Expr,
    /// The callback's one parameter's written type. The site reads what it
    /// resolved to: an element that can be an array keeps the form as written.
    pub param_ty: &'a Type,
}

/// See [`SetForm`]. `src` is the file the spans index.
///
/// The callback is one parameter, plain and typed, with a body that is one `return` or
/// an arrow expression. `b` is a variable other than the parameter and the
/// callback's own name. Such a body assigns nothing, so `b` has the value
/// the callback captured when the filter runs.
pub(crate) fn filter_contains<'a>(
    expr: &'a Expr,
    exprs: &ExprTypeTable,
    src: &SourceFile,
) -> Option<SetForm<'a>> {
    const ARR: &str = r"Core\Arr";
    let text = |span: Span| strip_sigil(span_text(src, span));
    let [a, predicate] = positional(core_call(expr, exprs, ARR, "filter")?, 2)? else {
        return None;
    };
    let ExprKind::Fn(callback) = &predicate.value.kind else {
        return None;
    };
    let [param] = callback.params.as_slice() else {
        return None;
    };
    let param_ty = param.ty.as_ref()?;
    if param.inout || param.variadic || param.default.is_some() {
        return None;
    }
    let returned = match &callback.body {
        FnBody::Expr(returned) => &**returned,
        FnBody::Block(block) => match block.stmts.as_slice() {
            [
                Stmt {
                    kind: StmtKind::Return(Some(returned)),
                    ..
                },
            ] => returned,
            _ => return None,
        },
    };
    let (method, test) = match &returned.kind {
        ExprKind::Unary {
            op: UnaryOp::Not,
            expr: test,
        } => ("diff", &**test),
        _ => ("intersect", returned),
    };
    let [haystack, needle] = positional(core_call(test, exprs, ARR, "contains")?, 2)? else {
        return None;
    };
    let name = text(param.name);
    let (ExprKind::Variable(_), ExprKind::Variable(_)) = (&haystack.value.kind, &needle.value.kind)
    else {
        return None;
    };
    let captured = text(haystack.value.span);
    if text(needle.value.span) != name
        || captured == name
        || captured == "this"
        || callback.name.is_some_and(|own| text(own) == captured)
    {
        return None;
    }
    Some(SetForm {
        method,
        a,
        b: &haystack.value,
        param_ty,
    })
}

/// Rows `caught-conversion` and `caught-parse`, matched on the written `try`:
/// its body is one `$v = …;`, it has one `catch` arm whose body is
/// `$v = null;` and which does not bind `$v`, and it has no `finally`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CaughtNull<'a> {
    /// The variable both arms assign.
    pub target: &'a Expr,
    /// The value the body assigns, as written.
    pub value: &'a Expr,
    /// The one arm's class. The site checks that it is or contains the class
    /// a failed conversion throws.
    pub caught: &'a Type,
    /// What the body converts, and its fast form.
    pub fast: CaughtFast<'a>,
}

/// The conversion a [`CaughtNull`] body runs. Its operand is a variable, so
/// the conversion is the body's only failure.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CaughtFast<'a> {
    /// `X as T`, whose fast form is `X as ?T`. The site checks that the
    /// conversion can fail, so that `as ?T` is a conversion at all.
    Conversion { operand: &'a Expr, ty: &'a Type },
    /// `K::parse(X)` where `K` declares `tryParse`, whose fast form is
    /// `K::tryParse(X)`.
    Parse { call: &'a Expr, form: CoreForm<'a> },
}

/// The `$v` a statement `$v = value;` assigns, and the value.
fn assigned(stmt: &Stmt) -> Option<(&Expr, &Expr)> {
    let StmtKind::Expr(expr) = &stmt.kind else {
        return None;
    };
    let ExprKind::Assign {
        op: AssignOp::Assign,
        target,
        value,
        by_ref: false,
    } = &expr.kind
    else {
        return None;
    };
    matches!(target.kind, ExprKind::Variable(_)).then_some((&**target, &**value))
}

/// See [`CaughtNull`]. `src` is the file the spans index.
pub(crate) fn caught_null<'a>(
    body: &'a Block,
    catches: &'a [CatchClause],
    finally: Option<&Block>,
    exprs: &ExprTypeTable,
    src: &SourceFile,
) -> Option<CaughtNull<'a>> {
    let text = |expr: &Expr| strip_sigil(span_text(src, expr.span));
    let ([stmt], [arm], None) = (body.stmts.as_slice(), catches, finally) else {
        return None;
    };
    let (target, value) = assigned(stmt)?;
    let [reset] = arm.body.stmts.as_slice() else {
        return None;
    };
    let (reset_target, null) = assigned(reset)?;
    let name = text(target);
    if !matches!(null.kind, ExprKind::Null) || text(reset_target) != name {
        return None;
    }
    if arm
        .var
        .is_some_and(|var| strip_sigil(span_text(src, var)) == name)
    {
        return None;
    }
    let fast = match &value.kind {
        ExprKind::Conversion {
            expr: operand, ty, ..
        } if matches!(operand.kind, ExprKind::Variable(_)) => {
            CaughtFast::Conversion { operand, ty }
        }
        ExprKind::StaticCall { args, .. } => {
            let Some(ExprInfo::Call(call)) = exprs.lookup(value.span) else {
                return None;
            };
            let [operand] = positional(args, 1)? else {
                return None;
            };
            if call.method != "parse"
                || !matches!(operand.value.kind, ExprKind::Variable(_))
                || nvs_types::core_symbol_of(&call.class, "parse").is_none()
                || nvs_types::core_symbol_of(&call.class, "tryParse").is_none()
            {
                return None;
            }
            CaughtFast::Parse {
                call: value,
                form: CoreForm {
                    method: "tryParse",
                    args,
                    arity: None,
                    returns: Some(Ty::Tagged),
                },
            }
        }
        _ => return None,
    };
    Some(CaughtNull {
        target,
        value,
        caught: &arm.ty,
        fast,
    })
}

/// Row `match-null-test`: `operand` is `Core\Regex::match(s, p)` with no
/// `from` option, and its only use is the null test around it. The fast form
/// is `Core\Regex::matches(s, p)`, whose `true` is the test's `!= null`.
pub(crate) fn null_tested_match<'a>(
    operand: &'a Expr,
    exprs: &ExprTypeTable,
) -> Option<CoreForm<'a>> {
    let args = core_call(operand, exprs, r"Core\Regex", "match")?;
    positional(args, 2)?;
    Some(CoreForm {
        method: "matches",
        args,
        arity: Some(2),
        returns: Some(Ty::Bool),
    })
}

/// Which walk a `foreach` over a direct `keys` or `values` call is compiled as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Walk {
    /// `Core\Arr::keys(m)`: the loop over `m` binds each key as its value.
    Keys,
    /// `Core\Arr::values(m)`: the loop over `m` binds each value.
    Values,
}

/// Row `foreach-keys-or-values`: `subject` is a direct `Core\Arr::keys(m)` or
/// `Core\Arr::values(m)` call, and the fast form is the loop over `m` itself.
///
/// A loop walks the array it began with, so the body's writes to `m` reach
/// neither walk. The site applies it only to a by-value loop with no key
/// binding: a key over either list is its position, which `m` does not have.
pub(crate) fn foreach_keys_or_values<'a>(
    subject: &'a Expr,
    exprs: &ExprTypeTable,
) -> Option<(&'a Expr, Walk)> {
    const ARR: &str = r"Core\Arr";
    let (walk, args) = [("keys", Walk::Keys), ("values", Walk::Values)]
        .into_iter()
        .find_map(|(method, walk)| core_call(subject, exprs, ARR, method).map(|a| (walk, a)))?;
    let [arg] = positional(args, 1)? else {
        return None;
    };
    Some((&arg.value, walk))
}

/// Row `foreach-range`: the written arguments of a direct
/// `Core\Arr::range(start, end)` call, or one with `{step: X}` as its third
/// argument, positional or named `options`. Anything wider stays as written.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RangeForm<'a> {
    /// The first value.
    pub start: &'a Expr,
    /// The bound the values do not pass.
    pub end: &'a Expr,
    /// The written step, or `None` for `range`'s step of 1.
    pub step: Option<&'a Expr>,
}

/// [`RangeForm`]'s matcher, over a `foreach` subject.
pub(crate) fn foreach_range<'a>(
    subject: &'a Expr,
    exprs: &ExprTypeTable,
    src: &SourceFile,
) -> Option<RangeForm<'a>> {
    let CallArgs::List(list) = core_call(subject, exprs, r"Core\Arr", "range")? else {
        return None;
    };
    let plain = |arg: &Arg| !arg.spread && !arg.inout;
    let (start, end, options) = match list.as_slice() {
        [start, end] => (start, end, None),
        [start, end, options] => (start, end, Some(options)),
        _ => return None,
    };
    if !(plain(start) && plain(end) && start.name.is_none() && end.name.is_none()) {
        return None;
    }
    let step = match options {
        None => None,
        Some(options) => {
            if !plain(options)
                || options
                    .name
                    .is_some_and(|name| span_text(src, name) != "options")
            {
                return None;
            }
            let ExprKind::AnonObject(fields) = &options.value.kind else {
                return None;
            };
            match fields.as_slice() {
                [] => None,
                [field] if span_text(src, field.name) == "step" => Some(&field.value),
                _ => return None,
            }
        }
    };
    Some(RangeForm {
        start: &start.value,
        end: &end.value,
        step,
    })
}

/// Row `constant-key-literal`: the key each element of `items` is stored
/// at, where every element has a written key that is a string or integer
/// literal, no two of those keys are equal, and none is a spread.
///
/// An integer key is the decimal string the array stores. A write at a
/// decimal string key moves the append counter as a write at the integer
/// does, so one `ArrayNew` builds the same array as the chain of writes.
pub(crate) fn constant_keys(
    items: &[ArrayItem],
    src: &SourceFile,
    exprs: &ExprTypeTable,
) -> Option<Vec<String>> {
    let mut keys = Vec::with_capacity(items.len());
    let mut seen = rustc_hash::FxHashSet::default();
    for item in items {
        if item.spread || item.by_ref {
            return None;
        }
        let key = match &item.key.as_ref()?.kind {
            ExprKind::Str(span) if exprs.written_path(*span).is_none() => {
                crate::lower::cook_str_literal(src, *span)
            }
            ExprKind::Int(span) => {
                let (radix, digits) = crate::lower::int_literal_digits(src, *span);
                u64::from_str_radix(&digits, radix).ok()?.to_string()
            }
            _ => return None,
        };
        if !seen.insert(key.clone()) {
            return None;
        }
        keys.push(key);
    }
    Some(keys)
}

/// Row `unread-foreach-key`: whether `text`, a loop body's source, may read
/// the variable `name` (written without its `$`).
///
/// A variable is read only where its `$name` is written: an arrow function's
/// capture, a closure's `use` and an interpolation each spell it. So a body
/// whose text never has `$name` followed by a character that cannot continue
/// a name never reads it. A comment or a string that has `$name` keeps the
/// key, which is the safe direction.
pub(crate) fn text_names_variable(text: &str, name: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find('$') {
        let start = from + at + 1;
        from = start;
        if text[start..].starts_with(name) {
            let next = bytes.get(start + name.len()).copied();
            if !next.is_some_and(|b| b == b'_' || b.is_ascii_alphanumeric() || b >= 0x80) {
                return true;
            }
        }
    }
    false
}

/// Rows `first-of-filter`, `first-key-of-filter` and `any-of-filter`, matched
/// on the written tree: `first`, `firstKey` or `isEmpty` of
/// `Core\Arr::filter(a, p)`. The lowering emits the form as written and records
/// a [`FormCandidate`], and [`run`] decides from the callback.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FilterForm<'a> {
    /// The row's id in [`ROWS`].
    pub row: &'static str,
    /// The fast member's name on `Core\Arr`.
    pub fast: &'static str,
    /// Whether the fast form is the fast member negated: `isEmpty` of a
    /// filter is `!any`.
    pub negated: bool,
    /// The filter's written subject.
    pub a: &'a Arg,
    /// The filter's written callback.
    pub predicate: &'a Arg,
}

impl FilterForm<'_> {
    /// The fast form as the program would write it, negated where `negated`.
    pub(crate) fn text(&self, src: &SourceFile, negated: bool) -> String {
        format!(
            r"{}Core\Arr::{}({}, {})",
            if negated { "!" } else { "" },
            self.fast,
            span_text(src, self.a.span),
            span_text(src, self.predicate.span)
        )
    }
}

/// See [`FilterForm`]: `expr` is `first`, `firstKey` or `isEmpty` of a direct
/// `Core\Arr::filter` call with two positional arguments.
pub(crate) fn of_filter<'a>(expr: &'a Expr, exprs: &ExprTypeTable) -> Option<FilterForm<'a>> {
    const ARR: &str = r"Core\Arr";
    let (outer, args) = ["first", "firstKey", "isEmpty"]
        .into_iter()
        .find_map(|method| core_call(expr, exprs, ARR, method).map(|args| (method, args)))?;
    let [inner] = positional(args, 1)? else {
        return None;
    };
    let [a, predicate] = positional(core_call(&inner.value, exprs, ARR, "filter")?, 2)? else {
        return None;
    };
    let (row, fast, negated) = match outer {
        "first" => ("first-of-filter", "find", false),
        "firstKey" => ("first-key-of-filter", "findKey", false),
        _ => ("any-of-filter", "any", true),
    };
    Some(FilterForm {
        row,
        fast,
        negated,
        a,
        predicate,
    })
}

/// Row `any-of-filter` for a count: `lhs op rhs` is
/// `Core\Arr::count(Core\Arr::filter(a, p))` compared with the literal `0` by
/// `==`, `!=` or `>`. Gives the count call, whether the test is for none, and
/// the row's form.
///
/// A count is a `uint`, so `> 0` and `!= 0` test that the filter kept an
/// entry, and `== 0` that it kept none: `isEmpty` of the same filter.
pub(crate) fn count_of_filter_compared<'a>(
    op: BinaryOp,
    lhs: &'a Expr,
    rhs: &Expr,
    exprs: &ExprTypeTable,
    src: &SourceFile,
) -> Option<(&'a Expr, bool, FilterForm<'a>)> {
    let none = match op {
        BinaryOp::Eq => true,
        BinaryOp::NotEq | BinaryOp::Gt => false,
        _ => return None,
    };
    let ExprKind::Int(digits) = rhs.kind else {
        return None;
    };
    if span_text(src, digits) != "0" {
        return None;
    }
    let [inner] = positional(core_call(lhs, exprs, r"Core\Arr", "count")?, 1)? else {
        return None;
    };
    let [a, predicate] = positional(core_call(&inner.value, exprs, r"Core\Arr", "filter")?, 2)?
    else {
        return None;
    };
    let form = FilterForm {
        row: "any-of-filter",
        fast: "any",
        negated: true,
        a,
        predicate,
    };
    Some((lhs, none, form))
}

/// Rows `comparator-by` and `comparator-by-desc`, matched on the written tree:
/// `Core\Arr::sort`, `diff` or `intersect` with no `by` option and a
/// `comparator` option that is `fn(T $x, T $y) => E($x) <=> E($y)`. The
/// descending row is `E($y) <=> E($x)` on `sort` with no `order` option.
///
/// `E($y)` must be `E($x)` with the variable renamed, read from the source
/// text, and must not read `$y`. `E` is narrow, so it can be lowered a second
/// time as the `by` function's body: a resolved static or `Core` call whose
/// arguments are each a variable or a literal, or a conversion of a variable.
/// Its result is `int`, `uint` or `string`: for those, `<=>` gives `0` exactly
/// where the set members' own comparison finds two values the same, and orders
/// as the sort's own comparison does. A `float` is not taken, because `NaN`
/// and `-0.0` compare differently in the two.
pub(crate) struct ComparatorForm<'a> {
    /// The row's id in [`ROWS`].
    pub row: &'static str,
    /// The comparator's anonymous function, as written.
    pub comparator: &'a Expr,
    /// The `by` function: `fn(T $x) => E($x)`.
    pub by: FnExpr,
    /// What `E` gives.
    pub ret: Ty,
    /// The call's argument index of the `by` option, and of `comparator`.
    pub slots: (usize, usize),
    /// The argument index of `order` for the descending row.
    pub desc: Option<usize>,
    /// The fast form as the program would write it.
    pub text: String,
}

/// See [`ComparatorForm`].
pub(crate) fn comparator_by<'a>(
    expr: &'a Expr,
    exprs: &ExprTypeTable,
    src: &SourceFile,
    checked_types: &TypeInterner,
) -> Option<ComparatorForm<'a>> {
    const ARR: &str = r"Core\Arr";
    // The flattened argument indices: `sort(a, {by, order, comparator,
    // preserveKeys})` and `diff(a, b, {on, by, comparator})`.
    let (method, args) = ["sort", "diff", "intersect"]
        .into_iter()
        .find_map(|method| core_call(expr, exprs, ARR, method).map(|args| (method, args)))?;
    let CallArgs::List(list) = args else {
        return None;
    };
    let (positional, slots, order) = match method {
        "sort" => (1, (1, 3), Some(2)),
        _ => (2, (3, 4), None),
    };
    if list.len() != positional + 1
        || list
            .iter()
            .any(|arg| arg.name.is_some() || arg.spread || arg.inout)
    {
        return None;
    }
    let ExprKind::AnonObject(fields) = &list[positional].value.kind else {
        return None;
    };
    let field = |name: &str| fields.iter().find(|f| span_text(src, f.name) == name);
    if field("by").is_some() {
        return None;
    }
    let comparator = &field("comparator")?.value;
    let ExprKind::Fn(written) = &comparator.kind else {
        return None;
    };
    let [x, y] = written.params.as_slice() else {
        return None;
    };
    let plain = |p: &nvs_syntax::ast::Param| {
        !p.inout && !p.variadic && !p.covariant && p.default.is_none() && p.ty.is_some()
    };
    if written.name.is_some()
        || !plain(x)
        || !plain(y)
        || span_text(src, x.ty.as_ref()?.span) != span_text(src, y.ty.as_ref()?.span)
    {
        return None;
    }
    let FnBody::Expr(body) = &written.body else {
        return None;
    };
    let ExprKind::Binary {
        op: BinaryOp::Cmp,
        lhs,
        rhs,
    } = &body.kind
    else {
        return None;
    };
    let (xn, yn) = (
        strip_sigil(span_text(src, x.name)),
        strip_sigil(span_text(src, y.name)),
    );
    let (lt, rt) = (span_text(src, lhs.span), span_text(src, rhs.span));
    let (e, descending) =
        if text_names_variable(lt, xn) && !text_names_variable(lt, yn) && renamed(lt, xn, yn) == rt
        {
            (lhs.as_ref(), false)
        } else if order.is_some()
            && field("order").is_none()
            && text_names_variable(rt, xn)
            && !text_names_variable(rt, yn)
            && renamed(rt, xn, yn) == lt
        {
            (rhs.as_ref(), true)
        } else {
            return None;
        };
    let ret = by_expression_ty(e, exprs, checked_types)?;
    let by = FnExpr {
        is_static: written.is_static,
        name: None,
        params: vec![x.clone()],
        return_type: None,
        body: FnBody::Expr(Box::new(e.clone())),
    };
    let by_text = format!(
        "fn({} {}) => {}",
        span_text(src, x.ty.as_ref()?.span),
        span_text(src, x.name),
        span_text(src, e.span)
    );
    let written_args = list[..positional]
        .iter()
        .map(|arg| span_text(src, arg.span))
        .collect::<Vec<_>>()
        .join(", ");
    let (row, text) = if descending {
        (
            "comparator-by-desc",
            format!(
                r"Core\Arr::{method}({written_args}, {{by: {by_text}, order: Core\Arr::Order::Desc}})"
            ),
        )
    } else {
        (
            "comparator-by",
            format!(r"Core\Arr::{method}({written_args}, {{by: {by_text}}})"),
        )
    };
    Some(ComparatorForm {
        row,
        comparator,
        by,
        ret,
        slots,
        desc: if descending { order } else { None },
        text,
    })
}

/// `text` with every `$from` that is a whole variable written `$to`.
fn renamed(text: &str, from: &str, to: &str) -> String {
    let needle = format!("${from}");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(&needle) {
        let after = &rest[at + needle.len()..];
        let whole = !after
            .chars()
            .next()
            .is_some_and(|c| c == '_' || c.is_alphanumeric());
        out.push_str(&rest[..at]);
        out.push_str(if whole { "$" } else { "" });
        out.push_str(if whole { to } else { &needle });
        rest = after;
    }
    out.push_str(rest);
    out
}

/// What `E` of a comparator row gives, where it is one of the narrow shapes
/// [`ComparatorForm`] takes and gives `int`, `uint` or `string`.
fn by_expression_ty(e: &Expr, exprs: &ExprTypeTable, checked_types: &TypeInterner) -> Option<Ty> {
    let plain = |arg: &Expr| {
        matches!(
            arg.kind,
            ExprKind::Variable(_)
                | ExprKind::Int(_)
                | ExprKind::Str(_)
                | ExprKind::Bool(_)
                | ExprKind::Null
        )
    };
    let id = match &e.kind {
        ExprKind::Paren(inner) => return by_expression_ty(inner, exprs, checked_types),
        ExprKind::StaticCall {
            args: CallArgs::List(list),
            ..
        } => {
            let Some(ExprInfo::Call(call)) = exprs.lookup(e.span) else {
                return None;
            };
            if list
                .iter()
                .any(|arg| arg.name.is_some() || arg.spread || arg.inout || !plain(&arg.value))
            {
                return None;
            }
            call.return_ty
        }
        ExprKind::Conversion {
            expr,
            ty,
            legacy: false,
        } if matches!(expr.kind, ExprKind::Variable(_)) => exprs.declared_ty(ty.span)?,
        _ => return None,
    };
    let ty = crate::lower::erase_checked_ty(id, checked_types);
    matches!(ty, Ty::Int | Ty::Uint | Ty::Str).then_some(ty)
}

/// A callback form whose callback writes nothing but can fail. It is compiled
/// as written, and is kept so `nvs check` can name its fast form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    /// The written form's span.
    pub span: Span,
    /// The row's id in [`ROWS`].
    pub row: &'static str,
    /// The fast form as the program would write it.
    pub fast: String,
}

/// The pass that decides every recorded callback form. It runs after the
/// lowering and before `crate::ownership`, and each [`FormCandidate`] takes
/// one of the three outcomes from its callback ([`judge`]):
///
/// - [`Outcome::Fast`]: the filter's instruction becomes the fast member over
///   the filter's own operands, at the filter's place, where the callback is
///   still alive. The outer member's instruction becomes a relabelling of that
///   result (`find`, `findKey`) or its negation (`!any`), and the filter's
///   releases are removed.
/// - [`Outcome::Warning`]: the form stays, and its row is returned.
/// - [`Outcome::AsWritten`]: the form stays, and nothing is returned.
///
/// **What it spends:** while compiling, one effect summary for each function
/// when some function recorded a candidate, and nothing otherwise.
pub(crate) fn run(functions: &mut [Function]) -> Vec<Recorded> {
    if functions.iter().all(|f| f.form_candidates.is_empty()) {
        return Vec::new();
    }
    let summaries = Summaries::of(functions);
    let by_name: FxHashMap<&str, &Function> =
        functions.iter().map(|f| (f.name.as_str(), f)).collect();
    let mut recorded = Vec::new();
    let mut fast: Vec<(usize, Rewrite)> = Vec::new();
    let mut retargets: Vec<(usize, Retarget)> = Vec::new();
    for (at, f) in functions.iter().enumerate() {
        for candidate in &f.form_candidates {
            if let Some(twin) = &candidate.by {
                let Some(retarget) = Retarget::of(f, candidate.result, twin) else {
                    continue;
                };
                let by = format!("{}::invoke", twin.class);
                match judge(&by, &by_name, &summaries) {
                    Outcome::Fast => retargets.push((at, retarget)),
                    Outcome::Warning => recorded.push(Recorded {
                        span: candidate.span,
                        row: candidate.row,
                        fast: candidate.text.clone(),
                    }),
                    Outcome::AsWritten => {}
                }
                continue;
            }
            let Some(rewrite) = Rewrite::of(f, candidate) else {
                continue;
            };
            match judge(&rewrite.callback, &by_name, &summaries) {
                Outcome::Fast => fast.push((at, rewrite)),
                Outcome::Warning => recorded.push(Recorded {
                    span: candidate.span,
                    row: candidate.row,
                    fast: candidate.text.clone(),
                }),
                Outcome::AsWritten => {}
            }
        }
    }
    for (at, rewrite) in fast {
        rewrite.apply(&mut functions[at]);
    }
    for (at, retarget) in retargets {
        retarget.apply(&mut functions[at]);
    }
    for f in functions.iter_mut() {
        f.form_candidates.clear();
    }
    recorded
}

/// The `W1034` warning of each recorded form: the written form is its span,
/// its message names the member to use, and its one fix is the fast form's
/// text, marked an alternative so no batch applies it
/// (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`). It is raised
/// from [`run`]'s own decision and from nothing else, so `nvs check` and the
/// editor cannot disagree. A span recorded twice, as a body lowered twice
/// records it, warns once.
///
/// **What it spends:** one diagnostic per recorded form, while checking.
#[must_use]
pub fn warnings(recorded: &[Recorded]) -> Vec<Diagnostic> {
    let mut seen = FxHashSet::default();
    recorded
        .iter()
        .filter(|form| seen.insert(form.span))
        .map(|form| {
            let (message, label, fix) = if form.row.starts_with("comparator-by") {
                (
                    "this comparator compares one value of each side, and `by` gives the same \
                     order faster"
                        .to_owned(),
                    "`by` calls its function once for each entry",
                    "pass the value as `by`",
                )
            } else {
                let member = fast_member(&form.fast);
                (
                    format!("`{member}` gives the same result faster"),
                    "this calls the function for every entry",
                    "use the faster form",
                )
            };
            Diagnostic::warning(code::W_CHEAPER_FORM, message)
                .with_primary(form.span, label)
                .with_help(
                    "the function can throw an error, and the faster form calls it fewer times, \
                     so the compiler keeps this form",
                )
                .with_alternative_fix(form.span, form.fast.clone(), fix)
        })
        .collect()
}

/// The member a fast form's text calls: `Core\Arr::any` of
/// `!Core\Arr::any($a, $p)`.
fn fast_member(fast: &str) -> &str {
    let call = fast.trim_start_matches('!');
    call.find('(').map_or(call, |open| &call[..open])
}

/// Where one instruction is: its block's index and its own.
type At = (usize, usize);

/// One candidate's instructions, found in its function: what [`run`] rewrites
/// where the callback is proven.
struct Rewrite {
    /// The filter call's place.
    filter: At,
    /// The outer member's place.
    outer: At,
    /// The filter's result, which the fast member's result takes over.
    filtered: ValueId,
    /// The filter's subject and callback.
    operands: Vec<ValueId>,
    /// The fast member's symbol and its result type.
    fast: (&'static str, Ty),
    /// Whether the outer result is the fast result negated.
    negated: bool,
    /// The callback's body: `Class::invoke`.
    callback: String,
}

impl Rewrite {
    /// The candidate's instructions, or `None` where the IR is not the shape
    /// the lowering records: the outer member over a filter call whose result
    /// nothing else reads, with a callback made in this function.
    fn of(f: &Function, candidate: &FormCandidate) -> Option<Self> {
        let mut defs: FxHashMap<ValueId, At> = FxHashMap::default();
        for (b, block) in f.blocks.iter().enumerate() {
            for (i, inst) in block.insts.iter().enumerate() {
                if let Some(v) = inst.result {
                    defs.insert(v, (b, i));
                }
            }
        }
        let inst = |at: At| &f.blocks[at.0].insts[at.1];
        let outer = *defs.get(&candidate.result)?;
        let InstKind::CoreCall {
            args,
            subject: Some(subject),
            ..
        } = &inst(outer).kind
        else {
            return None;
        };
        let filtered = *args.get(*subject as usize)?;
        let filter = *defs.get(&filtered)?;
        let InstKind::CoreCall { symbol, args, .. } = &inst(filter).kind else {
            return None;
        };
        if *symbol != "nvs_core_arr_filter" || args.len() != 2 {
            return None;
        }
        let operands = args.clone();
        let callback = crate::effects::callback(&inst(*defs.get(&operands[1])?).kind)?;
        // Nothing but the outer member and the filter's releases reads it.
        let mut others = 0usize;
        for (b, block) in f.blocks.iter().enumerate() {
            for (i, inst) in block.insts.iter().enumerate() {
                let released =
                    matches!(inst.kind, InstKind::Release { operand } if operand == filtered);
                if (b, i) != outer && !released {
                    inst.kind
                        .for_each_operand(|v| others += usize::from(v == filtered));
                }
            }
            block
                .term
                .for_each_operand(|v| others += usize::from(v == filtered));
        }
        if others != 0 {
            return None;
        }
        let ty = if candidate.negated {
            Ty::Bool
        } else {
            inst(outer).ty?
        };
        Some(Self {
            filter,
            outer,
            filtered,
            operands,
            fast: (candidate.fast, ty),
            negated: candidate.negated,
            callback,
        })
    }

    /// Compiles the fast form in place of the written one.
    fn apply(self, f: &mut Function) {
        let filter = &mut f.blocks[self.filter.0].insts[self.filter.1];
        filter.kind = InstKind::CoreCall {
            symbol: self.fast.0,
            args: self.operands,
            subject: Some(0),
        };
        filter.ty = Some(self.fast.1);
        let outer = &mut f.blocks[self.outer.0].insts[self.outer.1];
        outer.kind = if self.negated {
            InstKind::UnOp {
                op: crate::ir::UnOp::Not,
                operand: self.filtered,
            }
        } else {
            InstKind::Reinterpret {
                operand: self.filtered,
            }
        };
        outer.on_error = None;
        let filtered = self.filtered;
        for block in &mut f.blocks {
            block.insts.retain(
                |inst| !matches!(inst.kind, InstKind::Release { operand } if operand == filtered),
            );
        }
    }
}

/// A comparator candidate's instructions, found in its function: what [`run`]
/// changes where the `by` function is proven.
///
/// The comparator's object becomes an object of the `by` function's class,
/// with its arity and parameter tags, and moves to the `by` option. The `by`
/// option's `null` moves to `comparator`. The descending row also turns the
/// default `order` into `Desc`. The call makes the same one object, so the
/// fast form allocates nothing more.
struct Retarget {
    /// The call's place.
    call: At,
    /// The `order` default's place, for the descending row.
    order: Option<At>,
    /// The `by` function, and where the comparator was made.
    twin: crate::ir::ByTwin,
}

impl Retarget {
    /// The candidate's instructions, or `None` where the IR is not the shape
    /// the lowering records: a `Core` call with both option slots, and for the
    /// descending row an `order` that is its own constant `Asc`.
    fn of(f: &Function, result: ValueId, twin: &crate::ir::ByTwin) -> Option<Self> {
        let mut defs: FxHashMap<ValueId, At> = FxHashMap::default();
        let mut uses: FxHashMap<ValueId, usize> = FxHashMap::default();
        for (b, block) in f.blocks.iter().enumerate() {
            for (i, inst) in block.insts.iter().enumerate() {
                if let Some(v) = inst.result {
                    defs.insert(v, (b, i));
                }
                inst.kind
                    .for_each_operand(|v| *uses.entry(v).or_default() += 1);
            }
            block
                .term
                .for_each_operand(|v| *uses.entry(v).or_default() += 1);
        }
        let inst = |at: At| &f.blocks[at.0].insts[at.1];
        let call = *defs.get(&result)?;
        let InstKind::CoreCall { args, .. } = &inst(call).kind else {
            return None;
        };
        let (by, comparator) = twin.slots;
        if args.len() <= by.max(comparator) {
            return None;
        }
        crate::effects::callback(&inst(*defs.get(&twin.object)?).kind)?;
        let order = match twin.desc {
            None => None,
            Some(slot) => {
                let value = *args.get(slot)?;
                let at = *defs.get(&value)?;
                if !matches!(inst(at).kind, InstKind::ConstInt(0)) || uses.get(&value) != Some(&1) {
                    return None;
                }
                Some(at)
            }
        };
        Some(Self {
            call,
            order,
            twin: twin.clone(),
        })
    }

    /// Compiles the fast form in place of the written one.
    fn apply(self, f: &mut Function) {
        let twin = self.twin;
        if let InstKind::CoreCall { args, .. } = &mut f.blocks[self.call.0].insts[self.call.1].kind
        {
            args.swap(twin.slots.0, twin.slots.1);
        }
        if let Some((b, i)) = self.order {
            f.blocks[b].insts[i].kind = InstKind::ConstInt(1);
        }
        for block in &mut f.blocks {
            for inst in &mut block.insts {
                let result = inst.result;
                match &mut inst.kind {
                    InstKind::New { class, .. } if result == Some(twin.object) => {
                        class.clone_from(&twin.class);
                    }
                    InstKind::FieldSet { object, class, .. } if *object == twin.object => {
                        class.clone_from(&twin.class);
                    }
                    InstKind::ConstInt(n) if result == Some(twin.arity) => *n = 1,
                    InstKind::ConstInt(n) if result == Some(twin.tags) => *n = twin.tags_word,
                    _ => {}
                }
            }
        }
    }
}

/// The outcome a callback gives its form. [`Outcome::Fast`] where its body,
/// and every body it calls, writes nothing, prints nothing and cannot fail.
/// [`Outcome::Warning`] where only a possible failure stands in the way.
/// [`Outcome::AsWritten`] where it has an effect or reaches code the compiler
/// cannot name. A `FATAL` is not a failure: no code observes it.
///
/// A safepoint's error edge is not a failure either. The entry poll's stack
/// check throws on the first call or on none, because the member calls the
/// callback at one depth for every entry, and the fast member makes that first
/// call too. A poll that ends a request at its time limit ends it sooner or
/// not at all in the fast form, as the memory ceiling does.
fn judge(callback: &str, bodies: &FxHashMap<&str, &Function>, summaries: &Summaries) -> Outcome {
    let Some(summary) = summaries.get(callback) else {
        return Outcome::AsWritten;
    };
    if summary.reads == Touches::Anything || summary.writes != Touches::default() {
        return Outcome::AsWritten;
    }
    let mut can_fail = false;
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut stack = vec![callback.to_owned()];
    while let Some(name) = stack.pop() {
        let Some(body) = bodies.get(name.as_str()) else {
            return Outcome::AsWritten;
        };
        if !seen.insert(name) {
            continue;
        }
        let tys = crate::effects::types(body);
        for block in &body.blocks {
            can_fail |= matches!(block.term, Terminator::Throw { .. });
            for inst in &block.insts {
                can_fail |= inst.on_error.is_some()
                    && !matches!(inst.kind, InstKind::Safepoint)
                    && !matches!(inst.kind, InstKind::CoreCall { symbol, .. } if throws_nothing(symbol));
                match quiet(&inst.kind, &tys) {
                    Quiet::Yes => {}
                    Quiet::Calls(callee) => stack.push(callee.to_owned()),
                    Quiet::No => return Outcome::AsWritten,
                }
            }
        }
    }
    if can_fail {
        Outcome::Warning
    } else {
        Outcome::Fast
    }
}

/// Whether the `Core` member at `symbol` throws no error a program can catch:
/// a member of a class on [`QUIET_CORE`] whose reference card lists no
/// errors. Every member carries a card (`rule:core-api/reference-card`), and
/// its `errors` list is every error the member throws, so an empty one is a
/// member that throws none. Its call keeps an error edge for the memory
/// ceiling and a stop, which are no outcome a program observes.
fn throws_nothing(symbol: &str) -> bool {
    let own = symbol.strip_suffix("_owned").unwrap_or(symbol);
    QUIET_CORE.iter().any(|prefix| own.starts_with(prefix))
        && nvs_stdlib::registry::CLASSES
            .iter()
            .flat_map(|class| class.members())
            .find(|method| method.symbol == own)
            .and_then(|method| method.doc)
            .is_some_and(|doc| doc.errors.is_empty())
}

/// Whether one instruction of a callback has an effect a program can observe.
enum Quiet<'a> {
    /// It has none.
    Yes,
    /// It has none of its own, and runs this body.
    Calls(&'a str),
    /// It may have one: it prints, changes something outside the call, or
    /// runs code the compiler cannot name.
    No,
}

/// The `Core` classes whose members change nothing and read no state, so a
/// call that passes them no callable is quiet. A member that draws a random
/// value is not: calling it fewer times changes what later draws give.
const QUIET_CORE: &[&str] = &[
    "nvs_core_str_",
    "nvs_core_math_",
    "nvs_core_arr_",
    "nvs_core_regex_",
];

/// See [`Quiet`]. A slot an instruction writes is in the summary [`judge`]
/// reads first. This answers for what a summary does not say: printing, the
/// world outside the program, and a call it cannot follow.
fn quiet<'a>(kind: &'a InstKind, tys: &FxHashMap<ValueId, Ty>) -> Quiet<'a> {
    match kind {
        InstKind::Call { target, .. } => Quiet::Calls(target),
        InstKind::New {
            ctor: Some(ctor), ..
        } => Quiet::Calls(ctor),
        InstKind::CoreCall { symbol, .. } => {
            let quiet_class = QUIET_CORE.iter().any(|prefix| symbol.starts_with(prefix));
            let draws = symbol.contains("random") || symbol.contains("shuffle");
            if quiet_class && !draws && !crate::effects::takes_an_object(kind, tys) {
                Quiet::Yes
            } else {
                Quiet::No
            }
        }
        InstKind::HelperCall { helper, .. } => match helper {
            Helper::IntToString
            | Helper::UintToString
            | Helper::FloatToString
            | Helper::BoolToString
            | Helper::DecimalToString
            | Helper::BytesToString
            | Helper::IntTruthy
            | Helper::UintTruthy
            | Helper::FloatTruthy
            | Helper::StrTruthy
            | Helper::BytesTruthy
            | Helper::ArrayTruthy
            | Helper::DecimalTruthy
            | Helper::IntToUint
            | Helper::UintToInt
            | Helper::IntToFloat
            | Helper::UintToFloat
            | Helper::FloatToInt
            | Helper::FloatToUint
            | Helper::StrToInt
            | Helper::StrToUint
            | Helper::StrToFloat
            | Helper::DecimalAdd
            | Helper::DecimalSub
            | Helper::DecimalMul
            | Helper::DecimalDiv
            | Helper::DecimalMod
            | Helper::DecimalNeg
            | Helper::DecimalEq
            | Helper::DecimalLt
            | Helper::DecimalLtEq
            | Helper::DecimalCmp
            | Helper::Identical
            | Helper::NumericEq
            | Helper::NumericLt
            | Helper::NumericLtEq
            | Helper::NumericCmp => Quiet::Yes,
            _ => Quiet::No,
        },
        _ => {
            let effects = kind.effects();
            if matches!(effects.user_code, UserCode::No) && matches!(effects.writes, Touch::Nothing)
            {
                Quiet::Yes
            } else {
                Quiet::No
            }
        }
    }
}
