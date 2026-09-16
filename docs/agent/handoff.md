# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 12's first item is landed**: `Core\Metrics` is a registered class
with `increment`, `observe` and `gauge`, and stages 0–11 are closed. What is left is stage 12's
call-site half, then stages 13–15 (`Core\Process::spawn`, the verification remainder, the rulebook).
Nothing is blocked.

The per-core registry moved **down**, from `nvs-server` to `nvs_runtime::metrics`, because
`nvs-stdlib` and `nvs-server` may not depend on each other and both write to one registry. Both
module docs record the split. `crates/nvs-server/src/metrics.rs` is now a re-export plus the one
declared-series case the floor names, so every `crate::metrics::…` caller in that crate is
unchanged. `meter_this_core` now **adopts** an exporter into a registry that already exists, since
`Core\Metrics` builds one on any thread that writes a metric without having been metered — which is
every `nvs run`.

`registry::RECORD_PRODUCERS` is now `registry::SOURCE_MEMBERS`, because a second rule now puts a
member on it: a metric name fixed to one kind throws naming both call sites, and the site arrives as
the same argument-0 constant a record producer takes. Each metrics member therefore has `args: [4]`
for three declared arguments.

## Next group

**Stage 12, the call-site half: a literal name and a label value are checked where they are
written** — one file set: the two checks are `nvs-types`' and the cases that pin them are reject
cases, so `crates/nvs-types/src/intrinsics.rs`, `crates/nvs-types/src/core_lib.rs` and
`tests/conformance/reject/` are open together.

- [ ] **A literal `$name` outside `[a-z][a-z0-9_]*` is a compile error** —
      `crates/nvs-types/src/intrinsics.rs:360` is `check_call`, the call-site literal inspection the
      four intrinsics already run, and `crates/nvs-types/src/intrinsics.rs:188` is the closed list it
      walks. `rule:observability/metrics-three-members` is the grammar ("validated at compile time
      against `[a-z][a-z0-9_]*`, by the same call-site literal inspection
      `rule:security/secret-sinks-refuse` performs"), and a non-literal name stays a run-time
      question — the registry fixes whatever it is handed. Closes
      `a_literal_metrics_name_outside_the_grammar_is_a_compile_error` under `-p nvs-types`.
- [ ] **A `tainted` label value is a compile-time diagnostic** — `crates/nvs-types/src/core_lib.rs:996`
      is where a `Qual::Sink` parameter refuses a qualified argument, and the open question is whether
      it reaches the **element** type of a `CoreTy::Array`: the row writes
      `CoreTy::Array(&CoreTy::Text(Qual::Sink))` (`crates/nvs-stdlib/src/metrics.rs:81`) and nothing
      yet proves the qualifier is read through the array. `rule:security/metric-label-refuses-tainted`
      is the rule and `$name` is a sink for the same reason. Closes
      `a_tainted_metrics_label_value_is_a_compile_time_diagnostic`.
- [ ] **A reject case for each**, beside
      `tests/conformance/reject/a-core-io-path-refuses-a-tainted-argument.nvst:1`, which is the
      nearest `Qual::Sink` refusal already pinned and the shape to copy — an `--EXPECTF-ERROR--`
      block reproducing the diagnostic's own indentation, which widens with the line number.

## Backlog

- Stage 13, `Core\Process::spawn` — `crates/nvs-stdlib/src/process.rs:36-50`, goal prose stage 13.
- `nvs_runtime::metrics` gap 1 (`[metrics] endpoint` has no pusher) — owner `unowned-closures`.
- `docs/agent/carried-gaps.md` lost two rows this session; the `Core\Process::spawn` row there is the
  one stage 13 strikes.
- `crates/nvs-stdlib/tests/migration-members-outstanding.txt:22` is struck with `spawn`.
