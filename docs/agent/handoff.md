# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31 is closed**, its last case (`mode-default-is-read-and-set.nvst`, D15) now on disk — the
finding had been ticked while the acceptance check's case was never written, which is what the
driver was failing on. **Item 32's first slice is closed too**: P12, P16, M5, M8, M9 and D30.

**The registry is the whole roster of `Core`, and the trust that used to sit above it is gone.** A
member miss is `E0405` wherever it is written — on an instance (`$uuid->version()`, which panicked
in `nvs-ir`), on a registered class, and on one the registry has never heard of (`Core\Env::EOL`).
What stays trusted is the bare *name*: `nvs_hir` resolves any `Core\…` without a declaration, so an
unimplemented class may still be named in a type position while no member of it can be reached.
`crates/nvs-types/src/core_lib.rs`'s module doc is that rule's one home.

**A diagnostic's help names a `Core` member only where the registry holds one.**
`nvs_syntax::parser`'s `superglobal_replacement` owns why: E0211 now cites ADR 0012 § 1's map
instead of restating a row of it, since every class in that table is still to be implemented and a
help that named one sent the reader out of the refusal and straight into E0405.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1029, six of
eight named cases written.

**`orient.py` still prints four dead `[context] modules` patterns** —
`crates/nvs-stdlib/src/capability.rs` (it is `crates/nvs-config/src/capability.rs`),
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs`. This session also needed `crates/nvs-types/src/{check,lower}.rs`,
`crates/nvs-hir/src/members.rs` and `crates/nvs-syntax/src/parser/mod.rs`, none of which
`[context] modules` names, and ADR 0012 § 1's mapping table — an `[context] adrs` gap.

## Next group

**Item 32, the rest — the refusals the checker still owes.** They share the expression checker's own
dispatch and its call path: `crates/nvs-types/src/expr/mod.rs`, `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-types/src/check.rs` and the `tests/conformance/reject/` cases named in
`docs/agent/loop-goal.toml`'s item-32 check.

- [ ] **P2 and P3** — an instance method called statically is E0458's user-class sibling (ADR 0008),
      and `$this` in a `static` method is refused rather than read; the `Core` half of the first is
      already `report_core_instance_member` and its doc says a user class belongs beside it
      (`crates/nvs-types/src/expr/members.rs:1117`, `crates/nvs-types/src/expr/calls.rs:210`,
      `crates/nvs-types/src/check.rs:467` where a body declares `$this`,
      `crates/nvs-types/src/check.rs:322` for the file scope that has none). Cases:
      `tests/conformance/reject/an-instance-method-is-not-called-statically.nvst` and
      `this-is-not-read-in-a-static-method.nvst`.
- [ ] **P7 and P8** — `throw` of a non-`Throwable` and `clone` of an array are both refused in the
      checker (ADR 0023 § 1 for `clone`); today each checks its operand and keeps whatever type it
      found (`crates/nvs-types/src/expr/mod.rs:687` for `Throw`,
      `crates/nvs-types/src/expr/mod.rs:591` for `Clone`). Cases:
      `tests/conformance/reject/throw-takes-a-throwable.nvst` and `clone-takes-an-object.nvst`.

## Backlog

- Items 33–35 of stage 0c — the modifiers, the lowering gaps and the docs (`docs/agent/loop-goal.md`).
- Stage 9's items 21–23: ADR 0119's expression `catch`, accepted and unimplemented.
- Stage 8's last two named cases (`docs/agent/loop-goal.toml`'s stage 8 check).
- ADR 0011 § 3 maps `PHP_EOL` to `Core\Env::EOL`; `docs/reference/tools/30-php-differences.md:31`
  says `"\n"` is the newline and there is no such class. One of the two is the bug — item 35's.
- Whether `Core\Uuid::version()` belongs in the spec's roster at all (M5, closed as *not a member*).
- The `[context]` manifest gaps named in `## State`, in `docs/agent/loop-goal.toml`.
