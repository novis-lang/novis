`HEAD` is implemented, and it is conformance rather than convention: RFC 9110 requires a general-purpose server to support it. The request runs as `GET`, the body is discarded, and `Content-Length` is kept — a `HEAD` on a `Get`-only route returns the `GET` headers with no body and the same `Content-Length`.

**`Core\Request::method()` reports `Get`.** The application passes the method to `Core\Router::match` itself, so reporting `Head` would fail the match against a `Get` route and produce the 404 the feature exists to prevent. `Core\Request::isHead()` exposes the truth for the rare caller that wants it, and it is the only member that carries the difference.

This is one of three conventions the server applies above the compiled route table; the other two are `rule:http-server/a-preflight-is-answered-before-any-code-runs` and `rule:http-server/a-trailing-slash-is-never-normalised`. They are three different questions, which is why they are three rules.
