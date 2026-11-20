# Runs `?parts=` parts of the same loop and returns the sum of their results.
# With four parts, each part runs in its own process from a `ProcessPoolExecutor`.
# Threads would not help: the standard Python build runs one thread at a time.
import asyncio
from concurrent.futures import ProcessPoolExecutor
from urllib.parse import parse_qs

from work import part


async def app(scope, receive, send):
    query = parse_qs(scope["query_string"].decode("ascii"))
    parts = int(query.get("parts", ["1"])[0])
    turns = int(query.get("turns", ["0"])[0])
    if parts == 4:
        loop = asyncio.get_running_loop()
        with ProcessPoolExecutor(max_workers=4) as pool:
            results = await asyncio.gather(*(loop.run_in_executor(pool, part, seed, turns) for seed in (1, 2, 3, 4)))
        total = sum(results)
    else:
        total = part(1, turns)
    await send({"type": "http.response.start", "status": 200, "headers": [(b"content-type", b"text/html; charset=UTF-8")]})
    await send({"type": "http.response.body", "body": str(total).encode()})
