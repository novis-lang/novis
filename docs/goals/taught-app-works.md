---
looper: 1
title: An app written the way the docs teach survives two writers and a keyed CSRF door
summary: "A SQLite statement with no deadline of its own waits for the write lock within the request's remaining time, a new file database opens in WAL unless [db.<name>] journal_mode says otherwise, Core\\Csrf::token returns a token bound to the session the response sends, the pages and examples teach the CSRF opt-out on #[Core\\Access], a multipart form's token in its action and compression that reads Accept-Encoding, every new feature has its proofs, and the checks below pass."
context: [docs/rules/security/csrf-is-on-by-default.md, docs/rules/core-classes/session-is-started-explicitly.md, docs/rules/testing/feature-proofs.md]
stages:
  - id: 1
    title: "a SQLite writer waits for the lock, and a new file database opens in WAL"
    status: done
    context: [crates/nvs-db/src/sqlite.rs, crates/nvs-db/src/conn.rs, crates/nvs-config/src/tree.rs, crates/nvs-config/src/defaults.rs, crates/nvs-config/src/default.toml, crates/nvs-stdlib/src/session/sqlite.rs, docs/reference/tools/20-config.md, docs/rules/core-classes/db-statement-members.md]
  - id: 2
    title: "`Core\\Csrf::token` returns a token bound to the session the response sends"
    status: done
    context: [crates/nvs-stdlib/src/csrf.rs, crates/nvs-stdlib/src/session.rs, crates/nvs-config/src/http.rs, crates/nvs-server/src/route.rs, docs/rules/security/csrf-is-on-by-default.md, docs/rules/core-classes/session-is-started-explicitly.md]
  - id: 3
    title: "`Core\\Csrf::token` has its feature proofs"
    status: done
    context: [docs/examples/core/Csrf/issue/about.md, benches/members/core/Csrf/issue.nvs, tests/hostile/core/Csrf/issue/01-one-token-for-a-second-session.nvs, docs/rules/testing/feature-proofs.md, docs/rules/testing/hostile-case-contract.md]
  - id: 4
    title: "the pages and examples teach the forms that work"
    status: done
    context: [website/src/content/docs/guides/example-apps/notes.mdx, website/src/content/docs/syntax/attributes/routes.mdx, website/src/content/docs/guides/cookbook/upload-file.mdx, docs/examples/core/Compress/compress/03-a-smaller-answer-over-the-network.nvs, crates/nvs-types/src/attributes.rs, docs/agent/prose.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: db-tests
    stage: 1
    name: two SQLite writers at once both succeed
    run: [bun, nv, verify, --fast, -p, nvs-db]
  - id: stdlib-tests
    stage: 2
    name: a token from `Core\Csrf::token` passes the door on the next request
    run: [bun, nv, verify, --fast, -p, nvs-stdlib]
  - id: csrf-token-proofs
    stage: 3
    name: "`Core\\Csrf::token` owes no proof"
    run: [bun, nv, proofs, --id, "Core\\Csrf::token"]
  - id: rules-check
    stage: 4
    name: every rule loads and every citation resolves
    run: [bun, nv, rules, --check]
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, taught-app-works]
    reach: true
    floor: false
---

# An app written the way the docs teach survives two writers and a keyed CSRF door

## Why

The 2026-10-21 audit built an app the way the docs and `apps/notes` teach, then tested it over the
network (the `bb-apps` and `bb-kitchen` parts). Its code held against injection, traversal, SSRF and
fixation. What broke was the path between the app and two defaults.

- **APP-1.** Two `POST`s at the same moment to `apps/notes` gave one `201` and one `500`. Fifty gave
  one `201` and 49 `500`s. A SQLite statement with no deadline of its own answers `SQLITE_BUSY` at once
  (`crates/nvs-db/src/sqlite.rs:1651`), and `Core\Db` opens no WAL (`crates/nvs-db/src/sqlite.rs:723`).
  The session store already opens its own file in WAL with a lock wait
  (`crates/nvs-stdlib/src/session/sqlite.rs:44`). An app's own database does not.
- **BBK-1, BBK-3.** With `[http] csrf_key` set, no documented form binds a token to the session. An app
  can only reach `Core\Csrf::issue(Core\Request::cookie("nvsid") ?? "", …)`. That issues a token for the
  empty session on a visitor's first page, so the first submit is a `403`. The key must also be decoded
  from base64 first, which no example shows. The audit's login-CSRF claim did not reproduce: the door
  refuses a token for the empty session on every cookieless request. `security-hardening` stage 9
  pins that.
