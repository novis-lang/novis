# Returns the version of each part. The driver writes the parts and later replaces them.
# `DEPLOY_PARTS` is the number of parts, 200 unless the row sets it.
import importlib.util
import os
from pathlib import Path


def load(path):
    spec = importlib.util.spec_from_file_location(path.stem, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


count = int(os.environ.get("DEPLOY_PARTS") or 200)
digits = len(str(count - 1))
parts = [load(Path(__file__).parent / "parts" / f"p{i:0{digits}d}.py") for i in range(count)]


async def app(scope, receive, send):
    body = "".join(p.version for p in parts) + "\n"
    await send({"type": "http.response.start", "status": 200, "headers": [(b"content-type", b"text/html; charset=UTF-8")]})
    await send({"type": "http.response.body", "body": body.encode()})
