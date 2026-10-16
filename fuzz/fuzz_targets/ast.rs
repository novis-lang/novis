//! Fuzz target: `Core\Ast::parse`'s own door must not panic on any input, and
//! every production it answers with must be one the typed roster has a class
//! for.
//!
//! The door is `nvs_syntax::walk::of_source`, which is the whole of the
//! member's body past the argument's tag check, so this fuzzes the member
//! without linking `nvs-stdlib` or standing a runtime context up. What it adds
//! over the `parse` target beside it is the walk: `parse` stops when the
//! parser returns, and a production the walk names wrongly — or names and the
//! roster does not — is invisible to it. `Core\Ast`'s `class_of` is a binary
//! search over `nvs_syntax::walk::KINDS` at the walk's own index, so a kind
//! absent from that table is a node handed back as another production's class.
//!
//! The seeds are `fuzz/seeds/parse/`, shared with the `parse` target because
//! both read one source text. `crates/nvs-stdlib/src/ast.rs`'s
//! `core_ast_parse_gives_the_compilers_verdict_on_every_parse_seed` replays
//! that same corpus on stable, which is where the seeds are asserted about at
//! all; this target is the unbounded half.
//!
//! Run with `cargo +nightly fuzz run ast` (needs `cargo-fuzz`; libFuzzer is
//! not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_syntax::walk::{self, KINDS, Node};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // A refusal is the other half of the member's contract and is checked on
    // the corpus by the stable replay; here it is simply not a tree to walk.
    let Ok(tree) = walk::of_source("fuzz.nvs", text) else {
        return;
    };
    every_kind_is_a_production(&tree);
});

/// Asserts that `node` and everything under it names a kind [`KINDS`] holds.
///
/// Recursive over a tree whose depth the parser bounds itself
/// (`nvs_syntax::parser`'s nesting limit), which is why a corpus input cannot
/// make this the thing that overflows.
fn every_kind_is_a_production(node: &Node) {
    assert!(
        KINDS.binary_search(&node.kind).is_ok(),
        "the walk answered with `{}`, which `KINDS` does not name — \
         `Core\\Ast` would give that node another production's class",
        node.kind
    );
    for child in &node.children {
        every_kind_is_a_production(child);
    }
}
