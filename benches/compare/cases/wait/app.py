# Waits 50 ms on a timer, then answers. This measures the cost of many
# requests in flight at once, as when a page waits for another service.
import asyncio

HEADERS = [(b"content-type", b"text/plain; charset=UTF-8")]


async def app(scope, receive, send):
    if scope["path"] != "/wait":
        await send({"type": "http.response.start", "status": 404, "headers": HEADERS})
        await send({"type": "http.response.body", "body": b"not found\n"})
        return
    await asyncio.sleep(0.05)
    await send({"type": "http.response.start", "status": 200, "headers": HEADERS})
    await send({"type": "http.response.body", "body": b"waited 50 ms\n"})
