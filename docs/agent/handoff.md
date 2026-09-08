# Handoff

## State

**Goal 17, stage 2 is closed.** Every name the stage's three checks ask for now runs and passes:
`crates/nvs-runtime/src/ctx/inbound.rs`'s test module carries `a_spec_becomes_an_inbound_with_every_field_it_named`,
`a_form_field_encodes_urlencoded_and_sets_its_content_type`, `a_json_field_encodes_the_value_and_sets_its_content_type`,
`a_files_field_builds_a_multipart_body_with_a_boundary` and `a_cookies_field_becomes_one_cookie_header`,
and the sixth name is a prefix of `two_body_spellings_in_one_spec_are_refused_naming_both`, which a
`cargo-named` check matches as a substring. The stage's other two checks were already on disk —
`crates/nvs-cli/src/main.rs:2032` and `crates/nvs-test/src/case.rs:1171`.

**Stages 1 and 3 were green before this session and stage 4's check is green without stage 4's work.**
`docs/agent/loop-goal.toml:5642` names five `nvs-stdlib` gates that hold on every tree, so the driver's
acceptance can now report the whole goal green while stage 4's deliverable is unwritten. That gap is
this session's next group and is recorded in [carried-gaps.md](carried-gaps.md) so it survives a goal
switch.

`python tools/verify.py` is 8 of 8 green at this commit — 3616 tests, conformance 1607, differential 276.

## Next group

**Stage 4: the signature, frozen** — one file set: `docs/rules/testing/`, `docs/spec/01-core-library.md`.

- [ ] **The rule carries `Core\Test::request`'s signature** — `docs/rules/testing/in-process-request.md:8`
      states two response readings where goal prose stage 4 item 1 asks for the shape: the options bag,
      the mutual-exclusion rule over the four body spellings, and `Core\Test\Response`'s roster
      (`status`, `header`, `headers`, `body`, `cookies`, `json()`, `jsonAs<T>()`). It gains the shape and
      nothing else — dispatch, `tainted` bodies and `#[Test(server:)]` keep saying what they say.
      `rule:testing/in-process-request` is the rule; `python tools/rules.py --render` after the edit, and
      the goal opens no ADR number for it.
- [ ] **Spec § 13's `Core\Test` row becomes a § 15-shaped bullet** — `docs/spec/01-core-library.md:999`
      is one English cell holding the whole roster; the class, its members one by one and the ADR, with
      `Core\Test\Response` getting a row of its own. This does **not** bring it under the coverage gate:
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:583` excludes §§ 13/16/17 on purpose.
- [ ] **Strike the row this pair closes** — `docs/agent/carried-gaps.md:57` is the entry, and the
      contract two headings above it says an entry leaves exactly one way.

## Backlog

- `docs/agent/carried-gaps.md:56` still names `Core\Request::clientIp`/`host`/`scheme` under owner 17;
  stage 3 landed all three, so check `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt`
  and strike what is done — the row's other half, `Response::html`/`sendFile`, is not stage 2–4's.
- `Core\Test\Response`'s `header`, `headers`, `cookies`, `json()` and `jsonAs<T>()` are documented by
  stage 4 and registered by nothing; `crates/nvs-stdlib/src/test.rs:1137` holds the two that exist.
  Registering them is outside this goal, which asks for the signature and not the members.
