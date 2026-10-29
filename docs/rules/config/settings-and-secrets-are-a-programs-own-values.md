Four blocks hold values a program reads by a name the operator chooses: `[settings]` and `[secrets]`
for every application, and `[app.settings]` and `[app.secrets]` under an `[[app]]` block for that
application. They are the only open blocks. Every other block stays closed, and an unknown key there
is still `E0601`.

```toml
[settings]
page_size = 20

[settings.api]
base = "https://api.example.com"

[secrets]
api_key_file = "/run/secrets/api_key"
```

**A key is a plain name**, `[A-Za-z_][A-Za-z0-9_]*`, at every depth. A sub-table adds a dotted
level, so `[settings.api] base` is read as `api.base`. A quoted key that is not a plain name is
`E0601`, because `"api.base"` and `[settings.api] base` would be two settings read by one name.

**A setting keeps its TOML type**: a string is a `string`, an integer an `int`, a float a `float`, a
boolean a `bool`, an array an array, and a table an array keyed by name. A TOML date or time is
`E0601`, and its help is the same value quoted as a string. Novis has no value a TOML date maps to
without a time-zone choice, and a string leaves that choice to the program.

**A secret is a string, written inline or read from a file.** Each `[secrets]` key is `name = "…"`
or `name_file = "…"`, exactly one of the two, and both is `E0608`, as for a secret directive
(`rule:config/a-secret-is-a-file-whose-content-is-the-value`). The file is read by the same reader,
under the same rules. A secret that is not a string is `E0601`, so `[secrets]` has no sub-table and
a secret's own name never ends in `_file`. Every value is carried beside the merged table under
`secrets.<name>` and never in it: the table keeps only the `_file` half, so `nvs config dump` prints
each secret as `<secret>` beside the file it came from. An inline secret names the configuration
file it was written in.

The key checks and the type checks run per file, before the merge, so a refusal names the line.
`[[include]]` overrides a value as it overrides any directive, and `[app.settings]` and
`[app.secrets]` fold over the global blocks key by key, least-specific first
(`rule:config/every-matching-app-block-applies-least-specific-first`). Both record the override with
both origins (`rule:config/later-wins-and-every-override-is-recorded`). That is how one machine sets
its own values: the base file holds the defaults, and a file on that machine overrides them.

**Two read-only methods, and nothing else in the API:**

| Method | Returns |
|---|---|
| `Core\Config::setting(string $name): ?(string\|int\|float\|bool\|array<mixed>)` | the value with its type, or `null` when no block sets it |
| `Core\Config::secret(string $name): ?secret string` | the secret, or `null` when no block sets it |

A program converts a setting with `as`: `Core\Config::setting('page_size') as int` throws when the
value is missing or is not a whole number. `secret`'s name is a sink, as `Core\Env::secret`'s is, so
a request cannot choose which secret it reads. A setting is not `tainted`: the operator wrote it in
a file only the runtime account or root may change. That is why `setting` returns a union of the
TOML types and not `mixed`: text out of `mixed` is `tainted` (`rule:security/taint-propagation`).
A table's values are `mixed`, so a program reads one untainted by its dotted name, `api.base`.

**`get`, `all`, `set` and `restore` never see the four blocks.** `Core\Config::get('secrets.x')`
returns `null`, and `all` lists no `settings.` or `secrets.` key. Without this, a secret would come
back from `get` as a plain `?string`, which every output accepts.

**A package reads the same values the application does**, as `Core\Env::secret` does not ask who
calls it. Which package may read what is the grants model's question
(`rule:security/package-authority-is-granted-one-line-at-a-time`).

**The typo check is a warning from `nvs config check`.** The checker records every name passed to
`setting` or `secret` as a literal string, per program. `nvs config check` compiles the programs the
tree names, each `[[app]] entry` and each `[[server.mount]]` entry, and every `--program <file>`
given, and raises `W1028` for each key in the four blocks that none of them reads. When any of them
passes a name that is not a literal, no key is judged, and one line says so. It is a warning because
a key may be read by a program the tree does not name. `nvs check` stays a source check that never
reads `nvs.toml` (`rule:config/nvs-toml-is-not-a-project-manifest`), and the boot compiles nothing
to look.

**What it spends.** A `setting` read is one lookup in the request's snapshot and one value built,
with nothing read from disk. The values are part of the config snapshot, stored once per snapshot
and never per request. A file that writes one of the four blocks is parsed once more at boot and at
each reload. `nvs config check` compiles each named program, which is seconds offline and nothing on
a server. Security: no capability can be set in the new blocks, every other block stays closed, and
a secret is typed `secret` and redacted wherever the configuration is printed.

**Declined:**
- **A free-form block with no declared keys and no typo check.** A typo'd key reads as "not set",
  which is the failure the unknown-key refusal exists to stop.
- **A schema the program declares, which the boot checks against.** `nvs.toml` is read before any
  program is compiled, and a schema would be source-tree state in a deployment file.
- **`${ENV}` interpolation into a setting**, already declined by
  `rule:config/the-file-is-nvs-toml-and-it-is-toml`.
- **Reading settings through `get` as strings.** Every value would need a parse, and a secret would
  come back as a plain string.
- **A `settings()` method that returns every value.** It is the way a whole set of secrets ends up
  in a log.
- **A `set` for settings.** A request that needs another value uses a variable.
- **A package that may not read the values.** A limit for these two methods would be a rule of its
  own, under the grants model.
