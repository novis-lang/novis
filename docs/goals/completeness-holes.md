---
looper: 1
title: Every hole the completeness audit found is closed
summary: "Every gap record this goal owns is closed and its record deleted, and the checks named below pass."
context: [tools/nv/cmd/owners.ts, data/schema/gap.schema.json, docs/rules/programs/memory-priority.md]
stages:
  - id: 1
    title: "a write through `mixed` checks the declared type"
    status: done
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-types/src/expr/assign.rs, docs/rules/types/erased-member-access.md, docs/rules/types/shape-type.md]
    extra:
      context:
        rules: [types/erased-member-access, types/shape-type]
  - id: 2
    title: a request head has one deadline for the whole head
    status: done
    context: [crates/nvs-server/src/io.rs, crates/nvs-config/src/default.toml, docs/rules/http-server/four-idle-waits-all-finite.md]
    extra:
      context:
        rules: [http-server/four-idle-waits-all-finite, http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime]
  - id: 3
    title: "the isolate boundary checks both directions, and `max_tasks` bounds the tree"
    status: done
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-runtime/src/graph.rs, crates/nvs-config/src/tree.rs, crates/nvs-cli/src/main.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/ctx/output.rs, crates/nvs-cli/src/wall.rs, crates/nvs-host/src/watchdog.rs, crates/nvs-config/src/app.rs, crates/nvs-config/src/resolve.rs, crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-stdlib/src/request.rs]
    extra:
      context:
        rules: [security/isolate-values-cross-by-copy, security/isolate-budget-is-the-trees, errors/on-limit, config/no-configuration-file-is-a-complete-configuration, config/one-parser-for-the-boot-path-and-config-set, config/ceilings-are-their-own-directives]
  - id: 4
    title: "a throwing `inout` callee leaks nothing, and a refused stack does not panic"
    status: done
    context: [crates/nvs-ir/src/ty.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-host/src/stack.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/group.rs, data/gaps/nvs-host/the-stack-pool-is-capped-by-a-constant.json]
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
    status: done
    context: [crates/nvs-runtime/src/stream.rs, crates/nvs-server/src/serve.rs, crates/nvs-stdlib/src/response.rs]
    extra:
      context:
        rules: [core-classes/spreadsheet-writes-as-a-stream]
  - id: 7
    title: "`[debug] mode` arms the probes, and the grant gates a query span"
    status: done
    context: [crates/nvs-config/src/debug.rs, crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-stdlib/src/db/span.rs]
    extra:
      context:
        rules: [testing/debug-mode-directive, testing/debug-probes, observability/a-query-is-a-trace-event]
  - id: 8
    title: compiled code does what the checker accepted
    status: done
    context: [crates/nvs-ir/src/ir.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-ir/src/lower/control.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-stdlib/src/instance.rs]
    extra:
      context:
        rules: [types/arrays, expressions/method-reference, iteration/foreach-subjects, expressions/nullable-conversion, types/anonymous-object]
  - id: 9
    title: "JIT pages are freed, and every call site has its probe"
    status: done
    context: [crates/nvs-codegen/src/lib.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/callable.rs, crates/nvs-cli/src/script.rs]
    extra:
      context:
        rules: [config/an-edit-reaches-the-next-request-without-a-restart, testing/debug-probes]
  - id: 10
    title: "syslog is a log target, and completion offers what the class reaches"
    status: done
    context: [crates/nvs-runtime/src/ctx/output.rs, crates/nvs-config/src/log.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/tests/completion.rs]
    extra:
      context:
        rules: [errors/engine-floor, ide/completion-offers-only-what-the-compiler-derived]
  - id: 11
    title: the checker reports what it accepts today
    status: done
    context: [crates/nvs-types/src/lower.rs, crates/nvs-types/src/expr/isolate.rs, crates/nvs-hir/src/qname.rs, crates/nvs-types/src/lateinit.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/defaults.rs, crates/nvs-types/src/error_lib.rs, data/gaps/nvs-types/a-throwable-property-is-writable.json]
    extra:
      context:
        rules: [types/type-alias, security/isolate-shares-nothing, core-api/reserved-namespace, classes/lateinit-read-before-write, core-classes/derive-attribute, types/decimal, classes/readonly-is-written-once]
  - id: 12
    title: "every error message names its cause, and every code is tested"
    status: done
    context: [crates/nvs-cli/src/main.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-diagnostics/src/lib.rs]
    extra:
      context:
        rules: [errors/on-limit, types/conversion, tooling/a-diagnostic-code-carries-its-card]
  - id: 13
    title: every shipped promise the audit named has its test
    status: done
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

