//! The one script frame a file's top level shares, and what is not visible across it.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// `rule:statements/storage-that-outlives-a-call`: a file's top-level statements are one synthesized frame
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

/// `$this` has no meaning at file scope — there is no enclosing class, and
/// no receiver at all. It is `E0779` rather than the undeclared-name
/// diagnostic for the reason that code's own doc gives: nothing a program
/// writes declares `$this`, so a message about a missing declaration sends
/// the reader looking for one to add. A `static` method's body reaches the
/// same arm, which `tests/conformance/reject` pins.
#[test]
fn this_at_file_scope_has_no_receiver() {
    let diags = check_src("<?nvs\necho $this;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_THIS_WITHOUT_A_RECEIVER)),
        "{diags:?}"
    );
}

/// A `namespace { ... }` block is refused by the parser (`E0243`) and parsed
/// whole anyway, so this pass still sees what is inside it: a namespace scopes
/// *names*, not storage, and the two blocks' top-level statements land in the
/// same synthesized frame, where the redeclaration still conflicts. The file
/// reports both problems in one run, which is the point of parsing a refused
/// construct rather than abandoning it.
#[test]
fn a_namespace_block_is_refused_and_still_shares_the_one_script_frame() {
    let diags = check_src_allowing_parse_errors(
        "<?nvs\nnamespace A { int $n = 1; }\nnamespace B { int $n = 2; }\n",
    );
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_BRACED_NAMESPACE_UNSUPPORTED))
            .count(),
        2,
        "{diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_REDECLARED_LOCAL)),
        "{diags:?}"
    );
}

/// A class body is still its own frame — a file-scope local is not
/// visible from inside a method (`rule:statements/static-is-a-member-modifier`'s storage-class table: "the
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
