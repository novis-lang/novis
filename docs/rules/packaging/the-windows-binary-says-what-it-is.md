`nvs.exe` is linked with an icon and with version information, so Explorer, the taskbar, Task Manager,
a file's *Properties → Details* and the firewall prompt `nvs serve` raises all name the program as Novis
and say which build it is. A release build carries `website/media/novis-logo.png`; a debug build carries
`novis-logo-file-icon-light-theme-nvst.svg`, the test file's mark, and describes itself as
`Novis (debug build)` with `VS_FF_DEBUG` set, so the two are told apart in a folder and in a process
list without running either.

Every string is a fact the tree already states, read at build time and never written a second time:

| Field | Value | From |
|---|---|---|
| `ProductName`, `FileDescription` | `Novis` | the name `nvs info` heads its report with |
| `FileVersion` and both numeric versions | `<version>` | `[workspace.package]`'s `version` |
| `ProductVersion` | `<version>+<commit>`, `-dirty` included | the same commit `rule:packaging/a-build-records-no-timestamp` records |
| `CompanyName` | the authors | `[workspace.package]`'s `authors` |
| `LegalCopyright` | `LICENSE`'s copyright line, and the license's name | `LICENSE`, `license` |
| `Comments` | the project's description and its homepage | `[workspace.package]` |
| `InternalName`, `OriginalFilename` | `nvs`, `nvs.exe` | the `[[bin]]` name |

`VS_FF_PRERELEASE` is set when the version has a pre-release tag. The date fields are zero, which is
`rule:packaging/a-build-records-no-timestamp` again: two builds of one commit are the same bytes.

`crates/nvs-cli/build/winres.rs` writes the compiled resource file itself and `build.rs` hands it to the
linker, so the build gains no dependency and runs no resource compiler. The MSVC linker is the one that
takes such a file, and both Windows release targets are MSVC; a GNU-targeted build links without the
resource and is otherwise the same binary. The two `.ico` files under `crates/nvs-cli/assets/` are
committed, and `bun nv exe-icons` cuts them again from the drawings when one changes. A
resource that cannot be written is a build warning and a binary without one, never a failed build.

A program built with `nvs build --compile` is a copy of the host, and on Windows it says what *it* is
when a `package.toml` is found from the working directory, as every package command finds it:

| Field | Value |
|---|---|
| `ProductName`, `FileDescription` | the package's name, else the executable's file stem |
| `FileVersion`, `ProductVersion` and both numeric versions | the package's `version`, else `0.0.0` |
| `InternalName`, `OriginalFilename` | the executable's file stem and file name |
| the icon | the `.ico` file `[bundle] icon` names, else the host's |

Without a `package.toml` the copy carries the host's icon and version information unchanged, the way it
carries the host's notice. An icon that does not read or is not an `.ico` file fails the build. The other
resources, the message table and the linker's manifest, are the host's.

`crates/nvs-cli/src/bundle/resource.rs` writes it. It reads the host's resources with `object`, which the
tree already has, builds the icon and the version block with `build/winres.rs`'s own layouts, and writes
the whole resource tree into a new section after the image's last one. The resource data directory names
the new section, and the old one stays in the file, unread, under the name `.oldrsrc`. A host image with
no room in its headers for one more section, or with a resource named by a string, keeps its own resource,
with a warning. Any change to the image breaks an Authenticode signature, so the certificate table is
dropped, as appending the payload already breaks it.

**Memory:** the icon's bytes, a few tens of kilobytes of data no request path reads, spent on the
program being recognisable to the person and the operating system running it. A bundle also carries the
host's resources twice, a few tens of kilobytes more, and nothing of it is read when the bundle starts.

**Declined:**
- **A build dependency that drives the resource compiler**, such as `winresource` or `embed-resource`.
  It adds a crate to a build script that declines dependencies on supply-chain grounds, and a lookup of
  `rc.exe` or `windres` that can fail a build.
- **A crate that edits a PE file's resources for the bundle**, such as `editpe`. It is a new supply
  chain for four layouts this tree already writes and tests, and a crate that calls Windows's
  `UpdateResource` would not run on the other build hosts.
- **Rewriting the old resource section in place.** A new tree larger than the old one would move every
  section after it, `.reloc` among them, and every address those sections hold.
- **A checked-in `.res` file compiled by hand.** The version and the commit would be frozen in a binary
  file nobody regenerates.
- **An application manifest in the same resource.** The linker already embeds its default manifest, a
  second one is a duplicate-resource error, and what a manifest changes, such as the active code page
  or long paths, is behaviour and needs a rule of its own.
- **The release icon on both builds.** The debug binary is the one a contributor runs by accident when
  measuring, and the icon is the cheapest place to say so.
