# Throws an error for the path `/crash`, and returns `ok` for every other path.


async def app(scope, receive, send):
    if scope["path"] == "/crash":
        raise RuntimeError("the handler failed")
    await send({"type": "http.response.start", "status": 200, "headers": [(b"content-type", b"text/html; charset=UTF-8")]})
    await send({"type": "http.response.body", "body": b"ok\n"})
