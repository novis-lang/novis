`nvs fmt` rewrites a `.nvs` file into one canonical layout. There is no config file, no per-project or
per-directory override, and no flag that changes output: the result for a given input is a pure function
of that input and nothing else, the stance `rule:core-api/identifier-casing` takes for naming extended to
layout. The only flags are I/O modes — `--check`, `--diff`, `--stdin` and the target paths — never a
style knob. A configurable knob would let two files in one project, or two projects run through the same
tool, disagree about what "formatted" means, which is the exact question a canonical formatter exists to
close.

The style is PER for every construct PER covers (`rule:tooling/fmt-base-style-is-per`), extended
with one layout for each construct PER has never seen (`rule:tooling/fmt-novis-constructs`). It is
deterministic in the gofmt sense, not Prettier's: it never reflows an expression to fit a width
(`rule:tooling/fmt-never-reflows`), running it twice changes nothing (`rule:tooling/fmt-is-idempotent`),
and it is a separate opt-in tool no compiler command ever runs (`rule:tooling/fmt-is-never-a-diagnostic`).

Changing any of these rules later is a real diff across every already-formatted file, not a settings
change, so the rule set is stable once it ships. The formatter reads the lossless tree, never the strict
parse that drops comments — a walk over the strict tree would delete every comment in the file.
