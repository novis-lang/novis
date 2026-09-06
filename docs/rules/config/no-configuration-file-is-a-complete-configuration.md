When no `--config` is given and no `./nvs.toml` exists, the host runs on the **shipped defaults**, and
those are a complete and valid configuration rather than a failure to find one: capabilities
deny-all, `[mode] default = "production"`, and every limit at its documented default. The boot log says
so in one line.

Every reader has to answer on such a host. `Core\Config::get` is `null`, `all` is empty, `set` is
`false` and `restore` does nothing — none of them throws, because "no configuration file anywhere" is
a valid host and a program has to be able to ask on one. That is every test context and every
embedder that has not built a snapshot.

The one distinction the shipped defaults keep is between *no tree* and *a tree that grants nothing*:
a program checked outside any project root has no configuration to be measured against, while a tree
that was read and says nothing about capabilities is an operator's written `no`. The offline audit
reports the first as a tree of zero files.
