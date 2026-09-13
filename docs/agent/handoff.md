# Handoff

## State

**Goal `websocket-client` — a program holds a WebSocket to another server, opened like an outbound call and closed with the task that opened it — has just started; nothing of it has landed yet.** Goal `outbound-proxy`'s whole list is this goal's Stage 1 floor.

**The design is settled with the user and is written into the goal's § *Standing decisions*; the record
that states it is not written yet.** Stage 2 writes it. The things not to re-decide: a socket is opened
through `Core\Http\Client`'s door under every rule a call is; it answers `Core\Socket\Message`; it
belongs to the task that opened it and is never a root isolate; every wait on it is bounded;
`permessage-deflate` is out.

## Next group

**Stage 2: the record** — one file set: `docs/decisions/`, `docs/rules/http-server*`,
`docs/rules/testing*`.

- [ ] **The record** — the next free number in `docs/decisions/`, `changes.creates` the four rules the
      goal's stage 2 table names, `changes.modifies` `http-server/allow-url-pins-the-address`. Its body
      is the goal's standing decisions and stages 3-5, argued, with the shipped `max_message` and
      `send_timeout` and the names of the row and the socket class.
- [ ] **The four rule fragments and their JSON entries**, all `designed`, and the amendment to
      `http-server/allow-url-pins-the-address`, then `python tools/rules.py --render`.

## Backlog

- Stage 3 — the scripted peer. `crates/nvs-stdlib/src/test.rs`, `registry.rs`. The keystone: every later
  `.nvst` case needs it.
- Stage 4 — the handshake. `crates/nvs-stdlib/src/http.rs`, `http/transport.rs`. Its own session.
- Stage 5 — the conversation and its bounds. The new module under `crates/nvs-stdlib/src/http/`,
  `socket.rs`, `crates/nvs-config/src/directive.rs`. Shares the transport with stage 4.
- Stage 6 — the flips. `docs/rules/` only.
- When this goal's last check goes green the driver takes goal `gap-zero`.
