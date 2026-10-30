A package names each file too large for its archive in a `[[download]]` table of its `package.toml`,
pinned by HTTPS URLs, a SHA-256 hash and a size, and `nvs pkg fetch` downloads it, checks it and places
it at its path in the package, running nothing.

```toml
[[download]]
path    = "fonts/NotoSansCJK-Regular.otf"
urls    = ["https://example.com/fonts/NotoSansCJK-Regular.otf",
           "https://mirror.example.org/NotoSansCJK-Regular.otf"]
sha256  = "…"
size    = 16384000
license = "OFL-1.1"
```

- **The package's digest covers each entry,** because `package.toml` is in the archive
  (`rule:packaging/a-package-is-its-digest`). A host can withhold a file, never change it, and the
  registry neither downloads nor stores one.
- **`nvs pkg download <url> --to <path>`** fetches the file, computes its hash and size, and writes the
  entry. `nvs pkg publish` and its `--dry-run` download every entry again and refuse one whose bytes do
  not match. The registry checks only the form: HTTPS URLs, a 64-digit hash, a size above zero, a
  `license` that is an SPDX identifier when present, and a path that follows the archive's path rules,
  does not end in `.nvs`, is not under `src/` and is not a file the archive carries
  (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
- **Files are fetched with the package, never at run time.** `nvs pkg add`, `fetch`, `update` and
  `vendor` download them, and a build reaches no network.
- **Every download is HTTPS, and every redirect hop is checked against the outbound address table,**
  at most five hops, so a package cannot make an installer reach a private address or the cloud
  metadata service.
- **The declared size is a hard ceiling.** The read stops at one byte past it, the hash is checked
  before the file is renamed into place, and a mismatch tries the next URL. After the last one, the fetch
  fails naming the package, the path and what each URL returned.
- **Nothing is unpacked and nothing runs** (`rule:packaging/nothing-runs-before-the-program`). A zip
  arrives as a zip, and the package's own code reads it, with no grant
  (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
- **A cache keyed by SHA-256** under the data folder, behind the ownership check the artifact cache
  uses, holds each file once across projects and versions.
- **`nvs pkg provide <file>`** puts a file obtained any other way into that cache, when its hash and
  size match an entry of a package in the lock, so a dead URL never strands a user.
- **Everything is shown.** `nvs pkg add` and `nvs pkg update` print each external file with its host,
  size and license, and the total. A host the lock did not hold before is printed as new, and the lock
  records each package's hosts for that comparison. The package page lists every entry.

SHA-256 rather than the BLAKE3 the archive uses: upstream projects publish SHA-256 checksums, and
`sha256sum` is on every system, so a maintainer or a user compares the entry with the upstream's own
list. A package that needs a large file has three other ways to get it, and each is worse: an install
script, which `rule:packaging/nothing-runs-before-the-program` forbids; a download at run time, which
puts a third-party host on the request path; or a manual step in a readme, which no hash checks.

The cost is availability. A version whose URLs all stop answering stays unfetchable until a user
provides the file or the maintainer publishes a patch with a new URL and the same hash. A third-party
host also sees the address of every installer, which is why the hosts are printed.

**Memory:** a download streams to disk, so memory does not grow with the file. Nothing is held per
request.

**Declined:**
- **The registry mirroring external files.** It would host bytes no commit holds and nobody reviewed,
  and its storage would grow with every large file any package names.
- **A size cap on an external file.** The declared size, the printed total and the new-host notice do
  that work, and a cap would bring back the manual step this rule removes.
- **Unpacking an archive at install.** It brings path traversal, compression bombs and links into the
  installer, which today only writes one file.
- **An `http:` URL.** The hash protects the bytes, but anybody on the network sees what an installer
  downloads and can replace it, so a fetch reads up to the full size before it learns the bytes are
  wrong.

`nvs pkg fetch`, `vendor`, `download` and `provide` and the cache are
`crates/nvs-cli/src/package/download.rs`, the report `report.rs` and the hosts `lock.rs`.
`nvs registry build` refuses a malformed `[[download]]` entry at publish, and the package page lists
each external file with its host, size, SHA-256 hash and license (`apps/registry/src/PackagePage.nvs`).
