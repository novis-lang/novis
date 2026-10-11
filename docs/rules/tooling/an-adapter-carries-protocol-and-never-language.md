`nvs agent init` installs the surface into a project by writing an `AGENTS.md` stanza, which is
harness-neutral, and beside it what the coding agent that runs it needs — found from the environment
variable that agent sets in the commands it runs, or named with `--agent`. An agent that reads
`AGENTS.md` itself needs nothing more; Claude Code gets a skill at `.claude/skills/novis/SKILL.md`, and
GitHub Copilot an instructions file at `.github/instructions/novis.instructions.md`. A file an earlier
run wrote is kept current whichever agent runs it now. Each pointer names the four `nvs agent`
commands, the `nvs check` loop with its `--json` form, and `nvs test`.

**The agent is found by the variable it sets, never by the tree.** A directory in the tree says that
somebody on the project once used a tool, not that the agent running `init` now is it, so one person's
run would write files for another person's tool, and an agent whose tool left no directory would get
nothing. `--agent <name>` replaces detection and may be given more than once; `--all` installs for every
agent and conflicts with `--agent`. A run that finds none writes the stanza alone and says how to name
one, so a person running `init` by hand gets nothing for any one tool. Claude Code gets its skill
because it reads `AGENTS.md` only in a project with no `CLAUDE.md`, and Copilot its instructions file
because which Copilot surface reads `AGENTS.md` is not documented. The Cursor rule an earlier `init`
wrote, `.cursor/rules/novis.mdc`, is retired: a re-run deletes an untouched copy and leaves an edited
one with a note. Keeping every file an earlier run wrote current, whoever runs `init`, is what keeps
every person on a team on the same files. Each agent's row rests on its tool's own documentation:
Claude Code's environment variable, memory and hooks pages; Cursor's agent terminal, rules, hooks and
third-party hooks pages; and `codex-rs/core/src/exec_env.rs` in the Codex repository.

**A hook is protocol too.** Where an agent's tool runs a plain command after an edit, `init` merges
one entry into that tool's settings — `hooks.PostToolUse` in `.claude/settings.json` for Claude Code,
`hooks.postToolUse` in `.cursor/hooks.json` for Cursor — and `--no-hooks` removes it. The entry runs
`nvs agent hook <agent>`, which checks the edited `.nvs` file as `nvs check` does and gives its errors
back in the tool's JSON. It runs the checker and states no language fact of its own. An agent whose
tool has no plain command hook gets no hook, and no wrapper or plugin stands in for one: Codex's hooks
give the patch text rather than the edited file's path, Copilot's are a preview whose fields naming an
edited file are not documented, and OpenCode runs code after an edit only as a plugin.

An entry is found again by its exact command, so a project may change its matcher. A missing entry is
added at the end of the event's array for a chosen agent, and an entry already there is left whoever
runs `init`. `--no-hooks` removes the entry wherever it is and deletes a file left with nothing but the
skeleton `init` creates, and `--check` names a missing hook. `.claude/settings.local.json` is each
person's own and is never read or written. The merge reads the file into an order-keeping document of
its own and writes every key and entry back in the order it found them, because the workspace's JSON
map sorts its keys. A file that is not a JSON object, or whose `hooks` or event value has the wrong
type, refuses the whole run and `--force` does not override it: the file holds the project's other
settings, and a file `init` cannot read is one it cannot rewrite without losing them.

`nvs agent hook` reads the tool's payload on standard input and takes the path from `tool_input`, or a
top-level `file_path`. Anything that goes wrong before the check — a payload that is not JSON, no path,
a path not ending in `.nvs` — ends it silently with status 0, because a hook must never break the
agent. It checks in the payload's `cwd`, or Cursor's first workspace root, by starting itself again
there, because a binary that changes its own working directory counts as one that can read any file.
It never writes a default `nvs.toml`: it runs after every edit, and the project did not ask for one.
Only errors are given back, because a warning that stands would repeat after every edit until the
agent learned to pass over the report. The first five are rendered by position, then the count of the
others and the `nvs check <path>` that shows them all; the whole-run rendering would misstate a capped
list. Claude Code gets `{"decision": "block", "reason": …}` with status 0: after an edit, `block` does
not undo it, and the reason reaches the agent as feedback to act on. Cursor gets `additional_context`,
the only text its output has. Cursor also loads Claude Code's hooks, so the Claude Code hook prints
nothing under Cursor when `.cursor/hooks.json` carries Cursor's own entry, and a Cursor session hears
about an edit once. No live Claude Code or Cursor session has run the hook, and which field of
Cursor's `Write` input names the file is not documented.

The hook costs one front-end run of the edited file after every `.nvs` edit, a process start and a
JSON parse after every other edit, and a second process start when the tool starts it outside the
payload's directory.

**No adapter states a language fact.** Not a member signature, not a refusal, not a type. A language
fact written into an adapter is a copy that goes stale the day the member changes, and every copy is
read by an agent that has no way to know it is old — which is the failure the whole surface is arranged
to avoid. An adapter says where to ask; the binary answers.

**A pointer follows the installed binary.** Each unit `init` writes carries a marker with a
fingerprint of the text it wrote — BLAKE3 over the text without the marker line, `\r\n` read as `\n`,
eight hex digits. A re-run replaces a unit that still matches its fingerprint and refuses, naming it
and writing nothing, one that does not, unless `--force`. Every unit is judged before any is written.
`--check` writes nothing and fails while a unit is missing, outdated or edited, and `nvs agent primer`
puts one line on standard error while a re-run would change a unit in the working directory, so the
agent reading the primer learns of it with nobody in between.

The fingerprint is what tells an upgrade from an edit, which from the text alone look the same. The
stanza opens on its marker, and an adapter carries it on the line under its front matter as an HTML
comment, never a front-matter key a harness might reject. BLAKE3 is fixed by its specification and
already in the binary for the code cache, so the fingerprint is the same on every platform and from
every build. A unit an `init` before fingerprints wrote carries the old marker or none; its text is
known, so its fingerprint is a constant in the binary and the old text itself is not kept. An edited
unit gets no note from the primer: it is the project's own text. A marker somebody rewrites by hand so
that text and fingerprint agree reads as untouched and is updated; that is somebody imitating `init`,
and it is accepted.

That is what keeps the adapter list open. Another harness is another short pointer file, adding one
decides nothing and reopens nothing, and none of them can disagree with the language, because none of
them says anything about it.

**Declined:**
- **A version number in the marker.** It says which binary wrote the unit, not whether somebody edited
  it since, which is the one thing a re-run has to know.
- **Keeping every old text in the binary.** It grows with every release; a fingerprint per old text is
  eight characters, and only the text written before fingerprints needs one.
- **Choosing an agent by the directories in the tree.** They name the tools somebody used, not the
  one running, and `.github/` is in almost every repository whatever it uses.
- **Claude Code's `CLAUDECODE` variable.** An editor's integrated terminal started from Claude Code
  also sets it, so a person typing `nvs agent init` there would be taken for the agent.
- **Turning on `serde_json`'s `preserve_order`.** It reorders every other JSON document the workspace
  writes.
- **A wrapper script or a plugin for an agent with no plain command hook.** Where the tool has none,
  there is no hook.
- **Warnings in the hook's report.** A warning that stands repeats after every edit.
- **`hookSpecificOutput.additionalContext` for Claude Code.** It is the field for context that is not
  an error.
- **Exit status 2 with the report on standard error.** Cursor treats status 2 as a denial.
