A `[[server.mount]]` carries no `mode`, no limits and no capabilities. Its key set is closed — `prefix`, `host`, `scan`, `entry` and `origin` — and those keys say where a request arrives and which file answers it. Everything about what the code answering it *may do* belongs to the `[[app]]` block, keyed on the entry file path (`rule:config/a-mount-routes-and-an-app-block-sets-policy`, `rule:config/an-application-is-its-entry-file-path`).

```toml
[[server.mount]]
prefix = "/shop"                      # routing
entry  = "shop/public/index.nvs"

[[app]]
root = "/srv/www/shop"                # policy
mode = "production"
```

The two usually cover the same tree, and that is the intended shape. An application's identity is its entry file path, not its mount, so `nvs run` on the command line has one too and per-app configuration is reachable with no server at all. A mixed-application host — production by default, each application selecting its own mode — is expressed here, not in a mount key (`rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode`).
