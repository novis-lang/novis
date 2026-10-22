---
looper: 1
title: "A request's failure paths free nothing early, leave nothing behind, and no input outgrows its memory limit"
summary: "A property, static or reference write stores before it releases, and an abandoned generator a finally brought back is not freed; the armed memory ceiling, its refusal and its reserve travel with their task across every switch, and a limit handler runs outside the reserve; a form or query name of any shape is refused at the memory ceiling inside one parse; multipart field names count against request_body, the part-header search is linear, a body reader that failed fails again, and max_multipart_parts is a [limits] row; a panic on the safepoint path is a FATAL; each shape has its attack or Rust test; every rule this goal touched says what is true; and the checks below pass."
context: [docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md, docs/rules/security/isolate-shares-nothing.md, docs/rules/http-server/the-residue-is-one-named-fault-class.md]
stages:
  - id: 1
    title: "an abandoned generator a `finally` brought back is not freed, and a write stores before it releases"
    status: done
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-runtime/src/release.rs, crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-codegen/src/emit.rs, docs/rules/iteration/generators.md]
    extra:
      context:
        rules: [iteration/generators, security/isolate-teardown-is-a-drain-then-a-sweep]
  - id: 2
    title: "the armed memory ceiling, its refusal and its reserve travel with their task"
    status: done
    context: [crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/ctx/current.rs, crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-runtime/src/ctx/hooks.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/ladder.rs, crates/nvs-server/src/serve.rs]
    extra:
      context:
        rules: [security/isolate-shares-nothing, errors/on-limit, http-server/admission-is-arithmetic-not-a-number]
  - id: 3
    title: a form or query name of any shape stops at the memory ceiling inside one parse
    status: done
    context: [crates/nvs-stdlib/src/uri.rs, crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/budget.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/multipart.rs]
    extra:
      context:
        rules: [errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself, http-server/request-body-and-upload-total-are-two-caps]
  - id: 4
    title: "a multipart body is read in linear time, a failed reader fails again, and `max_multipart_parts` is a `[limits]` row"
    status: done
    context: [crates/nvs-stdlib/src/multipart.rs, crates/nvs-server/src/body.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-config/src/app.rs, crates/nvs-config/src/defaults.rs, crates/nvs-config/src/default.toml, docs/rules/errors/multipart-part-count.md]
    extra:
      context:
        rules: [errors/multipart-part-count, http-server/the-body-is-read-on-demand-under-two-caps, config/ceilings-are-their-own-directives]
  - id: 5
    title: "a panic on the safepoint path is a `FATAL`, never a process abort"
    status: done
    context: [crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/abi.rs, crates/nvs-runtime/src/ctx/hooks.rs]
    extra:
      context:
        rules: [http-server/the-residue-is-one-named-fault-class, errors/on-limit]
  - id: 6
    title: "the attacks, and every rule this goal touched says what is true"
    status: done
    context: [tests/hostile/README.md, tests/hostile/core/Request/post/01-a-form-with-one-broken-escape.nvs, tests/hostile/core/Request/post/01-a-form-with-one-broken-escape.nvsr, tests/hostile/lang/iteration/generators/01-driving-a-generator-off-its-path.nvs, crates/nvs-cli/tests/request.rs]
    extra:
      context:
        rules: [testing/hostile-case-contract, errors/multipart-part-count]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: runtime-tests
    stage: 1
    name: "the release, generator and budget tests pass"
    run: [bun, nv, verify, --fast, -p, nvs-runtime]
  - id: host-tests
    stage: 2
    name: the scheduler carries the armed state across a switch
    run: [bun, nv, verify, --fast, -p, nvs-host]
  - id: server-tests
    stage: 2
    name: the state-bleed suite passes with its memory row
    run: [bun, nv, verify, --fast, -p, nvs-server]
  - id: stdlib-tests
    stage: 4
    name: "the query, form and multipart parsers pass"
    run: [bun, nv, verify, --fast, -p, nvs-stdlib]
  - id: rules-check
    stage: 6
    name: every rule loads and every citation resolves
    run: [bun, nv, rules, --check]
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, failure-paths-hold]
    reach: true
    floor: false
