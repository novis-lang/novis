---
looper: 1
title: A program reads its own settings from nvs.toml
summary: "A [settings] or [app.settings] value reaches a program with its TOML type through Core\\Config::setting, a [secrets] or [app.secrets] value reaches it as a secret string through Core\\Config::secret, nvs config check warns about a key no program reads, the feature owes no proof, and the checks below pass."
context: [docs/rules/config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one.md, docs/rules/config/a-secret-is-a-file-whose-content-is-the-value.md, docs/rules/config/ini-set-is-core-config-set.md, crates/nvs-config/src/tree.rs]
stages:
  - id: 1
    title: "the two blocks are typed, merged and dumped, and the rule is written"
    status: open
    context: [crates/nvs-config/src/secret.rs, crates/nvs-config/src/snapshot.rs, crates/nvs-config/src/resolve.rs, crates/nvs-config/src/audit.rs, crates/nvs-diagnostics/src/code/config.rs, docs/rules/config/every-block-is-argued-where-it-is-added.md, docs/rules/security/secret-has-no-ambient-source.md]
  - id: 2
    title: "Core\\Config::setting and Core\\Config::secret read them"
    status: open
    context: [crates/nvs-stdlib/src/config.rs, crates/nvs-config/src/request.rs, crates/nvs-stdlib/src/env.rs, crates/nvs-types/src/secret_directive.rs]
  - id: 3
    title: "nvs config check warns about a key no program reads"
    status: open
    context: [crates/nvs-cli/src/config.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-diagnostics/src/code/warnings.rs, docs/rules/config/check-and-dump-audit-the-tree-offline.md]
  - id: 4
    title: "both methods have their feature proofs, and the website shows the blocks"
    status: open
    context: [docs/examples/README.md, docs/examples/core/Config/get/about.md, docs/examples/core/Env/secret/about.md, website/src/content/docs/guides/how-to-use/configuration.mdx, docs/agent/prose.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: settings-block-tests
    stage: 1
    name: the two blocks are typed, merged, refused where malformed and redacted in the dump
    run: [cargo, test, --test, settings_block]
    tests:
      - a_settings_value_keeps_its_toml_type
      - an_app_settings_value_overrides_the_global_one_and_both_origins_are_recorded
      - a_later_file_overrides_a_setting_and_both_origins_are_recorded
      - a_settings_key_that_is_not_a_plain_name_is_refused
      - a_datetime_in_settings_is_refused
      - an_unknown_block_beside_settings_is_still_refused
      - a_secret_is_read_from_its_file_and_never_enters_the_table
      - a_secret_with_both_a_value_and_a_file_is_refused
      - a_secret_that_is_not_a_string_is_refused
      - dump_renders_every_secret_as_secret
  - id: settings-runtime-tests
    stage: 2
    name: a program reads its own settings and secrets, and nothing else reaches them
    run: [cargo, test, --test, config_settings]
    tests:
      - an_app_reads_its_own_settings_and_not_another_apps
      - a_reload_reaches_the_next_request_and_not_a_running_one
      - config_get_and_all_never_return_a_setting_or_a_secret
  - id: settings-check-tests
    stage: 3
    name: nvs config check names a key no program reads
    run: [cargo, test, --test, config_settings]
    tests:
      - config_check_warns_about_a_setting_no_program_reads
      - config_check_is_silent_when_every_setting_is_read
      - config_check_judges_no_key_when_a_name_is_computed
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, app-settings]
    reach: true
    floor: false
---

# A program reads its own settings from nvs.toml

## Why

`nvs.toml` refuses every key the typed tree does not know (`E0601`,
`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`), and the set of blocks is closed
(`rule:config/every-block-is-argued-where-it-is-added`). That refusal protects the directives that
grant capabilities: a typo'd `capabilties` must fail at boot. It also leaves an application author no
place for the program's own values, such as an API base URL, a page size or a feature switch, that
differ from one machine to the next.

Today such a value comes from `Core\Env::get`, which gives a tainted string, is shared by the whole
process, is inherited by every child process, and returns `null` for a misspelled name with no word
about it. Or the program opens a file of its own, which needs a file grant and its own parser. A
value only the program reads grants nothing, so a block for it can be open without weakening what
the unknown-key refusal protects.

## Standing decisions

