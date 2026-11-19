---
looper: 1
title: a session keeps short-lived and long-lived values and opens itself on first use
summary: "Reached when Core\\Session has setShortLived, setLongLived, clearShortLived, clearLongLived and clearAll in place of set, clear and destroy, opens itself on the first call with start removed, keeps short-lived values only while the browser keeps its session cookie and the short clock runs, keeps long-lived values for [session] long_lived, the checker refuses every session write inside Html::later and Task::afterResponse, and the rules, examples, pages and every .nvs file say so."
context: [crates/nvs-stdlib/src/session.rs, crates/nvs-stdlib/src/session/sqlite.rs, crates/nvs-runtime/src/ctx/wiring.rs, docs/rules/core-api/session-roster.md, docs/rules/core-classes/a-session-opens-on-its-first-use.md, docs/rules/http-server/session-expiry-belongs-to-the-store.md]
stages:
  - id: 1
    title: the rules say what a session with two lifetimes is
    status: done
    context: [docs/rules/core-classes/a-session-opens-on-its-first-use.md, docs/rules/core-api/session-roster.md, docs/rules/http-server/session-expiry-belongs-to-the-store.md, docs/rules/http-server/a-session-holds-a-secret-only-sealed.md, docs/rules/core-classes/html-later.md, docs/rules/concurrency/deferred-work-cannot-write-the-response.md, crates/nvs-config/src/tree.rs, crates/nvs-config/src/defaults.rs, crates/nvs-config/src/default.toml]
  - id: 2
    title: both stores keep each value with its lifetime, and the two clocks and the short cookie work
    status: done
    context: [crates/nvs-stdlib/src/session.rs, crates/nvs-stdlib/src/session/sqlite.rs, crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-stdlib/src/cache/redis.rs]
  - id: 3
    title: the new members replace the old ones, and the session opens on its first use
    status: done
    context: [crates/nvs-stdlib/src/session.rs, crates/nvs-stdlib/src/csrf.rs, crates/nvs-stdlib/src/passkey.rs, crates/nvs-syntax/src/parser/mod.rs]
  - id: 4
    title: the checker refuses every session write inside Html::later and Task::afterResponse
    status: done
    context: [crates/nvs-types/src/response.rs, crates/nvs-diagnostics/src/code/types_e08.rs, tests/conformance/reject/html-later-anonymous-function-cannot-change-the-response-head.nvst, tests/conformance/core/after-response-a-response-write-does-not-compile.nvst]
  - id: 5
    title: tests prove the two lifetimes, the clocks, the clears and the first-use open on both stores
    status: done
    context: [crates/nvs-stdlib/src/session.rs]
  - id: 6
    title: every program, test and package in the tree uses the new members
    status: done
    context: [tests/conformance/core/, tests/hostile/core/, apps/, packages/rest/src/Session.nvs, benches/]
  - id: 7
    title: every Session member owes no feature proof
    status: open
    context: [docs/examples/core/Session/, benches/members/core/Session/, tests/hostile/core/Session/, docs/agent/prose.md]
  - id: 8
    title: the website pages use the new members
    status: open
    context: [website/src/content/docs/guides/cookbook/upload-file.mdx, website/src/content/docs/guides/example-apps/chat.mdx, website/src/content/docs/guides/example-apps/notes.mdx]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: checker
    stage: 4
    name: "a session write inside Html::later or Task::afterResponse does not compile, and a read does"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/html-later-anonymous-function-cannot-change-the-response-head.nvst, --case, tests/conformance/core/after-response-a-response-write-does-not-compile.nvst, --case, tests/conformance/core/session-a-read-inside-later-or-after-response-compiles.nvst]
    timeout: 1800
  - id: lifetimes
    stage: 5
    name: "on every store, short-lived values end with the browser session or the short clock, long-lived values outlive both, a read refreshes a clock only past half its time, each clear keeps the other kind under a new identifier, clearAll leaves nothing, and the session opens on its first use"
    run: [cargo, test, --lib, --, a_short_lived_value_is_gone_once_the_browser_drops_its_short_cookie_on_any_store, a_long_lived_value_survives_a_new_browser_session_on_any_store, a_short_lived_value_is_gone_after_the_short_clock_on_any_store, a_read_refreshes_a_clock_only_when_less_than_half_is_left_on_any_store, writing_a_key_with_the_other_lifetime_replaces_it_on_any_store, clear_short_lived_keeps_long_lived_values_under_a_new_identifier_on_any_store, clear_long_lived_keeps_short_lived_values_under_a_new_identifier_on_any_store, clear_all_leaves_no_row_and_expires_both_cookies, the_identifier_cookie_has_max_age_only_while_a_long_lived_value_exists, the_first_call_opens_the_session_and_a_request_that_never_calls_one_reaches_no_store, a_session_write_inside_later_or_after_response_throws_for_an_existing_session_too, an_identifier_opened_by_hand_sees_its_short_lived_values_without_a_short_cookie, a_long_lived_secret_survives_a_new_browser_session_and_stays_sealed_on_any_store]
    timeout: 1800
    tests:
      - a_short_lived_value_is_gone_once_the_browser_drops_its_short_cookie_on_any_store
      - a_long_lived_value_survives_a_new_browser_session_on_any_store
      - a_short_lived_value_is_gone_after_the_short_clock_on_any_store
      - a_read_refreshes_a_clock_only_when_less_than_half_is_left_on_any_store
      - writing_a_key_with_the_other_lifetime_replaces_it_on_any_store
      - clear_short_lived_keeps_long_lived_values_under_a_new_identifier_on_any_store
      - clear_long_lived_keeps_short_lived_values_under_a_new_identifier_on_any_store
      - clear_all_leaves_no_row_and_expires_both_cookies
      - the_identifier_cookie_has_max_age_only_while_a_long_lived_value_exists
      - the_first_call_opens_the_session_and_a_request_that_never_calls_one_reaches_no_store
      - a_session_write_inside_later_or_after_response_throws_for_an_existing_session_too
      - an_identifier_opened_by_hand_sees_its_short_lived_values_without_a_short_cookie
      - a_long_lived_secret_survives_a_new_browser_session_and_stays_sealed_on_any_store
  - id: set-short-lived-proofs
    stage: 7
    name: "Core\\Session::setShortLived owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Session::setShortLived"]
    timeout: 900
  - id: set-long-lived-proofs
    stage: 7
    name: "Core\\Session::setLongLived owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Session::setLongLived"]
    timeout: 900
  - id: clear-short-lived-proofs
    stage: 7
    name: "Core\\Session::clearShortLived owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Session::clearShortLived"]
    timeout: 900
  - id: clear-long-lived-proofs
    stage: 7
    name: "Core\\Session::clearLongLived owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Session::clearLongLived"]
    timeout: 900
  - id: clear-all-proofs
    stage: 7
    name: "Core\\Session::clearAll owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Session::clearAll"]
    timeout: 900
  - id: set-short-lived-secret-proofs
    stage: 7
    name: "Core\\Session::setShortLivedSecret owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Session::setShortLivedSecret"]
    timeout: 900
  - id: set-long-lived-secret-proofs
    stage: 7
    name: "Core\\Session::setLongLivedSecret owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Session::setLongLivedSecret"]
    timeout: 900
  - id: open-with-identifier-proofs
    stage: 7
    name: "Core\\Session::openWithIdentifier owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Session::openWithIdentifier"]
    timeout: 900
