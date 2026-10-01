---
milestone: post-parity
position: last
---
# Loop goal 184 — one performance pass over everything, and no work that grows faster than linear

Novis is feature complete. This goal makes one full pass over it to find the big performance problems
and fix them at their root. It does not look for micro-optimizations.

**The base rule, by the user's decision of 2026-10-01:** work that grows linearly with its input is
correct, and a little steeper than linear is fine too. Work that grows quadratically, exponentially or
otherwise much faster than linear is a defect. That covers time, CPU and memory, for every input a
program, a request, a compiler run or an editor session can make large.

The pass has two halves. **Measuring:** every bench runs at more than one size, and the growth is read
from the counts. **Reading:** the code is reviewed area by area for problems a bench does not show,
above all the ones that appear only when a real program uses many features at once. Every problem
either half finds is fixed in this goal. At the end, `docs/perf/performance-pass.md` tells the user in
plain words what was found, what improved and by how much. Goal `growth-proof` comes next and turns
the growth check into a feature proof.

## Why here

**Last, behind goal `goal-closeout`, by the user's decision of 2026-10-01.** A performance pass
before the language was complete would have measured code that was about to change. Every feature now
exists, so one pass sees them all, and the problems that come from combining features are visible for
the first time. Goal `goal-closeout` runs first so the pass starts on a chain with no old goals and on
registers that are proven closed.

**What this spends:** memory, where a fix needs it. [ADR 0004](../../decisions/0004.md) puts memory
last, and a fix that turns a quadratic into a linear by keeping an index or a cache is the trade it
describes. Each such fix says what it spends, per request or per task, in its doc comment. A cache
must stay bounded and attributable to a request or to the process. A cache that grows with total
traffic is a leak and is never the fix.

## Stage 0 — the catch-up

None.

## Stage 1 — the floor

Nothing is carried. Goal `goal-closeout` makes a finished goal deleted, and the driver deletes it when
this goal starts. The suites, the `.nvst` trees and `nv verify` are the floor. This goal's own Stage 6
checks that the feature proofs and their perf figures are current.

## Stage 2 — the growth tool, the keystone

**Does:** Builds `bun nv scaling`, which runs a program at several sizes and reports how fast its cost grows.

`tools/nv/cmd/scaling.ts` is new. It reuses the counting run of `tools/nv/proofs/perf.ts` (`nvs run
--count`: statements, calls, allocations, bytes) and its clock. It has two modes.

- **`--iterations`, over the existing bench tree.** This is the user's suggestion: run the same code
  with a different number of iterations. **Small batches, never the bench's own count, by the user's
  decision of 2026-10-01.** A bench's `// bench: iterations N` is sized for a stable clock reading,
  and running every bench at N or above would take hours. The tool rewrites the literal in the
  closing `echo Bench::run(N)` line in a copy under `.agent-tmp/`.
  - **Start low and double until the pattern is clear, by the user's decision of 2026-10-01.** The
    first batch is small, and each next batch is double the one before. The ramp stops once the last
    three batches agree on the growth. **Only the counts decide that**, because they are the same on
    every run and on every machine, so a bench gets the same answer every time and noise can never
    push its size up. The clock is read at every batch and reported, but it never makes the ramp go
    further. The ramp also stops at a fixed ceiling that no session raises. The start, the agreement
    test and the ceiling are written in the tool's module doc, which is their one home.
  - **Valgrind only when the ramp shows no clear growth, by the user's decision of 2026-10-01.** A
    Rust member that does its work without allocating shows the same counts at any size, and its clock
    may be too noisy to read. Only when the ramp reaches the ceiling without a clear pattern does the
    tool run the same batches again under `valgrind --tool=callgrind` in WSL. Callgrind's instruction
    count is the same on every run, so it decides the way the counts do. A bench that is still unclear
    after that is reported, not judged.
  - **The counts decide.** They are exact at any batch size, so the per-operation counts must be the
    same in every batch. A count that rises per operation means each operation leaves something
    behind that the next one pays for: a leak, or a structure that grows with every call. The clock
    slope over the batches is the second signal, under the same bounds as a ladder.
  - `.scale.nvs` siblings run in the same pass, with the same ramp.
