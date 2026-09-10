# The parity program, as six loop goals

Goals `core-depth` through `server` of [the plan](../../implementation-plan.md) are one continuous unattended run: **PHP core
feature parity, all five SQL drivers, concurrency, governance and the server.** This directory holds it,
cut into six goals, and [the goals directory](README.md) is the order the driver walks them in. A seventh,
post-parity goal — `rule:core-classes/temporary-dir-sweep`'s
temporary-directory sweep — rides the same chain after the program's gate, because its server half needs
the `nvs-server` goal `server` creates. An eighth, [`Core\Program::id()`](10-program-id.md), follows it: one
member exposing the program fingerprint over hashes the artifact cache already computes. A ninth,
[`Core\Db\Schema`](11-schema.md), closes `rule:core-classes/db-one-api`'s own *Revisiting* item and
sits there because its acceptance property needs every driver goal `database` builds to be finished. A tenth,
[a typed `callable`](12-typed-callable.md), follows it: `rule:types/callable-signature`
gives the type a function value's parameters and return, and it goes after every goal that *writes*
callbacks so their registry rows are converted once rather than twice. An eleventh,
[doc comments](13-doc-comments.md), is last —
`rule:tooling/doc-comment-is-three-slashes`'s `///`, whose stage 2 builds
`rule:ide/one-grammar-one-tree`'s trivia layer because a doc comment cannot be read without it, so **M4B starts with its
own tree half already done**.

**Then M4B itself, as four goals** — [resilient-tree](14-resilient-tree.md),
[surface](15-surface.md), [lsp-server](16-lsp-server.md) and [editor](17-editor.md). Goal `surface` is
not editor work: it is `rule:expressions/pipeline-substitution`'s pipeline operator and `rule:php-migration/a-deprecation-is-a-refusal`'s PHP 8.6 refusals, the two M1 items
scheduled after M4 and never taken, and it sits before the grammar because a grammar written against a
surface about to change is written twice.

**Then two entries the user added after the chain was written**, both about the same thing from two sides:
what a program may read off a request, and what a test may say to build one.
[request-json](18-request-json.md) replaces spec § 15's three-way body exclusivity with the body-read rule it
opens — *buffering readers share, streaming readers consume* — adds `Core\Request::json()`/`jsonAs<T>()`, and gives
`.nvst` the `.phpt` request sections — without which no request-facing member can be proven by a case at
all. [test-request](19-test-request.md) freezes `Core\Test::request`'s shape (`rule:testing/in-process-request` has an
example and no signature), builds it as one shared `InboundSpec`, and lands the peer fields that
`Core\Request::clientIp`/`scheme`/`host` have been waiting on. They are last rather than beside goal `server`
because they were decided after it, and goal `request-json`'s stage 0 is what pays off the fixtures goal `server` wrote
against the rule it replaces.

**Then a third, from the same conversation**: [input-shapes](20-input-shapes.md) is what stops a
request reader's answer being `mixed`. Goal `input-shapes`'s record gives `Core\Arr` one converter from `array<mixed>` to a
declared shape and `Core\Request` the two members over it, so untrusted data is checked once, where it
arrives and where a `400` is still the right answer. Its stage 2 is type-surface work the other two need
nothing of and everything before it would have had to write twice — `rule:types/shape-type`'s shape gains an
optional field, and `rule:security/tainted-qualifier`'s qualifier learns to sit in front of one — which is why it comes after
the other two.

**Then a fourth**: [parses](21-parses.md) is the one the user asked for after that conversation ended.
Four binding surfaces — a route `{capture}`, a `#[Query]`, a command argument and an option — all ask
`nvs_types::commands::converts_from_string` whether a type can be built from text, and its class arm is a
comparison against the string `Core\Uuid`. Goal `parses`'s `Parses` record is what that arm becomes: one global
interface on `Comparable`'s precedent, one required member and one default body, so `Core\Uuid` reaches
the door through the same contract as a user's own `Slug` and stops being a name in four match arms. Its
stage 4 closes `nvs-runtime`'s two standing conversion gaps, which is why it goes after the goals that
wrote them. Goal `parses` is the last entry the parity program itself needs.

