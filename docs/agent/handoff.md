# Handoff

## State

**`Core\Process\Result` is observed by two conformance cases, and a `Core\Process::run` that
*completes* now happens in the tree for the first time.** Every other `Core\Process` case is refused
before a child starts, so all three members — `exitCode`, `stdout`, `stderr` — were reachable only
through `echo` lines that never ran; `python tools/gaps.py` ranked the class first at depth 0.0 and
now ranks it at 1.0.

- **The child is the platform's own shell, named absolutely**, because ADR 0044 § 1 makes a `PATH`
  lookup the caller's own and there is no one name all three runners have. `/bin/sh` on the two Unix
  runners, `C:\Windows\System32\cmd.exe` on the Windows one, each in its own `try`; the candidate
  that is not this platform's has nothing at its path, so the operating system answers `IOError` and
  that block starts nothing. Both cases count that **exactly one** started, so a run where neither
  did fails rather than printing sentinels that read like a platform difference. Starting an
  interpreter deliberately is what § 4's own refusal message tells a caller to do — what the ADR
  closes is a command line Novis builds, and neither case builds one.
- **Each case grants `exec = true` rather than a root list.** A root list would have to survive
  canonicalisation of a system directory on three platforms as well, which is a second claim; the
  door's scoping is already pinned by
  `process-run-asks-for-the-capability-before-it-looks-at-the-target.nvst`.
- **The stream case compares `bytes` and never converts**, and asserts both halves of each answer —
  the marker that must be there and the other stream's marker that must not — because `stdout` and
  `stderr` have the same signature and adjacent slots, so a body reading its neighbour is a
  one-character defect no "not empty" assertion sees.
- Stage 7's `[log]` block is unchanged and complete: `nvs_runtime::Ctx::write_log_record` reads
  `target`, `level` and `format` over the `Record` itself, and `E0613`/`E0614`/`E0615` are its three
  boot refusals. `syslog` is still spelled and not transported —
  `crates/nvs-config/src/log.rs:37`.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside
this goal and is not a regression.

## Next group

**`Core\Process\Result` is still the thinnest class in the tree — one case per member against a
floor of three — and the three slices below are the three questions the two landed cases do not ask.
All of them read a result off `Core\Process::run`, so the file set is the one this session already
had open: `crates/nvs-stdlib/src/process.rs` and `tests/conformance/core/`. Copy the two-`try`
platform shape and the `exec = true` grant from
`tests/conformance/core/process-a-completed-run-answers-the-status-the-child-chose.nvst`, and run
each new case on the Unix leg as well — the playbook's *Running things* bullet is the one call.**

- [ ] **A case that pins the empty capture** — a child that writes nothing answers a zero-length
      `stdout` and a zero-length `stderr`, not a `null` and not a one-byte newline, which is the
      absence edge both members are written around.
      `crates/nvs-stdlib/src/process.rs:382` and `crates/nvs-stdlib/src/process.rs:390`.
- [ ] **A case that holds two results at once** — one program run twice with different arguments,
      both results asked *after* both runs finished, so a member reading a buffer shared with the
      next run fails while answering plausibly to a single run.
      `crates/nvs-stdlib/src/process.rs:332`.
- [ ] **A case that asks each member twice and asserts agreement** — a captured stream is a value,
      not a pipe that drains, so the second answer must equal the first for all three members.
      `crates/nvs-stdlib/src/process.rs:373`.

## Backlog

- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case —
  `fastcgi_finish_request`, `crates/nvs-stdlib/src/task.rs:561`; it goes in `tests/differential/`,
  and the twin has to survive a CLI SAPI that does not define that function.
- `--errors`: `Core\Env::get`'s non-UTF-8 refusal is catchable and unasserted —
  `crates/nvs-stdlib/src/env.rs:200`, reachable with an `--ENV--` section.
- `Core\Csv::format`'s non-`string` column refusal is catchable and unasserted —
  `crates/nvs-stdlib/src/csv.rs:610`.
- `syslog` is spelled by `nvs_config::log::Target` and not transported —
  `crates/nvs-config/src/log.rs:37`.
- Stage 10's `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which are goal 6's.
