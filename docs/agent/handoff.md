# Handoff

## State

**Goal 17, stage 3 is landed whole.** `Core\Request::clientIp()`, `scheme()` and `host()` are
registered, carded, answered and `tainted`; the `.nvst` → `.nvsr` → carrier path carries
`--CLIENT_IP--` and `--SCHEME--`; and the two `nvs-server` tests the stage's acceptance list names
now assert the seam directly — `crates/nvs-server/src/serve.rs:3285` builds the walk's `Origin` and
puts it on an `Inbound`, without a listener, a thread or a program in between.

**Stage 2's filing is fixed.** `the_request_sections_build_an_inbound_spec` was filed under
`-p nvs-test`, which has no dependencies on purpose and can never name `InboundSpec`. It is now a
`-p nvs-cli` check and a unit test in the binary, over a new `inbound_of`
(`crates/nvs-cli/src/main.rs:1422`) that `inbound_from` delegates to — `nvs-cli` has no library
target, so the seam is reachable only from inside it.

**Stage 2's remaining red is five names, not five features.** Every behaviour its check asks about is
already asserted in `crates/nvs-runtime/src/ctx/inbound.rs`'s test module except the JSON body, and
no other check pins the names those tests currently carry.

## Next group

**Stage 2: the builder's acceptance names** — one file set:
`crates/nvs-runtime/src/ctx/inbound.rs`.

- [ ] **`a_spec_becomes_an_inbound_with_every_field_it_named`** — rename and widen
      `a_spec_carries_its_query_its_cookies_its_host_and_its_peer` at
      `crates/nvs-runtime/src/ctx/inbound.rs:2250` so it asks every field `InboundSpec::build`
      writes (`crates/nvs-runtime/src/ctx/inbound.rs:1245`), the method and path included.
      `rule:testing/in-process-request` is the rule.
- [ ] **`a_form_field_encodes_urlencoded_and_sets_its_content_type`** and
      **`a_files_field_builds_a_multipart_body_with_a_boundary`** — the same two assertions under the
      names the check gives, at `crates/nvs-runtime/src/ctx/inbound.rs:2120` and
      `crates/nvs-runtime/src/ctx/inbound.rs:2154`.
- [ ] **`a_json_field_encodes_the_value_and_sets_its_content_type`** — the one with no test at all;
      `SpecBody`'s JSON spelling is at `crates/nvs-runtime/src/ctx/inbound.rs:1060` and its encode is
      what `build` calls at `crates/nvs-runtime/src/ctx/inbound.rs:1274`.
- [ ] **`a_cookies_field_becomes_one_cookie_header`** — the join and the field a written `cookie`
      line suppresses, at `crates/nvs-runtime/src/ctx/inbound.rs:1266`.

## Backlog

- `__Host-`/`__Secure-` cookies still ignore the scheme on read, though the carrier now holds it —
  `crates/nvs-stdlib/src/request.rs:@cookie_of`'s doc names it as this module's known gap.
- Stage 4 is the signature frozen; every test its check names already passes under `-p nvs-stdlib`.
- `docs/reference/core/Request.md` is hand-written prose and says nothing of the three new members;
  `tools/reference.py --check` does not ask it to.
