# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stages 0, 2, 3a and 3b have
landed; 3c's blocking decision is now made, 3c's code, 4 and 5 are open.** Stage 1 is goal 15's whole
list, untouched. The design is settled in the goal prose's standing decisions, which the pack prints.

**The multipart question is answered: a multipart body's hold is whichever reader filled it, and only
one of the two kinds can hold the octets.** `post()` and `files()` parse such a body off the wire as it
arrives, so the hold they leave is the buffered fields and `body()` after either is refused — the
sentence `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it` already
carried for a `files()` walk, now carried for both. `body()`/`json()`/`jsonAs<T>()` hold the octets
instead under `[limits] request_body`, and a `post()` following one parses **those** rather than the
drained wire. The alternative — buffering every multipart body — was rejected because only the fields
are charged against `request_body` (`rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`), so
holding the octets would refuse at 8 MiB an upload the caps admit to 2 GiB. Recorded in the `held`
field's doc, `crates/nvs-runtime/src/ctx/inbound.rs:280`, and in the rule fragment; no new ADR number
was spent, the refinement being 0156 § 2's own sentence applied to the second reader.

Acceptance is still red on `examples/json-body.nvs`, which is stage 5's fixture and unwritten — an open
item, not a regression.

## Next group

**Stage 3c: the rule becomes true to a program** — one file set:
`crates/nvs-runtime/src/ctx/inbound.rs`, `crates/nvs-stdlib/src/request.rs`, and cases under
`tests/conformance/core/`. Take them in order; each is written against the one before.

- [ ] **`claim_body` asks what the reader *needs*, not what kind it is** —
      `crates/nvs-runtime/src/ctx/inbound.rs:656`. The kind cannot be fixed per member: a multipart
      `post()` consumes the wire when it is first and consumes nothing when a hold is already filled.
      What is fixed is what each reader needs, and the carrier already holds all three facts —
      `held`, `parts`, `claimed_by`. So the parameter is a need, and the matrix is the whole rule:
      `body`/`json`/`jsonAs` need the octets and are `Ok` iff `held` is filled or nothing has claimed;
      `post` needs the octets **or** a parse and is `Ok` iff `held` is filled, a parse exists, or
      nothing has claimed; `bodyStream`/`files` need the wire and are `Ok` iff nothing has claimed.
      The `Err` stays the first claimant's name.
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
- [ ] **`Reading` collapses into that matrix** — `crates/nvs-stdlib/src/request.rs:1232`, with
      `claim_form` at `:1255` and `form_of` at `:1292`. `First`/`Joining`/`Again` are three answers the
      matrix now derives: whether to build a parse is `parts`, whether to pull is the hold (stage 3b
      already asks it there). **`form_of` must claim after it has read `declared`**, not before, since
      a urlencoded and a multipart `post()` need different things.
- [ ] **A multipart `post()` following a buffering reader parses the hold** —
      `crates/nvs-stdlib/src/request.rs:1306`, against a new accessor beside
      `crates/nvs-runtime/src/ctx/inbound.rs:693`. `parts_mut` borrows the parse and the *wire*
      together; this needs the same both-at-once borrow over `held` instead, handing out a
      `RequestBody` cursor over the slice — in **bounded pieces, not one chunk**, because
      `crates/nvs-stdlib/src/multipart.rs:95`'s `buf` holds a whole chunk and a single-chunk cursor
      would put the body in memory twice. `Inbound`'s private `Buffered` is that shape already, minus
      the chunking. Without this, `body()` then `post()` installs a parse over a drained wire and
      answers a form with no fields.
- [ ] **`body()` becomes idempotent and the refusals are rewritten** —
      `crates/nvs-stdlib/src/request.rs:1989`, message at `:1206` and `:1255`. `body()` fills the hold
      and answers out of it, so a second call answers the same octets. The two messages must name what
      the hold has rather than "has already been read": after a multipart `post()`/`files()` it has
      the fields, which is why `body()` is refused and `post()` is not. Owe three conformance cases
      under `tests/conformance/core/` per the playbook, including both orders over a multipart body.

## Backlog

- Stage 4: `json()`/`jsonAs<T>()` themselves — `docs/agent/loop-goal.md` § *Standing decisions*.
- Stage 5: `examples/json-body.nvs` and `nvs run --request` — the acceptance check that is red.
- Spec § 15's roster sentence still names the old exclusive set — `docs/spec/01-core-library.md`.
- The rule's table row now reads "over a multipart body, the fields alone"; check `docs/novis.md`
  renders it, since that file filters to `shipped`.
