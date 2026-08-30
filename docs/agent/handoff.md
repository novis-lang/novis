# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31 is closed.** P15, D34, D5, D24, D25, D14, U15, U11 and D15 have all landed, so the next
group is item 32's — the panics that are missing refusals.

**U11's verdict is that the finding was right about the symptom and wrong about the cause**: the
secret file *was* read at boot. `Snapshot::retype` rebuilds the typed tree out of the merged table,
which holds `password_file` and never the content, so the value died at the snapshot boundary. It is
carried beside the table now — `nvs_config::secret::Secret`, on `Resolved` and on `Snapshot` — and
`secret::apply` puts it back inside `retype`, which is the one place every deserialization goes
through (a reload retypes a second time). `Core\Config::get`/`all` read it by name and `nvs config
dump` prints ADR 0103 § 9's `<secret>` row naming the file; `dump --toml` serializes the table, so it
still cannot carry a credential. `crates/nvs-cli/src/config.rs`'s doc comment claiming the opposite
was the bug, per *an ADR's body always states the current rule*.

**D15's spelling half was not a bug** — ADR 0091 § 4 says `mode.default`, and a bare `mode` is a
limit's name. What was missing is what the flip *does*: § 4's last bullet re-derives § 3's five
defaults and § 5's ceiling bounds it. `crates/nvs-config/src/mode.rs` is the one home of that closed
table and of the two-value order; `Request::flip_mode` applies both, leaving alone any row the file
or the request already set. **Known gap, in that module's doc:** § 3's defaults are applied by the
runtime flip only, never at boot, so an unset directive still has to be read as its production
default.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: six of eight named cases
written.

**A `#[Test]` cannot read the configuration.** `--RUN-- test` runs each test in its own isolate but
`runner::run` resolves no tree, so `Core\Config` answers empty there. The playbook's *Writing a test
case* bullet owns the spelling.

**`orient.py` still prints four dead `[context] modules` patterns** —
`crates/nvs-stdlib/src/capability.rs` (it is `crates/nvs-config/src/capability.rs`),
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs`. Fixing that manifest is in `## Backlog`. This session also needed
`crates/nvs-config/src/{directive,tree}.rs`, `crates/nvs-cli/src/config.rs` and
`crates/nvs-test/src/{case,run}.rs`, none of which `[context] modules` names, and ADR 0091 §§ 3-5 and
ADR 0103 §§ 7, 9 — an `[context] adrs` gap, since those sections are the rules items 31's last two
were judged against.

## Next group

**Item 32 — the panics that are missing refusals, `nvs-types` first.** They share the checker's call
and literal paths: `crates/nvs-types/src/expr/members.rs`, `crates/nvs-types/src/core_lib.rs` and the
`.nvst` cases that pin each refusal.

- [ ] **P12 + P16, M5, M8, M9, D30** — a member the registry does not hold on a `Core` *instance* is
      `E0405` like the static miss, refused in the checker rather than panicking in the lowering, and
      an unregistered `Core\…` name is no longer trusted (ADR 0008, ADR 0061;
      `crates/nvs-ir/src/lower/expr.rs:2682` is where it panics today,
      `crates/nvs-types/src/core_lib.rs:20` is the trust that has to go).
- [ ] **P2 and P3** — an instance method called statically is E0458's user-class sibling, and `$this`
      in a `static` method is a refusal rather than a panic (ADR 0008;
      `crates/nvs-types/src/expr/members.rs:411`).
- [ ] **P7 and P8** — `throw` of a non-`Throwable`, and `clone` of an array (ADR 0023 § 1), both
      refused in the checker (`crates/nvs-types/src/expr/mod.rs:1`).

## Backlog

- `[context] modules` in `docs/agent/loop-goal.toml` names four files that no longer exist, and
  `[context] adrs` is missing ADR 0091 §§ 3-5 and ADR 0103 §§ 7, 9.
- ADR 0091 § 3's defaults are not applied at boot — only by the runtime flip
  (`crates/nvs-config/src/mode.rs`'s module doc owns the gap).
- `[debug] inline` and `[log] access` are rows of ADR 0091 § 3 with no field in
  `crates/nvs-config/src/tree.rs` and no `debug` row in the directive registry, so a file cannot
  spell what a flip derives.
- Stage 8 is two `.nvst` cases short of its named eight (`docs/agent/loop-goal.md` § *Stage 8*).
- Stage 9's ADR 0119 lowering is written up and unimplemented (items 21–23).