- [x] `write_erased_property` checks the element type of an `array<T>` slot, the field classes of a shape slot and every class arm of a union. `crates/nvs-runtime/src/object.rs:5224`
- [x] A conformance case per shape of write, with `Kennel`/`Dog`/`Animal`, asserting the throw. `docs/rules/types/erased-member-access.md:1`
- [x] The module doc's "what a tag does not catch" list is rewritten from what is left, without the false "no memory-safety edge" sentence. `crates/nvs-runtime/src/object.rs:168`
- [x] A shape's field types are invariant in the checker, as the user decided, so a write through a wider view does not compile. `crates/nvs-types/src/expr/assign.rs:489`
- [x] An erased write through `object` or `mixed` into an anonymous object's slot cannot store an object of the wrong class. `crates/nvs-runtime/src/object.rs:5303`
  - [x] `write_erased_property` checks a shape slot against its current value (`same_shape`).
  - [x] A conformance case per receiver asserts the throw and the typed reads after it.
  - [x] The rule states the trade-off, the module doc is rewritten and the record is deleted.

### 2. a request head has one deadline for the whole head

**Does:** `header_timeout` bounds the whole request head, so a client that trickles header bytes is
closed when the deadline passes, and the other three waits stay idle waits.

- [x] `arm` keeps the head phase's deadline when bytes arrive, and sets it when the head starts. `crates/nvs-server/src/io.rs:309`, `crates/nvs-server/src/io.rs:417`
- [x] A test feeds a head one byte at a time and is closed at the deadline. `crates/nvs-server/src/io.rs:46`
- [x] The module doc's clock section, the default file's comment and the rule say three idle waits and one total. `crates/nvs-config/src/default.toml:536`, `docs/rules/http-server/four-idle-waits-all-finite.md:1`

### 3. the isolate boundary checks both directions, and `max_tasks` bounds the tree

**Does:** a spawn argument is checked against the child's class table as the result is against the
parent's, a tree that spawns past `max_tasks` is refused with a limit report, a run past its
`wall_time` or `max_output` is stopped with one, and a `[limits]` value that is not a quantity or is
above its ceiling stops the boot instead of leaving the request with no cap.

- [x] The `nvs_runtime::script` seam returns a unit's class table beside its entry point, and the argument walk runs with it. `crates/nvs-host/src/isolate.rs:64`, `crates/nvs-runtime/src/graph.rs:444`
- [x] A spawn past the root's `max_tasks` is refused, naming the directive. `crates/nvs-config/src/tree.rs:188`
- [x] Whether child CPU is summed against the root's `cpu_time` is checked, and either the code or the rule changes. `crates/nvs-host/src/isolate.rs:717`
- [x] `nvs run` arms a deadline from `wall_time`, and a write past `max_output` raises a flag the next poll reads. `crates/nvs-cli/src/main.rs:3681`, `crates/nvs-runtime/src/ctx/safepoint.rs:756`, `crates/nvs-runtime/src/ctx/output.rs:286`
  - [x] `SafepointFlags::WALL_TIME` and `OUTPUT_LIMIT`, `Limit::WallTime`, and the poll's wall-time branch.
  - [x] `crates/nvs-cli/src/wall.rs` raises the flag at the deadline, and `Core\Time::sleep` ends at it.
  - [x] `Ctx::write_output` raises `OUTPUT_LIMIT` at the crossing, and the handler runs with both flags lowered.
  - [x] Three `.nvst` cases under `tests/conformance/config/`, and the gap record deleted.
