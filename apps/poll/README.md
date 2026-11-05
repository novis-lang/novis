# Poll

A live poll written in Novis. Votes are saved in SQLite, and every open poll page shows the new results at once, with server-sent events.

Run these commands in this folder:

```sh
nvs serve public/index.nvs   # open http://127.0.0.1:8145/
nvs test .                   # runs the tests
```

In a clone of the repository, `bun nv app poll` starts the app from any folder.

The folder `deploy` has the production settings and the proxy files for Caddy and nginx. On a
server, start the app with both configuration files:

```sh
nvs serve public/index.nvs --config nvs.toml --config deploy/production.toml
```

The page [Example apps: Poll](https://novis-lang.org/guides/example-apps/poll/) explains each file
and each step of the deploy.