---

# A request's failure paths free nothing early, leave nothing behind, and no input outgrows its memory limit

## Why

On 2026-10-21 PHP published two critical advisories, GHSA-w8hw-w9fx-j79w and GHSA-6wmf-4rv5-p3rp. Both
were remote code execution, and both came from failure paths rather than ordinary ones:

- PHP leaves a request on out-of-memory with `longjmp()`. Cleanup is skipped, and shutdown then reads
  state that was only half built. Cheap multipart input reaches that memory limit: a body with no
  length, unbounded headers per part, and empty file parts under nested names.
- Global state outlives the request whose memory it points into. A fatal error skips the reset, or a
  late diagnostic fills it again. The next request on the worker reads it before its own setup runs.

An audit of Novis against both shapes found that the mechanism itself is absent. Every abort is a
status return through landing blocks. A memory refusal is a no-op taken before the allocation. No body
is parsed before the program asks for one. There is no global error or session slot. The audit also
found these holes:

1. **A use-after-free from plain Novis code, reproduced.** A property write releases the old value
   before it stores the new one (`crates/nvs-runtime/src/object.rs:3814`). Releasing a suspended
   generator runs its `finally` (`crates/nvs-runtime/src/object.rs:4146`). That `finally` can still read
   the property and keep the generator. `dismantle` then frees it whatever its count reads
   (`crates/nvs-runtime/src/object.rs:4155`). A thirty-line program reached a freed class pointer on the
   debug build. This is PHP's old "destructor runs during an assignment" class.
2. **The armed memory state outlives its request.** `CEILING`, `POLLED`, `REFUSED` and `REPORTING` are
   per thread (`crates/nvs-runtime/src/budget.rs:243`). `Ctx::new` saves them and `Ctx::drop` restores
   them, which holds only while contexts end in the order they began
   (`crates/nvs-runtime/src/ctx/wiring.rs:128`, `crates/nvs-runtime/src/ctx/mod.rs:1604`). An await
   already interleaves requests on one core, and `yield_on` swaps none of these
   (`crates/nvs-host/src/scheduler.rs:1599`). A resumed request is then checked against another
   request's ceiling, or against none. A dropped context can restore a `POLLED` word whose tree is
   already freed, and a later allocation ORs flag bits into freed memory. This is the advisory's shape:
   stale per-worker state from one request, consumed later. A limit handler also runs user code inside
   the reserve, which turns the pre-check off (`crates/nvs-runtime/src/ctx/hooks.rs:570`,
   `crates/nvs-host/src/ladder.rs:111`).
3. **One parse call runs far past the memory ceiling.** Form and query names build nested arrays with no
   bound but the input's length (`crates/nvs-stdlib/src/uri.rs:2240`). `NvsArray::new` asks no
   ceiling (`crates/nvs-runtime/src/array.rs:1037`), so an 8 MB body of `a[][][]…` builds an estimated
   gigabyte inside one `post()`. Multipart field names are not counted against `request_body`
   (`crates/nvs-stdlib/src/multipart.rs:494`). Several such requests together abort the process.
4. **Smaller holes.**
   - The part-header search restarts after every chunk, which is quadratic on 1-byte chunks (`crates/nvs-stdlib/src/multipart.rs:458`).
   - A caught body-limit throw followed by a second `post()` gives a form with a field missing (`crates/nvs-stdlib/src/multipart.rs:484`).
   - A read retried after a wire error may park until the wall-time limit (`crates/nvs-server/src/body.rs:184`).
   - `max_multipart_parts` is a constant that `rule:errors/multipart-part-count` calls a directive (`crates/nvs-stdlib/src/multipart.rs:68`).
   - A panic in `nvs_safepoint`'s slow branches aborts the process (`crates/nvs-runtime/src/ctx/safepoint.rs:777`).

Security and request isolation is the first priority, so this goal runs before every goal after the
live one.

## Standing decisions

