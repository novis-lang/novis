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
