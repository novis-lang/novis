# Handoff

## State

**Goal `event-streams` — stage 6's prose is landed.** Three rule fragments now carry this goal's
decisions: `rule:concurrency/connection-bounds-are-finite` gains the derived beat, the per-stream
reconnect draw and the unarmed `idle`; `rule:tooling/echo-always-has-a-sink` gains the connection
isolate's row; `rule:security/response-body-is-one-typed-member` counts seven members and classifies
the two that write over time. `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`
is **shipped** rather than designed, and says what `receive()`'s absence was ever about.

**`docs/decisions/0177.md` is accepted** — the two doors, the topics-only `receive()`, the three
`LogicError` refusals, the two derived waits, the emitted headers, the `Last-Event-ID` position and
the rejected `send`/`sendJson` split. `docs/decisions/0176.md`'s `changes:` block was empty and now
names the rule it shipped.

**Four of stage 6's five checks are green** (`rules.py --check`, `decisions.py --gate`,
`check-links.py`, `reference.py --check`). The fifth is red for a real reason, not a naming one:
`Core\Sse` and `Core\Response::stream` have no row anywhere in `docs/spec/01-core-library.md`, so the
spec-into-registry ratchets pass over them vacuously. That is the next group.

**`Core\Socket::receive` is still inside `Ctx::deliver`'s known gap.**

## Next group

**Stage 6: the spec rows** — one file set: `docs/spec/01-core-library.md` and the two ratchet files
beside it under `crates/nvs-stdlib/tests/`. Prose over landed behaviour; nothing here compiles
anything, and `cargo test -p nvs-stdlib` is what reads it.

- [ ] **`Core\Sse`'s § 16 row** — `docs/spec/01-core-library.md:1163` is the class table whose ADR
      column the ratchets read, and it has no connection door in it at all. The row is `upgrade`,
      `stream`, `current`, and on the handle `receive`, `send` and `retry`, written under the
      names `crates/nvs-stdlib/src/sse.rs:124`'s rows already carry — the test compares the spec's
      signature column against `names`. `rule:concurrency/two-doors-one-isolate`.
- [ ] **`Core\Response::stream` in § 15** — `docs/spec/01-core-library.md:1075`, the request-facing
      section whose `Core\Response` entry still lists five body members. It owes `stream`, its sink
      content type and the `Core\Response\Stream::write` handle, as
      `crates/nvs-stdlib/src/response.rs:296` declares them.
      `rule:security/response-body-is-one-typed-member`.
- [ ] **Re-derive the two ratchets afterwards** — `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:1`
      and its member sibling list what the spec names and the registry does not. Every row this group
      adds is already registered, so neither file should gain a key; a key appearing means the spec
      wrote a name the registry does not carry. `rule:core-api/shape-rules`.

## Backlog

- `Core\Socket` has no spec row either, in the same § 16 table — `docs/spec/01-core-library.md:1163`.
- `Core\Socket::receive` sits inside `Ctx::deliver`'s known gap — that module's own `# Known gaps`.
- No connection bound has a `[server]` key, so changing one is a rebuild —
  `crates/nvs-server/src/bounds.rs`'s `# Known gap`.
- **`[context]` gap:** the goal's manifest names no `docs/spec/` selector, so the pack printed
  nothing about the spec although stage 6's first check is entirely about it. Add a `docs` or `files`
  entry for `docs/spec/01-core-library.md` in `docs/agent/loop-goal.toml`.
