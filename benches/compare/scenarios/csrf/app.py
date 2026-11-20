# A `POST` to `/change` changes the account and returns `changed`.


async def app(scope, receive, send):
    if scope["method"] == "POST" and scope["path"] == "/change":
        # This is the type Bun gives a text answer with no `Content-Type`.
        await send({"type": "http.response.start", "status": 200, "headers": [(b"content-type", b"text/plain;charset=utf-8")]})
        await send({"type": "http.response.body", "body": b"changed\n"})
        return
    await send({"type": "http.response.start", "status": 404, "headers": []})
    await send({"type": "http.response.body", "body": b""})
