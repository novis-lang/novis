Four steps, first hit wins:

1. Every `--config <path>`, in the order given. The flag is **repeatable**, and the files form an
   ordered list merged exactly as an include list is. A `--config` naming a file that does not exist
   is always a hard refusal — optionality is a property a file declares about *its* includes, never
   something argv can assert.
2. `./nvs.toml` — **exactly one directory, never a walk upward.** Any explicit `--config` disables this
   step entirely, so an operator naming files never gets a surprise merge with whatever is in the
   working directory, even if every named file turns out to be missing.
3. Otherwise the shipped defaults
   (`rule:config/no-configuration-file-is-a-complete-configuration`). A **project command** — `run`,
   `serve`, `test`, `build` and `check` — first writes the shipped default file as `./nvs.toml`, the
   file step 2 looks for, and then reads it. Every key in it is commented out, so it resolves to
   exactly the shipped defaults. `--no-init`, or `NOVIS_NO_INIT` set to any value, turns the write
   off, and every other command leaves the directory as it found it. A write the directory refuses
   is no error: the run continues on the shipped defaults.

There is no platform path, no build-time path and no lookup beside the binary: two implicit lookups
are worse than one, and the deployments that want a fixed path run under a service manager, which
carries `--config` anyway. `nvs config check` and `nvs config dump` take the same list positionally.

What makes the working-directory step safe is `rule:config/ownership-is-the-trust-boundary`, the
announced path (`rule:config/the-resolved-root-is-announced-and-stored`) and an installer that
refuses a service whose configuration came from a working directory. What survives is one narrow case
— an interactive `nvs serve` in a directory the runtime account can write — stated rather than
defended: an operator who wants it closed passes `--config`.
