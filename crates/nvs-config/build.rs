//! Writes the BLAKE3 hash of `src/default.toml` into `OUT_DIR` as `default_hash.rs`, a string
//! literal of 64 lowercase hex characters, so `data::config_template`'s marker line is a constant
//! and no start hashes the shipped file. Then links integration tests the way every crate's do
//! (`tools/build/tests_without_pdb.rs`).

#![allow(
    clippy::print_stdout,
    reason = "`cargo:` directives on stdout are how a build script talks to Cargo"
)]

#[path = "../../tools/build/tests_without_pdb.rs"]
mod tests_without_pdb;

fn main() {
    println!("cargo:rerun-if-changed=src/default.toml");
    let text = std::fs::read("src/default.toml")
        .unwrap_or_else(|err| panic!("`src/default.toml` does not read: {err}"));
    let out = std::env::var_os("OUT_DIR").unwrap_or_else(|| panic!("cargo sets `OUT_DIR`"));
    let hash = blake3::hash(&text).to_hex();
    std::fs::write(
        std::path::Path::new(&out).join("default_hash.rs"),
        format!("\"{hash}\""),
    )
    .unwrap_or_else(|err| panic!("`default_hash.rs` does not write: {err}"));
    tests_without_pdb::main();
}
