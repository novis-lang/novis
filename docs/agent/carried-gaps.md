# Carried gaps — what a shipped feature still owes, and who owns it now

[carried-refusals.md](carried-refusals.md) does this for one kind of hole: a `nvs-ir` refusal site,
which `python tools/holes.py` can find on its own because the site is in the source. **This file is
for the other kind** — a gap that is real, written down in the module doc that owns it, and invisible
to every tool, because nothing in the tree is shaped wrong. A `Core\Log` record with two keys where
[ADR 0076](../adr/0076-observability-export.md) § 6 names six compiles, tests green and ships.

**It exists because the handoff cannot hold one.** `docs/agent/handoff.md` is *state*: `tools/loop.py`
overwrites it with the next goal's seed at every switch, and `tools/goal-switch.py` carries the
outgoing goal's `[[check]]` blocks forward and nothing else. So a `## Backlog` bullet lives exactly
until the goal that wrote it goes green — which is how ADR 0042's cache redesign came to be "in the
handoff's backlog" according to a module doc, and in no file at all according to the repository, and
how `§18 stream` came to be filed under "goal 5's" six goals after goal 5 closed. Same failure,
same fix, one file up.

## The contract

- **An entry names an owner.** A goal number that is a live `[[goal]]` in
  [goals/chain.toml](goals/chain.toml), a **milestone tag** whose plan already covers the gap, or the
  word **`unowned`** with the reason it is nobody's yet. `unowned` is a legitimate state — it is a
  scheduling question for the user — but it is never the *absence* of an answer, and it is never what a
  *future* milestone's scheduled work is called. Goal 27 turns these three kinds into a gate.
- **The ratchet files carry the same column.** `crates/nvs-stdlib/tests/`'s four `*-outstanding.txt`
  lists write `# <owner>` after every key, and `every_outstanding_key_names_an_owner` in
  `spec_registry_coverage.rs` fails on one no `[[goal]]` answers for. Two kinds rather than three
  there: a key is struck by a session and only a chain entry runs sessions, so a milestone nobody has
  cut into goals reads as `unowned` on a key.
- **An entry leaves exactly one way: the gap is closed.** Not when it is rewritten, not when it stops
  being convenient. An entry whose owner went green without closing it is the failure this file
  exists to make visible; strike the owner, not the entry.
- **One line of *what*, and a pointer to the module doc that owns the detail.** Every fact in this
  repository has one home, and for a gap that home is the module. This file is an index of who, not a
  second copy of what.
- **A session that finds a gap off its path writes it here**, not in the handoff, and moves on —
  [loop-authoring.md](loop-authoring.md) § 8's rule with a durable destination.

## Owned

Each of these is claimed by an entry on the chain and will be struck when that entry goes green.

| Gap | Owner | Where the detail lives |
|---|---|---|
| `nvs check` builds no grants, so ADR 0067 § 10's host diagnostic fires for nobody | 21 | `crates/nvs-types/src/intrinsics.rs` gap 6 |
| A cycle whose only closing edge is inside an `array<T>` survives `object::sweep` | 21 | `crates/nvs-runtime/src/object.rs` § *The five walks*, ADR 0116 § 2 |
| `Core\Db::stream`/`streamAs`, `Connection::close`, § 18's three readonly properties | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 5 |
| `[db.<name>.pool]` has no spelling, and `pool = false` cannot reach a program-opened connection | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 1 |
| `{timeout?: Duration}` is in both spec signatures and in neither registry row | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 6 |
| A `Core\Log` record carries `level` and `msg` and none of § 6's other four | 21 | `crates/nvs-stdlib/src/log.rs` § *The envelope* |
| `scope = "fleet"` parses, boots and is not armed | 21 | `crates/nvs-server/src/schedule.rs` § *What is not armed*, ADR 0073 § 3 |
| ADR 0133 § 3's computed `$reason` is not refused — was blocked on a full diagnostic band | 21 | `crates/nvs-stdlib/src/html.rs` § *Known gaps* |
| `queryAs<T>`'s three refusals are at run time — same blocker, same band | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 8 |
| Twenty CLDR plural rosters throw; ordinals absent; eight pattern letters refused | 21 | `crates/nvs-stdlib/src/cldr.rs` gaps 2–4 |

