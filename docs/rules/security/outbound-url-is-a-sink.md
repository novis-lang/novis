The URL parameter of an HTTP client's request members, and the address parameter of a raw connect, are
unqualified: a `tainted` value at either position is a compile-time diagnostic, exactly as at a query
text parameter. Server-side request forgery starts with an attacker-chosen address, and this is the
position where that is visible.

The plain form covers URLs the program itself authored — a literal, a configuration value, a composed
path — and a literal is additionally validated while checking
(`rule:expressions/intrinsic-constant-arguments`). A URL that genuinely came from outside goes through the
laundering member that also **pins** the address it resolved to, so the check and the connection
cannot disagree about which host was approved.

Refusing at the call site is only half of it: the address is judged again at run time against the
capability's own policy (`rule:security/net-address-policy`), because a hardcoded hostname can still
resolve into a private range.
