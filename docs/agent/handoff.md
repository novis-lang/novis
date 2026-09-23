# Handoff

## State

Goal `core-http-socket-and-1-more` (dossier). `Core\Http\Socket::close`, `::protocol` and
`::receive` have all their feature proofs: `about.md`, three examples each under
`docs/examples/core/Http-Socket/`, an attack each under `tests/hostile/core/Http-Socket/`, a bench
each under `benches/members/core/Http-Socket/`, `covers:` markers on the `.nvst` cases and the Rust
tests in `crates/nvs-stdlib/src/http/socket.rs`, and `Core\Http\Socket`'s class card
(`SOCKET_CARD`; it is off `CLASSES_STILL_OWING_A_CARD`). The `protocol` attack found that
`openSocket` accepted any `protocols` entry; each entry must now be one RFC 6455 token, and the
refusal happens before the scripted/live branch, so both paths refuse the same entries. Pinned by
`tests/conformance/core/a-socket-offers-only-subprotocol-names-that-are-one-token.nvst`.

## Next group

**The two send members** — one file set: `crates/nvs-stdlib/src/http/socket.rs` (rows,
`SEND_ERRORS`, `nvs_core_http_socket_send`/`_send_bytes`, the test module), the
`docs/examples/core/Http-Socket/` tree the close/protocol/receive proofs already model, and
`Core\Test::sentSocket` for reading back what was sent.

- [ ] **`Core\Http\Socket::send`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`; `crates/nvs-stdlib/src/http/socket.rs:229`. The `.nvst` case `an-outbound-socket-closes-once-and-answers-nothing-after.nvst` calls it and needs a `covers:` marker. Rust has no test that drives `written` yet.
- [ ] **`Core\Http\Socket::sendBytes`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`; `crates/nvs-stdlib/src/http/socket.rs:243`.
- [ ] **`Core\Http\Event::data`**, then `::name` and `::id` — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/http/stream.rs:388`, `crates/nvs-stdlib/src/http/stream.rs:397`, `crates/nvs-stdlib/src/http/stream.rs:406`. Different file set: a group of its own after the two sends.

## Backlog

- A scripted socket ignores `close`'s code and reason, because nothing records them. The live path is pinned by `a_close_carries_the_programs_code_and_reason`. A `Core\Test` reader for the close frame would let an example show it. Owner: `crates/nvs-stdlib/src/http/socket.rs` module doc, not yet recorded.
- A top-level `catch ($x)` binding and a later top-level declaration of the same name are `E0406`, because both are file scope. That cost one run of the `close` attack.
