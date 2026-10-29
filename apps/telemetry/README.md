# Telemetry

The service that receives the anonymous usage reports of `nvs telemetry`. It is a small web app
written in Novis.

A report is checked against a closed list of fields. An accepted report is added to weekly totals,
and the report itself is not saved. The totals are kept in SQLite.

Run these commands in this folder:

```sh
nvs serve public/index.nvs   # the app answers on http://127.0.0.1:8141/
nvs test .                   # runs the tests
```

In a clone of the repository, `bun nv app telemetry` starts the app from any folder. The app uses
port 8141. The other apps use other ports, so all of them can run at the same time.

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

## Send a report on your own machine

Start the app in this folder:

```sh
nvs serve public/index.nvs
```

`nvs telemetry upload` sends only a week that has ended. A new data folder has no such week yet, so
this example writes one by hand. It uses its own data folder `try`, so your own counters do not
change. Run it in a second terminal, in this folder. The `date` command is the one on Linux.

```sh
mkdir -p try/telemetry
echo on > try/telemetry/consent
printf 'uploads 0\nbytes 0\npending %s\ndays 1000000\ninvocation.run 3\n' "$(date -u -d '7 days ago' +%G-W%V)" > try/telemetry/counters
NVS_TELEMETRY_URL=http://127.0.0.1:8141/v1/report nvs --data try telemetry upload
```

The last command prints `telemetry: sent week ...`. Open `http://127.0.0.1:8141/` to see the week in
the totals. Delete the folder `try` when you are done.

## Deploy the collector

This guide runs the collector on a Linux host with systemd. [Caddy](https://caddyserver.com/) receives
the HTTPS requests and gets the certificate. It sends each request to `nvs serve` on
`127.0.0.1:8141`. The examples use the domain `telemetry.example.com`. Replace it with your own.

**1. Get the code.** Clone the repository to `/srv/novis` as `root`. Every user may read it, and only
`root` may change it.

```sh
sudo git clone https://github.com/novis-lang/novis /srv/novis
```

**2. Create an account for the service.** The service runs as the user `telemetry`. This user owns
only the folder of the database. Run other services, such as a package registry, under their own
accounts. The collector then cannot read their files.

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin telemetry
sudo install -d -o telemetry -g telemetry -m 0700 /var/lib/telemetry-db
```

**3. Write the configuration.** Copy `nvs.toml` to the data folder of the service:

```sh
sudo install -d -m 0750 /var/lib/telemetry
sudo cp /srv/novis/apps/telemetry/nvs.toml /var/lib/telemetry/nvs.toml
```

Then change three things in `/var/lib/telemetry/nvs.toml`. The database file goes into the folder of
step 2. The log lines go to the system journal. The two files a request may read get their full
paths, because a short path is read from the folder of `nvs.toml`.

```toml
[db.telemetry]
driver = "sqlite"
path = "/var/lib/telemetry-db/telemetry.sqlite"

[log]
client_ip = false
target = "stderr"

[capabilities.fs]
read = ["/srv/novis/apps/telemetry/known.toml", "/srv/novis/apps/telemetry/REVISION"]
```

The file keeps these settings from the copy:

- `[mode] default = "production"` runs the server with the settings for production.
- `[server] listen = ["127.0.0.1:8141"]` accepts connections from this host only, on port 8141.
- `[log] client_ip = false` keeps the address of the sender out of every log line.
- `[server] trusted_proxies = ["127.0.0.1"]` reads the address of the sender from Caddy's
  `X-Forwarded-For` header. The app uses the address only to limit the rate, and never saves it.

**4. Record the commit.** The bottom of each page shows the commit in the file `REVISION`. Write it
every time you update the code:

```sh
sudo git -C /srv/novis rev-parse HEAD | sudo tee /srv/novis/apps/telemetry/REVISION
```

**5. Install the service.** `nvs service install` registers the service with systemd. `--account`
runs it as the user `telemetry`.

```sh
sudo nvs service install telemetry --account telemetry -- serve --data /var/lib/telemetry /srv/novis/apps/telemetry/public/index.nvs
sudo nvs service start telemetry
```

**6. Configure Caddy.** Put this in `/etc/caddy/Caddyfile`:

```caddyfile
telemetry.example.com {
    reverse_proxy 127.0.0.1:8141
}
```

The site has no `log` directive, so Caddy writes no access log. Do not add one. An access log has the
address of every sender. Reload Caddy with `sudo systemctl reload caddy`.

**7. Update the code.** Pull the new commit, write `REVISION` again, and restart the service:

```sh
sudo git -C /srv/novis pull
sudo git -C /srv/novis rev-parse HEAD | sudo tee /srv/novis/apps/telemetry/REVISION
sudo nvs service stop telemetry
sudo nvs service start telemetry
```

**8. Point the client at it.** `nvs` sends nothing until `NVS_TELEMETRY_URL` is set, for example to
`https://telemetry.example.com/v1/report`. A release of `nvs` can also send to the collector by
default. That default address is one constant in `crates/nvs-cli/src/endpoints.rs`.
