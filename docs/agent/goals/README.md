# The goal chain

This directory holds the prose of the goals the unattended loop walks, `data/chain.json` is their order,
and the table below is that order for the hand-written goals. Everything between `dossier` and
`plain-comments` is one goal per group of features owing `rule:testing/feature-proofs`'s roster. Those
goals are ordinary goals, edited by hand like any other; a goal whose record says `position: last` stays behind all of them, and
`bun nv chain --check` refuses an unpinned goal behind a pinned one.

**[goal-plan.md](../goal-plan.md) is the whole chain in one file**: every goal in order with its stages,
which goal is live, and the side goals. It is rendered from the records, and `bun nv chain`, the goal
switch and every wrap write it again when they change them.

**Every goal states its own case, and this file does not restate it.** A goal's front matter names its
milestone, its opening says what it builds, and its `## Why here` is why it sits where it does, against the
goals on either side. A paragraph here describing a goal would be a second copy of a fact that already has
a home, so there is none. [loop-authoring.md](../loop-authoring.md) owns how a goal is *written* and
[coordinator.md](../coordinator.md) owns how one is *driven*. What is left for this file is the contract
that binds the chain as a whole: why the work is cut into goals at all, what the three files are, the four
rules over them, and what stops the run.

## Why the work is cut into goals

A goal is a finite contained group of work, and its record's `context` is what keeps a session under the
200k ceiling. One goal spanning `nvs-syntax` through a TDS driver would need a manifest naming every
module in the workspace, and every byte of it is charged to every session — including the ones that never
open a driver. One manifest per goal, each naming the six-to-fifteen modules that goal touches, is the
same work at a fraction of the per-session cost.

The split is **by file set, not by topic**. That is why M8 is two goals — `nvs-db` shares nothing with
`Core\Cli` — and why M5's reactor and its isolates are one, since both are `nvs-host`.

