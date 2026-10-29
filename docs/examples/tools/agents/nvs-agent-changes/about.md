`nvs agent changes` prints what changed in Novis. It reads the changelog that is built into the
`nvs` binary, so it describes the version you have installed.

Each release has one section, and the newest section is first. `--since 0.2.0` prints every release
after `0.2.0`. Without `--since`, the command prints the section of the installed version.

A coding agent runs this command before it edits a project that was written for an older version.
The *Breaking changes* heading lists what the project may need to change.

The *Formatter* heading lists every change to what `nvs fmt` writes. Run `nvs fmt` once after you
read it, and commit the result on its own.

**Good to know:** `--json` prints the same sections as one document, for a tool that reads JSON. The
command writes no file.
