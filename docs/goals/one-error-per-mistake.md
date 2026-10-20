---
looper: 1
title: One mistake in a program reports one error
summary: "A name that does not resolve gives its one root diagnostic and no later use of it reports again, a value written as mixed keeps every error it has, the rule says so with its reasons, and the checks below pass."
context: [crates/nvs-types/src/ty.rs, crates/nvs-types/src/lower.rs, docs/rules/types/unions-and-mixed.md]
stages:
  - id: 1
    title: "the error type exists, and every recovery site returns it"
    status: open
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/expr/mod.rs]
    extra:
      context:
        rules: [types/unions-and-mixed, types/one-mistake-reports-once]
  - id: 2
    title: "no report site reports the error type"
    status: open
    context: [crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/check.rs, crates/nvs-types/src/defaults.rs, crates/nvs-types/src/signatures.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-types/src/program.rs, crates/nvs-types/src/expr/members.rs]
    extra:
      context:
        rules: [types/one-mistake-reports-once]
  - id: 3
    title: "an unresolved `use` reports once, and the cases pin every root"
    status: open
    context: [crates/nvs-hir/src/resolve.rs, crates/nvs-hir/src/hierarchy.rs, tests/conformance/lang/a-class-name-constant-needs-a-class-the-compiler-resolves.nvst]
    extra:
      context:
        rules: [types/one-mistake-reports-once, statements/no-fallback-to-the-root-namespace]
checks:
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, one-error-per-mistake]
    reach: true
    floor: false
---

# One mistake in a program reports one error

## Why

A name that does not resolve (`E0303`, `E0306`) is reported once, and then the checker carries on with
a plain `mixed` in its place. Every later use of that value reports again: `E0401` "expected `string`,
found `mixed`", `E0403` "returns `mixed`", `E0743` "`Core\Program::implementing` ... `mixed` is
neither". One missing name gives many errors, and the first five an editor hook shows are mostly
noise that hides the one line to fix.

`enum Ty` (`crates/nvs-types/src/ty.rs:29`) has no error variant, so nothing downstream can tell a
recovery `mixed` from one the program wrote. The guard the checker has compares
`diags.error_count()` before and after one expression (`crates/nvs-types/src/expr/mod.rs:158`,
`crates/nvs-types/src/expr/assign.rs:874`, `crates/nvs-types/src/lower.rs:255`,
`crates/nvs-types/src/lower.rs:304`, `crates/nvs-types/src/lower.rs:351`,
`crates/nvs-types/src/expr/args.rs:917`, `crates/nvs-types/src/expr/literals.rs:1377`), so it works
inside one expression and nowhere past it. `nvs-hir` sets `Import::resolved` to `false` on `E0306`
(`crates/nvs-hir/src/resolve.rs:31`, `crates/nvs-hir/src/resolve.rs:278`), and the type checker never
reads it.

## Standing decisions

**A distinct error type replaces the recovery `mixed`.** It is either a `Ty::Error` variant or a
separate interned id that is never equal to `env.interner.mixed()`. Stage 1's first slice picks one,
and records why in `ty.rs`'s module doc and in `types/one-mistake-reports-once`. Safe default:
`Ty::Error`, because every exhaustive `match` over `Ty` then names a site that has to decide, and the
compiler finds each one. Whichever is picked:

- It prints as `mixed`, in a hover and anywhere else a type is shown.
- It is assignable to and from every type.
- A member read, a method call, a subscript and an operator on it report nothing, and give the error
  type again.
- Every recovery site returns it: `resolve_name_type` (`crates/nvs-types/src/lower.rs:1116`),
  `resolve_special` (`crates/nvs-types/src/lower.rs:869`), `resolve_parent`
  (`crates/nvs-types/src/lower.rs:888`), `check_new_target`
  (`crates/nvs-types/src/expr/calls.rs:2184`) and the two in `crates/nvs-types/src/expr/members.rs:289`.
  The line numbers drift while `compiler-holes` runs, so a session finds each by its function name.

**A value the program wrote as `mixed` keeps every error it has today.** Only the error type is
quiet. A conformance case pins it: a `mixed` parameter used as a `string` still reports `E0401`.

