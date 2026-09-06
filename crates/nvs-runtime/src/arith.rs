//! The arithmetic rows compiled code cannot spell as a machine instruction.
//!
//! Every other operator in
//! `rule:types/arithmetic`'s table is
//! one or two Cranelift instructions, so `nvs-codegen` emits it inline and
//! this module does not exist for it. `**` over two `float`s is the exception:
//! there is no `fpow` instruction on any target Cranelift supports and no
//! `LibCall::Pow` to defer to either, so the row has to be a call to something,
//! and the something is here.
//!
//! It is **not** an `nvs_ir::Helper`. `rule:errors/propagation`'s helper convention is
//! `(ctx, args, out) -> status` over a stack slot of tagged [`crate::Value`]s,
//! which would spend two tagged stores, a slot read-back and a status branch on
//! an operation that cannot fail and whose operand representation is already
//! statically known at the emit site. The direct two-`f64` call below is the
//! same choice — and for the same reason — that
//! [`crate::nvs_str_eq`] makes for a `string` pair.
//! `nvs_codegen::emit`'s own module doc records the decision.

/// `**` over two `float`s — `rule:types/arithmetic`'s "either operand a `float`" row.
///
/// Exactly [`f64::powf`], which is IEEE 754's `pow` and therefore PHP's own
/// answer: PHP's `**` on two floats is C's `pow`, including its edge cases
/// (`0.0 ** -1.0` is `INF`, `(-8.0) ** (1.0/3.0)` is `NaN`, `x ** 0.0` is `1.0`
/// for every `x` including `NaN`). Total — it raises nothing and returns no
/// status — which is why it is a plain call and not a helper.
#[expect(
    unsafe_code,
    reason = "compiled code calls this by symbol name, so it cannot be mangled"
)]
#[unsafe(no_mangle)]
pub extern "C" fn nvs_float_pow(base: f64, exponent: f64) -> f64 {
    base.powf(exponent)
}
