---
looper: 1
title: The configuration an example runs with is on the page that shows it
summary: "Reached when every Core, type and language page shows the nvs.toml its examples run with, no block of the root nvs.toml configures or names anything under docs/examples/ or website/snippets/, a test fails when one does, and every example still prints its .out."
context: [website/src/lib/feature.ts, website/src/components/FeatureExamples.astro, docs/examples/README.md, tools/nv/proofs/run.ts, nvs.toml]
stages:
  - id: 1
    title: "Core, type and language pages show the nvs.toml their examples run with"
    status: done
    context: ["website/src/pages/reference/[area]/[...slug].astro", "website/src/pages/reference/core/[...slug].astro", tools/nv/renderers/lang-chapters.ts]
  - id: 2
    title: "the Db examples' configuration moves into their own directories: connections, transactions and plans"
    status: done
    context: [docs/examples/core/Db-Row/instant/nvs.toml, docs/examples/core/Queue/push/nvs.toml]
  - id: 3
    title: "the Db examples' configuration moves into their own directories: rows, schemas and Db errors"
    status: done
    context: [docs/examples/core/Db-Row/instant/nvs.toml]
  - id: 4
    title: "the Ldap examples' configuration moves into their own directories"
    status: open
    context: [docs/examples/core/Db-Row/instant/nvs.toml]
  - id: 5
    title: "the Http, Mail, Cap and configuration examples carry their own nvs.toml"
    status: open
    context: [docs/examples/core/Net/connect/nvs.toml]
  - id: 6
    title: "a test fails when the root nvs.toml configures an example, and the rule is written"
    status: open
    context: [docs/rules/testing/examples-live-in-the-repository.md, tools/nv/cmd/site.ts]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: root-config-names-no-example
    stage: 6
    name: the root nvs.toml configures and names nothing under docs/examples/ or website/snippets/
    run: [bun, test, tools/nv/test/root-config-names-no-example.test.ts]
    timeout: 600
---

# The configuration an example runs with is on the page that shows it

## Why

The user decided on 2026-11-06 that a reader must see the configuration an example runs with.
`docs/examples/README.md` § *A grant the examples need* says a feature's grant lives in its own
directory's `nvs.toml` and "the reader sees it beside the examples". Two things make that untrue.

- **The Core pages never show the file.** 62 directories under `docs/examples/core/` have their own
  `nvs.toml`, but `configFor` (`website/src/lib/feature.ts:146`) reads only the files its glob loads,
  and the glob (`website/src/lib/feature.ts:39`) loads only `docs/examples/{config,tools}/**/nvs.toml`.
  The language examples with a file (`lang/attributes`, `lang/concurrency`, `lang/errors`) are not
  shown either.
- **The root `nvs.toml` configures 200 examples.** 398 `[[app]]` blocks are in it, and 200 of them
  name a path under `docs/examples/` with `root` or `entry`, in 84 directories. Most are `Db-*` and
  `Ldap-*`. One case: the block `root = "docs/examples/core/Http/allowUrl"` (`nvs.toml:85`) writes
  `internal = []`, so `docs/examples/core/Http/allowUrl/02-urls-that-are-refused.out` expects
  `127.0.0.1` to be refused. A reader who copies the example runs it as a CLI run, where loopback is
  allowed by default, and sees it allowed.
- **The root file is also read silently.** Its comments name config examples that print a value the
  root writes: `[limits]`'s `fatal_reserve_memory` and its four siblings, `[log]`'s
  `handler_reserve_memory` and its sibling, `[cache.memory]`, `[debug] keep_temporary`, `[deferred]`,
  `[http.client]` and the `[[app]] root = "."` block itself (`docs/examples/config/app/`). Those pages
  show no file, so the value the reader sees printed comes from nowhere they can see.

What the user decided: (a) Core method pages show each example's `nvs.toml`, (b) the root file's
example blocks move into each example's own directory, (c) a check fails when the root `nvs.toml`
configures anything under `docs/examples/`, so it cannot come back.

## Standing decisions

1. **One place renders the file: `FeatureExamples.astro`.** It shows the feature's `nvs.toml` once,
   above the first example, titled `nvs.toml`, with one plain sentence before it that says the
   examples below run with this file. Every page that shows examples goes through it: the Core member
   page, the type, configuration and tool pages (`[area]/[...slug].astro`) and `FeatureSection` on the
   language pages. The `[area]` page's own two `<Code … title="nvs.toml" />` renders
   (`website/src/pages/reference/[area]/[...slug].astro:92`, `:103`) are deleted, so a file is never
   shown twice. The glob covers every area: `docs/examples/{core,novis,types,config,tools,lang}/**/nvs.toml`.
   A feature with a file and no example still shows the file, so the callers' conditions read
   `configFor(f) || examplesFor(f).length > 0`.
