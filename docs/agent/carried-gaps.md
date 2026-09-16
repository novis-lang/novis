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
| `Core\Db::stream` on four drivers, `streamAs` whole, § 18's `serverVersion` | `gap-zero` | `crates/nvs-stdlib/src/db/mod.rs` gap 3 |
| `rule:core-classes/html-to-source`'s computed `$reason` is not refused — was blocked on a full diagnostic band | `M7` | `crates/nvs-stdlib/src/html.rs` § *Known gaps* |
| `queryAs<T>`'s three refusals are at run time; the band they waited on is open | `gap-zero` | `crates/nvs-stdlib/src/db/mod.rs` gap 4 |
| Spec § 13's `Core\Test` cell states the roster in English, so `request`'s bag is spelled nowhere in the spec | `unowned-closures` | `docs/spec/01-core-library.md:1001`; the bag itself is `crates/nvs-stdlib/src/test.rs`'s `REQUEST_OPTIONS`, and `docs/novis.md` renders the signature |
| `Core\Queue`'s `limits` and `grants` stay undeclared until an isolate enforces them | `gap-zero` | `crates/nvs-stdlib/src/queue.rs` gap 1 |
| `Core\Process::spawn` — `proc_open`'s and `popen`'s streaming half — is unregistered, and `run` is the whole of what there is | `m8-stdlib-depth` | `crates/nvs-stdlib/tests/migration-members-outstanding.txt` |

## Unowned

Nobody's, and each is a scheduling question rather than a session's. **One bullet each, and `python
tools/owners.py --unowned` is the roster derived from the modules themselves.** They arrive
three ways: an owner that went green without closing its gap and was struck rather than renamed, a
rule answered in full by code that no configuration key reaches, and a decision nobody has taken,
where taking it is the work and the code that follows it is not.

