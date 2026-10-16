Opens the database an operator set up, under the name they gave it.

A `[db.<name>]` block in `nvs.toml` holds the address, the user and the password. `Core\Db::connect`
opens the block with that name. Your program writes the name and nothing else, so a password never
reaches your source code, and a deployment can point the same program at a different server.

Every call with the same name gives back the same connection while the request runs. Each function
that needs the database can ask for it, instead of taking it as a parameter. The `shared` option
turns that off, and then every call opens a new connection.

A server keeps a limited number of connections for each name. When they are all busy the call waits,
and the `timeout` option says how long it waits before it throws an error.

**Good to know:** the operator decides which blocks a program may open. `[app.capabilities.db]
connect` in `nvs.toml` lists the names, and a name outside that list is refused.