**A dying generator that its `finally` brought back stays alive.** `dismantle` unlinks the object only
after the unwind entry returns. If the count is then above the one the resurrection made, it drops that
one reference, relinks the object and returns without sweeping its fields. The generator is finished, so
`advance()` returns `false` and no `finally` runs twice. This is the root fix, because it holds for
every route by which user code could reach the object, not only a property slot. The doc's sentence
"nothing else can observe it" (`crates/nvs-runtime/src/object.rs:4113`) is rewritten to what is then true.

**Every write that replaces a counted value stores first and releases after.** That covers properties,
statics and reference cells, in the runtime and in compiled code. A `finally` that runs inside the
release then reads the new value, which is what a program expects. Arrays already displace first and
release after (`crates/nvs-runtime/src/array.rs:1985`) and are left alone. Cost: none. The same two
operations run in the other order.

**The armed state travels with its task, the way `CurrentStack` does.** A new `budget::ArmedStack`
holds `CEILING`, `POLLED`, `REFUSED` and `REPORTING`. Its `take` leaves the thread at `Armed::NONE`, no
refusal and no reserve, and its `restore` puts the task's values back. `yield_on` takes it beside
`CurrentStack::take` and restores it beside `current.restore()`. Per-context nesting in `Ctx::new` and
`Ctx::drop` stays, because contexts on one stack still nest. Cost: four thread-local moves per switch
and four words per parked task, O(in-flight). The balance cells `LIVE`, `PEAK` and `WRITTEN` are not
moved here: charging the balance per turn is `core-time-slice` stage 3, and that goal's standing
decision says so.

**User code never runs inside the reserve.** The limit handler's `_reserve` guard closes before
`call_callable` (`crates/nvs-runtime/src/ctx/hooks.rs:625`), as `rule:errors/on-limit` and the reserve's
own doc already say (`crates/nvs-runtime/src/budget.rs:288`). The log ladder closes its reserve before
the handler isolate starts (`crates/nvs-host/src/ladder.rs:111`). Safe default if the report array
needs the reserve while the handler reads it: build the report inside the reserve and close it before
the call, which is what the comment at `crates/nvs-runtime/src/ctx/hooks.rs:610` already describes.

**No new nesting or count limit for form names.** The user's direction is no new limits. The memory
ceiling is the bound, and it is asked in front of each array a name builds. `NvsArray::try_new` asks
`budget::affords` for one header and its empty table. `uri::branch` uses it, and a refusal ends the parse
with the request's memory refusal as a `Fault`. `NvsArray::new` stays for arrays whose count the
runtime chooses. Safe default if threading the refusal through `insert` is awkward: `parse_query` asks
`budget::over_memory_limit` once per pair and stops there. That costs one thread-local load per pair.

**Multipart field names count against `request_body` beside their values,** in the same check before
the copy (`crates/nvs-stdlib/src/multipart.rs:484`).

**The part-header search resumes where it stopped.** After a fill it starts three bytes before the old
end of the buffer, so a block is scanned once whatever the chunking. The admission at
`crates/nvs-stdlib/src/multipart.rs:51` is deleted.

**A body reader that failed fails the same way again.** `Multipart` keeps its first error and returns it
to every later call, so a caught throw never yields a partial form. `Pull` and `whole_body` keep a
wire error and return it again instead of parking.

**`max_multipart_parts` becomes a `[limits]` row, default 1000, with a `[limits.hard]` ceiling of
10000,** placed like `upload_total` (`crates/nvs-config/src/app.rs:493`,
`rule:config/ceilings-are-their-own-directives`). The rule already decided the directive, and the code
lacks it. The breach stays a throw from the reading call, because a body is read on demand
(`rule:http-server/the-body-is-read-on-demand-under-two-caps`). The rule's "`413`" and its per-part
name and size caps are rewritten to the real bounds: the part count, the 16 KiB header block, which
bounds names and filenames, `request_body` for field names and values, and `upload_total` for files.

