Reads the text inside a JWE token that `Core\Jwe::encrypt` made. You give it the token and a list
of keys. It tries each key in order and returns the text from the first key that opens the token.

A list of keys lets you change a key without breaking old tokens. Put the new key first and keep
the old key after it until the old tokens expire.

If no key opens the token, `decrypt` throws a `RuntimeError`. A changed token, a token for another
key and a text that is not a token all give the same error message. An empty list, or a list with
a password key and another key, throws a `LogicError`.

The text it returns is `tainted`. This means Novis treats it like user input. The key proves who
wrote the text, not that the text is safe.

**The examples below** read a token back, keep an old key in the list, and read a session cookie.
