`.gitignore` carries `node_modules/`, `out/`, `.vscode-test/` and `*.vsix`: a session that commits
`node_modules` is a session whose commit nobody can review.

`package-lock.json` **is** committed, because `npm ci` is what the acceptance run uses and it requires one,
and because an unpinned dependency tree makes the grammar snapshots reproducible only by luck. CI grows two
jobs beside the ones already there — the headless suites on all three platforms and the extension-host run
on Linux — and `ci.yml` is the count of those.