| Goal | Milestone | Crates it opens |
|---|---|---|
| [core-depth](core-depth.md) | M4S tail | `nvs-stdlib`, `nvs-types`, `nvs-hir`, `nvs-cli` |
| [concurrency](concurrency.md) | M5 | **`nvs-host`** (new), `nvs-runtime`, `nvs-stdlib` |
| [governance](governance.md) | M6 | **`nvs-config`** (new), `nvs-host`, `nvs-cli`, `nvs-codegen` |
| [core-part-ii](core-part-ii.md) | M8, non-database | `nvs-stdlib`, `nvs-host` |
| [database](database.md) | M8, database | **`nvs-db`** (new), `nvs-stdlib`, `nvs-types` |
| [server](server.md) | M7 | **`nvs-server`** (new), `nvs-stdlib`, `nvs-host` |
| [temp-sweep](temp-sweep.md) | post-parity, `rule:core-classes/temporary-dir-sweep` | `nvs-runtime`, `nvs-host`, `nvs-stdlib`, `nvs-config`, `nvs-server`, `nvs-cli` |
| [program-id](program-id.md) | post-parity, `rule:programs/no-runtime-autoload` changed by a record | `nvs-config`, `nvs-hir`, `nvs-runtime`, `nvs-stdlib` |
| [schema](schema.md) | post-parity, one ADR slot | `nvs-db`, `nvs-stdlib`, `nvs-cli` |
| [typed-callable](typed-callable.md) | post-parity, `rule:types/callable-signature` | `nvs-syntax`, `nvs-types`, `nvs-stdlib`, `nvs-ir`, `nvs-codegen`, `nvs-runtime` |
| [doc-comments](doc-comments.md) | post-parity, `rule:tooling/doc-comment-is-three-slashes` + M4B's tree half | `nvs-syntax`, `nvs-diagnostics`, `nvs-hir`, `nvs-cli` |
| [resilient-tree](resilient-tree.md) | M4B, `rule:ide/one-grammar-one-tree`'s other half | `nvs-syntax`, `nvs-diagnostics`, `nvs-test`, `nvs-cli` |
| [surface](surface.md) | M1 items 5-6, `rule:expressions/pipeline-substitution` + `rule:php-migration/a-deprecation-is-a-refusal` | `nvs-syntax`, `nvs-diagnostics` |
| [lsp-server](lsp-server.md) | M4B, `rule:ide/the-request-set-is-closed`+5 + `rule:ide/redaction-ranges-come-from-the-server` | **`nvs-lsp`** (new), `nvs-cli`, `nvs-types`, `nvs-stdlib` |
| [editor](editor.md) | M4B, `rule:ide/highlighting-is-two-layers`+6 | **`editors/vscode`** (new, TypeScript) |
| [request-json](request-json.md) | M7, `rule:http-server/a-session-store-answers-four-operations` + spec § 15 | `nvs-stdlib`, `nvs-runtime`, `nvs-test`, `nvs-cli` |
| [test-request](test-request.md) | M8, `rule:testing/in-process-request` | `nvs-runtime`, `nvs-stdlib`, `nvs-server`, `nvs-test`, `nvs-cli` |
| [input-shapes](input-shapes.md) | M7, one new record + `rule:types/object-top`/0024 changed by a record | `nvs-syntax`, `nvs-types`, `nvs-stdlib` |
| [parses](parses.md) | M7, one new record + `rule:classes/comparable`/0066/0077/0102 changed by a record | `nvs-hir`, `nvs-types`, `nvs-stdlib`, `nvs-runtime`, `nvs-cli` |
| [unix-sockets](unix-sockets.md) | M8, `rule:config/cache-shared-is-the-grant-over-the-configured-store` + `rule:http-server/allow-url-pins-the-address`/0059 changed by a record | `nvs-config`, `nvs-stdlib`, `nvs-db`, `nvs-host`, `nvs-diagnostics` |
| [carried-gaps](carried-gaps.md) | post-parity, `rule:core-classes/db-one-api`/0073/0076/0116/0133 changed by a record | `nvs-config`, `nvs-cli`, `nvs-types`, `nvs-runtime`, `nvs-stdlib`, `nvs-server`, `nvs-db`, `nvs-diagnostics` |
| [warm-start](warm-start.md) | post-parity, `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` | `nvs-codegen`, `nvs-cli`, `nvs-config` |
| [per-core](per-core.md) | M7, one ADR slot + `rule:http-server/two-deployments-and-nothing-a-proxy-owns`/0017 changed by a record | `nvs-cli`, `nvs-host`, `nvs-server`, `nvs-config` |
| [net-os-signal](net-os-signal.md) | M8, one ADR slot + `rule:config/net-local-is-named-and-not-on-the-roster`'s deferred grant | `nvs-stdlib`, `nvs-host`, `nvs-config`, `nvs-runtime` |
| [formats](formats.md) | M8, one ADR slot (the shared decompression bound) | `nvs-stdlib`, `nvs-config`, `nvs-diagnostics` |
| [encoder-cycles](encoder-cycles.md) | M8, no ADR slot — [0164](../../decisions/0164.md) is already accepted | `nvs-stdlib` |
| [record-origin](record-origin.md) | M8, no ADR slot — [0165](../../decisions/0165.md) is already accepted | `nvs-render`, `nvs-runtime`, `nvs-stdlib`, `nvs-ir`, `nvs-codegen` |
| [agent-surface](agent-surface.md) | M10, no ADR slot — [0167](../../decisions/0167.md) is already accepted | `nvs-cli`, `nvs-hir`, `nvs-runtime`, and `tools/nv/cmd/reference.ts` — the surface a coding agent reads the language through |
| [xml-tree](xml-tree.md) | M8, one ADR slot + `rule:core-classes/html-parsing`'s change | `nvs-stdlib`, `nvs-diagnostics` |
| [gap-owners](gap-owners.md) | post-parity, no ADR — a process gate | `tools/`, every crate's module docs |
| [unowned-sweep](unowned-sweep.md) | post-parity, `rule:errors/propagation`/0033/0044 changed by a record | `nvs-stdlib`, `nvs-types`, `nvs-runtime` |
| [signed-urls](signed-urls.md) | M8, `rule:core-api/signing-is-over-a-payload` + `rule:security/protocol-roster`/0077 changed by a record | `nvs-stdlib`, `nvs-types`, `nvs-runtime` |
| [type-test](type-test.md) | M1, `rule:types/type-test` — the other half of goal `surface`'s reservation | `nvs-syntax`, `nvs-types`, `nvs-ir`, `nvs-codegen` |
| [queue-purge](queue-purge.md) | M8, `rule:concurrency/queue-deletion-is-explicit-and-bounded` + `rule:concurrency/queue-four-members`/0084 changed by a record | `nvs-stdlib`, `nvs-config`, `nvs-cli`, `nvs-db` |
| [sqlite-queue](sqlite-queue.md) | M8, one ADR slot — `rule:concurrency/claiming-is-one-statement`'s SQLite mechanism, which is named there and does not exist yet | `nvs-db`, `nvs-stdlib`, `nvs-cli` — the queue's third dialect, and the first whose own half needs no container |
| [serve-runs-the-queue](serve-runs-the-queue.md) | M7, `rule:concurrency/one-process-serves-requests-schedules-and-jobs` + `rule:config/reloadability-is-its-own-field`/0078 changed by a record | `nvs-cli`, `nvs-config`, `nvs-server` |
| [editor-install](editor-install.md) | post-parity, `rule:ide/the-extension-guides-an-install-and-never-bundles-one`/0155 | `nvs-lsp`, and `editors/vscode` — the only goal whose weight is TypeScript |
| [workspace-index](workspace-index.md) | M10, `rule:ide/five-features-are-one-reference-index` | `nvs-lsp` — one index, its five readers, and the requests M4B's closed list left out |
| [editor-surfaces](editor-surfaces.md) | M10, `rule:ide/tasks-carry-a-problem-matcher` | `editors/vscode` and the two CLI surfaces it queries — Tasks, the AST panel, a Test Explorer, template regions |
| [resource-ceilings](resource-ceilings.md) | M6, `rule:errors/on-limit` | `nvs-runtime`, `nvs-host`'s watchdog and the poll's emit site — a runaway is stopped whether it burns a core, allocates in a loop, or asks for everything at once |
| [config-is-written](config-is-written.md) | M6, `rule:config/no-configuration-file-is-a-complete-configuration` + `rule:config/ownership-is-the-trust-boundary` | `nvs-config`, `nvs-cli`, `nvs-server` — an implicit configuration is written down, and every key in it is read |
| [event-streams](event-streams.md) | M7, one ADR slot — 0083 § 5 amended, and its streaming-response claim corrected | `nvs-server`, `nvs-runtime`, `nvs-stdlib`, `nvs-types`, `nvs-config` — one body cell, two doors onto it |
| [finish-response](finish-response.md) | M7, one ADR — the fourth ending, and why it sits beside `exit` rather than inside it | `nvs-runtime`, `nvs-ir`, `nvs-codegen`, `nvs-host`, `nvs-stdlib` — one unwind, one member, one drain the served path was missing |
| [markup-literal](markup-literal.md) | M8, ADR 0169 landed with the goal — the literal, its hole grammar, and why no `Cli\Text` peer | `nvs-syntax`, `nvs-types`, `nvs-ir`, `nvs-codegen`, `nvs-stdlib` — one lexer mode reused, no HTML in the compiler |
| [fmt](fmt.md) | M10, no ADR slot — [0039](../../decisions/0039.md) and [0173](../../decisions/0173.md) are already accepted | **`nvs-fmt`** (new), `nvs-syntax`, `nvs-cli` — the one formatter, printed off the lossless tree |
| [template-format](template-format.md) | M10, no ADR slot — [0173](../../decisions/0173.md) is already accepted | `nvs-lsp`, `editors/vscode` — format-on-save formats the markup too, each chunk from the `?>` that opened it |
| [webcrypto](webcrypto.md) | M8, one ADR slot — the interop tier beside XChaCha20-Poly1305, and `Core\Jwe` as the roster's sixth protocol | `nvs-stdlib` and the workspace manifest — AES-256-GCM, PBKDF2, HKDF and ECDH that a browser's WebCrypto reads, nothing shipped removed |
| [http-client](http-client.md) | M8, one ADR slot — bodies, the dynamic verb, streamed replies, the pinned pool and the redirect credential rule | `nvs-stdlib`, `nvs-types`, `nvs-config` — the client an API is driven through, and a test that answers it from a table |
| [process-cache](process-cache.md) | M8, one ADR slot — the process tier, a lifetime on every tier, and a secret cached only sealed | `nvs-stdlib`, `nvs-config`, `nvs-cli` — the first state the cores share, and where a token lives between requests |
| [outbound-proxy](outbound-proxy.md) | M8, one ADR slot — a forward proxy the operator configures, and what the address policy keeps and loses through it | `nvs-stdlib`'s transport, `nvs-config` — a CONNECT tunnel that keeps the pin by default |
| [websocket-client](websocket-client.md) | M8, one ADR slot — an outbound WebSocket opened through the client's own door, bounded, and closed with the task that opened it | `nvs-stdlib`'s transport and `Core\Socket\Message`, `nvs-config` — `tungstenite`'s client half over the parking stream, no new dependency |
| [plan-truth](plan-truth.md) | post-parity, no ADR — the gap program's catch-up | `docs/plan/`, the plan index, module docs, `tools/nv/cmd/plan.ts` — every document says what the tree does before anything is derived from it |
| [gap-register](gap-register.md) | post-parity, no ADR — a process gate | `tools/nv/cmd/owners.ts`, `tools/nv/cmd/plan.ts`, the ratchet test — one register over every place a gap is written, and a future milestone is the only deferral |
| [m4-refusals](m4-refusals.md) | M4, no ADR slot | `nvs-ir`, `nvs-types` — every shape the checker admits lowers, or a diagnostic naming its rule refuses it |
| [m5-proofs](m5-proofs.md) | M5 | `nvs-host`, `nvs-runtime`, `benches/`, CI — the scheduler's claims proven at the scale M5 promised them |
| [m4b-editor](m4b-editor.md) | M4B, one ADR slot — the host tier runs locally and in CI | `editors/vscode`, CI — the extension tested in a real editor host and packaged by CI |
| [m7-server-surface](m7-server-surface.md) | M7, with M6's control-socket promises | `nvs-cli`, `nvs-server`, `nvs-stdlib`'s response and test classes — everything M7 promised a deployment is there to run |
| [m8-db-queue](m8-db-queue.md) | M8, database; one ADR slot — the streaming read across the five drivers | `nvs-db`, `nvs-stdlib`'s `Core\Db` and `Core\Queue`, CI — every member answers on all five drivers |
| [m8-stdlib-depth](m8-stdlib-depth.md) | M8, non-database | `nvs-stdlib`, `nvs-render`, `benches/` — every class M8 names as deep as its spec section |
| [unowned-closures](unowned-closures.md) | post-parity, at most one ADR — the prepared-pattern channel, if the user's answers build it | every crate with an `unowned` gap — each built to the answer the user's decision sheet gave |
| [class-scoped-types](class-scoped-types.md) | post-parity, one new record — `rule:types/type-alias` gains a second declaration site | `nvs-syntax`, `nvs-hir`, `nvs-types`, `nvs-diagnostics`, `nvs-fmt`, `nvs-cli`, `nvs-lsp` — a `type` alias as a member of a class, an interface or an enum |
| [worker-placement](worker-placement.md) | post-parity, no ADR — ADR 0184 §§ 2, 5 as written | `nvs-host`, `nvs-runtime`'s script seam, `nvs-cli`'s unit table — a path entry is placed like a method entry, and a serving core offers itself as a destination |
| [core-class-tests](core-class-tests.md) | post-parity, at most one ADR — only if the checker needs a `Core` class's identity in a shape the registry does not already answer | `nvs-types`, `nvs-ir`, `nvs-stdlib`'s descriptor table — `instanceof` and `as` name a `Core` class, so a `mixed` narrows to the one it holds |
| [tds-bytes](tds-bytes.md) | post-parity, no ADR — ADR 0067 §§ 1 and 9 state both halves | `nvs-db`'s TDS encoder — a bound value carries its form, a binary marker is declared `varbinary`, and the plan cache tells the two declarations apart on the comparison it already makes |
| [cache-shared-dial](cache-shared-dial.md) | post-parity, no ADR — four rule fragments state every half | `nvs-config`'s secret registry, `nvs-stdlib`'s Redis client, `nvs-host`'s TLS client — `[cache.shared]` gains a credential pair and a database index, and `rediss://` is a third transport arm |
| [decided-closures](decided-closures.md) | post-parity, at most one ADR — the prepared-pattern channel, built once for `cldr.rs` and `time.rs` | every crate with a `Decided:` gap — the 40 goal `unowned-closures` tagged and did not build, and the 8 owed to M1, M6, M7 and M8, each built to its sentence, struck as a bound, or deferred honestly; the gate is `bun nv owners --closes`, which a tag cannot meet |
| [one-type-test](one-type-test.md) | post-parity, one ADR — the record that ends every comparison with PHP on `is` and `instanceof`, written first | `nvs-syntax`, `nvs-types`, `nvs-diagnostics`, `nvs-ir`, `nvs-codegen`, `nvs-runtime`, `nvs-stdlib`, `nvs-lsp`, `nvs-fmt` — `instanceof` is refused naming `is`, `$x is $cls` is the dynamic class test, and the word survives only in the refusal and the divergence |
| [test-doubles](test-doubles.md) | post-parity, no ADR — ADR 0079 §§ 10, 11 and 16 decided it | `nvs-types`, `nvs-diagnostics`, `nvs-stdlib`, `nvs-runtime`, `nvs-ir` — `Core\Test::double<T>` and `partial<T>` are a shape of closures checked against an interface and *are* a `T`; `assertCalled`/`assertNeverCalled` read the record against a compile-checked method reference; `assertCompletes` runs under the test's clock |
| [bigint](bigint.md) | post-parity, no ADR — ADR 0054 § 5 decided it | `nvs-stdlib` — `Core\BigInt` over `num-bigint`, a `Core`-owned immutable instance, `Stringable` and `Comparable`, the class that replaces `gmp` and `bcpowmod` |
| [gap-zero](gap-zero.md) | post-parity, no ADR — the terminal gate | `tools/`, the ratchet test — no gap owed by anyone but a future milestone, M0–M8 complete, the index deleted |
| [dossier](dossier.md) | `rule:testing/feature-proofs` | none — it writes the goals that open all of them, then optimizes the loop for their shape |
| [tooling-overhaul](tooling-overhaul.md) | post-parity, two new records — `main` frozen until it is walked | `tools/` becomes `bun nv`, over typed records under `data/`; every check keyed on what it reads, the playbook triaged, the generated goals made ordinary, no Python left |
| the generated goals, `core-…`, `lang-…`, `types-…`, `tools-…` | `rule:testing/feature-proofs` | one group of features per goal, each an ordinary goal with its own `context` manifest, its prose under `docs/agent/goals/` and its record under `data/goals/` |
| [limit-handler-reach](limit-handler-reach.md) | `rule:errors/on-limit` | `crates/nvs-runtime/src/abi.rs`, `ctx/hooks.rs`, `sequence.rs` — a resource `FATAL` raised inside a member's own loop runs the program's `onLimit` handler, as one raised in compiled code already does |
| [core-class-cards](core-class-cards.md) | `rule:core-api/reference-card` | `crates/nvs-stdlib/src/registry.rs`, `tools/nv/cmd/class-cards.ts` — every `Core` class that landed before classes carried a card gains its `ClassDoc`, and the registry test's list of classes still owing one is emptied |
| [plain-comments](plain-comments.md) | `rule:testing/feature-proofs` — `position: last` | `docs/examples/`, `tests/hostile/`, `benches/members/` — behind every generated goal: each landed program's comments rewritten inside the plain-comment bounds, then `owes.all` in `data/proofs/policy.json` makes them owed |
| [foreach-var](foreach-var.md) | `rule:types/var-inference`, one new record — `position: last` | `crates/nvs-syntax/src/parser/stmt.rs`, `nvs-types/src/locals.rs`, `nvs-ir/src/lower/mod.rs`, `docs/reference/lang/` — a `foreach` binding may write `var` wherever a local may, and the reference, the rules and the examples already on disk say so |
| [var-array-literal](var-array-literal.md) | `rule:types/var-inference`, one new record — `position: last` | `crates/nvs-types/src/expr/literals.rs`, `nvs-types/src/locals.rs`, `nvs-lsp/src/hints.rs`, `docs/reference/lang/` — `var $ids = [1, 2, 3];` is an `array<int>`, and a literal whose elements differ is still refused with the type to write |
| [program-enumeration](program-enumeration.md) | `rule:programs/implementing`, one new record — `position: last` | `crates/nvs-types/src/program.rs`, `nvs-hir/src/requires.rs`, `nvs-stdlib/src/program.rs` — `implementing<T>` takes a base class, and `constructors<T, callable(...): T>()` returns typed constructors, so an enumerated class may take constructor arguments |
| [coalesce-assign](coalesce-assign.md) | `rule:php-migration/absent-storage-is-never-a-zero-value`, one new record — `position: last` | `crates/nvs-types/src/expr/assign.rs`, `nvs-ir/src/lower/stmt.rs`, `nvs-syntax/src/lexer.rs` — `??=` writes an absent key and is typed as the value it wrote, and `??+=`, `??-=` and `??.=` update a target that starts from the zero of its type when it is `null` or absent |
| [ci-green](ci-green.md) | post-parity, no ADR — `position: last` | `tools/nv/cmd/ci-green.ts`, `.github/workflows/ci.yml` — the last goal of the program, behind every generated one: the latest CI run on `main` succeeded for the code `HEAD` holds |
| [goal-closeout](goal-closeout.md) | post-parity, one new record — a finished goal is deleted from this goal on — `position: last` | `tools/nv/cmd/owners.ts`, `tools/nv/cmd/loop.ts`, `tools/nv/lib/chain.ts`, `docs/plan/` — every old goal proven finished, its unhomed decisions kept in `docs/agent/goal-decisions.md`, the switch made to delete a walked goal, and every goal in front of it deleted |
| [performance-pass](performance-pass.md) | post-parity, a record only if a fix changes a rule — `position: last` | `bun nv scaling` (new), `benches/scaling/` (new), every crate a finding names — one pass over everything: no work that grows faster than linear, the big problems fixed at the root, and a plain summary in `docs/perf/performance-pass.md` |

