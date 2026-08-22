//! Lowers one already-checked method body to a [`crate::ir::Function`] — see
//! the crate's own module docs for exactly which statement/expression shapes
//! this first slice covers, and why it trusts its input rather than
//! re-checking it.

use mwl_diagnostics::SourceFile;
use mwl_syntax::ast::{
    AssignOp, BinaryOp, Expr, ExprKind, MethodMember, StmtKind, Type, TypeAtom, TypeKind,
    UnaryOp as AstUnaryOp,
};
use rustc_hash::FxHashMap;

use crate::ids::{IdGen, ValueId};
use crate::ir::{BasicBlock, BinOp, Function, Inst, InstKind, Terminator, UnOp};
use crate::ty::Ty;
use crate::{span_text, strip_sigil};

/// Lowers `m` — which must have a body, and whose body must stay within this
/// slice's supported statement/expression shapes (see the crate docs) — to a
/// single-block [`Function`] named `name`.
///
/// # Panics
///
/// Panics, naming the unsupported shape, if `m` has no body or its body
/// leaves this slice's scope. This is not a diagnostic: callers are expected
/// to have already run `mwl_types::check_program` and to only route programs
/// within scope through this function until lowering widens.
#[must_use]
pub fn lower_method(name: &str, m: &MethodMember, src: &SourceFile) -> Function {
    let mut ids = IdGen::new();
    let entry = ids.next_block();
    let mut insts = Vec::new();
    let mut locals: FxHashMap<String, (ValueId, Ty)> = FxHashMap::default();
    let mut param_tys = Vec::new();

    for (i, p) in m.params.iter().enumerate() {
        let decl_ty =
            p.ty.as_ref()
                .unwrap_or_else(|| panic!("ADR 0007 § 1: every parameter has a declared type"));
        let ty = lower_scalar_type(decl_ty);
        let index = u32::try_from(i).expect("far more parameters than a call could ever take");
        let (v, _) = emit(&mut insts, &mut ids, ty, InstKind::Param(index));
        let pname = strip_sigil(span_text(src, p.name)).to_owned();
        locals.insert(pname, (v, ty));
        param_tys.push(ty);
    }

    let ret_ty = m.return_type.as_ref().map_or(Ty::Void, lower_scalar_type);
    let body = m.body.as_ref().expect(
        "lower_method requires a method with a body — nothing to lower for an abstract one",
    );

    let mut term = Terminator::Return(None);
    for stmt in &body.stmts {
        let stmt_id = ids.next_stmt(stmt.span);
        insts.push(Inst {
            result: None,
            ty: None,
            kind: InstKind::StmtMarker(stmt_id),
        });
        match &stmt.kind {
            StmtKind::LocalDecl {
                ty: Some(decl_ty),
                name: local_name,
                value: Some(value),
            } => {
                let expected = lower_scalar_type(decl_ty);
                let (v, _) = lower_expr(value, Some(expected), &locals, &mut ids, &mut insts, src);
                locals.insert(
                    strip_sigil(span_text(src, *local_name)).to_owned(),
                    (v, expected),
                );
            }
            StmtKind::Expr(e) => {
                lower_reassignment(e, &mut locals, &mut ids, &mut insts, src);
            }
            StmtKind::Return(value) => {
                term =
                    Terminator::Return(value.as_ref().map(|v| {
                        lower_expr(v, Some(ret_ty), &locals, &mut ids, &mut insts, src).0
                    }));
            }
            other => panic!(
                "mwl-ir's first slice only lowers a typed local declaration, a plain \
                 reassignment and `return` — got {other:?}; see the crate docs' known gaps"
            ),
        }
    }

    let (stmt_spans, edge_spans) = ids.into_spans();
    Function {
        name: name.to_owned(),
        params: param_tys,
        ret: ret_ty,
        blocks: vec![BasicBlock {
            id: entry,
            insts,
            term,
        }],
        entry,
        stmt_spans,
        edge_spans,
    }
}

