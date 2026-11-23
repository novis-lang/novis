Changes one setting for this request.

No other request sees the change, and it ends with the request. The value is written as in a
configuration file: `512M`, `30s`, `64`.

`set` returns `true` when the change was made. It returns `false` when the change is not allowed,
and the old value stays.

A request may change only these settings:

- `[mode]`: `default`
- `[limits]`: `memory`, `cpu_time`, `wall_time`, `max_output`, `request_body`, `upload_total`, `max_multipart_parts`, `max_tasks`, `max_regex_steps`
- `[log]`: `format`, `level`
- `[errors]`: `deprecated`
- `[deferred]`: `deadline`
- `[debug]`: `inline`, `store`
- `[http.headers]`: `content_type_options`, `frame_ancestors`, `referrer_policy`, `hsts`, `hsts_subdomains`, `content_security_policy`, `permissions_policy`
- `[http.cors]`: `origins`, `expose`, `credentials`
- `[http.cookies]`: `secure`, `http_only`, `same_site`, `path`
- `[http.client]`: `connect_timeout`, `deadline`, `max_redirects`, `idle`, `max_duration`
- `[http.client.socket]`: `max_message`, `send_timeout`

A name in the call that is not in this list does not compile.

A limit has a ceiling: the highest value a request may set. It is the limit's value in
`[limits.hard]`, or else its value in `[limits]`.
`mode.default` takes `development` only when `[mode] ceiling` allows it. `debug.inline` and
`debug.store` take only `false`.

Code in a package (a library your program installs) may only lower a `[limits]` setting.
`Core\Config::restore` puts back the value from the configuration file.