- **The ladders, over a new tree `benches/scaling/`.** One iteration count tells nothing about *input
  size*, and input size is where quadratic work hides. A ladder program takes its size in the same
  closing line, `echo Bench::run(N)`, and declares `// scaling: start 100`, `// scaling: max
  100000` and `// scaling: expect linear`. The size ramps the same way the batches do: it starts low
  and doubles until the last three sizes agree on the count slope, or the size reaches `max`. The tool fits
  the slope of log cost against log size, for each count and for the clock. A ladder only grows as
  far as it must to show its pattern, so it runs in seconds, not minutes.

A ladder has a **kind**, because not every cost is a program running:

| Kind | The size is | What is measured |
|---|---|---|
| `run` | the input the program builds | `nvs run --count` and the clock |
| `compile` | the size of the program the ladder prints | the ladder prints a program; the tool compiles it with `nvs check` and with `nvs run` |
| `lsp` | the size of the document or the workspace the ladder prints | `nvs lsp` over stdio, driven the way `tests/lsp/` drives it: open, edit, completion, hover, references |
| `fmt` | the size of the file the ladder prints | `nvs fmt` |
| `serve` | requests served, or connections, or the size of one request | `nvs serve` with a load the tool sends; per-request counts and peak memory |

**The compiler gets counts of its own.** `nvs run --count` counts what a program did. It says nothing
about what compiling the program cost, so a `compile` ladder would have only the clock, and the clock
never decides alone. This stage adds `nvs check --count`, which prints one line on stderr with the
work each phase did: tokens, syntax nodes, resolved names, typed expressions, and IR instructions
where the command lowers. `nvs run --count` adds the same line for its own compile. Where a phase's
output already has a length (a token list, a node arena, a function body), the count is that length,
read once when the phase ends. A counter inside a hot loop is added only where no such length exists,
and nothing is counted while the flag is off. A `compile` ladder fits a slope for each phase's count
as well as the total, so a steep compile names the phase and the count that carry it. `lsp` and
`fmt` ladders have no counts of their own. Their clock decides under the second-run bound, and
callgrind settles a doubtful slope, as it does for a bench.

**The bounds**, written in the tool's module doc, which is their one home:

- A count's slope above **1.35** fails. Counts are the same on every run, so they decide.
- A clock slope above **1.5** fails only when a second run agrees. The machine runs other work, so
  the clock is the second signal and never the first.
- `expect` may name an algorithm whose best known form is steeper than linear: `nlogn` for a sort,
  or the class that `Core\BigInt` multiplication has. A ladder is judged against what it declares.
  `quadratic` is never accepted as a declaration.
- A `serve` ladder over requests served must be flat per request, and its peak memory must not grow
  with the count. Memory is O(in-flight), never O(requests served).
- A ladder marked `// scaling: proposal` is reported as waiting for the user's decision and does not
  fail. Only Stage 5 adds that marker, and only for the cases its standing decision names.

`benches/scaling/README.md` says what a ladder is, as `benches/members/README.md` does for a bench.

## Stage 3 — the ladders, area by area

**Does:** Writes a ladder for every area, runs them, and records every problem as a gap owned by this goal.

The area list is fixed in `tools/nv/cmd/scaling.ts`. `bun nv scaling --areas` fails while an area has
no ladder, and `--areas --reviewed` also fails while an area has no section in
`docs/perf/performance-review.md`, which is Stage 4's check. The areas, and the sizes that can grow in
each:

| Area | What grows |
|---|---|
| `compiler` | functions, classes, statements in one function, nesting depth, length of one expression, string and array literal size, `match` arms, files joined by `require`, routes declared — `crates/nvs-syntax`, `nvs-hir`, `nvs-types`, `nvs-ir`, `nvs-codegen` |
| `values` | arrays passed and copied, object properties, deep and long chains freed at once, closures, exceptions through deep call stacks — `crates/nvs-runtime` |
| `arrays` | every `Core\Arr` member over n elements: sort, unique, diff, intersect, search, slice, splice, keys, merge |
| `strings` | `Core\Str` over n characters and n pieces, building a string in a loop, UTF-8 indexing |
| `regex` | subject length and pattern size under `Core\Regex` |
| `json` | document size and nesting depth, encode and decode |
| `markup` | `Core\Xml`, `Core\Html` parse and write, size and depth |
| `formats` | CSV, query strings, form bodies, multipart parts, cookies, `Core\Uri` |
| `templates` | markup literals and rendered pages over n rows — `crates/nvs-render` |
| `numbers` | `Core\BigInt` and `Core\Decimal` digit counts, judged against their algorithm |
| `time` | ranges and recurrences, time zone lookups |
| `database` | rows read, parameters bound, distinct statements prepared (the plan cache), connections — over SQLite, which needs no container; the other drivers where a difference is suspected — `crates/nvs-db` |
| `queue` | jobs waiting, jobs claimed, scheduled entries |
| `cache` | keys in the request, process and shared tiers, session size |
| `scheduler` | tasks, timers, channels and their waiters — `crates/nvs-host` |
| `request` | headers, header size, body size, query parameters, routes in the program — `crates/nvs-server` |
| `connections` | open connections, keep-alive requests on one connection, slow clients, event streams and WebSockets |
| `traffic` | requests served: per-request cost and peak memory stay flat |
| `http-client` | response size, redirects, pooled connections |
| `lsp` | document lines, files in the workspace, edits in a session — `crates/nvs-lsp` |
| `fmt` | file size, nesting depth, line length — `crates/nvs-fmt` |
| `app` | the combined workload below |

**The combined workload, `app`.** Real programs touch many features in one request, and a cost that is
small alone can multiply when they meet. `benches/scaling/app/` is one realistic program in the shape
of a small shop: routes with parameters, a middleware chain, a session, SQLite reads and writes,
validation of a form, a template that renders a list, a JSON API, the request cache and a queued job.
Its ladders grow the rows per page, the routes in the program, the requests served and the clients at
once. A second ladder generates the same application at n modules and measures `compile` and `lsp`
on it.

A problem becomes a gap record owned by `performance-pass`, under the crate that owes it. It names the
ladder that shows it, the measured slope, the `file:line` of the cause, and the growth the fix should
reach.

## Stage 4 — the review

**Does:** Reads the code of every area for growth problems no ladder shows, and records each one as a gap owned by this goal.

A ladder only finds what somebody thought to make large. This stage reads the code. It fans out to
subagents, one per area of Stage 3's table, each read-only, each looking for these shapes:

- a scan inside a loop over the same data: `contains`, `position`, `find` or a linear lookup per item
- `remove(0)`, `insert(0, …)` or a shift of a whole buffer per element
- a string or a buffer rebuilt from the start on each append
- a collection cloned per call or per element where a borrow or a shared reference would do
- work redone per request that could be done once per program: a pattern compiled, a template parsed,
  a route table built, a configuration read
- character indexing that walks from the start of a string each time (`chars().nth`), and line or
  column numbers counted from the start of a file for each position
- a full reparse, re-check or re-index of a document or workspace per keystroke
- recursion whose depth is the input's size, which fails on a deep input and often costs quadratic time
- a registry, cache, map or log that grows with total traffic and is never trimmed
- a lock held across a wait, or one global lock on the request path
- a sort, or a copy of everything, inside a loop
- work paid when nobody reads it: a clock read, a timer, a log line built or a metric recorded on the
  request path when no reader is configured
- output nobody asked for: diagnostic detail, debug records or metadata built on every run when only
  a flag or an error path reads them
- a cache whose answer can differ from computing it again. That is state, not a cache, and it is a
  correctness defect before it is a performance one

Each subagent reports the site, why it grows, and how a ladder would show it. The session writes the
ladder when one can show it, and the gap in every case. `docs/perf/performance-review.md` gets one
section per area: what was read, and what was found or that nothing was. That file is the technical
register of the pass. The summary for the user is Stage 6's.

