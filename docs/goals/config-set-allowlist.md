---
looper: 1
title: "Core\\Config::set changes only the keys on one allowlist, and every page and the compiler say which"
summary: "Reached when one SETTABLE table in nvs-config is the only thing that lets Core\\Config::set change a key, the class field and the refuse list are gone, a literal name set can never change is a compile error and a template no listed key fits is a warning, and the set card, about.md, each reference page and the editor list the settable keys."
context: [crates/nvs-config/src/directive.rs, crates/nvs-config/src/request.rs, docs/rules/config/three-changeability-classes.md]
stages:
  - id: 1
    title: "one allowlist decides what set changes"
    status: done
    context: [crates/nvs-config/tests/request.rs, crates/nvs-config/tests/directives.rs, crates/nvs-cli/tests/request_http_settings.rs, crates/nvs-config/src/default.toml]
  - id: 2
    title: "the compiler names a set that can never change anything"
    status: done
    context: [crates/nvs-types/src/capability.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-diagnostics/src/code/config.rs, crates/nvs-diagnostics/src/code/warnings.rs]
  - id: 3
    title: "the editor offers the settable keys"
    status: done
    context: [crates/nvs-lsp/src/arguments.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/tests/completion_file_values.rs]
  - id: 4
    title: "the reference card, the examples, the rule and the website list the keys"
    status: open
    context: [crates/nvs-stdlib/src/config.rs, docs/examples/core/Config/set/about.md, crates/nvs-cli/src/meta.rs, tools/nv/renderers/website-reference.ts, website/src/lib/reference.ts, docs/agent/prose.md, website/src/data/changes.json]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: allowlist
    stage: 1
    name: "set accepts a key exactly when the allowlist lists it, walked over every key the default file writes"
    run: [cargo, test, --test, request, set_accepts_only_the_keys_on_the_allowlist]
    timeout: 1800
    tests:
      - set_accepts_only_the_keys_on_the_allowlist
  - id: effect
    stage: 1
    name: "every listed key changes what the request that set it does"
    run: [cargo, test, --test, request_http_settings, every_settable_key_changes_what_a_request_does]
    timeout: 1800
    tests:
      - every_settable_key_changes_what_a_request_does
  - id: compiler
    stage: 2
    name: "a literal name set cannot change does not compile, and a template no listed key fits warns"
    run: [cargo, test, --lib, config_set_names]
    timeout: 1800
    tests:
      - a_literal_name_set_cannot_change_does_not_compile
      - a_template_no_settable_key_fits_warns
      - a_plain_variable_name_gets_no_diagnostic
  - id: compiler-conformance
    stage: 2
    name: "a program sees the compile error for a literal name that cannot be set"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/config-set-a-name-that-cannot-be-set-does-not-compile.nvst]
    timeout: 1800
  - id: editor
    stage: 3
    name: "the first argument of set and restore offers the settable keys and nothing else"
    run: [cargo, test, --test, completion_file_values, set_offers_only_the_settable_keys]
    timeout: 1800
    tests:
      - set_offers_only_the_settable_keys
  - id: proofs
    stage: 4
    name: "every feature this goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, config-set-allowlist]
    timeout: 1800
  - id: close
    stage: 5
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, config-set-allowlist]
    timeout: 900
---

# Core\Config::set changes only the keys on one allowlist, and every page and the compiler say which

## Why

Feedback from a user on 2026-11-23: it is not clear what `Core\Config::set` can set. Nothing tells
them.

- The `set` reference card (`crates/nvs-stdlib/src/config.rs:217-240`) and its `about.md`
  (`docs/examples/core/Config/set/about.md`) list only what `set` refuses. No list of what it accepts
  exists anywhere.
- The code decides by refusal, in three layers. A class per block row
  (`crates/nvs-config/src/directive.rs:87-333`). A refuse list of four `[http]` keys,
  `UNREAD_BY_A_REQUEST` (`crates/nvs-config/src/request.rs:81`). Per-kind checks in `set_checked`
  (`request.rs:367-432`), where a `RuntimeTighten` row that is not a quantity is refused whole
  (`request.rs:401-415`).
- The website's "who may change this" line comes from the row's class (`website/src/lib/reference.ts:36`),
  and four pages state wrong things. `[capabilities]` and `debug.mode` say a request may tighten them,
  and `set` always returns `false` for both. `debug.inline` and `debug.store` take only `false`. `[http]`
  says "up to the ceiling in `[limits.hard]`", which has no `[http]` keys, and four `[http]` keys are
  refused.

