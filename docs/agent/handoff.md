# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31 has landed P15, D34, D5, D24, D25, D14 and U15**; U11 and D15 are what is left of it.

**D14's verdict is that the finding was right**, against the item's own guess: ADR 0006
§ *Decision* pins `null` for a child that ends without a top-level `return` and names it
*deliberately not* `require`'s `1`, so `crates/nvs-ir/src/lower/mod.rs`'s doc comment was quoting
ADR 0021 § 3 at a frame that is not a `require`. `ScriptRole` is where the two part — the entry
frame seals with `null`, a `require`d file with the tagged `1` — and its own doc owns why the
difference cannot be made at the isolate's join instead. 19 `nvs-ir` snapshots moved with it.

**U15**: `--filter` now selects a program's `#[Test]` methods by the same containment rule the
`.nvst` tree's filter uses, over `Class::method` — `crates/nvs-cli/src/runner.rs`'s `selected` owns
it — so `--filter Class::` is a whole-class selector and a class with nothing selected is neither
announced nor charged for its `#[Fixture]`s. Case-sensitive on both sides, which is why the
finding's own `--filter clock` still selects nothing a name spells `Clock`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1023 against its
1050 floor, differential 206 against 210, six of eight named cases written.

**A `#[Test]` cannot read the configuration.** `--RUN-- test` runs each test in its own isolate but
`runner::run` resolves no tree, so `Core\Config` answers empty there. The playbook's *Writing a test
case* bullet owns the spelling; U11 below is the finding that has to work around it.

**`orient.py` still prints four dead `[context] modules` patterns** —
`crates/nvs-stdlib/src/capability.rs` (it is `crates/nvs-config/src/capability.rs`),
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs`. Fixing that manifest is in `## Backlog`. This session also
needed `crates/nvs-host/src/isolate.rs`'s `finish`, `crates/nvs-cli/src/script.rs` and
`crates/nvs-cli/src/runner.rs`, none of which the pack's `[context] modules` names, and ADR 0006
§ *Decision* — an `[context] adrs` gap, since that section is where the child's own answer is
written down.

## Next group

**The last two of item 31, in the order the acceptance check meets them.** They share the
configuration reader: `crates/nvs-config/src/secret.rs`, `crates/nvs-config/src/request.rs`,
`crates/nvs-stdlib/src/config.rs` and the two `.nvst` cases the check names.

- [ ] **U11** — `password_file` is not materialized under `nvs run`, so
      `Core\Config::get("db.main.password")` is `null` and `config dump` shows the path where
      `secret.rs` says `<secret>` (ADR 0103 § 7; docs/reference/findings.md:102). `materialize` at
      `crates/nvs-config/src/secret.rs:67` is the reader, `crates/nvs-config/src/snapshot.rs:109`
      the build that should call it on the `run` path, and `crates/nvs-cli/src/config.rs:234` the
      dump's own comment about what the stream holds. The case the check names is
      `tests/conformance/core/password-file-is-read-under-run.nvst`.
- [ ] **D15** — `Core\Config::get("mode")` answers `null` and `set("mode", …)` returns `false`
      while `mode.default` works for both; ADR 0091 § 4 spells the bare `mode`. The two arms are
      `crates/nvs-config/src/request.rs:83` and `:93`, reached from
      `crates/nvs-stdlib/src/config.rs:172` and `:189`. The case is
      `tests/conformance/core/mode-default-is-read-and-set.nvst`.

## Backlog

- `docs/agent/loop-goal.toml`'s `[context] modules` names four paths that match nothing, and no
  `[context] adrs` entry for ADR 0006 § *Decision* — the manifest is the file.
- Stage 9's items 21–23 (ADR 0119's expression `catch`), which resume when stage 0c is green.
- Stage 8: conformance 1023 against the 1050 floor, two of eight named cases unwritten.
- `docs/reference/findings.md` items 32–35, each with its own acceptance check.
