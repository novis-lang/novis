A package archive carries every `.nvs` file under `src/`, `package.toml`, the license and readme files
at the package root, and exactly the files `package.toml`'s `files` list names, and nothing else.

- **The root files** are those whose name, ignoring case, is `README` or `NOTICE` with any extension
  or none, or starts with `LICENSE` or `COPYING` (`LICENSE-MIT`, `COPYING.txt`). Only the package
  root is searched. **A package with no license file is refused** by `nvs pkg pack` and at publish.
- **`files`** is a list of exact paths relative to the package root: no glob, no directory. Each must
  be a regular file at the commit, or the pack fails naming it. A path ending in `.nvs`, and a path the
  default set already carries, is refused, so every line of code arrives through the autoload root,
  where `rule:packaging/a-package-file-stays-under-its-declared-prefix` checks it. A `.nvsx` component,
  a font, a template or a data table is listed here.
- **The limits:** the files' sizes add up to at most **16 MiB**, and the archive holds at most
  **5,000** files. Both are constants of the archive format, shared by `nvs pkg pack`, the registry's
  build and the fetch, and neither has a setting. A file too large for them is an external download
  (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`).
- **Every path works on every platform.** It is UTF-8 in NFC, at most 160 bytes, and no two paths are
  equal ignoring case. No component is a name Windows reserves (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to
  `COM9`, `LPT1` to `LPT9`, with any extension), contains a control character or one of `<>:"\|?*`, or
  ends in a dot or a space. 160 bytes leaves room under the Windows path limit for the folder a package
  is fetched into.
- **`nvs pkg publish --dry-run` and `nvs pkg pack` print every file with its size, and the total,** so
  the publisher sees exactly what ships before it ships.
- **A package reads its own files without a grant.** Code under a package's namespace may read, with
  any `Core` door that only reads, exactly the files its archive carries and its `[[download]]`
  entries placed (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`). The path is
  canonicalized and then compared with that set, so `..` and a symlink do not reach past it
  (`rule:security/path-scope-canonicalise-then-prefix`). Writing, listing a folder, and reading any
  other file, including another package's, still need a grant line
  (`rule:security/package-authority-is-granted-one-line-at-a-time`).

An allowlist fails closed. With an ignore list, every new file in a repository is published until
somebody remembers to exclude it, which is how keys, `.env` files and database dumps reach a registry,
where they can never be deleted. With an allowlist, a new file stays out until a person names it.
Documentation, tests, examples, CI files and agent instruction files are for the people who work on a
package, not for the program that runs it. The license file is the exception the law makes: MIT and
BSD require their text with every copy, and Apache-2.0 also its `NOTICE`. The readme is the one other
exception, because it is small and a vendored or offline copy then says what it is.

The read gives a package nothing it does not already have. The digest or the hash fixes those bytes,
and the package could carry the same bytes as a string constant in its source. A grant for it would
name a folder that changes with every version, and an application author asked for one on every
package with a font or a template learns to grant broad file access, which is the habit the grant
lines exist to prevent. The set is the listed files and not the package's folder, because an
application may put its own files there.

The limits are counted on the files, not on a compressed archive: compression is the transport's
(`rule:packaging/the-registry-builds-the-archive-from-git`), source compresses five to ten times while
a font or an image barely compresses, and a publisher predicts the exact figure locally. 16 MiB is
several times a large framework's source. A limit can be raised but never lowered, because a published
version is immutable (`rule:packaging/publishing-never-overwrites`): raising one ships in a client
release first, and an older client refuses a larger archive with a message naming the update.

**The fetch enforces the same limits while it reads.** It stops at one byte past the largest archive
the two limits allow, before the digest is checked, so a hostile mirror cannot fill a disk or memory
with an archive the digest would have refused afterwards.

**Memory:** the pack and the fetch stream the archive to and from disk, so memory does not grow with
it. The read check holds one set of paths per fetched package for the life of the process, and
nothing per request.

**Declined:**
- **An ignore list** (`.npmignore`, `exclude`). It fails open.
- **A glob or a directory in `files`.** Either picks up whatever is added to the repository later, which
  is the ignore list again.
- **Every `.md` file by default.** It ships `docs/` trees, contributor guides and agent instruction
  files to every installer, and a markdown file under `tests/` with them.
- **A changelog in the default set.** It grows with every release, so every version would ship the
  whole history again. The package page links the commit each version was built from.
- **A limit on the compressed size.** It gives source a hidden five- to tenfold allowance over assets,
  and no publisher can predict it exactly.
- **A grant line for a package to read its own files.** It is friction with no security in it, and
  it trains authors to grant more than that.
- **The whole package folder readable without a grant.** An application's own files placed there
  would become readable by the package.
- **A larger limit raised per package on request.** It needs a person to decide each request, and the
  operator only withholds (`rule:packaging/the-operator-only-withholds`).

`nvs pkg pack` builds the set, checks every path and both limits, and prints what it packs
(`crates/nvs-cli/src/package/pack.rs`, whose `MAX_BYTES` and `MAX_FILES` are the two constants).
It refuses a symbolic link anywhere it reads, since a link would make the set depend on what lies
outside the package.

**Not on disk.** The fetch does not yet stop at the limits while it reads, and there is no read
check for a package's own files.
