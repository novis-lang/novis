`[mode] ceiling` states the most permissive mode any code on the host may select. It is `System`-class — a request can never raise it (`rule:config/ceilings-are-their-own-directives`) — and **when unset it equals the mode the server started in.**

| Deployment | Written | Result |
|---|---|---|
| Production host | nothing | Started in `production`, ceiling `production`. No code path anywhere reaches development mode. |
| Developer's machine | `[mode] default = "development"` | Ceiling `development`. Flips are free; nothing else to configure. |
| One host, mixed applications | `[mode] default = "production"`, `[mode] ceiling = "development"` | Production by default, and each application selects its own — in its `[[app]]` block, or in code. |

**The ceiling bounds a runtime flip, not the startup value.** An `nvs.toml` that writes `[mode] default = "development"` and no ceiling simply works: the file sets the startup mode and the ceiling follows it. A ceiling that also bound startup would make every development configuration write the mode twice. Above the ceiling, `Core\Config::set("mode.default", …)` returns `false` and leaves the mode unchanged (`rule:config/a-program-may-read-and-flip-its-mode`).

For the mixed host, per-app configuration is the primary answer and the in-code flip is the escape hatch: the `[[app]]` block involves no application code and nothing the application can get wrong (`rule:config/a-mount-routes-and-an-app-block-sets-policy`). `Core\Config::set` is for when the application knows something the operator does not.
