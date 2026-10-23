Returns a CSRF token for the visitor's session. You put it in a form, and the server checks it when
the form comes back.

Call `Core\Session::start()` first, and set `[http] csrf_key` in `nvs.toml`. Without either one,
`token()` throws a `RuntimeError`. If the visitor has no session yet, the call creates one and the
response sends its cookie. A login form for a new visitor works the same way.

Send the token back in a form field named `_csrf`, in the `x-csrf-token` header or in a `_csrf`
query parameter. The server checks it before your route method runs. A request without a valid
token gets the status `403`, and your gate's `refused` method receives `Core\Router\Refusal::Csrf`.
A token works only together with the session cookie it was created for.

Every call returns a different token, and each one is valid for the whole session. A form open in a
second browser tab still works.

**The examples below** show a form for a new visitor, the form sent back with and without its token,
and the error before `start()`.
