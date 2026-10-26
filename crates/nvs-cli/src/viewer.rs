//! The debug viewer's front end, embedded in the binary.
//!
//! `bun nv viewer` builds `viewer/src/` into two files under `viewer/dist/`,
//! and both are committed, so a Rust build never runs Bun. `bun nv viewer
//! --check` fails when they are not the build of the committed source. They
//! are embedded here the way `info.rs` embeds the third-party notice, which
//! reproduces the licence of every package the bundle carries
//! (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`).
//!
//! `nvs debug` serves them to its own page
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//! Both are read-only data in every binary, and a server process never reads
//! them, so no page of them is touched outside `nvs debug`.

/// The viewer's JavaScript: one ES module that mounts the app into `#app`.
pub(crate) const JS: &[u8] = include_bytes!("../../../viewer/dist/viewer.js");

/// The viewer's stylesheet.
pub(crate) const CSS: &[u8] = include_bytes!("../../../viewer/dist/viewer.css");

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The embedded bytes are the committed files, so a build carries the
    /// bundle `bun nv viewer --check` held to the source.
    #[test]
    fn the_embedded_files_are_the_committed_ones() {
        let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../viewer/dist");
        for (name, embedded) in [("viewer.js", JS), ("viewer.css", CSS)] {
            let committed =
                std::fs::read(dist.join(name)).expect("the committed bundle is on disk");
            assert!(!embedded.is_empty(), "{name} is empty");
            assert!(
                committed == embedded,
                "the embedded {name} is not the committed file"
            );
        }
    }
}
