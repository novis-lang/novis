# Setting up a machine

What a machine needs before `python tools/verify.py` or the acceptance run mean anything. One-time, per
machine, and this file is the whole of it: the install list, then the few things a `git clone` does not
carry. How the repo is *driven* once it is set up — `verify.py`, `splice.py`, WSL one-liners, valgrind,
fuzzing — is [docs/agent/commands.md](agent/commands.md).

**On Windows, WSL is not optional and PHP is installed twice, at the same version.** That is the pair a new
machine gets wrong. The rest is a Rust toolchain that installs itself.

**Moving development to another machine is this file and nothing else.** Everything that decides what
happens next is committed — the plan, the goal, [handoff.md](agent/handoff.md) — so a clone plus the steps
below lands the new machine where the old one stopped. § *What a clone does not carry* is the part that is
not an install; § *Picking up where the last machine left off* is the order to do it all in.

## Getting the repository

```sh
git clone https://github.com/novis-lang/novis
```

**Where the tree lands does not matter.** Nothing in the repository hardcodes a checkout path: every tool
that needs the WSL side's `/mnt/<drive>/…` derives it from wherever the repo actually is. Keep it on the
Windows filesystem rather than inside the distro — the native leg is the primary one, and the distro
reaches it over the 9p mount.

## Every platform

| Need | Why | Check |
|---|---|---|
| Rust, the version pinned in [rust-toolchain.toml](../rust-toolchain.toml) | `rustup` installs it on the first `cargo` command inside the tree — nothing to do by hand. Never a different channel: the pin is what makes three platforms the same compiler. | `cargo --version` |
| Python 3.11+ | Everything in `tools/`. No third-party package is ever required. | `python --version` |
| The `claude` CLI on `PATH` — **the unattended loop only** | `tools/loop.py` spawns one `claude -p` per session and finds it with `shutil.which("claude")`. With nothing on `PATH` it falls back to the bare name and the run dies on session 1 with `FileNotFoundError: [WinError 2]`, *after* printing the launch line and building the orientation pack — so it reads like a loop bug rather than a missing install. **An IDE extension does not count.** The VS Code extension carries its own `claude` binary inside its versioned extension directory and never puts it on `PATH`, so a machine that runs Claude Code all day can still have none; `claude install stable`, runnable from that bundled binary, lands one in `~/.local/bin` (`%USERPROFILE%\.local\bin` on Windows) that updates itself independently of the editor. Nothing else in the tree spawns a session — `verify.py`, `--goal-only` and `--leg-only` never do. | `python -c "import shutil; print(shutil.which('claude'))"` — the CLI's own `--version` can pass on a shell alias that `loop.py` cannot see |
| PHP on `PATH`, at the version in [the plan](implementation-plan.md)'s status block § *Toolchain* — that field is the version's one home, and it reads 8.5 today | The differential oracle. A `tests/differential/` case runs its `--ORACLE--` twin under real PHP and compares stdout, so a machine without it **skips** those cases instead of failing them. It is also the fastest way to settle a semantics question while authoring: `php -r '…'`. | `php -v` |
| Node.js 20 LTS or newer, with `npm` — **from M4B onward** | `editors/vscode` is TypeScript, and its headless tests — the TextMate grammar snapshots and the LSP protocol round-trip against the real `nvs lsp` binary — are acceptance checks. Without Node they do not fail, they cannot run. Only the machine's native side needs it: those checks run once, not once per leg, so the WSL distro does not. | `node --version`, `npm --version` |
| Bun — **optional, benchmarks only** | The fourth engine in [benches/userland/](../benches/userland/), which runs the `.ts` twin of every case (`rule:tooling/bench-engine-list-is-data`). Nothing else in the tree reads it: without Bun, run `python tools/bench.py --engines nvs,php,python` and the suite is otherwise unchanged. No build, test or loop session touches it. Its Windows installer does not always land on `PATH`; `python tools/bench.py --bun <path>` takes the executable explicitly. | `bun --version` |
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
`tools/loop.py` spawns that same `claude` binary. No other harness needs anything here: Codex reads
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

