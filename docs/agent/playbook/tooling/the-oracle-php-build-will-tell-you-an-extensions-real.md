- **The oracle PHP build will tell you an extension's real signatures, and a migration row written
  from memory instead is wrong in a way no test catches.** Which spelling is the deprecated alias of
  which and what a function returns are not derivable from `bun nv migration --report`'s name list.
  A six-line `ReflectionFunction` loop over `get_defined_functions()["internal"]`, run as `php
  .agent-tmp/<name>.php`, prints every parameter type, return type and `isDeprecated()` in one call;
  copy the `Core\X::y` spellings out of an already-green sibling section rather than inventing them.
  [until: gone tools/nv/cmd/migration.ts:--report]
