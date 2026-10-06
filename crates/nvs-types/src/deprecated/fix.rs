//! `rule:attributes/a-deprecation-names-its-replacement-as-code`'s fix: the
//! template a deprecated declaration names, filled in at one use.
//!
//! **What is read.** The use's own text is parsed again where it was written
//! ([`nvs_diagnostics::SourceFile::code_at`]), so its receiver and arguments
//! are nodes with their real spans, and the template is parsed on its own.
//! Each placeholder of the template — a parameter, `$this`, `self`, a class
//! name — is a hole, written over with the text the use supplies, in
//! parentheses only where that text binds more loosely than the hole's place.
//! A class name is resolved where the template was written, and written at
//! the use as the shortest name that resolves to it there, with a `use` line
//! where the short name needs one.
//!
//! **Side effects.** An argument or the receiver is impure when it holds a
//! call, `new`, an assignment, an increment or a decrement, and any expression
//! this module does not know counts as impure. An impure one stays where it is
//! only when the template uses it once and in the order the use wrote it;
//! every other one moves to a `var` line in front of the statement. Where that
//! line would run at a different time than the use — the use is inside an
//! arrow function, after `&&`, `||` or `??`, in a `match` arm, a loop's
//! condition or a later `if` arm's condition — or where something impure in
//! the statement runs before the use, there is no fix.
//!
//! A template this module cannot fill in completely has no fix either, and
//! the warning still names the template. What it spends is one parse of the
//! use and one of the template per warning, and the file's
//! [`SyntaxIndex`], built once per body that warns and only when a fix
//! needs the statement around a use.

use std::fmt::Write as _;

use nvs_diagnostics::{Diagnostics, SourceFile, Span};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Arg, AssignOp, BinaryOp, CallArgs, Expr, ExprKind, MemberName, NamespaceDecl, NewTarget, Stmt,
    StmtKind, UnaryOp,
};
use nvs_syntax::{IndexNode, SyntaxIndex};
use rustc_hash::FxHashMap;

use super::{Deprecation, Member};
use crate::{Env, span_text, strip_sigil};

/// The namespace and the imports a name is resolved in.
#[derive(Clone, Default)]
pub(super) struct Scope {
    namespace: Vec<String>,
    imports: FxHashMap<String, QName>,
}

impl Scope {
    pub(super) fn new(namespace: &[String], imports: &FxHashMap<String, QName>) -> Self {
        Self {
            namespace: namespace.to_vec(),
            imports: imports.clone(),
        }
    }

    fn resolve(&self, text: &str) -> QName {
        nvs_hir::resolve_ref(text, &self.namespace, &self.imports)
    }
}

/// One parameter of the declaration a template fills in.
#[derive(Clone)]
pub(super) struct Param {
    name: String,
    variadic: bool,
    /// The default as written, and how tightly it binds. `None` when there is
    /// no default, or when it is more than a literal and so could mean
    /// something else at the use.
    default: Option<(String, u8)>,
}

impl Param {
    pub(super) fn of(param: &nvs_syntax::ast::Param, src: &SourceFile) -> Self {
        let default = param
            .default
            .as_ref()
            .filter(|value| literal(value))
            .map(|value| (span_text(src, value.span).to_owned(), precedence(value)));
        Self {
            name: strip_sigil(span_text(src, param.name)).to_owned(),
            variadic: param.variadic,
            default,
        }
    }
}

/// The edit that replaces one use with its template filled in.
pub(super) struct Fill {
    /// The filled-in template, as the warning names it.
    pub(super) shown: String,
    /// What the edit replaces: the use, or from the start of its statement
    /// when `var` lines go in front of it.
    pub(super) span: Span,
    pub(super) text: String,
    /// Each class the edit names that the use's file does not import yet.
    pub(super) imports: Vec<QName>,
}

