# Handoff

## State

Goal `core-http-socket-and-1-more` (dossier). Every `Core\Http\Socket` member — `close`,
`protocol`, `receive`, `send` and `sendBytes` — reports `complete.` under `python tools/dossier.py
--id`. The two send members got `about.md`, three examples, an attack and a bench each under the
`Http-Socket/send` and `Http-Socket/sendBytes` paths. Both are credited by the `covers:` markers on
`an-outbound-socket-closes-once-and-answers-nothing-after.nvst` and on the Rust test
`a_send_writes_one_whole_message_of_its_kind_and_nothing_past_the_lifetime`. That test reads the
wire through the loopback peer's new `Heard::said` in `crates/nvs-stdlib/src/http/transport.rs`.
`target/release/nvs.exe` was rebuilt this session and the stale socket and transport figures were
re-measured. What remains of the goal is `Core\Http\Event`.

## Next group

**Stage 2: `Core\Http\Event`'s three readers** — one file set: `crates/nvs-stdlib/src/http/stream.rs`
(the `Event` rows, their symbols and the test module), the `docs/examples/core/Http-Event/` tree
shaped like `docs/examples/core/Http-Socket/`, and `Core\Test::answerHttp`'s event-stream reply for
a scripted stream.

- [ ] **`Core\Http\Event::data`** — owes about, examples, hostile, perf, tests. `rule:testing/feature-proofs`; `crates/nvs-stdlib/src/http/stream.rs:388`. Run `python tools/dossier.py --id 'Core\Http\Event::data'` first for the paths.
- [ ] **`Core\Http\Event::name`** — owes the same. `rule:testing/feature-proofs`; `crates/nvs-stdlib/src/http/stream.rs:397`.
- [ ] **`Core\Http\Event::id`** — owes the same. `rule:testing/feature-proofs`; `crates/nvs-stdlib/src/http/stream.rs:406`.

## Backlog

- A scripted socket ignores `close`'s code and reason, because nothing records them. The live path is pinned by `a_close_carries_the_programs_code_and_reason`. A `Core\Test` reader for the close frame would let an example show it. Owner: `crates/nvs-stdlib/src/http/socket.rs` module doc, not yet recorded.
- `Core\Test::sentSocket`'s record (`nvs_runtime::AnswerTable::sent_frames`) is Rust memory outside the program's memory limit; a test that sends many megabytes grows it without a ceiling. Test-only by construction. Owner: `crates/nvs-runtime/src/ctx/answers.rs` doc, not yet recorded.
- A top-level `catch ($x)` binding and a later top-level declaration of the same name are `E0406`, because both are file scope. That cost one run of the `close` attack.