Counted on 2026-11-23 from the default file's leaf keys and the rows: **38 of 337 keys** are settable.
The user's call: the short list is the one to write down, as an allowlist. It also fails closed. A key
added to a `Runtime` block today is settable the moment it lands, and nobody has to notice. With an
allowlist, a new key is not settable until someone adds its row.

The 38: `mode.default`; `limits.memory`, `cpu_time`, `wall_time`, `max_output`, `request_body`,
`upload_total`, `max_multipart_parts`, `max_tasks`, `max_regex_steps`; `log.format`, `level`,
`access`; `errors.deprecated`; `deferred.deadline`; `debug.inline` and `debug.store` (`false` only);
`http.headers.*` (7 keys); `http.cors.origins`, `expose`, `credentials`; `http.cookies.*` (4 keys);
`http.client.connect_timeout`, `deadline`, `max_redirects`, `idle`, `max_duration`;
`http.client.socket.max_message`, `send_timeout`.

The user's calls on 2026-11-23: an allowlist; a literal name `set` can never change is a compile
error; a name built from a template is a warning only when no listed key can fit it; a plain variable
gets no diagnostic. The chain spot, right after `exact-invalidation`, is the user's too. Every other
decision below is mine.

## Standing decisions

- **`SETTABLE` is the one home of what a request may change.** A new module,
  `crates/nvs-config/src/settable.rs`, holds `pub const SETTABLE: &[Settable]`, one row per leaf key,
  `Settable { key: &'static str, how: How }`. `How` has two cases. `UpToCeiling`: any valid value up
  to the key's ceiling, which is `[limits.hard]` (or the start value) for a limit, `[mode] ceiling`
  for `mode.default`, and none for every other key. `OffOnly`: the value `false` and nothing else.
  `pub fn how(key) -> Option<How>` takes the canonical key, so a bare limit name (`memory`) finds
  `limits.memory`. Each row carries a one-line comment with its reason, moved from the row comments
  of `directive.rs`.
- **`set_checked` asks `settable::how` first**, and an unlisted key returns `false` before any other
  check. The value checks after it stay as they are: the unit, the ceiling, `stays_meaningful`, the
  mode flip.
- **The class field goes.** `Directive` keeps `key` and `apply` only (`rule:config/reloadability-is-its-own-field`
  is untouched). `Class`, `settable_by_a_request`, `UNREAD_BY_A_REQUEST` and `TIGHTEN_FLAGS` are deleted.
  `RuntimeTighten` disappears with them. `[capabilities]` and `debug.mode` were never settable in
  practice, so they are simply not listed. Every test assertion on `row.class` becomes an assertion on
  `settable::how(<leaf key>)`. A test that asserted a whole block was settable picks one leaf key of
  that block and says which in its doc.
- **The list is 37 keys, and each one proves its effect.** Stage 1's effect test sets each key in a
  request and checks that something the request does changes. A key with no effect is removed from
  `SETTABLE`. `limits.request_body` has an effect: the limit applies when the program reads the body.
- **`log.access` is not on the list.** The access line is written once the program has ended, from
  the snapshot's value (`crates/nvs-server/src/serve.rs:1994`, `crates/nvs-server/src/trace.rs:394`),
  so a request's own `set` changed nothing while it returned `true`.
  `docs/rules/http-server/the-access-log-is-a-mode-default.md:7` already says so. A program has a use
  for it (a health route turning its own line off), but no goal on the chain owns a gap record for
  it, so it is a backlog item, not a `data/gaps/` record. Making it work means reading the request's
  overlay at `serve.rs:1994`, where the context is still alive, and that is a new feature with its
  own goal.
- **A package is unchanged.** `set_as_package` still accepts only `limits.*` keys, lower only. Those
  keys must also be on the list.
- **The rule is renamed.** The old id `config/three-changeability-classes` becomes
  the fragment `docs/rules/config/set-changes-only-listed-keys.md`. Its text: `nvs.toml` gives what a request starts with,
  `SETTABLE` is the one list of keys a request may change, and each row says how. The Declined list
  keeps its two items and adds "a refuse list": a new key would be settable unless somebody noticed.
  Every citation of the old id moves to the new one.
