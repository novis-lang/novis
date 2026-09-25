- **A `// covers:` marker added at the top of a `--FILE--` block shifts every line number that
  case's `--EXPECTF-ERROR--` pins.** Such a section names the offending line three times over
  (`--> %s:29:14`, the gutter number and the caret row), and `%s` wildcards the path alone, so one
  inserted line fails the case on a number rather than on its claim — in `class/` and `lang/` as
  much as in `reject/`. Put the marker on the **last** line of the `--FILE--` block in any case
  carrying an `--EXPECTF-ERROR--` section, at the top everywhere else, and anywhere at all when
  `%A` swallows it. [until: gone tools/nv/proofs/collect.ts:COVERS_RE]
