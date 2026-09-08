# Handoff

## State

**Goal 16 — stage 5 is closed: every class `Core\Request` hands out now has a page.**
`docs/reference/core/Request-Mount.md` and `Request-PartContent.md` join the three landed last session,
`docs/novis.md` is regenerated around them, and `python tools/reference.py` reports 291 of 291 examples
hold. No `Core\Request` class renders from its reference cards alone any more. Nothing is blocked.

**The driver's red check is stage 2, and that is unwritten work, not a regression.** `nvs-test`'s five
request sections already *parse*: `crates/nvs-test/src/case.rs:534` builds the body, query, cookies and
headers, and `crates/nvs-test/src/case.rs:1039` already refuses `--POST--` beside `--POST_RAW--`. What
the check names is the other half — that a section reaches the `Wire` frozen into the file
`nvs run --request` reads — and the tests it lists do not exist under any name. Stage 2 is the earliest
failing stage, so it is the next group.

## Next group

**Stage 2: a section becomes the request the case answers** — one file set: `crates/nvs-test/src/request.rs`,
`crates/nvs-test/src/case.rs`.

- [ ] **The five section tests the check names** — `a_get_section_becomes_the_requests_query_string` and
      its four siblings, each asserting a section reaches the `Wire`, not merely the `Request`:
      `render` is `crates/nvs-test/src/request.rs:79` and `read` is `crates/nvs-test/src/request.rs:129`,
      with the parse that feeds them at `crates/nvs-test/src/case.rs:534`. `rule:testing/nvst-is-separate`
      is the rule — `.nvst` is unchanged as a format and the `.phpt` superset now includes these sections.
- [ ] **`a_post_and_a_post_raw_section_together_are_a_parse_error`** — the claim is already pinned at
      `crates/nvs-test/src/case.rs:1039` under the name the corpus took, so read that test before writing
      one: this is the playbook's "a drafted name describes a claim already pinned elsewhere", and the
      repair may be the check's name in both `docs/agent/loop-goal.toml` and
      `docs/agent/goals/16-request-json.toml` rather than a new test.
- [ ] **The stage's `nvs-cli` half, if the ceiling allows** —
      `a_request_file_becomes_the_inbound_the_program_answers` and
      `a_run_without_a_request_file_answers_no_request` over `crates/nvs-cli/src/main.rs:1`. A different
      file set, so take it only as a third slice, never as the first.

## Backlog

- Stage 2's three conformance cases (`tests/conformance/core/a-request-reads-*.nvst`) — `docs/agent/loop-goal.toml:5429`.
- `Request-Files.md`'s `[limits] request_body` clause names a constant; it needs editing when the
  configuration slice lands — `docs/reference/core/Request-Files.md:34`.
- `json()` and `jsonAs<T>()` themselves are still the goal's own subject — `docs/agent/loop-goal.md`.