One-time setup inside the distro, which reaches the repo over its `/mnt/<drive>/…` mount:

```sh
sudo apt-get update && sudo apt-get install -y build-essential clang valgrind
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
rustup toolchain install nightly
cargo install cargo-fuzz --locked
```

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

The native leg already *is* the second target, so there is no WSL leg — `tools/loop.py` detects that and
runs the valgrind sweep directly. Install a C toolchain, PHP 8.5, and (on Linux) `valgrind` the same way;
`cargo-fuzz` still needs a nightly toolchain. macOS has no valgrind, so the leak sweep is a Linux or WSL
machine's job.

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

1. **The commit hooks.** git does not version `.git/hooks`, so `tools/git-hooks/` is inert until this clone
   is pointed at it:

   ```sh
   git config core.hooksPath tools/git-hooks
   ```

   `verify.py` prints a line every run until it is set. What the hook rejects, and why, is
   [docs/agent/conventions.md](agent/conventions.md) § *A commit message*.
2. **A git identity**, if the machine has no global one — `git config user.name` and `user.email`. Every
   session ends in commits, so a machine that cannot commit cannot finish one.
3. **Machine-local harness settings.** `.claude/settings.json` is committed and carries the shared
   permission allowlist; `.claude/settings.local.json` is per-machine, is not, and is optional.
   **Trusting the workspace is not.** Until this clone is trusted the committed allowlist is ignored
   entirely — one `Ignoring N permissions.allow entries … this workspace has not been trusted` line, and
   then a prompt for every call the file already allows. Trust is per-machine state in
   `~/.claude.json`: open the tree in an interactive `claude` once and accept, or set
   `projects["<absolute path to the tree>"].hasTrustDialogAccepted: true`. The loop does not need it —
   `bypassPermissions` answers everything either way — so an untrusted tree costs an interactive session
   and nothing else.

CI installs `cargo-deny` and `cargo-geiger`; a development machine needs neither. `cargo-fuzz` is the WSL
side's, above.

## What does not travel, and should not

`.loop/`, `.agent-tmp/`, `target/` and `.nvs-cache/` are gitignored, and copying one to the new machine is
worse than leaving it: `.loop/goal-green.json` remembers which expensive checks were green *for a given
tree and toolchain fingerprint*, and it re-earns itself on the first run. `.agent-tmp/` is scratch. A stale
green is what costs a debugging session, so do not archive them "just in case".

## Proving the machine is set up

```sh
python tools/verify.py                                   # build, fmt, test, the .nvst trees, clippy, the extension
cargo run -q -p nvs-cli -- test tests/differential/       # must report 0 skipped
python tools/loop.py --leg-only                          # the whole Linux leg; drives WSL on Windows
```

From M4B onward, one more one-time step, because the extension-host tier downloads a VS Code build and
the acceptance run must never go to the network:

```sh
cd editors/vscode && npm ci && npm run test:prepare       # fetches the pinned VS Code into .vscode-test/
npm run test:headless                                     # grammar, contributions, protocol -- no editor
```

The middle one is the check that actually catches a missing PHP: what matters is **`0 skipped`**. A suite
whose oracle cannot be run prints one `no PHP oracle` line per case and still **exits 0**, so nothing else
in this repository will tell you the coverage is gone.

## Picking up where the last machine left off

Once those are green, in this order:

1. `git status` and `git log --oneline -5`. The old machine's last session committed everything it did, so
   a clean tree sitting at `origin/main` *is* the handover.
2. **`python tools/brief.py`** — the plan's status, one line per milestone and per module, the guard tests,
   what is on disk. Step 1 of every session, machine move or not ([AGENTS.md](../AGENTS.md)).
3. [docs/agent/handoff.md](agent/handoff.md) — where the work stands now, and the next group of slices with
   the file set they share. It is overwritten each session, so it is state rather than history.
4. `python tools/loop.py` if the unattended loop is what runs next; its design is
   [docs/agent/coordinator.md](agent/coordinator.md). It is the one thing here that needs a `claude` on
   `PATH` (§ *Every platform*), and the only step above will not have caught its absence.

None of that is machine-specific, which is the point: the handover is the repository.
