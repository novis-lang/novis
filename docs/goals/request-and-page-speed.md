---
looper: 1
title: "A keep-alive request skips its empty read, and an html page writes numbers, escapes and Core objects without detours"
summary: "Reached when a short read parks the next read without a syscall on Linux and macOS, integer placeholders write without Rust's formatter, escaped text is scanned once to measure and once to write, and a Core instance finds its descriptor without a name lookup."
context: [benches/compare/cases/html-page/main.nvs, benches/compare/profile.sh, docs/rules/testing/member-perf-ledger.md]
stages:
  - id: 1
    title: "a short read leaves the readiness bit clear, so a keep-alive request makes one read"
    status: done
    context: [crates/nvs-host/src/net.rs, crates/nvs-host/src/counts.rs, docs/rules/concurrency/try-the-syscall-then-park.md]
  - id: 2
    title: "integers join a string as digits written on the stack, never through Rust's formatter or a string of their own"
    status: open
    context: [crates/nvs-runtime/src/placeholder.rs, crates/nvs-runtime/src/helpers.rs, crates/nvs-runtime/src/string.rs, crates/nvs-ir/src/lower/expr.rs, docs/rules/types/string-holes.md]
  - id: 3
    title: "escaped text is scanned once to measure and once to write"
    status: open
    context: [crates/nvs-render/src/html.rs, crates/nvs-runtime/src/string.rs, docs/rules/core-classes/html-auto-escape.md]
  - id: 4
    title: "a Core instance finds its descriptor and checks its receiver without a name lookup"
    status: done
    context: [crates/nvs-stdlib/src/instance.rs, crates/nvs-stdlib/src/html.rs]
  - id: 5
    title: "the changed features are measured again, and the comparison's cases are recorded again"
    status: open
    context: [benches/members/README.md, tools/nv/cmd/bench-compare.ts, benches/compare/README.md, website/src/content/docs/in-depth/compare.mdx]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: one-read-per-request
    stage: 1
    name: a keep-alive request makes no read that answers WouldBlock on Linux and macOS, and one on Windows
    run: [cargo, test, --lib, --, a_keep_alive_request_reads_and_wakes_within_its_count, a_short_read_parks_the_next_read_on_edge_triggered_platforms]
    timeout: 1800
    tests:
      - a_keep_alive_request_reads_and_wakes_within_its_count
      - a_short_read_parks_the_next_read_on_edge_triggered_platforms
  - id: integer-digits
    stage: 2
    name: every integer placeholder and integer part measures and writes what the formatter wrote
    run: [cargo, test, --lib, --, every_integer_placeholder_writes_what_the_formatter_writes, a_bare_integer_placeholder_joins_as_an_integer_part]
    timeout: 1800
    tests:
      - every_integer_placeholder_writes_what_the_formatter_writes
      - a_bare_integer_placeholder_joins_as_an_integer_part
  - id: escape-one-scan
    stage: 3
    name: the measured escape length and the written escape agree with escape on every byte and control
    run: [cargo, test, --lib, --, escaped_len_and_escape_into_agree_with_escape_on_every_byte]
    timeout: 1800
    tests:
      - escaped_len_and_escape_into_agree_with_escape_on_every_byte
  - id: descriptor-by-address
    stage: 4
    name: every Core class with instances finds the descriptor its name finds, without the name
    run: [cargo, test, --lib, --, every_core_class_with_instances_finds_its_descriptor_by_address]
    timeout: 1800
    tests:
      - every_core_class_with_instances_finds_its_descriptor_by_address
---

# A keep-alive request skips its empty read, and an html page writes numbers, escapes and Core objects without detours

## Why

`bun nv bench-compare --profile` on 2026-11-08 at `81a279ca7a`, Novis alone on one pinned core:

