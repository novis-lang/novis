---
milestone: M7
---
# Loop goal 18 — a body is read once, and JSON is one of the ways to read it

Two facts about `Core\Request` that are one goal because they are one file set. **First**, the rule that
decides who may read a body is wrong in a way that costs a real program: spec § 15 makes `body`,
`bodyStream` and `files` exclusive, `claim_body` refuses any second claimant *including the same member
again*, and so `Core\Json::decode(Core\Request::body())` can be written exactly once per request. A
validating middleware that reads the body and a handler that wants it too is a `LogicError`, not a slow
path. `post()` already escapes this by holding its bytes and re-parsing, but as a special case with no
principle behind it. **Second**, there is no JSON body reader at all — the shape a JSON API sends is the
one shape `Core\Request` has no member for.

Both are unprovable in-language today, and that is the goal's third half: **a `.nvst` case answers no
request**, so every request-facing member in this repository is proven by a Rust `#[test]` name in a goal
manifest, and the three-`.nvst`-case floor `conformance_coverage.rs` enforces is met by cases asserting
the *refusal*. `.nvst` gets `.phpt`'s request sections, and then the members can be proven the way every
other member is.

Its floor is goal `editor`'s whole list, which is the parity program, the four post-parity goals and M4B.

## Why here

It is here rather than beside goal `server` because it was decided after that goal was written, and the
rule it replaces is one goal `server` landed: a fixture written against the old wording in the meantime is
a fixture rewritten twice, which is what its stage 0 pays off.

## What the members answer, in the two registers the cards carry

**In simple words**: *`json()` — the body the peer sent, read as a JSON document. Every call answers the
same document; the request is read once and the answer is kept. `jsonAs<T>()` — the same body hydrated
into a declared type, so a field the peer never sent is a throw rather than a surprise later.*

**In detailed words**: `Core\Request::json({maxDepth?: uint}): tainted mixed` is
[`Core\Json::decode`](../../spec/01-core-library.md) over the body's octets, carrying the same
`{maxDepth?}` bag and the same default of 512. `Core\Request::jsonAs<T>({maxDepth?: uint}): T` is
`Core\Json::decodeAs<T>` over the same octets, and it is the member with the security return: `decodeAs`
over a `tainted` argument diagnoses a `T` whose text-carrying fields are unqualified, naming the field —
and a request body is *always* tainted, so the type checker does the taint work at the call site. Both
throw `ParseError` on a malformed body **and on an absent or empty one**, which is the same failure a
handler maps to `400`. Neither looks at `Content-Type`: what a peer wrote in a header is not what decides
what a body is, which is `post()`'s own rule already.

## Stage 0 — the catch-up

The exclusivity fixtures written against the rule stage 3 replaces:
`tests/conformance/core/a-request-post-refuses-on-the-terms-the-other-body-readers-do.nvst`,
`a-request-post-refuses-before-it-reads-the-name-it-was-given.nvst` and the two `nvs-stdlib` unit tests
`a_body_is_claimed_by_the_member_that_read_it_and_refused_to_the_other` and
`body_stream_is_exclusive_with_body_and_with_files`. Their *questions* survive; the answers move. Do this
first: every case written against the old wording in the meantime is a case rewritten twice.

## Stage 1 — the floor

Goal `editor`'s whole acceptance list, never traded.

## Stage 2 — the keystone: a `.nvst` case can answer a request

Nothing else in this goal is provable in-language until this lands, and the three-case floor cannot be met
honestly without it.

1. **The sections**, in `crates/nvs-test/src/lib.rs:20`'s table and `case.rs`'s parse — `--GET--`,
   `--POST--`, `--POST_RAW--`, `--COOKIE--` and `--HEADERS--`, spelled as `.phpt` spells them so the M11
   corpus import stays mechanical. `--GET--` is a query string, `--POST--` urlencoded pairs, `--POST_RAW--`
   the body verbatim, `--COOKIE--` and `--HEADERS--` one field per line. `--POST--` and `--POST_RAW--`
   together is a parse error, not a merge.
2. **The carrier.** A case is run by spawning `nvs run` (`crates/nvs-cli/src/main.rs:1163`), so the
   sections have to cross a process boundary: `nvs run --request <file>` reads a frozen description and
   builds the `Inbound` before the program runs (`crates/nvs-cli/src/main.rs:923`, where `nvs run`'s `Ctx`
   is made; `Ctx::set_inbound` is `crates/nvs-runtime/src/ctx/inbound.rs:23`). The build itself is the
   dev server's, three calls deep: `Inbound::new` (`:339`), `push_header` (`:490`), `set_body` (`:511`) —
   `crates/nvs-cli/src/serve.rs:323` is the worked example.
3. **The section that says so.** `crates/nvs-test`'s module doc owns the format; its *What is parsed but
   not yet honoured* list is where these five are recorded as honoured, beside `--ENV--` and `--ARGS--`.

## Stage 3 — the rule: buffering readers share, streaming readers consume

1. **The body-read rule, a new `http-server/` fragment**, this goal's one new number. *A body is read once. A **streaming** reader — `bodyStream`,
   `files` — consumes it and refuses every later reader. A **buffering** reader — `body`, `post`, `json`,
   `jsonAs` — keeps what it read, so any buffering reader may follow another.* `post()` joining `files`
   stops being an exception and becomes a consequence: `files` buffers the non-file parts on its way past,
   so it leaves something behind.
