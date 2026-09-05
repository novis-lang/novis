# Handoff

## State

**Goal 6, M7 — stage 9's `-p nvs-server` check has seven names and two of them now exist and
pass.** `the_executable_path_set_after_boot_equals_the_expanded_mount_table` and
`the_path_traversal_suite_passes` are both in `crates/nvs-server/src/mount.rs`'s test module; the
first sweeps a generated corpus through `Table::resolve` and compares the set of `What::Run`
answers against `nvs_config::mount::expand`'s entries, the second is one row per traversal
technique with § 4 step 5 as every row's expectation. `Fake` there now implements
`nvs_config::resolve::Files` as well as `Existing`, so one described filesystem is both what the
boot enumerated and what steps 3 and 4 probe.

**The five names left are the whole of what stage 9 owes**:
`the_state_bleed_suite_passes_within_a_request_and_across_an_isolate_boundary`,
`a_multipart_body_far_over_the_memory_bound_is_received_at_bounded_resident_memory`,
`the_header_injection_suite_passes`, `the_request_smuggling_suite_passes` and
`a_client_disconnect_leaves_no_isolate_behind`.

**The rustdoc gate is green again**, and it was red on three links rather than the one the driver
reported: `crate::check::check_method` and `UnderTest::answer` are both private to their own
module, so no intra-doc link to either can resolve. All three are code spans now, as the five
sibling mentions of `check_method` in that crate already were. `python tools/verify.py --doc`
reports one error at a time, so a session that fixes the named one should re-run it.

## Next group

**The door's own claims, sharing `crates/nvs-server/src/serve.rs` and
`crates/nvs-server/src/secure.rs`** — every item below is about what `serve_connection` and
`answer` do with bytes a peer chose, so both files are open for any of them. Take them in this
order: the first loads the response-writing half the second and third both reason about.

- [ ] **The header injection suite passes** — ADR 0074 § 1. The door's half is
      `crates/nvs-server/src/serve.rs:1140`, inside `crates/nvs-server/src/serve.rs:1089`'s
      `answer`, which builds a `HeaderName`/`HeaderValue` from each `DeclaredHeader` the program
      set and `continue`s past a pair it cannot spell. **This is a two-layer defence and only one
      layer is in this crate**: the comment at `crates/nvs-server/src/serve.rs:1128` says
      `Core\Response::setHeader` refuses a non-token name and a value outside printable ASCII *at
      the member*, which is `nvs-stdlib`'s and not reachable from a `-p nvs-server` fixture. So
      the suite here asserts the door drops what a member would never have sent — CR, LF, NUL and
      a bare `\n` in a value, a name carrying `:` or a space — and the member's own refusal is a
      second case in the crate that owns it. `crates/nvs-server/src/secure.rs:158`'s `Secure::fill`
      is the other end of the same file set: it writes a policy header only where the name is
      absent, so a program header overrides rather than appends, and an injected one must not be
      able to split that.
- [ ] **The request smuggling suite passes** — ADR 0097 § 1's h1-only door, at
      `crates/nvs-server/src/serve.rs:603`'s `serve_connection`. Most of the framing is `hyper`'s;
      what this crate can assert is what the door does with a request `hyper` accepted — a
      conflicting `Content-Length` and `Transfer-Encoding`, a duplicated `Content-Length`, an
      obs-fold header — and that a connection carrying one is answered once and not twice. The
      playbook's bullet about a second connection applies: one socket, and assert the server side
      through what `serve_on_this_core` reports when it returns.
- [ ] **A client disconnect leaves no isolate behind** — M7's acceptance paragraph, at
      `crates/nvs-server/src/serve.rs:603`. A connection is a child task and the request is an
      isolate on it; the assertion is that dropping the socket mid-request leaves nothing parked,
      which `serve_on_this_core`'s tail already waits for.

## Backlog

- `a_multipart_body_far_over_the_memory_bound_is_received_at_bounded_resident_memory` — ADR 0105;
  read `crates/nvs-server/Cargo.toml` before writing it, the playbook's first bullet is about
  exactly this check's siblings.
- `the_state_bleed_suite_passes_within_a_request_and_across_an_isolate_boundary` — item 25, a
  parameterisation of one suite and not two, `docs/plan/m7.md`'s acceptance paragraph.
- ADR 0079 § 18's headers half of `Core\Test::request` — `crates/nvs-stdlib/src/test.rs`'s module
  doc owns why the body half is blocked and the headers half is not.