- **BBK-2.** `routes.mdx` says "`csrf: false` turns both checks off" but not where it is written. The
  audit put it on `#[Route]`, got `E0454`, and decided the opt-out did not exist. It exists on
  `#[Core\Access]` and works: a keyed door let a `csrf: false` route run cross-origin with no token.
- **BBK-4.** `rule:security/csrf-is-on-by-default` reads no multipart body for a token, and says why in
  its Declined list. The upload page never says where a multipart form's token goes instead.
- **BBK-5.** `Core\Compress`'s network example sends `Content-Encoding: gzip` without reading
  `Accept-Encoding`, and sets no `Vary`.

Each item names its finding. The evidence is `.agent-tmp/sec-audit/reports/bb-apps.md` and
`bb-kitchen.md` while `security-hardening` keeps them, and the paragraphs above once that goal has
deleted them.

## Standing decisions

**The user's calls, 2026-10-22.** A new file database defaults to WAL, and `[db.<name>]
journal_mode` turns it off. `Core\Csrf::token` counts as a write to the session, so the response sends
the session cookie. `csrf: false` stays the one opt-out, with a warning in the docs. This goal adds one
configuration key by the user's explicit choice. It is an exception to `security-hardening`'s "no new
configuration key".

**A lock wait is bounded by the time the request still has.** A statement that names no deadline of its
own files the request's remaining `wall_time` as its `sqlite3_busy_timeout`. The wait runs on
`nvs-host`'s blocking pool, never on a core, as every SQLite statement does
(`crates/nvs-db/src/sqlite.rs:1644`). An unbounded wait stays impossible
(`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`). A statement's own deadline still
wins. Outside a request, the run's own `wall_time` bounds it the same way. A run with no
`wall_time` waits `nvs_db::sqlite::LOCK_WAIT`, the five seconds a fresh connection already waits.
The request's time bounds the lock wait alone and does not interrupt a running statement.

**WAL is applied at open, to a file database the program may write.** `:memory:`, a read-only open and
`journal_mode = "delete"` skip it. The accepted values are `"wal"` and `"delete"`. Any other value is
refused at load naming the key. The pragma is the session store's line, reused rather than restated.

**`Core\Csrf::token` needs `Core\Session::start` first.** `rule:core-classes/session-is-started-explicitly`
declines a hidden start, so `token` throws naming itself before `start`, like every session member.
After `start`, it binds the token to the id the response will send, and marks the session written. That
is what makes the cookie go out under SESS-1. It reads the key with `nvs_config::http::csrf_key`,
the reader the door uses (`crates/nvs-config/src/http.rs:550`), so no app decodes base64. With no key
configured it throws naming `[http] csrf_key`. `Core\Csrf::issue` stays for a program that binds a
token to something else.

**Gate.** Stage 2 starts once SESS-1 has landed. Its proof is `security-hardening`'s session attack
`tests/hostile/core/Session/start/01-identifiers-nobody-issued.nvs` passing with its 20,000
cookieless starts. `security-hardening` is ahead on the chain, so the gate holds when this goal goes live.

**BBK-4 keeps the rule.** A multipart form carries its token in the form's `action` query, as the rule
says. Reading a multipart body for `_csrf` is declined there and is not proposed again.

**Lanes.** Stage 1 (`nvs-db`, `nvs-config`) and stage 2 (`nvs-stdlib`'s `csrf.rs` and `session.rs`)
touch disjoint files and may run as side goals. Stage 3 follows stage 2, because its examples print what
`token` returns. Stage 4 follows stage 2, because the notes page shows `token`. Stage 4's diagnostic
change is in `nvs-types` and shares no file with the others.

**Trade-offs.** WAL adds a `-wal` and a `-shm` file beside the database, does not work on a network
drive, and changes the file's journal mode for good. That is why the key exists. It buys readers
that do not wait for the writer. A lock wait holds one blocking-pool thread per waiting statement, at
most for the request's remaining time. That is per request in flight, attributed to it. `token` adds
one `Core` member and spends one session row per visitor who is shown a form. The store's TTL already
bounds that row. Language surface: one member and one key, each with its help.

**The three gaps `security-hardening` left here are built here.** `owners --closes` names them, and no
goal on the chain is about taint or `cpu_time`. The `cpu_time` one goes first, because it is a limit a
request can overrun (security and request isolation). The two taint ones only refuse safe code, so
they cost usability and no safety.

## Stages

### 1. a SQLite writer waits for the lock, and a new file database opens in WAL

**Does:** fifty simultaneous inserts through `Core\Db` all succeed, and a database file opened for
writing is in WAL unless its block says `journal_mode = "delete"`.

- [x] APP-1: a statement with no deadline of its own files the request's remaining `wall_time` as its busy timeout, through `set_busy_timeout` (`rule:core-classes/db-statement-members`). `crates/nvs-db/src/sqlite.rs:1658`, `crates/nvs-db/src/conn.rs:1221`
- [x] APP-1: `open` runs the session store's `PRAGMA journal_mode = WAL` for a writable file database, skipping `:memory:`, a read-only open and `journal_mode = "delete"`. `crates/nvs-db/src/sqlite.rs:723`, `crates/nvs-stdlib/src/session/sqlite.rs:44`
- [x] The `[db.<name>] journal_mode` key: `"wal"` by default, `"delete"` the other value, anything else refused at load naming the key, and its row in the config reference. `crates/nvs-config/src/tree.rs:1052`, `crates/nvs-config/src/defaults.rs:544`, `crates/nvs-config/src/default.toml:679`, `docs/reference/tools/20-config.md:62`
- [x] The rule says what a SQLite connection opens with and what WAL spends, and the statement rule says what a statement with no deadline waits for (`rule:core-classes/db-statement-members`). `docs/rules/core-classes/db-statement-members.md:1`
- [x] The attack, as a Rust test because the harness cannot hold two requests at once: fifty connections insert at once and all succeed, a lock held past the remaining time fails as `Deadlock`, and `journal_mode = "delete"` leaves no `-wal` file. It is named in the rule's `guardedBy`. `crates/nvs-db/tests/sqlite_contention.rs:1`

### 2. `Core\Csrf::token` returns a token bound to the session the response sends

**Does:** a page that calls `Session::start()` and `Csrf::token()` on a visitor's first request gets a
token the door accepts on the next one. A program that has not started its session gets an error that
names `start`.

- [x] BBK-1, BBK-3: `Core\Csrf::token(): string` binds the token to the id the response sends, marks the session written, and reads the key with `nvs_config::http::csrf_key` (`rule:security/csrf-is-on-by-default`). `crates/nvs-stdlib/src/csrf.rs:134`, `crates/nvs-stdlib/src/csrf.rs:305`, `crates/nvs-config/src/http.rs:550`
- [x] Session gives `token` the id it will send, including a fresh one, and counts the call as a write. `crates/nvs-stdlib/src/session.rs:172`
- [x] `token` before `start` throws naming `Core\Session::start`, and with no key it throws naming `[http] csrf_key`. `crates/nvs-stdlib/src/csrf.rs:305`
- [x] The session rule says a token counts as a write, and the CSRF rule names `Core\Csrf::token` as the way a page issues a token (`rule:core-classes/session-is-started-explicitly`, `rule:security/csrf-is-on-by-default`). `docs/rules/core-classes/session-is-started-explicitly.md:1`, `docs/rules/security/csrf-is-on-by-default.md:9`
- [x] Rust tests beside the member: a first request's token passes the door on the second request carrying the cookie the first sent, and a token sent with another session's cookie is refused. `crates/nvs-stdlib/src/csrf.rs:305`

### 3. `Core\Csrf::token` has its feature proofs

**Does:** `bun nv proofs --id 'Core\Csrf::token'` prints nothing owed.

**Split:** one writing subagent writes the three examples, the `about.md`, the bench and the attack,
after stage 2 has landed. The session checks each against `docs/agent/prose.md` and writes the help card.

- [x] The description and three examples: a form's token on a first visit, the token passing on the next request, and `token` before `start`. `docs/examples/core/Csrf/token/about.md:1`, `docs/examples/core/Csrf/token/01-a-token-on-a-first-visit.nvs:1`
- [x] A Novis test: the conformance case that a token from `token` verifies for the session the response sent, and for no other. `tests/conformance/core/csrf-token-binds-to-the-session-the-response-sends.nvst:1`
- [x] The bench and its growth. `token` takes no input, so the bench declares `complexity constant` and needs no `.scale.nvs`. `benches/members/core/Csrf/token.nvs:1`
- [x] The attack: a token from a page that sent no cookie, replayed with no cookie, with another visitor's cookie, and cross-origin, is refused each time (`rule:testing/hostile-case-contract`). `tests/hostile/core/Csrf/token/01-a-token-replayed-without-its-session.nvs:1`
- [x] The help card in the member table, in the voice of AGENTS.md § *Text an end user reads* (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/csrf.rs:134`

