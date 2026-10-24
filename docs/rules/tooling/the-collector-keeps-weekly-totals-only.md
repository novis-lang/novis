The collector adds an accepted report to running totals in one SQLite transaction, and the report is
never written anywhere: not to a table, a file, a queue or a log. Each total is keyed on the report's
`week`, `version`, `os` and `arch`, and there are two tables:

| Table | One row per | Columns |
|---|---|---|
| `reports` | week, version, os, arch | `reports`, `first_reports`, and `active_1` to `active_7`: how many reports had each `days_active` |
| `counters` | week, version, os, arch, counter | `total`, the sum of the counter's values, and `reports`, how many reports had it above zero |

`reports` is how many installs sent a report that week, because an install sends at most one per week.
A counter's `reports` is how many of those installs used the command. That is usually the question,
and its `total` alone is easily skewed by one install that ran the command in a loop.

A row that only one report made is still anonymous: the report carries no identifier, no address and
nothing a person typed, so the row says only that one install of that version on that system ran
those commands that week. The totals are kept for as long as the service runs, and every one of them
is public (`rule:tooling/telemetry-totals-are-public`). So the project sees exactly what everybody
sees.

- **Memory:** one transaction per request. The database grows by at most one row per week, version,
  system and counter, which is bounded by the closed lists and not by the number of reports.

**Not on disk.** No page or download publishes the totals yet.

**Declined:**
- **Storing raw reports and computing totals later.** A raw report is one install's week. Not
  storing it is the simplest way to make sure nobody, including the project, can ever look at one.
- **Separate one-field tables, with no week by version by system cross-table.** It answers fewer
  questions, and the cross-table reveals nothing about a person, because a report carries nothing
  about one.
- **Deleting old weeks.** The totals are anonymous and public already, and history is what makes them
  useful.