## The chain contract

**`data/chain.json` is the chain**: `goals`, a list of goal slugs whose order is the order the driver
walks, and `live`, the goal it works on. `live` is tracked in git, so every clone, CI and the pre-push
hook see the same live goal, and every goal in front of it is walked. There is no second file saying
what the order is, so **reordering the chain is editing that list** — and `bun nv chain` is how that is
done, never by hand. `--new`, `--move` and `--remove` each edit `goals` in that one file and never
`live`, which only the driver's goal switch moves; `--check` says whether every goal is one the driver
can walk. `tools/nv/cmd/chain.ts`'s module doc is the tool's one home; this section is the contract it
enforces.

A goal the run has **not reached** may be inserted, edited or appended while the loop is running: every
turn of the driver is a fresh process that reads the chain again. The live goal and every goal the run
has walked are not rewritten, because their checks are the floor of every goal behind them.

**Prose names a goal by its slug** — goal `parses`, not goal 21. A number is a position, and it moves the
moment anything is inserted in front of it. `bun nv chain --check` fails on prose that names a goal by
its number. [AGENTS.md](../../../AGENTS.md) § *The schedule is the chain* is that rule's home.

Each goal is three files, named for it:

| File | Holds |
|---|---|
| `docs/agent/goals/<slug>.md` | front matter naming its milestone, the target, `## Why here`, the item list grouped by file set, the standing decisions |
| `data/goals/<slug>.json` | the acceptance test as data, and the `context` a session reads — **a walked goal keeps its checks**, because they are the floor of every goal behind it |
| `data/goals/<slug>.handoff.json` | the handoff, written with the goal to name its first group and rewritten by every session's wrap — deleted when the goal is retired |

