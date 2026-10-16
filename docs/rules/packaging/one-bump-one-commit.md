Each dependency — or one coherent group that genuinely must move together, such as the `cranelift-*`
crates — gets **its own commit**, with its classification from
`rule:packaging/who-can-see-it-decides-the-release-slot` in the message: `patch`, `minor` or
`major`.

The whole reason is `git bisect`: a bisect across a release then lands on the bump rather than on a
wall of them, and that is reason enough. Nothing checks the rule; it is addressed to whoever runs the
sweep (`rule:packaging/the-sweep-is-fired-by-a-human`), and the sweep's report is where a violation
becomes visible.