/// The template `deprecation` names, filled in at the use at `span`, or `None`
/// where no fix is offered.
pub(super) fn fill(
    span: Span,
    owner: &QName,
    member: &Member,
    at_new: bool,
    deprecation: &Deprecation,
    index: &mut Option<SyntaxIndex>,
    env: &Env<'_>,
) -> Option<Fill> {
    let at = scope_at(env.stmts, env.src, span.start);
    let mut names = Names {
        at: &at,
        env,
        imports: Vec::new(),
    };
    let construct = at_new && deprecation.construct.is_some();
    if *member == Member::Type && !construct {
        let target = deprecation
            .scope
            .resolve(deprecation.replace.as_deref()?.trim());
        let written = names.write(&target)?;
        return Some(Fill {
            shown: written.clone(),
            span: class_name_span(span, env)?,
            text: written,
            imports: names.imports,
        });
    }
    let used = reparse(span, env)?;
    let (template, site) = match member {
        Member::Type => (deprecation.construct.as_deref()?, Site::call(&used, env)?),
        Member::Method(_) => (deprecation.replace.as_deref()?, Site::call(&used, env)?),
        Member::Property(_) => (deprecation.replace.as_deref()?, Site::property(&used, env)?),
        Member::Const(_) | Member::Case(_) => {
            (deprecation.replace.as_deref()?, Site::constant(&used, env)?)
        }
        Member::Param(..) => return None,
    };

    let copy = env.src.code_at(START, template)?;
    let mut diags = Diagnostics::new();
    let expr = nvs_syntax::parse_expression(&copy, &mut diags);
    let end = START + u32::try_from(template.trim_end().len()).ok()?;
    if diags.has_errors() || expr.span.end != end {
        return None;
    }
    let mut holes = Holes {
        src: &copy,
        scope: &deprecation.scope,
        found: Vec::new(),
    };
    holes.walk(&expr, LOWEST)?;
    let holes = holes.found;
    if matches!(member, Member::Property(_))
        && writes_to(span, index, env)
        && !matches!(
            expr.kind,
            ExprKind::PropertyAccess { .. }
                | ExprKind::StaticPropertyAccess { .. }
                | ExprKind::Index { .. }
        )
    {
        return None;
    }

    let mut values = site.values(&deprecation.params, owner, env)?;
    for hole in &holes {
        let Piece::Var(name) = &hole.piece else {
            continue;
        };
        let value = values.iter_mut().find(|value| value.param == *name)?;
        if value.used == 0 {
            value.first = hole.span.start;
        }
        value.used += 1;
    }
    if values
        .iter()
        .any(|value| value.default_missing && value.used > 0)
    {
        return None;
    }
    let moved = moved(&values);

    let text = copy.text();
    let mut shown = String::new();
    let mut last = START as usize;
    let mut fresh = Vec::new();
    let mut lines = String::new();
    let statement = if moved.is_empty() {
        None
    } else {
        let index = index.get_or_insert_with(|| SyntaxIndex::of_stmts(env.stmts));
        let statement = statement(span, index, env.src)?;
        if runs_before(statement, span, index) {
            return None;
        }
        let body =
            enclosing_body(span, index).map_or(env.src.text(), |body| span_text(env.src, body));
        let indent = indentation(statement.span, env.src);
        for &at in &moved {
            let value = &mut values[at];
            let name = fresh_name(&value.base, &mut fresh, body);
            let _ = write!(lines, "var ${name} = {};\n{indent}", value.text);
            value.text = format!("${name}");
            value.precedence = ATOM;
        }
        Some(statement.span)
    };
    for hole in &holes {
        shown.push_str(text.get(last..hole.span.start as usize)?);
        let (written, binds) = match &hole.piece {
            Piece::Var(name) => {
                let value = values.iter().find(|value| value.param == *name)?;
                (value.text.clone(), value.precedence)
            }
            Piece::Own => match &site.class {
                Some(class) => (class.clone(), ATOM),
                None => (names.write(owner)?, ATOM),
            },
            Piece::Class(target) => (names.write(target)?, ATOM),
        };
        if binds < hole.place {
            shown.push('(');
            shown.push_str(&written);
            shown.push(')');
        } else {
            shown.push_str(&written);
        }
        last = hole.span.end as usize;
    }
    shown.push_str(text.get(last..end as usize)?);

    let (span, text) = match statement {
        None => (span, shown.clone()),
        Some(statement) => {
            let before = span_text(
                env.src,
                Span {
                    file: span.file,
                    start: statement.start,
                    end: span.start,
                },
            );
            let edit = Span {
                file: span.file,
                start: statement.start,
                end: span.end,
            };
            (edit, format!("{lines}{before}{shown}"))
        }
    };
    Some(Fill {
        shown,
        span,
        text,
        imports: names.imports,
    })
}

