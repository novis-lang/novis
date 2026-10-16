Returns the address as text. For a `Uri` that `Core\Uri::parse` read, the result is exactly the text
that was read, byte for byte. Nothing is changed: `HTTP://Example.COM/a%2fb` comes back as
`HTTP://Example.COM/a%2fb`, with the same upper-case letters and the same escapes.

For a `Uri` that `with`, `withQueryParameter` or `resolve` made, the result is the new address.

`echo $uri` prints the same text as `echo $uri->toString()`. Use `toString()` when you need a
`string`, for example to store the address or to return it from a function.

Two texts can be the same address, such as `HTTPS://Example.com/` and `https://example.com/`.
`toString` keeps them different. `compareTo` checks if two addresses are the same.

**The examples below** print an address unchanged, compare two ways to write one address, and build
the `Link` header of an API.
