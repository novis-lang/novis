`$provided == $expected`, where both operands are statically `secret`, lowers to a **constant-time**
comparison helper rather than the short-circuiting one every other operand pair uses. Nothing new is
spelled: `==` is the only equality operator (`rule:expressions/one-equality-operator`), the qualifier
is already known at the comparison, and the lowering picks the helper. Where exactly one operand is
`secret`, the qualifier has already poisoned the other
(`rule:security/secret-propagation`), so the pair is `secret` and the rule applies.

The gap it closes is narrow and real. Constant-time comparison is guaranteed inside the protocol
roster, and a dedicated equality member is available to anyone who knows to reach for it — but a
program comparing its own session token, API key or signature with `==` sits outside both, is a timing
oracle, and receives no diagnostic anywhere.

The cost, stated: roughly eight extra nanoseconds per comparison of two 32-byte values, invisible at
any scale a request reaches, and the comparison cannot early-exit, which is the entire point.
