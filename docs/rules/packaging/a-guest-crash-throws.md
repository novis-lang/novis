A guest's failure reaches the program in one of three ways, decided by what failed.

| What happened | What the program sees |
|---|---|
| the export returned `err(invalid(m))`, `err(parse(m))` or `err(runtime(m))` | a `LogicError`, `ParseError` or `RuntimeError` with the message `m` |
| the guest trapped — a panic, `unreachable`, an out-of-bounds access in its own memory, invalid UTF-8 | an `ExtensionError`, naming the extension, the export and the trap |
| the guest reached the request's CPU or memory limit | a resource-limit `FATAL` (`rule:errors/on-limit`) |

`ExtensionError` is a global class extending `RuntimeError`, so a handler that catches runtime errors
catches it. A trap is contained by the sandbox and leaves the host's state intact, so the request goes
on: the trapped instance is discarded, and the next call to that extension in the same request gets a
fresh one. An application answers a crafted upload that crashed a decoder with a status code, as it
answers one the decoder refused.

A limit stays a `FATAL` because the request, not the extension, asked for too much, and a resource
limit is not a `Throwable` (`rule:errors/throwable-hierarchy`).

**Not on disk.** `ExtensionError` is in the compiler's exception tree under `RuntimeError` with no
property of its own (`nvs_hir::errors::TREE`) and in the runtime's thrown-class roster, but nothing
turns a guest's `err` or trap into a throw yet.
