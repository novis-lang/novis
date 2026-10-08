# syntax=docker/dockerfile:1.7
#
# The published Novis images. **Nothing is compiled here.** The binary this file copies is the
# one `release.yml`'s build matrix produced, attested and uploaded, so the bytes in
# `ghcr.io/novis-lang/novis:X.Y.Z` are the bytes in `nvs-X.Y.Z-linux-x86_64.tar.gz` and one
# `gh attestation verify` covers both. Compiling a second time inside a Docker build would
# produce a *different* binary that nothing has checked against the one users download, and it
# would pay for two cross-compiles under emulation to do it.
#
# The build context is `docker/`, and `release.yml`'s `stage the release binaries` step is what
# fills `bin/` in it — `bin/amd64/nvs`, `bin/arm64/nvs` and the notices, unpacked from the
# release archives. Building this by hand needs the same layout; `docs/docker.md` § *Building an
# image locally* has the two commands.
#
# Two runtime bases, two stages, one binary. `docs/docker.md` § *Which image* owns the choice
# between them; the short version is that `distroless` is the default because it has no shell to
# reach if a request ever escapes the runtime, and `debian` exists because sometimes you have to
# get inside a container to find out what is wrong.

ARG DISTROLESS_BASE=gcr.io/distroless/cc-debian12:nonroot
ARG DEBIAN_BASE=debian:bookworm-slim

# Neither base is pinned by digest yet, because a digest cannot be written down honestly until
# Dependabot is resolving it — `.github/dependabot.yml` has the `docker` ecosystem pointed here,
# so the first PR it opens is the one that adds them. A floating `:nonroot` is the safer of the
# two mistakes in the meantime: it takes the base's security updates, where a digest written by
# hand and never revisited stops taking them silently.

# ---------------------------------------------------------------------------------------------
# Stage `distroless` — the default image.
# ---------------------------------------------------------------------------------------------
# `cc` rather than `base` or `static`: a `x86_64-unknown-linux-gnu` build is dynamically linked
# against glibc and unwinds through `libgcc_s`, and `distroless/base` carries neither. The
# `static` variant is for a musl binary, which this repository builds but marks optional — see
# `docs/docker.md` § *Why not Alpine or scratch*.
#
# Three things this image needs that a `scratch` one would not have, all of them already inside
# `cc-debian12`:
#
#   - **glibc**, for the reason above, and for DNS: every hostname Novis resolves goes through
#     `std::net::ToSocketAddrs`, which is `getaddrinfo`.
#   - **`/usr/share/zoneinfo`**, because `jiff` reads the system IANA database on Unix and
#     bundles one only on Windows. Without it every named zone — `Core\Time`'s `Zone`,
#     `America/New_York` — fails at runtime and nothing else in the image is wrong.
#   - **`/etc/passwd`**, so the `nonroot` user below is a real account rather than a bare uid.
#
# A CA bundle is *not* on that list, and `ca-certificates` is deliberately not installed
# anywhere in this file: `nvs_host::tls` verifies against Mozilla's set compiled into the binary
# (`webpki-roots`), so outbound TLS works in an image with no trust store at all. That is also
# why the `debian` stage below runs no `apt-get` — there is nothing it would install.
FROM ${DISTROLESS_BASE} AS distroless

# BuildKit fills this per platform, which is the whole reason one `docker buildx build` can emit
# a manifest list for two architectures without QEMU running a single foreign instruction: no
# stage here executes anything, it only copies.
ARG TARGETARCH

COPY --chmod=0755 bin/${TARGETARCH}/nvs /usr/local/bin/nvs
# ADR 0065 puts the licence and the third-party notices beside every distributed binary, and an
# image is a distribution like the archives are.
COPY bin/notices/ /usr/share/doc/novis/

# The convention the default command below rests on: mount your application at `/app`, with its
# entry point at `/app/main.nvs`. It is also where `nvs` looks for `./nvs.toml` (ADR 0103 § 1
# step 2), so a mounted tree's own configuration is found with nothing passed on the command
# line.
WORKDIR /app

# Documentation, not a rule — it publishes nothing by itself. 8000 is ADR 0097 § 5's default
# port, kept so the container's port and the host's read the same.
EXPOSE 8000

# uid 65532, the `nonroot` account this base ships. Nothing in Novis wants root, and 8000 is
# above the privileged range, so there is no step that needs to drop privileges later.
USER nonroot:nonroot

# **`nvs serve` installs no signal handler today**, and a container's main process is pid 1,
# where the kernel discards any signal whose disposition is still the default. So a `SIGTERM`
# from `docker stop` would be *ignored* — not "handled badly", ignored — and every stop would
# hang for the full 10-second timeout before Docker sent `SIGKILL` anyway.
#
# Asking for that `SIGKILL` immediately is the same ending without the wait. This is the honest
# state of the runtime rather than a preference: ADR 0097 § 5's `health_path` already answers
# `503` while draining, so the machinery is designed and unbuilt. **When it lands, this line becomes `SIGTERM`** and
# `docs/docker.md` § *Stopping a container* changes with it.
STOPSIGNAL SIGKILL

# One binary, so one entry point, and every subcommand is reachable through it:
# `docker run … novis:X.Y.Z run script.nvs`, `… test tests/`, `… --version`.
ENTRYPOINT ["nvs"]

# `--listen 0.0.0.0:8000` is load-bearing and is the one thing to keep when overriding this.
# ADR 0097 § 5's default is `127.0.0.1:8000`, which inside a container means the port is
# published, the container is running, and every connection is refused — the failure looks like
# a networking problem and is a loopback bind. `docs/docker.md` says so where the override is
# offered.
CMD ["serve", "/app/main.nvs", "--listen", "0.0.0.0:8000"]

# ---------------------------------------------------------------------------------------------
# Stage `debian` — the `-debian` tag: the same binary, in an image you can get inside.
# ---------------------------------------------------------------------------------------------
# Identical in every respect except the base, on purpose: a debugging image that differs from
# the one in production in some second way is evidence about the wrong container. The whole
# difference is that this one has a shell, `apt`, and the ~90 packages that come with them.
FROM ${DEBIAN_BASE} AS debian

ARG TARGETARCH

# The one `RUN` in this file, and the only reason the build needs QEMU for a foreign
# architecture. A fixed uid rather than whatever `adduser` picks next, so a bind-mounted volume
# has one owner to chown to across rebuilds and across both images — 65532 is the uid the
# distroless stage's `nonroot` already is.
RUN groupadd --gid 65532 nonroot \
 && useradd --uid 65532 --gid 65532 --home-dir /home/nonroot --create-home --shell /usr/sbin/nologin nonroot

COPY --chmod=0755 bin/${TARGETARCH}/nvs /usr/local/bin/nvs
COPY bin/notices/ /usr/share/doc/novis/

WORKDIR /app
EXPOSE 8000
USER nonroot:nonroot
STOPSIGNAL SIGKILL
ENTRYPOINT ["nvs"]
CMD ["serve", "/app/main.nvs", "--listen", "0.0.0.0:8000"]

# ---------------------------------------------------------------------------------------------
# There is deliberately no `LABEL` block in this file.
# ---------------------------------------------------------------------------------------------
# An instruction belongs to the stage it follows, so a block written once at the foot would
# label `debian` and never `distroless`, and writing it twice puts the same nine facts in two
# places that can disagree. `release.yml` passes the full OCI set with `--label` instead, from
# `docker/metadata-action`, which derives them from the repository itself — so the source URL
# and the licence come from the repository rather than from a string typed here that would go
# stale on the move to `novis-lang/novis`. A hand build gets no labels, which is correct: a
# local image is not a published one.
