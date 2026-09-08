# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Every stage is on disk.**
Stage 5's three parts landed this session: the fixture, the reference page and the leak run. The next
acceptance sweep is expected to close the goal; nothing here is waiting on a decision.

**The fixture is two files.** `examples/json-body.nvs` prints the check's six `want` lines under
`nvs run --request examples/json-body.nvsr`, and the request file — the format
`crates/nvs-test/src/request.rs:1` owns — is now named in the check's `args`
(`docs/agent/loop-goal.toml:5518`, mirrored into `docs/agent/goals/16-request-json.toml`). Without it
the first line is `rule:security/request-state-throws-in-an-isolate` refusing.

**What one program cannot show is a second body.** One request carries one, so the fixture's two
`ParseError` lines run `Core\Json::decode` over octets it holds; the wire half is
`tests/conformance/core/a-malformed-json-body-is-a-parse-error.nvst` and `nvs-stdlib`'s
`an_absent_or_empty_body_is_a_parse_error_for_json`. A body cannot be handed to `decode` at all —
its `$json` is a plain `string`, so that call is `E0401`.

**The leak run is green.** `tools/leak-check.sh` now takes `--request <file>` before the fixture list,
because `loop.py`'s whole-suite sweep runs a fixture with no arguments and so never opens the hold at
`crates/nvs-runtime/src/ctx/inbound.rs:331` — the document `json()` leaves beside the octets. Under
valgrind in WSL, that fixture reports 0 failures.

**`docs/reference/core/Request.md` is written and `docs/novis.md` regenerated around it**;
`python tools/reference.py --check` is green, and the `json`/`jsonAs` cards were already in it.

## Next group

**Stage 5: the pages the body rule's other readers have none of** — one file set:
`docs/reference/core/`, `docs/novis.md`, `crates/nvs-stdlib/src/request.rs`.

- [ ] **`docs/reference/core/Request-BodyStream.md`** — the streaming half of
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`, which
      `Request.md` names and no page introduces; the class is
      `crates/nvs-stdlib/src/request.rs:825` and `docs/reference/README.md:97` is the fence grammar.
      A page's `nvs` example is run by `tools/reference.py:21` as a bare `nvs run`, so a member that
      needs a request goes in an `nvs skip` fence with a runnable refusal beside it, as `Request.md`
      does.
- [ ] **`docs/reference/core/Request-Files.md` and `Request-Part.md`** — the upload walk and the part
      it yields, `crates/nvs-stdlib/src/request.rs:882` and `crates/nvs-stdlib/src/request.rs:930`;
      `rule:http-server/a-part-is-consumed-in-one-of-three-ways` is what the pair has to say, and
      `examples/upload.nvs:36` is the shape already proven.
- [ ] **Regenerate and prove them in one call** — `python tools/reference.py` writes `docs/novis.md`
      and runs every example; `--examples-only --only Request` (`tools/reference.py:7`) runs just
      these while writing them.

## Backlog

- `docs/reference/core/` has no `Session.md` either, and `Core\Session` is the other request-scoped
  class whose members refuse outside one.
- `rule:security/derived-codec-qualifiers`' declaration half is landed in `nvs-types`' `derive.rs`;
  nothing else of the rule is open.
- A `Core\Db\…::queryAs<T>` gets no qualifier pass: a row is not a peer's document, and
  `derive.rs:@db_reachable`'s comment says § 6 makes every text column tainted on the way out.
- `docs/agent/carried-gaps.md` owns anything that must outlive this goal.
