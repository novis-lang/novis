---
looper: 1
title: One mistake in a program reports one error
summary: "A name that does not resolve gives its one root diagnostic and no later use of it reports again, a value written as mixed keeps every error it has, the rule says so with its reasons, and the checks below pass."
context: [crates/nvs-types/src/ty.rs, crates/nvs-types/src/lower.rs, docs/rules/types/unions-and-mixed.md]
stages:
  - id: 1
    title: "the error type exists, and every recovery site returns it"
    status: done
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/expr/mod.rs]
    extra:
      context:
        rules: [types/unions-and-mixed, types/one-mistake-reports-once]
  - id: 2
    title: "no report site reports the error type"
    status: done
    context: [crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/check.rs, crates/nvs-types/src/defaults.rs, crates/nvs-types/src/signatures.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-types/src/program.rs, crates/nvs-types/src/expr/members.rs]
    extra:
      context:
        rules: [types/one-mistake-reports-once]
  - id: 3
    title: "an unresolved `use` reports once, and the cases pin every root"
    status: done
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

- [x] The rule fragment and its `designed` record, with the policy, the reasons and the Declined list above (`types/one-mistake-reports-once`). `docs/rules/types/one-mistake-reports-once.md:1`, `data/rules/types/one-mistake-reports-once.json:1`
- [x] The error type, its display as `mixed`, and the choice recorded in the module doc. `crates/nvs-types/src/ty.rs:134`
- [x] It is assignable to and from every type. `crates/nvs-types/src/expr/assign.rs:209`
- [x] `resolve_name_type`, `resolve_special` and `resolve_parent` return it after their report. `crates/nvs-types/src/lower.rs:1125`, `crates/nvs-types/src/lower.rs:867`, `crates/nvs-types/src/lower.rs:902`
- [x] `check_new_target` and the two member recovery sites return it. `crates/nvs-types/src/expr/calls.rs:2196`, `crates/nvs-types/src/expr/members.rs:289`, `crates/nvs-types/src/expr/members.rs:293`
- [x] Lowering treats it as unreachable, with a debug assertion. `crates/nvs-ir/src/lower/mod.rs:4337`

### 2. no report site reports the error type

**Does:** every diagnostic that would name the error type as a value's type stays quiet for it.

**Split:** one `Explore` subagent first lists every `Diagnostic::error` in `crates/nvs-types/src/` whose
message renders a `TypeId`, as `file.rs:NN` and its code. Each one not named below is an item of this
stage.

