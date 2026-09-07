Ten series are present the moment an exporter is configured, with no application code written:

| series | kind | labels |
|---|---|---|
| `nvs_requests_total` | counter | `method`, `status`, `route` |
| `nvs_request_duration_seconds` | histogram | `method`, `status`, `route` |
| `nvs_db_query_duration_seconds` | histogram | `connection`, `operation` |
| `nvs_gc_pause_seconds` | histogram | — |
| `nvs_spawn_duration_seconds` | histogram | `kind` (`task`/`worker`/`script`) |
| `nvs_tasks_in_flight` | gauge | — |
| `nvs_deferred_trees` | gauge | — |
| `nvs_memory_bytes` | gauge | `scope` (`request`/`cache`/`process`) |
| `nvs_request_memory_peak_bytes` | histogram | `route` |
| `nvs_schedule_runs_total` | counter | `name`, `outcome` |

Every one is read from instrumentation that already exists — the `query`, `gc` and `spawn` event
kinds and the arena accounting `rule:programs/memory-priority` already requires — which is
`rule:observability/the-runtime-exports-what-it-already-measures` applied series by series.

The peak is a **histogram** where the live figure is a gauge, because the two answer different
questions: a gauge says what is held now, and only a distribution answers what fraction of requests
came near their ceiling. It reads the mark
`rule:observability/a-memory-peak-is-recorded-not-asked-for` already holds, so it adds no probe site.

`route` is the one label that would otherwise be unbounded, and
`rule:observability/route-label-is-the-declared-name` is what keeps it closed. The ten are seeded
into a core's registry ahead of `rule:observability/past-max-series-a-new-series-is-refused`'s
bound rather than counted through it, because a `max_series` small enough to refuse them would
quietly turn "present the moment an exporter is configured" into a different configuration.
