- **A goal record's check can fail on a file no session wrote — the user edits this tree too.** An
  uncommitted hand edit to a spec or an inventory the check reads can be right and still fail it,
  and staging it to make the check pass takes the user's in-flight work into your commit. `git
  status --short` before diagnosing says whose change it is; a fix that lands in a *tool* —
  `tools/nv/cmd/migration.ts`'s `AHEAD_OF_THE_BUILD` is the shape — is committable on its own
  without touching those files. [until: gone tools/nv/cmd/orient.ts:const TRIAGE]
