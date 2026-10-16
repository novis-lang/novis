The key the server checks every CSRF token against. Naming it is what turns the token half of the
check on.

Novis refuses a state-changing request whose `Origin` names somewhere other than where it arrived,
and it does that with nothing configured at all. What it cannot refuse until this key is named is a
covered request that brought no `Origin` — which is every non-browser client, and so every API
caller: with no key there is no token this deployment ever issued for such a request to be missing.
Name the key and those requests are covered too.

The key is the operator's alone. Every request is checked against this one value, so a program that
could write it would be choosing which forgeries the requests beside it accept. A rotated key
governs the next request rather than the next restart, and a key the door cannot read is refused at
startup rather than left quietly unarmed. `csrf_key_file` is the same value read out of a file, which
is the spelling an audited deployment uses.
