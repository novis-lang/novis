The registry service and its website are a Novis web application in this repository, `apps/registry/`,
and the byte-level work it needs is Rust in `nvs` subcommands the service runs — the same code the
client runs.

**The application** stores its data in SQLite through `Core\Db`, signs users in with `Core\Passkey`
(`rule:core-classes/passkey-verifies-a-webauthn-ceremony`), sends push notifications with `Core\WebPush`
(`rule:core-classes/web-push-sends-an-encrypted-message`), and runs the subcommands below as child
processes, granted in its own `nvs.toml`. It writes the static index that the client reads (one signed
JSON file per package and one archive per digest) to the stores its settings file names, and where
those files are served from is configuration (`rule:packaging/registry-hosting-is-configuration`).

**The byte work** is these commands:

| Command | Does |
|---|---|
| `nvs registry build` | clones a commit and packs and digests the package directory (`rule:packaging/the-registry-builds-the-archive-from-git`) |
| `nvs registry log append` | adds entries to the log and signs a new checkpoint (`rule:packaging/the-registry-keeps-an-append-only-log`) |
| `nvs registry log verify` | checks a log directory from scratch |
| `nvs registry index sign` | signs index files with the log key, the time and the current checkpoint (`rule:packaging/the-registry-keeps-an-append-only-log`) |
| `nvs pkg api`, `nvs pkg diff` | extract a package's API and compare two versions (`rule:packaging/only-a-major-may-break-the-api`) |
| `nvs pkg pack` | builds the deterministic archive, as `nvs registry build` does after its clone |

**The pages** are rendered on the server with `html` templates through one layout, and every page
sends `Content-Security-Policy: default-src 'self'; script-src 'self'; object-src 'none'; frame-ancestors 'none'`.
One same-origin script, `/registry.js`, runs every WebAuthn ceremony and sends every `POST` and
`DELETE` a page makes, to the same routes and with the same access as any other client. A step whose
route needs a passkey from the last 15 minutes runs the sign-in first when the route answers `403`. A
service worker, `/push.js`, shows each push message and sends the code of a test push back. There is no
framework, no build step and no third-party script. `tools/nv/test/registry-browser.test.ts` drives
the pages in a real Chromium with a virtual passkey.

`nvs registry` is operator surface for the service and prints no help in a user's command list. The
`nvs pkg` commands are the user's (`rule:packaging/package-toml-is-a-tool-input`).

Running the same Rust on both sides is what makes a digest predictable: `nvs pkg publish --dry-run` and
the registry's build produce the same bytes from the same commit. Writing the service in Novis makes the
registry a real program that depends on `Core`'s web surface, so a gap in that surface is found by the
project before a user finds it. A deployment guide states how to run it, so a private registry is the
same program.

**Memory:** the service's memory is per request, with SQLite on disk. A build or a log append is a
child process whose memory ends with it.

**Declined:**
- **A service in Rust or another language.** It would share no code path with the programs Novis
  exists to run, and its byte work would be a second implementation of the client's.
- **Byte work written in Novis.** Cloning, packing, digesting and log signing are the client's code
  already, and a second copy could produce a different digest.

The application's `README.md` is the deployment guide: the commands that run it and every setting of
`hosting.json`. `NvsBuilder` runs `nvs registry build`, `NvsLog` runs `nvs registry log append` and
`nvs registry index sign`, and runs `nvs registry log verify` when the registry restores itself
(`LogCheck`). The application's tests replace each of them with a double, and
`crates/nvs-cli/tests/registry_build.rs` runs the real service against the real `nvs`.
