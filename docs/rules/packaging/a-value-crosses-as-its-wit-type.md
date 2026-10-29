Every parameter and return of an extension's export has a Novis type, and one closed table gives its
WIT type. A value is copied whole in each direction by the canonical ABI, so an author receives native
types from `wit-bindgen` and writes no marshalling code. The one exception is a `bytes` in a
third-party export, which crosses as the host's `bytes` resource and is copied by its methods.

| Novis | WIT, a built-in world | WIT, a third-party export |
|---|---|---|
| `bool`, `int`, `uint`, `float` | `bool`, `s64`, `u64`, `f64` | the same |
| `string` | `string` — UTF-8 both ways, and invalid UTF-8 from a guest traps | the same |
| `bytes` | `list<u8>` | `borrow<bytes>` as a parameter, `own<bytes>` as a result |
| `array<T>` (a list) | `list<T>` | the same |
| `array<K, V>` | `list<tuple<K, V>>`, in order | the same |
| a shape `{a: T, b?: U}` | `record { a: T, b: option<U> }` | the same |
| an enum | `enum`, its cases in kebab-case | the same |
| `?T` | `option<T>` | the same |
| a closed union of shapes | `variant`, one case per shape | the same |
| a `Core` value class | the record `nvs:ext/types` defines for it | the same |
| `resource` | a resource the extension exports, alive until the request ends | the same |
| `mixed` | `borrow<value>` (`rule:packaging/values-cross-as-handles`) | the same |

A type not in the table cannot appear in an extension signature, and a manifest naming one does not
load. Every export returns `result<T, error>` on the WIT side, and the error throws the class
`rule:packaging/a-guest-crash-throws` maps it to. `uint` is `u64` with no conversion, the same
parametric signatures the built-ins use.

**A third-party `bytes` is the host's `bytes` resource**, at every position the manifest names it:
a parameter, a result, a list element, a shape field, an `option` and a union case. The host lends
an argument by number, and the guest copies it in with `read`, or a block at a time with `range`.
The guest returns a result as a `bytes` it made empty and filled with `append`, and the host takes
that buffer as the program's value without copying it again. Each method is a typed host function,
so each copies once. `read` and `range` are refused past the request's memory, and every `append`
is charged to the request before it copies, so a result past the budget ends the call at the memory
limit (`rule:packaging/a-guest-runs-under-the-requests-budget`). A third-party export whose WIT
has `list<u8>` where its manifest names `bytes` does not load, and the error names the resource.

This is because a third-party export is called on wasmtime's dynamic path, its types known only at
run time, and that path lifts and lowers a `list<u8>` as one `Val` per byte: about 122 host bytes
allocated for each byte crossed (`rule:packaging/the-boundary-is-the-cost`). A built-in world is
called typed, so its `list<u8>` already crosses in one copy, and it keeps the list. A `Core` record
keeps its byte field as `list<u8>` in both kinds, because `big-int`'s magnitude and `public-key`'s
`spki` are small by construction and a record is copied whole.

What it costs is the copy: a guest that needs one field of a large array still receives the array,
at about 12 ns per KiB. Coarse APIs are the rule already (`rule:packaging/the-boundary-is-the-cost`),
and `mixed` keeps the pull-only path for the case that needs it. A third-party `bytes` costs one
handle table entry while it is in flight, and a small one pays two or three typed host calls, about
30 ns. A result's buffer is charged to its request until the call returns.

**On disk.** The world is written: `wit/nvs-ext/types.wit` holds the `error` variant, the `value`
and `bytes` resources and one record per `Core` value class that crosses, and
`crates/nvs-stdlib/tests/ext_world.rs` names every other value class with the reason it cannot. A
manifest declares its enums, closed unions of shapes and resources by name, the loader holds each
against the export's `enum`, `variant`, record or resource, and `nvs_ext::convert` converts every
row to its WIT value and back, `mixed` by lending it as a handle and a resource as a number the
request keeps, which `nvs_ext::call::Request::end` drops (`crates/nvs-ext/tests/convert.rs`). A
third-party `bytes` is lent and taken by `nvs_ext::handle` (`crates/nvs-ext/tests/bytes.rs`), and
`nvs run` lends a `bytes` argument by number, as it does a `mixed` one. `nvs run` crosses every
row (`tests/conformance/ext/an-extension-call-crosses-every-row-of-the-value-table.nvst`,
`tests/conformance/ext/an-extension-resource-is-held-by-the-program-and-passed-back.nvst`); a
`Core` value class is read and rebuilt by `nvs_stdlib::ext_record`, with its constructor's checks.
An extension's enum is an `int`-backed enum in its class's namespace whose cases are numbered in
the manifest's order, and its resource is a member-less class in that namespace, held at run time
as an object whose one slot is the request's number for it (`nvs_types::ext_lib`).

**Declined:**
- **A handle for every value.** Nothing would be copied that the guest does not read, but every
  author would write marshalling code, and a record of ten fields would cost ten crossings. The
  common call is coarse, and a copy serves it faster.
- **Typed values and handles at the author's choice.** That is two ways to pass one argument.
- **A patched wasmtime with a byte-list `Val`.** It would keep `list<u8>` for every extension, but
  wasmtime is the sandbox, and a fork of it would be carried through every dependency update.
  Security and request isolation come first.
- **A closed set of typed call shapes for third-party exports.** The shapes multiply with each
  parameter, and they cover only some signatures.
- **The `bytes` resource in the built-in worlds.** Their exports are called typed, so their byte
  lists already cross in one copy.
