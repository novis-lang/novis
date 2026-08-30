# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open at D33**: P1, P4, D23 and now D27 are ticked, so
the driver's acceptance run fails on the item-34 `[[check]]`'s fifth case,
`tests/conformance/core/a-literal-adapts-to-a-generic-uint.nvst`. The ten cases after it in that block
are all still unwritten.

**A `#!` first line opens code mode, and it is lexed rather than skipped.**
`nvs_syntax::lexer`'s `Lexer::new` doc comment is that decision's only home: the outer mode starts
`Mode::Code` with `pos` still at 0, so code mode's own trivia rule consumes line 1 as the `#` comment
it already is — no token, no output, and bidi-checked (ADR 0087 § 2) at the one place in a file where
an unbalanced override reorders everything after it. `shebang_open` is the window in which an `<?nvs`
is `E0009` instead of a tag; the first `?>` closes it and the file is an ordinary template again.
Pinned by `tests/conformance/core/a-shebang-line-opens-code.nvst` and three lexer unit tests.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic, it reported the `E0009`
ADR 0100 had already reserved.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1061,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs: add **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**,
**0096 §§ 1-1a** and **0007 § 3** — that last one is what D33 needs next, for the generic parameter a
literal has to adapt to. (**0100 § 3** was this session's and is no longer needed.) `[context] modules`
still misses `crates/nvs-hir/src/members.rs`, `crates/nvs-types/src/attributes.rs`, `routes.rs`,
`commands.rs`, `derive.rs`, `testing.rs`, `consts.rs`, `retrieval.rs`, `generics.rs`,
`crates/nvs-stdlib/src/serialize.rs` and `secret.rs`, `crates/nvs-types/src/expr/args.rs`, `check.rs`,
`returns.rs`, and `crates/nvs-ir/src/lower/closure.rs` and `call.rs`. `orient.py` itself still warns
that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 34's next two findings, both written in `crates/nvs-types/src/expr/args.rs` — the file set is
that one file plus `crates/nvs-types/src/expr/literals.rs` and `crates/nvs-types/src/generics.rs`.
D33 leads because it is the case the acceptance check names.**

- [ ] **D33 a literal adapts to a generic `uint`** — findings.md § *Divergences* D33. `assertSame($u, 2)`
      against a `T`-typed parameter is `E0401: expected uint, found int`: the binding pass runs before
      the argument check, and a literal argument is excluded from it, so `2` is checked against the
      *unsubstituted* declared type. `crates/nvs-types/src/expr/args.rs:783` is the doc comment that
      already states that order and why; `crates/nvs-types/src/expr/args.rs:826` is the `Bindings` loop
      that skips the literal, and `crates/nvs-types/src/expr/literals.rs:74` is how a literal takes its
      type from the position it lands in. Decide there whether a literal is re-checked against the
      substituted signature or admitted into the binding pass, and say which in that doc comment. Case:
      `tests/conformance/core/a-literal-adapts-to-a-generic-uint.nvst`, named by the item-34 `[[check]]`
      at `docs/agent/loop-goal.toml:332`.
- [ ] **D22 `inout` takes only a local** — findings.md § *Divergences* D22. `M::bump(inout $a["k"])` is
      `E0439` "cannot be passed to an `inout` parameter **yet**" — the word `yet` is the whole finding,
      because a refusal that promises a later version is either a gap to close or a sentence to rewrite.
      The three refusal sites are `crates/nvs-types/src/expr/args.rs:698`, `:715` and `:728`, with the
      exactness rule beside them at `:749`. Decide whether an index or a property is a place an `inout`
      accepts, or whether the refusal is permanent and loses the `yet`.

## Backlog

- `check_every_path_returns` exempts `never` beside `void`, so a `never` body that falls off its end
  compiles — `crates/nvs-types/src/check.rs:701`; `nvs_ir::lower::erase_checked_ty` is why it is
  harmless.
- `nvs_types::returns` walks statements syntactically, so a *call* to a `never` member does not count
  as leaving the frame the way a `throw` does — same crate, `returns.rs`.
- `static::method(...)` binds the declaring class rather than the late-static one;
  `nvs_ir::lower::lower_callable`'s doc comment owns the divergence, the redesign is unscheduled.
- D28: the on-disk compile cache is unwired — `nvs run` compiles fresh every time
  (findings.md § *Divergences*).
- Stage 9: ADR 0119's expression `catch`, items 21–23, nothing implemented.
- The `[context]` manifest gaps above, in `docs/agent/loop-goal.toml`.
