`nvs meta --json` prints a description of the whole `Core` library as one JSON object.

The object has these keys: `classes`, `enums`, `exceptions`, `interfaces`, `attributes`,
`directives` and `capabilities`. A class has its methods, and each method has its name, signature,
parameters, return type and documentation. `directives` lists every key of `nvs.toml`.
`capabilities` lists each method that needs a permission, and the permission it needs.

With a file, as in `nvs meta --json cart.nvs`, the command also reads that program. It adds the key
`program` with the classes, enums and type aliases of the program. The documentation of a
declaration is the `///` comment above it.

**Good to know:** `--json` is required. A method that is not in `capabilities` needs no permission.