- [x] A served request is stopped at its `wall_time` by the watchdog's sweep, as `nvs run`'s clock stops a run. `crates/nvs-host/src/watchdog.rs:1`, `crates/nvs-cli/src/wall.rs:1`
  - [x] `stop_at_wall_time` raises `WALL_TIME` and expires the deadline for each published request past its deadline.
  - [x] An expired deadline is raised again only one interval past it, so a handler keeps its slice.
  - [x] A watchdog unit case and a served busy-loop case, and the gap record deleted.
- [x] The resolve refuses a malformed `[limits]` quantity as E0601 and a global value above its written ceiling, and the runtime never reads a malformed one as no cap. `crates/nvs-config/src/app.rs:134`, `crates/nvs-runtime/src/ctx/limits.rs:959`
  - [x] `app::quantities` parses every limits block's quantity in the resolve.
  - [x] `app::starts` refuses a start above its written ceiling, over the global tree and each folded snapshot.
  - [x] `configured_bytes` reads a malformed value as the key's shipped default.
  - [x] Cases under `nvs config check` and `nvs run`, and the gap record deleted.
- [x] A `__Host-` or `__Secure-` cookie is read only when the request's connection was secure. `crates/nvs-stdlib/src/request.rs:2096`
  - [x] `cookie_of` gives `None` for either prefix on a request whose scheme is not `https`.
  - [x] A unit case over both schemes, and the existing `__Host-` case moved to a secure request.
  - [x] Two conformance cases, one per scheme, and the gap record deleted.

### 4. a throwing `inout` callee leaks nothing, and a refused stack does not panic

**Does:** memory stays bounded by in-flight work when a callee throws past an `inout` write, and a
stack the OS refuses fails one request, not the process.

- [x] The staging reference is released on the throw edge. `crates/nvs-ir/src/lower/mod.rs:2641`, `crates/nvs-ir/src/ty.rs:309`
- [x] A refused reservation ends the one task as a limit, a root through its `Finished` and a `Core\Task` child through its group's `Outcome::Fatal`. `crates/nvs-host/src/scheduler.rs:916`, `crates/nvs-host/src/group.rs:504`
- [x] The pool is bounded by the admission arithmetic. `crates/nvs-host/src/stack.rs:97`

### 5. bytes cross the extension boundary in one copy each way

**Does:** an export with `list<u8>` parameters or results is called through wasmtime's typed calls,
and the lowered argument bytes are charged to the request.

- [ ] A typed call path for `list<u8>` signatures beside the dynamic one. `crates/nvs-ext/src/call.rs:893`
- [x] The lowered argument bytes are charged to the request, as the lifted results are. `crates/nvs-ext/src/call.rs:954`
  - [x] `Request::run` charges the arguments' host bytes before the call and gives them back after it.
  - [x] A call that cannot afford its arguments ends as `Limit::Memory` and gives everything back.
  - [x] `tests/call.rs`'s `size` export pins both, and fails without the charge.
- [ ] The image arms use a full-size frame, and `benches/results/image-guest.json` is recorded again. `crates/nvs-ext/tests/support/image_arms.rs:23`

### 6. a streamed body that ends early is cut off

**Does:** a throw, a limit stop or a cancel in the middle of a streamed body aborts the response, so
the client sees a failed download.

- [x] `Emit` gets an abort ending that `Drain` reports. `crates/nvs-runtime/src/stream.rs:260`
  - [x] `Emit::abort` sets the ending, and a later `finish` or drop does not undo it.
  - [x] `Drain::next_chunk` reports `Drained::Aborted` and discards a chunk in flight.
  - [x] The isolate's `finish` aborts the body when its program threw, met a limit or was cancelled.
- [x] The streaming body turns the abort into an error frame. `crates/nvs-server/src/serve.rs:466`
  - [x] `Drained::Aborted` ends in `BodyCut::ProgramStopped`, and the wire has no terminating chunk.
