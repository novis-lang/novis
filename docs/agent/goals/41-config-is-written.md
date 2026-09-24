---
milestone: M6
---
# Loop goal 41 — The configuration is written down, and every key in it is read

Two facts about `nvs.toml` today, and each is what makes the other hard to see:

**The configuration is implicit.** With no `--config` and no `./nvs.toml`, a run takes the shipped
defaults — capabilities deny-all, `[mode] default = "production"`, every limit at its documented
number — and there is nothing on disk saying so. An operator who wants to know what they are running
under reads the rulebook, not their own deployment. `rule:config/no-configuration-file-is-a-complete-configuration`
is right that this is a valid host; it is the *inspectability* that is missing, not the validity.

**And some of what the file can hold is read by nothing.** `crates/nvs-config/src/tree.rs` parses 174
leaf keys under `deny_unknown_fields`, so every one of them is accepted, validated at boot, reported
by `nvs config dump` — and a handful reach no reader at all. Writing `[server] socket_mode = "0660"`
today is silence: the parser takes it, the audit prints it, and no listener is ever chmod'ed.

They are one goal because neither half is honest alone. A generated file that documents 174 keys is a
promise about all 174, so it cannot be written until each one is either implemented or says it is not;
and the audit has nowhere to publish its answer until the file exists.

It sits after goal `resource-ceilings` because goal `resource-ceilings` is priority 1 — request isolation — and this is priority 4,
simplicity of the surface. Goal `resource-ceilings`'s whole acceptance list is this goal's floor.

## Why here

It closes the gap between what `nvs.toml` *parses* and what anything reads — 174 leaf keys under
`deny_unknown_fields`, of which a hand-audit found nine that reach no reader at all — and it makes
the tree inspectable by writing it down, so a deployment on the shipped defaults has a file saying
so instead of a rulebook chapter. Its keystone is goal `gap-owners`'s applied a third time — a fact gains a
tag beside the thing it is about, and a gate fails on an untagged one — so a key cannot land unread
again.

## What is wrong today, in one line each

Read out of the tree rather than inferred. Stage 0's tool is what turns this hand-audit into a
roster nobody has to keep.

1. **`[cache] dir` is parsed, classified `System`/`Boot` in `crates/nvs-config/src/directive.rs`, and
   read by nothing.** The artifact cache reads `opcache.file_cache_dir`
   (`crates/nvs-cli/src/cache.rs:@from_config`); `Cache::dir`'s only mentions outside `tree.rs` are in
   `crates/nvs-config/tests/`. Two spellings for one directory, one of them inert. Stage 1.
2. **`[server] socket_mode` reaches no code in the workspace.** Its rule,
   `rule:http-server/a-unix-socket-listener`, is *designed* rather than shipped, so this is a key
   whose feature was never built. Stage 1 marks it.
3. **`[metrics] listen`, `[metrics] endpoint` and `[trace] endpoint` name addresses nothing binds or
   pushes to.** `crates/nvs-server/src/metrics.rs` says it in its own module doc — *Nothing scrapes or
   pushes this* — and `rule:observability/the-exporters-are-crates` owns the missing half. The
   registry those three would export is built and bounded; only the exporter is absent. Stage 1 marks
   them.
4. **`capabilities.debug.trace` and `debug.profile` are grantable and never asked.** `Cap::DebugTrace`
   and `Cap::DebugProfile` exist in `crates/nvs-config/src/capability.rs` with names, rows and grant
   lookups; `Core\Debug` registers `dump` and `render` and nothing else, so no door ever asks either
   one. Their `[[app]]` and `[[schedule]]` mirrors are the same key. Stage 1 marks them.
5. **`[[extension]] path` and `sha256` are folded into the artifact-cache key and never acted on.**
   `nvs_config::cache::env_hash` reads both so a unit compiled under one extension set is not read
   under another — correct, and the whole of what happens: nothing loads a binary and nothing verifies
   a digest. M9 owns it. Stage 1 marks them.

