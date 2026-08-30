# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
Items 31 and 32 are closed; **item 33 is open with its first two findings landed** — U1 (`readonly`)
and U2 (`final`), both green at their `[[check]]` case names.

**A `readonly` property is written by its declaring class's `constructor` and nowhere else** —
`E0782`, reported from `crates/nvs-types/src/expr/assign.rs`'s `reject_readonly_write`, which every
one of the four write spellings reaches through `check_write_target`. `nvs_types::Ctx::in_constructor`
is the flag it asks, and that field's own doc comment is the home of why a closure written inside the
constructor does not inherit it.

**`final` is enforced from the declaration side** — `E0783` for an `extends` naming a `final` class,
`E0784` for a redeclared `final` method, both in `crates/nvs-types/src/conformance.rs`'s
`check_class_finality`, whose doc comment owns why the class half asks the direct superclass only.
Both modifiers are recorded per declaration on `ClassSignature` (`readonly_properties`, `is_final`,
`final_methods`) rather than on `MethodSig`, for the reason `final_methods`' own doc gives.

**A seeded class carries no modifier, so U13 is untouched by this.** `Throwable`'s four "readonly"
properties come from `nvs_types::error_lib`, and only user source writes a modifier at all — the
write to `$e->message` stays accepted until that finding's own slice.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet, so `brief.py` still
reports it as the next free `E01xx`. The next free `E02xx` is `E0247`, the next `E07xx` is `E0785`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1044, six of eight
named cases written.

**`orient.py`'s `[context]` gaps, unchanged and still costing time.** No field names
`docs/adr/README.md` § *Decisions taken at project start*; `[context] modules` names neither
`crates/nvs-syntax/src/parser/decl.rs` nor `crates/nvs-types/src/consts.rs`; a refusal's two tables —
`docs/reference/tools/30-php-differences.md` and its copy in `docs/novis.md` — are named by no field
at all, and both had to be found by grep again this session. Four dead `modules` patterns remain:
`crates/nvs-stdlib/src/capability.rs` (it is `crates/nvs-config/src/capability.rs`),
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs`.

## Next group

**Item 33's next two — a modifier and a hook, both refused at a site this session already opened.**
They share `crates/nvs-types/`'s `conformance.rs` and `expr/` (`assign.rs`, `calls.rs`), plus
`tests/conformance/reject/`, and each has its case name in `loop-goal.toml`'s item-33 `[[check]]`.

- [ ] **U3 `abstract`** — `new` on an abstract class is refused at `crates/nvs-types/src/expr/calls.rs:417`
      (`infer_new`, which already resolves the class and its constructor), and a bodiless method in a
      non-`abstract` class is refused beside `crates/nvs-types/src/conformance.rs:120` — that walk
      already reads `has_body` and `decl.modifiers`. Case:
      `tests/conformance/reject/abstract-is-not-instantiated.nvst`.
- [ ] **U12 get-only hook** — a write to a hooked property with no `set` is silently unobservable
      today. `crates/nvs-types/src/expr/assign.rs:486` is the place: `ExprInfo::HookedProperty`
      already arrives there carrying `set: Option<label>`, so the refusal is the `None` arm beside
      `crates/nvs-types/src/expr/assign.rs:592`'s readonly one. Case:
      `tests/conformance/reject/a-get-only-hook-is-not-written.nvst`.
- [ ] **U14 `#[Access]` with no `#[Route]`** — a stray marker attribute compiles. Case:
      `tests/conformance/reject/a-stray-access-attribute-is-refused.nvst`; the roster is
      `crates/nvs-types/src/derive.rs:77` and the walk is
      `crates/nvs-types/src/attributes.rs`'s `check_declaration`.

## Backlog

- U4–U6, P5: the rest of item 33's modifiers and attributes — `docs/reference/findings.md` § *Triage*.
- U13: `Throwable`'s four properties are not read-only — needs a modifier on a seeded class.
- Item 34, the lowering and library gaps — `docs/agent/loop-goal.toml`'s item-34 `[[check]]`.
- Stage 8: conformance 1044/1050, differential 206/210 — two named cases unwritten.
- Stage 9: ADR 0119's expression `catch`, items 21–23, resumes when 0c is green.
- `docs/agent/loop-goal.toml`'s `[context]` gaps named in *State*.
