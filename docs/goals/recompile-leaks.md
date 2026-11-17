---
looper: 1
title: "a recompile frees what it made on every way out, and a crashed compile is not retried"
summary: "Reached when a compile that fails or panics in code generation frees its compiled code, a compile that panics is recorded as failed until its content changes, a busy keep-alive connection keeps serving through many reloads, the memory each successful recompile keeps is measured and bounded, a static call through self or static in a generator runs, every construct compiles in every kind of body with no panic, and bun nv serve-mem prints the memory per recompile."
context: [crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/script.rs, crates/nvs-cli/src/serve/mounts.rs, docs/rules/config/a-broken-edit-fails-the-requests-that-resolve-it.md]
stages:
  - id: 1
    title: bun nv serve-mem measures memory per recompile and the keep-alive answers under edits, and the tests are written red
    status: open
    context: [tools/nv/cmd/lsp-start.ts, crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/script.rs]
  - id: 2
    title: a unit frees its compiled code on every way out of compile
    status: open
    context: [crates/nvs-codegen/src/lib.rs]
  - id: 3
    title: a compile that panics is recorded as failed until its content changes
    status: open
    context: [crates/nvs-cli/src/script.rs, docs/rules/config/a-broken-edit-fails-the-requests-that-resolve-it.md]
  - id: 4
    title: a busy keep-alive connection keeps serving through many reloads
    status: open
    context: [crates/nvs-server/src/serve.rs, crates/nvs-cli/src/serve/mounts.rs, crates/nvs-runtime/src/budget.rs]
  - id: 5
    title: what a successful recompile keeps is measured, bounded and written down
    status: open
    context: [crates/nvs-cli/src/serve/mounts.rs, crates/nvs-cli/src/script.rs]
  - id: 6
    title: no source text makes the compiler panic, from the parser to code generation
    status: open
    context: [crates/nvs-fuzz-checks/src/lib.rs, crates/nvs-fuzz-checks/src/prefix.rs, crates/nvs-syntax/tests/prefixes.rs, crates/nvs-codegen/src/lib.rs]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: pages
    stage: 2
    name: "a unit whose code generation returns an error or panics unmaps its pages, and the mapped-unit count returns to where it was"
    run: [cargo, test, --lib, --, a_unit_whose_codegen_fails_unmaps_its_pages, a_unit_whose_codegen_panics_unmaps_its_pages]
    timeout: 1800
    tests:
      - a_unit_whose_codegen_fails_unmaps_its_pages
      - a_unit_whose_codegen_panics_unmaps_its_pages
  - id: no-retry
    stage: 3
    name: "a compile that panics is compiled once per content, answers as a failed compile, and compiles again once its content changes"
    run: [cargo, test, --lib, --, a_compile_that_panics_is_not_compiled_again_until_its_content_changes]
    timeout: 1800
    tests:
      - a_compile_that_panics_is_not_compiled_again_until_its_content_changes
  - id: keep-alive
    stage: 4
    name: "a keep-alive connection that serves without a pause answers 200 with the newest version after many reloads"
    run: [cargo, test, --test, serve_reload, --, a_busy_keep_alive_connection_serves_the_newest_version_after_many_reloads]
    timeout: 1800
    tests:
      - a_busy_keep_alive_connection_serves_the_newest_version_after_many_reloads
  - id: no-panic
    stage: 6
    name: "every saved compile input, and every construct in every kind of body, compiles or reports diagnostics, and never panics or returns a code generation error"
    run: [cargo, test, --lib, --, compile_seeds_never_panic, every_construct_compiles_in_every_body]
    timeout: 1800
    tests:
      - compile_seeds_never_panic
      - every_construct_compiles_in_every_body
---

# a recompile frees what it made on every way out, and a crashed compile is not retried

## Why

