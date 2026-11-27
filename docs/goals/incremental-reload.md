---
looper: 1
title: a reload compiles only the functions an edit reached
summary: "Reached when a cache file past 65,535 sections loads warm on Windows, code generation runs on every core below the request workers' priority, each function's machine code is keyed on its own lowered body and every declaration it reads and is stored one file per source file under a small program list, a cold compile generates code once, a development server keeps each file's checked result and each function's code so that an edit inside a method body checks one file and compiles one function, a production server frees both after each compile, and the rules, the bench and the website say so."
context: [crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/cache.rs, docs/rules/config/an-edit-reaches-the-next-request-without-a-restart.md]
stages:
  - id: 1
    title: "a cache file past 65,535 sections loads warm on Windows"
    status: done
    context: [docs/rules/packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header.md, crates/nvs-cli/tests/compile_cache.rs]
  - id: 2
    title: "code generation runs on every core, below the request workers' priority, with the bytes one thread writes"
    status: done
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-host/src/affinity.rs]
  - id: 3
    title: "each function's machine code has a key that covers everything its code reads, and no number baked into it depends on where it sits"
    status: open
    context: [crates/nvs-ir/src/ir.rs, docs/rules/packaging/an-artifact-is-one-immutable-content-addressed-file.md]
  - id: 4
    title: "machine code is stored one file per source file under a program list, and a cold compile generates each new function once"
    status: open
    context: [docs/rules/packaging/a-warm-hit-skips-codegen-not-the-front-end.md, docs/rules/packaging/an-artifact-is-verified-whole-before-a-page-is-executable.md, crates/nvs-cli/tests/compile_cache.rs]
  - id: 5
    title: "a development server keeps each file's checked result and each function's code, and a production server frees both"
    status: open
    context: [crates/nvs-cli/src/script.rs, crates/nvs-cli/src/main.rs, crates/nvs-config/src/cache.rs, docs/rules/config/opcache-revalidation-is-system-class.md, docs/rules/config/a-reload-command-checks-now-and-waits.md, crates/nvs-cli/src/reload.rs]
  - id: 6
    title: "the work of a reload after a body edit is counted and bounded at every program size, and a bench shows its growth"
    status: open
    context: [crates/nvs-cli/tests/start_cost.rs, benches/scaling/README.md]
  - id: 7
    title: "the rules, the reference, the help, the feature proofs and the website say so"
    status: open
    context: [docs/rules/config/a-startup-default-is-never-flipped.md, website/src/data/changes.json]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: section-limit
    stage: 1
    name: "a unit whose object has more than 65,535 sections is stored, loaded warm and run"
    run: [cargo, test, --test, compile_cache]
    tests: [a_unit_past_the_section_limit_loads_warm]
    timeout: 1800
  - id: parallel-codegen
    stage: 2
    name: "code generated on many threads is byte for byte what one thread writes, and the threads run below normal priority"
    run: [cargo, test, --test, parallel_codegen]
    tests: [many_threads_write_the_object_one_thread_writes, a_unit_compiled_on_many_threads_runs_as_one_compiled_on_one, compile_threads_run_below_normal_priority]
    timeout: 1800
  - id: function-keys
    stage: 3
    name: "a body edit moves its own function's key and no other, every declaration a body reads moves the keys of the functions that read it, and a function's bytes do not depend on its place in the program"
    run: [cargo, test, --test, function_keys]
    tests: [a_body_edit_moves_only_its_own_key, every_declaration_a_body_reads_is_in_its_key, a_function_moved_in_the_program_keeps_its_bytes]
    timeout: 1800
  - id: per-file-store
    stage: 4
    name: "an unchanged program compiles no function, a body edit compiles one, a cold compile generates each function once, and a bad file is a miss"
    run: [cargo, test, --test, compile_cache]
    tests: [an_unchanged_program_compiles_no_function, a_body_edit_compiles_one_function, a_cold_compile_generates_each_function_once, a_corrupt_source_file_entry_is_a_miss, eviction_removes_entries_no_program_list_names]
    timeout: 1800
  - id: edit-state
    stage: 5
    name: "a development server checks one file and compiles one function after a body edit, a signature edit rechecks its dependents, and a production server keeps no edit state"
    run: [cargo, test, --test, incremental_reload]
    tests: [a_body_edit_reload_checks_one_file_and_compiles_one_function, a_signature_edit_rechecks_the_files_that_use_it, a_production_server_frees_the_edit_state_after_a_compile, the_reload_record_counts_compiled_and_reused_functions]
    timeout: 1800
  - id: edit-state-default
    stage: 5
    name: "opcache.incremental's startup default follows the mode, and the directive is System"
    run: [cargo, test, --test, incremental_reload]
    tests: [incremental_startup_default_follows_the_mode]
    timeout: 900
  - id: work-is-flat
    stage: 6
    name: "a body edit checks one file and compiles one function at every program size"
    run: [cargo, test, --test, incremental_reload]
    tests: [a_body_edit_costs_the_same_work_at_every_program_size]
    timeout: 1800
  - id: proofs
    stage: 7
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, incremental-reload]
    timeout: 1800
  - id: close
    stage: 8
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, incremental-reload]
    timeout: 900
