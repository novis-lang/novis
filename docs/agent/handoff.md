# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stages 0, 2 and 3 have
landed; stage 4 is most of the way and 5 is untouched.** Stage 1 is goal 15's whole list.

**`Core\Request::jsonAs<T>()` is on disk and green**, at `crates/nvs-stdlib/src/request.rs:2076`.
It is `Core\Json::decodeAs` over the octets `body()` holds: `WRITTEN_CLASS_MEMBERS` puts the
`ClassDesc` in slot 0 and the list flag in slot 1, it claims with `BodyNeed::Octets`, and it holds
nothing of what it built — every call hydrates the hold again, which is the rule's own row for it.

**The decode is two calls now, and that is what a request-side caller needs.**
`crates/nvs-stdlib/src/json.rs`'s `check_codec` asks the class's questions, `hydrate` builds from a
document already read, and `decode_as` is the two with `read` between them. The split is a borrow:
`held_octets` borrows the `Ctx` the hydration then needs mutably. All three carry a `member` string,
so a refusal names the member the program wrote rather than the decoder it reached; the case at
`tests/conformance/core/a-json-body-missing-a-declared-field-is-refused.nvst` pins that message.

**The taint roster at `crates/nvs-types/src/core_lib.rs:1440` names `jsonAs` and says why it is
plain**: a `T` written at the call site has no return type for the mark, so
`rule:security/derived-codec-qualifiers` asks for it on that class's declared fields instead. That
half of the rule has no diagnostic yet — see the backlog.

**`json()` still re-decodes per call**; the held `Value` is unwritten and is the next group's first
item. `tests/conformance/core/a-request-reads-its-body-as-json.nvst` pins that a `{maxDepth: 2}`
call still throws after a default-depth one, so the hold has to carry the depth it was decoded at.

The rule fragment stays `status: designed` until that hold lands. Acceptance is still red on
`examples/json-body.nvs`, stage 5's fixture and unwritten — an open item, not a regression.

## Next group

**Stage 4: the hold `json()` owes, and the unit tests the goal check names** — one file set:
`crates/nvs-runtime/src/ctx/inbound.rs`, `crates/nvs-stdlib/src/request.rs`.

- [ ] **The decoded `Value` is held on the request, with the depth it was decoded at** — the slot
      goes beside `held` at `crates/nvs-runtime/src/ctx/inbound.rs:298`, and
      `crates/nvs-runtime/src/ctx/inbound.rs:708`'s `hold_parts` is the `Box<dyn Any>` precedent for
      the shape but **not** for the ownership: a `Value` in the carrier owes a `release()` when the
      request drops, which `parts` does not model. A newtype with `Drop` in `nvs-runtime`, and a
      `valgrind` run on the new refcount edge. The reader is
      `crates/nvs-stdlib/src/request.rs:2059`, which re-decodes today. The rule fragment leaves
      `status: designed` in the same slice — its table already says `json()` keeps its `Value`.
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
- [ ] **The three unit tests the stage-4 check names**, beside
      `crates/nvs-stdlib/src/request.rs:5040`'s two and using
      `crates/nvs-stdlib/src/request.rs:5017`'s `read_json` shape:
      `json_decodes_the_body_and_the_second_call_answers_the_held_value`,
      `json_as_hydrates_a_declared_type_and_never_shares_an_object` and
      `json_and_json_as_share_one_reading_of_one_body`. **Ask first whether the last two can be
      hosted here at all**: they need a `ClassDesc` carrying a derived codec, which no
      `-p nvs-stdlib` test builds today, and the playbook has a family of bullets on a check named in
      a crate that cannot host it. If they cannot, split the check rather than moving it whole.
## Backlog

- `tests/conformance/reject/a-json-body-hydrated-into-an-unqualified-type-is-refused.nvst` is a
  **compiler slice, not a case**: the tainted half of `rule:security/derived-codec-qualifiers` has no
  diagnostic, only its `secret` half at `crates/nvs-diagnostics/src/lib.rs:966`. Next free code in
  the band is `E0810`.
- `examples/json-body.nvs`, stage 5's fixture — `docs/agent/loop-goal.toml:5509` names its six lines.
- The goal's `[context]` manifest printed no entry for spec § 15's `Core\Request` bullet
  (`docs/spec/01-core-library.md:1078` and its body-reader paragraph), which every member landing in
  this goal has to edit; it cost a read each time.
