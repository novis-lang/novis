---
milestone: post-parity
---
# Loop goal 122 — the tools are one typed program over structured records, and every Markdown file a person reads is rendered from them

When this goal is green:

- **The records.** Every fact the tools work with lives once, as a typed JSON record under `data/`.
  Every Markdown file a person reads is prose written by hand, or rendered from records by
  `bun nv render`. No tool parses Markdown for structure again.
- **The tools** are one TypeScript program on Bun, `bun nv <command>`. It has one library, one
  loader, one writer that always writes LF, and one SQLite index. The index's foreign keys and views
  do the work of the ten thousand lines that today only check that two copies of a fact agree.
- **Checks rerun only when their inputs change.** Every check the driver, `verify` and `proofs` run
  is keyed on what it really reads, and reruns only when that changes.
- **The playbook is triaged.** Every bullet names its file and expires by itself.
- **The feature-proof goals are ordinary goals.** The generated goals are ordinary chain goals, and
  the generator is gone.
- **No Python is left.** No Python file is tracked, CI and the git hooks run Bun, and every tracked
  text file is LF.

## Why here

**`main` is frozen until this goal is walked.** It is the user's call: nothing else runs, and no side
goal runs beside it. So it runs on `main` as the next chain goal, not in a worktree. That saves:

- a second `target/`;
- the memo seeding;
- the landing's rebase and repeated verify.

It sits in front of the 57 feature-proof goals still ahead because every one of them pays the tooling
tax this goal removes:

- goal-end sweeps are 23.6 of 28 check-hours;
- `dossier.py` is 40% of a session's wall clock;
- `chain.py --check` takes 9.4 s;
- about a third of the recent tooling commits fix a parser or a stale copy.

It needs nothing the chain has not built. It absorbs two side goals written for the same tools:

- `test-time`, which keys every check on what it reads;
- `playbook-triage`, which makes every bullet name a file and expire by itself.

Both are retired into it. Their measurements and their decisions are carried here whole.

**The git tag `pre-overhaul` marks the commit this goal starts from.** Rolling the whole goal back is
`git reset --hard pre-overhaul`, and only the user does that.

## What is on disk today, measured

Measured on 2026-09-24. Every cause has an anchor. Re-check a line number before you edit at it.

**The data, and the parsers over it:**

- **About 10k of the 40k lines in `tools/` exist to keep copies of one fact in agreement:**
  - `chain.py` (1,139 lines) renumbers files and rewrites three header regexes.
  - `records.py` checks that every `because` a rule names and every `changes:` a decision record names
    agree in both directions.
  - `goal-switch.py` copies checks into the next goal verbatim. 1,010 of the live goal's 1,017 checks
    are copies.
  - `plan.py --sync`, `decisions.py`'s digests, `check-links.py`, `owners.py`, `context-sync.py` and
    `loop.py:5104` `relocate_links` do the same kind of work.
- **About 16 formats are parsed by two to seven readers that disagree.** Front matter alone has seven:
  `goals.py:69`, `reference.py:85`, `dossier.py:907`, `records.py:203`, `decisions.py:219`,
  `brief.py:382` and `orient.py:1143`.
  - Markdown sections are read four ways: `orient.py:247`, `session.py:797`, `peek.py:260` and
    `playbook.py:903`.
  - `loop.py:4442` parses `orient.py`'s printed pack.
  - There is no shared library. Tools import each other through `sys.path`, and `orient.py` and
    `playbook.py` import each other in a cycle.
- **Live bugs:**
  - `chain.py:822-828` `--set --milestone` drops `position: last`.
  - `lints.py:162-175,210` deletes everything after `[lints]` in a `Cargo.toml`.
  - `holes.py:122` misses an item whose bold title wraps onto a second line.
  - `release.py:436` bumps fewer version pins than `release.py:530` checks.
  - `playbook.py:144`, `holes.py:50`, `plan.py:107` and `goal-switch.py:47` ignore a side run's goal.
  - `rules.py:515`, `context-sync.py:194` and `lints.py:210` write without `newline="\n"`. **198
    tracked files are CRLF** against `.gitattributes`'s `eol=lf`.
  - `tools/git-hooks/pre-push` calls a bare `python`, so it refuses every push on a machine that has
    only `python3`.
- **Duplicated state:**
  - `docs/agent/loop-goal.{md,toml}` and `docs/agent/handoff.md` are committed copies of the live
    goal's files. 492 commits in the last 7 days touched them, and the two copies disagree now.
  - `docs/rules/<topic>.json` repeats `_index.json`'s title and order.
  - `docs/decisions.toml` summarises 172 of 215 records.
  - The two `tmLanguage` grammars differ from their line 4:
    `editors/vscode/syntaxes/nvs.tmLanguage.json` and `website/config/novis.tmLanguage.json`.
  - Four spellings of "generated, do not edit" exist, and `check-links.py:196` knows one of them.
  - The website's `scripts/sync-rules.mjs` and `sync-core.mjs` are a third set of parsers over the
    same files.