- **The compiler checks the name of `set` and `restore`, never of `get`.** `get` reads every key, so
  `get("db.{$id}.driver")` stays valid. The check is a sibling of `reject_unknown_capability`
  (`crates/nvs-types/src/capability.rs:125`), called beside `prepare_config_set`
  (`crates/nvs-types/src/expr/calls.rs:819`):
  - A literal name (`literal_default` gives `ConstArg::Str`) that `settable::how` does not list is
    error `E0667` (`E_NOT_SETTABLE`, the next code after `E0666` in
    `crates/nvs-diagnostics/src/code/config.rs:952`; take the next free one if it is taken). In a
    package's namespace (`authority_of`, `capability.rs:304`) a literal key outside `limits.` is the
    same error with the package reason. The help line names the settable keys by block.
  - A template string whose fixed parts no listed key can fit is warning `W1037`
    (`W_SET_NEVER_FITS`, after `W1035` in `crates/nvs-diagnostics/src/code/warnings.rs:375`).
    `whole-grapheme-cuts` reserves `W1036`, so this goal skips it. "Fit" is a glob match: each
    `{...}` part matches one or more characters that are not a dot.
  - A plain variable, a call, a concatenation and a spread get no diagnostic. The checker cannot know
    the value, and the author cannot fix it.
  - *What it spends:* one lookup in 37 rows per `set` or `restore` call site while compiling, and
    nothing at run time.
- **The editor offers the 37 keys at the first argument of `set` and `restore`**, the way it offers zone
  names (`names_a_zone`, `crates/nvs-lsp/src/arguments.rs:362`). Hover on a written key says how it may
  change. `get` offers nothing new: its 337 keys are the reference's job.
- **The binary prints the list.** `nvs meta` gains a `"settable"` roster of `{ key, how }`
  (`crates/nvs-cli/src/meta.rs:241`), and `directive_json` (`meta.rs:369`) drops `class`. The website
  renderer (`tools/nv/renderers/website-reference.ts:77-97`) reads it per key, and `CHANGED_BY`
  (`website/src/lib/reference.ts:36`) gives one of three lines: only `nvs.toml` sets it; a request may
  change it, up to the ceiling where it has one; a request may only set it to `false`. A block page
  whose keys differ lists which keys a request may change.
- **The `set` page lists all the keys, one line per block.** `docs/examples/core/Config/set/about.md`
  writes `` - `[block]`: `leaf`, `leaf` `` for each block, and says in prose which keys have a
  ceiling and which take only `false`. A table of 37 rows was 551 words, and an `about.md` is
  bounded at 200. The reference card (`config.rs`) names the blocks in one sentence and says the
  editor offers each key and the `set` page lists them; there is no `nvs help` for a member.
  `the_set_page_and_card_name_every_settable_key` (`config.rs`) fails when the page's lines name
  another set of keys than `SETTABLE`, or the card misses a block.
- **Example `02-a-change-that-is-not-allowed.nvs` keeps its names in variables**, and its comment says
  a name written in the call does not compile. That comment is the compile error's example: the
  page already has three examples, and a fourth that cannot compile would show nothing.
- **The shipped file says who may change each key in words.** `default.toml`'s comments are each
  directive's help card, and every `Runtime` and `System` in them became "A request may change this
  with `Core\Config::set`" or "Only `nvs.toml` sets this".
- **Every directive feature gets one `changed` entry.** Removing the class moved every directive's
  tests, and each directive page's line about `set` changed, so one entry names all 45.
- **Out of scope:** `nvs config dump` marking the settable keys (backlog), and any change to which keys
  are settable beyond dropping one the effect test shows does nothing.
- **Trade-offs, for the change entries and the rule.** Security and request isolation: better, a key
  is refused unless listed. Latency: `set` is not on the request path, and the lookup costs what the
  row lookup costs today. Simplicity: three classes, a refuse list and two flag lists become one list.
  Memory: none. Breaking: a literal name that could never be set now fails to compile.
- **Lanes:** stage 1 runs first. Stages 2 and 3 touch disjoint crates and may run side by side once
  `settable::how` exists. Stage 4 follows all three.

## Stages

### 1. one allowlist decides what set changes

**Does:** adds `SETTABLE`, makes `set` read it first, deletes the class field and the refuse lists,
rewrites the tests that read the class, and proves each listed key has an effect.

**Split:** the rewrite of every `row.class` assertion in `crates/nvs-config/tests/directives.rs`
(about 50) goes to one writing subagent once `settable.rs` builds, with this stage's decisions in its
brief. The session writes the module, `request.rs` and the two new tests.

- [x] `crates/nvs-config/src/settable.rs:1`: `SETTABLE`, `Settable`, `How` and `how`, with the 38 rows
  of *Why* and each row's reason moved from `directive.rs`. Export from `crates/nvs-config/src/lib.rs`.
- [x] `crates/nvs-config/src/request.rs:367-432` `set_checked`: `settable::how` first. Delete
  `UNREAD_BY_A_REQUEST` (`:81`), `TIGHTEN_FLAGS` (`:90`) and the `Class` match; `OffOnly` takes the old
  flag path. Rewrite the module doc's refusal paragraph (`:17-41`) as "a key `SETTABLE` does not list,
  then the value checks".
