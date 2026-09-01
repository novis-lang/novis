# Handoff

## State

**Stage 7's `[log]` block is complete.** `nvs_runtime::Ctx::write_log_record` reads all three
directives — `target`, `level` and `format` — and it now takes the `Record` rather than rendered
bytes, so neither of ADR 0092 § 6's two writers chooses a destination, a floor or a rendering.
`Core\Log::write` and `nvs_runtime::floor::report` each build a record and hand it over; that method's
doc comment is the home of why each of the three is read there and not at the callers.

- **Three directives, three boot refusals, and each names a different failure.** `E0613` an
  unspelled target (records nowhere), `E0614` an unspelled level (the floor stays at `Debug` and the
  deployment silently collects more than it asked for), `E0615` an unspelled format (the right
  records in the shape the operator asked not to have). Their grammars live where the type does:
  `nvs_config::log::Target` and `::Format` in that crate, `nvs_render::Level::of` one crate down
  beside the enum a record already carries. `nvs-config` keeps only the refusals, because only it
  holds the tree and the origins to say which file the word was written in.
- **`format` has two values and not three.** ADR 0092 § 3 says so outright: the HTML rendering is
  what a response sink selects, so `format = "html"` is refused rather than answered with JSON.
- **The previous session's work was rescued, not rewritten.** Session 0001 of this run left
  `[log] level` complete and uncommitted across eight files; it verified green unchanged and is
  committed as its own slice. The playbook's new *Tooling* bullet is the general form.
- **`syslog` is still spelled and not transported**, unchanged — `crates/nvs-config/src/log.rs:37`.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside
this goal and is not a regression.

## Next group

**`Core\Process\Result` is the one class in the tree with a member no conformance case calls, and it
has three of them — `python tools/gaps.py` ranks it first at depth 0.0. All three are read off the
same `Core\Process::run` result, so one case can ask more than one of them and the group shares one
file set: `crates/nvs-stdlib/src/process.rs` and `tests/conformance/core/`. Check what an existing
`Core\Process` case runs as its program before writing a new one — CI has three hosted runners and
the argv has to exist on all of them.**

- [ ] **A case that reads a completed run's status** — `Core\Process\Result::exitCode`, both sides
      of the bound: a program that succeeds and one that does not, since an exit code asserted only
      at `0` passes on a member that answers `0` always.
      `crates/nvs-stdlib/src/process.rs:373`.
- [ ] **A case that reads the two streams apart** — `stdout` at
      `crates/nvs-stdlib/src/process.rs:382` and `stderr` at
      `crates/nvs-stdlib/src/process.rs:390`, asserted so that a member returning the *other*
      stream fails: write to both from one program and name which text landed where.
- [ ] **`Core\Task::afterResponse` is the last member with a PHP twin and no oracle case** —
      `fastcgi_finish_request`, `crates/nvs-stdlib/src/task.rs:561`. A different file set, so take
      it only if the two above leave room; it goes in `tests/differential/`, never in
      `tests/conformance/` (conventions.md).

## Backlog

- The syslog transport, and the severity argument a sink needs — `crates/nvs-config/src/log.rs:37`.
- One configured file, many contexts: the rotation bound is counted per handle, which is right for a
  CLI run and undecided for a served host — `crates/nvs-runtime/src/logfile.rs`'s module doc.
- Stage 10's `every_part_two_spec_member_is_registered` waits on spec §§ 15-19 (goal 6).
- `orient.py` reported two dead `[context] modules` patterns — `crates/nvs-stdlib/src/fatal.rs` and
  `src/script.rs` matched no module; the globs in `docs/agent/loop-goal.toml` want fixing.
