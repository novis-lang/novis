`nvs.completion.phpNames` takes `all` (the default), `resolved` or `off`. `resolved` keeps only the
items that insert something — for a developer who has finished migrating and wants the reminder layer
quiet — and `off` removes the layer. The split is on answer quality rather than on a source, because
*show me only the ones that go somewhere* is the request people actually have; a boolean would collapse
it into switching the feature off.

The setting governs this layer only. `Core` member completion is ordinary language completion and never
sits behind it: `off` returns a response with no item of this layer and every other completion source
intact. A setting that could switch off completion for the language itself would be a bug in the
wiring, not a preference.

The setting lives in the editor extension's frozen contribution roster under that roster's own rule —
added once, never renamed.
