Creates a CSRF token for one session. You put the token in a form. The next request sends it back,
and your program can then tell that the request came from your own page.

`Core\Csrf::issue` takes two things. The first is a value that stays the same for as long as the
token should be accepted, such as a session identifier. The second is a 32 byte key, as
`Core\Crypto::generateKey` creates one. The result is a short text of letters, digits, `-` and `_`.
A hidden field, a header and a URL all carry it unchanged.

Nothing is stored anywhere. The session is sealed inside the token, so a form opened in a second
browser tab does not break the form in the first one. Every call creates a different token, and all
of them work.

**Good to know:** `Core\Csrf::verify` is the only way to check a token. No method gives you the
token that was expected, so there is nothing to compare with `==`.