**That list is what one hand-audit found, not the answer.** Stage 0 exists because a hand-audit is
wrong the week after it is written.

## Stage 0 — the roster, mechanically

`tools/directives.py`, on `tools/lints.py`'s shape: derive, do not copy.

1. **The roster comes from `crates/nvs-config/src/tree.rs`** — the one home for what parses. Walk
   `Config`'s field graph to its leaves, carrying the dotted key. A map block (`[db.<name>]`,
   `[mail.<name>]`, `[storage.<name>]`) contributes its keys under a `<name>` segment.
2. **A reader is either spelling.** A field touched outside `tree.rs`, or the dotted key present as a
   string literal — `Core\Storage`'s root is read as `config.get("storage.{disk}.root")` and a
   field-access search alone calls it dead. Both halves are load-bearing; say so in the tool's own doc.
3. **An unread key carries its reason in its own doc comment**, as a trailer on the model of a
   playbook bullet's `[until:]`:

   ```rust
   /// `[metrics] listen` — the address the Prometheus scrape is served at.
   ///
   /// [unread: no exporter is built, so nothing binds this; the registry it would
   /// export is `crates/nvs-server/src/metrics.rs`. owner: rule:observability/the-exporters-are-crates]
   pub listen: Option<String>,
   ```

   The field's doc comment is the home because the field is the key. Nothing is duplicated into a
   manifest a second edit has to remember.
4. **`--check` fails both ways.** A key with no reader and no `[unread:]` trailer is a key that landed
   silently. A key with a trailer *and* a reader is a stale marker — which is the failure that matters
   most, because it is the one that makes the generated file lie about a feature that now works.
5. **It gates in `nv verify`**, beside the `lints` and `reference` steps and for their reason:
   it decides what the compile steps are allowed to mean.

## Stage 1 — implement or mark, and nothing in between

Run stage 0's tool, take its roster, and give every unread key one of two answers.

**Implement it** where the missing piece is wiring rather than a subsystem. `[cache] dir` is the clear
one and it is not a missing feature at all — it is a second spelling. Decide which name survives and
say so in the record: either `cache.dir` becomes the Novis spelling that `from_config` reads first
with `opcache.file_cache_dir` kept as the PHP-shaped alias, or `cache.dir` leaves the tree and writing
it becomes `E0604`. **Do not leave both parsed and one inert.** A key removed from a
`deny_unknown_fields` struct turns a silently-accepted file into a refused one, so whichever way it
goes it is a decision with a migration note, not a cleanup.

**Mark it** where the missing piece is a subsystem: the exporters, the `Core\Debug` probe members, the
extension loader, the Unix listener. The trailer names *what* is missing and *who owns it* — a rule id
or a milestone — and that text is what stage 2 prints into the file, so write it for an operator
rather than for a maintainer.

Every claim in a trailer carries a `file:line` or the words *not checked*, per
[grounding.md](../grounding.md). A trailer that says "no exporter is built" and is wrong is worse than
no trailer.

## Stage 2 — the default file, generated and gated

`crates/nvs-config/src/default.toml`, `include_str!`'d into the crate and reachable as
`nvs_config::default_file()`.

1. **Every key is present and commented out.** The file's effective content is empty, so a deployment
   that takes it still runs on the shipped defaults — and a default this project later tightens for a
   security reason still reaches every host that ran the binary once. That direction is the priority
   ordering's first item over its fourth, and it is the whole argument for the shape: a file of live
   values would freeze today's numbers into every deployment that ever started.