**Four blocks, and they are the only open ones.** `[settings]` and `[secrets]` at the top level hold
values for every application. `[app.settings]` and `[app.secrets]` under an `[[app]]` block hold
values for that application. Every other block stays closed, and an unknown key there is still
`E0601`. The two pairs are fields on `Config` (`crates/nvs-config/src/tree.rs:73`) and on `App`
(`crates/nvs-config/src/tree.rs:156`), each an `Option<toml::Table>` the resolver validates.

**A key is a plain name.** Each key matches `[A-Za-z_][A-Za-z0-9_]*`. A sub-table is allowed and
adds a dotted level: `[settings.api]` with `base = "…"` is read as `api.base`. A quoted key that is
not a plain name is `E0601`, because a dot inside a key would make two settings read the same.

**A setting keeps its TOML type.** A string is a `string`, an integer an `int`, a float a `float`, a
boolean a `bool`, an array an array, and a table an array keyed by name. A TOML date or time is
`E0601`, and its help is the same value quoted as a string. Why: Novis has no value a TOML date maps
to without a time-zone choice, and a quoted string leaves that choice to the program.

**A secret is a string, written inline or read from a file.** Each `[secrets]` key is either
`name = "…"` or `name_file = "/run/secrets/…"`, exactly one of the two. Both is `E0608`, as for a
secret directive. The file is read by the code that reads a directive's `_file`
(`crates/nvs-config/src/secret.rs:371`), with the same rules: one trailing newline stripped,
UTF-8, not empty, at most 64 KiB, and the ownership check. A secret that is not a string is
`E0601`. So a secret's own name never ends in `_file`. The value is carried in `Snapshot::secrets`
(`crates/nvs-config/src/snapshot.rs:133`) under `secrets.<name>`, never in the table.

**An app's value overrides the global one, key by key.** `Snapshot::fold`
(`crates/nvs-config/src/snapshot.rs:262`) already merges each matching `[[app]]` block's sub-tables
over the global blocks, later wins, with both origins recorded. `[app.settings]` and `[app.secrets]`
take that path unchanged. An `[[include]]`d file overrides a value the same way, which is how one
machine sets its own values: the base file holds the defaults, and a file on that machine overrides
them.

**Two read-only methods, and nothing else in the API.**

| Method | Returns |
|---|---|
| `Core\Config::setting(string $name): mixed` | the value with its type, or `null` when no block sets it |
| `Core\Config::secret(string $name): ?secret string` | the secret, or `null` when no block sets it |

The program converts a setting with `as`: `Core\Config::setting('page_size') as int` throws when the
value is missing or is not a whole number. `secret`'s name is a sink, as `Core\Env::secret`'s is,
so a request cannot choose which secret it reads. A setting is not `tainted`: the operator wrote it
in a file only the runtime account or root may change. There is no `set` and no `restore` for
either: a request that needs another value uses a variable.

**`get`, `all`, `set` and `restore` never see the four blocks.** `Core\Config::get('secrets.x')`
returns `null`, and `all` lists no `settings.` or `secrets.` key. Why: `Config::get`
(`crates/nvs-config/src/request.rs:165`) reads `Snapshot::secrets` by key, so without this a secret
would come back from `get` as a plain `?string`, which every output accepts. This is a test, not a
comment.

**A package reads the same values the application does.** That matches `Core\Env::secret`, which
does not ask who calls it. Which package may read what is the grants model's question
(`rule:security/package-authority-is-granted-one-line-at-a-time`), and a limit for these two
methods would be a rule of its own. The new rule's Declined list records it.

**The typo check is a warning from `nvs config check`.** The checker records every name passed to
`setting` or `secret` as a literal string, per program. `nvs config check` reads the programs the
tree names (each `[[app]] entry` and each `[[server.mount]]` entry) and every `--program <file>`
given. It raises a new warning, `W1028`, for each key in the four blocks that none of them reads.
When any of them passes a name that is not a literal, no key is judged, and the check prints one
line saying so. It is a warning and not an error, because a key may be read by a program the tree
does not name. `nvs check` stays a source check that never reads `nvs.toml`
(`rule:config/nvs-toml-is-not-a-project-manifest`), and the boot does not compile programs to look.
What it spends: `nvs config check` now compiles each named program, which is seconds offline and
nothing on a server.

**The rules this goal writes.** A new rule, `rule:config/settings-and-secrets-are-a-programs-own-values`,
holds every decision above with its Declined list:

