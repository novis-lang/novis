# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31 is closed. Item 32 has two findings left** — P9 and P11; P6, P13 and P14 closed this
session, and both of item 32's `catch` cases and its constant case are written.

**A constant now writes its type, which is a language change and not only a refusal.** `E0246` at
the `const` keyword, for a class as much as an interface; the inference `nvs_types::signatures::
ConstSig` did off the folded value is retired, and every doc example, `.nvst` case and unit test in
the tree is on the typed spelling. `docs/adr/README.md` § *Decisions taken at project start* is the
home of that decision, as it is of `E0245`'s.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet, so `brief.py`
still reports it as the next free `E01xx`. The next free `E02xx` is `E0247`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1040, six of
eight named cases written.

**`orient.py`'s `[context]` gaps, in the order they cost time.** There is no field naming
`docs/adr/README.md` § *Decisions taken at project start*, which is where three of this session's
rules live; `[context] modules` names neither `crates/nvs-syntax/src/parser/decl.rs` nor
`crates/nvs-types/src/consts.rs`, both of which every constant slice touches; and a refusal's two
tables — `docs/reference/tools/30-php-differences.md` and `docs/adr/divergences.md` — are named by
no field at all. Four dead `modules` patterns remain: `crates/nvs-stdlib/src/capability.rs` (it is
`crates/nvs-config/src/capability.rs`), `crates/nvs-host/src/budget.rs`,
`crates/nvs-types/src/calls.rs` and `crates/nvs-types/src/literals.rs`.

## Next group

**Item 32's last two, which are the same shape twice: a checker refusal where `nvs-ir` panics.**
Both land in `crates/nvs-types/src/expr/` and are pinned against a panic in
`crates/nvs-ir/src/lower/expr.rs`, so the file set is that pair plus `tests/conformance/reject/`.
`docs/reference/tools/30-php-differences.md` row 31 already states P9's rule.

- [ ] **P9** — `new $className()` and `$className::f()` with a `string` variable panic with "has no
      resolved class" (`crates/nvs-ir/src/lower/expr.rs:2313`) where the third spelling of the same
      mistake, `$x instanceof $className`, is already `E0496` in the checker
      (`crates/nvs-types/src/expr/members.rs:48` owns that code's reasoning). Extend it to the two
      call forms rather than inventing a code. Case:
      `tests/conformance/reject/new-takes-a-class-name-not-a-string.nvst`.
- [ ] **P11** — an enum case as an array key (`$m[E::A] = "a"`) reaches
      `crates/nvs-ir/src/lower/expr.rs:1976`'s "an array key lowered to …" panic; the two
      representations a key may take are that file's `crates/nvs-ir/src/lower/expr.rs:1913` doc
      comment, and the checker has to refuse anything else. Case:
      `tests/conformance/reject/an-enum-case-is-not-an-array-key.nvst`.
- [ ] **P7/P8** — `throw "x";` and `clone $a` on an array panic in the same lowerer for the same
      reason (`crates/nvs-ir/src/lower/expr.rs:2313` is their neighbour). Take these two only if the
      pair above leaves the session well short of the ceiling. Cases:
      `tests/conformance/reject/throw-takes-a-throwable.nvst`,
      `tests/conformance/reject/clone-takes-an-object.nvst`.

## Backlog

- Item 33–35 of stage 0c: `docs/agent/loop-goal.md` § *Stage 0c*.
- P5 — an `array<T>`-typed class constant still panics at use; `docs/reference/findings.md`.
- P15 — a memory-limit breach inside `try`/`finally` aborts with a Rust panic; same file.
- Stage 9's items 21–23, ADR 0119's expression `catch`; `docs/agent/loop-goal.md`.
- Stage 8's two unwritten conformance cases and the differential floor; `docs/plan/m6.md`.
- `[context]` in `docs/agent/loop-goal.toml`: the fields named in `## State` above.
