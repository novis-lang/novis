```toml
[server]
root = "/www"                             # every mount path must resolve inside this

[[server.mount]]
scan   = "*/public/index.nvs"             # a glob under root; * captures one segment
prefix = "/{1}"                           # or host = "{1}.example.com"
origin = "https://{1}.example.com"        # optional — what urlAbsolute prepends

[[server.mount]]
prefix = "/admin"
entry  = "Backoffice/public/index.nvs"    # an explicit mount overrides a scanned one
```

A mount matches on `prefix`, on `host`, or on both, and names **either** `entry` (one literal file) **or** `scan` (a glob); both or neither is a boot error. A scan expands against the disk **at boot** into ordinary mounts, and again on `nvs ctl reload` — in development also under hot reload's revalidation. `*` matches exactly one segment; a capture must match `[A-Za-z0-9._-]+`, may not begin with a dot, and is refused if it names a reserved Windows device. Every resolved path is checked to lie inside `[server] root`, once, at boot. An explicit mount overrides a scanned one at the same key; two explicit mounts at one key is a boot error. With no block written there is one implicit mount, `{ prefix = "/", entry = "public/index.nvs" }`.

The matched prefix is stripped: `Core\Request::path()` is the remainder, `Core\Request::mount()` answers what was removed and the `tainted` captures (`rule:routing/a-request-reads-its-mount`), and `Core\Router::url` prepends the prefix (`rule:routing/link-carries-the-mount-prefix`). A module is therefore relocatable — the same compiled route table serves at `/ModuleA` or at `/` with no recompile. `origin` is per mount, `System`-class and reloadable, with the `[[app]]` block's `origin` as the fallback (`rule:routing/an-origin-is-per-mount-and-checked-at-boot`).
