# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 3 has its door and nothing else
yet.** `nvs_runtime::capability::exec` (`crates/nvs-runtime/src/capability.rs:354`) starts `program`
with `argv` behind `Cap::ProcessExec`, pipes all three standard streams so a child never inherits the
server's own descriptors, and refuses a `.bat`/`.cmd`/`.ps1` target on every platform per ADR 0044
§ 4. It hands back the `Child` rather than the output, for `open_read`'s reason: `run`'s captured wait
and `spawn`'s streamed handle are one door. The capability is asked *first* and the target's kind
second, so this module's rule holds without an exception; both refusals are catchable `RuntimeError`.
No configuration changed — `Cap::ProcessExec` was already path-scoped.

The driver's acceptance check for `examples/process.nvs` still fails with ``Core\Process` has no
member named `run``. That is stage 3's open item and not a regression; the group below closes it.

**One thing to settle before writing the member.** `examples/process.nvs:35` reads `$result->exitCode`
and `$result->stdout` as **properties** and hands `stdout` to `Core\Str::trim`, while ADR 0044 § 1
says captured output is `bytes`, never `string`. The ADR wins over a fixture, and the fixture's source
is not frozen — only its three lines, in `docs/agent/loop-goal.toml` stage 3 — so the conversion goes
in the fixture.

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs` names
`http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an `[[app]]` block
naming `examples/logging/handler.nvs`; and every fixture that reaches the world still owes its
`process.exec` / `net.connect` grants.

`orient.py` still prints two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs` and
`crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**Stage 3's second half — `Core\Process::run` over the door that now exists.** One file set: a new
`crates/nvs-stdlib/src/process.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/lib.rs`, `crates/nvs-stdlib/src/instance.rs`, `examples/process.nvs` and
`nvs.toml`.

- [ ] **`Core\Process::run`, and the result it answers.** ADR 0044 §§ 1 and 6 are the member and the
      grant. The five-edit shape over a door is `crates/nvs-stdlib/src/io.rs:683`, the capability
      declaration is a row in `crates/nvs-stdlib/src/registry.rs:1121`, and the door to call is
      `crates/nvs-runtime/src/capability.rs:354`. If the result keeps the fixture's property
      spelling it is a **shape**, registered in `crates/nvs-stdlib/src/instance.rs:272`'s
      `SHAPE_ROSTER` with `Core\Issue` (`crates/nvs-stdlib/src/issue.rs:48`) as the precedent — its
      fields are held in sorted slot order.
- [ ] **A `.nvst` case, which `crates/nvs-stdlib/tests/conformance_coverage.rs` requires anyway.**
      Under `tests/conformance/core/`: ADR 0044 § 4's refusal is the case worth pinning, because it
      is the one answer that needs no child and no grant — the message it asserts is written at
      `crates/nvs-runtime/src/capability.rs:361`.
- [ ] **`examples/process.nvs` green**, at `examples/process.nvs:29`, with an `[[app]]` block
      granting `process.exec` in the repository's own `nvs.toml` — `examples/files.nvs`'s block,
      at line 62 there, is the shape to copy. The fixture's three frozen lines are stage 3's
      checks in `docs/agent/loop-goal.toml`.

## Backlog

- `Core\Process::spawn` and `ProcessHandle` — ADR 0044 § 2; the door already hands back a `Child`.
- `ProcessOptions` — cwd, env, timeout; ADR 0044 § 3. The door takes program and argv only today.
- § 5's coroutine-suspending wait: a blocking `run` ties up the worker thread until it lands.
- `examples/http.nvs` waits on stage 5's `http://127.0.0.1:8099` origin — `docs/agent/loop-goal.toml`.
- `examples/logging.nvs` needs an `[[app]]` block naming its handler — `docs/agent/loop-goal.toml`.
- Two dead `[context] modules` selectors in `docs/agent/loop-goal.toml` (`nvs-host` pool/stream).
