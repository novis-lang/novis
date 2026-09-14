`.gitignore` carries `node_modules/`, `out/`, `.vscode-test/` and `*.vsix`: a session that commits
`node_modules` is a session whose commit nobody can review. All four are anchored under `editors/` —
`/editors/*/out/` rather than `out/` — because the repository already has a documentation directory
named `out`, and an unanchored pattern silently stops tracking new files in it.

`package-lock.json` **is** committed, because `npm ci` is what the acceptance run uses and it requires one,
and because an unpinned dependency tree makes the grammar snapshots reproducible only by luck. CI carries
two jobs for the package beside the ones already there — `extension`, the headless suites on all three
platforms, and `extension-host` on Linux — and `ci.yml` is the count of those.
