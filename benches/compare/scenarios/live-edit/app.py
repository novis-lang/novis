# Returns the version of this file. The driver changes `1` to `2` while the server runs.


async def app(scope, receive, send):
    await send({"type": "http.response.start", "status": 200, "headers": [(b"content-type", b"text/html; charset=UTF-8")]})
    await send({"type": "http.response.body", "body": b"version 1\n"})