/// Where a template's text starts in the copy it is parsed in: after the
/// `#!` line [`SourceFile::code_at`] opens code mode with.
const START: u32 = 3;

// How tightly an expression binds, lowest first: a hole holds text bare when
// that text binds at least as tightly as the hole's place.
const LOWEST: u8 = 0;
const TERNARY: u8 = 2;
const COALESCE: u8 = 3;
const OR: u8 = 4;
const AND: u8 = 5;
const BIT_OR: u8 = 6;
const BIT_XOR: u8 = 7;
const BIT_AND: u8 = 8;
const EQUALITY: u8 = 9;
const COMPARISON: u8 = 10;
const CONCAT: u8 = 11;
const SHIFT: u8 = 12;
const ADD: u8 = 13;
const MUL: u8 = 14;
const TYPE_TEST: u8 = 15;
const NOT: u8 = 16;
const UNARY: u8 = 17;
const POW: u8 = 18;
const NEW: u8 = 19;
const ATOM: u8 = 20;
/// The place of a unary operator's operand, a conversion's or a type test's.
const OPERAND: u8 = NEW;

/// A binary operator's precedence, then the places of its left and right
/// operands.
const fn binary(op: BinaryOp) -> (u8, u8, u8) {
    let (level, right) = match op {
        BinaryOp::Add | BinaryOp::Sub => (ADD, false),
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => (MUL, false),
        BinaryOp::Pow => (POW, true),
        BinaryOp::Concat => (CONCAT, false),
        BinaryOp::BitAnd => (BIT_AND, false),
        BinaryOp::BitOr => (BIT_OR, false),
        BinaryOp::BitXor => (BIT_XOR, false),
        BinaryOp::Shl | BinaryOp::Shr => (SHIFT, false),
        BinaryOp::And => (AND, false),
        BinaryOp::Or => (OR, false),
        BinaryOp::Coalesce => (COALESCE, true),
        BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Cmp => {
            return (EQUALITY, EQUALITY + 1, EQUALITY + 1);
        }
        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
            return (COMPARISON, COMPARISON + 1, COMPARISON + 1);
        }
        // An operator this table does not know binds loosest, and its operands
        // are parenthesized unless they are atoms.
        _ => return (LOWEST, ATOM, ATOM),
    };
    if right {
        (level, level + 1, level)
    } else {
        (level, level, level + 1)
    }
}

/// How tightly `expr` binds, by its outermost operator.
fn precedence(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Binary { op, .. } => binary(*op).0,
        ExprKind::Ternary { .. } => TERNARY,
        ExprKind::Unary {
            op: UnaryOp::Not, ..
        } => NOT,
        ExprKind::Unary { .. }
        | ExprKind::PreIncDec { .. }
        | ExprKind::PostIncDec { .. }
        | ExprKind::Conversion { .. }
        | ExprKind::Clone(_)
        | ExprKind::Await(_) => UNARY,
        ExprKind::TypeTest { .. } => TYPE_TEST,
        ExprKind::New { .. } | ExprKind::Match { .. } => NEW,
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Duration(_)
        | ExprKind::Str(_)
        | ExprKind::Interpolated(_)
        | ExprKind::HtmlTemplate(_)
        | ExprKind::Variable(_)
        | ExprKind::ConstFetch(_)
        | ExprKind::SelfExpr
        | ExprKind::StaticExpr
        | ExprKind::ParentExpr
        | ExprKind::ArrayLiteral(_)
        | ExprKind::Call { .. }
        | ExprKind::MethodCall { .. }
        | ExprKind::StaticCall { .. }
        | ExprKind::PropertyAccess { .. }
        | ExprKind::StaticPropertyAccess { .. }
        | ExprKind::ClassConstAccess { .. }
        | ExprKind::ClassNameConst { .. }
        | ExprKind::Index { .. }
        | ExprKind::Paren(_)
        | ExprKind::AnonObject(_)
        | ExprKind::Isset(_)
        | ExprKind::Empty(_) => ATOM,
        _ => LOWEST,
    }
}

