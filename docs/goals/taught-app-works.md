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

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1 to 4 have landed. `Core\Csrf::token` has every proof (stage 3). Stage 4 landed in four
commits. The notes page lists the three steps a form needs (`notes.mdx:155`). The routes page says
`csrf: false` is written in `#[Core\Access]`, warns, and links `/reference/types/audience/`
(`routes.mdx:19`). The upload page shows the form tag with `_csrf` in its `action`
(`upload-file.mdx:27`). The compression example now sends itself two `Core\Test::request`s, with and
without `gzip`, and prints `Content-Encoding` and `Vary`. Its `.nvsr` was tried and dropped, because
`nvs run --request` prints the gzip bytes as the `.out`. An option of `#[Core\Access]` on `#[Core\Route]`
is still `E0454`, and its help now names `#[Core\Access]` (`crates/nvs-types/src/routes.rs:option_home`,
test `csrf_on_the_route_row_is_refused_with_a_help_naming_access` in `crates/nvs-types/tests/routes.rs`,
run once, green). `bun nv rules --check` is green.

A page may not carry an inline Novis fence (`tools/nv/cmd/site.ts:157`), and a snippet's
`.http.out` has no wildcard, so a served form with a fresh token can never match. Both pages show
the form as inline code for that reason.

The nightly CI fixes (a024f2253, 296755665, the `env_context.rs` `E0607`) are still local and not
pushed, so the sweep's CI list is stale until a push. The `env-secret-proofs` log says exit 0.
### Next

**Stage 5**, the gate. Start `bun nv affected --run` with `run_in_background`, end the turn, and
fix every failure in one pass. Then the goal-end gates: `bun nv verify --doc` and
`bun nv owners --closes taught-app-works`. Settle the two backlog items below.

`bun nv proofs` without `--nvs` reads `target/release/nvs`, so a member newer than that binary reads
as `no feature`. Pass `--nvs target/debug/nvs.exe`, or let `--record-perf` rebuild the release binary.

Tests that may go red at the gate: `crates/nvs-config/tests/defaults.rs` (the new `journal_mode`
row against its `# default` line), any test that asserts SQLite answers `SQLITE_BUSY` at once, any
case that reads a SQLite file's journal mode, and any roster count of `Core` members or of
`Core\Csrf`'s methods, and any check that compares the `issue` or `verify` benches with a run under
the root `nvs.toml`. Also `bun nv site --check` over the four stage 4 pages, and any test or
conformance case that pins the old `E0454` help text for `csrf` on `#[Route]`.

### Backlog

- `nvs run --request` lets a POST with a wrong token reach its handler, and `Core\Test::request`
  refuses it. Stage 5 checks whether the door is meant to run there, and fixes or records it.
- `journal_mode` has its line in `default.toml` and its row in the config reference. Stage 5 checks
  that `nvs agent show` reaches it, so its Help proof is not owed.
