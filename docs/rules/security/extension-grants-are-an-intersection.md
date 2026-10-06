What an extension may read, write or call at a given moment is the **intersection** of three sets,
each of which only narrows: what the operator grants on the extension's own `[[extension]]` entry, what
the extension's manifest declares it needs, and what the calling code's namespace holds at the call.

```toml
[[extension]]
path   = "geo.nvsx"
sha256 = "…"
grants = { read = ["data/geo/"], write = [], connect = ["tiles.example.com"] }
```

`read` and `write` name filesystem roots under the `fs.read` and `fs.write` capabilities, and become
`wasi:filesystem` preopens, opened read-only and read-write; `connect` names hosts under `net.connect`,
and governs `wasi:http`'s outgoing handler. Relative roots resolve against the file the entry is
written in (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`). No other
capability can be granted to a guest.

An entry with no `grants` holds no I/O, whatever its manifest requests, and `nvs ext inspect` prints
what a manifest requests so the operator can read it before writing a grant. An extension never acts
with more authority than the code that called it, so a request narrowed by an isolate or by its
namespace stays narrowed through the extension. The preopens are computed per instance from the
request's own configuration snapshot (`rule:security/capability-question-is-grant-and-scope`), and a
path is canonicalised and prefix-checked as every `Core\IO` door does
(`rule:security/path-scope-canonicalise-then-prefix`).

**Not on disk.** `nvs_config::tree::Grants` reads the entry's `grants`, refusing an unknown key,
and `nvs_config::extension::grants` makes its roots absolute against the file that wrote each list;
`env_hash` does not read it. Nothing intersects it with a manifest or a caller, and no guest holds a
preopen or a host.
