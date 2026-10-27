Eleven series are present the moment an exporter is configured, with no application code written:

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
| `nvs_log_handler_dropped_total` | counter | — |

Every one is read from instrumentation that already exists — the `query`, `gc` and `spawn` event
kinds and the arena accounting `rule:programs/memory-priority` already requires — which is
`rule:observability/the-runtime-exports-what-it-already-measures` applied series by series.

Five gauges are filled as a scrape or a push gathers the cores' series, so no request writes them:

- `nvs_memory_bytes{scope="request"}` is what every core holds for its requests, the sum admission
  reads (`rule:http-server/admission-is-arithmetic-not-a-number`).
- `nvs_memory_bytes{scope="cache"}` is what `Core\Cache`'s memory tier holds, as its cap counts it.
  The gather takes each shard's read lock once.
- `nvs_memory_bytes{scope="process"}` is the resident set's high-water mark, the reading
  `Core\Os::residentBytes()` gives (`rule:observability/memory-is-three-numbers-on-core-budget`).
  One system call per gather.
- `nvs_tasks_in_flight` is the sum of each core's live tasks, and `nvs_deferred_trees` the sum of
  each core's trees open for after-response work.

The last two change on every task and every tree, so each core publishes them into two words of its
own beside its registry, and the gather reads the words. A store into a word only that core writes
costs the task path nothing measurable, where a series write would take the registry's lock each
time. Spends: two `u64` per metered core.

The peak is a **histogram** where the live figure is a gauge, because the two answer different
questions: a gauge says what is held now, and only a distribution answers what fraction of requests
came near their ceiling. It reads the mark
`rule:observability/a-memory-peak-is-recorded-not-asked-for` already holds, so it adds no probe site.

`route` is the one label that would otherwise be unbounded, and
`rule:observability/route-label-is-the-path-pattern` is what keeps it closed. `method` is one of
RFC 9110's nine methods as the request wrote it, and `other` for any verb outside them, so a client
that makes up verbs adds one series and never one each. The eleven are seeded
into a core's registry ahead of `rule:observability/past-max-series-a-new-series-is-refused`'s
bound rather than counted through it, because a `max_series` small enough to refuse them would
quietly turn "present the moment an exporter is configured" into a different configuration.

**Declined:**
- **A statement label on `nvs_db_query_duration_seconds`**, normalised SQL beside `operation`.
  Valuable, and a cardinality question that needs a bound before it is offered.