**What goal `parses` deliberately does not do**: give `as` a class-building meaning. That was the shape the
proposal arrived in, and the operator half was rejected — `mixed as Foo` is already a checked downcast,
`as` is a closed laundering set, and `X as ?Foo` would need two inputs to decide its legality. `rule:expressions/nullable-conversion-availability`'s *the class row is absolute* survives intact; only the sites that already convert implicitly change.

**Then one more, added after goal `parses` was written**: [unix-sockets](22-unix-sockets.md) is
`rule:config/cache-shared-is-the-grant-over-the-configured-store` — the grant over a
store an operator configured stops naming a host (`cache.shared`, unscoped, on `mail.send`'s precedent),
which removes the loopback double-grant *and* the obstacle to a Unix socket, since `pin_host` needed an
address and a socket path has none. Its stage 4 puts an `AF_UNIX` connect under three of goal `database`'s
drivers. It goes in front of the dossier because that entry stops adding surface and this one adds some.

**Then, before any of that, two entries the user added** — inserted directly after goal `server`
rather than on the end, because goal `server` going green is what *makes* the problem they close.
[carried-gaps](7-carried-gaps.md) takes every gap a shipped feature already carries that no entry on
this chain claimed: an ADR-written `db.open` wildcard with no reader, `nvs check` never building the
grants its own diagnostic needs, a cycle closed through an array surviving `rule:security/isolate-teardown-is-a-drain-then-a-sweep`'s sweep, `rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`'s four missing log-record fields, `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`'s unarmed fleet lease, spec § 18's
`stream`/`streamAs`, two rules that were waiting on a diagnostic band that has since opened, and the
CLDR rosters that throw. Its keystone is the mechanism rather than any of those: an outstanding-members
key gains an owner column and the test fails when that owner is no longer a live entry, so a switch
cannot orphan work silently again. [warm-start](8-warm-start.md) is `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact cache, which
goal `governance` built exactly as specified and which has never had a caller — a subsystem rather than a gap,
because the payload needs a second `nvs-codegen` `Module` and a named symbol for every host address the
JIT bakes in, which is why it is its own entry.

**Then six entries the user asked for**, from one question — what is *unowned*, and can it
be made reachable? Answering it turned up two facts the repository had wrong (`Core\Metrics` was listed
unowned and is goal `server`'s; spec § 17's four classes were filed under M9, which carries the extension system
and none of them) and one it did not record at all: **50 `# Known gaps` blocks across the crates hold 152
enumerated items**, of which `carried-gaps.md` indexed 22 and `carried-refusals.md` 15. These six close
the real ones. [per-core](23-per-core.md) is M7's own scope that goal `server` shipped around, and the largest
measured performance item in the repository. [net-os-signal](24-net-os-signal.md),
[formats](25-formats.md) and [xml-tree](29-xml-tree.md) are M8's Tier 0 roster finished — the seven
`Core` classes `rule:core-api/tier-roster` names that M8's own goals walked past, after which
`spec-classes-part-two-outstanding.txt` holds no keys at all. [gap-owners](30-gap-owners.md) is goal
7's keystone applied one level down: a module-doc gap gains an owner tag and a gate fails on an untagged
one, so the ~110 unindexed items become a short list of scheduling questions instead of an unread
inventory. [unowned-sweep](31-unowned-sweep.md) closes what is left, four fifths of which is one
blocker — an options bag the registry could not spell, which is what goal `input-shapes` lands.

**Then one entry the user asked for**, and it is the first since goal `unix-sockets` that *adds* a
surface rather than closing one. [signed-urls](32-signed-urls.md) is
`rule:core-api/signing-is-over-a-payload`: Novis could
sign a cookie and a JWT and could not sign a link, which is what a password reset, an unsubscribe, a
download and a tamper-proof AJAX endpoint all are. It lands `Core\Signature` — `rule:security/protocol-roster`'s fifth and
final roster entry, over a payload map — and the two doors onto it, `$uri->sign` and `Core\Router`'s
pair. Its whole argument is that it invents **no** canonical form: `$uri->sign` signs what
`$uri->compareTo` already normalizes, which is where every other language's version of this feature has
gone wrong. It sits after goal `unowned-sweep` because it adds surface and 28 is the last entry that only closes,
and because `{keys, until}` needs the options bag goal `unowned-sweep` stage 2 lands.

**Then one the user asked for after reading the queue.** [queue-purge](34-queue-purge.md) is the other
half of a class that could create a job and not remove one: `nvs_jobs` grows with every job a deployment has
ever run, a cancelled batch of forty thousand leaves forty thousand rows, and the dead-letter table the
runtime is right never to sweep has no spelling an operator can sweep either — so the only answer today is
raw SQL against tables the runtime owns, which makes their column names a public contract by use.
[ADR 0153](../../decisions/0153.md) closes it with one column, two members, one capability and one
diagnostic, and its whole argument is that none of the four is new: `delete` is `cancel`'s twin, `purge` is
`stats`', the grant is `db.schema`'s shape, and `E0635` is `E0618` one class over. The `tag` column is the
only invention, and it exists because `key` means *at most one pending job* and a group means *many* — the
two are opposites at the point they touch. It sits here rather than beside goal `database` because what makes the
schema change cheap is goal `schema`'s converge: a nullable column with no default grades `Safe`, so a live
deployment takes it through the `nvs queue migrate` it already runs.

**Then its pair, from the next question in the same conversation.**
[serve-runs-the-queue](35-serve-runs-the-queue.md) is the other end of the same subsystem: goal `queue-purge` gives
the queue its missing member and this gives it its missing process. `[[schedule]]` fires under `nvs serve`
and `[queue] workers` does not — the workers are wired into `run_run` and nowhere else, so the key is read,
validated at boot and then silently ignored by the binary a deployment actually runs, which is why every
production deployment needs a second process it was never told about.
[ADR 0154](../../decisions/0154.md) closes it with one `Option` and one call in `serve.rs`, one predicate in
`worker.rs`, four directive rows and one sentence of help text. Its load-bearing half is the stop condition
rather than the arming: the server's loop ends when nothing is parked and a polling worker is always
parked, so a worker that ignores the drain is a server that cannot be stopped — which is why the goal's
stage 2 lands the predicate before stage 3 arms anything. The command keeps its name; § 6 of the record is
why, and the sentence under it is what changes.

**Then one inserted in front of all of them**, from the user's question about how anyone will write a
language no model has distilled. [agent-surface](28-agent-surface.md) is
[0167](../../decisions/0167.md) built: `nvs agent` answers a coding agent from the registry the binary
already carries — a generated primer, one line per member, a search over those lines, one card — and
`nvs agent init` writes one pointer per harness, none of which states a language fact of its own. It
sits early rather than late because every entry behind it is a consumer, and because it depends on
nothing that has not already shipped. Its record's investigation is the reason it is a query surface
and not a smaller document: three agents given the same task and three different documents all wrote
correct Novis quickly, and all then lost most of their budget to the same wall —
`rule:security/capability-declaration-is-one-table`'s table, `shipped`, naming two renderers that do
not exist. Stage 0 is that audit, and stage 2 gives the table the renderer it always claimed.

**Then one more the user asked for**, after reading what `Core\Sse` actually does.
[event-streams](41-event-streams.md) is M7's last unlanded piece and the one goal `server` left behind.
`rule:concurrency/a-stream-that-outlives-its-request-is-a-connection` draws a line between a streaming
response and a connection isolate, and **neither side of it is built**: `Answer` is an `Option<Bytes>`
with an exact `size_hint`, so nothing in this workspace can write a byte to a client after the head has
gone out. That is what makes `Core\Sse::upgrade` — registered, cell-carried and started in the right
order since goal `server` — a door onto nothing, and it is why ADR 0083 § 5's aside that a streaming response
is something "M7 already builds" has been false since it was written. The goal builds the body cell once
and spends it twice, on the two spellings the rule already names, plus `Core\Response::stream` for the
untyped case. It sits after goal `resource-ceilings` because a long-lived connection isolate is exactly the runaway shape
that goal's ceilings exist to stop — an event stream whose budget is declared and unenforced is priority
1 spent to buy priority 3 — and its own § 5 amendment is the interesting half: "send and no `receive`"
becomes "no *peer*", so an event stream can wait on a topic and fan-out stops being a poll.

**Then the fourth ending**, which the user asked for after reading what `exit` costs a request.
[finish-response](42-finish-response.md) gives a program a way to say *this response is finished* from
any frame — one that is neither a failure nor a termination, so every `finally` on the way out runs,
the exit queue fires, and `Core\Task::afterResponse`'s work drains. Today the only early exit is
`exit`, which runs no `finally` and drains no deferred work, so the PHP reflex `echo json_encode($x);
exit;` silently drops both halves of a request's cleanup with no diagnostic anywhere. It sits after
goal `event-streams` because that goal makes a response body two-valued and *what it means for a
response to end* has to be one answer across a buffer and a stream alike. Its keystone is the unwind:
only a `THROWN` takes the handler edge a `finally` lives behind, so a fifth status would be `exit`'s
mechanics under a new name — the goal is scheduled on the marker-object line instead, and § *Standing
decisions* closes the question. Its stage 2 is a defect it inherits rather than causes:
`Core\Script::onExit` never fires for a served request, because all three `run_exit_hooks` call sites
are the CLI's.

**Then the chain turns around.** [dossier](44-dossier.md) is the last hand-written entry and it writes
no proof of its own: one session runs `python tools/dossier.py --emit-goals`,
which puts `rule:testing/four-proofs`'s
whole roster — one goal per group of shipped features owing their four proofs — onto the end of *this*
chain, and then spends the rest of the session on an **optimization pass aimed forward** rather than
back: it is the only moment anyone holds all 93 generated goals at once and none of them has been walked,
so the shape they share is cheapest to fix there. The sweep said 795 features with one
complete, which emits as 93 goals over 794 owed. That pass is why
[optimization-prompt.md](../optimization-prompt.md) now carries menu item 8 — a defect in a generated
goal is fixed in the emitter and re-emitted, never by hand — which is also what the *automatic* pass
reaches for once the loop is walking goals a tool wrote. Everything after `dossier` is therefore
generated, and the run continues into it without a
restart: `Chain.refresh()` re-reads the directory when a goal goes green, adopting anything past the goal
the run is on and refusing a rewrite of one it has already walked — `.loop/chain.json` names the goal the
run stands on and every switch has folded one walked goal's checks into the next, which is what a rewrite behind
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
| [core-depth](1-core-depth.md) | M4S tail | `nvs-stdlib`, `nvs-types`, `nvs-hir`, `nvs-cli` |
| [concurrency](2-concurrency.md) | M5 | **`nvs-host`** (new), `nvs-runtime`, `nvs-stdlib` |
| [governance](3-governance.md) | M6 | **`nvs-config`** (new), `nvs-host`, `nvs-cli`, `nvs-codegen` |
| [core-part-ii](4-core-part-ii.md) | M8, non-database | `nvs-stdlib`, `nvs-host` |
| [database](5-database.md) | M8, database | **`nvs-db`** (new), `nvs-stdlib`, `nvs-types` |
| [server](6-server.md) | M7 | **`nvs-server`** (new), `nvs-stdlib`, `nvs-host` |
| [temp-sweep](9-temp-sweep.md) | post-parity, `rule:core-classes/temporary-dir-sweep` | `nvs-runtime`, `nvs-host`, `nvs-stdlib`, `nvs-config`, `nvs-server`, `nvs-cli` |
| [program-id](10-program-id.md) | post-parity, `rule:programs/no-runtime-autoload` changed by a record | `nvs-config`, `nvs-hir`, `nvs-runtime`, `nvs-stdlib` |
| [schema](11-schema.md) | post-parity, one ADR slot | `nvs-db`, `nvs-stdlib`, `nvs-cli` |
| [typed-callable](12-typed-callable.md) | post-parity, `rule:types/callable-signature` | `nvs-syntax`, `nvs-types`, `nvs-stdlib`, `nvs-ir`, `nvs-codegen`, `nvs-runtime` |
| [doc-comments](13-doc-comments.md) | post-parity, `rule:tooling/doc-comment-is-three-slashes` + M4B's tree half | `nvs-syntax`, `nvs-diagnostics`, `nvs-hir`, `nvs-cli` |
| [resilient-tree](14-resilient-tree.md) | M4B, `rule:ide/one-grammar-one-tree`'s other half | `nvs-syntax`, `nvs-diagnostics`, `nvs-test`, `nvs-cli` |
| [surface](15-surface.md) | M1 items 5-6, `rule:expressions/pipeline-substitution` + `rule:php-migration/a-deprecation-is-a-refusal` | `nvs-syntax`, `nvs-diagnostics` |
| [lsp-server](16-lsp-server.md) | M4B, `rule:ide/the-request-set-is-closed`+5 + `rule:ide/redaction-ranges-come-from-the-server` | **`nvs-lsp`** (new), `nvs-cli`, `nvs-types`, `nvs-stdlib` |
| [editor](17-editor.md) | M4B, `rule:ide/highlighting-is-two-layers`+6 | **`editors/vscode`** (new, TypeScript) |
| [request-json](18-request-json.md) | M7, `rule:http-server/a-session-store-answers-four-operations` + spec § 15 | `nvs-stdlib`, `nvs-runtime`, `nvs-test`, `nvs-cli` |
| [test-request](19-test-request.md) | M8, `rule:testing/in-process-request` | `nvs-runtime`, `nvs-stdlib`, `nvs-server`, `nvs-test`, `nvs-cli` |
| [input-shapes](20-input-shapes.md) | M7, one new record + `rule:types/object-top`/0024 changed by a record | `nvs-syntax`, `nvs-types`, `nvs-stdlib` |
| [parses](21-parses.md) | M7, one new record + `rule:classes/comparable`/0066/0077/0102 changed by a record | `nvs-hir`, `nvs-types`, `nvs-stdlib`, `nvs-runtime`, `nvs-cli` |
| [unix-sockets](22-unix-sockets.md) | M8, `rule:config/cache-shared-is-the-grant-over-the-configured-store` + `rule:http-server/allow-url-pins-the-address`/0059 changed by a record | `nvs-config`, `nvs-stdlib`, `nvs-db`, `nvs-host`, `nvs-diagnostics` |
| [carried-gaps](7-carried-gaps.md) | post-parity, `rule:core-classes/db-one-api`/0073/0076/0116/0133 changed by a record | `nvs-config`, `nvs-cli`, `nvs-types`, `nvs-runtime`, `nvs-stdlib`, `nvs-server`, `nvs-db`, `nvs-diagnostics` |
| [warm-start](8-warm-start.md) | post-parity, `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` | `nvs-codegen`, `nvs-cli`, `nvs-config` |
| [per-core](23-per-core.md) | M7, one ADR slot + `rule:http-server/two-deployments-and-nothing-a-proxy-owns`/0017 changed by a record | `nvs-cli`, `nvs-host`, `nvs-server`, `nvs-config` |
| [net-os-signal](24-net-os-signal.md) | M8, one ADR slot + `rule:config/net-local-is-named-and-not-on-the-roster`'s deferred grant | `nvs-stdlib`, `nvs-host`, `nvs-config`, `nvs-runtime` |
| [formats](25-formats.md) | M8, one ADR slot (the shared decompression bound) | `nvs-stdlib`, `nvs-config`, `nvs-diagnostics` |
| [encoder-cycles](26-encoder-cycles.md) | M8, no ADR slot — [0164](../../decisions/0164.md) is already accepted | `nvs-stdlib` |
| [record-origin](27-record-origin.md) | M8, no ADR slot — [0165](../../decisions/0165.md) is already accepted | `nvs-render`, `nvs-runtime`, `nvs-stdlib`, `nvs-ir`, `nvs-codegen` |
| [agent-surface](28-agent-surface.md) | M10, no ADR slot — [0167](../../decisions/0167.md) is already accepted | `nvs-cli`, `nvs-hir`, `nvs-runtime`, and `tools/reference.py` — the surface a coding agent reads the language through |
| [xml-tree](29-xml-tree.md) | M8, one ADR slot + `rule:core-classes/html-parsing`'s change | `nvs-stdlib`, `nvs-diagnostics` |
| [gap-owners](30-gap-owners.md) | post-parity, no ADR — a process gate | `tools/`, every crate's module docs |
| [unowned-sweep](31-unowned-sweep.md) | post-parity, `rule:errors/propagation`/0033/0044 changed by a record | `nvs-stdlib`, `nvs-types`, `nvs-runtime` |
| [signed-urls](32-signed-urls.md) | M8, `rule:core-api/signing-is-over-a-payload` + `rule:security/protocol-roster`/0077 changed by a record | `nvs-stdlib`, `nvs-types`, `nvs-runtime` |
| [type-test](33-type-test.md) | M1, `rule:types/type-test` — the other half of goal `surface`'s reservation | `nvs-syntax`, `nvs-types`, `nvs-ir`, `nvs-codegen` |
| [queue-purge](34-queue-purge.md) | M8, `rule:concurrency/queue-deletion-is-explicit-and-bounded` + `rule:concurrency/queue-four-members`/0084 changed by a record | `nvs-stdlib`, `nvs-config`, `nvs-cli`, `nvs-db` |
| [serve-runs-the-queue](35-serve-runs-the-queue.md) | M7, `rule:concurrency/one-process-serves-requests-schedules-and-jobs` + `rule:config/reloadability-is-its-own-field`/0078 changed by a record | `nvs-cli`, `nvs-config`, `nvs-server` |
| [editor-install](36-editor-install.md) | post-parity, `rule:ide/the-extension-guides-an-install-and-never-bundles-one`/0155 | `nvs-lsp`, and `editors/vscode` — the only goal whose weight is TypeScript |
| [workspace-index](37-workspace-index.md) | M10, `rule:ide/five-features-are-one-reference-index` | `nvs-lsp` — one index, its five readers, and the requests M4B's closed list left out |
| [editor-surfaces](38-editor-surfaces.md) | M10, `rule:ide/tasks-carry-a-problem-matcher` | `editors/vscode` and the two CLI surfaces it queries — Tasks, the AST panel, a Test Explorer, template regions |
| [resource-ceilings](39-resource-ceilings.md) | M6, `rule:errors/on-limit` | `nvs-runtime`, `nvs-host`'s watchdog and the poll's emit site — a runaway is stopped whether it burns a core, allocates in a loop, or asks for everything at once |
| [config-is-written](40-config-is-written.md) | M6, `rule:config/no-configuration-file-is-a-complete-configuration` + `rule:config/ownership-is-the-trust-boundary` | `nvs-config`, `nvs-cli`, `nvs-server` — an implicit configuration is written down, and every key in it is read |
| [event-streams](41-event-streams.md) | M7, one ADR slot — 0083 § 5 amended, and its streaming-response claim corrected | `nvs-server`, `nvs-runtime`, `nvs-stdlib`, `nvs-types`, `nvs-config` — one body cell, two doors onto it |
| [finish-response](42-finish-response.md) | M7, one ADR — the fourth ending, and why it sits beside `exit` rather than inside it | `nvs-runtime`, `nvs-ir`, `nvs-codegen`, `nvs-host`, `nvs-stdlib` — one unwind, one member, one drain the served path was missing |
| [gap-zero](43-gap-zero.md) | post-parity, one ADR — the streaming read across the five drivers | `tools/`, and every crate the register still names — last of the hand-written goals, because a gap register is emptied after everything that adds to it has run |
| [dossier](44-dossier.md) | `rule:testing/four-proofs` | none — it writes the goals that open all of them, then optimizes the loop for their shape |
| after `dossier` | `rule:testing/four-proofs`, generated | one group of features per goal, its own `[context]` manifest, under `goals/dossier/` |

## The chain contract

**This directory is the chain.** A goal is `N-<slug>.md` plus, until it is retired, a sibling `.toml`
and `.handoff.md`; the numbers run `1..N` with no gaps, and the order the driver walks is that number.
There is no second file saying what the order is, so **reordering the chain is renaming files** — and
`python tools/chain.py` is how that is done, never by hand. It scaffolds the three files below,
renumbers everything a move or an insert displaces, refuses an edit behind the live goal, and
`--check` says whether every goal is one the driver can walk. [commands.md](../commands.md) is the
tool's one home; this section is the contract it enforces.

Because a number is a position, **prose names a goal by its slug** — goal `parses`, not goal 21. The
only text carrying a number is the goal's own two file headers and a link target, which is a filename;
`chain.py` rewrites both when it renames, and `--check` fails on any other one.
[AGENTS.md](../../../AGENTS.md) § *The schedule is the chain* is that rule's home.

Each goal is three files, named for it:

| File | Holds |
|---|---|
| `N-<slug>.md` | front matter naming its milestone, the target, `## Why here`, the item list grouped by file set, the standing decisions |
| `N-<slug>.toml` | the acceptance test as data, and the `[context]` manifest — **deleted when the run leaves the goal**, which is what records the retirement |
| `N-<slug>.handoff.md` | the handoff the switch seeds, naming that goal's first group — deleted with it |

Four rules bind every one of them, and they are the reason the run can be left alone:

1. **A goal's acceptance list is the next goal's floor, mechanically.** `tools/goal-switch.py` copies every
   `[[check]]` out of the live `loop-goal.toml` into the next goal's own marker line, relabelled to the
   floor stage. Nothing is copied by hand. By goal `server` the floor is five goals deep, which is the point —
   the parity claim is only worth something if nothing under it was traded away to reach it.
2. **Every goal names the numbered ADRs it may open, and no session opens another.** The blanket "do not
   open a numbered ADR" rule that M4's goal carried does not survive this program: goals `concurrency` through `server` contain
   genuinely new designs — a reactor, a driver's wire I/O, a pool reset that is a security boundary — and
   a design of that size recorded as a paragraph in `docs/adr/README.md` is a design nobody can find
   later. So each goal's *Standing decisions* carries a short list of **ADR slots**, each one the first
   slice of the goal that needs it. Anything not on that list is still decided-and-recorded, never
   `BLOCKED`, and never a new number.
3. **A goal that cannot verify itself does not advance.** The driver runs the acceptance test; a session
   claiming `DONE` against a red check stops the run, exactly as it does today.
4. **A goal the run has left is retired, in the same commit as the switch.** Rule 1 makes the fold
   cumulative — goal `core-depth`'s 80 checks are in goal `concurrency`'s file and in every file after
   it — so a walked goal's own `.toml` is a duplicate of a duplicate, and by goal `server` the
   directory held 830K of floors no tool reads. `chain.py --retire N` proves every one of that goal's
   checks is in the live `loop-goal.toml`, then deletes its `.toml` and its `.handoff.md`. **That
   deletion is the record**: retirement is the `.toml` being gone, so there is no flag beside it to
   say otherwise. **The `.md` stays**, because it holds the number that fixes the goal's position and
   the prose that [the plan](../../implementation-plan.md) and the milestone files cite. The proof is
   not `--force`-able, and a retired goal at or after the live one is refused by `loop.py` at
   start-up — retiring is the claim that a goal's checks are already somebody's floor, and that claim
   is false anywhere but behind the run.

## Starting the chain

Three steps.

1. Confirm the goal the repository is currently running is green: `python tools/loop.py --goal-only`.
   Whatever that goal is becomes goal `core-depth`'s floor, so this is not a formality — it is the moment the floor
   is decided.
2. `python tools/loop-stats.py` and `python tools/loop-stats.py --attribute`.
   [loop-authoring.md](../loop-authoring.md) § 1 makes this step zero and § 9 says the numbers move. Set
   the slice budget from what it prints and **say which and why in the commit**. The 200k ceiling is not
   a number to re-derive; the *projection* is.
3. `python tools/loop.py`.

**The driver does the switching, including the first one.** It runs `tools/goal-switch.py` against the
entry it is about to install — which folds the live goal's whole acceptance list in as that entry's floor
— copies the three files into `docs/agent/loop-goal.md`/`.toml` and `docs/agent/handoff.md`, retires the
entry it just left (rule 4), and commits all of that as one switch before starting the session. On
`GOAL REACHED` it does the same for the next entry and keeps going. There is no flag for this and never
a second chain: a run that stopped at each green goal to wait for a human would be the same run with five
extra nights in it.

`.loop/chain.json` records which entry is installed, and it is what makes "exactly once per entry" a fact
rather than an intention: **`goal-switch.py` is not idempotent** — it inserts at a marker it leaves in
place, so running it twice inserts the floor twice. If you ever delete that state file, check the entry's
TOML for a doubled floor before restarting.

## What stops the run

- **The last goal goes green** — which, since goal `dossier`, means the last *generated* one: every group on
  `rule:testing/four-proofs`'s roster owing nothing, `python tools/dossier.py --gate` exiting 0 over the whole language.
  The parity program's own gate is still goal `server`'s final check —
  `python tools/check-migration.py` reporting 100% classified — every one of the oracle build's **1167
  functions and 255 types** accounted for, every `member` row registered, every one of them cased. The
  inventory grew from 925 when the oracle build gained `mysqli`, `pgsql`, `sqlite3`, `fileinfo` and
  `zip`: the APIs `rule:core-classes/db-one-api`, `Core\Zip` and `Core\Mime` replace are now inside the
  audit rather than a named hole beside it. Goals `temp-sweep` through `doc-comments` going green, in chain order, is then what
  ends the run.
- **A goal reports `BLOCKED`.** Reserved for a decision that is expensive to reverse *and* has no safe
  default. Every goal's standing decisions exist to make this rare.
- **`--max-stalls` consecutive sessions move `HEAD` nowhere.**
- **Goal `database`'s Docker preflight fails.** `rule:core-classes/db-one-api` verifies the drivers against real servers, so the driver
  checks for a reachable daemon before the first session of that goal and stops the run naming it. A run
  that grinds for six hours against a check that cannot pass is worse than one that stops in the first
  minute.

## What this program does not touch

`Web\Migration` and everything versioned about a schema change — ordering, history tables, fleet
locking, reversibility — which `rule:programs/no-migration-runner` records as
deliberately blocked. Goal `schema` builds convergence, which needs none of them, and does not close that gap.

Doc trimming and dependency sweeps, both of which the user fires and never a session
([doc-cleanup.md](../doc-cleanup.md), [dependency-update.md](../dependency-update.md)). And **PHP's
optional extensions** — `gd`, `intl`, `imap`, `zip` and the rest of the unaudited list in
[02-php-migration.md](../../spec/02-php-migration.md) — are not parity work: they are M9's, and a session
that finds one on its path puts it in the handoff's `## Backlog` and moves on.
