# Chat

A chat app written in Novis. Visitors talk in rooms over a WebSocket, and each room keeps its last
lines in SQLite. The same server sends the pages and runs every WebSocket connection.

Run these commands in this folder:

```sh
nvs serve public/index.nvs   # open http://127.0.0.1:8144/
nvs test .                   # runs the tests
```

In a clone of the repository, `bun nv app chat` starts the app from any folder.

The folder `deploy` has the production settings and the proxy files for Caddy and nginx. On a
server, start the app with both configuration files:

```sh
nvs serve public/index.nvs --config nvs.toml --config deploy/production.toml
```

The page [Example apps: Chat](https://novis-lang.org/guides/example-apps/chat/) explains each file
and each step of the deploy.