/// Whether `expr` is a literal: what a default may be for a fix to write it
/// at a use, where a name in it could mean something else.
fn literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Duration(_)
        | ExprKind::Str(_) => true,
        ExprKind::Unary { expr, .. } | ExprKind::Paren(expr) => literal(expr),
        ExprKind::ArrayLiteral(items) => items.iter().all(|item| {
            !item.spread
                && !item.by_ref
                && item.key.as_ref().is_none_or(literal)
                && literal(&item.value)
        }),
        _ => false,
    }
}

/// Whether evaluating `expr` twice, or at another time, changes nothing: it
/// holds no call, `new`, assignment, increment or decrement, and nothing this
/// function does not know.
fn pure(expr: &Expr) -> bool {
    match &expr.kind {
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
        | ExprKind::ParentExpr => true,
        ExprKind::Unary { expr, .. }
        | ExprKind::Paren(expr)
        | ExprKind::Conversion { expr, .. }
        | ExprKind::TypeTest { expr, .. }
        | ExprKind::PropertyAccess { object: expr, .. }
        | ExprKind::StaticPropertyAccess { class: expr, .. }
        | ExprKind::ClassConstAccess { class: expr, .. }
        | ExprKind::ClassNameConst { class: expr } => pure(expr),
        ExprKind::Binary { lhs, rhs, .. } => pure(lhs) && pure(rhs),
        ExprKind::Ternary { cond, then, else_ } => {
            pure(cond) && then.as_deref().is_none_or(pure) && pure(else_)
        }
        ExprKind::Index { base, index } => pure(base) && index.as_deref().is_none_or(pure),
        ExprKind::ArrayLiteral(items) => items
            .iter()
            .all(|item| !item.by_ref && item.key.as_ref().is_none_or(pure) && pure(&item.value)),
        ExprKind::AnonObject(fields) => fields.iter().all(|field| pure(&field.value)),
        _ => false,
    }
}

/// What a template's hole is written over with.
enum Piece {
    /// A parameter, or `this`, without its `$`.
    Var(String),
    /// `self`: the class the use wrote, or the declaring class.
    Own,
    /// A class name, resolved where the template was written.
    Class(QName),
}

struct Hole {
    span: Span,
    piece: Piece,
    /// The loosest precedence the hole holds without parentheses.
    place: u8,
}

/// The holes of one template, collected in source order.
struct Holes<'t> {
    src: &'t SourceFile,
    scope: &'t Scope,
    found: Vec<Hole>,
}

