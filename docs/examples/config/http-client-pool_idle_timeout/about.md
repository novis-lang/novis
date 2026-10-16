How long an unused outbound HTTP connection stays open before Novis closes it.

After a call finishes, Novis keeps the connection open, so the next call to the same server can use
it and does not need to connect again. Servers, proxies and load balancers close connections that
are unused for some time, and they do not announce it. A call on such a connection fails and must
be sent again. This setting closes an unused connection before the other side does.

`[http.client] pool_idle` is how many unused connections are kept. `pool_idle_timeout` is how long
each one is kept. The default is 30 seconds, which is shorter than the idle timeout of common
proxies. A value with no unit is in seconds, so `30` and `"30s"` are the same.

**Good to know:** a kept connection is used by many requests, so a program can read this setting
and cannot change it.

The example prints both settings and then tries to change the timeout. `Core\Config::set` returns
`false`.
