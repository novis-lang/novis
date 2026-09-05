# The parity program, as six loop goals

Goals 1–6 of [the plan](../../implementation-plan.md) are one continuous unattended run: **PHP core
feature parity, all five SQL drivers, concurrency, governance and the server.** This directory holds it,
cut into six goals, and [chain.toml](chain.toml) is the order the driver walks them in. A seventh,
post-parity goal — [ADR 0131](../../adr/0131-a-temporary-directory-dies-with-its-script-and-the-sweep-never-throws.md)'s
temporary-directory sweep — rides the same chain after the program's gate, because its server half needs
the `nvs-server` goal 6 creates. An eighth, [`Core\Program::id()`](8-program-id.md), follows it: one
member exposing the program fingerprint over hashes the artifact cache already computes. A ninth,
[`Core\Db\Schema`](9-schema.md), closes [ADR 0067](../../adr/0067-core-db.md)'s own *Revisiting* item and
sits there because its acceptance property needs every driver goal 5 builds to be finished. A tenth,
[a typed `callable`](10-typed-callable.md), follows it: [ADR 0136](../../adr/0136-a-callable-carries-its-signature.md)
gives the type a function value's parameters and return, and it goes after every goal that *writes*
callbacks so their registry rows are converted once rather than twice. An eleventh,
[doc comments](11-doc-comments.md), is last —
[ADR 0137](../../adr/0137-a-doc-comment-is-three-slashes-and-two-tags.md)'s `///`, whose stage 2 builds
ADR 0099 § 1's trivia layer because a doc comment cannot be read without it, so **M4B starts with its
own tree half already done**.

**Then M4B itself, as four goals** — [12 resilient-tree](12-resilient-tree.md),
[13 surface](13-surface.md), [14 lsp-server](14-lsp-server.md) and [15 editor](15-editor.md). Goal 13 is
not editor work: it is ADR 0098's pipeline operator and ADR 0124's PHP 8.6 refusals, the two M1 items
scheduled after M4 and never taken, and it sits before the grammar because a grammar written against a
surface about to change is written twice.

**Then two entries the user added after the chain was written**, both about the same thing from two sides:
what a program may read off a request, and what a test may say to build one.
[16 request-json](16-request-json.md) replaces spec § 15's three-way body exclusivity with ADR 0139's
*buffering readers share, streaming readers consume*, adds `Core\Request::json()`/`jsonAs<T>()`, and gives
`.nvst` the `.phpt` request sections — without which no request-facing member can be proven by a case at
all. [17 test-request](17-test-request.md) freezes `Core\Test::request`'s shape (ADR 0079 § 18 has an
example and no signature), builds it as one shared `InboundSpec`, and lands the peer fields that
`Core\Request::clientIp`/`scheme`/`host` have been waiting on. They are last rather than beside goal 6
because they were decided after it, and goal 16's stage 0 is what pays off the fixtures goal 6 wrote
against the rule it replaces.

**Then a third, from the same conversation**: [18 input-shapes](18-input-shapes.md) is what stops a
request reader's answer being `mixed`. ADR 0140 gives `Core\Arr` one converter from `array<mixed>` to a
declared shape and `Core\Request` the two members over it, so untrusted data is checked once, where it
arrives and where a `400` is still the right answer. Its stage 2 is type-surface work the other two need
nothing of and everything before it would have had to write twice — ADR 0036 § 3's shape gains an
optional field, and ADR 0024 § 1's qualifier learns to sit in front of one — which is why it comes after
the other two.

**Then a fourth**: [19 parses](19-parses.md) is the one the user asked for after that conversation ended.
Four binding surfaces — a route `{capture}`, a `#[Query]`, a command argument and an option — all ask
`nvs_types::commands::converts_from_string` whether a type can be built from text, and its class arm is a
comparison against the string `Core\Uuid`. ADR 0141's `Parses` is what that arm becomes: one global
interface on `Comparable`'s precedent, one required member and one default body, so `Core\Uuid` reaches
the door through the same contract as a user's own `Slug` and stops being a name in four match arms. Its
stage 4 closes `nvs-runtime`'s two standing conversion gaps, which is why it goes after the goals that
wrote them. Goal 19 is the last entry the parity program itself needs.

