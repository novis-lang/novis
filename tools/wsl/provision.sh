#!/usr/bin/env bash
# Turns a fresh Ubuntu 26.04 WSL distro into this repository's test distro, or brings one up to date.
#
# `bun nv wsl --setup` runs it twice: `provision.sh system` as root, then `provision.sh user` as the
# distro's own user. Do not run it by hand. The versions come from the environment, and
# `tools/nv/cmd/wsl.ts` reads each one from the file CI reads it from:
#
#   NVS_REPO        the repository as the distro sees it, such as /mnt/e/src/novis
#   NVS_USER        the user the distro runs as
#   NVS_NODE_MAJOR  Node's major version, from editors/vscode/.nvmrc
#   NVS_BUN         Bun's version, from package.json's packageManager
#   NVS_NEXTEST     cargo-nextest's version, from ci.yml
#   NVS_FUZZ        cargo-fuzz's version, from ci.yml
#
# Rust's stable toolchain is the one rust-toolchain.toml pins, and nightly is the latest, as in CI.
# Every step checks what is there first, so running the script again only installs what changed.
set -euo pipefail

mode=${1:?usage: provision.sh system|user}
: "${NVS_REPO:?}" "${NVS_USER:?}"

say() { printf 'provision: %s\n' "$*"; }

system() {
  : "${NVS_NODE_MAJOR:?}"
  export DEBIAN_FRONTEND=noninteractive
  say "apt packages"
  apt-get update -q
  apt-get install -y -q --no-install-recommends \
    build-essential clang llvm pkg-config cmake nasm python3 git curl ca-certificates \
    xz-utils unzip zstd valgrind sudo
  apt-get autoremove -y -q

  if [[ "$(node --version 2>/dev/null || true)" != "v${NVS_NODE_MAJOR}."* ]]; then
    say "Node ${NVS_NODE_MAJOR} from nodejs.org"
    local base="https://nodejs.org/dist/latest-v${NVS_NODE_MAJOR}.x"
    local sums file
    sums=$(curl -fsSL "$base/SHASUMS256.txt")
    file=$(awk '/linux-x64\.tar\.xz$/ { print $2 }' <<<"$sums")
    curl -fsSL -o "/tmp/$file" "$base/$file"
    (cd /tmp && grep " $file\$" <<<"$sums" | sha256sum --check --quiet -)
    tar -xJf "/tmp/$file" -C /usr/local --strip-components=1 --no-same-owner \
      --exclude=CHANGELOG.md --exclude=README.md --exclude=LICENSE
    rm -f "/tmp/$file"
  fi

  if ! id -u "$NVS_USER" >/dev/null 2>&1; then
    say "user $NVS_USER"
    useradd --create-home --shell /bin/bash --groups sudo "$NVS_USER"
  fi
  # Passwordless sudo, as on CI's runners. Playwright's `--with-deps` installs packages with it.
  printf '%s ALL=(ALL) NOPASSWD:ALL\n' "$NVS_USER" >/etc/sudoers.d/novis-test
  chmod 0440 /etc/sudoers.d/novis-test

  # systemd as on CI's runners. Windows' PATH stays out, so a Linux tool that is missing fails
  # instead of a Windows program of the same name running in its place.
  cat >/etc/wsl.conf <<EOF
[boot]
systemd=true

[user]
default=$NVS_USER

[interop]
appendWindowsPath=false
EOF

  # A test that holds a hundred thousand tasks needs more mappings than the default allows.
  printf 'vm.max_map_count=262144\n' >/etc/sysctl.d/99-novis.conf
  sysctl -q -w vm.max_map_count=262144 || true

  # The marker `bun nv wsl` checks before it deletes the distro, and `health` checks before each run.
  printf 'made by bun nv wsl --setup for %s\n' "$NVS_REPO" >/etc/novis-test-distro
}

user() {
  : "${NVS_BUN:?}" "${NVS_NEXTEST:?}" "${NVS_FUZZ:?}"
  export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"

  # A login shell reads ~/.profile, and every command bun nv runs here is a login shell.
  local mark="# novis-test paths"
  if ! grep -qF "$mark" "$HOME/.profile" 2>/dev/null; then
    printf '\n%s\nexport BUN_INSTALL="$HOME/.bun"\nexport PATH="$HOME/.cargo/bin:$BUN_INSTALL/bin:$PATH"\n' "$mark" >>"$HOME/.profile"
  fi

  if ! command -v rustup >/dev/null; then
    say "rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --default-toolchain none --profile minimal
  fi
  rustup self update || true
  say "Rust as rust-toolchain.toml pins it"
  local pinned
  pinned=$(cd "$NVS_REPO" && rustup toolchain install >/dev/null && rustup show active-toolchain | cut -d' ' -f1)
  rustup default "$pinned"
  say "nightly with Miri"
  rustup toolchain install nightly --profile minimal --component miri,rust-src
  rustup update nightly --no-self-update
  cargo +nightly miri setup

  if [[ "$(cargo nextest --version 2>/dev/null || true)" != *" $NVS_NEXTEST"* ]]; then
    say "cargo-nextest $NVS_NEXTEST"
    cargo install "cargo-nextest@$NVS_NEXTEST" --locked
  fi
  if [[ "$(cargo fuzz --version 2>/dev/null || true)" != *" $NVS_FUZZ"* ]]; then
    say "cargo-fuzz $NVS_FUZZ"
    cargo install cargo-fuzz --version "$NVS_FUZZ" --locked
  fi
  if [[ "$(bun --version 2>/dev/null || true)" != "$NVS_BUN" ]]; then
    say "Bun $NVS_BUN"
    curl -fsSL https://bun.sh/install | bash -s "bun-v$NVS_BUN" >/dev/null
  fi
}

case "$mode" in
  system) system ;;
  user) user ;;
  *) echo "usage: provision.sh system|user" >&2; exit 2 ;;
esac
say "$mode done"