---

# a reload compiles only the functions an edit reached

## Why

A developer's editor runs `nvs reload` on every save, so the reload is the whole edit loop. Today a
reload after a one-line edit compiles the whole program again, twice. A program of 440 autoloaded
classes with 12 methods each, about 5,300 functions, measured on 2026-11-27 on a 16-core Windows
machine with the release binary:

| | 110 files | 220 files | 440 files | 880 files |
|---|---|---|---|---|
| `nvs reload` after a one-line edit | | | 5.2 s | |
| `nvs run` after a one-line edit | 1.2 s | 2.5 s | 5.1 s | 10.3 s |
| `nvs check` of the same version, CPU | 0.2 s | | 0.42 s | 0.89 s |
| peak memory of the compile | 285 MiB | | 570 MiB | 1.1 GiB |
| cache file per version | 11 MB | 22 MB | 45 MB | 91 MB |
| server memory after load / after three reloads | | | 110 / 240 MiB | |

About 90 % is Cranelift. A unit is the whole program
(`rule:config/an-edit-reaches-the-next-request-without-a-restart`), so every change generates code
for every function. A cold compile walks every function twice: once through the JIT for the run and
once through the object backend for the cache file, on two threads
(`crates/nvs-cli/src/cache.rs:1682`, `compile_beside_object` at `:1749`). Each walk is one thread
over the functions in order (`crates/nvs-codegen/src/lib.rs:2957`). `NOVIS_OPT_LEVEL=none` saves 15 %.
Workers do not compile on their own: one compile per program per process is single-flighted
(`crates/nvs-cli/src/script.rs:120`). `nvs check` warms nothing for the server, because it stops at
the type check.

A second bug was found on the way. On Windows a COFF object counts its sections in 16 bits, and the
object writer emits about 17 sections per function. At 440 files the count wraps (about 91,000
written as 25,574), the loader cannot read the file, and every `nvs run` and every restart compiles
again: a warm `nvs run` of that program takes 15–20 s where the 220-file one takes 0.5 s.

Linear growth at 10× and 100× (see § *How it scales*) rules out every design whose reload cost grows
with the program. So the cost of a reload must grow with the edit, the memory a developer machine
keeps may grow with the program, and a server may take longer per update in exchange for keeping
nothing extra.

## Standing decisions

**The user's calls (2026-11-27):** one goal covering parallel code generation, machine code reused
per function, and front-end reuse; placed on the chain right after the live goal; compile threads on
every core at a priority below the request workers', with no setting; each function keyed on its
own, stored one file per source file; the edit state kept or freed by a new System directive whose
startup default the mode chooses, as `opcache.settle`'s is. Both machines are served: a developer
machine reloads fast after every save, and a server, updated a few times a day, may take a longer
update when that keeps its memory and disk writes down. Every other call here is the session's that
wrote the goal, open until the user changes one.

**Limits: what stops this goal.** Counts decide and never the clock: the proof of every stage is a
count of functions compiled, files checked and files written, and a timing only catches a slowdown
(`rule:programs/less-work-is-taken`). No target figure holds the goal open, including the 200 ms a
developer wants. An item gets one attempt; a stage gets two sessions, counted in `### State`. At the
ceiling the stage's open items move to `### Backlog` as declined, with the count that stopped them.
No item is added to a stage.

