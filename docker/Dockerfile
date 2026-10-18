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
# release archives. Building this by hand needs the same layout; `docker/README.md` § *Building an
# image locally* has the commands.
#
# Two runtime bases, two stages, one binary. The website's Docker guide
# (`website/src/content/docs/guides/production/docker.mdx` § *Two variants*) tells a user which
# to take: `distroless` is the default because it has no shell to
# reach if a request ever escapes the runtime, and `debian` exists because sometimes you have to
# get inside a container to find out what is wrong.

ARG DISTROLESS_BASE=gcr.io/distroless/cc-debian13:nonroot
ARG DEBIAN_BASE=debian:trixie-slim

# Neither base is pinned by digest yet, because a digest cannot be written down honestly until
# Dependabot is resolving it — `.github/dependabot.yml` has the `docker` ecosystem pointed here,
# so the first PR it opens is the one that adds them. A floating `:nonroot` is the safer of the
# two mistakes in the meantime: it takes the base's security updates, where a digest written by
# hand and never revisited stops taking them silently.

# ---------------------------------------------------------------------------------------------
# Stage `nvsdata` — one empty directory, and nothing else.
# ---------------------------------------------------------------------------------------------
# `nvs` keeps its compile cache, temporary files, editor stubs, default `nvs.toml` and a
# service's logs in the data folder, `.nvsdata` beside the binary — `/usr/local/bin/.nvsdata`
# in both images. `nonroot` cannot create a directory in `/usr/local/bin`, so the images ship it
# already made, owned by `nonroot` and `0700`, the mode `nvs` gives a data folder it creates
# itself. `WORKDIR` on `scratch` makes the directory without a `RUN`, so this stage executes
# nothing and needs no QEMU; the two runtime stages copy it with their own `--chown`, which
# resolves `nonroot` against their own `/etc/passwd`.
FROM scratch AS nvsdata
WORKDIR /.nvsdata

# ---------------------------------------------------------------------------------------------
# Stage `distroless` — the default image.
# ---------------------------------------------------------------------------------------------
# `cc` rather than `base` or `static`: a `x86_64-unknown-linux-gnu` build is dynamically linked
# against glibc and unwinds through `libgcc_s`, and `distroless/base` carries neither. The
# `static` variant is for a musl binary, which this repository builds but marks optional — see
# `docker/README.md` § *Why not Alpine or scratch*.
#
# Three things this image needs that a `scratch` one would not have, all of them already inside
# `cc-debian13`:
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
# The licence and the third-party notices go beside every distributed binary, and an image is a
# distribution like the archives are.
COPY bin/notices/ /usr/share/doc/novis/
COPY --from=nvsdata --chown=nonroot:nonroot --chmod=0700 /.nvsdata /usr/local/bin/.nvsdata

# Where a user mounts their application. `nvs` looks for `./nvs.toml` here
# (`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 2),
# so a mounted tree's own configuration is found with nothing passed on the command line, and a
# relative path in the user's command resolves against the mount.
WORKDIR /app

# Documentation, not a rule — it publishes nothing by itself. 8000 is `rule:http-server/the-server-block-is-boot-class`'s default
# port, kept so the container's port and the host's read the same.
EXPOSE 8000

# uid 65532, the `nonroot` account this base ships. Nothing in Novis wants root, and 8000 is
# above the privileged range, so there is no step that needs to drop privileges later.
USER nonroot:nonroot

# `nvs serve` handles `SIGTERM` itself, also as pid 1: it accepts no new connections, finishes
# the requests that are running, and exits. `[server] drain_timeout` (30s by default) bounds that
# wait, so give the container a stop timeout a little longer than it. The website's Docker guide
# § *Stop a container* has the settings.
STOPSIGNAL SIGTERM

# One binary, so one entry point, and every subcommand is reachable through it:
# `docker run … novis:X.Y.Z run script.nvs`, `… test tests/`, `… --version`.
ENTRYPOINT ["nvs"]

# The image runs nothing of its own. The user's command picks `serve` or `run` and every option,
# and an empty `docker run` prints the help and exits 0. `--help` rather than no `CMD`, because a
# bare `nvs` prints the same text and exits 2, which Compose and CI read as a failure. A `serve`
# command needs `--listen 0.0.0.0:8000`: `rule:http-server/the-server-block-is-boot-class`'s default is `127.0.0.1:8000`, the
# container's own loopback, which a published port never reaches. The website's Docker guide says
# so in every serve example.
CMD ["--help"]

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
COPY --from=nvsdata --chown=nonroot:nonroot --chmod=0700 /.nvsdata /usr/local/bin/.nvsdata

WORKDIR /app
EXPOSE 8000
USER nonroot:nonroot
STOPSIGNAL SIGTERM
ENTRYPOINT ["nvs"]
CMD ["--help"]

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
