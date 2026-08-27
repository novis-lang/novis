//! Workspace-manifest settings that are a safety policy rather than a tuning
//! choice.
//!
//! A profile flag leaves no trace a test could read back — `cargo test` builds
//! the dev profile, so a release-only setting is invisible from inside the
//! process — and the setting itself lives in a file no crate compiles. So the
//! policy is pinned at its source, exactly as `nvs-codegen`'s `backend_policy`
//! pins the Cranelift flags. Crude, and the only thing that actually fails when
//! the line is deleted.
//!
//! This lives in `nvs-runtime` because this crate is where a wrapped size
//! computation does its damage: every allocation length, refcount and index the
//! heap representation computes passes through here.

use std::fs;
use std::path::Path;

/// The workspace manifest's `[profile.release]` block, comments stripped.
///
/// Stripping the comments is what keeps the assertion honest: the block's own
/// prose names `overflow-checks` several times while explaining why it is on,
/// so a search over the raw text would still pass with the setting deleted.
fn release_profile() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));

    let mut lines = text
        .lines()
        .skip_while(|line| line.trim() != "[profile.release]");
    assert!(
        lines.next().is_some(),
        "the workspace manifest has no `[profile.release]` section at all"
    );

    lines
        .take_while(|line| !line.trim_start().starts_with('['))
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_release_profile_checks_integer_overflow() {
    // Cargo's own default for a release profile is `overflow-checks = false`,
    // so this is a setting that has to be present rather than one that has to
    // be absent — deleting the line reverts to silent wraparound with nothing
    // reporting it. What it buys, and the measurement saying it costs nothing,
    // are in the manifest comment sitting directly above the line.
    assert!(
        release_profile()
            .lines()
            .any(|line| line.trim() == "overflow-checks = true"),
        "`[profile.release]` no longer sets `overflow-checks = true`. Cargo's default for a \
         release profile is off, so a size or index computation that wraps becomes a wrong \
         length nothing reports rather than a panic ADR 0002 contains to one request. If this \
         is deliberate, the reasoning belongs beside the setting in the workspace manifest."
    );
}
