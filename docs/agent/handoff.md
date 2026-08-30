# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31 has landed P15, D34, D5, D24 and D25**; U15, D14, U11 and D15 are what is left of it. D24
was four edits and not one — the codegen guard alone dies with `an arithmetic throw with no error
edge`, and the playbook's new *Writing Novis itself* bullet owns the roster. It also added
`Core\Math::fdiv`: ADR 0007 § 4 names that member as the IEEE escape hatch and it did not exist, so
with `/` throwing there was no spelling left for an infinity at all. Eight math cases used `1.0 / $z`
as an instrument for reading a zero's sign and now call `fdiv`; `crates/nvs-stdlib/src/math.rs`'s
module doc owns why the one non-throwing division is not a hole in its own rule.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1022 against its
1050 floor, differential 206 against 210, six of eight named cases written.

**A `#[Test]` cannot read the configuration.** `--RUN-- test` runs each test in its own isolate but
the runner resolves no tree (`crates/nvs-cli/src/runner.rs:245`), so `Core\Config` answers empty
there. The playbook's *Writing a test case* bullet owns the spelling, and U15 below is in that file.

**`orient.py` still prints four dead `[context] modules` patterns** —
`crates/nvs-stdlib/src/capability.rs` (it is `crates/nvs-config/src/capability.rs`),
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs`. Fixing that manifest is in `## Backlog`. This session also needed
`crates/nvs-ir/src/lower/operator.rs` and `crates/nvs-runtime/src/helpers.rs`, neither of which the
pack names, and `docs/reference/lang/30-expressions.md` — a `[context] modules` and a `[context]
reference` gap respectively, since an operator's answer is written down in all three places.

## Next group

**What is left of item 31, in the order the acceptance check meets them.** There is no shared file
set — one crate and one wrong arm each — so the 120k gate decides how many fit rather than the file
test.

- [ ] **D14** — a child that ends without `return` answers `1`; the finding says `null`. **Read the
      code before writing any**: `crates/nvs-ir/src/lower/mod.rs:1138`'s doc comment says the `1` is
      deliberate and is *PHP's own answer* — which it is, `include` of a file with no `return` being
      `1` in PHP — and `crates/nvs-ir/src/lower/mod.rs:1177` is the `Terminator::Return(Some(one))`
      that emits it. Priority 2 is PHP-compatible observable behaviour, so the likely verdict is that
      the finding is wrong and `docs/reference/findings.md:156` gets a *not a bug* note rather than a
      fix. Decide it and record the verdict either way; the fixture is `refp/spawn/noret2`.
- [ ] **U15** — `nvs test --filter` filters `.nvst` paths only and is dropped for `#[Test]` methods.
      `crates/nvs-cli/src/main.rs:893` is the parameter and `crates/nvs-cli/src/main.rs:925` where it
      reaches the options; `crates/nvs-cli/src/runner.rs:245` is the `#[Test]` table's own path,
      which never consults it. ADR 0079 § 22 is the reporting contract a filtered run still owes.

## Backlog

- **U11** — `password_file` is not materialized under `nvs run`; `crates/nvs-config/src/secret.rs` is
  ADR 0103 § 7's home. `docs/reference/findings.md:102`.
- **D15** — `Core\Config::get("mode")` answers `null` and `set("mode", …)` returns `false`.
  `docs/reference/findings.md:157`.
- **The `[context]` manifest in `docs/agent/loop-goal.toml`** has four dead `modules` patterns and no
  selector for `lower/operator.rs`, `nvs-runtime/src/helpers.rs` or `docs/reference/lang/`.
- **`docs/reference/findings.md` boxes lag the tree** — D5, D25, P15 and D34 landed and are still
  `- [ ]`. D24 is ticked; the rest want one pass.
- **Stage 9** — ADR 0119's expression `catch`, items 21–23, anchors already written. Resumes when
  stage 0c is green.
- **Stage 8** — two of eight named cases unwritten, and both floors still short.
