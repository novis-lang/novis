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

**Not on disk.** Nothing links WASI or any other import for a guest.
