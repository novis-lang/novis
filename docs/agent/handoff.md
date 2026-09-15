# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 9 are complete. Stage 10, the metrics export, is the earliest red stage and is the
next group. Nothing is blocked.

**A fire runs under the deployment's tree, and under the one a reload published.** `Scheduled` holds
the `Arc<nvs_config::Current>` its core was handed and `isolate` takes a clone off it per fire,
before the resolve (`crates/nvs-cli/src/serve.rs:1246`), so `rule:config/a-scheduled-run-is-a-root-isolate`'s
two halves are live: `[limits]` is the run's budget and `[capabilities]` are its grants — the grant
half is what lets the resolve past `script.spawn` at all. Per fire and not per boot for the reason
the accept loop reads the holder per request, so an edited tree reaches the next fire.

**What a reload still does not reach is the roster.** `nvs_server::arm` is called once off the boot
tree (`crates/nvs-cli/src/serve.rs:906`) and every `[[schedule]]` key is `System`-class, so a reload
applies an entry this ticker will not fire until a restart. `Scheduled`'s own `# Known gaps` owns
it; closing it is either re-arming inside the ticker or `rule:config/a-reload-names-what-it-could-not-apply`
naming the roster, and the second is a rule edit rather than a code one.

## Next group

**Stage 10: the metrics export** — one file set: `crates/nvs-server/src/metrics.rs` with
`crates/nvs-server/src/serve.rs` for the call site, and `crates/nvs-cli/src/serve.rs` for the
listener.

- [ ] **Every serving core owns a registry and counts each request under its route label** —
      `crates/nvs-server/src/metrics.rs:532`'s `Registry::request` is the consumer with no caller,
      and `crates/nvs-server/src/metrics.rs:392`'s `Registry::of` is what builds one from `[metrics]`
      (`None` when no exporter is named, which is the second half of the stage's other check). The
      call site is where a response is finished under `crates/nvs-server/src/serve.rs:885`'s
      `serve_connection`, and the label is `crates/nvs-server/src/route.rs:46`'s, which says it has
      a consumer and no caller today. `rule:observability/a-registry-is-per-core-and-nothing-reads-it`
      is why it is per core and never shared. Test:
      `every_serving_core_owns_a_registry_and_counts_each_request_under_its_route_label`,
      `-p nvs-server`.
- [ ] **A scrape merges every core's series into the text exposition format** —
      `crates/nvs-server/src/metrics.rs:425`'s `Registry::series` already promises the stable order
      the format needs, and `crates/nvs-server/src/metrics.rs:437` and `:444` are the kind and the
      label names a `# TYPE` line is written from. The merge is arithmetic and never coordination —
      the rule above states that — so what crosses a thread is a snapshot of series and not the
      registry. Test: `a_scrape_merges_every_cores_series_into_the_text_exposition_format`,
      `-p nvs-server`.
- [ ] **`serve` answers a scrape at `[metrics] listen`, and a tree whose exporter is `false` binds
      nothing** — the key is `crates/nvs-config/src/tree.rs:1050`, carrying an `[unread:]` marker
      this slice strikes. `crates/nvs-cli/src/serve.rs:275`'s control endpoint is the shape for a
      second listener the goal's § *Standing decisions* puts on a thread of its own, and
      `crates/nvs-cli/src/serve.rs:194`'s `listen_on` is where the server's own addresses are
      resolved. Test: `serve_answers_a_scrape_at_the_metrics_listen_address`,
      `a_tree_whose_metrics_exporter_is_false_binds_nothing_and_builds_no_registry`, `-p nvs-cli`.

## Backlog

- The schedule roster is armed once off the boot tree, so a reload applies a `[[schedule]]` change
  nothing honours — `crates/nvs-cli/src/serve.rs`'s `Scheduled` § *Known gaps*.
- A fleet entry whose fire is held by `queue` or started by `kill` runs under no lease at all —
  `crates/nvs-server/src/schedule.rs`'s tick walk answers § 6 before § 3 asks, so this host had
  already lost the interval. If it should take the key instead, that is § 3's reading and belongs
  in the rule.
- A fleet lease's key is `nvs:lease:` and the ticker's key with no application binding, so two
  deployments sharing one store share the lease for an entry they both name the same —
  `crates/nvs-stdlib/src/cache.rs`'s `LEASE_PREFIX` doc.
- Stage 8's "the check re-runs on reload" is vacuous while `[server]` is `Boot`-class: a reload
  cannot change a mount or its origin, which the rule fragment now says.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `crates/nvs-server/src/bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
