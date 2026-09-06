stdio *is* the wire. One stray `println!` anywhere under the analysis corrupts the framing, which presents
as the server dying for no reason. So nothing but the protocol may write to stdout: the server logs to
stderr and, for anything a user should see, `window/logMessage`, and `nvs-lsp` does not carry `nvs-cli`'s
`clippy::print_stdout` allowance.

Today no library crate writes to stdout — `println!`/`print!` appears nowhere in `crates/` outside
`nvs-cli`, whose whole job is terminal output — so this is an invariant to keep rather than one to
establish, and it is kept by a test over every crate the server links rather than by care.
