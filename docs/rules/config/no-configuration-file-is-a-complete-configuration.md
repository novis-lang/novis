When no `--config` is given and neither `./nvs.toml` nor the data folder's `nvs.toml` exists, the host
runs on the **shipped defaults**, and
those are a complete and valid configuration rather than a failure to find one: every capability at
its default, `[mode] default = "production"`, and every limit at its documented default. The boot log says
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
start value, so nothing bounds a set of it either. **An unwritten `[limits.hard] memory` is no ceiling
on a CLI run**, so a CLI program's own code raises its memory limit freely; in server work it is the
start value, as every other key is.

**Every built-in default has one home: the config crate's table of defaults.** Each key the shipped
`nvs.toml` marks `# default` has its value there, and every piece of code that enforces the value
reads it from there — the runtime, the `Core\Config::set` bound and admission among them. A test holds every `# default` line of the shipped file to the table, so the file and the code
cannot disagree. A default that is not one value is computed when it is read: `cpu_time` by the kind
of run, the mode table's rows by the mode in force, `[server] dispatch`, `static` and
`[opcache] settle` by the mode the run started in, `[mode] ceiling` as that mode, and
`[server] workers` as this machine's cores.

- **Memory:** up to 128 MiB per request in flight where no `[limits] memory` is written. That is a
  request's ceiling and not what it is assumed to hold: admission counts what requests hold and
  leaves room for one more to reach the cap (`rule:http-server/admission-is-arithmetic-not-a-number`).
  It is spent to buy **Security and request isolation**: a host nobody configured can no longer be
  exhausted by one request.

Every reader has to answer on such a host, and none of them throws, because "no configuration file
anywhere" is a valid host and a program has to be able to ask on one. **`Core\Config::get` and `all`
give the value in force**: what the request set, then a secret, then what the file wrote, then the
built-in default, rendered the way the shipped file writes it (`128M`, `30s`, `1000000`). A key with
no default (`wall_time`) is `null`, and so is a key whose default is nothing for this run
(`cpu_time` under `nvs run`). An unwritten `[log] target` is `null` too, because it is not one
destination: a `Core\Log::write` record is the program's output and a floor record goes to stderr
(`rule:errors/engine-floor`). A default inside a named block (`[db.<name>]`) is in force only where
that block is written. `set` and `restore` behave as on any host: `set` is bounded as above, and
`restore` brings back the written value or the default. A context that was handed no configuration
at all — a test context, an embedder that has not built a snapshot — has no tree and no defaults:
`get` is `null`, `all` is empty, `set` is `false` and `restore` does nothing.

**An unwritten `fs.read` or `fs.write` is every path on a CLI run.** In server work an unwritten
`fs.read` is the app folder, less every path with a component below it that starts with `.` (`.env`,
`.git/config`) and every file named `nvs.toml`, and an unwritten `fs.write` is the directories
`Core\IO::temporaryDir` made for this context, which it also reads. A written key replaces its
default whole, so `read = []` reads nothing and leaves writing at its default. Every other
capability defaults to nothing (`rule:security/capability-question-is-grant-and-scope`). `nvs serve`
names, in one boot line per app folder whose tree writes no `fs.read`, the folder its requests read.

- **Memory:** the app folder is one path per context, and the temporary directories are the list
  the context already keeps for its sweep (`rule:core-classes/temporary-dir-sweep`). It buys
  **Simplicity**: a first program reads and writes its files with no `nvs.toml`, while a server
  never writes beside code it would run, never reads its own secrets, and one request cannot reach
  another's directory.

The one distinction the shipped defaults keep is between *no tree* and *a tree that grants nothing*:
a program checked outside any project root has no configuration to be measured against, while a tree
that was read and writes a capability's key as `[]` is an operator's written `no`. A context handed
no configuration at all grants nothing. The offline audit reports the first as a tree of zero files.

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
