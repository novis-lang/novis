---
looper: 1
title: "the service's own account judges its configuration files before it starts"
summary: "nvs service install, including --dry-run, and nvs service status run the configuration ownership check as the account the service runs as and print E0607 with its help, nvs config check --as runs the same check for a named account, nvs service status on a stopped service prints the last run's diagnostic, the commands have their feature proofs, every rule this goal amends says what the code does, and the checks below pass."
context: [crates/nvs-config/src/trust.rs, docs/rules/config/ownership-is-the-trust-boundary.md, docs/rules/config/the-ownership-check-runs-where-it-can-be-answered.md]
stages:
  - id: 1
    title: the ownership check judges a path for a named account
    status: done
    context: [crates/nvs-config/src/resolve.rs, crates/nvs-config/src/data.rs]
    extra:
      context:
        rules: [config/ownership-is-the-trust-boundary, config/the-ownership-check-runs-where-it-can-be-answered]
  - id: 2
    title: "`nvs config check --as <account>` runs the check for that account"
    status: done
    context: [crates/nvs-cli/src/config.rs, crates/nvs-cli/tests/config_audit.rs, crates/nvs-diagnostics/src/code/config.rs, docs/rules/config/check-and-dump-audit-the-tree-offline.md]
    extra:
      context:
        rules: [config/check-and-dump-audit-the-tree-offline, config/the-ownership-check-runs-where-it-can-be-answered]
  - id: 3
    title: "`nvs service install`, its `--dry-run` and `nvs service status` judge the configuration as the service's account"
    status: done
    context: [crates/nvs-cli/src/service.rs, docs/rules/packaging/a-service-runs-as-a-virtual-account.md, docs/rules/packaging/the-installer-is-a-sink.md]
    extra:
      context:
        rules: [packaging/a-service-runs-as-a-virtual-account, packaging/the-installer-is-a-sink, config/the-ownership-check-runs-where-it-can-be-answered]
  - id: 4
    title: the website and the reference pages say which account each command checks for
    status: done
    context: [website/src/content/docs/install/server.mdx, website/src/content/docs/guides/production/run-as-a-service.mdx, docs/examples/tools/install/what-novis-checks/about.md, docs/examples/tools/server/nvs-service/about.md, docs/examples/tools/cli/nvs-config-check-and-nvs-config-dump/about.md]
  - id: 5
    title: "`nvs service status` on a stopped service prints its last run's diagnostic"
    status: done
    context: [crates/nvs-cli/src/service.rs, crates/nvs-cli/src/dispatch.rs, docs/rules/packaging/a-service-answers-its-manager.md, docs/examples/tools/server/nvs-service/about.md]
    extra:
      context:
        rules: [packaging/a-service-answers-its-manager]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: config-check-proofs
    stage: 4
    name: nvs config check and dump owe no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:cli/nvs-config-check-and-nvs-config-dump"]
  - id: service-proofs
    stage: 5
    name: nvs service owes no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:server/nvs-service"]
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, service-owner-check]
    reach: true
    floor: false
---

# the service's own account judges its configuration files before it starts

## Why

`nvs serve` runs the ownership check (`rule:config/ownership-is-the-trust-boundary`, `E0607`) as the
account it runs as. Under a service manager that is the service's account, and on Windows the default
is `LocalSystem`. A configuration file owned by an administrator's own account passes for that
administrator and fails for `LocalSystem`. The failure shows only when the service starts: after the
binary is in place, and after the manager's restarts are used up.

Nothing an operator runs before the start finds it. `nvs config check` resolves through `Unowned`
(`crates/nvs-cli/src/config.rs:146`, `crates/nvs-config/src/resolve.rs:245`), which skips the check.
`nvs service install --dry-run` prints its steps and returns before any check
(`crates/nvs-cli/src/service.rs:2869`). `nvs service status` prints only the state word
(`crates/nvs-cli/src/service.rs:3972`, `crates/nvs-cli/src/service.rs:4307`). The `E0607` help already
prints the repair command with the path filled in. This goal makes the same error appear before the
start, judged for the account that will meet it.

## Standing decisions

