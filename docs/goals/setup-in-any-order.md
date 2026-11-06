---
looper: 1
title: "Setting up Novis works in any order, and nvs asks for admin rights itself"
summary: "Reached when the order test runs every order of nvs run, nvs init and nvs service install and the service starts with its data folder every time, nvs init sets up the data folder and the project file, and a command that needs admin rights asks for them itself."
context: [crates/nvs-config/src/trust.rs, crates/nvs-config/src/data.rs, docs/rules/config/ownership-is-the-trust-boundary.md]
stages:
  - id: 1
    title: "an owner that is an administrator passes the ownership check on Windows"
    status: done
    context: [crates/nvs-config/tests/trust.rs]
  - id: 2
    title: "the folder above a checked folder fails only when another account can replace that folder"
    status: done
    context: [crates/nvs-config/tests/trust.rs, crates/nvs-cli/tests/init.rs]
  - id: 3
    title: "nvs init sets up the data folder and the project file, and offers to restrict its folder"
    status: done
    context: [crates/nvs-cli/src/config.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/tests/init.rs]
  - id: 4
    title: "a command that needs admin rights asks for them itself"
    status: open
    context: [crates/nvs-cli/src/service.rs, crates/nvs-cli/src/main.rs, tools/nv/cmd/lints.ts, docs/rules/packaging/the-installer-is-a-sink.md]
  - id: 5
    title: "the order test passes, and the install guide shows the short path"
    status: open
    context: [crates/nvs-cli/src/service.rs, docs/reference/tools/15-install.md, website/src/content/docs/install/server.mdx]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: admin-owner
    stage: 1
    name: an owner in Administrators passes, and an owner outside it is refused
    run: [cargo, test, --test, trust]
    timeout: 1800
    tests:
      - an_owner_in_administrators_passes_for_any_account
      - an_owner_outside_administrators_is_refused_for_another_account
  - id: replace-only-parent
    stage: 2
    name: a parent that lets other accounts write but not replace the folder passes
    run: [cargo, test, --test, trust]
    timeout: 1800
    tests:
      - a_parent_with_modify_for_every_account_does_not_fail_the_folder_inside_it
      - a_parent_that_lets_every_account_delete_entries_fails_the_folder_inside_it
  - id: init-sets-up
    stage: 3
    name: nvs init creates the data folder and the project file, and a second run changes nothing
    run: [cargo, test, --test, init]
    timeout: 1800
    tests:
      - nvs_init_creates_the_data_folder_and_the_project_file
      - a_second_nvs_init_changes_nothing_and_reports_the_same
  - id: elevation
    stage: 4
    name: only the commands on the list ask for admin rights, and none asks where no prompt can show
    run: [cargo, test, --bin, nvs, "elevate::"]
    timeout: 1800
    tests:
      - only_listed_commands_ask_for_admin_rights
      - no_command_asks_where_no_prompt_can_show
  - id: any-order
    stage: 5
    name: every order of run, init and service install leaves a setup the service accepts
    run: [cargo, test, --test, setup_order]
    timeout: 1800
    tests:
      - every_order_of_run_init_and_install_leaves_a_setup_the_service_accepts
---

# Setting up Novis works in any order, and nvs asks for admin rights itself

## Why

The user set up a Windows server on 2026-11-06 and hit three problems in one session.

1. `nvs init` refused because the folder above the project folder, `D:\srv`, gave `Authenticated
   Users` the right to change it. Its help told the user to change `D:\srv`, a folder other projects
   share. `4366e4370d` fixed `nvs init` to check only its own folder, as `nvs serve` does.
2. The first `nvs run` in a normal prompt creates `.nvsdata` owned by the user's own account. A
   service account accepts only itself, `Administrators` or `SYSTEM` as owner, so a later `nvs service
   install` leaves a data folder the service does not accept. Only an elevated prompt creates the
   folder with `Administrators` as owner (`crates/nvs-config/src/data.rs:835`). So the order of
   commands decides whether the service works.
3. Every step that needs admin rights needs a prompt the user starts by hand with "Run as
   administrator".

The user's rules for this goal:

- **No order of commands may make a later step fail.** A user will run `nvs run` before the install.
- **`nvs init` is the recommended first step**, and creates what Novis needs to run. Skipping it
  works, but then nobody controls who creates the data folder, or when.