**The fixed floor.** Every program pays the cost of the smallest input before its own work starts:
an empty program run, an empty program compiled, one empty request served, an empty file opened in
the editor. That cost does not grow, so no ladder shows it, and it is on every path real programs
use. The session measures each one with its counts, reads what the counts are spent on, and records
it in a `## floor` section of `docs/perf/performance-review.md`. Work the floor pays for nothing is a
gap under the same in-scope test as any other.

**Old refusals are priced again.** A decision record or a perf note that declined a performance
change because of a figure measured on an earlier, smaller tree is measured again on today's tree.
A verdict that changes becomes a gap. One that holds gets a line in the area's section with the new
figure.

## Stage 5 — the fixes

**Does:** Fixes every gap owned by this goal at its root, with its ladder kept as the guard.

Each gap is fixed at its cause, never patched around. The fix keeps its ladder as the guard, so the
growth cannot come back unnoticed, and deletes the gap record. The order is by file set: gaps in one
crate are one group. A fix that moves a `Core` member's implementing file makes its perf figure stale,
and `bun nv proofs --record-perf --id '<feature>'` records it again in the same slice.

**Every fix follows the same steps.**

- **Profile, then write down the claim and its ceiling, before building anything.** The gap record
  says what the fix should save and the most it can save. The cheapest ceiling is a throwaway build
  that runs the phase twice: the extra cost is the most that removing the phase can ever save. A
  fix that does not change growth and whose ceiling is under the in-scope share (see the standing
  decisions) is not built. Its ceiling goes as one line into the area's section of
  `docs/perf/performance-review.md`, so a later pass can price it again.
- **Nothing else gets worse.** After a fix, every bench and ladder the change reaches runs again. If
  another one's counts rise, the fix is not kept as it stands. The session finds out why, and keeps
  the fix only when the rise is a bounded constant that the fix's doc comment names as what it spends.
- **Removed code is deleted.** It does not stay behind a flag or a setting, and no switch is added
  that nobody would ever turn off.

**The build, last.** When every gap is closed, the session races the release profile over the bench
tree with no code change: `lto = "fat"` against today's `"thin"`, a PGO build trained on the benches
of half the areas and raced on the other half, and an `x86-64-v3` build. JIT code is already compiled
for the host CPU (`host_isa` in `crates/nvs-codegen/src/lib.rs`), so a target level reaches only the
Rust code: the runtime, `Core` and the compiler. Two lines of the profile are not raced. `panic =
"unwind"` is what contains a panic to one request, and overflow checks stay on for security.
`Cargo.toml` says both beside them.

- The benches are sorted once, before the first race, into heavy, medium and light by their counted
  cost.
- A build change is worth proposing when the heavy tier's geometric mean improves by more than the
  noise and no heavy bench gets slower, or when the heavy tier is flat, the lighter tiers improve and
  nothing gets slower. A change that slows a heavy bench to speed up light ones is not proposed,
  whatever the total says.
- A build flag does not change the counts, so the races read callgrind's instruction count under WSL,
  with the clock beside it as the standing decision on the clock says.
- Each race's build goes to its own target directory under `.agent-tmp/`, which is deleted when the
  race is done.

None of them is built in this goal. Each one adds time to every release build the loop makes, and
PGO and a target level change how Novis is built and shipped. The figures go under *Decisions for
you*, each with the release build time it costs.

When a fix needs a large tradeoff (see the standing decisions), it is not built. The session writes it
into `docs/perf/performance-pass.md` under *Decisions for you*: the problem, what it costs today with a
measured figure, the fix, and what the fix would cost. It then marks the ladder `// scaling: proposal`
and deletes the gap. The pass continues.

## Stage 6 — the summary

**Does:** Writes `docs/perf/performance-pass.md`, a short plain summary for the user, and re-records every stale perf figure.

