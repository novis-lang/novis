A `.nvsx` is **one file**: a WebAssembly component implementing the versioned world `nvs:ext@1.0.0`,
with an `nvs.manifest` custom section inside it. There is no archive, no sidecar manifest, and no
per-platform variant — the same file loads on every host Novis runs on.

The manifest is what the host reads at load: the classes the extension declares, with their `static`
methods and `const` members; the `nvs.toml` directives it wants; and the qualifier declarations of
`rule:security/extension-manifest-only-tightens`. Because the manifest travels inside the component,
hashing an `[[extension]]` entry's pin covers everything the extension declares with no separate
manifest hash — which is what lets the loaded set fold into every compiled unit's key
(`rule:config/the-extension-set-is-in-every-unit-key`).

The world is WIT, so an extension gets rich types — records, variants, lists, strings, results,
resources — rather than everything marshalled through `i32`, and semantic versioning of the interface
is part of the contract. A PHP extension must be recompiled for every minor engine release; an
`.nvsx` compiled against `nvs:ext@1.0.0` is not.
