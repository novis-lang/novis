//! The two spellings that mention `Core\Script\Finished`, the class
//! `Core\Script::finish()` raises, and the diagnostic each one gets.
//!
//! The marker is a root of its own rather than a `Throwable`
//! (`nvs_hir::errors::TREE`'s own docs), which is what keeps every `catch` arm
//! false against it at run time. Both refusals here are that one fact read at
//! the two ends a program can reach: an arm that names it would match nothing,
//! and a `throw` of it would have nothing to be caught by.

mod common;

use common::check_src;
use nvs_diagnostics::{Diagnostics, code};

/// Whether `diags` carries the refusal a `catch` naming the marker gets.
fn refuses_the_arm(diags: &Diagnostics) -> bool {
    diags
        .iter()
        .any(|d| d.code == Some(code::E_CATCH_ARM_NAMES_THE_FINISH_MARKER))
}

/// A `catch` naming the marker is refused in every place one can be written:
/// the block form, the block form binding nothing, and the expression form.
///
/// All three, because each reaches the check by a different route — a clause
/// that binds nothing lowered no type at all before this refusal needed one,
/// and the expression arm is a second call site in a second module. A refusal
/// landing on only the spelling anybody writes first would leave the others
/// matching nothing, silently, which is the reading this diagnostic exists to
/// stop. A clause naming the marker *and* another class is
/// `E_CATCH_UNION_TYPE_UNSUPPORTED` before it is anything this owns.
#[test]
fn a_catch_arm_naming_the_finish_marker_is_a_diagnostic() {
    let bound = check_src(
        "<?nvs\ntry {\n  echo \"a\\n\";\n} catch (Core\\Script\\Finished $done) {\n  \
         echo \"b\\n\";\n}\n",
    );
    assert!(refuses_the_arm(&bound), "{bound:?}");

    let bare = check_src(
        "<?nvs\ntry {\n  echo \"a\\n\";\n} catch (Core\\Script\\Finished) {\n  echo \"b\\n\";\n}\n",
    );
    assert!(refuses_the_arm(&bare), "{bare:?}");

    let arm = check_src(
        "<?nvs\nint $n = 1 catch (Core\\Script\\Finished $done) => 0;\necho $n, \"\\n\";\n",
    );
    assert!(refuses_the_arm(&arm), "{arm:?}");

    // The control: the refusal is about the one class and not about `catch`.
    let ordinary = check_src(
        "<?nvs\ntry {\n  echo \"a\\n\";\n} catch (RuntimeError $e) {\n  echo \"b\\n\";\n}\n",
    );
    assert!(!ordinary.has_errors(), "{ordinary:?}");
}

/// A `throw` of the marker is refused as an operand outside spec § 10's tree,
/// which is `E0780` and needs no case of its own.
///
/// Pinned here rather than left implied: the marker's whole mechanism is that
/// it descends from `Throwable` not at all, so the day it is given a parent to
/// make some other site tidier this test fails beside the arm's — and a
/// program could otherwise raise the ending the host classifies by, from
/// anywhere, under a `throw` no `catch` admits.
#[test]
fn a_throw_of_the_finish_marker_is_a_diagnostic() {
    let diags = check_src("<?nvs\nthrow new Core\\Script\\Finished();\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_THROW_OPERAND_NOT_THROWABLE)),
        "{diags:?}"
    );
}
