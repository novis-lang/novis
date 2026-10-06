A guest links WASI with an **empty context**: no preopened directory, no network, no environment and
no arguments; stdout and stderr go to the request log, the clocks are the request's clock, and random
bytes come from `Core\Random`. A toolchain that links WASI for its own libc — wasi-sdk, TinyGo, Zig —
therefore builds a working component that holds no authority at all. Native extension code calls
`open()` and `connect()` directly and bypasses any capability system; a guest cannot, because it has
no syscalls.

Two kinds of I/O can be granted, and no other: `wasi:filesystem` preopens, and `wasi:http`'s outgoing
handler implemented by `Core\Http\Client`, so the outbound address policy, TLS trust and the proxy
apply exactly as they do to script code. What a guest holds is the intersection the operator, the
manifest and the calling code all allow (`rule:security/extension-grants-are-an-intersection`), derived
from the grants rather than declared beside them — there is no second place authority is written. A
component importing any other interface does not load, and the refusal names the import.

The same absence is what keeps an extension honest about qualifiers: with no ambient source and no
sink of its own, it can declare what it consumes and produces but cannot launder
(`rule:security/extension-cannot-launder`).

**Not on disk.** The import list is written as the `extension` world in `wit/nvs-ext/world.wit`, and
`crates/nvs-stdlib/tests/ext_world.rs` checks it imports exactly the allowed interfaces. `nvs_ext::wasi`
links the world's WASI interfaces but `wasi:http`, and the loader admits a WASI import only from the
ones it links (`crates/nvs-ext/tests/wasi.rs`). A guest holds a preopen for each root of its
effective set and no other (`crates/nvs-ext/tests/files.rs`), no environment, no
arguments and an empty stdin, and its stdout and stderr write each line to the log of the request
`nvs_ext::call::Budget` names, and its clocks and random bytes are that budget's, so a fixed clock
and a seed reach it. A libc-shaped guest fits one pool slot, and its `exit` throws `ExtensionError`.
`nvs run` and `nvs serve` hand it the request's own budget (`nvs_cli::extensions::Spent`): a line it
logs is a record of the request with the extension as its channel, and a test's fixed clock and seed
reach it. `nvs:ext/settings` reads the `[ext.<name>]` block of the request's snapshot, a
`System`/`Reload` block `nvs.toml` accepts and boot and reload check against the loaded manifests
(`nvs_cli::extensions::loaded`). `wasi:http` is not linked, so a `connect` grant reaches nothing.
