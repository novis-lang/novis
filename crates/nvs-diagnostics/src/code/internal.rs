//! The codes for an internal compiler error, `E09xx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E09xx` fails the
//! crate's tests.

codes! {
    range = "E09";

    /// The compiler reached a state it believes impossible.
    pub const E_INTERNAL: Code = Code::new("E0901").card(
        "The compiler found an error in itself. This is a bug in Novis and not in your \
         program. Please report it, with the code that caused it.",
    );
}
