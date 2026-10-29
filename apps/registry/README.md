# Registry

The package registry of Novis, written in Novis. An account is one or more passkeys and ten recovery
codes. It has no email and no password. The data is kept in SQLite.

Run these commands in this folder:

```sh
nvs serve public/index.nvs --config nvs.toml   # open http://localhost:8000/
nvs test . --config nvs.toml                   # runs the tests
```

## The settings

`hosting.json` has the settings of one installation. The file in this folder is for one computer,
and a deployment writes its own. The registry does not start when a file that a setting names does
not exist.

| Setting | What it is |
|---|---|
| `origin` | The address of the site. Passkeys work only on it. |
| `signUpWork` | The number of zero bits the proof-of-work of a sign-up needs. Each extra bit doubles the work. |
| `nvs` | The `nvs` program the registry runs to build, sign and log. |
| `gitCaFile` | A PEM file with the certificates a git server may use, or `""` for the system's certificates. |
| `projectAccount` | The account that owns the vendors `Core` and `Novis`, or `""`. |
| `work` | The folder where the registry makes the public files. It is never served. |
| `stores` | Where the public files go. Every store gets every file. |
| `hosts` | The `https:` addresses that clients read the public files from, in order. |
| `backup`, `backupKey`, `backupMinutes` | The store for the copies of the database, the key that encrypts them, and the minutes between two copies. |
| `downloadLog` | Where the access lines of the file hosts are, or `null`. |
| `logKey`, `logName` | The key file that signs the log and the index files, and the name of that key. |
| `pushKey`, `pushSubject` | The key that signs each Web Push message, and your contact address. |
| `checkpointCopy` | The `https:` address of a second copy of the log's checkpoint, on another host. You can leave it out. |

The registry makes the files that `nvs pkg` reads in the folder `work`: `registry.json`, the
index files, the archives and the log. Then it copies each new file to every store in `stores`.
The archives go first and the checkpoint of the log goes last. A publish is finished only when
every store has every file. If a store fails, publish the same commit again.

A store is a folder, `{ "kind": "dir", "path": "files" }`, or a web server that takes a `PUT`
request for each file, `{ "kind": "http", "url": "...", "header": "AccessKey", "secret": "..." }`.
`secret` is the path of a file with the value of the header. Bunny Storage and a WebDAV server
both work as an `http` store.

With no `hosts`, the registry sends the files itself, from the first `dir` store. It then counts
each download as it sends it, and `downloadLog` is `null`.

The registry writes a copy of the database, `registry.copy`, to `backup` after each publish. It
also writes one every `backupMinutes` minutes when something changed. `backupKey` is a file with
64 hex characters, such as the output of `openssl rand -hex 32`. With `"backupKey": ""`, the
registry makes no copies.

`logKey` is a file that `nvs registry key` makes. The committed `hosting.json` has
`"logKey": ""`, so the feeds at `/feeds/...` answer `503` on your computer. A deployment always
names a key.

`pushKey` is a PEM file with a P-256 private key. This command makes one:
`openssl ecparam -name prime256v1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt`. With
`"pushKey": ""`, no push device can be added. `pushSubject` is a `mailto:` or `https:` URL, and
each push service sees it.

After each retraction and each withhold, the registry writes the advisory feed `log/advisories`
and sends it with the log. The feed lists every retraction and withhold in the log, and the log's
key signs it. `nvs pkg audit` reads it.

`checkpointCopy` names a copy of the file `log/checkpoint` that somebody else publishes, such as
`https://mirror.example.org/registry.example.com/checkpoint`. `registry.json` gives this address
to every client. `nvs pkg audit` reads both copies. If they do not agree, it reports an error.
With no `checkpointCopy`, audit says that there is no second copy.

## Deploy on one server

This guide runs the registry on one Linux server. The public files are in Bunny Storage, in two
regions, and a second provider has the copies of the database. Caddy receives the HTTPS requests
and sends them to the registry. The examples use `registry.example.com` for the site and
`files.example.com` and `files2.example.com` for the file hosts.

### 1. Make the keys on a computer that is not online

```sh
nvs registry key log.key --name registry.example.com   # prints the public key for [registries]
openssl rand -hex 32 > backup.key
openssl ecparam -name prime256v1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt > push.pem
```

Keep an encrypted copy of `log.key` and `backup.key` on that computer, for example with
`age --passphrase`. Never put either key in a store or in a backup of the server. Without
`log.key`, the registry cannot sign the next checkpoint, and every client stops trusting it.
Without `backup.key`, nobody can read a copy of the database.

### 2. Make two accounts and the folders

The registry and the telemetry service (`apps/telemetry`) each run as their own account. Then
the telemetry service cannot read the database or the keys of the registry.

```sh
useradd --system --home /srv/registry registry
useradd --system --home /srv/telemetry telemetry
install -d -o root -g registry -m 0750 /etc/registry
install -o root -g registry -m 0440 log.key backup.key push.pem /etc/registry/
```

Copy `apps/registry` to `/srv/registry`, and `apps/telemetry` to `/srv/telemetry`. Each account
owns its own folder. Install `nvs` at `/usr/local/bin/nvs`. Set the server's clock to UTC, because
the daily job counts the downloads of the day before in UTC.

