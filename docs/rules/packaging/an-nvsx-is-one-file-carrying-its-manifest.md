A `.nvsx` is **one file**: a WebAssembly component implementing the versioned world `nvs:ext@1.0.0`,
with two custom sections inside it. There is no archive, no sidecar manifest, and no per-platform
variant — the same file loads on every host Novis runs on.

- **`nvs.manifest`** is JSON, `{"manifest": 1, ...}`: the one class the extension declares, the
  exported interface it maps, each export's Novis signature — parameter names, types, defaults, the
  trailing options shape — with its qualifier declarations
  (`rule:security/extension-manifest-only-tightens`), the class's `const` members, the settings block
  `[ext.<name>]` it reads, the I/O it requests (`rule:security/extension-grants-are-an-intersection`),
  the largest linear memory it needs, and each method's help text.
- **`nvs.source`** is the Novis source that builds on the class, as a flat list of
  `(relative path, bytes)`, every file under the extension's own namespace
  (`rule:packaging/an-extension-package-carries-two-payloads`).

At load the host checks the manifest against the component's actual WIT exports through
`rule:packaging/a-value-crosses-as-its-wit-type`'s table, and a mismatch refuses the load. Because both
sections travel inside the component, the `[[extension]]` entry's pin covers the code, the source and
every declaration with no separate hash — which is what lets the loaded set fold into every compiled
unit's key (`rule:config/the-extension-set-is-in-every-unit-key`).

The world is WIT and versioned by semver. A host implementing `1.y` loads a component built against
`1.x` for any `x ≤ y`, a minor version only adds imports and types, and a component built against a
newer minor or another major is refused, naming both versions. An `.nvsx` compiled against
`nvs:ext@1.0.0` is not recompiled for a minor engine release.

**Not on disk.** There is no world file, manifest reader or component loader in the tree.
