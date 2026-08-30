# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open at D27**: P1, P4 and D23 are ticked, so the
driver's acceptance run now fails on the item-34 `[[check]]`'s fourth case,
`tests/conformance/core/a-shebang-line-opens-code.nvst`. The eleven cases after
`a-named-closure-recurses` in that block are all still unwritten.

**ADR 0031 § 3's self-name resolves, and is not a binding.**
`nvs_types::expr::calls::check_fn_literal`'s doc comment is that decision's only home: why a local
would have been all three of the things § 3 says the name is not, why it reaches one body and not a
closure written inside it, and why a self-call answers with the literal's *declared* return type
where `$f(...)` answers `mixed`. Three ends read it — `nvs_hir::members`' `Env::fn_self` (which bare
call is not `E0320`), `ExprInfo::ClosureSelf`, and `nvs_ir::lower::expr`'s guarded `ConstFetch` arm,
which resolves to the invoke's own receiver under `FN_SELF` and so adds no field, no slot and no
capture. Pinned by `tests/conformance/core/a-named-closure-recurses.nvst`.

**One consequence worth knowing before writing a case: `mixed` does not satisfy a declared `int`
(`E0401`).** So a call through a `callable` *variable* cannot be returned or bound where a concrete
type is declared, self-name or not — `nvs_types::expr`'s `ExprKind::Call` arm is why, and it is what
makes the declared-return-type rule above load-bearing rather than a convenience.

**Two refusals the checker owes, deliberately left open.** `check_every_path_returns`
(`crates/nvs-types/src/check.rs:701`) exempts `never` beside `void`, so a `never` body that falls
off its end compiles and comes back; and `nvs_types::returns` walks statements syntactically, so a
*call* to a `never` member does not count as leaving the frame the way a `throw` does. Both in
`## Backlog`; `nvs_ir::lower::erase_checked_ty`'s erasure is what makes the first harmless.

**One divergence stated rather than hidden: `static::method(...)` binds the declaring class.**
`lower_callable`'s doc comment is that fact's home; the redesign is in `## Backlog`.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247`, and `E07xx` is `E0794` — this session added no diagnostic.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1060,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs: add **0031 § 3** (the one this session needed and did not get), **0027 § 1**, **0033 §§ 3-4**,
**0047 § 2**, **0011**, **0086 § 6**, **0096 §§ 1-1a** and **0007 § 3**; **0100** is the next one, for
D27. `[context] modules` still misses `crates/nvs-hir/src/members.rs`,
`crates/nvs-types/src/attributes.rs`, `routes.rs`, `commands.rs`, `derive.rs`, `testing.rs`,
`consts.rs`, `retrieval.rs`, `generics.rs`, `crates/nvs-stdlib/src/serialize.rs` and `secret.rs`,
`crates/nvs-types/src/expr/args.rs`, `check.rs`, `returns.rs`, and `crates/nvs-ir/src/lower/closure.rs`
and `call.rs`. `orient.py` itself still warns that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 34's next three findings. D27 leads because it is the case the acceptance check names; D33 and
D22 are both refusals written in `crates/nvs-types/src/expr/args.rs` and share it, so take them
together after it.**

- [ ] **D27 a shebang line opens code** — findings.md § *Divergences* D27, ADR 0100. A file whose
      first line is `#!` starts in HTML mode and copies that line to stdout: the lexer opens every
      file in `Mode::Html` at `crates/nvs-syntax/src/lexer.rs:90`, with no look at the first two
      bytes. Decide there whether the shebang is consumed as a token or skipped before lexing, and
      say which in `Lexer::new`'s doc comment. Case:
      `tests/conformance/core/a-shebang-line-opens-code.nvst`, named by the item-34 `[[check]]` at
      `docs/agent/loop-goal.toml:331`.
- [ ] **D33 a literal adapts to a generic `uint`** — findings.md § *Divergences* D33.
      `assertSame($uintValue, 2)` is `E0401: expected uint, found int`: the literal is checked
      against the *bound* `T` rather than adapting to it. `crates/nvs-types/src/expr/args.rs:789`'s
      doc comment already names `crate::generics` as owning why, and
      `crates/nvs-types/src/expr/literals.rs:241` (`infer_int_literal`) is the adaptation itself.
      Case: `tests/conformance/core/a-literal-adapts-to-a-generic-uint.nvst`
      (`docs/agent/loop-goal.toml:332`).
- [ ] **D22 `inout` takes only a local** — findings.md § *Divergences* D22.
      `M::bump(inout $a["k"])` is `E0439` at `crates/nvs-types/src/expr/args.rs:712`, whose help
      cites ADR 0007 § 5: a copy-on-write element has no stable address. Either stage-and-copy-back
      at the call site (`nvs_ir::lower::call`'s `staged_targets`, `crates/nvs-ir/src/lower/mod.rs:2501`)
      or make the refusal settled and drop the "yet" — a real design call, and the write-back's
      visibility to a callee that reads the same array is the question that decides it.

## Backlog

- `check_every_path_returns` exempts `never`, so a `never` body may fall off its end — `crates/nvs-types/src/check.rs:701`.
- `nvs_types::returns` does not count a call to a `never` member as leaving the frame — `crates/nvs-types/src/returns.rs`.
- A nested closure's *non*-visibility of an enclosing self-name has no reject case — `tests/conformance/reject/`.
- `static::method(...)` binds the declaring class rather than the called one — `nvs_ir::lower::closure`'s `lower_callable`.
- Stage 9's expression `catch` — ADR 0119, items 21–23, `E0126` reserved for its `return` refusal.
- Item 35's doc findings — `docs/reference/findings.md` § *Triage*, last-but-one row.
