---
looper: 1
title: A queue job's id survives a request, and a JSON stream reaches a response
summary: "Reached when Core\\Queue\\Id has a string form that a later request parses back, a guessed or changed string finds no job, Core\\Json::encodeEach's pieces carry what Core\\Json::encode's text carries so Core\\Response::stream writes them, and each changed member carries its feature proofs, with an attack on a forged and another user's id."
context: [crates/nvs-stdlib/src/queue.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/registry.rs, docs/rules/concurrency/queue-four-members.md, docs/rules/core-classes/json-whole-and-stream.md]
stages:
  - id: 1
    title: "Core\\Json::encodeEach's pieces carry what Core\\Json::encode's text carries, and a response streams them"
    status: done
    context: [crates/nvs-types/tests/tainted.rs, crates/nvs-stdlib/src/response.rs, docs/rules/security/response-body-is-one-typed-member.md, docs/examples/core/Json/encodeEach/about.md, docs/reference/core/Json.md, docs/reference/core/Json-Pieces.md]
  - id: 2
    title: "a job row carries a random token, and every statement that takes an Id matches it"
    status: done
    context: [docs/rules/core-classes/queue-storage-is-a-table.md, docs/rules/core-classes/schema-converges.md, crates/nvs-cli/src/worker.rs, crates/nvs-cli/src/queue.rs, crates/nvs-stdlib/tests/queue_sqlite.rs, crates/nvs-cli/tests/queue_migrate.rs]
  - id: 3
    title: "every queue table the tree creates by hand has the token column"
    status: done
  - id: 4
    title: "Core\\Queue\\Id has toString, parse and tryParse"
    status: done
    context: [crates/nvs-stdlib/src/uuid.rs, docs/rules/expressions/try-parse.md, docs/rules/classes/stringable.md, docs/rules/security/tainted-qualifier.md]
  - id: 5
    title: "the three Id members carry their feature proofs, and an attack sends a forged and another user's id"
    status: done
    context: [docs/examples/README.md, tests/hostile/README.md, benches/members/README.md, docs/rules/testing/feature-proofs.md, docs/reference/core/Queue-Id.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: json-pieces-taint
    stage: 1
    name: encodeEach's pieces carry what encode's text carries, and a stream's write takes them
    run: [cargo, test, --test, tainted]
    timeout: 1800
    tests:
      - an_encoded_value_carries_the_outside_text_it_contains
      - encode_each_pieces_carry_what_encode_text_carries
      - a_response_stream_writes_encode_each_pieces
  - id: queue-token
    stage: 2
    name: a row's token is written at push, returned on a dedupe hit, kept in the dead-letter table, and a wrong one finds no job
    run: [cargo, test, --test, queue_sqlite]
    timeout: 1800
    tests:
      - a_pushed_row_carries_a_sixteen_byte_token
      - a_dedupe_hit_returns_the_pending_jobs_token
      - status_with_another_token_finds_no_job
      - cancel_and_delete_with_another_token_change_nothing
      - a_dead_lettered_job_keeps_its_token
  - id: queue-migrate-token
    stage: 2
    name: nvs queue migrate adds the token column to a table that has none
    run: [cargo, test, --test, queue_migrate]
    timeout: 1800
    tests:
      - migrate_adds_the_token_column_to_an_existing_jobs_table
  - id: queue-id-string
    stage: 4
    name: an Id's string parses back to the same Id, and a bad string throws before anything is read
    run: [cargo, test, --lib, "queue::"]
    timeout: 1800
    tests:
      - id_to_string_parses_back_to_the_same_id
      - id_string_uses_only_url_safe_characters
      - id_parse_throws_on_a_string_longer_than_the_longest_id
      - id_parse_message_quotes_none_of_the_string
      - id_try_parse_returns_null_where_parse_throws
---

# A queue job's id survives a request, and a JSON stream reaches a response

## Why

The user decided two `Core` API changes on 2026-11-06.

**`Core\Queue\Id` cannot leave the request that made it.** It has no methods
(`crates/nvs-stdlib/src/queue.rs:2686`), it cannot be printed or converted to a string, and nothing
rebuilds one. It is two slots, the row number and the queue name (`queue.rs:1926`). So a web app
cannot push a job in one request and check its status from a later one, which is the common case: a
browser starts an export and polls for it. The user chose a string form and a way to read it back. The
string read back is client input, so it is `tainted`, and the app must check that the job belongs to
the user. The about page says so.

**`Core\Json::encodeEach`'s pieces do not reach a streamed response.** The pieces are `tainted bytes`
(`crates/nvs-stdlib/src/registry.rs:3555`), and `Core\Response\Stream::write` refuses a tainted chunk
(`crates/nvs-stdlib/src/response.rs:645`), so `$stream->write($piece)` does not compile (`E0401`,
expected `string|bytes`, found `tainted bytes`). `Core\Json::encode` returns a plain `string`
(`crates/nvs-stdlib/src/json.rs:418`), so the same data written whole does compile.
`rule:core-classes/json-whole-and-stream` already says a response writes `encodeEach`'s pieces to
`Core\Response::stream`: the rule is ahead of the code. The user chose that the pieces carry what
`encode`'s text carries, so whatever works for `encode` works for a streamed response.

## Standing decisions

1. **The Id's three members**, by `rule:expressions/try-parse` and `rule:classes/stringable`:
   - `toString(): string`, an instance member, so interpolation, `echo` and `as string` render an Id.
   - `Core\Queue\Id::parse(tainted string $s): Core\Queue\Id`, which throws `RuntimeError` on a string
     that is not an Id's, as `Core\Uuid::parse` does.
   - `Core\Queue\Id::tryParse(tainted string $s): ?Core\Queue\Id`, which is `parse` with the throw
     caught, never a second implementation.
   - Considered and declined: `fromString` (the try-parse rule fixes `parse`/`tryParse` as the pair for
     "this text is a `T`"), and `Core\Queue::idOf(string)` (a parse belongs to the class it gives).
2. **What makes a string an Id: a random token in the row.** At `push`, the row gets 16 random bytes
   from the operating system's secure generator, in a new `token` column of both tables. The Id gets a
   third slot, `token`. `status`, `cancel` and `delete` match the row on its id, its queue and its
   token, in the same statement, so a changed or guessed string finds no row. The options considered:
   - **The row number and the queue as plain text.** Declined: row numbers count up, so one missed
     ownership check lets any client read or cancel every job by counting.
   - **A signature with a configured key** (an HMAC over the row number and the queue). Declined: it
     adds a secret to the configuration, a key leak forges every id, rotating the key breaks every id
     already handed out, and a signature proves origin, not who may use it
     (`rule:security/verification-does-not-launder`).
   - **A hash of the token in the row.** Declined: `push` with a `key` returns the pending job's Id
     (`docs/reference/core/Queue-Id.md:10`), and that needs the token back. Whoever can read the jobs
     table can already read every job's arguments, so the token in it exposes nothing new.
   - Chosen: the random token. It needs no key and no configuration, a deleted row takes its token with
     it, and a guess has 2^128 possibilities.
3. **The token column.** `token`, 16 bytes, in `schema()` (`queue.rs:329`) for both tables, with the
   binary type `nvs_db::ddl` emits in each dialect. It is nullable, so `nvs queue migrate` converges an
   existing table onto it (`rule:core-classes/schema-converges`); a null never equals a token, so a row
   written before the column never matches a parsed Id. A push with a `key` that finds a pending job
   returns that job's token with its id. A job moved to the dead-letter table keeps its token, so its Id
   still reads `Dead`.
4. **The string's format is opaque, and only `parse` reads it.** It is
   `<row number>.<token>.<queue>`: the row number in decimal, the token as 22 base64url characters,
   and the queue name as base64url with no padding. Every character is in `[A-Za-z0-9._-]`, so the
   string goes into a URL path, a query, a header, a cookie or JSON with no escaping. `parse` throws
   on a string longer than the longest valid one before it decodes anything; that length is a constant
   computed from the queue column's 255-byte width, and a test holds it to the schema. The message
   quotes none of `$s`, because a real Id is a bearer value that a log should not keep.
5. **Taint.** `parse` and `tryParse` take `CoreTy::Text(Qual::Neutral)`, as `Core\Uuid::parse` does
   (`crates/nvs-stdlib/src/uuid.rs:169`): the text arrived from a client, so the parameter admits a
   `tainted` one, and the Id that comes back has no text a program can read. `toString` returns a
   plain `string`, built from checked parts in the safe character set. Considered and declined:
   - **A `tainted` Id.** `rule:security/tainted-qualifier` puts the qualifier on `string`, `bytes`
     and shapes of them, and an instance cannot carry it.
   - **A `secret` string.** The string exists to be sent to the client, and every response member
     refuses `secret`.
6. **The ownership check is the app's**, as the user decided. The about pages of `parse`, `tryParse`
   and `toString`, and the class card, show one pattern: the app stores the Id's string beside its own
   record of who started the job, and a later request looks the job up by the string *and* the
   signed-in user before it calls `status` or `cancel`. Considered and declined: an `owner` option on
   `push` that `status` checks. The user chose the app's check, and the token already makes a guessed
   id useless, so what is left is an id the app handed out, whose owner only the app knows.
7. **No answer tells a forged Id from a deleted job.** A changed token, row number or queue gets what
   a deleted job gets: `status` throws its `RuntimeError` for a missing job, and `cancel` and `delete`
   return `false`. One statement serves both, so the time taken is the same too.
8. **The pieces carry what `encode`'s text carries.** `encode` declares a `mixed` parameter and returns
   `CoreTy::Str`, a plain `string`, whatever the value held: the serializer owns the framing, so a
   tainted string becomes a JSON string value and cannot break out of it
   (`rule:security/response-body-is-one-typed-member`). Stage 1's first item confirms this with a test
   before anything changes. The pieces' element in `ITERABLES` becomes the `bytes` that matches it,
   `CoreTy::Bytes`. If the test shows `encode`'s answer is qualified in some case, the pieces follow
   the same classification instead, so the two members never differ.
9. **Examples.** `encodeEach`'s first example answers its request through
   `Core\Response::stream('application/json')`, and every `encodeEach` example binds `bytes $piece`.
   The Id's examples push to a `[queue]` on an in-memory SQLite database, as
   `docs/examples/core/Queue/push/nvs.toml` does, and create the two tables by hand with the token
   column. Every `.nvs` and `about.md` follows AGENTS.md § *Text an end user reads*.
10. **Feature proofs.** `bun nv proofs --id '<member>'` prints what each of
    `Core\Queue\Id::toString`, `Core\Queue\Id::parse`, `Core\Queue\Id::tryParse` and
    `Core\Json::encodeEach` owes, and stage 5 and stage 1 write it. The attack is
    `tests/hostile/core/Queue-Id/parse/01-a-job-id-the-client-changed.nvs`, with one numbered step per
    case: a changed token, a changed row number, another queue's name, a string of a million
    characters, and another user's real id that the app's own check refuses.
11. **Lanes.** Stage 1 touches `json.rs`, `registry.rs`'s `ITERABLES`, the JSON docs and the taint
    tests, and none of the queue files, so it may run as a side goal beside stages 2 to 5. Stages 2, 3,
    4 and 5 run in order: the Id members need the token, and their examples need the column.
12. **What this spends.** 16 bytes per job row in both tables, and one more slot per `Core\Queue\Id`.
    `push` draws 16 random bytes per job. `parse` reads at most the longest Id and allocates one Id.
    The JSON change spends nothing.
13. **`encode`'s answer is qualified, so the pieces follow it per call.** Stage 1's check showed it:
    `encode` returns a `tainted string` whenever its value can hold outside text, `mixed` included
    (`an_encoded_value_carries_the_outside_text_it_contains`), by `encodes_outside_text`
    (`crates/nvs-types/src/expr/quals.rs`). Decision 8's second branch applies. `Core\Json\Pieces`
    is generic in its piece type and is a `CELL_CLASSES` row, as `Core\Csv\Rows` is. `encodeEach`
    returns `Pieces<bytes>`, and `Pieces<tainted bytes>` when `encodes_outside_text` says so; a walk
    argument is judged by the element its `foreach` gives. So a stream's `write` takes the pieces of
    a list the program built and refuses those of a list from a request, exactly as it would refuse
    `encode`'s string. Considered and declined:
    - **A plain `encode`, now that its text escapes `'`.** The escaping makes the text inert in HTML,
      but the client still chooses its content, and the text can reach a path, a command or a header.
      Security outranks the convenience.
    - **A `write` that accepts tainted pieces because the serializer owns their framing.** The piece
      reaches `write` as `bytes`, which has lost where it came from, so the exception would be a
      second classification for one type.
    - **A second class for the tainted pieces.** Two classes with one iteration protocol, where the
      generic class already exists in `Core\Csv\Rows`.
14. **The `token` column is text: the 16 bytes as 32 lower-case hex characters**, which replaces
    decision 3's binary type. Every value the queue binds goes out as text on all four send paths, and
    a binary parameter is spelled differently on each (`bytea` text form, `varbinary` from
    `nvarchar`, a blob value). Hex has one written form per byte string, so MySQL's and SQL Server's
    case-insensitive default collations still compare two tokens exactly; base64url text would not.
    It spends 16 more bytes per row than binary, and memory is last in the ordering. The Id's third
    slot holds the same hex text, `queue::token_text` writes it, and stage 4's `parse` decodes the
    string's 22 base64url characters to 16 bytes and writes them through `token_text` before any
    statement runs. A dedupe hit on a row written before the column returns an empty token, which
    matches no row.
15. **The longest Id is computed from 1,020 bytes of queue name, not 255**, which refines decision 4.
    `KEY_WIDTH` counts characters (`varchar(255)` under `utf8mb4`), so a queue name the column
    admits can take four bytes a character. `ID_TEXT_MAX` is 20 + 1 + 22 + 1 + 1,360 = 1,404, and
    `id_string_uses_only_url_safe_characters` holds it to the `queue` column of `schema()`. Each part
    has one written form: the row number has no sign and no leading zero, and the base64url decoder
    refuses trailing bits. An Id whose token slot is empty (a row written before the column) renders
    sixteen zero bytes, and its string reads back as an Id that finds no job, as the empty token did.

## Stages

### 1. Core\Json::encodeEach's pieces carry what Core\Json::encode's text carries, and a response streams them

**Does:** decision 8 and the JSON half of decision 9. The tree had uncommitted work in
`crates/nvs-stdlib/src/json.rs` when this goal was written; the stage starts from whatever is
committed then.

- [x] The test `an_encoded_value_carries_the_outside_text_it_contains`, run once by name, confirms
  how `encode` decides its result's taint before anything changes: it is qualified (decision 13).
  `crates/nvs-types/tests/tainted.rs:740`
- [x] `Core\Json\Pieces<T>`'s element follows `encode`'s classification, and the rows' comments say
  why (decision 13). `crates/nvs-stdlib/src/registry.rs:3574`
- [x] The other two tests of the `json-pieces-taint` check. `a_neutral_walk_parameter_admits_a_walk_of_tainted_bytes`
  keeps its tainted array arm and stops naming `encodeEach`. `crates/nvs-types/tests/tainted.rs:110`
- [x] A conformance case: a request streams `encodeEach`'s pieces through `Core\Response::stream`.
  `tests/conformance/core/json-encode-each-pieces-stream-a-response.nvst:1`
- [x] The cards say the pieces are `bytes`, and `tainted bytes` for items with outside text.
  `crates/nvs-stdlib/src/json.rs:587`, `docs/reference/core/Json-Pieces.md:10`
- [x] The about page and `Json.md` name a streamed response, and example 01 answers through
  `Core\Response::stream` and binds `bytes $piece`; examples 02 and 03 bind no piece.
  `docs/examples/core/Json/encodeEach/about.md:1`, `docs/reference/core/Json.md:19`,
  `docs/examples/core/Json/encodeEach/01-answer-with-a-filtered-list.nvs:32`
- [x] `rule:core-classes/json-whole-and-stream` says the pieces carry `encode`'s taint.
  `docs/rules/core-classes/json-whole-and-stream.md:18`
- [x] `bun nv proofs --id 'Core\Json::encodeEach'` owes only the perf record, which stage 6's
  `--record-perf` pays, and `bun nv proofs --comments` over the changed `.nvs` files passes.

### 2. a job row carries a random token, and every statement that takes an Id matches it

**Does:** decisions 2, 3 and 7, under the rules first. After it, every test and example that creates
the queue tables by hand fails on insert until stage 3; the handoff names them.

- [x] `rule:concurrency/queue-four-members` says an Id has a string form, a token makes it hard to
  guess, and the app checks the owner. `docs/rules/concurrency/queue-four-members.md:1`
- [x] `rule:core-classes/queue-storage-is-a-table` names the `token` column and why it is nullable.
  `docs/rules/core-classes/queue-storage-is-a-table.md:1`
- [x] `schema()` has the column in both tables, and `schema_shortfall` names it.
  `crates/nvs-stdlib/src/queue.rs:329`, `crates/nvs-stdlib/src/queue.rs:435`
- [x] The insert writes the token in all four dialects and returns the pending job's on a dedupe hit.
  `crates/nvs-stdlib/src/queue.rs:535`, `crates/nvs-stdlib/src/queue.rs:4212`
- [x] `status`, `cancel` and `delete` match the token in all four dialects.
  `crates/nvs-stdlib/src/queue.rs:1331`, `crates/nvs-stdlib/src/queue.rs:1400`,
  `crates/nvs-stdlib/src/queue.rs:1466`
- [x] The Id's third slot, read wherever an Id is read. `crates/nvs-stdlib/src/queue.rs:1926`,
  `crates/nvs-stdlib/src/queue.rs:3382`
- [x] The dead-letter move carries the token. `crates/nvs-cli/src/worker.rs:1557`
- [x] The tests of the `queue-token` and `queue-migrate-token` checks.
  `crates/nvs-stdlib/tests/queue_sqlite.rs:1`, `crates/nvs-cli/tests/queue_migrate.rs:1`

### 3. every queue table the tree creates by hand has the token column

**Split:** an `Explore` subagent lists every file that runs `create table nvs_jobs` or
`create table nvs_dead_jobs` (76 on 2026-11-06: 36 under `docs/examples/core/`, 13 under
`tests/conformance/core/`, 12 under `tests/hostile/core/`, 13 under `benches/members/core/`, one under
`benches/scaling/`, and `docs/reference/core/Queue-Id.md`). Writing subagents add the column, one per
tree, side by side. None builds.

- [x] The list. `docs/reference/core/Queue-Id.md:38`
- [x] Each file's two `create table` statements gain `token`, in the type SQLite takes for it.
- [x] `bun nv proofs --run --group 'Core\Queue'` and `--group 'Core\Queue\Stats'`, and the queue
  conformance cases by name, run once; every `.out` is unchanged.

### 4. Core\Queue\Id has toString, parse and tryParse

**Does:** decisions 1, 4 and 5.

- [x] The three rows, with `Text(Qual::Neutral)` on `parse` and `tryParse`, and `toString` marking the
  class as one that renders. `crates/nvs-stdlib/src/queue.rs:2747`
- [x] The format, its longest length as a constant, and `parse`'s checks in order: length, characters,
  the three parts. `crates/nvs-stdlib/src/queue.rs:3441`
- [x] The class card and the three member cards; the class card loses "It has no methods of its own".
  `crates/nvs-stdlib/src/queue.rs:2757`
- [x] The five tests of the `queue-id-string` check. `crates/nvs-stdlib/src/queue.rs:5606`
- [x] Conformance cases: a string round-trips through a later `status`; a `tainted` string compiles
  into `parse`; a changed token reads as a deleted job.
  `tests/conformance/core/queue-an-id-read-back-from-its-string-reads-the-same-job.nvst:1`

### 5. the three Id members carry their feature proofs, and an attack sends a forged and another user's id

**Split:** one writing subagent per member for the about page and three examples, side by side; one
for the bench and the attack. Their files are disjoint. None builds.

- [x] `Core\Queue\Id::toString`: about page and three examples, one of which sends the id to the
  browser as JSON. `docs/examples/core/Queue-Id/toString/about.md:1`
- [x] `Core\Queue\Id::parse`: about page and three examples, one of which shows decision 6's owner
  check over a `.nvsr` request. `docs/examples/core/Queue-Id/parse/about.md:1`
- [x] `Core\Queue\Id::tryParse`: about page and three examples. `docs/examples/core/Queue-Id/tryParse/about.md:1`
- [x] The attack of decision 10. `tests/hostile/core/Queue-Id/parse/01-a-job-id-the-client-changed.nvs:1`
- [x] A bench for `parse` with its growth, constant in the string's length past the bound, and its
  share with `bun nv scaling --budget --write`. `benches/members/core/Queue-Id/parse.scale.nvs:1`
- [x] `docs/reference/core/Queue-Id.md` shows the string form and the owner check, and no longer says
  an Id cannot be printed. `docs/reference/core/Queue-Id.md:1`
- [x] `bun nv proofs --id` owes nothing for each of the three, and `bun nv proofs --comments` over every
  new `.nvs` passes; each line it names is written again.

### 6. the goal is verified once, and every failure is fixed

- [x] `bun nv proofs --record-perf` over the members whose figures are stale.
- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 5 are done. `bun nv proofs --id` prints `complete.` for each of `Core\Queue\Id::toString`,
`parse` and `tryParse`: about pages and three examples each under `docs/examples/core/Queue-Id/`, one
attack each under `tests/hostile/core/Queue-Id/` (decision 10's five steps in
`parse/01-a-job-id-the-client-changed.nvs`), and benches under `benches/members/core/Queue-Id/` with
`parse.scale.nvs` reading flat. Their perf is recorded in `data/perf/members.ndjson` and their shares
in `data/perf/bench-budget.json`. `docs/reference/core/Queue-Id.md` describes the string form and
ends its example with the owner check; `NVS_BIN=<abs path> bun nv reference --only Queue-Id` holds.
`status`'s missing-job message no longer says "receipt" and has no dash (`queue.rs:5156`), and
`queue-delete-and-purge-answer-the-same-way-on-sqlite.nvst` and `parse/02`'s `.out` carry the new text.

The sweep's reds are stale, as the last four sessions found: no `env-secret-proofs` check is
defined, and nightly run 42359979484 ran on 26dda36627, before the Miri ignores in f13f40646a.

Stage 6 has begun. `bun nv proofs --record-perf` appended 18 fresh records for the queue and JSON
members (b84382ec2f), and the first `looper verify` over the whole goal is running.

### Next

**Stage 6**, the gate. Read the first `looper verify` report, fix every failure it names in one
pass, and run `looper verify` again; then the goal-end gates of AGENTS.md § *In the loop*.
`docs/goals/queue-id-and-json-stream.md:307`

`bun nv scaling --budget --write` exits 1 on many benches outside this goal that are over their share
(`novis/Spreadsheet-*` among them); it still wrote the three new shares. Check whether CI's budget job
reads that as red before the goal ends.

Expected red, for stage 6: unit tests in `crates/nvs-stdlib/src/queue.rs` and
`crates/nvs-cli/src/worker.rs` that compare statement texts, column lists or placeholder counts, since
only stage 2's new tests ran; the server suite `crates/nvs-stdlib/tests/queue.rs` under `bun nv
db-matrix --all`; the three `image-*` conformance cases; and any registry snapshot or roster test
that counts `Core` members or expects `Core\Queue\Id` to have none.

### Backlog

- The JSON API guide could show a streamed response beside its file export.
  `website/snippets/guides/simple-programs/json-api/large-lists.nvs:29`
- A list read from a request cannot be streamed as JSON through `Core\Response::stream`, because its
  pieces are tainted (decision 13). `Core\Response::json` sends it whole. A streamed JSON response
  member that accepts tainted items, as `json` does, is the user's call.
- `crates/nvs-stdlib/tests/queue.rs`'s `status` helper sends `STATUS_MYSQL` to every driver but
  PostgreSQL, SQL Server included; no SQL Server case calls it today.
- `==` between an Id and `Core\Queue\Id::parse($id->toString())` is `false`, since an Id compares by
  identity. Whether two Ids of one job should be equal is the user's call.
- `exit(0)` in one branch does not narrow a nullable variable in the code after it.
- The release binary ran Id benches only after `--record-perf` rebuilt it. `target/release/nvs.exe`
  predates the `status` message change.