**The slow branches of `nvs_safepoint` run inside `catch_unwind`.** A panic in the limit handler's
runtime half, in collection or in `run_helper`'s pre-check becomes the request's `FATAL`, as a panic in
a helper body already does (`crates/nvs-runtime/src/abi.rs:466`). The fast path stays one load and one
branch. The ctx-less primitives such as `nvs_array_set` are out of scope. Their bodies call no user code,
and they are in `## Backlog`.

**The attacks are hostile cases where a `.nvsr` can carry the request, and Rust tests otherwise.** A
chunked body, 1-byte chunks and two interleaved requests cannot be written in a `.nvsr`. They go beside
`crates/nvs-cli/tests/request.rs:342` and the state-bleed suite
(`crates/nvs-server/src/serve.rs:8510`). Safe default if a `.nvsr` cannot carry a body large enough to
cross the default memory limit: the case lowers `[limits] memory` the way the cases under
`tests/hostile/config/` set a limit.

**No reporter's names.** The advisories are public. The tests, comments and commits name the shapes,
never the advisory text.

**Lanes.** Stage 1 (object model and lowering), stages 3 and 4 (`crates/nvs-stdlib`) and stages 2 and 5
(budget, scheduler and safepoint) touch disjoint files. They can run as side goals. Stage 4 follows
stage 3, and stage 5 follows stage 2, because each pair shares a file. Stage 6 needs all of them.

**Trade-offs.** Security gains on every finding above. Latency: four thread-local moves per task
switch, one compare per array built from input, and a linear header search instead of a quadratic one.
Memory: four words per parked task. Simplicity: one new `[limits]` row, which the rule already promised.
The rest is code that now does what its comments say.

## Stages

### 1. an abandoned generator a `finally` brought back is not freed, and a write stores before it releases

**Does:** the reproduced program prints that the `finally` saw the new value, and no freed object is
ever reached.

**Split:** an `Explore` subagent lists every site that releases a slot's old value before writing the
new one, in `crates/nvs-runtime/src` and `crates/nvs-ir/src/lower`, as `file:line` with one line each.

- [x] `dismantle` unlinks after the unwind, and an object whose count rose during it is kept on its list, not freed (`rule:iteration/generators`). `crates/nvs-runtime/src/object.rs:4129`, `crates/nvs-runtime/src/object.rs:4106`
- [x] Runtime slot writes store, then release. `crates/nvs-runtime/src/object.rs:3814`, `crates/nvs-runtime/src/object.rs:5671`, `crates/nvs-runtime/src/object.rs:6124`
- [x] Compiled property, static, `set` hook body and reference-cell writes store, then release. `crates/nvs-ir/src/lower/stmt.rs:1385`, `crates/nvs-ir/src/lower/mod.rs:3792`, `crates/nvs-ir/src/lower/mod.rs:1735`, `crates/nvs-ir/src/lower/stmt.rs:1352`, `crates/nvs-codegen/src/emit.rs:3351`
- [x] A conformance case: a `finally` reads the property its generator is being cleared from, and keeps what it reads. It prints that the value is `null`. `tests/conformance/iter/a-finally-reads-the-property-its-generator-is-cleared-from.nvst:1`
- [x] Rust test: a dying generator whose unwind keeps a reference stays alive, finished, and is freed once at the last release. `crates/nvs-runtime/src/object.rs:4129`
- [x] Rust test: in-flight collection of a suspended generator in a cycle runs no `finally` against a nulled local. `crates/nvs-runtime/src/object.rs:3330`
- [x] The rule says what a `finally` run by an abandoned generator sees, and that a generator it keeps stays alive. `docs/rules/iteration/generators.md:12`

### 2. the armed memory ceiling, its refusal and its reserve travel with their task

**Does:** two requests that interleave on one core are each checked against their own ceiling, and no
thread ever names a freed safepoint word.

