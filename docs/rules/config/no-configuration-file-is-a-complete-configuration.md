When no `--config` is given and neither `./nvs.toml` nor the data folder's `nvs.toml` exists, the host
runs on the **shipped defaults**, and
those are a complete and valid configuration rather than a failure to find one: capabilities
deny-all, `[mode] default = "production"`, and every limit at its documented default. The boot log says
so in one line. They stay the configuration whenever a project command could not write the data
folder's `nvs.toml` (`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`).

**An unwritten `[limits] memory` is 128 MiB, for every kind of run.** **An unwritten `cpu_time` is 30
seconds for server work** — a request, a connection, a scheduled run, a queued job and every isolate
in their trees — **and no ceiling for `nvs run`, a `#[Command]` method and `nvs test`**, as a CLI
program has no execution-time limit in PHP either. `wall_time` has no default. `false` written for
either key is no ceiling, and the default is what a written `false` is distinguished from. A context
that was handed no configuration at all takes neither default: it has no tree to read one from.

**An unwritten `[limits.hard]` key is the `[limits]` value the run started with** — the written one
after `[app.limits]` folded into it, or the default above — so a program narrows its own limit with
`Core\Config::set` and raises it only once the operator writes a ceiling above the start
(`rule:config/ceilings-are-their-own-directives`). Under `nvs run` an unwritten `cpu_time` has no
start value, so nothing bounds a set of it either. The two numbers are constants in the config crate
that the runtime, the `Core\Config::set` bound and the admission arithmetic all read.

- **Memory:** up to 128 MiB per request in flight where no `[limits] memory` is written, and that is
  the per-request cap admission divides by (`rule:http-server/admission-is-arithmetic-not-a-number`).
  It is spent to buy **Security and request isolation**: a host nobody configured can no longer be
  exhausted by one request.

Every reader has to answer on such a host. `Core\Config::get` is `null`, `all` is empty, `set` is
`false` and `restore` does nothing — none of them throws, because "no configuration file anywhere" is
a valid host and a program has to be able to ask on one. That is every test context and every
embedder that has not built a snapshot.

The one distinction the shipped defaults keep is between *no tree* and *a tree that grants nothing*:
a program checked outside any project root has no configuration to be measured against, while a tree
that was read and says nothing about capabilities is an operator's written `no`. The offline audit
reports the first as a tree of zero files.

**A `[block]` header with no key under it says nothing, and the resolver drops it before the tree is
typed**, at any depth and for a named block as much as a fixed one: `[metrics]` alone is no
`[metrics]`, and `[db.main]` alone names no connection. No reader of a block is ever handed one that
is present and empty, so none has to decide what that would mean. That is what lets the shipped file
carry its headers live — setting a key there is removing one `#` — and still resolve to exactly what
no file resolves to. An `[[entry]]` is the exception, because writing one says the entry exists: an
`[[app]]` with no path is refused rather than skipped, and the shipped file keeps every `[[entry]]`
header, and the tables belonging to one, commented out — and the two `pool` tables under `[db]`,
each of which is the same TOML key as the `pool = false` beside it and cannot be live with it.
`bun nv directives
--check-template` refuses a key left under a commented-out header when the live header above it has
a key of the same name, which is the one arrangement where uncommenting the key alone would be
accepted into the wrong block.
