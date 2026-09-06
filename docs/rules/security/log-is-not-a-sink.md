A log field is data: the writer is a structured serializer, so a field is framed by the serializer and
never by string concatenation into a line. Log forging is therefore closed by the writer's shape
(`rule:errors/log-write`), independently of the qualifier system, and logging tainted content is
*desired* rather than tolerated — recording exactly what an attacker sent is the point of a security
log.

The same holds for a bidirectional control, which the writer escapes by construction
(`rule:security/bidi-boundaries`), and for a JSON decode, whose input is data the parse returns to the
program and whose result is `tainted`.

The `secret` axis does not follow: a log field **refuses** a `secret` value, because that axis is
about confidentiality rather than structure and a log is an output
(`rule:security/secret-sinks-refuse`). Stating both here is what stops a future reader from applying
one rule's answer to the other axis.
