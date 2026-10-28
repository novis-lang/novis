# Registry

The package registry of Novis, written in Novis. An account is one or more passkeys and ten recovery
codes. It has no email and no password. The data is kept in SQLite.

Run these commands in this folder:

```sh
nvs serve public/index.nvs --config nvs.toml   # open http://localhost:8000/
nvs test . --config nvs.toml                   # runs the tests
```

`hosting.json` has the settings of one installation. `origin` is the address of the site, and
passkeys work only on it. `signUpWork` is the number of zero bits the proof-of-work of a sign-up
needs. Each extra bit doubles the work. The file in this folder is for one computer, and a
deployment writes its own.

The registry makes the files that `nvs pkg` reads in the folder `work`: `registry.json`, the
index files, the archives and the log. Then it copies each new file to every store in `stores`.
The archives go first and the checkpoint of the log goes last. A publish is finished only when
every store has every file. If a store fails, publish the same commit again.

A store is a folder, `{ "kind": "dir", "path": "files" }`, or a web server that takes a `PUT`
request for each file. An example is
`{ "kind": "http", "url": "https://storage.example.com/registry/", "header": "AccessKey", "secret": "/etc/registry/storage.key" }`.
`secret` is the path of a file with the value of the header. The registry does not start when a
file that a setting names does not exist.

`hosts` lists the file hosts that `registry.json` names. With no hosts, the registry sends the
files itself, from the first `dir` store. `backup` is the store for copies of the database, and
`backupKey` is the file with the key that encrypts them. The key is 64 hex characters, such as the
output of `openssl rand -hex 32`. With `"backupKey": ""`, the registry makes no copies. The
registry writes a copy, `registry.copy`, after each publish. It also writes one every
`backupMinutes` minutes when something changed. When the registry starts with no database, it
reads the newest copy and the log from the first store. Then it adds every release in the log
that the copy does not have. `downloadLog` is the store with the access
lines of the file hosts, or `null` when the registry sends the files itself. Its `format` is
`bunny`, and its `file` is the name of the file of each day, such as `12345.log` for Bunny's pull
zone 12345. The daily job reads the lines of the day before from `MM-DD-YY/12345.log`, counts the
downloads by `nvs`, and keeps no line. `logKey` is the key
file that signs the log and the index files, and `nvs registry key` makes one. `logName` is the
name of that key.

`pushKey` is the file with the private key that signs each Web Push message. This command makes
one: `openssl ecparam -name prime256v1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt`. With
`"pushKey": ""`, no push device can be added. `pushSubject` is your contact address, a `mailto:`
or `https:` URL. Each push service sees it.