**The account is a parameter of the check, and `Files` is the seam.** `trust::check` judges for this
process's account today: Unix reads `geteuid()` itself (`crates/nvs-config/src/trust.rs:249`,
`crates/nvs-config/src/trust.rs:274`), and Windows `examine` reads `token_user()` and builds the trusted
set from it (`crates/nvs-config/src/trust.rs:650`). The account becomes a value, `trust::Account`: a
uid on Unix and a SID on Windows, with `Account::current()` what `check` passes today. `check_as(path,
&Account)` is the new entry point, and `check` calls it, so `nvs serve`'s judgement is unchanged. A new
`Files` impl beside `Disk` and `Unowned` (`crates/nvs-config/src/resolve.rs:181`,
`crates/nvs-config/src/resolve.rs:245`) judges through `check_as`. It gives every caller below the
check without a change to `resolve()`. Unix `judged(path, &Stat, me, ..)` already takes the uid.
Why: simplicity, and serve's judgement and the pre-flight one cannot drift, because they are one
function.

**The first failing file stops the check, as it stops `serve`.** A pre-flight resolves through the new
`Files` impl, so it names the same file, with the same `E0607` and help, that the boot would name.
`rule:config/check-and-dump-audit-the-tree-offline` already says `check` stops at the first refusal.
Fallback: none needed; listing every failing file is a later change if operators ask.

**Naming an account.** `LocalSystem`, `SYSTEM` and `NT AUTHORITY\SYSTEM` are the well-known
`WinLocalSystemSid` (`crates/nvs-config/src/trust.rs:808`), spelled as `is_local_system` reads them
(`crates/nvs-cli/src/service.rs:2198`). Any other Windows name goes through `LookupAccountNameW`
(`Win32_Security` is already a workspace feature, `Cargo.toml:239`). An `NT SERVICE\<name>` account
does not resolve until the SCM holds that service, so at install it is computed as Windows computes
it: `S-1-5-80-` and the SHA-1 of the upper-case service name in UTF-16LE, as five sub-authorities.
`nvs-config` takes `sha1.workspace = true` on Windows for it, because `Account::named` lives beside
the SID type, and a test pins the value `sc showsid Shop` prints. On Linux a user name is looked up
with `getpwnam_r` in `trust.rs` too, so `Account::named` is the one entry point on both platforms,
and a number is taken as a uid. A name this machine cannot resolve is `E0662`
(`E_CHECK_NO_SUCH_ACCOUNT`), never `E0654`, whose note is about systemd.

**The message names the account it judged for.** "neither this account nor an administrative one"
(`crates/nvs-config/src/trust.rs:658`) becomes "neither the account `LocalSystem` nor an
administrative one" when the account is not this process's own. A note says the service runs as that
account. The help is the repair commit `f23fb408a` wrote, unchanged where it already fits: on Windows
`/setowner` to `Administrators` passes for every service account. The message, the note and the help
are held to AGENTS.md § *Text an end user reads*.

**A real install with a failing check is refused, and `--dry-run` prints the same error and exits
`1`.** No flag overrides it. Why: the check is the function `serve` runs at every boot, on this host,
for the account the install names, so the result is the boot's own result. A service that cannot start
is never what the operator asked for. Two precedents in the same code refuse for that reason: a
configuration that does not resolve (`Unresolved::Refuses`, `crates/nvs-cli/src/service.rs:4353`) and
an account that does not exist (`E0654`, `crates/nvs-cli/src/service.rs:2393`). The unreadable-file
check beside it warns instead (`crates/nvs-cli/src/service.rs:2342`), because a supplementary group or
an ACL it does not read can still grant the read. The ownership check reads everything it judges, so
that reason does not apply. The refusal comes before `prepare_data` and before the manager is touched
(`crates/nvs-cli/src/service.rs:2880`), so a refused install changes nothing
(`rule:packaging/the-installer-is-a-sink`). Fallback: if a platform's install-time judgement is found to
differ from its boot judgement for some path, that path is a warning, and the rule says why.

**Paths the install itself writes are judged as the install leaves them, and they pass.** On Linux the
data folder and its `nvs.toml` become root's (`Action::Own`, `crates/nvs-cli/src/service.rs:2497`), so
the check skips a path an `Own` action of this install sets. On Windows the data folder is created by
`crates/nvs-config/src/data.rs:701`, whose descriptor names no owner, so the owner is the installing
token's default owner. Stage 3 judges that folder and its `nvs.toml` for `LocalSystem` and for a
virtual account in a test. If they fail, the descriptor names an owner the service trusts (`O:BA`), and
the test is the proof.