- [x] A download workbook a program never finishes is logged and cut off. `extensions/spreadsheet/nvs/Workbook.nvs:218`
  - [x] `Core\Response::stream`'s `$mustEnd` labels a stream that must be ended, and `Core\Response\Stream::end` ends it; a write after it throws.
  - [x] `Ctx::end_unfinished_stream` aborts and logs an unended one, from the isolate's finish, `nvs run` and the context's `Drop`.
  - [x] `createForDownload` passes the name, `finish` calls `end()`, and `.nvst` cases pin both.
  - [x] `end`'s proofs, the rule rewritten, and the record deleted.

### 7. `[debug] mode` arms the probes, and the grant gates a query span

**Does:** a `[debug] mode` value turns its probe bits on, and a query span renders only under the
`debug.trace` grant.

- [x] A directive row for `[debug] mode`, read into the request's `DebugFlags`. `crates/nvs-config/src/tree.rs:514`
  - [x] The `debug.mode` row, and a name outside the four refused at boot (`crate::debug::validate`).
  - [x] `Ctx::set_config` arms the named bits off server work, and drops the bits the replaced configuration armed.
  - [x] A server refuses any named probe at boot and at reload (`crate::debug::refuse_serving`).
- [x] The grant gates the span's render on the traced path; a sampled span needs none. `crates/nvs-stdlib/src/db/span.rs:51`
  - [x] `QueryWatch::named` asks `debug.trace` when `DebugFlags::TRACE` is on and the request is not sampled.
  - [x] A unit test pins the refused, granted and untraced cases.

### 8. compiled code does what the checker accepted

**Does:** an array literal keeps every value, a method reference calls the override, an `await`
result is the shape it is typed as, and the two compiler panics on accepted code are gone.

- [x] A positional element after an integer key takes the next key after the highest. `crates/nvs-ir/src/ir.rs:1693`
- [x] `$cls::f(...)` resolves through the class `$cls` holds. `crates/nvs-types/src/expr_table.rs:612`
- [x] `foreach` over `mixed` lowers or is refused by the checker. `crates/nvs-ir/src/lower/control.rs:992`
- [x] `as ?{...}` from `mixed` lowers or is refused by the checker. `crates/nvs-ir/src/lower/convert.rs:443`
- [x] The `Core` shape descriptors take the `$shape{...}` label, so an `await` result renders and encodes as a shape. `crates/nvs-stdlib/src/instance.rs:652`, `crates/nvs-stdlib/src/script.rs:476`

### 9. JIT pages are freed, and every call site has its probe

**Does:** a recompile under `nvs serve` frees the old unit's pages once nothing runs in them, and a
callable or `Core` call passes the call probes.

- [x] A dropped compiled unit frees its pages behind the no-live-frame proof. `crates/nvs-codegen/src/lib.rs:362`, `crates/nvs-cli/src/script.rs:1263`
  - [x] The proof: the pages are owned by the unit's class table, which the unit and every `ErrorClass` a context holds share (`nvs_runtime::ClassTable::keep_code`).
  - [x] `JitPages`'s `Drop` calls `JITModule::free_memory`, and the `Unit` doc says so.
  - [x] `tests/jit_pages.rs` holds `mapped_jit_units` steady over a recompile loop, and the record is deleted.
- [x] The probe pair around `nvs_call_callable` and `CoreCall`, with a label for a call that has no compile-time callee name. `crates/nvs-codegen/src/emit.rs:627`
  - [x] A `Core` call is labelled `Core\Class::member` (`nvs_stdlib::registry::member_label`), and a callable call `Class::invoke`.
  - [x] Codegen emits the pair around a `CoreCall`; `call_callable` makes it around every callable call, a `Core` member's callback included.
  - [x] `tests/calls.rs`'s `a_trace_records_a_core_call_and_every_callback_it_makes` pins the trace and the count.

### 10. syslog is a log target, and completion offers what the class reaches

**Does:** `[log] target = "syslog"` sends records to syslog, and completion offers inherited members
filtered by visibility.

