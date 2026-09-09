# Carried gaps — what a shipped feature still owes, and who owns it now

[carried-refusals.md](carried-refusals.md) does this for one kind of hole: a `nvs-ir` refusal site,
which `python tools/holes.py` can find on its own because the site is in the source. **This file is
for the other kind** — a gap that is real, written down in the module doc that owns it, and invisible
to every tool, because nothing in the tree is shaped wrong. A `Core\Log` record with two keys where
`rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active` names six compiles, tests green and ships.

**It exists because the handoff cannot hold one.** `docs/agent/handoff.md` is *state*: `tools/loop.py`
overwrites it with the next goal's seed at every switch, and `tools/goal-switch.py` carries the
outgoing goal's `[[check]]` blocks forward and nothing else. So a `## Backlog` bullet lives exactly
until the goal that wrote it goes green — which is how `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s cache redesign came to be "in the
handoff's backlog" according to a module doc, and in no file at all according to the repository, and
how `§18 stream` came to be filed under "goal `database`'s" six goals after goal `database` closed. Same failure,
same fix, one file up.

## The contract

- **An entry names an owner.** A goal number that is a live goal in
  [the goals directory](goals/), a **milestone tag** whose plan already covers the gap, or the
  word **`unowned`** with the reason it is nobody's yet. `unowned` is a legitimate state — it is a
  scheduling question for the user — but it is never the *absence* of an answer, and it is never what a
  *future* milestone's scheduled work is called. Goal `gap-owners` turns these three kinds into a gate.
- **The ratchet files carry the same column.** `crates/nvs-stdlib/tests/`'s four `*-outstanding.txt`
  lists write `# <owner>` after every key, and `every_outstanding_key_names_an_owner` in
  `spec_registry_coverage.rs` fails on one no goal answers for. Two kinds rather than three
  there: a key is struck by a session and only a chain entry runs sessions, so a milestone nobody has
  cut into goals reads as `unowned` on a key.
- **An entry leaves exactly one way: the gap is closed.** Not when it is rewritten, not when it stops
  being convenient. An entry whose owner went green without closing it is the failure this file
  exists to make visible; strike the owner, not the entry. The *Owner* column is the row's expiry
  declaration: `python tools/playbook.py --check` flags a row whose owner is retired in
  [the goals directory](goals/) and not yet struck. A § *Unowned* bullet has no owner to
  watch, so it ends with the `[until: ...]` trailer [tools/playbook.py](../../tools/playbook.py)'s
  module doc defines, naming the state of the tree that closes it.
- **One line of *what*, and a pointer to the module doc that owns the detail.** Every fact in this
  repository has one home, and for a gap that home is the module. This file is an index of who, not a
  second copy of what.
- **A session that finds a gap off its path writes it here**, not in the handoff, and moves on —
  [loop-authoring.md](loop-authoring.md) § 8's rule with a durable destination.

## Owned

Each of these is claimed by an entry on the chain and will be struck when that entry goes green.

