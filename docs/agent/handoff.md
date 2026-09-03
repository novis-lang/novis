# Handoff

## State

**Goal 6, Stage 4 item 1 is closed: `E0801` refuses `echo` beside a `Core\Response` body
member.** `crates/nvs-types/src/response.rs` is the whole rule and its module doc owns every
decision below; `nvs_diagnostics::code`'s `E08xx` band is new and its header says why.

**The rule is checked in a `#[Route]` handler and nowhere else.** ADR 0088 § 3 binds `echo` by
*context*, so which sink a body writes to is a run-time fact for every body but one — ADR 0102
§ 1 makes a handler a response body by declaration. Outside one nothing fires, which is what
keeps the nine CLI-shaped `.nvst` cases that observe a body member green; the entry-script half
of § 4's row (ADR 0097 § 4 runs a file's top-level frame as the request body) is a stated known
gap in that module doc, and closing it needs a declaration that a file is an entry.

**The reach inside a handler is that handler's own body**, closures included. State lives on
`Env::body_writers`, installed and put back per method body by `check_method`
(`crates/nvs-types/src/check.rs:605`); the two note sites are the `Echo` arm
(`crates/nvs-types/src/locals.rs:1278`) and the resolved static call
(`crates/nvs-types/src/expr/calls.rs:296`). A helper method the handler calls is not seen —
that is a whole-program question and this is body-local.

**Unchanged limits.** Two typed members in one handler are not refused: § 4's row names `echo`
and a typed writer, and inventing the second rule is the ADR's call, not a session's. `html`
and `sendFile` are already in the module's roster, so landing either member needs no edit here.
The driver's `native examples/upload.nvs` failure is stage 5's frozen `want`, not a regression.

**`[context]` gaps still open**, now reported three times: `adrs` prints `0097 §2`, `§4` and
`0106 §13` but not **`0097 §5`**, and **`0088 §§ 3-4`** — the section every Stage 4 item is
specified by — had to be sliced by hand again. `modules` still names no `nvs-cli` pattern.

## Next group

**Stage 4 items 2 and 3: what a response carries besides its body** — spec § 15 and ADR 0088
§ 4. One file set, none of it loaded by this session, which is why it stopped at one slice:
`crates/nvs-stdlib/src/response.rs`, `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-host/src/isolate.rs`, `crates/nvs-server/src/serve.rs`.

- [ ] **`setStatus`, and the status crosses the same way the content type does** — spec § 15.
      `crates/nvs-stdlib/src/response.rs:232` is the member shape to follow (declare, then
      write), `crates/nvs-runtime/src/ctx.rs:4128` is `declare_content_type`, the neighbour a
      status declaration sits beside and the model for the `Completion` field, and
      `crates/nvs-server/src/serve.rs:445` is `answer`, which turns a declaration into what the
      peer sees. A status the response never sent has no meaning off a request — module gap 1's
      reasoning applies unchanged.
- [ ] **`setHeader`, a header sink over that same channel** — spec § 15 says it overrides a
      default, so the ADR 0074 defaults `answer` writes are what it has to override:
      `crates/nvs-server/src/serve.rs:445`. The name and the value are both sinks — a header
      field cannot carry what `crates/nvs-stdlib/src/response.rs:216`'s `spellable` refuses, and
      CR/LF in either half is header injection, which M7's acceptance names as a suite.

## Backlog

- `each_body_member_sets_its_own_content_type` is filed under the `-p nvs-types` check in
  `docs/agent/loop-goal.toml:3313`; a content type is not a types question, and the crate that
  can host it is `nvs-server` or `nvs-stdlib` — fix the live copy and `docs/agent/goals/` both.
- Two typed body members in one handler disagree exactly as `echo` and one do, and nothing
  refuses it — `crates/nvs-types/src/response.rs`'s module doc, needs an ADR 0088 sentence.
- The entry-script half of § 4's sixth row — same module doc, needs a way to declare an entry.
- `Core\Response::html` waits on `Core\Html\Markup` as a registry parameter; `sendFile` on the
  mount root a path resolves against — `crates/nvs-stdlib/src/response.rs`'s module doc.
- Stage 5 (routing, sessions, uploads) is what `native examples/upload.nvs` is waiting for.