- a free-form block with no declared keys and no typo check
- a schema the program declares, which the boot checks against (`nvs.toml` is read before any
  program is compiled, and a schema would be source-tree state in a deployment file)
- `${ENV}` interpolation into a setting (already declined by `rule:config/the-file-is-nvs-toml-and-it-is-toml`)
- reading settings through `get` as strings
- a `settings()` method that returns every value (the way a whole set of secrets ends up in a log)
- a `set` for settings
- a package that may not read the values

`rule:config/every-block-is-argued-where-it-is-added` gets the four blocks in its table and says they
are the open ones. `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one` names the
exception. `rule:config/a-secret-is-a-file-whose-content-is-the-value` says `[secrets]` takes the
same pair. `rule:config/ini-set-is-core-config-set` lists six methods. `rule:security/secret-has-no-ambient-source`
names `Core\Config::secret` as a member whose contract is confidentiality.
`rule:config/check-and-dump-audit-the-tree-offline` says `check` reads the named programs for `W1028`.
The new rule is `designed` with `goal: app-settings` from stage 1, and stage 4 flips it to `shipped`
with its `guardedBy`.

**Trade-offs.** Security: no capability can be set in the new blocks, every other block stays
closed, and a secret is typed `secret` and redacted wherever the config is printed. Latency: a
`setting` read is one lookup in the request's snapshot and one value built, with nothing read from
disk. Memory: the values are part of the config snapshot, stored once per snapshot and never per
request. Simplicity: four blocks and two methods. An author learns one way to write a per-machine
value.

**Lanes.** Stages 1 to 3 run in order: stage 2 reads the blocks stage 1 types, and stage 3 records
the calls stage 2 adds. Stage 4's examples, attacks and benches need stage 2's methods in a built
binary. The goal depends on no other chain goal.

## Stages

### 1. the two blocks are typed, merged and dumped, and the rule is written

**Does:** the four blocks parse, validate, merge per app and per file, read secrets from files, and
show up in `nvs config dump`, and the rules say so.

- [ ] `settings` and `secrets` fields on `Config` and on `App`, as `Option<toml::Table>`. `crates/nvs-config/src/tree.rs:73`, `crates/nvs-config/src/tree.rs:156`
- [ ] A validating pass: plain key names, no date or time, a secret is a string, and exactly one of `name` and `name_file`, each refusal naming the file and line. `crates/nvs-config/src/resolve.rs:512`, `crates/nvs-diagnostics/src/code/config.rs:11`
- [ ] `[secrets]` and `[app.secrets]` keys read through the `_file` reader into `Snapshot::secrets` under `secrets.<name>`, and kept out of the table. `crates/nvs-config/src/secret.rs:371`, `crates/nvs-config/src/snapshot.rs:133`
- [ ] The per-app fold carries both sub-tables and both secrets maps, and an override records both origins. `crates/nvs-config/src/snapshot.rs:262`, `crates/nvs-config/src/resolve.rs:961`
- [ ] `nvs config dump` prints every setting, and every secret as `<secret>` with the file it came from. `crates/nvs-config/src/audit.rs:73`, `crates/nvs-config/src/secret.rs:320`
- [ ] The new rule fragment and its data file as `designed`, and the five rewritten fragments, then `bun nv rules --render`. `docs/rules/config/settings-and-secrets-are-a-programs-own-values.md:1`, `data/rules/config/settings-and-secrets-are-a-programs-own-values.json:1`, `docs/rules/config/every-block-is-argued-where-it-is-added.md:8`
- [ ] The tests the `settings-block-tests` check names. `crates/nvs-config/tests/settings_block.rs:1`

### 2. Core\Config::setting and Core\Config::secret read them

**Does:** a program reads its values, typed, and `get` and `all` stop at the four blocks.

- [ ] Two registry rows with their reference cards: `setting` returns `CoreTy::Mixed`, and `secret` returns `Nullable(SecretStr)` with a sink name, as `Core\Env::secret` does. `crates/nvs-stdlib/src/config.rs:88`, `crates/nvs-stdlib/src/env.rs:157`
- [ ] Their runtime functions, reading the request's snapshot and building the value from its TOML type. `crates/nvs-stdlib/src/config.rs:237`, `crates/nvs-config/src/request.rs:165`
- [ ] `get` returns `null` for a `settings.` or `secrets.` name, and `all` lists neither. `crates/nvs-config/src/request.rs:165`, `crates/nvs-config/src/request.rs:474`
- [ ] Conformance: a value of each TOML type, an app's value over the global one, a secret that cannot be printed (a `reject/` case), and a missing setting converted with `as int` throws. `tests/conformance/config/a-setting-keeps-its-toml-type.nvst:1`, `tests/conformance/reject/a-secret-setting-cannot-be-printed.nvst:1`
- [ ] The tests the `settings-runtime-tests` check names. `crates/nvs-cli/tests/config_settings.rs:1`

