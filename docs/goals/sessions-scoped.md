---
looper: 1
title: "a destroyed session stays destroyed, and a session opens only in its own application"
summary: "Reached when the end-of-request write changes only a row that still exists on every session store, a session row is keyed by its application as well as its id, and Core\\Csrf::token writes the store only when it draws the session's identifier."
context: [crates/nvs-stdlib/src/session.rs, crates/nvs-stdlib/src/session/sqlite.rs, docs/rules/http-server/a-session-store-answers-four-operations.md]
stages:
  - id: 1
    title: only create inserts a session row
    status: open
    context: [crates/nvs-stdlib/src/cache/redis.rs, crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-config/src/session.rs, docs/rules/http-server/a-session-is-loaded-once-and-written-whole.md, docs/rules/security/a-session-id-the-store-did-not-issue-is-rejected.md]
  - id: 2
    title: a session row is keyed by its application
    status: open
    context: [crates/nvs-stdlib/src/cache.rs, crates/nvs-cli/tests/per_mount_config.rs, docs/rules/http-server/no-session-block-means-the-data-folder-store.md, docs/rules/concurrency/the-memory-tier-is-one-store-per-process.md]
  - id: 3
    title: "Core\\Csrf::token writes the session only when it draws the identifier"
    status: open
    context: [crates/nvs-stdlib/src/csrf.rs, docs/rules/security/csrf-is-on-by-default.md]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: destroyed-stays-destroyed
    stage: 1
    name: "on every session store, a request that ends after destroy or regenerate writes no row, and only create inserts one"
    run: [cargo, test, --lib, --, a_destroyed_session_is_not_written_back_on_any_store, a_regenerated_session_moves_only_a_row_that_still_exists, only_create_inserts_a_session_row]
    timeout: 1800
    tests:
      - a_destroyed_session_is_not_written_back_on_any_store
      - a_regenerated_session_moves_only_a_row_that_still_exists
      - only_create_inserts_a_session_row
  - id: keyed-by-application
    stage: 2
    name: "on every session store, a row one application wrote is absent for another application that presents its id"
    run: [cargo, test, --lib, --, a_session_one_application_wrote_is_absent_in_another_on_every_store]
    timeout: 1800
    tests:
      - a_session_one_application_wrote_is_absent_in_another_on_every_store
  - id: two-mounts-two-sessions
    stage: 2
    name: "two mounts under two [[app]] blocks do not open each other's sessions on the wire"
    run: [cargo, test, --test, per_mount_config, --, two_mounts_do_not_open_each_others_sessions]
    timeout: 1800
    tests:
      - two_mounts_do_not_open_each_others_sessions
  - id: token-render-writes-nothing
    stage: 3
    name: "a token rendered for a loaded session sends no store write, and a new visitor's first token sends one"
    run: [cargo, test, --lib, --, a_token_for_a_loaded_session_makes_no_store_write, a_token_for_a_new_visitor_creates_the_row_once]
    timeout: 1800
    tests:
      - a_token_for_a_loaded_session_makes_no_store_write
      - a_token_for_a_new_visitor_creates_the_row_once
---

# a destroyed session stays destroyed, and a session opens only in its own application

## Why

Three defects share `crates/nvs-stdlib/src/session.rs`. Two are security defects and one is a cost on
the request path.

**A destroyed session comes back.** The end-of-request write-back (`send_at_end`,
`crates/nvs-stdlib/src/session.rs:1611`) saves the record under the id the request loaded at `start`.
The save creates the row when it is missing. On the `db` store it is
`INSERT ... ON CONFLICT (id) DO UPDATE` (`crates/nvs-stdlib/src/session/sqlite.rs:167`). On the
shared store it is a plain `SET ... PX` (`crates/nvs-stdlib/src/cache/redis.rs:296`). Nothing records
whether this request drew the id or loaded it. So a request that loaded a session and changed it is a
pending insert for as long as it runs. When another request calls `Core\Session::destroy`
(`crates/nvs-stdlib/src/session.rs:1578`) or `regenerate` (`crates/nvs-stdlib/src/session.rs:1536`),
the row is deleted. When the first request ends, the row is written again, signed in, with a fresh
`[session] ttl`. A remote client that holds a copied session cookie reaches this. It keeps one slow
request open on any handler that writes the session, and the owner's sign-out is undone.
`regenerate` has the same gap from the other side: it inserts the record it loaded under a new id
without asking whether the old row still exists.
`rule:http-server/a-session-is-loaded-once-and-written-whole` accepts last-write-wins between two
writers of a live record. It does not accept a write after a delete, and
`rule:security/a-session-id-the-store-did-not-issue-is-rejected` says the opposite.