**The skip lives in one function.** Every `E0401` goes through `mismatch()`
(`crates/nvs-types/src/expr/assign.rs:659`), so the hand-built ones are folded into it, and
`mismatch()` returns without reporting when either side is the error type. `E0403` and `E0743` each
have one site and get the same test there. Why: simplicity. A later report site that calls the
helper is quiet for free, and a contributor has nothing to remember.

**The error-count guards stay.** They still stop a second report inside one expression. One that the
error type makes redundant may be removed in the slice that proves it with a case.

**The target for an unresolved `use` is the root error alone.** `E0306` at the `use` line, and no
`E0303` at any of the places the short name is written. The checker reads `Import::resolved == false`
and gives the error type with no report. If a session finds that a use site must still report, one
`E0303` per written use is allowed only with its reason written into the rule. That is the fallback,
never the aim.

**The error type never reaches lowering.** A program with an error diagnostic does not compile, so
`nvs-ir` never sees it. Lowering treats it as unreachable, with a debug assertion that names this rule.

**The rule is `types/one-mistake-reports-once`, a new fragment.** It states the policy and its
reasons, and its Declined list carries:

- Suppressing by error count only. It cannot see past one expression, and it would hide a second,
  independent mistake.
- Hiding `E0306`. The `use` line is where the fix is written, so its error is the one to keep.
- Stopping after the first error. Two independent mistakes in one program should give two errors.

**Gate.** The goal starts when `compiler-holes` is reached, because both change `nvs-types`. The proof
is that `docs/goals/compiler-holes.md` no longer exists. It sits right behind it on the chain, so the
gate holds when this goal starts.

**Lanes.** One lane. All three stages change `nvs-types` and run in order. Stage 3 reads `nvs-hir`
and changes it only where `E0303` is reported for a name an unresolved `use` brought in.

**Trade-offs.** Security does not change: a program with an error never runs, so the error type
never reaches the runtime. Latency on the request path does not change, because this is the checker
alone. The checker pays one type compare per report site. Memory: one interned type per checker run.
Simplicity: one internal type that the user never sees, because it prints as `mixed`. Usability: one
missing name gives one error, so an editor hook that shows the first five shows the real ones.

## Stages

### 1. the error type exists, and every recovery site returns it

**Does:** the checker has a type that means "already reported", and each place that reports an
unresolved name returns it.

- [ ] The rule fragment and its `designed` record, with the policy, the reasons and the Declined list above (`types/one-mistake-reports-once`). `docs/rules/types/one-mistake-reports-once.md:1`, `data/rules/types/one-mistake-reports-once.json:1`
- [ ] The error type, its display as `mixed`, and the choice recorded in the module doc. `crates/nvs-types/src/ty.rs:29`
- [ ] It is assignable to and from every type. `crates/nvs-types/src/ty.rs:29`
- [ ] `resolve_name_type`, `resolve_special` and `resolve_parent` return it after their report. `crates/nvs-types/src/lower.rs:1116`, `crates/nvs-types/src/lower.rs:869`, `crates/nvs-types/src/lower.rs:888`
- [ ] `check_new_target` and the two member recovery sites return it. `crates/nvs-types/src/expr/calls.rs:2184`, `crates/nvs-types/src/expr/members.rs:289`, `crates/nvs-types/src/expr/members.rs:292`
- [ ] Lowering treats it as unreachable, with a debug assertion. `crates/nvs-ir/src/lower/expr.rs:1`

### 2. no report site reports the error type

**Does:** every diagnostic that would name the error type as a value's type stays quiet for it.

**Split:** one `Explore` subagent first lists every `Diagnostic::error` in `crates/nvs-types/src/` whose
message renders a `TypeId`, as `file.rs:NN` and its code. Each one not named below is an item of this
stage.