Four rules bind every one of them, and they are the reason the run can be left alone:

1. **A goal's acceptance list is the floor of every goal behind it, mechanically.** A goal's plan is its
   record with every check of every walked goal carried in, each once, under a stage titled `floor`
   (`goalPlan` in `tools/nv/lib/chain.ts`), which is how `tools/nv/driver/accept.ts` tells a carried
   check from the goal's own. The floor is a view: nothing is copied, by hand or by the switch. The
   floor is every goal deep, which is the point — the parity claim is only worth something if nothing
   under it was traded away to reach it.
2. **Every goal names the numbered ADRs it may open, and no session opens another.** The blanket "do not
   open a numbered ADR" rule that M4's goal carried does not survive this program: goals `concurrency` through `server` contain
   genuinely new designs — a reactor, a driver's wire I/O, a pool reset that is a security boundary — and
   a design of that size recorded as a paragraph in `docs/adr/README.md` is a design nobody can find
   later. So each goal's *Standing decisions* carries a short list of **ADR slots**, each one the first
   slice of the goal that needs it. Anything not on that list is still decided-and-recorded, never
   `BLOCKED`, and never a new number.
3. **A goal that cannot verify itself does not advance.** The driver runs the acceptance test; a session
   claiming `DONE` against a red check gets a retry session handed that check, and a retry that fails
   on the same check holds the run as `done-claim`.