| Gap | Owner | Where the detail lives |
|---|---|---|
| `nvs check` builds no grants, so `rule:core-classes/db-literal-query-checking`'s host diagnostic fires for nobody | 21 | `crates/nvs-types/src/intrinsics.rs` gap 6 |
| A cycle whose only closing edge is inside an `array<T>` survives `object::sweep` | 21 | `crates/nvs-runtime/src/object.rs` § *The five walks*, `rule:security/isolate-teardown-is-a-drain-then-a-sweep` |
| `Core\Db::stream`/`streamAs`, `Connection::close`, § 18's three readonly properties | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 5 |
| `{timeout?: Duration}` is in both spec signatures and in neither registry row | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 6 |
| `scope = "fleet"` parses, boots and is not armed | 21 | `crates/nvs-server/src/schedule.rs` § *What is not armed*, `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease` |
| `rule:core-classes/html-to-source`'s computed `$reason` is not refused — was blocked on a full diagnostic band | 21 | `crates/nvs-stdlib/src/html.rs` § *Known gaps* |
| `queryAs<T>`'s three refusals are at run time — same blocker, same band | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 8 |
| Twenty CLDR plural rosters throw; ordinals absent; eight pattern letters refused | 21 | `crates/nvs-stdlib/src/cldr.rs` gaps 2–4 |
| `Core\Request::clientIp`/`host`/`scheme`, `Response::html`/`sendFile` | 17 | `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` |
| `rule:testing/in-process-request` states two response readings rather than `Core\Test`'s signature, and spec § 13's `Core\Test` row is one English cell | 17 | `docs/rules/testing/in-process-request.md`, `docs/spec/01-core-library.md:999` |
| `goto` labels, grouped `use`, `var` as a property declarator, an enum case named with a keyword | 13 | `crates/nvs-syntax/src/lib.rs` § *Known gaps* — M1's own *Verify* is a `php-src` corpus parse |
| `nvs serve` runs on one core, and no path in the process starts a second | 23 | `crates/nvs-cli/src/serve.rs:42`, [m7.md](../plan/m7.md)'s own scope |
| `Core\Net`, `Core\Os`, `Core\Signal` — spec § 16, named by no milestone at all | 24 | `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` |
| `Core\Compress`, `Core\Mime`, `Core\Zip` — spec § 17 | 25 | the same file; `rule:core-api/tier-roster` puts all three at Tier 0 |
| `Core\Xml`'s tree and stream, and the two gaps behind it — `Core\Html::sanitize` and `rule:core-classes/html-parsing`'s parser | 26 | `crates/nvs-stdlib/src/html.rs` § *Known gaps*, `rule:core-classes/html-parsing` |
| ~110 module-doc `# Known gaps` items name no owner and are in no index | 27 | this file's own contract, applied one level down |
| `Core\Uri::with` replaces a component and cannot remove one | 28 | `crates/nvs-stdlib/src/uri.rs` gap 1 |
| `Core\Queue`'s `limits`/`grants` are undeclared and `$args` does not refuse a `secret` | 28 | `crates/nvs-stdlib/src/queue.rs` gaps 1–2 |
| `array<T>` is invariant — **decided: widen to a covariant read** | 28 | `crates/nvs-stdlib/src/lib.rs` gap 4, `nvs_types::expr::assign` |
| No custom panic hook — `rule:errors/helper-abi`; its blocker went away in M5 | 28 | `crates/nvs-runtime/src/lib.rs` gap 4 |
| `[limits] max_output` bounds no capture, at `Core\Process` or at `Core\IO::read` | 28 | `crates/nvs-stdlib/src/process.rs` gap 1 |
| The driver matrix has no socket leg, so `AF_UNIX` is asserted against no real server | 20 | `crates/nvs-db/src/matrix.rs` gap 1 |
| An integer where a grant expects a bool, a path or a list validates clean and grants nothing: `[capabilities.fs] read = 1` passes `nvs config check` at `0 warnings` and is denied at run time | 37 | `crates/nvs-config/src/tree.rs:50`'s untagged `Setting`, which every directive shares |

## Unowned

Nobody's, and each is a scheduling question rather than a session's. **Five entries.**
The six that stood here before were made reachable as goals `per-core` through `formats` and 29–31, and spec § 17's four
classes — filed
under an M9 that carries the extension system and none of them — are goals `formats` and `xml-tree` now. The two that
came back are the contract's second rule in plain sight: an owner that went green without closing its
gap is struck, not renamed. The last is the other way a gap arrives unowned: a rule answered in full,
by code that no key reaches.

