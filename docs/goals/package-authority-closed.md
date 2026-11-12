---
looper: 1
title: every door asks the calling namespace
summary: "Reached when every Core class has capability rows or a stated reason for none and a class with neither fails a test, spawn script, Core\\Env and every extension member ask the caller's grants, and a package can neither set nor restore the application's configuration."
context: [crates/nvs-types/src/capability.rs, crates/nvs-stdlib/src/cap.rs, crates/nvs-cli/tests/package_grants.rs, docs/rules/security/package-authority-is-granted-one-line-at-a-time.md, docs/rules/security/authority-is-the-enclosing-namespace.md]
stages:
  - id: 1
    title: a package needs the env grant to read the process environment
    status: done
    context: [crates/nvs-config/src/capability.rs, crates/nvs-config/src/defaults.rs, crates/nvs-runtime/src/capability.rs, crates/nvs-stdlib/src/env.rs, docs/rules/security/capability-roster-is-closed.md]
  - id: 2
    title: "spawn script, a queued job and an upgrade ask the calling namespace"
    status: open
    context: [crates/nvs-types/src/expr/isolate.rs, crates/nvs-runtime/src/script.rs, crates/nvs-stdlib/src/socket.rs, docs/rules/security/script-spawn-capability.md]
  - id: 3
    title: every Core class has capability rows or a reason for none
    status: open
    context: [crates/nvs-stdlib/tests/capability.rs, crates/nvs-stdlib/src/classes.rs, docs/rules/testing/capability-completeness-test.md, docs/rules/security/capability-declaration-is-one-table.md]
  - id: 4
    title: an extension member and an extension's own source ask the namespace
    status: open
    context: [crates/nvs-types/src/ext_lib.rs, crates/nvs-ext/src/manifest.rs, crates/nvs-hir/src/symbol.rs, docs/rules/security/extension-grants-are-an-intersection.md, docs/rules/packaging/an-extension-package-carries-two-payloads.md]
  - id: 5
    title: a package sets only its own limits and restores only what a package set
    status: open
    context: [crates/nvs-config/src/request.rs, crates/nvs-stdlib/src/config.rs, docs/rules/config/three-changeability-classes.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: env-needs-a-grant
    stage: 1
    name: "a package with an empty grant line does not compile a read of the process environment, and the application reads it with no line written"
    run: [cargo, test, --test, package_grants]
    timeout: 1800
    tests:
      - a_package_with_an_empty_grant_line_does_not_read_the_environment
      - the_application_reads_the_environment_with_no_line_written
  - id: env-door-asks
    stage: 1
    name: "the environment door asks env, so an isolate that dropped it reads no variable"
    run: [cargo, test, --lib, --, an_isolate_that_dropped_env_reads_no_variable, env_is_granted_on_every_run_where_its_key_is_not_written]
    timeout: 1800
    tests:
      - an_isolate_that_dropped_env_reads_no_variable
      - env_is_granted_on_every_run_where_its_key_is_not_written
  - id: spawn-asks-the-namespace
    stage: 2
    name: "a package with an empty grant line does not compile a spawn, an enqueue or an upgrade"
    run: [cargo, test, --test, package_grants]
    timeout: 1800
    tests:
      - a_package_with_an_empty_grant_line_does_not_spawn_a_script
      - a_package_with_an_empty_grant_line_does_not_spawn_a_method
      - a_package_with_an_empty_grant_line_does_not_enqueue_a_job
      - a_package_with_an_empty_grant_line_does_not_upgrade_a_connection
  - id: the-table-is-closed
    stage: 3
    name: "every Core class has capability rows or a class row with a reason, and every gate that reaches outside a request is on the door list"
    run: [cargo, test, --test, capability]
    timeout: 1800
    tests:
      - every_core_class_has_capability_rows_or_a_reason_for_none
      - no_class_with_a_reason_for_none_calls_a_door
      - every_construct_that_reaches_a_door_names_its_capability
  - id: extensions-ask-the-namespace
    stage: 4
    name: "a package with an empty grant line reaches no file and no host through an extension, and a third-party extension's own source holds only its grant line"
    run: [cargo, test, --test, package_grants]
    timeout: 1800
    tests:
      - a_package_with_an_empty_grant_line_does_not_call_an_extension_that_requests_io
      - an_extension_that_requests_nothing_is_called_with_no_grant
      - a_third_party_extensions_source_holds_only_its_grant_line
      - a_third_party_extensions_source_is_held_where_no_grant_table_exists
      - a_built_in_components_source_keeps_its_authority
  - id: package-config-is-narrow
    stage: 5
    name: "set from a package returns false for every key outside limits, and restore from a package leaves the application's value in force"
    run: [cargo, test, --lib, --, a_package_sets_no_key_outside_limits, a_package_restores_only_what_a_package_set, a_package_restore_returns_to_the_applications_value]
    timeout: 1800
    tests:
      - a_package_sets_no_key_outside_limits
      - a_package_restores_only_what_a_package_set
      - a_package_restore_returns_to_the_applications_value
---

# every door asks the calling namespace

## Why

A Novis application compiles third-party packages in. Each package namespace holds only what its
`[grants]` line in the application's `package.toml` names, and a call outside it is compile error
`E0658`. `rule:security/package-authority-is-granted-one-line-at-a-time` promises that a package with
an empty line holds "arbitrary computation with no authority at all". Five doors do not keep it.

The check is one function, `reject_ungranted_call` (`crates/nvs-types/src/capability.rs:235`). It
asks `nvs_stdlib::cap::needs` (`crates/nvs-stdlib/src/cap.rs:203`), which reads the table
`registry::CAPABILITIES` (`crates/nvs-stdlib/src/registry.rs:1975`). A member with no row needs
nothing. The completeness test holds only a class that already has one row
(`crates/nvs-stdlib/tests/capability.rs:298`). A class with no row at all is never looked at.

- **The environment.** `Core\Env::get`, `secret` and `all` have no row, and their door asks nothing
  (`crates/nvs-stdlib/src/env.rs:560`, `:589`, `:613`, `crates/nvs-runtime/src/capability.rs:1826`).
  A package with an empty line reads every variable of the server process. `get` returns a `tainted
  string`, and any `Core` escape member makes it plain. `crates/nvs-stdlib/src/env.rs:10` records the
  missing door as a decision. Its reason covers the operator's own application, not a package.
- **Starting a script.** `spawn script` is a language construct, and the checker never asks its
  namespace (`crates/nvs-types/src/expr/isolate.rs:154`). `Core\Queue::push` declares `None`
  (`crates/nvs-stdlib/src/registry.rs:2427`). `Core\Socket` and `Core\Sse` have no row
  (`crates/nvs-stdlib/src/socket.rs:985`, `crates/nvs-stdlib/src/sse.rs:574`). The run-time door asks
  the operator's grant alone (`crates/nvs-runtime/src/script.rs:477`), and that covers the app folder
  by default. A package runs any application script with arguments it chooses.
- **An extension's member.** `cap::needs` reads the `Core` table and returns nothing for an extension
  class. The run-time caller carries no namespace (`crates/nvs-cli/src/extensions.rs:805`). A package
  with an empty line reads a file through an extension, where `Core\IO` gives `E0658`.
- **An extension's own source.** A `.nvsx` carries Novis source beside its component. The check passes
  every namespace no line matches, and passes everything where the table is empty
  (`crates/nvs-types/src/capability.rs:243`). The loader checks only the namespace of each carried file
  (`crates/nvs-ext/src/load.rs:515`). The source holds everything the application holds.
- **The application's configuration.** `Request::set_as_package` narrows `limits.*` alone and passes
  every other key to `set` (`crates/nvs-config/src/request.rs:380`). Since `e2dd3a7c31` a request's
  `http.cookies.*`, `http.cors.*` and `http.headers.*` settings reach its response. A package clears
  the session cookie's `http_only` flag. `Core\Config::restore` takes no package word
  (`crates/nvs-stdlib/src/config.rs:378`), so a package undoes a limit the application lowered.

Each door is reached by code a package manager fetched, direct or transitive, with nothing written by
a human. None needs `net.connect`, `fs.read` or `process.exec`. The cause is one: a row somebody did
not write is read as "needs none". This goal closes the table, so a forgotten row fails a test.

## Standing decisions

**One closed table decides authority, and it is closed over every `Core` class.** Every class in
`CLASSES` (`crates/nvs-stdlib/src/classes.rs:14`) is in exactly one of two lists in `registry.rs`. In
`CAPABILITIES`, every member has a row, as now. In a new `NEEDS_NONE: &[(&str, &str)]`, the class has
one row with a reason, and none of its members reaches a door. A test reads `CLASSES` and fails naming
a class in neither list or in both. **What it spends:** nothing at run time, and one more list read by
a test. Usability is unchanged. Simplicity: one screen of one file still answers the whole question.
Safe default: a class whose members differ leaves `NEEDS_NONE`, and each member gets its own row.
Declined: a `None` row for every member of all 187 classes. Thousands of equal rows hide the rows
that matter. Declined: an exception list in the test, which `rule:testing/capability-completeness-test`
forbids.

**A reason for none is checked, not trusted.** The door scan
(`crates/nvs-stdlib/tests/capability.rs:585`) already reads every door call in `nvs-stdlib`. A new
test fails when the module of a `NEEDS_NONE` class calls a door. A gate in `nvs-runtime` that reaches
outside the request is a door: it takes the member and asks a capability. The environment readers move
into `nvs_runtime::capability` as `env_var` and `env_vars`, and `script::resolve` takes the member.
**What it spends:** test time only. Declined: a compile-time `const` assertion over both lists. It
needs string compares in `const fn` for no gain over a test that the gate runs.

**The process environment is the capability `env`.** It is a full roster entry: `Cap::Env`, the name
`env`, and the key `[capabilities] env`. With the key not written it is granted on every kind of run,
so the application writes nothing and reads as now. `Core\Env::get`, `secret` and `all` have `env`
rows, and `mode` and `context` have `None` rows. A package needs `"env"` on its grant line. No
existing capability fits: reading a variable opens no file, no host and no program. **What it
spends:** one authority check per `Core\Env` read, which is a branch and a snapshot read. It
allocates nothing. Usability: a package that reads its settings from the environment needs one written
word. Safe default: if the roster cannot hold a name with one segment, the name is `env.read` under
`[capabilities.env]`. Declined: a grant scoped by variable name. A package grant carries no scope, and
`all` has no name to ask about. Declined: a capability only the checker knows. `Core\Cap::has` would
report it and an operator's `env = false` would do nothing.

**The `env` door asks at run time too, so an isolate that dropped it reads nothing.** A child spawned
with a `grants:` list that leaves out `env` gets a `RuntimeError` from `Core\Env::get`. This follows
`rule:security/no-runtime-grant`: a capability has two layers, and narrowing only drops. The module
doc of `env.rs` is rewritten to say the operator chose the environment for the application, and a
package is granted it. Safe default: where an in-tree child script breaks, its spawn site adds `"env"`.
The door is never removed to keep a caller working.

**A deployment credential is one capability whichever member returns it.** `Core\Config::secret`
(`crates/nvs-stdlib/src/config.rs:424`) returns a secret directive, as `Core\Env::secret` returns a
variable. It gets the `env` row and its door asks. `Core\Config::get`, `all` and `setting` get `None`
rows when the session has confirmed that none returns a secret directive's value to a package. One
that does gets the `env` row. `set` and `restore` get `None` rows, because the package word of the
last decision below holds them. **What it spends:** one authority check per `Core\Config::secret`.

**From a package, every `Core\Config::get` needs `env`, because the namespace check is per member.**
`get` returns a secret directive's value, such as `db.main.password`, unredacted, and its prepared
word already requires a literal name for one. `all` redacts each secret directive, and `setting`
reads `[settings]` alone, so both have `None` rows. At run time `get` asks `env` only for a secret
key, so the application and an isolate pay nothing for any other key. **What it spends:** one
`is_secret_key` test per `get`, which the member already made. Usability: a package that reads
`limits.memory` needs `"env"` on its line. Declined for now: a check that asks `env` only where the
literal name is a secret key. It is a second source of capability needs beside the one table, and
stage 3 closes that table. The backlog keeps it.

**Starting a script is `script.spawn`, for the construct and for every member that names a script.**
`spawn script` asks the calling namespace in both forms, the path form and the `Class::method` form.
`Core\Queue::push`, `Core\Socket::upgrade` and `Core\Sse::upgrade` get `script.spawn` rows. No new
capability: each one names code that will run as its own isolate, which is what `script.spawn`
already means. The construct's row lives beside the table as `CONSTRUCT_CAPABILITIES`, so `nvs meta`
renders it. **What it spends:** one table lookup per spawn site while compiling, and nothing on the
request path. Usability: a package that enqueues its own job needs one written word. Safe default: if
the lowering has no place for an optional-capability guard at a spawn site, an optional `script.spawn`
there is `E0658`. Declined: a `queue.push` capability. A second grant over one effect lets a
deployment close one and leave the other open. Declined: a `db.*` row for `push`. The operator named
the queue's connection in `[queue] connection`. A job's recorded grants stay the isolate's narrowing:
the grant line is the human's word that this package may start application scripts.

**One function says whose authority a piece of code holds.** `prepare_config_set`, `prepare_cap_has`
and `reject_ungranted_call` each repeat the test "no namespace, an empty table, or no matching line"
(`crates/nvs-types/src/capability.rs:170`, `:204`, `:243`). One `authority_of` replaces the three and
returns the application, a written line, or an implied empty line. Every new check in this goal calls
it. **What it spends:** nothing new. Declined: a fourth copy of the test in the spawn check.

**A call to an extension member is asked of the calling namespace.** The capabilities are the ones
the manifest's `requests` imply (`crates/nvs-ext/src/manifest.rs:395`): `fs.read` for a non-empty
`read`, `fs.write` and `fs.read` for a non-empty `write`, and `net.connect` for a non-empty `connect`.
A manifest declares its requests for the whole component, so every method of its class needs all of
them. A manifest that requests nothing needs no grant. `reject_ungranted_call` reads them for an
extension class as it reads `cap::needs` for a `Core` class. **What it spends:** one more lookup per
extension call site while compiling. Usability: a package that calls a file-reading extension needs
`fs.read` on its line. A package may still call a granted package's public method, as between any two
packages. Safe default: if a manifest gains requests per method, the check reads those. Declined: the
namespace carried to run time in `Caller`. It costs an argument on every extension call, and the
checker already knows the namespace.

**Novis source inside a third-party `.nvsx` is a package.** Its namespace holds what a `[grants]` line
names and nothing else. Where no line matches, `authority_of` returns an empty line, also where the
application has no grant table at all. An extension shipped inside the `nvs` binary keeps the
application's authority: `ExtensionClass::component` names it (`crates/nvs-hir/src/symbol.rs:63`),
and only Novis declares under `Novis\` (`crates/nvs-ext/src/load.rs:842`). The key is the namespace,
so application code that declares a namespace under the extension's is held to the same line. A
manifest class with one segment has no namespace to key on, so it carries no source or does not load.
**What it spends:** one scan of the loaded extension classes per namespace lookup while compiling.
Usability: an extension whose source reads a file needs one line the operator writes, and `E0658`
prints that line. Safe default: deny. Declined: reading the `[[extension]]` entry's `grants`. Those
are the guest's file roots and hosts, which is another set. Declined: an audit warning alone.

**From a package, `Core\Config::set` returns `false` for every key outside `limits.*`.** Under
`limits.*` it may lower a value and never raise one, as now. The mode, `[http]`, `[log]` and every
other block are the application's. **What it spends:** one prefix compare per `set` from a package.
Usability: an application that wants a package's policy calls `set` itself with the package's value.
Declined: letting a package tighten a boolean such as `http.cookies.secure`. It needs a direction for
every key, and the application can make the call.

**`Core\Config::restore` takes the same package word, and a package restores only what a package
set.** When a package lowers a key the application also set, the overlay keeps the application's value
under the package's. A package's `restore` puts that value back, or drops the entry when the
application set none. It never removes a value the application set. `restore` still returns `void`:
a name the caller never set is not an error. A method reference carries no word, so it restores as a
package. **What it spends:** one more optional string for each overlay entry a package set, per
request, freed with the request. Declined: telling one package from another. It needs a line number in
the prepared word, and a limit is no wall between two packages that share one request's budget.

**Each door gets an attack, one file per door.** A file marked `// hostile: expect-refusal` passes on
its first refusal, so two doors in one file would prove one. The attacks that need a package live in
`tests/hostile/tools/packages/grants-what-each-namespace-may-do/`, whose `package.toml` already holds
an empty line. The extension attacks live in
`tests/hostile/tools/config/extension-grants-what-a-component-may-reach/`. The `Core\Config` attacks
run to the end and print `false`. Every comment in them follows `AGENTS.md` § *Text an end user reads*.

**A caller in the tree that breaks gets its grant line, never a looser check.** A package, an example
or a fixture that a new row refuses adds the capability to its `[grants]` line. A first-party
`Novis\` package is a package like any other.

**The classification adds no capability beyond `env`.** Where stage 3 finds a class that reaches
outside the request and has no decision here, the session uses the nearest roster capability. Where
none fits, the class gets its `NEEDS_NONE` row saying what it reaches, and a gap record under
`data/gaps/` names it. This follows the security priority first and the simplicity priority second.

**Lanes:** one lane, in order. Stages 1, 2 and 3 share `crates/nvs-stdlib/src/registry.rs`. Stages 2,
4 and 5 share `crates/nvs-types/src/capability.rs`. Stage 5 also shares
`crates/nvs-config/src/request.rs` and `crates/nvs-stdlib/src/config.rs` with goal `limits-hold`,
which runs after this goal on the chain.

## Stages

### 1. a package needs the env grant to read the process environment

**Does:** add `env` to the capability roster with a default that grants it, make the environment door
ask it, and give `Core\Env` and `Core\Config` their rows.

**Split:** an `Explore` subagent lists every file that lists the roster by name, as `path:NN`: the
configuration reference, `default.toml`, the capabilities example and its `about.md`, the `nvs meta`
snapshots and the editor's completion data. A second one lists every package namespace in the tree
that calls `Core\Env::get`, `secret`, `all` or `Core\Config::secret`, with its `package.toml`.

- [x] `Cap::Env` joins the roster: the variant, `ALL`, `name`, `grant` and `grant_mut`, and
  `Capabilities` gains `env` (`rule:security/capability-roster-is-closed`,
  `crates/nvs-config/src/capability.rs:66`, `crates/nvs-config/src/capability.rs:673`,
  `crates/nvs-config/src/capability.rs:833`, `crates/nvs-config/src/tree.rs:323`).
- [x] `defaults::capability` returns `Granted::Everything` for `env` on every kind of run, and its doc
  says so (`rule:config/no-configuration-file-is-a-complete-configuration`,
  `crates/nvs-config/src/defaults.rs:426`). Test `env_is_granted_on_every_run_where_its_key_is_not_written`
  sits in that crate's lib tests.
- [x] The two readers become the doors `capability::env_var` and `capability::env_vars`. Each takes
  the context and the member and asks `env` unscoped. `environment.rs` is deleted
  (`rule:security/capability-check-at-the-door`, `crates/nvs-runtime/src/capability.rs:1826`,
  `crates/nvs-runtime/src/capability.rs:2035`). Test `an_isolate_that_dropped_env_reads_no_variable`
  sits in `crates/nvs-runtime/src/capability.rs`.
- [x] `Core\Env::get`, `secret` and `all` pass their context and member to the door, and the module doc
  is rewritten as a whole (`rule:security/tainted-sources`, `crates/nvs-stdlib/src/env.rs:560`,
  `crates/nvs-stdlib/src/env.rs:589`, `crates/nvs-stdlib/src/env.rs:613`,
  `crates/nvs-stdlib/src/env.rs:10`). The lib test at `crates/nvs-stdlib/src/env.rs:752` follows.
- [x] `Core\Config::secret` asks `env` at its door (`rule:config/a-secret-is-a-file-whose-content-is-the-value`,
  `crates/nvs-stdlib/src/config.rs:424`). The session confirms what `get`, `all` and `setting` return
  for a secret directive and records it in the row's comment.
- [x] Rows in `CAPABILITIES`: `Core\Env` with `env` for `get`, `secret` and `all`, and `None` for
  `mode` and `context`. `Core\Config` with `env` for `secret` and a row for each other member
  (`rule:security/capability-declaration-is-one-table`, `crates/nvs-stdlib/src/registry.rs:1975`). The
  door lists in the test gain the two doors (`crates/nvs-stdlib/tests/capability.rs:436`).
- [x] Tests `a_package_with_an_empty_grant_line_does_not_read_the_environment` and
  `the_application_reads_the_environment_with_no_line_written`
  (`rule:security/package-authority-is-granted-one-line-at-a-time`,
  `crates/nvs-cli/tests/package_grants.rs:65`). The first covers `get`, `secret`, `all` and
  `Core\Config::secret`, and expects `E0658` naming `env` for each.
- [x] Case `tests/conformance/cap/grants-a-package-reads-the-environment-only-with-env.nvst:1`: a
  package namespace with `["env"]` reads a variable, and one with `[]` does not compile.
- [x] Attack `tests/hostile/tools/packages/grants-what-each-namespace-may-do/03-read-the-process-environment.nvs:1`,
  marked `// hostile: expect-refusal`. Its package class calls `Core\Env::all()` under the empty line.
- [x] Rule fragments: the roster names `env` and why no older capability fits
  (`docs/rules/security/capability-roster-is-closed.md:1`). The package rule lists the environment
  among what a package cannot read (`docs/rules/security/package-authority-is-granted-one-line-at-a-time.md:8`).
  `rule:security/request-state-throws-in-an-isolate` no longer says the environment needs no grant.
- [x] Every roster listing the first subagent named gains `env`, and every package the second one named
  gains `"env"` on its line. `bun nv proofs --id` names what the new capability still owes.

### 2. spawn script, a queued job and an upgrade ask the calling namespace

**Does:** hold `spawn script` to its namespace's line in the checker, and give the three members that
name a script their `script.spawn` rows.

**Split:** an `Explore` subagent lists every package namespace in the tree that writes `spawn script`
or calls `Core\Queue::push`, `Core\Socket::upgrade` or `Core\Sse::upgrade`, with its `package.toml`.

- [ ] One `authority_of` replaces the three repeated tests, and one `reject_ungranted` takes the
  capabilities and the text that names what needs them (`rule:security/authority-is-the-enclosing-namespace`,
  `crates/nvs-types/src/capability.rs:170`, `crates/nvs-types/src/capability.rs:204`,
  `crates/nvs-types/src/capability.rs:243`).
- [ ] `check_spawn_script` asks `script.spawn` of the enclosing namespace for the path form and the
  method form. The diagnostic says "`spawn script` needs `script.spawn`"
  (`rule:security/script-spawn-capability`, `crates/nvs-types/src/expr/isolate.rs:154`).
- [ ] `CONSTRUCT_CAPABILITIES` holds `spawn script` with `script.spawn` beside the table, the checker
  reads it through `nvs_stdlib::cap`, and `nvs meta` renders it
  (`rule:security/capability-declaration-is-one-table`, `crates/nvs-stdlib/src/registry.rs:1975`,
  `crates/nvs-stdlib/src/cap.rs:203`).
- [ ] `Core\Queue::push` declares `script.spawn`, and the comment above its rows is rewritten
  (`rule:concurrency/queue-four-members`, `crates/nvs-stdlib/src/registry.rs:2427`).
- [ ] `Core\Socket` and `Core\Sse` enter the table: `upgrade` with `script.spawn`, and every other
  member of both classes with `None` and its reason (`rule:concurrency/an-upgrade-is-spawn-shaped`,
  `crates/nvs-stdlib/src/socket.rs:985`, `crates/nvs-stdlib/src/sse.rs:574`).
- [ ] `script::resolve` takes the member and names it in its refusal. The door scan reads a
  `script::resolve` call as a door that asks `script.spawn`
  (`rule:testing/capability-completeness-test`, `crates/nvs-runtime/src/script.rs:477`,
  `crates/nvs-stdlib/src/socket.rs:728`, `crates/nvs-runtime/src/host.rs:308`,
  `crates/nvs-host/src/ladder.rs:129`, `crates/nvs-cli/src/worker.rs:1514`,
  `crates/nvs-cli/src/serve.rs:2149`, `crates/nvs-stdlib/tests/capability.rs:585`).
- [ ] Four tests named in check `spawn-asks-the-namespace`, each expecting `E0658` naming
  `script.spawn`, and each with a twin namespace that holds the grant and compiles
  (`crates/nvs-cli/tests/package_grants.rs:65`).
- [ ] Case `tests/conformance/cap/grants-a-package-spawns-a-script-only-with-script-spawn.nvst:1`.
- [ ] Attacks in `tests/hostile/tools/packages/grants-what-each-namespace-may-do/`, each marked
  `// hostile: expect-refusal`: `04-spawn-an-application-script.nvs:1`,
  `05-enqueue-an-application-script.nvs:1`, `06-open-a-socket-handler.nvs:1` and
  `07-open-an-event-stream-handler.nvs:1`.
- [ ] Rule fragments: `script.spawn` is asked of the namespace too
  (`docs/rules/security/script-spawn-capability.md:21`), and
  `rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` says who may enqueue.
- [ ] Every package the subagent named gains `"script.spawn"` on its line, and the example folder's
  `about.md` for the grant table says a package needs it to start a script.

### 3. every Core class has capability rows or a reason for none

**Does:** add `NEEDS_NONE`, classify every class that has no row, and add the tests that read
`CLASSES` and the door calls.

**Split:** `reader` subagents, one per third of `CLASSES`, list each class with no row in
`CAPABILITIES`. For each they report the `nvs_runtime` functions its module calls that reach a file, a
socket, a process, the terminal, a shared store or another isolate, as `file.rs:NN`. The session
writes the rows.

- [ ] `NEEDS_NONE: &[(&str, &str)]` beside `CAPABILITIES`, with one row and one reason for each class
  whose members reach no door. The doc of `CAPABILITIES` is rewritten as a whole
  (`rule:security/capability-declaration-is-one-table`, `crates/nvs-stdlib/src/registry.rs:1912`,
  `crates/nvs-stdlib/src/classes.rs:14`).
- [ ] A class the subagents show reaching outside the request gets member rows with the nearest roster
  capability. Where none fits, it gets its `NEEDS_NONE` row and a gap record
  (`rule:security/capability-roster-is-closed`, `crates/nvs-stdlib/src/registry.rs:1975`).
- [ ] Test `every_core_class_has_capability_rows_or_a_reason_for_none`. It reads `CLASSES` and fails
  naming each class in neither list, in both, or with an empty reason
  (`rule:testing/capability-completeness-test`, `crates/nvs-stdlib/tests/capability.rs:298`).
- [ ] Test `no_class_with_a_reason_for_none_calls_a_door`. It runs the door scan over the module of each
  `NEEDS_NONE` class and fails naming the call (`crates/nvs-stdlib/tests/capability.rs:585`).
- [ ] Test `every_construct_that_reaches_a_door_names_its_capability`. A door call whose label is not a
  `Core` member must be in `CONSTRUCT_CAPABILITIES` (`crates/nvs-stdlib/tests/capability.rs:544`).
- [ ] `cap::needs` and its doc say a class in `NEEDS_NONE` needs nothing, and a class in neither list
  cannot exist (`crates/nvs-stdlib/src/cap.rs:187`). `nvs meta` renders `NEEDS_NONE` with its reasons.
- [ ] Rule fragments rewritten to the two lists: `docs/rules/testing/capability-completeness-test.md:1`
  and `docs/rules/security/capability-declaration-is-one-table.md:1`.

### 4. an extension member and an extension's own source ask the namespace

**Does:** make a call to an extension member need the capabilities its manifest requests, and hold a
third-party extension's carried source to its own grant line.

**Split:** an `Explore` subagent lists every example, case and fixture that calls an extension class
from a namespace with a `[grants]` line, and every third-party `.nvsx` fixture that carries source.

- [ ] `ext_lib::seed` records, for each manifest class, the capabilities its `requests` imply and
  whether a built-in component declares it (`rule:security/extension-grants-are-an-intersection`,
  `crates/nvs-types/src/ext_lib.rs:1`, `crates/nvs-ext/src/manifest.rs:395`,
  `crates/nvs-hir/src/symbol.rs:50`).
- [ ] `reject_ungranted_call` asks those capabilities for an extension class
  (`crates/nvs-types/src/capability.rs:261`).
- [ ] `authority_of` returns an empty line for a namespace under a third-party extension's own, where
  no `[grants]` line matches and also where the table is empty
  (`rule:packaging/an-extension-package-carries-two-payloads`, `crates/nvs-types/src/capability.rs:243`).
- [ ] The `E0658` note for an implied line says no line names the namespace, and its help prints the
  line to write in `package.toml` (`crates/nvs-types/src/capability.rs:301`). The text follows
  `AGENTS.md` § *Text an end user reads*.
- [ ] The loader refuses carried source beside a manifest class that has one segment
  (`crates/nvs-ext/src/load.rs:857`).
- [ ] `nvs ext inspect` prints the capabilities a calling package needs
  (`rule:security/extension-grants-are-an-intersection`, `crates/nvs-cli/src/ext.rs:412`).
- [ ] Five tests named in check `extensions-ask-the-namespace`
  (`crates/nvs-cli/tests/package_grants.rs:65`). Safe default: where loading an extension needs
  fixtures of `crates/nvs-cli/tests/live_config.rs`, the tests go there and the check's `run` follows.
- [ ] Case `tests/conformance/ext/a-package-calls-an-extension-only-with-its-capabilities.nvst:1`.
- [ ] Attacks in `tests/hostile/tools/config/extension-grants-what-a-component-may-reach/`, each marked
  `// hostile: expect-refusal`, under the next free numbers:
  `02-a-package-with-no-grant-reads-through-the-component.nvs:1` and
  `03-the-extensions-own-source-opens-a-file.nvs:1`. The folder gains a `package.toml` with an empty
  line.
- [ ] Rule fragments reworded: the third set is the caller's grant line, asked while compiling
  (`docs/rules/security/extension-grants-are-an-intersection.md:1`). Carried source is a package and
  holds only what a line names, and a built-in keeps its authority
  (`docs/rules/packaging/an-extension-package-carries-two-payloads.md:10`). Third-party extension
  source is never unmatched (`docs/rules/security/an-unmatched-namespace-holds-the-application.md:5`).
- [ ] The example's `about.md` says a calling package needs the capability, and every caller the
  subagent named gets its line.

### 5. a package sets only its own limits and restores only what a package set

**Does:** refuse every key outside `limits.*` in `set_as_package`, and give `restore` the package word
and the application's value to return to.

- [ ] `set_as_package` returns `false` for every key that does not start with `limits.`
  (`rule:security/package-authority-is-granted-one-line-at-a-time`,
  `crates/nvs-config/src/request.rs:380`).
- [ ] The overlay keeps the application's value under a package's, and `restore_as_package` puts it
  back or drops a package's own entry. An application `set` over a package's entry clears the kept
  value (`rule:config/ini-set-is-core-config-set`, `crates/nvs-config/src/request.rs:563`,
  `crates/nvs-config/src/request.rs:307`). The doc says what it spends per request.
- [ ] `Core\Config::restore` joins `PREPARED_MEMBERS`, takes the word as argument 0 and calls
  `restore_as_package` without it. The module doc section on packages is rewritten
  (`crates/nvs-stdlib/src/registry.rs:3398`, `crates/nvs-stdlib/src/config.rs:378`,
  `crates/nvs-stdlib/src/config.rs:55`).
- [ ] `prepare_config_set` prepares `restore` too, through `authority_of`
  (`crates/nvs-types/src/capability.rs:160`, `crates/nvs-types/src/expr/calls.rs:762`).
- [ ] Test `a_package_sets_no_key_outside_limits`. It reads every key from the configuration's own key
  list and fails naming a key outside `limits.` that `set_as_package` accepts
  (`crates/nvs-config/src/request.rs:380`).
- [ ] Tests `a_package_restores_only_what_a_package_set` and
  `a_package_restore_returns_to_the_applications_value` (`crates/nvs-config/src/request.rs:563`). The
  lib test at `crates/nvs-stdlib/src/config.rs:840` follows.
- [ ] Case `tests/conformance/cap/grants-a-package-sets-and-restores-only-its-own-limits.nvst:1`: a
  package sets `http.cookies.http_only` and gets `false`, and restores a limit the application lowered
  and the limit stays.
- [ ] Attacks that run to the end and print what each call returned:
  `tests/hostile/tools/packages/grants-what-each-namespace-may-do/08-change-the-applications-cookies.nvs:1`
  and `tests/hostile/tools/packages/grants-what-each-namespace-may-do/09-undo-the-applications-limit.nvs:1`.
- [ ] Rule fragments: the package paragraph names every key and `restore`
  (`docs/rules/security/package-authority-is-granted-one-line-at-a-time.md:26`), and
  `docs/rules/config/three-changeability-classes.md:1` says a package sets `limits.*` alone. The
  `Core\Config` example's `about.md` says so in one sentence.

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done (`7a6e8cce23`, `39d6b5259a`), and checks `env-needs-a-grant` and `env-door-asks`
are green. `Cap::Env` is on the roster as `env`, the one name with no family. `Cap::header` spells
the table a grant goes under, so a refusal of `env` points at `[capabilities]`, and the four places
that spelled `[capabilities.<family>]` by hand call it. `nvs_runtime::environment` is deleted: its
readers are the doors `capability::env_var` and `env_vars` (`crates/nvs-runtime/src/capability.rs:1826`).
`Core\Config::get` has the `env` row and asks `env` at run time for a secret directive alone (see
the standing decision on it). No in-tree package reads the environment, so no grant line changed.

The sweep's reds were stale. The nightly run built `f4c1d5c7`, and its one failing test,
`live_config::a_changed_server_timeout_applies_to_the_next_connection`, was fixed after it by
`305c04f660`. No `env-secret-proofs` check exists in the tree.

Expected red: none.

### Next

Stage 2, over `crates/nvs-types/src/capability.rs`, `crates/nvs-types/src/expr/isolate.rs`,
`crates/nvs-stdlib/src/registry.rs` and `crates/nvs-cli/tests/package_grants.rs`. One `Explore`
subagent goes out first, as the stage's **Split:** says.

- [ ] `crates/nvs-types/src/capability.rs:160`: `authority_of` replaces the three repeated tests in
  `prepare_config_set`, `prepare_cap_has` and `reject_ungranted_call` (`:194`, `:235`).
- [ ] `crates/nvs-types/src/expr/isolate.rs:154`: `check_spawn_script` asks `script.spawn` of the
  namespace, both forms.
- [ ] `CONSTRUCT_CAPABILITIES` beside `CAPABILITIES` (`crates/nvs-stdlib/src/registry.rs:2033`).
- [ ] `Core\Queue::push` gets `script.spawn` (`crates/nvs-stdlib/src/registry.rs:2506`), and
  `Core\Socket` and `Core\Sse` enter the table.
- [ ] `script::resolve` takes the member (`crates/nvs-runtime/src/script.rs:477`), and the door scan
  reads it as a `script.spawn` door (`crates/nvs-stdlib/tests/capability.rs:436`).
- [ ] `crates/nvs-cli/tests/package_grants.rs:286`: the four tests of check `spawn-asks-the-namespace`.
  `reads_the_environment` there is the shape to copy.
- [ ] `tests/hostile/tools/packages/grants-what-each-namespace-may-do/03-read-the-process-environment.nvs:1`:
  attacks `04` to `07` beside it, the case, and the rule fragments the stage names.

### Backlog

- Grant enforcement through a callable, a `mixed` receiver, inheritance and namespace identity is not
  reviewed. No item here covers it.
- The package client is the goal `package-files-contained`, the registry is `registry-waits-hold`, and
  reflection is `calls-by-name-are-checked` and `crossings-carry-data-only`.
- `require` of an application file from a package namespace was not traced.
- A package's `Core\Config::get` of a key that is not a secret directive needs `env` only because a
  grant line is asked per member. Asking it only for a literal secret key would let a package read
  `limits.*` with no line (`crates/nvs-types/src/capability.rs:235`).

The open items `qualifiers-hold` left when it was retired:

- An array literal of the program's own ints, such as `[1, 2]`, passed to a method of a `mixed`
  receiver is refused as `E0895`, which calls it `mixed` text. The refusal is in the safe direction,
  but `rule:security/taint-propagation` says such a literal carries no outside text
  (`report_args_with_no_parameter_list`, `crates/nvs-types/src/expr/calls.rs`, with
  `check_array_literal`).
- A property-path or string-with-holes failure still names the bad segment's text, such as
  ``has no public property `$nmae` `` (`nvs_property_path_mismatch`, `nvs_string_holes_mismatch`,
  `crates/nvs-runtime/src/helpers.rs`). Name the segment by position instead.
- `$fn->call($obj, $a)` on a typed callable checks only `$a`'s qualifier against the written
  parameter, not its type (`infer_callable_rebind`, `crates/nvs-types/src/expr/calls.rs`).
- A `.=` compound write through an erased property is not asked the untyped-write question, only the
  plain `=` (`crates/nvs-types/src/expr/assign.rs:1083`).
- A third-party extension method that takes a resource may return text built from an earlier
  `tainted` argument as plain text (`crates/nvs-types/src/ext_lib.rs:290`). Not traced to a run.
- `slot_check` and `test_shape` are two maps from a type to a run-time test. An intersection type
  and `iterable` stay `SlotCheck::Any` in the first, and a single-value `string` keeps its tag alone
  (`crates/nvs-ir/src/lower/mod.rs:602`, `crates/nvs-ir/src/lower/expr.rs:7881`).
- A callable's `invoke` row carries no parameter type spellings, so a bare `callable`'s refusal
  names no type (`crates/nvs-ir/src/lower/anon_fn.rs:374`, `crates/nvs-runtime/src/dispatch.rs:286`).
- `check_param_class` runs on every call of an anonymous function, a proven one too
  (`crates/nvs-ir/src/lower/anon_fn.rs:454`).
- `Core\Str::format('%1$s', $o)` over a `Stringable` object throws "it does not implement
  `Stringable`": `stringify` finds no compiled `toString` (`crates/nvs-runtime/src/helpers.rs:2049`).
  Once it works, `crates/nvs-codegen/tests/borrowed_reads.rs` gains a row for it.
