Two values cross as handles. A `mixed` parameter crosses as an opaque `value` resource, and a
third-party export's `bytes` crosses as the `bytes` resource
(`rule:packaging/a-value-crosses-as-its-wit-type`). Each is an index into a per-call handle table
the host bounds-checks, and the guest reads it through accessor functions. Every other type crosses
as its WIT type, copied whole. A guest cannot forge a host pointer; it can only present an index,
which is validated, and a guest reading past the end of the host heap gets nothing rather than
adjacent memory.

The host stays authoritative for refcounting and copy-on-write, and the guest never sees a refcount.
Behind a handle a large array or string is not copied wholesale — the guest pulls what it reads — and
for byte strings it may request a bulk copy into its own linear memory, which is memcpy-bound at
roughly 12 ns per KiB. A `bytes` handle the guest makes itself is the one handle it writes: it adds
to it with `append`, each part charged to the request, and returns it as its result.

This is also why an extension exposes no per-pixel or per-element accessor across the boundary: each
accessor call is a fixed cost, a bulk copy is nearly free, and the design that wins moves whole buffers
a few times rather than words many times (`rule:packaging/the-boundary-is-the-cost`,
`rule:core-classes/image-pipeline`).

What it spends: one table entry per handle in flight, at most 65,536 of each kind at once, and a
`bytes` result's buffer, charged to its request until the call returns.

**Partly on disk.** `nvs_ext::handle` is the per-call table, the `value` accessors and the `bytes`
methods, and `nvs_ext::call::Request::call_values` lends a `mixed` argument and a third-party
`bytes` through it; a handle kept past its call traps when it is read
(`crates/nvs-ext/tests/convert.rs`, `crates/nvs-ext/tests/bytes.rs`). `nvs run` calls an extension
from a program, and the row-table case passes `mixed` values to the fixture's `kindOf`
(`tests/conformance/ext/an-extension-call-crosses-every-row-of-the-value-table.nvst`). A `bytes`
value or an open `Core\IO\File` passed as `mixed`, and a `bytes` argument of a third-party export,
is lent by number and not copied, and the host serves each `range` the guest reads from the
program's own copy between two polls of the call
(`rule:core-classes/spreadsheet-reads-as-a-stream`). `as-bytes`, `range`, `read` and `append` are
typed host functions that copy a `list<u8>` whole, so a block a guest reads or writes through a
handle is one bulk copy (`crates/nvs-ext/src/handle.rs`). Every built-in component's exports are
called typed (`crates/nvs-ext/src/typed.rs`), so their `bytes` cross as `list<u8>` in one copy
each way and never through a handle.
