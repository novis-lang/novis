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

**`[secrets]` and `[app.secrets]` take the same pair** for every name the operator writes there:
`name` or `name_file`, exactly one, read by the same reader under the same rules. Their values are
`Core\Config::secret`'s and never `get`'s
(`rule:config/settings-and-secrets-are-a-programs-own-values`).

**`get` returns a secret directive as a `?secret string`, and only by its written name.** The
checker types `Core\Config::get('db.main.password')` as `?secret string` when the literal is a
secret directive, read from the same table the boot fills them from, so no output, log or
serialiser takes the value (`rule:security/secret-sinks-refuse`). Every other directive stays a
`?string`. A name the program computes cannot be typed by its value, so `get` throws `LogicError`
when a computed name turns out to be a secret directive: its result would otherwise be a plain
`?string` any sink takes. A method reference to `get` is a computed name too. **What it spends:**
one walk of the secret rows per `get`, and one word the compiler passes beside the name.

**Declined:**
- **A `secret` return for every `get`.** Most settings are a size, a duration or a level a program
  prints and compares, and every one of them would need a conversion.
- **A computed secret name answered `null`.** It reads as "not configured", which is the one fact a
  program checks before it connects, and it would be wrong.
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