A user's `nvs serve` reached 6 GB in a day of editing, and a WSL machine froze. The user measured that
seven failed recompiles in five minutes, with no requests, took the server from 237 MB to 611 MB, and
none of it came back. The same day had at least 47 failed recompiles and 37 panics of the recompile
thread within one minute.

Measured on 2026-11-17 on copies of `apps/registry` and `benches/compare/cases/html-page`, on Windows
and in a Linux container (scripts in `.agent-tmp/serve-mem/`):

- **Request load alone does not grow memory:** 1.8 million requests moved it by under 0.1 MB.
- **A type or syntax error costs nothing per failure.** It stops before code generation. The first one
  costs 9 MB on Linux and 38 MB on Windows, once.
- **A successful recompile keeps memory.** The first one after boot adds 50 MB on the registry copy.
  Each later one adds 0.7 MB on Linux with no requests, and 8 to 11 MB on Windows with requests. None of
  it comes back when idle.
- **Under load with edits, busy keep-alive connections break.** At about 60k requests a second with
  the small app rewritten every 1.5 s, after 20 to 40 edits, every busy keep-alive connection answers
  `500` with an empty body in about 0.02 ms. It goes on after the edits stop. New connections and a slow
  keep-alive connection get `200`. It reproduced twice on Windows and was not seen on Linux at a third of
  the rate.

Read from the code, and confirmed against it:

1. **A compile that stops in code generation leaks its compiled code.** `compile`
   (`crates/nvs-codegen/src/lib.rs:1122`) makes a `JITModule` in `UnitBuilder::new` (`:2500`), then
   runs `compile_all(program)?` and `finish()`. Only `finish` wraps the module in `JitPages` (`:2840`),
   and only `JitPages::drop` (`:506`) calls `free_memory`. `UnitBuilder` (`:1461`) has no `Drop`.
   `cranelift-jit` 0.137.1's `Memory::drop` (`memory/system.rs:146`) forgets every allocation by
   design. So a `CodegenError`, a `finalize_definitions` error or a panic between `:2500` and `:2840`
   loses the unit's code, read-only and data pages for good.
2. **A compile that panics is compiled again every pass.** A panic in `self.compile`
   (`crates/nvs-cli/src/script.rs:1001`) unwinds past the `Failed` state (`:1012-1014`). `Landing`
   (`:463`) wakes the waiters and leaves nothing under the key. The watch thread catches the panic
   (`:1769`) and compiles the same content on its next pass, 2 s later. A request with nothing else to
   serve compiles it too.

Together, a file that panics in code generation leaks one unit every 2 s, with no saves and no
requests.

**The reported panic, reproduced on the current build.** A static call through `self::` or `static::`
inside a generator passes `nvs check` and panics in `nvs-ir` (`crates/nvs-ir/src/lower/mod.rs:2982`).
The message says `Base::pages$gen::advance` names `static` but has neither a receiver nor a called
class. Writing the class name, as in `Base::one()`, works. Reading a constant through `self::` works.

```nvs
class Base
{
    public static function run(): Iterator<int> { return self::pages(); }
    private static function pages(): Iterator<int> { yield self::one(); }
    private static function one(): int { return 1; }
}
```

Measured with that file served, with no requests: the server compiled it again about every 2 s, 66
panics in 140 s, and its threads went from 24 to 21. Its memory stayed at 9 MB. So a panic in lowering
leaks nothing on a small file. It happens before any code is mapped. The reported 50 MB per failure is
not explained yet. Stage 5 measures a panic on the registry copy before it closes.

## Standing decisions

