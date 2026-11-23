---
looper: 1
title: "Windows builds link with rust-lld, and the shipped binary keeps its protections"
summary: "Reached when bun nv pe-flags checks that nvs.exe has ASLR, high-entropy addresses, NX, its 8 MiB stack and its version resource, the Windows build leg runs it on every build, a measured local rebuild decides whether rust-lld links the workspace faster than link.exe, and when it does .cargo/config.toml names rust-lld for x86_64-pc-windows-msvc, a panic still names its file and line, the forced unwind test passes on the proof profile, and a green push run records the new Windows build times."
context: [crates/nvs-cli/build.rs, rust-toolchain.toml]
stages:
  - id: 1
    title: "bun nv pe-flags checks the protections of a Windows binary, and every Windows build runs it"
    status: done
    context: [.github/workflows/build.yml, tools/nv/main.ts]
  - id: 2
    title: a local rebuild measures link.exe against rust-lld
    status: done
    context: [docs/agent/commands.md]
  - id: 3
    title: "rust-lld links every Windows build, and nothing it links behaves differently"
    status: done
    context: [.github/workflows/ci.yml, crates/nvs-host/src/scheduler.rs]
  - id: 4
    title: "the change is pushed, its run is green, and the Windows build times are recorded"
    status: done
    context: [docs/goals/ci-green.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: pe-flags
    stage: 1
    name: "bun nv pe-flags passes a PE32+ image with every protection and names each one a header lacks"
    run: [bun, test, tools/nv/test/pe-flags.test.ts]
    timeout: 300
  - id: lld-green
    stage: 4
    name: "origin/main holds the linker change, and its push runs of CI and Website ended green"
    run: [bun, nv, ci, --green]
    timeout: 300
  - id: close
    stage: 5
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, windows-rust-lld]
    timeout: 900
---

# Windows builds link with rust-lld, and the shipped binary keeps its protections

## Why

Every Windows build links with MSVC's `link.exe`, because the tree names no linker: there is no
`.cargo/config.toml`. A Windows `tests` leg of CI spends 21 minutes in `cargo build --workspace
--all-targets` (run 44940832132), and much of that is linking. The tree has about 320 integration
test files, and each one links as a binary of its own that carries most of the compiler. The same
links run on this machine in every session's `cargo build`.

The toolchain already ships LLVM's linker as `rust-lld.exe`
(`<sysroot>/lib/rustlib/x86_64-pc-windows-msvc/bin/`). It reads the same inputs and is usually much
faster on large binaries. Nothing here is measured yet, so stage 2 measures it before anything is
changed.

The shipped `nvs.exe` must not lose a protection to a linker change. No check reads its headers
today. So stage 1 adds one, which is worth having whatever the linker: ASLR (`DYNAMIC_BASE`),
high-entropy addresses (`HIGH_ENTROPY_VA`), `NX_COMPAT`, the 8 MiB main-thread stack that
`crates/nvs-cli/build.rs:89` asks for, and the version resource `crates/nvs-cli/build.rs:168` links
(`rule:packaging/the-windows-binary-says-what-it-is`).

The user's call: a time saving with no trade-off beyond one-time work is taken. This goal moves no
feature, so it owes the website nothing.

## Standing decisions

**The protection check comes first and stays, whatever stage 2 measures.** `bun nv pe-flags --bin
<path>` reads the PE headers in TypeScript, with no tool from the toolchain. It checks a PE32+ image
for `DYNAMIC_BASE`, `HIGH_ENTROPY_VA` and `NX_COMPAT` in `DllCharacteristics`, a `SizeOfStackReserve`
of 8 MiB, and a non-empty resource directory. It names each one that is missing and exits 1. Its test
builds small headers in memory, one with each flag missing, and never reads a real binary. Declined:
`llvm-readobj` or `dumpbin`. Each is one more tool every machine and CI runner needs, for 60 bytes of
header.