2. **The whole file is shown, once.** A directory whose examples need different grants writes the
   shared part as top-level tables and an `[[app]] entry = "<file>"` block only for the example that
   differs. The entry path resolves against the file it is written in
   (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`). The website shows the
   file as it is on disk, so a reader who copies the directory copies the same thing. The language
   chapters keep their own per-example cut (`ownBlocks`, `tools/nv/renderers/lang-chapters.ts:72`),
   because they write one example to a folder.
3. **How a moved file is written.**
   - The shape is the one `docs/examples/core/Queue/push/nvs.toml` and
     `docs/examples/core/Db-Row/instant/nvs.toml` have: top-level `[capabilities.*]`, no `[[app]]`
     unless decision 2 needs one, and the connection block the examples open.
   - **The connection keeps the name the examples use** (`[db.notes]`, `[db.main]`, `[db.tight]`,
     `[db.schema]`, `[db.mssql]`, `[ldap.corp]`, `[ldap.corp-accounts]`, `[mail.shop]`), so no `.nvs`
     and no `.out` changes. A `tls_ca_file` is the relative path from the directory to
     `tests/db/ca.crt`, as `Db-Row/instant`'s is.
   - Examples of one directory whose root blocks grant the same thing get one file. The root writes a
     block per example file today (`entry = "docs/examples/core/Db-Row/bool/01-…"`), and those merge.
   - Each comment is text a user reads (AGENTS.md § *Text an end user reads*): one or two lines above a
     block, saying what it allows. `bun nv proofs --comments` does not read TOML, so the writer holds
     each comment to that section by hand.
   - **An `.out` that changes after a move means the file misses something.** The writer finds what
     the root gave the example (`[limits]`, `[http.client]`, `[queue]`, the `spawn` grant) and writes
     it, and never blesses the `.out` to hide the difference.
4. **What the root keeps.** A `[db.*]`, `[ldap.*]`, `[mail.*]` or other table the root writes stays
   while an attack, a bench, a test or `examples/` still reads it. A table that only the moved
   examples read is deleted. Every comment that names a path under `docs/examples/` is rewritten
   without it, because decision 5's test reads comments too.
5. **The check is a Bun test, `tools/nv/test/root-config-names-no-example.test.ts`.** It parses the
   root `nvs.toml` with `Bun.TOML.parse` and fails on every `[[app]]` whose `root` or `entry` lies under
   `docs/examples/` or `website/snippets/`. It also reads the raw text and fails on every line,
   comments included, that names a path under those two trees: a comment naming an example is how the
   root says a table is there for that example to print. Each failure names the line and says the
   configuration goes into the example's own directory. `bun nv verify` runs it, and so do CI and
   `bun nv ci --check`. It needs no new command.
6. **Which trees the check covers, verified on 2026-11-06.**
   - `docs/examples/`: yes. The website shows these programs and their output.
   - `website/snippets/`: yes. No root block names a snippet today, so the test only keeps it that way.
     A snippet's own `nvs.toml` is already read by `snippetRun` and `serveRun`
     (`tools/nv/cmd/site.ts:988`).
   - `benches/members/` (91 root blocks) and `tests/hostile/` (96 root blocks, including the
     `root = "tests/hostile"` block that narrows every attack's defaults): **no**. The website shows no
     bench and no attack source; it renders only their counts (`tools/nv/renderers/website-numbers.ts:57`)
     and links two attacks on GitHub. Moving them is in the Backlog.
7. **The new rule `testing/an-example-carries-its-own-configuration`** (stage 6): an example that
   needs configuration carries it in its own directory's `nvs.toml`, the website shows that file
   beside the examples, and the root `nvs.toml` configures and names nothing under `docs/examples/`
   or `website/snippets/`. Its reason is the reader who copies an example. Its Declined list holds
   decision 6's two trees with their reason, and running every example under the shipped defaults
   (Backlog). `docs/examples/README.md` § *A grant the examples need* cites it and keeps the mechanics.
8. **Runs.** A move stage's examples run once with `bun nv proofs --run --group '<class>'` for each
   class it moved, because a move cannot be finished without seeing every `.out` still hold. The Db
   examples need `postgres` and the Ldap examples `samba-ad` from `tests/db/compose.yaml`; the
   runner brings each up from the examples' `requires:` lines.
9. **Lanes.** Stage 1 touches only `website/src/`, so it may run as a side goal beside stages 2 to 5.
   Stages 2 to 5 all edit the root `nvs.toml`, so they run in order. Stage 6 needs 2 to 5. Another goal
   that adds an example needing configuration after stage 6 writes it into the example's directory.
10. **What this spends.** Nothing at run time. Each moved directory repeats the connection block it
    opens, about eight lines, and the website shows one more block per feature that has a file.

## Stages

### 1. Core, type and language pages show the nvs.toml their examples run with

**Does:** decision 1 and 2 on the website. No example changes.

- [x] The glob covers every area. `website/src/lib/feature.ts:39`
- [x] `FeatureExamples` shows the feature's file once, above the first example, with its sentence.
  `website/src/components/FeatureExamples.astro:1`
- [x] The `[area]` page's two `nvs.toml` renders are deleted, and its conditions read decision 1's.
  `website/src/pages/reference/[area]/[...slug].astro:89`
- [x] The Core member page's `examples` section is present when the member has a file.
  `website/src/pages/reference/core/[...slug].astro:64`
- [x] `bun nv site --check` passes, and the built pages of `Core\Queue::push`, `Core\Net::connect`
  and `docs/examples/config/ext` each show their file once. `website/src/lib/feature.ts:146`

### 2. the Db examples' configuration moves into their own directories: connections, transactions and plans

**Split:** an `Explore` subagent first lists, for the classes below, each example directory with the
root blocks that name it and the root tables those blocks open (`nvs.toml:line` each). Then writing
subagents, one per class, write the directories' files side by side; their directories are disjoint.
The session alone edits the root `nvs.toml` and runs the examples.

The directories: `Db` (1), `Db-Connection` (11), `Db-Transaction` (8), `Db-Write` (3), `Db-Column` (3),
`Db-Plan` (1) and `Db-Plan-Step` (4), all under `docs/examples/core/`.

- [x] The list. `nvs.toml:1584`
- [x] Each directory's `nvs.toml`, by decision 3. `docs/examples/core/Db-Connection/execute/nvs.toml:1`
- [x] Their blocks deleted from the root, and the root tables decision 4 frees. `nvs.toml:1584`
- [x] `bun nv proofs --run --group` over the seven classes, every `.out` unchanged.

### 3. the Db examples' configuration moves into their own directories: rows, schemas and Db errors

**Split:** as stage 2.

The directories: `Db-Row` (13), `Db-Rows` (6) and `Db-Schema` (4) under `docs/examples/core/`, and
`Db-DbError` and `Db-RolledBack` under `docs/examples/types/`.

- [x] The list. `nvs.toml:1584`
- [x] Each directory's `nvs.toml`, by decision 3. `docs/examples/core/Db-Row/bool/nvs.toml:1`
- [x] Their blocks deleted from the root, and the root tables decision 4 frees. `nvs.toml:1584`
- [x] `bun nv proofs --run --group` over the five classes, every `.out` unchanged.

### 4. the Ldap examples' configuration moves into their own directories

**Split:** as stage 2.

The directories: `Ldap` (2), `Ldap-Connection` (8), `Ldap-Entry` (2), `Ldap-Entries`, `Ldap-Changes`,
`Ldap-Ad-AccountFlags` and `Ldap-Ad-GroupType` under `docs/examples/core/`, and `Ldap-Connection-Scope`,
`Ldap-Entry-AccountType`, `Ldap-LdapError`, `Ldap-LdapError-Kind` and `Ldap-Tls` under
`docs/examples/types/`.

- [ ] The list. `nvs.toml:1`
- [ ] Each directory's `nvs.toml`, by decision 3; `[ldap.corp]`'s password is the test directory's,
  as the root writes it. `docs/examples/core/Ldap-Connection/search/nvs.toml:1`
- [ ] Their blocks deleted from the root, and the root tables decision 4 frees. `nvs.toml:2772`
- [ ] `bun nv proofs --run --group` over the twelve classes, every `.out` unchanged.

### 5. the Http, Mail, Cap and configuration examples carry their own nvs.toml

**Split:** an `Explore` subagent lists every `.out` under `docs/examples/config/` that prints a value
the root `nvs.toml` writes (`256M`, `512M`, `1M`, `50ms`, `64M`, `1000`, `16M`, `5s`, `32M`, `256`,
`16`, `30s`, `false`, `https://example.test`), and every directory a root comment names. Writing
subagents then write the files side by side, one for the Core directories and one for the
configuration ones.

- [ ] The Core directories: `Http/allowUrl`; the nine member directories of `Http-Client` and the
  two of `Http-Part`, which one `root` block each configures today (`nvs.toml:108`, `:133`);
  `Mail/send` and `Cap/has`. A granted path becomes relative to the file, so `Http-Part/file`'s
  `fs.read` is `["data"]`. `docs/examples/core/Http/allowUrl/nvs.toml:1`
- [ ] `docs/examples/config/app/` and `docs/examples/config/capabilities/`, whose root blocks
  (`nvs.toml:21`, `:359`) move with the value each prints. `docs/examples/config/app/nvs.toml:1`
- [ ] Each configuration directory the list names, with the table it prints:
  `limits-fatal_reserve_memory` and its four siblings, `log-handler_reserve_memory` and its sibling,
  `cache-memory`, `debug-keep_temporary`, `deferred-deadline`, `deferred-max_concurrent`,
  `http-client-pool_idle`, `http-client-socket` and `http-client-tls`.
  `docs/examples/config/limits-fatal_reserve_memory/nvs.toml:1`
- [ ] The root's blocks and comments for them deleted or rewritten, by decision 4. `nvs.toml:1`
- [ ] `bun nv proofs --run --group` over the classes and configuration groups moved, every `.out`
  unchanged.

### 6. a test fails when the root nvs.toml configures an example, and the rule is written

- [ ] The test of decision 5, with a fixture string for each kind of failure and one that passes.
  `tools/nv/test/root-config-names-no-example.test.ts:1`
- [ ] The new rule `testing/an-example-carries-its-own-configuration`, and its chapter rendered.
  `docs/rules/testing/an-example-carries-its-own-configuration.md:1`
- [ ] `docs/examples/README.md` § *A grant the examples need* cites the rule and says the website
  shows the file on every page. `docs/examples/README.md:248`
- [ ] The root `nvs.toml`'s header says what the file configures now. `nvs.toml:1`
- [ ] The runner's doc comment says what is true now. `tools/nv/proofs/run.ts:588`

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 3 are done. Every Db example directory now has its own `nvs.toml`: 31 in stage 2 and
25 in stage 3, `Db-Row/instant` already had one. The root lost 165 `[[app]]` blocks. `[db.notes]` and
`[db.schema]` stay, because attacks, benches and `examples/schema.nvs` open them. Every group ran
green with each `.out` unchanged. The two types have no group, so they run with
`bun nv proofs --run --only 'Core\Db\DbError' 'Core\Db\RolledBack'`.

Two things the next stages can reuse:

- A relative `[[app]] entry` in a directory's file matches its example. Proved by hand:
  `Db-Plan-Step/grade/02-…` throws `needs the capability db.schema` without its block and passes
  with it. `Db-Plan/steps`, `Db-Plan-Step/{grade,isRefused}`, `Db-Schema/fromArray` and
  `types/Db-RolledBack` use that form, so an example that needs no grant gets none (decision 2).
- The blocks were near-identical, so a Bun script under `.agent-tmp/` listed them with
  `Bun.TOML.parse` and deleted them line by line. That was quicker than the split's Explore list and
  per-class subagents. Each group comment left above an attack or bench block was then rewritten by
  hand, so it no longer names an example.

The sweep's `env-secret-proofs` red and the three nightly Miri failures are stale. No such check or
log exists, and that nightly ran on 26dda36627, before f13f40646a.

No test is expected to go red. 50 lines of the root still name `docs/examples/`.

### Next

**Stage 4**, the Ldap examples. They differ from the Db ones in three ways. Some root blocks are
`root = "docs/examples/core/Ldap-…"`, not `entry`. Two grant names are in use (`corp`,
`corp-accounts`). Some examples print a refusal (`Ldap/connect/02-a-directory-the-operator-did-not-allow`).
Most need `samba-ad`. Decision 8's runner starts it.

- [ ] The list, by directory, with grant and `root`/`entry`. `nvs.toml:557`
- [ ] Each directory's `nvs.toml`, with `[ldap.corp]` and `[ldap.corp-accounts]` copied as the root writes them. `docs/examples/core/Ldap-Connection/search/nvs.toml:1`
- [ ] Their blocks deleted from the root, and each comment above what is left rewritten. `nvs.toml:557`
- [ ] The twelve groups run, types by `--only`, every `.out` unchanged. `tools/nv/proofs/run.ts:618`

### Backlog

- The 91 root blocks for `benches/members/` and the 96 for `tests/hostile/` (decision 6). Both trees
  already read a directory's own `nvs.toml`, so a move would follow this goal's stages.
- Running every example that has no `nvs.toml` under the shipped defaults, and not under the root
  file. It would also catch an example that depends on a root table with no comment naming it. It
  re-checks every `.out` in the tree, and a snippet served under the root file needs the root's
  `[queue]` on PostgreSQL (`tools/nv/cmd/site.ts:1221`).
- A snippet's own `nvs.toml` is copied by hand onto its page (`website/src/content/docs/guides/how-to-use/limits.mdx:55`).
  `<Snippet>` could show it as `FeatureExamples` does; goal `website-code-checked` checks those
  copies meanwhile.
