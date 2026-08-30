# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
Items 31 and 32 are closed; **item 33 has six of its nine findings landed** — U1 (`readonly`),
U2 (`final`), U3 (`abstract`), U6 (the `#[Command]` shape), U12 (the `get`-only hook) and U14 (the
stray `#[Access]`), each green at its `[[check]]` case name. **U4, U5 and P5 are what is left.**

**A recognized attribute is refused where nothing reads it, and all four such refusals are asked
from one walk** — `crates/nvs-types/src/attributes.rs`'s per-method walk, because each owning pass
selects the methods its own attribute marks and so cannot see a marker on any other method.
`E0788` joins `E0765`/`E0766`/`E0771` there: an `#[Access]` with no `#[Route]` guards nothing, and
it is a refusal rather than a silence because ADR 0096 § 2 keeps the compiler from interpreting
`allow` — the route table is the decision's only reader. `routes.rs`'s `check_stray_access` owns
that reasoning.

**A `#[Command]` method is `static` and returns `void` or `uint`** — `E0789`, from `commands.rs`'s
`check_command_shape`, read off the resolved signature and worded from what the declaration did, as
the `#[Test]` and `#[Fixture]` shape refusals are. A method writing **no** return type is left
alone: ADR 0007 § 3 already refuses that, and a second error would name the `mixed` the author never
wrote. Both halves are ADR 0086 § 6 — the `static` one is what its dispatch requires, since the row
carries a `Class::method` string and no constructor arguments anywhere in it.

**`docs/novis.md` is generated, never edited** — `python tools/reference.py --no-examples` rebuilds
it from the reference pages in under a second, and a landed refusal owes an edit to the page that
states the rule (`docs/reference/lang/90-attributes.md` for both of this session's).

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247`, and `E07xx` is now `E0790`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1048,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] modules` names none of the
nvs-types files item 33's attribute half lives in: add one `crates/nvs-types/src/expr/*.rs` pattern
plus `attributes.rs`, `routes.rs`, `commands.rs`, `derive.rs` and `testing.rs`. `[context] adrs`
should gain **0086 § 6** and **0096 §§ 1-1a**, both sliced by hand this session, and **0014 § 1**
(property hooks) from the one before. Four dead `modules` patterns remain:
`crates/nvs-stdlib/src/capability.rs` (it is `crates/nvs-config/src/capability.rs`),
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs`.

## Next group

**Item 33's last three findings. The first two share `crates/nvs-types/src/expr/quals.rs` and
`tests/conformance/reject/`; the third is a second file set, in `nvs-ir`.** Each has its case name
in `loop-goal.toml`'s item-33 `[[check]]`.

- [ ] **U4a `secret` reaches `echo` and interpolation** — ADR 0033 § 4 lists output among the sinks.
      The refusal to model it on is `crates/nvs-types/src/expr/quals.rs:420`'s `Core\Debug` argument,
      and the per-body walk that holds an `echo`'s operands is
      `crates/nvs-types/src/locals.rs:1190`. Case:
      `tests/conformance/reject/echo-refuses-a-secret.nvst`.
- [ ] **U4b `Core\Json::encode` refuses a `secret`** — the same qualifier at a `Core` sink, which is
      `crates/nvs-types/src/expr/quals.rs:506`'s `E_SECRET_CROSSES_A_BOUNDARY` and the roster it
      reads. Case: `tests/conformance/reject/json-encode-refuses-a-secret.nvst`.
- [ ] **P5 an `array<T>`-typed class constant folds instead of panicking** — second file set:
      `crates/nvs-ir/src/lower/expr.rs:286` is the "no value recorded" panic, and ADR 0057 is how
      the fold is specified. Case: `tests/conformance/core/an-array-constant-folds.nvst`.

## Backlog

- **U5** `Core\Secret::reveal` does not exist yet two help texts name it — `docs/reference/findings.md`.
- Item 34 (lowering and library gaps) and item 35 (docs in the tree) — `docs/agent/loop-goal.md`.
- Stage 9's ADR 0119 lowering, items 21–23 — `docs/agent/loop-goal.md`.
- Stage 8 owes two more differential cases against its 210 floor — `docs/plan/m6.md`.
- ADR 0014 § 1 defers virtual-vs-backed properties to `docs/spec/`, which is unwritten — `E0787`'s
  scope rule stands in `expr/assign.rs`'s doc comment meanwhile.