- [x] `crates/nvs-config/src/directive.rs:24-48`: delete `Class` and `settable_by_a_request`; drop
  `class` from every row (`:91-332`). The module doc says who may set a key is `settable.rs`'s.
- [x] `crates/nvs-config/tests/request.rs:161`, `:188`, `:413`, `:559`, `:677`, `:762`,
  `crates/nvs-config/tests/snapshot.rs:1145`, `crates/nvs-host/tests/limits.rs:1538` and
  `crates/nvs-cli/tests/request_http_settings.rs:239-247`: read `settable::how`.
- [x] `crates/nvs-config/tests/request.rs`: `set_accepts_only_the_keys_on_the_allowlist` walks
  `request::keys()` and checks `set` with a valid value is `true` only for a listed key. Run the check
  `allowlist` once.
- [x] `crates/nvs-cli/tests/request_http_settings.rs`: `every_settable_key_changes_what_a_request_does`,
  one probe per listed key. Drop any key that shows no effect, per *Standing decisions*. Run the check
  `effect` once.

### 2. the compiler names a set that can never change anything

**Does:** adds `E0667` and `W1037`, and the checker hook that raises them.

**Split:** none, one session.

- [x] `crates/nvs-diagnostics/src/code/config.rs:952`: `E_NOT_SETTABLE` after `E_BAD_RESTRICTION_REPLACE`,
  with a card in user words.
- [x] `crates/nvs-diagnostics/src/code/warnings.rs:375`: `W_SET_NEVER_FITS = Code::new("W1037")` with a
  card in user words.
- [x] `crates/nvs-types/src/capability.rs:178`, beside `prepare_config_set`: `reject_unsettable_name`
  for `set` and `restore`, per *Standing decisions*, and its call at
  `crates/nvs-types/src/expr/calls.rs:819`.
- [x] Its test module, `config_set_names`: the three tests the check `compiler` names, plus a package
  literal outside `limits.`. Run the check `compiler` once.
- [x] `tests/conformance/core/config-set-a-name-that-cannot-be-set-does-not-compile.nvst:1`. Run the
  check `compiler-conformance` once.
- [x] Every `.nvs` in the tree that the new error stops (`bun nv check` names them) builds its name at
  run time or drops the call.

### 3. the editor offers the settable keys

**Does:** completes and hovers the first argument of `set` and `restore`.

**Split:** none, one session.

- [x] `crates/nvs-lsp/src/arguments.rs:362`, beside `names_a_zone`: `names_a_settable_key` for the first
  argument of `Core\Config::set` and `restore`.
- [x] `crates/nvs-lsp/src/completion.rs` and `crates/nvs-lsp/src/hover.rs`, at the zone arms: offer the
  listed keys, and hover a written key with how it may change.
- [x] `crates/nvs-lsp/tests/completion_file_values.rs`: `set_offers_only_the_settable_keys`, and that
  `get` offers none. Run the check `editor` once.

### 4. the reference card, the examples, the rule and the website list the keys

**Does:** writes the list where a user looks, renames the rule, and pays the proofs.

**Split:** the citation sweep of the old rule id (45 files, `bun nv brief --where three-changeability`)
goes to one writing subagent once the fragment is renamed. The session writes the rest.

- [x] `docs/rules/config/three-changeability-classes.md` renamed to `set-changes-only-listed-keys.md`
  and rewritten per *Standing decisions*; `bun nv rules --render`. Cite it from `settable.rs`.
- [x] `crates/nvs-stdlib/src/config.rs:217-240` `SET_DOC` and `:243` `RESTORE_DOC`: what can be
  changed, in user words. The drift test beside them reads `about.md`.
- [x] `docs/examples/core/Config/set/about.md`: the table of keys by block. `02-a-change-that-is-not-allowed.nvs`
  per *Standing decisions*, and its `.out`.
- [x] `crates/nvs-cli/src/meta.rs:241`, `:369`: the `settable` roster, and no `class`.
  `tools/nv/renderers/website-reference.ts:77-97`, `tools/nv/test/render.test.ts:181`,
  `website/src/lib/reference.ts:36` and `website/src/pages/reference/[area]/[...slug].astro:66`: the
  per-key line. `bun nv render --website`.
- [x] `website/src/content/docs/guides/how-to-use/configuration.mdx:212-215` and
  `website/src/content/docs/guides/how-to-use/limits.mdx`: one sentence and a link to the `set` page.
