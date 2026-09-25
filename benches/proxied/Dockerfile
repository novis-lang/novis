# syntax=docker/dockerfile:1.7
#
# The Novis origin for the proxied benchmark. `benches/proxied/README.md` owns every decision this
# file carries out; the two stages below are only how the binary gets in.
#
# The build context is the repository root, and `Dockerfile.dockerignore` beside this file is
# what keeps that affordable -- `target/` alone is tens of gigabytes, and sending it would cost more
# than the compile does. BuildKit reads a `<dockerfile>.dockerignore` in preference to the context's
# own, which is why the exclusion list is scoped to this build instead of landing at the repo root
# where every other tool would inherit it.

# --------------------------------------------------------------------------------------
# Stage `build` -- nvs for Linux, from this checkout.
# --------------------------------------------------------------------------------------
# Pinned to `rust-toolchain.toml`'s channel rather than to `latest`: a benchmark that silently
# compiled under a different rustc than CI does is reporting the compiler, not the tree. Bookworm on
# both halves because the runtime below is bookworm too -- a binary built against a newer glibc than
# the image it runs in fails at exec with a message that says nothing about why.
FROM rust:1.97.1-slim-bookworm AS build

# `ring` (via rustls) has a `cc` build script, so the slim image's missing toolchain is a hard
# failure rather than a fallback to something pure-Rust. `pkg-config` and `perl` are ring's other two
# documented build-time needs; installing them costs a layer and saves the afternoon spent reading a
# build-script error that names none of them.
RUN apt-get update \
 && apt-get install -y --no-install-recommends build-essential pkg-config perl \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . /src

# `.git/` is excluded from the context (it is large and nothing compiles from it), so `build.rs`
# would record the commit as `unknown` and the binary in the image would not say which source it is.
# `NVS_BUILD_COMMIT` is that build script's own override for exactly this case -- a source tree with
# no `.git` is a supported way to build Novis -- and `bun nv bench-proxied` passes the checkout's
# HEAD through it, so `nvs info` inside the container agrees with the record written outside it.
ARG NVS_BUILD_COMMIT=""
ENV NVS_BUILD_COMMIT=${NVS_BUILD_COMMIT}

# The cargo caches are BuildKit cache mounts rather than image layers, so a rebuild after a source
# edit is incremental and the image carries none of it. `sharing=locked` because two concurrent
# builds against one target directory is how a corrupt `.rlib` gets produced.
#
# The binary is copied OUT of the mount inside the same RUN: a cache mount is gone by the time the
# next instruction runs, so `COPY --from=build /src/target/release/nvs` would find nothing.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --release -p nvs-cli \
 && mkdir -p /out \
 && cp /src/target/release/nvs /out/nvs

# --------------------------------------------------------------------------------------
# Stage `source` -- the default runtime, carrying the binary the stage above built.
# --------------------------------------------------------------------------------------
FROM debian:bookworm-slim AS source

# `ca-certificates` is not needed by the `hello` case and is installed anyway: a case added later
# that reaches an origin would otherwise fail inside a benchmark, where a TLS error is the least
# legible place to discover a missing root store. `curl` is the healthcheck's, and it is here rather
# than in the compose file because the PHP image ships its own way to answer one and the two stacks
# must be declared ready by an equivalent test -- a stack that is called up before it can serve puts
# its first requests, and only its first requests, in a different server than the rest.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

COPY --from=build /out/nvs /usr/local/bin/nvs

# `/app` is the read-only mount of `benches/serve` the compose file supplies, and the working
# directory is it: `nvs serve` resolves `./nvs.toml` out of the working directory (ADR 0103 § 1
# step 2), and that directory deliberately holds none -- the repository's own `nvs.toml` is written
# around fixtures with database and capability blocks, and a benchmark inheriting them would be
# measuring a configuration no operator has.
WORKDIR /app
EXPOSE 8080
ENTRYPOINT ["nvs"]
CMD ["serve", "/app/hello.nvs", "--listen", "0.0.0.0:8080"]

# --------------------------------------------------------------------------------------
# Stage `prebuilt` -- the same runtime, taking a binary the host already built.
# --------------------------------------------------------------------------------------
# Selected by `bun nv bench-proxied --nvs-bin PATH`, which stages the file at
# `benches/proxied/.bin/nvs` and builds this target instead. It exists because the WSL leg already
# produces a Linux release binary on this machine (`docs/agent/commands.md` § *Fuzzing and callgrind
# on Windows*), and paying a second full release build to measure it is minutes per iteration for
# nothing.
#
# The driver checks the file is a Linux ELF and a release build before it stages it. Nothing here
# repeats that check -- a Dockerfile is the wrong place to discover that a debug binary is being
# benchmarked, because by then the image is built and the number is already wrong.
FROM debian:bookworm-slim AS prebuilt

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

COPY benches/proxied/.bin/nvs /usr/local/bin/nvs
RUN chmod +x /usr/local/bin/nvs

WORKDIR /app
EXPOSE 8080
ENTRYPOINT ["nvs"]
CMD ["serve", "/app/hello.nvs", "--listen", "0.0.0.0:8080"]
