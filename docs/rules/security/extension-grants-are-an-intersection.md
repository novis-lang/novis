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

The manifest's `read` and `write` lists say whether it asks for that kind at all, because an author
cannot know the operator's folders; its `connect` hosts are matched one by one. The caller narrows by
path: a root it holds inside an entry root replaces that root. A `write` root is opened read-write, so
the caller must hold both `fs.write` and `fs.read` over it.

**Not on disk.** `nvs_config::tree::Grants` reads the entry's `grants`, refusing an unknown key,
and `nvs_config::extension::grants` makes its roots absolute against the file that wrote each list;
`env_hash` does not read it. `nvs_ext::grants::effective` computes the intersection
(`crates/nvs-ext/tests/grants.rs`), and nothing calls it yet, so no guest holds a preopen or a host.
