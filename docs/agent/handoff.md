# Handoff

## State

**Goal `m8-db-queue`, stage 8. Recording and applying both land; only the call-site half is left.**
`nvs_jobs` carries `grants` and `limits`, every dialect's enqueue writes them, and every dialect's
claim now answers them at the end of its own list — `crates/nvs-stdlib/src/queue.rs:498` and the
three texts beside it, held together by `all_three_dialects_answer_a_claim_with_the_same_columns`.
A worker reads them by ordinal at `crates/nvs-cli/src/worker.rs:637`.

`nvs_stdlib::queue::narrowing` (`crates/nvs-stdlib/src/queue.rs:2653`) reads the pair back into the
`nvs_runtime::host::Narrowing` a `spawn script … with(…)` already carries, and `run`
(`crates/nvs-cli/src/worker.rs:1055`) hands it to `Isolate::narrowed_by`, so what applies it is
`nvs_config::Request::set` — the one reader that refuses a widening. A pair it cannot read is a
refused attempt, never a job run at the deployment's own ceiling.

**What that unblocks is the option pair on `push`.** Gap 1
(`crates/nvs-stdlib/src/queue.rs:64`) now says what is still owed: a written `grants:` naming a
capability the enqueuing request does not hold has to be refused where it stands, since a row is
narrowed *from* that request and never widened. Stage 9's socket leg is untouched beside it —
`tools/db-matrix.py` has no `AF_UNIX` endpoint.

## Next group

**Stage 8: `push` declares the narrowing and refuses a widening** — one file set:
`crates/nvs-stdlib/src/queue.rs` and the goal file
`docs/agent/goals/58-m8-db-queue.toml`. `rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue`
owns what may be written and what may not.

- [ ] **`push` declares `limits` and `grants`, and refuses a grant the enqueuing request does not
      hold.** The rows go beside `tag` in the one trailing bag, at
      `crates/nvs-stdlib/src/queue.rs:1870`; the refusal goes in `nvs_core_queue_push`
      (`crates/nvs-stdlib/src/queue.rs:2931`) *before* the connection is reached, which is that
      member's own stated ordering, and it narrows what
      `crates/nvs-stdlib/src/queue.rs:2964` records rather than replacing it. The check names
      `push_refuses_a_grant_the_enqueuing_request_does_not_hold`; `grants` is a list of capability
      names in the spelling `nvs.toml` grants them under, and `limits` its sub-caps one option each
      (gap 1, `crates/nvs-stdlib/src/queue.rs:64`).
- [ ] **Retire gap 1 and the test that stands in for it.**
      `limits_and_grants_are_refused_by_name_until_an_isolate_enforces_them`
      (`crates/nvs-stdlib/src/queue.rs:6670`) asserts the two options are *absent*, so it inverts
      the moment the item above lands — and its name is one a `[[check]]` names, at
      `docs/agent/goals/58-m8-db-queue.toml:6597`, so the rename happens in both places in the same
      slice. Gap 1's whole entry goes with it.

## Backlog

- Stage 9's socket leg: `tools/db-matrix.py` has no `AF_UNIX` endpoint (the goal's stage 9).
- `crates/nvs-stdlib/src/queue.rs` gaps 2–3 are goal `unowned-closures`'s, per the goal's
  § *Standing decisions* — not this one's.
- No end-to-end `nvs serve` runaway test for the resource ceilings (`docs/agent/carried-gaps.md`).
