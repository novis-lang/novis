# Novis — The Web-Native Programming Language: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance, and nothing
     trims to it. What IS checked is growth: `bun nv session --wrap` refuses an edit that
     leaves a field over 5x that and bigger than it was, so past the ceiling a sentence is replaced
     and never added; a shrink is always taken. This comment is the one home of both numbers.
     History lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md § *Keep
     each slice small, commit every one of them*. -->

> **Status:** **M4's loop goal is reached** — every check in its acceptance list passes, so the
> language surface is closed and nothing a CLI program reaches for panics below the front end. **The
> parity program is closed too**: it landed PHP core feature parity, all five SQL drivers,
> concurrency, governance and the server, and the loop has walked past them — *Open now* below says
> what it is walking through instead. Dependencies: `regex` + `fancy-regex` and `jiff` are named by
> the user; the rest the loop picks under `rule:packaging/a-c-dependency-answers-two-questions`.
>
> **Done:** **M0 through M8 are complete but M4** — everything from project setup to the stdlib and
> the five SQL drivers, which is the whole program ahead of the extension system. Goal
> `class-scoped-enums` reopens M4 for one language member, and M9 is the first later milestone with
> work still in front of it. `bun nv plan --past` is what says so, and it derives the
> answer rather than reading it: no goal on the chain carries a milestone, and no register still
> tags an item to it. Each milestone file under [docs/plan/](plan/) states its own acceptance.
>
> **On disk:** the workspace and its CI (three platforms, with miri, asan, tsan and fuzz legs), and
> the nine crates — `nvs-diagnostics`, `nvs-syntax`, `nvs-hir`, `nvs-types`, `nvs-ir`,
> `nvs-runtime`, `nvs-stdlib`, `nvs-codegen`, `nvs-cli` — plus `nvs-test`, `nvs-lsp`, `fuzz/`,
> `tools/`, `benches/abi-probe`, `editors/vscode`, and the two case trees `tests/conformance` and
> `tests/differential`. **Each crate's own module doc is the authority on what it holds and what it
> still owes**; `bun nv brief` prints one map line each, and `bun nv disk` the live counts.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows
> SDK 10.0.26100 for linking, PHP 8.5.9 as the differential oracle — on the Windows `PATH` and
> inside the WSL distro alike, at the same version — `cargo-fuzz` 0.13.2 and `valgrind` under a WSL
> nightly toolchain (docs/setup.md is what a machine installs, and why).
>
> **ADR slices landed:** **each ADR's own *Verification* section is the authority on what its slice
> covers, and this field never restates one** — `bun nv brief --where <keyword>` routes to the ADR
> that owns a topic, and `bun nv records --stats` shapes the whole set. What a crate still owes is
> its own module doc's `# Known gaps`. What landed in which session is in `git log`.
>
> **Open now:** **`data/chain.json` is the list of what is open**, in the order the loop walks it,
> and each goal is its prose at `docs/agent/goals/<slug>.md` plus its record under `data/goals/` —
> this field restates neither. A goal's plan is its own record: the goal switch deletes the goal it
> leaves and carries none of its checks. What a feature still owes of `rule:testing/feature-proofs`
> is `bun nv proofs --owed`, and what a crate still owes is a gap record under `data/gaps/` naming
> its owner.
>
> **Blocking:** Every `spec-*-outstanding.txt` ratchet under `crates/nvs-stdlib/tests/` holds zero
> keys, and what a shipped feature still owes is a gap record under `data/gaps/` naming its owner.
> The driver's owner gate refuses to reach a goal while a gap still names it, so no goal walks with
> its gaps open. Every design call a goal reaches is pre-authorized in its own § *Standing
> decisions*, and each goal names the numbered ADRs it may open and no others. One standing
> precondition, and not a block: the goals whose floor carries container-backed checks need a
> reachable Docker daemon, and the goals directory preflights it per entry rather than letting a
> session discover it mid-run. Picking every dependency but the two the user named is pre-authorized
> under `rule:packaging/a-c-dependency-answers-two-questions`.

**How the plan relates to the ADRs.** The plan is the record of *what* gets built, in what order, and how
each milestone is verified. It states decisions but does not argue them. The reasoning lives in
[docs/adr/](adr/README.md), and where a decision has its own ADR the plan links to it instead of restating
it — follow the link rather than expecting the argument here. For decisions with no ADR of their own, the
*why* is in [adr/README.md](adr/README.md) under *Decisions taken at project start* and the *mechanics*
are [docs/plan/design.md](plan/design.md) § *Architecture*.

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