**`nvs service status` judges the stored configuration for the stored account.** It reads what the
platform holds (`Manager::stored`, `crates/nvs-cli/src/service.rs:2132`): the argv, whose `--config`
or data folder names the files the way `describe_host` reads them (`crates/nvs-cli/src/service.rs:4388`),
and the account. It prints the state, then the `E0607`, and exits `1` when the check fails. A file that
status cannot examine (an unelevated prompt on a private folder) is a warning line that names the path
and says an administrator prompt can check it, and the exit stays `0`. `Control::Status`'s doc line
"the manager's own answer and nothing more" (`crates/nvs-cli/src/service.rs:2079`) is rewritten.

**`nvs config check --as <account>` runs the check for that account, and plain `config check` is
unchanged and offline.** This answers the reason
`rule:config/the-ownership-check-runs-where-it-can-be-answered` gives for skipping it: it ran as the
wrong account. So that rule is amended, not reversed. `config dump` takes no `--as`. The limits are
stated in the rule and on the reference page: `--as` judges the files on this machine as they are now.
It does not see the server's own copy of the files, a container's read-only mount (the waiver at
`crates/nvs-config/src/trust.rs:234`), or a deploy step that changes an owner. The check trusts no
group except the administrators one, so the account's group membership does not change its answer; a
session that finds a place where it does names it in the rule as not judged.

**The service's account comes from the manager, as `Manager::judged_for`.** The answer is the
machine's, like `Manager::account`, and a case's recording manager answers with this process's own
account. A case then judges its tree on every runner, elevated or not, and on a Windows host running
a Linux-platform case. The method has no default, so a new manager cannot skip the check. **With no
`--config` file on disk the account is not looked up**: there is nothing to judge, and the SCM
refuses an account that does not exist at registration anyway.

**On Windows the data folder an install creates is owned by the installing token's default owner.**
An install runs elevated, and an elevated administrator's default owner is `BUILTIN\Administrators`,
which every service account trusts. The next session makes that explicit (`O:BA` when the token is an
administrator), so the claim does not rest on a policy default, and a test proves it.

**`nvs service unit` does not run the check.** It prints a definition for review, often on a machine
that is not the server. `install --dry-run` is the pre-flight on the host.

**A stopped service's last run is the platform's own record of it.** On Windows: the SCM's
`dwWin32ExitCode` and `dwServiceSpecificExitCode` from `QueryServiceStatusEx`, appended to the
`stopped` answer, and the Application log records the service's own event source wrote since its last
*starting* record, read newest first with `ReadEventLogW` (`crates/nvs-cli/src/service.rs:3628`). The
run starts at a new `Lifecycle::Starting` record (message id 4, which the table already had), written
first in `service_main`. Not the *started* record, which a refused configuration never reaches, and
not the previous run's end, because the unjoined reader threads can land a line after it.
`ReadEventLogW` rather than `EvtQuery`: it gives the event id and the text in the record and stops at
the run's start. On Linux: `systemctl show -p InvocationID --value`, then `journalctl
_SYSTEMD_INVOCATION_ID=<id> -o cat --no-pager -n 20`, by argv with no shell
(`rule:core-classes/process-is-argv-only`). Windows keeps the run's warning and error records, and
Linux every line, because systemd stores stdout and stderr at one priority; both keep the last 20
(`dispatch::LAST_RUN_LINES`). A running service prints what it printed before. The reading is
`Manager::last_run`, whose default returns nothing (`crates/nvs-cli/src/service.rs:2186`).

**Trade-offs.** Security: the check an operator runs before the start is the boot's own, so a passing
pre-flight means the service starts past this check. Latency: none on the request path. Install and
status each add one resolution of the configuration tree and one descriptor read per file. Memory: none
held after the command. Simplicity: one new flag (`--as`) and two commands that print more. An install
that used to succeed and then fail at start now fails at install.

**Lanes.** Stage 1 is `nvs-config` alone and runs first: every later stage builds on `check_as`.
Stages 2 (`crates/nvs-cli/src/config.rs`) and 3 (`crates/nvs-cli/src/service.rs`) touch disjoint code
but amend the same rule, `docs/rules/config/the-ownership-check-runs-where-it-can-be-answered.md`, so
they run in order. Stage 4 is docs only. Stage 5 shares `service.rs` with stage 3 and runs after it.

## Stages

### 1. the ownership check judges a path for a named account

**Does:** `trust::check_as(path, &Account)` judges for any account, `check` is `check_as` for this
process, and a `Files` impl resolves a tree through it.

- [x] `trust::Account` (a uid on Unix, a SID on Windows) with `Account::current()`, and `check_as` and `check_alone_as` beside `check` and `check_alone`, which call them. `crates/nvs-config/src/trust.rs:161`, `crates/nvs-config/src/trust.rs:190`
- [x] Unix `platform::check` and `check_link_holder` take the account in place of `geteuid()`. `crates/nvs-config/src/trust.rs:245`, `crates/nvs-config/src/trust.rs:270`
- [x] Windows `examine` takes the account's SID in place of `token_user()`, and the breach message names the account when it is not this process's own. `crates/nvs-config/src/trust.rs:643`, `crates/nvs-config/src/trust.rs:554`
- [x] `Account::named(name)` on Windows: the `LocalSystem` names, `LookupAccountNameW`, and the computed `NT SERVICE\<name>` SID. `crates/nvs-config/src/trust.rs:808`
- [x] The `Files` impl that judges for a given account, beside `Disk` and `Unowned` (`rule:config/ownership-is-the-trust-boundary`). `crates/nvs-config/src/resolve.rs:181`, `crates/nvs-config/src/resolve.rs:245`
- [x] Tests: on Unix a `Stat` owned by uid 1000 passes for 1000 and fails for another uid; on Windows (`#[cfg(windows)]`) a descriptor built in memory from SDDL with an ordinary user as owner passes for that user and fails for `LocalSystem`, and one owned by `Administrators` passes for both; the `NT SERVICE` SID matches its pinned value. `crates/nvs-config/src/trust.rs:897`