2. **Each key's comment states what it does and what the default is**, in that order, as short as it
   can be and no shorter. Verbose where a wrong value is a security question (`[capabilities]`,
   `[limits.hard]`, `[mode] ceiling`); one line where it is not. `docs/agent/doc-style.md` governs the
   prose.

   **`cpu_time` and `wall_time` are the two keys an operator asks about first**, and their comments
   answer the questions before they are asked: waiting costs no CPU time, the budget is the request
   tree's, the check runs about twice a second, `cpu_time` equal to `wall_time` never fires, PHP's
   `max_execution_time` is `cpu_time` on Linux, and a heavy report raises its own limit instead of
   the default being raised for everyone. The meaning is `docs/reference/tools/20-config.md` §
   *`[limits]` and `[limits.hard]`*; the comment is its short form:

   ```toml
   [limits]
   # cpu_time — processor time one request may spend computing: its thread's own
   # user + kernel time, shared by every isolate and task the request spawns.
   # Waiting on a database, a socket or a sleep costs none of it, so this stops a
   # runaway loop without touching a slow query. Checked every ~0.5s, so a request
   # may overrun by up to that much. Keep it below wall_time: a request cannot
   # compute longer than it runs, so an equal value never fires.
   # PHP: what max_execution_time measures on Linux. Default: 5s.
   #cpu_time = "5s"

   # wall_time — how long one request may run, start to finish, waiting included:
   # how long a client can be kept waiting.
   # PHP: PHP-FPM's request_terminate_timeout. Default: 30s.
   #wall_time = "30s"

   [limits.hard]
   # The most a request may raise itself to with Core\Config::set. A report that
   # needs 40s of CPU raises its own cpu_time; don't raise the default for everyone.
   #cpu_time = "60s"
   #wall_time = "300s"
   ```

   **The `Default:` lines are true only once an unset key falls back to them.** Today an unset
   `cpu_time` is no limit at all (`crates/nvs-runtime/tests/configured_limits.rs`'s
   `an_unstated_uncapped_or_malformed_cpu_time_is_no_ceiling`), which
   `rule:config/no-configuration-file-is-a-complete-configuration` says it must not be. This stage
   either lands that fallback first or prints what is actually true.
3. **An unread key prints its trailer**, under a `# NOT IMPLEMENTED` line naming the missing piece and
   its owner. An operator reading the file learns that writing the key does nothing *before* they
   write it, which is the whole point of emitting it rather than hiding it.
4. **A map block appears once, as a commented example** — `[db.main]`, `[mail.default]`,
   `[storage.local]` — because there is no key roster for a name an operator has not chosen yet.
5. **`tools/directives.py --check` gates the file against the tree**: every leaf key appears exactly
   once, every `[unread:]` key carries its `# NOT IMPLEMENTED` note, and no key appears that the tree
   does not parse. This is what stops the file drifting the way a hand-written sample always does.

## Stage 3 — a project command writes it

