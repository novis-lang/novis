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

A program built with `nvs build --compile` is a copy of the host, so on Windows it carries this icon
and this version information too, the way it already carries the host's notice.

The cost is the icon's bytes, a few tens of kilobytes of data no request path reads: priority 5, spent
on the program being recognisable to the person and the operating system running it.