- [x] `budget::ArmedStack` takes and restores `CEILING`, `POLLED`, `REFUSED` and `REPORTING`, modelled on `CurrentStack` (`rule:security/isolate-shares-nothing`). `crates/nvs-runtime/src/budget.rs:243`, `crates/nvs-runtime/src/ctx/current.rs:107`
- [x] `yield_on` takes it beside `CurrentStack::take` and restores it beside `current.restore()`, and its doc lists it. `crates/nvs-host/src/scheduler.rs:1552`, `crates/nvs-host/src/scheduler.rs:1599`
- [x] `arm`'s caller obligation and `Ctx::drop`'s restore comment say what holds now. `crates/nvs-runtime/src/budget.rs:443`, `crates/nvs-runtime/src/ctx/mod.rs:1599`
- [x] The limit handler runs after the reserve closes, and the log ladder closes its reserve before the handler isolate (`rule:errors/on-limit`). `crates/nvs-runtime/src/ctx/hooks.rs:570`, `crates/nvs-runtime/src/ctx/hooks.rs:625`, `crates/nvs-host/src/ladder.rs:111`
- [x] Rust test: task A arms and parks, task B makes and drops a context, A resumes under its own ceiling, and after both end the thread's `POLLED` is null. `crates/nvs-host/src/scheduler.rs:1615`
- [x] The state-bleed suite gains a memory row with two interleaved requests, one over its limit and one under. `crates/nvs-server/src/serve.rs:8540`
- [x] `core-time-slice`'s standing decision on memory says the arming already travels, and its stage 3 charges the balance only. `docs/goals/core-time-slice.md:138`
- [x] The rule says every per-task value the allocator reads travels with the task. `docs/rules/security/isolate-shares-nothing.md:34`

### 3. a form or query name of any shape stops at the memory ceiling inside one parse

**Does:** a form whose names nest past the memory limit stops with the memory error before the request
holds more than its ceiling.

- [x] `NvsArray::try_new` asks `budget::affords` for one header and its empty table (`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`). `crates/nvs-runtime/src/array.rs:1037`
- [x] `uri::branch` and `insert` build through it, and a refusal ends `parse_query` and `place` with the memory refusal. `crates/nvs-stdlib/src/uri.rs:2211`, `crates/nvs-stdlib/src/uri.rs:2251`, `crates/nvs-stdlib/src/uri.rs:3181`, `crates/nvs-stdlib/src/uri.rs:3234`
- [x] `post()`, `query()` and their array forms surface it as the request's memory error. `crates/nvs-stdlib/src/request.rs:1801`, `crates/nvs-stdlib/src/request.rs:1838`
- [x] The doc that says nesting needs no bound is rewritten: the ceiling bounds it, in front of each array. `crates/nvs-stdlib/src/uri.rs:2240`
- [x] Multipart field names count against `request_body` beside their values. `crates/nvs-stdlib/src/multipart.rs:480`
- [x] Rust tests: a name of 100 000 `[]` under a 4 MiB limit stops with the memory error, and the peak stays under the limit; a multipart name counts against `request_body`. `crates/nvs-stdlib/src/uri.rs:3279`, `crates/nvs-stdlib/src/multipart.rs:1`
- [x] The rule says arrays built from input are refused in front of the allocation, and stops saying the fallible constructor is the only way to make one. `docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md:8`

### 4. a multipart body is read in linear time, a failed reader fails again, and `max_multipart_parts` is a `[limits]` row

**Does:** 1000 parts of 16 KiB headers in 1-byte chunks are scanned once, a caught body throw is thrown
again by the next read, and the part count is configuration.

**Split:** an `Explore` subagent lists where the `[limits]` rows are documented for a user (reference
card, website configuration page, `nvs` help) as `file:line`. That text follows AGENTS.md § *Text an end
user reads*.

