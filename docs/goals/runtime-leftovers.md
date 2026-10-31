---
looper: 1
title: "the regex deny directive, socket activation, the debugger's task tree, a task's child report and the owned-alias rename exist"
summary: "each of these rules' Not-on-disk parts is built and tested, and none of their fragments has a Not-on-disk section left."
context: [docs/agent/prose.md]
stages:
  - id: 1
    title: "the [regex] block exists, and a backtracking pattern literal is silent, a warning or a compile error as it says"
    status: done
    context: [docs/rules/core-classes/regex-compile-time-tiering.md, crates/nvs-config/src/tree.rs, crates/nvs-config/src/directive.rs, crates/nvs-config/src/errors.rs, crates/nvs-types/src/intrinsics.rs]
  - id: 2
    title: "under deny a run-time pattern that needs backtracking throws, and the directive has its docs and feature proofs"
    status: done
    context: [crates/nvs-stdlib/src/regex.rs, crates/nvs-config/src/default.toml, docs/reference/tools/20-config.md, docs/rules/config/every-block-is-argued-where-it-is-added.md]
  - id: 3
    title: "a service on a privileged port gets its socket from a systemd socket unit and holds no capability"
    status: open
    context: [docs/rules/packaging/the-generated-unit-is-hardened.md, crates/nvs-cli/src/service.rs, crates/nvs-cli/src/serve.rs, docs/reference/tools/25-server.md]
  - id: 4
    title: "nvs dap answers the task tree, and the VS Code extension shows it"
    status: open
    context: [docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md, crates/nvs-cli/src/dap/stopped.rs, crates/nvs-cli/src/dap/mod.rs, crates/nvs-host/src/debug.rs, editors/vscode/src/debug.ts]
  - id: 5
    title: "a type alias a class declares is renamed by the editor and by nvs code rename"
    status: open
    context: [docs/rules/ide/no-refactoring-introduces-an-alias.md, docs/rules/types/class-scoped-alias.md, crates/nvs-lsp/src/rename.rs, crates/nvs-lsp/src/index.rs]
  - id: 6
    title: "a child process a Core\\Task child task leaves running is ended and reported when that task ends, and the grant-table marker is gone"
    status: open
    context: [docs/rules/core-classes/process-spawn.md, crates/nvs-runtime/src/ctx/held.rs, crates/nvs-stdlib/src/process.rs, docs/rules/security/authority-is-the-enclosing-namespace.md]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: regex-backtracking-config
    stage: 1
    name: "[regex] backtracking reads allow, warn or deny, defaults to allow, is System class, and is in the environment hash"
    run: [cargo, test, --test, request, regex_backtracking]
    tests:
      - regex_backtracking_defaults_to_allow_in_every_mode
      - regex_backtracking_refuses_a_value_not_in_its_list
      - regex_backtracking_is_system_class
      - regex_backtracking_is_in_the_environment_hash
  - id: regex-backtracking-compile
    stage: 1
    name: "a backtracking pattern literal compiles silently under allow, warns under warn and does not compile under deny"
    run: [cargo, test, --test, intrinsics, regex_backtracking]
    tests:
      - regex_backtracking_allow_compiles_a_backtracking_literal_silently
      - regex_backtracking_warn_reports_a_backtracking_literal_as_a_warning
      - regex_backtracking_deny_refuses_a_backtracking_literal
      - regex_backtracking_deny_compiles_a_linear_literal
  - id: regex-backtracking-cases
    stage: 2
    name: "the regex policy holds from a program: a denied literal does not compile, and a denied run-time pattern throws"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/regex-a-backtracking-literal-under-deny-does-not-compile.nvst, --case, tests/conformance/core/regex-a-backtracking-pattern-built-at-run-time-under-deny-throws.nvst, --case, tests/conformance/core/regex-a-linear-pattern-runs-under-deny.nvst]
    tests:
      - tests/conformance/reject/regex-a-backtracking-literal-under-deny-does-not-compile.nvst
      - tests/conformance/core/regex-a-backtracking-pattern-built-at-run-time-under-deny-throws.nvst
      - tests/conformance/core/regex-a-linear-pattern-runs-under-deny.nvst
  - id: regex-backtracking-proofs
    stage: 2
    name: "the [regex] backtracking directive owes no proof"
    run: [bun, nv, proofs, --id, "directive:regex.backtracking"]
  - id: socket-unit
    stage: 3
    name: "a privileged port gets a socket unit, the service holds no capability, and install and uninstall handle both files"
    run: [cargo, test, --bin, nvs, socket_activation]
    tests:
      - socket_activation_writes_a_socket_unit_for_a_privileged_port
      - socket_activation_leaves_the_service_no_capability
      - socket_activation_lists_every_tcp_address_and_no_unix_path
      - socket_activation_is_not_used_without_a_privileged_port
      - socket_activation_install_enables_the_socket_and_uninstall_removes_it
  - id: socket-adoption-linux
    stage: 3
    name: "nvs serve on Linux adopts the sockets systemd passed, and refuses one no listen entry names"
    run: [bun, nv, linux-test, --bin, nvs, adopting_]
    tests:
      - adopting_a_socket_answers_on_its_listen_address
      - adopting_a_socket_no_listen_entry_names_stops_the_start
      - adopting_ignores_listen_fds_meant_for_another_process
  - id: dap-task-tree
    stage: 4
    name: "nvs dap reports the task tree capability and answers each stopped task with its parent"
    run: [cargo, test, --test, dap, task_tree]
    tests:
      - initialize_reports_the_task_tree_capability
      - the_task_tree_names_each_task_s_parent_while_stopped
      - the_task_tree_is_one_main_task_while_nothing_is_stopped
  - id: vscode-task-tree
    stage: 4
    name: "the VS Code extension nests each task under the task that started it"
    run: [npm, --prefix, editors/vscode, run, "test:headless"]
    tests:
      - "nests each task under the task that started it"
      - "shows no task while no nvs session is stopped"
  - id: alias-rename-lsp
    stage: 5
    name: "the editor renames a class-scoped type alias at every site and refuses a name its owner already uses"
    run: [bun, nv, suite, tests/lsp/, --case, tests/lsp/rename/a-class-scoped-type-alias-is-renamed-at-every-site.lspt, --case, tests/lsp/rename/a-class-scoped-type-alias-is-refused-a-name-its-owner-uses.lspt, --case, tests/lsp/rename/prepare-rename-on-a-class-scoped-type-alias-names-the-alias.lspt]
    tests:
      - tests/lsp/rename/a-class-scoped-type-alias-is-renamed-at-every-site.lspt
      - tests/lsp/rename/a-class-scoped-type-alias-is-refused-a-name-its-owner-uses.lspt
      - tests/lsp/rename/prepare-rename-on-a-class-scoped-type-alias-names-the-alias.lspt
  - id: alias-rename-code
    stage: 5
    name: "nvs code rename renames a class-scoped type alias the way the editor does"
    run: [cargo, test, --test, code, code_rename_]
    tests:
      - code_rename_renames_a_class_scoped_type_alias_at_every_site
      - code_rename_refuses_a_class_scoped_alias_name_its_owner_uses
  - id: task-child-case
    stage: 6
    name: "a child process a Core\\Task child task leaves running is stopped and reported when that task ends"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/process-spawn-a-child-a-child-task-leaves-running-is-stopped-when-that-task-ends.nvst, --case, tests/conformance/core/process-spawn-a-handle-used-outside-its-task-names-no-other-child.nvst]
    tests:
      - tests/conformance/core/process-spawn-a-child-a-child-task-leaves-running-is-stopped-when-that-task-ends.nvst
      - tests/conformance/core/process-spawn-a-handle-used-outside-its-task-names-no-other-child.nvst
  - id: task-child-exit-status
    stage: 6
    name: "under nvs run a child task's report does not change the exit status"
    run: [cargo, test, --test, run_command, a_child_a_task_left_running]
    tests:
      - a_child_a_task_left_running_is_reported_and_the_run_still_exits_0
  - id: no-not-on-disk-left
    stage: 6
    name: "none of this goal's fragments still says a part is not built, and the editor no longer says the alias rename is not available"
    run: [git, grep, -n, -i, -E, "Not on disk|is not built|cannot be written yet|deferred rather than refused|not available yet", --, docs/rules/core-classes/regex-compile-time-tiering.md, docs/rules/packaging/the-generated-unit-is-hardened.md, docs/rules/security/authority-is-the-enclosing-namespace.md, docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md, docs/rules/core-classes/process-spawn.md, crates/nvs-lsp/src/rename.rs]
    exit: nonzero
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, runtime-leftovers]
    reach: true
    floor: false
