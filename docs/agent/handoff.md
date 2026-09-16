# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 12's call-site half is landed**: a literal series name outside
`[a-z][a-z0-9_]*` is `E0769` where it was written, on all three verbs, and a `tainted` label value
or series name is `E0401` at the argument. Stages 0–11 are closed.

**Stage 12 is not finished, and the driver's failing check is what is left**: of the three Rust
tests its acceptance names under `-p nvs-stdlib`, only `core_metrics_is_a_registered_class` exists.
The other two are the next group and neither needs new behaviour — the code they assert is landed.

The grammar's one home is `nvs_stdlib::metrics::validate_name`, and that module is now `pub` so the
checker reaches it. `Grammar::MetricName` is the one row on the intrinsic list whose refusal the
runtime does **not** repeat — the registry fixes whatever it is handed, because a metric write may
not fail the request that made it — and `crates/nvs-types/src/intrinsics.rs`' module doc, fourth
bullet, is where that asymmetry is argued. Nothing is blocked.

## Next group

**Stage 12, the two acceptance tests that do not exist yet** — one file set: both are `#[cfg(test)]`
tests in `crates/nvs-stdlib/src/metrics.rs`'s module at `:428`, reading `nvs_runtime::metrics`.

- [ ] **A name used as a gauge and then incremented throws naming both sites** —
      `crates/nvs-stdlib/src/metrics.rs:364` is `mismatch`, which builds the `LogicError` and reads
      where the name was fixed off `nvs_runtime::metrics::fixed_at`
      (`crates/nvs-runtime/src/metrics.rs:906`). `rule:observability/metrics-three-members` is the
      rule; `tests/conformance/core/metrics-a-name-is-fixed-to-one-kind-and-the-throw-names-both-sites.nvst`
      already pins it from a program, so this is the same claim asked of the helper. Closes
      `a_metrics_name_used_as_a_gauge_then_incremented_throws_naming_both_sites` under
      `-p nvs-stdlib`.
- [ ] **A build with the exporter feature off still accumulates** —
      `crates/nvs-runtime/src/metrics.rs:888` is `record`, which builds a registry on any thread
      that writes a metric without having been metered, and `:921` is `on_this_core`, the only way
      to read one back. `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` is
      the rule: behaviour is identical across builds except for the export path. Closes
      `core_metrics_accumulates_with_the_exporter_feature_off` under `-p nvs-stdlib`.

## Backlog

- Stage 13, `Core\Process::spawn` and M8's two open `Core\Process` bullets — `docs/agent/loop-goal.md`
  § *Stage 13*, `crates/nvs-stdlib/src/process.rs:36-50`, guard at
  `benches/abi-probe/tests/perf_guards.rs:436`.
- Stages 14–15, the verification remainder and the `shipped` flips — the `[[check]]` blocks tagged
  `15 the rulebook` in `docs/agent/loop-goal.toml`.
- `rule:observability/metrics-three-members` spells the bag `array<string, string>` while the row
  declares `array<string>` (`crates/nvs-stdlib/src/metrics.rs:116`). One of the two is wrong; stage
  15's rulebook pass is where it is settled.
- `[context] modules` is missing `crates/nvs-stdlib/src/metrics.rs` — the pack printed
  `crates/nvs-runtime/src/metrics.rs` for stage 12 but not the class's own module, which is where
  both of the next group's tests go.
