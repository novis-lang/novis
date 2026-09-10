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

- **An entry names an owner.** A goal slug naming a live goal in
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
| `nvs check` builds no grants, so `rule:core-classes/db-literal-query-checking`'s host diagnostic fires for nobody | `carried-gaps` | `crates/nvs-types/src/intrinsics.rs` gap 6 |
| A cycle whose only closing edge is inside an `array<T>` survives `object::sweep` | `carried-gaps` | `crates/nvs-runtime/src/object.rs` § *The five walks*, `rule:security/isolate-teardown-is-a-drain-then-a-sweep` |
| `Core\Db::stream` on four drivers, `streamAs` whole, § 18's `serverVersion` | `gap-zero` | `crates/nvs-stdlib/src/db/mod.rs` gap 3 |
| `scope = "fleet"` parses, boots and is not armed | `carried-gaps` | `crates/nvs-server/src/schedule.rs` § *What is not armed*, `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease` |
| `rule:core-classes/html-to-source`'s computed `$reason` is not refused — was blocked on a full diagnostic band | `carried-gaps` | `crates/nvs-stdlib/src/html.rs` § *Known gaps* |
| `queryAs<T>`'s three refusals are at run time; the band they waited on is open | `gap-zero` | `crates/nvs-stdlib/src/db/mod.rs` gap 4 |
| Twenty CLDR plural rosters throw; ordinals absent; eight pattern letters refused | `carried-gaps` | `crates/nvs-stdlib/src/cldr.rs` gaps 2–4 |
| `Core\Request::clientIp`/`host`/`scheme`, `Response::html`/`sendFile` | `test-request` | `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` |
| `rule:testing/in-process-request` states two response readings rather than `Core\Test`'s signature, and spec § 13's `Core\Test` row is one English cell | `test-request` | `docs/rules/testing/in-process-request.md`, `docs/spec/01-core-library.md:999` |
| `nvs serve` runs on one core, and no path in the process starts a second | `per-core` | `crates/nvs-cli/src/serve.rs:42`, [m7.md](../plan/m7.md)'s own scope |
| `Core\Net`, `Core\Os`, `Core\Signal` — spec § 16, named by no milestone at all | `net-os-signal` | `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` |
| `Core\Compress`, `Core\Mime`, `Core\Zip` — spec § 17 | `formats` | the same file; `rule:core-api/tier-roster` puts all three at Tier 0 |
| `Core\Xml`'s tree and stream, and the two gaps behind it — `Core\Html::sanitize` and `rule:core-classes/html-parsing`'s parser | `xml-tree` | `crates/nvs-stdlib/src/html.rs` § *Known gaps*, `rule:core-classes/html-parsing` |
| ~110 module-doc `# Known gaps` items name no owner and are in no index | `gap-owners` | this file's own contract, applied one level down |
| `Core\Uri::with` replaces a component and cannot remove one | `unowned-sweep` | `crates/nvs-stdlib/src/uri.rs` gap 1 |
| `Core\Queue`'s `limits`/`grants` are undeclared and `$args` does not refuse a `secret` | `unowned-sweep` | `crates/nvs-stdlib/src/queue.rs` gaps 1–2 |
| `array<T>` is invariant — **decided: widen to a covariant read** | `unowned-sweep` | `crates/nvs-stdlib/src/lib.rs` gap 4, `nvs_types::expr::assign` |
| No custom panic hook — `rule:errors/helper-abi`; its blocker went away in M5 | `unowned-sweep` | `crates/nvs-runtime/src/lib.rs` gap 4 |
| `[limits] max_output` bounds no capture, at `Core\Process` or at `Core\IO::read` | `unowned-sweep` | `crates/nvs-stdlib/src/process.rs` gap 1 |
| The driver matrix has no socket leg, so `AF_UNIX` is asserted against no real server | `input-shapes` | `crates/nvs-db/src/matrix.rs` gap 1 |
| An integer where a grant expects a bool, a path or a list validates clean and grants nothing: `[capabilities.fs] read = 1` passes `nvs config check` at `0 warnings` and is denied at run time | `config-is-written` | `crates/nvs-config/src/tree.rs:50`'s untagged `Setting`, which every directive shares |