---

# the regex deny directive, socket activation, the debugger's task tree, a task's child report and the owned-alias rename exist

## Why

The user decided that every unbuilt part a rule describes is built before the first release. Six
rules still say a part of them is missing, or the editor says so. Five are features an operator or
a developer reaches for: `[regex] backtracking = "deny"`, a service on port 443 that holds no
capability, the debugger's tree of tasks, the report of a process a child task left running, and
renaming a type alias a class declares. The sixth, the per-namespace grant table, is built and only
its marker is stale. Each stage builds one part, proves it, and deletes the marker and the gap
record `release-hygiene` filed for it.

## Standing decisions

**Gate.** Stage 1 starts once `docs/goals/release-hygiene.md` is gone, `bun nv owners --check` exits
0, and `bun nv owners --closes runtime-leftovers` lists this goal's gap records. Each item that
lands deletes the record `--closes` lists for it, in the item's own commit. If `release-hygiene` has
already removed a marker this goal names, the item builds the part anyway and says so in its commit.

**The six parts, and which are built.** Five are built here. The grant table is built: the checker
reads `[grants]` per namespace (`crates/nvs-types/src/capability.rs:235`), and
`tests/conformance/cap/grants-a-namespace-holds-only-what-it-is-granted.nvst` guards file-scope code,
an anonymous function and a `use` import. Its item only deletes the stale line. The child-task report
is mostly built: each `Core\Task` child runs on a context of its own, and that context's drop calls
`Ctx::end_abandoned_children` (`crates/nvs-runtime/src/ctx/mod.rs:1816`). Its item proves it with
tests, fixes the message, and closes the handle hole below.

