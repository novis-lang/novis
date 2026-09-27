Adds a cookie to the response. The browser stores it and sends it back with its next requests,
where `Core\Request::cookie` reads it. Replaces PHP's `setcookie`.

A cookie without options is safe by default. It is sent over HTTPS only, JavaScript cannot read it,
and other websites cannot send it with their forms. These defaults come from `[http.cookies]` in
`nvs.toml`, and each call can change them. `maxAge` sets how long the browser keeps the cookie, and
`maxAge: 0s` deletes it.

A value that contains a space, a comma, a semicolon, a backslash or a quote throws a `LogicError`.
So does a name with the `__Host-` or `__Secure-` prefix whose options break the prefix's rules.
Each call adds one cookie, and nothing is added when the call throws.

The examples show a cookie with the default options, a value that is not allowed, and the cookies a
login page sends.
