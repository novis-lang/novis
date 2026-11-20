# `/hold` keeps 8 MiB for 300 ms and then returns `held`.
# Every other path returns `ok`.
import asyncio

HEADERS = [(b"content-type", b"text/html; charset=UTF-8")]


async def app(scope, receive, send):
    if scope["path"] == "/hold":
        held = bytearray(b"\x01") * 8388608
        await asyncio.sleep(0.3)
        await send({"type": "http.response.start", "status": 200, "headers": HEADERS})
        await send({"type": "http.response.body", "body": b"held\n" if held[-1] == 1 else b""})
        return
    await send({"type": "http.response.start", "status": 200, "headers": HEADERS})
    await send({"type": "http.response.body", "body": b"ok\n"})
