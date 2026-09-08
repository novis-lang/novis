# Handoff

## State

**Goal 33 — The extension guides an install instead of shipping a binary — has just started; nothing of it has landed yet.** Goal 32's whole list is this goal's Stage 1 floor.

**The design is settled and is not a session's to reopen.** [ADR 0155](../decisions/0155.md) landed
with `rule:ide/the-extension-guides-an-install-and-never-bundles-one`, and the goal prose's
*Standing decisions* carry its four load-bearing calls: no bundled binary on any platform, the
managed copy is tried **last** and never written to `nvs.path`, the hash check lands in the same
slice as the fetch, and nothing reaches the network without a user invoking a command. A session
that finds any of those inconvenient writes a better message, not a background task.

**What is already on disk to build on.** `.github/workflows/release.yml` builds the seven archives,
writes `SHA256SUMS` over them and attaches Sigstore provenance — none of it added for this goal, all
of it what makes the fetch checkable. `editors/vscode/src/version.ts` already computes the
`major.minor` series stage 2's selector needs. `crates/nvs-lsp/tests/extension_reference.rs` is the
shape stage 2's pin copies: one Rust test holding two files in different languages together.

**The trap that will cost a session if it is not read first:** the extension's dependency allowlist
is asserted by `contributions.test.ts`, so the release-matrix pin cannot be TypeScript — reading the
workflow means a YAML parser, and a new dependency fails that test. It is Rust for that reason and
no other.

## Next group

**Stage 2: the platform table, pinned to the release matrix** — one file set: a new
`editors/vscode/src/install.ts` and a new `crates/nvs-lsp/tests/extension_release.rs`. Nothing here
touches the network, so the whole stage is provable offline.

- [ ] **The target table** — `editors/vscode/src/install.ts`. Every `(process.platform,
      process.arch)` pair the extension claims onto one of the seven names
      `.github/workflows/release.yml:205` builds. An unsupported pair resolves to nothing rather
      than throwing; stage 4 is what reports it.
- [ ] **musl detection**, same file. `process.report.getReport().header.glibcVersionRuntime` absent
      means musl. It runs on the remote, because the extension is `extensionKind: ["workspace"]`,
      so the remote's libc is the one that decides.
- [ ] **The archive name and the series selector**, same file. `nvs-<version>-<name>.tar.gz`
      (`.zip` on Windows) matching `tools/release.py --package`; and the newest release whose series
      equals `editors/vscode/src/version.ts:@series` of the client's own version — never "latest",
      because `refusal` in that file would refuse what arrived.
- [ ] **The pin** — `crates/nvs-lsp/tests/extension_release.rs`, reading both the table and the
      workflow matrix and failing when either side gains a target the other lacks.

## Backlog

- **Stage 3 (the fetch and its refusal)** shares `editors/vscode/src/install.ts` with stage 2 and
  adds `editors/vscode/test/install/`. Cheap to take in the same session if stage 2's slices land
  under the context gate — it is the same file plus a new suite directory.
- **Stage 4 (the chain, the two commands, the status item)** is a different file set:
  `editors/vscode/src/extension.ts`, `editors/vscode/package.json`, `editors/vscode/src/version.ts`.
  It needs stage 3 to exist, because the refusal cannot offer an install that has no code behind it.
- **Stage 5 (the reference chapter)** is `docs/reference/tools/40-editor.md` alone, and is the only
  stage that touches no TypeScript.
- **Not in this goal, and not to be swept in:** the three frozen-and-unanswered commands from goal
  15 (`docs/agent/playbook.md:1261` holds that trap and its `[until:]`), Marketplace publishing, and
  any version pin or multi-version switching.
- When this goal's last check goes green the driver takes goal 50.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
