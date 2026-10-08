The trailing argv is the sharpest sink in the project (`rule:security/sink-predicate`): the command
runs elevated, and what it stores is executed by a privileged account at every boot until somebody
removes it. So the default is refusal, and the allowlist is closed:

| Refused | Because |
|---|---|
| A subcommand other than `serve` or `run` | everything else exits at once — a crash loop, forever — or needs a terminal |
| `--fault-inject`, on any subcommand | a hook that must never be reachable from a served request, now with a privileged account |
| An argv this binary's own parser refuses — `serve` with no entry file, an option it does not have | it exits at once with a usage error written to a console that is not there, which is the first row's crash loop reached through an allowed subcommand |
| Any relative path: a `--config` or `--data` value, the entry file, or the `[log] target` file, `[opcache] file_cache_dir` or `[io] temp_root` of the configuration the service reads | a service starts in `System32` or `/`: a first-boot failure as an opaque service-manager code, and for the configuration's three a log, a cache or a temporary root in a directory nobody chose, under a grant made against the installing shell's |
| A data folder the service cannot use, when the stored `--config` is that folder's own `nvs.toml` | that file is the service's whole configuration, so a service installed without it refuses to start at every boot |
| An `--account` password on the command line | readable by other users; it is prompted, and is `secret` for its whole life (`rule:security/secret-qualifier`) |
| Running from a bundle | `rule:packaging/a-bundle-may-not-install-itself` |

**An argv with no `--config` is completed, not refused.** The stored argv gets `--config` and the
absolute path of the service's data folder's `nvs.toml` right after the subcommand, so a service never
looks for `./nvs.toml` in the directory its manager starts it in
(`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`). The data folder is the
argv's own `--data`, else `.nvsdata` beside the binary the service runs, and every check above runs
over that file as if the operator had named it. An install that is not a dry run creates the folder
privately and writes the shipped template there if no file is there yet, before the service manager is
asked for anything. With a `--config` named elsewhere, an unusable data folder is the warning every
command prints for it, the install proceeds, and nothing is granted on the folder.

There is no row for where the service's output goes, and no `--log-file` option: a hosted process's
stdout and stderr are the platform log's — the journal under systemd, the event log under the SCM
(`rule:packaging/a-service-answers-its-manager`) — so a refused compile leaves its diagnostic where an
administrator looks without the installer being told a path. Once the configuration is read, a
service whose configuration names no `[log] target` writes its log records to `logs/nvs.log` in its
data folder (`rule:errors/engine-floor`), which is why the account's grant includes `logs/`. A
`[log] target` of `stderr` is accepted like any other.

Every surviving path is canonicalized and stored absolute. Each refusal is an `E0630`, `E0631`,
`E0633`, `E0634` or `E0653` diagnostic naming what was refused and why — rows that share a reason share
a code — never a bare non-zero exit. The refusals run in front of `nvs service unit` too, so an
operator learns what would have been refused without an elevated shell and without installing
anything.

An install the service manager stops part way fails closed as well. The error names the step it
stopped on, and the steps applied in front of it are undone last first, so a failed install leaves
what a refused one leaves. The undo is built from the steps that were applied and never from the plan:
a registration refused because the name is taken is followed by no deregistration, which would delete
the service that holds the name. An undo that does not finish says what is left and that `nvs service
uninstall` removes it.
