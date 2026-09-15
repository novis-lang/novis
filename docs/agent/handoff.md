# Handoff

## State

**Goal `m8-db-queue`, stage 8. The recording half is on disk; nothing applies it yet.**
`nvs_jobs` carries `grants` and `limits` (`crates/nvs-stdlib/src/queue.rs:352`), every dialect's
enqueue writes them at the end of its own column list, and `push` reads them off the enqueuing
context through `grants_recorded` (`crates/nvs-stdlib/src/queue.rs:2593`) and `limits_recorded`
(`:2612`). Both are held in the shape `nvs_runtime::host::Narrowing` carries — capability names, and
`[limits]` keys as the text they were written as — so the isolate half applies them through
`nvs_config::Request::set`, which is the one reader that refuses a widening. A context that narrowed
neither half records null, which is a job held to the deployment's own ceiling.

**Stage 8's other two items are unbuilt, and their order is a security question**, stated in the
module doc's gap 1 (`crates/nvs-stdlib/src/queue.rs:64`): `push` declares neither option until the
isolate applies what the row carries, because a declared-and-dropped narrowing hands the job the
authority its request meant to give up. So the claim-and-apply item lands before the option item, and
`limits_and_grants_are_refused_by_name_until_an_isolate_enforces_them` stays green until it does.
Stage 9's socket leg is still unbuilt beside it: `tools/db-matrix.py` has no `AF_UNIX` endpoint.

## Next group

**Stage 8: what the row recorded reaches the isolate that runs the job** — one file set:
`crates/nvs-cli/src/worker.rs` and `crates/nvs-stdlib/src/queue.rs`.
`rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` owns what is narrowed and
`rule:concurrency/a-job-runs-as-a-root-isolate` owns what the job is; `crates/nvs-cli/src/worker.rs:44`
§ *Why the grants are the run's own* is the paragraph this group makes untrue and has to rewrite.

- [ ] **The claim answers the two columns**, at the end of each dialect's list, where a worker reads
      by ordinal: `crates/nvs-stdlib/src/queue.rs:495` (`CLAIM_POSTGRES`), `:778` (`CLAIM_MYSQL`),
      `:809` (`CLAIM_SQLITE`) and `:841` (`CLAIM_SQLSERVER`), with `Job` gaining the pair at
      `crates/nvs-cli/src/worker.rs:450` and the four readers at `crates/nvs-cli/src/worker.rs:518`,
      `:611`, `:726` and `:860`. `all_three_dialects_answer_a_claim_with_the_same_columns` derives
      from the texts, so it covers the new pair the day it lands.
- [ ] **The job runs under what the row recorded, narrowed and never widened.**
      `crates/nvs-cli/src/worker.rs:963` builds the isolate with
      `nvs_host::Isolate::new(program, args, Output::Capture).run(ctx)`, and `ctx` there is the
      *worker's own* context, held for the whole run — so the narrowing goes on a child, through
      `nvs_runtime::Ctx::narrow` (`crates/nvs-runtime/src/ctx/isolate.rs:571`) against a
      `nvs_runtime::host::Narrowing` (`crates/nvs-runtime/src/host.rs:371`) parsed back out of the
      two columns, and never on `ctx` itself, which would narrow every later job too. The test is
      `a_job_runs_under_the_grants_and_limits_recorded_at_enqueue`.
- [ ] **`push` declares `limits` and `grants` and refuses a grant the enqueuing request does not
      hold**, which is the widening — only after the two above, for the reason gap 1 gives. The bag
      is `push`'s one trailing options row in `crates/nvs-stdlib/src/queue.rs:1798`, the refusal
      sits with the other argument judgements in `crates/nvs-stdlib/src/queue.rs:2861`, and
      `limits_and_grants_are_refused_by_name_until_an_isolate_enforces_them` is rewritten in the same
      slice into the assertion that both are declared. The test is
      `push_refuses_a_grant_the_enqueuing_request_does_not_hold`.

## Backlog

- The per-app widening the narrowing does not close: a worker resolves a job under the run's boot
  snapshot, so a request under a narrower `[app.capabilities]` can enqueue a script that runs wider.
  Decide where that is answered — `rule:concurrency/a-job-runs-as-a-root-isolate`, or the worker.
- Stage 9's `mysql over a socket: ok` and its two siblings need an `AF_UNIX` endpoint in
  `tools/db-matrix.py`.
- `crates/nvs-stdlib/src/queue.rs` gaps 2–3 are goal `unowned-closures`', per the goal's
  § *Standing decisions*.
