//! `Core\Issue` — [ADR 0071](/docs/adr/0071-derived-codecs.md)
//! § 5's one shape, and the `array<Issue>` a failed decode carries.
//!
//! ```php
//! type Core\Issue = {path: string, message: string};
//! ```
//!
//! # A shape, not a class
//!
//! That is what the ADR writes, and it is the cheaper of the two: a shape needs
//! no registry row, no member table and no name a program has to import, and
//! `rule:types/shape-type` already
//! makes `{path: string, message: string}` a type the checker compares
//! structurally. So `Core\Issue` exists as a *type* in
//! `nvs_types::error_lib::issue_shape` and as a *layout* in
//! [`crate::instance`]'s shape roster, and nowhere else.
//!
//! **The slot order is the sorted field order** — `message`, then `path` —
//! because `nvs_types::ty::TypeInterner::shape` canonicalizes a shape's fields
//! by name. [`FIELDS`] is this side of that agreement.
//!
//! # What it spends
//!
//! One object allocation per issue plus one array for the list, and **only on
//! the failing path**: ADR 0071 § 5's "the accumulator is allocated only when
//! the first issue is recorded" is honoured by nothing here running until a
//! member actually calls [`list`]. A request that decodes successfully
//! allocates none of it.
//!
//! # Reading one back
//!
//! `$issue->path` is an ordinary field read: the checker resolves the name to
//! its slot in the sorted list above and `nvs-ir` reads that slot by index, so
//! the agreement [`FIELDS`] states is the only thing keeping the two sides
//! together. Reorder it and a program reads the wrong field rather than
//! failing to compile — which is why the order is stated here, in
//! `nvs_types::error_lib::issue_shape`'s interned type, and nowhere else.

use nvs_runtime::{NvsArray, NvsStr, Value};

/// The descriptor name `crate::instance`'s shape roster registers this under.
///
/// Cosmetic — a shape value's class descriptor is never resolved by name — but
/// it is what a dump or a leak report prints, so it is the spelling the ADR
/// uses.
pub(crate) const SHAPE: &str = r"Core\Issue";

/// The shape's fields **in slot order**, which is sorted field-name order; see
/// this module's docs.
pub(crate) const FIELDS: &[&str] = &["message", "path"];

/// One `{path, message}` value.
///
/// `path` is a dotted path into the payload — `"address.city"`, `"tags.3"` —
/// and is empty for a failure that is about the document rather than a field
/// in it, which is what a syntax error is.
pub(crate) fn one(path: &str, message: &str) -> Value {
    crate::instance::shape(
        SHAPE,
        [
            Value::str(NvsStr::new(message.as_bytes())),
            Value::str(NvsStr::new(path.as_bytes())),
        ],
    )
}

/// An `array<Issue>` of every `(path, message)` in `issues`, in the order
/// given — ADR 0071 § 5's "in declaration order".
pub(crate) fn list<'a>(issues: impl IntoIterator<Item = (&'a str, &'a str)>) -> Value {
    let mut array = NvsArray::new();
    for (path, message) in issues {
        array.append(one(path, message));
    }
    Value::array(array)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::Tag;

    #[test]
    fn an_issue_holds_its_two_fields_in_the_sorted_slot_order() {
        let value = one("address.city", "required field missing");
        let ptr = value.obj_ptr().expect("an issue is an object");
        let message = crate::instance::slot(ptr, 0);
        let path = crate::instance::slot(ptr, 1);
        assert_eq!(message.as_str_bytes(), Some(&b"required field missing"[..]));
        assert_eq!(path.as_str_bytes(), Some(&b"address.city"[..]));
        #[expect(unsafe_code, reason = "this frame owns the one reference `one` built")]
        unsafe {
            value.release();
        }
    }

    #[test]
    fn a_list_holds_one_entry_per_issue_in_order() {
        let value = list([("a", "first"), ("b", "second")]);
        assert_eq!(value.tag(), Some(Tag::Array));
        let ptr = value.array_ptr().expect("a list is an array");
        let array = crate::arr::borrowed(ptr);
        assert_eq!(array.count(), 2);
        #[expect(unsafe_code, reason = "this frame owns the one reference `list` built")]
        unsafe {
            value.release();
        }
    }
}