### 4. the pages and examples teach the forms that work

**Does:** a developer who follows the notes page, the routes page, the upload page and the
compression example ends with a form that works on the first visit, an opt-out written on
`#[Core\Access]`, an upload form that carries its token, and compression only when the client offered it.

- [x] BBK-1: the notes page's "add a login and a CSRF token" note shows `Core\Session::start()` and `Core\Csrf::token()` in a form. `website/src/content/docs/guides/example-apps/notes.mdx:156`
- [x] BBK-2: the routes page says `csrf: false` is written on `#[Core\Access]`. It says to use it only on a route that does not trust the session cookie, such as a signed webhook, and links the webhook example. `website/src/content/docs/syntax/attributes/routes.mdx:19`, `docs/examples/types/Audience/03-a-webhook-another-server-calls.nvs:1`
- [x] BBK-2: `csrf` written on `#[Core\Route]` keeps `E0454`, and its help says the option is written on `#[Core\Access]`; a diagnostic test pins it. `crates/nvs-types/src/attributes.rs:541`
- [x] BBK-4: the upload page's form puts the token in its `action` query, and one sentence says a multipart form carries it there (`rule:security/csrf-is-on-by-default`). `website/src/content/docs/guides/cookbook/upload-file.mdx:9`
- [x] BBK-5: the network example reads `Accept-Encoding`, compresses only when `gzip` is offered, and sets `Vary: Accept-Encoding`; its `.out` is regenerated. `docs/examples/core/Compress/compress/03-a-smaller-answer-over-the-network.nvs:1`

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [x] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [x] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`
- [ ] The three gaps `security-hardening` left with this goal are built and their records deleted, and `bun nv owners --closes taught-app-works` is green. `data/gaps/nvs-db/a-slow-sqlite-statement-runs-past-cpu-time.json:1`, `data/gaps/nvs-stdlib/arr-haskey-refuses-a-tainted-key.json:1`, `data/gaps/nvs-stdlib/decode-as-refuses-a-tainted-string.json:1`
  - [ ] `Core\Arr::hasKey` takes a `tainted` key, and the overlay case uses `hasKey` again.
  - [ ] The `decodeAs`/`shapeAs` doors take a `tainted string` where the field check runs, with a conformance case.
  - [ ] A SQLite statement stops near the request's remaining `cpu_time`, through the progress handler, with a test.
  - [ ] `bun nv affected --run` is green over the three.

## Handoff

### State

Stages 1 to 4 have landed, and stage 5's gate is green: `bun nv affected --run` passed 22 of 22
steps on the third run. The first run failed on two stale rule pages and the second on the CSRF
token attack's layout (`nvs-fmt`'s `identity` test). Both are fixed in fed0fb506. `bun nv verify
--doc` is green, and `bun nv proofs --id 'Core\Csrf::token' --nvs target/debug/nvs.exe` says complete.

Both backlog items are settled. `nvs run --request` now takes the CSRF door through
`runner::refuse_a_forged_write` (`crates/nvs-cli/src/runner.rs`), the same check `Core\Test::request`
takes, with the test `a_request_file_with_a_forged_token_is_refused_before_its_handler` in
`crates/nvs-cli/tests/request.rs` (75daa865f). `nvs agent show "[db.main] journal_mode"` prints the
key's entry, so its Help proof is not owed.

`bun nv owners --closes taught-app-works` is still red, on the three gaps `security-hardening` left
here (see § *Standing decisions*). The fourth record, the session id for a CSRF token, is deleted,
because `Core\Csrf::token` closes it.

The nightly CI fixes are still local and not pushed, so the sweep's CI list is stale until a push.
The `env-secret-proofs` log says exit 0.

### Next

**Stage 5's last item**, the three gaps, in this order:

1. `data/gaps/nvs-db/a-slow-sqlite-statement-runs-past-cpu-time.json`: a progress-handler deadline from
   the request's remaining CPU time, beside the interrupt the handler already checks
   (`crates/nvs-db/src/sqlite.rs`, near the busy timeout at `:1658`). A test where an endless
   statement stops near `cpu_time`.
2. `data/gaps/nvs-stdlib/arr-haskey-refuses-a-tainted-key.json`: `hasKey`'s key parameter accepts
   `tainted` (`crates/nvs-stdlib/src/arr.rs`). The conformance case
   `arr-overlay-deep-at-a-key-is-itself-one-level-down-and-keeps-the-bases-order` uses `hasKey` again.
3. `data/gaps/nvs-stdlib/decode-as-refuses-a-tainted-string.json`: the `decodeAs`/`shapeAs` doors
   admit a tainted argument where `written_class_of`'s `subject_outside` check runs
   (`crates/nvs-stdlib/src/json.rs`). A conformance case for both a tainted and a plain property.

Delete each record as it is built, then run `bun nv affected --run` once more, and
`bun nv owners --closes taught-app-works`. Then `DONE`.

Tests that may go red: any test or case that pins `hasKey` or `decodeAs` refusing a tainted
argument, a roster or signature snapshot of those members, and any SQLite test that expects a long
statement to run past `cpu_time`.

### Backlog

- None.
