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
| `src/Charts.nvs` | Draws the charts of the pages as SVG |
| `src/Data.nvs` | Builds the downloads as JSON and CSV |
| `src/Gate.nvs` | Sets the `Content-Security-Policy` header and writes the error answers |
| `src/Limit.nvs` | Limits each address to ten reports in one hour |
| `src/Pages.nvs` | Builds the HTML pages |
| `src/Report.nvs` | Checks one report |
| `src/Totals.nvs` | Adds a report to the weekly totals, and reads them |

`POST /v1/report` takes one report as JSON, at most 4 KiB. It returns `204` when the report is added,
`400` when a field is wrong, `413` when the body is too large and `415` when the body is not JSON.
The eleventh report from one address in one hour gets `429`. Its `Retry-After` header is the number
of seconds to wait.

Every total is public:

| Path | What it shows |
|---|---|
| `/` | The reports of every week, and the details of the newest week |
| `/collect` | What a report contains, and how to turn telemetry on and off |
| `/data` | The list of downloads, and the license |
| `/data/totals.json`, `/data/totals.csv` | Both tables |
| `/data/2026-W42.json` | The rows of one week. A week with no rows gets `404`. |

The pages load no script and set no cookie. The bottom of each page names the commit in the file
`REVISION`, or `unknown` when the file is missing.
