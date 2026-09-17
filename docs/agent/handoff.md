# Handoff

## State

**Goal `cache-shared-dial` is met.** `[cache.shared]` reads three schemes, a `password`/`password_file`
pair and a `database` index; the dial carries all of it as one value
(`crates/nvs-stdlib/src/cache/redis.rs:141`), and `Connection::ensure`
(`crates/nvs-stdlib/src/cache/redis.rs:180`) is the one place it is applied — on every connection,
including the one nobody asked for.

Stage 5 closed both halves. On the wire,
`auth_and_select_precede_the_first_command_in_that_order` and
`a_reconnect_after_a_dropped_socket_sends_both_again` (`crates/nvs-stdlib/src/cache/redis.rs:1030`)
read the bytes off a scripted store: the two steps precede the command that opened the connection,
in that order, and the store's second accept gets both again. Against a real store,
`examples/cache-shared-tls.nvs` dials `rediss://127.0.0.1:16380` at `database = 7` and reads
`greeting` — `examples/cache.nvs`'s entry at index zero of that same Redis — before it writes
anything, so the index line is a claim and not a restatement of its config: run against
`database = 0` the fixture prints `the connection answered out of index zero` instead. The store's
private CA reaches the client as `[http.client.tls] roots = ["bundled", "../tests/db/ca.crt"]` in
the fixture's own tree, which is the standing decision's "add it to the process's roots" and not a
key on `[cache.shared]`.

Stages 2 and 3 were green under test names the acceptance data does not use, which is invisible to
`cargo test` and refuses the `DONE`; all five now carry the frozen names, and the playbook bullet
above is why. `python tools/verify.py` is green, `--doc` is green, and `owners.py --closes` /
`playbook.py --closes` name nothing for this goal.

## Next group

**Stage 5 is closed, so the next group belongs to the goal the chain names after this one** — the
driver's goal switch writes it. If the acceptance sweep comes back red on a stage-5 check, the file
set is `crates/nvs-stdlib/src/cache/redis.rs` and the `examples/cache-shared-tls.*` pair, and the
first thing to check is the precondition in `## Backlog` below rather than the client.

- [x] **`AUTH` then `SELECT` go out before the first command, and again after a dropped socket** —
      two cases at `crates/nvs-stdlib/src/cache/redis.rs:1030` and
      `crates/nvs-stdlib/src/cache/redis.rs:1065`, under
      `rule:config/cache-shared-is-the-grant-over-the-configured-store`.
- [x] **`examples/cache-shared-tls.nvs` and its own `examples/cache-shared-tls.toml`** — the store
      `tests/db/compose.yaml:293` serves on `16380`, at a non-zero index, with the negative control
      described above.
- [x] **The five tests stages 2 and 3 are checked by carry the names the toml froze** —
      `crates/nvs-stdlib/src/cache.rs:4178`, `crates/nvs-config/tests/secret.rs:540` and
      `crates/nvs-config/tests/resolve.rs:1353`.

## Backlog

- `tests/db/ca.crt` is gitignored and reissued with the `certs` volume, so a machine that has not
  run the one `docker compose cp` in `tests/db/compose.yaml`'s header fails the TLS fixture at boot
  rather than at the handshake — that file's header owns the command.
- `Core\Session`'s `shared` backend gained all three of this goal's halves without a line of its
  own, and has no fixture that says so — `docs/agent/goals/65-cache-shared-dial.md` § *Standing
  decisions* is why that was left out.
