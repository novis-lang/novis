`Core\SignedCookie::open` reads a cookie that `Core\SignedCookie::seal` made, and returns the value
stored in it. You give it the same list of keys that you gave `seal`. A cookie made with any key in
the list is accepted.

`open` throws a `RuntimeError` when the cookie was changed, when it is empty, when it is not a
cookie at all, or when it was made with a key that is no longer in the list. All of these give the
same message, so an attacker cannot tell which one happened.

A cookie comes from a request, so its text is usually `tainted` (marked as input from outside).
`open` returns the value without that mark, because your own program stored it.

**The examples below** open a cookie, treat a changed cookie as a guest, and change to a new key
while old cookies still work.
