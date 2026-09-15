# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and **stages 3 to 6 are complete**. Stage 6's three checks are green: `Core\Response::sendFile` is
registered, the name it leaves reaches a `Completion`, the server answers it through the static
policy, and the stage's four `.nvst` cases are on disk. Nothing is blocked.

**`sendFile` hands over a name and never bytes.** The member refuses at the call what it cannot send
— a missing path, a directory, a file the process may not read — and then `Ctx::declare_file_body`
records the path and answers whether anything will read it. Inside a request the finish path lifts it
onto `nvs_runtime::host::Completion::file_body` and `crates/nvs-server/src/serve.rs`'s `sent` hands
it to `crate::statics::send`, so the media type, the `ETag`, the range and the conditional are the
one policy both deployments already have. Off a request there is no response to declare onto and the
member writes the bytes into the program's own output a chunk at a time, which is
`nvs_stdlib::response`'s gap 1 and what makes the `.nvst` cases assertable at all.

**Which request headers the file policy reads has one home**, `crate::statics::asked`: a program
declares a file body only after it has run, by which time the request belongs to the isolate, so the
accept loop copies that subset before the handler and a request naming neither allocates nothing.

## Next group

**Stage 7: the request's input and the test request** — one file set:
`crates/nvs-stdlib/src/request.rs` and `crates/nvs-stdlib/src/test.rs`.

- [ ] **The member: `Core\Request::bytes(): tainted bytes`** — the binary reading of the body, named
      after `Core\Response::bytes`, and a **buffering** reader that shares `body()`'s hold rather than
      taking a second copy of it. The row goes in the `CLASS` table at
      `crates/nvs-stdlib/src/request.rs:212`, beside `body` at `crates/nvs-stdlib/src/request.rs:288`
      whose card is at `crates/nvs-stdlib/src/request.rs:572` and whose helper is at
      `crates/nvs-stdlib/src/request.rs:2582`. The goal's standing decisions settle both the spelling
      and that an ill-formed body is refused rather than repaired, so `body` names `bytes` in its
      own refusal. `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
      Cases: `tests/conformance/core/request-bytes-reads-a-binary-body-whole-and-tainted.nvst` and
      `tests/conformance/core/request-body-refuses-a-body-that-is-not-utf-8-and-names-bytes.nvst`;
      the named test is `request_bytes_and_body_share_one_hold_and_bytes_never_copies_it_twice`.
- [ ] **The test request carries headers and a body** — `Core\Test::request`'s options bag, whose keys
      the goal fixes as `headers` and `body`. The row is `crates/nvs-stdlib/src/test.rs:437`, its card
      `crates/nvs-stdlib/src/test.rs:702` and its helper `crates/nvs-stdlib/src/test.rs:1864`; what
      arrives in the child is `tainted`, like everything else off a request. `rule:testing/in-process-request`.
      Case: `tests/conformance/core/test-request-carries-headers-and-a-body-that-arrive-tainted.nvst`;
      the named test is `a_synthetic_request_carries_its_headers_and_body_into_the_program`.

## Backlog

- The Windows service dispatcher (`StartServiceCtrlDispatcherW` and the two beside it) is still
  unwritten; `crates/nvs-cli/src/service.rs`'s module doc owns the gap and no check names it.
- No case runs `sendFile` through a real `nvs serve`, so the path from a declared body to a socket is
  asserted at `crates/nvs-server/src/serve.rs`'s seam and not end to end.
- `Core\Metrics`'s three rows belong to goal `m8-stdlib-depth`, not here.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30` and its three siblings wait on goal
  `unowned-closures`' decision sheet.