impl Holes<'_> {
    /// `None` for a construct this module does not fill in.
    fn walk(&mut self, expr: &Expr, place: u8) -> Option<()> {
        match &expr.kind {
            ExprKind::Null
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Duration(_)
            | ExprKind::Str(_) => {}
            ExprKind::Variable(name) => {
                let name = strip_sigil(span_text(self.src, *name)).to_owned();
                self.hole(expr.span, Piece::Var(name), place);
            }
            ExprKind::SelfExpr => self.hole(expr.span, Piece::Own, place),
            ExprKind::ArrayLiteral(items) => {
                for item in items {
                    if item.spread || item.by_ref {
                        return None;
                    }
                    if let Some(key) = &item.key {
                        self.walk(key, LOWEST)?;
                    }
                    self.walk(&item.value, LOWEST)?;
                }
            }
            ExprKind::AnonObject(fields) => {
                for field in fields {
                    self.walk(&field.value, LOWEST)?;
                }
            }
            ExprKind::Unary { expr, .. } => self.walk(expr, OPERAND)?,
            ExprKind::Binary { op, lhs, rhs } => {
                let (_, left, right) = binary(*op);
                self.walk(lhs, left)?;
                self.walk(rhs, right)?;
            }
            ExprKind::Assign {
                op: AssignOp::Assign,
                target,
                value,
                by_ref: false,
            } if !matches!(target.kind, ExprKind::Variable(_)) => {
                self.walk(target, ATOM)?;
                self.walk(value, LOWEST)?;
            }
            ExprKind::Ternary { cond, then, else_ } => {
                self.walk(cond, TERNARY + 1)?;
                if let Some(then) = then {
                    self.walk(then, LOWEST)?;
                }
                self.walk(else_, TERNARY + 1)?;
            }
            ExprKind::Conversion {
                expr,
                ty,
                legacy: false,
            } if !span_text(self.src, ty.span).contains(char::is_uppercase) => {
                self.walk(expr, OPERAND)?;
            }
            ExprKind::MethodCall {
                object,
                method: MemberName::Ident(_),
                type_args,
                args,
                ..
            } if type_args.is_empty() => {
                self.walk(object, ATOM)?;
                self.args(args)?;
            }
            ExprKind::StaticCall {
                class,
                method: MemberName::Ident(_),
                type_args,
                args,
            } if type_args.is_empty() => {
                self.class(class)?;
                self.args(args)?;
            }
            ExprKind::PropertyAccess {
                object,
                property: MemberName::Ident(_),
                ..
            } => self.walk(object, ATOM)?,
            ExprKind::StaticPropertyAccess { class, .. }
            | ExprKind::ClassConstAccess { class, .. }
            | ExprKind::ClassNameConst { class } => self.class(class)?,
            ExprKind::Index { base, index } => {
                self.walk(base, ATOM)?;
                if let Some(index) = index {
                    self.walk(index, LOWEST)?;
                }
            }
            ExprKind::New {
                target: NewTarget::Name(name),
                type_args,
                args,
            } if type_args.is_empty() => {
                self.named(name.span);
                self.args(args)?;
            }
            ExprKind::Paren(inner) => self.walk(inner, LOWEST)?,
            _ => return None,
        }
        Some(())
    }

    fn args(&mut self, args: &CallArgs) -> Option<()> {
        let CallArgs::List(args) = args else {
            return None;
        };
        for arg in args {
            if arg.spread || arg.inout {
                return None;
            }
            self.walk(&arg.value, LOWEST)?;
        }
        Some(())
    }

    fn class(&mut self, class: &Expr) -> Option<()> {
        match &class.kind {
            ExprKind::ConstFetch(name) => self.named(name.span),
            ExprKind::SelfExpr => self.hole(class.span, Piece::Own, ATOM),
            _ => return None,
        }
        Some(())
    }

    fn named(&mut self, span: Span) {
        let target = self.scope.resolve(span_text(self.src, span));
        self.hole(span, Piece::Class(target), ATOM);
    }

    fn hole(&mut self, span: Span, piece: Piece, place: u8) {
        self.found.push(Hole { span, piece, place });
    }
}

/// What a use supplies to its template: the receiver `$this` becomes, the
/// class `self` becomes, and the arguments.
struct Site<'u> {
    receiver: Option<&'u Expr>,
    class: Option<String>,
    args: &'u [Arg],
}

impl<'u> Site<'u> {
    fn call(used: &'u Expr, env: &Env<'_>) -> Option<Self> {
        match &used.kind {
            ExprKind::MethodCall {
                object,
                nullsafe: false,
                args: CallArgs::List(args),
                ..
            } => Some(Self {
                receiver: Some(object),
                class: None,
                args,
            }),
            ExprKind::StaticCall {
                class,
                args: CallArgs::List(args),
                ..
            } => Some(Self {
                receiver: None,
                class: Some(span_text(env.src, class.span).to_owned()),
                args,
            }),
            ExprKind::New {
                target: NewTarget::Name(name),
                args: CallArgs::List(args),
                ..
            } => Some(Self {
                receiver: None,
                class: Some(span_text(env.src, name.span).to_owned()),
                args,
            }),
            _ => None,
        }
    }

    fn property(used: &'u Expr, env: &Env<'_>) -> Option<Self> {
        match &used.kind {
            ExprKind::PropertyAccess {
                object,
                nullsafe: false,
                ..
            } => Some(Self {
                receiver: Some(object),
                class: None,
                args: &[],
            }),
            ExprKind::StaticPropertyAccess { class, .. } => Some(Self {
                receiver: None,
                class: Some(span_text(env.src, class.span).to_owned()),
                args: &[],
            }),
            _ => None,
        }
    }

