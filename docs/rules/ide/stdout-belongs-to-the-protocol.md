stdio *is* the wire. One stray `println!` anywhere under the analysis corrupts the framing, which presents
as the server dying for no reason. So nothing but the protocol may write to stdout: the server logs to
stderr and, for anything a user should see, `window/logMessage`, and `nvs-lsp` does not carry `nvs-cli`'s
`clippy::print_stdout` allowance.

`println!`/`print!` appears nowhere in `crates/` outside `nvs-cli`, whose whole job is terminal
output, so this is an invariant to keep rather than one to establish, and it is kept by a test over
every crate the server links rather than by care. The server links the type checker, so `nvs-stdlib`
and `nvs-runtime` beneath it are in that dependency set, and the runtime does write to stdout: it is where a
program's `echo` goes under `nvs run`. That is not an exception, because it is a sink a caller wires
into a `Ctx` and the server builds no `Ctx` at all — so the test exempts the sink's own
implementation and separately checks that nothing under the server wires one.
