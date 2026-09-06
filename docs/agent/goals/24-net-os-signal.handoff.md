# Handoff

## State

**Goal 24 — `Core\Net`, `Core\Os` and `Core\Signal` — has just started; nothing of it has landed yet.**
Goal 23's whole list is this goal's Stage 1 floor.

These are the three classes in spec § 16 that **no milestone and no goal named at all** — the others in
`spec-classes-part-two-outstanding.txt` each had an owner once it was looked for. They are M8's:
`rule:core-api/tier-roster` puts all three at Tier 0, and M8's stdlib
goals (4 and 5) walked without them.

`Core\Socket` (`crates/nvs-stdlib/src/socket.rs`) is ADR 0083's WebSocket upgrade and is **not** what
this goal extends — the name collision is the trap worth knowing before opening the file.

## Next group

**Stage 2: `Core\Net`** — one file set: `crates/nvs-host/src/net.rs`,
`crates/nvs-config/src/capability.rs`, `crates/nvs-runtime/src/capability.rs`, and the new
`crates/nvs-stdlib/src/net.rs`.

- [ ] **The ADR first.** One number for the socket surface: what a program may open, what the
      capability answers, and why there is no second event loop. `rule:core-api/tier-roster`'s row is the whole design
      on disk today.
- [ ] **`net.local` joins the roster** — ADR 0142 § 6 named it and goal 20 deliberately did not add it
      ("it has no caller until `Core\Net` lands"). This goal is that caller. `net.listen` is separate
      and neither widens `net.connect`.
- [ ] **TCP over `NvsTcp`/`NvsListener`** (`net.rs:227`, `:313`) — a program-supplied host still walks
      ADR 0058 § 3's denied-range table through `pin_host`; goal 20's carve-out was for *configured*
      stores and does not reach a program's own `connect`.
- [ ] **UDP is the one addition to `nvs-host`** — there is no datagram type in that crate. `NvsUdp`
      over `mio::net::UdpSocket`, shaped like `NvsStream` so it parks rather than blocking the core.
- [ ] **Unix needs no new transport** — `NvsUnix` is at `net.rs:460`. A program-supplied path is
      refused at this door as a *target*, per goal 20's standing decision.

## Backlog

- **Stage 3 (`Core\Os`)** is five facts and its own small file set. `cpuCount` answers
  `available_parallelism`, which after goal 23 is the number the server actually fans out over;
  `crates/nvs-cli/src/info.rs:145` is today's only caller. `loadAverage` throws on Windows naming the
  platform rather than inventing an answer.
- **Stage 4 (`Core\Signal`)** is graceful shutdown only — no `kill`, no `alarm`, no signal number as an
  integer. A handler enters the drain that already exists (`crates/nvs-server/src/control.rs`), never a
  second state machine, and runs at a safepoint rather than in a signal context.
- **Stage 5** strikes the three keys from `spec-classes-part-two-outstanding.txt`; registering a class
  and striking its line are one slice.
