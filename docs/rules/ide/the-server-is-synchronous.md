`crates/nvs-lsp` is a library plus a thin `nvs lsp` subcommand speaking LSP over stdio, built on
rust-analyzer's `lsp-server` and `lsp-types` — pure Rust, no build script — with `serde` and `serde_json`
from the workspace. No tokio, no tower, no `async-trait`: `rule:concurrency/one-scheduler` is why, and the
manifest-policy test is what pins it.

Concurrency is a thread and a channel, which is the shape `lsp-server` hands over and the shape the rest
of the project already uses: a reader thread, a writer thread, and one analysis thread that owns the
document store. A running analysis is never interrupted. An analysis is not started for a version the
client has already replaced, and one overtaken while it ran publishes nothing, because its result is
about a document version nobody is looking at any more (`crates/nvs-lsp/src/server.rs`, `publish`).
`$/cancelRequest` stops a request the thread has not reached yet, answering `RequestCancelled` with no
work; a request already being answered runs to the end. A request whose document has a later
`didChange` already queued behind it is answered `ContentModified` with no work, because it was made
against text that is gone. Without that, each request between two held-down keystrokes runs the
analysis the next keystroke replaces, and the debounce merges nothing. A request that carries its own
text is about that text and is answered.

**The index's waiting entries are analysed on worker threads** (`crates/nvs-lsp/src/workers.rs`). There
are as many workers as cores but one, and at least one. They start when an entry waits and stop when none
does, so a settled server has no worker thread. A worker keeps its own parse and check session and its own
configuration cache. Each job carries a copy of the open buffers' text, the lenders, shared and read-only,
and the set of files the index held when the job was sent. A worker returns what it found, which owns
everything it holds, and touches no shared mutable state, so nothing on the analysis path takes a lock.
The store, the index and every answer stay on the one thread. It files each result in entry path order
and makes the serial loop's decision at each entry's turn, so the index is the serial one, byte for byte.
A result is analysed again when something it relied on moved after its job was sent: a file it read was
edited, or a file it took as indexed was dropped. A request that needs every file waits for the workers
to drain. The open documents are still analysed on the one thread, before the answer that needs them.
It costs one session and one configuration cache per worker while entries wait, and an analysis an
earlier entry's result made unneeded, at most one per worker. That memory buys a cold index on every core.

The server holds one document store on one thread, so an async runtime would schedule nothing. Taking one
would put tokio, tower and `async-trait` into every later dependency review and every build, for a process
that never has two analyses running at once.

**Declined:**
- **`tower-lsp`.** It brings tokio into a workspace that has no async runtime, for a server whose state
  is one store on one thread.
- **JSON-RPC written by hand over `serde_json`.** The framing is only a `Content-Length` header, but the
  message structs `lsp-types` carries are the real content of the dependency, and writing them again is a
  maintenance surface with nothing gained.
- **One parse cache shared by every worker, behind a lock.** A lock on the analysis path costs every
  analysis latency to save memory, which is the wrong direction.
- **A setting for the worker count.** Nobody asked for one, and a fixed rule is one thing less to tune.