**`[regex] backtracking`.** A new top-level block `[regex]` with one key, `backtracking`, whose
values are `"allow"`, `"warn"` and `"deny"`. The rule spells it so, and the block roster gains a row
for it (`rule:config/every-block-is-argued-where-it-is-added`).
- **Default `allow` in every mode.** It is today's behaviour, and the step budget already bounds a
  backtracking match (`rule:core-classes/regex-two-tiers`). A stricter default would break working
  programs. Fallback: none needed.
- **`System` class, `Apply::Reload`.** It changes what compiles, so no request may set it. Its value
  is fed into `env_hash` (`crates/nvs-config/src/cache.rs:186`), so a changed value is a different
  unit key and a reload recompiles. `rule:config/the-extension-set-is-in-every-unit-key` names it as
  an input.
- **Global only.** `[[app]]` does not take a `[regex]` block. A per-app policy is not described by
  any rule.
- **A module of its own,** `crates/nvs-config/src/regex.rs`, in the shape of `crates/nvs-config/src/errors.rs`:
  `KEY`, an enum `Backtracking { Allow, Warn, Deny }`, and `in_force`.
- **Compile time.** The checker gets the value through a new `nvs_types::Options` field beside
  `grants` (`crates/nvs-types/src/lib.rs:473`), filled by every caller that fills `grants` from the
  same configuration. A caller with no configuration passes `Allow`. At the literal site
  (`crates/nvs-types/src/intrinsics.rs:652`), `Tier::Backtracking` under `Warn` is a new warning
  code, the next free one in `crates/nvs-diagnostics/src/code/warnings.rs` (W1029 when this was
  written). Under `Deny` it is a new error code, the next free one in
  `crates/nvs-diagnostics/src/code/types_e08.rs` (E0893 when this was written).
- **Run time.** Under `Deny`, a pattern built at run time that only the backtracking engine accepts
  throws the same `RuntimeError` a malformed pattern throws, at the call that compiles it
  (`crates/nvs-stdlib/src/regex.rs:1189`). So under `deny` no request can backtrack at all, which is
  what the rule promises the operator. Under `Warn` and `Allow` a run-time pattern runs as today, and
  nothing is logged on the request path. The rule's "with none of the three benefits" sentence is
  rewritten to say this.
- **The messages,** to be written in that voice: the error "This pattern needs the backtracking
  regex engine. `[regex] backtracking` is `deny` in `nvs.toml`." with the help "Write the pattern
  without lookaround and without backreferences, or set `backtracking` to `warn` or `allow`." The
  warning says the first sentence and "`[regex] backtracking` is `warn` in `nvs.toml`." The run-time
  throw says the same as the error.
- **Proofs.** `directive:regex.backtracking` appears in `bun nv proofs` once its row is in
  `crates/nvs-config/src/directive.rs`. It owes a test, one example under
  `docs/examples/config/regex-backtracking/` and one attack under
  `tests/hostile/config/regex-backtracking/`. The attack: a request calls `Core\Config::set` on the
  key, which returns `false`, then builds a backtracking pattern at run time under `deny`, which
  throws.