- **The checks stay safe but lose what guards nothing.** The user doubts the strict install checks.
- **A command that needs admin rights asks for them itself.** The user accepts a UAC prompt and
  never starts an elevated terminal by hand.

## Standing decisions

1. **The one rule Novis keeps:** no other account can change what Novis reads. The configuration
   file and its folder, the cache folder and the data folder keep the full write test. This goal
   changes who may own a path (decision 2) and what the folder *above* a folder is tested for
   (decision 3). Nothing else is relaxed.
2. **An owner in `Administrators` passes on Windows**, for any account the check judges for. An
   administrator can take ownership of any file, so trusting one as owner gives nobody a new right.
   Membership is resolved with `NetUserGetLocalGroups` and `LG_INCLUDE_INDIRECT`, so a domain account
   in a nested group counts. It costs one lookup per path whose owner is not the account, `SYSTEM` or
   `Administrators`, at start and at reload only, never per request. A lookup that fails counts as
   not a member, so the check fails closed. `give_to_administrators` and the special case for an
   elevated prompt are deleted, because no owner needs changing any more. Linux and macOS keep
   `root` or the account: a normal user cannot create `.nvsdata` beside a binary in `/usr/local/bin`.
   Its home is `rule:config/ownership-is-the-trust-boundary`.
3. **The folder above a checked folder fails only when another account can replace the folder.**
   On Windows that is `FILE_DELETE_CHILD`, `WRITE_DAC` or `WRITE_OWNER` on the parent for one of the
   untrusted groups. The `Modify` right Windows gives `Authenticated Users` on a non-system drive has
   none of these, so it passes. On Unix it is the write bit for group or others, unless the sticky
   bit is set and the folder's owner passes. The folder's own `DELETE` right stays in its own write
   test. The configuration file's own folder keeps the full write test, because an account that can
   add a file there can add one the configuration reads.
4. **One function creates the data folder, whichever command runs first.** `nvs init` calls the
   same `Folder::prepare` that `nvs run` calls (`crates/nvs-config/src/data.rs:333`), so the result is
   the same whoever runs first. `nvs init` adds `init` to the commands in `prepares`
   (`crates/nvs-cli/src/main.rs:356`).
5. **`nvs init` is the setup command, and running it again is a check.** It creates the data folder,
   writes the project `nvs.toml` where it writes it today, checks what `nvs serve` will check, and
   prints one summary line per path: created, kept, or failed with its fix. A second run creates
   nothing, overwrites nothing, and prints the same summary.
6. **`nvs init` offers to restrict the one folder it writes into, and no other.** When that folder
   fails the write test, an interactive terminal asks `Restrict <folder> so only you and
   administrators can change it? [Y/n]`. `--yes` answers yes. Without a terminal it prints the two
   `icacls` commands, or `chmod go-w`, and exits `1` as today. It never changes a folder above it.
7. **The commands that may ask for admin rights are a closed list:** `nvs service install`,
   `uninstall`, `start` and `stop`, and `nvs init` when it must change a folder the user does not own.
   `nvs run`, `nvs serve`, `nvs test` and every other command never ask, because program code would
   then run as administrator. The list is one `match` in `crates/nvs-cli/src/elevate.rs`.
8. **How Windows asks:** `ShellExecuteExW` with the verb `runas` starts the same `current_exe` with
   the same arguments plus a hidden `--elevated-for <pid>`, with its window hidden. The first process
   waits and returns the elevated process's exit code. The elevated process re-parses its arguments
   and runs every check again, trusting nothing the first process found. Stage 4 starts with a probe
   of how the output reaches the user's terminal: `AttachConsole(<pid>)` in the elevated process
   first, and a named pipe whose DACL admits only the user's SID as the fallback. The session picks
   whichever works in Windows Terminal and in `cmd.exe`, and records the result in the rule.
9. **Where no prompt can show, nothing waits.** Over SSH, in a service, in CI, without a console,
   or with `--no-elevate`, the command prints one line, `run this in a terminal started as
   administrator:`, and the command, and exits `1`. On Linux and macOS the same commands run `sudo`
   with the same arguments in the same terminal, and print the line when `sudo` is missing or there
   is no terminal. Its home is a new rule, `packaging/a-command-that-needs-admin-rights-asks-for-them`.
