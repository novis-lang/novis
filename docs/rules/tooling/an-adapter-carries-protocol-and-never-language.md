`nvs agent init` installs the surface into a project by writing one pointer per harness — an `AGENTS.md`
stanza, which is harness-neutral, and beside it an adapter for each harness that is present: a Claude
Code skill at `.claude/skills/novis/SKILL.md`, a Cursor rule at `.cursor/rules/novis.mdc`, and a GitHub
Copilot instructions file at `.github/instructions/novis.instructions.md`. Each names the four
`nvs agent` commands, the `nvs check` loop with its `--json` form, and `nvs test`.

**No adapter states a language fact.** Not a member signature, not a refusal, not a type. A language
fact written into an adapter is a copy that goes stale the day the member changes, and every copy is
read by an agent that has no way to know it is old — which is the failure the whole surface is arranged
to avoid. An adapter says where to ask; the binary answers.

**A pointer follows the installed binary.** Each unit `init` writes carries a marker with a
fingerprint of the text it wrote — BLAKE3 over the text without the marker line, `\r\n` read as `\n`,
eight hex digits. A re-run replaces a unit that still matches its fingerprint and refuses, naming it
and writing nothing, one that does not, unless `--force`. Every unit is judged before any is written.
`--check` writes nothing and fails while a unit is missing, outdated or edited, and `nvs agent primer`
puts one line on standard error while the working directory holds an outdated unit, so the agent
reading the primer learns of it with nobody in between.

That is what keeps the adapter list open. Another harness is another short pointer file, adding one
decides nothing and reopens nothing, and none of them can disagree with the language, because none of
them says anything about it.
