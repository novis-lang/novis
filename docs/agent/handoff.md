# Handoff

## State

**Goal 6, M7 — stage 9 is the last failing gate, and its two hot-reload claims now pass.** ADR
0017's unit table is `crates/nvs-cli/src/script.rs` and it is the tree's only in-memory cache of
compiled units; `Compiler` now carries the `compiles` counter `docs/plan/m7.md`'s acceptance
paragraph asks the "exactly once" claim to be asserted against, and two tests over it — ten
thousand requests created before any of them runs compile the file once, and none of them
suspends. The module doc's § *A single core collapses the concurrent half of it* is the home of
why one core needs no `Compiling` state; the two tests are that paragraph stated as numbers.

**Stage 9's check was filed one crate away and is now split.** Both names sat under `-p
nvs-server`, which could never have run either: that crate compiles no path — it is handed a
program. `docs/agent/loop-goal.toml` and its byte-identical `docs/agent/goals/6-server.toml` now
carry a `nvs-cli (the hot-reload claims)` check beside the server one, with the reasoning in the
comment above it. **The seven names left in the `nvs-server` check do not exist yet**, and that is
the whole of what stage 9 still owes.

**ADR 0079 § 18's first mechanism is untouched** — `Core\Test::request` still carries no
`{headers: …}` and no body. `crates/nvs-stdlib/src/test.rs`'s module doc is the one home of why
the body half is blocked; the headers half is landable on its own and is in the Backlog.

## Next group

**Stage 9's `-p nvs-server` names, sharing `crates/nvs-server/src/mount.rs` and
`crates/nvs-server/src/statics.rs`** — every item below is a claim about the one table that turns
a URL into a file, so both files are open for any of them. Take them in this order: the first
builds the sweep the second reuses.

- [ ] **The executable path set after boot equals the expanded mount table** — ADR 0097 § 2, item
      26, as a *set equality* rather than a list of refusals. `crates/nvs-config/src/mount.rs:207`'s
      `expand` is the set the boot produced; `crates/nvs-server/src/mount.rs:234`'s `Table::resolve`
      is the only thing that turns a request into a file to run, and
      `crates/nvs-server/src/mount.rs:199`'s `mounts()` is what the answers are compared against.
      Sweep a generated corpus of paths through `resolve` and assert every `What::Run` it ever
      answers is a member of that set.
- [ ] **The path traversal suite passes** — ADR 0097 §§ 2 and 4 step 3, over the same
      `crates/nvs-server/src/mount.rs:234` and `crates/nvs-server/src/statics.rs:149`'s `send`.
      `crates/nvs-server/src/mount.rs:542`'s `Fake` is the `Existing` stand-in a case drives the
      disk with, so no file has to be written. One `#[test]`, not one per escape: the check name is
      a suite.
- [ ] **The header injection suite passes** — ADR 0074 § 1, at `crates/nvs-server/src/secure.rs:158`'s
      `fill`. Triage this one before writing it: `hyper`'s `HeaderValue` refuses CR/LF on
      construction, so the question is who can write a header at all, and the answer may put half of
      the suite beside `Core\Response`'s member in `nvs-stdlib` rather than here — the playbook's
      "a check name can be a conjunction, and the two halves are two crates" bullet is the shape.

## Backlog

- `Core\Test::request` carries no `{headers: …}` bag — ADR 0079 § 18, at
  `crates/nvs-stdlib/src/test.rs:1087` and `crates/nvs-cli/src/runner.rs:479`.
- `Core\Test::server(): Core\Http\Target`, and § 18's synthetic body — same helper; the body is
  blocked on a `nvs_runtime::RequestBody` over held bytes, per that module's own doc.
- Stage 9's four remaining server names: the state-bleed parameterisation, ADR 0105's high-water
  multipart case, the smuggling suite and the client-disconnect case — `docs/agent/loop-goal.toml`.
- ADR 0079 § 17's deadlock message stays blocked until § 2's parallelism lands —
  `crates/nvs-cli/src/runner.rs`'s module doc, § *What is owed*.