The lookup does not change: `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
step 2 is still `./nvs.toml` in the working directory and still never a walk upward. What changes is
that a project command reaching step 3 **writes step 2's file first**, then resolves normally — so the
file it creates is the file it will next read, in the one directory the lookup already looks in.

1. **Only project commands.** `run`, `serve`, `test`, `build`, `check` — the commands that resolve a
   tree in order to execute something. `config check`, `config dump`, `info`, `meta`, `lsp`,
   `lsp-test` and `service` never write: an audit that creates the file it is auditing reports on its
   own output, and an LSP that writes into every folder an editor opens is a defect. The list is one
   table in `crates/nvs-cli/src/main.rs` with that sentence above it, not a flag threaded through
   every arm.
2. **Any `--config` disables it entirely**, on step 1's own precedent: an operator who named files
   never gets a surprise write in the working directory, even if every named file was missing.
3. **`--no-init`, and `NOVIS_NO_INIT` in the environment**, because a container image built by running
   the binary once should not bake a config file nobody wrote.
4. **A directory the ownership check refuses is not written to.** `nvs_config::trust::check` on the
   working directory first: creating a configuration file in a directory another local account can
   write manufactures exactly the surface `rule:config/ownership-is-the-trust-boundary` exists to
   close, and `nvs serve` would then read it at the next boot. Refusing to write is the fail-closed
   direction and costs an operator one `nvs init`.
5. **A failure to write is not an error anyone hears about**, on the artifact cache's discipline
   (`crates/nvs-cli/src/cache.rs` § 4): a read-only working directory, a full disk or a race with a
   concurrent `nvs` leaves the run taking the shipped defaults, which is what it did before this stage
   existed. One `Info` record, never a diagnostic, and never a non-zero exit.
6. **`nvs init` is the explicit door** — writes the same file, refuses rather than overwriting an
   existing one, and is what every refusal above points the operator at.
7. **The boot line changes.** `rule:config/no-configuration-file-is-a-complete-configuration`'s one
   line said *running on the shipped defaults*; where this stage wrote the file it says so and names
   the path, and where it declined it says which of the reasons above applied.

## Stage 4 — the records and the rulebook

1. **A decision record** — the next free number — carrying stage 2's commented-versus-live argument,
   stage 3's command split and its four refusals, and stage 1's `cache.dir` decision with its
   migration note.
2. **`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` gains step 3's write.**
   Its step 2 and its "never a walk upward" are unchanged and must stay unchanged — this goal adds
   what happens *at* step 3, it does not add a lookup.
3. **`rule:config/no-configuration-file-is-a-complete-configuration` stays true and gains one
   sentence**: the shipped defaults remain a complete configuration, and are what a run uses whenever
   the file could not be written.
4. **One new fragment** for stage 0's gate — every key the tree parses is read, or says in its own
   doc comment what is missing and who owns it — because that is a rule about this repository's
   surface that outlives the goal.
5. **`nvs.toml` at the repository root still carries ADR references** (`ADR 0103 § 1 step 2`,
   `ADR 0118 § 2`) from before the docs migration re-pointed everything at `rule:` tokens. Stage 4 is
   where they become rule ids, since this is the goal that touches that file's subject.

## Standing decisions

- **Every key in the generated file is commented out, and this is not a placeholder for live
  values.** A file of live defaults pins today's numbers into every deployment that ever ran the
  binary, so a default this project later tightens for a security reason reaches nobody — priority 1
  losing to priority 4, which the ordering does not permit. The operator inspects the default by
  reading it and overrides it by uncommenting. Closed; do not reopen it on the grounds that an
  active file is more useful.
- **Only project commands write, and the list is a table rather than a flag.** `run`, `serve`,
  `test`, `build`, `check`. An audit that creates the file it audits reports on its own output, and
  an `nvs lsp` that writes into every folder an editor opens is a defect, not a convenience. If a
  sixth command wants the behaviour, it joins the table with a sentence.
- **The lookup does not change.** Step 2 stays `./nvs.toml` in the working directory and stays
  *never a walk upward*; step 1 stays a hard refusal for a named file that is missing. This goal
  adds what happens at step 3 and adds no second implicit path — not beside the binary, not a
  platform directory, not an ancestor. Two implicit lookups are worse than one, and that argument is
  already in the rule.
- **An unimplemented key is emitted and marked, never hidden and never removed from the parser.**
  Hiding it leaves an operator who read the rulebook writing a key that is silently accepted;
  removing it turns a file that parses today into `E0604` tomorrow, which is a migration this goal
  is not buying except for `cache.dir`, where the whole point is that there are two spellings of one
  thing.
- **A failure to write is silent and the run continues.** The artifact cache's discipline
  (`crates/nvs-cli/src/cache.rs` § 4), for its reason: a read-only working directory is a
  configuration this repository supports, and a run that refused to start because it could not write
  a file nobody asked for would be a regression against every deployment that works today.
- **Refusing to write beats writing into a directory that fails the ownership check.** The
  alternative — write it anyway and let `nvs serve` refuse it at the next boot — creates the file an
  attacker wanted in the one directory where they can edit it. `nvs init` is the operator's answer
  and it is one command.
- **Stage 0's tool counts a dotted-key string literal as a reader.** Both spellings are load-bearing:
  `Core\Storage` reads `config.get("storage.{disk}.root")` and eight live keys look dead to a search
  for field access alone. A later session that finds the second half redundant and deletes it turns
  this gate into a generator of false gaps.
