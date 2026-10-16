Every client — the HTTP client, the raw socket layer, a program-supplied database target, and any
socket a host import hands to an extension — is subject to the same policy, enforced at the point the
connection is made rather than inside any one of them.

A policy held by a client is a policy the next client does not have. Putting it in the capability
means an extension cannot be granted a socket that escapes it, which matters because an extension may
legitimately be an I/O source and several network clients sit outside the standard library. It also
means the answer to "what may this deployment reach" is one grant an operator reads, not a survey of
every class that opens a connection.

The exception is the same one `rule:security/net-address-policy` names — a config-named endpoint the
operator has already approved by writing it — and it is a property of the address, not of the client
that dials it.
