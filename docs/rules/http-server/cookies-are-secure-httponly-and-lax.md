`[http.cookies]` ships as `secure = true`, `http_only = true`, `same_site = "Lax"`, `path = "/"`,
and every `Core\Response::addCookie` call inherits each field it did not write. So a cookie
written with no options bag is `Secure; HttpOnly; SameSite=Lax; Path=/`, and a cookie that
genuinely needs to be readable by script says so at the call site, in one field, visibly —
omitting `HttpOnly` and nothing else.

The options bag carries the same four settings as booleans and a `SameSite` enum in
`rule:enums/closed-integer-type`'s shape, never the string the directive uses
(`rule:core-api/shape-rules` R11). A fourth `same_site` spelling in the file is refused rather
than read as one of the three, and `"None"` without `secure` is refused outright
(`rule:http-server/a-wildcard-origin-with-credentials-is-refused`).

A cookie's *name* is not this rule's business: `rule:errors/cookie-name-bytes` owns it — a name
matches byte for byte with no substitution, and `__Host-` and `__Secure-` semantics are enforced
on read and on write. The two rules meet at the same header and are otherwise independent.
