# Handoff

## State

**Goal `plan-truth` is through stages 2, 3 and 4, and stage 5 is two rows from green.**
`python tools/playbook.py --check` now names `docs/agent/carried-gaps.md:54` and `:58` alone. Six rows
left the § *Owned* table because the code closed them, seven took the owner their module doc names or
the live goal whose stage builds them, and § *Unowned*'s duplicate of the driver matrix's socket leg is
gone — `crates/nvs-db/src/matrix.rs:43` carries that gap with its own `— owner:` tag.

Closed and struck, each read in the source first: `nvs serve` starts one worker per core
(`crates/nvs-cli/src/serve.rs:446` spawns, `:460` joins); a cycle closed through an `array<T>` is swept
(`crates/nvs-runtime/src/object.rs:1822` tallies a solely owned array, guarded at `:4479` and `:4508`);
`Core\Uri::with` removes a component (`crates/nvs-stdlib/src/uri.rs:2501`, test at `:4073`); every
module-doc gap names an owner (`owners.py --check --untagged-is-an-error --reasons` is green over 154);
`Core\Net`, `Core\Os` and `Core\Signal` are registered (`crates/nvs-stdlib/src/registry.rs:1607`,
`:1619`, `:1961`) and off the outstanding-classes list; and the `nvs check` grants row is already
carried whole by § *Unowned*'s intrinsic-pass bullet.

Stage 1 is goal `websocket-client`'s list, carried as the floor.

## Next group

**Stage 5: the index** — one file set: `docs/agent/carried-gaps.md` and the ratchet list its rows point
at. § *The contract* is the rule for all three: a row leaves only when the gap is closed, and an owner
that went green is struck rather than renamed to hide it.

- [ ] **The `Core\Test` documentation row** — `docs/agent/carried-gaps.md:54`, owner `test-request`,
      retired. What it names is thin prose rather than absent code: `rule:testing/in-process-request`
      states two response readings where `Core\Test`'s registry signature is the fact, and
      `docs/spec/01-core-library.md:999` is one English cell. Goal `m7-server-surface` says its tests
      send headers and a body (`docs/agent/goals/57-m7-server-surface.md:11`) — read its stage list for
      `Core\Test` before writing that owner, and otherwise strike the row to § *Unowned* naming what has
      to be decided.
- [ ] **The untyped grant value row** — `docs/agent/carried-gaps.md:58`, owner `config-is-written`,
      retired. `crates/nvs-config/src/tree.rs:49-60` is the untagged `Setting` every directive shares,
      and `:40-47` is why each arm is there, so an `[capabilities.fs] read = 1` validates clean and is
      denied at run time. No live goal names configuration validation; decide between a milestone tag
      and § *Unowned* with the decision written out.
- [ ] **The ratchet file carries the same stale owners** —
      `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:30` and `:31` say `# test-request`
      for `Response::html` and `sendFile`, whose row now says `m7-server-surface`, and `:35` says
      `# carried-gaps` for `§18 streamAs`, whose row says `gap-zero`. `every_outstanding_key_names_an_owner`
      in `crates/nvs-stdlib/tests/spec_registry_coverage.rs` is the gate, so this one runs `cargo test`.

## Backlog

- `nvs_types::expr::is_assignable`'s own docs are cited as saying no variance was committed to, while
  `rule:types/arrays` states invariance flatly — same stage-4 pass, and `owners.py` does not see it
  because it is not a `# Known gaps` item. `crates/nvs-types/src/expr/`.
- `rule:security/isolate-teardown-is-a-drain-then-a-sweep` tallies field slots only; the sweep also
  tallies a solely owned array (`crates/nvs-runtime/src/object.rs:1822`). The code is ahead of the rule,
  so the fragment is the edit, by the rule's own process.
- `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:5`'s worked example names the retired
  goal `carried-gaps`.
- `crates/nvs-test/src/case.rs:351`'s `NOT_YET` reason string still names M6; correcting it is a code
  change, which this goal's § *Standing decisions* forbids.
- `crates/nvs-types/src/lib.rs:156` says definite assignment is "no longer conservative" — changelog
  wording a comment may not carry.
- Stage 1 is goal `websocket-client`'s carried floor and nothing in it is known red.