- [x] A syslog sink, or `syslog` dropped from the rule and the parser. `crates/nvs-runtime/src/ctx/output.rs:1044`
- [x] `declared_members` walks the class graph and filters by visibility. `crates/nvs-lsp/src/completion.rs:2928`
  - [x] Reuse `nvs_hir::ClassGraph`'s edges and `nvs_types::signatures::is_visible_from`.
  - [x] Walk parents and interfaces nearest first, so an override hides what it overrides.
  - [x] One `.lspt` case per visibility, and one for inheritance.
- [x] The receiver half, a written type and a string literal each get their own list. `crates/nvs-lsp/src/completion.rs:56`
  - [x] A cursor in the receiver half offers the variables in scope, pinned by a case.
  - [x] Inside a string literal only the variables in scope are offered, and nothing in a single-quoted one.
  - [x] A property's type and a type argument offer types, and the record is deleted.

### 11. the checker reports what it accepts today

**Does:** seven programs the checker accepts or misreports get the right diagnostic, each with a test.

- [x] A recursive alias gets one diagnostic at the alias. `crates/nvs-types/src/lower.rs:68`
  - [x] Alias resolution walks shape fields and the `property<T>`, `propertyPath<T>` and `string<K>` arguments, so a self-naming shape is E0310 once.
  - [x] The bare-class check runs in alias resolution, so an alias of an alias is no E0307.
  - [x] The depth message names no kind of type, and the record and the playbook bullet are deleted.
- [x] A literal `args:` map is checked at compile time. `crates/nvs-types/src/expr/isolate.rs:229`
  - [x] `check_literal_args` compares a method entry's parameters with a literal map's string keys, as `entry_names_agree` does at the spawn.
  - [x] An unknown key is E0486 at the key, and a parameter with no entry is E0402.
  - [x] A reject case, the runtime fixture's map moved into a variable, and the record deleted.
- [x] `use Core\Nope;` is an unknown class. `crates/nvs-hir/src/qname.rs:146`
  - [x] `nvs_types::core_lib::importable` lists every class, interface, enum, `Core` exception and attribute.
  - [x] `CoreRoster::Names` carries it as `imports`, and a `use` naming nothing on it, nor a namespace above one, is E0306.
  - [x] A type annotation naming a `Core` name not on it is E0303.
  - [x] A reject case, the rule's sentence, and the record deleted.
- [x] An inherited `lateinit` read gets E0428. `crates/nvs-types/src/lateinit.rs:48`
  - [x] `lateinit_in_reach` adds each ancestor's non-private `lateinit` property not declared again nearer.
  - [x] Unit cases for a grandparent, a parent constructor call, a private and a redeclared property, and the record deleted.
- [x] A second `#[Json\Field]` is reported. `crates/nvs-types/src/derive.rs:2478`
  - [x] `attribute_fields` reports each later field attribute of the format as E0464, naming the first.
  - [x] The rule, the card and a reject case say one per property, and the record is deleted.
- [x] A `decimal` property default is accepted. `crates/nvs-types/src/defaults.rs:79`
  - [x] `FieldDefault::Decimal`, made by `FieldDefault::decimal` from the literal's parts.
  - [x] The refusal is gone, and an accept case replaces the reject case, and the record is deleted.
- [x] A write to a `Throwable` property after construction is E0782. `crates/nvs-types/src/error_lib.rs:106`, `crates/nvs-types/src/signatures.rs:1696`
  - [x] `error_lib::seed` marks every seeded property readonly (`SignatureTable::seed_readonly`).
  - [x] The layout marks the tree's slots readonly, so a write through `mixed` throws.
  - [x] A reject case and a run case, and the record is deleted.

### 12. every error message names its cause, and every code is tested

**Does:** a limit stop in runtime bookkeeping names its directive, a failed class conversion names
the class, and a check fails for a diagnostic code no test raises.