**A session opens in every application on its store.** A row is keyed by its id alone: `key_of`
(`crates/nvs-stdlib/src/session.rs:605`) and the `nvs_session` table
(`crates/nvs-stdlib/src/session/sqlite.rs:46`). The default store is one `session.sqlite` in the data
folder (`crates/nvs-config/src/data.rs:170`), which 780e9653b2 made the store of every deployment
that configures none. `start` accepts any id with a live row
(`crates/nvs-stdlib/src/session.rs:929`). So two `[[app]]` blocks on one server accept each other's
sessions. A person with an account in one application sends its cookie to the other and is signed in
there with the same record. The same crate already treats this boundary as real: a memory cache key
starts with the application (`crates/nvs-stdlib/src/cache.rs:1553`), and a sealed session secret is
bound to it (`crates/nvs-stdlib/src/session.rs:1067`).

**Every page with a form writes the store.** `sent_id` sets `dirty` on every call
(`crates/nvs-stdlib/src/session.rs:1254`), and `Core\Csrf::token` calls it
(`crates/nvs-stdlib/src/csrf.rs:410`). 89b33475bd bound the token to the session and left this. The
row must be written only for a session with no row yet. For a loaded session the record is unchanged,
yet the request pays one more store round trip: one statement under SQLite's single writer lock, or
one `SET` on the shared store. It also makes every such request a pending writer for the first
defect.

## Standing decisions

**The end-of-request write is an update of a row that still exists, on every store.** A store answers
two writes. `create` inserts and never replaces: a plain `INSERT` on the `db` store and `SET ... NX
PX` on the shared store. `replace` changes only a live row: `UPDATE ... WHERE id = ? AND expires_at >
now` on the `db` store and `SET ... XX PX` on the shared store. `nvs_runtime::Session` gains one
`bool`, `stored`, which says whether the store holds a row under its id. `start` sets it for a loaded
row. The write-back calls `replace` when it is set and `create` when it is not, and sets it after a
`create`. A `replace` that finds no row drops the request's changes and reports nothing, because that
is what a destroyed session means. **What it spends:** one `bool` per in-flight session, and no extra
round trip. Usability: a write that races a sign-out is lost, which the rule already says of two
writers. Declined: a generation counter on the row, which needs a read before every write. Declined:
a lock, which `rule:http-server/a-session-is-loaded-once-and-written-whole` declines. Safe default if
a store cannot express `replace` in one command: that store's write-back is skipped for a loaded
session whose row a read finds absent, and the read is the cost.

**`regenerate` moves only a session that still exists.** For a stored session it creates the row
under the new id, then destroys the old row, as `rule:core-api/session-roster` orders the steps.
`destroy` on a store returns whether it removed a row. When it removed none, another request
destroyed the session first. `regenerate` then destroys the new row, sends no cookie, and leaves the
request with a fresh, empty session with no identifier, as `start` does for an absent id. For a
session this request drew and has not stored, `regenerate` draws a new id and sends its cookie with
no store call. **What it spends:** nothing on the ordinary path, and one more delete in the race.
Declined: throwing from `regenerate`, because a sign-in handler would fail for a visitor whose old
session merely ended.

**The store list is `nvs_config::session::BACKENDS`.** It holds `shared` and `db`
(`crates/nvs-config/src/session.rs:58`), and the memory tier is refused at boot. Each test that says
"every store" loops over `BACKENDS` and builds the store with a `match` that has no wildcard arm. A
backend added later fails to compile there until it has a case. The shared store runs against the
fake server in the test module (`crates/nvs-stdlib/src/session.rs:1786`), which learns `NX`, `XX`
and the nil reply. The `db` store runs against a scratch file.

