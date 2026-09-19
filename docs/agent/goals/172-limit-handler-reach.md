---
milestone: dossier
---
# Loop goal 172 — every resource FATAL reaches the program's onLimit handler

`rule:errors/on-limit` says `Core\Fatal::onLimit`'s handler fires for every resource-limit `FATAL`,
and on one shape it does not: a breach raised inside a `Core` member's own loop stops the request
and prints the ceiling, and the program's handler never runs. Once this goal is green a program
gets its report wherever the breach was raised, so a request that dies on its memory ceiling looks
the same to the program whether the last allocation was an array append in compiled code or an
element a native drive took.

## Why here

It needs the ask that raises the breach to exist, and goal `lang-iteration` is what put one inside
a native loop: `crates/nvs-runtime/src/sequence.rs`'s drain asks the balance per element, which is
what bounds `Core\Arr::from` over a sequence with no end. The gap it exposed is a reporting seam,
not a bound, so nothing after it waits on this — it sits behind the generated proof goals because
every one of them may find another member with the same shape, and a fix written once here covers
all of them.

## Stage 0 — the catch-up

`crates/nvs-runtime/src/sequence.rs`'s `# Known gaps` item 1 is this goal's own statement of the
bug and is deleted by the session that closes it.

## Stage 1 — the floor

Goal `the-description-is-owed`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the handler runs, the keystone

One seam decides it: find where a breach raised by `crate::abi::affordable` inside a member body is
reported, and make that path run `Ctx::run_limit_handler` the way
`crate::abi::report_memory_breach` does for the breach `run_helper` finds ahead of a body. A
conformance case registering `Core\Fatal::onLimit` and driving `Core\Arr::from` over a sequence
with no end is what pins it, beside the case that already pins the stop.

## Standing decisions

- **The handler runs on the reserve, and the reserve is what makes this safe.**
  `rule:errors/on-limit` carves it out of the request's budget at request start, so running the
  handler while the request is over its ceiling is the designed behaviour and needs no new
  headroom.
- **The stop is not traded for the report.** A session that cannot make the handler run without
  weakening the bound leaves the bound alone and records what it found; the request being stopped
  is priority 1 and the program's report is not.
- **No numbered ADR is opened by this goal.** The rule already decided that the handler fires for
  every resource `FATAL`; this is the implementation catching up to it.