- [x] The empty `FATAL: ` path raises the usual limit report. `crates/nvs-runtime/src/ctx/error.rs:860`
  - [x] `Ctx::take_thrown` builds the promoted error inside `budget::Reporting`.
  - [x] A conformance case pins the report for a ledger grown past `[limits] memory`, and fails without the fix.
- [x] The `class<T>` conversion error names the operand. `crates/nvs-ir/src/lower/convert.rs:1083`
  - [x] The refused edge concatenates the string, or a narrowed descriptor's name.
  - [x] The conformance case covers a name, an unrelated class and a narrowing, and valgrind finds no leak.
- [x] A check over `code::ALL`, and tests for E0001, E0004, E0005, E0217 and E0768. `crates/nvs-diagnostics/tests/every_code_is_tested.rs:1`
  - [x] `every_code_is_asserted_by_its_code_in_a_test` fails for a code no test names as `[E…]`, `"E…"` or `code::NAME`.
  - [x] E0104, W1001 and W1004, which nothing raised, are retired.
  - [x] Reject cases for E0001, E0004 and E0005; E0217 and E0768 already had Rust tests by code.
  - [x] Tests for E0641 and W1014 to W1019, and the record is deleted.

### 13. every shipped promise the audit named has its test

**Does:** each `no-test-*` record and the examples record is closed by the test it names.

- [x] Each record under `data/gaps/` whose slug starts `no-test-` gets its test, and the record is deleted. `crates/nvs-db/src/pg.rs:6081`
  - [x] An `inout` alias at both copy sites: E0793 for a spawn entry, E0714 at `Core\Serialize::encode`.
  - [x] `bindTo`, `bind` and `call` on a function a method made without `$this`.
  - [x] A derived decoder reporting four bad fields, and a revealed secret crossing spawn, worker and serialize.
  - [x] A failed pool reset destroys the connection for MySQL, SQL Server and SQLite.
  - [x] `Core\Server::traceId` throws in a spawned child, the one request member of that class.
  - [x] The three `nvs-cli` records: no `stat` per resolve, `nvs queue migrate` twice, `nvs run --dump-asm`.
  - [x] The `nvs-codegen` one: a JIT page is not writable after the flip.
- [x] `examples/*.nvs` run in a test the compiler and runtime reach. `crates/nvs-cli/tests/examples.rs:1`
  - [x] `crates/nvs-cli/tests/examples.rs` runs every example and checks its status.
  - [x] `examples/cycles.nvs`, which the test found broken, runs again.
  - [x] The record is deleted.

### 14. a prose known gap without a record fails a check

**Does:** `bun nv owners --check` fails on a module-doc "known gap" that names no record, so owed
work cannot live only in prose again.

- [x] The scan reads "known gap" headings and numbered gap references, not only the "owed" wording. `tools/nv/cmd/owners.ts:225`
  - [x] A `Known gaps` block is a finding unless its first line is the pointer to its own file and a record names that file.
  - [x] `gap N` on any line of a source, test, manifest or conformance case is a finding.
  - [x] `bun nv owners --prose` lists them by kind and exits 1 while any remains; the roster lists them too.
- [x] `bun nv owners --check` fails on a prose gap, once `--prose` is empty. `tools/nv/cmd/owners.ts:610`
- [x] Every module the scan names gets a record or loses the prose: about 40 numbered "gap N" references remain (`rg -n 'gap [0-9]+' crates tests`), and the prose gaps at `crates/nvs-runtime/src/ctx/error.rs:777` and `crates/nvs-ir/src/ty.rs:224`. `tools/nv/cmd/gaps.ts:30`
- [ ] No comment cites a decision record: the ~270 "ADR" citations cite the rule that holds the decision, or go (`rg -n '\bADR\b' crates`). `crates/nvs-types/src/lib.rs:1`

## Handoff

### State

Stages 1 to 4 and 6 to 13 are done. Stage 14's first three items are done: `bun nv owners --check` runs
the prose scan and exits 1 on a finding (`tools/nv/cmd/owners.ts:611`), and the scan is empty. Every
numbered `gap N` reference is now present-tense prose or cites its record. The assertion-ledger gap is
`data/gaps/nvs-runtime/an-assertion-outside-a-test-grows-the-ledger.json`, owned by `test-surface`. The
`Ty::Tagged` known gap was stale: arithmetic, truthiness and an array read through `mixed` all run, and
the doc says so.