The user reads this file, not a contributor. It is short and has few technical details. It follows
the spirit of [AGENTS.md](../../../AGENTS.md) § *Text an end user reads*: short sentences, real
numbers, no internals. It has these sections:

- **What we found** — the problems, each in a sentence or two that says where somebody would meet it.
  For example: "A page that listed many rows got slower with every row added."
- **What got better** — before and after for each fix, as a plain comparison at a stated size: "At
  10,000 rows the page renders 40 times faster", "memory now stays flat after a million requests".
- **Decisions for you** — every large tradeoff Stage 5 recorded and did not build, one short
  paragraph each.
- **Benches to look at** — what the ramp itself turned up: every bench that reached the ceiling
  without a clear pattern even under callgrind, and every bench that needed callgrind because the
  ramp alone showed no clear growth. One line each, saying what happened. Write "none" if there is
  nothing.
- **What we checked and found fine** — the areas with no problem, in a few lines.

`bun nv proofs --gate` must owe nothing at the end, so every figure a fix made stale is recorded again.

## Standing decisions

- **The base rule is the user's.** Linear is correct and a little steeper is fine. Quadratic,
  exponential and anything much steeper is a defect. An algorithm whose best known form is steeper,
  such as a comparison sort, is judged against that algorithm.
- **No micro-optimizations.** A fix is in scope when it changes how cost grows, or when it removes a
  cost that a measurement shows is at least half of a path real programs use: the request path, the
  compile of a large program, the editor on a large file. Anything smaller is left alone and is not
  recorded.
- **The priority ordering holds.** Security and request isolation are never traded for speed. Language
  semantics are never changed for speed. Memory may be spent, and the fix says what it spends.
- **An API change is ordinary work, by the user's decision of 2026-10-01.** No Novis code has shipped,
  so changing a `Core` member's signature or a configuration key is allowed when it is the root fix. It
  updates the rule, the reference, the help, the examples and the tests in the same slice.
- **A large tradeoff is recorded, not built, and the pass continues, by the user's decision of
  2026-10-01.** Large means one of these: a change to observable language behaviour, a memory cost that
  grows per request by more than a small constant, a new crate dependency, new `unsafe` code, or a
  weaker security or isolation property. The user decides all of them together when the goal is done.
- **Nothing the pass finds stops the run, by the user's decision of 2026-10-01.** A finding is fixed,
  or written into `docs/perf/performance-pass.md`. A large tradeoff goes under *Decisions for you*,
  and a bench the growth check could not judge goes under *Benches to look at*. No session reports
  `BLOCKED` for either. The user reads them all together when the goal is done.
- **Counts decide and the clock confirms.** The machine runs other work, so a clock figure alone never
  fails a ladder and never proves a fix. A fix's before and after are read from counts where the
  counts see the work. For work inside a Rust member that only the clock sees, use the best of
  several runs. When the clock compares two builds, they run in alternating rounds at high priority,
  each column takes its best round, and an unchanged control column shows how large the noise is.
  Under WSL, `valgrind --tool=callgrind` gives an instruction count that is the same on every run,
  and it settles a doubtful case.
- **A guarantee is never given up on a guess.** A fix that would weaken a check, a precision or an
  isolation property for speed is first measured as a throwaway probe, and the probe is deleted. Its
  figure is what *Decisions for you* reports as the gain.
- **Frozen output stays frozen.** No expected output of a test, a case or an example is edited to make
  a fix pass.
- **Subagents.** Stage 4 runs one read-only subagent per area. Stage 3 may run one per area to write
  and run ladders. Each stays under 250k of context, writes any scratch file under `.agent-tmp/`, and
  deletes it before it reports. Only the session edits code in `crates/`, so two agents never edit
  one file.
- **Unix-only paths** run on the WSL leg. A ladder that needs Unix sockets declares `// requires: unix`
  as a bench does.
- **ADR slots:** a fix that changes what a rule says opens one new record for it, and names no number
  in advance. The growth proof is goal `growth-proof`'s, not this goal's. The growth tool and the ladders need no record. `benches/scaling/README.md` and the
  tool's module doc are their homes.