| A route capture reaches the handler still percent-encoded | 21 | `crates/nvs-runtime/src/routes.rs` gap 3 |
| ADR 0042's artifact cache is written, tested and has no caller | 22 | `crates/nvs-cli/src/cache.rs` § *Known gaps* |
| `Core\Request::clientIp`/`host`/`scheme`, `Response::html`/`sendFile` | 17 | `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` |
| `goto` labels, grouped `use`, `var` as a property declarator, an enum case named with a keyword | 13 | `crates/nvs-syntax/src/lib.rs` § *Known gaps* — M1's own *Verify* is a `php-src` corpus parse |
| `nvs serve` runs on one core, and no path in the process starts a second | 23 | `crates/nvs-cli/src/serve.rs:42`, [m7.md](../plan/m7.md)'s own scope |
| `Core\Net`, `Core\Os`, `Core\Signal` — spec § 16, named by no milestone at all | 24 | `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` |
| `Core\Compress`, `Core\Mime`, `Core\Zip` — spec § 17 | 25 | the same file; ADR 0051 § 3 puts all three at Tier 0 |
| `Core\Xml`'s tree and stream, and the two gaps behind it — `Core\Html::sanitize` and ADR 0122's parser | 26 | `crates/nvs-stdlib/src/html.rs` § *Known gaps*, ADR 0122 § 4 |
| ~110 module-doc `# Known gaps` items name no owner and are in no index | 27 | this file's own contract, applied one level down |
| `Core\Uri::with` replaces a component and cannot remove one | 28 | `crates/nvs-stdlib/src/uri.rs` gap 1 |
| `Core\Queue`'s `limits`/`grants` are undeclared and `$args` does not refuse a `secret` | 28 | `crates/nvs-stdlib/src/queue.rs` gaps 1–2 |
| `array<T>` is invariant — **decided 2026-09-05: widen to a covariant read** | 28 | `crates/nvs-stdlib/src/lib.rs` gap 4, `nvs_types::expr::assign` |
| No custom panic hook — ADR 0002 § *Corollary*; its blocker went away in M5 | 28 | `crates/nvs-runtime/src/lib.rs` gap 4 |
| `[limits] max_output` bounds no capture, at `Core\Process` or at `Core\IO::read` | 28 | `crates/nvs-stdlib/src/process.rs` gap 1 |

## Unowned

Nobody's, and each is a scheduling question rather than a session's. **Three entries, on 2026-09-05.**
The six that stood here before were made reachable as goals 23–28, and spec § 17's four classes — filed
under an M9 that carries the extension system and none of them — are goals 25 and 26 now. The two that
came back are the contract's second rule in plain sight: an owner that went green without closing its
gap is struck, not renamed.

- **`Core\Metrics`** — spec § 16's class, filed as goal 6's item 19 and left behind by it.
  [ADR 0076](../adr/0076-observability-export.md)'s exporter, both of its config blocks and the nine
  metrics a core meters all landed; `registry::CLASSES` has no row for the class a program reads them
  through, so no Novis program can name one. `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.
- **`Core\Process::spawn`** — `proc_open`'s and `popen`'s streaming half, which
  `docs/spec/02-php-migration.md` points a migrating program at and which no entry on the chain builds.
  `Core\Process::run` is registered and is the whole of what there is.
  `crates/nvs-stdlib/tests/migration-members-outstanding.txt`.
- **[ADR 0116](../adr/0116-an-isolates-arena-is-an-ownership-root.md)'s optional in-flight cycle
  collector**, for a long-running CLI script that builds cycles *between* teardowns. That ADR's
  *Consequences* says outright that it "remains open"; goal 21's item 7 closes the *leak* at teardown
  and does not build the collector. This is an **open decision, not an unclosed gap**, and it stays
  here so that it stays visible. Its consequence shows up in one other place and that is not a
  duplicate: `nvs_safepoint` clears and ignores two of its four flags (`crates/nvs-runtime/src/lib.rs`
  gap 5), and `COLLECT` is inert because this collector does not exist. The other, `DEBUG_BREAK`, waits
  on `nvs dap` and is M10's.

## What is *not* on either list

Two kinds of thing look like an unowned gap and are not, and saying so here is cheaper than each
session deciding again:

- **A gap a future milestone's plan already covers is scheduled work, not a hole.**
  `crates/nvs-cli/src/bundle.rs`'s `.nvsx` embedding is M9's; the inlining items in
  `crates/nvs-runtime/src/decimal.rs` and the string fast path in `crates/nvs-runtime/src/lib.rs` are
  M12's. Only a gap in a milestone that has *already been carried*, and that no chain entry claims, is
  unowned. Goal 27 makes this distinction machine-readable.
- **A decision is not a gap.** `crates/nvs-stdlib/src/time.rs`'s "there is no `Core\Month`, and there
  is not going to be one" and `crates/nvs-syntax/src/casing.rs`'s "left out deliberately" are settled
  positions that happen to sit under a `# Known gaps` heading. Goal 27 moves them out of the block, so
  the roster counts what is owed.

**The list this file indexes is not the whole inventory.** There are 50 `# Known gaps` blocks across the
crates holding 152 enumerated items; this file names the ones whose ownership needed an argument, and
[carried-refusals.md](carried-refusals.md) 901 covers `nvs-ir`'s. The rest were unindexed until goal 27,
which puts the owner in the module doc beside the gap and derives the roster rather than copying it —
this file's own contract, applied one level down.