## Unowned

Nobody's, and each is a scheduling question rather than a session's. **Twenty-eight entries.** They arrive
three ways: an owner that went green without closing its gap and was struck rather than renamed, a
rule answered in full by code that no configuration key reaches, and a decision nobody has taken,
where taking it is the work and the code that follows it is not.

- **The route table is walked by comparison rather than by a trie**, which answers identically and
  costs one comparison per row of the right verb where a trie costs one step per path segment.
  `rule:routing/path-grammar` names `matchit`'s left-to-right precedence as the model and no
  structure, and no milestone claims routing-table performance — M12 is the JIT tier, and M7 scopes
  `Core\Router::match`'s surface rather than what walks under it. What has to be decided is whether
  the curve is ever the request path's problem at all: a table of tens of rows may never measure, so
  the decision is a measurement on `benches/serve-proxied.json`'s arms and the code that follows it
  is one function. `crates/nvs-runtime/src/routes.rs` gap 1. [until: reviewed 2026-09-10]
- **A closure is recognized by the `invoke` method on its class**, the same test `call_closure` makes,
  so a `mixed` carrying a user class that declares an `invoke` of its own is refused as a closure when
  a graph holding it is copied. `rule:types/declaration` makes that a compile-time rejection at nearly
  every copy site, so what is left is narrow, and what has to be decided is whether a closure earns a
  bit of its own on the class descriptor rather than a member-name test — a representation question
  that answers for both callers at once. `crates/nvs-runtime/src/graph.rs` gap 1.
  [until: reviewed 2026-09-10]
- **An encoded `Core` instance does not decode**, because `decode` resolves a class through the
  program's table only, so a `Core\Time\Instant` that crossed comes back unresolvable rather than
  rebuilt. The table it would need is `nvs_stdlib::instance`'s, and this crate cannot reach it —
  `crates/nvs-runtime/Cargo.toml:56` says outright that it cannot call `nvs-stdlib`. What has to be
  decided is whether that resolver is *installed* on `Ctx` at boot, the way the route and command
  tables are, or whether a `Core` instance stays outside what `Core\Serialize` round-trips.
  `crates/nvs-runtime/src/graph.rs` gap 2. [until: reviewed 2026-09-10]
- **A division whose intermediate exceeds 128 bits throws where the quotient would have fit**, in the
  one corner where a wide mantissa and a wide scale difference meet: the fold of the two operands'
  scales is a `checked_mul` over `u128` taken before the divide. ADR 0054 § *Consequences* already
  predicts the wider intermediate that closes it. What has to be decided is whether every division
  pays a 192-bit fold to remove a corner refusal, or that refusal stands as a stated bound of the
  type. `crates/nvs-runtime/src/decimal.rs` gap 1. [until: reviewed 2026-09-10]
- **An array header carries no interned element-type descriptor**, which `rule:types/arrays` gives it
  so a value arriving through `mixed`, `json_decode` or an isolate boundary can be checked. Nothing
  builds an array by those routes yet, and both readers that would want the word are specified to
  walk without it — `as array<T>` does, and `rule:types/type-test` makes `is array<int>` the same
  O(n) walk. What has to be decided is whether the word is carried before a reader needs it, given
  that adding it later is a widening of `ArrayHeader` rather than a redesign.
  `crates/nvs-runtime/src/array.rs`. [until: reviewed 2026-09-10]
