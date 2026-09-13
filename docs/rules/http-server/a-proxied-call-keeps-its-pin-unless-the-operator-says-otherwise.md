Every `[http.client.proxy]` block writes `resolve`, and there is no default: a block without it refuses
the boot naming both words, and so does a third word.

**`resolve = "local"` keeps everything the pin buys.** Novis resolves the destination and checks every
address against `rule:security/net-address-policy` exactly as it does for a direct call, and then asks the
proxy to `CONNECT` to **an address it approved**, with `Host` naming the same. Over the tunnel it speaks
what it speaks today: TLS with the server name `rule:http-server/allow-url-pins-the-address` approved, or
plain HTTP for an `http` URL. There is no second resolution anywhere in the path, so a proxy does not
reopen the check-then-connect gap; falling back across the approved set works unchanged, one `CONNECT` per
address.

**`resolve = "proxy"` is for the network where only the proxy can resolve a name** — an application subnet
with no DNS route outward. `CONNECT` carries the host name, Novis judges what the URL's text can be judged
on — the scheme, the `net.connect` grant's host list, the tainted-URL check — and cannot check the address,
because it never learns one. The address question moves to the proxy.

Because that is a real weakening, it is never silent:

- **Every boot writes one `Warn` record** naming the block, the word and `rule:security/net-address-policy`
  — on every start, so a deployment that has run this way for a year still says so in today's log.
- **`nvs config dump` renders the key beside that rule's id**, so the offline audit
  (`rule:config/check-and-dump-audit-the-tree-offline`) shows the narrowing with no server running.

The word is mandatory for `rule:config/scope-has-no-default`'s reason. Both answers are commonly correct
and either default silently does the wrong thing in somebody's production: `local` fails outright where
only the proxy resolves, which makes the ordinary corporate deployment look broken, and `proxy` gives up
the pin in every deployment whose proxy could have dialled an address, with nothing at run time to
distinguish that from a deployment that meant it.

A program cannot tell which word is written. There is no member, option or constant reporting it, for the
same reason there is no per-call spelling: whether this deployment's egress is proxied is not a thing
program code decides or branches on.
