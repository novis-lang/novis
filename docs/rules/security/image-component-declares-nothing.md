The image component declares **no qualifier deviation**: ordinary contagion is exactly right, so a
thumbnail of a `tainted` upload is `tainted` and reaches HTML output through the existing sinks like
any other value, while an image decoded from a trusted file comes back plain. `secret` never crosses
into it (`rule:security/secret-does-not-cross-an-extension`).

It requests **no capability**: it reads no file and opens no socket, and a program feeds it bytes it
obtained under its own grants. That is the shape a component should have — the effect stays on the
program's side of the boundary, where the grant already is
(`rule:security/capability-check-at-the-door`) — and it is recorded because "declares nothing" is a
claim worth being able to check rather than an absence nobody looked at.

`extensions/image/manifest.json` declares neither, and
`crates/nvs-ext/tests/manifest.rs`'s `the_image_manifest_declares_no_qualifier_and_no_capability`
reads it through the manifest parser and fails on any `source`, `secret` or `sink` mark and on any
entry under `requests`.