/// `$x = expr;` as a bare expression statement — SSA renaming needs no join
/// logic here since a straight-line body has exactly one predecessor for
/// every use.
fn lower_reassignment(
    e: &Expr,
    locals: &mut FxHashMap<String, (ValueId, Ty)>,
    ids: &mut IdGen,
    insts: &mut Vec<Inst>,
    src: &SourceFile,
) {
    let ExprKind::Assign {
        op: AssignOp::Assign,
        target,
        value,
        by_ref: false,
    } = &e.kind
    else {
        panic!(
            "mwl-ir's first slice only lowers a plain `$x = expr;` reassignment as an \
             expression statement — got {:?}; see the crate docs' known gaps",
            e.kind
        );
    };
    let ExprKind::Variable(name_span) = &target.kind else {
        panic!(
            "mwl-ir's first slice only lowers reassignment to a plain local, not {:?}",
            target.kind
        );
    };
    let lname = strip_sigil(span_text(src, *name_span)).to_owned();
    let expected = locals.get(&lname).map(|&(_, t)| t);
    let (v, ty) = lower_expr(value, expected, locals, ids, insts, src);
    locals.insert(lname, (v, ty));
}

fn lower_expr(
    expr: &Expr,
    expected: Option<Ty>,
    locals: &FxHashMap<String, (ValueId, Ty)>,
    ids: &mut IdGen,
    insts: &mut Vec<Inst>,
    src: &SourceFile,
) -> (ValueId, Ty) {
    match &expr.kind {
        ExprKind::Bool(b) => emit(insts, ids, Ty::Bool, InstKind::ConstBool(*b)),
        // ADR 0007 § 4, mirroring `mwl_types::expr::infer`'s own rule: a bare
        // integer literal means `uint` exactly where that's the expected
        // type, `int` otherwise. Magnitude range-checking is a known gap
        // here, same as it already is there.
        ExprKind::Int(span) => {
            let digits = clean_digits(src, *span);
            if expected == Some(Ty::Uint) {
                let n: u64 = digits.parse().unwrap_or_else(|_| {
                    panic!("mwl-ir: integer literal `{digits}` doesn't fit a `uint`")
                });
                emit(insts, ids, Ty::Uint, InstKind::ConstUint(n))
            } else {
                let n: i64 = digits.parse().unwrap_or_else(|_| {
                    panic!("mwl-ir: integer literal `{digits}` doesn't fit an `int`")
                });
                emit(insts, ids, Ty::Int, InstKind::ConstInt(n))
            }
        }
        ExprKind::Float(span) => {
            let digits = clean_digits(src, *span);
            let n: f64 = digits
                .parse()
                .unwrap_or_else(|_| panic!("mwl-ir: float literal `{digits}` failed to parse"));
            emit(insts, ids, Ty::Float, InstKind::ConstFloat(n))
        }
        ExprKind::Variable(span) => {
            let name = strip_sigil(span_text(src, *span));
            let &(v, ty) = locals.get(name).unwrap_or_else(|| {
                panic!(
                    "mwl-ir: undeclared local `${name}` — lower_method trusts its input already \
                     passed mwl_types::check_program"
                )
            });
            (v, ty)
        }
        ExprKind::Unary { op, expr: inner } => {
            let (v, ty) = lower_expr(inner, expected, locals, ids, insts, src);
            let uop = match op {
                AstUnaryOp::Neg => UnOp::Neg,
                AstUnaryOp::Not => UnOp::Not,
                other => panic!(
                    "mwl-ir's first slice only lowers unary `-`/`!` — got {other:?}; see the \
                     crate docs' known gaps"
                ),
            };
            emit(
                insts,
                ids,
                ty,
                InstKind::UnOp {
                    op: uop,
                    operand: v,
                },
            )
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let (lv, lty) = lower_expr(lhs, expected, locals, ids, insts, src);
            let (rv, _) = lower_expr(rhs, Some(lty), locals, ids, insts, src);
            let (bop, ty) = match op {
                BinaryOp::Add => (BinOp::Add, lty),
                BinaryOp::Sub => (BinOp::Sub, lty),
                BinaryOp::Mul => (BinOp::Mul, lty),
                BinaryOp::Div => (BinOp::Div, lty),
                BinaryOp::Mod => (BinOp::Mod, lty),
                BinaryOp::Eq | BinaryOp::Identical => (BinOp::Eq, Ty::Bool),
                BinaryOp::NotEq | BinaryOp::NotIdentical => (BinOp::NotEq, Ty::Bool),
                BinaryOp::Lt => (BinOp::Lt, Ty::Bool),
                BinaryOp::LtEq => (BinOp::LtEq, Ty::Bool),
                BinaryOp::Gt => (BinOp::Gt, Ty::Bool),
                BinaryOp::GtEq => (BinOp::GtEq, Ty::Bool),
                other => panic!(
                    "mwl-ir's first slice only lowers arithmetic/equality/ordering operators \
                     — got {other:?}; see the crate docs' known gaps"
                ),
            };
            emit(
                insts,
                ids,
                ty,
                InstKind::BinOp {
                    op: bop,
                    lhs: lv,
                    rhs: rv,
                },
            )
        }
        other => panic!(
            "mwl-ir's first slice only lowers literals, locals, and unary/binary operators over \
             them — got {other:?}; see the crate docs' known gaps"
        ),
    }
}

