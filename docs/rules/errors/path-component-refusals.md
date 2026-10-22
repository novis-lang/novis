`Core\IO::within` — the launderer, and the one place the base directory is known — refuses a
component that

- is a Windows reserved device name (`CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`,
  `COM1`–`COM9`, `LPT1`–`LPT9`, and `COM¹`–`COM³` and `LPT¹`–`LPT³`), in any case, with or without
  an extension, a trailing dot or space, or a stream;
- ends in a dot or a space;
- contains a colon;
- matches the 8.3 short-name pattern (`NAME~digit`).

**A device name is refused in more places than the launderer.** Every file door refuses a path with
one in any component, with a `RuntimeError` before any grant is read, whatever the grant covers.
Static serving declines a segment that is one, and a mount's capture that is one is a boot error.
All of them ask `nvs_config::capability::is_reserved_device`. On Windows such a name opens a device
in every folder, so a granted folder does not contain what it opens, and a read of `CONIN$` waits
for a key forever.

**So is every alias, by its text, before anything opens the path.** Every file door refuses a path
that starts with two separators (`\\host\share`, `\\?\`, `\\.\`, `//host`), has a `:` after the
drive letter, or has a component that ends in a dot or a space or is an 8.3 short name, with a
`RuntimeError` before any grant is read. The grant's check canonicalizes the path, and on Windows
canonicalizing `\\host\share\x` connects to `host`, sends the server account's credentials, and
holds the core for as long as the network takes. `nvs_config::capability::windows_alias` is the
test, and `nvs_config::capability::resolved` answers nothing for a path with two separators
first, so no caller that skips the door opens one either. An extension's `wasi:filesystem`
refuses the same components and the device names, on every platform. A short name the
configuration writes, such as a temporary folder under `C:\Users\RUNNER~1`, is resolved to its
long name once and never reaches a door as written.

**On every platform, not only Windows.** A component's spelling and its resolution come apart on
Windows by design: `foo.` opens `foo`, `CON` is a device in every directory, `file.txt::$DATA` opens
the default stream, and `PROGRA~1` aliases a long name — so a component can pass a lexical
containment check and resolve outside the base. A rule that fires on one host and not another means
a suite that passes in CI and fails in production, and the developer writing the check is usually on
the host where it silently does nothing.

The cost is stated plainly: a POSIX file named `foo.`, `a:b` or `v~2`, and a share mounted at a
`//` path, is unreachable from a program on any platform. A Windows share is reached through a
drive letter mapped to it, which the operator writes in the grant. The check is one scan of the
path per door call and no system call.

`Core\Path` is untouched by this. It stays pure string algebra, touches no disk, carries no policy,
and goes on accepting both separators on every platform. `Path::normalize` is **not** a launderer,
because the base is not part of its input.

**Declined:**

- **Enforcing these refusals only on Windows.** A platform-conditional security rule is tested on the
  platform where it does nothing, and conformance runs on a native Windows leg and a WSL leg, so one
  member would need two expectations.
- **Putting them in `Core\Path::normalize`.** No `Path` member carries policy, and putting policy there
  invites the belief that normalising is safe. Containment needs the base, and only `Core\IO::within`
  has it.