10. **The Win32 calls live in `nvs-cli`**, which is already on the unsafe roster
    (`tools/nv/cmd/lints.ts:96`). Its reason string gains "asking for admin rights".
11. **Lanes:** stages 1 and 2 both edit `crates/nvs-config/src/trust.rs` and run in order. Stage 3
    edits `crates/nvs-cli/src/config.rs` and may run beside stages 1 and 2. Stage 4 edits
    `crates/nvs-cli/src/service.rs` and a new `elevate.rs`, and may run beside stage 3. Stage 5 runs
    last, after all four.

## Stages

### 1. an owner that is an administrator passes the ownership check on Windows

**Does:** decision 2.

- [x] The owner test accepts a member of `Administrators`, through one membership lookup.
  `crates/nvs-config/src/trust.rs:887`
- [x] `give_to_administrators` and its calls are deleted, with the module doc's paragraph about it.
  `crates/nvs-config/src/data.rs:700` `crates/nvs-cli/src/config.rs:347`
- [x] The two named tests. The second one uses `LOCAL SERVICE` as the judged account and a scratch
  folder owned by the test's own account. If that account is itself an administrator, the second test
  is skipped with a printed reason. `crates/nvs-config/tests/trust.rs:1`
- [x] The rule fragment says who may own a path, and why an administrator may.
  `docs/rules/config/ownership-is-the-trust-boundary.md:26`

### 2. the folder above a checked folder fails only when another account can replace that folder

**Does:** decision 3.

- [x] A parent test separate from the full one, for Windows and for Unix. `check_as` uses it for the
  parent. `crates/nvs-config/src/trust.rs:172` `crates/nvs-config/src/trust.rs:448`
  `crates/nvs-config/src/trust.rs:798`
- [x] The two named tests. `crates/nvs-config/tests/trust.rs:1`
- [x] The cache half of `nvs_init_checks_its_folder_alone_the_cache_one_folder_up_and_nvs_run_not_the_config`
  is updated: a parent with write but no replace right no longer warns. `crates/nvs-cli/tests/init.rs:347`
- [x] The rule fragment and the install reference's table say what the parent is tested for.
  `docs/rules/config/ownership-is-the-trust-boundary.md:1` `docs/reference/tools/15-install.md:121`

### 3. nvs init sets up the data folder and the project file, and offers to restrict its folder

**Does:** decisions 4, 5 and 6.

- [x] `init` joins `prepares`, and its summary prints one line per path.
  `crates/nvs-cli/src/main.rs:356` `crates/nvs-cli/src/config.rs:399`
- [x] The question, `--yes`, and the printed commands without a terminal. `crates/nvs-cli/src/config.rs:399`
- [x] The two named tests, and one for `--yes` on a folder every account may change.
  `crates/nvs-cli/tests/init.rs:1`
- [x] The help text of `nvs init` and `--yes`, held to AGENTS.md § *Text an end user reads*.
  `crates/nvs-cli/src/main.rs:1`

### 4. a command that needs admin rights asks for them itself

**Does:** decisions 7, 8, 9 and 10.

- [ ] The probe of decision 8, run by hand in Windows Terminal and in `cmd.exe`. Its result goes
  into the new rule. `crates/nvs-cli/src/elevate.rs:1`
- [ ] `elevate.rs`: the closed list, the check for an elevated token (`CheckTokenMembership` with a
  null token; stage 1 deleted the copy in `trust.rs`, so `elevate.rs` writes its own),
  `runas` with the wait and the exit code, the output path the probe chose, and `sudo` on Unix.
  `crates/nvs-cli/src/elevate.rs:1`
- [ ] The service verbs and `nvs init` call it before they need the rights.
  `crates/nvs-cli/src/service.rs:446` `crates/nvs-cli/src/config.rs:399`
- [ ] `--no-elevate`, and the two named tests, which check the list and the no-prompt cases without
  starting a prompt. `crates/nvs-cli/src/elevate.rs:1`
- [ ] The rule `packaging/a-command-that-needs-admin-rights-asks-for-them`, and the unsafe roster's
  reason for `nvs-cli`. `docs/rules/packaging/the-installer-is-a-sink.md:1` `tools/nv/cmd/lints.ts:96`

