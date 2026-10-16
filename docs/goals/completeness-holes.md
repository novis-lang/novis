---
looper: 1
title: Every hole the completeness audit found is closed
summary: "Every gap record this goal owns is closed and its record deleted, and the checks named below pass."
context: [tools/nv/cmd/owners.ts, data/schema/gap.schema.json, docs/rules/programs/memory-priority.md]
stages:
  - id: 1
    title: "a write through `mixed` checks the declared type"
    status: open
    context: [crates/nvs-runtime/src/object.rs, docs/rules/types/erased-member-access.md, data/gaps/nvs-runtime/a-write-through-mixed-skips-the-element-and-shape-check.json]
    extra:
      context:
        rules: [types/erased-member-access]
  - id: 2
    title: a request head has one deadline for the whole head
    status: open
    context: [crates/nvs-server/src/io.rs, crates/nvs-config/src/default.toml, docs/rules/http-server/four-idle-waits-all-finite.md, data/gaps/nvs-server/header-timeout-is-an-idle-wait-not-a-whole-head-deadline.json]
    extra:
      context:
        rules: [http-server/four-idle-waits-all-finite, http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime]
  - id: 3
    title: "the isolate boundary checks both directions, and `max_tasks` bounds the tree"
    status: open
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-runtime/src/graph.rs, crates/nvs-config/src/tree.rs, crates/nvs-cli/src/main.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/ctx/output.rs, data/gaps/nvs-host/an-isolate-argument-is-not-checked-for-an-unresolvable-class.json, data/gaps/nvs-config/max-tasks-is-enforced-nowhere.json, data/gaps/nvs-cli/wall-time-and-max-output-are-not-enforced-under-nvs-run.json, crates/nvs-config/src/app.rs, crates/nvs-config/src/resolve.rs, crates/nvs-runtime/src/ctx/limits.rs, data/gaps/nvs-config/a-malformed-limit-or-one-above-its-ceiling-is-not-refused.json]
    extra:
      context:
        rules: [security/isolate-values-cross-by-copy, security/isolate-budget-is-the-trees, errors/on-limit, config/no-configuration-file-is-a-complete-configuration, config/one-parser-for-the-boot-path-and-config-set, config/ceilings-are-their-own-directives]
  - id: 4
    title: "a throwing `inout` callee leaks nothing, and a refused stack does not panic"
    status: open
    context: [crates/nvs-ir/src/ty.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-host/src/stack.rs, data/gaps/nvs-ir/a-callee-that-throws-leaks-its-inout-staging-retain.json, data/gaps/nvs-host/the-stack-pool-is-capped-by-a-constant-and-a-refused-stack-panics.json]
    extra:
      context:
        rules: [statements/inout-is-the-by-reference-spelling, concurrency/a-task-stack-is-reserved-wide-and-pooled]
  - id: 5
    title: bytes cross the extension boundary in one copy each way
    status: open
    context: [crates/nvs-ext/src/convert.rs, crates/nvs-ext/src/call.rs, data/gaps/nvs-ext/bytes-cross-one-val-per-byte.json]
    extra:
      context:
        rules: [packaging/values-cross-as-handles, packaging/the-boundary-is-the-cost, core-classes/image-pipeline]
  - id: 6
    title: a streamed body that ends early is cut off
    status: open
    context: [crates/nvs-runtime/src/stream.rs, crates/nvs-server/src/serve.rs, data/gaps/nvs-server/a-streamed-body-cannot-be-cut-off.json]
    extra:
      context:
        rules: [core-classes/spreadsheet-writes-as-a-stream]
  - id: 7
    title: "`[debug] mode` arms the probes, and the grant gates a query span"
    status: open
    context: [crates/nvs-config/src/tree.rs, crates/nvs-db/src/span.rs, data/gaps/nvs-config/debug-mode-is-read-by-nothing.json, data/gaps/nvs-db/span-renders-on-a-flag-no.json]
    extra:
      context:
        rules: [testing/debug-mode-directive, testing/debug-probes, observability/a-query-is-a-trace-event]
  - id: 8
    title: compiled code does what the checker accepted
    status: open
    context: [crates/nvs-ir/src/ir.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-ir/src/lower/control.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-stdlib/src/instance.rs, data/gaps/nvs-stdlib/an-await-result-is-a-named-class-not-a-shape.json]
    extra:
      context:
        rules: [types/arrays, expressions/method-reference, iteration/foreach-subjects, expressions/nullable-conversion, types/anonymous-object]
  - id: 9
    title: "JIT pages are freed, and every call site has its probe"
    status: open
    context: [crates/nvs-codegen/src/lib.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/callable.rs, crates/nvs-cli/src/script.rs, data/gaps/nvs-codegen/jit-pages-leak-on-every-recompile.json, data/gaps/nvs-runtime/callable-call-passes-no-call-probe.json]
    extra:
      context:
        rules: [config/an-edit-reaches-the-next-request-without-a-restart, testing/debug-probes]
  - id: 10
    title: "syslog is a log target, and completion offers what the class reaches"
    status: open
    context: [crates/nvs-runtime/src/ctx/output.rs, crates/nvs-config/src/log.rs, crates/nvs-lsp/src/completion.rs, data/gaps/nvs-runtime/log-target-syslog-is-not-transported.json, data/gaps/nvs-lsp/two-of-them-are-the-class.json]
    extra:
      context:
        rules: [errors/engine-floor, ide/completion-offers-only-what-the-compiler-derived]
  - id: 11
    title: the checker reports what it accepts today
    status: open
    context: [crates/nvs-types/src/lower.rs, crates/nvs-types/src/expr/isolate.rs, crates/nvs-hir/src/qname.rs, crates/nvs-types/src/lateinit.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/defaults.rs, crates/nvs-types/src/error_lib.rs, data/gaps/nvs-types/a-throwable-property-is-writable.json]
    extra:
      context:
        rules: [types/type-alias, security/isolate-shares-nothing, core-api/reserved-namespace, classes/lateinit-read-before-write, core-classes/derive-attribute, types/decimal, classes/readonly-is-written-once]
  - id: 12
    title: "every error message names its cause, and every code is tested"
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-diagnostics/src/lib.rs]
    extra:
      context:
        rules: [errors/on-limit, types/conversion, tooling/a-diagnostic-code-carries-its-card]
  - id: 13
    title: every shipped promise the audit named has its test
    status: open
    context: [docs/rules/testing/feature-proofs.md]
    extra:
      context:
        rules: [testing/feature-proofs]
  - id: 14
    title: a prose known gap without a record fails a check
    status: open
    context: [tools/nv/cmd/owners.ts, tools/nv/cmd/gaps.ts]
    extra:
      context:
        rules: [tooling/a-repository-fact-is-one-json-record]
