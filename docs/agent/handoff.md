# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open**: P1, P4, D23, D27 and now D33 and D22 are
ticked, so the item-34 `[[check]]`'s first unwritten case is its sixth,
`tests/conformance/error/iterator-current-outside-the-protocol-throws.nvst`, with eight after it.

**A literal is *placed* against the parameter type substitution produced, not compared to it.**
`nvs_types::expr::args::check_generic_args`'s doc comment is that decision's only home (findings.md
D33). Three parts: a literal at a position whose declared type still mentions a variable is checked
with no expectation and then checked *again*, in the third pass, against the substituted type; it
binds a variable only in a second binding round, where `or_insert` leaves it a fallback for a
variable nothing written bound; and the one thing an open position can still say to a literal —
`uint`, for a digit run above `i64::MAX` — is `literals::unplaced_expectation`. The second check
runs only where the first reported nothing about that literal, which is what keeps "nothing is
diagnosed twice" true. `literals::is_unplaced_literal` is the roster, and owns why an array literal
is on it and an object literal cannot be.

**An array element is not an `inout` place, permanently** — `yet` is out of `E0439`'s message, and
`check_inout_arg`'s doc comment says why the call-site copy that would fake a reference is not
offered: it is a reference only for as long as the callee does not reach the same array.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1062,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs: add **0053 § 1** (what U21 needs next), **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**,
**0047 § 2**, **0011**, **0086 § 6** and **0096 §§ 1-1a**. `[context] modules` still misses
`crates/nvs-ir/src/lower/generator.rs`, `crates/nvs-hir/src/members.rs`,
`crates/nvs-types/src/attributes.rs`, `routes.rs`, `commands.rs`, `derive.rs`, `testing.rs`,
`consts.rs`, `retrieval.rs`, `generics.rs`, `crates/nvs-stdlib/src/serialize.rs` and `secret.rs`,
`crates/nvs-types/src/expr/args.rs`, `check.rs`, `returns.rs`, and
`crates/nvs-ir/src/lower/closure.rs` and `call.rs`. `orient.py` itself still warns that
`crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 34's next three findings, in the order the item-34 `[[check]]` names their cases. U21 leads
because it is the case the acceptance run will report; it is alone in `crates/nvs-ir`, and D16 and
D17 share the `crates/nvs-stdlib` registry rows, so take those two together.**

- [ ] **U21 `Iterator::current()` outside the protocol** — findings.md § *Undocumented* U21. On a
      generator it answers `0` before the first `advance()` and the last value after exhaustion,
      where ADR 0053 § 1 says it throws. The synthesized member is
      `crates/nvs-ir/src/lower/generator.rs:300`'s `GEN_CURRENT_METHOD`; the protocol's own row is
      `crates/nvs-types/src/iter_lib.rs:124`. Decide which § 10 class it throws and say so in
      `generator.rs`'s module doc. Case:
      `tests/conformance/error/iterator-current-outside-the-protocol-throws.nvst`, named at
      `docs/agent/loop-goal.toml:333`.
- [ ] **D16 `Core\Arr::from` refuses the `Core` collections** — findings.md § *Divergences* D16.
      `from($objectSet)` is `E0401 expected array<T>|Iterable<T>|Iterator<T>`, and on `ObjectMap`
      the message leaks an unsubstituted `array<K>`, although `foreach` over both works. The row is
      `crates/nvs-stdlib/src/arr.rs:518`. Case:
      `tests/conformance/core/arr-from-takes-a-core-collection.nvst`, at
      `docs/agent/loop-goal.toml:334`.
- [ ] **D17 `Core\ObjectSet::union` loses its element type** — findings.md § *Divergences* D17. The
      three set members return a bare `Core\ObjectSet`, so `foreach … as Tag $t` is "expected Tag,
      found T". The rows are `crates/nvs-stdlib/src/objset.rs:98`, `:107` and `:116`. Case:
      `tests/conformance/core/objectset-union-keeps-its-element-type.nvst`, at
      `docs/agent/loop-goal.toml:335`.

## Backlog

- Item 34's remaining findings after these three: D1, D7, D8, D10, D12, D21, D35, M1 —
  `docs/reference/findings.md` § *Triage*.
- Item 35, and stage 9's items 21–23 (ADR 0119), both untouched — `docs/agent/loop-goal.md`.
- A `[1, 2]` beside a generic `array<uint>` costs a second walk of the literal's elements —
  `nvs_types::expr::literals::is_unplaced_literal` says what that buys.
