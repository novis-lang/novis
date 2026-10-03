# Setting up a machine

What a machine needs before `bun nv verify` or the acceptance run mean anything. One-time, per
machine, and this file is the whole of it: the install list, then the few things a `git clone` does not
carry. How the repo is *driven* once it is set up — `nv verify`, `nv splice`, WSL one-liners, valgrind,
fuzzing — is [docs/agent/commands.md](agent/commands.md).

**On Windows, WSL is not optional and PHP is installed twice, at the same version.** That is the pair a new
machine gets wrong. The rest is a Rust toolchain that installs itself.

**Moving development to another machine is this file and nothing else.** Everything that decides what
happens next is committed — the plan, the chain and its live goal, the goal's handoff record — so a clone plus the steps
below lands the new machine where the old one stopped. § *What a clone does not carry* is the part that is
not an install; § *Picking up where the last machine left off* is the order to do it all in.

## Getting the repository

```sh
git clone https://github.com/novis-lang/novis
```

**Where the tree lands does not matter.** Nothing in the repository hardcodes a checkout path: every tool
that needs the WSL side's `/mnt/<drive>/…` derives it from wherever the repo actually is. Keep it on the
Windows filesystem rather than inside the distro — the native leg is the primary one. The WSL leg does not
run over the slow 9p mount: it syncs its own copy of the tree onto the distro's disk before each build.

## Every platform

| Need | Why | Check |
|---|---|---|
| Rust, the version pinned in [rust-toolchain.toml](../rust-toolchain.toml) | `rustup` installs it on the first `cargo` command inside the tree — nothing to do by hand. Never a different channel: the pin is what makes three platforms the same compiler. | `cargo --version` |
| Python 3.11+ — **optional, for `bun nv bench` only** | The Python engine of the cross-language bench: `bun nv bench` runs the `.py` twin of every case under [benches/userland/](../benches/userland/). Nothing else in the tree runs Python. No third-party package is ever required. | `python --version` |
| The `claude` CLI on `PATH` — **the unattended loop only** | `bun nv loop` spawns one `claude -p` per session and finds it with `Bun.which("claude")`. With nothing on `PATH` it falls back to the bare name and the run dies on session 1, *after* printing the launch line and building the orientation pack — so it reads like a loop bug rather than a missing install. **An IDE extension does not count.** The VS Code extension carries its own `claude` binary inside its versioned extension directory and never puts it on `PATH`, so a machine that runs Claude Code all day can still have none; `claude install stable`, runnable from that bundled binary, lands one in `~/.local/bin` (`%USERPROFILE%\.local\bin` on Windows) that updates itself independently of the editor. Nothing else in the tree spawns a session — `nv verify` and `nv loop --goal-only` never do. | `bun -e "console.log(Bun.which('claude'))"` — the CLI's own `--version` can pass on a shell alias that `bun nv loop` cannot see |
| PHP on `PATH`, at the version in [the plan](implementation-plan.md)'s status block § *Toolchain* — that field is the version's one home, and it reads 8.5 today | The differential oracle. A `tests/differential/` case runs its `--ORACLE--` twin under real PHP and compares stdout, so a machine without it **skips** those cases instead of failing them. It is also the fastest way to settle a semantics question while authoring: `php -r '…'`. | `php -v` |
| Node.js 20 LTS or newer, with `npm` — **from M4B onward** | `editors/vscode` is TypeScript, and its headless tests — the TextMate grammar snapshots and the LSP protocol round-trip against the real `nvs lsp` binary — are acceptance checks. Without Node they do not fail, they cannot run. Only the machine's native side needs it: those checks run once, not once per leg, so the WSL distro does not. | `node --version`, `npm --version` |
| Bun, at the version `package.json`'s `engines` pins | It runs the repository's tools: `bun nv <command>`, from `tools/nv/`. Run `bun install` once after a clone and again whenever `bun.lock` changes. It installs `typescript`, `@types/bun` and `smol-toml` into the git-ignored `node_modules/`. `nv verify`'s `nv` step runs `bun nv selftest`, so a machine without Bun fails the gate. It is also the fourth engine in [benches/userland/](../benches/userland/), which runs the `.ts` twin of every case (`rule:tooling/bench-engine-list-is-data`). Its Windows installer does not always land on `PATH`; `bun nv bench --bun <path>` takes the executable explicitly. | `bun --version`, then `bun nv selftest` |
| Docker, with Compose — Docker Desktop on Windows and macOS | The floor's database matrix, `bun nv db-matrix --all`, runs each database server in a container, and a goal whose record names `env.docker` stops before its first session when `docker` is not on `PATH` or its daemon does not answer (`tools/nv/driver/gates.ts`). `bun nv bench-proxied` is containers too. `--driver sqlite` needs no container. | `docker compose version` |
| VS Code — **from M4B onward** | Two different things. `@vscode/test-electron` downloads its **own** pinned build into `editors/vscode/.vscode-test/` for the extension-host tier, so a system install is not what that test runs against; the system install is what you drive the extension in by hand, which is the entire point of pulling M4B ahead of M10. Fetch the test build once (below) and nothing afterwards touches the network. | `code --version` |