checks:
  - id: owners-gate
    stage: 14
    name: every gap record names an owner the gate accepts
    run: [bun, nv, owners, --check]
  - id: owners-closes
    stage: 14
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, completeness-holes]
    reach: true
    floor: false
---

# Every hole the completeness audit found is closed

## Why

A read-only audit of the tree found 27 holes: places where the code does not do what a rule
promises, or does it with no test to prove it. One is a memory-safety bug. Several more break
request isolation. The rest are wrong results, compiler panics on code the checker accepts,
diagnostics that say the wrong thing, and shipped promises with no test. Each hole is a gap record
under `data/gaps/` that names this goal, so `bun nv owners --closes completeness-holes` refuses to
reach the goal while one is open. `bun nv gaps --module <file>` prints a record with its anchors.

## Standing decisions

**Memory safety first.** The priority ordering puts security and request isolation above
everything, so the stages run in that order: the memory-safety bug, then the isolation holes, then
the correctness items, then diagnostics and test holes, then the tooling stage.

**Prototyping regime, and no compatibility to keep.** The workspace is `0.0.1` and Novis is not
released (`rule:programs/no-compatibility-promise`). A fix that changes what a program observes goes
in as it is. There is no deprecation step.

**No new limits unless the user approves one.** A hole closes by enforcing a limit a rule already
names, never by adding a configuration key or a cap of its own. A fix that seems to need a new
limit stops and asks.

