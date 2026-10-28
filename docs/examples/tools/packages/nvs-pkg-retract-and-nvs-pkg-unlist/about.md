`nvs pkg retract <version> --reason <text>` marks a version of your package as broken or unsafe.
`nvs pkg add` and `nvs pkg update` no longer choose it. The package index shows the reason. The
reason cannot be empty, and it has at most 500 characters. `--fixed-in` names the version that
fixes the problem.

`nvs pkg unlist` hides your package, or one version of it, from search and listings. `--undo`
shows it again. Only an owner who signs in with a passkey can unlist.

Run both commands in the folder of your package. No version is ever deleted. A retracted or
unlisted version can still be downloaded, so a `package.lock` that names it still works.

**Good to know:** with a token, a retraction starts seven days later, and every owner gets a
message. The people who run the registry can withhold a version that contains malware. A withheld
version cannot be downloaded.