**A session row's key carries the application, read by `crate::cache::application`.** That function
(`crates/nvs-stdlib/src/cache.rs:1580`) returns the canonical path of the innermost `[[app]]` block
that matched, and the memory cache and the sealed session secret already use it. The session key
reuses it, so the three cannot disagree. The configuration generation is not in the key, because a
reload must not sign everybody out. On the shared store the key is the prefix, the application, one
`NUL` and the id. On the `db` store the table is `nvs_sessions` with `PRIMARY KEY (app, id)`. No new
setting. **What it spends:** the application's path bytes once per key, per store call, and nothing
else. Usability: two mounts that share a login share one `[[app]]` block, and they still do. A file
under its own `entry` block is its own application, as it already is for the memory cache. A
deployment with no `[[app]]` block is one application with an empty identity, as
`rule:concurrency/the-memory-tier-is-one-store-per-process` says. Two programs that share one data
folder and write no block still share sessions. The rule fragment says each needs its own `[[app]]`
block or its own `--data` folder. Declined: the app folder as the identity, because two entry files
in two folders of one site would stop sharing a session. Declined: a `[session] scope` key, because
`[[app]]` already names an application.

**Sessions written under the old key stop working once.** No Novis code has shipped, so no migration
is written. The `db` store's schema drops the old `nvs_session` table when a connection opens, one
statement per core per process start. Old shared-store keys match nothing and expire by their own
`ttl`.

**`Core\Csrf::token` marks the session changed only when it draws the identifier.** The token itself
is never stored: it is sealed over the session's id on each call. So the only store write a token
needs is the row of a session that had none. `sent_id` sets `dirty` inside its fresh branch alone.
A token render no longer extends a session's lifetime, and a plain read never did. **What it
spends:** nothing, and it removes one store write per page that renders a token. The guard counts
store writes, never time.

**The proofs are Rust tests, and no attack file is added.** The first defect needs two requests in
flight on one session, and an attack file runs one request at a time. The store tests drive the two
orders directly, and the wire test serves two mounts.

**Lanes:** one lane. All three stages edit `crates/nvs-stdlib/src/session.rs`, so they run in order.

## Stages

### 1. only create inserts a session row

**Does:** split the store's save into `create` and `replace`, make the write-back and `regenerate`
use them, and prove the destroy-while-in-flight case on every store.

- [ ] `nvs_runtime::Session` gains `stored: bool`, documented as the standing decision states it
  (`rule:http-server/a-session-is-loaded-once-and-written-whole`,
  `crates/nvs-runtime/src/ctx/wiring.rs:47`). Every constructor sets it: `start`'s two
  (`crates/nvs-stdlib/src/session.rs:931`, `crates/nvs-stdlib/src/session.rs:943`), `regenerate`'s
  (`crates/nvs-stdlib/src/session.rs:1541`), and the test fixtures in `session.rs`,
  `crates/nvs-stdlib/src/csrf.rs:494` and `crates/nvs-stdlib/src/passkey.rs:1946`.
- [ ] The shared store: `Connection` gains the two writes, `SET ... NX PX` and `SET ... XX PX`, each
  returning whether the key was written, and `del` returns whether a key was removed
  (`rule:http-server/a-session-store-answers-four-operations`,
  `crates/nvs-stdlib/src/cache/redis.rs:288`, `crates/nvs-stdlib/src/cache/redis.rs:356`).
- [ ] The `db` store: `save` becomes `create`, a plain `INSERT`, and `replace`, an `UPDATE` of a row
  whose `expires_at` is still ahead. Both keep the sweep. `destroy` returns whether it deleted a row
  (`rule:http-server/session-expiry-belongs-to-the-store`,
  `crates/nvs-stdlib/src/session/sqlite.rs:163`, `crates/nvs-stdlib/src/session/sqlite.rs:189`). The
  module doc's cost line says an update, not an upsert (`crates/nvs-stdlib/src/session/sqlite.rs:27`).
- [ ] `session.rs`: `save` and `put` split into the two writes for both stores, and `forgot` returns
  the store's answer (`crates/nvs-stdlib/src/session.rs:696`, `crates/nvs-stdlib/src/session.rs:715`,
  `crates/nvs-stdlib/src/session.rs:819`).
- [ ] `send_at_end` calls `replace` for a stored session and `create` for one this request drew, and
  a `replace` that changed no row is not reported (`crates/nvs-stdlib/src/session.rs:1611`).
- [ ] `regenerate` follows its standing decision: no store call for a session not yet stored, and a
  fresh empty session when the old row was already gone
  (`rule:core-api/session-roster`, `crates/nvs-stdlib/src/session.rs:1516`).
- [ ] The fake shared server in the test module answers `NX`, `XX` and a nil reply
  (`crates/nvs-stdlib/src/session.rs:1786`).
- [ ] Test `a_destroyed_session_is_not_written_back_on_any_store`, in the `session.rs` test module.
  For each row of `nvs_config::session::BACKENDS`: one context loads a stored session and changes
  it, the row is destroyed through the store, the context ends, and the store holds no row under
  that id (`crates/nvs-stdlib/src/session.rs:1681`).
