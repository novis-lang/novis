A derived codec introduces no qualifier rule of its own. A decoder assigns into declared property
types, so a payload that carries `tainted` requires the fields receiving it to declare it, and the
check happens where the qualifier is statically known — the **call site** that decodes, not inside the
codec. Only `string` and `bytes` carry a qualifier, so an integer, a decimal, an enum or an instant
field needs nothing.

A `secret` property on a class carrying a derive attribute is a **compile error at the declaration**,
with skipping the field as the stated fix. This adds no sink — encoding was already one
(`rule:security/secret-sinks-refuse`) — it moves the report from wherever the value happened to reach
the encoder to the declaration that put it on the wire contract, and it replaces a silent omission
with a written one.