- [x] The header search resumes three bytes before the old end of the buffer, and the admission of the quadratic cost is deleted. `crates/nvs-stdlib/src/multipart.rs:492`, `crates/nvs-stdlib/src/multipart.rs:51`
- [x] `Multipart` keeps its first error and returns it to every later call. `crates/nvs-stdlib/src/multipart.rs:365`
- [x] `Pull` and `whole_body` keep a wire error and return it again rather than parking. `crates/nvs-server/src/body.rs:283`, `crates/nvs-stdlib/src/request.rs:3365`, `crates/nvs-runtime/src/ctx/inbound.rs:996`
- [x] `[limits] max_multipart_parts`, default 1000, ceiling 10000, read where `MAX_PARTS` is (`rule:config/ceilings-are-their-own-directives`). `crates/nvs-config/src/app.rs:545`, `crates/nvs-config/src/defaults.rs:87`, `crates/nvs-config/src/default.toml:82`, `crates/nvs-runtime/src/ctx/limits.rs:1147`
- [x] The head's byte cap is set in Novis code beside `MAX_HEADER_LINES`, at hyper's current default. `crates/nvs-server/src/serve.rs:238`
- [x] Rust tests: 1000 parts in 1-byte chunks scan each header block once; a second `post()` after a caught throw throws the same error; a read after a wire error returns it. `crates/nvs-stdlib/src/multipart.rs:1162`, `crates/nvs-stdlib/src/request.rs:5394`, `crates/nvs-server/src/body.rs:361`
- [x] The rule names the real bounds and says a breach is a throw from the reading call (`rule:errors/multipart-part-count`). `docs/rules/errors/multipart-part-count.md:1`
- [x] The user-facing `[limits]` documentation gains the row. `crates/nvs-config/src/default.toml:63`, `docs/reference/tools/20-config.md:127`

### 5. a panic on the safepoint path is a `FATAL`, never a process abort

**Does:** a panic forced in the limit handler's runtime half or in collection ends that request with a
`FATAL`, and the next request on the core is answered.

- [x] The slow branches of `nvs_safepoint` run inside `catch_unwind`, and the "nothing here can panic" reasons are rewritten. `crates/nvs-runtime/src/ctx/safepoint.rs:778`, `crates/nvs-runtime/src/ctx/safepoint.rs:937`
- [x] `run_helper`'s memory pre-check and collection run inside the same `catch_unwind` as the body. `crates/nvs-runtime/src/abi.rs:445`
- [x] Rust test: a panic forced in the collection branch is a `FATAL` and the context drops cleanly. `crates/nvs-runtime/src/ctx/safepoint.rs:1028`
- [x] The residue rule says a Rust panic is never part of the residue. `docs/rules/http-server/the-residue-is-one-named-fault-class.md:1`

### 6. the attacks, and every rule this goal touched says what is true

**Does:** each shape this goal closed has an attack or a Rust test that fails if it opens again.

**Split:** a writing subagent writes the hostile cases one after another, once stages 1 to 5 have
landed. The session checks each against `rule:testing/hostile-case-contract` and AGENTS.md § *Text an
end user reads*.

- [x] Attack: a urlencoded form whose names nest past the memory limit stops with the memory error. `tests/hostile/core/Request/post/02-a-form-whose-names-nest-past-the-memory-limit.nvs:1`, `tests/hostile/core/Request/post/02-a-form-whose-names-nest-past-the-memory-limit.nvsr:1`
- [x] Attack: the same shape in a query string. `tests/hostile/core/Request/query/02-a-query-whose-names-nest-past-the-memory-limit.nvs:1`, `tests/hostile/core/Request/query/02-a-query-whose-names-nest-past-the-memory-limit.nvsr:1`
- [x] Attack: more parts than `max_multipart_parts`, most of them empty files, some with many headers. `tests/hostile/core/Request/files/02-a-body-of-many-empty-parts.nvs:1`, `tests/hostile/core/Request/files/02-a-body-of-many-empty-parts.nvsr:1`
- [x] Attack: a program catches the body-limit error and reads the form again. `tests/hostile/core/Request/post/03-reading-the-form-again-after-its-limit.nvs:1`, `tests/hostile/core/Request/post/03-reading-the-form-again-after-its-limit.nvsr:1`
- [x] Attack: a `finally` keeps the generator that is being freed. `tests/hostile/lang/iteration/generators/02-a-finally-that-keeps-its-own-generator.nvs:1`
- [x] Rust tests: a chunked body over `request_body` and one over `upload_total`, read by `post()`. `crates/nvs-cli/tests/request.rs:342`
- [x] `bun nv rules --check` and `bun nv owners --closes failure-paths-hold` are green, and every rule fragment touched above is in the present tense. `docs/rules/errors/multipart-part-count.md:1`

