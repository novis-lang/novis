# Novis — The Web-Native Programming Language: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, Open now, Blocking. A
     field is overwritten in place through `bun nv plan --apply`, never extended with a paragraph,
     and no field name is added. Aim for ~400 bytes a field — guidance, and nothing trims to it.
     What IS checked is growth: `bun nv plan --apply` refuses an edit that leaves a field
     over 5x that and bigger than it was, so past the ceiling a sentence is replaced and never added;
     a shrink is always taken. This comment is the one home of both numbers. History lives in
     `git log`, and what a crate still owes in its gap records under `data/gaps/`. -->

> **Status:** **The language, `Core`, concurrency and script isolates, configuration and
> capabilities, the built-in HTTP server, the stdlib and the five SQL drivers, and the extension
> system are built**, with the editor support and the four built-in components: image, intl, PDF and
> spreadsheet. Goal `completeness-holes` is closing the holes a completeness audit found, a
> memory-safety bug first. Dependencies: `regex` + `fancy-regex` and `jiff` are named by the user;
> the rest the loop picks under `rule:packaging/a-c-dependency-answers-two-questions`.
>
> **Done:** Everything the Status field names as built. A finished milestone's scope file and record
> are deleted, so the table below lists only the milestones still ahead, and `git log` is the
> history.
>
> **On disk:** the workspace and its CI (three platforms, with miri, asan, tsan and fuzz legs); 21
> crates under `crates/`, from `nvs-syntax` to `nvs-server`; the four built-in components under
> `extensions/` and their WIT worlds under `wit/`; `fuzz/`, `tools/`, `benches/`, `editors/vscode`
> and `website/`; and the case trees under `tests/`, `tests/conformance` the largest. **Each crate's
> own module doc is the authority on what it holds and what it still owes**; `bun nv brief` prints
> one map line each, and `bun nv disk` the live counts.
>
> **Toolchain:** Rust 1.99.1 stable (pinned), Cranelift 0.136, wasmtime 49, MSVC 14.50 + Windows SDK
> 10.0.26100 for linking, PHP 8.5.11, optional and used by the PHP benchmarks only, `cargo-fuzz`
> 0.13.2 and `valgrind` under a WSL nightly toolchain (docs/setup.md is what a machine installs, and
> why).
>
> **Open now:** **Looper's chain, `docs/goals/README.md`, is the list of what is open**, in the
> order the loop walks it, and each goal is its file under `docs/goals/`. This field restates
> neither. A goal's plan is its own front matter: looper deletes a goal once it is reached and
> carries none of its checks. What a feature still owes of `rule:testing/feature-proofs` is
> `bun nv proofs --owed`, and what a crate still owes is a gap record under `data/gaps/` naming its
> owner.
>
> **Blocking:** Nothing. What a shipped feature still owes is a gap record under `data/gaps/` naming
> its owner, and the driver's owner gate refuses to reach a goal while a gap still names it, so no
> goal walks with its gaps open. Every design call a goal reaches is pre-authorized in its own §
> *Standing decisions*, and picking every dependency but the two the user named is pre-authorized
> under `rule:packaging/a-c-dependency-answers-two-questions`.

**How the plan relates to the rules.** The plan is the record of *what* is still to be built, and how
each milestone is verified. It states decisions but does not argue them. The reasoning lives in the rule
each decision settles, under [docs/rules/](rules/), and the plan cites the rule instead of restating it.
The architecture's *why* and its *mechanics* are [docs/plan/design.md](plan/design.md) § *Architecture*.

## Milestones

Each milestone ends with something runnable and its own tests. **The table lists only the milestones
still ahead**: a finished milestone's scope file and its record are deleted, and `git log` is the
history.

**A milestone's number is its identity, and the schedule is not in this table.** The schedule is
looper's chain, `docs/goals/README.md` — the goals in the order the loop walks them, each tagged in
its `extra` with the milestone whose work it does — and a milestone's state is its record under
`data/plan/milestones/`. The **Carried by** cell names the goals that do a milestone's work. A milestone
no goal carries says where it stands on its own: `ongoing`, or `backlog N` for its place in the queue
behind the chain. Those words are the whole vocabulary of the column. Every cell of the table is written
from the records and the chain, and `bun nv plan --check` compares each one with them and fails CI when
one has drifted. Rows are in **identity order**, because a table that is not the schedule has no business
being sorted like one. Nothing is ever renumbered, because a number that moves breaks every link to it.

| Carried by | Milestone | What it builds | Loop-days |
|---|---|---|---|

| goals `diagnostic-records`, `formatter-complete`, `cli-housekeeping`, `editor-refactors`, `editor-fixes`, `api-document`, `test-surface`, `mutation-testing`, `trace-timeline`, `bench-counters`, `debugger-core`, `debugger-adapter` | [M10](plan/m10.md) | Developer tooling and IDE integration (~14 weeks; scope shifted by `rule:ide/every-feature-is-staged-behind-its-dependency`, net change undetermined) | ~8 |
| ongoing | [M12](plan/m12.md) | Optimising JIT tier (ongoing) | measurement-bound |
| backlog 4 | [M15](plan/m15.md) | Packages, the registry and the supply chain (~8 weeks) | ~3 + a calendar floor |
| backlog 5 | [M16](plan/m16.md) | `nvs/web`, `nvs new`, and the framework (~12 weeks) | ~4 |
| ongoing | [M17](plan/m17.md) | The image component's second wave (not yet sized) | not estimated |

**One milestone is not one block of schedule, which is why the cell holds a list.** A milestone's work
can spread across several goals, and a goal may carry no milestone at all. A single number per milestone
could say none of that, and a chain number could say it only until something was inserted ahead of it.
**So the goal is the unit of schedule and the milestone the unit of identity: say "goal `<slug>`", never
"in M10".** A cell naming a goal means that milestone has work scheduled.

Each row is a file under [docs/plan/](plan/). `bun nv plan --show M10` prints one
without you needing to know that, and `--show M10:verify` prints only its acceptance paragraph.
The decisions those milestones sit inside, the architecture and the verification strategy are
[docs/plan/design.md](plan/design.md).

**The two columns are not the same unit.** The parenthesised weeks are the original estimate, written for
a human team; **Loop-days** is the projection for this project's unattended loop, elapsed and continuous.
The conversion is ~23×, it is not applied uniformly, and it carries a rework tax and four ways it breaks:
[docs/plan/velocity.md](plan/velocity.md) is the one home for all of that. Neither figure gates anything.
