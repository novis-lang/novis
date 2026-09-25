---
milestone: post-parity
---
# Loop goal 62 — both entry forms reach a worker core, and a serving core offers itself

A child spawned `on: "worker"` starts on another core whichever way its entry is written, so the
placement is a fact about what the program asked for rather than about the spelling the spawn site
used. Under `nvs serve` the core it reaches is a sibling serving core, which is the destination
ADR 0184 § 5 decides on and the one the process is already running a scheduler on. Afterwards
`crates/nvs-host/src/placed.rs` and `crates/nvs-host/src/worker.rs` own no gap, and
`rule:concurrency/on-worker-runs-the-child-on-another-core`'s last two paragraphs state a placement
with nothing subtracted from it.

## Why here

The crossing is built and is not what this goal touches: `crate::placed` already encodes an argument,
carries a `PlacedIsolate` and decodes a `Crossed` answer at the join, and `crate::worker` already
holds the inbox, the lazily started cores and the answer slot. What it is missing is a **resolver the
destination core can reach**, and that exists too — `nvs_cli::script::Compiler` keeps its path table
and its unit table behind `RwLock`s and is already handed to every serving core as one
`Arc<Compiler>` (`crates/nvs-cli/src/serve.rs:703`), so the fleet compiles a source once for the
process. Only the *seam* is still per-thread by borrow: `nvs_runtime::script::install` takes a
`&'static dyn Resolver` and `scoped` a borrow on the installing core's own stack, and a core
`nvs-host` started for itself was handed neither.

So this goal is a seam and a destination set, both over parts that are on disk, and it could not have
been written before either half existed. Goal `gap-zero` is what needs it: that goal's gate is that no
register item names a goal, and these two items name this one.

## Stage 0 — the catch-up

The sentences on disk that go wrong the day this goal is green, each with the file that holds them:

- `rule:concurrency/on-worker-runs-the-child-on-another-core`, its last two paragraphs — *Which entry
  crosses is a fact about the spawn's form* and the `nvs serve` exception. Both are written as what
  runs and both stop being true; the rule is amended in the slice that makes it so, per AGENTS.md
  § *Where to look*.
- `crates/nvs-host/src/placed.rs` — the `# Known gaps` block and
  [`destination_for`]'s second question, *an entry the far core can prepare, which today is the method
  form alone*, which `crosses` spells as `entry.is_method()`.
- `crates/nvs-host/src/group.rs` — the `# Known gaps` block, which states the same gap a second time
  because `SchedulerHost::start_isolate` is where the fall-through lands.
- `crates/nvs-host/src/worker.rs` — the `# Known gaps` block, which is the destination half.
- `crates/nvs-runtime/src/script.rs` — the comment above its test resolvers, *a real implementor holds
  a unit cache, is `!Sync` for that reason and goes through `scoped` instead*. The unit cache the tree
  actually has is shared across the fleet already; what `scoped` buys is a lifetime, not thread
  confinement, and this goal makes the distinction load-bearing.

## Stage 1 — the floor

Goal `class-scoped-types`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: a resolver the destination core can reach

`nvs_runtime::script` gains a form of the seam that can be **published to a core that has not started
yet** — a handle the placing core writes and the started core installs on its own thread, rather than a
borrow that only unwinds with the stack that took it. The unit table behind it is unchanged and stays
`nvs-cli`'s: the seam's module doc already owns why the compiler is reached this way round rather than
from `nvs-host`, and nothing here gives a `Resolver` a notion of a running unit.

Once a started core can resolve, `crosses` stops asking which form the entry is and a path child is
prepared where a method child already is — by whichever core is about to run it
(`crates/nvs-runtime/src/script.rs`'s *What crosses, and who owns it afterwards*). The class-table
question stays exactly as it is: it is about the context, not about the entry.

## Stage 3 — a serving core offers itself as a destination

A serving core registers its inbox as it starts, so a placement under `nvs serve` reaches a sibling
serving core round-robin over the cores this process already runs schedulers on, rather than starting
one of `crate::worker`'s lazily started cores beside them. That is ADR 0184 § 5 as written, and the
record's *Revisiting* trigger — placement starving the requests those cores exist to answer — is the
one thing that would take it back, on a measurement rather than on a session's judgement.

The lazily started cores stay, because they are what every other binary has: one scheduler is all
`nvs run`, a queue job and a test have.

## Standing decisions

- **The seam's shape is this goal's to choose, and the unit cache does not move.** Whatever carries a
  resolver to a core that has not started yet, `nvs-cli` goes on owning the table
  (`crates/nvs-cli/src/script.rs` § *Decision: one unit per written path*), and a `Resolver` gains no
  operation answering "the unit this thread is running" — that was refused where the seam is written
  and is not re-opened here.
- **A fall-through is not how a placement fails.** ADR 0184 § *Diagnostics* is the standing answer: a
  placement that cannot be honoured surfaces as the spawn failing, with `ok = false` and the reason in
  `error.message`, because a program that asked for another core and silently got this one is
  measuring a speedup that is not there. Where this goal removes a reason for the fall-through it does
  not add another.
- **What it spends is written per module**, per `rule:programs/memory-priority`: a published resolver
  handle is one per process, and a serving core registering an inbox adds no thread to a deployment
  that places nothing.
- **Not this goal**: work stealing between serving cores, which ADR 0184 rejects as a different
  decision; any change to what crosses, which `rule:security/isolate-values-cross-by-copy` owns.
