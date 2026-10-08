A command-line flag that maps to a directive **wins over every file, and the set of such flags is
closed** — `--mode`, `--listen`, `--port`, and whatever a later rule argues for by name where it
introduces the directive. Stated once, so that no rule introducing a flag has to assert its own
"ordinary CLI precedence".

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

So `nvs serve --mode=development` on a mixed host does not drag an application that pins `production`
along with it, and `[mode] ceiling` still bounds what any of them may select.