**The Windows leg of `build.yml` runs `bun nv pe-flags` on the binary it archives.** It runs before
the archive step, so a release never ships a binary that fails it. The CI run builds the same leg, so
every push checks it too.

**Stage 2 decides with a measurement, not a guess.** Two target folders under
`.agent-tmp/lld-probe/`: `a` links with `link.exe`, and `b` with `rust-lld.exe` through
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`. Each is warmed with `cargo build --workspace
--all-targets` once. Then the measure is a touch of `crates/nvs-runtime/src/lib.rs` and the same build,
in the order a, b, a, b. The figure is each build's wall time and the number of binaries it linked,
which must be equal. `bun nv disk` runs first, and the probe does not start below 80 GB free. Both
folders are deleted when the figures are written.

**rust-lld is adopted when `b` is faster in both pairs, by more than the difference between the two
`a` runs.** Then `.cargo/config.toml` is created with `[target.x86_64-pc-windows-msvc] linker =
"rust-lld.exe"`, and it applies to every profile: dev, test, proof, bench and release. Reason:
one linker everywhere, so the binary CI tests is linked like the one users get. Safe default if
`bun nv pe-flags` or a test of stage 3 fails only under rust-lld for the release profile: the release
leg of `build.yml` sets `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=link.exe`, and the handoff names
why.

**When `b` is not faster, the goal ends after stage 2.** The session writes the figures under
`### State`, ticks stage 3's items as not needed, and deletes the `lld-green` check. Stage 1's check is
kept, and the goal closes.

**Only Windows changes.** Linux and macOS keep the linker their toolchain picks. Stage 2 checks which
linker 1.99.1 uses on `x86_64-unknown-linux-gnu` in the installed toolchain's release notes and
writes it in the handoff. If Linux does not use rust-lld already, that is a backlog line, not this goal's
work.

**The home of the choice is the comment in `.cargo/config.toml`.** It says which linker, why, what was
measured and the one thing that would undo it. No rule fragment is written, because no program
behaves differently.

**Pushing.** Stage 4 pushes with `bun nv ci --check`, then `bun nv ci --push`, and watches with `bun
nv ci --watch --next`, as `ci-green`'s decisions say. Never `git push`, never a force push, never
another branch, and no release. The first run after the change may start with a cold Rust cache on
the Windows legs, so its times are not the figure. Once it is green, `bun nv ci --dispatch full` runs
the same commit again on the warm cache, and stage 4 records that run's times.

**What stops it.** One attempt per item, and two sessions per stage, counted in `### State`. A test
that fails only under rust-lld is investigated once. If the cause is not found, the safe default above
applies and the goal goes on. No minute holds the goal open.

**Lanes.** One lane. Stage 1 is independent of the rest, and stages 2 to 4 run in order. This goal
follows `ci-critical-path`, whose `bun nv ci --times` stage 4 uses. Gate: `bun nv ci --times --help`
names `--times`.

## Stages

### 1. bun nv pe-flags checks the protections of a Windows binary, and every Windows build runs it

- [x] `tools/nv/cmd/pe-flags.ts:1`: the command, registered in `tools/nv/main.ts:69`, as the standing
  decision gives it. Its head comment names each flag and why each one matters.
- [x] `tools/nv/test/pe-flags.test.ts:1`: a header with every protection passes, and each missing one
  is named. A file that is not PE32+ is refused with its reason. It runs once, by name.
- [x] `bun nv pe-flags --bin target/release/nvs.exe` passes on this machine's release binary, built
  by `bun nv relink --build` if it is older than the tree's last change to `crates/nvs-cli/build.rs`.
- [x] `.github/workflows/build.yml:1`: the Windows leg runs it before the archive step, with a comment
  that says what it checks.

### 2. a local rebuild measures link.exe against rust-lld

- [x] `bun nv disk`, then the two warm builds and the four measured builds, as the standing decision
  says. The figures go under `### State`: each build's wall time and binaries linked.
