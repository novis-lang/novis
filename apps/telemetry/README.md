# Telemetry

The service that receives the anonymous usage reports of `nvs telemetry`. It is a small web app
written in Novis.

A report is checked against a closed list of fields. An accepted report is added to weekly totals,
and the report itself is not saved. The totals are kept in SQLite.

Run these commands in this folder:

```sh
nvs serve public/index.nvs   # the app answers on http://127.0.0.1:8000/
nvs test .                   # runs the tests
```

| File | What it does |
|---|---|
| `known.toml` | The systems, processors and commands a report may name |
| `src/App.nvs` | The routes |
| `src/Report.nvs` | Checks one report |
| `src/Totals.nvs` | Adds a report to the weekly totals |

`POST /v1/report` takes one report as JSON, at most 4 KiB. It returns `204` when the report is added,
`400` when a field is wrong, `413` when the body is too large and `415` when the body is not JSON.
