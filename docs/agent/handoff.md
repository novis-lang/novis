# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed.** Item 34 is next and its `[[check]]` is the one the driver's
acceptance run still fails on, naming `tests/conformance/core/a-first-class-callable-lowers.nvst`.

**`Core\Secret` exists.** `crates/nvs-stdlib/src/secret.rs` holds `reveal` and `revealBytes` — two
names rather than ADR 0033 § 3's "overload" because the registry has one row per name, and both
alternatives are worse in ways that module's doc states. They are the only rows that write
`Qual::Reveal`, which is the mark that **admits** a `secret` argument; the answer drops the
qualifier by not declaring it, so nothing removes `secret` at a call site outside this class.
`tainted` still crosses — `carries_contagion` counts `Reveal` alongside `Contagious` — so a
`secret tainted string` reveals to a `tainted string` and the escape hatch launders one axis only.
`tests/conformance/reject/reveal-removes-secret-and-not-tainted.nvst` is that half, pinned as a
refusal because the leak-shaped failure is the one that compiles.

**This was goal 4's item 13, taken now on purpose.** Eight of ADR 0033 § 4's refusals already name
`Core\Secret::reveal(..., "reason")` in their help text (`crates/nvs-types/src/expr/quals.rs`), so
the goal-3 sinks were shipping a dead end. `docs/agent/loop-goal.md`'s item 33 no longer says U5 is
answered by rewording those texts.

**The checker side is two functions and one arm.** `quals::admits_secret_argument` is the `secret`
axis's twin of `admits_tainted_argument` and is asked separately, because the axes are independent
bits; `quals::unsecret` is `untainted`'s twin; `args::Admitted` carries both answers into
`check_arg_admitting_quals`, which narrows the *comparison* only and hands back the argument's own
inferred type.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247`, and `E07xx` is `E0793`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1056,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's
own ADRs: this session needed **0033 § 3** and the pack printed 0103, 0078, 0042 and 0119 instead —
add it, plus **0047 § 2**, **0011**, **0033 § 4**, **0086 § 6** and **0096 §§ 1-1a** from the three
sessions before. `[context] modules` still misses `crates/nvs-types/src/attributes.rs`, `routes.rs`,
`commands.rs`, `derive.rs`, `testing.rs`, `defaults.rs`, `consts.rs`, `signatures.rs`,
`retrieval.rs` and — new this session — `crates/nvs-stdlib/src/registry.rs`'s neighbours
`serialize.rs` and `secret.rs`, and `crates/nvs-types/src/expr/args.rs`. `orient.py` itself still
warns that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 34's lowering gaps, which is what the driver's acceptance check is failing on. They share
`crates/nvs-ir/src/lower/expr.rs` and its `Lowering::lower_expr` dispatch — take P1 first, since
its case is the one named in the failing `[[check]]`.**

- [ ] **P1 a first-class callable lowers** — findings.md § *Panics* P1, item 34. The checker
      records `ExprInfo::CallableRef` at `crates/nvs-types/src/expr/calls.rs:317` and the lowering
      has no arm for it (`crates/nvs-ir/src/lower/expr.rs:2874`); ADR 0027 keeps the spelling.
      Case: `tests/conformance/core/a-first-class-callable-lowers.nvst`, already named by the
      item-34 `[[check]]` in `docs/agent/loop-goal.toml:324`.
- [ ] **P4 a `: never` method is a terminator** — findings.md § *Panics* P4, item 34. The
      `KNOWN_ICE` row at `crates/nvs-ir/tests/type_atoms.rs:123` goes with it, and the terminator
      is decided in the same `lower_expr` dispatch (`crates/nvs-ir/src/lower/expr.rs:2874`).
      Case: `tests/conformance/core/a-never-method-is-a-terminator.nvst`.
- [ ] **D23 a named closure recurses** — findings.md § *Divergences* D23, ADR 0031 § 3: a named
      closure's recursive call resolves as a free function at
      `crates/nvs-types/src/expr/calls.rs:1092`. Same file as P1's anchor, different end.
      Case: `tests/conformance/core/a-named-closure-recurses.nvst`.

## Backlog
- ADR 0046 § 2's named constant forms — another class's `const`, an enum case, `Foo::class` — still
  fold to nothing; `crates/nvs-types/src/defaults.rs`'s `const_reference_default` resolves exactly
  those at a property default and needs the `Ctx` the constant-collection pass holds.
- `Core\Secret` has no `docs/spec/01-core-library.md` § of its own — registered under the
  "no spec § " convention `CLASSES` already uses, with ADR 0033 § 3 as its home.
- Item 35 of stage 0c, once item 34 is closed — `docs/agent/loop-goal.md` § *Stage 0c*.
- Stage 9's items 21–23 (ADR 0119's expression `catch`), anchors already written.
- Stage 8's two remaining differential cases — `docs/agent/loop-goal.md` § *Stage 8*.