- [ ] `mismatch()` returns without a report when either side is the error type (`types/one-mistake-reports-once`). `crates/nvs-types/src/expr/assign.rs:659`, `crates/nvs-types/src/expr/assign.rs:636`
- [ ] The hand-built `E0401` sites go through `mismatch()`. `crates/nvs-types/src/check.rs:766`, `crates/nvs-types/src/defaults.rs:452`, `crates/nvs-types/src/signatures.rs:1542`, `crates/nvs-types/src/expr/assign.rs:1476`, `crates/nvs-types/src/expr/mod.rs:1033`, `crates/nvs-types/src/expr/operators.rs:1834`
- [ ] `E0403` is quiet for a returned error type. `crates/nvs-types/src/expr/assign.rs:883`
- [ ] `E0743` is quiet when the type argument is the error type. `crates/nvs-types/src/program.rs:484`, `crates/nvs-types/src/program.rs:494`
- [ ] A member read, a method call, a subscript and an operator on the error type report nothing and give the error type. `crates/nvs-types/src/expr/members.rs:289`, `crates/nvs-types/src/expr/calls.rs:1819`, `crates/nvs-types/src/expr/operators.rs:1834`
- [ ] Each site the subagent listed is quiet for the error type, or the rule says why it reports. `docs/rules/types/one-mistake-reports-once.md:1`

### 3. an unresolved `use` reports once, and the cases pin every root

**Does:** a short name brought in by a `use` that names nothing gives the error type, and conformance
cases pin one diagnostic per mistake.

**Split:** after the code lands, one writing subagent writes the `.nvst` cases below, one after
another. The session reads each expected output against this goal's decisions.

- [ ] The checker reads `Import::resolved == false` and gives the error type with no `E0303` at each use. `crates/nvs-hir/src/resolve.rs:31`, `crates/nvs-hir/src/resolve.rs:278`, `crates/nvs-types/src/lower.rs:1116`
- [ ] `nvs-hir` reports no `E0303` for a name an unresolved `use` brought in. `crates/nvs-hir/src/hierarchy.rs:651`, `crates/nvs-hir/src/hierarchy.rs:766`
- [ ] A parameter typed with an undeclared class, then used as a `string`, gives `E0303` alone. `tests/conformance/lang/an-undeclared-parameter-type-reports-once.nvst:1`
- [ ] A function declared `string` returns such a value, and gives `E0303` alone. `tests/conformance/lang/a-returned-value-of-an-undeclared-type-reports-once.nvst:1`
- [ ] `Core\Program::implementing<Undeclared>()` gives `E0303` alone. `tests/conformance/lang/an-undeclared-program-type-argument-reports-once.nvst:1`
- [ ] A `use` that names nothing, then its short name in three places, gives `E0306` alone. `tests/conformance/lang/an-unresolved-use-reports-once.nvst:1`
- [ ] `new Undeclared()`, then a member read and a call on the result, give `E0303` alone. `tests/conformance/lang/a-new-of-an-undeclared-class-reports-once.nvst:1`
- [ ] A value written as `mixed`, used as a `string` and returned as one, still gives `E0401` and `E0403`. `tests/conformance/lang/a-written-mixed-keeps-its-errors.nvst:1`
- [ ] The rule `shipped` with every case above in `guardedBy`, and `bun nv owners --closes one-error-per-mistake` passes. `data/rules/types/one-mistake-reports-once.json:1`

## Handoff

### State

The goal is new and nothing is built. Every choice is settled in § *Standing decisions*. The anchors
were read from the tree while `compiler-holes` was changing `nvs-types`, so each line number may have
moved. A session finds each site by its function name.

### Next

**Stage 1, one group**, on `nvs-types`. Gate: `docs/goals/compiler-holes.md` no longer exists.

- [ ] The rule fragment and its `designed` record (`types/one-mistake-reports-once`). `docs/rules/types/one-mistake-reports-once.md:1`, `data/rules/types/one-mistake-reports-once.json:1`
- [ ] The error type, its display, and the choice recorded. `crates/nvs-types/src/ty.rs:29`
- [ ] The three name recovery sites return it. `crates/nvs-types/src/lower.rs:1116`, `crates/nvs-types/src/lower.rs:869`, `crates/nvs-types/src/lower.rs:888`
- [ ] `check_new_target` and the member recovery sites. `crates/nvs-types/src/expr/calls.rs:2184`, `crates/nvs-types/src/expr/members.rs:289`
- [ ] Lowering treats it as unreachable. `crates/nvs-ir/src/lower/expr.rs:1`

### Backlog
