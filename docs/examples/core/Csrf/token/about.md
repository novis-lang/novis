Returns a CSRF token for the visitor's session. CSRF (cross-site request forgery) is an attack where
another website sends a form in the visitor's name. You put the token in your form, and the server
checks it when the form comes back.

Set `[http] csrf_key` in `nvs.toml`. Without it, `token()` throws a `RuntimeError`. Outside a web
request, `token()` throws a `LogicError`.

For a new visitor, the response sends a session cookie with the token. The server saves no session
data until your program writes a value with `Core\Session`. So a login form for a new visitor also
has a valid token.

Send the token back in a form field named `_csrf` or in the `x-csrf-token` header. The server checks
it before your route method runs. A request without a valid token gets the status `403`. The token is
valid only with the session cookie that was sent with it.

Every call returns a different token. All of them stay valid, so a form open in a second tab still
works.

**The examples below** show a form for a new visitor, the form sent back with and without its token,
and the error in a command-line program.
