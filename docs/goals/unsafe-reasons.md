---
looper: 1
title: "Every unsafe reason is true, and the standard library walks an array in one place"
summary: "The unsafe roster names each crate's real reason, no standard library member walks an array by hand, the touched members' benches are not slower, and the checks below pass."
context: [tools/nv/cmd/lints.ts]
stages:
  - id: 1
    title: "the unsafe roster states each crate's real reason"
    status: open
    context: [tools/nv/cmd/lints.ts, crates/nvs-config/src/trust.rs, crates/nvs-cli/src/stop.rs]
  - id: 2
    title: "the standard library walks an array through one iterator"
    status: open
    context: [crates/nvs-stdlib/src/str.rs, crates/nvs-stdlib/src/http.rs, crates/nvs-stdlib/src/process.rs, crates/nvs-stdlib/src/bytes.rs]
checks:
  - id: lints
    stage: 1
    name: every unsafe crate's lint table matches the roster
    run: [bun, nv, lints, --check]
---

# Every unsafe reason is true, and the standard library walks an array in one place

## Why

Seven shipped crates use `unsafe`, and `UNSAFE_CRATES` in `tools/nv/cmd/lints.ts:53` is the one
home for why. A review of every block found the reasons sound and well checked: each block has its
own `#[expect(unsafe_code, reason = …)]`, and CI runs Miri, ASAN, TSAN and the fuzzers over them.
Two things were wrong. The roster gives `nvs-config` and `nvs-cli` a reason that is not true, and
the standard library writes the same array walk by hand in twelve places, each with its own
`unsafe` block and a copied reason. This goal fixes both. It does not aim at a lower `unsafe` count.

## Standing decisions

**The user decided the scope.** Tested `unsafe` code is good code. Nothing moves into a wrapper
crate such as `nix` or `windows`, because that hides the `unsafe` without removing it. No
`undocumented_unsafe_blocks` lint, because the reason is already stated once per function. The
helper ABI between compiled code and the runtime keeps its raw pointers, because a safe layer there
costs time on every call on the request path.

**Stage 2 is only worth it if it costs nothing.** The iterator makes the same two runtime calls the
hand-written loops make, so it should compile to the same code. If a member bench that stage 2
reaches turns red in the loop's bench turn, the slice is reverted, not tuned.

**Trade-offs.** None for a program: no syntax, message or result changes. Stage 2 holds no new
memory: an iterator is an array pointer and a cursor on the stack. Simplicity of the implementation
gains: the cursor logic and its safety reason exist once.

## Stages

### 1. the unsafe roster states each crate's real reason

**Does:** every entry of `UNSAFE_CRATES` names what that crate's `unsafe` blocks do today.

- [ ] `nvs-config`'s entry names the config file permission check (`geteuid` on Unix, the file's ACL through `advapi32` on Windows), the physical memory reading, and the Windows directory created with a security descriptor. `tools/nv/cmd/lints.ts:60`, `crates/nvs-config/src/trust.rs:180`, `crates/nvs-config/src/server.rs:972`, `crates/nvs-config/src/data.rs:595`
- [ ] `nvs-cli`'s entry names signal handling, Windows service registration and dispatch, the account lookup, the socket `umask`, and the cached code pointer it makes callable. `tools/nv/cmd/lints.ts:61`, `crates/nvs-cli/src/stop.rs:144`, `crates/nvs-cli/src/service.rs:2889`, `crates/nvs-cli/src/dispatch.rs:377`, `crates/nvs-cli/src/serve.rs:2569`, `crates/nvs-cli/src/cache.rs:1097`
- [ ] The other five entries are read against their crate's `#[expect(unsafe_code)]` reasons, and an entry that names less than the crate does is completed. `nvs-host`'s entry does not yet name the CPU clock reads or the ThreadSanitizer hooks. `tools/nv/cmd/lints.ts:59`, `crates/nvs-host/src/cpuclock.rs:150`, `crates/nvs-host/src/tsan.rs:87`
- [ ] `bun nv lints --check` passes.

### 2. the standard library walks an array through one iterator

**Does:** no `Core` member calls `nvs_array_next_slot` itself. `crate::str::Elements`
(`crates/nvs-stdlib/src/str.rs:1845`) is the one walk over values, a keyed twin beside it is the one
walk over keys and values, and each states its safety reason once.

- [ ] A keyed iterator beside `Elements` gives each slot's key and value, borrowed as `Elements` borrows them. `crates/nvs-stdlib/src/str.rs:1863`
- [ ] The eight value-only walks use `Elements`. `crates/nvs-stdlib/src/bytes.rs:1089`, `crates/nvs-stdlib/src/bytes.rs:1502`, `crates/nvs-stdlib/src/http.rs:2927`, `crates/nvs-stdlib/src/mail.rs:507`, `crates/nvs-stdlib/src/process.rs:754`, `crates/nvs-stdlib/src/script.rs:810`, `crates/nvs-stdlib/src/str.rs:3970`, `crates/nvs-stdlib/src/str.rs:4035`
- [ ] The four keyed walks use the keyed iterator. `crates/nvs-stdlib/src/http.rs:3098`, `crates/nvs-stdlib/src/http.rs:3659`, `crates/nvs-stdlib/src/process.rs:875`, `crates/nvs-stdlib/src/str.rs:2820`
- [ ] A walk that also needs the slot number, or stops early, keeps that behaviour: `enumerate` or `take_while` over the iterator, never a second cursor.
- [ ] The loop's bench turn shows no member this stage reaches red.

## Handoff

### State

The goal is new. A read of every `unsafe` block in the seven shipped crates found the two problems
the stages name, and nothing else worth changing. The anchors above were current when it was
written.

### Next

Stage 1, the roster, on the file set of `tools/nv/cmd/lints.ts` and the crates it names:
- `nvs-config`'s and `nvs-cli`'s entries (`tools/nv/cmd/lints.ts:60`)
- the other five entries against their crates (`tools/nv/cmd/lints.ts:54`)

Stage 2 follows in its own session, on `crates/nvs-stdlib/src/`.

### Backlog

- The Windows-only `unsafe` code in `nvs-config` and `nvs-cli` runs under no sanitizer, because every sanitizer job runs on Linux. Accepted for startup and install code; reopen if that code grows. `docs/goals/unsafe-reasons.md`
