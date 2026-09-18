# The goal chain

This directory **is** the schedule the unattended loop walks, and the table below is that order for the
hand-written goals. Everything between `dossier` and `ci-green` is generated — one goal per group of
features owing `rule:testing/four-proofs`'s roster, written onto this same chain by
`python tools/dossier.py --emit-goals`, which keeps a goal whose front matter says `position: last`
behind what it appends.

**Every goal states its own case, and this file does not restate it.** A goal's front matter names its
milestone, its opening says what it builds, and its `## Why here` is why it sits where it does, against the
goals on either side. A paragraph here describing a goal would be a second copy of a fact that already has
a home, so there is none. [loop-authoring.md](../loop-authoring.md) owns how a goal is *written* and
[coordinator.md](../coordinator.md) owns how one is *driven*. What is left for this file is the contract
that binds the chain as a whole: why the work is cut into goals at all, what the three files are, the four
rules over them, and what stops the run.

## Why the work is cut into goals

A goal is a finite contained group of work, and the `[context]` manifest is what keeps a session under the
200k ceiling. One goal spanning `nvs-syntax` through a TDS driver would need a manifest naming every
module in the workspace, and every byte of it is charged to every session — including the ones that never
open a driver. One manifest per goal, each naming the six-to-fifteen modules that goal touches, is the
same work at a fraction of the per-session cost.

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
| [sqlite-queue](35-sqlite-queue.md) | M8, one ADR slot — `rule:concurrency/claiming-is-one-statement`'s SQLite mechanism, which is named there and does not exist yet | `nvs-db`, `nvs-stdlib`, `nvs-cli` — the queue's third dialect, and the first whose own half needs no container |
| [serve-runs-the-queue](36-serve-runs-the-queue.md) | M7, `rule:concurrency/one-process-serves-requests-schedules-and-jobs` + `rule:config/reloadability-is-its-own-field`/0078 changed by a record | `nvs-cli`, `nvs-config`, `nvs-server` |
| [editor-install](37-editor-install.md) | post-parity, `rule:ide/the-extension-guides-an-install-and-never-bundles-one`/0155 | `nvs-lsp`, and `editors/vscode` — the only goal whose weight is TypeScript |
| [workspace-index](38-workspace-index.md) | M10, `rule:ide/five-features-are-one-reference-index` | `nvs-lsp` — one index, its five readers, and the requests M4B's closed list left out |
| [editor-surfaces](39-editor-surfaces.md) | M10, `rule:ide/tasks-carry-a-problem-matcher` | `editors/vscode` and the two CLI surfaces it queries — Tasks, the AST panel, a Test Explorer, template regions |
| [resource-ceilings](40-resource-ceilings.md) | M6, `rule:errors/on-limit` | `nvs-runtime`, `nvs-host`'s watchdog and the poll's emit site — a runaway is stopped whether it burns a core, allocates in a loop, or asks for everything at once |
| [config-is-written](41-config-is-written.md) | M6, `rule:config/no-configuration-file-is-a-complete-configuration` + `rule:config/ownership-is-the-trust-boundary` | `nvs-config`, `nvs-cli`, `nvs-server` — an implicit configuration is written down, and every key in it is read |
| [event-streams](42-event-streams.md) | M7, one ADR slot — 0083 § 5 amended, and its streaming-response claim corrected | `nvs-server`, `nvs-runtime`, `nvs-stdlib`, `nvs-types`, `nvs-config` — one body cell, two doors onto it |
| [finish-response](43-finish-response.md) | M7, one ADR — the fourth ending, and why it sits beside `exit` rather than inside it | `nvs-runtime`, `nvs-ir`, `nvs-codegen`, `nvs-host`, `nvs-stdlib` — one unwind, one member, one drain the served path was missing |
| [markup-literal](44-markup-literal.md) | M8, ADR 0169 landed with the goal — the literal, its hole grammar, and why no `Cli\Text` peer | `nvs-syntax`, `nvs-types`, `nvs-ir`, `nvs-codegen`, `nvs-stdlib` — one lexer mode reused, no HTML in the compiler |
| [fmt](45-fmt.md) | M10, no ADR slot — [0039](../../decisions/0039.md) and [0173](../../decisions/0173.md) are already accepted | **`nvs-fmt`** (new), `nvs-syntax`, `nvs-cli` — the one formatter, printed off the lossless tree |
| [template-format](46-template-format.md) | M10, no ADR slot — [0173](../../decisions/0173.md) is already accepted | `nvs-lsp`, `editors/vscode` — format-on-save formats the markup too, each chunk from the `?>` that opened it |
| [webcrypto](47-webcrypto.md) | M8, one ADR slot — the interop tier beside XChaCha20-Poly1305, and `Core\Jwe` as the roster's sixth protocol | `nvs-stdlib` and the workspace manifest — AES-256-GCM, PBKDF2, HKDF and ECDH that a browser's WebCrypto reads, nothing shipped removed |
| [http-client](48-http-client.md) | M8, one ADR slot — bodies, the dynamic verb, streamed replies, the pinned pool and the redirect credential rule | `nvs-stdlib`, `nvs-types`, `nvs-config` — the client an API is driven through, and a test that answers it from a table |
| [process-cache](49-process-cache.md) | M8, one ADR slot — the process tier, a lifetime on every tier, and a secret cached only sealed | `nvs-stdlib`, `nvs-config`, `nvs-cli` — the first state the cores share, and where a token lives between requests |
| [outbound-proxy](50-outbound-proxy.md) | M8, one ADR slot — a forward proxy the operator configures, and what the address policy keeps and loses through it | `nvs-stdlib`'s transport, `nvs-config` — a CONNECT tunnel that keeps the pin by default |
| [websocket-client](51-websocket-client.md) | M8, one ADR slot — an outbound WebSocket opened through the client's own door, bounded, and closed with the task that opened it | `nvs-stdlib`'s transport and `Core\Socket\Message`, `nvs-config` — `tungstenite`'s client half over the parking stream, no new dependency |
| [plan-truth](52-plan-truth.md) | post-parity, no ADR — the gap program's catch-up | `docs/plan/`, the plan index, module docs, `tools/plan.py` — every document says what the tree does before anything is derived from it |
| [gap-register](53-gap-register.md) | post-parity, no ADR — a process gate | `tools/owners.py`, `tools/plan.py`, the ratchet test — one register over every place a gap is written, and a future milestone is the only deferral |
| [m4-refusals](54-m4-refusals.md) | M4, no ADR slot | `nvs-ir`, `nvs-types` — every shape the checker admits lowers, or a diagnostic naming its rule refuses it |
| [m5-proofs](55-m5-proofs.md) | M5 | `nvs-host`, `nvs-runtime`, `benches/`, CI — the scheduler's claims proven at the scale M5 promised them |
| [m4b-editor](56-m4b-editor.md) | M4B, one ADR slot — the host tier runs locally and in CI | `editors/vscode`, CI — the extension tested in a real editor host and packaged by CI |
| [m7-server-surface](57-m7-server-surface.md) | M7, with M6's control-socket promises | `nvs-cli`, `nvs-server`, `nvs-stdlib`'s response and test classes — everything M7 promised a deployment is there to run |
| [m8-db-queue](58-m8-db-queue.md) | M8, database; one ADR slot — the streaming read across the five drivers | `nvs-db`, `nvs-stdlib`'s `Core\Db` and `Core\Queue`, CI — every member answers on all five drivers |
| [m8-stdlib-depth](59-m8-stdlib-depth.md) | M8, non-database | `nvs-stdlib`, `nvs-render`, `benches/` — every class M8 names as deep as its spec section |
| [unowned-closures](60-unowned-closures.md) | post-parity, at most one ADR — the prepared-pattern channel, if the user's answers build it | every crate with an `unowned` gap — each built to the answer the user's decision sheet gave |
| [class-scoped-types](61-class-scoped-types.md) | post-parity, one new record — `rule:types/type-alias` gains a second declaration site | `nvs-syntax`, `nvs-hir`, `nvs-types`, `nvs-diagnostics`, `nvs-fmt`, `nvs-cli`, `nvs-lsp` — a `type` alias as a member of a class, an interface or an enum |
| [worker-placement](62-worker-placement.md) | post-parity, no ADR — ADR 0184 §§ 2, 5 as written | `nvs-host`, `nvs-runtime`'s script seam, `nvs-cli`'s unit table — a path entry is placed like a method entry, and a serving core offers itself as a destination |
| [core-class-tests](63-core-class-tests.md) | post-parity, at most one ADR — only if the checker needs a `Core` class's identity in a shape the registry does not already answer | `nvs-types`, `nvs-ir`, `nvs-stdlib`'s descriptor table — `instanceof` and `as` name a `Core` class, so a `mixed` narrows to the one it holds |
| [tds-bytes](64-tds-bytes.md) | post-parity, no ADR — ADR 0067 §§ 1 and 9 state both halves | `nvs-db`'s TDS encoder — a bound value carries its form, a binary marker is declared `varbinary`, and the plan cache tells the two declarations apart on the comparison it already makes |
| [cache-shared-dial](65-cache-shared-dial.md) | post-parity, no ADR — four rule fragments state every half | `nvs-config`'s secret registry, `nvs-stdlib`'s Redis client, `nvs-host`'s TLS client — `[cache.shared]` gains a credential pair and a database index, and `rediss://` is a third transport arm |
| [decided-closures](66-decided-closures.md) | post-parity, at most one ADR — the prepared-pattern channel, built once for `cldr.rs` and `time.rs` | every crate with a `Decided:` gap — the 40 goal `unowned-closures` tagged and did not build, and the 8 owed to M1, M6, M7 and M8, each built to its sentence, struck as a bound, or deferred honestly; the gate is `owners.py --closes`, which a tag cannot meet |
| [one-type-test](67-one-type-test.md) | post-parity, one ADR — the record that ends every comparison with PHP on `is` and `instanceof`, written first | `nvs-syntax`, `nvs-types`, `nvs-diagnostics`, `nvs-ir`, `nvs-codegen`, `nvs-runtime`, `nvs-stdlib`, `nvs-lsp`, `nvs-fmt` — `instanceof` is refused naming `is`, `$x is $cls` is the dynamic class test, and the word survives only in the refusal and the divergence |
| [test-doubles](68-test-doubles.md) | post-parity, no ADR — ADR 0079 §§ 10, 11 and 16 decided it | `nvs-types`, `nvs-diagnostics`, `nvs-stdlib`, `nvs-runtime`, `nvs-ir` — `Core\Test::double<T>` and `partial<T>` are a shape of closures checked against an interface and *are* a `T`; `assertCalled`/`assertNeverCalled` read the record against a compile-checked method reference; `assertCompletes` runs under the test's clock |
| [bigint](69-bigint.md) | post-parity, no ADR — ADR 0054 § 5 decided it | `nvs-stdlib` — `Core\BigInt` over `num-bigint`, a `Core`-owned immutable instance, `Stringable` and `Comparable`, the class that replaces `gmp` and `bcpowmod` |
| [gap-zero](70-gap-zero.md) | post-parity, no ADR — the terminal gate | `tools/`, the ratchet test — no gap owed by anyone but a future milestone, M0–M8 complete, the index deleted |
| [dossier](71-dossier.md) | `rule:testing/four-proofs` | none — it writes the goals that open all of them, then optimizes the loop for their shape |
| after `dossier` | `rule:testing/four-proofs`, generated | one group of features per goal, its own `[context]` manifest, under `goals/dossier/` |
| [ci-green](172-ci-green.md) | post-parity, no ADR — `position: last` | `tools/ci-green.py`, `.github/workflows/ci.yml` — always the last goal, behind every generated one: the latest CI run on `main` succeeded for the code `HEAD` holds |