**Stage 1: the section limit.** Find what writes about 17 sections per function into the object
(`for_object`, `crates/nvs-codegen/src/lib.rs:2885`; one guess is a section per data object or per
unwind record) and write fewer: every function in one `.text`, every literal in one `.rdata`, as the
two big sections already are. If `cranelift-object` 0.137 cannot do that, write the big-object COFF
form, whose section count is 32 bits, and teach the loader (`crates/nvs-cli/src/cache.rs:576`,
`Layout::of`) to read it. ELF and Mach-O already count past 65,535; the test runs on every platform.
A file the loader cannot read stays a miss, never an error
(`rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`), but it must no longer be written:
the writer refuses to publish an object its own loader would refuse, and that is a test too.
**Built (stage 1):** the sections were `.rdata$.refptr` cells, one per symbol the code reached
through the GOT (every literal and every called function), which the `object` crate's COFF writer
gives a section each. The object backend now compiles with `is_pic` off, as the JIT does: every
reference is an absolute relocation, the object has one `.text` and one `.rdata` at any size, the
file is 27% smaller at 1,000 functions, and a warm hit runs the JIT's instruction shapes. The
loader's landing area (GOT slots, call stubs) is no longer reached by `nvs-codegen`'s payloads and
stays for other forms. `cache::loadable` is the writer's check. `nvs run --count` already prints
`functions=` and `reused=` (whole-unit today: one of them is zero), which the stage 1 test reads.