### 3. Set up Bunny

Make these in Bunny's panel:

- Two storage zones for the public files, with their main region in two places, such as
  `registry-files` in Frankfurt and `registry-files-ny` in New York.
- A pull zone for each of them, with the addresses `files.example.com` and `files2.example.com`.
- A third storage zone, `registry-logs`, for the access lines. Turn on *Permanent Log Storage* in
  the first pull zone, and choose `registry-logs`. Do not connect a pull zone to `registry-logs`.

Write the password of each storage zone to its own file in `/etc/registry`, with the same owner
and mode as the keys. The password of a storage zone works only for that zone. The registry never
needs the API key of your Bunny account.

Bunny removes the last part of each address in the access lines unless you change that setting.
Then all the addresses of one network count as one download per day.

### 4. Set the cache time of each file

The service never clears the cache of a pull zone. A file that can change must have a short cache
time. Add these edge rules to each pull zone:

| Files | Cache time |
|---|---|
| `/archive/*`, and `/log/tile/*` except `*.p/*` | one year. These files never change. |
| `/registry.json`, `/index/*`, `/log/checkpoint` and `*.p/*` | 60 seconds |

### 5. Write `hosting.json`

```json
{
  "origin": "https://registry.example.com",
  "signUpWork": 20,
  "nvs": "/usr/local/bin/nvs",
  "gitCaFile": "",
  "projectAccount": "",
  "work": "work",
  "stores": [
    { "kind": "http", "url": "https://storage.bunnycdn.com/registry-files/", "header": "AccessKey", "secret": "/etc/registry/files.key" },
    { "kind": "http", "url": "https://ny.storage.bunnycdn.com/registry-files-ny/", "header": "AccessKey", "secret": "/etc/registry/files-ny.key" }
  ],
  "hosts": ["https://files.example.com/", "https://files2.example.com/"],
  "backup": { "kind": "http", "url": "https://webdav.example.net/registry/", "header": "Authorization", "secret": "/etc/registry/webdav.key" },
  "backupKey": "/etc/registry/backup.key",
  "backupMinutes": 5,
  "downloadLog": { "kind": "http", "url": "https://storage.bunnycdn.com/registry-logs/pullzone-logs/registry-files/", "header": "AccessKey", "secret": "/etc/registry/logs.key", "format": "bunny" },
  "logKey": "/etc/registry/log.key",
  "logName": "registry.example.com",
  "pushKey": "/etc/registry/push.pem",
  "pushSubject": "mailto:registry@example.com"
}
```

`signUpWork` 20 makes each sign-up 4,096 times the work of the 8 in the committed file. `webdav.key` has the whole value of the header,
such as `Basic dXNlcjpwYXNzd29yZA==`. The backup store must not be a public store.

`downloadLog` is the folder of the first pull zone in `registry-logs`. Bunny writes the access lines
of each day in parts, such as `2026/03/04_100-0-a1b2c3d4.gzip`. The daily job reads every part of
the day before, counts the downloads by `nvs`, and keeps no line.

### 6. Add the server settings to `nvs.toml`

Add these blocks to `/srv/registry/nvs.toml`:

```toml
[mode]
default = "production"

# Caddy runs on this server and sends each request from 127.0.0.1.
[server]
listen = ["127.0.0.1:8000"]
trusted_proxies = ["127.0.0.1"]

# No line the server writes has the address of the sender.
[log]
client_ip = false

# The registry runs this program, and no other.
[capabilities.process]
exec = ["/usr/local/bin/nvs"]
```

Add `"/etc/registry"` to the `read` list of `[capabilities.fs]`, so the registry can read the keys
and the passwords.

### 7. Start the two services

```ini
# /etc/systemd/system/registry.service
[Unit]
Description=Novis registry
After=network-online.target

[Service]
User=registry
WorkingDirectory=/srv/registry
ExecStart=/usr/local/bin/nvs serve public/index.nvs --config nvs.toml
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

`telemetry.service` is the same, with `User=telemetry`, `WorkingDirectory=/srv/telemetry` and
`nvs serve public/index.nvs` on another port. Then:

```sh
systemctl enable --now registry telemetry
```

Caddy sends `registry.example.com` to `127.0.0.1:8000`. The registry runs its jobs itself, from the
`[[schedule]]` blocks in `nvs.toml`.

## Move to a new server

1. Stop the old server, and make sure it cannot start again: `systemctl disable --now registry`.
2. Set up the new server as above, with the same `hosting.json` and the same keys.
3. Start it with no `registry.sqlite`.

The registry then restores itself. It reads the newest copy of the database from `backup`, takes
the log from the first store, checks the log with `nvs registry log verify`, and adds every release
in the log that the copy does not have. If the stores have a log and `backup` has no copy, the
registry does not start.

Never run two servers at the same time. Two servers sign two different checkpoints of the same
size, and every client then stops with an error that says the log was forked.

A restore does not bring back everything:

- The signature of an index file is not checked during a restore.
- A release that only the log has gets an empty folder in the repository, and its API file is
  missing. The next release of that package is then built with no API to compare to.
- An unlist, a retraction, a withheld version or a confirmation after the newest copy comes back
  only from the index files, not from the log.
- A token made after the newest copy is lost. Its owner makes a new one.