- **`Core\Metrics`** — spec § 16's class, filed as goal `server`'s item 19 and left behind by it.
  `rule:observability/the-runtime-exports-what-it-already-measures`'s exporter, both of its config blocks and the nine
  metrics a core meters all landed; `registry::CLASSES` has no row for the class a program reads them
  through, so no Novis program can name one. `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.
  [until: exists crates/nvs-stdlib/src/registry.rs:metrics::CLASS]
- **`Core\Process::spawn`** — `proc_open`'s and `popen`'s streaming half, which
  `docs/spec/02-php-migration.md` points a migrating program at and which no entry on the chain builds.
  `Core\Process::run` is registered and is the whole of what there is.
  `crates/nvs-stdlib/tests/migration-members-outstanding.txt`.
  [until: exists crates/nvs-stdlib/src/process.rs:name: "spawn"]
- **`rule:security/arena-is-an-ownership-root`'s optional in-flight cycle
  collector**, for a long-running CLI script that builds cycles *between* teardowns. That ADR's
  *Consequences* says outright that it "remains open"; goal `carried-gaps`'s item 7 closes the *leak* at teardown
  and does not build the collector. This is an **open decision, not an unclosed gap**, and it stays
  here so that it stays visible. Its consequence shows up in one other place and that is not a
  duplicate: `nvs_safepoint` clears and ignores two of its four flags (`crates/nvs-runtime/src/lib.rs`
  gap 5), and `COLLECT` is inert because this collector does not exist. The other, `DEBUG_BREAK`, waits
  on `nvs dap` and is M10's. [until: reviewed 2026-09-06]
- **`rule:routing/a-shared-name-is-one-endpoint-everywhere` is shipped and unguarded.** Its three
  observations — `Core\Router::url` answers the one path, `Match::name` answers for every verb, the
  API document suffixes the operation with the verb — have no fixture: no `.nvst` under
  `tests/conformance/` declares two `#[Route]`s sharing a name, and `crates/nvs-cli/tests/openapi.rs`
  has no suffix case. The rows themselves are guarded by `crates/nvs-types/tests/routes.rs`. Found by
  the docs migration's sweep (unit C8), which could record it and not write it.
  [until: exists tests/conformance/core/a-shared-route-name-is-one-endpoint-everywhere.nvst]
- **A WebSocket connection's bounds are finite and configurable by nobody.**
  `rule:concurrency/connection-bounds-are-finite` asks for finite rather than for configurable, so
  `nvs-server`'s `bounds` answers it in full and every number is a constant: an operator who wants a
  different idle, lifetime, frame size or open-connection ceiling rebuilds. It is unowned rather than
  goal `config-is-written`'s because that goal closes keys that parse and reach no reader, and these are readers no key
  reaches — `nvs_config::tree::Server` has no field for any of them, so the audit cannot see them.
  `crates/nvs-server/src/bounds.rs` § *Known gap*, which names where the keys belong.
  [until: gone crates/nvs-server/src/bounds.rs:Known gap: none of these has a]

## What is *not* on either list

Two kinds of thing look like an unowned gap and are not, and saying so here is cheaper than each
session deciding again:

- **A gap a future milestone's plan already covers is scheduled work, not a hole.**
  `crates/nvs-cli/src/bundle.rs`'s `.nvsx` embedding is M9's; the inlining items in
  `crates/nvs-runtime/src/decimal.rs` and the string fast path in `crates/nvs-runtime/src/lib.rs` are
  M12's. Only a gap in a milestone that has *already been carried*, and that no chain entry claims, is
  unowned. Goal `gap-owners` makes this distinction machine-readable.
- **A decision is not a gap.** `crates/nvs-stdlib/src/time.rs`'s "there is no `Core\Month`, and there
  is not going to be one" and `crates/nvs-syntax/src/casing.rs`'s "left out deliberately" are settled
  positions that happen to sit under a `# Known gaps` heading. Goal `gap-owners` moves them out of the block, so
  the roster counts what is owed.

**The list this file indexes is not the whole inventory.** There are 50 `# Known gaps` blocks across the
crates holding 152 enumerated items; this file names the ones whose ownership needed an argument, and
[carried-refusals.md](carried-refusals.md) 901 covers `nvs-ir`'s. The rest were unindexed until goal `gap-owners`,
which puts the owner in the module doc beside the gap and derives the roster rather than copying it —
this file's own contract, applied one level down.
