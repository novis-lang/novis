# Handoff

## State

**Goal `m7-server-surface` is complete.** Stage 1 is the carried floor, stage 2 is done
([0186](../decisions/0186.md) is the only ADR number this goal opened), and stages 3 to 12 are built.
Stage 12's `[[check]]` names three tests and all three are green. `python tools/verify.py` is 11 of 11
and `python tools/verify.py --doc` says every link resolves. Nothing is blocked.

**A broken edit is now asserted where the rule puts it.**
`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`'s first paragraph decides three things
at once and `a_revalidation_that_fails_to_compile_fails_only_the_requests_that_resolve_it_afterwards`
(`crates/nvs-cli/src/script.rs:1446`) asserts all three: the caller that resolves the broken content
gets the failure as checked-return data, the path's pointer is left naming the content that compiled
because step 4 never runs, and `Compiler::record`'s `keep` is why that unit is still in the table
beside the failure. `a_storm_against_a_broken_file_costs_one_compile_and_one_rendering_of_its_spans`
(`:1515`) is the cost half, over the fleet shape the cold-storm case uses: every request after the
first lands on the same `UnitKey` and is answered from the table, so `Compiler::compiles` moves once.

**The doc gate was red on links nothing else checks**, across `nvs-cli` and `nvs-server`: a
`#[cfg(test)]` `Recording`, a private `service::at_host` named from `main.rs`, the `registration::`
items linked from that module's own outer doc where the parent scope is what resolves, and the
`nvs_host::block_on` and `unit` names that are each ambiguous now that something shares them. The
playbook bullet this session added is the trap; the fix is the link syntax and no prose changed.

## Next group

**Stage 12: the served path, end to end** — nothing open. The stage's three cases are on disk and the
goal's acceptance list has no other red check, so the next session's work is whatever
`python tools/chain.py` installs after this goal.

- [x] A served `while (true)` is ended as a fatal and the core answers the next request
      (`crates/nvs-cli/src/serve.rs:4145`).
- [x] A served allocation loop is ended as a fatal and the core answers the next request.
- [x] A revalidation that fails to compile fails only the requests that resolve it afterwards
      (`crates/nvs-cli/src/script.rs:1446`), with its storm half beside it (`:1515`).

## Backlog

- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `crates/nvs-server/src/bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
- Stale prose: `docs/plan/m7.md`'s carrier list, `crates/nvs-cli/src/serve.rs:79-90`'s "no
  configuration" gap and the "one core" rows — goal `plan-truth`.
- `python tools/verify.py` does not run the doc leg, so a goal's broken intra-doc links all surface in
  the one session that claims `DONE` — `tools/verify.py`.
