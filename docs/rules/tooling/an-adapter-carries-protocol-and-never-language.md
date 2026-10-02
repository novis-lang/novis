`nvs agent init` installs the surface into a project by writing an `AGENTS.md` stanza, which is
harness-neutral, and beside it what the coding agent that runs it needs — found from the environment
variable that agent sets in the commands it runs, or named with `--agent`. An agent that reads
`AGENTS.md` itself needs nothing more; Claude Code gets a skill at `.claude/skills/novis/SKILL.md`, and
GitHub Copilot an instructions file at `.github/instructions/novis.instructions.md`. A file an earlier
run wrote is kept current whichever agent runs it now. Each pointer names the four `nvs agent`
commands, the `nvs check` loop with its `--json` form, and `nvs test`.

**A hook is protocol too.** Where an agent's tool runs a plain command after an edit, `init` merges
one entry into that tool's settings — `hooks.PostToolUse` in `.claude/settings.json` for Claude Code,
`hooks.postToolUse` in `.cursor/hooks.json` for Cursor — and `--no-hooks` removes it. The entry runs
`nvs agent hook <agent>`, which checks the edited `.nvs` file as `nvs check` does and gives its errors
back in the tool's JSON. It runs the checker and states no language fact of its own. An agent whose
tool has no plain command hook gets no hook, and no wrapper or plugin stands in for one.

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

That is what keeps the adapter list open. Another harness is another short pointer file, adding one
decides nothing and reopens nothing, and none of them can disagree with the language, because none of
them says anything about it.