- **An integer where a grant expects a bool, a path or a list validates clean and grants nothing.**
  `[capabilities.fs] read = 1` passes `nvs config check` at `0 warnings` and is denied at run time,
  because `crates/nvs-config/src/tree.rs:50`'s `Setting` is `#[serde(untagged)]` and every directive
  shares that one enum. Narrowing it is not the fix — `:39-43` records that `memory = false` and
  `exporter = false` are load-bearing second spellings — so the arms a directive accepts are
  per-directive and nothing declares them. What has to be decided is where that table lives, beside
  each field in the tree or beside the capability roster the checker already walks; `docs/plan/m6.md:52`
  promises the verb and not the refusal, so no milestone's plan carries it either.
  `crates/nvs-config/src/tree.rs:50`. [until: reviewed 2026-09-13]
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
- **An object holding a host handle crosses the copy**, which `rule:classes/graph-copy` names as one of
  its three refusals and no carrier makes: a `Core\Http\Socket` passed to `spawn script … with(args:)`
  arrives in the child with every slot intact, and each `Core` class whose slot holds a key into a
  request's own table is in the same position. The copied key indexes the *receiving* side's table, so
  it reads whatever that side opened at that index rather than nothing. What has to be decided is where
  the mark saying a class holds a handle lives — a bit on the class descriptor, which is the same
  representation question gap 1 asks for a closure, or the declared type at the copy site — and it
  answers for every `Core` class at once. `crates/nvs-runtime/src/graph.rs` gap 3.
  [until: reviewed 2026-09-13]
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
- **`nvs-runtime`'s missing representations, which nothing on the chain asks for.**
  Every string producer but `nvs_str_append`'s in-place path allocates its result, so what has to be
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
- **The derive machinery is a descriptor read by native Rust**, and both halves of `Core\Json` walk a
  per-class `nvs_runtime::CodecField` list rather than straight-line code emitted per class. The two
  narrower holes that waited on which of the two it stays are closed against the descriptor — a
  constructor parameter's default rides on the field as a constant, and a hand-written half is one
  `ClassDesc::method` lookup at the walk's own class arm — so what is left is only whether the
  machinery stays a descriptor at all. `crates/nvs-stdlib/src/json.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/json.rs:rather than straight-line]
- **`Core\Json::encode`'s real bound is the native stack rather than its own `DEPTH_CEILING`**, so a
  document that is legal and merely very deep aborts the process where every other refusal throws.
  Goal `resource-ceilings` names the stack ceiling out of its own scope, so this is nobody's: what
  has to be decided is whether the walk carries an explicit stack, which makes the bound an
  allocation the request is charged for, or the ceiling is read from the space
  `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled` reserves.
  `crates/nvs-stdlib/src/json.rs` gap 2.
  [until: gone crates/nvs-stdlib/src/json.rs:The encoder's real bound is the native stack]
- **The queue's two tables are the runtime's own, and two questions about what they carry are open.**
  `nvs_jobs` gets its dedupe guarantee from a plain unique key over `dedupe_pending`, which
  `rule:core-classes/queue-storage-is-a-table` states and the runtime's own schema carries — but that
  key exists only where an operator has run `nvs queue migrate`, and `push` proves nothing about
  whether they did, so a deployment behind on the converge is racy at `read committed` while reading
  as healthy. What has to be decided is whether a member proves the constraint is there — an
  introspection on first use, or a refusal to serve a queue whose schema is behind — or whether that
  window stays an operational bound the migration command's own text names. Beside it, `stats`
  answers the four counters the rule names and a dead-lettered job's attempts are in none of them,
  because `nvs_dead_jobs` deliberately carries nothing beyond `id` and `queue`; what has to be decided
  there is whether that table keeps the columns a fifth counter would read, through the same
  `nvs queue migrate` converge goal `queue-purge` takes its `tag` column through.
  `crates/nvs-stdlib/src/queue.rs` gaps 3 and 4.
  [until: gone crates/nvs-stdlib/src/queue.rs:deployment that never ran]
- **Nothing measures what spec §§ 13–20 still owe.** `tests/spec_registry_coverage.rs` and
  `tests/conformance_coverage.rs` both stop at § 12, and past it there is no outstanding-members
  file at all, so a row nobody has written fails nothing — `Core\Db`'s `stream`, `streamAs` and
  `Connection::close` are spec § 18 rows in exactly that position. What has to be decided is whether
  those two gates widen past § 12 at all, which means listing rows for classes no goal has built yet
  and failing `cargo test -p nvs-stdlib` on every one of them from the day the file appears, or
  whether §§ 13–20 stay measured one module doc at a time with this file as the index of what nobody
  owns. `crates/nvs-stdlib/src/lib.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/lib.rs:§§ 13–20 hold whatever]
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
- **SQL Server refuses a `bytes` parameter, and unrefusing it is a plan-cache decision.** The read
  half of ADR 0067 § 9's row is whole on that driver; the bind half is the one place a `bytes` does
  not reach a server, because every TDS parameter goes out as one `nvarchar` for the server to cast
  and `varbinary` is the type no text form casts back to. A real `@params` entry makes
  `sp_prepexec`'s declaration a function of the *values* a call binds, which § 1's cache key —
  SQL text plus expansion arity — does not distinguish. What has to be decided is whether that key
  grows the bound types, or the `bytes` binds outside the cache. Goal `m8-db-queue` takes the row
  side and stage 10's gate names every tag it closes, so it does not claim this one.
  `crates/nvs-db/src/tds/mod.rs` gap 1.
  [until: gone crates/nvs-db/src/tds/mod.rs:owner: unowned]
- **Nothing reads a `debug.trace` grant into the flag every probe is gated on.** `Capability::DebugTrace`
  is in the vocabulary and `DebugFlags::TRACE` is the bit codegen branches on, and outside a test
  nothing sets the second from the first — so a query span, an HTTP span and every call probe render
  on a flag a program cannot ask for. What has to be decided is where the read lives: once per
  request as the capability set is resolved into the `Ctx`, or on the probe path each time, which is
  the difference between a grant that can be tightened mid-request and one fixed at dispatch.
  `crates/nvs-db/src/span.rs` gap 1.
  [until: gone crates/nvs-db/src/span.rs:owner: unowned]
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
- **Zip64 is not read, so no archive over 4 GiB or past 65535 entries has a spelling.** One records
  its sizes in an extra field and writes `0xFFFFFFFF` where this reader looks, and is refused as
  malformed rather than misread, so nothing about it is unsafe — the bound is measured rather than
  taken from a header either way. What has to be decided is whether Novis reads one at all:
  `rule:core-classes/decompression-bound`'s ceiling defaults to 64 MiB, so an archive that size is
  already past what any default extracts whole, and the question is whether *listing* one, and
  pulling a small entry from it, buys enough to carry a second header format. Goal `formats` is
  retired and no milestone's plan names it. `crates/nvs-stdlib/src/zip.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/zip.rs:Zip64 is not read]
