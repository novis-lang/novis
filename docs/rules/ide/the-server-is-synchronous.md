`crates/nvs-lsp` is a library plus a thin `nvs lsp` subcommand speaking LSP over stdio, built on
rust-analyzer's `lsp-server` and `lsp-types` — pure Rust, no build script — with `serde` and `serde_json`
from the workspace. No tokio, no tower, no `async-trait`: `rule:concurrency/one-scheduler` is why, and the
manifest-policy test is what pins it.

Concurrency is a thread and a channel, which is the shape `lsp-server` hands over and the shape the rest
of the project already uses: a reader thread, a writer thread, and one analysis thread that owns the
document store. A request that arrives while an older analysis is in flight cancels it, because its result
is about a document version nobody is looking at any more; `$/cancelRequest` cancels an in-flight request
the same way.
