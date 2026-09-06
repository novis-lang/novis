The rejection of "a manifest file found by walking up from the entry file"
(`rule:programs/no-runtime-autoload`) stands, and so does
`rule:config/nvs-toml-is-not-a-project-manifest`. `package.toml` is neither, because the distinction
both draw is about *who reads the file*.

**Neither the runtime nor name resolution ever reads `package.toml`.** Program semantics do not
depend on it, no compiled unit's cache key includes it, and a program whose `vendor/` is already
populated resolves every name identically whether the file is present, absent or malformed. That is
a statement about *name resolution*, not about the build as a whole: the manifest's
`required`/`optional` split (`rule:security/optional-capability-degrades`) and its declared
namespace prefix are compiler inputs, reached through the fetched layout, so a malformed manifest
still fails a build.

It is read by the **`nvs` CLI** — `add`, `fetch`, `update`, `vendor`, `audit`, `publish` — which may
find it by walking up from the working directory the way `git` finds `.git`, because a tool locating
its own project is a different question from a language locating a declaration.

It is TOML with unknown fields refused (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`),
and its integration with the program is the generated file
`rule:packaging/a-fetched-package-is-an-autoload-line` describes.
