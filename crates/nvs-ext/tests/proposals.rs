//! `rule:packaging/an-extension-is-a-sandboxed-wasm-component`'s proposal set: the host's engine
//! compiles every built-in component, and refuses a guest that uses a wasm proposal outside
//! `nvs_ext::call::PROPOSALS`, which a validator with every feature on accepts.

use nvs_ext::call::Host;
use nvs_ext::load::Loader;
use wasmparser::{Validator, WasmFeatures};
use wasmtime::component::Component;

/// The fields of one core module per proposal the host turns off, each using only that proposal.
const OUTSIDE: [(&str, &str); 9] = [
    ("tail calls", "(func $f (return_call $f))"),
    ("memory64", "(memory i64 1)"),
    ("multi-memory", "(memory 1) (memory 1)"),
    ("threads", "(memory 1 1 shared)"),
    ("GC", "(type (struct))"),
    ("exceptions", "(tag)"),
    ("externref", "(func (param externref))"),
    (
        "relaxed SIMD",
        "(func (param v128 v128 v128) (result v128)
            local.get 0 local.get 1 local.get 2 f32x4.relaxed_madd)",
    ),
    (
        "wide arithmetic",
        "(func (param i64 i64 i64 i64) (result i64 i64)
            local.get 0 local.get 1 local.get 2 local.get 3 i64.add128)",
    ),
];

#[test]
fn every_built_in_component_compiles_under_the_hosts_proposals() {
    let host = Host::new(1, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    assert_eq!(builtins.len(), 4);
    for builtin in &builtins {
        builtin
            .extension()
            .unwrap_or_else(|err| panic!("{} compiles: {err:?}", builtin.name()));
    }
}

#[test]
fn a_guest_using_a_proposal_outside_the_set_does_not_compile() {
    let host = Host::new(1, |_| Ok(())).expect("the host starts");
    for (proposal, fields) in OUTSIDE {
        let bytes = wat::parse_str(format!("(component (core module {fields}))"))
            .unwrap_or_else(|err| panic!("the {proposal} guest parses: {err}"));
        Validator::new_with_features(WasmFeatures::all())
            .validate_all(&bytes)
            .unwrap_or_else(|err| {
                panic!("a validator with every feature accepts {proposal}: {err}")
            });
        assert!(
            Component::new(host.engine(), &bytes).is_err(),
            "the host compiled a guest that uses {proposal}",
        );
    }
}
