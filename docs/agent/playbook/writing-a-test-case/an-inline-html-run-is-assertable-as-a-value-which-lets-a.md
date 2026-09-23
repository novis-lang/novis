- **An inline-HTML run is assertable as a *value*, which lets a case count one rather than read
  it.** A run is a statement and a closure's block body is a statement list, so
  `Core\Out::capture(fn (): void => { ?>text<?nvs })` lowers, and the `Core\Cli\Text` it answers
  takes `as string`. `Core\Str::compare($t as string, "text")` is the comparison — the only way to
  put a raw span and an `echo` of the same literal side by side. [until: reviewed 2026-09-06]