- **A command argument typed as a *subset* of an enum's cases is refused when the command is run,
  not when it is written.** `Log\Level` converts; the `Log\Level::Warn|Log\Level::Error` that § 3 also
  admits is `ArgConv::Unconverted`. Both facts a conversion needs are already answered on
  `ArgConv::Enum`, so the code is a filter rather than a design; what has to be decided is which end
  closes it — reading the union's members where § 6 owns the spelling, or refusing the declaration
  outright so the error arrives where it was written. `crates/nvs-runtime/src/commands.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A local declaration cannot be typed with a bare inline shape type**, although every other
  declaration slot can. Statement-initial `{` commits to a block, and telling a type-prefix apart from
  one needs lookahead past a matched, possibly-nested `{...}` to the `$name` behind it. What has to be
  decided is whether the grammar buys that lookahead or whether `rule:types/shape-type`'s own example
  — `type Point = {x: int}; Point $point;` — is the answer for a local: a surface question, not a
  parser one, and the parser cost is the reason it is worth asking.
  `crates/nvs-syntax/src/lib.rs` § *Known gaps*. [until: reviewed 2026-09-10]
- **The compiled-unit cache loads on no `aarch64` host**, one of which
  [design.md](../plan/design.md)'s platform row (line 94) names as supported for macOS. Making freshly
  written bytes executable there needs instruction-cache maintenance `mprotect` does not imply, and
  what has to be decided is where that lives — this loader, or whatever maps the page in
  `nvs-codegen` — rather than whether it is wanted; Mach-O's leading underscore rides along with it,
  since neither is worth a format branch on its own. Until then every run on such a host compiles:
  slow, never wrong. `crates/nvs-cli/src/cache.rs` gaps 2–3. [until: reviewed 2026-09-10]
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
- **`nvs-ir`'s lowering residue, beyond the refusal sites
  [carried-refusals.md](carried-refusals.md)'s item 901 carries.** That entry's argument covers the
  whole file: M4 reached its loop goal, the milestone is carried, and no live entry on the chain
  writes `nvs-ir` lowering again, so what that acceptance list did not ask for stayed. It is a panic
  each — `<=>` and `**` have no `ir::BinOp` row, a ternary's two arms have no recorded result type to
  widen to, `as ?"a"` has no merge for the arm that answers `null` — plus one leak on the throw path
  and one **security** row, a `secret` compared against a `mixed` falling out of constant time, which
  closes by teaching `nvs_runtime::value_identical` the property rather than by adding a lowering
  arm. The items that index a hole another module's doc owns are tagged here too, and for the
  scheduling question rather than for the detail: each names the doc that holds it. What has to be
  decided is which goal reopens the crate — item 901 named one, `typed-callable`, and it retired
  without closing a site. `crates/nvs-ir/src/lib.rs` § *Known gaps* carries the tags.
  [until: gone crates/nvs-ir/src/lib.rs:owner: unowned]
- **`nvs-runtime`'s three missing representations, which nothing on the chain asks for.**
  `Tag::Closure` and `Tag::Resource` are rows of the roster the plan's § *Value representation*
  names and nothing constructs one: a closure is an ordinary object today, and whether Novis has a
  `resource` value **at all** is the decision, not the implementation of one. Beside them, every
  string producer but `nvs_str_append`'s in-place path allocates its result, so what has to be
  decided there is whether each gains the sole-ownership check `NvsArray`'s copy-on-write already
  pays for; and an exception this crate raises carries a message and no backtrace, where the
  decision is whether a raise captures one at all — `rule:errors/throw-is-not-slower` prices a
  throw at a return that allocates nothing, and a capture is the other side of that trade.
  `crates/nvs-runtime/src/lib.rs` § *Known gaps* carries the tags.
  [until: gone crates/nvs-runtime/src/lib.rs:owner: unowned]
- **A route link's named argument is not folded, so two different failures throw one message.**
  `nvs_types::links` folds a literal route name while compiling and refuses an unknown one
  (`E0754`); a **named** argument is not folded at all, so it reaches `Core\Router`'s runtime throw
  by the path a *computed* name takes, and a reader has to look up which of the two they hit. What
  has to be decided is whether the fold grows a named-argument case — the checker already holds the
  route's typed parameter list — or whether § 4's "a computed name throws" is the whole contract and
  the message is what names both. Until it is, `no_such_route` answers more than the case § 4 named.
  `crates/nvs-types/src/reasons.rs`'s spread argument is the same question one position over: that
  roster carries the parameter's own name, so a *named* argument is read there, while a spread moves
  every position at once — so what is undecided is again whether a compiler-known-call pass is handed
  the mapping. `crates/nvs-types/src/links.rs` § *Known gaps*,
  `crates/nvs-types/src/reasons.rs` § *Known gaps* and `crates/nvs-stdlib/src/router.rs` § *Known
  gaps*. [until: gone crates/nvs-types/src/links.rs:A named argument is not folded]
- **`nvs-types`' two unwritten checker passes, which no entry on the chain asks for.** Exhaustive
  control-flow reachability is one — "every path through this non-void function returns", and with
  it whether a bare `return;` is legal where it stands — and an equality-operand compatibility
  check is the other, without which `==` between two different enum types, and `int` against
  `uint`, are both accepted. Both are M4 checker holes and M4 is carried: its loop goal's
  acceptance list passes, and no live entry on the chain writes this crate's checking passes again.
  What has to be decided is whether the checker grows the flow analysis they need at all — the walk
  is structural rather than a CFG by `crates/nvs-types/src/locals.rs`'s own design note — and, for
  equality, whether an operand-compatibility rule is taken for every type pair at once, since
  taking it for enums alone leaves the operator inconsistent with itself.
  `crates/nvs-types/src/lib.rs` § *Known gaps* carries the tags.
  [until: gone crates/nvs-types/src/lib.rs:owner: unowned]
- **`nvs-types`' intrinsic pass, whose six gaps are each a decision rather than a backlog.** The
  pass validates a literal pattern wherever `rule:expressions/intrinsic-list-is-closed`'s roster
  names one, and it is complete at that. What is open is whether it also *prepares* one, which
  needs a channel from the checker to `nvs-ir` that no live entry on the chain builds, and whether
  a diagnostic can point at the offset inside a literal, which needs `crate::string_lit`'s decoder
  to carry a position map that every literal in the program pays for. Three narrower ones sit
  beside them: whether the roster grows a per-row restriction column, so that a member's own
  refusal of a well-formed pattern (`Core\Time::parse`'s civil fields) is compile-time; whether
  these compiler-known-call passes are handed the slot mapping `check_args_typed` builds, so that a
  named or spread argument is read at all — the same question the `links.rs` entry above asks; and
  which of two module docs is right about an unterminated string literal in a query. The sixth is
  not this pass's: its host check fires for nobody because `nvs check` reads no `nvs.toml`, and
  whether that command reads configuration — and so can be failed by a broken one — is a decision
  about the command. `crates/nvs-types/src/intrinsics.rs` § *Known gaps* carries the tags, and
  `crates/nvs-stdlib/src/time.rs` and `crates/nvs-stdlib/src/cldr.rs` are the same question one
  layer down: `format` and `parse` compile their pattern per call, through the one `compile` that
  module owns, and neither can prepare anything until that channel exists.
  [until: gone crates/nvs-types/src/intrinsics.rs:owner: unowned]
- **The third of `rule:core-classes/derive-attribute`'s rule that lives at run time, and whether it
  moves.** `nvs-types`' derive pass checks a `#[Json\Derive]`/`#[Db\Derive]` class whole and asks the
  same rule again of the class a `queryAs<T>` wrote, but there is no `Core\Json::decodeAs` half:
  `decodeAs<array<T>>` is a legitimate JSON array document, so "the mapping cannot fill the
  constructor" is a question about a *document* rather than about the class, and the
  missing-attribute refusal stays `nvs_stdlib::json`'s at run time. What has to be decided is whether
  that refusal moves to compile time at all, which is the rule's call rather than one this pass may
  widen into — and until it is taken, one of the two members enforces the attribute where the program
  is written and the other only where it runs.
  `crates/nvs-types/src/derive.rs` § *Known gaps* carries the tag.
  [until: gone crates/nvs-types/src/derive.rs:owner: unowned]
- **A derived field's type roster stops where `nvs_runtime::CodecTy` does.** A `decimal`, a
  `Core\Time\Instant`, an inline shape reached as a field and an `array<T>` of one of those are all
  codec-reachable by `rule:core-classes/derive-field-list` and all land on `CodecTy::Opaque`, which
  both doors refuse. What has to be decided is what each of those types *is* on the wire — a
  `decimal` as a JSON string or as a number, an `Instant` as its RFC 3339 text — because no variant
  can be carried before the encoding it stands for is chosen, and the same answer binds `Core\Db`'s
  row codec. `crates/nvs-stdlib/src/json.rs` gap 1, with `crates/nvs-stdlib/src/db/mod.rs` gap 4 the
  other door on the same knot. [until: gone crates/nvs-stdlib/src/json.rs:type roster is narrower]
- **The derive machinery is a descriptor read by native Rust**, where
  `rule:core-classes/derive-generates-what-is-missing` costs it as straight-line code emitted per
  class. Both halves of `Core\Json` walk a per-class `nvs_runtime::CodecField` list instead, and two
  narrower holes wait on which of the two it stays: a constructor parameter's default is a constant
  `nvs_types::defaults` evaluates at a *call site*, which a native decoder is not, and a hand-written
  `toJson()` needs a `ClassDesc::method` lookup back into compiled code. Both are free in emitted
  code and both a widening of the descriptor otherwise, so the decision is one and the code that
  follows it is two. `crates/nvs-stdlib/src/json.rs` gaps 2, 3 and 4.
  [until: gone crates/nvs-stdlib/src/json.rs:rather than straight-line]
- **`Core\Json::encode`'s real bound is the native stack rather than its own `DEPTH_CEILING`**, so a
  document that is legal and merely very deep aborts the process where every other refusal throws.
  Goal `resource-ceilings` names the stack ceiling out of its own scope, so this is nobody's: what
  has to be decided is whether the walk carries an explicit stack, which makes the bound an
  allocation the request is charged for, or the ceiling is read from the space
  `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled` reserves.
  `crates/nvs-stdlib/src/json.rs` gap 5.
  [until: gone crates/nvs-stdlib/src/json.rs:The encoder's real bound is the native stack]
- **`Core\Uuid` has no `bytes` round trip**, so a driver binding a native `UUID` column carries the
  36-character text between the two. Nothing under it is missing — `nvs_runtime::Tag::Bytes` is a
  live tag and a `Core` instance holds whatever Novis holds — so what has to be decided is whether
  spec § 11's second table gains the pair at all: all four members it names are built, and a fifth
  widens a surface the spec fixed rather than repairing it.
  `crates/nvs-stdlib/src/uuid.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/uuid.rs:owner: unowned]
- **`nvs-db`'s § 5 normalization, which both directions of the schema plan record as owed and
  which nothing on the chain writes.** The emitter's map into a dialect is lossy — one `BYTEA` for
  both bytes widths, `CHAR(36)` for a `UUID`, the next width up and a `CHECK` for a `uint` no
  catalog reports as a type — so a schema applied and read straight back describes as a
  *different* vocabulary case, and a diff converges only once the declared side is folded through
  the same map. What has to be decided is where that fold lives: one normalisation over both sides
  before the diff, or a per-case comparison inside it. Three narrower decisions sit beside it —
  whether a SQL Server default's server-generated constraint name enters the vocabulary at all,
  since neither `Change` nor the catalog reads carry one and a default change there is emitted
  without its default; whether a default spelling the vocabulary cannot hold is dropped, refused
  as a `SchemaError` or given an opaque case; and, for the two constructs the vocabulary holds
  that no backend takes portably, whether a builder refuses them or an emitter grows a spelling.
  Goals `database` and `schema` are retired and M8's database half is carried with them, so no
  milestone claims this either. `crates/nvs-db/src/ddl.rs`, `crates/nvs-db/src/catalog.rs` and
  `crates/nvs-db/src/schema.rs` § *Known gaps* carry the tags.
  [until: gone crates/nvs-db/src/ddl.rs:owner: unowned]
- **The database matrix's `AF_UNIX` leg is asked for and never published**, so the transport
  `rule:core-classes/db-unix-socket-path` gives MySQL, MariaDB and PostgreSQL is asserted against
  listeners `nvs-db` binds itself and against no real server. `Location` has its third arm and
  `tests/handshake.rs` dials whichever one it is handed; `tools/db-matrix.py` sets no `SOCKET_VAR`,
  which would need a container's socket directory bind-mounted onto the host. What has to be
  decided is what that leg runs: the whole TCP case list a second time, which is what would say
  the driver agrees across both transports and doubles the matrix, or a handshake-only case that
  says the transport connects and no more. `crates/nvs-db/src/matrix.rs` gap 1.
  [until: reviewed 2026-09-10]
- **How much of a `mixed` value's tag dispatch is written at once.**
  `rule:types/erased-member-access` has already decided what an erased read, write and call *do* — a
  checked, catchable throw when the name is not there, never a silent value — but `nvs_ir::Ty::Tagged`
  implements exactly one position of it, rendering, and every other position panics naming the case.
  So `$e->previous->message` needs the value bound to a local and narrowed first, which is why
  `crates/nvs-types/src/error_lib.rs` records the chain as built but not readable through. Each
  position closes the same way — one `nvs_ir::ir::Helper` variant dispatching on the tag — and what
  has to be decided is whether they are taken together, since arithmetic, the truthy table, a subscript
  and this member access pay one design cost between them and taking them one at a time leaves the
  rule half-true for however long the others wait.
  `crates/nvs-types/src/error_lib.rs` § *Known gaps* carries the tag.
  [until: gone crates/nvs-types/src/error_lib.rs:owner: unowned]
- **Bounds for a pool no `[db]` block describes.** `[db.<name>.pool]` reaches every connection whose
  settings hash is that block's, and the unscoped `[db] pool = false` reaches every connection the
  process opens, but a literal naming an endpoint an operator wrote no block for — or naming a hashed
  field the block left implicit, a written `port` where the block took the server's default — is a
  second settings key and so a second pool, at `PoolBounds::DEFAULT`. What has to be decided is where
  bounds for a key only the *program* knows would be written: an option on `open`, a block that
  matches a pattern rather than a name, or nowhere, on the grounds that a deployment wanting its own
  bounds writes its own block. It is `rule:security/db-pool-reset-is-a-boundary`'s call rather than a
  shape `nvs-stdlib` may pick, and goals `database` and `schema` are retired with M8's database half
  carried, so nothing claims it. `crates/nvs-stdlib/src/db/mod.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/db/mod.rs:naming an endpoint an operator wrote no block for]
- **`Db\DbError` is outside spec § 10's error tree, so a database refusal carries no `issues`.** The
  class declares all five of § 18's values and `RuntimeError` is its parent, but `issues` is declared
  by `ParseError` alone, so `queryAs`'s per-column refusals are thrown as a `ParseError` naming the
  columns rather than as the class § 8 gives the database. What has to be decided is whether `DbError`
  joins that tree — § 10 giving it the property, which is a spec change and a registry one — or the
  split stands and a row that does not fit its class is a parse failure by design. Neither goal that
  would have taken it is live: `database` and `schema` are retired with M8's database half carried.
  `crates/nvs-stdlib/src/db/mod.rs` gap 2.
  [until: gone crates/nvs-stdlib/src/db/mod.rs:spec § 10's error tree]

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

**The list this file indexes is not the whole inventory.** Every crate's own `# Known gaps` block holds
enumerated items — `python tools/owners.py` counts them and prints who owns each — and this file names
the ones whose ownership needed an argument, while
[carried-refusals.md](carried-refusals.md) 901 covers `nvs-ir`'s. The rest were unindexed until goal `gap-owners`,
which puts the owner in the module doc beside the gap and derives the roster rather than copying it —
this file's own contract, applied one level down.