**Speed.** Timed on this machine with the loop idle. Python 3.11 starts in about 0.1 s.

| Command | Time | Why |
|---|---|---|
| `chain.py --check` | 9.4 s | `orient.manifest_findings` re-reads 1,533 playbook files and rebuilds the rulebook once per goal. That is 110k file reads for 13k files. |
| `session.py --check` | 3.5 s | |
| `check-links.py` | 3.2 s | It stats every link, which is slow on Windows. |
| `dossier.py --id` | 2.7 s | It collects the whole roster for one feature, and `rel()` resolves 8k paths. |
| `gaps.py` | 2.0 s | |
| `brief.py` | 1.7 s | Nine multiline regexes run over 27 MB of Rust. |

A probe loaded 3,109 of these files into SQLite in 0.51 s with Python and 0.22 s with Bun.
**`Bun.TOML.parse` rejects valid TOML**: three of our goal files, on `[context.stage.2]`.

**Checks that run again for nothing** (this is side goal `test-time`'s measurement, from
`.loop/check-times.ndjson` over 201 sweeps):

- **Goal-end sweeps cost 23.6 of 28 check-hours.** A test-only or card edit in
  `crates/nvs-stdlib/src/*.rs` re-keys nearly every check.
- **`crates/nvs-cli/src/script.rs:2505` runs 10,000 compiles in a debug build.** That is 315 s, the
  tail of every `verify.py`. The floor names this test by its prefix `units_held_stay_bounded_after_`.
- **`tools/loop.py:2190` `plain_crate_test` misses `--bin` and `--lib` checks.** Such a check pays
  `cargo` a second time.
- **The tiers in `tools/verify_keys.py` are too wide:**
  - The `shipped` tier (`:34`) is used by only four steps (`:413-417`).
  - An embedded file is raw at every tier (`:353-356`). Seven stdlib files are embedded by their own
    policy tests.
- **`tools/impact.py:313` keys a dependency at the `code` tier.** A stdlib test edit therefore re-runs
  125 of 180 binaries.
- **`tools/loop.py:2024-2028` `COMMAND_READS` is too wide:**
  - fuzz is keyed on stdlib, but `fuzz/fuzz_targets/prefix.rs:24-26` uses only `nvs-syntax` and
    `nvs-diagnostics`;
  - TSan is keyed on `tools/`, `docs/` and every crate's tests;
  - `EDITOR_READS` is raw `crates/`.
- **`tools/loop.py:3276` widens a gate to the whole tree when it spawns anything unexpected.**
  `dossier.py:1526` `platform.system()` spawns `cmd /c ver`, so no floor proof check is ever
  remembered. `PATHS` puts the tree's path set into every observed key. `.loop/check-reads.json` is
  73 MB.
- **`tools/dossier.py`:**
  - `:3501` `collect` reads the whole roster whatever is scoped.
  - `--id` does not narrow `--run` or `--record-perf` (`:3485-3500`).
  - `:542-556` `current_binary` relinks thin-LTO release on any newer file under `crates/`.
  - `:501-515` misses the files the binary embeds from outside `crates/` (`tools/loop.py:1864-1868`).
    So **an edit to a reference chapter leaves proofs judged on a stale binary**.
  - `:1425` `binary_key` is size and mtime.
  - `:1593` `impl_hash` goes stale on a comment edit.
- **Doc-tests and flaky tests:**
  - `verify.py` runs rustdoc over 18 crates to find five doc-tests, which costs 32 s.
  - Its pool sets no `RUST_TEST_THREADS`, and four tests fail under load:
    - `nvs-host` `watchdog.rs:926`;
    - `nvs-stdlib` `http/transport.rs:4117` and `socket.rs:1241`;
    - `nvs-lsp` `latency.rs:212`.