---

# a session keeps short-lived and long-lived values and opens itself on first use

## Why

A program that wants to remember something about a browser has two tools today. `Core\Session` keeps
values on the server, but they last only `[session] ttl` (2 hours) and die when the browser closes,
because the session cookie has no `Max-Age`. A cookie lasts longer, but it travels with every request,
the user can change it, and it holds a few kilobytes of text. So a theme, a language, a consent choice
or a guest cart ends up in a cookie, with all three costs.

`Core\Session` can carry both. Each value gets one of two lifetimes, and the developer picks it in the
method name. Short-lived values are today's session: they end at logout, when the browser closes, or
after `[session] short_lived`. Long-lived values survive all three and last `[session] long_lived`.
Logout clears the short-lived values and keeps the theme.

The session also stops needing `Core\Session::start`. Every program that uses a session must call it
first today, and forgetting it is a `RuntimeError` at the first call. Starting costs nothing until a
value is written (the rule `a-session-opens-on-its-first-use` replaces), so the first call can open the
session itself at the same cost.

Both changes are the person's design. Every choice below marked *my call* is the authoring session's,
made under the priority ordering, and stands until the person picks another.

## Standing decisions

**Gate: `session-per-key` is reached first.** This goal builds on its per-key store layout, its `move`
operation and its write-back script. The proof it landed is
`docs/rules/http-server/a-session-is-loaded-once-and-written-back-per-key.md` existing and
`session-per-key` gone from `docs/goals/README.md`'s chain. Every `session.rs` line anchor below was
taken before that goal ran. A session re-anchors by symbol (`bun nv peek --locate`) when a line moved.