- [ ] Test `a_regenerated_session_moves_only_a_row_that_still_exists`, same loop: a context loads a
  session, the row is destroyed, the context calls `regenerate`, and the store holds no row at all
  and the context holds an empty session with no identifier.
- [ ] Test `only_create_inserts_a_session_row`, same loop: `replace` on an absent id and on an
  expired row writes nothing, and `create` on a held id fails and leaves the held record.
- [ ] The existing tests that assert the upsert follow: the write-back, `regenerate` and `destroy`
  cases (`crates/nvs-stdlib/src/session.rs:2057`, `crates/nvs-stdlib/src/session.rs:2145`,
  `crates/nvs-stdlib/src/session.rs:2207`).
- [ ] The module doc and the member docs of `start`, `regenerate` and `destroy` say what is true now
  (`crates/nvs-stdlib/src/session.rs:1`).
- [ ] `rule:http-server/a-session-store-answers-four-operations` says `save` replaces a row that
  still exists and `issue` is the only insert
  (`docs/rules/http-server/a-session-store-answers-four-operations.md:1`).
- [ ] `rule:http-server/a-session-is-loaded-once-and-written-whole` says a write after `destroy` or
  `regenerate` is dropped, and what `regenerate` does when the old row is gone
  (`docs/rules/http-server/a-session-is-loaded-once-and-written-whole.md:1`).
- [ ] `rule:http-server/no-session-block-means-the-data-folder-store` says "one update" in its
  latency line (`docs/rules/http-server/no-session-block-means-the-data-folder-store.md:23`).

### 2. a session row is keyed by its application

**Does:** put the application into the key of both stores, and prove that a second application does
not open the row.

- [ ] `key_of` takes the application and the id, and `load`, `create`, `replace` and `destroy` pass
  `crate::cache::application(ctx)` on both stores
  (`rule:http-server/a-session-store-answers-four-operations`,
  `crates/nvs-stdlib/src/session.rs:605`, `crates/nvs-stdlib/src/cache.rs:1580`).
- [ ] The `db` store's schema: table `nvs_sessions` with an `app` column and `PRIMARY KEY (app, id)`,
  its expiry index, and one `DROP TABLE IF EXISTS nvs_session`. Every statement binds the
  application (`crates/nvs-stdlib/src/session/sqlite.rs:43`).
- [ ] `fetched`, the two writes and `forgot` carry the application from the context
  (`crates/nvs-stdlib/src/session.rs:679`).
- [ ] Test `a_session_one_application_wrote_is_absent_in_another_on_every_store`, in the `session.rs`
  test module, over `BACKENDS`: a context under one `[[app]]` block stores a session, and a context
  under a second block that presents the same id gets a fresh, empty session. A context under the
  first block still loads it (`crates/nvs-stdlib/src/session.rs:1681`).
- [ ] Test `two_mounts_do_not_open_each_others_sessions`, beside
  `two_mounts_do_not_share_memory_tier_cache_entries`: two mounts under two `[[app]]` blocks, a
  session written through the first, and the first mount's cookie sent to the second
  (`crates/nvs-cli/tests/per_mount_config.rs:487`).
- [ ] The existing key tests follow, `a_record_is_keyed_under_its_own_prefix` among them
  (`crates/nvs-stdlib/src/session.rs:2749`).
- [ ] `rule:http-server/no-session-block-means-the-data-folder-store` says a row belongs to one
  application, what a deployment with no `[[app]]` block is, and that two programs on one data
  folder need a block each or a `--data` folder each
  (`docs/rules/http-server/no-session-block-means-the-data-folder-store.md:1`).
- [ ] `rule:security/a-session-id-the-store-did-not-issue-is-rejected` says an id another
  application's row holds is absent
  (`docs/rules/security/a-session-id-the-store-did-not-issue-is-rejected.md:1`).
- [ ] The doc of `Folder::sessions` and the `sqlite` module doc name the new key
  (`crates/nvs-config/src/data.rs:166`, `crates/nvs-stdlib/src/session/sqlite.rs:1`).

### 3. Core\Csrf::token writes the session only when it draws the identifier

**Does:** stop a token render from marking a loaded session changed, and count the store writes.

- [ ] `sent_id` sets `dirty` inside its fresh branch alone, and its doc comment says so
  (`rule:security/csrf-is-on-by-default`, `crates/nvs-stdlib/src/session.rs:1237`).
