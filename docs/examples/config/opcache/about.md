When this process looks at a source file it has already compiled.

Novis compiles what it runs and keeps the result. This block says whether it ever checks the file
behind that result again — never, whenever the timestamp moves, or whenever the contents hash
differently — and how often it is allowed to check at all.

**In plain words:** the first key decides how a deployment learns that its code changed, and the
second one keeps that learning from costing a disk check on every request.

Neither is a program's to choose. A request that could stop the re-check would be pinning the version
of the code it likes past the fix that was shipped for it, and one that could ask for a check every
time would be turning a hot path into a storm the whole deployment pays for.

What a host starts at, having written neither, is picked by the run mode: production never re-checks,
development watches timestamps. That is a starting point and nothing more — a value written in the
file wins over the mode beside it.

The example prints what this checkout re-checks and is turned away trying to change it.
