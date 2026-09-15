# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3, 4 and 5 are complete. **Stage 6 is half landed**: `Core\Response::html` is registered
and `echo` into a request's sink escapes, so two of that stage's three checks are green. `sendFile` is
the whole of what is left, and it is the stage's remaining `nvs-suite` and `cargo-named` check both.

**The escape has one home and it is `nvs_render::html::escape`**
(`crates/nvs-render/src/html.rs`), a peer of `text::substitute`: `Core\Html::escape` calls it,
`echo` calls it, and neither holds a table. Which transform runs is read off the sink —
`crates/nvs-runtime/src/helpers.rs`'s `write_rendered` asks `Ctx::carrier()`, so
`rule:tooling/echo-always-has-a-sink`'s carrier column and rendering column cannot disagree. The raw
path is `writes_raw`, which compares the operand's class to **that sink's** carrier rather than to the
roster, so a `Cli\Text` echoed in a request is escaped and a `Markup` echoed from a CLI program is
substituted.

`Core\Response::html` takes `CoreTy::Instance(MARKUP_NAME)` unmarked — the carrier's own doors are
the refusal — and reads its bytes through `crate::html::markup_slot`, now `pub(crate)`, so nothing
grew a second reader of the carrier's slot. Nothing is blocked.

## Next group

**Stage 6: `Core\Response::sendFile`, the last of the response body** — one file set:
`crates/nvs-stdlib/src/response.rs`, `crates/nvs-runtime/src/ctx/output.rs` and
`crates/nvs-server/src/serve.rs`.

- [ ] **The member: `Core\Response::sendFile(string $path)`** — the path alone, as a `Qual::Sink`,
      with a missing path, a directory or an unreadable file throwing at the call. The row goes in the
      `CLASS` table at `crates/nvs-stdlib/src/response.rs:266`, beside `html` at
      `crates/nvs-stdlib/src/response.rs:269`; the `fs.read` grant is a row in the capability table at
      `crates/nvs-stdlib/src/registry.rs:2145`, where `Core\IO::read` has one. The key is struck from
      `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:30`, and
      `crates/nvs-stdlib/src/response.rs:12`'s "the one still owed" goes with it.
      `rule:security/response-body-is-one-typed-member`, `rule:security/sink-predicate`.
- [ ] **The declared file body on the context** — what the member leaves behind for the isolate's
      finish path, in the shape `Core\Response::stream`'s writing half already has at
      `crates/nvs-runtime/src/ctx/output.rs:528`: a path and no bytes, taken once, so nothing is read
      whole into memory. `rule:security/response-body-is-one-typed-member`.
- [ ] **The server answers it through the static policy** — `crates/nvs-server/src/serve.rs:1499`'s
      `answer` hands a declared file body to `statics::send`
      (`crates/nvs-server/src/statics.rs:145`), which already owns the media-type table, the range and
      the conditional. The two names the check wants are
      `a_declared_file_body_is_streamed_under_the_static_policys_media_type` and
      `a_declared_file_body_answers_a_range_and_a_conditional_request`, plus the cases
      `tests/conformance/core/response-send-file-streams-the-file-under-its-media-type.nvst` and
      `tests/conformance/reject/response-send-file-refuses-a-tainted-path.nvst`.
      [0186](../decisions/0186.md) § 4.

## Backlog

- The Windows service dispatcher (`StartServiceCtrlDispatcherW` and the two beside it) is still
  unwritten; `crates/nvs-cli/src/service.rs`'s module doc owns the gap and no check names it.
- `Core\Metrics`'s three rows belong to goal `m8-stdlib-depth`, not here.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30` and its three siblings wait on goal
  `unowned-closures`' decision sheet.
