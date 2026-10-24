The place where a client sends the key of an `ApiKey` security scheme. You write it as `in:` in an
`#[Core\Api\Scheme]`.

There are three cases:

- `Query` is a query parameter in the URL.
- `Header` is a request header.
- `Cookie` is a cookie.

`parameter:` is the name of that query parameter, header or cookie. `name:` is the name of the
scheme itself. A route uses that name in `#[Core\Api(security: [...])]`.

`nvs build --openapi` writes the case as the scheme's `in`, for example `header`.

**Good to know:** a key in the query is part of the URL. Servers and proxies often write URLs to
their logs, so a header is usually the safer place for a key.
