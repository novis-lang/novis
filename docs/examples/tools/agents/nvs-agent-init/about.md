`nvs agent init` writes the files that tell a coding agent about the `nvs agent` commands. Run it
in the directory of your project.

The command adds a short section to the end of `AGENTS.md`, and your own text in that file stays as
it is. When the project has no `AGENTS.md`, the command creates one. It also writes one file for
each agent tool the project uses: a Claude Code skill when there is a `.claude/` directory, a Cursor
rule when there is a `.cursor/` directory, and GitHub Copilot instructions when there is a
`.github/copilot-instructions.md` file or a `.github/instructions/` directory. With `--all`, it
writes all of them.

These files contain no facts about the language. They name the commands, and the commands give the
facts.

Run the command again after you install a new `nvs`. It prints `wrote <path>` for a new file,
`updated <path>` for a file it replaced with the current text, and `up to date` when there is
nothing to do. `nvs agent primer` prints a note when the files are older than the installed `nvs`.

**Good to know:** each file has a fingerprint, so the command can see whether you changed the file.
It never replaces a file that you changed. It stops with an error that names the file and writes
nothing. `--force` replaces the file. `--check` writes nothing and fails when a file is missing,
old or changed, so you can run it in CI.
