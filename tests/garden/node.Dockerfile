# The Node reference origin: the official Node image running the Garden's own
# `images/node_stdlib/app.js`. `bun nv garden` stages this file as `Dockerfile` beside that
# `app.js`. The Garden's image compiles Node from source, which takes an hour and more memory
# than the WSL VM has to spare while other work runs. A release of Node has the same HTTP
# parser, llhttp, so it reads requests the same way.
FROM node:24.22.0-trixie-slim@sha256:7f486cabe892bff8fef04c0f7da8dba84a560be3e4e2740b7a86afc57ebec315

RUN apt-get update \
 && apt-get install -y --no-install-recommends openssl \
 && rm -rf /var/lib/apt/lists/*

# `app.js` serves TLS with the certificate the Garden's base image makes, so this makes the
# same one at the same paths.
WORKDIR /app
RUN openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout garden.crt.key -out garden.crt -subj "/CN=garden"

COPY app.js /app/app.js
CMD ["node", "app.js"]
