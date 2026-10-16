Returns the fragment of an address. The fragment is the text after the `#`, such as `install` in
`https://example.com/guide#install`. A browser uses it to scroll to one part of a page, and does
not send it to the server.

`$uri->fragment()` returns the text without the `#`, as it was written. Escapes such as `%20` stay
in it. Use `Core\Uri::decodeComponent` to read its text.

The result is `null` when the address has no `#`. The result is `""` when nothing is written after
the `#`, as in `https://example.com/guide#`.

**The examples below** read a fragment, show the `null` and `""` results, and remove the fragment
before a program saves a link.
