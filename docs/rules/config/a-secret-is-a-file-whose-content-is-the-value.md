A directive the registry marks **secret** — `[db.<name>] password`, `[mail.<name>] password`,
`[ldap.<name>] password`, `[http.client.proxy] password`, `[http] csrf_key` and `[cache.shared] password` —
gains a `_file` sibling. Exactly one of the pair may be set; both is a refusal, so this is two sources
for one value rather than a second spelling. The registry is one table, one row per pair, and a
credential on the typed tree with no row fails a census rather than parsing and never being read.

```toml
[db.main]
driver = "postgres"
password_file = "/run/secrets/db_password"
```

**The file's entire content is the value**, with exactly one trailing `\n` — and a `\r` immediately
before it — stripped if present, and nothing else trimmed. A password may legitimately begin or end
with a space, and removing it is `rule:errors/ambiguous-input-refused`'s failure of repairing input
instead of reading it; an edge space is reported as a warning and kept. One newline goes because
`echo secret > f` produces one; a value that genuinely ends in a newline is written with two. The
content must be valid UTF-8, and an empty file, a whitespace-only file, or one over 64 KiB is refused
(`E0608`).

Only the `password_file` that won the merge is opened. The value never enters the merged table, is
never logged, and `nvs config dump` renders it `<secret>` beside the file it came from —
`password_file` stays set so the dump can name it. `Core\Config::all` renders every secret directive
`<secret>` too, whether a file or the tree set it, because a page or a log line that prints every
setting is the usual way a credential leaks. `Core\Config::get` returns the value: it is the one
reader a program that connects with the credential needs.

**Declined:**
- **Decrypting SOPS, `age` or a KMS envelope inside the binary**, so that a directive could name an
  encrypted file. It does not compose with the format: SOPS has no TOML store, so an encrypted
  configuration would be one opaque blob ahead of the merge, the includes and the dump. It puts the
  key that unlocks every credential into the process that compiles a program and parses hostile bytes,
  where today that process holds one plaintext credential. And it adds nothing: `sops -d`, a Vault
  agent, Sealed Secrets and a Kubernetes secret volume all end in a file, which a `_file` sibling
  reads unchanged. A deployment target that cannot produce a file at all is what would reopen this.
- **A `_file` sibling for every string directive.** It doubles the key space of the format and
  complicates the unknown-key refusal, to serve the handful of directives the registry marks.
- **An `_env` sibling, a third source naming an environment variable** (`password_env =
  "DB_PASSWORD"`). It is the narrow form of interpolation: it cannot grant, and it serves a platform
  that injects credentials only as variables. It still puts the credential where every child process
  inherits it and `/proc/<pid>/environ` and a crash dump show it, so `process.exec` would owe a scrub,
  and it serves no target that cannot also mount a file. The trigger above reopens it too.
