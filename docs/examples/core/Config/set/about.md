Changes one setting for this request.

No other request sees the change, and it is gone when the request ends. The name is the same as for
`Core\Config::get`. The value is written as in a configuration file: `512M`, `30s`, `64`.

The result is `true` when the change was made. It is `false` when the change is not allowed, and the
old value stays. This happens for a setting that is not in the table below, a value in the wrong
format, or a value above the ceiling. `set` throws no error, so check the result.

When you write the name in the call, the compiler checks it. A name that is not in the table does
not compile. A name in a variable is checked when the program runs, and `set` returns `false`.

A request may change only these settings. A limit can also be written with its short name, such as
`memory`.

| Setting | What a request may set |
|---|---|
| `mode.default` | `production`, or `development` when `[mode] ceiling` allows it |
| `limits.memory` | any size up to the ceiling |
| `limits.cpu_time` | any duration up to the ceiling |
| `limits.wall_time` | any duration up to the ceiling |
| `limits.max_output` | any size up to the ceiling |
| `limits.request_body` | any size up to the ceiling |
| `limits.upload_total` | any size up to the ceiling |
| `limits.max_multipart_parts` | any count up to the ceiling |
| `limits.max_tasks` | any count up to the ceiling |
| `limits.max_regex_steps` | any count up to the ceiling |
| `log.format` | any value |
| `log.level` | any value |
| `errors.deprecated` | any value |
| `deferred.deadline` | any value |
| `debug.inline` | only `false` |
| `debug.store` | only `false` |
| `http.headers.content_type_options` | any value |
| `http.headers.frame_ancestors` | any value |
| `http.headers.referrer_policy` | any value |
| `http.headers.hsts` | any value |
| `http.headers.hsts_subdomains` | any value |
| `http.headers.content_security_policy` | any value |
| `http.headers.permissions_policy` | any value |
| `http.cors.origins` | any value |
| `http.cors.expose` | any value |
| `http.cors.credentials` | any value |
| `http.cookies.secure` | any value |
| `http.cookies.http_only` | any value |
| `http.cookies.same_site` | any value |
| `http.cookies.path` | any value |
| `http.client.connect_timeout` | any value |
| `http.client.deadline` | any value |
| `http.client.max_redirects` | any value |
| `http.client.idle` | any value |
| `http.client.max_duration` | any value |
| `http.client.socket.max_message` | any value |
| `http.client.socket.send_timeout` | any value |

"Any value" means any value the configuration file allows for that setting.

The ceiling of a limit is in `[limits.hard]`. Without it, the ceiling is the starting value, so a
program can lower that limit but not raise it. A command-line program may raise `memory` to any
value, unless `[limits.hard]` sets `memory`.

`debug.inline` and `debug.store` take only `false`. A request can turn them off for itself, and no
request can turn them on. `Core\Config::restore` gives them the value from the file again.

Code in a package may change only a `[limits]` setting, and only to a lower value. For every other
setting, `set` in a package returns `false`.

**Good to know:** `Core\Config::restore` puts back the value from the configuration file, or the
built-in default.
