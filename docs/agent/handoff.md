# Handoff

## State

**Goal 24 — `Core\Net`, `Core\Os` and `Core\Signal`. The datagram third of `Core\Net` is on disk:
three of the five entry points are registered, and the two Unix-domain ones are what is left.**
`Core\Net::bindDatagram` answers a `Core\Net\Datagram` (`send`, `receive`, `port`, `close`), and a
receive answers a `Core\Net\Datagram\Message` (`payload`, `host`, `port`) — the first four-segment
class name in the tree, and it resolves like any other. All five stage-2 `-p nvs-host` and
`nvs-config` checks are green, as are three of the five `-p nvs-stdlib` names.

The two grants are asked at two moments and both are declared in `registry::CAPABILITIES`: the bind
asks `net.listen` of its endpoint, and `Core\Net\Datagram::send` asks `net.connect` through
`nvs_runtime::capability::pin_host` because a datagram socket is bound and never connected, so the
send is the only place its address is written (`rule:security/net-address-policy`).

`Core\Net\Datagram\Message::host` answers a plain `string` and not a `tainted` one, on purpose:
`send`'s `$host` is a sink, there is no launderer for an address, and a qualified answer would be a
datagram socket that could not reply. `crates/nvs-stdlib/src/net.rs`'s module doc § *Decision: a
receive answers a message, not octets* is the home of that.

Nothing is blocked.

## Next group

**Stage 2 (cont.): the two Unix-domain entry points, which close stage 2** — one file set:
`crates/nvs-stdlib/src/net.rs`, `crates/nvs-stdlib/src/registry.rs`, `tests/conformance/core/`.

- [ ] **The handle types have to carry two transports before either door can be written.**
      `crates/nvs-stdlib/src/net.rs:847`'s `Connected(NvsTcp)` and `:858`'s `Bound(NvsListener)` are
      newtypes over one type each, and `crates/nvs-stdlib/src/net.rs:983`'s `stream_of` answers
      `&mut NvsTcp` — but `Core\Net\Stream::read`/`write` must work over a Unix-domain connection
      too, since `rule:core-classes/net-one-api-three-transports` says the connected transports
      answer one `Read` and one `Write` shaped like every other stream. Make `Connected` an enum
      over `NvsTcp` and `nvs_host::NvsUnix` (`crates/nvs-host/src/net.rs:604`) and have `stream_of`
      answer something both satisfy; the deadline is on both, since `set_deadline` is on
      `NvsStream<S: Source>` at `crates/nvs-host/src/net.rs:204`. `Bound` needs the same over the
      listening half. This is the whole design question in the group — the doors after it are
      `listen`'s body with a different opener.
- [ ] **`Core\Net::connectLocal` and `Core\Net::listenLocal`, both asking `net.local`.**
      `rule:config/net-local-is-named-and-not-on-the-roster` is the rule and it is `designed`, not
      shipped: `nvs_config::Cap::NetLocal` exists at `crates/nvs-config/src/capability.rs:98`, and
      the grant is asked at `Scope::Path` under
      `rule:security/path-scope-canonicalise-then-prefix`, carries **no** address policy, and
      **governs both ends** — binding a path is granted exactly as connecting to one is. Rows and
      cards at `crates/nvs-stdlib/src/net.rs:172`, the `address()` arm at
      `crates/nvs-stdlib/src/net.rs:817`, the roster at `crates/nvs-stdlib/src/registry.rs:1724`
      and the capability rows at `crates/nvs-stdlib/src/registry.rs:2036`. Neither member takes a
      host and no member takes both, which is
      `rule:security/a-path-is-not-a-url` holding by construction.
- [ ] **The two `-p nvs-stdlib` names the stage-2 acceptance check still wants, neither of which
      exists.** `a_program_supplied_unix_path_is_refused_as_a_target_at_every_door` — a path handed
      to `connect`, `listen`, `bindDatagram` and `Core\Db::open` is refused at each, because
      `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it` stays exactly as
      strict and `connectLocal` is a *separate member*, not a widening. And
      `no_door_dispatches_on_a_url_scheme` — `unix:`, `tcp:` and `php:` prefixes are hostnames that
      do not resolve, asked of every door as agreement rather than one at a time. Beside them at
      `crates/nvs-stdlib/src/net.rs:1300`-ish, and three `.nvst` cases per new member on
      `tests/conformance/core/net-a-datagram-names-who-sent-it-so-a-reply-can-be-addressed.nvst:1`'s
      pattern; a Unix path under Windows is the thing to check first, since `AF_UNIX` is there but
      the path shape is not a `/run/...` one.

## Backlog

- Stage 3 `Core\Os` — the acceptance check wants `examples/os-facts.nvs`, which no session has
  written; five host facts, `docs/agent/loop-goal.md` § *Standing decisions* says it builds rather
  than decides.
- Stage 4 `Core\Signal` — graceful shutdown and nothing else, per `docs/decisions/0148.md`.
- `docs/spec/01-core-library.md` § 16 has a roster row for `Core\Net` and no member table, so
  nothing in this class is covered by `spec_registry_coverage.rs`. Not a gate today; it is the one
  place a member could be added with no spec row to check it against.