### 5. the order test passes, and the install guide shows the short path

**Does:** proves the user's first rule, and rewrites the install steps around `nvs init`.

**Split:** after the test is green, one writing subagent rewrites the install reference and the
website's install page, and another the Windows install example. Their files are disjoint.

- [ ] `crates/nvs-cli/tests/setup_order.rs`: every order of `nvs run`, `nvs init` and the install
  with the recording manager (`crates/nvs-cli/src/service.rs:5588`), then `trust::check_as` for
  `LOCAL SERVICE` on the data folder, its `nvs.toml`, the cache folder and the project file.
  `crates/nvs-cli/tests/setup_order.rs:1`
- [ ] The install reference: put `nvs` in a folder, run `nvs init`, run `nvs service install`. The
  `icacls` steps stay only for a layout `nvs init` cannot restrict. `docs/reference/tools/15-install.md:142`
- [ ] The website's install page, the same way. `website/src/content/docs/install/server.mdx:120`
- [ ] The Windows install example and its output. `docs/examples/tools/install/windows/about.md:1`

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1, 2 and 3 are done. Stage 3: `Folder::prepare` returns `Prepared::{Created, Kept}` and
`Unusable` keeps the check's help. `main` calls `nvs_config::data::prepare` for `init` like every
command in `prepares`, skips the warning for `init`, and hands the result to `config::init`
(`crates/nvs-cli/src/config.rs:405`). That prints `created|kept the data folder `…``, then
`created|kept the project file `…``, and exits `1` if either failed. A file already there is kept
and given `trust::check` (file and folder), so a second run exits `0`. `Untrusted::Breach` gained
`repair: Option<Box<Repair>>` (path, and the SID on Windows). `trust.rs` sets it for a group's write
right on Windows and for a Unix mode this account may `chmod` without `sudo`. `restrict_folder`
(`config.rs:524`) asks on a terminal or takes `--yes`. It applies `icacls /inheritance:d` and
`/remove:g <SID>` from `System32`, or `chmod go-w`, to the checked folder only, then checks again.
A refusal with a repair adds `help: `nvs init --yes` runs these commands for you`. Row 4 of the install
table ("it already exists") is gone from `docs/reference/tools/15-install.md` and the website page.
The rule fragment has the paragraph. Ran by name, green on Windows: the three new `init` tests and
`each_refusal_names_a_path_…`; `looper check --name init-sets-up` is green. The Unix half of
`restrict` has not compiled or run. Stage 6 runs it, or `bun nv linux-test`.

The sweep's `env-secret-proofs` red is stale state in looper's service. No check by that name exists
in the tree or its history since json-streaming retired, and `looper check --name env-secret-proofs`
says `no check matches`. `bun nv rules --check` is red on `docs/goals/byte-sources.md:194,201,456`,
another goal's citations.

### Next

**Stage 4**, over `crates/nvs-cli/src/service.rs`, a new `crates/nvs-cli/src/elevate.rs` and
`tools/nv/cmd/lints.ts`; no gate.

- [ ] The probe of decision 8 by hand, and its result into the new rule. `crates/nvs-cli/src/elevate.rs:1`
- [ ] The closed list and the asking itself. `crates/nvs-cli/src/elevate.rs:1`
  The elevated-token check, `runas` with wait and exit code, the output path, `sudo` on Unix.
- [ ] The service verbs call it, at `Args::run` (`crates/nvs-cli/src/service.rs:446`). `nvs init`
  calls it only when `restrict` fails with access denied on a folder another account owns
  (`crates/nvs-cli/src/config.rs:524`, `restrict_folder`).
- [ ] `--no-elevate` and the two named tests. `crates/nvs-cli/src/elevate.rs:1`
- [ ] The rule and the roster reason. `docs/rules/packaging/the-installer-is-a-sink.md:1` `tools/nv/cmd/lints.ts:96`

Expected red until stage 6: none known. Not run since the change: `the_icacls_steps_…`
(`crates/nvs-cli/tests/init.rs`) and `nvs_init_checks_its_folder_alone_…`, whose refusals now
also carry the `--yes` help line, which they do not assert against.

### Backlog

- Whether `.nvsdata` beside the binary stays the default on Linux, where a normal user cannot
  create it next to `/usr/local/bin/nvs`, is not this goal's question.
