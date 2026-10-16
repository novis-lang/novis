Checks a URL before your program sends a request to it.

A URL that comes from a user is `tainted`, and the request methods of `Core\Http` do not accept it.
`Core\Http::allowUrl` checks the URL and returns a `Core\Http\Target`. You give this value to a
request method in place of the URL.

The check has three parts. The scheme must be `http`, `https`, `ws` or `wss`. The host must be
allowed by `net.connect` in `[capabilities.net]` in `nvs.toml`. And the host's addresses must not be
inside your own network: loopback, private and link-local addresses throw a `RuntimeError`, unless
`net.internal` names them.

The `Core\Http\Target` keeps the addresses that were checked, and the request connects to one of
them. The host name is not looked up a second time.

**Good to know:** a refused URL throws a `RuntimeError`. The examples show one allowed URL, four
refused URLs, and webhook URLs that customers save.
