---
milestone: post-parity
---
# Loop goal 7 — The gaps no goal owned

Every gap in this goal is one a shipped feature already carries, written down in the module that owns
it, and claimed by **no entry on the chain** — so nothing but this goal would ever have closed it.
When it is green a grant an operator wrote is a grant the compiler and the runtime both read, a cycle
closed through an array is reclaimed rather than retained for the life of the process, a log line can
be jumped to from a trace, a `fleet` schedule entry actually fires, `Core\Db` answers spec § 18's
whole roster, and the two rules that were waiting on a full diagnostic band are enforced.

It sits directly after goal `server` because **that is where the orphans are made.** A goal's unclosed items
are not carried by `tools/goal-switch.py`; only its `[[check]]` blocks are. So the moment goal `server` goes
green its handoff's `## Backlog` is overwritten by goal `temp-sweep`'s seed and everything on it stops existing —
which is how `§18 stream`'s owner comment came to name goal `database`, six goals after goal `database` closed. Running
this entry first is what stops that list being written a second time.

Goal `server`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

A goal switch carries the outgoing goal's [[check]] blocks forward and its unclosed *items* not at
all, so goal `server`'s handoff backlog stops existing the moment goal `server` goes green — which is how `§18
stream` came to be filed under "goal `database`'s" six goals after goal `database` closed.

## Stage 0 — the catch-up

Nothing. No fixture predates a rule this goal changes, because this goal changes no rule that a
fixture could have been written against — every item is a hole rather than a different answer. The one
exception is stage 10, whose spec amendment changes two return types; the fixtures that call them are
listed in that stage and rewritten as part of it, not before it.

## Stage 1 — the floor

Goal `server`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for
anything above it.

## Stage 2 — the keystone: an outstanding key names a goal that is still on the chain

The mechanism that would have caught most of this goal, and the reason it is the keystone rather than
an item: **without it, the same orphans are made again by the next goal switch.**

`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` and
`spec-classes-part-two-outstanding.txt` already list every spec §§ 14–19 member and class the registry
does not declare, and the list only ever shrinks — which is a *ratchet*, not a schedule. A key sits
there green forever, and the prose above each group names the goal that will strike it in a comment no
program reads. `§18 stream` and `§18 streamAs` are grouped under "goal `database`'s"; goal `database` has closed.

1. **A key carries its owner, in a column rather than in a comment** —
   `§18 stream  # 21` — and `crates/nvs-stdlib/tests/spec_registry_coverage.rs:@…` parses it.
   `crates/nvs-stdlib/tests/spec-members-outstanding.txt` gains the same column and needs no keys,
   being empty.
2. **The test fails on a key whose owner is not a live goal** — read out of
   `docs/agent/goals/`, which is the one home of what is scheduled. An owner that has gone
   green and left the key behind is exactly the orphan this goal exists to close, and it now stops a
   run instead of a reader.
