Reads the text of an address and returns a `Uri`, the same way `Core\Uri::parse` does. The two
methods are different only when the text is not an address. `parse` throws a `RuntimeError`, and
`tryParse` returns `null`.

The result is `null` for exactly the texts that `parse` does not accept. A space, a control
character, a `%` without two hex digits and a port above `65535` are some of them.

A relative link, such as `/docs/start`, is an address, so the result is a `Uri`. To check if an
address is absolute, test that its scheme is not `null`: `tryParse($text)?->scheme() != null`.

**The examples below** check the text a user typed, test if a link is absolute, and check the
address a login page sends the user to.
