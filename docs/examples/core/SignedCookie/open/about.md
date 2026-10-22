`Core\SignedCookie::open` reads a cookie that `Core\SignedCookie::seal` made, and returns the value
stored in it. You give it the same list of keys that you gave `seal`. A cookie made with any key in
the list is accepted.

`open` throws a `RuntimeError` when the cookie was changed, when it is empty, when it is not a
cookie at all, or when it was made with a key that is no longer in the list. All of these give the
same message, so an attacker cannot tell which one happened.

The value `open` returns is a `tainted string` (marked as input from outside). Your program may
have stored text that a visitor sent, so the value is treated like any other input. Escape it or
check it before you use it in HTML, SQL or a file path.

**The examples below** open a cookie, treat a changed cookie as a guest, and change to a new key
while old cookies still work.