- [x] `mismatch()` returns without a report when either side is the error type (`types/one-mistake-reports-once`). `crates/nvs-types/src/expr/assign.rs:670`, `crates/nvs-types/src/expr/assign.rs:718`
- [x] The hand-built `E0401` sites go through `type_mismatch()`, which `mismatch()` calls. `crates/nvs-types/src/check.rs:765`, `crates/nvs-types/src/defaults.rs:450`, `crates/nvs-types/src/signatures.rs:1540`, `crates/nvs-types/src/expr/assign.rs:1497`, `crates/nvs-types/src/expr/mod.rs:1031`, `crates/nvs-types/src/expr/operators.rs:1824`
- [x] `E0403` is quiet for a returned error type, through `is_assignable`. `crates/nvs-types/src/expr/assign.rs:903`
- [x] `E0743` is quiet when the type argument is the error type. `crates/nvs-types/src/program.rs:487`
- [x] A member read, a method call, a subscript, an operator, a `foreach` and `::class` on the error type report nothing and give the error type. `crates/nvs-types/src/expr/members.rs:2134`, `crates/nvs-types/src/expr/calls.rs:172`, `crates/nvs-types/src/expr/mod.rs:818`, `crates/nvs-types/src/expr/operators.rs:488`, `crates/nvs-types/src/expr/iteration.rs:173`, `crates/nvs-types/src/expr/members.rs:1738`
- [x] The remaining `Ty::Mixed | Ty::Error` arms decide: quiet, or act as `mixed` with the reason in a comment. `crates/nvs-types/src/expr/quals.rs:175`, `crates/nvs-types/src/expr/quals.rs:211`, `crates/nvs-types/src/expr/quals.rs:1183`, `crates/nvs-types/src/expr/type_test.rs:362`, `crates/nvs-types/src/derive.rs:1042`, `crates/nvs-types/src/attributes.rs:652`, `crates/nvs-types/src/commands.rs:271`
- [x] Recovery sites stage 1 did not name, each `mixed` right after a report, return the error type. `crates/nvs-types/src/lower.rs:133`, `crates/nvs-types/src/lower.rs:521`, `crates/nvs-types/src/lower.rs:657`, `crates/nvs-types/src/lower.rs:699`, `crates/nvs-types/src/lower.rs:724`, `crates/nvs-types/src/lower.rs:826`, `crates/nvs-types/src/expr/members.rs:115`
- [x] The conversion, type-test and text sites are quiet for the error type: `as`, `is`, `echo` and interpolation. `crates/nvs-types/src/expr/operators.rs:2038`, `crates/nvs-types/src/expr/operators.rs:2192`, `crates/nvs-types/src/expr/operators.rs:2277`, `crates/nvs-types/src/expr/operators.rs:2310`, `crates/nvs-types/src/expr/operators.rs:2411`, `crates/nvs-types/src/expr/operators.rs:2857`, `crates/nvs-types/src/expr/operators.rs:3014`, `crates/nvs-types/src/expr/operators.rs:637`, `crates/nvs-types/src/expr/operators.rs:1860`, `crates/nvs-types/src/expr/operators.rs:1900`, `crates/nvs-types/src/expr/type_test.rs:181`, `crates/nvs-types/src/expr/type_test.rs:264`, `crates/nvs-types/src/expr/type_test.rs:287`, `crates/nvs-types/src/expr/type_test.rs:304`
- [x] The expression sites are quiet for the error type. `crates/nvs-types/src/expr/mod.rs:1330`, `crates/nvs-types/src/locals.rs:2109`, `crates/nvs-types/src/expr/literals.rs:1466`, `crates/nvs-types/src/expr/calls.rs:1489`, `crates/nvs-types/src/expr/members.rs:403`, `crates/nvs-types/src/expr/members.rs:569`, `crates/nvs-types/src/expr/members.rs:629`, `crates/nvs-types/src/expr/members.rs:684`, `crates/nvs-types/src/expr/members.rs:1882`, `crates/nvs-types/src/expr/members.rs:2308`, `crates/nvs-types/src/expr/args.rs:1454`, `crates/nvs-types/src/expr/args.rs:1985`, `crates/nvs-types/src/expr/args.rs:2031`, `crates/nvs-types/src/expr/iteration.rs:210`, `crates/nvs-types/src/expr/iteration.rs:324`, `crates/nvs-types/src/expr/iteration.rs:426`, `crates/nvs-types/src/expr/iteration.rs:450`, `crates/nvs-types/src/expr/quals.rs:258`, `crates/nvs-types/src/expr/quals.rs:971`
- [x] The declaration sites are quiet for the error type. `crates/nvs-types/src/check.rs:1129`, `crates/nvs-types/src/check.rs:1152`, `crates/nvs-types/src/check.rs:1221`, `crates/nvs-types/src/defaults.rs:299`, `crates/nvs-types/src/defaults.rs:370`, `crates/nvs-types/src/conformance.rs:126`, `crates/nvs-types/src/conformance.rs:248`, `crates/nvs-types/src/conformance.rs:788`, `crates/nvs-types/src/conformance.rs:1158`, `crates/nvs-types/src/conformance.rs:1373`, `crates/nvs-types/src/lower.rs:266`, `crates/nvs-types/src/lower.rs:328`, `crates/nvs-types/src/lower.rs:362`, `crates/nvs-types/src/lower.rs:647`
- [x] The feature sites are quiet for the error type, or the rule says why one reports. `crates/nvs-types/src/program.rs:152`, `crates/nvs-types/src/program.rs:325`, `crates/nvs-types/src/program.rs:351`, `crates/nvs-types/src/derive.rs:1264`, `crates/nvs-types/src/derive.rs:1747`, `crates/nvs-types/src/derive.rs:1819`, `crates/nvs-types/src/derive.rs:2461`, `crates/nvs-types/src/routes.rs:1582`, `crates/nvs-types/src/routes.rs:2132`, `crates/nvs-types/src/routes.rs:2378`, `crates/nvs-types/src/retrieval.rs:339`, `crates/nvs-types/src/match_attribute.rs:58`, `crates/nvs-types/src/commands.rs:521`, `crates/nvs-types/src/commands.rs:947`, `crates/nvs-types/src/testing.rs:415`, `crates/nvs-types/src/testing.rs:707`, `crates/nvs-types/src/testing.rs:740`, `crates/nvs-types/src/testing.rs:1091`, `crates/nvs-types/src/test_hooks.rs:395`, `crates/nvs-types/src/attributes.rs:672`, `crates/nvs-types/src/intrinsics.rs:1067`, `crates/nvs-types/src/deprecated/template.rs:515`, `docs/rules/types/one-mistake-reports-once.md:1`