4. **A goal switch copies and retires nothing.** A walked goal keeps its checks, because rule 1 reads
   them from its record. A goal whose record has no checks is retired: it counts as walked wherever it
   sits, and its checks are no floor, so a goal is emptied only when its checks are already carried by
   a goal the run has walked. **The empty list is the record**, so there is no flag beside it to say
   otherwise. **The prose and the rest of the record stay**, because they hold the prose that
   [the plan](../../implementation-plan.md) and the milestone files cite. `bun nv chain --check` refuses
   a retired goal at the live one or later in the chain — retiring is the claim that a goal's checks are
   already somebody's floor, and that claim is false anywhere but among the goals the run has walked.

## Side goals

**A side goal is a goal the chain never walks.** It is `docs/agent/goals/side/<slug>.md` and
`data/goals/side/<slug>.json` plus its handoff record beside that: the same three files, the same shapes
and the same rules as a chain goal, with three differences.

- **No place in the chain.** Nothing walks to it, so `data/chain.json` does not name it: its `.md`
  opens `# Side goal — <title>`, and prose names it by its slug as it names any goal. `bun nv chain
  --check` refuses a side goal with checks that has no prose, no such H1 or no handoff record.
- **A person starts its run by hand**, with `bun nv loop --side <slug>` typed in the main tree. It
  makes branch `side/<slug>` and its worktree at `.agent-tmp/worktrees/side/<slug>` when they are
  missing, and runs sessions there on that goal and nothing else. The driver never picks a side goal on
  its own. A side run and the chain run may run at once, because they share no tree, no build and no
  state file.
