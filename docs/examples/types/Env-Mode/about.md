Which deployment your program is running in. There are two modes and there is no third.

`Production` is the careful one: an error shows a short message rather than its details, the log is
written as JSON at info level, and requests are not logged one by one. `Development` is the other
end, and you ask for it in `nvs.toml`. A program that has configured nothing is in `Production`, so
a server where nobody wrote the line is still the careful one.

A mode is only a shorthand for the starting value of those five settings, and each of them stays
settable on its own. A host that wants production with one thing changed writes that one line
instead of reaching for the other mode.

**Good to know:** the mode is never taken from an environment variable, and nothing guesses it from
a hostname or from how the program was built. Staging is not a third mode — a staging server runs in
`Production` with its own settings.
