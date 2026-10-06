Every parameter and return of an extension's export has a Novis type, and one closed table gives its
WIT type. A value is copied whole in each direction by the canonical ABI, so an author receives native
types from `wit-bindgen` and writes no marshalling code.

| Novis | WIT |
|---|---|
| `bool`, `int`, `uint`, `float` | `bool`, `s64`, `u64`, `f64` |
| `string` | `string` — UTF-8 both ways, and invalid UTF-8 from a guest traps |
| `bytes` | `list<u8>` |
| `array<T>` (a list) | `list<T>` |
| `array<K, V>` | `list<tuple<K, V>>`, in order |
| a shape `{a: T, b?: U}` | `record { a: T, b: option<U> }` |
| an enum | `enum`, its cases in kebab-case |
| `?T` | `option<T>` |
| a closed union of shapes | `variant`, one case per shape |
| a `Core` value class | the record `nvs:ext/types` defines for it |
| `resource` | a resource the extension exports, alive until the request ends |
| `mixed` | `borrow<value>` (`rule:packaging/values-cross-as-handles`) |

A type not in the table cannot appear in an extension signature, and a manifest naming one does not
load. Every export returns `result<T, error>` on the WIT side, and the error throws the class
`rule:packaging/a-guest-crash-throws` maps it to. `uint` is `u64` with no conversion, the same
parametric signatures the built-ins use.

What it costs is the copy: a guest that needs one field of a large array still receives the array,
at about 12 ns per KiB. Coarse APIs are the rule already (`rule:packaging/the-boundary-is-the-cost`),
and `mixed` keeps the pull-only path for the case that needs it.

**Not on disk.** The world is written: `wit/nvs-ext/types.wit` holds the `error` variant, the `value`
resource and one record per `Core` value class that crosses, and `crates/nvs-stdlib/tests/ext_world.rs`
names every other value class with the reason it cannot. A manifest declares its enums, closed
unions of shapes and resources by name, the loader holds each against the export's `enum`,
`variant`, record or resource, and `nvs_ext::convert` converts every row to its WIT value and back,
`mixed` by lending it as a handle and a resource as a number the request keeps, which
`nvs_ext::call::Request::end` drops (`crates/nvs-ext/tests/convert.rs`). `nvs run` crosses every
scalar, `bytes`, a list, a keyed array, `?T` and a shape both ways
(`tests/conformance/ext/an-extension-call-crosses-every-row-of-the-value-table.nvst`). The checker
does not type a method that names an extension's enum or resource, and a returned enum, `Core`
value class or resource does not cross back yet.
