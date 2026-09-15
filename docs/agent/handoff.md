# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3, 4 and 5 are complete. **Stage 5's two checks are green**: `nvs service` names all seven
verbs, and all nine named cases run — the three installs, the uninstall, the control verbs, the
refusal that precedes any manager call, `status` over the control socket, and the two that drive a
service manager's own controls.

**A service manager's controls are answered with the operations that already exist.**
`service::hosted` (`crates/nvs-cli/src/service.rs:807`) maps an SCM control code to one of two
things: a `STOP` or a `PRESHUTDOWN` enters `crate::stop::deliver_to`, the one drain a `SIGTERM`
enters, and is reported with a checkpoint that advances while requests finish; a `PARAMCHANGE` enters
`Controlled::reload`, the one reload every other spelling ends in. The mapping is a value on both
platforms, and on Windows a case holds its three numbers to `windows-sys`'s own.

**What is not on disk is the dispatcher that would hand it a control** — `StartServiceCtrlDispatcherW`,
`RegisterServiceCtrlHandlerExW` and `SetServiceStatus`, which only a process the SCM started itself
can use, and which invert `nvs serve`'s entry so the serving happens inside `ServiceMain`. No check
names it and `service.rs`'s module doc owns the gap. Nothing is blocked.

## Next group

**Stage 6: the response body, and `echo` into it** — one file set:
`crates/nvs-stdlib/src/response.rs`, `crates/nvs-stdlib/src/html.rs` and
`crates/nvs-runtime/src/ctx/output.rs`.

- [ ] **`Core\Response::html(Core\Html\Markup $body)`** — the carrier's bytes written as they are,
      declaring `text/html; charset=utf-8`. The member row goes in the `CLASS` table at
      `crates/nvs-stdlib/src/response.rs:261`, beside `bytes` at
      `crates/nvs-stdlib/src/response.rs:284`; `Markup` is `crates/nvs-stdlib/src/html.rs:146`'s.
      `crates/nvs-stdlib/src/response.rs:13`'s reason for the member's absence is false today and
      goes with it, and the
      key is struck from `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:30`.
      `rule:core-classes/html-auto-escape`.
- [ ] **`echo` into an HTML response escapes, and the escape gets one home** — under
      `OutputSink::Body` a non-`Markup` value takes the five-character escape whether or not it is
      tainted, and `Markup` is written as it is; every other sink keeps the terminal's rendering. The
      sink is `crates/nvs-runtime/src/ctx/output.rs:464`, the carrier test is
      `crates/nvs-runtime/src/ctx/output.rs:54`, and `Core\Html::escape`
      (`crates/nvs-stdlib/src/html.rs:146`) becomes a caller of it rather than its home.
      `rule:core-classes/html-auto-escape`, `rule:tooling/echo-always-has-a-sink`.
- [ ] **`Core\Response::sendFile(string $path)`** — the path alone, as a `Qual::Sink` checked against
      `fs.read` at the call, with a missing path, a directory or an unreadable file throwing there;
      the `Ctx` then holds a declared file body that the server answers through the static-file
      policy, streamed and never read whole. Same table at
      `crates/nvs-stdlib/src/response.rs:261`, and the body reaches the writer at
      `crates/nvs-runtime/src/ctx/output.rs:248`.
      `rule:security/response-body-is-one-typed-member`.

## Backlog

- The SCM dispatcher that hands `service::hosted` a control — `crates/nvs-cli/src/serve.rs`, and the
  module doc at `crates/nvs-cli/src/service.rs:100` owns the gap.
- `crates/nvs-cli/src/service.rs` is past 4,000 lines; `hosted` and `registration` are the two
  seams that would split out cleanly.
- Stage 6's four `.nvst` cases and the two `nvs-server` cases named at
  `docs/agent/loop-goal.toml:10222` are all still unwritten.