**Stage 2: code generation on every core.** `emit::emit_function` stays on the compiling thread,
because it writes the unit's literals and declarations. Cranelift's compile of each function's
`Context` (optimisation, register allocation, emission) runs on a scoped pool of
`std::thread::available_parallelism()` threads, which already respects a container's CPU quota.
The compiled bytes are defined into the module in the program's function order with
`define_function_bytes`, so the object is byte for byte the one a single thread writes, and that is
the test. Each compile thread lowers its own priority when it starts: `THREAD_PRIORITY_BELOW_NORMAL`
on Windows, `setpriority` on the thread id on Linux, `QOS_CLASS_UTILITY` on macOS. A server's request
workers keep their CPU, and a compile on an idle machine gets every core. The JIT walk and the object
walk both use the pool. **Cost:** one Cranelift context per thread while a compile runs, freed with
the pool; no thread outlives the compile. The pool is started per compile, not kept, because an idle
server holds no threads (the user's rule that an idle server uses almost nothing).
**Built (stage 2):** `crates/nvs-codegen/src/pool.rs` holds the pool and the priority calls,
because `nvs-codegen` cannot depend on `nvs-host` (that would pull the server's I/O stack into the
compiler). `compile_all` emits each body on the compiling thread (`emit_body`), the pool runs
`compile_body`, and `define_compiled` defines the bytes with `define_function_bytes`, the same
bytes, alignment and relocations `define_function` defines. At most two contexts per thread are
in flight, so the memory does not grow with the program. A one-thread compile (one core, or one
function) takes the old path and starts no thread. The first function that fails in program
order is the error, and a panic on a pool thread resumes on the compiling thread. Linux raises the
thread's nice value by 10 (at most 19). `with_compile_threads` fixes the count for a test only.

**Stage 3: a function's key.** The key is BLAKE3 over the function's lowered IR, every declaration
its emit reads (each class layout, slot, descriptor name, static slot, shape codec, enum, extension
trampoline and callee signature it touches), the environment hash and the optimisation level. The
emit already names every target by symbol (`function_code_hashes`, `crates/nvs-codegen/src/lib.rs:1448`,
compiles one function alone that way); what it does not do yet is record what it read, so the key
is built from a record the emit writes as it reads, never from a list kept beside it. A list beside
it is the failure this stage exists to prevent: a declaration the list forgot reuses stale code,
which is a security bug. The test enumerates every table `UnitTables` exposes and fails naming one
whose change does not move the key of a function that reads it. Today the statement, edge and query
numbers and the function index are baked into each body (`compile_all`, `lib.rs:2959`); they become
per-function bases the linker writes, a relocation or a per-function table entry, whichever the
session measures as fewer instructions on the request path, and the choice goes into the module doc.

**Stage 4: one file per source file, and one code walk.** A source file's entry holds the
relocatable code of the functions it declares, each under its own key, and is keyed by the keys it
holds; the program's entry becomes a list of source-file keys, which replaces today's whole-program
artifact (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file` changes its unit; the
store, its header, its verify and its eviction stay). A cold compile looks up each function by key,
compiles only the misses through the object backend, and places and relocates the whole list. That
removes the JIT walk, and so it reverses the decision at `cache.rs:1682` to pay codegen twice: with
per-function reuse the loader is on every compile anyway, and one walk halves a cold compile's CPU.
`nvs run --count` and `nvs check --count` print `functions=` (compiled) and `reused=` on the
`compile:` line. **Disk:** an edit writes one source file's entry and a new list. **Cost of the
read path:** one open per source file in a process with nothing in memory, measured at 10 µs per
file on a Dev Drive and 33 µs on a drive the antivirus scans.

**Stage 5: the edit state.** A new directive, `[opcache] incremental`, `System`-class, boolean, with
the startup default the mode chooses, exactly as `settle`'s is
(`rule:config/a-startup-default-is-never-flipped`, `crates/nvs-config/src/cache.rs:588`): `true` in
`development`, `false` in `production`. It is not a row of `rule:config/a-mode-is-five-defaults`,
for the reason `settle` is not. With it `true`, the serving process keeps, per program, each source
file's checked result keyed by its content digest and each function's relocatable code, both for the
unit in force only. A reload re-reads and re-checks only the files whose digest moved; when no
declaration's signature moved, nothing else is re-checked, and when one did, the files that use it
are. With it `false`, both are freed when the compile ends, as today. **Memory:** about what
`nvs check` peaks at for the program, plus its machine code; about 200 MiB at 440 files. It is per
program, never per request or per edit, and freed with the unit.

**Not built: shared code between unit versions.** Relinking places the whole program's code again,
29 MB at 440 files and about 2.9 GB at 100× that size, an estimated 1–2 s. Sharing unchanged code
between versions needs calls between functions to go through a per-version table, one more load per
call on the request path. It waits for a measured program that hits it, and the bound goes into
`crates/nvs-cli/src/cache.rs`'s module doc in stage 4.

**Not changed:** `nvs check` does not write machine code or warm a server. A function is not compiled
on its first call: every file a server serves is compiled before its socket is bound, and nothing on
the request path is made on its first use.

## How it scales

Linear from the measured 110–880 files. Front end ≈ `nvs check`.

| | 1× (440 files) | 10× | 100× |
|---|---|---|---|
| today: a reload's compile, CPU | 8.5 s | ~85 s | ~14 min |
| today: cache file written per save | 45 MB | ~450 MB | ~4.5 GB |
| stage 2 only: an update's wall time, 16 cores | ~0.6 s | ~6 s | ~1 min |
| one file per function, read by a fresh process | 5,300 files | 53k | 530k, 5–17 s |
| one file per source file, read by a fresh process | 440 files | 4.4k | 44k, 0.45–1.5 s |
| development edit state kept in memory | ~200 MiB | ~2 GiB | ~20 GiB |

## Stages

### 1. a cache file past 65,535 sections loads warm on Windows

- [x] `crates/nvs-cli/tests/compile_cache.rs`: `a_unit_past_the_section_limit_loads_warm`, generating
  a program whose object passes 65,535 sections, running it twice, and checking the second run's
  provenance is `Loaded`. It is red on Windows before the fix.
- [x] `crates/nvs-codegen/src/lib.rs:2885`: the object writes fewer sections, or the big-object form
  (§ *Stage 1*).
- [x] `crates/nvs-cli/src/cache.rs:1749`: the writer does not publish an object its loader refuses.
- [x] `cargo test --test compile_cache` once.

### 2. code generation runs on every core, below the request workers' priority, with the bytes one thread writes

- [x] `crates/nvs-codegen/src/lib.rs:3097`: `compile_function` splits into the emit, kept on the
  compiling thread, and the Cranelift compile, which returns the bytes and relocations.
- [x] `crates/nvs-codegen/src/lib.rs:2957`: `compile_all` runs the compiles on the scoped pool and
  defines them in function order (§ *Stage 2*).
- [x] The priority call per platform, beside the existing thread placement code in
  `crates/nvs-host/src/affinity.rs` or in `nvs-codegen` if that crate cannot depend on it.
- [x] `crates/nvs-codegen/tests/parallel_codegen.rs` (new): the three tests of the check.

### 3. each function's machine code has a key that covers everything its code reads, and no number baked into it depends on where it sits

**Split:** the key and the numbering touch the same emit sites, so one session per item, in order.

- [ ] `crates/nvs-codegen/src/emit.rs`: every read of a unit table records what it read, and
  `UnitBuilder` turns the record into the function's key (§ *Stage 3*).
- [ ] `crates/nvs-codegen/src/lib.rs:2943` (`Base`): the statement, edge and query bases and the
  function index are bound at link time.
- [ ] `crates/nvs-codegen/tests/function_keys.rs` (new): the three tests of the check.

### 4. machine code is stored one file per source file under a program list, and a cold compile generates each new function once

**Split:** the store and the walk share `cache.rs`, so one session after another.

- [ ] `crates/nvs-cli/src/cache.rs:1652` and `:1695`: `unit_key` and `unit_for` read and write the
  program list and the source-file entries (§ *Stage 4*).
- [ ] `crates/nvs-cli/src/cache.rs:1749`: `compile_beside_object` is gone; a miss compiles objects
  only, and the unit is placed from the list.
- [ ] `crates/nvs-cli/src/cache.rs:1386`: eviction removes an entry no program list names, on the
  roll it already rides.
- [ ] The `compile:` line's `functions=` and `reused=` (`crates/nvs-cli/src/main.rs:1051`).
- [ ] The five tests of the check `per-file-store`.

### 5. a development server keeps each file's checked result and each function's code, and a production server frees both

- [ ] `crates/nvs-config/src/cache.rs:588`: `incremental` with its startup row, and
  `incremental_startup_default_follows_the_mode` in `crates/nvs-cli/tests/incremental_reload.rs`,
  read through `nvs config dump` in each mode.
- [ ] `crates/nvs-cli/src/main.rs:1287` and `crates/nvs-cli/src/script.rs:1780`: the front end takes
  the program's kept file results and re-reads and re-checks only the moved files (§ *Stage 5*).
- [ ] `crates/nvs-cli/src/script.rs:1831`: the compile takes the kept function code before the disk.
- [ ] `crates/nvs-cli/src/reload.rs:1042`: the `program reloaded` record carries `compiled` and
  `reused` function counts.
- [ ] `crates/nvs-cli/tests/incremental_reload.rs` (new): the four tests of the check `edit-state`.

### 6. the work of a reload after a body edit is counted and bounded at every program size, and a bench shows its growth

- [ ] `crates/nvs-cli/tests/incremental_reload.rs`: `a_body_edit_costs_the_same_work_at_every_program_size`,
  over 110, 220 and 440 generated files: one file checked, one function compiled, one source-file
  entry written.
- [ ] A bench of `nvs reload` after a one-line edit at those sizes, where `benches/scaling/README.md`
  says a server bench lives, with its growth recorded.

### 7. the rules, the reference, the help, the feature proofs and the website say so

- [ ] `docs/rules/config/an-edit-reaches-the-next-request-without-a-restart.md`,
  `docs/rules/packaging/an-artifact-is-one-immutable-content-addressed-file.md`,
  `docs/rules/packaging/a-warm-hit-skips-codegen-not-the-front-end.md` and
  `docs/rules/config/opcache-revalidation-is-system-class.md`: what each now says, with the
  decisions above, their reasons and their Declined lists (lazy compile on first call, one file per
  function, shared code between versions for now).
- [ ] The help of `[opcache] incremental` and of `nvs reload`, and the website's development and
  deploy pages.
- [ ] The feature proofs `bun nv proofs --owed --goal incremental-reload` names.
- [ ] `website/src/data/changes.json:1`: the entries, then `bun nv site --check stale`, `changes` and
  `review`.

### 8. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done, each in one session. Stage 2: code generation runs on
`crates/nvs-codegen/src/pool.rs`'s scoped pool, below normal priority, and defines each function
in program order (§ *Standing decisions*, *Built (stage 2)*). `cargo test --test parallel_codegen`
passes its three tests on Windows and in WSL. The failpoint unit tests in `nvs-codegen` now run on
one thread and on four, and pass. The macOS priority code was not compiled here; CI's macOS job is
its first build.

The nightly's nine failures (run 45813627231) were fixed on `main` before stage 1:
`c115a735a1`, `28a4d7a6bf` and `544711a8fb`.

Tests that may go red elsewhere: none expected. A compile now starts threads, so a test that
counts the process's threads during a compile sees more of them.

### Next

Stage 3 from its first item: every read of a unit table in `crates/nvs-codegen/src/emit.rs`
records what it read (`emit::UnitTables`, built in `emit_body`, `crates/nvs-codegen/src/lib.rs:3263`).
The second item is the bases in `Base` (`lib.rs:2943`), passed to `emit_body`. Stage 4's `--count`
item only needs `nvs check --count` and the per-function counts; `with_codegen` in
`crates/nvs-cli/src/main.rs` is the place.

### Backlog` as declined, with the count that stopped them.
No item is added to a stage.

**Stage 1: the section limit.** Find what writes about 17 sections per function into the object
(`for_object`, `crates/nvs-codegen/src/lib.rs:2885`; one guess is a section per data object or per
unwind record) and write fewer: every function in one `.text`, every literal in one `.rdata`, as the
two big sections already are. If `cranelift-object` 0.137 cannot do that, write the big-object COFF
form, whose section count is 32 bits, and teach the loader (`crates/nvs-cli/src/cache.rs:576`,
`Layout::of`) to read it. ELF and Mach-O already count past 65,535; the test runs on every platform.
A file the loader cannot read stays a miss, never an error
(`rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`), but it must no longer be written:
the writer refuses to publish an object its own loader would refuse, and that is a test too.
**Built (stage 1):** the sections were `.rdata$.refptr` cells, one per symbol the code reached
through the GOT (every literal and every called function), which the `object` crate's COFF writer
gives a section each. The object backend now compiles with `is_pic` off, as the JIT does: every
reference is an absolute relocation, the object has one `.text` and one `.rdata` at any size, the
file is 27% smaller at 1,000 functions, and a warm hit runs the JIT's instruction shapes. The
loader's landing area (GOT slots, call stubs) is no longer reached by `nvs-codegen`'s payloads and
stays for other forms. `cache::loadable` is the writer's check. `nvs run --count` already prints
`functions=` and `reused=` (whole-unit today: one of them is zero), which the stage 1 test reads.

**Stage 2: code generation on every core.** `emit::emit_function` stays on the compiling thread,
because it writes the unit's literals and declarations. Cranelift's compile of each function's
`Context` (optimisation, register allocation, emission) runs on a scoped pool of
`std::thread::available_parallelism()` threads, which already respects a container's CPU quota.
The compiled bytes are defined into the module in the program's function order with
`define_function_bytes`, so the object is byte for byte the one a single thread writes, and that is
the test. Each compile thread lowers its own priority when it starts: `THREAD_PRIORITY_BELOW_NORMAL`
on Windows, `setpriority` on the thread id on Linux, `QOS_CLASS_UTILITY` on macOS. A server's request
workers keep their CPU, and a compile on an idle machine gets every core. The JIT walk and the object
walk both use the pool. **Cost:** one Cranelift context per thread while a compile runs, freed with
the pool; no thread outlives the compile. The pool is started per compile, not kept, because an idle
server holds no threads (the user's rule that an idle server uses almost nothing).
**Built (stage 2):** `crates/nvs-codegen/src/pool.rs` holds the pool and the priority calls,
because `nvs-codegen` cannot depend on `nvs-host` (that would pull the server's I/O stack into the
compiler). `compile_all` emits each body on the compiling thread (`emit_body`), the pool runs
`compile_body`, and `define_compiled` defines the bytes with `define_function_bytes`, the same
bytes, alignment and relocations `define_function` defines. At most two contexts per thread are
in flight, so the memory does not grow with the program. A one-thread compile (one core, or one
function) takes the old path and starts no thread. The first function that fails in program
order is the error, and a panic on a pool thread resumes on the compiling thread. Linux raises the
thread's nice value by 10 (at most 19). `with_compile_threads` fixes the count for a test only.

**Stage 3: a function's key.** The key is BLAKE3 over the function's lowered IR, every declaration
its emit reads (each class layout, slot, descriptor name, static slot, shape codec, enum, extension
trampoline and callee signature it touches), the environment hash and the optimisation level. The
emit already names every target by symbol (`function_code_hashes`, `crates/nvs-codegen/src/lib.rs:1448`,
compiles one function alone that way); what it does not do yet is record what it read, so the key
is built from a record the emit writes as it reads, never from a list kept beside it. A list beside
it is the failure this stage exists to prevent: a declaration the list forgot reuses stale code,
which is a security bug. The test enumerates every table `UnitTables` exposes and fails naming one
whose change does not move the key of a function that reads it. Today the statement, edge and query
numbers and the function index are baked into each body (`compile_all`, `lib.rs:2959`); they become
per-function bases the linker writes, a relocation or a per-function table entry, whichever the
session measures as fewer instructions on the request path, and the choice goes into the module doc.

**Stage 4: one file per source file, and one code walk.** A source file's entry holds the
relocatable code of the functions it declares, each under its own key, and is keyed by the keys it
holds; the program's entry becomes a list of source-file keys, which replaces today's whole-program
artifact (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file` changes its unit; the
store, its header, its verify and its eviction stay). A cold compile looks up each function by key,
compiles only the misses through the object backend, and places and relocates the whole list. That
removes the JIT walk, and so it reverses the decision at `cache.rs:1682` to pay codegen twice: with
per-function reuse the loader is on every compile anyway, and one walk halves a cold compile's CPU.
`nvs run --count` and `nvs check --count` print `functions=` (compiled) and `reused=` on the
`compile:` line. **Disk:** an edit writes one source file's entry and a new list. **Cost of the
read path:** one open per source file in a process with nothing in memory, measured at 10 µs per
file on a Dev Drive and 33 µs on a drive the antivirus scans.

**Stage 5: the edit state.** A new directive, `[opcache] incremental`, `System`-class, boolean, with
the startup default the mode chooses, exactly as `settle`'s is
(`rule:config/a-startup-default-is-never-flipped`, `crates/nvs-config/src/cache.rs:588`): `true` in
`development`, `false` in `production`. It is not a row of `rule:config/a-mode-is-five-defaults`,
for the reason `settle` is not. With it `true`, the serving process keeps, per program, each source
file's checked result keyed by its content digest and each function's relocatable code, both for the
unit in force only. A reload re-reads and re-checks only the files whose digest moved; when no
declaration's signature moved, nothing else is re-checked, and when one did, the files that use it
are. With it `false`, both are freed when the compile ends, as today. **Memory:** about what
`nvs check` peaks at for the program, plus its machine code; about 200 MiB at 440 files. It is per
program, never per request or per edit, and freed with the unit.

**Not built: shared code between unit versions.** Relinking places the whole program's code again,
29 MB at 440 files and about 2.9 GB at 100× that size, an estimated 1–2 s. Sharing unchanged code
between versions needs calls between functions to go through a per-version table, one more load per
call on the request path. It waits for a measured program that hits it, and the bound goes into
`crates/nvs-cli/src/cache.rs`'s module doc in stage 4.

**Not changed:** `nvs check` does not write machine code or warm a server. A function is not compiled
on its first call: every file a server serves is compiled before its socket is bound, and nothing on
the request path is made on its first use.

## How it scales

Linear from the measured 110–880 files. Front end ≈ `nvs check`.

| | 1× (440 files) | 10× | 100× |
|---|---|---|---|
| today: a reload's compile, CPU | 8.5 s | ~85 s | ~14 min |
| today: cache file written per save | 45 MB | ~450 MB | ~4.5 GB |
| stage 2 only: an update's wall time, 16 cores | ~0.6 s | ~6 s | ~1 min |
| one file per function, read by a fresh process | 5,300 files | 53k | 530k, 5–17 s |
| one file per source file, read by a fresh process | 440 files | 4.4k | 44k, 0.45–1.5 s |
| development edit state kept in memory | ~200 MiB | ~2 GiB | ~20 GiB |

## Stages

### 1. a cache file past 65,535 sections loads warm on Windows

- [x] `crates/nvs-cli/tests/compile_cache.rs`: `a_unit_past_the_section_limit_loads_warm`, generating
  a program whose object passes 65,535 sections, running it twice, and checking the second run's
  provenance is `Loaded`. It is red on Windows before the fix.
- [x] `crates/nvs-codegen/src/lib.rs:2885`: the object writes fewer sections, or the big-object form
  (§ *Stage 1*).
- [x] `crates/nvs-cli/src/cache.rs:1749`: the writer does not publish an object its loader refuses.
- [x] `cargo test --test compile_cache` once.

### 2. code generation runs on every core, below the request workers' priority, with the bytes one thread writes

- [x] `crates/nvs-codegen/src/lib.rs:3097`: `compile_function` splits into the emit, kept on the
  compiling thread, and the Cranelift compile, which returns the bytes and relocations.
- [x] `crates/nvs-codegen/src/lib.rs:2957`: `compile_all` runs the compiles on the scoped pool and
  defines them in function order (§ *Stage 2*).
- [x] The priority call per platform, beside the existing thread placement code in
  `crates/nvs-host/src/affinity.rs` or in `nvs-codegen` if that crate cannot depend on it.
- [x] `crates/nvs-codegen/tests/parallel_codegen.rs` (new): the three tests of the check.

### 3. each function's machine code has a key that covers everything its code reads, and no number baked into it depends on where it sits

**Split:** the key and the numbering touch the same emit sites, so one session per item, in order.

- [ ] `crates/nvs-codegen/src/emit.rs`: every read of a unit table records what it read, and
  `UnitBuilder` turns the record into the function's key (§ *Stage 3*).
- [ ] `crates/nvs-codegen/src/lib.rs:2943` (`Base`): the statement, edge and query bases and the
  function index are bound at link time.
- [ ] `crates/nvs-codegen/tests/function_keys.rs` (new): the three tests of the check.

### 4. machine code is stored one file per source file under a program list, and a cold compile generates each new function once

**Split:** the store and the walk share `cache.rs`, so one session after another.

- [ ] `crates/nvs-cli/src/cache.rs:1652` and `:1695`: `unit_key` and `unit_for` read and write the
  program list and the source-file entries (§ *Stage 4*).
- [ ] `crates/nvs-cli/src/cache.rs:1749`: `compile_beside_object` is gone; a miss compiles objects
  only, and the unit is placed from the list.
- [ ] `crates/nvs-cli/src/cache.rs:1386`: eviction removes an entry no program list names, on the
  roll it already rides.
- [ ] The `compile:` line's `functions=` and `reused=` (`crates/nvs-cli/src/main.rs:1051`).
- [ ] The five tests of the check `per-file-store`.

### 5. a development server keeps each file's checked result and each function's code, and a production server frees both

- [ ] `crates/nvs-config/src/cache.rs:588`: `incremental` with its startup row, and
  `incremental_startup_default_follows_the_mode` in `crates/nvs-cli/tests/incremental_reload.rs`,
  read through `nvs config dump` in each mode.
- [ ] `crates/nvs-cli/src/main.rs:1287` and `crates/nvs-cli/src/script.rs:1780`: the front end takes
  the program's kept file results and re-reads and re-checks only the moved files (§ *Stage 5*).
- [ ] `crates/nvs-cli/src/script.rs:1831`: the compile takes the kept function code before the disk.
- [ ] `crates/nvs-cli/src/reload.rs:1042`: the `program reloaded` record carries `compiled` and
  `reused` function counts.
- [ ] `crates/nvs-cli/tests/incremental_reload.rs` (new): the four tests of the check `edit-state`.

### 6. the work of a reload after a body edit is counted and bounded at every program size, and a bench shows its growth

- [ ] `crates/nvs-cli/tests/incremental_reload.rs`: `a_body_edit_costs_the_same_work_at_every_program_size`,
  over 110, 220 and 440 generated files: one file checked, one function compiled, one source-file
  entry written.
- [ ] A bench of `nvs reload` after a one-line edit at those sizes, where `benches/scaling/README.md`
  says a server bench lives, with its growth recorded.

### 7. the rules, the reference, the help, the feature proofs and the website say so

- [ ] `docs/rules/config/an-edit-reaches-the-next-request-without-a-restart.md`,
  `docs/rules/packaging/an-artifact-is-one-immutable-content-addressed-file.md`,
  `docs/rules/packaging/a-warm-hit-skips-codegen-not-the-front-end.md` and
  `docs/rules/config/opcache-revalidation-is-system-class.md`: what each now says, with the
  decisions above, their reasons and their Declined lists (lazy compile on first call, one file per
  function, shared code between versions for now).
- [ ] The help of `[opcache] incremental` and of `nvs reload`, and the website's development and
  deploy pages.
- [ ] The feature proofs `bun nv proofs --owed --goal incremental-reload` names.
- [ ] `website/src/data/changes.json:1`: the entries, then `bun nv site --check stale`, `changes` and
  `review`.

### 8. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stage 1 is done in one session (1 of 2). The object backend compiles with `is_pic` off, so a
COFF object has two sections at any size (§ *Standing decisions*, *Built (stage 1)*). The writer
publishes only what `cache::loadable` accepts. `nvs run --count` prints `functions=` and `reused=`
at the end of the `compile:` line, before a debug build's `phases:` line, and the line is now
printed after code generation. `a_unit_past_the_section_limit_loads_warm` (8,000 functions, about
20 s in a debug build) and `cache::tests::the_writer_publishes_only_what_its_loader_reads` pass on
Windows, and `cargo test --test compile_cache` is green.

The nightly's nine failures (run 45813627231) were fixed on `main` before this session:
`c115a735a1` (zone-database tests skip Miri), `28a4d7a6bf` (`Core\Uuid` is `Parses` at run time)
and `544711a8fb` (the serve stack-page bound).

Tests that may go red elsewhere: none expected. A test that reads the `compile:` line of `nvs run
--count` sees two more fields, and a run that fails between lowering and code generation no
longer prints that line.

### Next

Stage 2 from its first item: `compile_function` (`crates/nvs-codegen/src/lib.rs:3097`) splits into
the emit and the Cranelift compile. Stage 4's `--count` item only needs `nvs check --count` and
the per-function counts now; `with_codegen` in `crates/nvs-cli/src/main.rs` is the place.

### Backlog
