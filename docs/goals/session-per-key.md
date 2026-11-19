---
looper: 1
title: "a session is written back per key, so two requests that change different keys keep both"
summary: "Reached when a session's write-back sends only the keys its request changed, on the shared and the db store, two requests that change different keys of one session keep both changes, a destroyed session stays destroyed, regenerate moves every key the store holds, and the rules and pages say so."
context: [crates/nvs-stdlib/src/session.rs, docs/rules/http-server/a-session-is-loaded-once-and-written-back-per-key.md, docs/rules/http-server/a-session-store-answers-five-operations.md]
stages:
  - id: 1
    title: the rules say a session is written back per key, and the shared tier's client reads a hash
    status: done
    context: [crates/nvs-stdlib/src/cache/redis.rs, data/rules/http-server/a-session-is-loaded-once-and-written-back-per-key.json, docs/rules/core-api/session-roster.md, docs/rules/http-server/session-expiry-belongs-to-the-store.md]
  - id: 2
    title: the session is kept and written back per key on both stores
    status: done
    context: [crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-stdlib/src/session/sqlite.rs, crates/nvs-stdlib/src/cache/redis.rs]
  - id: 3
    title: tests prove that two requests keep both keys and that the script holds on a real store
    status: done
    context: [crates/nvs-stdlib/src/session/sqlite.rs, crates/nvs-stdlib/src/cache/redis.rs]
  - id: 4
    title: the pages say what two requests at once do, and the session benches are recorded again
    status: done
    context: [docs/examples/core/Session/set/about.md, docs/examples/core/Session/clear/about.md, docs/examples/core/Session/regenerate/about.md, benches/members/core/Session/get.scale.nvs, benches/members/README.md, docs/agent/prose.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: wire
    stage: 1
    name: "the shared tier's client reads a hash back as its fields and values, and a move to a taken identifier answers false"
    run: [cargo, test, --lib, --, a_hash_is_read_back_as_its_fields_and_values, a_move_to_a_taken_identifier_answers_false_and_a_move_from_a_gone_one_answers_gone]
    timeout: 1800
    tests:
      - a_hash_is_read_back_as_its_fields_and_values
      - a_move_to_a_taken_identifier_answers_false_and_a_move_from_a_gone_one_answers_gone
  - id: per-key
    stage: 3
    name: "on every store, two requests that change different keys keep both, two that change one key keep the last, a key a request only read is not written back, clear deletes every key the store holds, a session with no keys left still exists, regenerate moves a key another request wrote, and a destroyed session is not written back"
    run: [cargo, test, --lib, --, two_requests_that_change_different_keys_keep_both_on_any_store, two_requests_that_change_one_key_keep_the_last_on_any_store, a_key_a_request_only_read_is_not_written_back_on_any_store, clear_deletes_every_key_the_store_holds_when_the_request_ends, a_session_whose_last_key_is_removed_still_exists_on_any_store, regenerate_moves_a_key_another_request_wrote_on_any_store, a_destroyed_session_is_not_written_back_on_any_store, a_regenerated_session_moves_only_a_row_that_still_exists, get_and_set_decode_and_encode_only_their_own_key]
    timeout: 1800
    tests:
      - two_requests_that_change_different_keys_keep_both_on_any_store
      - two_requests_that_change_one_key_keep_the_last_on_any_store
      - a_key_a_request_only_read_is_not_written_back_on_any_store
      - clear_deletes_every_key_the_store_holds_when_the_request_ends
      - a_session_whose_last_key_is_removed_still_exists_on_any_store
      - regenerate_moves_a_key_another_request_wrote_on_any_store
      - a_destroyed_session_is_not_written_back_on_any_store
      - a_regenerated_session_moves_only_a_row_that_still_exists
      - get_and_set_decode_and_encode_only_their_own_key
  - id: script
    stage: 3
    name: "on a real Redis, the write-back script writes nothing once the session is gone, keeps a session with no keys, refuses a session past the reply ceiling, and gives the same hash when it runs twice"
    run: [cargo, test, --lib, --, the_write_back_script_writes_nothing_once_the_session_is_gone, the_write_back_script_keeps_a_session_with_no_keys, the_write_back_script_refuses_a_session_past_the_reply_ceiling, the_write_back_script_run_twice_leaves_the_same_hash]
    timeout: 1800
    tests:
      - the_write_back_script_writes_nothing_once_the_session_is_gone
      - the_write_back_script_keeps_a_session_with_no_keys
      - the_write_back_script_refuses_a_session_past_the_reply_ceiling
      - the_write_back_script_run_twice_leaves_the_same_hash
---

# a session is written back per key, so two requests that change different keys keep both

## Why

A session is one record today. `Core\Session::start` loads all of it, and the write-back at the end of
the request replaces all of it (`crates/nvs-stdlib/src/session.rs:1727` `sent_to`). Two requests of one
session that run at once each write back the copy they loaded. So the second one to end undoes every
change the first one made, also to keys the second one never touched.

A busy user meets this: several tabs open, and pages that start many requests or jobs at once. A
developer expects two requests that write the *same* key to clash. Nobody expects a request that wrote
only `theme` to remove another request's change to `cart`.

There is also a security cost. A slow request that loaded `role = admin` and changes only `lastPage`
writes `role = admin` back over a demotion that another request made while it ran. Only a demotion
that also calls `regenerate` is safe today. With a write-back per key, the slow request sends
`lastPage` alone.

The rule declined a write-back per key because "one session becomes many entries", so `clear`,
`regenerate` and `destroy` would each need a step over many keys. That holds only when every session
key is its own store key. One Redis hash per session is still one Redis key, and the `db` store is a
local SQLite file with transactions. So every whole-session act stays one step, and the round trips
stay the same.

## Standing decisions

**The change (the person's call).** A session's write-back sends only the keys its request set or
removed. The API does not change. Two requests that change one key keep the last write, and a key's
value is written whole, so an array under one key is one value. Sessions written before this goal are
not converted: Novis is pre-alpha, and every user is signed out once.

**Each write-back is one atomic step (my call, from security and request isolation).** One `EVAL` on
the shared store and one transaction on the `db` store. So two requests that both write `userId` and
`role` leave all of one request's values, never half of each. Nothing is held between `start` and the
write-back, so there is still no lock and no request waits for another.

**The shared store's layout (my call).** One Redis hash per session. Its key is `key_of`'s
(`crates/nvs-stdlib/src/session.rs:612`) under a new `PREFIX` (`:575`), `nvs:session-keys:`. So an
entry an earlier build wrote as a string is never read as a hash: `HGETALL` of a string is a
`WRONGTYPE` error, and `start` would throw on it until the entry expired. The old entries expire by
their own TTL. Each session key is the field `k:` followed by the key's bytes. The one field `m` holds
the total length of all `k:` fields. A program cannot reach `m`, because every key a program writes is
prefixed. `m` also keeps a session with no keys in the store: Redis deletes a hash when its last field
goes, and the next write-back would then treat the session as destroyed. The shared tier is one Redis
server (`crates/nvs-stdlib/src/cache/redis.rs` follows no `MOVED`), so a command over two keys, as
`RENAMENX` is, is allowed. If cluster support arrives, the session key takes a hash tag.

**The five store operations (my call).** `issue` creates a session under a drawn identifier and
fails for one that is live. `load` returns every key, or absent. `save` applies one request's changes
to a session that is still live, refreshes its expiry, and writes nothing for one that is gone. `move`
renames a live session to a new identifier, and answers *done*, *the new one is taken* or *the old one
is gone*. `destroy` deletes the session and says whether there was one. `move` is new: today
`regenerate` is `issue` plus `destroy` and copies the request's own view. That view lacks a key another
request wrote meanwhile. On the shared store each operation is one command:

| Operation | Shared store | Replay after a lost reply |
|---|---|---|
| `issue` | `EVAL` of the issue script: if the key exists, `0`; else `HSET` `m` and the fields, `PEXPIRE` | `OnlyIfUnsent`, as `set_if_absent` (`redis.rs:317`) |
| `load` | `HGETALL` | `Idempotent` |
| `save` | `EVAL` of the write-back script | `Idempotent`: running it twice leaves the same hash |
| `move` | `RENAMENX`, which keeps the TTL | `OnlyIfUnsent`: a replay after it ran finds the old key gone |
| `destroy` | `DEL`, as now (`session.rs:823`) | `Idempotent` |

**The write-back script (my call).** Its arguments are the TTL, whether the request called `clear`,
the removed keys, the changed keys with their values, and the ceiling. It returns `0` when the key does
not exist and writes nothing. Then it works out the new total from `m` and each field's old and new
`HSTRLEN`. A total over the ceiling returns `-1` and writes nothing. Otherwise it runs, in order: on
`clear`, `HDEL` of every `k:` field; `HDEL` of the removed keys; `HSET` of the changed keys and `m`;
`PEXPIRE`. It returns `1`. Each step reads the hash as it is now, so a second run gives the same hash.

**What the ceiling counts (stage 1's call, from security and request isolation).** `HGETALL`'s reply
is bounded twice in `redis.rs`: `REPLY_CEILING` over all its bytes, and `STRING_ELEMENT_CEILING`,
65 536 elements, which is 32 768 fields. So `m` counts, for each `k:` field, the field name's bytes,
the value's bytes and 32 bytes of RESP framing, and the script compares that to the ceiling argument
`session.rs` passes (`REPLY_CEILING` less 64 KiB). The script also refuses a write-back that leaves
more than 32 767 `k:` fields (`HLEN` less one for `m`). A request past either bound gets the same
`-1`, and `send_at_end` reports the size. Counting only `HSTRLEN` would let many small keys pass the
script and still fail every later `HGETALL`, which wedges the session as before.

**The ceiling (my call).** `HGETALL` reads one reply, and `REPLY_CEILING` (`redis.rs:102`, 8 MiB)
bounds it. Today a request that writes a bigger record wedges its session until it expires, because
every later `start` fails to read it. With writes per key, two requests could reach the ceiling
together. So the script refuses a write-back that would take `m` past the ceiling. `send_at_end`
(`session.rs:1696`) reports that to the floor as a lost write that names the size. The `db` store has
no read ceiling today and gains none.

**The `db` store's layout (my call).** `crates/nvs-stdlib/src/session/sqlite.rs:55` `SCHEMA` drops
`nvs_sessions`, as it already drops the older `nvs_session` (`:57`). It creates two tables:
`nvs_session_heads (app, id, expires_at)` with the existing expiry index, and
`nvs_session_entries (app, id, key BLOB, value BLOB, PRIMARY KEY (app, id, key))`, both `WITHOUT
ROWID`. `load` is one statement: a `LEFT JOIN` from a live head to its entries. No row is absent, and
one row with a `NULL` key is a session with no keys. `save` is one transaction. It first runs `UPDATE`
on the head's expiry where `expires_at` is still ahead. No changed row means the session is gone, so
the transaction rolls back and returns `false`. Then it runs, in order: on `clear`, `DELETE` of every
entry; `DELETE` of the removed keys; and `INSERT … ON CONFLICT DO UPDATE` of the changed ones. `issue`
keeps the overwrite of an expired head that `:225-227` does now. `move` is one transaction that runs
only while the old head is live: insert the new head, which fails for a live one, re-point the
entries, and delete the old head. `destroy` deletes the head and its entries. The sweep (`:324`)
deletes the entries of expired heads, then the heads. `STATEMENTS` (`:75`) counts the new set. A
transaction lasts only as long as its own statements, on the core's own connection.

**The record in the request (my call).** `nvs_runtime::Session`
(`crates/nvs-runtime/src/ctx/wiring.rs:47`) replaces `record: Vec<u8>` with
`entries: BTreeMap<Vec<u8>, Vec<u8>>`, which maps each key to its value encoded alone by
`nvs_runtime::encode`. It also gains `touched: BTreeSet<Vec<u8>>`, the keys set or removed since the
load, and `cleared: bool`. `dirty` stays, for a drawn identifier that must create its row while still
empty (`session.rs:1277` `sent_id`). At the write-back, a touched key in `entries` is a change and a
touched key absent from `entries` is a removal. `get` decodes one value, and `set` encodes one value.
Today `record` (`session.rs:1171`) decodes the whole record on every call, and `write_back` (`:1225`)
encodes all of it on every `set`. A `BTreeMap` gives the script and the tests one write order. What it
spends: one encoded value per key and the touched key names, per in-flight request. Nothing grows with
sessions served.

**`clear` (my call).** `clear` empties the session. At the write-back it deletes every key the store
holds, also one that another request wrote after this request loaded. It is a whole-session act, like
`destroy`. A key set after `clear` in the same request is kept.

**`regenerate` (my call).** `move` takes the stored session to the new identifier in one step, so a key
another request wrote survives. The changes this request has not sent stay touched, and the write-back
sends them to the new identifier. A session with no row yet draws its identifier and has nothing to
move. *The old one is gone* and *the new one is taken* give what they give today
(`session.rs:1601-1621`): a fresh empty session with no cookie, and the same throw.

**The rules (my call).** The rule `a-session-is-loaded-once-and-written-whole` is renamed to
`docs/rules/http-server/a-session-is-loaded-once-and-written-back-per-key.md` and rewritten.
The rule `a-session-store-answers-four-operations` is renamed to
`docs/rules/http-server/a-session-store-answers-five-operations.md` and rewritten. Every citation follows
(`git grep` both ids; about forty lines, most in `session.rs`, `wiring.rs` and `sqlite.rs`). The first
rule's Declined list keeps the generation counter and the request-long lock. It drops "A per-key write
instead of a whole-record one" and adds two entries. **A whole-record write-back:** a request undoes
other requests' changes to keys it never touched, and can write back a stale value it only read.
**One store key per session key, with a key listing the others:** `clear`, `move` and `destroy` would
each touch many keys, which the shared tier cannot do in one step. A hash is the same layout as one
key.

**What it costs.** The same round trips as today: one at `start` and one at the write-back.
`regenerate` takes one round trip, where it took two. Fewer bytes are sent, because only changed keys
travel. Each `get` and `set` encodes or decodes one value, where it took the whole record. The Redis
side holds one small hash per session, which Redis stores compactly. The SQLite side holds one row per
key. The code gains two Lua scripts, a second table and the touched set.

**`clear` always earns a write (stage 2's call, from security and request isolation).** A `clear` on
a session with an identifier marks it dirty, even when this request saw it empty, because the store may
hold keys another request saved. A session with no identifier has nothing to clear and writes nothing.

**Lanes.** Stages 1 to 3 run in order: each one changes `session.rs` or `redis.rs`. Stage 4 touches
`docs/examples/core/Session/` and `benches/members/core/Session/` alone, and may run as a side goal
once stage 2 is done. Its bench record needs stage 2's code.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the rules say a session is written back per key, and the shared tier's client reads a hash

**Does:** writes both rules as § *Standing decisions* says, and teaches the RESP client the commands
and the reply shape stage 2 needs. Nothing a session does changes yet.

- [x] `docs/rules/http-server/a-session-is-loaded-once-and-written-back-per-key.md:1`: renamed to
  `a-session-is-loaded-once-and-written-back-per-key.md` with its `data/rules/http-server/` JSON, and
  rewritten. It covers what a write-back sends, one atomic step per write-back, the last write winning
  for one key, `clear`, the ceiling, and that a destroyed session stays destroyed. Its Declined list is
  the one the standing decision gives. The new title is "A session is loaded once at `start`, and only
  the keys a request changed are written back, in one step and never locked".
- [x] `docs/rules/http-server/a-session-store-answers-five-operations.md:1`: renamed to
  `a-session-store-answers-five-operations.md` with its JSON, and rewritten with the five operations
  and their commands from the table. The strict-id half (`:10-14`) stays as it is.
- [x] Every citation of both old ids moves to the new ones (`git grep` lists them).
  `docs/rules/core-api/session-roster.md:12` says `regenerate` moves the session.
  `docs/ground-rules.md:980` and `:983` take the new titles. Then `bun nv rules --render`, and
  `bun nv rules --check` is green.
- [x] `crates/nvs-stdlib/src/cache/redis.rs:659` `Reply` gains an array of bulk strings, read at `:725`
  `one`. The client gains `hash_all` (`HGETALL`, `Idempotent`, returns `None` for an empty array) and
  `rename_if_absent` (`RENAMENX`, `OnlyIfUnsent`). `RENAMENX` answers `0` for a taken new key, and the
  error `ERR no such key` for a gone old one, which is read as *gone*, not as a failure. `eval`
  (`:414`) takes the `Replay` from its caller, so the write-back script can be `Idempotent`. The
  limiter (`crate::ratelimit`) and the lease (`:440`) keep `OnlyIfUnsent`. The module doc's list of
  commands (`:33-55`) is rewritten to match.
- [x] `crates/nvs-stdlib/src/cache/redis.rs:994`: the two fake-store tests the `wire` check names,
  beside `a_put_and_a_get_are_one_set_and_one_get_on_one_connection`, and the check run once.

### 2. the session is kept and written back per key on both stores

**Does:** the keystone. The request keeps the session per key, both stores read and write it per key
through the five operations, and `regenerate` moves it. The existing tests are changed to the new
shape as they break.

**Split:** once the code builds, a writing subagent changes the fake store `serving`
(`crates/nvs-stdlib/src/session.rs:1924`) and the existing tests in `mod tests` (`:1799`). The fake
learns `HGETALL`, `RENAMENX` and the two scripts. It matches a script by exact text against the module
constants and applies the same steps in Rust, and any other script panics, as an unknown command does
now. The real script is proved against Redis in stage 3. The session runs `cargo test --lib --
session::` once after the subagent ends.

- [x] `crates/nvs-runtime/src/ctx/wiring.rs:47`: `Session` takes `entries`, `touched` and `cleared`
  in place of `record`, with doc comments that state what each spends. Every place that builds a
  `Session` follows: `crates/nvs-stdlib/src/session.rs:975`, `:996` `fresh_session` and `:1626`.
- [x] `crates/nvs-stdlib/src/session.rs:1171` `record` and `:1225` `write_back` become a read of one
  key and a write of one key. `get` (`:1357`), `set` (`:1396`), `setSecret` (`:1437`), `getSecret`
  (`:1469`), `remove` (`:1493`), `clear` (`:1520`), `put_bytes` (`:1308`) and `take_bytes` (`:1327`)
  use them. A removal of an absent key still marks nothing (`:1320-1322`).
- [x] `crates/nvs-stdlib/src/session.rs:575` `PREFIX` becomes `nvs:session-keys:`. `:809-870` become
  the five shared-store operations with the two scripts as module constants. `:685-742` dispatch the
  five on both stores, with `moved` beside `forgot`. The `a_record_is_keyed_under_its_own_prefix`
  test (`:3323`) takes the new prefix.
- [x] `crates/nvs-stdlib/src/session/sqlite.rs:55` `SCHEMA`, `:75` `STATEMENTS`, `:161` `load`,
  `:216` `create`, `:245` `replace`, `:272` `written`, `:306` `destroy`, `:324` `sweep`, `:340` `rows`
  and `:354` `lapse` follow the `db` layout, and a new `move` is added. The test at `:397` becomes
  `two_requests_of_one_session_keep_both_keys_in_the_file`.
- [x] `crates/nvs-stdlib/src/session.rs:1601` `regenerated` uses `moved`, and `:1727` `sent_to` sends
  the touched keys through `save` or `issue`. `:1696` `send_at_end` reports the ceiling refusal with
  its size. Their doc comments, the `:1388` comment about last-write-wins and the module doc say what
  is now true.
- [x] The cards in end-user prose (`AGENTS.md` § *Text an end user reads*). `SET_DOC` (`:362`) gains
  "Two requests that change different keys at the same time keep both changes. If they change the
  same key, the request that ends last wins." `CLEAR_DOC` (`:431`) says that `clear()` also deletes
  keys that another request saved meanwhile. `REGENERATE_DOC` (`:445`) says that every key is kept.
- [x] The split's subagent ends, and `cargo test --lib -- session::` runs once and is green.

### 3. tests prove that two requests keep both keys and that the script holds on a real store

**Does:** the tests the `per-key` and `script` checks name. The cross-store cases run each scenario on
the fake shared store and on the `db` file through `a_store` (`crates/nvs-stdlib/src/session.rs:2379`),
as `a_destroyed_session_is_not_written_back_on_any_store` (`:2409`) does. Two requests are two
contexts, each with its own `start`, whose write-backs run in the order the case names.

**Split:** one writing subagent writes the seven cross-store cases into `mod tests`, copying the shape
of `:2409`. The session writes the four real-store cases. They share `session.rs`, so the subagent
ends first.

- [x] `crates/nvs-stdlib/src/session.rs:2409`: the seven cases the `per-key` check names that do not
  exist yet. `get_and_set_decode_and_encode_only_their_own_key` puts a large value under one key and
  shows that `get` of another key does not decode it. Use a count or an allocation probe the crate
  already has. Do not time it.
- [x] `crates/nvs-stdlib/src/session.rs:1799`: a `real_store(label)` helper in the shape of
  `lease_case` (`crates/nvs-stdlib/src/cache/redis.rs:1386`). Where no store answers, it prints the
  compose command and returns `None`. CI starts Redis on every runner (`.github/workflows/ci.yml:258-273`).
  With it, the four cases the `script` check names. The ceiling case sets a small ceiling through
  the script's argument. It does not write 8 MiB.
- [x] Both checks run once.

### 4. the pages say what two requests at once do, and the session benches are recorded again

**Does:** the user pages say what stage 2's cards say, and the benches record the new cost. Every
line here is text an end user reads, held to `AGENTS.md` § *Text an end user reads*.

- [x] `docs/examples/core/Session/set/about.md:13`: a `**Good to know:**` paragraph after the
  paragraph about saving once. Two requests that change different keys at the same time keep both
  changes. If they change the same key, the request that ends last wins.
- [x] `docs/examples/core/Session/clear/about.md:1` and `docs/examples/core/Session/regenerate/about.md:9`:
  one sentence each, matching `CLEAR_DOC` and `REGENERATE_DOC`.
- [x] `bun nv proofs --record-perf` records the `benches/members/core/Session/` members again, because
  the implementing file's hash changed (`benches/members/README.md:186`). `get.scale` reads `user`
  while `cart` grows, so its growth should now be constant. Write the measured growth into the
  handoff, whatever it is.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and this goal's own tests at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done. The session is kept per key (`nvs_runtime::Session` has `entries`, `touched`
and `cleared`), and both stores answer the five operations. On the shared store: `load` (`HGETALL`),
`issue` (`EVAL` of `ISSUE`, `session.rs:607`), `save` (`EVAL` of `SAVE`, `:627`), `relocate` (the rule's
`move`, `RENAMENX`) and `destroy`. On the `db` store, the same five in `sqlite.rs`, over
`nvs_session_heads` and `nvs_session_entries`, each write in one `BEGIN IMMEDIATE` transaction
(`atomically`). `set_if_present` is removed from `redis.rs`, and `REPLY_CEILING` and
`STRING_ELEMENT_CEILING` are `pub(crate)`. The cards say what two requests at once do. `set`, `remove`
and `setSecret` no longer throw `ParseError`, because they decode nothing. One standing decision is new:
*`clear` always earns a write*.

Stage 3 is done. The seven `per-key` cases sit after
`a_regenerated_session_moves_only_a_row_that_still_exists`, on helpers `a_request`, `ended`,
`stored_under`, `int_bytes` and `a_store_with_record` (`session.rs:2907`). Two requests are two
`Ctx::buffered()` loaded from one store, each ended through `CASE_STORE`.
`get_and_set_decode_and_encode_only_their_own_key` (`:3136`) uses a cut 64 KiB value that cannot
decode, not a probe: `get` of the other key reads, and `set` leaves the cut bytes as they were.
`real_store(label)` (`:3180`) reaches the compose Redis on 16379 through `redis.rs`'s test
`REDIS` and `STORE_WAIT`, now `pub(crate)` in a `pub(crate) mod tests`. The four `script` cases
follow it; the ceiling case passes `200` as `ARGV[3]` through `eval`. Both checks ran once: 13 of 13
green, and the script cases reached the real store (no "asserted nothing" line). Clippy is green.

Stage 4 is done. The `set` page has a `**Good to know:**` paragraph on two requests at once, and the
`clear` and `regenerate` pages say what happens to a key another request saved meanwhile; `bun nv
prose` is green. All nine `Core\Session` perf records are measured on the new code. `get` and
`remove` failed their ramp as `complexity linear` with scale siblings that counted `cart`'s setup per
op, so both now declare `complexity constant`, and `get.scale.nvs` and `remove.scale.nvs` use the
`// bench: rounds 64` form (`benches/members/README.md:115-119`). Measured: `get` growth unclear
(the counts never agreed by the ceiling, in `growth_notes`), **size growth flat**; `remove` growth
flat, size flat; `set`, `regenerate` growth flat, size unclear; `getSecret`, `setSecret` flat and
flat; `start` flat. `bun nv proofs --gate` lists no `Session` gap. Other features' perf records are
stale from earlier goals (`time.rs` 86, `io.rs` 42, `str.rs` 40, `http.rs` 28, and more): the full
gate's debt, not this goal's.

The sweep's reds are stale: no push since nightly 44510687786; `env-secret-proofs` is retired.

### Next

Stage 5: close the goal as the pack says (a build, clippy and this goal's three acceptance checks
`wire`, `per-key`, `script` by name, unless the pack names this goal the full gate). Then `bun nv
owners --closes session-per-key`, move any decision only the goal's prose holds into
`crates/nvs-stdlib/src/session.rs`'s module doc or the two rules, and delete the goal file. At the full
gate, conformance cases about `Core\Session` throwing `ParseError` from `set` may go red, and the
pages under `docs/examples/core/Session/` may need `bun nv site --stamp`.

### Backlog
