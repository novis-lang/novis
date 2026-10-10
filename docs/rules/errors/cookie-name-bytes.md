`Core\Request::cookie(name)` matches **byte for byte**. No character is substituted, in either
direction.

The runtime enforces the prefixes rather than documenting them:

- `__Host-` requires `Secure`, requires `Path=/`, and forbids `Domain`.
- `__Secure-` requires `Secure`.

A non-conforming cookie carrying either prefix is **not visible** to the program on read, and is
refused on write.

A prefix whose meaning depends on every read site checking it is the arrangement that has produced
consecutive CVEs — the second being the incomplete fix for the first, where cookie-name
mangling let a plain cookie be read as a `__Host-` one. Enforcing it once, in the one place that
parses the header, is the whole fix.

PHP's dot-and-space mangling of cookie names served variable names built from request input, and Novis
has no superglobals for it to serve.

**Declined:**

- **Leaving prefix enforcement to the program**, with a cookie helper. The prefix then means nothing
  unless every read site checks it, which is the arrangement that produced the bypass above.
- **A compatibility mode that reproduces cookie-name mangling.** The mangling is the bypass, and PHP
  syntax is an on-ramp rather than a compatibility promise.
