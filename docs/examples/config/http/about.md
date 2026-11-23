`[http]` configures every response the server sends. It sets the security headers, the response to
a cross-origin call, the defaults of every cookie, and how much an error page shows. A cross-origin
call is a call from a web page on another site. CORS (Cross-Origin Resource Sharing) is the set of
headers that allows such a call.

With nothing written, every value is the safest choice. The security headers are on. Cross-origin
calls are not allowed until you name the allowed origins. Cookies are `Secure`, `HttpOnly` and
`SameSite=Lax`. An error page says only that something went wrong.

A program may change most of these values with `Core\Config::set`. The change applies only to the
response of the request it is handling. The next request uses the values from the file again.

The page of `Core\Config::set` lists the settings a program may change. `Core\Config::set`
returns `false` for every other setting, such as these:

- `http.csrf_key` and `http.csrf_key_file`, the key that signs cross-site request tokens.
- `http.errors.detail`, because the error page is sent before the program starts.
- `http.cors.methods`, `http.cors.headers` and `http.cors.max_age`. Only the response to a CORS
  preflight request uses them. The server sends that response before any program runs.
