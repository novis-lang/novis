`[http]` configures every response the server sends. It sets the security headers, the answer to
a cross-origin call, the defaults of every cookie, and how much an error page shows.

With nothing written, every value is the safest choice. The security headers are on. Cross-origin
calls get no answer until you name the allowed origins. Cookies are `Secure`, `HttpOnly` and
`SameSite=Lax`. An error page says only that something went wrong.

A program may change most of these values with `Core\Config::set`. The change applies only to the
response of the request it is handling. The next request uses the values from the file again.

`Core\Config::set` returns `false` for the settings a program cannot change:

- `http.csrf_key` and `http.csrf_key_file`, the key that signs cross-site request tokens.
- `http.errors.detail`, because the error page is sent before the program starts.
- `http.cors.methods`, `http.cors.headers` and `http.cors.max_age`. Only the answer to a CORS
  preflight request uses them, and the server sends that answer before any program runs.
