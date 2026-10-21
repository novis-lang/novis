---
looper: 1
title: "Every unsafe reason is true, and the standard library walks an array in one place"
summary: "The unsafe roster names each crate's real reason, no standard library member walks an array by hand, the touched members' benches are not slower, and the checks below pass."
context: [tools/nv/cmd/lints.ts]
stages:
  - id: 1
    title: "the unsafe roster states each crate's real reason"
    status: done
    context: [tools/nv/cmd/lints.ts, crates/nvs-config/src/trust.rs, crates/nvs-cli/src/stop.rs]
  - id: 2
    title: "the standard library walks an array through one iterator"
    status: open
    context: [crates/nvs-stdlib/src/str.rs, crates/nvs-stdlib/src/http.rs, crates/nvs-stdlib/src/process.rs, crates/nvs-stdlib/src/bytes.rs, crates/nvs-stdlib/src/mail.rs, crates/nvs-stdlib/src/script.rs]
checks:
  - id: lints
    stage: 1
    name: every unsafe crate's lint table matches the roster
    run: [bun, nv, lints, --check]
---

# Every unsafe reason is true, and the standard library walks an array in one place

## Why

Seven shipped crates use `unsafe`, and `UNSAFE_CRATES` in `tools/nv/cmd/lints.ts:60` is the one
home for why. A review of every block found the reasons sound and well checked: each block has its
own `#[expect(unsafe_code, reason = …)]`, and CI runs Miri, ASAN, TSAN and the fuzzers over them.
Two things were wrong. The roster gives `nvs-config` and `nvs-cli` a reason that is not true, and
the standard library writes the same array walk by hand in twelve places, each with its own
`unsafe` block and a copied reason. This goal fixes both. It does not aim at a lower `unsafe` count.

## Standing decisions

**The scope.** Tested `unsafe` code is good code. Nothing moves into a wrapper crate such as `nix` or
`windows`, because that hides the `unsafe` without removing it. No `undocumented_unsafe_blocks` lint,
because the reason is already stated once per function. The helper ABI between compiled code and the
runtime keeps its raw pointers, because a safe layer there costs time on every call on the request
path.

**Stage 2 is only worth it if it costs nothing.** The iterator makes the same two runtime calls the
hand-written loops make, so it should compile to the same code. The proof is the member ledger
(`rule:testing/member-perf-ledger`): `bun nv proofs --record-perf --group '<group>'` over each group
stage 2 reaches. A bench whose count rises above its previous record, or that misses its declaration,
has its slice reverted, not tuned. The clock is not judged, because the ledger refuses a clock delta
across machines.

**Lanes.** Stage 1 edits `tools/nv/cmd/lints.ts` and reads the unsafe crates. Stage 2 edits
`crates/nvs-stdlib/src/` only. They share no file and run in either order. The whole goal touches no
file `docs/goals/release-hardening.md` names, so it can run as a side goal beside the chain. No stage waits on
another goal.

**Trade-offs.** None for a program: no syntax, message or result changes. Stage 2 holds no new
memory: an iterator is an array pointer and a cursor on the stack. Simplicity of the implementation
gains: the cursor logic and its safety reason exist once.

## Stages

### 1. the unsafe roster states each crate's real reason

**Does:** every entry of `UNSAFE_CRATES` names what that crate's `unsafe` blocks do today.

**Split:** five `Explore` subagents go out together, one per crate of the third item (`nvs-runtime`,
`nvs-stdlib`, `nvs-codegen`, `nvs-host`, `nvs-ext`). Each returns that crate's
`#[expect(unsafe_code, reason = …)]` sites as `file.rs:NN` and the reason in one line. The session
writes `lints.ts`.

- [x] `nvs-config`'s entry names the config file permission check (`geteuid` on Unix, the file's ACL through `advapi32` on Windows), the physical memory reading, and the Windows directory created with a security descriptor. `tools/nv/cmd/lints.ts:67`, `crates/nvs-config/src/trust.rs:180`, `crates/nvs-config/src/server.rs:974`, `crates/nvs-config/src/data.rs:545`
- [x] `nvs-cli`'s entry names signal handling, Windows service registration and dispatch, the account lookup, the socket `umask`, and the cached code pointer it makes callable. `tools/nv/cmd/lints.ts:68`, `crates/nvs-cli/src/stop.rs:144`, `crates/nvs-cli/src/service.rs:1691`, `crates/nvs-cli/src/dispatch.rs:377`, `crates/nvs-cli/src/serve.rs:2655`, `crates/nvs-cli/src/cache.rs:1097`
- [x] The other five entries are read against their crate's `#[expect(unsafe_code)]` reasons, and an entry that names less than the crate does is completed. `nvs-host`'s entry does not yet name the CPU clock reads or the ThreadSanitizer hooks. `tools/nv/cmd/lints.ts:61`, `tools/nv/cmd/lints.ts:66`, `tools/nv/cmd/lints.ts:69`, `crates/nvs-host/src/cpuclock.rs:150`, `crates/nvs-host/src/tsan.rs:87`

