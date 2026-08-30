# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31 has landed P15, D34, D5 and D25**; U15, D24, D14, U11 and D15 are what is left of it.
D5 is closed at its cause: `Path::parent` of a bare relative name is the empty path, which
canonicalizes nowhere, so `resolved`'s walk toward the deepest existing ancestor ran out of
components on its first step and denied every bare name under every grant —
`crates/nvs-config/src/capability.rs:287` `here` spells that parent `.` and asks the same
canonicalizer. D25 was one missing match arm: `crates/nvs-types/src/defaults.rs` accepts a written
`null` wherever `TypeInterner::is_nullable` says the declared type admits one, and that module's doc
comment owns the rule now that its *Known gap* paragraph is gone.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8 is as before**: six of its
eight named cases written, conformance/differential against floors of 1050 / 210.

**A `#[Test]` cannot read the configuration.** `--RUN-- test` runs each test in its own isolate but
the runner resolves no tree (`crates/nvs-cli/src/runner.rs:245`), so `Core\Config` answers empty
there. The playbook's *Writing a test case* bullet owns the spelling, and U15 below is in that file.

**`orient.py` printed four dead `[context] modules` patterns**, and one of them mislabelled this
session's item: `crates/nvs-stdlib/src/capability.rs` is `crates/nvs-config/src/capability.rs`.
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs` match nothing either. Fixing that manifest is in `## Backlog`.

## Next group

**Item 31's three remaining wrong answers, in the order the acceptance check meets them.** There is
no shared file set — one crate and one wrong arm each — so the 120k gate decides how many fit rather
than the file test; each is small, and the first is the check's next failure.

- [ ] **D24** — `1.0 / 0` answers `INF`. `crates/nvs-codegen/src/emit.rs:1232` runs the zero-divisor
      guard only for `Ty::Int | Ty::Uint`, so `crates/nvs-codegen/src/emit.rs:1256`'s
      `BinOp::Div if float => fdiv` divides and hands back an infinity; ADR 0007 § 4 now says `/ 0`
      throws `ArithmeticError` whatever the operand types.
      `crates/nvs-codegen/src/emit.rs:1822` `raise_arithmetic_error` is the raiser to reuse, and
      `crates/nvs-codegen/src/emit.rs:1458` is the integer call site to copy. Decide `%` and `**`
      with it — the same guard covers `Mod` and `Pow` — and say in the case which of them it pins.
      The case is `tests/conformance/error/float-division-by-zero-throws.nvst`.
- [ ] **D14** — a child that ends without `return` hands back `null`, not `1`;
      ADR 0006 § *Values cross by copy* is the rule. `crates/nvs-host/src/isolate.rs:404` and
      `crates/nvs-host/src/isolate.rs:459` are the two `Completion` sites that carry
      `Value::null()`. **Run it before assuming a code change** — the case name
      `tests/conformance/core/a-child-without-return-answers-null.nvst` says `null` is the wanted
      answer, so this may be a document that is wrong rather than a binary.
- [ ] **U15** — `nvs test --filter` filters `.nvst` paths only and is dropped for `#[Test]` methods.
      `crates/nvs-cli/src/main.rs:893` takes it and `crates/nvs-cli/src/main.rs:925` puts it on the
      options; `crates/nvs-cli/src/runner.rs:245` is the runner that never reads it. The check is
      `cargo-named` and wants a test called `test_filter_selects_test_methods_by_name` under
      `-p nvs-cli`; none of `crates/nvs-cli/tests/*.rs` mentions `filter` today, so it needs a home
      as well as a body.

## Backlog

- **U11** — `password_file` under `nvs run`, and the dump's masked value: ADR 0103 §§ 7, 9; the case
  is named in `docs/agent/loop-goal.toml:252`.
- **D15** — `mode.default` is the key ADR 0091 § 4 spells, and setting it re-derives the defaults;
  case at `docs/agent/loop-goal.toml:254`.
- **The `[context]` manifest's four dead module patterns**, `docs/agent/loop-goal.toml` — the pack
  names a `capability.rs` in the wrong crate, which cost this session an anchor.
- **Items 32–35** — the missing refusals and the rest of the findings: `docs/agent/loop-goal.md`
  § *Stage 0c*.
- **Stage 9** — ADR 0119's expression `catch`, items 21–23, nothing implemented yet.
- **Stage 8** — two of eight named cases unwritten; floors 1050 / 210 in `docs/agent/loop-goal.toml`.
