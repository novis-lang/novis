//! The property names a property callback compares with its name parameter
//! (`rule:classes/a-compared-property-name-is-a-use`).
//!
//! The runtime calls five methods with a property's name as their first
//! parameter, the [`CALLBACKS`]. Inside one, a string literal compared with
//! that parameter names a property of the method's class. [`of_method`] reads
//! one method's body and answers which literals those are, which other plain
//! literals the body writes, and where the body first writes the parameter,
//! which makes no comparison in it readable. `nvs_types` warns at a compared
//! name no property has, and `nvs_lsp`'s index records each compared name as
//! an occurrence of its property, so rename, references and highlight follow
//! it with no code of their own.
//!
//! A comparison is `==` or `!=` with the parameter on either side, a `match`
//! on the parameter, whose arm conditions are compared with it, and a
//! `switch` on it, whose `case` values are. An anonymous function in the body
//! reads the parameter by value, so its body is read too, unless one of its
//! own parameters has the same name. An anonymous class is not read: it has
//! no access to the parameter.
//!
//! **What it spends:** two vectors of spans per callback method, made when a
//! caller asks and dropped with its answer. Nothing on the request path.

use nvs_diagnostics::Span;
use nvs_syntax::ast::{
    ArrayItem, BinaryOp, CallArgs, ClassMember, ClassMemberKind, DestructureElement,
    DestructureTarget, Expr, ExprKind, FnBody, FnExpr, MethodMember, Stmt, StmtKind,
};
use nvs_syntax::visit::{StmtChild, each_child_expr, each_child_of_stmt};

use crate::interfaces::{LATE_INIT, PROPERTY_GUARD, PROPERTY_OBSERVER};

/// Each method the runtime calls with a property's name as its first
/// parameter, after the interface that declares it.
///
/// `PropertyChangeObserver::onPropertyChanged` joins this list when that
/// interface is declared.
pub const CALLBACKS: [(&str, &str); 5] = [
    (LATE_INIT, "initProperty"),
    (PROPERTY_OBSERVER, "onPropertyGet"),
    (PROPERTY_OBSERVER, "onPropertySet"),
    (PROPERTY_GUARD, "beforePropertySet"),
    (PROPERTY_GUARD, "beforePropertyUnset"),
];

/// The interface that declares the callback named `method`, or `None` for a
/// method that is no callback.
#[must_use]
pub fn interface_of(method: &str) -> Option<&'static str> {
    CALLBACKS
        .iter()
        .find(|(_, name)| *name == method)
        .map(|(interface, _)| *interface)
}

/// What one callback's body does with its name parameter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComparedNames {
    /// Each literal compared with the name parameter, as the span of its
    /// text between the quotes, in source order.
    pub compared: Vec<Span>,
    /// Each other plain string literal the body writes, between its quotes,
    /// in source order.
    pub other: Vec<Span>,
    /// Where the body first writes the name parameter. A comparison after a
    /// write may not compare a property's name, so a caller reads none of
    /// [`Self::compared`] while this is set.
    pub assigned: Option<Span>,
}

/// Every method of `members` named after one of the [`CALLBACKS`] that has a
/// body, with the interface that declares it and what it compares. `source`
/// is the text of the file the members are in. Whether the class implements
/// that interface is the caller's question, since it needs the class graph.
#[must_use]
pub fn callbacks_in<'a>(
    members: &'a [ClassMember],
    source: &str,
) -> Vec<(&'a MethodMember, &'static str, ComparedNames)> {
    members
        .iter()
        .filter_map(|member| {
            let ClassMemberKind::Method(method) = &member.kind else {
                return None;
            };
            let interface = interface_of(source.get(method.name.range())?)?;
            Some((method, interface, of_method(method, source)?))
        })
        .collect()
}

