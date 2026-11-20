# Returns the search term `?q=` in a heading, written with an f-string on `/` and joined
# with `+` on `/direct`.
from urllib.parse import parse_qs


async def app(scope, receive, send):
    term = parse_qs(scope["query_string"].decode("ascii")).get("q", [""])[0]
    body = "<h1>Results for " + term + "</h1>\n" if scope["path"] == "/direct" else f"<h1>Results for {term}</h1>\n"
    await send({"type": "http.response.start", "status": 200, "headers": [(b"content-type", b"text/html; charset=UTF-8")]})
    await send({"type": "http.response.body", "body": body.encode()})
