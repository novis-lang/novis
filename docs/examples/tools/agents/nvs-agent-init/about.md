`nvs agent init` writes the files that tell a coding agent about the `nvs agent` commands. Run it
in the directory of your project.

The command adds a short section to the end of `AGENTS.md`, and your own text stays as it is.
Claude Code also gets a skill, and GitHub Copilot an instructions file. These files contain no
facts about the language.

Claude Code and Cursor also get a hook. After the agent edits a `.nvs` file, the hook checks the
file and shows the agent its errors. `--no-hooks` removes the hooks.

The command finds the agent that runs it. Run by hand, it writes only `AGENTS.md`. Use
`--agent claude-code` to choose an agent, and `--all` for every agent.

Run the command again after you install a new `nvs`. It prints `up to date` when there is nothing
to do.

**Good to know:** each file has a fingerprint, so the command can see whether you changed the file.
It never replaces a file that you changed. `--force` replaces it. `--check` writes nothing and fails
when a file is missing, old or changed, so you can run it in CI.