- **The generated API document is honest and incomplete, and completing it grows four sources
  outside the emitter.** A class response body and a request body need the codec roster on the route
  row rather than a property map, `components.securitySchemes` needs somewhere in the tree to
  declare a scheme, `info.version` needs an `nvs.toml` key, and an enum-case subset needs
  `Core\Router::match` to decide a case's segment spelling. Each is *absent* from the document
  rather than guessed at, which is
  `rule:routing/api-document-is-generated-from-the-route-table`'s call and not in question. What
  has to be decided is whether Novis commits to a document a strict validator accepts, because that
  is four surfaces rather than an emitter change, and `nvs openapi` is useful as it stands.
  `crates/nvs-cli/src/openapi.rs` gaps 1–5.
  [until: gone crates/nvs-cli/src/openapi.rs:A response body that is an object]
- **The shared cache tier speaks `redis://host[:port]` and nothing else**, so a store behind a
  password, one addressed by database index, and a `rediss://` one are each refused with a sentence
  rather than dialled half-served. What has to be decided is whether `[cache.shared]` grows a
  configuration surface for them at all, and that is one question rather than three: a credential
  may not ride in a URL a merged tree prints, so it arrives by
  `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s secret file or not at all, and an
  index is a second namespace nothing else in the tree names. No goal on the chain names this
  module and no milestone's plan carries it. `crates/nvs-stdlib/src/cache.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/cache.rs:A shared store behind a password]
- **A stored entry's CRC is not checked, so a corrupted one is answered as content.** A deflate entry
  that has been corrupted fails to decode and is refused; a stored one is handed back, and written,
  as it stands. The module reads its own job as what an archive is *allowed* to do rather than
  whether it survived a disk, but nothing states that as a rule, and the recorded CRC is not exposed
  either, so a caller cannot make the check the class declines to — `Core\Hash`'s `Digest::Crc32`
  (`docs/spec/01-core-library.md:825`) computes exactly this value and has nothing to compare against.
  What has to be decided is which of the three it is: verified by default, an argument, or a non-goal
  with the recorded value handed out. `crates/nvs-stdlib/src/zip.rs` gap 2.
  [until: gone crates/nvs-stdlib/src/zip.rs:An entry's CRC is not checked]
- **EBML's magic is distinctive and the table has no row for it, so a `.webm` and a `.mkv` both
  answer `Unknown`.** The two containers share one magic and are told apart by the `DocType`
  element, which is a parse rather than the prefix comparison
  `crates/nvs-stdlib/src/mime.rs`'s table is built out of, so answering `video/webm` for either
  would be confidently wrong rather than usefully silent. What has to be decided is the third option
  nobody has taken: a shared `Core\Mime\Type::Ebml` case, naming the container the octets actually
  show the way `Zip` does — honest, and a case a caller cannot narrow — against leaving a format
  with a perfectly good magic detecting as nothing at all. No rule owns `Core\Mime`, and spec § 17's
  row defers the roster to that module doc. `crates/nvs-stdlib/src/mime.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/mime.rs:EBML's magic is shared]
- **`Core\Xml` reads a qualified name as its spelling, so no `xmlns` declaration is resolved.**
  `<x:a/>` answers `x:a`, which is right for a document a program controls and not enough for one it
  does not: two documents meaning the same thing under different prefixes compare unequal. What has
  to be decided is not whether resolving one is possible but where the resolved answer would live —
  both shapes answer one node family, the one `rule:core-classes/html-parsing` makes both parsers
  produce (`rule:core-classes/xml-tree-and-stream`), so a namespace URI on a node is a change to a
  family the HTML parser fills too and HTML has no namespaces to put there. Goal `xml-tree` is
  retired and no milestone's plan names namespaces. `crates/nvs-stdlib/src/xml.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/xml.rs:prefix and all]