    fn constant(used: &'u Expr, env: &Env<'_>) -> Option<Self> {
        let ExprKind::ClassConstAccess { class, .. } = &used.kind else {
            return None;
        };
        Some(Self {
            receiver: None,
            class: Some(span_text(env.src, class.span).to_owned()),
            args: &[],
        })
    }

    /// One value per placeholder the template may use: the receiver as
    /// `this`, then each parameter bound to its argument or its default.
    /// `None` when an argument matches no parameter.
    fn values(&self, params: &[Param], owner: &QName, env: &Env<'_>) -> Option<Vec<Value>> {
        let mut values = Vec::new();
        if let Some(receiver) = self.receiver {
            let mut base = owner.short_name().to_owned();
            if let Some(first) = base.get(..1) {
                base.replace_range(..1, &first.to_lowercase());
            }
            values.push(Value::of(receiver, "this", base, 0, env));
        }
        let mut bound: Vec<Option<(usize, &Arg)>> = vec![None; params.len()];
        let mut next = 0;
        for (written, arg) in self.args.iter().enumerate() {
            if arg.spread || arg.inout {
                return None;
            }
            let at = match arg.name {
                Some(name) => {
                    let name = span_text(env.src, name);
                    params.iter().position(|param| param.name == name)?
                }
                None => {
                    next += 1;
                    next - 1
                }
            };
            let slot = bound.get_mut(at)?;
            if slot.is_some() || params[at].variadic {
                return None;
            }
            *slot = Some((written + 1, arg));
        }
        for (param, arg) in params.iter().zip(bound) {
            values.push(match arg {
                Some((written, arg)) => {
                    Value::of(&arg.value, &param.name, param.name.clone(), written, env)
                }
                None => match &param.default {
                    Some((text, precedence)) => Value {
                        param: param.name.clone(),
                        base: param.name.clone(),
                        text: text.clone(),
                        precedence: *precedence,
                        pure: true,
                        written: usize::MAX,
                        used: 0,
                        first: 0,
                        default_missing: false,
                    },
                    None => Value {
                        param: param.name.clone(),
                        base: param.name.clone(),
                        text: String::new(),
                        precedence: ATOM,
                        pure: true,
                        written: usize::MAX,
                        used: 0,
                        first: 0,
                        default_missing: true,
                    },
                },
            });
        }
        Some(values)
    }
}

/// What one placeholder is filled in with.
struct Value {
    /// The placeholder, without its `$`.
    param: String,
    /// What a `var` line names it, before it is made fresh.
    base: String,
    text: String,
    precedence: u8,
    pure: bool,
    /// Where the use wrote it: the receiver first, then each argument in
    /// order. A default was not written.
    written: usize,
    /// How many holes it fills, and where the first one starts.
    used: usize,
    first: u32,
    /// An omitted parameter with no default a fix can write.
    default_missing: bool,
}

impl Value {
    fn of(expr: &Expr, param: &str, base: String, written: usize, env: &Env<'_>) -> Self {
        Self {
            param: param.to_owned(),
            base,
            text: span_text(env.src, expr.span).to_owned(),
            precedence: precedence(expr),
            pure: pure(expr),
            written,
            used: 0,
            first: 0,
            default_missing: false,
        }
    }
}

/// The impure values that move to a `var` line, in the order the use wrote
/// them. One stays only when the template uses it once, after every impure
/// value written before it, and no value that moves was written after it:
/// what moves runs first.
fn moved(values: &[Value]) -> Vec<usize> {
    let mut impure: Vec<usize> = (0..values.len()).filter(|&at| !values[at].pure).collect();
    impure.sort_by_key(|&at| values[at].written);
    let mut cut = 0;
    'again: loop {
        let mut last = None;
        for (rank, &at) in impure.iter().enumerate().skip(cut) {
            let value = &values[at];
            if value.used != 1 || last.is_some_and(|last| value.first < last) {
                cut = rank + 1;
                continue 'again;
            }
            last = Some(value.first);
        }
        break;
    }
    impure.truncate(cut);
    impure
}

/// Writes a class name the way the use's file resolves it, and collects the
/// imports that takes.
struct Names<'n, 'e> {
    at: &'n Scope,
    env: &'n Env<'e>,
    imports: Vec<QName>,
}