### 7. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [x] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [x] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Every stage is done, and the goal's gate is green. Stages 1 to 6 landed what the reached-when line
names. Each rule they touched says what is true, and `bun nv owners --closes failure-paths-hold`
names no gap.

Stage 7 ran `bun nv affected --run` three times. The first stopped at `render`: the generated
`docs/reference/lang/80-concurrency.md` was stale after stage 2's rule edit, and `bun nv render`
wrote it. The second stopped at `test`, and its failures had five causes:

- `nvs-ir` lib: six lowering snapshots pinned the release of a property's old value before its
  store. Stage 1 moved the store first, so each `.snap` under `crates/nvs-ir/src/lower/snapshots/`
  now has `field.set` above `release`. Nothing else changed.
- `nvs-stdlib` `capability`: `nvs_stdlib_reaches_the_os_only_through_the_gate`
  (`crates/nvs-stdlib/tests/capability.rs:338`) stopped at the first `#[cfg(test)]` and allowed one
  per file. Stage 4 added one on a field and one on a block in `multipart.rs`. The scan now stops
  only at a `#[cfg(test)]` whose next line opens a `mod`, and allows one such module per file. It
  scans more lines than before.
- `nvs-host` `limits`: `a_limit_handler_is_handed_a_report_naming_the_limit` and
  `a_childs_memory_breach_arrives_as_ok_false_and_the_parent_keeps_running`. The five report
  handlers in `crates/nvs-host/tests/limits.rs` built a `"limit"` key string of their own. Since
  stage 2 a handler runs outside the reserve, and the tests' 1 MiB hog is past even the widened
  ceiling, so that string was refused and the lookup found nothing. The handlers now read the
  report with the borrowed `NvsArray::get(b"limit")`. The runtime is unchanged.
- `nvs-cli` `live_edit`: `a_file_that_changes_during_a_compile_discards_that_compile` and
  `a_link_switch_during_a_compile_never_builds_a_program_from_both_releases` missed their 30 s swap
  bound beside the other 238 binaries, and pass alone in 4.7 s.
- `nvs-cli` `examples`: `queue-purge.nvs` and `cache-shared-tls.nvs` hit the socket-readiness
  deadline to the healthy Postgres and Redis containers under the same load, and pass alone.

The third run was green, 22 of 22. It re-ran the 11 binaries the store selected, `live_edit` and
`examples` among them, with less load beside them. The two load-bound cases are not fixed at their
cause, and the backlog names them. `bun nv verify --doc` is green. The acceptance sweep's reds were
stale again: the `env-secret-proofs` log shows exit 0, and nightly run 39567373487 ran at
`afbb1edc2`.

### Next

The goal is reached. The driver runs the acceptance checks, and the goal is then retired from the
chain the way `e9c83e4fe` retired `defaults-that-work`. `core-time-slice` is next.

### Backlog

- The ctx-less primitives (`nvs_array_set`, `nvs_str_concat`) are `extern "C"` with no `catch_unwind`. Their bodies call no user code, so a panic there is a bug in Rust code and aborts the process. `crates/nvs-runtime/src/array.rs:1985`
- Two `live_edit` cases compile 2000 methods twice inside a fixed 30 s bound, which a full coverage gate run can exceed. A bound measured against the boot compile would hold under any load. `crates/nvs-cli/tests/live_edit.rs:29`
- `NvsTcp`'s readiness wait reports a timeout when the event arrives after the deadline, so a loaded gate run fails `examples` on a socket that was ready. `crates/nvs-host/src/net.rs:1670`
- 382 `.profraw` files (about 1 GB, from 9 October onward) sit in `crates/nvs-stdlib/src`, so some coverage run starts a binary there with a relative profile path. They are ignored by git and were not removed. `.gitignore:55`