Novis generates native code, so "it compiles here" is a weaker claim in this repository than in most. CI
builds all three supported targets — `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`,
`aarch64-apple-darwin` — and a developer machine is expected to cover two of them (see below).

## Claude Code needs the pointer file

The rules every agent reads live in [AGENTS.md](../AGENTS.md), and the root carries no harness's own file.
Claude Code cannot find that name: its project-doc loader reads `CLAUDE.md`, `.claude/CLAUDE.md`,
`CLAUDE.local.md` and `.claude/rules/`, and nothing else. So the tree ships
[.claude/CLAUDE.md](../.claude/CLAUDE.md), whose whole job is the `@../AGENTS.md` line that inlines the
real file — the import resolves against the directory of the file holding it, which is why the `../` is
load-bearing.

A clone carries it, so there is normally nothing to do. **If it is absent — an older revision, or a machine
that keeps `.claude/` out of git — write it before the first session**, with that import line and nothing
project-specific in it. Without it a Claude session opens with no project instructions whatsoever and reads
like a model that has never seen this repository; the unattended loop fails the same way, because
`bun nv loop` spawns that same `claude` binary. No other harness needs anything here: Codex reads
`AGENTS.md` at the root directly.

## Windows

The primary development platform, and the only one with real setup:

1. **The MSVC C++ build tools** — the linker. The `x86_64-pc-windows-msvc` target has no other.
2. **PHP on the Windows `PATH`.**
3. **WSL, with a second full toolchain inside it.** Required, not a convenience: three things have no
   Windows story at all — `valgrind`/`callgrind` (no native build,
   `rule:testing/perf-two-mechanisms`), `cargo-fuzz` (needs libFuzzer), and the
   acceptance run's second leg, which exists because a JIT is exactly where a calling-convention
   divergence between two targets hides.
