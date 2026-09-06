`Core\IO::within` — the launderer, and the one place the base directory is known — refuses a
component that

- is a Windows reserved device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`),
  with or without an extension;
- ends in a dot or a space;
- contains a colon;
- matches the 8.3 short-name pattern (`NAME~digit`).

**On every platform, not only Windows.** A component's spelling and its resolution come apart on
Windows by design: `foo.` opens `foo`, `CON` is a device in every directory, `file.txt::$DATA` opens
the default stream, and `PROGRA~1` aliases a long name — so a component can pass a lexical
containment check and resolve outside the base. A rule that fires on one host and not another means
a suite that passes in CI and fails in production, and the developer writing the check is usually on
the host where it silently does nothing.

The cost is stated plainly: a POSIX file named `foo.` or `a:b` is unreachable *through the
launderer*. It stays reachable by an absolute path the program supplies itself, which is not
laundered and never was.

`Core\Path` is untouched by this. It stays pure string algebra, touches no disk, carries no policy,
and goes on accepting both separators on every platform. `Path::normalize` is **not** a launderer,
because the base is not part of its input.