impl Names<'_, '_> {
    /// The shortest name that resolves to `target` at the use, or `None` when
    /// no name does without an alias.
    fn write(&mut self, target: &QName) -> Option<String> {
        let short = target.short_name();
        if self.at.resolve(short) == *target || self.imports.contains(target) {
            return Some(short.to_owned());
        }
        let taken = self.at.imports.contains_key(short)
            || self.imports.iter().any(|other| other.short_name() == short)
            || self
                .env
                .symbols
                .contains(&QName::join(&self.at.namespace, short));
        if !taken {
            self.imports.push(target.clone());
            return Some(short.to_owned());
        }
        (target.segments().len() > 1).then(|| target.to_string())
    }
}

/// The namespace and imports in force at `at`, read the way
/// [`super::collect`] reads them.
fn scope_at(stmts: &[Stmt], src: &SourceFile, at: u32) -> Scope {
    let mut scope = Scope::default();
    for stmt in stmts {
        if stmt.span.start > at {
            break;
        }
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let namespace = name.as_ref().map_or_else(Vec::new, |name| {
                    QName::parse(span_text(src, name.span)).segments().to_vec()
                });
                match body {
                    Some(block) if block.span.start <= at && at < block.span.end => {
                        let mut inner = scope_at(&block.stmts, src, at);
                        if inner.namespace.is_empty() {
                            inner.namespace = namespace;
                        }
                        return inner;
                    }
                    Some(_) => {}
                    None => {
                        scope = Scope {
                            namespace,
                            imports: FxHashMap::default(),
                        };
                    }
                }
            }
            StmtKind::UseDecl(use_decl) => {
                let target = QName::parse(span_text(src, use_decl.path.span));
                scope.imports.insert(target.short_name().to_owned(), target);
            }
            _ => {}
        }
    }
    scope
}

/// The use, parsed again from its own text where it was written.
fn reparse(span: Span, env: &Env<'_>) -> Option<Expr> {
    let copy = env.src.code_at(span.start, span_text(env.src, span))?;
    let mut diags = Diagnostics::new();
    let expr = nvs_syntax::parse_expression(&copy, &mut diags);
    (!diags.has_errors() && expr.span.start == span.start && expr.span.end == span.end)
        .then_some(expr)
}

/// Where a deprecated class's name is written in the use at `span`: the whole
/// span in a type position, or the class part of a static access or a `new`.
fn class_name_span(span: Span, env: &Env<'_>) -> Option<Span> {
    let text = span_text(env.src, span);
    if !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '\\')
    {
        return Some(span);
    }
    match reparse(span, env)?.kind {
        ExprKind::StaticCall { class, .. }
        | ExprKind::StaticPropertyAccess { class, .. }
        | ExprKind::ClassConstAccess { class, .. }
        | ExprKind::ClassNameConst { class } => {
            matches!(class.kind, ExprKind::ConstFetch(_)).then_some(class.span)
        }
        ExprKind::New {
            target: NewTarget::Name(name),
            ..
        } => Some(name.span),
        _ => None,
    }
}

/// The statements a `var` line may be written in front of.
const STATEMENTS: &[&str] = &[
    "Expr",
    "Return",
    "Echo",
    "LocalDecl",
    "If",
    "While",
    "DoWhile",
    "For",
    "Foreach",
    "Switch",
    "Unset",
    "Global",
    "StaticLocal",
    "Destructure",
];

/// The statements whose body is a single statement when it has no braces.
const BODIES: &[&str] = &["If", "While", "DoWhile", "For", "Foreach"];

/// The nodes that run code a `var` line in front of the statement would run
/// earlier.
const EFFECTS: &[&str] = &[
    "Call",
    "MethodCall",
    "StaticCall",
    "New",
    "Assign",
    "PreIncDec",
    "PostIncDec",
    "Yield",
    "Await",
    "Print",
    "Throw",
    "Require",
    "SpawnScript",
    "Exit",
    "Clone",
];