/// What `method`'s body does with its first parameter, or `None` for a
/// method with no body or no parameter. `source` is the text of the file
/// `method` is in.
#[must_use]
pub fn of_method(method: &MethodMember, source: &str) -> Option<ComparedNames> {
    let body = method.body.as_ref()?;
    let param = method.params.first()?;
    let name = source.get(param.name.range())?;
    let mut scan = Scan {
        source,
        name,
        found: ComparedNames::default(),
    };
    scan.stmts(&body.stmts);
    let mut found = scan.found;
    found.compared.sort_by_key(|span| span.start);
    found.compared.dedup();
    let compared = found.compared.clone();
    found.other.retain(|span| !compared.contains(span));
    found.other.sort_by_key(|span| span.start);
    found.other.dedup();
    Some(found)
}

/// The text between the quotes of a single- or double-quoted literal at
/// `span`, or `None` for a heredoc, a nowdoc or anything else.
fn quoted(span: Span, source: &str) -> Option<Span> {
    let text = source.get(span.range())?;
    let quote = text.chars().next().filter(|c| matches!(c, '\'' | '"'))?;
    (text.len() >= 2 && text.ends_with(quote)).then_some(Span {
        file: span.file,
        start: span.start + 1,
        end: span.end - 1,
    })
}

/// One walk of one callback's body.
struct Scan<'s> {
    source: &'s str,
    /// The name parameter, `$` included.
    name: &'s str,
    found: ComparedNames,
}

