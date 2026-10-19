The collector publishes every total it keeps, and the project reads the same
pages and files as anyone else. There is no private view and no admin page.

| Path | What it shows |
|---|---|
| `/` | reports per week over all weeks, first reports per week, and for the last finished week: versions, systems, how many installs used each command, the `days_active` spread, and internal errors per command |
| `/collect` | what a report contains, field by field, an example, how to turn telemetry on and off, and links to the source and the deployed commit |
| `/data` | the download list and the CC0 notice |
| `/data/totals.json`, `/data/totals.csv` | both tables in full |
| `/data/<week>.json` | one week's rows |

The pages are HTML rendered on the server, and the charts are SVG in the page. A page loads no
script, sets no cookie and requests nothing from another host. Its `Content-Security-Policy` is
`default-src 'none'; style-src 'self'; img-src 'self'`. Reports for a week arrive for up to four weeks
after it ends, so the newest four finished weeks are marked as still arriving. The text on the pages is
written to `AGENTS.md` § *Text an end user reads*.

The data is CC0: anyone may copy, chart or republish it without asking.

- **Memory:** per request, the rows of the page it renders. The pages read the totals and write nothing.

**Not on disk.** There is no `apps/telemetry/`.

**Declined:**
- **A JavaScript chart library.** It is a script on a page that promises to run none, and SVG drawn
  on the server shows the same lines.
- **Suppressing rows that few reports made.** The project would then see more than the public, and
  a row has nothing personal in it to hide (`rule:tooling/the-collector-keeps-weekly-totals-only`).