## The chain contract

**This directory is the chain.** A goal is `N-<slug>.md` plus, until it is retired, a sibling `.toml`
and `.handoff.md`; the numbers run `1..N` with no gaps, and the order the driver walks is that number.
There is no second file saying what the order is, so **reordering the chain is renaming files** — and
`python tools/chain.py` is how that is done, never by hand. It scaffolds the three files below,
renumbers everything a move or an insert displaces, refuses an edit behind the live goal, and
`--check` says whether every goal is one the driver can walk. [commands.md](../commands.md) is the
tool's one home; this section is the contract it enforces.

A goal the run has **not reached** may be inserted, edited or appended while the loop is running:
`Chain.refresh()` (`tools/loop.py:3337`) re-reads the directory whenever a goal goes green and adopts
whatever is past the one it stands on. What it refuses is a rewrite at or behind the live goal, because
every switch has already folded that goal's checks into the next one's floor.

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

- **The last goal goes green** — goal `ci-green`, behind the last *generated* one, which is every group on
  `rule:testing/four-proofs`'s roster owing nothing, `python tools/dossier.py --gate` exiting 0 over the whole language.
  The parity program's own gate — goals `core-depth` through `server`, PHP core feature parity — is
  still that goal's final check:
  `python tools/check-migration.py` reporting 100% classified — every one of the oracle build's **1167
  functions and 255 types** accounted for, every `member` row registered, every one of them cased. The
  inventory grew from 925 when the oracle build gained `mysqli`, `pgsql`, `sqlite3`, `fileinfo` and
  `zip`: the APIs `rule:core-classes/db-one-api`, `Core\Zip` and `Core\Mime` replace are now inside the
  audit rather than a named hole beside it.
- **A goal reports `BLOCKED`.** Reserved for a decision that is expensive to reverse *and* has no safe
  default. Every goal's standing decisions exist to make this rare.
- **`--max-stalls` consecutive sessions move `HEAD` nowhere.**
- **Goal `database`'s Docker preflight fails.** `rule:core-classes/db-one-api` verifies the drivers against real servers, so the driver
  checks for a reachable daemon before the first session of that goal and stops the run naming it. A run
  that grinds for six hours against a check that cannot pass is worse than one that stops in the first
  minute.

## What no goal on this chain takes

`Web\Migration` and everything versioned about a schema change — ordering, history tables, fleet
locking, reversibility — which `rule:programs/no-migration-runner` records as
deliberately blocked. Goal `schema` builds convergence, which needs none of them, and does not close that gap.

Doc trimming and dependency sweeps, both of which the user fires and never a session
([doc-cleanup.md](../doc-cleanup.md), [dependency-update.md](../dependency-update.md)). And **PHP's
optional extensions** — `gd`, `intl`, `imap`, `zip` and the rest of the unaudited list in
[02-php-migration.md](../../spec/02-php-migration.md) — are not parity work: they are M9's, and a session
that finds one on its path puts it in the handoff's `## Backlog` and moves on.
