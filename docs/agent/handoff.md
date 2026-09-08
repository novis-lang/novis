# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stages 0, 2 and 3 have
landed; stage 4 is half-open and 5 is untouched.** Stage 1 is goal 15's whole list.

**`Core\Request::json()` is on disk and green.** It is `Core\Json::decode` over the octets `body()`
holds: it claims with `BodyNeed::Octets`, reads `held_octets`, and decodes through `crate::json::read`
at `crate::json::DECODE_OPTIONS`' depth — so it follows `body()` and `post()` in any order, refuses
after a streaming reader, and consults no `Content-Type`. An absent, empty, non-UTF-8 or malformed
body is one `ParseError` carrying one issue, in `Core\Json::decode`'s own shape.
`crates/nvs-stdlib/src/request.rs:2050` is the member and `:2078` the decode.

**`jsonAs<T>()` is unwritten**, so the spec bullet names `json` alone and the rule fragment stays
`status: designed` — its table says `json()` keeps the `Value` it decoded, and that half is not on
disk either.

**The decoded `Value` is not held; `json()` re-decodes from the held octets per call.** That is the
goal's stated fallback, in place rather than skipped, and nothing is blocked on it. **When the hold
lands it has to carry the depth it was decoded at**:
`tests/conformance/core/a-request-reads-its-body-as-json.nvst` pins that `{maxDepth: 2}` after a
default-depth call still throws — the option is the call's, not the request's — so an unconditional
hold turns that case red. Hold the pair and re-decode where the depth differs.

Acceptance is still red on `examples/json-body.nvs`, stage 5's fixture and unwritten — an open item,
not a regression.

## Next group

**Stage 4: `jsonAs<T>()`, and the hold `json()` owes** — one file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/json.rs`,
`crates/nvs-runtime/src/ctx/inbound.rs`, and cases under `tests/conformance/`.

- [ ] **`jsonAs<T>()` registers and hydrates the octets `json()` reads** — the row goes beside
      `json`'s at `crates/nvs-stdlib/src/request.rs:282` with `return_ty: CoreTy::Written("T")`, and
      `crates/nvs-stdlib/src/registry.rs:2382`'s `WRITTEN_CLASS_MEMBERS` is what makes a `<T>` legal
      at the call site. The body is `crates/nvs-stdlib/src/request.rs:2050`'s shape with
      `crates/nvs-stdlib/src/json.rs:928`'s argument layout: slot 0 the `ClassDesc`, slot 1 the list
      flag, the option bag last. It holds nothing beyond the octets.
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
- [ ] **The decoded `Value` is held on the request, with the depth it was decoded at** — the slot
      goes beside `held` at `crates/nvs-runtime/src/ctx/inbound.rs:298`, and
      `crates/nvs-runtime/src/ctx/inbound.rs:708`'s `hold_parts` is the `Box<dyn Any>` precedent for
      the shape but **not** for the ownership: a `Value` in the carrier owes a `release()` when the
      request drops, which `parts` does not model. A newtype with `Drop` in `nvs-runtime`, and a
      `valgrind` run on the new refcount edge.
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
- [ ] **The cases and the two gates** — the three `.nvst` files `loop-goal.toml`'s stage-4 check
      names for `jsonAs` (`tests/conformance/core/a-request-hydrates-its-body-into-a-declared-type`,
      `…missing-a-declared-field-is-refused`, `…past-max-depth-is-refused`, and the `reject/` one for
      an unqualified `T`), the spec bullet at `docs/spec/01-core-library.md:1078` gaining `jsonAs`,
      and `python tools/reference.py` re-run. The unit tests still owed are
      `json_decodes_the_body_and_the_second_call_answers_the_held_value`,
      `json_as_hydrates_a_declared_type_and_never_shares_an_object` and
      `json_and_json_as_share_one_reading_of_one_body`, beside
      `crates/nvs-stdlib/src/request.rs:5040`'s two.

## Backlog

- `examples/json-body.nvs`, stage 5's fixture — `docs/agent/loop-goal.toml:5509` names its six lines.
- The rule fragment leaves `status: designed` once both members and the hold are on disk —
  `docs/rules/http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it.md`.
- `rule:testing/nvst-is-separate`'s "`.nvst` is unchanged" sentence still owes the amendment the
  goal's standing decisions name.
