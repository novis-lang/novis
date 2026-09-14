# Handoff

## State

**Goal `m4b-editor`, stage 2 (the record) is landed.** [0185](../decisions/0185.md) argues the host
tier, `rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone` is rewritten whole and no
longer says "CI only", and `memoize = false` is now read by the driver
(`tools/loop.py:2443`) and carried by the three host checks.

Nothing of stages 3–8 is on disk: `editors/vscode/scripts/host.mjs` and `editors/vscode/test/host/`
do not exist, so the three `test:host` checks fail on a missing script until stage 3 lands. Stage 1 is
goal `m5-proofs`'s whole list, untouched. CI is not running (billing block), so stage 6 is proven by
reading `ci.yml`.

Stage 6's "the acceptance cache sees an extension change" is already true on disk: `tools/loop.py:1682`
has an `editors` partition and `tools/loop.py:1706`'s `EDITOR_READS` gives an npm check under
`editors/` the `crates` + `editors` hash. What that stage still owes is only a test of it, and `tools/`
has no test suite to host one.

## Next group

**Stage 3: the host harness** — one file set: `editors/vscode/`. The record settles what it must do;
the goal's § *Stage 3* settles the shape.

- [ ] **`scripts/host.mjs`** (new, beside `editors/vscode/scripts/headless.mjs:23`): `runTests` against
      a pinned build cached in `.vscode-test/`, passing `--user-data-dir`, `--extensions-dir`,
      `--disable-extensions`, a copy of `test/host/fixture/` as the workspace, and `nvs.path` from
      `--nvs` into that profile's `settings.json` alone. Isolation is
      `rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`, argued in
      [0185](../decisions/0185.md) § 2; the `--nvs` fallback copies
      `editors/vscode/test/protocol/session.ts:72`.
- [ ] **`test/host/index.ts`** (new): Mocha with the `spec` reporter, report to
      `.vscode-test/host-report.txt` ending `host: N passing, M failing`, which `host.mjs` prints and
      exits non-zero on. First case `runs in a throwaway profile and never the developer's` — the
      `want` of the stage 3 check in `docs/agent/loop-goal.toml:9930`.
- [ ] **The manifest and the headless runner**: `@vscode/test-electron` and a `test:host` script in
      `editors/vscode/package.json:289` and `:299`, its allowlist entry beside
      `editors/vscode/test/contributions/contributions.test.ts:108`
      (`rule:ide/dependencies-are-allowlisted` — runtime `dependencies` stay `vscode-languageclient`
      alone), and `editors/vscode/scripts/headless.mjs:21` skipping `out/test/host/`.
- [ ] **The two README paragraphs stage 0 names**: `editors/vscode/README.md:20-23` says the host suite
      is CI's, and `:29-33` reasons *No pixel tier* from "anything needing a display sits outside the
      tier the loop gates on". The decision stands; rewrite its reason whole.

## Backlog

- `tools/` has no test suite, so stage 6's memo test has no host — decide at stage 6 (goal § *Stage 6*).
- The goal's § *Standing decisions* "Not this goal" list still names widening `MEMO_DIRS`; that work is
  on disk and the name is gone (`tools/loop.py:1682`).
- Publishing to the Marketplace or Open VSX stays open
  (`rule:ide/one-server-two-thin-clients` § *Revisiting*).
- The stale trivia paragraph in `docs/plan/m4b.md` is goal `plan-truth`'s.
- The `unowned` module-doc gaps in `crates/nvs-lsp/src/index.rs` and `hints.rs` are goal
  `unowned-closures`'s.
