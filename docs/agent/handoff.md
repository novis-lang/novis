# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and **stages 3 to 7 are complete**. Stage 7's two checks are green: `Core\Request::bytes` is
registered and shares `body()`'s hold, `Core\Test::request` takes a `{headers, body}` bag, and the
stage's three `.nvst` cases are on disk with a fourth pinning the qualifier. Nothing is blocked.

**`bytes()` is `body()`'s reading for octets, not a second copy of them.** Both claim
`BodyNeed::Octets` and answer out of `held_octets` (`crates/nvs-stdlib/src/request.rs:2879`), so either
may follow the other and the wire is pulled once —
`rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it` now lists five
buffering readers rather than four. `body()` refuses a body that is not UTF-8 with a `ParseError`
naming `bytes()`, because a `string` is UTF-8 for its whole lifetime
(`rule:types/string-is-utf8`) and a lossy decode would change a signed payload without saying so.

**A synthetic request is described, never assembled.** `described`
(`crates/nvs-stdlib/src/test.rs:1902`) turns the two parameters and the bag into an
`nvs_runtime::InboundSpec`, which is what derives `content-type` and `content-length` and what leaves
a field line the bag wrote alone; `Core\Test::request` then hands the built carrier to
`nvs_runtime::inproc::answer`. That split is also what makes the bag assertable without a compiled
unit under test. `nvs-stdlib`'s `test` module gap 3 is closed and gone.

## Next group

**Stage 8: the routes, the link half** — one file set: `crates/nvs-stdlib/src/router.rs` and
`crates/nvs-types/src/links.rs`.

- [ ] **An enum capture matches its backing value and hands the case over** — a `{param}` declared as
      a `Core` enum matches the spelling in the path and the handler receives the case, not the text.
      The `#[Route]` half is `crates/nvs-types/src/routes.rs:1` and the match is
      `crates/nvs-stdlib/src/router.rs:231`'s class.
      `rule:routing/link-name-and-params-are-checked`, `rule:routing/route-attribute`.
      Case: `tests/conformance/core/router-an-enum-capture-matches-its-backing-value-and-hands-over-the-case.nvst`.
- [ ] **`url` prepends the mount prefix the door stripped, and refuses a case outside the captured
      subset** — the row is `crates/nvs-stdlib/src/router.rs:233` with its helper at
      `crates/nvs-stdlib/src/router.rs:1351`, and the compile-time half is
      `crates/nvs-types/src/links.rs:75`. `rule:routing/link-name-and-params-are-checked`.
      Cases: `tests/conformance/core/router-url-prepends-the-mount-prefix-the-door-stripped.nvst` and
      `tests/conformance/reject/router-url-refuses-an-enum-case-outside-the-captured-subset.nvst`.
## Backlog

- Stage 8's second file set — a served request receives its mount's resolved origin, and a mount
  whose unit calls `urlAbsolute` with no origin refuses the boot (`crates/nvs-cli/src/serve.rs`,
  named tests `a_served_request_receives_its_mounts_resolved_origin` and
  `a_mount_whose_unit_calls_url_absolute_and_resolves_no_origin_refuses_the_boot`). Take it as its
  own group once the link half above lands.

- `Core\Test::request`'s bag spells one header line per name; a repeated field wants
  `ANSWER_HEADER`'s second arm (`crates/nvs-stdlib/src/test.rs:224`) — owned by this goal if a case
  ever needs it.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `crates/nvs-server/src/bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
- `docs/plan/m7.md`'s carrier list and `crates/nvs-cli/src/serve.rs:79-90`'s "no configuration" gap
  are stale prose — goal `plan-truth`.
- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
- `nvs-cli`'s `a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile`
  (`crates/nvs-cli/src/script.rs:1387`) failed once beside the other test binaries and passed alone
  and on the next full run — it leans on timing under load, which `verify.py` § *Why `test` runs its
  binaries side by side* says to fix in the test rather than by running it apart.
