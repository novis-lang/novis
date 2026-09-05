# Handoff

## State

**Goal 6, M7 — ADR 0079 § 18's first mechanism is on disk.**
`Core\Test::request(Core\Http\Method $method, string $path): Core\Test\Response` runs a synthetic
request through the program under test in this process — no socket, no port — with ADR 0102 § 1's
match already on the carrier, and answers with `status()` and `body()`. The seam is
`crates/nvs-runtime/src/inproc.rs`, whose module doc is the one home of why the program crosses a
trait rather than a `Core` member holding a compiled unit, and of the **re-entry refusal**: the
entry answering an in-process request is the entry that asked, so a second one inside it is refused
at the door. `crates/nvs-cli/src/runner.rs`'s `UnderTest` is the only implementor and is installed
over both `nvs test` and `nvs run`, which is what lets a `.nvst` case reach the member at all.

**Two halves of § 18 are still owed, and they are named in `crates/nvs-stdlib/src/test.rs`'s module
doc** (§ *§ 18's in-process request, and what of it is still owed*): the `{headers: …}` bag and a
synthetic body — a body needs a `nvs_runtime::RequestBody` over held bytes, which nothing in
`nvs-stdlib` builds — and `#[Test(server: true)]`, which is the runner's mechanism rather than this
class's. ADR 0079 § 18's body now records the landed surface, including why
`Core\Test\Response` is a `Core`-owned instance with two members rather than the shape its worked
example wrote: there is no registry spelling for a member that *returns* an ADR 0036 shape.

**ADR 0079 § 14's updater is still blocked** on the same fact its helper's doc comment at
`crates/nvs-stdlib/src/test.rs` records: splicing needs the `$expected` literal's span, and no
runtime record holds one.

## Next group

**What is left of ADR 0079's runner, all of it through `crates/nvs-cli/src/runner.rs`** —
`run_in_isolate` is where a case's options are read and the child's context is armed, and
`mod tests` at `crates/nvs-cli/src/runner.rs:1415` is where each check's named test goes. The first
two share the same insertion point; the third adds a parameter to a member the first two do not
touch, so take it last.

- [ ] **`#[Test(server: true)]` gets an ephemeral listener** — ADR 0079 § 18's second mechanism, at
      `crates/nvs-cli/src/runner.rs:825`, where the option is read and the listener has to be bound
      before the isolate starts. `crates/nvs-cli/src/serve.rs:274` is this binary's one existing
      `NvsListener::bind` and `crates/nvs-cli/src/serve.rs:497`'s `serve_on_this_core` is the loop
      beside it. Decide there how the test learns the address — a `Core\Test` row is the cheap
      answer and `Core\Server` is the other one.
- [ ] **`#[Test(db: "test")]` runs inside a transaction the runner rolls back** — ADR 0079 § 17, at
      `crates/nvs-cli/src/runner.rs:825`, the same place the option is read.
- [ ] **A synthetic request carries headers and a body** — ADR 0079 § 18's first paragraph, at
      `crates/nvs-stdlib/src/test.rs:325` (the `request` row) and
      `crates/nvs-cli/src/runner.rs:434` (`UnderTest::answer`). The body half needs a
      `nvs_runtime::RequestBody` over held bytes; `crates/nvs-runtime/src/ctx/inbound.rs:578` is
      `set_body`.

## Backlog

- `nvs test --update` splices an inline snapshot — ADR 0079 § 14, blocked on the expression span;
  the design is in the helper's doc comment in `crates/nvs-stdlib/src/test.rs`.
- Raw/unparsed body access for an arbitrary content-type — `Core\Request`'s module doc owns it.
