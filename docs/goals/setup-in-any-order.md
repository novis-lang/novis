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
    status: done
    context: [crates/nvs-cli/src/service.rs, crates/nvs-cli/src/main.rs, tools/nv/cmd/lints.ts, docs/rules/packaging/the-installer-is-a-sink.md]
  - id: 5
    title: "the order test passes, and the install guide shows the short path"
    status: done
    context: [crates/nvs-cli/src/service.rs, docs/reference/tools/15-install.md, website/src/content/docs/install/server.mdx]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: done
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
12. **The output path is `AttachConsole`, chosen without the hand probe.** An unattended session
    cannot answer a consent prompt, so decision 8's probe could not run. The attach needs no channel
    of its own and creates nothing another account can open, and it is how gsudo's attached mode
    reaches Windows Terminal and `cmd.exe` from an elevated process. The second process then opens
    `CONOUT$` and `CONIN$` as its standard handles. The named pipe stays the fallback if a person
    finds a terminal where the attach fails. A failed attach loses the output and still returns the
    exit code. Decision 9's "a service" is the no-terminal case, so no session-0 check is added.
    The Unix line is `run this as root:` and the command with `sudo` in front.
13. **The order test runs the built binary, and the install in it is `--dry-run`.** The acceptance
    check is `cargo test --test setup_order`, and `nvs-cli` has no library target, so the recording
    manager is out of reach. A real install would register a service on an elevated CI runner. The
    dry run still runs the install's judgement for the service's account, and the folder it would
    create is the same `Folder::prepare` that `nvs run` and `nvs init` call (decision 4), which
    `an_install_prepares_the_data_folder_…` in `service.rs` pins. The test is `#[cfg(windows)]`: on
    Linux the install gives each path to the service's account itself, so no order can break it.

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

- [x] The probe of decision 8 could not run unattended; decision 12 records the choice, and the
  backlog keeps the hand check. `crates/nvs-cli/src/elevate.rs:1`
- [x] `elevate.rs`: the closed list, the check for an elevated token (`CheckTokenMembership` with a
  null token; stage 1 deleted the copy in `trust.rs`, so `elevate.rs` writes its own),
  `runas` with the wait and the exit code, the output path the probe chose, and `sudo` on Unix.
  `crates/nvs-cli/src/elevate.rs:1`
- [x] The service verbs and `nvs init` call it before they need the rights.
  `crates/nvs-cli/src/service.rs:446` `crates/nvs-cli/src/config.rs:399`
- [x] `--no-elevate`, and the two named tests, which check the list and the no-prompt cases without
  starting a prompt. `crates/nvs-cli/src/elevate.rs:1`
- [x] The rule `packaging/a-command-that-needs-admin-rights-asks-for-them`, and the unsafe roster's
  reason for `nvs-cli`. `docs/rules/packaging/the-installer-is-a-sink.md:1` `tools/nv/cmd/lints.ts:96`

### 5. the order test passes, and the install guide shows the short path

**Does:** proves the user's first rule, and rewrites the install steps around `nvs init`.

**Split:** after the test is green, one writing subagent rewrites the install reference and the
website's install page, and another the Windows install example. Their files are disjoint.

- [x] `crates/nvs-cli/tests/setup_order.rs`: every order of `nvs run`, `nvs init` and
  `service install --dry-run` through the binary (decision 13), then `trust::check_as` for
  `LOCAL SERVICE` on the data folder, its `nvs.toml`, the cache folder and the project file.
  `crates/nvs-cli/tests/setup_order.rs:1`
- [x] The install reference: put `nvs` in a folder, run `nvs init`, run `nvs service install`. The
  `icacls` steps stay only for a layout `nvs init` cannot restrict. `docs/reference/tools/15-install.md:8`
- [x] The website's install page, the same way. `website/src/content/docs/install/server.mdx:13`
- [x] The Windows install example and its output. `docs/examples/tools/install/windows/about.md:1`

### 6. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done, and all five acceptance checks are green (`looper check --name` each). The gate's verify found five reds, each fixed at its cause: `prepares` lists `init`
(`crates/nvs-cli/src/main.rs` test), the named-folder trust test grants a right that can replace
the folder (`crates/nvs-config/src/data.rs`, `replaceable_by_others`), the junction helper gives
`cmd` backslash paths (`crates/nvs-cli/tests/live_edit.rs`, `point`), the spreadsheet reader's
requests get a one-day CPU deadline (`crates/nvs-ext/tests/spreadsheet_read.rs`, `CPU`), and
`nvs_coverage`'s check-cfg moved to `[workspace.lints.rust]` with `bun nv lints` rendering a list.
The last full verify was 23 of 23 green. A later run's one red was another session's untracked
`tools/nv/cmd/walkthrough.ts`.

Goal-end gates: `bun nv verify --doc` is green after the `platform::attach` link fix in
`elevate.rs:14`, `bun nv owners --closes` names no gap. `bun nv ci --check` has no red of this
goal's: `tools:install/set-up-a-server` now has its proofs, the Windows `about.md` fits its bound,
and both install pages' questions are stamped. What stays red there is other sessions' work: stale
perf on `arr.rs`, `reflect.rs` and `queue.rs`, the walkthrough pages, the `http`, `io` and `json`
pages' questions, and four chain goals missing from `roadmap.json`'s themes. `bun nv linux-test`
built the Unix half of `elevate.rs` and `restrict`, and `--test init` (7) and `elevate::` (2) pass
on Linux.

The sweep's nightly Miri reds were three 8 MiB JSON ladder tests timing out at 300 s.
`request.rs`'s one is committed with `cfg_attr(miri, ignore)`. The two in
`crates/nvs-stdlib/src/json.rs` carry the same attribute in the tree, uncommitted: the guard
refused to stage that file while session goal `json-streaming` claims it.

### Next

Nothing in this goal. The goal is reached. The `json.rs` Miri attributes go in with
`json-streaming`'s next commit, or in a commit of their own once its claim ends.

### Backlog

- Whether `.nvsdata` beside the binary stays the default on Linux, where a normal user cannot
  create it next to `/usr/local/bin/nvs`, is not this goal's question.
- Decision 8's hand probe: a person runs `nvs service start <name>` from a normal Windows Terminal
  and from `cmd.exe`, accepts the prompt, and sees the output and the exit code in the same window.
  If the attach fails in one of them, the named-pipe fallback is built. `crates/nvs-cli/src/elevate.rs:1`