**Extension byte crossing uses typed calls.** The call bridge uses wasmtime's typed calls for
exports whose parameters or results are `list<u8>`: one copy each way, and the lowered argument
bytes are charged to the request. The `nvs:ext@1.0.0` world does not change. Third-party exports
with other signatures stay on the dynamic path. The user decided this.

**The request head gets one deadline for the whole head.** `header_timeout` becomes a whole-head
deadline, fixed when the head starts (at accept, or at the first byte after a keep-alive wait) and
not pushed out by progress. The body, write and keep-alive waits stay idle waits. There is no new
configuration key and no head size cap, because a proxy owns size. A connection in its head phase
counts against no cap. The rule is rewritten as three idle waits and one total, as
`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime` words its own pair. The user
decided this, and because it is a slow-loris hole it is stage 2.

**A hole closes when its record is deleted.** A session that closes one deletes its record in the
same commit and rewrites the module prose that described it, present tense and whole. A record
whose fix turns out to be a decision only the user can make stays open, and the handoff names the
question.

**Trade-offs.** Stage 1 adds a check per element on a `mixed` write into a typed array, which costs
latency only on the erased path. Stage 5 removes a per-byte host allocation, which saves memory and
latency. Stage 9 frees code pages, which turns memory that grows with edits into memory bounded by
what is loaded. No stage is expected to spend memory per request. A stage that finds it must says
what it spends, as `rule:programs/memory-priority` asks.

## Stages

### 1. a write through `mixed` checks the declared type

**Does:** an erased write into an `array<Class>`, a shape or a union-typed property is checked
against the declared type before it stores, so no typed read can see an object of the wrong class.

- [ ] `write_erased_property` checks the element type of an `array<T>` slot, the field classes of a shape slot and every class arm of a union. `crates/nvs-runtime/src/object.rs:5224`
- [ ] A conformance case per shape of write, with `Kennel`/`Dog`/`Animal`, asserting the throw. `docs/rules/types/erased-member-access.md:1`
- [ ] The module doc's "what a tag does not catch" list is rewritten from what is left, without the false "no memory-safety edge" sentence. `crates/nvs-runtime/src/object.rs:168`

### 2. a request head has one deadline for the whole head

**Does:** `header_timeout` bounds the whole request head, so a client that trickles header bytes is
closed when the deadline passes, and the other three waits stay idle waits.

- [ ] `arm` keeps the head phase's deadline when bytes arrive, and sets it when the head starts. `crates/nvs-server/src/io.rs:309`, `crates/nvs-server/src/io.rs:417`
- [ ] A test feeds a head one byte at a time and is closed at the deadline. `crates/nvs-server/src/io.rs:46`
- [ ] The module doc's clock section, the default file's comment and the rule say three idle waits and one total. `crates/nvs-config/src/default.toml:536`, `docs/rules/http-server/four-idle-waits-all-finite.md:1`

### 3. the isolate boundary checks both directions, and `max_tasks` bounds the tree

**Does:** a spawn argument is checked against the child's class table as the result is against the
parent's, a tree that spawns past `max_tasks` is refused with a limit report, a run past its
`wall_time` or `max_output` is stopped with one, and a `[limits]` value that is not a quantity or is
above its ceiling stops the boot instead of leaving the request with no cap.

- [ ] The `nvs_runtime::script` seam returns a unit's class table beside its entry point, and the argument walk runs with it. `crates/nvs-host/src/isolate.rs:64`, `crates/nvs-runtime/src/graph.rs:444`
- [ ] A spawn past the root's `max_tasks` is refused, naming the directive. `crates/nvs-config/src/tree.rs:188`
- [ ] Whether child CPU is summed against the root's `cpu_time` is checked, and either the code or the rule changes. `crates/nvs-host/src/isolate.rs:717`
- [ ] `nvs run` arms a deadline from `wall_time`, and a write past `max_output` raises a flag the next poll reads. `crates/nvs-cli/src/main.rs:3681`, `crates/nvs-runtime/src/ctx/safepoint.rs:756`, `crates/nvs-runtime/src/ctx/output.rs:286`
- [ ] The resolve refuses a malformed `[limits]` quantity as E0601 and a global value above its written ceiling, and the runtime never reads a malformed one as no cap. `crates/nvs-config/src/app.rs:134`, `crates/nvs-runtime/src/ctx/limits.rs:959`