**The surface (the person's call; `openWithIdentifier` is my call).**

| Method | What it does |
|---|---|
| `setShortLived($key, $value)` | Saves a value with the short lifetime |
| `setLongLived($key, $value)` | Saves a value with the long lifetime |
| `get($key)` | Reads a value of either lifetime |
| `remove($key)` | Deletes one value of either lifetime |
| `clearShortLived()` | Deletes every short-lived value, and moves the session to a new identifier |
| `clearLongLived()` | Deletes every long-lived value, and moves the session to a new identifier |
| `clearAll()` | Deletes the session, and expires both cookies |
| `regenerate()` | Moves the session to a new identifier and keeps every value |
| `setShortLivedSecret($key, $value, $keys)` | `setSecret` renamed: saves a sealed secret with the short lifetime |
| `setLongLivedSecret($key, $value, $keys)` | Saves a sealed secret with the long lifetime |
| `getSecret($key, $keys)` | Unchanged |
| `openWithIdentifier($id)` | Optional. Opens the session under an identifier the program read itself, such as a header |

`start`, `set`, `clear`, `destroy` and `setSecret` are removed, with no alias: Novis is pre-alpha.
There is no plain `set`, so every write states its lifetime. All values share one set of keys, and the
last write of a key sets its lifetime, for a sealed secret too. A long-lived secret is readable with the
identifier cookie alone after the browser closes. Its card says so, and says that a secret tied to a
login belongs in `setShortLivedSecret`. Sessions written before this goal are not converted.

**The session opens on its first use (the person's call).** The first call of any member loads the
session, once per request. That keeps today's cost: a request that calls no member reaches no store,
and a new visitor gets no row and no cookie until a value is written. A program answering no request
still throws `LogicError` at the first call (`rule:security/request-state-throws-in-an-isolate`). The
"second `start`" error goes away. `openWithIdentifier` replaces `start($id)`
(`docs/examples/core/Session/start/02-an-identifier-in-a-header.nvs:7`). It throws when the session is
already open in this request, naming the member that opened it.

**The two clocks (my call).** `[session] short_lived` replaces `[session] ttl`, with the same default,
`2h` (`crates/nvs-config/src/defaults.rs:213`). `[session] long_lived` is new, default `180d`. A
session record has a short clock and a long clock. The store expires the record when the long clock
ends, or the short clock where it has no long-lived value. A short-lived value is visible only while
the short clock runs. A write-back refreshes both clocks, as `save` refreshes the expiry today. **A read
also refreshes a clock when less than half of its time is left.** Today a read never refreshes, so a
signed-in user who only reads is signed out 2 hours after their last write. The half rule costs at most
one write per clock per half-lifetime of an active session, and a request that reads a fresh session
still makes one round trip. Safe fallback: refresh on writes only, and say so in the rule.

**The short cookie (my call, from security).** Today closing the browser ends the session, because the
session cookie has no `Max-Age`. A long-lived value needs a cookie that survives the browser closing,
and with one identifier the login would survive too. So the short-lived values are bound to a second
cookie:

- The identifier cookie, `[session] cookie`, has `Max-Age = long_lived` while the record has a
  long-lived value, and no `Max-Age` otherwise. It is resent with a new `Max-Age` when the long clock
  is refreshed.
- The short cookie, named `[session] cookie` plus `-short`, carries a 128-bit token from the same
  CSPRNG as the identifier. It never has `Max-Age`. It is drawn at the first short-lived write in a
  session with no valid token, and drawn again by `regenerate` and `clearShortLived`.
- The record keeps the token. At load, short-lived values are visible only when the presented token
  matches, compared in constant time. A mismatch or a missing token makes them absent, and the
  session counts as changed, so the write-back deletes them. That is one write per browser restart.
- A session opened by `openWithIdentifier` has no browser, so its short-lived values are bound to the
  identifier alone and no token is checked. The member's card says so.

Both cookies take every other attribute from `[http.cookies]`, as `issue_cookie`
(`crates/nvs-stdlib/src/session.rs:1017`) does now. A refresh that cannot send its header, because the
first use was inside `Html::later` or after the response, is skipped, and a later request does it.
Safe fallback: one cookie, and the rule states that a login survives a browser restart while the
session has a long-lived value.

**The store layout (my call), on top of `session-per-key`'s.** On the shared store, the hash's `k:`
field prefix becomes `s:` for a short-lived key and `l:` for a long-lived one, and the write-back
script deletes the other prefix's field when it writes a key. The fields `t` (the short token) and `x`
(the short clock, epoch milliseconds) join `m`. The hash's `PEXPIRE` is the long clock where an `l:`
field exists, and the short clock otherwise. On the `db` store, `nvs_session_entries` gains
`long INTEGER NOT NULL`, and `nvs_session_heads` gains `short_token BLOB` and `short_expires_at`.
`expires_at` stays the record's expiry. Every operation stays one command or one transaction.

**The clears (the person's names; the behaviour is my call).** Every clear sends a cookie, so all
three throw where the head is closed, as `regenerate` does.

- `clearShortLived` deletes the short-lived values, moves the session to a new identifier with
  `session-per-key`'s `move`, and draws a new short token. It is the logout.
- `clearLongLived` deletes the long-lived values and moves the session to a new identifier.
- `clearAll` deletes the row, sends both cookies with `Max-Age=0`, and leaves the request a fresh,
  empty session with no identifier. A later write in the same request draws a new identifier, like a
  first visit. Today `destroy` sends no cookie and makes every later call throw. Both of those end.
- Across requests, `session-per-key`'s rule holds: a write-back never brings back a session another
  request cleared.

**A session write is refused where the head is closed (the person's call; the run-time half is my
call).** At compile time, `HEAD_MEMBERS` (`crates/nvs-types/src/response.rs:197`) gains
`setShortLived`, `setLongLived`, `setShortLivedSecret`, `setLongLivedSecret`, `remove`,
`clearShortLived`, `clearLongLived`
and `clearAll` beside `regenerate`. So `E0842` and `E0857` refuse each inside an anonymous function
passed to `Core\Html::later` or `Core\Task::afterResponse`. `get` and `getSecret` stay allowed. At run
time, every write member calls `head_open` (`crates/nvs-stdlib/src/response.rs:1286`) first. Today
only the first write in a new session does. So a write in those places throws for every visitor, and a
test passed on a signed-in development machine fails the same way in production. The checker's reach
stays lexical. A function held in a variable is left to the run-time error.

**No new size limit (my call).** The record's bound is `session-per-key`'s ceiling. A long-lived row
costs its bytes for `long_lived`, so the store holds one row per browser that kept a value within
`long_lived`. That grows with visitors, not with requests, and the long clock bounds it.

**The rules (my call).**
- `docs/rules/core-classes/a-session-opens-on-its-first-use.md` is renamed
  `a-session-opens-on-its-first-use.md` and rewritten. Its Declined list drops "A session that starts
  on the first `Core\Session` access" and gains **An explicit `start`**: a program that forgets it
  throws, and the explicit line bought nothing that the first-use open does not keep.
- `docs/rules/core-api/session-roster.md` lists the members of the surface table.
- `docs/rules/http-server/session-expiry-belongs-to-the-store.md` describes the two clocks and the
  half rule.
- A new fragment, `docs/rules/http-server/a-session-value-is-short-lived-or-long-lived.md`, holds the
  two lifetimes, the short cookie and the clears. Its Declined list: **two stores behind two classes**
  (two concepts for one mechanism), **one cookie** (a login would survive a browser restart), and **a
  long clock that starts only when the browser presents the cookie again** (it keeps cookieless bots
  out, but a visitor who saw one page loses their consent choice).
- `docs/rules/http-server/a-session-holds-a-secret-only-sealed.md` names `setShortLivedSecret` and
  `setLongLivedSecret`, and says what a long-lived secret is exposed to.
- `docs/rules/core-classes/html-later.md` and
  `docs/rules/concurrency/deferred-work-cannot-write-the-response.md` say that a session write is
  refused there, and a read is not.

**What it costs.** Per request: one round trip at the first use, as today. A second cookie of 22
characters on requests of a session with short-lived values. A constant-time compare at load. Writes:
one per clock per half-lifetime of an active session, and one per browser restart. Store: the long
clock keeps rows for `long_lived`. Code: a second clock, a token and two field prefixes.

**Stage 2's store details (my call, built).** The long clock moves only where the identifier cookie
is sent with it: at the first long-lived write, at a long-lived write past half the clock, and by
the half rule at the first use. A write-back always moves the short clock, and sends the long one as
the request holds it. So the store and the cookie end together. On the shared store the long clock
is a hash field `y` beside `t` and `x`, because `HGETALL` does not return the expiry; the key prefix
is now `nvs:session-lives:`, so an older hash is never read. On the `db` store the head row also
gains `long_expires_at`, and a table without the new columns is dropped at open (sessions are not
converted). Each clock in a save moves forward only (`max`), so a request that loaded an older record
never shortens one another request refreshed. `http-server/session-expiry-belongs-to-the-store.md`
still says "a write-back refreshes both clocks"; stage 3 corrects it to this.

**Stage 3's clears (my call, built).** A clear of one lifetime is a flag the write-back carries
(`nvs_runtime::Session::cleared_short` / `cleared_long`, `Changes` the same), and the store deletes
every field of that lifetime in the save: `SAVE`'s `ARGV[5]` is `s`, `l`, `sl` or empty, and the
SQLite save runs `DELETE … AND long = ?`. Deleting only the keys the request saw would let a value
another request saved before the move survive a logout. `opened_by: &'static str` on the session
names the member `openWithIdentifier`'s error reports. `clearAll` keeps `by_hand` on the fresh
session, and sends no cookie for a session opened by hand. A failure to find an open session after
`first_use` is a fatal fault (`unstarted`), because no program can reach it.

**Lanes.** Stages 1 to 5 run in order: each one changes `session.rs` or the rules it cites. Stages 6,
7 and 8 touch disjoint files and may run side by side once stage 3 is done.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the rules say what a session with two lifetimes is

**Does:** writes every rule this goal changes, as § *Standing decisions* says, and renames the
configuration key. Nothing a session does changes yet.

- [x] Gate: `docs/rules/http-server/a-session-is-loaded-once-and-written-back-per-key.md` exists and
  `session-per-key` is gone from the chain.
- [x] `docs/rules/core-classes/a-session-opens-on-its-first-use.md:1`: renamed
  `a-session-opens-on-its-first-use.md` with its `data/rules/core-classes/` JSON, and rewritten.
  Every citation of the old id follows (`git grep session-is-started-explicitly`; about twenty lines,
  listed in the explore report of 2026-11-19: `docs/rules/http-server.md`, `security.md`,
  `core-classes.md`, `config.md`, `concurrency.md`, `docs/ground-rules.md:772`).
- [x] `docs/rules/core-api/session-roster.md:1` rewritten with the surface table.
- [x] `docs/rules/http-server/session-expiry-belongs-to-the-store.md:1` rewritten for the two clocks.
- [x] `docs/rules/http-server/a-session-value-is-short-lived-or-long-lived.md:1`: the new fragment
  and its JSON.
- [x] `docs/rules/http-server/a-session-holds-a-secret-only-sealed.md:1`,
  `docs/rules/core-classes/html-later.md:1` and
  `docs/rules/concurrency/deferred-work-cannot-write-the-response.md:3` updated.
- [x] `crates/nvs-config/src/tree.rs:1557` `ttl` becomes `short_lived`, and `long_lived` is added.
  `crates/nvs-config/src/defaults.rs:213` and `:570` and `crates/nvs-config/src/default.toml:683`
  follow. Then `bun nv rules --render`, and `bun nv rules --check` is green.

### 2. both stores keep each value with its lifetime, and the two clocks and the short cookie work

**Does:** the keystone. The record in the request knows each key's lifetime, both stores keep it, and
the clocks, the half rule and the short cookie work. The old members still exist and write
short-lived values, so the tree keeps building.

- [x] `crates/nvs-runtime/src/ctx/wiring.rs:47`: `Session` keeps each entry's lifetime, the short token
  and both clocks, with doc comments that state what each spends.
- [x] `crates/nvs-stdlib/src/session.rs:581` `TTL` and `:754` `ttl` become the two clocks.
- [x] `crates/nvs-stdlib/src/session/sqlite.rs:55` `SCHEMA` and its statements follow the `db`
  layout. The shared store's scripts in `session.rs` follow the `s:`/`l:` layout.
- [x] `crates/nvs-stdlib/src/session.rs:1017` `issue_cookie` sends the identifier cookie with or
  without `Max-Age`, and a new `issue_short_cookie` sends the token. The load at `:973` checks the
  token and applies the half rule.
- [x] `cargo test --lib -- session::` runs once, and every test it breaks is updated to the new shape.

### 3. the new members replace the old ones, and the session opens on its first use

**Does:** the registry rows, the first-use open, `openWithIdentifier`, and the cards. Every card is
end-user prose (`AGENTS.md` § *Text an end user reads*).

- [x] `crates/nvs-stdlib/src/session.rs:939` `nvs_core_session_start` becomes the private first-use
  open, reached by every member, and `openWithIdentifier` takes its argument path.
- [x] `:1396` `set`, `:1437` `setSecret`, `:1522` `clear` and `:1655` `destroy` become
  `setShortLived` and `setLongLived`, `setShortLivedSecret` and `setLongLivedSecret`,
  `clearShortLived` and `clearLongLived`,
  and `clearAll`. `:1562` `regenerate` draws a new short token.
- [x] Every write member calls `crate::response::head_open` first.
- [x] The cards: `START_DOC` (`:301`) becomes `OPEN_WITH_IDENTIFIER_DOC`, and `SET_DOC` (`:362`),
  `CLEAR_DOC` (`:431`), `DESTROY_DOC` (`:468`) and `SET_SECRET_DOC` (`:490`) become the new members'
  cards. No card names `start()` again.
- [x] `crates/nvs-stdlib/src/csrf.rs:203` and `:386-392` open the session on first use, and the
  "has not called `start`" text goes. `crates/nvs-stdlib/src/passkey.rs:353`, `:438`, `:747` and the
  test at `:2168` follow. `put_bytes` (`session.rs:1308`) writes a short-lived value.
  - [x] `token`, `put_bytes`, `take_bytes` and passkey's challenge open on first use
  - [x] the card text (`csrf.rs` `TOKEN_DOC` ~`:195`/`:203`, `passkey.rs` ~`:353`/`:438`), the
    module docs (`csrf.rs:47`, `passkey.rs:26`) and the tests (`csrf.rs` ~`:555-581`,
    `passkey.rs` `the_members_throw_before_a_session_is_started` ~`:2162`) still name `start()`
- [x] `crates/nvs-syntax/src/parser/mod.rs:1050` names a member that exists.
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green.

### 4. the checker refuses every session write inside Html::later and Task::afterResponse

- [x] `crates/nvs-types/src/response.rs:197` `HEAD_MEMBERS` gains the eight write members, and its doc
  comment says why a write to an existing session is refused too.
- [x] `tests/conformance/reject/html-later-anonymous-function-cannot-change-the-response-head.nvst:46`
  and `tests/conformance/core/after-response-a-response-write-does-not-compile.nvst:48` gain one
  call and one expected error per new member.
- [x] `tests/conformance/core/session-a-read-inside-later-or-after-response-compiles.nvst:1`: a new
  case. `get` inside both compiles; inside `later` it prints the value, and after the response it
  throws `LogicError`, because deferred work has no request (Backlog).
- [x] The `checker` check runs once. Its three cases pass under `nvs test`; the suite command
  stays red until stage 6, because it also reruns the cases that still call `start` and `set`.

### 5. tests prove the two lifetimes, the clocks, the clears and the first-use open on both stores

**Does:** the tests the `lifetimes` check names, each on the fake shared store and the `db` file
through `a_store`, as `session-per-key`'s cross-store cases do.

**Split:** one writing subagent writes the thirteen tests into `crates/nvs-stdlib/src/session.rs`'s
`mod tests`, in the shape of `a_destroyed_session_is_not_written_back_on_any_store`. A clock test
moves time with the clock the crate's tests already use, never by waiting.

- [x] The thirteen tests exist.
- [x] The `lifetimes` check runs once.

### 6. every program, test and package in the tree uses the new members

**Does:** each file below loses `Core\Session::start()` and uses the new members. `set` becomes
`setShortLived` unless the value is a preference, which becomes `setLongLived`. `destroy` at sign-out
becomes `clearShortLived`. A test whose subject was `start` itself is rewritten for the first-use
open or `openWithIdentifier`, or deleted where its subject is gone (the second `start`, a member
before `start`).

**Split:** three writing subagents, one per group, side by side. The session runs each group's suite
once after all three end.

- [x] Conformance: `tests/conformance/core/` `a-nullable-core-parameter-keeps-the-mark-of-its-type`,
  `a-session-survives-a-request-on-another-process`,
  `csrf-token-binds-to-the-session-the-response-sends`,
  `csrf-token-for-a-returning-visitor-sends-no-new-cookie`,
  `csrf-token-needs-a-key`,
  `passkey-a-challenge-verifies-once-and-only-for-its-origin`,
  `passkey-a-stored-credential-signs-in-while-its-counter-rises`,
  `passkey-credential-readers-after-a-sign-in`, `passkey-every-ceremony-member-needs-a-request`,
  `session-a-secret-round-trips-sealed-and-survives-regenerate`,
  `session-get-answers-null-for-a-sealed-value`,
  `session-a-first-use-reaches-the-configured-shared-store-and-never-one-of-its-own`,
  `session-open-with-identifier-and-a-first-use-reach-the-same-store-question`,
  `session-a-first-use-where-no-request-arrived-is-a-logic-error`,
  `session-a-first-use-with-no-block-opens-the-data-folder-store`,
  `session-every-member-names-itself-where-no-request-arrived`,
  `session-every-member-gives-one-answer-where-no-request-arrived`,
  `session-the-refusal-where-no-request-arrived-is-catchable-at-the-root`,
  `tests/conformance/reject/session-a-write-refuses-a-secret-and-names-its-sealed-sibling.nvst:1`, and
  `tests/conformance/isolate/a-child-refuses-request-state-and-still-reads-the-process.nvst:1`.
  A case renamed for its new subject keeps its expected output's meaning.
- [x] Programs: `apps/chat/src/App.nvs:1`, `apps/registry/src/App.nvs:1`,
  `apps/registry/src/Gate.nvs:1`, `packages/rest/src/Session.nvs:6`, the hostile files
  `tests/hostile/core/Csrf/token/01-a-token-replayed-without-its-session.nvs:1`,
  `tests/hostile/core/Passkey-Credential/signCount/01-counters-at-the-limits.nvs:1`, the four under
  `tests/hostile/core/Passkey/`, `tests/hostile/package/Novis-Rest-Session/01-plant-forged-tokens-in-the-session.nvs:1`,
  and the Rust strings at `crates/nvs-cli/tests/live_config.rs:1336` and
  `crates/nvs-cli/tests/per_mount_config.rs:485`.
- [x] Benches and examples outside `Session`: `benches/compare/scenarios/session/main.nvs:1`,
  `benches/members/core/Csrf/token.nvs:1`, the six under `benches/members/core/Passkey/`,
  `benches/members/package/Novis-Rest-Session.nvs:1`, `benches/scaling/app/clients.nvs:1`,
  `benches/scaling/app/requests.nvs:1`, `docs/examples/core/Csrf/token/` (three examples and
  `about.md`; `03-token-before-start-throws.nvs` is replaced, since its subject is gone),
  `docs/examples/core/Passkey-Credential/signCount/`, the four `docs/examples/core/Passkey/` members,
  `docs/examples/core/Test/advance/03-a-session-that-ends-after-idle-time.nvs:1` and
  `docs/examples/package/Novis-Rest-Session/`. Every comment is end-user prose.
- [x] `docs/agent/playbook.md` and its two bullets that name `Session::start` say what is now true.

### 7. every Session member owes no feature proof

**Does:** the feature proofs of the twelve members (`rule:testing/feature-proofs`). `bun nv proofs
--id 'Core\Session::<member>'` prints what each owes. The old members' folders are moved where the
member was renamed (`set` to `setShortLived`, `setSecret` to `setShortLivedSecret`, `destroy` to
`clearAll`, `clear` to `clearShortLived`, `start` to `openWithIdentifier`) and rewritten, and
`setLongLived`, `setLongLivedSecret` and `clearLongLived` are new. This covers `docs/examples/core/Session/`,
`benches/members/core/Session/` and `tests/hostile/core/Session/`. Every `about.md`, comment and
example is end-user prose.

**Split:** writing subagents, one per member, side by side: their folders are disjoint. The session
then runs `bun nv proofs --record-perf` once for the `Session` benches.

- [ ] The eight proof checks are green, and `get`, `remove`, `regenerate` and `getSecret` owe nothing.

### 8. the website pages use the new members

- [ ] `website/src/content/docs/guides/cookbook/upload-file.mdx:33`,
  `website/src/content/docs/guides/example-apps/chat.mdx:336` and
  `website/src/content/docs/guides/example-apps/notes.mdx:163` use the new members.
- [ ] The generated `website/src/data/core.json`, `website/src/data/questions.json` and
  `website/public/llms-full.txt` are rebuilt by the command that owns each, never edited by hand.

### 9. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and this goal's own tests at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 6 are done. `cargo build`, `cargo clippy --all-targets -- -D warnings` and
`cargo fmt --check` are green.

Stage 6: outside `Session`'s own proof folders (stage 7) and `website/` (stage 8), nothing in the
tree calls `start`, `set`, `clear`, `destroy` or `setSecret` (`debugger.rs` and `Test/advance/03`
name their own classes). Checked: the changed conformance cases and the `checker` cases, 23 passed
under `nvs test`; `bun nv proofs --run` over Passkey, Csrf and `Novis\Rest\Session`, 21 examples
and 7 attacks passed; `live_config`'s
`a_changed_session_backend_applies_to_new_requests` (its fake store now looks for `EVAL`, the
command the store's issue script sends; it looked for `SET`, red since `session-per-key`) and
`per_mount_config`'s `two_mounts_do_not_open_each_others_sessions` pass.

A security gap found and closed: `quals.rs` `reject_secret_session_argument` refused a `secret`
only for the removed `set`, so `setShortLived` and `setLongLived` took one silently. Both now get
E0775, whose help names the sealed sibling of the same lifetime;
`tests/conformance/reject/session-a-write-refuses-a-secret-and-names-its-sealed-sibling.nvst` and
`qualifiers_hold.rs` `every_secret_sink_refuses_a_secret_mixed` prove it.

Every doc comment and rule fragment that named `Core\Session::start` says what is true now, and
`bun nv rules --render` rewrote the chapters and `docs/ground-rules.md`. The conformance case
`a-nullable-core-parameter-keeps-the-mark-of-its-type` lost its `start(?id)` half, since
`openWithIdentifier` takes no nullable; its `Core\Path` half remains. The sweep's reds are stale
(retired `env-secret-proofs`; nightly 44510687786 built `adc7487822`, fixed on `main`).

### Next

Stages 7 and 8, side by side (each stage's `**Split:**` line names its subagents). Stage 7's folders
are `docs/examples/core/Session/`, `benches/members/core/Session/` and `tests/hostile/core/Session/`,
which still use the removed members. Stage 8's pages: `website/src/content/docs/guides/cookbook/upload-file.mdx`,
`website/src/content/docs/guides/example-apps/notes.mdx` and `chat.mdx`, then the generated
`website/src/data/core.json` and `website/public/llms-full.txt`. Expect red until stage 7 in
`bun nv proofs` for every `Session` member and in `perf: stale` for the Passkey and Csrf members
(`passkey.rs` changed); stage 9 records perf, per the playbook's `perf: stale` bullet.

### Backlog

- `Core\Csrf::token` inside a `Core\Html::later` slot throws for a new visitor, because it draws a
  session identifier and the head is closed. It works for a visitor with a session. That is the same
  trap as a session write there, but refusing it in the checker would forbid forms in a slot. It needs
  its own design, and becomes a gap record if this goal ends with it open.
- A `Core\Session::get` inside `Core\Task::afterResponse` throws the no-request `LogicError`: the
  deferred work runs on a context with no inbound request and no session (`nvs-host`), so it never
  sees the session the request saved. This was true before this goal too, where it threw the
  "start first" error. The roster rule says `get` stays allowed there, which compiles but does not
  run. Either the deferred context gets a read-only view of the request's session, or the rule says
  a read there throws. It needs its own design, and becomes a gap record if this goal ends with it
  open.
