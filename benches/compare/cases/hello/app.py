# Returns one line for every request. This measures the server alone.

HEADERS = [(b"content-type", b"text/html; charset=UTF-8")]


async def app(scope, receive, send):
    await send({"type": "http.response.start", "status": 200, "headers": HEADERS})
    await send({"type": "http.response.body", "body": b"hello, world\n"})
