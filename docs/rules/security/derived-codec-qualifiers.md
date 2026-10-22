A derived codec introduces no qualifier rule of its own. A decoder assigns into declared property
types, so a payload that carries `tainted` requires the fields receiving it to declare it, and the
check happens where the qualifier is statically known — the **call site** that decodes, not inside the
codec. Only `string` and `bytes` carry a qualifier, so an integer, a decimal, an enum or an instant
field needs nothing. Declaring `tainted` on a decoded field is friction at the moment somebody is
trying to be quick, and it is the same declaration hand-written hydration already needed; the derive
is only where a newcomer meets it first.

A member that reads its own payload decides by its owner: every `Core\Request` decoder,
`Core\Http\Response::jsonAs` and `Core\Jwt::verifyIssued` carry `tainted` always. `Core\Arr::shapeAs`
converts the array it is handed, so its argument decides: by the test an encoder applies
(`rule:security/taint-propagation`), an array whose type carries `tainted` or `mixed` anywhere, or a
literal holding such a value, requires every text field it fills to be `tainted` (`E0810`). An
`array<string>` or a literal of the program's own values fills a plain `string`. A literal element that
is a call is read as outside text, because its type is not kept there; binding it to a typed variable
first is the fix.

A `secret` property on a class carrying a derive attribute is a **compile error at the declaration**,
with skipping the field as the stated fix. This adds no sink — encoding was already one
(`rule:security/secret-sinks-refuse`) — it moves the report from wherever the value happened to reach
the encoder to the declaration that put it on the wire contract, and it replaces a silent omission
with a written one.