- **It lands instead of switching.** When its list is green the run ends on `SIDE GOAL GREEN` and
  prints the landing steps. The person rebases the branch onto `main`, runs `nv verify` and the whole
  list again, and fast-forwards `main` while the chain run holds between two sessions. **The goal is
  deleted as it lands**: one commit removes its prose, its record and its handoff, and the worktree and
  branch are removed after it. A side goal is never retired, because no plan section or milestone
  cites it and `git log` already keeps what it was. `bun nv chain --check` refuses a side goal with no
  checks, which is one that landed and was kept.

Its list is checked over **main's carried floor** as well as its own checks, so a side branch cannot
land anything that breaks a walked goal. The live chain goal's own checks are not part of it. A side
session writes no plan section and never edits the chain run's files.

## Starting the chain

Three steps.

1. Confirm the goal the repository is currently running is green: `bun nv loop --goal-only`.
   Whatever that goal is becomes the next goal's floor, so this is not a formality — it is the moment the
   floor is decided.
2. `bun nv loop-stats` and `bun nv loop-stats --attribute`.
   [loop-authoring.md](../loop-authoring.md) § 1 makes this step zero and § 9 says the numbers move. Set
   the slice budget from what it prints and **say which and why in the commit**. The 200k ceiling is not
   a number to re-derive; the *projection* is.
