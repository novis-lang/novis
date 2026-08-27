//! The one script frame a file's top level shares, and what is not visible across it.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// ADR 0008 § 2: a file's top-level statements are one synthesized frame
// whose variables are locals. Each of these has an exact counterpart in
// the `check_in_method` fixtures above — the point is that the two
// report identically.

#[test]
fn reading_an_undeclared_local_at_file_scope_is_diagnosed() {
    let diags = check_src("<?nvs\necho $missing;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );
}

#[test]
fn a_declared_and_assigned_file_scope_local_reads_fine() {
    let diags = check_src("<?nvs\nint $n = 1;\n$n = $n + 1;\necho $n;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn redeclaring_a_file_scope_local_is_diagnosed() {
    let diags = check_src("<?nvs\nint $n = 1;\nint $n = 2;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_REDECLARED_LOCAL)),
        "{diags:?}"
    );
}

#[test]
fn a_file_scope_type_mismatch_is_diagnosed() {
    let diags = check_src("<?nvs\nint $n = 1;\n$n = \"x\";\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// `$this` has no meaning at file scope — there is no enclosing class,
/// so it is an ordinary undeclared name rather than a special case.
#[test]
fn this_at_file_scope_is_an_undeclared_local() {
    let diags = check_src("<?nvs\necho $this;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );
}

/// A `namespace { ... }` block scopes *names*, not storage: its top-level
/// statements land in the same synthesized frame as every other one in
/// the file, so a redeclaration across two blocks still conflicts.
#[test]
fn a_namespace_block_shares_the_one_script_frame() {
    let diags = check_src("<?nvs\nnamespace A { int $n = 1; }\nnamespace B { int $n = 2; }\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_REDECLARED_LOCAL)),
        "{diags:?}"
    );
}

/// A class body is still its own frame — a file-scope local is not
/// visible from inside a method (ADR 0008's storage-class table: "the
/// script's own frame, unreachable from a function").
#[test]
fn a_file_scope_local_is_not_visible_inside_a_method() {
    let diags = check_src("<?nvs\nint $n = 1;\nclass T {\n  function m(): void { echo $n; }\n}\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );
}
