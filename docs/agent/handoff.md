# Handoff

## State

**Goal 6, M7, ADR 0083 § 2 — `Core\Socket::upgrade` is a registry row.** A new
`crates/nvs-stdlib/src/socket.rs` holds the five edits of *A `Core` member*: the row, its card, an
`address` arm, and three `.nvst` cases. **The body throws** — § 1's root isolate is not built, so
the member reports that rather than answering as though a peer were attached, and the module doc
owns what is missing behind it. `Core\Sse` (§ 5) and `Core\Topic` (§ 4) are still unregistered.

**Two design calls are recorded in that module's doc, not here.** `upgrade` answers `void` where
ADR 0083 § 2 writes a returned `Http\Response`: ADR 0077 § 4 and ADR 0102 § 1 are one rule — the
server matches and "never decides what a return value means" — so nothing in this tree can read a
handler's return, and `Core\Response` is a class of `void` members rather than a value type. **ADR
0083 § 2's own sentence still says "returning the upgrade is what performs it" and is now wrong**;
folding that into the ADR body is the first Backlog item. And the row declares only `entry` and
`args`: `nvs_types::expr::isolate` refuses `spawn script`'s `limits:`, `grants:` and `on:` with
`E_SPAWN_OPTION_UNSUPPORTED`, so declaring them here would be the silent drop that refusal exists
to prevent — a program writing `grants: {}` gets `E0486` instead.

**No `CAPABILITIES` row yet**, and that is about the body: what throws performs no effect. The
slice that opens the connection owes the declaration.

**The driver's failing check was the machine, not the tree.** `examples/cache.nvs` failed because
every `novis-db-*` container had exited; the stack is back up and the check is green with no tree
change. See the new playbook bullet.

## Next group

**ADR 0083 § 2's entry rule at its second site**, over `crates/nvs-types/src/expr/isolate.rs`,
`crates/nvs-types/src/expr/calls.rs` and `tests/conformance/core/`. Item 1 landed the row; the
method-reference half of the operand rule does **not** type-check through it yet.

- [ ] **`check_entry` runs at `Core\Socket::upgrade` too** (ADR 0083 § 2, ADR 0006 § *Decision*) —
      `crates/nvs-types/src/expr/isolate.rs:206` is the rule's one home and is `fn`, not
      `pub(crate)`; the call site to route is the `Core` static call at
      `crates/nvs-types/src/expr/calls.rs:232` (`infer_static_call`), with the resolved signature
      built at `crates/nvs-types/src/expr/calls.rs:984` (`resolved_call`). Today `Chat::run(...)`
      reaches a `CoreTy::Text(Qual::Sink)` parameter and is reported as an ordinary
      `callable`/`string` mismatch, where the ADR wants `E_SPAWN_ENTRY_NOT_A_PATH_OR_METHOD` and
      wants the method form *accepted*. Decide whether the row marks itself (a `CoreTy` for an
      entry) or the checker names the member; the first is the one that generalises to `Core\Sse`.
- [ ] **The `.nvst` half of that rule** (ADR 0083 § *Verification*, "Entry by callable") —
      `tests/conformance/core/a-socket-upgrade-declines-the-options-its-sibling-does-not-enforce.nvst:1`
      is the shape to copy for the refusal, and
      `tests/conformance/core/a-socket-upgrade-reports-the-connection-it-cannot-open-yet.nvst:1`
      for the accepting form. ADR 0006's own fixture for `spawn script`, run at this second site.
- [ ] **Fold ADR 0083 § 2's return sentence** (`docs/adr/0083-persistent-connections-are-isolates.md:125`)
      — the body must state the current rule, and "returning the upgrade is what performs it" is
      not it. `crates/nvs-stdlib/src/socket.rs:1`'s module doc holds the reasoning to move.

## Backlog

- ADR 0083 § 2's return sentence contradicts the landed row — `docs/adr/0083-…:125`.
- `Core\Sse::upgrade` (§ 5) and `Core\Topic` (§ 4) are unregistered — `crates/nvs-stdlib/src/socket.rs`'s module doc.
- No `tungstenite` in any manifest; the framing crate is pre-authorized but unspent — `docs/agent/loop-goal.md` § *Standing decisions*.
- `spawn script`'s `limits:`/`grants:`/`on:` are still refused — `crates/nvs-types/src/expr/isolate.rs:131`.
- **`orient.py` gap:** the item named ADR 0083 §§ 1-2 and the pack sliced neither. Add `0083` to `[context] adrs` in `docs/agent/loop-goal.toml`; it cost two calls to fetch what the pack exists to inline.