### 3. nvs config check warns about a key no program reads

**Does:** the checker records each literal name, and `nvs config check` compares them with the keys.

- [ ] A collector of the literal names passed to `setting` and `secret`, and a flag for a computed one, beside `types_secret`. `crates/nvs-types/src/setting_names.rs:1`, `crates/nvs-types/src/expr/calls.rs:754`
- [ ] `W1028`, its message and its help, after `W1027`. `crates/nvs-diagnostics/src/code/warnings.rs:286`
- [ ] `nvs config check` compiles each named entry and each `--program`, and prints `W1028` per unread key, or one line when a name is computed. `crates/nvs-cli/src/config.rs:66`, `crates/nvs-cli/src/config.rs:1121`
- [ ] `rule:config/check-and-dump-audit-the-tree-offline` says what `check` now reads. `docs/rules/config/check-and-dump-audit-the-tree-offline.md:17`
- [ ] The tests the `settings-check-tests` check names. `crates/nvs-cli/tests/config_settings.rs:1`

### 4. both methods have their feature proofs, and the website shows the blocks

**Does:** `Core\Config::setting` and `Core\Config::secret` owe no proof, and a reader finds the blocks
on the configuration page.

**Split:** the two members' directories are disjoint, so two writing subagents write them side by
side: each writes its `about.md`, three examples with their `.out` files, the feature's `nvs.toml`,
two attacks under `tests/hostile/` and the bench. None of them builds. Each brief names
§ *Text an end user reads* in `AGENTS.md`. The session writes the website section and blesses the
`.out` files itself.

- [ ] `setting`: `about.md`, three examples and the `nvs.toml` they run under. `docs/examples/core/Config/setting/about.md:1`, `docs/examples/core/Config/setting/nvs.toml:1`
- [ ] `secret`: `about.md`, three examples, the `nvs.toml` and a secret file beside it. `docs/examples/core/Config/secret/about.md:1`, `docs/examples/core/Config/secret/nvs.toml:1`
- [ ] Attacks: a program tries to print a secret, a program tries to read another app's settings, and a request tries to choose which secret is read. `tests/hostile/core/Config/setting/01-read-another-apps-settings.nvs:1`, `tests/hostile/core/Config/secret/01-print-a-secret.nvs:1`
- [ ] A bench for each, with its growth. `benches/members/core/Config/setting.nvs:1`, `benches/members/core/Config/secret.nvs:1`
- [ ] The configuration page has a section on the four blocks, with one per-machine override through `[[include]]`. `website/src/content/docs/guides/how-to-use/configuration.mdx:35`
- [ ] `bun nv proofs --id 'Core\Config::setting'` and `--id 'Core\Config::secret'` owe nothing, and `bun nv proofs --comments` passes on every new `.nvs`. `tools/nv/cmd/proofs.ts:1`
- [ ] The new rule is `shipped`, with the conformance cases and tests as its `guardedBy`, and `bun nv owners --closes app-settings` passes. `data/rules/config/settings-and-secrets-are-a-programs-own-values.json:1`

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

The goal is new and nothing is built. Every choice is settled in § *Standing decisions*.

### Next

**Stage 1, one group**, on `nvs-config`. No gate.

- [ ] The two fields on `Config` and `App`. `crates/nvs-config/src/tree.rs:73`, `crates/nvs-config/src/tree.rs:156`
- [ ] The validating pass and its refusals. `crates/nvs-config/src/resolve.rs:512`
- [ ] Secrets read into `Snapshot::secrets`. `crates/nvs-config/src/secret.rs:371`
- [ ] The fold and the dump. `crates/nvs-config/src/snapshot.rs:262`, `crates/nvs-config/src/audit.rs:73`
- [ ] The rule fragments, rendered. `docs/rules/config/settings-and-secrets-are-a-programs-own-values.md:1`
- [ ] The `settings-block-tests`. `crates/nvs-config/tests/settings_block.rs:1`

### Backlog
