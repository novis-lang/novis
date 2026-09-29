Adds a signature to an address, so that nobody can change the address without the server noticing.
`sign` returns a new `Uri` with a `_sig` parameter at the end of the query. Later, `verifySignature`
checks it with the same keys.

The signature covers the scheme, the host, the port, the path and every query parameter. If somebody
adds, removes or changes a parameter, the check fails. The order of the parameters and the fragment
(the part after `#`) do not matter.

You always write two settings. `keys` is a list of 32-byte keys, and the first key signs. `until` is
the moment the link stops working. `null` means it never stops, and you must write it.

**Good to know:** the signature makes the address about twice as long. A relative address, such as
`/download?file=a`, has no host in its signature, so it is valid on any site.

**The examples below** sign a download link for one hour, sign a link with no end date, and send a
password reset link that the server checks.