### 2. the standard library walks an array through one iterator

**Does:** no `Core` member calls `nvs_array_next_slot` itself. `crate::str::Elements`
(`crates/nvs-stdlib/src/str.rs:1852`) is the one walk over values, a keyed twin beside it is the one
walk over keys and values, and each states its safety reason once.

**Split:** at stage start, an `Explore` subagent maps each of the twelve walk sites below to the
`Core` member that calls it and that member's bench under `benches/members/core/`, one line each.
That list is the last item's set of groups.

- [ ] A keyed iterator beside `Elements` gives each slot's key and value, borrowed as `Elements` borrows them. `crates/nvs-stdlib/src/str.rs:1870`
- [ ] The eight value-only walks use `Elements`. `crates/nvs-stdlib/src/bytes.rs:1086`, `crates/nvs-stdlib/src/bytes.rs:1498`, `crates/nvs-stdlib/src/http.rs:2970`, `crates/nvs-stdlib/src/mail.rs:513`, `crates/nvs-stdlib/src/process.rs:765`, `crates/nvs-stdlib/src/script.rs:822`, `crates/nvs-stdlib/src/str.rs:3974`, `crates/nvs-stdlib/src/str.rs:4039`
- [ ] The four keyed walks use the keyed iterator. `crates/nvs-stdlib/src/http.rs:3141`, `crates/nvs-stdlib/src/http.rs:3702`, `crates/nvs-stdlib/src/process.rs:886`, `crates/nvs-stdlib/src/str.rs:2826`
- [ ] A walk that also needs the slot number, or stops early, keeps that behaviour: `enumerate` or `take_while` over the iterator, never a second cursor. `crates/nvs-stdlib/src/str.rs:1870`
- [ ] `bun nv proofs --record-perf --group` over `Core\Str`, `Core\Bytes`, `Core\Http`, `Core\Mail`, `Core\Process` and `Core\Script` records no count above its previous record (`rule:testing/member-perf-ledger`). `data/perf/members.ndjson:1`

## Handoff

### State

Stage 1 is done. Every `UNSAFE_CRATES` entry in `tools/nv/cmd/lints.ts:60` was rewritten from that
crate's `#[expect(unsafe_code)]` sites. Two old claims were false: `nvs-runtime` has no request arena
(`crates/nvs-runtime/src/lib.rs:184` says it is refused), and `nvs-host` sets no CPU affinity. The
added uses are the global allocator in `nvs-runtime`, `Send`/`Sync` for a unit in `nvs-codegen`, the
task group's reference releases in `nvs-host`, and the monotonic clock in `nvs-cli`. `nvs-ext` and
`benches/abi-probe` were already true.

The nightly CI failures the sweep named were all on `6544156`, 89 commits behind this stage's head,
and none reproduces there. `bun nv proofs --gate` owes nothing, and `bun nv render --website --check`
is current. `miri` timed out at 45 minutes as one job, and `51e0534f6` already split it into two parts.

### Next

**Stage 2: one array walk**, on `crates/nvs-stdlib/src/` only. Start with the stage's `Explore`
subagent. The stage 1 survey found 18 `nvs_array_next_slot` or `nvs_array_count` sites, not twelve.
The six beyond the stage's list are `crates/nvs-stdlib/src/arr.rs:2288`, `arr.rs:2317`,
`db/open.rs:1132`, `os.rs:400`, `test.rs:3493` and `str.rs:1874`. `str.rs:1874` is `Elements`
itself. The subagent says which of the other five are walks and which only count.

- [ ] The keyed iterator beside `Elements`. `crates/nvs-stdlib/src/str.rs:1852`
- [ ] The value-only walks use `Elements`. `crates/nvs-stdlib/src/bytes.rs:1086`
- [ ] The keyed walks use the keyed iterator. `crates/nvs-stdlib/src/http.rs:3141`
- [ ] The member ledger over each group reached. `data/perf/members.ndjson:1`

### Backlog

- The Windows-only `unsafe` code in `nvs-config` and `nvs-cli` runs under no sanitizer, because every sanitizer job runs on Linux. Accepted for startup and install code; reopen if that code grows. `docs/goals/unsafe-reasons.md`
