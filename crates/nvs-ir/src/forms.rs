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

use nvs_diagnostics::{SourceFile, Span};
use nvs_syntax::ast::{
    Arg, AssignOp, Block, CallArgs, CatchClause, Expr, ExprKind, FnBody, Stmt, StmtKind, Type,
    UnaryOp,
};
use nvs_types::expr_table::{ExprInfo, ExprTypeTable};

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