### 3. an unresolved `use` reports once, and the cases pin every root

**Does:** a short name brought in by a `use` that names nothing gives the error type, and conformance
cases pin one diagnostic per mistake.

**Split:** after the code lands, one writing subagent writes the `.nvst` cases below, one after
another. The session reads each expected output against this goal's decisions.

- [x] The checker reads `Import::resolved == false` and gives the error type with no `E0303` at each use. `crates/nvs-hir/src/resolve.rs:31`, `crates/nvs-hir/src/resolve.rs:291`, `crates/nvs-hir/src/symbol.rs:181`, `crates/nvs-types/src/lower.rs:1120`
- [x] `nvs-hir` reports no `E0303` for a name an unresolved `use` brought in. `crates/nvs-hir/src/hierarchy.rs:638`, `crates/nvs-hir/src/hierarchy.rs:673`
- [x] A parameter typed with an undeclared class, then used as a `string`, gives `E0303` alone. The stage 1 case pins it (`string $text = $item;`). `tests/conformance/lang/a-name-that-does-not-resolve-reports-once-and-a-written-mixed-keeps-its-error.nvst:16`
- [x] A function declared `string` returns such a value, and gives `E0303` alone. The stage 1 case pins it too. `tests/conformance/lang/a-name-that-does-not-resolve-reports-once-and-a-written-mixed-keeps-its-error.nvst:25`
- [x] `Core\Program::implementing<Undeclared>()` gives `E0303` alone. `tests/conformance/lang/an-undeclared-program-type-argument-reports-once.nvst:1`
- [x] A `use` that names nothing, then its short name in five places, gives `E0306` alone. `tests/conformance/lang/an-unresolved-use-reports-once.nvst:1`
- [x] `new Undeclared()`, then a member read and a call on the result, give `E0303` alone. `tests/conformance/lang/a-new-of-an-undeclared-class-reports-once.nvst:1`
- [x] A static call or static property on an undeclared class, or a missing static member, gives its `E0303` or `E0309` alone. `crates/nvs-types/src/expr/calls.rs:695`, `crates/nvs-types/src/expr/mod.rs:724`, `tests/conformance/lang/a-static-member-of-an-undeclared-class-reports-once.nvst:1`
- [x] A value written as `mixed`, used as a `string` and returned as one, still gives `E0401` and `E0403`. `tests/conformance/lang/a-written-mixed-keeps-its-errors.nvst:1`
- [x] The rule `shipped` with every case above in `guardedBy`, and `bun nv owners --closes one-error-per-mistake` passes. `data/rules/types/one-mistake-reports-once.json:1`

## Handoff

### State

Every stage is done, and the goal is reached: `bun nv verify` is 22 of 22 green, and `bun nv verify
--doc` and `bun nv owners --closes one-error-per-mistake` pass. The rule is `shipped`, and its six cases
are in `guardedBy`.

