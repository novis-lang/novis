At load the host reads an extension's manifest — its declared class, its `static` methods and `const`
members — and registers the class into a layer of the compiler's signature table that the loaded set
writes. There is no function- or constant-shaped registration: an extension follows the same
class-only shape `rule:classes/no-free-functions-or-constants` requires of user code, and it may not
register under `Core\` (`rule:core-api/core-means-always-present`).

The registered methods carry the manifest's qualifier declarations in the same signature fields a
`Core` member's carry, so the call site is checked by the same code with no extension-specific branch
(`rule:security/extension-manifest-only-tightens`). `nvs check`, `nvs test` and the language server
read the manifests of the resolved configuration's set and never instantiate a component.

Consequently `nvs check` type-checks a call into an extension at compile time, and codegen emits a
**direct call** to a per-export trampoline that converts the arguments by
`rule:packaging/a-value-crosses-as-its-wit-type`'s table, rather than a dynamic dispatch. PHP can do
neither.

The direct call is why the loaded extension set is a codegen input: an artifact compiled against one set
holds a jump into a trampoline that another set may have moved, so the set is part of every compiled
unit's key and a changed set is an ordinary cache miss
(`rule:config/the-extension-set-is-in-every-unit-key`).

**Not on disk.** `Core` classes are a constant table seeded into the signature table; nothing
registers a class a configuration names.
