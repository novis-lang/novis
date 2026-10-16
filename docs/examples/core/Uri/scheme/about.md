Returns the scheme of an address, which is the part before the first `:`. For
`https://example.com/`, `scheme()` returns `https`. For `mailto:ann@example.com`, it returns
`mailto`.

The scheme is returned as it was written, so `HTTPS` stays upper case. Schemes do not depend on
case, so make the result lower case with `Core\Str::lower` before you compare it.

The result is `null` when the address has no scheme. A path such as `/about` or `photo.jpg` has no
scheme. An address that starts with `//`, such as `//cdn.example.com/logo.png`, has no scheme too.

**The examples below** read the scheme of some addresses, tell a full address from a path, and
accept a website link only when it uses `http:` or `https:`.
