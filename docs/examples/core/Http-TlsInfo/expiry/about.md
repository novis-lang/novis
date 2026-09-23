Returns the time when the certificate of the server a connection went to stops being valid, as a
`Core\Time\Instant`.

Every certificate has an end date. After that time, a client does not accept it, and calls to that
server fail until somebody installs a new certificate. `expiry()` returns the end date as an
instant, in whole seconds. You can compare it with another instant, or subtract one from it with
`since()` to get the time that is left.

You get a `Core\Http\TlsInfo` from `Core\Http\Response::tls()`. That method returns `null` for a
reply that did not come over TLS, so check for `null` first.

**Good to know:** the examples build their TLS connections with `Core\Test::tlsSession`, so they
run without a network. They show the end date, the number of days that are left, and a daily check
that warns about certificates that expire soon.