**A milestone's number is its identity, and the schedule is not in this table.** The schedule is
`data/chain.json` — the goals in the order the driver walks them, each tagged with the milestone whose
work it does — and a milestone's state is its record under `data/plan/milestones/`. The **Carried by**
cell names the goals that do a milestone's work. Every cell of the table is written from those two, and
`bun nv plan --check` compares each one with them and fails CI when one has drifted, so this table cannot
drift away from what is actually being run. A
milestone the program has walked past and **finished** says `done` instead, whatever carried it, because
naming the goals then says where the work was rather than where it is; `bun nv plan --past` is what decides
that, from two facts read off the tree — every goal carrying it has walked, and no register still tags an
item to it — and `bun nv plan --check` refuses a milestone record that says `done` when it does not call
it complete. A milestone no goal carries and nothing
calls finished says where it stands on its own — `ongoing`, or `backlog N` for its place in the queue
behind the chain — and those words are the whole vocabulary of the column. Rows are in **identity order**, because a table
that is not the schedule has no business being sorted like one. Nothing is ever renumbered, because a
number that moves invalidates ~1300 cross-references across `docs/` and every one of them is a link
somebody has already followed.

| Carried by | Milestone | What it builds | Loop-days |
|---|---|---|---|
| done | [M0](plan/m0.md) | Project setup (~3 days) | 0.3 |
| done | [M1](plan/m1.md) | Front end (~3 weeks) | 0.7 |
| done | [M2](plan/m2.md) | HIR, types, IR (~4 weeks) | 1.5 |
| done | [M3](plan/m3.md) | Baseline Cranelift backend → **Hello World** (~3 weeks) | 0.5 |
| goal `class-scoped-enums` | [M4](plan/m4.md) | Language completeness — a usable CLI language (~10 weeks) | ~3 |
| done | [M4S](plan/m4s.md) | The `Core` API contract and its pure half (~5 weeks) | ~1.5 |
| done | [M4B](plan/m4b.md) | Minimal `nvs-lsp`, syntax highlighting and the VS Code extension (~3 weeks) | ~1.5 |
| done | [M5](plan/m5.md) | Concurrency and script isolates (~5 weeks) | ~3.5 |
| done | [M6](plan/m6.md) | Config, limits, capabilities, disk cache (~3 weeks) | ~1 |
| done | [M7](plan/m7.md) | Built-in HTTP server (~4 weeks) | ~2 |
| goal `ldap` | [M8](plan/m8.md) | Stdlib and databases (~16 weeks) | ~6.5 |
| ongoing | [M9](plan/m9.md) | Extension system, and the `nvs:ext@1.0.0` world it freezes (not yet sized) | not estimated |
| ongoing | [M10](plan/m10.md) | Developer tooling and IDE integration (~14 weeks; scope shifted by `rule:ide/every-feature-is-staged-behind-its-dependency`, net change undetermined) | ~8 |
| ongoing | [M12](plan/m12.md) | Optimising JIT tier (ongoing) | measurement-bound |
| backlog 4 | [M15](plan/m15.md) | Packages, the registry and the supply chain (~8 weeks) | ~3 + a calendar floor |
| backlog 5 | [M16](plan/m16.md) | `nvs/web`, `nvs new`, and the framework (~12 weeks) | ~4 |
| goals `pdf`, `spreadsheet` | [M17](plan/m17.md) | Document components: the image second wave, `nvs/pdf` and `nvs/spreadsheet` (not yet sized) | not estimated |

**One milestone is not one block of schedule, which is why the cell holds a list.** M8's work is spread
across a row of goals, and the goals tagged `post-parity` in the chain land in no milestone at all. A
single number per milestone could say none of that, and a chain number could say it only until something
was inserted ahead of it. **So the goal is the unit of schedule and the milestone the unit of identity:
say "goal `<slug>`", never "in M7".** A cell naming a goal means that milestone still has work
scheduled — `done` is the only thing that means finished. The optional subsystems PHP ships as
extensions, such as `gd`, `intl` and `imap`, are M9's.

Each row is a file under [docs/plan/](plan/). `bun nv plan --show M8` prints one
without you needing to know that, and `--show M8:verify` prints only its acceptance paragraph.
The decisions those milestones sit inside, the architecture and the verification strategy are
[docs/plan/design.md](plan/design.md).

**PHP core feature parity is measured by `bun nv migration` reporting 100% classified**: every function
and type of the oracle build accounted for as a `member`, `language` or `dropped` row, every `member`
row's member registered, and every one of them carrying a conformance case. A count of conformance cases
is a proxy for parity; a table that enumerates the source of truth is not.

**The two columns are not the same unit.** The parenthesised weeks are the original estimate, written for
a human team before any code existed; **Loop-days** is what this project's unattended loop actually spends,
elapsed and continuous — M0–M3 are measured, the rest projected. The measured conversion is ~23×, it is
not applied uniformly, and it carries a rework tax and four ways it breaks:
[docs/plan/velocity.md](plan/velocity.md) is the one home for all of that. Summed, the milestones left
come to **~7 weeks** against the ~99 the original column still shows. Neither figure gates anything.
