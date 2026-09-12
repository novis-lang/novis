```
Core\Http::allowUrl(tainted string $url): Core\Http\Target
```

The launderer parses the URL, refuses a scheme outside the grant, resolves the host, checks every
resolved address against `rule:security/net-address-policy` — one denied refuses the host — and
answers a `Target` carrying **the URL and every address that was approved**
(`rule:http-server/an-outbound-call-tries-every-approved-address`). It throws, naming which check
failed, rather than returning a falsy value. The text is judged before the deployment is asked,
so a refusal on the URL itself never depends on a grant.

The `Target` return is the load-bearing part. A launderer answering a plain `string` would leave a
gap between the check and the connection in which a second DNS resolution could return a
different address — the classic rebinding attack. Because every `Core\Http\Client` member
connects to an address inside the `Target`, there is no second resolution to poison. That is why
this is the one launderer in `rule:security/launderers-are-sink-named`'s roster whose output is a
value, and why `Target` has no members: a program that could read the approved addresses back out
could rebuild a request around an address that was never approved.

A plain `string` URL — the form kept for a URL the program itself authored — passes the same four
questions at the member that connects, so there is one implementation of the policy and not two,
and it lives in the capability rather than the client
(`rule:security/the-policy-lives-in-the-capability`).
