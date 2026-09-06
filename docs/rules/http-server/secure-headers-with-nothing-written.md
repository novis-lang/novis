Every response the server writes carries `X-Content-Type-Options: nosniff`, a
`Content-Security-Policy` of `frame-ancestors 'none'`, and `Referrer-Policy:
strict-origin-when-cross-origin`, with no `[http.headers]` block present. The block's fields are
the shipped values, and a written block is read field by field: `false` or the empty string drops
a header, and a value the wire cannot carry falls back to the shipped default rather than to
nothing.

There is no default `Content-Security-Policy` beyond `frame-ancestors`. A `default-src` policy
that is wrong breaks the page silently and is the most abandoned security header there is, while
a policy that is right is application-specific; `frame-ancestors 'none'` is the one directive
safe for every application, because it governs who may frame the response rather than what the
response may load. The XSS half of what a CSP buys is closed structurally by
`rule:core-classes/html-auto-escape`, which does not depend on being configured.

A header the response already wrote is left alone
(`rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`), and HSTS is the one
member of the set with a condition of its own (`rule:http-server/hsts-follows-the-effective-scheme`).
