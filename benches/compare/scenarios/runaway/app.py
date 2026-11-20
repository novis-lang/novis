# Loops forever for `/loop`, and adds 1 MiB to a list forever for `/alloc`.
# Every other path returns `ok`.


async def app(scope, receive, send):
    if scope["path"] == "/loop":
        turns = 0
        while True:
            turns += 1
    if scope["path"] == "/alloc":
        chunks = []
        while True:
            chunks.append(b"\x01" * 1048576)
    await send({"type": "http.response.start", "status": 200, "headers": [(b"content-type", b"text/html; charset=UTF-8")]})
    await send({"type": "http.response.body", "body": b"ok\n"})