2. **`hold_body`**, generalizing `Inbound::hold_form` (`crates/nvs-runtime/src/ctx/inbound.rs:602`),
   and `claim_body` (`:551`) rewritten to answer the *class* of the holder rather than its name.
   `nvs_stdlib::request`'s `claim_body`/`claim_form` pair (`crates/nvs-stdlib/src/request.rs:1032`,
   `:1081`) collapses into one call against it.
3. **`body()` becomes idempotent** (`crates/nvs-stdlib/src/request.rs:1696`) — the same octets on every
   call, which is what makes middleware-then-handler work whether or not a JSON member is the one reading.

## Stage 4 — the members

Two `Core` members, the five edits each, in `crates/nvs-stdlib/src/request.rs` beside `body` (the rows at
`:255`, the cards after them, the bodies, the `address()` arm at `:982`), plus:

1. **The decode** reuses `crate::json::read` (`crates/nvs-stdlib/src/json.rs:864`) and
   `DECODE_OPTIONS`/`DEFAULT_MAX_DEPTH` (`:339`, `:367`) — no second JSON reader and no second default.
2. **`json()` caches its decoded value on the request**; `jsonAs<T>()` never does. `Inbound::hold_parts`'s
   `Box<dyn Any>` (`crates/nvs-runtime/src/ctx/inbound.rs:571`) is the precedent for the slot, and the value is
   dropped with the request.
3. **The spec** — § 15's `Core\Request` bullet gains both members and loses the three-way exclusivity
   sentence to the new body-read rule. `spec_registry_coverage.rs` reads that bullet as the roster, so this edit
   is what makes the registry rows legal rather than a separate chore.
4. **Three `.nvst` cases each**, each asking a different question, over stage 2's sections — the first
   request-facing members in this repository proven the way every other member is.

## Stage 5 — the proofs

`examples/json-body.nvs` as the runnable fixture, the reference page at `docs/reference/core/Request.md`
(there is none today), and the leak sweep — the cached decoded value is a new refcount edge held across a
request boundary, so it gets a `valgrind` run of its own rather than riding the general one.

## Standing decisions

- **Both members land, `json()` and `jsonAs<T>()`** — the pair mirrors `Core\Json`'s own `decode`/
  `decodeAs` and R6 decides the spelling. This is settled; it is not re-opened on the grounds that
  `json()` alone is close to `rule:core-api/tier-placement` test 6's line. It is not an alias for `Json::decode(body())`: it
  takes a *claim* on the body, which the composition cannot express, and that is `post()`'s own defence.
- **The rule generalizes; it is not patched.** Adding `json` to a three-member exclusive set and giving it
  `post()`'s hold-and-re-read as a second special case was considered and rejected — two exceptions to a
  rule are the rule, unwritten.
- **`json()` holds its decoded `Value`, and `jsonAs<T>()` holds nothing.** `decode` produces only arrays
  and scalars and arrays are COW, so handing out a refcount bump is safe; `decodeAs<T>` builds objects and
  two callers must never be handed the same one. **What this spends**, per
  `rule:programs/memory-priority`'s ledger: the held octets, ≤ `[limits] request_body`
  (8 MiB) per in-flight request — already `post()`'s bill — plus the decoded value for a request that
  called `json()`, freed with the request and O(in-flight), never O(requests served).
- **The fallback, if the held `Value` fights the carrier's ownership rules**: hold the octets only and
  re-decode per call, which is exactly `post()`'s shape and costs latency rather than an invariant.
  Decided-and-recorded in `nvs-runtime`'s `Inbound` doc comment, never `BLOCKED`.
- **No `Content-Type` gate**, on `post()`'s stated reasoning. A mislabelled but valid document is read; a
  malformed one throws `ParseError`. `rule:errors/ambiguous-input-refused`
  refuses ambiguity, not mislabelling.
- **An absent or empty body is a `ParseError`, never `null` and never `LogicError`.** `?mixed` cannot
  distinguish "no body" from a body holding the document `null`, and a peer must never be able to make a
  program throw `LogicError`.
- **This goal may open the body-read rule (one new `http-server/` fragment and its record) and no other new number.** Everything else is an amendment folded into
  the existing body: spec § 15's roster and exclusivity sentence,
  `rule:http-server/the-body-is-read-on-demand-under-two-caps` and
  `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename` and `rule:http-server/a-part-is-consumed-in-one-of-three-ways`
  where they state the old rule, and
  `rule:testing/nvst-is-separate`, whose "`.nvst` is unchanged" sentence
  becomes "unchanged as a format, and the `.phpt` superset now includes its request sections".
- **`nvs run --request <file>` is a documented flag, not an environment variable.** A variable that
  changes whether a program is answering a request is a semantic change nothing at the call site shows,
  and the flag is independently useful for reproducing a request without a listener. Its file format is
  `crates/nvs-test`'s to own, since that crate writes it.
- **`--POST--` and `--POST_RAW--` in one case is a parse error.** Two spellings of one body, merged, is
  the silent-wrong-answer this repository refuses everywhere else.