- **A `Core\Ast` node carries no position and no source text, so a walk classifies but cannot
  quote.** A `#[Test]` that walks the tree can fail today and cannot name the `file:line` it failed
  about, and a structural rule that cannot point is a check rather than a report
  (`docs/adr/tooling-parity.md`, the Deptrac row). What has to be decided is a qualifier question
  rather than a slot: that module's § *Decision: the tree is inert because there is nothing in it to
  run* says a node answering its own source text is the point at which `parse`'s `$source` stops
  being `Qual::Neutral`, so the text comes back out `tainted` and every consumer of it becomes a
  sink question. `crates/nvs-stdlib/src/ast.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/ast.rs:carries no position and no text]
- **A `secret` value held in an `array<T>` element or in a shape literal's field is dumped in full**,
  because redaction reads a *declared* property's bit off `nvs_runtime::ClassDesc::field_is_secret`
  and neither container has a declaration to read: `rule:security/secret-qualifier` does not compose
  the qualifier onto an element type, and a `rule:types/object-top` shape field's type is inferred
  from its initializer rather than written. Both ends that exist are closed — the argument is refused
  where the call is written and a `secret` property is a `nvs_render::Node::Redacted`. What has to be
  decided is whether the qualifier composes onto a container at all, which answers once for the type
  lattice and is not a dump walk's call to make. `crates/nvs-runtime/src/record.rs` gap 1.
  [until: reviewed 2026-09-10]
- **An enum case dumps as its backing integer**, because an enum has no tag of its own at run time
  and a case arriving through `mixed` is indistinguishable from an `int`. The rendering half is not
  the hole — `nvs_render::Node::EnumCase` exists and is what a producer with a static type would
  build — so what has to be decided is whether an enum earns a representation of its own, a tag or a
  bit the roster can test. That answers for every `mixed` consumer at once rather than for this walk,
  and `rule:types/conversion` is where it lands. `crates/nvs-runtime/src/record.rs` gap 2.
  [until: reviewed 2026-09-10]
- **Seven of `Core\Regex`'s eight pattern parameters refuse a `tainted` argument by default rather
  than by a mark**, because `nvs_types::core_lib`'s `qual_of` answers `None` for a
  `crate::registry::CoreTy::Union` and those seven take `Pattern|string`.
  `rule:security/regex-pattern-is-a-sink`'s refusal holds either way, `None` and `Qual::Sink` both
  refusing, so what is open is the spelling. What has to be decided is whether a union type carries a
  `Qual` slot at all: without one a union that ever wanted `Qual::Launder` has nowhere to hold it,
  which is a registry-shape question and not a `Core\Regex` one.
  `crates/nvs-stdlib/src/regex.rs` gap 1. [until: reviewed 2026-09-10]
- **`Core\Regex::matchAll` reports positions in O(n·k)** over an *n*-byte subject with *k* matches,
  because `crate::granularity::Unit::index_of_byte` counts from the start for each one. The matches
  arrive in increasing order, so the fix is a cursor counting only the gap since the previous match
  and it is one function. What has to be decided is what a position means when a cluster spans a
  match boundary — the case that makes the cursor's answer differ from the restart's — and no
  milestone claims a stdlib member's complexity curve, M12 being the JIT tier.
  `crates/nvs-stdlib/src/regex.rs` gap 3. [until: reviewed 2026-09-10]
- **`Core\Cli::arguments` is empty inside a served request**, because it answers whatever the launcher
  wrote with `nvs_runtime::Ctx::set_command_line` and only `nvs-cli` writes one. Empty is not wrong —
  a request has no command line — but the member is rostered for every program rather than for a CLI
  one, and nothing this module can reach decides which. What has to be decided is what the whole of
  `Core\Cli` means off the terminal: an empty roster, a refusal, or the host's own argv, and the same
  answer settles `colorDepth` and the output sink. `crates/nvs-stdlib/src/cli.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A generated help page names neither a parameter's declared type nor its default**, because § 6's
  table deliberately carries no declared type and `nvs_types::commands`' own gap 1 refuses inventing a
  second copy of the signature the handler already holds — so rendering the default alone would read
  as though the type were absent from the declaration. What has to be decided is whether a help row
  reaches that signature at render time rather than carrying a copy of it, which is a table-shape
  question in `nvs-types` and not a renderer's. `crates/nvs-stdlib/src/command.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A UNC path re-renders without the doubled separator that makes it one**, because
  `\\server\share\f` parses as an ordinary absolute path whose components are `server`, `share` and
  `f`. Spec § 8's roster is whole, so this is a modelling hole and not a missing member, and the fix
  is a third root shape beside `Parts::drive` rather than a change of interface. What has to be
  decided is whether Novis models UNC at all: nothing on the path to `examples/collect.nvs` writes
  one, and a root shape no supported deployment reaches is surface bought for nothing.
  `crates/nvs-stdlib/src/path.rs` gap 1. [until: reviewed 2026-09-10]
