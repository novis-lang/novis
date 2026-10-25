Four steps, first hit wins:

1. Every `--config <path>`, in the order given. The flag is **repeatable**, and the files form an
   ordered list merged exactly as an include list is. A `--config` naming a file that does not exist
   is always a hard refusal — optionality is a property a file declares about *its* includes, never
   something argv can assert.
2. `./nvs.toml` — **exactly one directory, never a walk upward.** Any explicit `--config` disables this
   step and the next entirely, so an operator naming files never gets a surprise merge with whatever is
   in the working directory or the data folder, even if every named file turns out to be missing.
3. `nvs.toml` in the **data folder** — `.nvsdata` beside the running binary, or the folder the global
   `--data` flag names — if it exists. A process with no usable data folder skips this step. The file
   Novis writes there starts with a line naming its version and a hash of the rest, and before the
   first lookup reads it, an unedited one another version wrote is replaced with this version's
   template; a file without the line or with another hash is never touched, and a refresh that cannot
   write reads the file as it is.
4. Otherwise the shipped defaults
   (`rule:config/no-configuration-file-is-a-complete-configuration`). A **project command** — `run`,
   `serve`, `test`, `build`, `check` and `dap` — first writes the shipped default file as the data folder's
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

The data folder is beside the file the binary really is: the running binary's path is canonicalized,
so a binary reached through a link uses the folder beside its target. An unedited template is
refreshed because a block a later version removes would otherwise make every command of that version
stop on the file an older one wrote. The cost is usability: a binary in a directory its account cannot
write, such as a packaged `/usr/bin/nvs` run by an ordinary user, finds no usable default folder,
prints the one warning, and runs without a compile cache until `--data` names a folder. One binary
serving several deployments shares one default folder, so a deployment that wants its own passes
`--config` or `--data`, and a service always stores one.

- **Memory:** none. Locating the folder is one canonicalization per process, and preparing it is one
  create attempt per folder at startup.

What makes the working-directory step safe is `rule:config/ownership-is-the-trust-boundary`, the
announced path (`rule:config/the-resolved-root-is-announced-and-stored`) and an installer that always
stores an absolute `--config` for a service (`rule:packaging/the-installer-is-a-sink`). What survives
is one narrow case — an interactive `nvs serve` in a directory the runtime account can write — stated
rather than defended: an operator who wants it closed passes `--config`. If the announced path shows
`./nvs.toml` being read in production, the answer is to confine the working-directory step to the
commands attached to a terminal, not to remove it.

**Declined:**
- **A platform default path, such as `/etc/nvs/nvs.toml`.** A deployment that wants a fixed path runs
  under a service manager, and the installer stores an absolute `--config` for it anyway
  (`rule:packaging/the-installer-is-a-sink`).
- **A path built into the binary at compile time**, as nginx's `--conf-path` is. It is one more
  resolution step for a case `--config` already covers.
- **Walking up from the working directory or the entry file.** The single-directory step stops one
  step short of a walk (`rule:config/nvs-toml-is-not-a-project-manifest`).
- **Per-platform locations for Novis's own files**, such as a cache under the user's local application
  data and a temporary root in the system temporary directory. They are several places an operator has
  to know, one of them shared with every other program on the host.
- **A data folder under the user's profile.** It moves with the account, not with the binary, so two
  installed versions or a portable copy would share it, and a service account's profile is a place
  nobody looks.
