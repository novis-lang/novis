`package.toml` is read by the `nvs pkg` commands, and the compiler reads **one table of the
application's file, `[grants]`**, and nothing else of it. Neither the runtime nor name resolution ever
reads `package.toml`.

**Every package and registry command a user types is a subcommand of `nvs pkg`**: `add`, `fetch`,
`update`, `outdated`, `vendor`, `audit`, `publish`, `retract`, `unlist`, `login`, `whoami`, `api`,
`diff`, `pack`, `download` and `provide`. That is the one spelling, short as `nvs ext` is
(`rule:packaging/the-cli-surface-is-novis-own`): there is no `nvs package` alias and no flat `nvs add`
or `nvs update`, so a package verb never collides with a toolchain command such as `nvs update-check`.
The files keep their full names, `package.toml` and `package.lock`. The registry service's own byte work is `nvs registry`
(`rule:packaging/the-registry-is-a-novis-program`), which is operator surface.

The rejection of "a manifest file found by walking up from the entry file"
(`rule:programs/no-runtime-autoload`) stands, and so does
`rule:config/nvs-toml-is-not-a-project-manifest`. Both are about where a declaration is found, and no
declaration is in `package.toml`: a program whose `vendor/` is populated resolves every name the same
way whether the file is present, absent or malformed. A fetched package reaches the program as an
ordinary `autoload` line (`rule:packaging/a-fetched-package-is-an-autoload-line`).

**`[grants]` is the application's grant table** (`rule:security/grants-are-keyed-on-a-namespace`).
`nvs pkg add` and `nvs pkg fetch` write one line per package in the resolved graph, an empty one
included, and never rewrite a line a person already edited. A person reviews and edits the lines. The
operator's `nvs.toml` can only narrow what they grant. A package's own `package.toml` declares its name,
version, dependencies and the capabilities it requests, split into `required` and `optional`
(`rule:security/optional-capability-degrades`); the compiler reads that name and that split for a file
inside the package, and nothing else of a package's file.

**Only the grants fold into the cache key.** Every compiled unit's key carries the grant table and the
capability declarations it is intersected with, the way it carries the extension set
(`rule:config/the-extension-set-is-in-every-unit-key`), so an edited grant line is an ordinary cache
miss. A changed version, dependency or comment changes no key, because none of them changes what
compiles.

Every `nvs pkg` command finds the file by walking up from the working directory, the way `git` finds
`.git`, and the compiler finds it by walking up from the entry file's directory: a tool locating its
own project is a different question from a language locating a declaration. The file is TOML with unknown fields refused
(`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`).

**Memory:** the grant table is compile-time input, and nothing of it is held per request.

**Declined:**
- **The grant table in `nvs.toml`.** That file is the operator's and deployment-scoped
  (`rule:config/nvs-toml-is-not-a-project-manifest`). A grant is the application author's decision
  about a dependency, and it changes in the same commit as the dependency graph. The operator keeps the
  ceiling through the intersection.
- **The whole file in the cache key.** A version bump or a comment would invalidate every unit, while
  only the grants change what compiles.
- **Flat package verbs, `nvs add` and `nvs publish`.** They spread one workflow across the toolchain's
  top level, and `nvs update` would sit beside `nvs update-check` meaning something else.
- **`nvs package` as a second spelling of `nvs pkg`.** Two spellings of one command are two things to
  document, complete and search for.

Every `nvs pkg` subcommand is a module under `crates/nvs-cli/src/package/`, which `mod.rs` lists. The
editor reads `[grants]` from the snapshot the runtime enforces, and a route's hover shows them
(`crates/nvs-lsp/src/grants.rs`).
