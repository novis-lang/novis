//! The workspace's `wasmtime` row carries only the features the engine uses
//! (`rule:packaging/an-extension-is-a-sandboxed-wasm-component`). A feature that builds code for a
//! proposal `nvs_ext::call::PROPOSALS` turns off, or one nothing in the tree calls, is code a guest
//! could reach in the component whose job is isolation.

use nvs_ext::call::{Host, PROPOSALS};
use wasmtime::WasmFeatures;
use wasmtime::component::Component;

/// The features the engine uses, each for the reason the row's comment gives.
const KEPT: [&str; 7] = [
    "async",
    "component-model",
    "cranelift",
    "parallel-compilation",
    "pooling-allocator",
    "runtime",
    "std",
];

/// Each wasmtime feature that builds support for a wasm proposal, and that proposal.
const PROPOSAL_FEATURES: [(&str, WasmFeatures); 7] = [
    ("gc", WasmFeatures::GC),
    ("gc-drc", WasmFeatures::GC),
    ("gc-copying", WasmFeatures::GC),
    ("gc-null", WasmFeatures::GC),
    ("threads", WasmFeatures::THREADS),
    ("stack-switching", WasmFeatures::STACK_SWITCHING),
    ("component-model-async", WasmFeatures::CM_ASYNC),
];

/// The features the workspace `Cargo.toml` turns on for `wasmtime`.
fn row() -> Vec<String> {
    let text = std::fs::read_to_string(nvs_repo::path("Cargo.toml"))
        .expect("the workspace Cargo.toml reads");
    let manifest: toml::Table = text.parse().expect("the workspace Cargo.toml parses");
    let entry = &manifest["workspace"]["dependencies"]["wasmtime"];
    assert_eq!(
        entry.get("default-features").and_then(toml::Value::as_bool),
        Some(false),
        "the wasmtime row takes wasmtime's defaults",
    );
    entry["features"]
        .as_array()
        .expect("the wasmtime row lists its features")
        .iter()
        .map(|feature| feature.as_str().expect("a feature is a string").to_owned())
        .collect()
}

#[test]
fn wasmtime_builds_with_no_feature_the_engine_turns_off() {
    let features = row();
    for feature in &features {
        if let Some((_, proposal)) = PROPOSAL_FEATURES.iter().find(|(name, _)| name == feature) {
            assert!(
                PROPOSALS.contains(*proposal),
                "the wasmtime row turns on `{feature}`, and the engine turns that proposal off",
            );
        }
        assert!(
            KEPT.contains(&feature.as_str()),
            "the wasmtime row turns on `{feature}`, which nothing in the engine uses",
        );
    }
    for kept in KEPT {
        assert!(
            features.iter().any(|feature| feature == kept),
            "the wasmtime row lacks `{kept}`, which the engine uses",
        );
    }
}

#[test]
fn a_component_in_text_form_does_not_compile() {
    let host = Host::new(1, |_| Ok(())).expect("the host starts");
    let text = "(component (core module))";
    let binary = wat::parse_str(text).expect("the component text parses");
    Component::from_binary(host.engine(), &binary).expect("the component's binary compiles");
    assert!(
        Component::new(host.engine(), text.as_bytes()).is_err(),
        "the engine compiled a component in text form",
    );
}
