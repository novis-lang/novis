A Tier 1 extension is one wasm component, portable across every target Novis ships for. If the
program's `require` graph depends on one, `nvs build --compile` embeds the `.nvsx` file itself as an
opaque entry in the same flat list as the source
(`rule:packaging/a-bundle-carries-source-not-artifacts`). There is no target-matrix problem, because a
`.nvsx` never had one.

An extension embedded in a bundle behaves identically to the same extension loaded from an
`[[extension]]` entry for a plain `nvs run`: it is the same component, the same manifest and the same
pin, read from a different byte source.
