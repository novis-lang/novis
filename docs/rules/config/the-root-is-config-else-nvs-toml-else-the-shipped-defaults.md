Four steps, first hit wins:

1. Every `--config <path>`, in the order given. The flag is **repeatable**, and the files form an
   ordered list merged exactly as an include list is. A `--config` naming a file that does not exist
   is always a hard refusal — optionality is a property a file declares about *its* includes, never
   something argv can assert.
2. `./nvs.toml` — **exactly one directory, never a walk upward.** Any explicit `--config` disables this
   step and the next entirely, so an operator naming files never gets a surprise merge with whatever is
   in the working directory or the data folder, even if every named file turns out to be missing.
3. `nvs.toml` in the **data folder** — `.nvsdata` beside the running binary, or the folder the global
   `--data` flag names — if it exists. A process with no usable data folder skips this step.
4. Otherwise the shipped defaults
   (`rule:config/no-configuration-file-is-a-complete-configuration`). A **project command** — `run`,
   `serve`, `test`, `build` and `check` — first writes the shipped default file as the data folder's
   `nvs.toml`, the file step 3 looks for, and then reads it. Every key in it is commented out, so it
   resolves to exactly the shipped defaults. `--no-init`, or `NOVIS_NO_INIT` set to any value, turns
   that one write off; the data folder itself is still created. A write the folder refuses is no
   error: the run continues on the shipped defaults.

**Novis never writes `./nvs.toml` on its own.** The working directory holds a file only when somebody
put it there, or asked `nvs init` to write it. The language server's per-document lookup takes the
same four steps with the document's folder in place of the working directory. `nvs config check` and
`nvs config dump` take the `--config` list positionally.

There is no platform path and no build-time path. The data folder is the one lookup beside the binary:
it is a folder Novis creates private to its account (`rule:config/ownership-is-the-trust-boundary`),
it comes after both explicit steps, and a deployment that wants a file of its own names it with
`--config` or moves the folder with `--data`.

What makes the working-directory step safe is `rule:config/ownership-is-the-trust-boundary`, the
announced path (`rule:config/the-resolved-root-is-announced-and-stored`) and an installer that always
stores an absolute `--config` for a service (`rule:packaging/the-installer-is-a-sink`). What survives
is one narrow case — an interactive `nvs serve` in a directory the runtime account can write — stated
rather than defended: an operator who wants it closed passes `--config`.