- **A drive-relative path is one component named `C:log`**, because `split_drive` declines a drive
  with no separator after the colon, so Windows' "the current directory *on* drive C" round-trips as
  a relative path rather than as a root. The obvious fix is worse — treating `C:` as a root makes
  `Path::split('a:b')` answer `['a:', 'b']` for an ordinary relative path — so what has to be decided
  is whether a drive-relative root is distinguishable from a colon inside a segment at all, which is
  a grammar question spec § 8 does not answer. `crates/nvs-stdlib/src/path.rs` gap 2.
  [until: reviewed 2026-09-10]
- **`Core\Random\Seeded` is not registered, and the tree states two designs for it.** Spec § 11 makes
  it a separate object with the same members whose guarantee is reproducibility rather than
  unpredictability, while `docs/novis.md`'s `Core\Random` chapter says there is no seeded generator
  under any name. ADR 0079 § 12 takes the second route without retiring the first: `#[Test(seed:)]`
  seeds `Core\Random` itself for the isolate a test runs in, which is the reproducibility a test needs
  and nothing a program outside one can reach. What has to be decided is which of the two the language
  means, since a class registered under § 11 would be the thing § 12's attribute exists to make
  unnecessary. `crates/nvs-stdlib/src/random.rs` gap 1. [until: reviewed 2026-09-10]
- **Two typed body members in one handler are admitted, and the last one wins**, because
  `rule:security/response-body-is-one-typed-member`'s compile error is written over `echo` and a
  typed writer rather than over two typed writers, and `E0801` enforces what is written. Its
  reasoning covers both — they disagree about the body's content type, and letting the last one win
  is how a JSON endpoint acquires an HTML prelude — so what has to be decided is whether the rule
  extends to the pair it does not name, which is an amendment to the rule rather than a check the
  class it is enforced against can add. `crates/nvs-stdlib/src/response.rs` gap 2.
  [until: reviewed 2026-09-10]
- **`Core\Test::assertEquals` names `assertEqualsDeep` at run time where
  `rule:testing/assertions-are-typed` refuses at compile time**, because the refusal wants the
  argument's class graph and `nvs-stdlib` holds none — `nvs_types` does, and
  `crates/nvs-types/src/core_lib.rs:84` already seeds `Comparable` onto the `Core` classes carrying
  `compareTo`. What has to be decided is whether an assertion joins
  `rule:expressions/intrinsic-list-is-closed`'s closed list of the `Core` members whose argument the
  checker reads, since that list is closed by rule and a member added to it is a rule change rather
  than a check. `crates/nvs-stdlib/src/test.rs` gap 1. [until: reviewed 2026-09-10]
- **`assertThrows` matches nothing when no exception class table is installed**, because
  `nvs_runtime::Ctx::pending_conforms_to` reads ancestry off a descriptor and a helper-raised failure
  carries none until `Ctx::set_runtime_error_class` has run — which a compiled unit always does, so
  only a host embedding the runtime itself reaches it. What has to be decided is whether such a host
  is a supported configuration: if it is, this member owes a refusal rather than a silent non-match,
  and if it is not, the requirement belongs to whatever states the embedding contract rather than to
  an assertion. `crates/nvs-stdlib/src/test.rs` gap 2. [until: reviewed 2026-09-10]
- **`rule:classes/definite-property-initialization` is checked over the part of a constructor body the
  walk can see, and three corners sit outside it.** A property backed by a `set` hook is exempted
  rather than checked against whether the hook commits a value; `scan_expr` descends into a handful of
  composite expression forms, so a `$this->prop = ...` inside a closure body or a `match` arm is
  diagnosed spuriously rather than missed silently; and a class with no explicit `constructor` is
  never checked against the `parent::constructor(...)` obligation, since it has no body of its own for
  such a call to sit in. These are M4 checker holes and M4 is carried: that milestone's loop goal
  passes, and no live entry on the chain writes this crate's checking passes again. What has to be
  decided is
  whether this pass models a hook's body at all — `rule:classes/lateinit`'s *Revisiting* section
  defers the hooked-property question to `docs/spec/`, so it is a language decision before it is a
  checker one — and whether PHP's inherited constructor is modeled here or left to the runtime throw.
  `crates/nvs-types/src/ctor_init.rs` gaps 1–3. [until: reviewed 2026-09-10]
