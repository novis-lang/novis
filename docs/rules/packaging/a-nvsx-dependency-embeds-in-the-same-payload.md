A Tier 1 extension is one wasm component, portable across every target Novis ships for.
`nvs build --compile` embeds each `.nvsx` the build's configuration lists, with its pin, as an opaque
entry in the same flat list as the source (`rule:packaging/a-bundle-carries-source-not-artifacts`).
There is no target-matrix problem, because a `.nvsx` never had one. The built-in components travel
inside the host binary the bundle copies, so a bundle carries them with no entry at all
(`rule:packaging/the-first-party-components-are-built-in`).

An extension embedded in a bundle behaves identically to the same extension loaded from an
`[[extension]]` entry for a plain `nvs run`: it is the same component, the same manifest and the same
pin, read from a different byte source.

An embedded entry keeps its `grants` as written, each root made absolute at build time, and loads
before the run's own `[[extension]]` entries, so a class both declare refuses the start. Each embedded
pin is checked when the bundle builds and again when it starts.

**Not on disk.** The built-in components do not exist yet. `nvs build --compile` embeds each listed
`.nvsx` and its entry, and the bundle loads them from its payload with no file beside it
(`crates/nvs-cli/tests/bundle.rs`; the layout is in `crates/nvs-cli/src/bundle.rs`'s module doc).
