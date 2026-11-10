**In development, a request from a loopback address must name a host this server knows in `Host`:** `localhost`, a name under `.localhost`, a loopback address, a host a `[[server.mount]]` lists, or an entry of `[server] development_hosts`. Any other host reaches no mount and is answered `421 Misdirected Request`, bodiless, before step 1 of [`http-server/a-request-resolves-in-five-steps`](http-server.md#http-server-a-request-resolves-in-five-steps). Only the health probe still answers at its path. That closes DNS rebinding, where a web page makes its own name resolve to `127.0.0.1` and the browser sends that name to a server on this machine.

**Who is asked.** The peer is the client address after the forwarded walk, so a trusted proxy that forwards a remote client's address is not asked, and a local proxy serving a browser on this machine is. A request from another machine is not asked, so a development server on `0.0.0.0` still answers its network address. A request over a Unix socket is not asked. Production never asks: the mode is `[mode] default` as the tree states it, so a reload that flips the mode turns the guard on or off for the next request.

**`[server] development_hosts`** lists the extra hosts, for a local proxy that sends its own name. Each entry is a host name or an IP address, an IPv6 one in brackets, compared exactly with the folded `Host`: ASCII lower case, no trailing `.`, no port. An entry with a scheme, a port, a wildcard, a path or a space is `E0663` at boot and at reload, because an entry that can never equal a `Host` is a proxy an operator believes is let in. The key is checked in production too, so a tree a reload flips to development cannot carry one. It is `System` and reloads with the `[server]` block.

**The refusal is logged.** Its access line is a `Warn`, written whatever `[log] access` says, with a `host` field and a message that names the key. `421` and not the table's `404`, because a `404` looks exactly like a mount that does not exist and the server looks broken behind a proxy that works.

- **Memory:** the list, once per configuration tree, and one copy of the refused host per refused request, freed with its response. Nothing per accepted request.

**Declined:**

- **A wildcard or suffix entry.** One entry would let in every name under a domain the operator does not control the DNS of, which is the rebinding the guard exists to close. A proxy sends a fixed list of names.
- **Turning the guard off with a switch.** A list states what is let in; a switch lets in every page on the web.
- **Trusting any host a trusted proxy sends.** A local proxy with a catch-all server block forwards a rebinding page's host as readily as its own.
- **The table's `404`.** It is indistinguishable from a missing mount, so a correct proxy setup reads as a broken server.
- **A body explaining the refusal.** A rebinding page can read it; the log line is for the person who runs the server, who is the one who can fix it.