3. **A key with no owner at all is the same failure**, so the seeding pass is where every one of the
   fifteen keys gets read and assigned. Two of them are this goal's; five are goal `test-request`'s
   (`Request::clientIp`/`host`/`scheme` by that goal's own § 2, `Response::html`/`sendFile`); the
   eight §§ 16–17 classes have no owner on the chain at all and are **recorded in
   `docs/agent/carried-gaps.md` and assigned there**, never invented here.
4. **`docs/agent/carried-gaps.md` is the durable list beside it**, on
   `docs/agent/carried-refusals.md`'s precedent and for its reason: a fact that survives a goal switch
   may not live in a file the switch overwrites. This goal's own entries are struck from it as they
   land, and `docs/agent/session-prompt.md` § *the handoff's shape* and
   `docs/agent/loop-authoring.md` § 8 both point at it, so a session that finds a gap off its path has
   somewhere to put it that goal `warm-start` will still be able to read.

Files: `crates/nvs-stdlib/tests/spec_registry_coverage.rs`, the three `spec-*-outstanding.txt` lists,
`docs/agent/carried-gaps.md`.

## Stage 3 — a grant an operator wrote is a grant something reads

One file set: `crates/nvs-config/src/capability.rs`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-types/src/check.rs` and `crates/nvs-types/src/intrinsics.rs`. Both items are
`rule:core-classes/db-capabilities` and § 10, and both are *priority 1* — a capability that
silently does nothing is the failure mode the whole grant system exists to prevent.

5. **`db.open`'s host grant learns the leading-label wildcard the ADR writes.**
   `crates/nvs-config/src/capability.rs:269`'s `host_granted` is
   `entry.eq_ignore_ascii_case(host)` and nothing else, so `rule:core-classes/db-capabilities`'s own worked configuration —
   `db.open = ["*.tenants.internal"]`, line 116 of that ADR — matches no host at all. It fails
   *closed*, so this is a config an operator writes in good faith that denies everything, not a hole
   something gets through. The rule is in that stage's standing decisions below;
   `Capabilities::allows_host` at `:311` and `allows` at `:289` are the two callers and both go
   through the one predicate, so there is one place to change.
   `crates/nvs-types/src/intrinsics.rs`'s known gap 7 is struck when it lands.
6. **`nvs check` reads the configuration, so § 10's host diagnostic fires for somebody.**
   `crates/nvs-cli/src/main.rs:623`'s `front_end` builds no `nvs_types::Env::grants`, so
   `run_check` at `:695` calls `check_program` with `None` and
   `crates/nvs-types/src/check.rs:108`'s `check_program_granted` — which is written, correct and
   tested — has no caller outside `crates/nvs-types/tests/`. `rule:core-classes/db-one-api`'s *Verification* names
   "an `open` host matching no grant" as an **M8 `nvs check` acceptance item**, and M8's four goals
   have all been written without it. `crates/nvs-types/src/intrinsics.rs`'s known gap 6 is struck when
   it lands.

## Stage 4 — a cycle closed through an array is swept

One file: `crates/nvs-runtime/src/object.rs`, with `examples/cycles.nvs` as the fixture goal `core-part-ii` already
wrote and the WSL valgrind leg as the check.

7. **`object_fields` tallies references held in array elements, not only in field slots.**
   `crates/nvs-runtime/src/object.rs:1816` returns a member's `Tag::Object` *field* slots and nothing
   else, so `sweep` at `:1658` reads an object reachable only through an array as externally held and
   leaves it — and a cycle whose only closing edge is inside an `array<T>` therefore survives a
   context's teardown. [ADR 0116](../../decisions/0116.md)
   § *Consequences* is the reason this is not a footprint question but a correctness one: it argues
   the sweep into existence with "in the server, a leak growing with requests served, which is what
   made the sweep an obligation rather than an option", and that sentence is still true of this
   shape. AGENTS.md's priority ordering says the same thing in one line — growth with total traffic
   is a leak, not a trade-off.

## Stage 5 — `Core\Db` answers spec § 18's whole roster

One file set: `crates/nvs-stdlib/src/db/`, `crates/nvs-db/src/`, `crates/nvs-config/src/db.rs`.

8. **`stream` and `streamAs` at constant memory, and the connection-busy `LogicError`.**
   Spec § 18's two remaining rows, the last two keys of stage 2's part-two list, and
   `rule:core-classes/db-one-api`'s own *Verification* names them in its per-driver M8 bullet:
   "large-result streaming at constant memory, and the connection-busy `LogicError`". They answer an
   `Iterable<…>`, so `nvs_stdlib::cursor` and `nvs_stdlib::instance`'s dispatch roster are the shape,
   not a new one — `crates/nvs-stdlib/src/db/mod.rs`'s known gap 5 is the inventory.
9. **`Connection::close` and § 18's three readonly properties**, from the same gap 5. Neither is a
   key on stage 2's list, because § 18 states them as bullets rather than as table rows — which is
   its own small finding and is why stage 2 reads the section rather than the table.
10. **`[db.<name>.pool]`, and `pool = false` reaching a program-opened connection.**
    `crates/nvs-stdlib/src/db/mod.rs`'s known gap `nvs-stdlib/buffered-read-is-not-held-to`: `max`, `idle`, `lifetime` and `acquire` are
    `PoolBounds::DEFAULT` for an `open`, because a settings literal has no `[db.<name>.pool]` table to
    read them from, and § 13's `pool = false` is written per block and so cannot reach an `open` at
    all. The deployment that notices is the audited one that needs every connection to map to one
    request: it can switch off every block an operator wrote and not the connections a program opens
    for itself. That module says outright this is "an `rule:security/db-pool-reset-is-a-boundary` question and not a shape this
    module may pick on its own" — so it is answered in the standing decisions below and folded into
    `rule:security/db-pool-reset-is-a-boundary`.
11. **`{timeout?: Duration}` on `query` and `execute`**, gap 6 of the same module: the option is in
    both spec signatures and deliberately in neither registry row, because a deadline on a statement
    has to reach the socket the way `nvs_db::PgConn::connect`'s does and there is no seam for one on
    the statement path. The seam is this item; it rides here because it is the same file set and the
    same five drivers as items 8–10.

## Stage 6 — a log line can be jumped to from a trace

One file set: `crates/nvs-stdlib/src/log.rs`, `crates/nvs-runtime/src/ctx/`,
`crates/nvs-server/src/trace.rs`.

12. **`ts`, `request_id`, `trace_id` and `span_id` on the record envelope.**
    `rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active` lists four fields beside `level` and `msg`
    and `crates/nvs-stdlib/src/log.rs:46` carries none of them. The reason that module recorded — "no
    request, no trace and no clock" — stopped being true inside goal `server`, which landed the server,
    `nvs_server::trace`'s context and the whole `[trace]` block. `trace_id`/`span_id` on a record is
    the entire mechanism by which a log line reaches the trace it belongs to, so tracing is a
    half-delivered feature until this lands, and § 6's last paragraph is the home of the rule.
    § 6 already says a field with no value is **omitted rather than empty**, which is what makes this
    additive: a CLI run with no request keeps today's two-key envelope byte for byte.

## Stage 7 — a `fleet` schedule entry fires

One file set: `crates/nvs-server/src/schedule.rs`, `crates/nvs-stdlib/src/cache.rs`.

13. **`scope = "fleet"` is armed, under `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`
    's lease.** `crates/nvs-server/src/schedule.rs:232`'s `arm` skips a fleet entry and prints a
    note saying so (`:241`), so a directive an operator wrote parses, boots, and does nothing. § 3
    makes a fleet-scoped interval exactly one run across the deployment held by a lease, and what is
    missing is a compare-and-set on the shared tier. The named test the handoff has carried since
    goal `core-part-ii` —`a_fleet_scoped_entry_fires_once_across_the_fleet_under_its_lease` — is this item's check
    and is written here rather than invented.

## Stage 8 — the two rules a full diagnostic band was blocking

One file set: `crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-types/`,
`crates/nvs-stdlib/src/html.rs`, `crates/nvs-stdlib/src/db/`.

14. **`rule:core-classes/html-to-source`'s `$reason` must be a source literal, refused where it is written.**
    `crates/nvs-stdlib/src/html.rs`'s known gap says the rule "waits on a decision about the band
    layout" because `E0499` and `E0799` are both full. That decision was taken: the `E08xx` band is
    open at `crates/nvs-diagnostics/src/lib.rs:2953` and holds one code, `E0801`. The blocker is
    gone and nobody went back, which is the whole of why this item exists.
15. **`queryAs<T>`'s three run-time refusals move to the call site**, from
    `crates/nvs-stdlib/src/db/mod.rs`'s known gap 8 and for the same reason: a `T` carrying no
    `#[Db\Derive]` codec, a `queryAs<array<C>>` whose list form means nothing, and a field the derive
    pass erased to `CodecTy::Opaque` are each a property of the call site or of the class alone, and
    each is refused per row today because "both bands the checker would take a code from are full".

## Stage 9 — the rosters that throw

One file: `crates/nvs-stdlib/src/cldr.rs`.

16. **The twenty named plural-rule absences**, gap 3 of that module — `be`, `he`, `mt`, `dsb`, `hsb`,
    `gd`, `br`, `kw`, `gv`, `is`, `mk`, `tzm`, `shi`, `si`, `ak`, `bh`, `guw`, `nso`, `wa` and `naq`
    each throw today, and each is an arm rather than a row because its rule shape is one `RULES`
    (`crates/nvs-stdlib/src/cldr.rs:1322`) does not already have.
17. **Ordinal rules, and the `Core\Cldr` member for them**, gap 4: a separate CLDR table with its own
    categories per language, which nothing in the framework's § 3 half asks for and which no other
    entry on the chain will ever add.
18. **The eight refused pattern letters** — `G`, `Q`, `w`, `W`, `L`, `c`, `F`, `u` — gap 2, each an
    addition to the existing table rather than a different design.

## Stage 10 — a percent-decoder answers octets, and a route capture is decoded once

One file set: `crates/nvs-stdlib/src/uri.rs`, `crates/nvs-runtime/src/routes.rs`,
`docs/spec/01-core-library.md` § 12. The order inside the stage is fixed: item 19 first, because item
20 calls what it changes.

19. **`decodeComponent` and `parseQuery` answer `bytes`.**
    `crates/nvs-stdlib/src/uri.rs`'s known gap 2 states the case and calls it "a spec question":
    percent-decoding is defined over octets, a client may send any of them, and a `string` answer
    means `decodeComponent("%FF")` throws rather than answering. It is the one place that module
    diverges from PHP, whose strings are byte strings. The amendment is in the standing decisions.
20. **A route capture is decoded where it crosses into the program.**
    `crates/nvs-runtime/src/routes.rs`'s known gap 3: a capture's value is the segment as it arrived,
    still percent-encoded, and that module is right that a second decoder next to the matcher is the
    two-that-agree-today failure the laundering rules exist to prevent. So the decode goes through
    `nvs_stdlib::uri`'s one decoder at the crossing, which is `Core\Request::route()` and the handler
    binding, and `nvs-runtime` gains no decoder of its own. A `uint` capture is unaffected — no digit
    has an encoded spelling.

## Standing decisions

- **This goal opens no new ADR number.** It may change, each through a record whose `changes:` block names it,
  `rule:security/db-pool-reset-is-a-boundary` (item 10's pool bounds),
  `rule:core-classes/db-capabilities` (item 5's wildcard, if the ADR's own wording needs
  sharpening to match what lands) and `docs/spec/01-core-library.md` § 12 (item 19), and no others.
  Everything else is decided-and-recorded in the module doc that already owns the gap.
- **Item 5's wildcard rule, decided here.** A grant entry beginning `*.` matches a host whose name
  ends with the entry's remainder **at a label boundary** — `*.tenants.internal` matches
  `a.tenants.internal` and `a.b.tenants.internal` and does **not** match `tenants.internal` itself or
  `evil-tenants.internal`. Matching stays case-insensitive because DNS is. `*` alone is **not** a
  spelling: `Grant::All` already means "every host" and a second way to write it is exactly the
  reachable-two-ways R20 forbids. The wildcard is `Cap::DbOpen`'s only —
  `Cap::NetConnect` is asked of a *resolved address* (`capability.rs:87`), where a name has already
  gone, so a wildcard there would be a widening with nothing to match against. Recorded in
  `capability.rs`'s own module doc.
- **Item 6's configuration question, decided here.** `nvs check` resolves `nvs.toml` exactly as
  `nvs run` does, and a malformed one makes `nvs check` fail with that config error rather than with a
  program diagnostic — a command that reads configuration is a command a broken configuration can
  fail, which is the answer `crates/nvs-types/src/intrinsics.rs`'s gap 6 asked for and left open. No
  configuration found means no grants, which is today's behaviour and stays silent.
- **Item 7's safe direction is unchanged.** The sweep still frees only what it can *show* is
  unreachable; widening the tally to array elements may only ever move an object from "left alone" to
  "shown unreachable", never the reverse. If a walk turns out not to be able to prove an element edge
  — a nested `array<array<T>>`, a shared copy-on-write buffer — the element is treated as an external
  hold and the object is left, exactly as today. A `debug_assertions` assertion pins it and
  `examples/cycles.nvs` gains an array-closed cycle under the WSL valgrind leg.
- **Item 10's pool answer, decided here.** An `open`'s bounds come from the `[db.<name>.pool]` table
  of the block whose *settings hash* the connection was opened under when there is one, and from
  `PoolBounds::DEFAULT` when there is not; `pool = false` becomes a directive an operator may write
  **unscoped** as well as per block, and unscoped it reaches every connection including a program's
  own — which is the audited deployment's whole requirement. Folded into `rule:security/db-pool-reset-is-a-boundary`.
- **Item 12 is additive and stays additive.** A field with no value is omitted, per `rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`, so
  no existing record changes shape. A fixture asserts the *keys* a served request produces and the
  two-key envelope a CLI run produces; it never freezes a timestamp or an id.
- **Item 13's fallback.** A shared store that cannot compare-and-set leaves a fleet entry unarmed and
  says so at boot — today's behaviour, kept deliberately as the fallback rather than replaced by a
  best-effort arm that would fire the entry on every host. The lease is a set-if-absent with an
  expiry on the shared tier, not a new `Core\Cache` member.
- **Item 19's amendment, decided here.** Spec § 12's two decoder rows answer `bytes`; a caller that
  wants text writes `as string`, which is `rule:types/conversion`'s checked row and throws in exactly the place
  the member throws today, so no program is denied an answer it could have used. `encodeComponent` and
  `encodeFormValue` are untouched — they take text and answer text, and they are the class's two
  `Qual::Launder` rows. Every fixture calling a decoder is listed in the stage and corrected as source,
  never as expected output.
- **What this goal does not take, and where it went instead.** The on-disk artifact cache is
  goal `warm-start`'s, whole: it needs a second `nvs-codegen` `Module` and a named symbol for every host address
  the JIT bakes in, which is a subsystem rather than a gap. The eight spec §§ 16–17 classes
  (`Core\Metrics`, `Core\Net`, `Core\Os`, `Core\Signal`, `Core\Compress`, `Core\Mime`, `Core\Xml`,
  `Core\Zip`) have no owner on the chain and are **not** invented one here — stage 2 item 3 records
  them in `docs/agent/carried-gaps.md` and they are the user's to schedule. `rule:security/arena-is-an-ownership-root`'s optional
  in-flight cycle collector for a long-running CLI script stays an open *decision*, which that ADR's
  *Consequences* already says; item 7 is the leak, not the collector.
