//! `nvs_codegen::compile_pieces` compiles only the functions whose key the
//! caller does not hold, and a function's piece names every target so that
//! the piece is the same wherever the function sits in the program.
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_codegen::pieces::{RelocKind, Target};

/// Two classes and a script that calls both. `body` is `A::first`'s return
/// expression, and `before` is a class declared ahead of both.
fn program(before: &str, body: &str) -> String {
    format!(
        "<?nvs
{before}
class A {{
    public static function first(int $n): int {{
        echo \"a\";
        return {body};
    }}
}}
class B {{
    public static function second(int $n): int {{
        if ($n > 2) {{
            echo \"b\";
        }}
        return A::first($n) * 3;
    }}
}}
echo A::first(1), B::second(3);
"
    )
}

/// A class on one line, so the functions after it keep their lines: a
/// backtrace prints them, so they are in a function's key and its literals.
const PADDING: &str = "class Pad { public static function grow(int $n): int { if ($n > 1) { echo \"1\"; } if ($n > 2) { echo \"2\"; } return $n; } }";

#[test]
fn a_function_whose_key_is_held_is_not_compiled() {
    let before = lower(&program("", "$n + 1"));
    let after = lower(&program("", "$n + 1 + $n * 7"));

    let all = nvs_codegen::compile_pieces(&before, &|_| false).expect("the fixture compiles");
    assert_eq!(
        all.keys,
        nvs_codegen::function_keys(&before).expect("the fixture emits"),
        "the pieces' keys are not the keys `function_keys` gives"
    );
    assert_eq!(
        all.compiled.len(),
        before.functions.len(),
        "a compile that holds no key did not compile every function"
    );
    for piece in &all.compiled {
        assert!(
            piece.relocs.iter().all(|reloc| reloc.kind != RelocKind::Other),
            "`{}` has a relocation kind no loader places",
            piece.name
        );
    }

    let held: Vec<[u8; 32]> = all.keys.iter().map(|(_, key)| *key).collect();
    let edited =
        nvs_codegen::compile_pieces(&after, &|key| held.contains(key)).expect("the edit compiles");
    let compiled: Vec<&str> = edited.compiled.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(compiled, ["A::first"], "a body edit compiled other functions");
    let none = nvs_codegen::compile_pieces(&before, &|key| held.contains(key))
        .expect("the fixture compiles");
    assert!(none.compiled.is_empty(), "an unchanged program compiled a function");
}

#[test]
fn a_piece_is_the_same_wherever_its_function_sits() {
    let near = lower(&program("", "$n + 1"));
    let far = lower(&program(PADDING, "$n + 1"));
    let near = nvs_codegen::compile_pieces(&near, &|_| false).expect("the near fixture compiles");
    let far = nvs_codegen::compile_pieces(&far, &|_| false).expect("the far fixture compiles");
    let second = |pieces: &nvs_codegen::pieces::Pieces| {
        pieces
            .compiled
            .iter()
            .find(|piece| piece.name == "B::second")
            .cloned()
            .expect("the fixture has `B::second`")
    };
    let (near_second, far_second) = (second(&near), second(&far));
    assert_eq!(
        near_second, far_second,
        "`B::second`'s piece changed when a class was declared before it"
    );
    // The piece calls `A::first` by its name, so the case reaches a target
    // whose symbol carries a program index.
    assert!(
        near_second
            .relocs
            .iter()
            .any(|reloc| reloc.target == Target::Function("A::first".to_owned())),
        "`B::second` does not name `A::first` as a function target"
    );
    let literals = near.literals_for(&[&near_second]);
    assert_eq!(
        literals,
        far.literals_for(&[&far_second]),
        "`B::second`'s literals changed when a class was declared before it"
    );
    assert!(!literals.is_empty(), "`B::second` reaches no literal");
}