### 2. `nvs config check --as <account>` runs the check for that account

**Does:** `nvs config check --as <account>` resolves through the stage 1 `Files` impl and prints
`E0607` for the first file that account would refuse.

- [x] The `--as` argument on `Command::Check`, with help text held to AGENTS.md § *Text an end user reads*. `crates/nvs-cli/src/config.rs:80`
- [x] `check` resolves through the account's `Files` impl when `--as` is given, and through `LocalFiles` otherwise. `crates/nvs-cli/src/config.rs:1008`, `crates/nvs-cli/src/config.rs:146`
- [x] `E0662`, the config-area code for an account this machine cannot resolve, with its card. `crates/nvs-diagnostics/src/code/config.rs:902`
- [x] Tests: on Unix (run in WSL with `bun nv linux-test`), a tree owned by the test's account passes `--as` that account and fails `--as` another uid with `E0607`; an unknown name fails with the new code. `crates/nvs-cli/tests/config_audit.rs:1`
- [x] Amend `rule:config/the-ownership-check-runs-where-it-can-be-answered` (`config check --as` answers the question, with the limits the standing decision states) and `rule:config/check-and-dump-audit-the-tree-offline`, and the first rule's JSON title. `docs/rules/config/the-ownership-check-runs-where-it-can-be-answered.md:1`, `docs/rules/config/check-and-dump-audit-the-tree-offline.md:31`, `data/rules/config/the-ownership-check-runs-where-it-can-be-answered.json:2`

### 3. `nvs service install`, its `--dry-run` and `nvs service status` judge the configuration as the service's account

**Does:** install and its dry run refuse with `E0607` when the service's account would refuse a
configuration file, and status prints the same error under the state.

**Split:** first, an `Explore` subagent lists the recording-manager cases for install and status
(`crates/nvs-cli/src/service.rs:5897`, `crates/nvs-cli/src/service.rs:6320`,
`crates/nvs-cli/src/service.rs:6508`) as `file:line` and one line each, so the new cases take their
shape.

- [x] The service's account as a `trust::Account`: `account_of` on Windows, `plan.account` (or root) on Linux, through `Manager::judged_for`. `crates/nvs-cli/src/service.rs:2170`, `crates/nvs-cli/src/service.rs:2224`
  - [x] `Manager::judged_for`, required: `Systemd` and `Scm` call `Account::named`, `Recording` answers `Account::current()`
- [x] `registration::install` judges the named configuration through the account's `Files` impl after `resolve_account` and before both the dry-run return and `prepare_data`, skipping the paths an `Own` action of this install sets. `crates/nvs-cli/src/service.rs:2224`, `crates/nvs-cli/src/service.rs:2263`
  - [x] `judge` resolves every `--config` that is a file, and looks no account up when there is none
  - [x] `Installing` passes a path an `Own` sets together with its directory