- **Trade-offs.** Security: an operator can now rule out backtracking for the whole deployment.
  Latency: none at run time for a literal; one branch on the compile path of a run-time pattern.
  Memory: one byte per snapshot. Simplicity: one block, one key, three values.

**Socket activation.**
- **Only when a privileged port is in the set.** The set is what `listeners`
  (`crates/nvs-cli/src/service.rs:5038`) already computes, `--listen` and `--port` included. It is
  extended to return the TCP addresses. Without a privileged port nothing changes.
- **With one, `nvs service install` writes `<name>.socket` beside `<name>.service`.** It has one
  `ListenStream=` for each TCP address of the set, privileged or not, so TCP has one acquisition path.
  `<name>.service` gains `Requires=<name>.socket` and `After=<name>.socket`, keeps
  `CapabilityBoundingSet=`, and no longer carries `AmbientCapabilities=`. A Unix socket in the set is
  still bound by `nvs serve` under `RuntimeDirectory=`, as today. The `.socket` file's values are
  written through `unit_word` and `unit_path`, as every other unit value is.
- **Install and uninstall.** `install` writes both files, then `systemctl enable --now` names the
  socket first. `uninstall` disables and removes both. The stored-unit reader
  (`crates/nvs-cli/src/service.rs:2140`) finds the `.socket` file beside the service.
- **`nvs serve` adopts on Linux only.** At boot, before `bind_all` (`crates/nvs-cli/src/serve.rs:624`),
  it reads `LISTEN_PID` and `LISTEN_FDS`. It uses them only when `LISTEN_PID` is its own pid, then
  unsets both. Each passed socket is matched to a TCP listen entry by its local address. A passed
  socket that no entry names stops the start with "systemd passed a socket for `<address>`, and
  `[server] listen` does not list that address." An entry with no passed socket is bound as today. On
  macOS and Windows nothing changes. It needs no new dependency: the protocol is two variables and
  fds from 3.
- **A listen change needs a new install.** `server.listen` is `Apply::Boot` already, so a changed set
  means a restart, and under activation it also means `nvs service install` again. The reference says
  so.
- **Tests.** The unit text is pure and is tested on every platform in `service.rs`'s tests module.
  The adoption is Linux-only code (`#[cfg(target_os = "linux")]`), and its tests run in WSL with
  `bun nv linux-test` (AGENTS.md § *Code and tests hold on every CI runner*). A test passes its own
  pid in `LISTEN_PID` and a socket it bound at fd 3, never a real systemd.
- **Proofs.** `tools:server/nvs-service` owes nothing today. A second example shows the two files a
  privileged port gives. No new attack is owed.
- **Trade-offs.** Security: the service process never holds `CAP_NET_BIND_SERVICE`. Latency: none
  per request; a restart keeps the socket open, so no connection is refused during it. Memory: one
  fd per address. Simplicity: one more generated file, only for a privileged port.

**The debugger's task tree.**
- **A custom DAP request, `nvsTaskTree`,** answered while stopped. Its body is
  `{ "tasks": [ { "threadId", "parentThreadId", "name", "source", "line" } ] }`, where
  `parentThreadId` is absent for a root and `source` and `line` are the top frame's, mapped through
  `pathMappings`. While nothing is stopped it answers the one `main` task. `initialize` reports
  `supportsNvsTaskTree: true` (`crates/nvs-cli/src/dap/mod.rs:302`). The standard `threads`
  answer does not change.
- **The parent comes from the scheduler.** `Stop` (`crates/nvs-host/src/debug.rs:284`) gains
  `parent: Option<TaskId>`, filled from `Scheduler::parent_of` (`crates/nvs-host/src/scheduler.rs:1344`)
  when the task stops. A `spawn`ed isolate's root names the task that spawned it when the debugger
  has it, and is a root otherwise.
- **The VS Code view.** A tree view `nvs.tasks`, named "Novis Tasks", in the `debug` view
  container, shown when `debugType == nvs`. A debug adapter tracker refreshes it on each `stopped` and
  `continued` message. Selecting a task opens its top frame's file at its line. The view id is added
  and never renamed (`rule:ide/contributions-are-frozen-and-only-ever-added`). It is VS Code's own
  tree view fed with data, so `rule:ide/the-extension-builds-no-ui-the-editor-already-has` holds, and
  its Built list names it.
- **Proofs.** `tools:editor/nvs-dap` owes nothing today. Its `about.md` gains a paragraph and
  `docs/reference/tools/40-editor.md` § *nvs dap* names the request. No example or attack is owed.
