A call may name the address it connects to with `connectTo: string`, and only where a `net.connect_to`
grant lists the URL's host. The value is an IP literal; the call connects there instead of resolving the
host, and the certificate is still checked against the host the URL named.

The option widens nothing, and that is what makes it grantable. The address is judged by
`rule:security/net-address-policy` and by `net.internal`'s exceptions exactly as a resolved one is, so
`connectTo` chooses *among addresses the deployment already allows* — it is `curl --resolve` with the
address policy still underneath, which is how a program reaches one node of a cluster, or a canary behind
a shared name, without the deployment having to loosen anything else.

A `Core\Http\Target` already carries the addresses its laundering approved
(`rule:http-server/an-outbound-call-tries-every-approved-address`), so `connectTo` beside one is a
`LogicError` rather than a silent override of the pin: the two are answers to the same question, and a
call that supplies both has not decided which one it meant.
