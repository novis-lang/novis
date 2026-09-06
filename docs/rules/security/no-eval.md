There is no `eval`, no string-argument `assert`, and no `Core` member that compiles and runs a string
produced at run time.

Four mechanisms depend on the set of code in a program being known before it runs: the static
inclusion graph a single-file executable bundles, the artifact cache keyed on unit content, definite
assignment, and taint tracking. `eval` does not weaken these one at a time — it makes all four unsound
at once, and the "no eval reached this file" analysis that would keep them is exactly as hard as not
having it.

The two legitimate uses have better answers already. Inspecting code as data is a parse that hands
back an inert tree with no path into execution (`rule:security/reflection-needs-no-capability`).
Running code chosen at run time is `spawn script`: in an isolate, spending the parent's budget, under
`script.spawn` — which is what a template engine or a plugin loader actually needs, with a boundary
`eval` never had.
