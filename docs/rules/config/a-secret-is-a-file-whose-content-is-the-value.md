A directive the registry marks **secret** — `[db.<name>] password`, `[mail.<name>] password`,
`[http.client.proxy] password`, `[http] csrf_key` and `[cache.shared] password` today —
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
`password_file` stays set so the dump can name it.
