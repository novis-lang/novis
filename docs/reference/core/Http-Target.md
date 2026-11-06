---
summary: A URL that `Core\Http::allowUrl` checked, with the addresses it may connect to. Pass it to a `Core\Http\Client` method in place of a URL that came from a user
keywords: check a URL, user URL, webhook URL, outbound request, SSRF, private address, internal network, net.connect, net.internal, allowed hosts
---

`Core\Http\Target` is a URL that `Core\Http::allowUrl` checked. You pass it to a
`Core\Http\Client` method, such as `get`, `post` or `stream`, in place of the URL. You can use
the same `Core\Http\Target` for more than one request.

A URL that a user typed is `tainted`. A `Core\Http\Client` method does not accept a `tainted` URL,
and the program does not compile. Without this check, a user could make your server send requests
to machines inside your own network. So a program calls `Core\Http::allowUrl` first, and passes
the result to the client.

`Core\Http::allowUrl` checks three things. The scheme must be `http`, `https`, `ws` or `wss`, and
the URL must have a host. If `net.connect` in `nvs.toml` lists hosts, the host must be one of
them. And the host's addresses must not be private, link-local or unspecified, unless
`net.internal` lists them. A loopback address such as `127.0.0.1` is allowed on the command line
and in development mode.

A URL that fails the check throws a `RuntimeError`, and the message gives the reason. No request is
sent. The `Core\Http\Target` has the addresses that were checked, at most eight. The request
connects only to these addresses. The host name is not looked up a second time.

```nvs
<?nvs
// `Core\Test::answerHttp` gives a fake reply, so this example does not use the network.
Core\Test::answerHttp('https://203.0.113.10/*', 200, { json: ['received' => true] });

// Text from a user is `tainted`. `Core\Http::allowUrl` checks it and returns a `Core\Http\Target`.
tainted string $typed = 'https://203.0.113.10/hooks/order';
Core\Http\Target $hook = Core\Http::allowUrl($typed);
Core\Http\Response $reply = Core\Http\Client::post($hook, { json: ['order' => 7] });
echo $reply->status(), ' ', $reply->text(), "\n";

// 10.0.0.7 is a private address, so `allowUrl` throws a `RuntimeError`.
try {
    Core\Http\Target $inside = Core\Http::allowUrl('http://10.0.0.7/admin');
} catch (RuntimeError $error) {
    echo $error->message, "\n";
}
```
```output
200 {"received":true}
Core\Http::allowUrl cannot connect to 10.0.0.7: it is private (10/8, 172.16/12, 192.168/16), which `net.internal` does not except
help: add `internal = ["10.0.0.7"]` under `[capabilities.net]` in nvs.toml
```
