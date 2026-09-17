# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `crates/nvs-stdlib/src/db/mod.rs` has **no
`# Known gaps` section left**. Both gaps are struck as stated bounds, written as the module's own
prose: pool bounds are a block's to state, and a settings key no `[db.<name>]` block describes —
an endpoint no operator wrote a block for, or a literal differing from its block in any hashed
field — takes `PoolBounds::DEFAULT` and is told nothing about it; `Db\DbError` sits outside spec
§ 10's error tree and so declares no `issues`, and a refusal about a *column* is a `ParseError`
naming them, the class `crate::json`'s `decodeAs` throws for the same question.

`rule:core-classes/db-connection-is-named` needed no amendment: its fragment
(`docs/rules/core-classes/db-connection-is-named.md`) promises a memo key and a release, and says
nothing about pool bounds, so neither strike contradicted it.

`python tools/owners.py --closes decided-closures` names **10**, down from 12.

**One flake fixed, unrelated to the goal**: `socket_max_duration_ends_an_endless_conversation`
(`crates/nvs-stdlib/src/http/socket.rs:1431`) failed under the side-by-side `cargo test` run and
passed alone — its `maxDuration` was shorter than the handshake the same clock covers. The
playbook bullet under *Writing a test case* owns the shape.

## Next group

**Stage 4: the two `crates/nvs-stdlib` gaps that wait on the HTTP response existing** — one file
set: `crates/nvs-stdlib/src/html.rs` and `crates/nvs-stdlib/src/response.rs`, the same pair of
module docs, plus the milestone plan file if either is deferred rather than struck. Neither item
carries a `Decided:` sentence, so the goal's § *Standing decisions* third route applies: strike it
as a bound, or re-tag it to a milestone at M9 or later **whose plan file states the scope** —
`python tools/owners.py --deferrals` is what proves the second, and `python tools/plan.py --show
M<N>` is where the scope has to already be.

- [ ] **`crates/nvs-stdlib/src/html.rs:74` — the sink's automatic lift.** The registered half is
      complete and the prose says so; what waits is every non-`Markup` interpolation into an HTML
      response being escaped and wrapped with no call written at the site, which waits on that
      response existing — the same wait `Core\Request` is on. Decide which milestone owns the HTTP
      response and defer to it if its plan states the scope, per `rule:core-classes/html-auto-escape`;
      otherwise state the bound. The `— owner:` line is `crates/nvs-stdlib/src/html.rs:90` and the
      item has no number, so the strike deletes the paragraph under `# Known gaps` and the heading.
- [ ] **`crates/nvs-stdlib/src/response.rs:197` — gap 1, the same gap from the other side.**
      § 3's table says the sink in force selects a rendering; `echo`'s rendering is the terminal's
      everywhere today, so this member off a request writes its bytes verbatim, and `setStatus` and
      `setHeader` off a request declare onto a context nobody asks. Take the same answer as
      `html.rs` — the two are one gap seen twice, so a deferral that splits them is the dishonest
      kind.
- [ ] **`crates/nvs-stdlib/src/cli.rs:132` — gap 1, struck as a stated bound.** Cheap, same crate,
      take it if the two above leave room: the `Decided:` sentence is *answer empty/neutral values
      and state it as the contract*, so write that `Core\Cli`'s words are the launcher's and a
      served request has none, then delete the item, its `Decided:` lines and the `# Known gaps`
      heading.

## Backlog

- `crates/nvs-stdlib/src/command.rs:74` — the one **build** left in the stdlib register: the help
  renderer reaches the handler's signature at render time, the row holding a reference rather than
  a copy. Its file set is `nvs_runtime::commands` plus `nvs_types::commands`' own gap 1, not a
  stdlib module doc, so it wants a group of its own.
- `crates/nvs-diagnostics/src/embedded.rs:30`, `crates/nvs-syntax/src/lib.rs:96`,
  `crates/nvs-types/src/defaults.rs:58`, `crates/nvs-types/src/response.rs:29` — the register
  outside `nvs-stdlib`; `python tools/owners.py --closes decided-closures` is the live list.
- `crates/nvs-stdlib/src/ast.rs:83`, `regex.rs:69` — the two stdlib items neither of this
  session's groups reached.
