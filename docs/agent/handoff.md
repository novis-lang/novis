# Handoff

## State

**Goal `unowned-closures`. The register is `unowned: 15`** (`python tools/owners.py`),
`--deferrals` green. The 15 unowned are the scheduling questions, none of them this goal's own gap,
and they are what stage 6's check reads — stage 5's work does not move that number.

**The server door now refuses an unchecked cross-origin write** —
`crates/nvs-server/src/route.rs:177`. The refusal has two grounds and they arm separately: the
origin half needs nothing configured, the token half is armed by `[http] csrf_key`. That module's
own section is the home of both, and `rule:security/csrf-is-on-by-default`'s fragment states them.
Both of `nvs-cli`'s doors take the verdict — `crates/nvs-cli/src/serve.rs:954` off the wire and
`crates/nvs-cli/src/runner.rs:1528` for `rule:testing/in-process-request`, which reads the tree
once at bind time because the answering closure is `'static`.

**`nvs_runtime::csrf` is the one home of the token format**, and `Core\Csrf` is a caller of it:
`nvs-server` cannot see `nvs-stdlib`, so a door that rebuilt the construction would have been an
application issuing tokens its own door refuses. `nvs_config::http::csrf_key` decodes the written
key, and `validate` refuses one the door could not read (`E0650`), so the check has no silently
unarmed state.

**Stage 5's remaining items are two server-door gaps**, plus `nvs-runtime`'s metrics one in the
backlog. The `nvs-cli`, `nvs-db` and `nvs-server`/`route.rs` entries in the goal prose's list are
closed; the register is what is true.

## Next group

**Stage 5: the door's two remaining gaps** — one file set:
`crates/nvs-server/src/{schedule,trace}.rs`. Both are `# Known gaps` items in modules that run
beside the accept loop rather than inside a request, so the `Ctx` wiring and the `nvs_config`
snapshot reads are shared.

- [ ] **A fire's context carries the configuration its `limits` sub-cap is measured against** —
      `crates/nvs-server/src/schedule.rs:80` gap 1, whose rule is
      `rule:config/a-scheduled-run-is-a-root-isolate`. `fire` builds the root on a bare `Ctx`, so
      `narrow_under` has no ceiling to narrow and `script.spawn` is denied; what is missing is the
      deployment's snapshot on that context, not the narrowing over it.
- [ ] **A sampled request's graph gets more than its root span** —
      `crates/nvs-server/src/trace.rs:78` gap 1, whose rule is
      `rule:observability/a-call-never-becomes-a-span` for what must stay out of it. The gate to
      build is "is this trace recorded", asked where a `query`, an `http` and a `spawn` are filed,
      rather than `DebugFlags::TRACE`.

## Backlog

- `[metrics] endpoint` has no pusher — `crates/nvs-runtime/src/metrics.rs:105` gap 1, stage 5.
- aarch64 artifacts need instruction-field relocations — `crates/nvs-cli/src/cache.rs` `# Known
  gaps`; nothing on this x86-64 host executes what it would build.
- A `.nvst` case for the door's refusal would need `server: true` plus a tree naming a key —
  `tests/conformance/`, and the Rust guards in `crates/nvs-server/src/route.rs` cover the verdict.