### 4. a throwing `inout` callee leaks nothing, and a refused stack does not panic

**Does:** memory stays bounded by in-flight work when a callee throws past an `inout` write, and a
stack the OS refuses fails one request, not the process.

- [ ] The staging reference is released on the throw edge. `crates/nvs-ir/src/lower/mod.rs:2641`, `crates/nvs-ir/src/ty.rs:309`
- [ ] The pool is bounded by the admission arithmetic, and a refused reservation reaches the spawn as an error. `crates/nvs-host/src/stack.rs:54`, `crates/nvs-host/src/stack.rs:196`

### 5. bytes cross the extension boundary in one copy each way

**Does:** an export with `list<u8>` parameters or results is called through wasmtime's typed calls,
and the lowered argument bytes are charged to the request.

- [ ] A typed call path for `list<u8>` signatures beside the dynamic one. `crates/nvs-ext/src/call.rs:893`
- [ ] The lowered argument bytes are charged to the request, as the lifted results are. `crates/nvs-ext/src/call.rs:954`
- [ ] The image arms use a full-size frame, and `benches/results/image-guest.json` is recorded again. `crates/nvs-ext/tests/support/image_arms.rs:23`

### 6. a streamed body that ends early is cut off

**Does:** a throw, a limit stop or a cancel in the middle of a streamed body aborts the response, so
the client sees a failed download.

- [ ] `Emit` gets an abort ending that `Drain` reports. `crates/nvs-runtime/src/stream.rs:260`
- [ ] The streaming body turns the abort into an error frame. `crates/nvs-server/src/serve.rs:466`

### 7. `[debug] mode` arms the probes, and the grant gates a query span

**Does:** a `[debug] mode` value turns its probe bits on, and a query span renders only under the
`debug.trace` grant.

- [ ] A directive row for `[debug] mode`, read into the request's `DebugFlags`. `crates/nvs-config/src/tree.rs:514`
- [ ] The grant gates the span's render on the sampled and unsampled paths. `crates/nvs-db/src/span.rs:63`

### 8. compiled code does what the checker accepted

**Does:** an array literal keeps every value, a method reference calls the override, an `await`
result is the shape it is typed as, and the two compiler panics on accepted code are gone.

- [ ] A positional element after an integer key takes the next key after the highest. `crates/nvs-ir/src/ir.rs:1693`
- [ ] `$cls::f(...)` resolves through the class `$cls` holds. `crates/nvs-types/src/expr_table.rs:612`
- [ ] `foreach` over `mixed` lowers or is refused by the checker. `crates/nvs-ir/src/lower/control.rs:992`
- [ ] `as ?{...}` from `mixed` lowers or is refused by the checker. `crates/nvs-ir/src/lower/convert.rs:443`
- [ ] The `Core` shape descriptors take the `$shape{...}` label, so an `await` result renders and encodes as a shape. `crates/nvs-stdlib/src/instance.rs:652`, `crates/nvs-stdlib/src/script.rs:476`

### 9. JIT pages are freed, and every call site has its probe

**Does:** a recompile under `nvs serve` frees the old unit's pages once nothing runs in them, and a
callable or `Core` call passes the call probes.

- [ ] A dropped compiled unit frees its pages behind the no-live-frame proof. `crates/nvs-codegen/src/lib.rs:362`, `crates/nvs-cli/src/script.rs:1263`
- [ ] The probe pair around `nvs_call_callable` and `CoreCall`, with a label for a call that has no compile-time callee name. `crates/nvs-codegen/src/emit.rs:627`

### 10. syslog is a log target, and completion offers what the class reaches

**Does:** `[log] target = "syslog"` sends records to syslog, and completion offers inherited members
filtered by visibility.