- **`rule:classes/lateinit`'s compile-time half stops at the declaring class.** An inherited
  `lateinit` property read through `$this` in a subclass's own method is not checked by this pass,
  which tracks a class's *own* properties only and leaves that read to the rule's runtime throw;
  `scan_expr` shares `crate::ctor_init`'s partial descent, so a read inside a closure body or a
  `match` arm is silently unchecked; and a `set`-hooked `lateinit` property is not modeled specially.
  What has to be decided is whether the pass walks the `extends` chain — `own_required_properties`
  took the opposite decision for the same reason, so taking this one alone leaves the two passes
  inconsistent with each other — and the hooked case is the same `docs/spec/` question that rule's
  *Revisiting* section defers. `crates/nvs-types/src/lateinit.rs` gaps 1–3.
  [until: reviewed 2026-09-10]
- **A named constant is a legal property default and an illegal parameter default.** An enum case
  (`Mode $m = Mode::Fast`) or another class's `const` becomes a `nvs_runtime::FieldDefault`
  materialized straight into a slot, where the case *is* the integer it folded to; at a parameter
  default `nvs_ir::lower::emit_const_arg` would have to hand a `ConstArg::Int` to a
  `nvs_ir::ty::Ty::Enum` position, so `literal_default` refuses it as
  `E_PARAM_DEFAULT_NOT_LITERAL`. What has to be decided is whether that emitter carries the
  position's own IR type rather than the constant's — which closes the parameter half by widening
  `literal_default`, and changes what every omitting call site emits — or the asymmetry stands as a
  stated bound of a parameter default. `crates/nvs-types/src/defaults.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A mount's entry script may echo a page and call a response body member, and nothing refuses it.**
  `rule:http-server/a-request-resolves-in-five-steps`'s steps 4 and 5 run a `.nvs` file's top-level
  frame as the request body, which is the one row of the shared-output table this pass does not
  refuse; the same file is one compiled unit whether `nvs serve` ran it or `nvs run` did, so a static
  refusal there would refuse the CLI use of every such file. What has to be decided is whether the
  language gains a declaration that a file is an entry — the fact that is missing, and a surface
  question rather than a checker one — or the row stays answered at run time by the default this
  module's own doc names, where `echo` means `text/html` and a body member wins the `Content-Type` it
  declared last. `crates/nvs-types/src/response.rs` gap 1. [until: reviewed 2026-09-10]
- **A `Core` target on `extends`/`implements` is trusted to exist.** `QName::is_core` is a spelling
  test, and this crate cannot hold a name to `nvs_stdlib::registry`'s roster the way
  `nvs_types::expr::calls` holds a `new` target to it: `crates/nvs-hir/Cargo.toml` depends on
  `nvs-diagnostics` and `nvs-syntax` and nothing else, which is the graph position that lets a class
  link resolve before the stdlib exists at all. What has to be decided is which end closes it — a
  roster this crate is handed at construction, or the `Core` half of a link checked in `nvs-types`,
  which already reaches the registry — because the second answer makes one link error arrive from a
  different pass than every other. `crates/nvs-hir/src/hierarchy.rs` gap 1.
  [until: reviewed 2026-09-10]
- **`require`'s statically-known path is a plain string literal and nothing else, with three more
  corners around it.** Heredoc/nowdoc and any expression built out of a literal — a concatenation, a
  `const`, an `as` conversion — is dynamic here even where a reader could work the value out; the
  double-quoted cooker recognises a practical escape subset and leaves octal/hex/unicode un-cooked;
  the name harvest is an over-approximation whose miss costs a class that fails to autoload; and
  `rule:packaging/autoload-probes-fold-into-the-cache-key`'s probe trace is produced and then
  dropped. What has to be decided for each is where it lives rather than what the code is: a constant
  folder that runs before the graph is walked, a string-literal cooker something besides this module
  needs, a harvest derived from the AST rather than hand-written so a new node cannot be missed, and
  whether the artifact key grows the `PathEntry` table
  `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` names and nothing has built.
  `crates/nvs-hir/src/requires.rs` gaps 1–4. [until: reviewed 2026-09-10]
- **A `type` alias's own name is held to no casing rule.** `rule:core-api/identifier-casing`'s scope
  table lists the categories the checker enforces and "type alias" is not one of them, so the
  declaration walk passes over the one name it could check and does not guess a rule. What has to be
  decided is whether that table gains the row — a rule change rather than a check, and the reason
  this sits here rather than in the pass: the pass is four lines once the rule says which casing an
  alias takes. `crates/nvs-syntax/src/casing.rs` gap 1. [until: reviewed 2026-09-10]
- **`compiler_version_hash` is the release version, so two builds of the same version share an
  artifact key.** A developer who rebuilds the compiler without bumping `CARGO_PKG_VERSION` keeps
  every artifact keyed against the old one; `debug_assertions` separates a debug build from a release
  build and nothing separates two debug builds. What has to be decided is whether the workspace takes
  a `build.rs` for a stamp that moves with the source — this crate has none, and a stamp is a
  workspace-wide build dependency rather than a line in this key — or a developer's stale artifact
  stays their own problem. `crates/nvs-config/src/cache.rs` gap 1. [until: reviewed 2026-09-10]
- **A bundled program resolves a `require` and not an autoload root.** `rule:programs/no-runtime-autoload`'s
  probing lists real directories and is not routed through the embedded payload, so a name reached
  only through an autoload root does not resolve inside a bundle, while
  `rule:packaging/a-bundled-require-resolves-at-build-time` makes `require` closed-world and does.
  What has to be decided is whether a bundle's build resolves its autoload roots into the payload —
  which makes the root set part of what `nvs build --compile` freezes — or an autoload root is
  declared unsupported in a bundle and refused where it is written.
  `crates/nvs-diagnostics/src/embedded.rs` gap 1. [until: reviewed 2026-09-10]
- **The stack ceiling is asserted rather than discovered.** `Ctx::new` arms from the stack pointer at
  construction and a fixed `STACK_CEILING`, which is correct on a stack at least that deep and
  permissive on a shallower one — where the guard page is still reached first and `enable_probestack`
  still makes that a clean crash rather than a stack clash. Reading a thread's true bounds needs a
  platform call this crate has no dependency for, and the module doc expects the request's stack to
  become Novis's own to size at M6, which that milestone's plan does not state. What has to be
  decided is where a request's stack comes from at all: a platform dependency that reads the running
  thread's bounds, or a stack the runtime allocates and therefore already knows.
  `crates/nvs-runtime/src/ctx/mod.rs` gap 1. [until: reviewed 2026-09-10]
- **The door decides which requests a CSRF check covers and refuses none of them, and the route label
  it derives has no caller.** Verification is `rule:security/protocol-roster`'s constant-time
  comparison against a key bound to the issuing session, and this crate has neither half in reach:
  `Core\Csrf::verify` is `nvs-stdlib`'s, deliberately not a dependency, and no `[http]` directive
  names a key. The label is what `rule:observability/route-label-is-the-declared-name` reaches, and
  no core owns a metrics registry because nothing exports one yet. What has to be decided is which
  side verifies a token — the door with a configured key, or a handler calling `Core\Csrf` — and
  which crate owns the registry a label lands in. `crates/nvs-server/src/route.rs` gaps 1 and 2.
  [until: reviewed 2026-09-10]
- **A write to a field of `Core\Task::all`'s result is checked against the tags of the literal that
  started it.** The result is built with the argument's own shape class, because
  `rule:types/object-top`'s shape class is named for its field names alone and those are identical on
  both sides, but that descriptor's per-slot tags come from the literal, where every field is a
  closure — so `$page->count = 5` is refused with a message naming `object`, while reading is
  unaffected. What has to be decided is whether `nvs-ir` records the *result* shape's representations
  at the call site, which degrades the shared class's tags to unchecked exactly as two disagreeing
  literals of the same shape already do, or the two shapes stop sharing a class at all.
  `crates/nvs-stdlib/src/task.rs` gap 1. [until: reviewed 2026-09-10]
- **The queue's schema has no spelling a program can reach, so a conformance case over a converged
  queue writes the DDL out itself.** `nvs_stdlib::queue::schema()` is the one home and `nvs queue
  migrate` is the only thing that applies it, and a `.nvst` case reaches no operator subcommand — so
  the three `tests/conformance/core/queue-*-on-sqlite*.nvst` cases each carry the text that command
  prints under `--dry-run`, and a schema change that a statement reads breaks them rather than
  drifting past them. What has to be decided is whether the value gets a `Core` spelling — which a
  deploy script would use as much as a case would, and which is a surface addition nothing has asked
  the user for yet. `crates/nvs-stdlib/src/queue.rs`'s `schema`.
  [until: gone tests/conformance/core/queue-cancel-releases-a-dedupe-key-on-sqlite.nvst:create table nvs_jobs]
- **A parameter hint is drawn only for the call variant that carries parameter names**, because
  `nvs_types::ResolvedCall::param_names` is reached through `ExprInfo::Call` and a `new`, a call
  through a `callable` signature and an erased call on a `mixed` receiver each record a different
  variant — one of which has no names to give at any price. What has to be decided is whether the
  *type* phase records more for every program so the editor can draw two more hints: widening is a
  table question, and the table is on the compile path while the hint is not, so the cost lands on
  `AGENTS.md`'s priority 3 to buy something at priority 4. `crates/nvs-lsp/src/hints.rs` gap 1.
  [until: reviewed 2026-09-11]
- **Emmet and HTML validation do not reach a template region**, where completion, hover, the colour
  picker and tag renaming do. Forwarding runs a *provider* over a virtual document, and neither of
  those two is one: Emmet expands from the language of the document the cursor is in, and validation
  is published by the HTML service for the documents it owns.
  `rule:ide/a-template-region-gets-the-editors-services-and-formatter` names both, so what has to be
  decided is whether `emmet.includeLanguages` mapping `nvs` to `html` — which turns abbreviation
  expansion on in the Novis half of the file too — is the trade, or whether the client grows a
  second, real document the service can own. `editors/vscode/src/regions.ts` § *What forwarding does
  not reach*. [until: reviewed 2026-09-11]
- **A child written as a path runs on its parent's core however it is placed**, so `spawn script
  "child.nvs" with(on: "worker")` buys nothing and says nothing, which is the silent fall-through
  `docs/decisions/0184.md` § *Diagnostics* rejects for the one failure it did foresee. The method
  form crosses because a label is looked up in a class table every core reads, while a path becomes
  code through a resolver only the booting thread was installed with. What has to be decided is who
  owns the unit cache across cores — a resolver a worker core can reach is the fix, and refusing the
  spawn instead would take a whole form away from `on: "worker"` to buy honesty.
  `crates/nvs-host/src/placed.rs` § *Known gaps*, and `crates/nvs-host/src/group.rs`'s the same gap
  seen from the seam above it. [until: reviewed 2026-09-14]
- **A serving core offers itself as no destination**, so a worker placement under `nvs serve` starts
  one of the lazily started worker cores rather than reaching the sibling serving core
  `docs/decisions/0184.md` § 5 decides on — which is that record's *Revisiting* fallback, taken
  without the measurement its trigger describes. `rule:concurrency/on-worker-runs-the-child-on-another-core` is written as what runs, so nothing
  observable is wrong; what it costs is a thread per placed-to core on a process that already has one
  per core. What has to be decided is whether a serving core registers an inbox as it starts, which
  is a question about `nvs serve`'s boot order and not about the crossing.
  `crates/nvs-host/src/worker.rs` § *Known gaps*. [until: reviewed 2026-09-14]
- **The test suite makes and joins its isolates one at a time**, so a suite's wall time is the sum of
  its cases where `docs/decisions/0079.md:158` promises an isolate per test *and* a parallel suite,
  and that record's milestone table carries both to a milestone the program has passed
  (`docs/decisions/0079.md:872`). Only the parallelism is open — every case already runs in an
  isolate of its own — so what has to be decided is what bounds it: a runner that spawns the suite as
  a task group buys `rule:concurrency/limit-and-deadline-are-the-only-bounds`'s two bounds and owes a
  decision about a case that reads the terminal. `crates/nvs-cli/src/runner.rs` gap 1.
  [until: reviewed 2026-09-14]

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
