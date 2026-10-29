# The published images

`Dockerfile` packages the release binaries into `ghcr.io/novis-lang/novis`. What a user does with
an image is the website's Docker guide,
[website/src/content/docs/guides/production/docker.mdx](../website/src/content/docs/guides/production/docker.mdx).
The release procedure is [docs/release.md](../docs/release.md). Every push to `main` whose CI run is
green also publishes `:dev` and `:dev-debian`, from that push's release binaries. The dev build,
`dev-build.yml` on the `landing` branch, runs after CI, and
[image.yml](../.github/workflows/image.yml) builds both kinds of image.

## Building an image locally

The Dockerfile compiles nothing — it packages a binary that already exists. Point it at one:

```sh
mkdir -p docker/bin/amd64 docker/bin/notices
cp target/x86_64-unknown-linux-gnu/release/nvs docker/bin/amd64/nvs
cp LICENSE THIRD-PARTY-LICENSES.txt README.md CHANGELOG.md docker/bin/notices/

docker build docker --target distroless --platform linux/amd64 -t novis:dev
bash tests/docker/smoke.sh novis:dev
```

`--target debian` builds the other variant. `docker/bin/` is ignored by git; the release workflow
fills it from the published archives, which is what makes the image's bytes and the archive's the
same bytes. `tests/docker/smoke.sh` runs the image the ways the guide describes and exits non-zero
when one of them fails. The release runs it on each variant before it pushes, and CI runs it when
`docker/` changes.

**Build the binary against glibc 2.41 or older.** Both images are Debian trixie, and a binary
linked against a newer glibc does not start in them; the error names a missing `GLIBC_` version.
The release builds on Ubuntu 22.04, which links against 2.35. On a newer host, or on Windows, the
`build` stage of [benches/proxied/Dockerfile](../benches/proxied/Dockerfile) compiles one in a
trixie container, with the toolchain pinned to `rust-toolchain.toml`:

```sh
docker build -f benches/proxied/Dockerfile --target build -t novis-build .
docker create --name novis-build novis-build
docker cp novis-build:/out/nvs docker/bin/amd64/nvs
docker rm novis-build
```

A release binary also has its debug sections compressed. A local build skips that step, so its
image is larger. The size is the only difference.

## Why not Alpine or `scratch`

A ~15 MB static image is tempting and would break three things, each quietly:

- **`jiff` reads `/usr/share/zoneinfo` on Unix** and bundles the IANA database only on Windows. A
  `scratch` image has no tzdb, so every named zone — `America/New_York` — fails at runtime while
  everything else works.
- **DNS goes through `std::net::ToSocketAddrs`**, which is `getaddrinfo`. A musl build resolves
  with musl's own resolver instead, which is not the one this project tests against.
- **The musl build leg is `optional: true`** in the release matrix, because `ring` and `libsqlite3-sys`
  compile C that needs `musl-gcc`. Shipping an image from a leg that is allowed to fail would
  make the image the least reliable artifact of the release.

`distroless/cc` gets most of the size benefit — no shell, no package manager, a third of the
`-debian` variant — with none of that. No CA bundle is needed either: `nvs_host::tls` verifies
against Mozilla's set compiled into the binary (`webpki-roots`), so neither image installs
`ca-certificates`.
