---
milestone: M8
---
# Loop goal 24 — `Core\Net`, `Core\Os` and `Core\Signal`

Spec § 16's last three unregistered classes, and the only three in the whole part that **no milestone
and no goal named at all**. When this goal is green a Novis program can open a socket the runtime's own
reactor drives, ask the host what process it is, and be told to shut down gracefully — and
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` is three keys shorter.

**They are M8's**, not a future milestone's:
`rule:core-api/tier-roster` puts all three at Tier 0 (`Core\Net` at
`0051:143`, `Core\Os` at `0051:101`, `Core\Signal` at `0051:195`), and M8 is the milestone that builds
Core's stdlib. M8's stdlib goals — 4 and 5 — have walked without them, which is what made these a gap
in a past milestone rather than work on a future one.

**It sits after goal `per-core`** because `Core\Os::cpuCount` should answer the number the server actually
fans out over, not a number nothing reads.

## Stage 0 — the catch-up

1. **The three rows in `docs/spec/01-core-library.md` § 16** are the signatures, and they are thin —
   one sentence each. Each class's surface is settled *here*, in this goal's stages, and the spec row
   is edited to match rather than the other way round. That is the one place this goal writes the spec.
2. **`Core\Net` has no ADR of its own** — `rule:core-api/tier-roster`'s row is the whole design on disk. Stage 2's
   first act is the ADR this goal is allowed to open.

## Stage 1 — the floor

Goal `per-core`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — `Core\Net`, over the reactor and nothing else

1. **The ADR.** One number for the socket surface: what a program may open, what the capability
   answers, and why there is no second event loop — "over the runtime's own reactor" is `rule:core-api/tier-placement`'s
   sentence and this is where it becomes a rule.
2. **`net.local` joins the capability roster.**
   `rule:config/net-local-is-named-and-not-on-the-roster` names it and
   goal `unix-sockets` deliberately did **not** add it — "it has no caller until `Core\Net` lands". This goal is
   that caller. `net.connect` keeps the outbound host scope it has; a listening socket is a separate
   question and gets `net.listen`.
3. **TCP.** `connect`, and a listener that accepts — over `nvs_host::NvsTcp`/`NvsListener`
   (`crates/nvs-host/src/net.rs:227`, `:313`), which park on the reactor and are what the drivers and
   the server already speak through. A program-supplied host walks
   `rule:security/net-address-policy`'s denied-range table through
   `nvs_runtime::capability::pin_host`, unchanged — goal `unix-sockets`'s carve-out was for *configured* stores and
   does not reach a program's own `connect`.
4. **Unix.** `NvsUnix` exists (`net.rs:460`) and needs no new transport. A program-supplied socket
   path is refused at this door under goal `unix-sockets`'s standing decision — refused as a *target*, so it cannot
   be told apart from a path that could not be opened.
5. **UDP is new to the host.** `nvs-host` has no datagram type; `NvsUdp` over `mio::net::UdpSocket` is
   this goal's one addition to that crate, written to the same shape as `NvsStream` so it parks the
   coroutine rather than blocking the core.
6. **No scheme dispatch anywhere** — `rule:security/a-path-is-not-a-url`. A socket is
   opened by an address, never by a URL that something interprets.

## Stage 3 — `Core\Os`, five facts about the host

1. **`pid`, `hostname`, `cpuCount`, `memoryUsage`, `loadAverage`** — the spec § 16 row, whole.
   `cpuCount` answers `std::thread::available_parallelism`, which after goal `per-core` is the number the
   server fans out over; `crates/nvs-cli/src/info.rs:145` already calls it and stops being the only
   caller.
2. **`loadAverage` has no Windows answer**, and the member says so rather than inventing one — it
   throws with a message naming the platform, which is the same shape `Core\Path`'s Windows-only
   members already use.
3. **Neutral on both qualifier axes, and none of it is a capability.** These are facts about the
   process, not about the world: `pid` and `hostname` are not `tainted` (nothing outside the host wrote
   them) and nothing here is a sink.

## Stage 4 — `Core\Signal`, graceful shutdown and nothing else

1. **The roster is closed at shutdown.** `rule:core-api/tier-roster`'s row is "what remains of `pcntl_*` after `fork`
   is refused", and the surface is a handler for the terminating signals and nothing that resembles
   job control. There is no `kill`, no `alarm` and no signal number as an integer.
2. **It composes with the drain that already exists.** `rule:concurrency/a-drain-closes-a-connection-cleanly`'s drain answers the probe and
   `isDraining()`; a `Core\Signal` handler is the CLI-side entry into the same state machine, not a
   second one. On the server the signal path is the process's, and after goal `per-core` it is fleet-wide.
3. **A handler runs as ordinary Novis code at a safepoint**, never in a signal context — the delivery
   sets a flag the safepoint reads, which is the shape `nvs_safepoint` already has.

## Stage 5 — the three keys are struck

1. **`spec-classes-part-two-outstanding.txt` loses `§16 Core\Net`, `§16 Core\Os` and
   `§16 Core\Signal`** — registering a class and striking its line are one slice, and that file's test
   fails on a stale line as loudly as on an unlisted one. `§16 Core\Budget` is stage 6's to strike.
2. **The migration table's rows** for `socket_*`, `stream_socket_*`, `fsockopen`, `posix_*` minus fork,
   `php_uname`, `getrusage`, `sys_getloadavg` and the surviving `pcntl_*` answer a
   `Core` spelling, and `migration-members-outstanding.txt` shrinks by whatever they name.

## Stage 6 — `Core\Budget`'s three numbers, and the peak that reaches an operator unasked

1. **`Core\Budget::memoryHeld`, `memoryPeak` and `memoryLimit`** are registered, over the counter
   `crates/nvs-runtime/src/budget.rs` already keeps in every build, and `Core\Os::residentBytes`
   answers the process question beside them ([0148](../../decisions/0148.md) §§ 11-12). The four keys
   leave `migration-members-outstanding.txt` and `§16 Core\Budget` leaves
   `spec-classes-part-two-outstanding.txt`.
2. **The high-water mark is recorded in `budget::add`**, inside the branch that already tests for a
   positive delta, and a nested `Ctx` restores `max(enclosing, reached)` on drop rather than
   clobbering the mark of the request that spawned it (0148 § 15). A conformance case allocates a
   known-size buffer, drops it, and reads a peak above the drop against a held figure below it; a
   second asserts a parent's peak survives a child that allocated less. The cost claim is measured in
   `benches/abi-probe`, never asserted.
3. **The three unasked readings land**: `Script\ExitReport::memoryPeak`, the
   `nvs_request_memory_peak_bytes` histogram beside the other default series, and the
   `[limits] memory_high_water` fraction whose crossing writes one `Warn` (0148 § 14). Unwritten is
   off, off is silent, and a value outside `0.0..=1.0` is refused at boot by the typed-value path
   `crates/nvs-config` already has.

## Standing decisions

- **This goal may open one ADR number** — the `Core\Net` socket surface. `Core\Os`, `Core\Signal` and
  `Core\Budget` are `rule:core-api/tier-roster` rows with no design question left between them and
  get none: the one that existed is settled ahead of the goal in
  [0148](../../decisions/0148.md), which is why stage 6 builds rather than decides.
- **A second event loop is never the answer.** Every socket parks on the runtime's reactor. If a
  shape cannot be expressed that way the shape is cut, not the rule — this is what "over the runtime's
  own reactor rather than a second event loop" already decides.
- **`net.listen` and `net.local` are separate grants, and neither widens `net.connect`.** A program
  that may reach a host may not therefore bind one, and a program that may open a local socket may not
  therefore reach the network.
- **UDP gets no reliability layer.** Datagrams are datagrams; anything resembling ordering or
  retransmission is the program's, not the library's.
- **Ambiguity about surface resolves toward the narrowest member that answers the migration table's
  row**, recorded in the module doc. A PHP function with no Novis answer is a `dropped` row in
  `docs/spec/02-php-migration.md`, never a member spelled to fill a hole.
- **What this spends**, per `rule:programs/memory-priority`: one reactor
  registration per open socket, per request, released with the request's arena. Nothing per process
  and nothing that grows with sockets served.
