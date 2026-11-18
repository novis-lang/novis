//! The type checker's codes, continued from `types_e08.rs`: `E10xx`. The
//! `E08xx` range filled at `E0899`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E10xx` fails the
//! crate's tests.

codes! {
    range = "E10";

    /// A method's route attributes that do not pair:
    /// `#[Core\Router\Params]` with no `#[Core\Router\Target]` in the same
    /// method, or a second of either
    /// (`rule:routing/a-route-target-parameter-is-checked-like-a-link`). A
    /// `Params` array is checked against the route its `Target` names, so one
    /// with no `Target`, or with two to choose from, has nothing to be checked
    /// against.
    pub const E_ROUTE_ATTRIBUTES_DO_NOT_PAIR: Code = Code::new("E1000").card(
        "A method can have one `#[Core\\Router\\Target]` parameter and one \
         `#[Core\\Router\\Params]` parameter. The `Params` array is checked against the route \
         that the `Target` parameter gets, so a method with `Params` also needs a `Target`. \
         Delete the extra attribute, or add the missing `#[Core\\Router\\Target]`.",
    );
}