- [ ] Test `a_token_for_a_loaded_session_makes_no_store_write`, in the `session.rs` test module,
  against the fake shared server with a counter of write commands. A context loads a stored session,
  calls `Core\Csrf::token` twice and ends: the counter is 0. The same context shape with one
  `Core\Session::set` ends at 1 (`crates/nvs-stdlib/src/session.rs:1681`,
  `crates/nvs-stdlib/src/csrf.rs:402`).
- [ ] Test `a_token_for_a_new_visitor_creates_the_row_once`: a context with a fresh session calls
  `token` twice and ends, the counter is 1, and the row exists under the id the cookie carries.
- [ ] `rule:security/csrf-is-on-by-default` says a token for a session that has a row writes nothing
  (`docs/rules/security/csrf-is-on-by-default.md:30`).
- [ ] `rule:http-server/a-session-is-loaded-once-and-written-whole` names the token render beside
  the read-only request (`docs/rules/http-server/a-session-is-loaded-once-and-written-whole.md:4`).

### 4. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built yet. The tree holds the upsert write-back, rows keyed by id alone, and a `sent_id`
that marks every session changed. Both stores and the fake shared server in the test module exist.

### Next

Stage 1, only create inserts a session row. Files: `crates/nvs-stdlib/src/session.rs`,
`crates/nvs-stdlib/src/session/sqlite.rs`, `crates/nvs-stdlib/src/cache/redis.rs`,
`crates/nvs-runtime/src/ctx/wiring.rs` and three rule fragments.

- [ ] `crates/nvs-runtime/src/ctx/wiring.rs:47`: `nvs_runtime::Session` gains `stored: bool`, and
  every constructor sets it (`crates/nvs-stdlib/src/session.rs:931`,
  `crates/nvs-stdlib/src/session.rs:943`, `crates/nvs-stdlib/src/session.rs:1541`,
  `crates/nvs-stdlib/src/csrf.rs:494`, `crates/nvs-stdlib/src/passkey.rs:1946`).
- [ ] `crates/nvs-stdlib/src/cache/redis.rs:288`: the shared store's `SET ... NX PX` and
  `SET ... XX PX`, and a `del` that returns whether it removed a key
  (`crates/nvs-stdlib/src/cache/redis.rs:356`).
- [ ] `crates/nvs-stdlib/src/session/sqlite.rs:163`: the `db` store's `create` and `replace`, a
  `destroy` that returns whether it deleted a row, and the cost line
  (`crates/nvs-stdlib/src/session/sqlite.rs:189`, `crates/nvs-stdlib/src/session/sqlite.rs:27`).
- [ ] `crates/nvs-stdlib/src/session.rs:696`: `save`, `put` and `forgot` split and return the
  store's answer (`crates/nvs-stdlib/src/session.rs:715`, `crates/nvs-stdlib/src/session.rs:819`).
- [ ] `crates/nvs-stdlib/src/session.rs:1611`: `send_at_end` replaces a stored session and creates
  a drawn one.
- [ ] `crates/nvs-stdlib/src/session.rs:1516`: `regenerate` moves only a session that still exists.
- [ ] `crates/nvs-stdlib/src/session.rs:1786`: the fake shared server answers `NX`, `XX` and nil.
- [ ] `crates/nvs-stdlib/src/session.rs:1681`: tests
  `a_destroyed_session_is_not_written_back_on_any_store`,
  `a_regenerated_session_moves_only_a_row_that_still_exists` and `only_create_inserts_a_session_row`
  over `nvs_config::session::BACKENDS` (`crates/nvs-config/src/session.rs:58`).
- [ ] `crates/nvs-stdlib/src/session.rs:2057`: the existing write-back, `regenerate` and `destroy`
  tests follow (`crates/nvs-stdlib/src/session.rs:2145`, `crates/nvs-stdlib/src/session.rs:2207`).
- [ ] `crates/nvs-stdlib/src/session.rs:1`: the module doc and the member docs.
- [ ] `docs/rules/http-server/a-session-store-answers-four-operations.md:1`: the three rule
  fragments (`docs/rules/http-server/a-session-is-loaded-once-and-written-whole.md:1`,
  `docs/rules/http-server/no-session-block-means-the-data-folder-store.md:23`).

No existing test is expected red before the stage's own edits.

### Backlog

- Two programs that share one data folder and write no `[[app]]` block still share sessions.
