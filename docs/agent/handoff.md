# Handoff

## State

**Stage 8 — the corpus and the guards — is the open stage**, and the plan's *Open now* said it was
closed because its floors were raised under it: `docs/agent/loop-goal.toml:1930` names eight
conformance cases and floors of 1050 conformance / 210 differential. Four of the eight are now
written — `tests/conformance/config/`'s pair and `tests/conformance/cap/`'s pair — and the counts
stand at 1015 and 206. Every other stage the goal names (0b, 2–7) is closed at its acceptance.

**The gate is green except for `cargo fmt --check` on `crates/nvs-cli/src/meta.rs`**, a file another
agent is holding uncommitted in this tree (it changed under this session, and nothing in this work
touches it). Everything else was run: the workspace build, `-p nvs-test`'s fmt, clippy and 39 tests,
and both suites end to end. Re-run `python tools/verify.py` once that file is committed or reverted.

**The `.nvst` format gained one `--RUN--` spelling, `config dump --origin`.** It is the only one that
runs no program: it names no file on the command line, so `nvs config dump` resolves ADR 0103 § 1
step 2's `./nvs.toml` out of the case's own working directory, and a tree written with
`--FILE nvs.toml--` / `--FILE conf.d/…--` becomes the subject. That is the only place § 3's
obligation — every override recorded with **both** origins — is observable end to end, since no
program can ask where a value was written. `crates/nvs-test/src/case.rs`'s `Subcommand` doc owns why,
including why such a case still carries a `--FILE--`.

## Next group

**The four remaining named cases of stage 8's conformance check.** One file set: `tests/conformance/`
under `config/`, `reject/`, `isolate/` and `error/`, with `--FILE nvs.toml--` as the setup mechanism
in every one of them — the two written this session are the worked examples.

- [ ] **`tests/conformance/reject/config-set-above-the-hard-ceiling-returns-false.nvst`** — the goal's
      standing decision and M6's *Verify*: above `[limits]` succeeds, above `[limits.hard]` returns
      `false` with the previous value intact, and neither throws. Anchors:
      `crates/nvs-config/src/request.rs:93`, `crates/nvs-stdlib/src/config.rs:189`.
- [ ] **`tests/conformance/config/config-set-is-invisible-to-the-next-request.nvst`** — ADR 0078 § 1's
      copy-on-write overlay. Decide first what the *second* observer is: a `.nvst` is one `nvs run`,
      so either a `spawn script` child reads the snapshot the parent wrote over, or the case belongs
      in `-p nvs-config` and the check's `cases` list is the half that is wrong (the playbook's bullet
      on a check naming a test its crate cannot host). Anchors:
      `crates/nvs-stdlib/src/config.rs:36`, `crates/nvs-config/src/request.rs:93`.
- [ ] **`tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`** —
      a parent narrows `fs.read`, the child reads inside it and is refused outside it with the same
      sentence `capability::denial` writes for the parent. Anchors:
      `crates/nvs-host/src/isolate.rs:90`, `crates/nvs-runtime/src/capability.rs:52`.
- [ ] **`tests/conformance/error/a-limit-fatal-is-not-catchable.nvst`** — ADR 0020: a `[limits]` breach
      is a `FATAL` that no `catch` sees, reported to a registered `Core\Fatal::onLimit` instead. The
      case expects the run to *fail*, so it is `--EXPECTF-ERROR--`. Anchors:
      `crates/nvs-runtime/src/ctx.rs:1210`, `crates/nvs-host/tests/limits.rs:33`.

## Backlog

- The two counts stage 8 also floors: conformance 1015 of 1050, differential 206 of 210 —
  `docs/agent/loop-goal.toml:1925`.
- Route ADR 0061's autoload probing through `nvs_diagnostics::embedded` — this session's item before
  the acceptance check displaced it — `crates/nvs-hir/src/autoload.rs:402`.
- A `.nvst`-free regression for a bundle whose `require` escapes the entry's directory —
  `crates/nvs-cli/tests/bundle.rs`.
- `[context] adrs` at `docs/agent/loop-goal.toml:85` wants ADR 0048 §§ 2-5 and ADR 0103 §§ 4, 9; both
  were opened by hand, this session and last.
- `[context] modules` names `crates/nvs-host/src/budget.rs`, which does not exist — the module is
  `crates/nvs-runtime/src/budget.rs`, and `orient.py` warns about the dead selector every session.