/// The statement around the use at `span`, when a `var` line in front of it
/// runs exactly when the use's own code would have run first.
fn statement(span: Span, index: &SyntaxIndex, src: &SourceFile) -> Option<IndexNode> {
    let path = index.at(span.start);
    let nodes = path.nodes();
    let mut from: Option<IndexNode> = None;
    for (at, node) in nodes.iter().enumerate() {
        let is_statement = STATEMENTS.contains(&node.kind);
        if !is_statement && span.start <= node.span.start && node.span.end <= span.end {
            from = Some(*node);
            continue;
        }
        let child = from?;
        let children = index.children_of(*node);
        let rank = children.iter().position(|other| *other == child)?;
        if is_statement {
            let in_header = match node.kind {
                "If" | "Switch" | "Foreach" => rank == 0,
                "While" | "DoWhile" | "For" => false,
                _ => true,
            };
            let unbraced = nodes
                .get(at + 1)
                .is_some_and(|parent| BODIES.contains(&parent.kind));
            return (in_header && !unbraced).then_some(*node);
        }
        let later = match node.kind {
            "Fn" => true,
            "Ternary" | "Match" | "Catch" => rank > 0,
            "Binary" | "Assign" => {
                rank == 1
                    && children.first().is_some_and(|left| {
                        let between = src
                            .text()
                            .get(left.span.end as usize..child.span.start as usize)
                            .unwrap_or_default()
                            .trim_start();
                        ["&&", "||", "??"].iter().any(|op| between.starts_with(op))
                    })
            }
            "Method" | "Function" | "Property" | "Const" | "EnumCase" | "ClassDecl"
            | "InterfaceDecl" | "EnumDecl" | "NamespaceDecl" | "Block" | "Try" => true,
            _ => false,
        };
        if later {
            return None;
        }
        from = Some(*node);
    }
    None
}

/// Whether code with a side effect in `statement` runs before the use at
/// `span`, so a `var` line in front of the statement would run before it.
fn runs_before(statement: IndexNode, span: Span, index: &SyntaxIndex) -> bool {
    let mut stack = vec![statement];
    while let Some(node) = stack.pop() {
        if node.span.start >= span.start {
            continue;
        }
        if node.span.end <= span.start && EFFECTS.contains(&node.kind) {
            return true;
        }
        stack.extend(index.children_of(node));
    }
    false
}

/// Whether the use at `span` is written to: the target of an assignment, an
/// increment or a decrement.
fn writes_to(span: Span, index: &mut Option<SyntaxIndex>, env: &Env<'_>) -> bool {
    let index = index.get_or_insert_with(|| SyntaxIndex::of_stmts(env.stmts));
    let path = index.at(span.start);
    let Some(parent) = path.nodes().iter().find(|node| {
        node.span != span && node.span.start <= span.start && span.end <= node.span.end
    }) else {
        return false;
    };
    match parent.kind {
        "PreIncDec" | "PostIncDec" => true,
        "Assign" => index
            .children_of(*parent)
            .first()
            .is_some_and(|target| target.span == span),
        _ => false,
    }
}

/// The method or function the use at `span` is in, whose variables a fresh
/// name must not meet.
fn enclosing_body(span: Span, index: &SyntaxIndex) -> Option<Span> {
    index
        .at(span.start)
        .nodes()
        .iter()
        .find(|node| matches!(node.kind, "Method" | "Function"))
        .map(|node| node.span)
}

/// The blanks in front of `span` on its line, when nothing else is.
fn indentation(span: Span, src: &SourceFile) -> String {
    let line = src.line_index(span.start);
    let start = src.line_start(line).unwrap_or(span.start);
    let before = src
        .text()
        .get(start as usize..span.start as usize)
        .unwrap_or_default();
    if before.chars().all(char::is_whitespace) {
        before.to_owned()
    } else {
        String::new()
    }
}

/// `base`, or `base` with the first number from 2 up that makes it a name
/// `body` does not write and no earlier `var` line took.
fn fresh_name(base: &str, taken: &mut Vec<String>, body: &str) -> String {
    let mut count = 1;
    loop {
        let name = match count {
            1 => base.to_owned(),
            _ => format!("{base}{count}"),
        };
        count += 1;
        if !taken.contains(&name) && !mentions(body, &name) {
            taken.push(name.clone());
            return name;
        }
    }
}

fn mentions(body: &str, name: &str) -> bool {
    let needle = format!("${name}");
    body.match_indices(&needle).any(|(at, _)| {
        !body[at + needle.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
    })
}
