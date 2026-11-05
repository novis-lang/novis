# Chat

A chat app written in Novis. Visitors talk in rooms over a WebSocket, and each room keeps its last
lines in SQLite. The same server sends the pages and runs every WebSocket connection.

Run these commands in this folder:

```sh
nvs serve public/index.nvs   # open http://127.0.0.1:8144/
nvs test .                   # runs the tests
```

In a clone of the repository, `bun nv app chat` starts the app from any folder.

On a production server, start it with the files in `deploy`:

```sh
nvs serve public/index.nvs --config deploy/production.toml
```

`deploy/Caddyfile` and `deploy/nginx.conf` are the configuration of the reverse proxy.

The page [Example apps: Chat](https://novis-lang.org/guides/example-apps/chat/) explains each file.
