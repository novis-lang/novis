Whether each standard stream is a terminal, the colour depth — honouring `NO_COLOR`,
`CLICOLOR_FORCE`, `FORCE_COLOR` and `TERM=dumb`, and enabling Windows virtual terminal processing
where it exists — and the width and height are **resolved once per process, not per call**.
`Cli::isTty(Cli\Stream)`, `Cli::width()`, `Cli::height()` and `Cli::colorDepth()` read that profile;
`80`/`24` is the answer when there is no controlling terminal. `Cli\Stream` is an enum of `In`, `Out`
and `Err`, because "is a tty" is always a question about one stream and *colour on stdout while stdin
is a pipe* is the common case a single `isTty()` cannot express.

A program never asks what the terminal supports. It writes `Cli\Text`, and styling degrades truecolor
→ 256 → 16 → none against the profile. **When the stream is not a terminal, styling is dropped
entirely**, so `myprog | grep` and a CI log are plain — while the control-byte substitution of
`rule:tooling/terminal-output-is-a-sink` still applies, because that is a safety rule and not a
presentation one. Resolving once is what makes two reads of the width in one run the same number by
construction, so a program that measures at the top and draws at the bottom cannot tear a frame; the
cost is that a resize is not noticed until the process restarts.