**The fix (the person's call).** A recompile frees what it made on every way out. A crashed compile
is not retried until its content changes. The keep-alive `500`s are found and fixed. What a successful
recompile keeps is measured and bounded. This goal comes before `lsp-memory`, because every developer
who uses the server while editing meets it.

**The compiled code is owned from the moment it is mapped (my call).** `UnitBuilder::new` puts the
`JITModule` into `JitPages` at once, and `MAPPED_JIT_UNITS` counts it from there. `finish` takes the
pages out of the builder and into the class table, as now. A builder dropped on an error or by an unwind
drops `JitPages`, which frees the pages. No function of the unit was called, so the `SAFETY` reason
holds as written: no address was taken. The object-file backend (`UnitBuilder<ObjectModule>`) maps
nothing and does not change. **Declined:** a custom `memory_provider` that frees on drop. It changes
the allocator of every unit to fix the one path that never ends in `finish`.

**A panic is a failed compile for its content (my call).** `compile` in `script.rs` runs under
`catch_unwind`. A panic becomes `CompileState::Failed` with the message "the compiler stopped on an
internal error" and the panic text, recorded under the content's key like any other failure. So it is
answered and not compiled again until the content or something it read changes. stderr gets one line
per content, not one per pass. The server answers it as `rule:config/a-broken-edit-fails-the-requests-that-resolve-it`
says for a broken edit. That rule says so. A panic is still a bug, so the line asks the reader to report it.

**The keep-alive `500`s are found before they are fixed (my call).** The lead is the per-thread byte
counter (`crates/nvs-runtime/src/budget.rs:164-189`). A unit compiled on one thread and freed on a
serving core moves that core's balance down. So a request's memory reading could drift until the
memory check stops every request at once. Stage 4 first proves or rules this out. It reruns the
reproduction with `[limits] memory` raised, and reads the stop reason the server logs. The fix follows
the cause. If the cause is the counter, a unit's bytes are counted as detached (`budget.rs`
`detached_bytes`), because a unit belongs to the process and not to a request. Raising the limit is
never the fix.

**The bound on kept units stays, and is written down (my call).** Each core's mount slot holds an
`Rc<Entry>` (`crates/nvs-cli/src/serve/mounts.rs:172`) until that core serves a request. So the units
alive are at most the cache's three plus one per core, which is about 190 MB on 16 cores for a 10 MB
unit. That is O(in-flight), not growth with traffic. Stage 5 measures the rest of the creep per success
with the live-byte counters and fixes what grows with the number of recompiles. Growth with recompiles
served is a leak, the same as growth with requests served.