impl Scan<'_> {
    fn text(&self, span: Span) -> &str {
        self.source.get(span.range()).unwrap_or_default()
    }

    /// Whether `expr` reads the name parameter.
    fn is_name(&self, expr: &Expr) -> bool {
        matches!(&expr.unparenthesized().kind, ExprKind::Variable(written) if self.text(*written) == self.name)
    }

    /// Whether a write to `target` writes the name parameter: the variable
    /// itself, an element of it, or a variable an array pattern binds.
    fn writes_name(&self, target: &Expr) -> bool {
        let mut target = target.unparenthesized();
        while let ExprKind::Index { base, .. } = &target.kind {
            target = base.unparenthesized();
        }
        match &target.kind {
            ExprKind::ArrayLiteral(items) => items
                .iter()
                .any(|item: &ArrayItem| self.writes_name(&item.value)),
            _ => self.is_name(target),
        }
    }

    fn assigned_at(&mut self, span: Span) {
        self.found.assigned.get_or_insert(span);
    }

    /// Records a binding written at `name`, when it binds the name parameter.
    fn binds(&mut self, name: Span) {
        if self.text(name) == self.name {
            self.assigned_at(name);
        }
    }

    /// Records `expr` as a compared name when it is a quoted literal.
    fn compare(&mut self, expr: &Expr) {
        if let ExprKind::Str(span) = &expr.unparenthesized().kind
            && let Some(inner) = quoted(*span, self.source)
        {
            self.found.compared.push(inner);
        }
    }

    fn destructure(&mut self, target: &DestructureTarget) {
        for element in &target.elements {
            match element {
                DestructureElement::Leaf { name, .. } => self.binds(*name),
                DestructureElement::Nested { target, .. } => self.destructure(target),
                _ => {}
            }
        }
    }

    fn call_args(&mut self, args: &CallArgs) {
        if let CallArgs::List(args) = args {
            for arg in args {
                if arg.inout && self.writes_name(&arg.value) {
                    self.assigned_at(arg.value.span);
                }
            }
        }
    }

    fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Switch { subject, cases } if self.is_name(subject) => {
                for case in cases {
                    if let Some(value) = &case.cond {
                        self.compare(value);
                    }
                }
            }
            StmtKind::Foreach { key, value, .. } => {
                if let Some(key) = key {
                    self.binds(key.name);
                }
                self.binds(value.name);
            }
            StmtKind::Try { catches, .. } => {
                for clause in catches {
                    if let Some(var) = clause.var {
                        self.binds(var);
                    }
                }
            }
            StmtKind::LocalDecl { name, .. } => self.binds(*name),
            StmtKind::Destructure { target, .. } => self.destructure(target),
            StmtKind::StaticLocal { vars, .. } => {
                for var in vars {
                    self.binds(var.name);
                }
            }
            StmtKind::Global(names) => {
                for name in names {
                    self.binds(*name);
                }
            }
            StmtKind::Unset(targets) => {
                for target in targets {
                    if self.writes_name(target) {
                        self.assigned_at(target.span);
                    }
                }
            }
            _ => {}
        }
        each_child_of_stmt(stmt, &mut |child| match child {
            StmtChild::Stmt(nested) => self.stmt(nested),
            StmtChild::Expr(expr) => self.expr(expr),
        });
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Str(span) => {
                if let Some(inner) = quoted(*span, self.source) {
                    self.found.other.push(inner);
                }
            }
            ExprKind::Binary {
                op: BinaryOp::Eq | BinaryOp::NotEq,
                lhs,
                rhs,
            } => {
                if self.is_name(lhs) {
                    self.compare(rhs);
                }
                if self.is_name(rhs) {
                    self.compare(lhs);
                }
            }
            ExprKind::Match { subject, arms } if self.is_name(subject) => {
                for arm in arms {
                    for condition in arm.conditions.iter().flatten() {
                        self.compare(condition);
                    }
                }
            }
            ExprKind::Assign { target, .. }
            | ExprKind::PreIncDec { expr: target, .. }
            | ExprKind::PostIncDec { expr: target, .. } => {
                if self.writes_name(target) {
                    self.assigned_at(target.span);
                }
            }
            ExprKind::Call { args, .. }
            | ExprKind::MethodCall { args, .. }
            | ExprKind::StaticCall { args, .. }
            | ExprKind::New { args, .. } => self.call_args(args),
            ExprKind::Fn(function) => self.function(function),
            _ => {}
        }
        each_child_expr(expr, &mut |child| self.expr(child));
    }

    /// An anonymous function's body, which reads the name parameter by value
    /// unless one of its own parameters has that name.
    fn function(&mut self, function: &FnExpr) {
        if function
            .params
            .iter()
            .any(|param| self.text(param.name) == self.name)
        {
            return;
        }
        match &function.body {
            FnBody::Expr(body) => self.expr(body),
            FnBody::Block(block) => self.stmts(&block.stmts),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_diagnostics::{Diagnostics, SourceMap};

    /// The compared, other and assigned texts of the first method of the
    /// first class in `source`.
    fn read(source: &str) -> (Vec<String>, Vec<String>, bool) {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", source.to_owned());
        let file = map.file(id);
        let stmts = nvs_syntax::parse_file(file, &mut Diagnostics::new());
        let decl = stmts
            .iter()
            .find_map(|stmt| match &stmt.kind {
                StmtKind::ClassDecl(decl) => Some(decl),
                _ => None,
            })
            .expect("a class");
        let (_, _, found) = callbacks_in(&decl.members, file.text())
            .into_iter()
            .next()
            .expect("a callback");
        let texts = |spans: &[Span]| {
            spans
                .iter()
                .map(|span| source[span.range()].to_owned())
                .collect::<Vec<_>>()
        };
        (
            texts(&found.compared),
            texts(&found.other),
            found.assigned.is_some(),
        )
    }

    #[test]
    fn a_comparison_on_either_side_a_match_and_a_switch_are_compared() {
        let (compared, other, assigned) = read(
            "<?nvs\nclass Page implements LateInit {\n\
             public function initProperty(string $name): void {\n\
             if ($name == 'views') { return; }\n\
             if (\"title\" != $name) { return; }\n\
             echo match ($name) { 'body', 'tags' => 'text', default => 'x' };\n\
             switch ($name) { case 'author': break; }\n\
             }\n}\n",
        );
        assert_eq!(compared, ["views", "title", "body", "tags", "author"]);
        assert_eq!(other, ["text", "x"]);
        assert!(!assigned);
    }

    #[test]
    fn a_written_name_parameter_is_assigned_and_a_shadowing_function_is_not_read() {
        let (compared, _, assigned) = read(
            "<?nvs\nclass Page implements LateInit {\n\
             public function initProperty(string $name): void {\n\
             $check = fn(string $name): bool => $name == 'other';\n\
             $name = 'views';\n\
             }\n}\n",
        );
        assert!(compared.is_empty());
        assert!(assigned);
    }
}
