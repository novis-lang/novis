# The Garden's harness. `bun nv garden` stages this file as `Dockerfile` beside the Garden's
# `tools/`, `docker-compose.yml` and `quirks.yml`, with `sweep.py` in `tools/` and this
# directory's `external-services.yml` in place of the Garden's. The files are copied in, never
# mounted, so the run needs no bind mount from the host.
FROM python:3.13-slim

RUN pip install --no-cache-dir "docker>=7.1.0" "pyyaml>=6.0.3" "tqdm>=4.67.3"

COPY . /garden
WORKDIR /garden
ENTRYPOINT ["python", "tools/sweep.py"]