4. **The live service check's task.** `nvs service install`, `start` and `stop` talk to the service
   control manager, which only an elevated process may do, and neither a terminal nor an agent
   session is one. UAC is not weakened for that: `tools/service-live.ps1` registers **one**
   scheduled task, `novis-service-live`, whose only action is that script, elevated, and which its
   owner then starts from any shell with no prompt. From the repository root, once — this is the
   one UAC prompt:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File tools\service-live.ps1 -Register
   ```

   From then on the same line without `-Register` runs the check — install, start, a request,
   stop, `sc qc`, the event log, uninstall — and prints what it saw; `-Unregister` takes the task
   away. The task points at the clone that registered it, so a clone moved runs `-Register` again.

One-time setup inside the distro, which reaches the repo over its `/mnt/<drive>/…` mount:

```sh
sudo apt-get update && sudo apt-get install -y build-essential clang valgrind git time
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
rustup toolchain install nightly --component rust-src
cargo install cargo-fuzz --locked
```

`rust-src` is not optional here: `tools/tsan.sh` builds the standard library from source under the thread
sanitizer, and without that component the leg stops before it compiles anything of Novis's. `git` is what
the WSL leg syncs its copy of the tree with (`tools/nv/driver/mirror.ts`), and Ubuntu's WSL image usually
has it already. `time` is `/usr/bin/time`, which the machine probe uses to measure one fixture; without it
the probe records less and the valgrind sweep still runs.

### PHP goes in the distro too, at the same version

The oracle's whole value is that a difference in output is attributable to **Novis**. If Windows runs 8.5 and
the distro runs 8.3, a differential case that passes on one leg and fails on the other says nothing about
Novis's code generation — which is the only question the second leg exists to answer. So both sides carry
PHP, and the major *and* minor must match:

```sh
php -v                              # Windows
wsl.exe -- bash -lc 'php -v'        # the distro
```

A distribution's own archive is usually a release or two behind; on Ubuntu, `ppa:ondrej/php` carries 8.5:

```sh
sudo add-apt-repository -y ppa:ondrej/php && sudo apt-get install -y php8.5-cli
```

Whatever the source, treat a version mismatch as a machine that is not set up.

## Linux and macOS

The native leg already *is* the second target, so there is no WSL leg, and the valgrind sweep runs
directly. Install a C toolchain, PHP 8.5, and (on Linux) `valgrind` the same way;
`cargo-fuzz` still needs a nightly toolchain, with `rust-src` on it for the same reason the WSL one needs
it. macOS has no valgrind, so the leak sweep is a Linux or WSL machine's job, and so is `tools/tsan.sh`:
`-Zsanitizer=thread` targets `x86_64-unknown-linux-gnu`.

**One sysctl, Linux only — and the WSL distro is a Linux machine for this.** A task's stack is a
reservation that leaves more than one mapping behind
(`rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`), and a Linux process may hold only
`vm.max_map_count` of those. The stock setting is well under what the release-only test holding a hundred
thousand tasks at once needs, so on an unraised kernel that test fails naming this sysctl — it demands the
kernel rather than shrinking to fit, because a run proving 32k tasks on the platform servers deploy to has
not proved the claim. Nothing else in the tree cares, so this is only worth doing on a machine that runs
the release checks:

```sh
sudo sysctl -w vm.max_map_count=262144                  # until the next boot
echo 'vm.max_map_count=262144' | sudo tee /etc/sysctl.d/99-novis.conf   # across one
```

## What a clone does not carry

Three things sit outside what git tracks. The first is not optional, and neither is the trust flag in the
third if anyone will work in this tree interactively.

1. **The git hooks.** git does not version `.git/hooks`, so `tools/git-hooks/` is inert until this clone
   is pointed at it. One setting enables every hook in that directory, and a hook added there later
   needs nothing more:

   ```sh
   git config core.hooksPath tools/git-hooks
   git config core.hooksPath                   # prints tools/git-hooks when it is set
   ```

   `nv verify` prints a line every run until it is set. There are two:

   | Hook | Refuses | Cleared by |
   |---|---|---|
   | `commit-msg` | a commit message carrying an attribution trailer — [docs/agent/conventions.md](agent/conventions.md) § *A commit message* | deleting the lines it names |
   | `pre-push` | a push while a carried check is not green over the tree, or while tracked files have uncommitted changes | `bun nv loop --settle`, which runs what is owed and only that; `bun nv loop --owed` names it and runs nothing |

   The second exists because verification runs what a change can reach and defers the checks that
   cost minutes; [docs/agent/commands.md](agent/commands.md) § *What is owed, and where it is collected*
   is the reasoning. Neither hook is bypassed with `--no-verify`. On Linux and macOS a hook must also
   be executable, which the repository records; if a checkout lost the bit, `chmod +x
   tools/git-hooks/*` puts it back.
2. **A git identity**, if the machine has no global one — `git config user.name` and `user.email`. Every
   session ends in commits, so a machine that cannot commit cannot finish one.
3. **Machine-local harness settings.** `.claude/settings.json` is committed and carries the shared
   permission allowlist; `.claude/settings.local.json` is per-machine, is not, and is optional.
   **Trusting the workspace is not.** Until this clone is trusted the committed allowlist is ignored
   entirely — one `Ignoring N permissions.allow entries … this workspace has not been trusted` line, and
   then a prompt for every call the file already allows. Trust is per-machine state in
   `~/.claude.json`: open the tree in an interactive `claude` once and accept, or set
   `projects["<absolute path to the tree>"].hasTrustDialogAccepted: true`. The loop needs it
   too: its sessions run under `--permission-mode auto`, where the allowlist is what lets a session run
   `cargo`, `git commit` and `bun nv` without each call going to the harness's classifier.

CI installs `cargo-deny` and `cargo-geiger`; a development machine needs neither. `cargo-fuzz` is the WSL
side's, above.

## What does not travel, and should not

`.loop/`, `.agent-tmp/`, `.cache/`, `target/` and `.nvs-cache/` are gitignored, and copying one to the new
machine is worse than leaving it: `.cache/select.sqlite` holds what every check was observed to use and
whether it was green *on this machine's build and platform*, and `bun nv select --seed` records it again. `.agent-tmp/` is scratch. A stale
green is what costs a debugging session, so do not archive them "just in case".

## Proving the machine is set up

```sh
bun nv verify                                            # build, fmt, test, the .nvst trees, clippy, the extension
cargo run -q -p nvs-cli -- test tests/differential/       # must report 0 skipped
```

The WSL leg and the valgrind sweep are the driver's: every `bun nv loop` sweep runs them once the native
sweep is green (`tools/nv/driver/legs.ts`).

From M4B onward, one more one-time step, because the extension-host tier downloads a VS Code build and
the acceptance run must never go to the network:

```sh
cd editors/vscode && npm ci && npm run test:prepare       # fetches the pinned VS Code into .vscode-test/
npm run test:headless                                     # grammar, contributions, protocol -- no editor
```

The second one is the check that actually catches a missing PHP: what matters is **`0 skipped`**. A suite
whose oracle cannot be run prints one `no PHP oracle` line per case and still **exits 0**, so nothing else
in this repository will tell you the coverage is gone.

## Picking up where the last machine left off

Once those are green, in this order:

1. `git status` and `git log --oneline -5`. The old machine's last session committed everything it did, so
   a clean tree sitting at `origin/main` *is* the handover.
2. **`bun nv orient --full`** — where the run and the work stand, one line per module, the rules, the
   shapes and the plan's fields. Step 1 of every session, machine move or not ([AGENTS.md](../AGENTS.md)).
3. The live goal's handoff, which `bun nv orient` prints from `data/goals/<slug>.handoff.json` —
   where the work stands now, and the next group of slices with the file set they share. It is
   overwritten each session, so it is state rather than history.
4. `bun nv loop` if the unattended loop is what runs next; its design is
   [docs/agent/coordinator.md](agent/coordinator.md). It is the one thing here that needs a `claude` on
   `PATH` (§ *Every platform*), and the only step above will not have caught its absence.

None of that is machine-specific, which is the point: the handover is the repository.