3. `bun nv loop`, typed by hand: it is the launcher [coordinator.md](../coordinator.md) § *Files*
   describes.

**The driver does the switching.** When a sweep with the floor gate open is green and both goal-end
gates are green — the rustdoc gate `bun nv verify --doc`, and the owner gate `bun nv owners --closes
<slug>` with `bun nv playbook --closes <slug>` — it moves `live` in `data/chain.json` to the next
goal, commits that one file as ``docs(loop): the chain advances from `a` to `b` ``, preflights and
brings up the new goal's `env.docker` services, and carries on with the next session. After the last
goal it ends the run on `CHAIN COMPLETE`. There is no flag for this and never a second chain: a run
that stopped at each green goal to wait for a human would be the same run with five extra nights in it.
A red gate holds the goal open, and `bun nv orient` prints the finding from `.loop/doc-gate.json` or
`.loop/owner-gate.json`.

## What stops the run

- **The last goal goes green** — goal `performance-pass`. In front of it, goal `ci-green` closes the
  program the chain was written for: it sits behind the last *generated* one, which is every group on
  `rule:testing/feature-proofs`'s roster owing nothing, `bun nv proofs --gate` exiting 0 over the whole language.
  The parity program's own gate — goals `core-depth` through `server`, PHP core feature parity — is
  still that goal's final check:
  `bun nv migration` reporting 100% classified — every one of the oracle build's **1167
  functions and 255 types** accounted for, every `member` row registered, every one of them cased. The
  inventory grew from 925 when the oracle build gained `mysqli`, `pgsql`, `sqlite3`, `fileinfo` and
  `zip`: the APIs `rule:core-classes/db-one-api`, `Core\Zip` and `Core\Mime` replace are now inside the
  audit rather than a named hole beside it.
