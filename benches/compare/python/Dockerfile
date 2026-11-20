# Python's side of the comparison: the pinned interpreter with `uvicorn`, `uvloop` and `httptools`
# installed from `requirements.txt`, built once per run. The programs are not in the image: the compose
# file mounts the case's folder at `/app`, as Bun's and Node.js's do.
#
# The build context is `benches/compare/`. `--only-binary` installs the published wheels, so the build
# needs no compiler and the code that runs is the code PyPI ships.

FROM python:3.15.2-slim-trixie

COPY python/requirements.txt /srv/requirements.txt
RUN pip install --no-cache-dir --only-binary=:all: -r /srv/requirements.txt

# Every flag but the program's folder and the worker count, which the compose file and `--direct` pass.
# `uvloop` and `httptools` are the event loop and the HTTP parser uvicorn's documentation recommends.
# The ASGI programs have no startup or shutdown steps, so `--lifespan off`. `--no-access-log`, so no line is
# written per request, as for every peer. The keep-alive timeout is longer than nginx's upstream
# keep-alive, so nginx never reuses a connection uvicorn has just closed.
ENTRYPOINT ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8080", "--loop", "uvloop", "--http", "httptools", "--lifespan", "off", "--no-access-log", "--timeout-keep-alive", "65"]
EXPOSE 8080