- [x] `website/src/data/changes.json`: `changed`, `breaking: true` for `Core\Config::set` and
  `restore`, with an `instead`; `new` for `E0667` and `W1037`. Every page `bun nv site --check stale`
  names is reread and stamped.
- [ ] `bun nv proofs --owed --goal config-set-allowlist` names nothing.
  - [x] every proof but `reviewed` is paid
  - [ ] the newcomer review of the 86 features it names, and their stamps

### 5. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of `about.md`, the two diagnostic cards and the change entries
  ([prose.md](../agent/prose.md) § *A newcomer reads it*).
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 3 are done, and stage 4 but its newcomer review. `SETTABLE`
(`crates/nvs-config/src/settable.rs`) is the one allowlist; `E0667` and `W1037` are in
`crates/nvs-types/src/capability.rs`; the editor offers the keys (`crates/nvs-lsp/src/arguments.rs`).
The rule, the `set` page and cards, `nvs meta`'s `settable` roster, the website and `changes.json`
are written, and `site --check changes` and `stale` are green.

The review's first round read all 176 texts. Fixed: the unexplained "a name written in the call
does not compile" line in 90 examples and attacks, the cards and the pages (each now names
`Core\Config::set('<key>', ...)`); 45 help cards in `default.toml`; the `Core\Config` chapter,
which named the removed `System` class; the `deferred.deadline` page. No text is stamped `reviewed`.

The cancelled nightly run `45166887327` was cancelled by hand and needs nothing.

A block directive (`[log]`, `[http]`, `[session]`, ...) has no help card, so its "text" in the
brief is its `nvs meta` row as JSON (`tools/nv/lib/texts.ts:146`). There is no prose to fix there.

No test is expected to go red.

### Next

- [ ] `tests/hostile/core/Debug/startCoverage/01-branch-coverage-the-mode-does-not-allow.nvs:7`: the
  first round's open findings, then a second round. Fix these, one line each:
  - `startCoverage/01`: "Each step catches its error ..." is written twice (`:7`, `:18`).
  - `tests/hostile/config/deferred-max_concurrent/01-*.nvs`: the top says each call returns `false`, and
    step 3 says each try throws. Make them agree with the code.
  - `tests/hostile/tools/config/limits-and-limits-hard/01-*.nvs`: say where `48M` comes from.
  - `tests/hostile/config/trace/01-*.nvs`: "all four `[trace]` keys" names none of them.
  - `docs/examples/config/log-handler_reserve_time/01-*.nvs`: "These two reserves" names one.
  - `docs/examples/config/log-handler/01-*.nvs`: "This function" has no referent.
  - `docs/examples/config/capabilities/01-*.nvs`: "take a right away", "give a right back".
  - `docs/examples/config/http/about.md`: write out CORS once, and replace "answer".
  - `docs/examples/config/regex-backtracking/about.md`: "**The example below**", and define
    lookaround and backreference in a few words.
  - `docs/examples/config/queue-max_attempts/01-*.nvs`: define the dead-letter table.
- [ ] `docs/agent/prose.md:20`: the second round. Run `bun nv site --review`, batch the keys by 20 with
  `.agent-tmp/` and `xargs -d '\n' bun tools/nv/main.ts site --review --brief`, and send each brief to
  a `reader` (the prompt names the brief's file and says to read no other). The second round
  reads the new wording, so stamp only what it passes, with `bun nv site --review --stamp`. Also read
  `settable_sentence` (`crates/nvs-lsp/src/completion_files.rs:368`) and the `E0667` and `W1037`
  cards: they were not on the review list. Then `bun nv proofs --gate --goal config-set-allowlist`
  is green, and stage 4 is done.
- [ ] Stage 5: the stage's check, as the pack names it (`docs/goals/config-set-allowlist.md:288`).
  Stage 5's newcomer item is the review above.

### Backlog

- `nvs config dump` could mark each key a request may change.
- About 30 examples print `refused` when `set` returns `false` (for example
  `docs/examples/config/limits/01-what-a-request-starts-with-and-may-change.nvs`). A reader takes it
  for a word the prose rules forbid. `not allowed`, as the `set` example prints, needs each `.out`
  changed with it.
- `tools/nv/lib/texts.ts:146` gives a block directive with no help card its JSON row as its text,
  so a change to `nvs meta`'s row shape puts every block back on the review list.
- `log.access` set by a request: the access line could read the request's overlay at
  `crates/nvs-server/src/serve.rs:1994`, so a route can turn its own line off. It needs a goal of its
  own, which then puts the key back on `SETTABLE` with a probe.
