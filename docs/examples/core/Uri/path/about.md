Returns the path of an address. The path is the part after the host and before the `?` or the
`#`, such as `/docs/start` in `https://example.com/docs/start?page=2`.

`$uri->path()` returns the path as it was written. Escapes such as `%20` stay in it, and so do parts
such as `/./` and `/../`. Use `Core\Uri::decodeComponent` on one part of the path to read its text.

The result is never `null`, because every address has a path. An address with nothing after the
host, such as `https://example.com`, has the path `""`.

**The examples below** read a path, show that it is returned as it was written, and choose which
page a server shows for a request.
