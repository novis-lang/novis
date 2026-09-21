The trailing argv is the sharpest sink in the project (`rule:security/sink-predicate`): the command
runs elevated, and what it stores is executed by a privileged account at every boot until somebody
removes it. So the default is refusal, and the allowlist is closed:

| Refused | Because |
|---|---|
| A subcommand other than `serve` or `run` | everything else exits at once — a crash loop, forever — or needs a terminal |
| `--fault-inject`, on any subcommand | a hook that must never be reachable from a served request, now with a privileged account |
| Any relative path, in the argv, an installer option, or the named configuration's `[log] target` file and `[opcache] file_cache_dir` | a Windows service starts in `System32`: a first-boot failure as an opaque SCM code, and for the configuration's two a log or a cache in a directory nobody chose, under a grant made against the installing shell's |
| An argv with no `--config` | it would fall back to `./nvs.toml` (`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`), making the configuration a property of the starting directory; a service names it absolutely |
| Neither `--log-file` nor a `[log]` file or syslog destination | a service has no console handle, so stderr goes nowhere and a refused compile leaves no trace; `stderr` is not a destination |
| An `--account` password on the command line | readable by other users; it is prompted, and is `secret` for its whole life (`rule:security/secret-qualifier`) |
| Running from a bundle | `rule:packaging/a-bundle-may-not-install-itself` |

Every surviving path is canonicalized and stored absolute. Each refusal is an `E0630`–`E0634`
diagnostic naming what was refused and why — two pairs of rows share a code because they share a
reason — never a bare non-zero exit. The refusals run in front of `nvs service unit` too, so an
operator learns what would have been refused without an elevated shell and without installing
anything.