- [x] The data folder and the `nvs.toml` an install creates pass for the service's account; if not, the descriptor names an owner it trusts. `crates/nvs-config/src/data.rs:701`, `crates/nvs-cli/src/service.rs:781`
- [x] `status` reads `Manager::stored`, judges its configuration for its account, prints the state and then the `E0607`, and exits `1`; a file it cannot examine is a warning with exit `0`. `crates/nvs-cli/src/service.rs:4307`, `crates/nvs-cli/src/service.rs:2132`, `crates/nvs-cli/src/service.rs:2079`
- [x] Tests through the recording manager: a dry run and a real install over a configuration the service's account refuses both print `E0607` and touch no manager; the same tree for an account it passes installs; status on a stored service over such a tree prints the state and `E0607`. `crates/nvs-cli/src/service.rs:6802`
  - [x] install: `an_install_over_a_configuration_the_service_account_refuses_is_e0607_before_the_manager`, green on Windows and in WSL
  - [x] status: `Recording` takes a seeded `Stored`, and a case over a tree opened to others prints the state and `E0607`
- [x] Amend `rule:packaging/a-service-runs-as-a-virtual-account` (install refuses what the account would refuse at boot) and `rule:config/the-ownership-check-runs-where-it-can-be-answered` (install and status run it). `docs/rules/packaging/a-service-runs-as-a-virtual-account.md:29`, `docs/rules/config/the-ownership-check-runs-where-it-can-be-answered.md:29`
  - [x] install, in both rules, rendered
  - [x] status, in the ownership-check rule
- [x] `bun nv linux-test` runs the Linux cases once by name, because the change reads paths and accounts. `crates/nvs-cli/src/service.rs:6802`
  - [x] the install case
  - [x] the status case

### 4. the website and the reference pages say which account each command checks for

**Does:** the install page, the service guide and the two reference pages say that the check is
judged for the account the service runs as, and which command finds a failure before the start.

**Split:** one writing subagent writes the three pages under `website/` and `docs/examples/tools/install/`,
and a second writes the two reference `about.md` files and the example; their files are disjoint. Both
are held to AGENTS.md § *Text an end user reads*, and each runs `bun nv proofs --comments` on the
`.nvs` files it wrote.

- [x] `## What Novis checks`: the owner row says "the account the server runs as", the command table gains `nvs service install`, `nvs service status` and `nvs config check --as`, and `## When the check fails` says to run `nvs config check --as <account>` before a service starts. `website/src/content/docs/install/server.mdx:113`, `website/src/content/docs/install/server.mdx:217`
- [x] The install example's about page matches the table. `docs/examples/tools/install/what-novis-checks/about.md:1`
- [x] The service guide: install refuses a configuration its account would refuse, and status prints the error. `website/src/content/docs/guides/production/run-as-a-service.mdx:92`
- [x] The `nvs service` reference page says what install and status check. `docs/examples/tools/server/nvs-service/about.md:1`
- [x] The `nvs config check` reference page documents `--as` and its limits, with one example that shows it. `docs/examples/tools/cli/nvs-config-check-and-nvs-config-dump/about.md:1`, `docs/examples/tools/cli/nvs-config-check-and-nvs-config-dump/02-check-a-configuration-for-the-service-account.nvs:1`
- [x] `bun nv proofs --id 'tools:cli/nvs-config-check-and-nvs-config-dump'` and `--id 'tools:server/nvs-service'` owe nothing, each Rust test from stages 2 and 3 is in its feature's list, and `bun nv render --website` is rerun. `website/src/data/reference.json:1063`

### 5. `nvs service status` on a stopped service prints its last run's diagnostic

**Does:** a stopped service's status prints its state and then the warning and error lines of its last
run, read from the platform's own record.

- [x] The `Manager` method that reads the last run, with a default that returns nothing. `crates/nvs-cli/src/service.rs:2186`
- [x] Windows: the exit codes from `QueryServiceStatusEx`, and the Application log records of the service's source since its last *starting* record, read newest first with `ReadEventLogW`. `crates/nvs-cli/src/service.rs:3628`, `crates/nvs-cli/src/dispatch.rs:134`
  - [x] `Lifecycle::Starting` (message id 4) is written first in `service_main`
  - [x] the reader, checked once against records a throwaway probe wrote to this machine's Application log
