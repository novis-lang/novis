# this file as a temporary decision and future plans note
Do not read it, do not use it in your daily workflow. Its a reminder note for humans only.


## Big index file
We should generate one big file with all the core features, syntax, everything what the language contains and is about, with correct anchors, keywords, descriptions, etc... so search engines, llms and human can have one complete file for everything that exist in novis.

The file need to be markdown. Well structured. Add instructions add the top of how a machine/llm/humans should traverse through this document without requiring to read all of it at once. So, if a llm need to just understand the whole syntax, it should be easily findable. Same rules goes on for every core feature, enum, constant, method, function, class, etc...

The file should be getting automatically generated of single files, so we dont need to edit the huge file as a whole.
Create examples for all the core features, syntax, etc... You can reuse existing examples.
Note: We should not have duplications. One method should be described once, with all possibilities. Short, clear, readable.

Note: probably we should not reuse the website examples, they are multiple and to much. They show features twice or more often, the summary file requires only a complete feature list with no duplications.

Create tooling for that, that does regenarate the sum file automatically. Make sure, it will be automatically updated, whenever we change the core. Should be fast, so calling it should not take that long.
We should be ready for this kind of automatism, because our repo is setup very structurally, a lot of that is already in the rust source.

Proof it, by reading it yourself and check if you can generate code in any depth just out of this one huge file. Act like a new llm seing this file or the language for the first time, not knowing anything about this repository. Ask it questions of an coding average job and watch if the llm can generate fully functional files and examples. Verify this by running it against our binary. You can use a subagent for this with a clean context. Ask it to create simple and more complex codes (also can span over multiple files). Ask it to generate the code and how the user then can execute it.

Also create a tool for the "proof", so we can rerun it later with an agent.

Only add things that actually already exist in the core, not whats just only planned.

If you have questions, ask me. If you have recommendations, show me.
When you doubt that this is a good idea, also show it to me.


## nvs doc — an API documentation generator
Novis has no answer to phpDocumentor/Doctum: nothing renders API docs from a user's own program.
The compiler already holds every signature and doc comment (Part B of docs/novis.md is generated
from the same registry), so an `nvs doc` is cheap the day someone wants it. Decision 2026-09-01:
recorded here only — no milestone owns it, and it gets an ADR when it is scheduled.
docs/adr/tooling-parity.md carries the tool-by-tool context.


## nvs fix — the reopen trigger for Novis-to-Novis rewrites
ADR 0039 § 9 declined an `nvs fix` batch-rewrite verb ("no user has asked for one yet"), and
`nvs convert` only covers PHP to Novis. Decision 2026-09-01: the door stays closed before 1.0, but
the first breaking change to the language surface after 1.0 must ship with an automated migration
rewrite — reopening ADR 0039's declination is part of that change's cost, so it is written down
here where a human planning that change will look.