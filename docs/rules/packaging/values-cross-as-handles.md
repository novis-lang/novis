A `mixed` parameter is the one value that crosses as a handle: the guest receives an opaque `value`
resource — an index into a per-call handle table the host bounds-checks — and reads it through
accessor functions. Every other type crosses as its WIT type, copied whole
(`rule:packaging/a-value-crosses-as-its-wit-type`). A guest cannot forge a host pointer; it can only
present an index, which is validated, and a guest reading past the end of the host heap gets nothing
rather than adjacent memory.

The host stays authoritative for refcounting and copy-on-write, and the guest never sees a refcount.
Behind a handle a large array or string is not copied wholesale — the guest pulls what it reads — and
for byte strings it may request a bulk copy into its own linear memory, which is memcpy-bound at
roughly 12 ns per KiB.

This is also why an extension exposes no per-pixel or per-element accessor across the boundary: each
accessor call is a fixed cost, a bulk copy is nearly free, and the design that wins moves whole buffers
a few times rather than words many times (`rule:packaging/the-boundary-is-the-cost`,
`rule:core-classes/image-pipeline`).

**Partly on disk.** `nvs_ext::handle` is the per-call table and the `value` accessors, and
`nvs_ext::call::Request::call_values` lends a `mixed` argument through it; a handle kept past its
call traps when it is read (`crates/nvs-ext/tests/convert.rs`). `nvs run` calls an extension from a
program, and the row-table case passes `mixed` values to the fixture's `kindOf`
(`tests/conformance/ext/an-extension-call-crosses-every-row-of-the-value-table.nvst`). A `bytes`
value or an open `Core\IO\File` passed as `mixed` is lent by number and not copied, and the host
serves each `range` the guest reads from the program's own copy between two polls of the call
(`rule:core-classes/spreadsheet-reads-as-a-stream`). `as-bytes` and `range` are typed host
functions that return a `list<u8>` whole, so a block a guest reads through a handle is one bulk copy
(`crates/nvs-ext/src/handle.rs:262-285`). A `bytes` parameter or result of an export does not
go through a handle. Every built-in component's exports are called typed (`crates/nvs-ext/src/typed.rs`),
so their bytes cross in one copy each way. A third-party export's cross as one wasmtime `Val` per byte,
because its types are known only at run time (`crates/nvs-ext/src/convert.rs:53`).
