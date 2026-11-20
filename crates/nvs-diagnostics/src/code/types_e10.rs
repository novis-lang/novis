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
    /// `implements Revocable` on a class that has a parent class
    /// (`rule:classes/revocable-objects`). Only a root class opts in, so an
    /// access through a class type that is not revocable needs no check. The
    /// help names the root class, which is often in another file, so the code
    /// has no quick fix.
    pub const E_REVOCABLE_BELOW_ROOT: Code = Code::new("E1001").card(
        "Only a class with no parent class can implement `Revocable`. Every subclass of that \
         class is then revocable too. Write `implements Revocable` on the root class. The root \
         class is the class at the top of the hierarchy. Delete it from the subclass.",
    );
    /// An interface that extends `Revocable`
    /// (`rule:classes/revocable-objects`). An enum that implements it already
    /// takes `rule:enums/no-class-machinery`'s error, and gets no second one.
    pub const E_REVOCABLE_NOT_A_CLASS: Code = Code::new("E1002").card(
        "Only a class can implement `Revocable`. An interface cannot extend it. Write \
         `implements Revocable` on the root class of each class that should be revocable.",
    );
    /// A class that declares `revoke` or `isRevoked` while it is revocable
    /// (`rule:classes/revocable-objects`). The runtime runs both methods, and
    /// the one exception to `rule:classes/interface-default-methods`'
    /// override is that a class cannot replace them.
    pub const E_REVOCABLE_METHOD_DECLARED: Code = Code::new("E1003").card(
        "A revocable class cannot declare `revoke()` or `isRevoked()`. `Revocable` provides \
         both methods, and a class cannot replace them. Rename your method, or delete it and \
         use the one `Revocable` provides.",
    );
}
