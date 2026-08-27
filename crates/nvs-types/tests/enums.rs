//! ADR 0010's closed integer enum: case access, conversion, and the arithmetic it refuses.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

// ADR 0010 § 4-5: an enum's name is a real, distinct type — a case
// access recovers `Ty::Enum`, not `mixed`, so it type-checks like any
// other declared type rather than accepting anything at all.

#[test]
fn an_enum_case_access_types_as_its_enum() {
    let diags = check_src(
        "<?mwl\nenum Status { Active, Banned }\nclass T {\n  function m(): void {\n    Status $s = Status::Active;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_enum_case_assigned_into_a_different_enums_local_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nenum Status { Active, Banned }\nenum Color { Red, Blue }\nclass T {\n  function m(): void {\n    Color $c = Status::Active;\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "`Status::Active` should type as `Status`, not `mixed`, so this must mismatch: {diags:?}"
    );
}

#[test]
fn arithmetic_directly_on_an_enum_case_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nenum Permission: uint { Read = 1, Write = 2 }\nclass T {\n  function m(): void {\n    Permission::Read + Permission::Write;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ENUM_ARITHMETIC_UNSUPPORTED)),
        "{diags:?}"
    );
}

#[test]
fn bitwise_or_directly_on_an_enum_case_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nenum Permission: uint { Read = 1, Write = 2 }\nclass T {\n  function m(): void {\n    Permission::Read | Permission::Write;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ENUM_ARITHMETIC_UNSUPPORTED)),
        "{diags:?}"
    );
}

#[test]
fn converting_an_enum_case_to_its_underlying_type_is_fine() {
    let diags = check_src(
        "<?mwl\nenum Permission: uint { Read = 1, Write = 2 }\nclass T {\n  function m(): void {\n    uint $bits = Permission::Write as uint;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn converting_one_enum_to_a_different_enum_via_as_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nenum Status { Active, Banned }\nenum Color { Red, Blue }\nclass T {\n  function m(): void {\n    Status::Active as Color;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ENUM_CONVERSION_UNSUPPORTED)),
        "{diags:?}"
    );
}

#[test]
fn converting_an_enum_to_itself_via_as_is_fine() {
    let diags = check_src(
        "<?mwl\nenum Status { Active, Banned }\nclass T {\n  function m(): void {\n    Status::Active as Status;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}
