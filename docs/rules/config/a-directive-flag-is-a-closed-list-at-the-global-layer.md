A command-line flag that maps to a directive **wins over every file, and the set of such flags is
closed**: `nvs serve --listen` and `nvs serve --port`, both over `[server] listen`
(`crates/nvs-cli/src/main.rs:535-551`). A later rule that wants another one argues for it by name
where it introduces the directive. Stated once, so that no rule introducing a flag has to assert its
own "ordinary CLI precedence".

`--listen` names one address and replaces the whole `[server] listen` set. `--port` keeps the host
each entry chose and replaces only its port. The two conflict, because two spellings of one address
are two requests and guessing which was meant is worse than saying so
(`crates/nvs-cli/src/serve.rs:2176`).

**There is no `--set <key>=<value>`.** It would be a second spelling for every directive in the
format; it would put values into argv, which is world-readable through `ps` and `/proc/*/cmdline`, so a
secret could be set there in a way `rule:config/ownership-is-the-trust-boundary` has no equivalent of;
and the ergonomic case for it disappeared when every project command acquired an `nvs.toml` to edit
for free, in the data folder, and `nvs init` wrote one into a project on request.

A flag **replaces the global value**, and per-app blocks still layer over it. The whole stack,
outermost first:

```
shipped defaults
 -> the file tree      (--config list + includes, later wins)
   -> CLI flags        (closed list, replaces the global value)
     -> [[app]] blocks (all that match, least-specific root first)
       -> Core\Config::set  (per-request overlay, unchanged)
```

**Declined:**
- **`nvs serve --mode`.** The mode a server starts in is `[mode] default` in the root-owned file and
  nowhere else, so a review of the file shows the mode a server runs in, and saving it changes the
  mode with no restart (`rule:config/the-config-is-an-immutable-snapshot`). A flag would be a second
  source the file does not show, and a stale `--mode=development` in a deploy script would start a
  public server in development mode.
