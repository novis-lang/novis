# Handoff

## State

**Goal `m4b-editor`, stages 3, 4 and 5 are landed and green.** `npm run test:host -- --nvs <path>`
prints `host: 12 passing, 0 failing`: the isolation claim, stage 4's six colour tests, and stage 5's
five surface tests in the order the acceptance check greps for them. The first run on a machine
downloads VS Code 1.136.2 (332 MB); later ones reuse `editors/vscode/.vscode-test/` and take about a
minute.

`activate` now returns a `Surface` — the status item's text and severity, the AST view's provider, and
the ranges each visible editor was last handed per decoration kind (`editors/vscode/src/surface.ts`).
Nothing on it acts, because it is `extension.exports` and every other extension in the window can reach
it; `editors/vscode/README.md` § *Decided here* is that decision's home. `redactions.ts` records what
`draw` hands out, since a decoration cannot be read back off an editor.

Nothing of stages 6–8 is on disk. Stage 1 is goal `m5-proofs`'s whole list, untouched. CI is not
running (billing block), so stage 6 is proven by two checks that read `ci.yml` and `tools/ci-changes.py`,
and what that stage still owes beyond the workflow is a test of `tools/loop.py:1706`'s `EDITOR_READS`,
which `tools/` has no suite to host.

The pack prints this goal's § *Standing decisions* and the failing check, but never the `## Stage N`
prose that names each stage's exact `it` titles, and `[context]` has no field that would: reading it is
one `peek.py` target on `docs/agent/loop-goal.md:"## Stage N"`, which is cheap but has to be remembered.

## Next group

**Stage 6: CI builds, tests and packages the extension** — one file set: `tools/ci-changes.py`,
`.github/workflows/ci.yml`, `editors/vscode/.nvmrc`. The goal's § *Stage 6* names every marker the two
checks grep for; they are the acceptance list, so copy the spellings rather than paraphrasing them. CI
is not running, so nothing here can be proven by a run — only by the files
(`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`).

- [ ] **The `editor` lane** (`tools/ci-changes.py:34`, the `LANES` table, whose module doc at
      `tools/ci-changes.py:5` is the policy's only home): `editors/`, `crates/`, `Cargo.toml`,
      `Cargo.lock` and `.github/workflows/`, because the protocol and host suites drive the real binary
      (`rule:testing/ci-lanes`). It is emitted as an output of the `changes` job
      (`.github/workflows/ci.yml:70`).
- [ ] **`editors/vscode/.nvmrc` (new) and job `extension`** (`.github/workflows/ci.yml:105` is the
      three-platform matrix to copy; `editors/vscode/package.json:289` is every script it runs):
      `setup-node` with `node-version-file`, `npm ci`, `npm run lint`,
      `npm run test:headless` with `NVS_BIN` at the built binary, `npm run package`, and on Linux only
      an `actions/upload-artifact` of `editors/vscode/nvs.vsix` pinned by SHA
      (`rule:ide/the-lockfile-is-committed-and-build-output-is-not`). The local Node is v24.14.1 and
      `website/.nvmrc` is the shape.
- [ ] **Job `extension-host`** (`.github/workflows/ci.yml:168` is `extension-sandbox`, the neighbour it
      sits beside): `ubuntu-latest`, the same setup, then `xvfb-run -a npm run test:host -- --nvs <built
      binary>`, which is the argv `editors/vscode/scripts/host.mjs:44` reads, caching
      `editors/vscode/.vscode-test/` on the version pinned at `editors/vscode/scripts/host.mjs:31`
      (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`).

## Backlog

- `tools/loop.py:1606`'s `MEMO_DIRS` hashes `crates/` and `examples/` alone, so a cached check over
  `editors/` stays green after an extension change — the goal's § *Standing decisions* holds it open.
- `tools/loop.py:1706`'s `EDITOR_READS` has no test, and `tools/` has no suite to host one (stage 6).
- The stale trivia paragraph in `docs/plan/m4b.md` is goal `plan-truth`'s.
- The `unowned` module-doc gaps in `crates/nvs-lsp/src/index.rs` and `hints.rs` are goal
  `unowned-closures`'s.
- Publishing to the Marketplace or Open VSX stays open under `rule:ide/one-server-two-thin-clients`
  § *Revisiting*.
