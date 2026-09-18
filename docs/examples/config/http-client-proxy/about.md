The forward proxy every outbound HTTP call leaves through, named by the deployment and by nothing
else.

A call goes through a proxy when an operator has written this block in the configuration file, and
never otherwise. There is no call option and no program-side spelling for one, and no environment
variable is read — not `HTTP_PROXY`, not `HTTPS_PROXY`, not `NO_PROXY`, in any spelling. Leaving the
block out is how a deployment says it wants no proxy at all.

The reason is not the cost of a proxy but who gets to choose the destination. Novis pins every
outbound call to an address it checked before dialling, and a proxy is the one thing that can stand
between the program and that check. A per-call proxy would therefore be a per-call way to move the
address question somewhere else, which is exactly the widening the address rules exist to refuse.
The environment is the same argument with a worse reach: ambient, shared by every request, settable
by anything, and written down nowhere an auditor looks.

The block names the proxy, whether Novis or the proxy resolves the destination, which hosts are
reached directly, and a credential sent on the tunnel request alone.
