//! The codes of the IR and code generation, `E05xx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E05xx` fails the
//! crate's tests.

codes! {
    range = "E05";

    // `E0501` is retired and is never reused: the IR has no verifier of its
    // own, and Cranelift's verifier rejection is a `CodegenError` like any other.
    // `E0502` is retired and is never reused: a `CodegenError` is an internal
    // error, and `E0901` is the one code for an internal error.
}
