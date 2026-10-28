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

A mount matches on `prefix`, on `host`, or on both, and names **either** `entry` (one literal file) **or** `scan` (a glob); both or neither is a boot error. A scan expands against the disk **at boot** into ordinary mounts, and again, in every mode, whenever the background check of `rule:config/an-edit-reaches-the-next-request-without-a-restart` sees a scanned directory's stamp move: a new module is compiled, has the `origin` check run on it, and is served; a removed one answers `404`; a new one that does not compile fails its own requests and nothing else. An explicit mount whose entry file appears or disappears is the same case. **A mount that does not compile at boot is kept the same way:** its diagnostic goes to the log, its requests answer `500` until an edit compiles, `nvs service status` lists it while the service runs, and its `origin` check runs in the pass it first compiles in. Boot fails only when no mount compiles, or when a compiled one fails the `origin` check. `*` matches exactly one segment; a capture must match `[A-Za-z0-9._-]+`, may not begin with a dot, and is refused if it names a reserved Windows device. `{n}` expands to the nth capture exactly as the disk spells it, and `{n:lower}` to the same capture in ASCII lower case — so a directory named `Blog` for the namespace it holds is served at `/blog` by `prefix = "/{1:lower}"`, while `Core\Request::mount()` still returns `Blog`. `lower` is the only transform. Any other brace in `prefix`, `host` or `origin` is a boot error and an `nvs config check` error: a reference past the last `*`, braces around anything that is not `n` or `n:lower`, and a brace that pairs with nothing. A host is held in ASCII lower case, which is how a request's `Host` is compared, so two hosts that differ only in case are one key. **`[server] root` is always written.** A server is told which directory it serves out of, and a mount only maps a URL onto a file under it; a mount table with no root is a boot error and an `nvs config check` error (`E0621`, at the first block), because a root that defaulted to the directory the process was started in would make the executable set a property of the shell. A relative root resolves against the file it is written in. Every resolved path is checked to lie inside `[server] root` each time the table is expanded, and a match boot would refuse is logged and left out of a later expansion while the rest of the table is published. An explicit mount overrides a scanned one at the same key; two explicit mounts at one key is a boot error. With no block written there is one implicit mount, `{ prefix = "/", entry = "public/index.nvs" }`.

The matched prefix is stripped: `Core\Request::path()` is the remainder, `Core\Request::mount()` answers what was removed and the `tainted` captures (`rule:routing/a-request-reads-its-mount`), and `Core\Router::url` prepends the prefix (`rule:routing/link-carries-the-mount-prefix`). A module is therefore relocatable — the same compiled route table serves at `/ModuleA` or at `/` with no recompile. `origin` is per mount, `System`-class and reloadable, with the `[[app]]` block's `origin` as the fallback (`rule:routing/an-origin-is-per-mount-and-checked-at-boot`).

A reload reaches the table too. The expansion reads `[server] root` and `[[server.mount]]` from the
snapshot a reload published, and a reload that changes either expands the table again at once, with no
wait for `settle`. A reload that changes `[[app]] origin` folds it into the rows again. Either way the
`origin` check is asked of every row that changed.

Because a scan reads the disk, dropping a directory in place makes a module reachable, and a
deployment that did not intend that must not put an unexpected directory under `[server] root`.
`nvs info --config` prints the expanded table, which is what makes that inspectable.

**Declined:**
- **A boot that fails when any mount does not compile.** A table of many modules would then stop
  serving every one of them for a typo in one, and a supervised server would restart in a loop that
  cannot fix the typo. A running server already keeps a broken mount and fails only its requests, so
  boot and a later edit now behave the same. A deploy that waits only for the port counts a partly
  broken start as healthy, and `nvs service status` is where the operator sees the broken mounts.
- **One entry point per process.** A document root of many modules, each with its own
  `public/index.nvs`, is the normal shape of a PHP deployment. One entry point per process answers it
  with one process, one unit cache and one resident footprint per module, giving up the shared
  compiled-code cache, to defend an invariant (`rule:http-server/a-path-is-never-derived-from-a-url`)
  that a table of any size already keeps.
- **A `case = "lower"` key on the mount block.** It reopens the closed key set
  (`rule:http-server/a-mount-carries-no-policy`), and it cannot say "lower in the prefix, verbatim in
  the origin's path"; a transform on the reference can.
- **Lower-casing every capture in a prefix unasked.** It would move the URLs of every existing
  mixed-case deployment.
- **An unwritten `[server] root` resolved beside the configuration file.** A service would then find
  what the shell found, but it is still a root nobody wrote, and the directory a server executes files
  out of is the one setting that is never inferred.
- **A root required only by `nvs service install`.** One configuration would then mean one set of
  files from one shell and another from a second, and be refused by a third command.
- **An `upper` transform.** No deployment asks for it, and the closed set is one line to extend.
- **A brace that is not a reference, read as a literal brace.** A host name never carries a brace and a
  URL path carries one only percent-encoded, while the prefix is compared with the path as sent, so such
  a mount boots and nothing reaches it.
