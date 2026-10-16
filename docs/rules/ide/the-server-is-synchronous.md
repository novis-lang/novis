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
work; a request already being answered runs to the end.

The server holds one document store on one thread, so an async runtime would schedule nothing. Taking one
would put tokio, tower and `async-trait` into every later dependency review and every build, for a process
that never has two analyses running at once.

**Declined:**
- **`tower-lsp`.** It brings tokio into a workspace that has no async runtime, for a server whose state
  is one store on one thread.
- **JSON-RPC written by hand over `serde_json`.** The framing is only a `Content-Length` header, but the
  message structs `lsp-types` carries are the real content of the dependency, and writing them again is a
  maintenance surface with nothing gained.
