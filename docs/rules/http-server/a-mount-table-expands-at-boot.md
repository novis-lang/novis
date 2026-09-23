```toml
[server]
root = "/www"                             # required; every mount path must resolve inside this

[[server.mount]]
scan   = "*/public/index.nvs"             # a glob under root; * captures one segment
prefix = "/{1}"                           # or host = "{1}.example.com"
origin = "https://{1:lower}.example.com"  # optional — what urlAbsolute prepends

[[server.mount]]
prefix = "/admin"
entry  = "Backoffice/public/index.nvs"    # an explicit mount overrides a scanned one
```

A mount matches on `prefix`, on `host`, or on both, and names **either** `entry` (one literal file) **or** `scan` (a glob); both or neither is a boot error. A scan expands against the disk **at boot** into ordinary mounts, and again, in every mode, whenever the background check of `rule:config/an-edit-reaches-the-next-request-without-a-restart` sees a scanned directory's stamp move: a new module is compiled, has the `origin` check run on it, and is served; a removed one answers `404`; a new one that does not compile fails its own requests and nothing else. An explicit mount whose entry file appears or disappears is the same case. `*` matches exactly one segment; a capture must match `[A-Za-z0-9._-]+`, may not begin with a dot, and is refused if it names a reserved Windows device. `{n}` expands to the nth capture exactly as the disk spells it, and `{n:lower}` to the same capture in ASCII lower case — so a directory named `Blog` for the namespace it holds is served at `/blog` by `prefix = "/{1:lower}"`, while `Core\Request::mount()` still returns `Blog`. `lower` is the only transform. Any other brace in `prefix`, `host` or `origin` is a boot error and an `nvs config check` error: a reference past the last `*`, braces around anything that is not `n` or `n:lower`, and a brace that pairs with nothing. A host is held in ASCII lower case, which is how a request's `Host` is compared, so two hosts that differ only in case are one key. **`[server] root` is always written.** A server is told which directory it serves out of, and a mount only maps a URL onto a file under it; a mount table with no root is a boot error and an `nvs config check` error, because a root that defaulted to the directory the process was started in would make the executable set a property of the shell. A relative root resolves against the file it is written in. Every resolved path is checked to lie inside `[server] root` each time the table is expanded, and a match boot would refuse is logged and left out of a later expansion while the rest of the table is published. An explicit mount overrides a scanned one at the same key; two explicit mounts at one key is a boot error. With no block written there is one implicit mount, `{ prefix = "/", entry = "public/index.nvs" }`.

The matched prefix is stripped: `Core\Request::path()` is the remainder, `Core\Request::mount()` answers what was removed and the `tainted` captures (`rule:routing/a-request-reads-its-mount`), and `Core\Router::url` prepends the prefix (`rule:routing/link-carries-the-mount-prefix`). A module is therefore relocatable — the same compiled route table serves at `/ModuleA` or at `/` with no recompile. `origin` is per mount, `System`-class and reloadable, with the `[[app]]` block's `origin` as the fallback (`rule:routing/an-origin-is-per-mount-and-checked-at-boot`).

**What is on disk.** All of it for a change on disk (`nvs_cli::serve`'s `mounts`). The expansion
reads the configuration the server booted on, so a reload that changes `[[server.mount]]`,
`[server] root` or `[[app]] origin` does not reach the table yet.
