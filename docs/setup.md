# Setting up a machine

What a machine needs before `python tools/verify.py` or the acceptance run mean anything. One-time, per
machine. How the repo is *driven* once it is set up — `verify.py`, `splice.py`, WSL one-liners, valgrind,
fuzzing — is [docs/agent/commands.md](agent/commands.md); this file is only the install list.

**On Windows, WSL is not optional and PHP is installed twice, at the same version.** That is the pair a new
machine gets wrong. The rest is a Rust toolchain that installs itself.

## Every platform

| Need | Why | Check |
|---|---|---|
| Rust, the version pinned in [rust-toolchain.toml](../rust-toolchain.toml) | `rustup` installs it on the first `cargo` command inside the tree — nothing to do by hand. Never a different channel: the pin is what makes three platforms the same compiler. | `cargo --version` |
| Python 3.11+ | Everything in `tools/`. No third-party package is ever required. | `python --version` |
| PHP on `PATH`, at the version in [the plan](implementation-plan.md)'s status block § *Toolchain* — that field is the version's one home, and it reads 8.5 today | The differential oracle. A `tests/differential/` case runs its `--ORACLE--` twin under real PHP and compares stdout, so a machine without it **skips** those cases instead of failing them. It is also the fastest way to settle a semantics question while authoring: `php -r '…'`. | `php -v` |
| Node.js 20 LTS or newer, with `npm` — **from M4B onward** | `editors/vscode` is TypeScript, and its headless tests — the TextMate grammar snapshots and the LSP protocol round-trip against the real `mwl lsp` binary — are acceptance checks. Without Node they do not fail, they cannot run. Only the machine's native side needs it: those checks run once, not once per leg, so the WSL distro does not. | `node --version`, `npm --version` |
| Bun — **optional, benchmarks only** | The fourth engine in [benches/userland/](../benches/userland/), which runs the `.ts` twin of every case ([ADR 0100](adr/0100-against-python-mwl-claims-the-tool-that-gets-handed-over.md) § 5). Nothing else in the tree reads it: without Bun, run `python tools/bench.py --engines mwl,php,python` and the suite is otherwise unchanged. No build, test or loop session touches it. | `bun --version` |
| VS Code — **from M4B onward** | Two different things. `@vscode/test-electron` downloads its **own** pinned build into `editors/vscode/.vscode-test/` for the extension-host tier, so a system install is not what that test runs against; the system install is what you drive the extension in by hand, which is the entire point of pulling M4B ahead of M10. Fetch the test build once (below) and nothing afterwards touches the network. | `code --version` |

MWL generates native code, so "it compiles here" is a weaker claim in this repository than in most. CI
builds all three supported targets — `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`,
`aarch64-apple-darwin` — and a developer machine is expected to cover two of them (see below).

## Windows

The primary development platform, and the only one with real setup:

1. **The MSVC C++ build tools** — the linker. The `x86_64-pc-windows-msvc` target has no other.
2. **PHP on the Windows `PATH`.**
3. **WSL, with a second full toolchain inside it.** Required, not a convenience: three things have no
   Windows story at all — `valgrind`/`callgrind` (no native build,
   [ADR 0026](adr/0026-performance-measurement-methodology.md)), `cargo-fuzz` (needs libFuzzer), and the
   acceptance run's second leg, which exists because a JIT is exactly where a calling-convention
   divergence between two targets hides.

One-time setup inside the distro, which mounts the repo at `/mnt/<drive>/<repo>`:

```sh
sudo apt-get update && sudo apt-get install -y build-essential clang valgrind
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
rustup toolchain install nightly
cargo install cargo-fuzz --locked
```

### PHP goes in the distro too, at the same version

The oracle's whole value is that a difference in output is attributable to **MWL**. If Windows runs 8.5 and
the distro runs 8.3, a differential case that passes on one leg and fails on the other says nothing about
MWL's code generation — which is the only question the second leg exists to answer. So both sides carry
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

## Proving the machine is set up

```sh
python tools/verify.py                                   # build, test, clippy, fmt, the extension
cargo run -q -p mwl-cli -- test tests/differential/       # must report 0 skipped
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