**What goal 19 deliberately does not do**: give `as` a class-building meaning. That was the shape the
proposal arrived in, and the operator half was rejected — `mixed as Foo` is already a checked downcast,
`as` is a closed laundering set, and `X as ?Foo` would need two inputs to decide its legality. ADR 0066
§ 3's *the class row is absolute* survives intact; only the sites that already convert implicitly change.

**Then one more, added after goal 19 was written**: [20 unix-sockets](20-unix-sockets.md) is
[ADR 0142](../../adr/0142-a-configured-store-is-authorized-by-its-configuring.md) — the grant over a
store an operator configured stops naming a host (`cache.shared`, unscoped, on `mail.send`'s precedent),
which removes the loopback double-grant *and* the obstacle to a Unix socket, since `pin_host` needed an
address and a socket path has none. Its stage 4 puts an `AF_UNIX` connect under three of goal 5's
drivers. It goes in front of the dossier because that entry stops adding surface and this one adds some.

**Then, before any of that, two entries the user added on 2026-09-05** — inserted directly after goal 6
rather than on the end, because goal 6 going green is what *makes* the problem they close.
[21 carried-gaps](21-carried-gaps.md) takes every gap a shipped feature already carries that no entry on
this chain claimed: an ADR-written `db.open` wildcard with no reader, `nvs check` never building the
grants its own diagnostic needs, a cycle closed through an array surviving ADR 0116 § 2's sweep, ADR
0076 § 6's four missing log-record fields, ADR 0073 § 3's unarmed fleet lease, spec § 18's
`stream`/`streamAs`, two rules that were waiting on a diagnostic band that has since opened, and the
CLDR rosters that throw. Its keystone is the mechanism rather than any of those: an outstanding-members
key gains an owner column and the test fails when that owner is no longer a live entry, so a switch
cannot orphan work silently again. [22 warm-start](22-warm-start.md) is ADR 0042's artifact cache, which
goal 3 built exactly as specified and which has never had a caller — a subsystem rather than a gap,
because the payload needs a second `nvs-codegen` `Module` and a named symbol for every host address the
JIT bakes in, which is why it is its own entry.

**Then six entries the user asked for on 2026-09-05**, from one question — what is *unowned*, and can it
be made reachable? Answering it turned up two facts the repository had wrong (`Core\Metrics` was listed
unowned and is goal 6's; spec § 17's four classes were filed under M9, which carries the extension system
and none of them) and one it did not record at all: **50 `# Known gaps` blocks across the crates hold 152
enumerated items**, of which `carried-gaps.md` indexed 22 and `carried-refusals.md` 15. These six close
the real ones. [23 per-core](23-per-core.md) is M7's own scope that goal 6 shipped around, and the largest
measured performance item in the repository. [24 net-os-signal](24-net-os-signal.md),
[25 formats](25-formats.md) and [26 xml-tree](26-xml-tree.md) are M8's Tier 0 roster finished — the seven
`Core` classes ADR 0051 § 3 names that M8's own goals walked past, after which
`spec-classes-part-two-outstanding.txt` holds no keys at all. [27 gap-owners](27-gap-owners.md) is goal
21's keystone applied one level down: a module-doc gap gains an owner tag and a gate fails on an untagged
one, so the ~110 unindexed items become a short list of scheduling questions instead of an unread
inventory. [28 unowned-sweep](28-unowned-sweep.md) closes what is left, four fifths of which is one
blocker — an options bag the registry could not spell, which is what goal 18 lands.

**Then the chain turns around.** [50 dossier](50-dossier.md) is the last hand-written entry and it writes
no proof of its own: one session runs `python tools/dossier.py --emit-goals --append-chain
docs/agent/goals/chain.toml`, which puts [ADR 0134](../../adr/0134-every-shipped-feature-owes-four-proofs.md)'s
whole roster — one goal per group of shipped features owing their four proofs — onto the end of *this*
chain, and then spends the rest of the session on an **optimization pass aimed forward** rather than
back: it is the only moment anyone holds all 93 generated goals at once and none of them has been walked,
so the shape they share is cheapest to fix there. The sweep on 2026-09-04 said 795 features with one
complete, which emits as 93 goals over 794 owed. That pass is why
[optimization-prompt.md](../optimization-prompt.md) now carries menu item 8 — a defect in a generated
goal is fixed in the emitter and re-emitted, never by hand — which is also what the *automatic* pass
reaches for once the loop is walking goals a tool wrote. Everything from goal 51 on is therefore generated, and the run continues into it without a
restart: `Chain.refresh()` re-reads this file when a goal goes green, adopting anything past the goal the
run is on and refusing a rewrite of one it has already walked — `.loop/chain.json` is an index into the
list and every switch has folded one walked entry's checks into the next, which is what a rewrite behind
the run would invalidate. Nothing has been folded into an entry the run has not reached, so a hand-written
goal may be **inserted** in front of the dossier mid-run, not only appended after it. The dossier is last
for the reason a proof is written at all — it pins behaviour, and behaviour that is still moving is not
worth pinning.

[loop-authoring.md](../loop-authoring.md) owns how a goal is *written* and [coordinator.md](../coordinator.md)
owns how one is *driven*. This file owns only what is specific to running six of them back to back, and it
does not restate either.

## Why six and not one

A goal is a finite contained group of work, and the `[context]` manifest is what keeps a session under the
200k ceiling. One goal spanning `nvs-syntax` through a TDS driver would need a manifest naming every
module in the workspace, and every byte of it is charged to every session — including the ones that never
open a driver. Six manifests, each naming the six-to-fifteen modules its own goal touches, is the same
work at a fraction of the per-session cost.

The split is **by file set, not by topic**. That is why M8 is two goals — `nvs-db` shares nothing with
`Core\Cli` — and why M5's reactor and its isolates are one, since both are `nvs-host`.

| Goal | Milestone | Crates it opens |
|---|---|---|
| [1 core-depth](1-core-depth.md) | M4S tail | `nvs-stdlib`, `nvs-types`, `nvs-hir`, `nvs-cli` |
| [2 concurrency](2-concurrency.md) | M5 | **`nvs-host`** (new), `nvs-runtime`, `nvs-stdlib` |
| [3 governance](3-governance.md) | M6 | **`nvs-config`** (new), `nvs-host`, `nvs-cli`, `nvs-codegen` |
| [4 core-part-ii](4-core-part-ii.md) | M8, non-database | `nvs-stdlib`, `nvs-host` |
| [5 database](5-database.md) | M8, database | **`nvs-db`** (new), `nvs-stdlib`, `nvs-types` |
| [6 server](6-server.md) | M7 | **`nvs-server`** (new), `nvs-stdlib`, `nvs-host` |
| [7 temp-sweep](7-temp-sweep.md) | post-parity, ADR 0131 | `nvs-runtime`, `nvs-host`, `nvs-stdlib`, `nvs-config`, `nvs-server`, `nvs-cli` |
| [8 program-id](8-program-id.md) | post-parity, ADR 0061 amendment | `nvs-config`, `nvs-hir`, `nvs-runtime`, `nvs-stdlib` |
| [9 schema](9-schema.md) | post-parity, one ADR slot | `nvs-db`, `nvs-stdlib`, `nvs-cli` |
| [10 typed-callable](10-typed-callable.md) | post-parity, ADR 0136 | `nvs-syntax`, `nvs-types`, `nvs-stdlib`, `nvs-ir`, `nvs-codegen`, `nvs-runtime` |
| [11 doc-comments](11-doc-comments.md) | post-parity, ADR 0137 + M4B's tree half | `nvs-syntax`, `nvs-diagnostics`, `nvs-hir`, `nvs-cli` |
| [12 resilient-tree](12-resilient-tree.md) | M4B, ADR 0099 § 1's other half | `nvs-syntax`, `nvs-diagnostics`, `nvs-test`, `nvs-cli` |
| [13 surface](13-surface.md) | M1 items 5-6, ADR 0098 + ADR 0124 | `nvs-syntax`, `nvs-diagnostics` |
| [14 lsp-server](14-lsp-server.md) | M4B, ADR 0099 §§ 3+5 + ADR 0101 | **`nvs-lsp`** (new), `nvs-cli`, `nvs-types`, `nvs-stdlib` |
| [15 editor](15-editor.md) | M4B, ADR 0099 §§ 4+6 | **`editors/vscode`** (new, TypeScript) |
| [16 request-json](16-request-json.md) | M7, ADR 0139 + spec § 15 | `nvs-stdlib`, `nvs-runtime`, `nvs-test`, `nvs-cli` |
| [17 test-request](17-test-request.md) | M8, ADR 0079 § 18 | `nvs-runtime`, `nvs-stdlib`, `nvs-server`, `nvs-test`, `nvs-cli` |
| [18 input-shapes](18-input-shapes.md) | M7, ADR 0140 + ADR 0036/0024 amendments | `nvs-syntax`, `nvs-types`, `nvs-stdlib` |
| [19 parses](19-parses.md) | M7, ADR 0141 + ADR 0013/0066/0077/0102 amendments | `nvs-hir`, `nvs-types`, `nvs-stdlib`, `nvs-runtime`, `nvs-cli` |
| [20 unix-sockets](20-unix-sockets.md) | M8, ADR 0142 + ADR 0058/0059 amendments | `nvs-config`, `nvs-stdlib`, `nvs-db`, `nvs-host`, `nvs-diagnostics` |
| [21 carried-gaps](21-carried-gaps.md) | post-parity, ADR 0067/0073/0076/0116/0133 amendments | `nvs-config`, `nvs-cli`, `nvs-types`, `nvs-runtime`, `nvs-stdlib`, `nvs-server`, `nvs-db`, `nvs-diagnostics` |
| [22 warm-start](22-warm-start.md) | post-parity, ADR 0042 | `nvs-codegen`, `nvs-cli`, `nvs-config` |
| [23 per-core](23-per-core.md) | M7, one ADR slot + ADR 0097/0017 amendments | `nvs-cli`, `nvs-host`, `nvs-server`, `nvs-config` |
| [24 net-os-signal](24-net-os-signal.md) | M8, one ADR slot + ADR 0142 § 6's deferred grant | `nvs-stdlib`, `nvs-host`, `nvs-config`, `nvs-runtime` |
| [25 formats](25-formats.md) | M8, one ADR slot (the shared decompression bound) | `nvs-stdlib`, `nvs-config`, `nvs-diagnostics` |
| [26 xml-tree](26-xml-tree.md) | M8, one ADR slot + ADR 0122 § 4's fold | `nvs-stdlib`, `nvs-diagnostics` |
| [27 gap-owners](27-gap-owners.md) | post-parity, no ADR — a process gate | `tools/`, every crate's module docs |
| [28 unowned-sweep](28-unowned-sweep.md) | post-parity, ADR 0002/0033/0044 amendments | `nvs-stdlib`, `nvs-types`, `nvs-runtime` |
| 29–49 | free | the gap the dossier's number leaves, so a new hand-written entry costs one `[[goal]]` block and no renumber |
| [50 dossier](50-dossier.md) | ADR 0134 | none — it writes the goals that open all of them, then optimizes the loop for their shape |
| 51 onward | ADR 0134, generated | one group of features per goal, its own `[context]` manifest |

## The chain contract

**`python tools/chain.py` is how an entry is added, reordered, renumbered or retired** — it scaffolds the
three files below, splices the `[[goal]]` block without touching a comment it did not mean to, refuses an
edit behind the live entry, and `--check` says whether every entry is one the driver can walk.
[commands.md](../commands.md) is the tool's one home; this section is the contract it enforces.

Each goal is three files, named for it:

| File | Holds |
|---|---|
| `N-<name>.md` | the target, the item list grouped by file set, the standing decisions |
| `N-<name>.toml` | the acceptance test as data, and the `[context]` manifest |
| `N-<name>.handoff.md` | the handoff the switch seeds, naming that goal's first group |

Three rules bind every one of them, and they are the reason the run can be left alone:

1. **A goal's acceptance list is the next goal's floor, mechanically.** `tools/goal-switch.py` copies every
   `[[check]]` out of the live `loop-goal.toml` into the next goal's own marker line, relabelled to the
   floor stage. Nothing is copied by hand. By goal 6 the floor is five goals deep, which is the point —
   the parity claim is only worth something if nothing under it was traded away to reach it.
2. **Every goal names the numbered ADRs it may open, and no session opens another.** The blanket "do not
   open a numbered ADR" rule that M4's goal carried does not survive this program: goals 2–6 contain
   genuinely new designs — a reactor, a driver's wire I/O, a pool reset that is a security boundary — and
   a design of that size recorded as a paragraph in `docs/adr/README.md` is a design nobody can find
   later. So each goal's *Standing decisions* carries a short list of **ADR slots**, each one the first
   slice of the goal that needs it. Anything not on that list is still decided-and-recorded, never
   `BLOCKED`, and never a new number.
3. **A goal that cannot verify itself does not advance.** The driver runs the acceptance test; a session
   claiming `DONE` against a red check stops the run, exactly as it does today.

## Starting the chain

Three steps.

1. Confirm the goal the repository is currently running is green: `python tools/loop.py --goal-only`.
   Whatever that goal is becomes goal 1's floor, so this is not a formality — it is the moment the floor
   is decided.
2. `python tools/loop-stats.py` and `python tools/loop-stats.py --attribute`.
   [loop-authoring.md](../loop-authoring.md) § 1 makes this step zero and § 9 says the numbers move. Set
   the slice budget from what it prints and **say which and why in the commit**. The 200k ceiling is not
   a number to re-derive; the *projection* is.
3. `python tools/loop.py --chain docs/agent/goals/chain.toml --max-sessions <n>`.

**The driver does the switching, including the first one.** It runs `tools/goal-switch.py` against the
entry it is about to install — which folds the live goal's whole acceptance list in as that entry's floor
— copies the three files into `docs/agent/loop-goal.md`/`.toml` and `docs/agent/handoff.md`, commits that
switch, and starts the session. On `GOAL REACHED` it does the same for the next entry and keeps going.
Without `--chain` the run stops six times and waits for a human, which is the same run with five extra
nights in it.

`.loop/chain.json` records which entry is installed, and it is what makes "exactly once per entry" a fact
rather than an intention: **`goal-switch.py` is not idempotent** — it inserts at a marker it leaves in
place, so running it twice inserts the floor twice. If you ever delete that state file, check the entry's
TOML for a doubled floor before restarting.

## What stops the run

- **The last goal goes green** — which, since goal 50, means the last *generated* one: every group on
  ADR 0134's roster owing nothing, `python tools/dossier.py --gate` exiting 0 over the whole language.
  The parity program's own gate is still goal 6's final check —
  `python tools/check-migration.py` reporting 100% classified — every one of the oracle build's **1151
  functions and 253 types** accounted for, every `member` row registered, every one of them cased. The
  inventory grew from 925 on 2026-08-29, when the oracle build gained `mysqli`, `pgsql` and `sqlite3`:
  the three APIs [ADR 0067](../../adr/0067-core-db.md) replaces are now inside the audit rather than a
  named hole beside it. Goals 7 through 11 going green, in chain order, is then what ends the run.
- **A goal reports `BLOCKED`.** Reserved for a decision that is expensive to reverse *and* has no safe
  default. Every goal's standing decisions exist to make this rare.
- **`--max-stalls` consecutive sessions move `HEAD` nowhere.**
- **Goal 5's Docker preflight fails.** ADR 0067 verifies the drivers against real servers, so the driver
  checks for a reachable daemon before the first session of that goal and stops the run naming it. A run
  that grinds for six hours against a check that cannot pass is worse than one that stops in the first
  minute.

## What this program does not touch

`Web\Migration` and everything versioned about a schema change — ordering, history tables, fleet
locking, reversibility — which [ADR 0082](../../adr/0082-the-first-party-framework.md) § 7 records as
deliberately blocked. Goal 9 builds convergence, which needs none of them, and does not close that gap.

Doc trimming and dependency sweeps, both of which the user fires and never a session
([doc-cleanup.md](../doc-cleanup.md), [dependency-update.md](../dependency-update.md)). And **PHP's
optional extensions** — `gd`, `intl`, `imap`, `zip` and the rest of the unaudited list in
[02-php-migration.md](../../spec/02-php-migration.md) — are not parity work: they are M9's, and a session
that finds one on its path puts it in the handoff's `## Backlog` and moves on.