Stage 3:

- An unresolved `use` reports `E0306` at its line and nothing at any use of its short name.
  `resolve_imports` records the target with `SymbolTable::note_unresolved_import`.
  `undeclared_name` and `undeclared_name_at` (`hierarchy.rs`) return `Option<Diagnostic>`, and give
  `None` for a short name an unresolved `use` brought in. Each caller reports only `Some`, so the
  skip lives in one function. `Undeclared` carries the `imports` map for it. A `use Core\Bogus;` is
  quiet the same way, and a short name with no `use` still reports its `E0303`.
- A static call whose written class side resolves no signature types as the error type
  (`infer_static_call`, `calls.rs`). Its one error is the `E0303` or `E0309` from `nvs_hir::members`,
  or the `Core` unknown-member report. A static property's two fallbacks do the same (`expr/mod.rs`).
  The case-writing subagent found this, as `CoreArr::count()` giving `E0303` and then `E0403`.
- The stage's parameter and return cases are the stage 1 case's lines 16 and 25, so they were not
  written twice.

Stages 1 and 2: `Ty::Error` is the error type (`ty.rs`'s module doc says why it is a variant).
It prints as `mixed`, is assignable both ways, and absorbs a union or an intersection it is in. Every
recovery site returns it, and every report site that would name it as a value's type is quiet. Every
`E0401` goes through `type_mismatch()` (`assign.rs`). The stage 1 conformance case
`a-name-that-does-not-resolve-reports-once-and-a-written-mixed-keeps-its-error` pins one line per group:
expressions, conversions, defaults, declarations and features.

Stage 2's last group, found by probe programs through `nvs check`:

- `class<T>`, `property<T>` and `string<K>` over an unresolved name were quiet only on the first
  lowering. `Diagnostics::report` drops an identical diagnostic without counting it, so the
  error-count guard misses a second lowering of the same annotation. Each now also tests
  `is_error(arg)` and recovers as the error type (`lower.rs`). An alias read `Alias::Case` over an
  error-type expansion reports nothing.
- Quiet now: the delegate field check (`conformance.rs`, answers `true` so no member is owed), a
  `#[Command]` return, a duplicate `#[Fixture]` of the error type (`check_fixture_shape` gives
  `None`), a data row against an error-typed parameter, an unsupplied error-typed test parameter
  (`testing.rs`), a hook's return and parameter type (`test_hooks.rs`), `Attributes::get<Missing>`
  (`retrieval.rs`, `match_attribute.rs`), `Program::constructors` with an error-typed maker
  parameter (`program.rs`), and a derived property against its constructor parameter (`derive.rs`).
- Already quiet when probed: a constant, property or override redeclared at an error type, a double's
  callable, a valueless or missing return, a generator, `#[Route]` captures, `#[Query]`, `#[Option]`,
  a codec's reachability, format templates and attribute names.
- A check a declaration fails whatever its types are still reports, such as a `#[BeforeClass]` method
  that declares a parameter. The rule now says so.

The nightly CI failures the sweep named (run 39462919214) were fixed on main before stage 3. CI run
39487313208 on `67e83f008` passed every `proofs` job and the chapter examples.

During verify, two `nvs-cli` timing tests failed under load once and passed alone:
`parallel_tests_overlap_on_more_than_one_core` and
`serve_arms_a_fleet_entry_when_the_shared_tier_can_take_a_lease`. The next full run was green.

### Next

None. The goal is reached, and the driver retires it once its acceptance check passes.

### Backlog

- `Missing::Red` in a type position gives one `E0405` ("`Missing` has no constant named `Red`") and
  no `E0303`. That is one error, but the message names the wrong mistake. A goal that touches
  `nvs-hir`'s alias reads can reword it. `crates/nvs-hir/src/members.rs:1515`
- `check_class_name_const` asks `symbols.get` where `nvs-hir` asks `symbols.contains`, so `X::class`
  on an extension's class may report `E0303`. Not probed. `crates/nvs-types/src/expr/members.rs:1655`
