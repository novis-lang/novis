# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
Items 31 and 32 are closed; **item 33 is open with four of its findings landed** — U1 (`readonly`),
U2 (`final`), U3 (`abstract`) and U12 (the `get`-only hook), each green at its `[[check]]` case name.

**`abstract` is enforced from both sides.** `E0785` refuses `new` on a class with no instances —
`crates/nvs-types/src/expr/calls.rs`'s `reject_abstract_instantiation`, whose whole question is
`nvs_hir::ClassLinks::concrete`, so an interface is refused by the same lookup and named as one.
`new static()` is exempt and the case pins it. `E0786` refuses a bodiless method in a class that is
not `abstract`, from `conformance.rs`'s `check_abstract_members`; the question there is the missing
*body*, not the modifier, for the reason that function's doc gives.

**A `get`-only hooked property is read-only from outside its declaring class** — `E0787`, from
`expr/assign.rs`'s `reject_get_only_hook_write`, reading the `set: None` the
`ExprInfo::HookedProperty` entry already carries. The rule is **scope-shaped, not
backedness-shaped**: Novis keeps a slot for every hooked property, so the declaring class's own
`$this->p = v` is how such a property holds anything, while an outside write would store where the
`get` may never read. That doc comment owns the reasoning; `signatures::PropertyHooks` no longer
claims the old acceptance, and `docs/reference/tools/30-php-differences.md` carries the divergence
row (PHP lets an outside write through to a *backed* one).

**`docs/novis.md` is generated, never edited** — `python tools/reference.py --no-examples` rebuilds
it from the reference pages in under a second. The previous handoff's complaint that a refusal has
"two tables" was wrong: edit `docs/reference/**` and regenerate.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247`, and `E07xx` is now `E0788`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1046,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] modules` names none of the
nvs-types files this session actually edited or read: `crates/nvs-types/src/expr/assign.rs`,
`expr/quals.rs`, `expr_table.rs`, `attributes.rs`, `routes.rs`, `commands.rs` — one
`crates/nvs-types/src/expr/*.rs` pattern plus those four would cover the whole of item 33's
remaining work. `[context] adrs` should gain **0014 § 1** (property hooks), which had to be sliced
by hand to decide U12. Four dead `modules` patterns remain: `crates/nvs-stdlib/src/capability.rs`
(it is `crates/nvs-config/src/capability.rs`), `crates/nvs-host/src/budget.rs`,
`crates/nvs-types/src/calls.rs` and `crates/nvs-types/src/literals.rs`.

## Next group

**Item 33's attribute half — two refusals over the declaration passes, sharing
`crates/nvs-types/`'s `attributes.rs`, `routes.rs` and `commands.rs` plus
`tests/conformance/reject/`.** Each has its case name in `loop-goal.toml`'s item-33 `[[check]]`.

- [ ] **U14 `#[Access]` with no `#[Route]`** — a stray marker attribute compiles, unlike
      `#[Query]`/`#[Option]`/`#[Api]`, which already have the refusal to copy. The roster check is
      `crates/nvs-types/src/attributes.rs:158`; which methods carry a `#[Route]` is what
      `crates/nvs-types/src/routes.rs:501`'s `check_class_routes` already walks, so the question is
      answered in the pass that holds both. Case:
      `tests/conformance/reject/a-stray-access-attribute-is-refused.nvst`.
- [ ] **A `#[Command]` method is `static` and returns `void` or `uint`** — the shape check belongs
      beside `crates/nvs-types/src/commands.rs:199`'s `check_class_commands`, which already resolves
      the attribute and the method it sits on. Case:
      `tests/conformance/reject/a-command-is-static-and-returns-void-or-uint.nvst`.
- [ ] **U4 `secret` reaches `echo`, interpolation and `Core\Json::encode`** — a second file set
      (`crates/nvs-types/src/expr/quals.rs:419` is the `Core\Debug` refusal to model it on, and ADR
      0033 § 4 lists output among the sinks). Take it only with a fresh context. Cases:
      `tests/conformance/reject/echo-refuses-a-secret.nvst`,
      `tests/conformance/reject/json-encode-refuses-a-secret.nvst`.

## Backlog

- ADR 0014 § 1 defers virtual-vs-backed properties to `docs/spec/`, which is unwritten; `E0787`'s
  scope-shaped rule stands in its place and `reject_get_only_hook_write`'s doc is its only home.
- `docs/reference/lang/50-classes.md` § *`abstract`* claims a concrete subclass "must declare every
  one of them, or it is a compile error" — unverified against the tree, where `E0449` covers
  interface obligations only. If it is false it is a finding, not a doc edit.
- Item 33's remainder after the group above: U5, U6, P5, and
  `tests/conformance/core/an-array-constant-folds.nvst`.
- Item 34 (lowering and library gaps) and item 35 (docs) are untouched — `findings.md` § *Triage*.
- Stage 8 needs conformance 1050 and differential 210; two of its eight named cases are unwritten.
