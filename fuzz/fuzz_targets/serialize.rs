//! Fuzz target: `Core\Serialize::decode` must not panic on any payload, and
//! whatever it accepts must encode again to a payload that decodes and encodes
//! to the same bytes.
//!
//! The class table is the one a forged payload meets: `Owner` declares an
//! `int`, a `Dog` and an `array<Dog>` field, and `Cat` has the same shape as
//! `Dog`. So the fuzzer reaches the per-field type checks that keep a decoded
//! object from holding a value its slot does not admit
//! (`rule:classes/serialize-is-a-closed-format`), which is where a missed check
//! is a memory-safety hole rather than a wrong answer.
//!
//! Run with `cargo +nightly fuzz run serialize` (needs `cargo-fuzz`; libFuzzer
//! is not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_runtime::{ClassDesc, ClassTable, SlotCheck, Tag, decode, encode};

/// `Dog`, `Cat` and a typed `Owner`.
fn classes() -> ClassTable {
    let mut table = ClassTable::new();
    table.define("Dog", &["name"], &[]);
    table.define("Cat", &["name"], &[]);
    let owner = table.define("Owner", &["count", "pet", "pets"], &[]);
    table.set_field_tags(
        owner,
        vec![Some(Tag::Int), Some(Tag::Object), Some(Tag::Array)],
    );
    table.set_field_types(
        owner,
        vec!["int".into(), "Dog".into(), "array<Dog>".into()],
    );
    table.set_field_classes(owner, vec![None, Some(vec!["Dog".into()]), None]);
    table.set_field_checks(
        owner,
        vec![
            None,
            None,
            Some(SlotCheck::Array(Box::new(SlotCheck::Class("Dog".into())))),
        ],
    );
    table
}

/// Re-encodes `value`, consuming the caller's reference to it.
fn encoded(value: nvs_runtime::Value) -> Vec<u8> {
    encode(value).expect("a value `decode` built encodes again")
}

fuzz_target!(|data: &[u8]| {
    let table = classes();
    let resolve = |name: &str| -> Option<*const ClassDesc> {
        table.id_of(name).map(|id| table.desc(id))
    };
    let Ok(once) = decode(data, &resolve) else {
        return;
    };
    let first = encoded(once);
    let twice = decode(&first, &resolve).expect("a payload `encode` wrote decodes again");
    let second = encoded(twice);
    assert_eq!(first, second, "one round trip settles the payload");
});