- **Trade-offs.** Latency: one request per stop, only from the editor. Memory: one parent id per
  stopped task, freed on resume. Simplicity: one request and one view.

**The class-scoped alias rename.**
- **What renames.** `Owner::Name` declared as a `type` member (`rule:types/class-scoped-alias`). The
  same branch (`crates/nvs-lsp/src/rename.rs:387`) also refuses a class-scoped enum
  (`rule:enums/class-scoped-enum`). If an owned enum reaches it, it is built with the same code.
- **Sites.** The declaration, every `Owner::Name` written anywhere (the last segment changes), and
  every bare `Name` inside the owner's body that resolves to it. An owned alias has no file of its
  own, so nothing moves.
- **Refused names.** A built-in type name (`BUILT_IN`), and a name the owner already gives a
  constant, an enum case, an enum or an alias (the `E0304` set). Also refused: a name that a type
  declared in, or imported into, the owner's file already has, because a bare site in the body would
  then name two things. Each refusal says which name is taken and by what.
- **One function, two clients.** `nvs code rename Owner::Name NewName` reaches it through
  `crates/nvs-lsp/src/query.rs:326` (`rule:ide/one-server-two-thin-clients`). `AN_OWNED_ALIAS` is
  deleted. The `rename` verb's help (`crates/nvs-cli/src/code.rs:131`) names the alias.
- **Proofs.** `tools:cli/nvs-code` and `tools:editor/nvs-lsp` owe nothing today, and no new example
  or attack is owed.

**The child-task report.**
- **The rule stands as written.** A child left running by a `Core\Task` child task is ended and
  reported when that task ends. Under `nvs run` that report does not change the exit status. The
  task's end is not the program's end, and the exit status says how the program ended. Under
  `nvs serve` the record goes to the request's log.
- **The message names the task.** A child task's context reports "A process started with
  `Core\Process::spawn` was still running when the task that started it ended, so it was stopped.
  Call `wait()` or `kill()` on its handle before the task ends." The root keeps today's message.
- **A handle used in another task never names another child.** A handle's key indexes the table of
  the task that spawned it (`crates/nvs-runtime/src/ctx/held.rs:411`). A handle returned out of a
  child task would index the parent's table, and could name the parent's own child. If the checker
  already keeps a `Core\Process\Handle` inside its task, the item cites that and adds only the case.
  Otherwise the handle gets a slot with its context's identity, and a use from another context is
  the existing fault in `child_of` (`crates/nvs-stdlib/src/process.rs:1266`). Fallback: the same
  fault for every cross-task use.
- **Proofs.** `Core\Process::spawn` owes nothing today. Its `about.md` gains one sentence on child
  tasks.

**The text a user reads.** Every new diagnostic, help line, `about.md` paragraph, example comment
and attack comment follows AGENTS.md § *Text an end user reads*. `bun nv proofs --comments` runs on
each new `.nvs` file.

**Lanes.** Stages 1 and 2 share `crates/nvs-config` and run in order. Stages 3, 4 and 5 touch
disjoint files and may run as side goals. Stage 6 re-renders `docs/rules/core-classes.md`, as stage
2 does, so it runs after stage 2. Every stage that changes a fragment runs `bun nv rules --render`.

## Stages

### 1. the [regex] block exists, and a backtracking pattern literal is silent, a warning or a compile error as it says

**Does:** `nvs.toml` takes `[regex] backtracking`, and the checker reports a backtracking literal
by it.

