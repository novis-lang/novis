# Handoff

## State

**Goal `unowned-closures`, stage 4 is done; stage 5 is the goal's earliest red check.** The
hand-written JSON codec is closed at both halves. A class that wrote `fromJson` is decoded through it
at every level of a document, and one that wrote `toJson` is encoded through it the same way:
`Encodable::serialize_object` (`crates/nvs-stdlib/src/json.rs:975`) asks the class table for the
member before it reads the derived field list, so a written half beats the attribute's list and that
list stays the decoder's where a class carries both.

The re-entrancy that call opens is settled and **stated** in `nvs_stdlib::json`'s module doc rather
than left to be discovered. `Reentry` (`crates/nvs-stdlib/src/json.rs:578`) carries the `*mut Ctx`
down the walk and the raised `Fault` back out past `S::Error`, which is a string and would lose the
class a `catch` names; `Standing` (`crates/nvs-stdlib/src/json.rs:848`) retains this value and each
of its ancestors across the call, so a member that frees or mutates what the walk is standing on
cannot leave a borrowed handle over freed memory. Every door into the encoder now goes through
`rendered` and carries a `&mut Ctx` for it — `Core\Json::encode`, `Core\Response::json`,
`Core\Sse::send`, `Core\Jwt::signObject`, `Core\Queue::push`, `Core\Test::answerHttp` and the HTTP
client's `json` body. Nothing is blocked.

## Next group

**Stage 5: the server and the cache** — one file set: `crates/nvs-server/src/bounds.rs` and
`crates/nvs-config/src/cache.rs`, which are the two halves of the goal's one red check
(`docs/agent/loop-goal.toml:11114`). Neither test it names exists yet.

- [ ] **The server's connection bounds are read from the `[server]` block** —
      `crates/nvs-server/src/bounds.rs:243`, `rule:concurrency/connection-bounds-are-finite`. Every
      bound is finite before anything is configured, which is the rule's floor; what is missing is
      the configured half, so a deployment cannot raise or lower one. The check wants
      `server_connection_bounds_are_read_from_the_server_block`.
- [ ] **A rebuilt compiler is a new cache key** — `crates/nvs-config/src/cache.rs:140`,
      `rule:config/the-extension-set-is-in-every-unit-key`. `env_hash` mixes a compiler *version*
      hash, so two builds of one release version key their units together and a rebuilt compiler
      serves the old artifact. The check wants
      `two_builds_of_one_release_version_key_their_units_apart`.

## Backlog

- A `#[Test]` result is a producer, so § 22's three output formats are one record rendered —
  `crates/nvs-cli/src/runner.rs:418`, where the three fan out today. Carried since stage 4; it shares
  no file with any group that has come up since.
- `nvs_stdlib::json` gap 1 — the codec walk is a descriptor read by native Rust rather than emitted
  code (`crates/nvs-stdlib/src/json.rs:216`). The gap's `Decided:` keeps the descriptor and the rule
  now describes it, so what is left is whether the machinery stays one at all.
- `nvs_stdlib::json` gap 2 — the encoder's real bound is the native stack, not `DEPTH_CEILING`
  (`crates/nvs-stdlib/src/json.rs:229`).
