At load the host reads an extension's manifest — its declared classes, their `static` methods and
`const` members, and any `nvs.toml` directives it contributes — and registers them into the compiler's
symbol table. There is no function- or constant-shaped registration: an extension follows the same
class-only shape `rule:classes/no-free-functions-or-constants` requires of user code, and it may not
register under `Core\` (`rule:core-api/core-means-always-present`).

Consequently `nvs check` type-checks a call into an extension at compile time, and codegen emits a
**direct call** to the extension's trampoline rather than a dynamic dispatch. PHP can do neither.

The direct call is why the loaded extension set is a codegen input: an artifact compiled against one set
holds a jump into a trampoline that another set may have moved, so the set is part of every compiled
unit's key and a changed set is an ordinary cache miss
(`rule:config/the-extension-set-is-in-every-unit-key`).