**The compiler never panics, on any input (the person's call).** A panic is a bug in Novis whatever
text reached it: broken, half-saved or valid. It is found by search, not from a report. The fuzz targets
in `crates/nvs-fuzz-checks` stop at the parser (`parse`, `prefix`, `ast`, `lex`), and nothing searches
the type checker, the lowering or code generation. An editing loop that saves half-written files is the
input most likely to reach them. So stage 6 adds a `compile` target over the whole pipeline, and a
matrix test that puts each construct into each kind of body. The reported panic is that shape: a
construct that reads the enclosing class, in a generator. The matrix is the check, because it runs in
seconds on every `cargo test`. A timed search is not part of the goal, and the person declined one.
The properties are: no panic, and a program that type-checks with no error never gets a `CodegenError`.
Such an error is a shape the checker let through and the backend cannot lower. The fix for each case is
in the front end or the backend, wherever the case belongs. Catching the panic (stage 3) stays, as the
last line of defence.

**What it costs.** A builder holds its pages in `JitPages` one call earlier, which is no new memory. A
compile runs under one more `catch_unwind`. A panicked content keeps one failure message in the
compile table, as a failed compile does now.

**Lanes.** Stages 2 and 3 touch disjoint crates and may run as side goals after stage 1. Stage 4 needs
stage 1's command. Stage 5 needs stages 2 and 3. Stage 6 may run as a side goal once stage 2 is done,
because its fixes may touch `crates/nvs-codegen/src/lib.rs`.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. bun nv serve-mem measures memory per recompile and the keep-alive answers under edits, and the tests are written red

**Does:** makes the measurements of § *Why* one command, and writes the tests the checks name.

- [ ] `tools/nv/cmd/serve-mem.ts:1`: from `.agent-tmp/serve-mem/fail.ts` and `ka500.ts`. It copies an app
  (`registry` or `html`) under `.agent-tmp/serve-mem/`, starts `target/release/nvs serve` on it, and
  writes broken, good and restored edits with a pause between them. After each step it prints the
  private bytes (Windows `PrivateMemorySize64`, Linux `RssAnon`) and `MAPPED_JIT_UNITS` if the server
  logs it. `--load <conns>` adds keep-alive load and counts `200`s and `500`s per round. The header
  comment names every flag. It deletes its copy when it ends.
- [ ] Run `bun nv serve-mem --app registry --fails 10 --oks 10` and
  `bun nv serve-mem --app html --load 64 --edits 70` once. Write the figures into the handoff as
  the baseline.
- [ ] `crates/nvs-codegen/src/lib.rs`: the two tests the `pages` check names, in its `mod tests`. Each
  builds a unit through a `#[cfg(test)]` failpoint in `compile_all` that returns a `CodegenError` or
  panics after one function is defined. It reads `MAPPED_JIT_UNITS` before and after. Red until stage 2.
- [ ] `crates/nvs-cli/src/script.rs`: the test the `no-retry` check names, through a `#[cfg(test)]`
  failpoint in `Compiler::compile` (`:1481`) that panics for one content. It counts compiles over three
  `revalidate` passes, then changes the content. Red until stage 3.
- [ ] `crates/nvs-cli/tests/serve_reload.rs:1`: the test the `keep-alive` check names. One keep-alive
  connection sends requests without a pause while the entry is rewritten 60 times, and every answer is
  `200` with the version written last or one before it. Red until stage 4 if it reproduces. If it does
  not reproduce on this machine, the handoff says so, and stage 4 starts from `serve-mem --load`.

### 2. a unit frees its compiled code on every way out of compile

- [ ] `crates/nvs-codegen/src/lib.rs:2500`: `UnitBuilder::new` wraps the module in `JitPages` at once,
  and every use of `self.module` (`:2826`, `:2836`) goes through it. `finish` (`:2825`) moves the
  pages into the class table as now. The `SAFETY` comment at `:520` says why freeing a builder that
  never finished is sound.
- [ ] The `pages` check runs once.

### 3. a compile that panics is recorded as failed until its content changes

- [ ] `crates/nvs-cli/src/script.rs:1001`: the compile runs under `catch_unwind`. A panic becomes
  `CompileState::Failed`, recorded as § *Standing decisions* says. `Landing` (`:463`) and its comment
  say what is now true. The watch thread's own `catch_unwind` (`:1769`) stays for a panic outside a
  compile.
- [ ] The stderr line for a panicked compile, printed once per content, is held to `AGENTS.md`
  § *Text an end user reads*. It says that the compiler stopped on an internal error, names the file,
  and asks the reader to report it.
- [ ] `docs/rules/config/a-broken-edit-fails-the-requests-that-resolve-it.md:1` says that a compile
  that panics is a broken edit for its content. Then `bun nv rules --render`, and
  `bun nv rules --check` is green.
- [ ] The `no-retry` check runs once.

### 4. a busy keep-alive connection keeps serving through many reloads

- [ ] Rerun `bun nv serve-mem --app html --load 64 --edits 70` with `[limits] memory` raised in the
  copy, and read the stop reason the server logs for each `500`. Write in the handoff which path
  answers `500` (`failed`, `crates/nvs-server/src/serve.rs:3499`, and its callers), and why.
- [ ] Fix that cause as § *Standing decisions* says. Name the file and line in the handoff.
- [ ] The `keep-alive` check runs once, and `serve-mem --load` shows no `500`.

### 5. what a successful recompile keeps is measured, bounded and written down

- [ ] Run `bun nv serve-mem --app registry --oks 30` with no requests, and read the live bytes of the
  compile thread and of each core with the runtime's counters. Write in the handoff what grows per
  success, and what holds it.
- [ ] Before stage 6 fixes the panic, write a `yield self::…` call into a generator of the registry
  copy, and let it panic ten times with no requests. Write in the handoff what each panic adds, and
  which threads end (24 became 21 on the small file).
- [ ] Fix what grows with the number of recompiles or panics. A bound of three cached units plus one
  per core stays as it is.
- [ ] The module doc of `crates/nvs-cli/src/serve/mounts.rs` and of `crates/nvs-cli/src/script.rs`
  say what a recompile keeps, per unit and per core, with the measured figures.
- [ ] Rebuild with `bun nv relink --build`, run `bun nv serve-mem --app registry --fails 10 --oks 30`
  once, and write the figures beside the baseline.

### 6. no source text makes the compiler panic, from the parser to code generation

**Does:** fixes the reported panic, and adds a matrix test that puts every expression and statement
kind into every kind of body. The matrix runs in seconds under `cargo test`, and adds no timed search
to any run.

**Split:** after the matrix exists, each failing cell is one item for a writing subagent, one at a time,
because most fixes share `crates/nvs-ir` or `crates/nvs-codegen`.

- [ ] `crates/nvs-ir/src/lower/mod.rs:2982`: a static call through `self::` or `static::` inside a
  generator panics there, because the generator's `advance` function has neither a receiver nor a called
  class. A generator keeps the class it was called on, from its creation, in its state, and `advance`
  reads `self` and `static` from there. The case in § *Why* (`yield self::one();` in a static generator,
  `yield static::one();`, and `yield self::one();` in an instance generator) is the first row the
  matrix proves.
- [ ] `crates/nvs-fuzz-checks/src/compile.rs:1`: `compile(data)` runs a source through parse, check,
  lowering and `nvs_codegen` with the object-file backend, so nothing is mapped or run. It asserts no
  panic, and no `CodegenError` when the check reported no error. The crate takes the dependencies it
  needs. `fuzz/fuzz_targets/compile.rs` is its one-line target, for a search somebody starts by hand.
  `tests.rs` replays `fuzz/seeds/compile/` in `compile_seeds_never_panic`. The reported case is its first
  seed.
- [ ] `every_construct_compiles_in_every_body`, in the same file: a matrix of the body kinds (a
  function, an instance method, a static method, a generator of each, an anonymous function, an arrow
  function, and each of these nested in a generator) times the constructs that read the enclosing
  context (`self::`, `static::` and `parent::` calls and constants, `$this`, a captured variable, `yield`,
  `return`, `throw`, `try`, `match`). Each cell is one small program through `compile`. It is built from
  the grammar, not from the corpus. Its run time goes into the handoff, and is expected under ten
  seconds.
- [ ] Every failing cell is fixed at its cause, and the `no-panic` check runs once. The handoff lists
  each cause in one line.

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-11-17 from a user report and the measurement in § *Why*. The scripts the measurement
used are in `.agent-tmp/serve-mem/`: `fail.ts`, `drive.ts`, `suite.ts`, `load.ts`, `ka500.ts`, with
raw samples in `fail-results.jsonl` and `results.jsonl`. Stage 1 turns them into
`bun nv serve-mem` and then deletes that folder. Nothing is built yet. The person decided the four
fixes and the place before `lsp-memory`. Every other choice in § *Standing decisions* is the authoring
session's call under the priority ordering.

### Next

Stage 1 from its first item: `tools/nv/cmd/serve-mem.ts`, then the four tests, which are expected to
fail until stages 2 to 4.

### Backlog

- A stress test can take most of a host while it runs: in-flight memory is limited by the host's memory
  less 128 MiB and `max_in_flight = 10000`, at `limits.memory = 128M` per request
  (`crates/nvs-server/src/admit.rs`). It is not a leak. Whether the default suits a container with no
  memory limit is a later question.
- The first successful recompile after boot adds about 50 MB on the registry copy, once. Not
  attributed yet.