Found while checking the rewrites: `Core\Router::url(params: …, name: …)` type-checked and then died
with a FATAL, because lowering read the arguments by position. `ExprInfo::RouteLink` now records
`params_at` and `settings_at`, lowering evaluates them in written order, and
`tests/conformance/core/a-route-table-answers-a-url-for-every-route-it-holds.nvst` pins it.

Tree-wide perf drift, not from this goal: a `Core` member call counts as one `calls` per op, so every
bench declaring `calls 0` fails `--record-perf`.

The nightly run the sweep reports (`39013536848`) tested `e110c9c0b`, which is `origin/main` and far
behind local `main`.

`bun nv rules --render` also rewrites `core-api.md`, `errors.md` and `iteration.md`, and `bun nv render`
drops `done` from `data/schema/milestone.schema.json`. That drift predates these sessions and is not
committed.

### Next

**Stage 14's last item: no comment cites a decision record. About 250 `ADR` citations in about 140 files
(`rg -n '\bADR\b' crates tests`), split by crate across subagents that run one after another.**

- [ ] `crates/nvs-types` and `crates/nvs-ir`: each `ADR` citation cites the rule that holds the decision (`bun nv peek rule:<id>` to check the id), or is rewritten as present tense. Start at `crates/nvs-types/src/expr/operators.rs:1` (`rule:testing/feature-proofs`, `tools/nv/cmd/rules.ts:1`).
- [ ] `crates/nvs-stdlib`: the same. Start at `crates/nvs-stdlib/src/registry.rs:1`.
- [ ] `crates/nvs-config`, `nvs-runtime`, `nvs-server`, `nvs-host`, `nvs-db`, `nvs-cli`, `nvs-diagnostics`, `nvs-codegen`, `nvs-render` and `tests/conformance`: the same. Start at `crates/nvs-diagnostics/src/lib.rs:1`.
- [ ] Make a check fail on a new `ADR` citation, beside the prose scan (`tools/nv/cmd/owners.ts:242`'s `outsideBlocks`, or `bun nv rules --check`).

### Backlog

- Stage 5 follows `data/gaps/nvs-ext/bytes-cross-one-val-per-byte.json`: generated bindings per first-party world, then the image arms at full size, plus `Markup + Markup` in place (`docs/perf/performance-pass.md:257`).
- A `Core` member call counts as a Novis call in the perf counters, so every `calls 0` bench fails `--record-perf`: find the commit, then fix the count or the declarations (`tools/nv/proofs/collect.ts`, `benches/members/core/Response-Stream/length.nvs:5`).
- `rule:security/isolate-shares-nothing` says a *required* parameter `args:` omits is the error, but `entry_names_agree` and `check_literal_args` need every parameter, defaults included. Decide which is right and make the two agree (`crates/nvs-stdlib/src/script.rs:970`).
- Tests that fail under the full verify's load and pass alone, causes not found: `test_request_dispatches_in_process_through_the_compiled_route_table` (`crates/nvs-cli/src/runner.rs:3410`), two `live_edit` swaps (`crates/nvs-cli/tests/live_edit.rs:1010`), and five `ldap` tests against the Samba server (`crates/nvs-stdlib/tests/ldap.rs:102`).
- CI's Windows warm-start budget fails: `nvs run`'s work is 12-13 ms against 6, about 4.7 ms of it `nvs_ext::section::read`'s `parse_all` (`crates/nvs-ext/src/section.rs:35`). Proof debt fails CI's `proofs` jobs: `bun nv benches --owed` and `bun nv proofs --owed` (`.github/workflows/ci.yml:1`).
- A per-connection child refused a stack is dropped unrun, so its client likely sees the socket close and no `503`; not checked (`crates/nvs-server/src/serve.rs:95`).