- [x] The linker `x86_64-unknown-linux-gnu` uses on 1.99.1, from the installed toolchain's release
  notes, under `### State`.
- [x] The decision is taken by the standing decision's rule, and `.agent-tmp/lld-probe/` is deleted.

### 3. rust-lld links every Windows build, and nothing it links behaves differently

Only when stage 2 adopted rust-lld.

- [x] `.cargo/config.toml:1`: the linker for `x86_64-pc-windows-msvc`, with the comment the standing
  decision names.
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green.
- [x] A panic still names its file and line: `cargo test --test panic_location` if such a test exists,
  else one written in `crates/nvs-cli/tests/` that runs `nvs` on a program that panics in the
  runtime and reads the location from stderr. It runs once, by name.
- [x] `cargo test --profile proof --test forced_unwind` passes (`crates/nvs-host/src/scheduler.rs`,
  `Task::force_unwind`).
- [x] `cargo build --release --bin nvs`, then `bun nv pe-flags --bin target/release/nvs.exe` passes,
  and the file properties still show the version resource.
- [x] `nvs-ext` builds its guest crates with a child cargo (`crates/nvs-ext/build.rs:80`). The
  extension tests that build a guest run once by name and pass.

### 4. the change is pushed, its run is green, and the Windows build times are recorded

- [x] `bun nv ci --check`, then `bun nv ci --push`. Then `bun nv ci --watch --next` with
  `run_in_background` and a timeout of 7200000, and end the turn.
- [x] A red job is fixed as `ci-green`'s decisions say. A job that is red only under rust-lld gets the
  safe default.
- [x] `bun nv ci --green` exits 0.
- [x] `bun nv ci --dispatch full`, watched as above. Its `bun nv ci --times <id>`: each Windows
  leg's build step against run 44940832132, under `### State`.

### 5. the goal is verified once, and every failure is fixed

- [x] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [x] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Every stage is done, and the goal is reached. Stage 5 closed it without the full gate: `cargo
build`, `cargo clippy --all-targets -- -D warnings`, `cargo test --test panic_location` (2 pass),
`bun test tools/nv/test/pe-flags.test.ts` (6 pass) and `bun nv close --goal windows-rust-lld` are
all green, with nothing to fix. The goal joins the chain's `ungated:` list.

rust-lld links every Windows profile (`.cargo/config.toml`), and no safe
default was needed. Push run 45080779341 on c6c8111e4 is green, Website too, and `bun nv ci --green`
exits 0. The first push run's reds all came from taint-fail-closed, none from the linker; they are
fixed in fa4bc3f965, a47f3eb473, 3d4b3d4408 and c6c8111e4a.
Windows build steps, warm run 45084959508 against link.exe run 44940832132, in minutes:

| Leg | Step | link.exe | rust-lld |
|---|---|---|---|
| build windows-x86_64 | Build (release) | 18.6 | 13.7 |
| tests 1/3 | `cargo build --workspace --all-targets` | 20.8 | 19.7 |
| tests 2/3 | `cargo build --workspace --all-targets` | 20.7 | 14.5 |
| tests 3/3 | `cargo build --workspace --all-targets` | 21.1 | 19.4 |
| proofs | `cargo build --profile proof --bin nvs` | 10.4 | 10.6 and 14.1 (two parts now) |
| debug nvs | `cargo build -p nvs-cli` | 6.6 | 7.0 |

The proofs job was split in two between the runs, so its row is not like for like.

Sessions per stage: stage 1, one. Stage 2, one. Stage 3, one. Stage 4, one. Stage 5, one.

### Next

None: the goal is reached. Its decisions live in `.cargo/config.toml`'s comment and the header of
`tools/nv/cmd/pe-flags.ts`, so the file can be deleted. The next goal on the chain is
`review-hardening`. Expected red: none.

### Backlog