**The playbook** (side goal `playbook-triage`'s measurement, from 2026-09-23):

- There are 1,533 bullets and they grow by about 43 a day.
- 1,178 end in `[until: reviewed <date>]`, which nothing retires.
- 533 name no file.
- Only 43 can reach a session, through 11 manifest selectors.

## The data model

**Types and records:**

- **A record type is declared once**, in `tools/nv/schema/`. That declaration gives the static type,
  the runtime check and the JSON Schema that `data/schema/` publishes for the website and editors.
- **A record is one JSON file under `data/`.** It is written only by the `nv` writer:
  - keys in schema order, two-space indent, LF, one trailing newline;
  - one entity per file, so git merges line by line.
- **An id is a slug or a number, and it never changes.** A reference is an id that the index checks as
  a foreign key.

**Prose:**

- **Prose stays where it is under `docs/`**: rule fragments, decision records, goal prose, milestone
  files, reference chapters, `about.md`. That keeps its path, and the tree's `rule:` and `NNNN § N`
  citations keep working.
- **A prose file holds prose and nothing a tool reads.** Its front matter and any header fields are
  output: `nv render` writes them from the record, so GitHub still shows them.
- **One scanner in the library reads a citation or a link out of prose** into the index. Nothing else
  looks inside prose.

| Entity | Record | Prose it pairs with | Replaces |
|---|---|---|---|
| goal | `data/goals/<slug>.json`: milestone, position, context, stages, checks with stable ids, env tables | `docs/agent/goals/<slug>.md` | `N-slug.toml`, the goal's front matter, `loop-goal.*` |
| chain | `data/chain.json`: the ordered slugs, and nothing else | — | the number in every goal file name, `chain.py`'s renumbering |
| handoff | `data/goals/<slug>.handoff.json`: `state` (Markdown string), next group `{stage, files, items[]}`, `backlog[]` | — | `N-slug.handoff.md`, `docs/agent/handoff.md` |
| side goal | `data/goals/side/<slug>.json` + handoff | `docs/agent/goals/side/<slug>.md` | the same three files |
| rule | `data/rules/<topic>/<slug>.json`: title, status, because, divergesFromPhp, seeAlso, guardedBy | `docs/rules/<topic>/<slug>.md` (unchanged) | `docs/rules/<topic>.json` |
| topic | `data/rules/<topic>.json`: title, order, the ordered rule slugs | — | `_index.json` and the topic json's copy of it |
| decision | `data/decisions/NNNN.json`: status, scope, dependsOn, validatedBy, summary `{group, headline, body, pin}` | `docs/decisions/NNNN.md`, body only | the YAML front matter, the bold bullet fields, `docs/decisions.toml`, the digest; `changes:` becomes derived from rules' `because` |
| plan field | `data/plan/status.json`: the seven fields, a closed set | — | the `> **Field:**` block |
| milestone | `data/plan/milestones/<id>.json`: title, estimate, loop-days | `docs/plan/<id>.md` | the milestone table; `Carried by` becomes a view |
| gap | `data/gaps/<crate>/<slug>.json`: module, title, text, owner (goal slug or milestone) | — | `//! # Known gaps` blocks, positional numbers, `— owner:` trailers |
| playbook bullet | `data/playbook/<section>/<slug>.json`: lead, body, files[], until `{kind, arg}` | — (short prose lives in the record) | `docs/agent/playbook/**/*.md` |
| reference chapter | `data/reference/<path>.json`: id, title, summary, keywords | `docs/reference/**.md` | the chapters' front matter |
| spec row | `data/spec/core-members.json`, `data/spec/php-migration.json` | the spec chapters' prose | the tables `gaps.py`, `check-migration.py`, `reference.py` and `sync-core.mjs` parse |
| proof policy | `data/proofs/policy.json` (the skips), `data/proofs/help-backlog.json` | — | `tools/data/dossier-policy.toml`, `help-backlog.toml` |
| impact probe | `data/impact-probes.json` | — | (new, Stage 6) |

**What stays where it is:**

- **Stays beside the code it describes**, read by one parser module:
  - `// covers:`, `// bench:`, `// hostile:` and `// requires:` directives;
  - `.nvst` sections;
  - the registry's reference cards (`rule:core-api/reference-card`, unchanged);
  - `crates/nvs-config/src/tree.rs`'s `[unread:]` trailer.
- **Stays structured already:** the perf ledgers (`docs/perf/*.ndjson`).
- **Runtime state is not committed:**
  - `.loop/state.sqlite` holds the memo, the observed reads, check times and the chain pointer.
  - `.cache/nv.sqlite` is the index, rebuilt per file on size and mtime and then hash.
  - Both are gitignored.
- **Committed and rendered** by `bun nv render`, marked `GENERATED by bun nv render — do not edit`,
  `linguist-generated` in `.gitattributes`, and checked by `render --check` in CI:
  - `docs/rules/<topic>.md`, `docs/ground-rules.md`, `docs/divergences.md`;
  - `docs/decisions.md`, `docs/novis.md`;
  - the plan and the milestone table;
  - the goals README table;
  - `docs/agent/playbook/<section>.md`;
  - the spec tables;
  - `docs/perf/members.md`;
  - `website/src/data/*.json` and the website's rule and core pages;
  - every prose file's front matter.

## Stage 0 — the catch-up

This goal makes most of the process docs wrong. They are rewritten whole in Stage 12, once, from what
is true then. The sentences are rewritten earlier in these two cases:

- **The session that changes a behaviour** rewrites any comment or module doc that describes it:
  - `tools/verify_keys.py`'s tiers;
  - `tools/impact.py` § *What a test binary reads*;
  - `tools/loop.py`'s `COMMAND_READS`, `BUILDS`, `EDITOR_READS`, `plain_crate_test` and
    `observed_inputs` docs;
  - `tools/dossier.py`'s `RELEASE_STAMP`, `BUILD_INPUTS`, `current_binary`, `binary_key`,
    `impl_hash` and `fingerprint` docs;
  - the bounded-units test's comment.
- **`docs/rules/testing/member-perf-ledger.md`** — its paragraph starting "A figure is re-measured
  only when" is rewritten whole in Stage 7.

## Stage 1 — the floor

Goal `core-json-and-6-more`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`.
It is never traded.

**258 floor checks run a Python tool**: `dossier.py` 97, `rules.py` 91, `owners.py` 18, and 18 others.
Stage 9's cutover rewrites each one's `argv` to its `bun nv` form with a script. The check's name
changes only where it says `dossier`. **Every `want` line stays.** So the output line a floor check
wants is a contract the port keeps.

## Stage 2 — the Rust and Cargo cuts first

One file set: `crates/nvs-cli/src/script.rs`, the four flaky tests' files, the crates' `Cargo.toml`,
the workspace `Cargo.toml`, `tools/verify.py`. They go first because they make every later session's
`verify` minutes faster.

- **The bounded-units test runs 200 edits** and is renamed
  `units_held_stay_bounded_after_two_hundred_edits`. The assertion after every edit stays.
- **`doctest = false`** is set in every crate with no doc-test. The one-line
  `RUST_TEST_THREADS` change in `verify.py`'s pool sets pool width times threads to about the
  machine's cores.
- **The four load-flaky tests wait on a condition, never on a wall-clock bound.**
- **`[profile.proof]`** inherits `release` with `lto = false`, `incremental = true` and
  `codegen-units = 256`. Overflow checks stay on and debug assertions stay off. Stage 7 wires it in.

## Stage 3 — the foundation: `bun nv`, its library and its index

One file set: `package.json`, `bun.lock`, `tsconfig.json`, `tools/nv/**`, `.gitignore`,
`tools/verify.py` (one step).

- **The package:**
  - `package.json` at the root has one script, `"nv": "bun tools/nv/main.ts"`, and pins Bun in
    `engines`.
  - Dev dependencies are `typescript` and `@types/bun` only. The one runtime dependency is
    `smol-toml`, for the TOML files other programs own (`Cargo.toml`, `rust-toolchain.toml`,
    `nvs.toml`). It is never used for a record.
  - `tsconfig.json` is `strict`.
- **`tools/nv/lib/` is the one library:**
  - `schema`: the builder that yields a type, a validator and a JSON Schema from one declaration;
  - `store`: the loader and the writer, which is the only code that writes a record;
  - `index`: SQLite with foreign keys, the views, and incremental rebuild;
  - `prose`: fence-aware headings, sections and the citation scanner;
  - `git`, `paths`, `proc` (spawn with a timeout, never through a shell);
  - `render`: templates, the one generated marker, and LF writing.
- **Commands:**
  - `nv check` runs schema, foreign-key and view invariants and prints each finding with the record
    path.
  - `nv query "<sql>"` runs read-only SQL over the index.
  - `nv render [--check]` writes or checks the rendered files.
  - `nv selftest` runs `tsc --noEmit` and `bun test`.
- **Tests:** `bun test` covers the writer's round trip, the schema, the index rebuild and the prose
  scanner. `verify.py` gains a `nv selftest` step.

## Stage 4 — the importer

One file set: `tools/nv/import/**`.

- **`bun nv import`** reads every legacy home in the entity table and builds the records in memory.
  - `--check` writes nothing. It prints every file it could not read, with the reason. It then
    renders from the records and compares with every generated file on disk.
  - `--write` writes `data/` and rewrites the prose files' front matter as output.
- **Where it and the old tools disagree**, a declared-differences list in `tools/nv/import/known.json`
  names each file and why. Examples are a wrapped title `holes.py` never saw, and a bold-run gap
  block. That list is deleted with Python.
- **Gaps get a slug** from their bold title. Every citation of a gap by position is listed for Stage 9
  to rewrite.
- **Goal numbers become `data/chain.json`.** The 102 goals under `docs/agent/goals/dossier/` join the
  others as ordinary goals, in the same order.

## Stage 5 — the read-only tools, proven by parity

One file set: `tools/nv/cmd/{peek,brief,plan,records,rules,decisions,owners,gaps,holes,links,layout,disk,directives,migration,reference}.ts`, `tools/nv/parity/**`.

- **Each replaces a Python tool's read-only modes**, under the same subcommand name:
  - `nv peek` (with `--locate` and `--outline`);
  - `nv brief` (with `--where`);
  - `nv plan --show`, `nv records --stats`;
  - `nv owners`, `nv gaps`, `nv holes`;
  - `nv links` (was `check-links.py`), `nv layout`, `nv disk`, `nv directives`;
  - `nv migration` (was `check-migration.py`), `nv reference`.
- **Each checks it replaces becomes a query in `nv check`.** Its old flag stays as an alias that prints
  the same verdict lines.
- **`bun nv parity <group>`** runs the Python tool and its replacement on the same tree and compares
  their output. The comparison ignores only what `tools/nv/parity/known.json` declares. The main
  declared difference is a goal named by slug where Python prints a number. Parity is proven for every
  mode the floor, a goal `.toml` or a process doc invokes.

## Stage 6 — one key for what a check reads (the keystone)

One file set: `tools/nv/keys/**`, `tools/nv/cmd/{impact,why}.ts`, `data/impact-probes.json`.

This is side goal `test-time`'s design, built once in TypeScript. Nothing of it is built in Python
first.

- **One function returns the key of what a binary is built from:**
  - its cargo closure (from `cargo metadata`, never a parse of `Cargo.toml`), each dependency at the
    `shipped` tier;
  - its own package raw, but only for the test binary built from it;
  - the toolchain and the manifests;
  - the files embedded at an include site outside a test module. An `include_str!` inside a test
    module is an input of that test binary alone.

  `verify`, `impact`, `loop` and `proofs` all call it, and no second copy of the rule exists.
- **A card tier below `shipped`** blanks the initialiser of every `const` whose type is a registry
  card type: `ClassDoc`, `MethodDoc`, `ParamDoc`, `ShapeKeyDoc`, `ErrorDoc`, `EnumDoc`, `CaseDoc` and
  any later `*Doc`, found by type name. It also blanks a slice of them.
  - **These checks key on the card tier**, because they only run programs: `valgrind `, native
    examples, fixtures, the `.nvst` suites, the proof runs, fuzz, TSan and the database matrix.
  - **Every other check keeps `shipped`.** That includes every check that runs `nvs meta`,
    `nvs agent`, `nvs doc` or `nvs lsp`, the cost margins, and every test binary.
- **`COMMAND_READS` rows name what each command builds and opens:**
  - fuzz: the closure of the crates its target's source `use`s, plus `fuzz/**`;
  - TSan: `tools/tsan.sh`'s `-p` packages raw, their closure at `shipped`, the script, and
    `examples/`;
  - the database matrix: the same, plus `tests/db/`;
  - abi-probe: drops `crate-tests`;
  - the editor: `editors/`.
- **`PATHS` enters a key only for a gate that lists files.** A test check of any shape takes
  `verify`'s per-binary record, and so do the conformance and differential verdicts.
- **`bun nv why "<check name>"`** prints `source: <verify record | binary key | observed | package
  key | partitions | everything>`, then the key's inputs grouped by partition.
- **`data/impact-probes.json` is the contract, and `bun nv impact --probe` holds every key to it.**
  - Each probe applies a synthetic edit to an in-memory copy of the tree and names what must re-run
    **and** what must not. Both lists are required, and an empty `rerun` list is refused.
  - The ten probes are `test-module`, `self-included-test-module`, `card`, `stdlib-code`,
    `syntax-code`, `host-code`, `embedded-chapter`, `driver-tool`, `new-example` and `editor`. They
    are exactly side goal `test-time`'s table:

    | Probe | Must re-run | Must not re-run |
    |---|---|---|
    | `test-module` (a `#[test]` in `jwt.rs`'s `mod tests`) | stdlib lib tests, clippy | other binaries, doc-tests, `valgrind `, examples, suites, fuzz, TSan, the database matrix, proofs, builds, vsix |
    | `self-included-test-module` (the same in `jwe.rs`) | the policy test's binary, clippy | what `test-module` keeps |
    | `card` (a `MethodDoc` `short` in `json.rs`) | stdlib lib tests, `nvs meta`/`agent`/`doc` checks, `nvs-lsp` | `valgrind `, examples, suites, proof runs, fuzz, TSan, the database matrix |
    | `stdlib-code` | `valgrind `, examples, suites, proofs, the database matrix, cost margins | fuzz, TSan |
    | `syntax-code` | fuzz, `valgrind `, suites | — |
    | `host-code` | TSan | fuzz |
    | `embedded-chapter` | what prints it; "the proof binary is current" turns to no | fuzz, TSan |
    | `driver-tool` (a line of the driver) | the gates that read it | `valgrind `, suites, TSan, fuzz, the database matrix, proofs |
    | `new-example` | that member's proof group | every other group |
    | `editor` | vsix | `valgrind `, suites; a `crates/` edit does not re-run vsix |

## Stage 7 — proofs: the dossier becomes ordinary feature-proof machinery

One file set: `tools/nv/cmd/proofs.ts`, `tools/nv/proofs/**`, `Cargo.toml`'s proof profile,
`docs/perf/members.ndjson`.

- **`bun nv proofs` replaces `dossier.py`.** It keeps only:
  - the roster;
  - what a feature owes (`--id`, `--owed`, `--gaps`);
  - `--verify`, `--run`, `--bless` and `--comments`;
  - `--record-perf` and the perf report.
- **These are deleted, with no replacement:**
  - `--emit-goals`, `--check-goals`, `--per-goal`, `--all-groups`, `--out`;
  - the fan-out workers: `--partition`, `--workers`, `--brief`, `--findings`, `--clear`;
  - the driver's half of them: `loop.py:4831`, `loop.py:7564` `generated_by`, `.loop/dossier-*`.

  No goal is generated again.
- **One pass.**
  - `--verify` takes several `--group` values and builds the roster once.
  - The driver runs every proof check a sweep owes in one `nv proofs` call. It hands each check its
    own group's verdict lines, and memoises each on its own key.
  - `--id` narrows `--run`, `--verify` and `--record-perf`.
  - **`--run --id <feature> --show`** prints each of the feature's programs as it runs, in the
    order examples, attacks, bench: a `== <path>` line (repository-relative, forward slashes), its
    output, then `exit N · T ms`. An attack's line ends `T ms of L ms` against its
    `timeout-ms`, or the 10 s default. It runs on the proof binary. A session writing a feature's
    proofs reads this instead of running `nvs run` over each file by hand, which in the 63 loop
    sessions logged before this goal was 287 `nvs run` calls and about 45 timing wrappers.
  - Nothing is spawned but `nvs`. `fingerprint` is computed only where perf is recorded.
- **Two binaries.**
  - **The proof binary** (`target/proof/`) runs `--bless`, `--run` and `--verify` in a session and in
    a scoped sweep.
  - **The release binary** runs `--record-perf`, the cost margins, and every gate-open sweep's proof
    checks.
  - "The binary is current" is decided by the Stage 6 key, never by mtime. So a relink that changed
    nothing re-runs nothing, and a reference-chapter edit rebuilds.
  - `nv disk` counts `target/proof/`.
- **Perf currency ignores comments, layout and cards.** `impl_hash` hashes the implementing file at the
  card tier. The same commit rewrites every `impl_hash` in `docs/perf/members.ndjson` whose old-form
  hash matches the file on disk. `rule:testing/member-perf-ledger`'s currency paragraph is rewritten
  whole, with one new decision record.
- **Directives:** `// dossier: exit N` becomes `// proof: exit N`, and `// dossier: known-gap
  <file> -- …` becomes `// proof: gap <gap-id>`. A script does both.

## Stage 8 — the writers and the driver

One file set: `tools/nv/cmd/{verify,orient,session,chain,side,loop,respawn,splice,bg}.ts`,
`tools/nv/driver/**`.

- **`nv verify`** takes over `verify.py`'s steps unchanged, over the Stage 6 keys.
- **`nv orient`** builds the pack from records:
  - `[context]` resolves by id, and the index answers it in one query;
  - the traps are ranked by the files the item names (Stage 10);
  - it prints `N of M` for a goal's position, computed from `chain.json`.
- **`nv session --wrap`** keeps the wrap file's shape, Markdown with `## <kind>:` sections. It is input,
  not storage. One parser reads it, and it applies every write as records, all or nothing.
  - The playbook section writes bullet records with `files` and `until`.
  - The plan section writes `status.json`.
  - The handoff section writes the handoff record.
- **`nv chain`:**
  - `--new`, `--move` and `--remove` edit `chain.json` and nothing else;
  - `--check` is a query.
  - No file is renamed and no number is written anywhere.
- **A goal switch copies nothing.** The floor is the view "every check of every walked goal", and the
  live goal is `.loop/state.sqlite`'s pointer, by slug. `goal-switch.py`, `context-sync.py` and
  `relocate_links` have no successor. A session adds a module to `[context]` with `nv goal context
  --add <path>`.
- **`nv loop` is the driver.** It keeps `respawn`'s protocol: exit `75` means start me again, and
  `NOVIS_LOOP_RUN` names the run. It also keeps every flag the process docs name, and
  `FLOOR_GATE_EVERY`, `AUDIT_EVERY`, the heavy legs and the fuzz campaign unchanged.
  - **`--list` takes `--stage <label>`, `--name <text>` and `--feature <id>`.** Each narrows the
    plan to the checks that match, and the last line is `list: N check(s) match`. A session
    finds its own checks this way. In the 63 sessions logged before this goal, sessions found
    them in the 13,766-line goal file with `grep` and `sed -n`, in 48 calls.
- **`nv bg <command…>`** starts any long command detached and prints a job id.
  - `nv bg --wait <id>` blocks until the job ends, prints the tail of its output and exits with
    its exit status. `nv bg --list` names the jobs still running.
  - A job and its log live under `.agent-tmp/bg/`. `nv verify --start` and `--wait` are this
    command over verify's own steps, not a second mechanism.
  - A session never polls a harness's task file with `until … sleep` again. In the 63 sessions
    logged before this goal, that was 59 calls, waiting on `cargo build --release`, `cargo test`
    and `--record-perf`.
  - `bun test tools/nv/test/bg.test.ts` starts a job, waits on it, and checks its output and
    exit status, for a job that exits 0 and for one that does not.
- **Parity:**
  - `bun nv parity writers` shows that a wrap applied by both tools to two copies of the tree gives the
    same records and the same rendered files.
  - `bun nv parity orient` shows that the packs for the live goal carry the same sections and items.

## Stage 9 — the cutover

One slice, in one commit.

1. `bun nv import --write`.
2. Delete every legacy structured home the import replaced:
   - the topic JSON files and `_index.json`;
   - `docs/decisions.toml`;
   - every goal `.toml` and `.handoff.md`, `docs/agent/loop-goal.*`, `docs/agent/handoff.md`;
   - `docs/agent/goals/dossier/`, whose prose moves beside the other goals, slug-named;
   - the numbered file names;
   - the `# Known gaps` blocks, replaced by one line naming `bun nv gaps --module <path>`;
   - `tools/data/*.toml`.
3. A script rewrites:
   - the 258 floor `argv`s;
   - every `dossier:` check name, to `proofs:`;
   - every citation of a gap by position;
   - every link to a numbered goal file.
4. **`tools/loop.py` becomes a shim of a few lines** that runs `bun nv loop` with the same arguments
   and passes its exit code through, so the running `respawn.py` starts the new driver at the next
   turn.
5. `.loop/chain.json` becomes the state database's pointer.
6. Every tracked text file is renormalised to LF.
7. `bun nv render`, then `bun nv check`.

Before the commit, `bun nv loop --goal-only` must be green under the new driver. The next turn is the
new driver.

## Stage 10 — the playbook triage, on records

One file set per section: `data/playbook/<section>/`, plus the module whose doc comment a moved
bullet lands in, and `tools/nv/cmd/playbook.ts`. This is side goal `playbook-triage`'s work.

- **`bun nv playbook --triage <section>`** prints, for every bullet, what a session needs to decide it
  without opening anything else:
  - its text;
  - the files it names, and the full path a crate or a bare file name most likely means;
  - a proposed `gone <path>:<word>` on a backticked word that is in that file today;
  - `LIVE MANIFEST` when a goal's context reaches it;
  - `NO NAMED WORD IN ITS FILE`;
  - its near-duplicates.

  It ends `triage: <section>: <n> bullet(s), <m> still owe a decision`, or `triage: <section>: every
  bullet names a file and declares a condition the tree decides`. It writes nothing.
- **A session decides bullets in batches with a script it writes for the batch.** It applies the
  proposals it accepts, and commits each batch. The sections go in this order:

  | Order | Section | Bullets |
  |---|---|---|
  | 1 | `divergences` | 37 |
  | 1 | `splitting-a-file` | 13 |
  | 2 | `running-things` | 163 |
  | 3 | `writing-novis` | 351 |
  | 4 | `tooling` | 422 |
  | 5 | `writing-a-test-case` | 546 |

  Tooling bullets about a tool this goal deleted are deleted.
- **Served by file.** `nv orient` ranks the whole playbook against the paths the session's item names
  and prints the best 20 in full. A manifest's selectors, now bullet ids, still add bullets on top.
  `bun nv orient --traps <path>...` prints the traps section alone.
- **The gate.**
  - The schema requires `files` to be non-empty, with every path existing or named by a `gone`
    trailer.
  - It allows `until.kind` in `gone | exists | test | rule` only.
  - `nv check` fails on either, and CI's docs job runs it.

## Stage 11 — every other tool, CI, the website and the hooks

One file set per bullet.

- **Ported to `bun nv <name>`:**
  - `bench`, `bench-load`, `bench-proxied`, `db-matrix`, `release`, `gen-attribution`;
  - `ci-changes`, `ci-green`, `lints`, `loop-stats`, `machine`, `observe`, `proctree`;
  - `relink`, `try`, `class-cards`, `exe-icons`, `origin`, `guard-read` (the context hook), `written`;
  - `webcrypto-vectors.mjs`.
- **What stays as it is**, because its platform is its purpose: `tsan.sh`, `leak-check.sh`,
  `service-live.ps1`, `dump-php-builtins.php`.
- **The Cargo-editing tools** read manifests with `smol-toml` and write by line-scoped edits. Each
  edit is verified by `cargo metadata`. The `lints.py` tail deletion and the `release.py` pin
  mismatch are gone with them.
- **CI:**
  - every `python tools/...` step in `.github/workflows/*.yml` becomes `bun nv ...`;
  - `oven-sh/setup-bun` is pinned by SHA, as every action here is;
  - the docs job runs `nv check` and `nv render --check`.
- **The website.**
  - `website/scripts/sync-rules.mjs` and `sync-core.mjs` are deleted. Their data comes from
    `bun nv render --website`.
  - The website's `tmLanguage` is rendered from the editor's grammar, unless the difference turns out
    to be deliberate. Then the reason is written in the grammar's record, and both files stay.
- **`tools/git-hooks/*`** call `bun nv`. `docs/setup.md` says Bun is installed and `bun install` runs
  after a clone.

## Stage 12 — no Python, and the contract says what is true

One file set: `AGENTS.md`, `docs/agent/*.md`, `docs/agent/goals/README.md`, `docs/setup.md`,
`docs/rules/` wherever a rule is about the old tooling.

- **Every tracked `.py` file is deleted, except two:**
  - `tools/respawn.py`, the running process;
  - the `tools/loop.py` shim.

  The run ends at this goal's `GOAL REACHED` on purpose. The driver's last turn prints the new start
  command, deletes both files in its closing commit, and exits with a code that ends `respawn`. The
  user starts the next run with `bun nv loop`.
- **The contract is rewritten whole**, by the authority § *Standing decisions* grants:
  - AGENTS.md: the routing table, the rules that name a tool, and the numbering and copy rules that
    no longer apply;
  - commands.md, conventions.md (`A playbook bullet`, `A status-block field`, `Citing a document`);
  - loop-authoring.md, coordinator.md, session-prompt.md, playbook.md, doc-style.md;
  - goals/README.md.
- **The decision records.** One new decision record states the data model, the runtime and the
  generated-output rule. A second one states the perf-currency change of Stage 7, if Stage 7 did not
  already write it.

## Standing decisions

These are the user's calls, made on 2026-09-24. No session re-decides one.

**Where it runs:**

- **`main` is frozen.** This goal runs on `main` as a chain goal. No other goal, side goal or hand
  work runs until it is walked. Side goals `test-time` and `playbook-triage` are retired into it.
  Side goal `mode-startup-switches` waits, and the importer converts it with the rest.

**The data:**

- **The runtime is Bun + TypeScript.** Records are JSON, and `Bun.TOML` is never used.
- **The chain order is `data/chain.json`, a list of slugs.** Goal files are named by slug alone. A
  position is printed as `N of M`, computed from the list. This keeps the 2026-09-09 decision's aim,
  one order that cannot drift, and removes its mechanism.
- **Known gaps are records**, with a stable slug and an owner foreign key. A crate's module doc no
  longer holds them.
- **Rendered Markdown is committed**, with one marker, `linguist-generated`, and `render --check` in
  CI. After a merge conflict in a rendered file, you re-render it. You never hand-merge it.
- **Prose stays in `docs/` at its path.** Its front matter is generated output, and no tool reads a
  field out of prose.

**The tools:**

- **One CLI, `bun nv <command>`.** A subcommand keeps the name of the Python tool it replaces, so every
  move is mechanical.
- **Dependencies:**
  - zero runtime dependencies except `smol-toml`, for TOML that other programs own;
  - dev dependencies are `typescript` and `@types/bun`;
  - adding any other dependency is out of scope.
- **The feature-proof goals are ordinary goals.** `dossier.py`'s generation and fan-out are deleted,
  and no new generated goal is added. What proves a feature is `bun nv proofs`. The word "dossier"
  leaves the tree: in check names, directives, tools and prose.
- **A mechanical change over more than a handful of files is made by a script written for it.** That
  covers the import, citation and `argv` rewrites, the renames, LF, and triage proposals.
  - The script lives under `.agent-tmp/` and is never committed, unless it is `nv import`.
  - The session runs it, reads a sample of the diff, and commits.
  - Hand-editing file after file is the mistake this rule exists to stop.
- **Parity before deletion.** A Python tool is deleted only in a slice after its replacement's parity
  check is green, and every declared difference is listed with its reason.

**The floor:**

- **The floor is never traded.** Its `argv`s are rewritten at the cutover. Its `want` lines stay. A
  check whose invariant a foreign key now makes automatic still runs.

**Test-time decisions (carried from side goal `test-time`):**

- **Every narrowed key is proved by `nv impact --probe` and single commands, never by a sweep.**
  Starting a `--settle`, a `--goal-only --full` or a full `verify` to show that a cut works is the
  mistake the user stopped on 2026-09-23.
- **One key function for what a binary is built from.** The cards stay in the registry, and a key
  tier skips them.
- **The proof profile is the user's choice.**
  - Its cost is one more target directory.
  - Its risk is that a difference that depends on optimisation shows up only at a gate-open sweep,
    which runs on release.
- **Perf currency ignores comments.** That reverses the rule's "a comment edit stales its file's
  figures", and it gets a decision record. The ledger is migrated in the same commit.
- **Nothing is dropped, and no gate frequency changes.** A narrower key is the only lever.

**The playbook:**

- **The triage rubric**, applied to each bullet in this order:
  1. **Delete** it when:
     - it is no longer true;
     - a tool now refuses the mistake it describes;
     - another bullet says the same thing (keep the clearer one);
     - its fact already has a home;
     - it is a session's story;
     - it describes a tool this goal deleted.
  2. **Move** a fact about how one subsystem works into that module's doc comment, rewritten whole,
     then delete the bullet.
  3. **Keep** everything else, with its files and a mechanical `until`.

  When unsure between keep and delete, keep it with a `gone` trailer. `git log -S` holds every deleted
  bullet.

**Authority:**

- **Full authority.** This goal may rewrite AGENTS.md, the process docs and the rulebook. It may also
  delete a rule or a section that exists only because of the old tooling: numbers in prose, the copy
  rules, `reviewed` bullets. AGENTS.md's priority ordering and § *Text an end user reads* keep their
  substance.
- **Decision records.** It opens up to two new ones, the data model and the perf currency, and no other
  number. The number is taken when the file lands.

**The tradeoffs, stated once:**

- **Security and correctness:** nothing is lost. Every key is at least as wide as what the build
  reads, the probes' `rerun` cells hold it there, and the stale-proof-binary bug is fixed.
- **Latency:** none on any request path, because this is tooling.
- **Memory:** none at run time.
- **Disk:** the proof target directory and a small index.
- **Usability:** every developer and CI job needs Bun, a single binary on Windows, macOS and Linux.
- **Simplicity:** one program, one library and one home per fact, in place of 50 scripts and their
  checkers.