- [x] `rule:core-classes/regex-compile-time-tiering`: the `Regex` struct and the `regex` field on `Config`, beside `[errors]`. `crates/nvs-config/src/tree.rs:100`, `crates/nvs-config/src/tree.rs:605`
- [x] `rule:core-classes/regex-compile-time-tiering`: `crates/nvs-config/src/regex.rs` with `KEY`, `Backtracking` and `in_force`, in `errors.rs`'s shape. `crates/nvs-config/src/regex.rs:1`, `crates/nvs-config/src/errors.rs:26`
- [x] `rule:config/three-changeability-classes`: the `regex.backtracking` row, `System` and `Reload`, and its default row. `crates/nvs-config/src/directive.rs:168`, `crates/nvs-config/src/defaults.rs:504`
- [x] `rule:config/the-extension-set-is-in-every-unit-key`: the value is fed into the environment hash, and the fragment names it. `crates/nvs-config/src/cache.rs:186`, `docs/rules/config/the-extension-set-is-in-every-unit-key.md:1`
- [x] The `nvs.toml` template gets the commented `[regex]` block, and `bun nv directives --check` and `--check-template` pass. `crates/nvs-config/src/default.toml:321`, `tools/nv/cmd/directives.ts:1`
- [x] The tests the `regex-backtracking-config` check names, beside the `errors.deprecated` tests. `crates/nvs-config/tests/request.rs:607`
- [x] `rule:core-classes/regex-compile-time-tiering`: the new `Options` field, filled where `grants` is. `crates/nvs-types/src/lib.rs:473`, `crates/nvs-cli/src/main.rs:1154`
- [x] The warning code and the error code, with their cards. `crates/nvs-diagnostics/src/code/warnings.rs:298`, `crates/nvs-diagnostics/src/code/types_e08.rs:1172`
- [x] The literal site reports `Tier::Backtracking` under `Warn` and `Deny`. `crates/nvs-types/src/intrinsics.rs:652`
- [x] The tests the `regex-backtracking-compile` check names, each run once by name. `crates/nvs-types/tests/intrinsics.rs:659`

### 2. under deny a run-time pattern that needs backtracking throws, and the directive has its docs and feature proofs

**Does:** `deny` holds for a pattern built at run time too, and the directive is documented and
proven.

**Split:** the example and the attack are two similar files. One writing subagent writes both once
the run-time throw is landed. It builds nothing.

- [x] `rule:core-classes/regex-compile-time-tiering`: the compile path reads the policy and throws under `Deny` before the backtracking engine is built. `crates/nvs-stdlib/src/regex.rs:1189`
- [x] The three conformance cases the `regex-backtracking-cases` check names, each run once by name. `tests/conformance/reject/regex-a-backtracking-literal-under-deny-does-not-compile.nvst:1`, `tests/conformance/core/regex-a-backtracking-pattern-built-at-run-time-under-deny-throws.nvst:1`, `tests/conformance/core/regex-a-linear-pattern-runs-under-deny.nvst:1`
- [x] `rule:config/every-block-is-argued-where-it-is-added`: a `[regex]` row in the roster table. Landed with stage 1. `docs/rules/config/every-block-is-argued-where-it-is-added.md:30`
- [x] The config reference's block table names `[regex]`, and `Core\Regex::compile`'s `about.md` says what `deny` does. `docs/reference/tools/20-config.md:56`, `docs/examples/core/Regex/compile/about.md:1`
- [x] The example and the attack the standing decision names, with `bun nv proofs --comments` passing on both. `docs/examples/config/regex-backtracking/about.md:1`, `docs/examples/config/regex-backtracking/01-a-server-that-allows-no-backtracking.nvs:1`, `tests/hostile/config/regex-backtracking/01-a-request-tries-to-backtrack-anyway.nvs:1`
- [x] `bun nv proofs --id 'directive:regex.backtracking'` owes nothing. `crates/nvs-config/src/directive.rs:168`
- [x] The fragment's last paragraph and its run-time paragraph are rewritten to say what is built, the conformance cases join `guardedBy`, then `bun nv rules --render` and `bun nv render --website`. Delete the gap record `--closes` lists for it. `docs/rules/core-classes/regex-compile-time-tiering.md:14`, `data/rules/core-classes/regex-compile-time-tiering.json:1`

### 3. a service on a privileged port gets its socket from a systemd socket unit and holds no capability

**Does:** `nvs service install` writes a socket unit for a privileged port, and `nvs serve` adopts
what systemd passes it.

**Split:** none. The adoption tests run in WSL with `bun nv linux-test --bin nvs adopting_`, once,
after the build is green.

