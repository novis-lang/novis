//! What compiled code spends on a revocable object: the inline release of an
//! object keeps its one compare, whose bound is the revoked flag, so an object
//! that is not revoked runs the instructions it ran before. The tests read the
//! Cranelift text, as `refcount_calls.rs` does, and none reads a clock
//! (`rule:testing/bench-counters`).
// covers: lang:classes/revocable-objects

mod common;

use common::*;

/// One function's Cranelift IR out of the text `nvs_codegen::clif` renders,
/// which heads each function with `; <name>`.
fn section<'a>(text: &'a str, name: &str) -> &'a str {
    let head = format!("; {name}\n");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("the unit defines no `{name}`:\n{text}"))
        + head.len();
    let body = &text[start..];
    match body.find("\n; ") {
        Some(end) => &body[..end],
        None => body,
    }
}

/// `drop` releases the object in its typed local, which is not a literal, so
/// the release is inline.
const PLAIN: &str = r#"<?nvs
class Box {
    public int $n = 1;
}
class K {
    public static function drop(): int {
        Box $b = new Box();
        return $b->n;
    }
}
echo K::drop();
"#;

#[test]
fn the_inline_release_of_an_object_that_is_not_revoked_is_unchanged() {
    let program = lower(PLAIN);
    let text = nvs_codegen::clif(&program).expect("the unit compiled");
    let clif = section(&text, "K::drop");
    // The bound is the revoked flag less two: counts from 2 up to one below
    // the flag are decremented inline.
    let bound = format!("{:#x}", nvs_runtime::revoke::REVOKED - 2);
    let compares: Vec<&str> = clif
        .lines()
        .filter(|line| line.contains("icmp ult"))
        .collect();
    assert!(
        compares.iter().any(|line| line.replace('_', "").contains(&bound)),
        "`K::drop`'s release compares against {bound}:\n{clif}"
    );
    // The flag costs no instruction: nothing masks the count inline.
    assert!(
        !clif.contains("band_imm"),
        "`K::drop` masks a count inline:\n{clif}"
    );
}