- [x] Linux: `systemctl show -p InvocationID --value` and `journalctl _SYSTEMD_INVOCATION_ID=<id> -o cat --no-pager -n 20`, by argv with no shell. `crates/nvs-cli/src/service.rs:2194`, `crates/nvs-cli/src/service.rs:3242`
- [x] `status` prints the last run's lines under the state when the service is stopped, and nothing more when it runs; `Control::Status`'s doc says so. `crates/nvs-cli/src/service.rs:4752`
- [x] Tests: the recording manager's stopped service prints its recorded lines; the Linux argv is pinned as `service_start_stop_and_status_reach_the_manager_by_argv_with_no_shell` pins the control argv; the selection of the last run's records from a list that holds two runs is a pure function with its own test. `crates/nvs-cli/src/service.rs:7076`, `crates/nvs-cli/src/dispatch.rs:134`
  - [x] green on Windows and in WSL (`bun nv linux-test last_run`)
- [x] Amend `rule:packaging/a-service-answers-its-manager`'s § *Output*: status reads back the last run. The `nvs service` reference page and the service guide say so. `docs/rules/packaging/a-service-answers-its-manager.md:18`, `docs/examples/tools/server/nvs-service/about.md:1`, `website/src/content/docs/guides/production/run-as-a-service.mdx:111`
- [x] `bun nv proofs --id 'tools:server/nvs-service'` owes nothing, and `bun nv owners --closes service-owner-check` passes. `docs/examples/tools/server/nvs-service/about.md:1`

### 6. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the whole goal
once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group the
failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1 to 5 are done, and stage 6 is part way. The sweep's `env-secret-proofs` is green when run
again (`bun nv proofs --gate --only 'Core\Env::secret'`), and `cea10a7cd` covers its Miri reds.

Verify has run twice. The first run stopped at `rules-render` and `fmt`, and `9b7f28ea5` commits that
fallout, the fuzz lock's `sha1` included. The second run left three binaries that failed beside the
others and passed alone, all load timing in code this goal does not touch:

- `http::transport::tests::a_dead_first_address_falls_back_to_the_next_within_connect_timeout`.
  Fixed in `2655ff4d8`: a 60 s budget, bounded at 30 s (`crates/nvs-stdlib/src/http/transport.rs:5296`).
- `nvs-stdlib test ldap`, three Samba cases past their 30 s deadline. All the cargo cases take
  `nvs_repo::turn`. The conformance `ldap-*.nvst` cases reach the same Samba server without it, and
  verify's tail started them beside the binaries. `f4e9afdb4` makes the tail wait for
  `SAMBA_BINARIES` (`tools/nv/cmd/verify.ts:159`). That is the likeliest cause, not a proven one:
  this run's conformance log was older than the test log.
- `serve::tests::a_served_while_true_does_not_stall_a_second_connection_on_its_core`
  (`crates/nvs-cli/src/serve.rs:5910`): the runaway's socket read EOF with no response head. A
  subagent suggested that hyper's EOF probe during the run arms `Phase::Body`'s `body_idle` wait
  (`crates/nvs-server/src/io.rs:352`), so a program running past 30 s of wall time is closed with no
  answer. A case with `body_idle` at 100 ms and `cpu_time` at 1 s passed **without** that fix, so the
  theory is not proven, and the server change was reverted. Still open.

### Next

**Stage 6: verify again.** Start `bun tools/looper/main.ts verify` with `run_in_background` and end
the turn to wait. If the serve case fails again, find which socket closes and why before changing
`nvs_server::io`. One way to test the theory: `a_runaway_then_an_answer`
(`crates/nvs-cli/src/serve.rs:5743`) takes a `Waits` and a program that sleeps, not one that spins, past
a short `body_idle`. If the Samba cases fail again, check whether the conformance step overlapped
them. `tools/nv/proofs/run.ts:485` keeps its own two-slot Samba limit, not the shared turn.
Once verify is green, run the goal-end gates AGENTS.md § *In the loop* names: `bun nv verify --doc`,
`bun nv owners --closes service-owner-check` and `bun nv ci --check`. The goal's three checks are
`config-check-proofs`, `service-proofs` and `owners-closes`. Before `DONE`, move each decision only
this file's prose holds into a rule or a module doc. The last-run decision is in
`rule:packaging/a-service-answers-its-manager` already.

### Backlog

- Listing every failing file in one pre-flight, rather than the first, if operators ask (`rule:config/check-and-dump-audit-the-tree-offline`).