- [ ] A syslog sink, or `syslog` dropped from the rule and the parser. `crates/nvs-runtime/src/ctx/output.rs:1044`
- [ ] `declared_members` walks the class graph and filters by visibility. `crates/nvs-lsp/src/completion.rs:2918`

### 11. the checker reports what it accepts today

**Does:** seven programs the checker accepts or misreports get the right diagnostic, each with a test.

- [ ] A recursive alias gets one diagnostic at the alias. `crates/nvs-types/src/lower.rs:68`
- [ ] A literal `args:` map is checked at compile time. `crates/nvs-types/src/expr/isolate.rs:229`
- [ ] `use Core\Nope;` is an unknown class. `crates/nvs-hir/src/qname.rs:146`
- [ ] An inherited `lateinit` read gets E0428. `crates/nvs-types/src/lateinit.rs:48`
- [ ] A second `#[Json\Field]` is reported. `crates/nvs-types/src/derive.rs:2478`
- [ ] A `decimal` property default is accepted. `crates/nvs-types/src/defaults.rs:79`
- [ ] A write to a `Throwable` property after construction is E0782. `crates/nvs-types/src/error_lib.rs:106`, `crates/nvs-types/src/signatures.rs:1696`

### 12. every error message names its cause, and every code is tested

**Does:** a limit stop in runtime bookkeeping names its directive, a failed class conversion names
the class, and a check fails for a diagnostic code no test raises.

- [ ] The empty `FATAL: ` path raises the usual limit report. `crates/nvs-cli/src/main.rs:3789`
- [ ] The `class<T>` conversion error names the operand. `crates/nvs-ir/src/lower/convert.rs:1070`
- [ ] A check over `code::ALL`, and tests for E0001, E0004, E0005, E0217 and E0768. `crates/nvs-diagnostics/src/lib.rs:5871`

### 13. every shipped promise the audit named has its test

**Does:** each `no-test-*` record and the examples record is closed by the test it names.

- [ ] Each record under `data/gaps/` whose slug starts `no-test-` gets its test, and the record is deleted. `crates/nvs-db/src/pg.rs:6081`
- [ ] `examples/*.nvs` run in a test the compiler and runtime reach. `data/gaps/nv/the-examples-run-end-to-end-only-under-valgrind.json:1`

### 14. a prose known gap without a record fails a check

**Does:** `bun nv owners --check` fails on a module-doc "known gap" that names no record, so owed
work cannot live only in prose again.

- [ ] The scan reads "known gap" headings and numbered gap references, not only the "owed" wording. `tools/nv/cmd/owners.ts:225`
- [ ] Every module the scan names gets a record or loses the prose. `tools/nv/cmd/gaps.ts:30`

## Handoff

### State

The goal is new and nothing is built. Each of the 27 audit holes is a gap record naming this goal,
and so is each of the four holes `docs/reference/findings.md`'s re-check confirmed (U7, U9, U13,
D29, in stages 3, 12, 11 and 8); `bun nv owners` lists them. The extension byte-crossing gap and the two designed rules it
blocks (`rule:packaging/values-cross-as-handles`, `rule:core-classes/image-pipeline`) moved here
from M9. `bun nv owners --check` accepts a gap owned by a goal further down the chain.

### Next

**Stage 1, the memory-safety bug.** Files: `crates/nvs-runtime/src/object.rs` and its conformance
cases.

- [ ] `write_erased_property` checks an `array<T>` slot's elements, a shape slot's field classes and a union's class arms before it stores (`rule:types/erased-member-access`). `crates/nvs-runtime/src/object.rs:5224`
- [ ] A conformance case per write shape, asserting the throw. `docs/rules/types/erased-member-access.md:1`
- [ ] The module doc's list is rewritten from what is left. `crates/nvs-runtime/src/object.rs:168`

### Backlog

- Many module docs still carry numbered or prose "known gap" text with no record; stage 14's scan finds them (`tools/nv/cmd/owners.ts:225`).
- `Markup + Markup` in place is a decision for the user (`docs/perf/performance-pass.md:258`).
- `rule:packaging/values-cross-as-handles` may be fully built once stage 5 lands; ship it then (`data/rules/packaging/values-cross-as-handles.json`).
