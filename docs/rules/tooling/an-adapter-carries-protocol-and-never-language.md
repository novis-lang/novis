`nvs agent init` installs the surface into a project by writing one pointer per harness — an `AGENTS.md`
stanza, which is harness-neutral, and beside it an adapter for each harness that is present, such as a
Claude Code skill at `.claude/skills/novis/SKILL.md`. Each names the four `nvs agent` commands and the
`nvs check` loop.

**No adapter states a language fact.** Not a member signature, not a refusal, not a type. A language
fact written into an adapter is a copy that goes stale the day the member changes, and every copy is
read by an agent that has no way to know it is old — which is the failure the whole surface is arranged
to avoid. An adapter says where to ask; the binary answers.

That is what keeps the adapter list open. Another harness is another short pointer file, adding one
decides nothing and reopens nothing, and none of them can disagree with the language, because none of
them says anything about it.
