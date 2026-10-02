`nvs agent init` writes the files that tell a coding agent about the `nvs agent` commands. Run it
in the directory of your project.

The command adds a short section to the end of `AGENTS.md`, and your own text stays as it is. It
also writes one file for each agent tool the project uses: Claude Code (`.claude/`), Cursor
(`.cursor/`) and GitHub Copilot (`.github/instructions/`). With `--all`, it writes all of them.

These files name the commands and contain no facts about the language.

Run the command again after you install a new `nvs`. It prints `wrote <path>` for a new file,
`updated <path>` for a file it replaced, and `up to date` when there is nothing to do.

**Good to know:** each file has a fingerprint, so the command can see whether you changed the file.
It never replaces a file that you changed: it stops with an error and writes nothing. `--force`
replaces the file. `--check` writes nothing and fails when a file is missing, old or changed, so
you can run it in CI.
