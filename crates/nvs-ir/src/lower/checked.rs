//! The checked build: with [`check_types`] on, the lowering tests each typed
//! read of a value that carries its type at run time, and a value that fails
//! stops the process.
//!
//! The reads are a property read, a call's result, an array element, a
//! `static::` constant read and a method's parameter at entry. Each is tested
//! against the type the checker gave it, by the same test `is` lowers to
//! ([`super::expr`]'s `test_shape`). A test that fails calls
//! [`Helper::TypeCheckFailed`], whose `FATAL` names the line, the declared type
//! and what the value is. No `catch` sees a `FATAL`, which is right: a value
//! that is not its declared type is a compiler bug, never a program's error.
//!
//! **Only what carries its type is tested**: an object's class, a union's tag
//! and a `null` where the type has none. A value lowered at [`Ty::Int`],
//! [`Ty::Float`] or any other single representation carries no tag, so a scalar
//! read as another scalar is `bun nv soundness`'s to find. An array is tested
//! for its tag and not walked, and a shape for being an object, so one test
//! costs one comparison or one descriptor walk.
//!
//! With the switch off, [`Lowering::check_read`] and
//! [`Lowering::emit_type_check`] return before they emit anything, so a unit
//! has not one instruction more. The switch is process-wide and one-way, like
//! `nvs_codegen::set_opt_level`, and for the same reason: the run is the
//! evidence, and a unit compiled the other way partway through it would make
//! it evidence about neither. A caller that sets it keeps the artifact cache
//! off, since an artifact's key does not cover it.

use std::sync::atomic::{AtomicBool, Ordering};

use super::expr::{TestShape, test_shape};
use super::*;

/// Whether every unit lowered from now on carries the checked build's tests.
/// Set only through [`check_types`].
static CHECK_TYPES: AtomicBool = AtomicBool::new(false);

/// Lowers every unit this process builds after the call with a type test after
/// each typed read. `NOVIS_CHECK_TYPES` calls it, through
/// `nvs_codegen::compile_checked_types`.
pub fn check_types() {
    CHECK_TYPES.store(true, Ordering::Relaxed);
}

/// Whether [`check_types`] has been called in this process.
#[must_use]
pub fn is_checking_types() -> bool {
    CHECK_TYPES.load(Ordering::Relaxed)
}

impl Lowering<'_> {
    /// Tests `read`, the value [`Self::lower_expr`] lowered for `expr`, against
    /// the type the checker recorded for it, when `expr` is one of the reads
    /// this module's doc lists. A read in a chain with a `?->` in it may also
    /// be `null`, since the chain gives `null` when a link is `null`, and so
    /// may a subscript the checker guarded for absence (`??`, `isset`) and a
    /// `??=` target.
    pub(crate) fn check_read(
        &mut self,
        expr: &Expr,
        read: (ValueId, Ty),
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        if !is_checking_types() {
            return;
        }
        let (declared, mut nullable) = match self.exprs.lookup(expr.span) {
            Some(ExprInfo::Property { ty, narrowed, .. }) => (narrowed.unwrap_or(*ty), false),
            Some(
                ExprInfo::HookedProperty { ty, .. }
                | ExprInfo::ClassConstLate { ty, .. }
                | ExprInfo::StaticProperty { ty, .. },
            ) => (*ty, false),
            Some(ExprInfo::Index { elem_ty, guarded }) => (*elem_ty, *guarded),
            Some(ExprInfo::Call(call) | ExprInfo::ClassRefCall(call)) => (call.return_ty, false),
            _ => return,
        };
        // `??=` and the defaulting operators read their target as `??` does.
        nullable |= self.reading_guarded_target || in_nullsafe_chain(expr);
        self.emit_type_check(
            read,
            declared,
            nullable,
            expr.span,
            "the value read here",
            env,
            cur,
        );
    }

    /// The test itself: `value` against `declared`, or `null` too when
    /// `nullable`. `what` names the read in the message. `cur` moves to the
    /// block where the test passed.
    #[expect(
        clippy::too_many_arguments,
        reason = "the value and its type, the read's own facts, and the env and \
                  block every emitted test threads"
    )]
    pub(crate) fn emit_type_check(
        &mut self,
        (value, ty): (ValueId, Ty),
        declared: TypeId,
        nullable: bool,
        span: Span,
        what: &str,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        if !is_checking_types() || !matches!(ty, Ty::Object | Ty::Tagged) {
            return;
        }
        let Some(shape) = test_shape(declared, self.exprs, self.checked_types, self.enums) else {
            return;
        };
        let shape = match nullable {
            true => TestShape::Any(vec![shallow(shape), TestShape::Tag(Ty::Null)]),
            false => shallow(shape),
        };
        let (holds, _) = self.emit_test_shape(shape, value, ty, span, env, cur);
        let failed = self.new_block();
        let held = self.new_block();
        let held_edge = self.ids.next_edge(span);
        let failed_edge = self.ids.next_edge(span);
        self.seal(
            *cur,
            Terminator::Branch {
                cond: holds,
                then_block: held,
                then_edge: held_edge,
                else_block: failed,
                else_edge: failed_edge,
            },
        );
        let (line, _) = self.src.line_col(span.start);
        let message = format!(
            "the checked build stopped at {}:{}: {what} is declared `{}`",
            self.src.name(),
            line + 1,
            self.checked_types.describe(declared)
        );
        let (message, _) = self.emit(failed, Ty::Str, InstKind::ConstStr(message));
        let tagged = match ty {
            Ty::Tagged => value,
            _ => self.coerce(failed, value, ty, Ty::Tagged, env),
        };
        self.emit_fallible(
            failed,
            Ty::Tagged,
            InstKind::HelperCall {
                helper: Helper::TypeCheckFailed,
                args: vec![message, tagged],
            },
            env,
        );
        // The helper never returns success, so this edge is never taken. It
        // seals the block the way every block is sealed.
        self.seal(failed, Terminator::Jump(held));
        *cur = held;
    }
}

/// `shape` with every walk replaced by the test that starts it: an array's
/// elements are not read, and neither are a shape's fields. A class row
/// becomes [`TestShape::IfDeclared`], so a class only an extension declares
/// does not fail the unit.
fn shallow(shape: TestShape) -> TestShape {
    match shape {
        TestShape::Class(name) => TestShape::IfDeclared(name),
        TestShape::ArrayOf(_) | TestShape::Every(_) => TestShape::Tag(Ty::Array),
        TestShape::Field { .. } => TestShape::All(Vec::new()),
        TestShape::Any(rows) => TestShape::Any(rows.into_iter().map(shallow).collect()),
        TestShape::All(rows) => TestShape::All(rows.into_iter().map(shallow).collect()),
        other => other,
    }
}

/// Whether a `?->` is written anywhere in the member chain `expr` ends.
fn in_nullsafe_chain(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::MethodCall {
            object, nullsafe, ..
        }
        | ExprKind::PropertyAccess {
            object, nullsafe, ..
        } => *nullsafe || in_nullsafe_chain(object),
        ExprKind::Index { base, .. } => in_nullsafe_chain(base),
        _ => false,
    }
}