- [ ] `rule:packaging/the-generated-unit-is-hardened`: `listeners` also returns the TCP addresses, and `Plan` carries them. `crates/nvs-cli/src/service.rs:5038`, `crates/nvs-cli/src/service.rs:518`
- [ ] The `.socket` text, and `unit` adds `Requires=` and `After=` and drops `AmbientCapabilities=` when a socket unit is written. `crates/nvs-cli/src/service.rs:923`, `crates/nvs-cli/src/service.rs:985`
- [ ] `destination`, the install actions and the uninstall actions handle both files. `crates/nvs-cli/src/service.rs:1798`, `crates/nvs-cli/src/service.rs:2691`, `crates/nvs-cli/src/service.rs:2795`
- [ ] The tests the `socket-unit` check names, and the old `AmbientCapabilities` assertion becomes their first. `crates/nvs-cli/src/service.rs:5766`
- [ ] `nvs serve` adopts the passed sockets on Linux before `bind_all`. `crates/nvs-cli/src/serve.rs:624`, `crates/nvs-cli/src/serve.rs:2858`
- [ ] The tests the `socket-adoption-linux` check names, run with `bun nv linux-test`. `crates/nvs-cli/src/serve.rs:3731`
- [ ] The server reference and the `nvs service` feature describe the socket unit, with a second example. `bun nv proofs --id 'tools:server/nvs-service'` owes nothing. `docs/reference/tools/25-server.md:391`, `docs/examples/tools/server/nvs-service/about.md:1`, `docs/examples/tools/server/nvs-service/02-a-service-on-port-443.nvs:1`
- [ ] The fragment's example unit, its `AmbientCapabilities` paragraph and its last paragraph say what is built, then `bun nv rules --render`. Delete the gap record `--closes` lists for it. `docs/rules/packaging/the-generated-unit-is-hardened.md:77`

### 4. nvs dap answers the task tree, and the VS Code extension shows it

**Does:** the adapter answers `nvsTaskTree`, and the extension shows the tasks as a tree.

**Split:** the adapter (Rust) lands first. One writing subagent then writes the VS Code view and its
test, which touch only `editors/vscode/`. It builds nothing.

- [ ] `rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`: `Stop` gains `parent`, filled from the scheduler. `crates/nvs-host/src/debug.rs:284`, `crates/nvs-host/src/scheduler.rs:1344`
- [ ] `initialize` reports `supportsNvsTaskTree`, and `nvsTaskTree` is answered from the round. `crates/nvs-cli/src/dap/mod.rs:302`, `crates/nvs-cli/src/dap/mod.rs:356`, `crates/nvs-cli/src/dap/stopped.rs:128`
- [ ] The tests the `dap-task-tree` check names. `crates/nvs-cli/tests/dap.rs:609`
- [ ] The `nvs.tasks` view, its tracker and its contribution. `editors/vscode/src/debug.ts:34`, `editors/vscode/src/tasktree.ts:1`, `editors/vscode/package.json:313`
- [ ] The tests the `vscode-task-tree` check names. `editors/vscode/test/client/tasktree.test.ts:1`
- [ ] `rule:ide/the-extension-builds-no-ui-the-editor-already-has` names the view in its Built list. `docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md:16`
- [ ] The editor reference and the `nvs dap` feature name the request and the view. `bun nv proofs --id 'tools:editor/nvs-dap'` owes nothing. `docs/reference/tools/40-editor.md:296`, `docs/examples/tools/editor/nvs-dap/about.md:1`
- [ ] The fragment's task bullet says what is built, the tests join `guardedBy`, then `bun nv rules --render`. Delete the gap record `--closes` lists for it. `docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md:26`, `data/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.json:1`

### 5. a type alias a class declares is renamed by the editor and by nvs code rename

**Does:** a rename on `Owner::Name` changes every site, and refuses a name that is taken.

**Split:** the three `.lspt` cases are similar files. One writing subagent writes them once the
rename is landed. It builds nothing.

- [ ] `rule:ide/no-refactoring-introduces-an-alias`: the owned-alias subject, its sites and its refusals, and `AN_OWNED_ALIAS` deleted. `crates/nvs-lsp/src/rename.rs:108`, `crates/nvs-lsp/src/rename.rs:387`
- [ ] The index gives every bare site inside the owner's body. `crates/nvs-lsp/src/index.rs:1712`
- [ ] The `.lspt` cases the `alias-rename-lsp` check names. `tests/lsp/rename/a-class-scoped-type-alias-is-renamed-at-every-site.lspt:1`, `tests/lsp/rename/a-class-scoped-type-alias-is-refused-a-name-its-owner-uses.lspt:1`, `tests/lsp/rename/prepare-rename-on-a-class-scoped-type-alias-names-the-alias.lspt:1`
- [ ] `nvs code rename` reaches it by name, its help names the alias, and the tests the `alias-rename-code` check names. `crates/nvs-lsp/src/query.rs:326`, `crates/nvs-cli/src/code.rs:131`, `crates/nvs-cli/tests/code.rs:477`
- [ ] The CLI reference's `nvs code rename` lists the alias. `docs/reference/tools/10-cli.md:1045`
- [ ] The rule's Built list names the class-scoped alias, the cases join `guardedBy`, then `bun nv rules --render`. Delete the gap record `--closes` lists for it. `docs/rules/ide/no-refactoring-introduces-an-alias.md:18`, `data/rules/ide/no-refactoring-introduces-an-alias.json:1`

