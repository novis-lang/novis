The operating system your program runs on.

`Core\Env::Platform` is declared in the class `Core\Env`. The constant `Core\Env::OS` is one of its
cases.

The cases are `Windows`, `Darwin` (macOS), `Linux`, `BSD`, `Solaris` and `Unknown`. The value of
`Core\Env::OS` is fixed when Novis is built, so it never changes while your program runs.

Compare `Core\Env::OS` with a case, for example `Core\Env::OS == Core\Env::Platform::Windows`. A
misspelled case does not compile. To print the platform, use a `match` that gives a name for each
case.
