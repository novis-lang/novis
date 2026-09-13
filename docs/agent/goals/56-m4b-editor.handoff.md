# Handoff

## State

**Goal `m4b-editor` (the extension is tested in a real editor host and built by CI) has just started;
nothing of it has landed yet.** Its Stage 1 floor is goal `m5-proofs`'s whole list.

The design is settled in the goal's § *Standing decisions*. What a session must not re-decide:

- The host tier runs locally as well as in CI, isolated, and not memoized.
- The `.vsix` is an ignored build artefact (`.gitignore:98`).
- Colour is asserted through the editor's commands, never as pixels.
- The test surface `activate` returns is read-only.
- There is one new record, which modifies one rule and creates none.

CI is not running (a billing block), so the CI stage is proven by reading `ci.yml`.

## Next group

**Stage 2: the record**. One file set: `docs/decisions/` and `docs/rules/ide*`, plus the floor
comment in `docs/agent/goals/56-m4b-editor.toml`.

- [ ] **The record**: the next free number in `docs/decisions/`, with `changes.modifies`
      `ide/headless-gates-the-loop-the-host-run-gates-the-milestone`. Its body is the goal's standing
      decisions on the host tier, argued. It covers why isolation answers the attach-and-exit and
      `settings.json` reasons the rule gave for "CI only", and why the host check is not memoized
      (`tools/loop.py:1606`).
- [ ] **The rule rewritten whole**, in `docs/rules/ide/headless-gates-the-loop-the-host-run-gates-the-milestone.md:1-16`.
      Its title goes in `docs/rules/ide.json`, and `because` gains the record. Its body names
      `npm run test:host`, then run `python tools/rules.py --render`
      (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`).
- [ ] **The floor comment**: in `docs/agent/goals/56-m4b-editor.toml`, the comment over the
      `vsix packages` floor check is rewritten to point at the amended rule.

## Backlog

- Stage 3 is the host harness, in `editors/vscode/package.json`, `scripts/host.mjs`,
  `scripts/headless.mjs`, `test/host/`, the contributions allowlist and `README.md`. It is the keystone.
- Stages 4 and 5 are the host suites, in `editors/vscode/test/host/`. Stage 5 also touches
  `src/extension.ts`, `redactions.ts`, `ast.ts` and `tasks.ts`. They share the harness with stage 3.
- Stage 6 is CI, in `.github/workflows/ci.yml`, `tools/ci-changes.py` and `editors/vscode/.nvmrc`. It
  gets its own session.
- Stage 7 is the fuzz run, in `docs/agent/commands.md:592-617` and `fuzz/seeds/prefix/`. Whether WSL has
  nightly and `cargo-fuzz` is not checked.
- Stage 8 is the three flips, in `docs/rules/ide*` only.
- When this goal's last check goes green the driver takes goal `gap-zero`.