### 6. a child process a Core\Task child task leaves running is ended and reported when that task ends, and the grant-table marker is gone

**Does:** a child task's abandoned process is proven ended and reported at the task's end, a handle
cannot cross tasks, and the stale grant-table line is deleted.

Two groups that share no file: the child-task report (items 1 to 5), then the grant marker (item 6).

- [ ] `rule:core-classes/process-spawn`: a child task's context reports the task message, the root keeps its own. `crates/nvs-runtime/src/ctx/held.rs:470`, `crates/nvs-runtime/src/ctx/held.rs:489`
- [ ] A handle used outside its task never names another child, as the standing decision says. `crates/nvs-stdlib/src/process.rs:1240`, `crates/nvs-stdlib/src/process.rs:1266`
- [ ] The two conformance cases the `task-child-case` check names, each run once by name. `tests/conformance/core/process-spawn-a-child-a-child-task-leaves-running-is-stopped-when-that-task-ends.nvst:1`, `tests/conformance/core/process-spawn-a-handle-used-outside-its-task-names-no-other-child.nvst:1`
- [ ] The test the `task-child-exit-status` check names. `crates/nvs-cli/tests/run_command.rs:48`, `crates/nvs-cli/src/main.rs:2647`
- [ ] The fragment's Not-on-disk paragraph becomes present-tense prose, the cases join `guardedBy`, and `Core\Process::spawn`'s `about.md` gains its sentence. Then `bun nv rules --render`, and delete the gap record `--closes` lists for it. `docs/rules/core-classes/process-spawn.md:35`, `data/rules/core-classes/process-spawn.json:1`, `docs/examples/core/Process/spawn/about.md:1`
- [ ] `rule:security/authority-is-the-enclosing-namespace`: delete the stale Not-on-disk line, then `bun nv rules --render`, and delete the gap record `--closes` lists for it. `docs/rules/security/authority-is-the-enclosing-namespace.md:16`
- [ ] The `no-not-on-disk-left` check and `bun nv owners --closes runtime-leftovers` pass. `tools/nv/cmd/owners.ts:1`

### 7. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done, and all four regex checks are green. Under `deny` a pattern built at run
time that only the backtracking engine accepts throws a `RuntimeError` at the call that compiles
it, before that engine is built, on a cache hit as on a miss (`compiled_prepared` and `build`'s
`Backtrack` argument in `crates/nvs-stdlib/src/regex.rs`). A malformed pattern keeps its own
message. Only `Core\Regex::compile` takes a checked literal, so a literal given to `matches` or
`replace` meets the run-time throw. The three conformance cases pass, the reference, the template,
`Core\Regex::compile`'s `about.md` and the fragment say so, and gap
`nvs-stdlib/regex-deny-at-run-time` is deleted. `directive:regex.backtracking` owes no proof.
The sweep's `env-secret-proofs` red is stale: no such check and no log exist.

### Next

Stage 3, socket activation, in `crates/nvs-cli/src/service.rs` and `crates/nvs-cli/src/serve.rs`.
Stages 4 and 5 touch disjoint files and may run beside it.

- [ ] `listeners` also returns the TCP addresses, and `Plan` carries them. `crates/nvs-cli/src/service.rs:5042`, `crates/nvs-cli/src/service.rs:520`
- [ ] The `.socket` text; `unit` adds `Requires=` and `After=` and drops `AmbientCapabilities=` when a socket unit is written. `crates/nvs-cli/src/service.rs:927`, `crates/nvs-cli/src/service.rs:990`
- [ ] `destination`, install and uninstall handle both files. `crates/nvs-cli/src/service.rs:1802`
- [ ] The `socket-unit` tests; the old `AmbientCapabilities` assertion becomes their first. `crates/nvs-cli/src/service.rs:5770`
- [ ] `nvs serve` adopts passed sockets on Linux before `bind_all`, and the `socket-adoption-linux` tests run with `bun nv linux-test`. `crates/nvs-cli/src/serve.rs:624`, `crates/nvs-cli/src/serve.rs:2858`

No test is expected to go red.

### Backlog

- A `Core\IO\File` handle is a key into its task's table too (`crates/nvs-runtime/src/ctx/held.rs:370`). If stage 6 confirms the cross-task hole for a process handle, file a gap record for the file handle under `data/gaps/nvs-runtime/`.