- **`hello`: 47,560 requests a second, 21.0 µs each.** About half the core is the kernel's TCP
  send and receive. `strace` counts 4.2 syscalls per request: one `sendto`, one `clock_gettime` (the
  CPU limit's thread clock), and two `recvfrom`, of which one always answers `EAGAIN`. That one is the
  read after the answer: hyper asks for the next request, the socket is empty, and only then does the
  stream park. The request's first read already came back shorter than its buffer, which on an
  edge-triggered reactor means the socket was empty. The rest of the user-space cost is the isolate per
  request, the CPU-limit clock and the watchdog publication, which the security priority keeps.
- **`html-page`: 9,373 requests a second, 106.7 µs each.** Three detours stand out:
  - **Integers.** A bare `%d` in `Core\Str::format` lowers to `helper.int_to_string`, a string of its
    own, three per row (`--dump-ir` of the case: `v20`, `v46`, `v54`). `nvs_int_to_string` is 5.3% with
    its allocation. Every other integer placeholder renders through `write!` twice, once to measure and
    once to write (`IntPlaceholder::digits`, `core::fmt::write` 3.9%).
  - **Escaping.** An escaped hole is scanned four times: `changes` and `changed_len` to measure, and
    `changes` and `write_changed` to write. Every product name has an `&`, so every row takes the long
    path. `nvs_render::html::changes` is 4.3% by itself.
  - **`Core` objects.** Every `Core\Html\Markup` built hashes its class name into an `FxHashMap`
    (`descriptor`), and every one read binary-searches the receivers and compares two names
    (`receiver`, 3.3%).

Each is a detour around work that is already cheap, so removing it changes no behaviour and spends
no memory. The evidence is counts: syscalls per request, and the member ledger's allocations and
instructions.

## Standing decisions

1. **A short read clears the readiness bit on Unix only.** After a read that returns `n` with
   `0 < n < buf.len()`, `NvsStream::poll_read` sets `quiet` exactly as a `WouldBlock` does, when the
   stream is registered for reading under the running task (`still_quiet`'s own test). The reactor is
   edge-triggered there (epoll, and kqueue with `EV_CLEAR`), so bytes that arrive afterwards raise a
   new event and bump the count. Tokio does the same, and also leaves Windows out. Windows keeps
   today's extra read, because its readiness is emulated and a short read promises nothing. A stream
   not yet registered keeps today's path. The rule fragment
   `rule:concurrency/try-the-syscall-then-park` gains the paragraph. **Fallback:** if any CI runner
   shows a lost wake, the clear is limited to Linux, and the macOS case goes in the fragment's Declined
   list with the evidence.
2. **One digit writer, no formatter.** `IntPlaceholder::digits` writes decimal through
   `helpers::write_decimal`, and hex, octal and binary through one loop over a radix. The `Digits`
   `fmt::Write` impl goes if nothing else uses it. A measuring pass counts digits without writing
   them (`u64::checked_ilog10`, or the bit length for a power-of-two radix). What every placeholder
   prints does not change, and the stage 2 test compares against `format!` over every base, flag and
   edge value.
3. **A bare integer argument joins as an integer part.** `FormatPiece::Argument` over a `Ty::Int`
   lowers to a `PartKind::Int` part, not to `concat_operand`'s `helper.int_to_string`. A `Ty::Uint`
   keeps its conversion, and a `.` join keeps `concat_operand` too. That keeps `nvs_str_concat_n`'s
   in-place append for string-only joins. A `Uint` part and the `.` join stay in the Backlog.
4. **The escape keeps one table and gets one-pass halves.** `escaped_len` counts the extra bytes of
   each escapable byte in the same scan that finds a bidi lead byte, and walks the bidi controls only
   when a lead byte appeared. `escape_into` writes in one pass. `escape` stays built on the two, so
   the bytes written are exactly today's (`rule:core-classes/html-auto-escape`).
5. **A `Core` descriptor is found by the class's address.** The `&'static CoreClass` a helper passes
   is the key of a thread-local cache of 64 entries, filled on a miss from today's lookup by name.
   A table built once cannot work: a `Core` class is a `const`, so each codegen unit may keep its own
   copy at its own address, and no table lists every copy. `descriptor`, `build`, `receiver` and
   `is_instance` take a `&'static CoreClass`, so no other class is ever at a key's address.
   `receiver` first compares the object's class word with that descriptor, and falls back to
   today's rows only for the AST productions that share `Core\Ast\Node`. A failed check still
   throws the same `Fault` with the same text. Memory: 1 KiB per thread, fixed and never allocated.
6. **Out of scope:** the isolate per request, the CPU-limit clock read, the watchdog publication, the
   accounting allocator, a `Markup` rope and `Core\Html::join` (declined before, for the security
   priority and for simplicity). An item that finds one of them cheaper than it looks writes a gap
   record. It never changes it in this goal.
7. **Lanes.** Stage 1 (`nvs-host`, `nvs-server`'s test) and stage 4 (`nvs-stdlib`) share no file with
   anything else, so either may run as a side goal. Stages 2 and 3 both edit
   `crates/nvs-runtime/src/string.rs`, so they run in order. Stage 5 runs after stages 1 to 4.
8. **The comparison is recorded from a detached worktree with the loops running.** Cases only, not the
   scenario rows, which this goal does not change. The commit says the figures are noisier for it.

## Stages

### 1. a short read leaves the readiness bit clear, so a keep-alive request makes one read

**Does:** removes the `recvfrom` that answers `EAGAIN` after every keep-alive request on Linux and
macOS.

- [x] Set `quiet` after a short read under decision 1, and rewrite `quiet`'s doc as a whole.
  `rule:concurrency/try-the-syscall-then-park` `crates/nvs-host/src/net.rs:1548`
  `crates/nvs-host/src/net.rs:203`
- [x] Add the paragraph to the rule fragment: what a short read means on each reactor, why Windows is
  left out, and what it spends (nothing). `docs/rules/concurrency/try-the-syscall-then-park.md:1`
- [x] Lower `WOULD_BLOCK_READS_PER_REQUEST` to 0 under `cfg(unix)` and keep 1 on Windows, and rewrite
  the test's doc. `crates/nvs-server/src/serve.rs:10634`
- [x] Write `a_short_read_parks_the_next_read_on_edge_triggered_platforms` beside `poll_read`'s
  tests. It runs on every platform and asserts that platform's count: no `WouldBlock` read after a
  short read on Unix, one on Windows. `crates/nvs-host/src/net.rs:1548`
- [x] Run the `one-read-per-request` check here and in WSL with `bun nv linux-test`, since the change
  is a socket path (AGENTS.md § *The rules only this project has*).

### 2. integers join a string as digits written on the stack, never through Rust's formatter or a string of their own

**Does:** removes three digit strings per row and the formatter from every integer placeholder.

- [x] Rewrite `IntPlaceholder::digits` with decision 2's writer, and give `len` a counting path that
  writes nothing. `rule:types/string-holes` `crates/nvs-runtime/src/placeholder.rs:149`
  `crates/nvs-runtime/src/placeholder.rs:174` `crates/nvs-runtime/src/helpers.rs:69`
- [x] Measure `Piece::Int` by counting digits, not by writing them. `crates/nvs-runtime/src/string.rs:1678`
- [x] Write `every_integer_placeholder_writes_what_the_formatter_writes`: every base, `+`, left and
  zero padding, width, precision, and `0`, `-1`, `i64::MIN`, `i64::MAX`, `u64::MAX`, compared with
  `format!`, for both `len` and `write`. `crates/nvs-runtime/src/placeholder.rs:245`
- [ ] Lower `FormatPiece::Argument` over an `int` to a `PartKind::Int` part under decision 3.
  `crates/nvs-ir/src/lower/expr.rs:2437`
- [ ] Write `a_bare_integer_placeholder_joins_as_an_integer_part` in the lowering tests: the IR of
  `Core\Str::format('Teapot %d & lid', $i)` has no `helper.int_to_string`.
  `crates/nvs-ir/src/lower/tests.rs:1`

### 3. escaped text is scanned once to measure and once to write

**Does:** halves the passes over every escaped hole.

- [ ] Rebuild `escaped_len` and `escape_into` as single passes under decision 4, keeping `escape`
  built on them. `rule:core-classes/html-auto-escape` `crates/nvs-render/src/html.rs:184`
  `crates/nvs-render/src/html.rs:195` `crates/nvs-render/src/html.rs:209`
- [ ] Check that `Piece::HtmlText`'s `len` and `write` call one half each and nothing else.
  `crates/nvs-runtime/src/string.rs:1678` `crates/nvs-runtime/src/string.rs:1691`
- [ ] Write `escaped_len_and_escape_into_agree_with_escape_on_every_byte`: every ASCII byte
  alone and inside text, every bidi control terminated and open, and mixed text. Each case's
  `escaped_len` equals the length `escape_into` writes, and those bytes equal `escape`'s.
  `crates/nvs-render/src/html.rs:539`

### 4. a Core instance finds its descriptor and checks its receiver without a name lookup

**Does:** takes the name hash out of every `Core` construction and the name compare out of every
receiver check.

- [x] Find the descriptor by the class's address under decision 5.
  `crates/nvs-stdlib/src/instance.rs:689` `crates/nvs-stdlib/src/instance.rs:784`
  `crates/nvs-stdlib/src/instance.rs:476`
- [x] Compare the class word first in `receiver` and `is_instance`, falling back to the rows only for
  AST productions. `crates/nvs-stdlib/src/instance.rs:986` `crates/nvs-stdlib/src/instance.rs:759`
- [x] Write `every_core_class_with_instances_finds_its_descriptor_by_address`: it walks
  `registry::CLASSES` and `INTERNAL_CLASSES`, and for each class with instances the address lookup
  returns what `Descriptors::find(name)` returns. `crates/nvs-stdlib/src/instance.rs:660`

### 5. the changed features are measured again, and the comparison's cases are recorded again

**Does:** records what the goal bought, in counts, and puts it on the comparison page.

- [ ] Run `bun nv proofs --gate`, then `bun nv proofs --record-perf --force --id '<feature>'` for each
  figure it names as stale. Expect at least `Core\Str::format`, `Core\Html::escape`,
  `Core\Html::join` and `lang:types/values-written-in-the-code`. `benches/members/README.md:1`
- [ ] Where a figure's allocations dropped, lower that bench's `// bench: allocations N` line.
  `benches/members/core/Str/format.nvs:1`
- [ ] Run `bun nv bench-compare --profile hello` and `--profile html-page`, and put requests a
  second, the `recvfrom` count and the top shares beside *Why*'s in the commit message.
  `benches/compare/profile.sh:1`
- [ ] Record the cases from a detached worktree at HEAD under `.agent-tmp/worktrees/` with
  `bun nv bench-compare --record benches/compare.json`, copy the record back, and delete the
  worktree. `tools/nv/cmd/bench-compare.ts:1`
- [ ] Run `bun nv render --website`, fix every figure the page's prose states by hand, and commit
  record and page together. `website/src/content/docs/in-depth/compare.mdx:1`

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done in `649760357a`. On Unix a short read in `poll_read` sets `quiet` when the stream is
registered for reading under the running task (`quiet_after_a_short_read`). The keep-alive count
test's bound is 0 reads that answer `WouldBlock` per request on Unix and 1 on Windows. Both tests of
`one-read-per-request` pass on Windows and in WSL. macOS is untested until CI runs: if a macOS runner
shows a lost wake, decision 1's fallback applies. *Why*'s figures are from `81a279ca7a`.

Stage 4 is done. `descriptor` reads a thread-local cache keyed by the class's address and looks up
by name only on a miss (decision 5, amended: a table built once misses the per-unit copies of a
`const` class). `receiver` compares the class word first. Every helper that passes a class now takes
`&'static CoreClass`. `every_core_class_with_instances_finds_its_descriptor_by_address` passes.

The last sweep's items were already settled. `env-secret-proofs` is a check from a retired goal with
no log on disk. The nightly run `42468563179` ran on `81a279ca7a`, which is older than `c2b8671f2`,
the commit `ci-green` recorded green.

### Next

Stage 2, then stage 3, in order, since both edit `crates/nvs-runtime/src/string.rs`.

- `crates/nvs-runtime/src/placeholder.rs:149` and `:174` — `IntPlaceholder::digits` and `len`.
- `crates/nvs-runtime/src/helpers.rs:69` — `write_decimal`, decision 2's writer.
- `crates/nvs-runtime/src/string.rs:1678` — `Piece::Int`'s measure.
- `crates/nvs-ir/src/lower/expr.rs:2437` — `FormatPiece::Argument`, lowered under decision 3.

Expected red until stage 2 lands: `every_integer_placeholder_writes_what_the_formatter_writes` and
`a_bare_integer_placeholder_joins_as_an_integer_part`, which do not exist yet.

### Backlog

- `crates/nvs-host/src/reactor.rs:19` and `:956`, and `rule:concurrency/the-parking-contract`'s rule 1,
  call the poller level-triggered. mio is edge-triggered on epoll and kqueue. `net.rs` no longer says
  so, and those three can be reworded in a slice of their own.

- A `Uint` string part, and a `.` join lowered to parts when it has an integer operand. Both were left
  by `strings-in-place`'s decision 11, and decision 3 here leaves them again.