- **A goal reports `BLOCKED`.** Reserved for a decision that is expensive to reverse *and* has no safe
  default. Every goal's standing decisions exist to make this rare.
- **`--max-stalls` consecutive sessions move `HEAD` nowhere.** The run gets a repair session first,
  and holds when that is spent.
- **A goal's Docker preflight fails.** `rule:core-classes/db-one-api` verifies the drivers against real
  servers, so the driver checks for a reachable daemon when a run starts on a goal with `env.docker`
  and at the switch to one, and stops naming it (`chain-error`, which gets a repair session and then
  holds). A run that grinds for six hours against a check that cannot pass is worse than one that stops
  in the first minute.

## What no goal on this chain takes

`Web\Migration` and everything versioned about a schema change — ordering, history tables, fleet
locking, reversibility — which `rule:programs/no-migration-runner` records as
deliberately blocked. Goal `schema` builds convergence, which needs none of them, and does not close that gap.

Doc trimming, dependency sweeps and user reports, all of which the user fires and never a session
([doc-cleanup.md](../doc-cleanup.md), [dependency-update.md](../dependency-update.md),
[user-report.md](../user-report.md)); a report's outcome may be a goal, which is then written like any
other. And **PHP's
optional extensions** — `gd`, `intl`, `imap`, `zip` and the rest of the unaudited list in
[02-php-migration.md](../../spec/02-php-migration.md) — are not parity work: they are M9's, and a session
that finds one on its path puts it in the handoff's `## Backlog` and moves on.
