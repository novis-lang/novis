`nvs agent init` writes the files that tell a coding agent about the `nvs agent` commands. Run it
in the directory of your project.

The command adds a short section to the end of `AGENTS.md`, and your own text in that file stays as
it is. When the project has no `AGENTS.md`, the command creates one. When the project has a
`.claude/` directory, the command also writes a Claude Code skill at
`.claude/skills/novis/SKILL.md`. With `--all`, it writes that file when there is no such directory
too.

These files contain no facts about the language. They name the commands, and the commands give the
facts. The command prints `wrote <path>` for each file it created, and `up to date` when it created
none.

**Good to know:** when the section or the skill file has other text than this version writes, the
command stops with an error and changes nothing. The message names the file. Delete the block and
run the command again.