fn emit(insts: &mut Vec<Inst>, ids: &mut IdGen, ty: Ty, kind: InstKind) -> (ValueId, Ty) {
    let v = ids.next_value();
    insts.push(Inst {
        result: Some(v),
        ty: Some(ty),
        kind,
    });
    (v, ty)
}

/// Reads a numeric-literal span's text with `_` digit separators stripped.
fn clean_digits(src: &SourceFile, span: mwl_diagnostics::Span) -> String {
    span_text(src, span).chars().filter(|&c| c != '_').collect()
}

fn lower_scalar_type(ty: &Type) -> Ty {
    match &ty.kind {
        TypeKind::Atom(TypeAtom::Bool) => Ty::Bool,
        TypeKind::Atom(TypeAtom::Int) => Ty::Int,
        TypeKind::Atom(TypeAtom::Uint) => Ty::Uint,
        TypeKind::Atom(TypeAtom::Float) => Ty::Float,
        TypeKind::Atom(TypeAtom::Void) => Ty::Void,
        TypeKind::Paren(inner) => lower_scalar_type(inner),
        other => panic!(
            "mwl-ir's first slice only lowers bool/int/uint/float/void types — got {other:?}; \
             see the crate docs' known gaps"
        ),
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use mwl_diagnostics::{Diagnostics, SourceId, SourceMap};
    use mwl_syntax::ast::{ClassMemberKind, StmtKind as TopStmtKind};
    use mwl_syntax::parse_file;

    use super::*;
    use crate::print::print_function;

    /// Parses `src`, pulls out `T`'s first method, and lowers it —
    /// `src` is expected to already be a program `mwl_types::check_program`
    /// would accept with no errors (this crate does not re-check it).
    fn lower_first_method(src: &str) -> (Function, SourceMap, SourceId) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");

        let decl = stmts
            .iter()
            .find_map(|s| match &s.kind {
                TopStmtKind::ClassDecl(decl) => Some(decl),
                _ => None,
            })
            .expect("fixture must declare a class");
        let method = decl
            .members
            .iter()
            .find_map(|m| match &m.kind {
                ClassMemberKind::Method(method) => Some(method),
                _ => None,
            })
            .expect("fixture class must declare a method");

        let name = span_text(map.file(file), method.name).to_owned();
        let f = lower_method(&name, method, map.file(file));
        (f, map, file)
    }

    #[test]
    fn straight_line_arithmetic_and_return() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function add(int $a, int $b): int {\n    int $sum = $a + $b;\n    return $sum;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A plain local reassignment gets a fresh SSA value rather than mutating
    /// the one already bound to `$n` — the point of routing even
    /// straight-line reassignment through `lower_reassignment`.
    #[test]
    fn reassignment_produces_a_fresh_ssa_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function bump(int $n): int {\n    int $out = $n;\n    $out = $out + 1;\n    return $out;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Unary negation/not and a comparison operator, over a `uint`-defaulted
    /// bare literal (ADR 0007 § 4) — exercises the operators the arithmetic
    /// test above doesn't.
    #[test]
    fn unary_and_comparison_operators() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function check(int $n): bool {\n    bool $neg = -$n < 0;\n    return !$neg;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `uint` local initialized from a bare integer literal takes the
    /// literal as `uint`, not `int` — ADR 0007 § 4's target-directed rule,
    /// mirrored from `mwl_types::expr::infer`.
    #[test]
    fn a_bare_literal_targeting_uint_is_lowered_as_uint() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): uint {\n    uint $n = 1;\n    return $n;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    #[test]
    #[should_panic(expected = "known gaps")]
    fn control_flow_is_out_of_scope_for_this_slice() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(): int {\n    if (true) { return 1; }\n    return 0;\n  }\n}\n",
        );
    }
}
